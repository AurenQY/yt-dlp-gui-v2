//! URL-only source control flow emitted from upstream Python branches.
use pcre2::bytes::{Regex, RegexBuilder};
use serde_json::Value;
use std::collections::HashMap;

pub(super) enum SourceDecision {
    Scope(u8),
    Branch(SourceGuard, Box<Self>, Box<Self>),
}

pub(super) enum SourceGuard {
    Regex(Regex),
    Query(String, Option<Option<String>>),
    Capture(Vec<Regex>, String, Option<Option<String>>),
}

fn compile_regex(pattern: &str) -> Result<Regex, String> {
    RegexBuilder::new()
        .utf(true)
        .ucp(true)
        .build(&pattern.replace(r"\ufeff", r"\x{FEFF}"))
        .map_err(|error| error.to_string())
}

fn compile_guard(value: &Value) -> Result<SourceGuard, String> {
    if let Some(pattern) = value.as_str() {
        return Ok(SourceGuard::Regex(compile_regex(pattern)?));
    }
    let row = value.as_array().ok_or("Invalid source value guard")?;
    fn expectation(row: &[Value], offset: usize) -> Result<Option<Option<String>>, String> {
        if row.len() == offset {
            return Ok(None);
        }
        if row.len() != offset + 1 {
            return Err("Invalid source guard expectation".into());
        }
        match &row[offset] {
            Value::Null => Ok(Some(None)),
            Value::String(s) => Ok(Some(Some(s.clone()))),
            _ => Err("Invalid source guard equality".into()),
        }
    }
    match row.first().and_then(Value::as_str) {
        Some("q") if row.len() >= 2 => Ok(SourceGuard::Query(
            row[1]
                .as_str()
                .ok_or("Invalid source query key")?
                .to_owned(),
            expectation(row, 2)?,
        )),
        Some("g") if row.len() >= 3 => {
            let patterns = match &row[1] {
                Value::String(pattern) => vec![pattern.as_str()],
                Value::Array(patterns) if !patterns.is_empty() => patterns
                    .iter()
                    .map(|pattern| pattern.as_str().ok_or("Invalid source capture regex"))
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err("Invalid source capture regex".into()),
            };
            let regexes = patterns
                .into_iter()
                .map(compile_regex)
                .collect::<Result<Vec<_>, _>>()?;
            let name = row[2]
                .as_str()
                .ok_or("Invalid source capture name")?
                .to_owned();
            if regexes.iter().any(|regex| {
                !regex
                    .capture_names()
                    .iter()
                    .any(|n| n.as_deref() == Some(&name))
            }) {
                return Err("Missing source capture name".into());
            }
            Ok(SourceGuard::Capture(regexes, name, expectation(row, 3)?))
        }
        _ => Err("Invalid source guard kind".into()),
    }
}

impl SourceGuard {
    fn evaluate(&self, url: &str) -> Result<bool, String> {
        fn compare(value: Option<String>, expected: &Option<Option<String>>) -> bool {
            match expected {
                Some(expected) => value == *expected,
                None => value.is_some_and(|v| !v.is_empty()),
            }
        }
        match self {
            Self::Regex(regex) => Ok(regex
                .find(url.as_bytes())
                .map_err(|error| error.to_string())?
                .is_some_and(|found| found.start() == 0)),
            Self::Query(key, expected) => {
                let value = reqwest::Url::parse(url).ok().and_then(|url| {
                    url.query_pairs()
                        .find(|(k, value)| k == key && !value.is_empty())
                        .map(|(_, value)| value.into_owned())
                });
                Ok(compare(value, expected))
            }
            Self::Capture(regexes, name, expected) => {
                for regex in regexes {
                    if let Some(captures) = regex
                        .captures(url.as_bytes())
                        .map_err(|error| error.to_string())?
                    {
                        if captures.get(0).is_some_and(|matched| matched.start() == 0) {
                            let value = captures.name(name).map(|value| {
                                String::from_utf8_lossy(value.as_bytes()).into_owned()
                            });
                            return Ok(compare(value, expected));
                        }
                    }
                }
                Ok(compare(None, expected))
            }
        }
    }
}

pub(super) fn compile_rules(
    rows: Vec<(usize, Value)>,
    count: usize,
) -> Result<HashMap<usize, SourceDecision>, String> {
    fn compile(value: &Value, depth: usize) -> Result<SourceDecision, String> {
        if depth > 32 {
            return Err("Source branch depth exceeded".into());
        }
        if let Some(code) = value.as_u64().filter(|code| *code <= 2) {
            return Ok(SourceDecision::Scope(code as u8));
        }
        let row = value
            .as_array()
            .filter(|row| row.len() == 4 && row[0].as_u64() == Some(3))
            .ok_or("Invalid source branch")?;
        Ok(SourceDecision::Branch(
            compile_guard(&row[1])?,
            Box::new(compile(&row[2], depth + 1)?),
            Box::new(compile(&row[3], depth + 1)?),
        ))
    }
    let mut result = HashMap::new();
    for (index, value) in rows {
        if index >= count || result.insert(index, compile(&value, 0)?).is_some() {
            return Err("Invalid/duplicate source branch index".into());
        }
    }
    Ok(result)
}

impl SourceDecision {
    pub(super) fn evaluate(&self, url: &str) -> Result<u8, String> {
        match self {
            Self::Scope(code) => Ok(*code),
            Self::Branch(guard, yes, no) => {
                let matched = guard.evaluate(url)?;
                if matched {
                    yes.evaluate(url)
                } else {
                    no.evaluate(url)
                }
            }
        }
    }
}
