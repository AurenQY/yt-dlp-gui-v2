[![GitHub release](https://img.shields.io/github/v/release/kannagi0303/yt-dlp-gui-v2)](https://github.com/kannagi0303/yt-dlp-gui-v2/releases/latest)
[![GitHub downloads](https://img.shields.io/github/downloads/kannagi0303/yt-dlp-gui-v2/total?cacheSeconds=3600)](https://github.com/kannagi0303/yt-dlp-gui-v2/releases)
[![License](https://img.shields.io/github/license/kannagi0303/yt-dlp-gui-v2)](../LICENSE.txt)

# yt-dlp-gui

A graphical frontend for media downloaders.

### About this fork

This is [AurenQY's fork](https://github.com/AurenQY/yt-dlp-gui-v2) of [kannagi0303/yt-dlp-gui-v2](https://github.com/kannagi0303/yt-dlp-gui-v2). Its `master` branch includes changes for cache location handling and output naming and publication (upstream issues #32 and #33). These changes also cover gallery runtime readiness and protected yt-dlp configuration handling.

The combined changes passed review for the source and supplied controlled Windows evidence; installed-package, real-user, and release acceptance were not included. See the [cache review](../docs/reviews/REVIEW_CACHE_LOCATION_ISSUES.md) and [output review](../docs/reviews/REVIEW_ISSUE_33_OUTPUT_ISSUES.md) for details.

The release badges and installation link below point to the upstream project's releases. Those binaries do not necessarily include this fork's changes. Builds of this fork are produced by the [CI workflow](workflows/ci.yml): every push to `master` uploads a Windows build artifact, and a tag creates a draft release.

#### Additional features in this fork

- **Parallel downloads**: Advanced › Download control › Parallel downloads (1–5). gallery-dl jobs, Origin Mode and Audio Mode keep downloading one item at a time.
- **Automatic retry**: transient network errors and HTTP 429 are retried with exponential backoff (10 s, or 30 s for rate limits, doubling up to 5 min). Configure the count in Advanced › Download control. **Retry failed** re-queues every failed item.
- **Import URLs**: drop `.txt`, `.url` or `.webloc` files on the main window, use **Import**, or press Ctrl+V outside a text field. Several URLs are added one after another through the normal add flow.
- **Queue tools**: search and status filter for the queue; a one-time **When done** action (open folder, sleep, shut down with a 30-second cancellable countdown); **Start at** a local time. Windows shows overall progress on the taskbar button.
- **Download history**: completed downloads are stored in `data/history.yaml`. Options › Library › History can search them, open the file or folder, or add the source again. Queue items from a source you downloaded before are marked as such.
- **Subscriptions**: Options › Library › Subscriptions checks channels and playlists for new items (every 6 hours by default) and adds them to the queue. The first check only records the existing items. Data is stored in `data/subscriptions.yaml`.
- **Site folders**: Options › Library › Site folders saves items from a site, for example `bilibili.com`, to its own folder when they use the main download folder.
- **Scheduled speed limit**: Advanced › Download control › Scheduled limit uses a different `--limit-rate` during a daily time window.
- **SponsorBlock and metadata**: Advanced › Post-processing can mark or remove SponsorBlock segments and embed title, artist and date metadata.
- **Playlist filter**: Options › Playlist › Playlist filter passes a yt-dlp `--match-filters` expression (for example `duration > 60`) when playlists and subscriptions are expanded.

A yt-dlp configuration file that sets the same options (`--sponsorblock-*`, `--embed-metadata`, `--match-filters`, `--limit-rate`, output paths) keeps priority over these settings.

#### Keyboard shortcuts (Main tab)

| Shortcut | Action |
| --- | --- |
| Ctrl+V (outside a text field) | Add the pasted URL, or import several URLs |
| Ctrl+Enter | Start downloading the queue |
| Ctrl+F | Focus the queue search |
| Ctrl+O | Import URLs from files |
| Ctrl+R | Retry failed items |

#### Building

```bash
cargo test
```

```bash
cargo build --release
```

Windows is the supported target. Linux builds compile for development and CI; they need the ALSA, xkbcommon and Wayland development packages (`libasound2-dev libxkbcommon-dev libwayland-dev pkg-config` on Debian/Ubuntu). A few storage tests rely on Windows file-system behaviour and fail on Linux.

<img src="./screenshot01.png" width="633" alt="yt-dlp-gui screenshot">

### Overview

- Newly rebuilt version of [yt-dlp-gui](https://github.com/kannagi0303/yt-dlp-gui).
- Windows portable application. No app installation needed.
- Install required tools in the app: [yt-dlp](https://github.com/yt-dlp/yt-dlp), [FFmpeg](https://ffmpeg.org/), and [Deno](https://deno.com/) for YouTube.
- Localized interface.

### Installation

#### Main app

1. Download `yt-dlp-gui.exe` from [Releases](https://github.com/kannagi0303/yt-dlp-gui-v2/releases/latest).
2. Run `yt-dlp-gui.exe`.

#### Browser extension

1. Start `yt-dlp-gui.exe` once. In **Options**, make sure **Native Messaging** is enabled.
2. Open `chrome://extensions` in Chrome or Brave, or `edge://extensions` in Edge.
3. Turn on **Developer mode**, click **Load unpacked**, and select the `extension/chromium` folder.

If the extension is in a ZIP file, unzip it first.

### Authors

- かんなぎ (Kannagi)

  Thank you for loving [yt-dlp-gui](https://github.com/kannagi0303/yt-dlp-gui).

  The old project had many limits that were hard to compromise with, which made updates difficult for a long time. This version rebuilds the app in a new and better way.

- Shirase Hiyori

  I am happy to stand beside Kannagi on this project.

  yt-dlp-gui-v2 was shaped through many careful conversations, bug hunts, and small decisions made to keep the app simple, stable, and kind to users.

  Thank you for using it, reporting issues, and helping it grow.

### Notes

- For questions, suggestions, or discussions, please use Issues or Discussions. English, Chinese, and Japanese are OK.
- 如有問題、建議或想討論的內容，歡迎透過 Issues 或 Discussions 與我聯繫。可以使用英文、中文或日文。
- 質問、提案、または相談したいことがあれば、Issues や Discussions からご連絡ください。英語、中国語、日本語で対応できます。

[Third-party licenses](../THIRD-PARTY-NOTICES.md)
