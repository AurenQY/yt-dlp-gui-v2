use super::*;
use std::io::{Cursor, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Instant;

fn png() -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 3)
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    output.into_inner()
}

fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    Instant::now() < deadline,
                    "test proxy received no connection"
                );
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("{error}"),
        }
    }
}

fn respond(stream: &mut TcpStream, status: u16, bytes: &[u8]) {
    let header = read_header(stream);
    if header.starts_with(b"CONNECT ") {
        stream
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .unwrap();
        read_header(stream);
    }
    write!(
        stream,
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )
    .unwrap();
    stream.write_all(bytes).unwrap();
}

fn read_header(stream: &mut TcpStream) -> Vec<u8> {
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 8192);
    }
    header
}

fn http_proxy(responses: Vec<(u16, Vec<u8>)>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        for (status, bytes) in responses {
            respond(&mut accept(&listener), status, &bytes);
        }
    });
    (proxy, worker)
}

#[test]
fn thumbnail_missing_and_invalid_images_fall_back_through_http_proxy() {
    let (proxy, worker) = http_proxy(vec![
        (404, Vec::new()),
        (200, b"not an image".to_vec()),
        (200, png()),
    ]);
    let urls =
        ["missing", "invalid", "good"].map(|path| format!("http://thumbnail.invalid/{path}"));
    let result = fetch_thumbnail_candidates(&urls, &proxy, false);
    worker.join().unwrap();
    assert_eq!(result.unwrap().size, [2, 3]);
}

#[test]
fn thumbnail_service_failure_retries_later_without_scanning_candidates() {
    let (proxy, worker) = http_proxy(vec![(503, Vec::new())]);
    let urls = ["first", "second"].map(|path| format!("http://thumbnail.invalid/{path}"));
    let result = fetch_thumbnail_candidates(&urls, &proxy, false);
    worker.join().unwrap();
    let error = result.unwrap_err();
    assert!(error.retryable);
    assert!(!error.try_next_candidate);
    assert!(error.message.contains("503"));
}

#[test]
fn thumbnail_http_auth_failures_are_terminal_and_missing_images_can_fall_back() {
    for status in [401, 403] {
        let error = format_ureq_error(ureq::Error::StatusCode(status));
        assert!(!error.retryable);
        assert!(!error.try_next_candidate);
    }
    for status in [404, 410] {
        let error = format_ureq_error(ureq::Error::StatusCode(status));
        assert!(!error.retryable);
        assert!(error.try_next_candidate);
    }
}

#[test]
fn thumbnail_transient_retry_budget_stops_after_three_attempts() {
    assert!(thumbnail_retry_delay(1).is_some());
    assert!(thumbnail_retry_delay(2) > thumbnail_retry_delay(1));
    assert!(thumbnail_retry_delay(3).is_none());
    assert!(thumbnail_retry_delay(4).is_none());
}

#[test]
fn thumbnail_worker_reports_transient_failure_then_recovers_on_retry() {
    let (proxy, server) = http_proxy(vec![(503, Vec::new()), (200, png())]);
    let (tx, rx) = std::sync::mpsc::channel();
    let urls = vec!["http://thumbnail.invalid/recover".to_owned()];
    run_thumbnail_fetch_worker(
        "test".to_owned(),
        urls.clone(),
        proxy.clone(),
        false,
        1,
        tx.clone(),
    );
    let first = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(first.result.is_err());
    assert!(first.retryable);
    assert_eq!(first.attempt, 1);
    run_thumbnail_fetch_worker("test".to_owned(), urls, proxy, false, 2, tx);
    let second = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    server.join().unwrap();
    assert_eq!(second.attempt, 2);
    assert_eq!(second.result.unwrap().size, [2, 3]);
}

#[test]
fn thumbnail_socks5_proxy_handshake_delivers_decodable_image() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy = format!("socks5://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let mut stream = accept(&listener);
        let mut greeting = [0; 2];
        stream.read_exact(&mut greeting).unwrap();
        assert_eq!(greeting[0], 5);
        let mut methods = vec![0; greeting[1] as usize];
        stream.read_exact(&mut methods).unwrap();
        assert!(methods.contains(&0));
        stream.write_all(&[5, 0]).unwrap();
        let mut connect = [0; 4];
        stream.read_exact(&mut connect).unwrap();
        assert_eq!(&connect[..3], &[5, 1, 0]);
        let address_len = match connect[3] {
            1 => 4,
            4 => 16,
            3 => {
                let mut length = [0];
                stream.read_exact(&mut length).unwrap();
                length[0] as usize
            }
            other => panic!("unexpected SOCKS address type: {other}"),
        };
        let mut target = vec![0; address_len + 2];
        stream.read_exact(&mut target).unwrap();
        stream
            .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 80])
            .unwrap();
        respond(&mut stream, 200, &png());
    });
    let result = fetch_thumbnail_image("http://127.0.0.1/thumbnail", &proxy, false);
    worker.join().unwrap();
    assert_eq!(result.unwrap().size, [2, 3]);
}

#[test]
fn thumbnail_candidates_preserve_site_ranking_and_remove_duplicates() {
    let json = serde_json::json!({
        "thumbnail": "https://example.test/other.png",
        "thumbnails": [
            {"url": "https://example.test/small.png", "width": 10, "height": 10},
            {"url": "https://example.test/large.png", "width": 100, "height": 100},
            {"url": "https://example.test/large.png", "width": 100, "height": 100}
        ]
    });
    assert_eq!(
        crate::app::metadata::thumbnail_candidate_urls(&json),
        [
            "https://example.test/large.png",
            "https://example.test/small.png",
            "https://example.test/other.png",
        ]
    );
}
