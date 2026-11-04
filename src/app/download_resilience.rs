use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DownloadErrorKind {
    TransientNetwork,
    RateLimited,
    FragmentFailure,
    AuthRequired,
    CookieInvalidOrExpired,
    FormatUnavailable,
    PostprocessThumbnailFailure,
    PostprocessMetadataFailure,
    ToolMissingOrBroken,
    FatalUnsupported,
    Cancelled,
    Unknown,
}

#[derive(Clone, Debug)]
pub(super) struct DownloadAttemptFailure {
    pub kind: DownloadErrorKind,
    pub message: String,
    pub recovered_output_path: Option<String>,
}

impl DownloadAttemptFailure {
    pub fn new(kind: DownloadErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            recovered_output_path: None,
        }
    }

    pub fn from_tool_setup_error(message: impl Into<String>) -> Self {
        let message = message.into();
        let kind = classify_download_error(&[], &[], &message);
        Self::new(kind, message)
    }

    pub fn from_attempt_output(
        stdout_lines: &[String],
        stderr_lines: &[String],
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let kind = classify_download_error(stdout_lines, stderr_lines, &message);
        Self::new(kind, message)
    }

    pub fn with_recovered_output_path(mut self, output_path: impl Into<String>) -> Self {
        self.recovered_output_path = Some(output_path.into());
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecoveryDecision {
    DoNotRecover,
    LogOnly {
        action: &'static str,
        detail: &'static str,
    },
    KeepMainOutput {
        action: &'static str,
        detail: &'static str,
    },
    RetryWithoutThumbnail {
        action: &'static str,
        detail: &'static str,
    },
    RetryWithFormatFallback {
        action: &'static str,
        detail: &'static str,
    },
    /// Wait, then repeat the same request (transient network errors and 429).
    RetryAfterBackoff {
        action: &'static str,
        attempt: usize,
        max_attempts: usize,
        delay: Duration,
    },
}

/// First backoff for transient network failures; doubles per attempt.
const NETWORK_RETRY_BASE_DELAY: Duration = Duration::from_secs(10);
/// Rate limits wait longer so retries stay website-friendly.
const RATE_LIMIT_RETRY_BASE_DELAY: Duration = Duration::from_secs(30);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(300);

pub(super) fn backoff_delay(kind: DownloadErrorKind, attempt: usize) -> Duration {
    let base = if kind == DownloadErrorKind::RateLimited {
        RATE_LIMIT_RETRY_BASE_DELAY
    } else {
        NETWORK_RETRY_BASE_DELAY
    };
    let exponent = attempt.saturating_sub(1).min(16) as u32;
    base.saturating_mul(1u32 << exponent).min(MAX_RETRY_DELAY)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DownloadAttemptContext {
    thumbnail_retry_used: bool,
    format_fallback_used: bool,
    network_retries_used: usize,
    max_network_retries: usize,
}

impl DownloadAttemptContext {
    pub fn with_network_retries(max_network_retries: usize) -> Self {
        Self {
            thumbnail_retry_used: false,
            format_fallback_used: false,
            network_retries_used: 0,
            max_network_retries,
        }
    }

    pub fn mark_network_retry(&mut self) {
        self.network_retries_used += 1;
    }

    pub fn mark_thumbnail_retry(&mut self) {
        self.thumbnail_retry_used = true;
    }

    pub fn mark_format_fallback(&mut self) {
        self.format_fallback_used = true;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct DownloadResiliencePolicy;

impl DownloadResiliencePolicy {
    pub fn decide(
        self,
        kind: DownloadErrorKind,
        context: &DownloadAttemptContext,
        request_has_thumbnail: bool,
        recovered_output_available: bool,
        format_fallback_available: bool,
    ) -> RecoveryDecision {
        match kind {
            DownloadErrorKind::PostprocessThumbnailFailure
                if request_has_thumbnail && !context.thumbnail_retry_used =>
            {
                RecoveryDecision::RetryWithoutThumbnail {
                    action: "retry without thumbnail",
                    detail: "thumbnail postprocess failed; retrying once without thumbnail embed",
                }
            }
            DownloadErrorKind::PostprocessMetadataFailure if recovered_output_available => {
                RecoveryDecision::KeepMainOutput {
                    action: "keep downloaded file",
                    detail: "metadata postprocess failed, but the main media file was kept",
                }
            }
            DownloadErrorKind::AuthRequired => RecoveryDecision::LogOnly {
                action: "cookie required",
                detail: "cookie rescue may be needed; automatic cookie flow was not started",
            },
            DownloadErrorKind::CookieInvalidOrExpired => RecoveryDecision::LogOnly {
                action: "cookie expired",
                detail: "cookie data may be invalid or expired; update cookies before retrying",
            },
            DownloadErrorKind::RateLimited
            | DownloadErrorKind::TransientNetwork
            | DownloadErrorKind::FragmentFailure
                if context.network_retries_used < context.max_network_retries =>
            {
                let attempt = context.network_retries_used + 1;
                RecoveryDecision::RetryAfterBackoff {
                    action: if kind == DownloadErrorKind::RateLimited {
                        "retry after rate limit"
                    } else {
                        "retry after network error"
                    },
                    attempt,
                    max_attempts: context.max_network_retries,
                    delay: backoff_delay(kind, attempt),
                }
            }
            DownloadErrorKind::RateLimited => RecoveryDecision::LogOnly {
                action: "rate limited",
                detail: "server rate limit detected; v2 avoids immediate retry to stay website-friendly",
            },
            DownloadErrorKind::TransientNetwork | DownloadErrorKind::FragmentFailure => {
                RecoveryDecision::LogOnly {
                    action: "yt-dlp retry exhausted",
                    detail: "yt-dlp already handled its own retry policy; v2 will not immediately retry the site",
                }
            }
            DownloadErrorKind::FormatUnavailable
                if format_fallback_available && !context.format_fallback_used =>
            {
                RecoveryDecision::RetryWithFormatFallback {
                    action: "retry with safe format selector",
                    detail: "selected format is unavailable; retrying once with a conservative yt-dlp fallback selector",
                }
            }
            DownloadErrorKind::FormatUnavailable => RecoveryDecision::LogOnly {
                action: "format unavailable",
                detail: "selected format is unavailable; automatic fallback was not applied again",
            },
            _ => RecoveryDecision::DoNotRecover,
        }
    }
}

pub(super) fn classify_download_error(
    stdout_lines: &[String],
    stderr_lines: &[String],
    fallback_message: &str,
) -> DownloadErrorKind {
    let normalized = combined_error_text(stdout_lines, stderr_lines, fallback_message);

    if contains_any(
        &normalized,
        &[
            "download cancelled",
            "cancelled",
            "canceled",
            "operation was canceled",
        ],
    ) {
        return DownloadErrorKind::Cancelled;
    }

    if contains_any(
        &normalized,
        &[
            "yt-dlp was not found",
            "ffmpeg was not found",
            "executable not found",
            "could not start yt-dlp",
            "permission denied",
            "access is denied",
        ],
    ) {
        return DownloadErrorKind::ToolMissingOrBroken;
    }

    if contains_any(
        &normalized,
        &[
            "cookies are no longer valid",
            "unable to extract cookies",
            "could not copy browser cookie database",
            "failed to decrypt",
            "cookie file is invalid",
            "invalid cookie",
        ],
    ) {
        return DownloadErrorKind::CookieInvalidOrExpired;
    }

    if contains_any(
        &normalized,
        &[
            "sign in to confirm",
            "login required",
            "private video",
            "members-only",
            "members only",
            "age-restricted",
            "confirm your age",
            "use --cookies",
            "cookies from browser",
        ],
    ) {
        return DownloadErrorKind::AuthRequired;
    }

    if contains_any(
        &normalized,
        &[
            "http error 429",
            "http 429",
            "too many requests",
            "rate limit",
            "rate-limit",
        ],
    ) {
        return DownloadErrorKind::RateLimited;
    }

    if contains_any(
        &normalized,
        &[
            "requested format is not available",
            "format is not available",
            "no video formats found",
            "no audio formats found",
            "no formats found",
            "unable to extract video data",
        ],
    ) {
        return DownloadErrorKind::FormatUnavailable;
    }

    if contains_all(&normalized, &["thumbnail", "postprocess"])
        || contains_all(&normalized, &["thumbnail", "ffmpeg"])
        || contains_all(&normalized, &["attached pic", "ffmpeg"])
        || contains_all(
            &normalized,
            &[
                "extracted extension",
                "unusual",
                "skipped for safety reasons",
            ],
        )
        || contains_any(
            &normalized,
            &[
                "unable to embed thumbnail",
                "failed to embed thumbnail",
                "error embedding thumbnail",
            ],
        )
    {
        return DownloadErrorKind::PostprocessThumbnailFailure;
    }

    if contains_all(&normalized, &["metadata", "postprocess"])
        || contains_any(
            &normalized,
            &[
                "unable to embed metadata",
                "failed to embed metadata",
                "error embedding metadata",
            ],
        )
    {
        return DownloadErrorKind::PostprocessMetadataFailure;
    }

    if contains_any(
        &normalized,
        &[
            "unable to download fragment",
            "fragment retries",
            "fragment retry",
            "http error 403 on fragment",
        ],
    ) || (normalized.contains("fragment") && normalized.contains("http error 403"))
    {
        return DownloadErrorKind::FragmentFailure;
    }

    if contains_any(
        &normalized,
        &[
            "http error 403",
            "http 403",
            "timed out",
            "timeout",
            "connection reset",
            "connection aborted",
            "temporarily unavailable",
            "unable to download video data",
            "got server http error",
            "remote end closed connection",
            "connection refused",
        ],
    ) {
        return DownloadErrorKind::TransientNetwork;
    }

    if contains_any(
        &normalized,
        &[
            "unsupported url",
            "no suitable extractor",
            "drm",
            "live event will begin",
            "video unavailable",
            "video has been removed",
            "this video is unavailable",
        ],
    ) {
        return DownloadErrorKind::FatalUnsupported;
    }

    DownloadErrorKind::Unknown
}

fn combined_error_text(
    stdout_lines: &[String],
    stderr_lines: &[String],
    fallback_message: &str,
) -> String {
    let mut text = String::new();
    for line in stdout_lines.iter().chain(stderr_lines.iter()) {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str(fallback_message);
    text.to_ascii_lowercase()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn contains_all(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| haystack.contains(needle))
}

#[cfg(test)]
mod backoff_tests {
    use super::*;

    fn decide(kind: DownloadErrorKind, context: &DownloadAttemptContext) -> RecoveryDecision {
        DownloadResiliencePolicy.decide(kind, context, false, false, false)
    }

    #[test]
    fn transient_errors_retry_until_the_limit() {
        let mut context = DownloadAttemptContext::with_network_retries(2);
        for expected_attempt in 1..=2 {
            match decide(DownloadErrorKind::TransientNetwork, &context) {
                RecoveryDecision::RetryAfterBackoff {
                    attempt,
                    max_attempts,
                    ..
                } => {
                    assert_eq!(attempt, expected_attempt);
                    assert_eq!(max_attempts, 2);
                }
                other => panic!("expected retry, got {other:?}"),
            }
            context.mark_network_retry();
        }
        assert!(matches!(
            decide(DownloadErrorKind::TransientNetwork, &context),
            RecoveryDecision::LogOnly { .. }
        ));
    }

    #[test]
    fn disabled_retries_keep_previous_policy() {
        let context = DownloadAttemptContext::with_network_retries(0);
        for kind in [
            DownloadErrorKind::RateLimited,
            DownloadErrorKind::TransientNetwork,
            DownloadErrorKind::FragmentFailure,
        ] {
            assert!(matches!(
                decide(kind, &context),
                RecoveryDecision::LogOnly { .. }
            ));
        }
        assert!(matches!(
            decide(
                DownloadErrorKind::AuthRequired,
                &DownloadAttemptContext::with_network_retries(3)
            ),
            RecoveryDecision::LogOnly { .. }
        ));
    }

    #[test]
    fn backoff_doubles_and_is_capped() {
        assert_eq!(
            backoff_delay(DownloadErrorKind::TransientNetwork, 1),
            Duration::from_secs(10)
        );
        assert_eq!(
            backoff_delay(DownloadErrorKind::TransientNetwork, 3),
            Duration::from_secs(40)
        );
        assert_eq!(
            backoff_delay(DownloadErrorKind::RateLimited, 1),
            Duration::from_secs(30)
        );
        assert_eq!(
            backoff_delay(DownloadErrorKind::RateLimited, 10),
            MAX_RETRY_DELAY
        );
    }
}
