#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppMode {
    #[default]
    Origin,
    Standard,
    Audio,
}

impl AppMode {
    pub const ALL: [Self; 3] = [Self::Origin, Self::Standard, Self::Audio];

    pub fn from_config_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "origin" => Self::Origin,
            "standard" => Self::Standard,
            "audio" => Self::Audio,
            _ => Self::Origin,
        }
    }

    pub fn config_value(self) -> &'static str {
        match self {
            Self::Origin => "origin",
            Self::Standard => "standard",
            Self::Audio => "audio",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Origin => "app_mode.origin",
            Self::Standard => "app_mode.standard",
            Self::Audio => "app_mode.audio",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueDisplayMode {
    Normal,
    Audio,
}

impl QueueDisplayMode {
    pub fn from_app_mode(mode: AppMode) -> Self {
        match mode {
            AppMode::Audio => Self::Audio,
            AppMode::Origin | AppMode::Standard => Self::Normal,
        }
    }

    pub fn config_value(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Audio => "audio",
        }
    }
}
