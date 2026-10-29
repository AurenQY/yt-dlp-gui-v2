# SPDX-License-Identifier: GPL-2.0-only
"""Standalone gallery-dl entry point with bounded capability reporting."""
import json
import sys

sys.dont_write_bytecode = True


def main():
    if sys.argv[1:] == ["--yt-dlp-gui-cache-capabilities"]:
        import ssl
        ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        import yt_dlp.version
        print(json.dumps({
            "protocol": "yt-dlp-gui-cache-policy/v1",
            "upstream_version": "1.32.18",
            "source_commit": "175d21fd8f7e52963911c02916d0634461cddb24",
            "patch_revision": 1,
            "yt_dlp_version": yt_dlp.version.__version__,
            "platform": "windows-x86_64",
            "features": ["owned-part-directory", "playlist-artifact-finalization",
                         "actual-artifact-events", "metadata-no-media-write"],
        }, sort_keys=True))
        return 0
    import gallery_dl
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(line_buffering=True, write_through=True)
    return gallery_dl.main()


if __name__ == "__main__":
    sys.exit(main())
