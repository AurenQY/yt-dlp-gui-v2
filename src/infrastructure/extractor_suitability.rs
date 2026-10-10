//! Validated portable suitable() predicates. Unknown never becomes a definite match.
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn validate_rules(
    rows: Vec<(usize, Value)>,
    count: usize,
) -> Result<HashMap<usize, Value>, String> {
    fn validate(p: &Value, count: usize, depth: usize) -> bool {
        let Some(p) = p.as_array() else { return false };
        if depth > 64 {
            return false;
        }
        match p.first().and_then(Value::as_u64) {
            Some(0) => p.len() == 1,
            Some(1) => p.len() == 2 && p[1].as_u64().is_some_and(|i| i < count as u64),
            Some(5) => p.len() == 2 && p[1].is_string(),
            Some(6) => p.len() == 2 && p[1].is_boolean(),
            Some(4) => p.len() == 2 && validate(&p[1], count, depth + 1),
            Some(2 | 3) => p.len() >= 3 && p[1..].iter().all(|v| validate(v, count, depth + 1)),
            _ => false,
        }
    }
    let mut result = HashMap::new();
    for (index, p) in rows {
        if index >= count || !validate(&p, count, 0) || result.insert(index, p).is_some() {
            return Err("Invalid/duplicate suitability rule".into());
        }
    }
    Ok(result)
}

pub(super) fn query_nonblank(url: &str, key: &str) -> bool {
    reqwest::Url::parse(url).ok().is_some_and(|url| {
        url.query_pairs()
            .any(|(k, value)| k == key && !value.is_empty())
    })
}

pub(super) fn evaluate(
    predicate: &Value,
    url: &str,
    regex: &impl Fn(usize) -> Result<bool, String>,
) -> Result<Option<bool>, String> {
    let p = predicate.as_array().ok_or("Invalid predicate")?;
    Ok(match p[0].as_u64().ok_or("Invalid predicate opcode")? {
        0 => None,
        1 => Some(regex(
            p[1].as_u64().ok_or("Invalid predicate index")? as usize
        )?),
        5 => Some(query_nonblank(
            url,
            p[1].as_str().ok_or("Invalid query predicate")?,
        )),
        6 => p[1].as_bool(),
        4 => evaluate(&p[1], url, regex)?.map(|value| !value),
        op @ (2 | 3) => {
            let mut unknown = false;
            for child in &p[1..] {
                match evaluate(child, url, regex)? {
                    Some(value) if value == (op == 3) => return Ok(Some(value)),
                    None => unknown = true,
                    _ => {}
                }
            }
            if unknown { None } else { Some(op == 2) }
        }
        _ => return Err("Invalid predicate opcode".into()),
    })
}
