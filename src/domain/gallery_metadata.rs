//! Canonical gallery metadata shared by UI projection and embedded XMP.
//!
//! Extractor field names are resolved in the app layer.  This domain model is
//! deliberately vocabulary-neutral so presentation and persistence cannot
//! grow independent gallery-dl mapping rules.

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GalleryCanonicalMetadata {
    pub title: Option<String>,
    pub creator: Option<String>,
    pub creator_handle: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    /// ISO 8601 content creation time suitable for `xmp:CreateDate`.
    pub created_at: Option<String>,
    pub source_url: Option<String>,
    pub resource_identifier: Option<GalleryCanonicalIdentifier>,
    pub content_types: Vec<String>,
    pub languages: Vec<String>,
    pub rights: Option<String>,
    pub usage_terms: Option<String>,
    pub user_comment: Option<String>,
    pub location: Option<String>,
    pub category: String,
    pub subcategory: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GalleryCanonicalIdentifier {
    pub scheme: String,
    pub value: String,
}
