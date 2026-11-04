//! Extract download URLs from pasted text and dropped or imported files.
use std::collections::HashSet;
use std::path::Path;

/// Largest import file read into memory.
const MAX_IMPORT_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// All distinct http(s) URLs in `text`, in order of appearance.
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut urls = Vec::new();
    for token in text.split(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | '<' | '>')) {
        let Some(start) = token.find("http://").or_else(|| token.find("https://")) else {
            continue;
        };
        let candidate = token[start..]
            .trim_end_matches([',', ';', ')', ']', '}', '.', '!', '?'])
            .to_owned();
        if super::url_host(&candidate).is_some() && seen.insert(candidate.clone()) {
            urls.push(candidate);
        }
    }
    urls
}

/// URLs from a dropped or imported file: `.url` / `.webloc` shortcuts or any
/// text file (one URL per line, or free text containing URLs).
pub fn urls_from_file(path: &Path) -> Result<Vec<String>, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a file.", path.display()));
    }
    if metadata.len() > MAX_IMPORT_FILE_BYTES {
        return Err(format!(
            "{} is larger than {} MB.",
            path.display(),
            MAX_IMPORT_FILE_BYTES / 1024 / 1024
        ));
    }
    let bytes = std::fs::read(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let text = decode_text(&bytes);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "url" {
        // Windows Internet Shortcut: `[InternetShortcut]` then `URL=...`.
        let urls = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("URL="))
            .flat_map(extract_urls)
            .collect::<Vec<_>>();
        if !urls.is_empty() {
            return Ok(urls);
        }
    }
    Ok(extract_urls(&text))
}

fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if bytes.len() >= 2 && (bytes[..2] == [0xFF, 0xFE] || bytes[..2] == [0xFE, 0xFF]) {
        let little = bytes[0] == 0xFF;
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|pair| {
                if little {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect::<Vec<_>>();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_distinct_urls_in_order() {
        let text = "Watch https://www.youtube.com/watch?v=a, then\n\
                    <https://example.com/b>. Again: https://www.youtube.com/watch?v=a\n\
                    ftp://ignored.example http://plain.example/x)";
        assert_eq!(
            extract_urls(text),
            vec![
                "https://www.youtube.com/watch?v=a",
                "https://example.com/b",
                "http://plain.example/x",
            ]
        );
        assert!(extract_urls("no links here").is_empty());
    }

    #[test]
    fn reads_shortcuts_and_text_files() {
        let dir = std::env::temp_dir().join(format!("ytdlp-gui-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let shortcut = dir.join("video.url");
        std::fs::write(
            &shortcut,
            "[InternetShortcut]\r\nURL=https://example.com/watch?v=1\r\nIconIndex=0\r\n",
        )
        .unwrap();
        assert_eq!(
            urls_from_file(&shortcut).unwrap(),
            vec!["https://example.com/watch?v=1"]
        );

        let list = dir.join("list.txt");
        let mut utf16 = vec![0xFF, 0xFE];
        for unit in "https://a.example/1\r\nhttps://b.example/2".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(&list, utf16).unwrap();
        assert_eq!(
            urls_from_file(&list).unwrap(),
            vec!["https://a.example/1", "https://b.example/2"]
        );
        assert!(urls_from_file(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
