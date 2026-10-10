//! Native validation and extension collection share the packaged PSL snapshot.
//! This is a website boundary, including its subdomains/paths, not one URL.
use std::{collections::HashSet, sync::OnceLock};

use super::youtube_login_rescue::host_from_http_url;

struct PublicSuffixRules {
    exact: HashSet<&'static str>,
    wildcard: HashSet<&'static str>,
    exception: HashSet<&'static str>,
}

fn rules() -> &'static PublicSuffixRules {
    static RULES: OnceLock<PublicSuffixRules> = OnceLock::new();
    RULES.get_or_init(|| {
        let mut rules = PublicSuffixRules {
            exact: HashSet::new(),
            wildcard: HashSet::new(),
            exception: HashSet::new(),
        };
        for line in
            include_str!("../../extension/chromium/resources/public-suffix-list.dat").lines()
        {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Some(rule) = line.strip_prefix('!') {
                rules.exception.insert(rule);
            } else if let Some(rule) = line.strip_prefix("*.") {
                rules.wildcard.insert(rule);
            } else {
                rules.exact.insert(line);
            }
        }
        rules
    })
}

pub(super) fn site_domain_for_url(url: &str) -> Result<String, String> {
    let host = host_from_http_url(url).ok_or("Browser cookie request URL is invalid.")?;
    if host.parse::<std::net::IpAddr>().is_ok() || !host.contains('.') {
        return Ok(host);
    }
    let labels = host.split('.').collect::<Vec<_>>();
    let rules = rules();
    let mut suffix_length = 1;
    let mut exception_length = 0;
    for index in 0..labels.len() {
        let suffix = labels[index..].join(".");
        let length = labels.len() - index;
        if rules.exception.contains(suffix.as_str()) {
            exception_length = exception_length.max(length);
        }
        if rules.exact.contains(suffix.as_str()) {
            suffix_length = suffix_length.max(length);
        }
        if index > 0 && rules.wildcard.contains(suffix.as_str()) {
            suffix_length = suffix_length.max(length + 1);
        }
    }
    if exception_length > 0 {
        suffix_length = exception_length - 1;
    }
    if labels.len() <= suffix_length {
        return Ok(host);
    }
    Ok(labels[labels.len() - suffix_length - 1..].join("."))
}

pub(super) fn domain_is_in_site(domain: &str, site: &str) -> bool {
    let domain = domain.trim_start_matches('.').to_ascii_lowercase();
    domain == site || domain.ends_with(&format!(".{site}"))
}
