#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    mp4_video_codec_is_supported(video_codec) && mp4_audio_codec_is_supported(audio_codec)
}

pub fn codecs_support_webm_container(video_codec: &str, audio_codec: &str) -> bool {
    webm_video_codec_is_supported(video_codec) && webm_audio_codec_is_supported(audio_codec)
}

fn webm_video_codec_is_supported(codec: &str) -> bool {
    let codec = codec.trim().to_ascii_lowercase();
    codec.starts_with("vp8")
        || codec.starts_with("vp9")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webm_accepts_vp9_opus_and_rejects_h264_aac() {
        assert!(codecs_support_webm_container("VP9", "OPUS"));
        assert!(codecs_support_webm_container("AV1", "Vorbis"));
        assert!(!codecs_support_webm_container("H.264", "AAC"));
    }

    #[test]
    fn mp4_accepts_h264_aac_and_rejects_vp9_opus() {
        assert!(codecs_support_mp4_container("H.264", "AAC"));
        assert!(codecs_support_mp4_container("AV1", "mp4a.40.2"));
        assert!(!codecs_support_mp4_container("VP9", "OPUS"));
    }
}
