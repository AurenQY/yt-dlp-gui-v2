#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DownloadContainerPreference {
    #[default]
    Auto,
    Mkv,
    Mp4,
    Webm,
    Mov,
    Flv,
}

impl DownloadContainerPreference {
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Mkv => Some("mkv"),
            Self::Mp4 => Some("mp4"),
            Self::Webm => Some("webm"),
            Self::Mov => Some("mov"),
            Self::Flv => Some("flv"),
        }
    }

    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase()
            .as_str()
        {
            "mkv" => Some(Self::Mkv),
            "mp4" => Some(Self::Mp4),
            "webm" => Some(Self::Webm),
            "mov" => Some(Self::Mov),
            "flv" => Some(Self::Flv),
            _ => None,
        }
    }
}

pub fn codecs_support_mp4_container(video_codec: &str, audio_codec: &str) -> bool {
    mp4_video_codec_is_supported(video_codec)
        && (audio_codec.is_empty() || mp4_audio_codec_is_supported(audio_codec))
}

pub fn codecs_support_webm_container(video_codec: &str, audio_codec: &str) -> bool {
    webm_video_codec_is_supported(video_codec)
        && (audio_codec.is_empty() || webm_audio_codec_is_supported(audio_codec))
}

/// One source keeps its container; only a separate video/audio pair needs merging.
pub fn default_download_container_extension(
    merge_container: Option<&str>,
    muxed: bool,
    video_ext: &str,
    audio_ext: &str,
) -> String {
    let normalize = |value: &str| value.trim().trim_start_matches('.').to_ascii_lowercase();
    if let Some(container) = merge_container.filter(|value| !value.trim().is_empty()) {
        return normalize(container);
    }
    let video = normalize(video_ext);
    let audio = normalize(audio_ext);
    if (muxed || audio.is_empty()) && !video.is_empty() {
        return video;
    }
    match (video.as_str(), audio.as_str()) {
        ("webm", "webm") => "webm",
        ("mp4", "m4a" | "aac") => "mp4",
        _ => "mkv",
    }
    .to_owned()
}

fn webm_video_codec_is_supported(codec: &str) -> bool {
    let codec = codec.trim().to_ascii_lowercase();
    codec.starts_with("vp8")
        || codec.starts_with("vp9")
        || codec.starts_with("vp09")
        || codec.starts_with("av1")
        || codec.starts_with("av01")
}

fn webm_audio_codec_is_supported(codec: &str) -> bool {
    let codec = codec.trim().to_ascii_lowercase();
    codec.starts_with("opus") || codec.starts_with("vorbis")
}

fn mp4_video_codec_is_supported(codec: &str) -> bool {
    let codec = codec.trim().to_ascii_lowercase();
    codec.starts_with("h264")
        || codec.starts_with("vp9")
        || codec.starts_with("vp09")
        || codec.starts_with("h.264")
        || codec.starts_with("avc")
        || codec.starts_with("hevc")
        || codec.starts_with("h265")
        || codec.starts_with("h.265")
        || codec.starts_with("av1")
        || codec.starts_with("av01")
}

fn mp4_audio_codec_is_supported(codec: &str) -> bool {
    let codec = codec.trim().to_ascii_lowercase();
    codec.starts_with("aac") || codec.starts_with("mp4a") || codec.starts_with("mp3")
}
