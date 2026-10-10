//! Root extractor source scope shared by retention and known-total progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GallerySourceScope {
    SinglePost,
    Timeline,
    Unknown,
}

/// Never infer scope from a bounded one/two-post analysis sample. Unknown
/// extractor semantics remain explicit so callers can avoid data loss/false totals.
pub(crate) fn gallery_source_scope(root: &str) -> GallerySourceScope {
    let kind = root
        .rsplit(" · ")
        .next()
        .unwrap_or(root)
        .to_ascii_lowercase();
    const SINGLE: &[&str] = &[
        "post",
        "tweet",
        "status",
        "note",
        "work",
        "unlisted",
        "artwork",
        "deviation",
        "illustration",
        "image",
        "photo",
        "file",
        "directlink",
        "direct link",
        "article",
        "entry",
        "submission",
        "pin",
        "novel",
        "video",
        "audio",
        "avatar",
        "background",
    ];
    const TIMELINE: &[&str] = &[
        "user",
        "posts",
        "photos",
        "reels",
        "tagged",
        "saved",
        "stories",
        "stories tray",
        "highlights",
        "followers",
        "following",
        "tag",
        "home",
        "notifications",
        "search",
        "hashtag",
        "timeline",
        "tweets",
        "with replies",
        "media",
        "likes",
        "bookmark",
        "bookmarks",
        "list",
        "list members",
        "community",
        "communities",
        "quotes",
        "feed",
        "artworks",
        "favorite",
        "followed",
        "ranking",
        "popular",
        "recent",
        "uploads",
        "user posts",
        "user works",
        "creator posts",
        "creator media",
    ];
    // Inherited rules in the embedded index use class-style names (e.g.
    // DanbooruPost/MastodonUser), while ordinary rules use category · subcategory.
    let single_class = [
        "post",
        "tweet",
        "status",
        "note",
        "illustration",
        "image",
        "file",
    ]
    .iter()
    .any(|suffix| kind.ends_with(suffix));
    let timeline_class = [
        "user",
        "search",
        "tag",
        "favorite",
        "bookmark",
        "following",
        "feed",
        "tweets",
        "notes",
        "submissions",
        "popular",
    ]
    .iter()
    .any(|suffix| kind.ends_with(suffix));
    if SINGLE.contains(&kind.as_str()) || single_class {
        GallerySourceScope::SinglePost
    } else if TIMELINE.contains(&kind.as_str()) || timeline_class {
        GallerySourceScope::Timeline
    } else {
        GallerySourceScope::Unknown
    }
}
