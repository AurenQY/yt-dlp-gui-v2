//! Saved output policy and a deliberately small, non-evaluating filename grammar.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputNamingMode {
    #[default]
    Title,
    TitleId,
    UploaderTitleId,
    Custom,
}

impl OutputNamingMode {
    pub const ALL: [Self; 4] = [
        Self::Title,
        Self::TitleId,
        Self::UploaderTitleId,
        Self::Custom,
    ];
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Title => "options.output.title",
            Self::TitleId => "options.output.title_id",
            Self::UploaderTitleId => "options.output.uploader_title_id",
            Self::Custom => "options.output.custom",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExistingFilePolicy {
    #[default]
    KeepBoth,
    Skip,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputDisposition {
    #[default]
    Published,
    SkippedExisting,
    Cancelled,
    Failed,
    PartialFailure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputPreferences {
    pub naming: OutputNamingMode,
    pub existing: ExistingFilePolicy,
    pub template: String,
}
impl Default for OutputPreferences {
    fn default() -> Self {
        Self {
            naming: OutputNamingMode::Title,
            existing: ExistingFilePolicy::KeepBoth,
            template: "{title}".into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct OutputNameContext {
    /// True only after source metadata was read; missing fields then use fallbacks.
    pub resolved: bool,
    pub title: String,
    pub id: String,
    pub uploader: String,
    pub upload_date: String,
    pub source: String,
}

/// Frozen at dispatch; retries and processing share this exact operation owner.
#[derive(Clone, Default)]
pub struct OutputRequest {
    pub status: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    pub preferences: OutputPreferences,
    pub context: OutputNameContext,
    pub automatic: bool,
    pub literal_name: String,
    pub transaction:
        Option<std::sync::Arc<std::sync::Mutex<super::output_transaction::OutputTransaction>>>,
    pub transcode: Option<super::TranscodeIntentSettings>,
    pub cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl OutputNameContext {
    pub(crate) fn needs_field(&self, key: &str) -> bool {
        !self.resolved
            && match key {
                "title" => self.title.trim().is_empty(),
                "id" => self.id.trim().is_empty(),
                "uploader" => self.uploader.trim().is_empty(),
                "upload_date" => self.upload_date.trim().is_empty(),
                _ => false,
            }
    }
    pub fn from_json(value: &serde_json::Value, source: &str) -> Self {
        let text = |key: &str| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        Self {
            resolved: true,
            title: text("title"),
            id: text("id"),
            uploader: text("uploader"),
            upload_date: text("upload_date"),
            source: source.into(),
        }
    }
}

impl OutputRequest {
    pub fn validate_work(&self, path: &std::path::Path) -> Result<bool, String> {
        let Some(owner) = &self.transaction else {
            return Ok(false);
        };
        let guard = owner
            .lock()
            .map_err(|_| "Output transaction is unavailable")?;
        if path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err("Output work contains parent traversal.".into());
        }
        if path.starts_with(&guard.work) {
            super::storage_paths::reject_links(path)?;
            Ok(true)
        } else {
            Err("Output work escaped its operation directory.".into())
        }
    }
}

impl OutputPreferences {
    fn naming_template(&self) -> &str {
        match self.naming {
            OutputNamingMode::Title => "{title}",
            OutputNamingMode::TitleId => "{title} [{id}]",
            OutputNamingMode::UploaderTitleId => "{uploader} - {title} [{id}]",
            OutputNamingMode::Custom => &self.template,
        }
    }
    pub(crate) fn needs_metadata(&self, context: &OutputNameContext) -> bool {
        let template = self.naming_template();
        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    continue;
                }
                let key: String = chars.by_ref().take_while(|c| *c != '}').collect();
                if context.needs_field(&key) {
                    return true;
                }
            } else if ch == '}' && chars.peek() == Some(&'}') {
                chars.next();
            }
        }
        false
    }
    pub fn validate(&self) -> Result<(), String> {
        render_template(&self.template, &OutputNameContext::default()).map(|_| ())
    }
    pub fn basename(&self, context: &OutputNameContext) -> Result<String, String> {
        let template = self.naming_template();
        render_template(template, context)
    }
}

pub fn safe_stem(text: &str) -> String {
    let mut cleaned: String = text
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    cleaned = cleaned.trim().trim_end_matches(['.', ' ']).to_owned();
    if cleaned.is_empty() || cleaned.chars().all(|c| c == '.') {
        cleaned = "Untitled".into();
    }
    let first = cleaned
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(first.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (first.len() == 4
            && (first.starts_with("COM") || first.starts_with("LPT"))
            && matches!(first.as_bytes()[3], b'1'..=b'9'))
    {
        cleaned.insert(0, '_');
    }
    // Leave room for suffixes, extensions and sidecar language identifiers.
    let mut units = 0;
    cleaned
        .chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= 180
        })
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_owned()
}

pub fn render_template(template: &str, context: &OutputNameContext) -> Result<String, String> {
    if template.trim().is_empty() {
        return Err("The filename template must not be empty.".into());
    }
    let fallback_id = format!(
        "source-{}",
        &super::sha256::sha256_hex_bytes(context.source.as_bytes())[..12]
    );
    let value = |key: &str| -> Result<String, String> {
        let (text, fallback) = match key {
            "title" => (&context.title, "Untitled"),
            "id" => (&context.id, fallback_id.as_str()),
            "uploader" => (&context.uploader, "Unknown uploader"),
            "upload_date" => (&context.upload_date, "Unknown date"),
            _ => return Err(format!("Unsupported filename field: {{{key}}}")),
        };
        Ok(safe_stem(if text.trim().is_empty() {
            fallback
        } else {
            text
        }))
    };
    let mut result = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                result.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                result.push('}');
            }
            '{' => {
                let mut key = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some('{') | None => {
                            return Err("Unbalanced filename template braces.".into());
                        }
                        Some(c) => key.push(c),
                    }
                }
                result.push_str(&value(&key)?);
            }
            '}' => return Err("Unbalanced filename template braces.".into()),
            c if c.is_control() || "<>:\"/\\|?*".contains(c) => {
                return Err(format!("Invalid literal filename character: {c:?}"));
            }
            c => result.push(c),
        }
    }
    if result.trim().is_empty() || result.trim().chars().all(|c| c == '.') {
        return Err("The filename template must produce a filename.".into());
    }
    Ok(safe_stem(&result))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trusted_partial_metadata_only_resolves_required_fields() {
        let mut context = OutputNameContext {
            title: "Cached title".into(),
            ..Default::default()
        };
        let mut preferences = OutputPreferences::default();
        assert!(!preferences.needs_metadata(&context));
        preferences.naming = OutputNamingMode::Custom;
        preferences.template = "{{id}} {title}".into();
        assert!(!preferences.needs_metadata(&context));
        preferences.template = "{id} {upload_date}".into();
        assert!(preferences.needs_metadata(&context));
        context.resolved = true;
        assert!(!preferences.needs_metadata(&context));
    }
    #[test]
    fn grammar_treats_metadata_as_data() {
        let context = OutputNameContext {
            title: "空间 100% / CON".into(),
            id: "abc".into(),
            ..Default::default()
        };
        assert_eq!(
            render_template("{{{title}}} [{id}]", &context).unwrap(),
            "{空间 100% _ CON} [abc]"
        );
        for text in [
            "",
            "..",
            "{title",
            "{title:03}",
            "../{title}",
            "C:{title}",
            "{title}\\x",
            "{ext}",
        ] {
            assert!(render_template(text, &context).is_err(), "{text}");
        }
        assert_eq!(safe_stem("NUL.txt"), "_NUL.txt");
        assert_eq!(safe_stem("abc. "), "abc");
        assert!(
            render_template("{id}", &OutputNameContext::default())
                .unwrap()
                .starts_with("source-")
        );
    }
}
