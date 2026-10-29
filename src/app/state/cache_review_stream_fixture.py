"""Synthetic extractor using the pinned real yt-dlp engine and cache API."""
import os
import pathlib
import subprocess
import sys
import time


def wait(path):
    end = time.monotonic() + 45
    while not path.exists():
        if time.monotonic() > end:
            raise RuntimeError(f"Fixture gate timeout: {path}")
        time.sleep(0.01)


if sys.argv[1:2] == ["--drain"]:
    control = pathlib.Path(sys.argv[2])
    (control / "draining").write_text("inherited stderr is still open")
    wait(control / "drain-release")
    sys.exit(0)

import yt_dlp
from yt_dlp.extractor.common import InfoExtractor
from yt_dlp.globals import extractors


class ReviewStreamIE(InfoExtractor):
    _VALID_URL = r"reviewstream:(?P<id>.+)"

    def _real_extract(self, url):
        control = pathlib.Path(self._match_id(url))
        (control / "startup").write_text(yt_dlp.version.__version__)
        wait(control / "start")
        self._downloader.cache.store("review-stream", "active", {"synthetic": True})
        assert self._downloader.cache.load("review-stream", "active")["synthetic"]
        (control / "cache-access").write_text("real engine cache store/load complete")
        wait(control / "finish")
        if control.name.endswith("drain"):
            # Only a stderr handle survives. No detached cache writer exists.
            # The real owner must wait for pipe consumption after its child exits.
            subprocess.Popen([sys.executable, "-B", __file__, "--drain", str(control)],
                             stdout=subprocess.DEVNULL, stderr=sys.stderr,
                             stdin=subprocess.DEVNULL, close_fds=False)
            if "flat-update" in control.name:
                os._exit(0)  # Exercise its no-entry + stderr-drain branch.
        return self.playlist_result([
            {"_type": "url", "url": "https://example.invalid/offline", "id": "entry", "title": "Synthetic offline entry", "ie_key": "Generic"}
        ], playlist_id="synthetic", playlist_title="Synthetic list")


yt_dlp.extractor.import_extractors()
extractors.value = {"ReviewStreamIE": ReviewStreamIE, **extractors.value}
yt_dlp.main()
