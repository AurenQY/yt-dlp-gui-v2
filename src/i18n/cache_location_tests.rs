//! Scoped checks for the cache settings UI; unrelated localization is not rewritten.
fn cache_keys(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| {
            let start = line.find("\"options.cache")? + 1;
            let rest = &line[start..];
            Some(&rest[..rest.find('"')?])
        })
        .collect()
}

#[test]
fn i18n_keys_cache_location() {
    let canonical = cache_keys(include_str!("en_us.rs"));
    assert!(!canonical.is_empty());
    for (name, source) in [
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
    ] {
        assert_eq!(
            cache_keys(source),
            canonical,
            "Cache key coverage/order: {name}"
        );
    }
}

#[test]
fn i18n_used_keys_cache_location() {
    let canonical = cache_keys(include_str!("en_us.rs"));
    for source in [
        include_str!("../app/ui/options_cache.rs"),
        include_str!("../app/ui/options_layout.rs"),
        include_str!("../app/state/cache_actions.rs"),
    ] {
        for key in cache_keys(source) {
            assert!(canonical.contains(&key), "Unknown cache UI key: {key}");
        }
    }
}
