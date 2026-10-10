//! Typed, extensible upstream metadata indexed by stable class key and rule.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractorMetadata {
    pub key: String,
    pub class: String,
    pub module: String,
    pub order: usize,
    pub rules: Vec<usize>,
    pub scope: u8,
    pub declared_scope: u8,
    /// Upstream declaration, not a live website health check. Absent means unknown.
    #[serde(default)]
    pub working: Option<bool>,
    #[serde(flatten)]
    pub details: BTreeMap<String, Value>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RegistryDocument {
    #[serde(default)]
    pub y: Vec<ExtractorMetadata>,
    #[serde(default)]
    pub g: Vec<ExtractorMetadata>,
}

pub(super) struct RegistryMetadata {
    pub records: Vec<ExtractorMetadata>,
    by_rule: HashMap<usize, usize>,
}

impl RegistryMetadata {
    pub fn compile(records: Vec<ExtractorMetadata>, rule_count: usize) -> Result<Self, String> {
        let mut by_rule = HashMap::new();
        let mut keys = HashSet::new();
        for (record_index, record) in records.iter().enumerate() {
            if record.key.is_empty()
                || !keys.insert(record.key.clone())
                || record.scope > 3
                || record.declared_scope > 3
            {
                return Err("Invalid extractor registry metadata".into());
            }
            for rule in &record.rules {
                if *rule >= rule_count || by_rule.insert(*rule, record_index).is_some() {
                    return Err("Invalid extractor metadata rule".into());
                }
            }
        }
        if !records.is_empty() && by_rule.len() != rule_count {
            return Err("Incomplete extractor registry metadata".into());
        }
        Ok(Self { records, by_rule })
    }

    pub fn for_rule(&self, rule: usize) -> Option<&ExtractorMetadata> {
        self.by_rule.get(&rule).map(|index| &self.records[*index])
    }
}
