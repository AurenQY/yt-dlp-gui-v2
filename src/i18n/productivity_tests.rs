//! Scoped checks for the queue productivity UI keys; other zones are untouched.
const PREFIXES: [&str; 5] = [
    "\"queue.",
    "\"history.",
    "\"subscriptions.",
    "\"site_rules.",
    "\"item.downloaded_before",
];
const SCOPED_KEYS: [&str; 15] = [
    "advance.parallel_downloads",
    "advance.auto_retry",
    "advance.auto_retry_off",
    "advance.auto_retry_times",
    "advance.scheduled_rate",
    "advance.scheduled_rate_active",
    "advance.sponsorblock",
    "advance.metadata",
    "advance.embed_metadata",
    "options.playlist_filter",
    "options.library",
    "options.history",
    "options.record_history",
    "options.subscriptions",
    "options.site_rules",
];

fn scoped_keys(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('"') || !trimmed.contains("=>") {
                return None;
            }
            let key = &trimmed[1..trimmed[1..].find('"')? + 1];
            let quoted = format!("\"{key}");
            let scoped = PREFIXES.iter().any(|prefix| quoted.starts_with(prefix))
                || key.starts_with("advance.sponsorblock")
                || key.starts_with("options.history")
                || key.starts_with("options.subscriptions")
                || key.starts_with("options.site_rules")
                || key.starts_with("options.playlist_filter")
                || SCOPED_KEYS.contains(&key);
            scoped.then_some(key)
        })
        .collect()
}

/// The value of `"key" => "value",` or the rustfmt-wrapped `"key" => { "value" }`.
fn value_for<'a>(source: &'a str, key: &str) -> &'a str {
    let needle = format!("\"{key}\" =>");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("missing {key}"))
        + needle.len();
    let rest = &source[start..];
    let open = rest.find('"').unwrap() + 1;
    let rest = &rest[open..];
    &rest[..rest.find('"').unwrap()]
}

fn placeholders(value: &str) -> Vec<&str> {
    let mut found = value
        .match_indices('{')
        .filter_map(|(start, _)| {
            value[start..]
                .find('}')
                .map(|end| &value[start..start + end + 1])
        })
        .collect::<Vec<_>>();
    found.sort_unstable();
    found
}

const LOCALES: [(&str, &str); 14] = [
    ("ar-MA", include_str!("ar_ma.rs")),
    ("de-DE", include_str!("de_de.rs")),
    ("el-GR", include_str!("el_gr.rs")),
    ("es-ES", include_str!("es_es.rs")),
    ("fr-FR", include_str!("fr_fr.rs")),
    ("it-IT", include_str!("it_it.rs")),
    ("ja-JP", include_str!("ja_jp.rs")),
    ("ko-KR", include_str!("ko_kr.rs")),
    ("pl-PL", include_str!("pl_pl.rs")),
    ("pt-BR", include_str!("pt_br.rs")),
    ("ru-RU", include_str!("ru_ru.rs")),
    ("uk-UA", include_str!("uk_ua.rs")),
    ("zh-CN", include_str!("zh_cn.rs")),
    ("zh-TW", include_str!("zh_tw.rs")),
];

#[test]
fn i18n_keys_productivity() {
    let english = include_str!("en_us.rs");
    let canonical = scoped_keys(english);
    assert!(canonical.len() >= 50, "{canonical:?}");
    for (name, source) in LOCALES {
        assert_eq!(
            scoped_keys(source),
            canonical,
            "Productivity key coverage/order: {name}"
        );
        for key in &canonical {
            assert_eq!(
                placeholders(value_for(source, key)),
                placeholders(value_for(english, key)),
                "Placeholder mismatch for {key} in {name}"
            );
        }
    }
}

#[test]
fn i18n_used_keys_productivity() {
    let english = include_str!("en_us.rs");
    for source in [
        include_str!("../app/ui/queue_tools.rs"),
        include_str!("../app/ui/options_library.rs"),
        include_str!("../app/ui/advance_download_controls.rs"),
        include_str!("../app/ui/advance_post_processing.rs"),
        include_str!("../app/ui/batch_list_template.rs"),
        include_str!("../app/state/productivity_actions.rs"),
        include_str!("../infrastructure/config.rs"),
    ] {
        for (start, _) in source.match_indices('"') {
            let rest = &source[start + 1..];
            let Some(end) = rest.find('"') else { continue };
            let candidate = &rest[..end];
            let looks_like_key = [
                "queue.",
                "history.",
                "subscriptions.",
                "site_rules.",
                "advance.",
                "options.",
                "item.",
            ]
            .iter()
            .any(|prefix| candidate.starts_with(prefix))
                && candidate.chars().all(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '.' || ch == '_'
                });
            if looks_like_key {
                assert!(
                    english.contains(&format!("\"{candidate}\" =>")),
                    "Unknown UI key: {candidate}"
                );
            }
        }
    }
}
