//! Option/value boundaries for the supported flat yt-dlp configuration.
//! Names and arities follow the retained yt-dlp 2026.10.21 option definitions.
//! This does not execute options or replace yt-dlp's own configuration parser.

#[derive(Clone, Debug)]
pub(super) struct ConfigOption {
    pub(super) name: String,
    pub(super) spelling: String,
    pub(super) values: Vec<String>,
}

fn specification(spelling: &str) -> Option<(&'static str, usize)> {
    let exact = OPTIONS.iter().find(|(names, _)| names.contains(&spelling));
    let selected = if let Some(exact) = exact {
        exact
    } else if spelling.starts_with("--") {
        // optparse accepts an unambiguous long-option abbreviation. Count
        // aliases separately, including zero-argument flags, as yt-dlp does.
        let mut candidates = OPTIONS.iter().flat_map(|spec| {
            spec.0
                .iter()
                .filter(move |name| name.starts_with(spelling))
                .map(move |_| spec)
        });
        let selected = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        selected
    } else {
        return None;
    };
    Some((
        selected
            .0
            .iter()
            .copied()
            .find(|name| name.starts_with("--"))
            .unwrap_or(selected.0[0]),
        selected.1,
    ))
}

pub(super) fn interpret(arguments: &[String]) -> Result<Vec<ConfigOption>, String> {
    let mut result = Vec::new();
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        if word == "--" {
            return Err("Unsupported yt-dlp config terminator: --. Remove it so application cache, output and analysis options remain effective.".into());
        }
        if !word.starts_with('-') || word == "-" {
            continue;
        }
        let mut short = (!word.starts_with("--")).then(|| word[1..].char_indices().peekable());
        loop {
            let (spelling, attached) = if let Some(short) = short.as_mut() {
                let Some((offset, letter)) = short.next() else {
                    break;
                };
                let spelling = format!("-{letter}");
                let count = specification(&spelling).map_or(0, |(_, count)| count);
                let end = 1 + offset + letter.len_utf8();
                let attached = (count > 0 && end < word.len()).then(|| word[end..].to_owned());
                (spelling, attached)
            } else {
                let (spelling, value) = word
                    .split_once('=')
                    .map_or((word.as_str(), None), |(name, value)| {
                        (name, Some(value.to_owned()))
                    });
                (spelling.to_owned(), value)
            };
            let (name, count) = specification(&spelling).map_or((spelling.as_str(), 0), |v| v);
            let mut option = ConfigOption {
                name: name.to_owned(),
                spelling,
                values: attached.into_iter().collect(),
            };
            while option.values.len() < count {
                option.values.push(
                    words
                        .next()
                        .ok_or_else(|| {
                            format!(
                                "{} requires a value in yt-dlp configuration",
                                option.spelling
                            )
                        })?
                        .clone(),
                );
            }
            result.push(option);
            if short.is_none() || count > 0 {
                break;
            }
        }
    }
    Ok(result)
}

// Extracted option names/arities only; executable option behavior remains in yt-dlp.
const OPTIONS: &[(&[&str], usize)] = &[
    (&["--xattr-set-filesize"], 0),
    (&["--dump-user-agent"], 0),
    (&["--youtube-include-dash-manifest"], 0),
    (&["--no-youtube-skip-dash-manifest"], 0),
    (&["--youtube-skip-dash-manifest"], 0),
    (&["--no-youtube-include-dash-manifest"], 0),
    (&["--youtube-include-hls-manifest"], 0),
    (&["--no-youtube-skip-hls-manifest"], 0),
    (&["--youtube-skip-hls-manifest"], 0),
    (&["--no-youtube-include-hls-manifest"], 0),
    (&["--youtube-print-sig-code"], 0),
    (&["--sponskrub"], 0),
    (&["--no-sponskrub"], 0),
    (&["--sponskrub-cut"], 0),
    (&["--no-sponskrub-cut"], 0),
    (&["--sponskrub-force"], 0),
    (&["--no-sponskrub-force"], 0),
    (&["--prefer-avconv"], 0),
    (&["--no-prefer-ffmpeg"], 0),
    (&["--prefer-ffmpeg"], 0),
    (&["--no-prefer-avconv"], 0),
    (&["-C"], 0),
    (&["--call-home"], 0),
    (&["--no-call-home"], 0),
    (&["--include-ads"], 0),
    (&["--no-include-ads"], 0),
    (&["--write-annotations"], 0),
    (&["--no-write-annotations"], 0),
    (&["--sponskrub-location"], 1),
    (&["--sponskrub-args"], 1),
    (&["--cn-verification-proxy"], 1),
    (&["-h", "--help"], 0),
    (&["--version"], 0),
    (&["-U", "--update"], 0),
    (&["--no-update"], 0),
    (&["--update-to"], 1),
    (&["-i", "--ignore-errors"], 0),
    (&["--no-abort-on-error"], 0),
    (&["--abort-on-error", "--no-ignore-errors"], 0),
    (&["--list-extractors"], 0),
    (&["--extractor-descriptions"], 0),
    (&["--use-extractors", "--ies"], 1),
    (&["--force-generic-extractor"], 0),
    (&["--default-search"], 1),
    (&["--ignore-config", "--no-config"], 0),
    (&["--no-config-locations"], 0),
    (&["--config-locations"], 1),
    (&["--plugin-dirs"], 1),
    (&["--no-plugin-dirs"], 0),
    (&["--js-runtimes"], 1),
    (&["--no-js-runtimes"], 0),
    (&["--remote-components"], 1),
    (&["--no-remote-components"], 0),
    (&["--flat-playlist"], 0),
    (&["--no-flat-playlist"], 0),
    (&["--live-from-start"], 0),
    (&["--no-live-from-start"], 0),
    (&["--wait-for-video"], 1),
    (&["--no-wait-for-video"], 0),
    (&["--mark-watched"], 0),
    (&["--no-mark-watched"], 0),
    (&["--no-colors", "--no-colours"], 0),
    (&["--color"], 1),
    (&["--compat-options"], 1),
    (&["--alias"], 2),
    (&["-t", "--preset-alias"], 1),
    (&["--proxy"], 1),
    (&["--socket-timeout"], 1),
    (&["--source-address"], 1),
    (&["--impersonate"], 1),
    (&["--list-impersonate-targets"], 0),
    (&["-4", "--force-ipv4"], 0),
    (&["-6", "--force-ipv6"], 0),
    (&["--enable-file-urls"], 0),
    (&["--geo-verification-proxy"], 1),
    (&["--xff"], 1),
    (&["--geo-bypass"], 0),
    (&["--no-geo-bypass"], 0),
    (&["--geo-bypass-country"], 1),
    (&["--geo-bypass-ip-block"], 1),
    (&["--playlist-start"], 1),
    (&["--playlist-end"], 1),
    (&["-I", "--playlist-items"], 1),
    (&["--match-title"], 1),
    (&["--reject-title"], 1),
    (&["--min-filesize"], 1),
    (&["--max-filesize"], 1),
    (&["--date"], 1),
    (&["--datebefore"], 1),
    (&["--dateafter"], 1),
    (&["--min-views"], 1),
    (&["--max-views"], 1),
    (&["--match-filters"], 1),
    (&["--no-match-filters"], 0),
    (&["--break-match-filters"], 1),
    (&["--no-break-match-filters"], 0),
    (&["--no-playlist"], 0),
    (&["--yes-playlist"], 0),
    (&["--age-limit"], 1),
    (&["--download-archive"], 1),
    (&["--no-download-archive"], 0),
    (&["--max-downloads"], 1),
    (&["--break-on-existing"], 0),
    (&["--no-break-on-existing"], 0),
    (&["--break-on-reject"], 0),
    (&["--break-per-input"], 0),
    (&["--no-break-per-input"], 0),
    (&["--skip-playlist-after-errors"], 1),
    (&["-N", "--concurrent-fragments"], 1),
    (&["-r", "--limit-rate", "--rate-limit"], 1),
    (&["--throttled-rate"], 1),
    (&["-R", "--retries"], 1),
    (&["--file-access-retries"], 1),
    (&["--fragment-retries"], 1),
    (&["--retry-sleep"], 1),
    (
        &[
            "--skip-unavailable-fragments",
            "--no-abort-on-unavailable-fragments",
        ],
        0,
    ),
    (
        &[
            "--abort-on-unavailable-fragments",
            "--no-skip-unavailable-fragments",
        ],
        0,
    ),
    (&["--keep-fragments"], 0),
    (&["--no-keep-fragments"], 0),
    (&["--buffer-size"], 1),
    (&["--resize-buffer"], 0),
    (&["--no-resize-buffer"], 0),
    (&["--http-chunk-size"], 1),
    (&["--test"], 0),
    (&["--playlist-reverse"], 0),
    (&["--no-playlist-reverse"], 0),
    (&["--playlist-random"], 0),
    (&["--lazy-playlist"], 0),
    (&["--no-lazy-playlist"], 0),
    (&["--hls-prefer-native"], 0),
    (&["--hls-prefer-ffmpeg"], 0),
    (&["--hls-use-mpegts"], 0),
    (&["--no-hls-use-mpegts"], 0),
    (&["--download-sections"], 1),
    (&["--downloader", "--external-downloader"], 1),
    (&["--downloader-args", "--external-downloader-args"], 1),
    (&["-a", "--batch-file"], 1),
    (&["--no-batch-file"], 0),
    (&["--id"], 0),
    (&["-P", "--paths"], 1),
    (&["-o", "--output"], 1),
    (&["--output-na-placeholder"], 1),
    (&["--autonumber-size"], 1),
    (&["--autonumber-start"], 1),
    (&["--restrict-filenames"], 0),
    (&["--no-restrict-filenames"], 0),
    (&["--windows-filenames"], 0),
    (&["--no-windows-filenames"], 0),
    (&["--trim-filenames", "--trim-file-names"], 1),
    (&["-w", "--no-overwrites"], 0),
    (&["--force-overwrites", "--yes-overwrites"], 0),
    (&["--no-force-overwrites"], 0),
    (&["-c", "--continue"], 0),
    (&["--no-continue"], 0),
    (&["--part"], 0),
    (&["--no-part"], 0),
    (&["--mtime"], 0),
    (&["--no-mtime"], 0),
    (&["--write-description"], 0),
    (&["--no-write-description"], 0),
    (&["--write-info-json"], 0),
    (&["--no-write-info-json"], 0),
    (&["--write-playlist-metafiles"], 0),
    (&["--no-write-playlist-metafiles"], 0),
    (&["--clean-info-json", "--clean-infojson"], 0),
    (&["--no-clean-info-json", "--no-clean-infojson"], 0),
    (&["--write-comments", "--get-comments"], 0),
    (&["--no-write-comments", "--no-get-comments"], 0),
    (&["--load-info-json"], 1),
    (&["--cookies"], 1),
    (&["--no-cookies"], 0),
    (&["--cookies-from-browser"], 1),
    (&["--no-cookies-from-browser"], 0),
    (&["--cache-dir"], 1),
    (&["--no-cache-dir"], 0),
    (&["--rm-cache-dir"], 0),
    (&["--write-thumbnail"], 0),
    (&["--no-write-thumbnail"], 0),
    (&["--write-all-thumbnails"], 0),
    (&["--list-thumbnails"], 0),
    (&["--write-link"], 0),
    (&["--write-url-link"], 0),
    (&["--write-webloc-link"], 0),
    (&["--write-desktop-link"], 0),
    (&["-q", "--quiet"], 0),
    (&["--no-quiet"], 0),
    (&["--no-warnings"], 0),
    (&["-s", "--simulate"], 0),
    (&["--no-simulate"], 0),
    (&["--ignore-no-formats-error"], 0),
    (&["--no-ignore-no-formats-error"], 0),
    (&["--skip-download", "--no-download"], 0),
    (&["-O", "--print"], 1),
    (&["--print-to-file"], 2),
    (&["-g", "--get-url"], 0),
    (&["-e", "--get-title"], 0),
    (&["--get-id"], 0),
    (&["--get-thumbnail"], 0),
    (&["--get-description"], 0),
    (&["--get-duration"], 0),
    (&["--get-filename"], 0),
    (&["--get-format"], 0),
    (&["-j", "--dump-json"], 0),
    (&["-J", "--dump-single-json"], 0),
    (&["--print-json"], 0),
    (
        &[
            "--force-write-archive",
            "--force-write-download-archive",
            "--force-download-archive",
        ],
        0,
    ),
    (&["--newline"], 0),
    (&["--no-progress"], 0),
    (&["--progress"], 0),
    (&["--console-title"], 0),
    (&["--progress-template"], 1),
    (&["--progress-delta"], 1),
    (&["-v", "--verbose"], 0),
    (&["--dump-pages"], 0),
    (&["--write-pages"], 0),
    (&["--load-pages"], 0),
    (&["--print-traffic"], 0),
    (&["--encoding"], 1),
    (&["--legacy-server-connect"], 0),
    (&["--no-check-certificates"], 0),
    (&["--prefer-insecure", "--prefer-unsecure"], 0),
    (&["--user-agent"], 1),
    (&["--referer"], 1),
    (&["--add-headers"], 1),
    (&["--bidi-workaround"], 0),
    (&["--sleep-requests"], 1),
    (&["--sleep-interval", "--min-sleep-interval"], 1),
    (&["--max-sleep-interval"], 1),
    (&["--sleep-subtitles"], 1),
    (&["-f", "--format"], 1),
    (&["-S", "--format-sort"], 1),
    (&["--format-sort-reset"], 0),
    (&["--format-sort-force", "--S-force"], 0),
    (&["--no-format-sort-force"], 0),
    (&["--video-multistreams"], 0),
    (&["--no-video-multistreams"], 0),
    (&["--audio-multistreams"], 0),
    (&["--no-audio-multistreams"], 0),
    (&["--all-formats"], 0),
    (&["--prefer-free-formats"], 0),
    (&["--no-prefer-free-formats"], 0),
    (&["--check-formats"], 0),
    (&["--check-all-formats"], 0),
    (&["--no-check-formats"], 0),
    (&["-F", "--list-formats"], 0),
    (&["--list-formats-as-table"], 0),
    (&["--list-formats-old", "--no-list-formats-as-table"], 0),
    (&["--merge-output-format"], 1),
    (&["--allow-unplayable-formats"], 0),
    (&["--no-allow-unplayable-formats"], 0),
    (&["--write-subs", "--write-srt"], 0),
    (&["--no-write-subs", "--no-write-srt"], 0),
    (&["--write-auto-subs", "--write-automatic-subs"], 0),
    (&["--no-write-auto-subs", "--no-write-automatic-subs"], 0),
    (&["--all-subs"], 0),
    (&["--list-subs"], 0),
    (&["--sub-format"], 1),
    (&["--sub-langs", "--srt-langs"], 1),
    (&["-u", "--username"], 1),
    (&["-p", "--password"], 1),
    (&["-2", "--twofactor"], 1),
    (&["-n", "--netrc"], 0),
    (&["--netrc-location"], 1),
    (&["--netrc-cmd"], 1),
    (&["--video-password"], 1),
    (&["--ap-mso"], 1),
    (&["--ap-username"], 1),
    (&["--ap-password"], 1),
    (&["--ap-list-mso"], 0),
    (&["--client-certificate"], 1),
    (&["--client-certificate-key"], 1),
    (&["--client-certificate-password"], 1),
    (&["-x", "--extract-audio"], 0),
    (&["--audio-format"], 1),
    (&["--audio-quality"], 1),
    (&["--remux-video"], 1),
    (&["--recode-video"], 1),
    (&["--postprocessor-args", "--ppa"], 1),
    (&["-k", "--keep-video"], 0),
    (&["--no-keep-video"], 0),
    (&["--post-overwrites"], 0),
    (&["--no-post-overwrites"], 0),
    (&["--embed-subs"], 0),
    (&["--no-embed-subs"], 0),
    (&["--embed-thumbnail"], 0),
    (&["--no-embed-thumbnail"], 0),
    (&["--embed-metadata", "--add-metadata"], 0),
    (&["--no-embed-metadata", "--no-add-metadata"], 0),
    (&["--embed-chapters", "--add-chapters"], 0),
    (&["--no-embed-chapters", "--no-add-chapters"], 0),
    (&["--embed-info-json"], 0),
    (&["--no-embed-info-json"], 0),
    (&["--metadata-from-title"], 1),
    (&["--parse-metadata"], 1),
    (&["--replace-in-metadata"], 3),
    (&["--xattrs", "--xattr"], 0),
    (&["--concat-playlist"], 1),
    (&["--fixup"], 1),
    (&["--ffmpeg-location"], 1),
    (&["--exec"], 1),
    (&["--no-exec"], 0),
    (&["--exec-before-download"], 1),
    (&["--no-exec-before-download"], 0),
    (
        &["--convert-subs", "--convert-sub", "--convert-subtitles"],
        1,
    ),
    (&["--convert-thumbnails"], 1),
    (&["--split-chapters", "--split-tracks"], 0),
    (&["--no-split-chapters", "--no-split-tracks"], 0),
    (&["--remove-chapters"], 1),
    (&["--no-remove-chapters"], 0),
    (&["--force-keyframes-at-cuts"], 0),
    (&["--no-force-keyframes-at-cuts"], 0),
    (&["--use-postprocessor"], 1),
    (&["--sponsorblock-mark"], 1),
    (&["--sponsorblock-remove"], 1),
    (&["--sponsorblock-chapter-title"], 1),
    (&["--no-sponsorblock"], 0),
    (&["--sponsorblock-api"], 1),
    (&["--extractor-retries"], 1),
    (&["--allow-dynamic-mpd", "--no-ignore-dynamic-mpd"], 0),
    (&["--ignore-dynamic-mpd", "--no-allow-dynamic-mpd"], 0),
    (&["--hls-split-discontinuity"], 0),
    (&["--no-hls-split-discontinuity"], 0),
    (&["--extractor-args"], 1),
];
