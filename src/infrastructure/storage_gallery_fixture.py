"""Synthetic extractors, real gallery-dl/yt-dlp downloaders, localhost bytes only.

This fixture does not replace downloader, path, cache, or finalization logic.
It adds deterministic extractors to avoid real sites and personal credentials.
"""
import http.server
import atexit
import json
import os
from pathlib import Path
import struct
import zlib
import sys
import threading
import time

import gallery_dl
from gallery_dl import extractor
from gallery_dl.extractor.common import Extractor, Message
import yt_dlp
from yt_dlp.extractor.common import InfoExtractor
from yt_dlp.globals import extractors


def png_chunk(kind, payload):
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload))


PNG = (b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0))
       + png_chunk(b"IDAT", zlib.compress(b"\x00\xff\x00\x00")) + png_chunk(b"IEND", b""))


class MediaServer(http.server.BaseHTTPRequestHandler):
    transfers = 0

    def do_GET(self):
        type(self).transfers += 1
        if self.path.startswith("/missing"):
            self.send_error(404)
            return
        media_root = os.environ.get("YT_DLP_GUI_FIXTURE_MEDIA")
        payload = ((Path(media_root) / self.path.lstrip("/")).read_bytes() if media_root
                   else PNG if self.path.endswith(".png") else b"S" * (256 * 1024))
        self.send_response(200)
        self.send_header("Content-Type", "image/png" if self.path.endswith(".png") else "video/mp4")
        self.send_header("Content-Length", len(payload))
        self.end_headers()
        for start in range(0, len(payload), 2048):
            self.wfile.write(payload[start:start + 2048])
            self.wfile.flush()
            time.sleep(0.005)

    def log_message(self, *_):
        pass


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), MediaServer)
threading.Thread(target=server.serve_forever, daemon=True).start()
media_url = f"http://127.0.0.1:{server.server_port}/fixture.mp4"


class StoragePolicyIE(InfoExtractor):
    _VALID_URL = r"storagepolicy:(?P<id>single|playlist|nested|failed|collision|metadata|images|remux|merged)"

    def _real_extract(self, url):
        kind = self._match_id(url)
        self._downloader.cache.store("storage-policy", kind, {"synthetic": True})
        videos = [{"id": f"fixture-{i}", "title": f"fixture-{i}", "url": media_url,
                   "ext": "mp4", "format_id": "fixture", "playlist_index": i}
                  for i in (1, 2, 3)]
        if kind == "failed":
            videos[2]["url"] = media_url.replace("/fixture", "/missing")
        if kind == "collision":
            branches = []
            for number in (1, 2):
                branch = self.playlist_result([dict(videos[0])], f"branch-{number}")
                branch.update(extractor=self.IE_NAME, extractor_key=self.ie_key(),
                              webpage_url=f"storagepolicy:branch-{number}")
                branches.append(branch)
            return self.playlist_result(branches, "colliding-playlist")
        if kind == "images":
            for video in videos:
                video.update(url=media_url.replace(".mp4", ".png"), ext="png")
        if kind == "merged":
            for video in videos:
                video.pop("url")
                video.pop("format_id")
                video["formats"] = [
                    {"format_id": "video", "url": media_url, "ext": "mp4", "vcodec": "mpeg4", "acodec": "none"},
                    {"format_id": "audio", "url": media_url.replace("fixture.mp4", "audio.m4a"), "ext": "m4a", "vcodec": "none", "acodec": "aac"},
                ]
        if kind in {"playlist", "failed", "collision", "images", "remux", "merged"}:
            return self.playlist_result(videos, "synthetic-playlist")
        if kind == "nested":
            inner = self.playlist_result(videos, "nested")
            inner.update(extractor=self.IE_NAME, extractor_key=self.ie_key(), webpage_url="storagepolicy:inner")
            return self.playlist_result([inner], "synthetic-playlist")
        return videos[0]


yt_dlp.extractor.import_extractors()
extractors.value = {"StoragePolicyIE": StoragePolicyIE, **extractors.value}


@extractor.add
class StorageFixtureExtractor(Extractor):
    category = "storagefixture"
    subcategory = "item"
    pattern = r"storagefixture:(http|single|playlist|nested|failed|collision|images|remux|merged)$"
    directory_fmt = ("storagefixture",)
    filename_fmt = "{filename}.{extension}"
    archive_fmt = "{filename}"

    def __init__(self, match):
        super().__init__(match)
        self.kind = match[1]

    def items(self):
        metadata = {"filename": "fixture", "extension": "mp4", "title": "fixture"}
        yield Message.Directory, "", metadata
        url = media_url if self.kind == "http" else "ytdl:storagepolicy:" + self.kind
        yield Message.Url, url, metadata


def finish():
    server.shutdown()
    print("fixture-transfers=" + json.dumps({"get": MediaServer.transfers}), file=sys.stderr)


atexit.register(finish)
if __name__ == "__main__":
    print(f"gallery-dl {gallery_dl.version.__version__}; yt-dlp {yt_dlp.version.__version__}", file=sys.stderr)
    sys.exit(gallery_dl.main())
