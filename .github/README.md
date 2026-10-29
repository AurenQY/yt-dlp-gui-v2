[![GitHub release](https://img.shields.io/github/v/release/kannagi0303/yt-dlp-gui-v2)](https://github.com/kannagi0303/yt-dlp-gui-v2/releases/latest)
[![GitHub downloads](https://img.shields.io/github/downloads/kannagi0303/yt-dlp-gui-v2/total?cacheSeconds=3600)](https://github.com/kannagi0303/yt-dlp-gui-v2/releases)
[![License](https://img.shields.io/github/license/kannagi0303/yt-dlp-gui-v2)](../LICENSE.txt)

# yt-dlp-gui

A graphical frontend for media downloaders.

### About this fork

This is [AurenQY's fork](https://github.com/AurenQY/yt-dlp-gui-v2) of [kannagi0303/yt-dlp-gui-v2](https://github.com/kannagi0303/yt-dlp-gui-v2). Its `master` branch includes changes for cache location handling and output naming and publication (upstream issues #32 and #33). These changes also cover gallery runtime readiness and protected yt-dlp configuration handling.

The combined changes passed review for the source and supplied controlled Windows evidence; installed-package, real-user, and release acceptance were not included. See the [cache review](../docs/reviews/REVIEW_CACHE_LOCATION_ISSUES.md) and [output review](../docs/reviews/REVIEW_ISSUE_33_OUTPUT_ISSUES.md) for details.

The release badges and installation link below point to the upstream project's releases. Those binaries do not necessarily include this fork's changes.

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
