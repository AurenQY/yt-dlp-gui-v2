//! Shared replaceable URL routing dataset, with an embedded fallback.
//!
//! The same JSON is packaged with the Chromium extension.  `v` and
//! `versions` are intentionally ignored: the contract is the `y` and `g`
//! arrays of `[human_name, original_python_url_pattern]` pairs.
//! `yd` is the default Y source kind; `ys` is sparse [rule index, kind]
//! exceptions (0 unclassified, 1 single, 2 playlist, 3 any).
//! `yp` is portable suitable() IR; `yu` is conditional query-source evidence.
//! `yc` follows converted URL-only source branches after suitability selection.
//! Unsupported suitability blocks definite source inference. A match still
//! means "try this downloader", not "download will succeed".

use std::sync::OnceLock;

use pcre2::bytes::{Regex, RegexBuilder};
use serde::Deserialize;
#[path = "extractor_registry_metadata.rs"]
mod registry_metadata;
#[path = "extractor_source_branches.rs"]
mod source_branches;
#[path = "extractor_suitability.rs"]
mod suitability;
pub(crate) use registry_metadata::ExtractorMetadata;
#[path = "extractor_index_source.rs"]
mod index_source;

const EMBEDDED_INDEX: &str =
    include_str!("../../extension/chromium/resources/extractor-index.json");

#[derive(Debug, Deserialize)]
struct IndexDocument {
    y: Vec<(String, String)>,
    g: Vec<(String, String)>,
    #[serde(default)]
    ys: Vec<(usize, u8)>,
    #[serde(default)]
    yd: u8,
    #[serde(default)]
    yp: Vec<(usize, serde_json::Value)>,
    #[serde(default)]
    yu: Vec<(usize, String, u8)>,
    #[serde(default)]
    yc: Vec<(usize, serde_json::Value)>,
    #[serde(default)]
    gd: u8,
    #[serde(default)]
    gs: Vec<(usize, u8)>,
    #[serde(default)]
    meta: registry_metadata::RegistryDocument,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum YtSourceScope {
    Single,
    Playlist,
    Any,
    #[default]
    Unknown,
}

struct CompiledExtractor {
    name: String,
    pattern: Regex,
}

struct CompiledIndex {
    yt_dlp: Vec<CompiledExtractor>,
    gallery_dl: Vec<CompiledExtractor>,
    yt_scopes: Vec<YtSourceScope>,
    gallery_scopes: Vec<YtSourceScope>,
    yt_metadata: registry_metadata::RegistryMetadata,
    gallery_metadata: registry_metadata::RegistryMetadata,
    yt_predicates: std::collections::HashMap<usize, serde_json::Value>,
    yt_conditional: Vec<(usize, String, u8)>,
    yt_branches: std::collections::HashMap<usize, source_branches::SourceDecision>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatchedTools {
    pub(crate) yt_dlp: bool,
    pub(crate) gallery_dl: bool,
}

static COMPILED_INDEX: OnceLock<Result<CompiledIndex, String>> = OnceLock::new();

fn load_index() -> Result<CompiledIndex, String> {
    let content =
        index_source::load_from(&super::runtime_paths::application_root(), EMBEDDED_INDEX)?;
    compile_index(&content)
}

/// Worker-only source classification; unknown patterns/suitability need a bounded probe.
pub(crate) fn matching_yt_source_scope(url: &str) -> Result<YtSourceScope, String> {
    let compiled = COMPILED_INDEX
        .get_or_init(load_index)
        .as_ref()
        .map_err(Clone::clone)?;
    first_matching_yt_extractor(compiled, url)?
        .map(|(index, certain)| {
            if !certain {
                Ok(YtSourceScope::Unknown)
            } else if compiled
                .yt_conditional
                .iter()
                .any(|(i, key, _)| *i == index && suitability::query_nonblank(url, key))
            {
                Ok(YtSourceScope::Playlist)
            } else if let Some(decision) = compiled.yt_branches.get(&index) {
                Ok(match decision.evaluate(url)? {
                    1 => YtSourceScope::Single,
                    2 => YtSourceScope::Playlist,
                    _ => compiled.yt_scopes[index],
                })
            } else {
                Ok(compiled.yt_scopes[index])
            }
        })
        .unwrap_or(Ok(YtSourceScope::Unknown))
}

/// Called from an analysis worker, never the UI thread: compiling the initial
/// index can take time. An invalid index is reported instead of being treated
/// as a valid URL with zero matches. Forced analysis bypasses this function.
pub(crate) fn match_embedded_yt_dlp_url(url: &str) -> Result<bool, String> {
    let compiled = COMPILED_INDEX
        .get_or_init(load_index)
        .as_ref()
        .map_err(Clone::clone)?;
    Ok(first_matching_yt_extractor(compiled, url)?.is_some())
}

pub(crate) fn match_embedded_url(url: &str) -> Result<MatchedTools, String> {
    let compiled = COMPILED_INDEX
        .get_or_init(load_index)
        .as_ref()
        .map_err(Clone::clone)?;
    Ok(MatchedTools {
        yt_dlp: first_matching_yt_extractor(compiled, url)?.is_some(),
        gallery_dl: first_matching_extractor(&compiled.gallery_dl, url)?.is_some(),
    })
}

/// Root extractor identity for source-scope classification, only on worker threads.
pub(crate) fn matching_gallery_extractor(url: &str) -> Result<Option<String>, String> {
    let compiled = COMPILED_INDEX
        .get_or_init(load_index)
        .as_ref()
        .map_err(Clone::clone)?;
    Ok(first_matching_extractor(&compiled.gallery_dl, url)?
        .map(|index| compiled.gallery_dl[index].name.clone()))
}

/// Future consumers use stable identity/provenance without interpreting names.
/// Auto-routing only: explicit/forced choices are handled by the caller.
pub(crate) fn gallery_replaces_broken_yt(
    url: &str,
    matched: MatchedTools,
    gallery_available: bool,
) -> Result<bool, String> {
    if !matched.yt_dlp || !matched.gallery_dl || !gallery_available {
        return Ok(false);
    }
    Ok(matching_yt_extractor_metadata(url)?.is_some_and(|record| record.working == Some(false)))
}

pub(crate) fn matching_yt_extractor_metadata(
    url: &str,
) -> Result<Option<ExtractorMetadata>, String> {
    let compiled = COMPILED_INDEX
        .get_or_init(load_index)
        .as_ref()
        .map_err(Clone::clone)?;
    Ok(
        first_matching_yt_extractor(compiled, url)?.and_then(|(rule, certain)| {
            certain
                .then(|| compiled.yt_metadata.for_rule(rule).cloned())
                .flatten()
        }),
    )
}

#[allow(dead_code)]
pub(crate) fn matching_extractor_metadata(
    url: &str,
) -> Result<(Option<ExtractorMetadata>, Option<ExtractorMetadata>), String> {
    let compiled = COMPILED_INDEX
        .get_or_init(load_index)
        .as_ref()
        .map_err(Clone::clone)?;
    let y = first_matching_yt_extractor(compiled, url)?.and_then(|(rule, certain)| {
        certain
            .then(|| compiled.yt_metadata.for_rule(rule).cloned())
            .flatten()
    });
    let g = first_matching_extractor(&compiled.gallery_dl, url)?
        .and_then(|rule| compiled.gallery_metadata.for_rule(rule).cloned());
    Ok((y, g))
}

#[allow(dead_code)]
pub(crate) fn matching_gallery_source_scope(url: &str) -> Result<YtSourceScope, String> {
    let compiled = COMPILED_INDEX
        .get_or_init(load_index)
        .as_ref()
        .map_err(Clone::clone)?;
    Ok(first_matching_extractor(&compiled.gallery_dl, url)?
        .map(|rule| compiled.gallery_scopes[rule])
        .unwrap_or_default())
}

fn compile_index(json: &str) -> Result<CompiledIndex, String> {
    let index: IndexDocument = serde_json::from_str(json)
        .map_err(|error| format!("Extractor routing index has an invalid structure: {error}"))?;
    if index.y.is_empty() || index.g.is_empty() {
        return Err(
            "Embedded extractor index must contain non-empty y and g rule arrays.".to_owned(),
        );
    }
    fn scope_code(code: u8) -> Result<YtSourceScope, String> {
        match code {
            0 => Ok(YtSourceScope::Unknown),
            1 => Ok(YtSourceScope::Single),
            2 => Ok(YtSourceScope::Playlist),
            3 => Ok(YtSourceScope::Any),
            _ => Err("Invalid yt-dlp source classification kind".into()),
        }
    }
    let mut gallery_scopes = vec![scope_code(index.gd)?; index.g.len()];
    let mut gallery_positions = std::collections::HashSet::new();
    for (position, kind) in index.gs {
        let scope = gallery_scopes
            .get_mut(position)
            .ok_or("Invalid gallery source classification index")?;
        if !gallery_positions.insert(position) {
            return Err("Duplicate gallery source classification".into());
        }
        *scope = scope_code(kind)?;
    }
    let yt_metadata = registry_metadata::RegistryMetadata::compile(index.meta.y, index.y.len())?;
    let gallery_metadata =
        registry_metadata::RegistryMetadata::compile(index.meta.g, index.g.len())?;
    let mut yt_scopes = vec![scope_code(index.yd)?; index.y.len()];
    let mut scope_positions = std::collections::HashSet::new();
    for (position, kind) in index.ys {
        let scope = yt_scopes
            .get_mut(position)
            .ok_or("Invalid yt-dlp source classification index")?;
        if !scope_positions.insert(position) {
            return Err("Duplicate yt-dlp source classification".into());
        }
        *scope = scope_code(kind)?;
    }
    let yt_predicates = suitability::validate_rules(index.yp, index.y.len())?;
    let yt_branches = source_branches::compile_rules(index.yc, index.y.len())?;
    let mut conditional_positions = std::collections::HashSet::new();
    for (position, key, kind) in &index.yu {
        if *position >= index.y.len()
            || *kind != 2
            || !conditional_positions.insert((*position, key.clone()))
        {
            return Err("Invalid/duplicate conditional source scope".into());
        }
    }
    Ok(CompiledIndex {
        yt_dlp: compile_tool_rules("yt-dlp", index.y)?,
        gallery_dl: compile_tool_rules("gallery-dl", index.g)?,
        yt_scopes,
        gallery_scopes,
        yt_metadata,
        gallery_metadata,
        yt_predicates,
        yt_conditional: index.yu,
        yt_branches,
    })
}

fn compile_tool_rules(
    tool: &str,
    rules: Vec<(String, String)>,
) -> Result<Vec<CompiledExtractor>, String> {
    let mut compiled = Vec::with_capacity(rules.len());
    for (name, original_pattern) in rules {
        if name.trim().is_empty() || original_pattern.is_empty() {
            return Err(format!(
                "Embedded {tool} extractor index contains an empty name or pattern."
            ));
        }
        // Python's \uFFFF literal spelling has no PCRE2 equivalent. The
        // currently exported UnicodeBOM extractor uses \ufeff; preserve its
        // actual Unicode code point instead of dropping the whole rule.
        let pattern = original_pattern.replace(r"\ufeff", r"\x{FEFF}");
        let regex = RegexBuilder::new()
            .utf(true)
            .ucp(true)
            .build(&pattern)
            .map_err(|error| {
                format!("Embedded {tool} extractor '{name}' cannot compile: {error}")
            })?;
        compiled.push(CompiledExtractor {
            name,
            pattern: regex,
        });
    }
    Ok(compiled)
}

fn first_matching_yt_extractor(
    compiled: &CompiledIndex,
    url: &str,
) -> Result<Option<(usize, bool)>, String> {
    let regex = |index: usize| -> Result<bool, String> {
        let rule = compiled.yt_dlp.get(index).ok_or("Invalid rule reference")?;
        Ok(rule
            .pattern
            .find(url.as_bytes())
            .map_err(|e| e.to_string())?
            .is_some_and(|m| m.start() == 0))
    };
    for index in 0..compiled.yt_dlp.len() {
        if !regex(index)? {
            continue;
        }
        let suitable = match compiled.yt_predicates.get(&index) {
            Some(predicate) => suitability::evaluate(predicate, url, &regex)?,
            None => Some(true),
        };
        if suitable != Some(false) {
            return Ok(Some((index, suitable.is_some())));
        }
    }
    Ok(None)
}

fn first_matching_extractor(
    rules: &[CompiledExtractor],
    url: &str,
) -> Result<Option<usize>, String> {
    for (index, rule) in rules.iter().enumerate() {
        // PCRE2's search returns the leftmost match. A match beginning at
        // byte zero has the same anchoring behavior as Python re.match(),
        // without prepending an anchor (which could break leading inline flags).
        let matched = rule
            .pattern
            .find(url.as_bytes())
            .map_err(|error| format!("URL match failed for '{}': {error}", rule.name))?;
        if matched.is_some_and(|m| m.start() == 0) {
            return Ok(Some(index));
        }
    }
    Ok(None)
}
