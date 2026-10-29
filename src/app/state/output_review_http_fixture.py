"""Local, counted real-tool input server; all artifacts belong to the run root."""
import http.server
import json
import os
from pathlib import Path
import sys
import threading
import time
import urllib.parse

root = Path(sys.argv[1])
lock = threading.Lock()
count = 0
retry_requests = {}


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_HEAD(self):
        self.respond(False)

    def do_GET(self):
        self.respond(True)

    def respond(self, body):
        global count
        path = urllib.parse.urlsplit(self.path).path
        status, reason = 200, None
        if "/metadata-block" in path:
            (root / "metadata-blocked.txt").write_text(self.path, encoding="utf-8")
            time.sleep(30)  # Cancel must settle the tool without waiting here.
        if path.startswith("/metadata/") and path.endswith("feed.xml"):
            prefix = path.rsplit("/", 1)[0]
            address = f"http://{self.headers['Host']}{prefix}"
            data = (f'<?xml version="1.0"?><rss version="2.0"><channel>'
                    f'<title>Metadata playlist</title>'
                    f'<item><title>Metadata A</title><link>{address}/a.html</link></item>'
                    f'<item><title>Metadata B</title><link>{address}/b.html</link></item>'
                    f'</channel></rss>').encode()
            kind = "application/rss+xml"
        elif path.startswith("/metadata/") and path.endswith(".html"):
            title = "Metadata " + path.rsplit("/", 1)[1].split(".")[0].upper()
            data = (f'<html><head><title>{title}</title></head>'
                    f'<body><audio controls><source src="http://{self.headers["Host"]}/source.flac" '
                    f'type=\'audio/flac; codecs="flac"\'></audio></body></html>').encode()
            kind = "text/html"
            if path.endswith("retry.html"):
                with lock:
                    retry_requests[path] = retry_requests.get(path, 0) + int(body)
                    reject = body and retry_requests[path] == 2
                if reject:
                    status, reason = 403, "The page needs to be reloaded"
                    data = reason.encode()
        elif path.endswith(".flac"):
            data, kind = (root / "source.flac").read_bytes(), "audio/flac"
        elif path.endswith(".wav"):
            data, kind = (root / "source.wav").read_bytes(), "audio/wav"
        elif path.endswith("/alternate.webm"):
            data, kind = (root / "alternate.webm").read_bytes(), "video/webm"
        elif path.endswith((".m4a", ".webm")):
            ext = path.rsplit(".", 1)[1]
            data, kind = (root / f"dual-audio.{ext}").read_bytes(), f"audio/{ext}"
        elif path.endswith(".jpg"):
            data, kind = (root / "cover.jpg").read_bytes(), "image/jpeg"
        elif path.endswith(".vtt"):
            data, kind = b"WEBVTT\n\n00:00.000 --> 00:00.300\nExample subtitle\n", "text/vtt"
        else:
            data, kind = (root / "source.mp4").read_bytes(), "video/mp4"
        with lock:
            if body and path.endswith((".mp4", ".wav", ".m4a", ".webm", ".flac")):
                count += 1
            temporary = root / "media-get-count.tmp"
            temporary.write_text(str(count), encoding="utf-8")
            os.replace(temporary, root / "media-get-count.txt")
            with (root / "http-requests.jsonl").open("a", encoding="utf-8") as log:
                log.write(json.dumps({"method": self.command, "path": path, "media_gets": count,
                                      "status": status, "cookie_present": bool(self.headers.get("Cookie")),
                                      "user_agent": self.headers.get("User-Agent")}) + "\n")
        self.send_response(status, reason)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.close_connection = True
        if body:
            try:
                self.wfile.write(data)
            except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                pass  # yt-dlp may close its metadata probe early.


(root / "media-get-count.txt").write_text("0", encoding="utf-8")
server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
(root / "http-port.txt").write_text(str(server.server_port), encoding="utf-8")
server.serve_forever()
