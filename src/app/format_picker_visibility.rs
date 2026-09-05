use crate::domain::FormatOption;
use crate::infrastructure::UnknownFilesizeFormatPolicy;

pub(super) fn apply_unknown_filesize_format_policy(
    mut options: Vec<FormatOption>,
    policy: UnknownFilesizeFormatPolicy,
) -> Vec<FormatOption> {
    if policy == UnknownFilesizeFormatPolicy::Filter
        && options.iter().any(format_has_known_or_approximate_size)
    {
        options.retain(format_has_known_or_approximate_size);
    }
    options
}

fn format_has_known_or_approximate_size(option: &FormatOption) -> bool {
    !option.filesize.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MediaKind;

    fn option(id: &str, filesize: &str) -> FormatOption {
        FormatOption::video(
            id,
            id,
            MediaKind::Video,
            "1080p",
            "",
            "30",
            "mp4",
            "H.264",
            filesize,
        )
    }

    #[test]
    fn off_keeps_known_and_unknown_size_formats() {
        let options = vec![option("known", "10.0 MiB"), option("unknown", "")];

        let visible =
            apply_unknown_filesize_format_policy(options, UnknownFilesizeFormatPolicy::Off);

        assert_eq!(
            visible
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["known", "unknown"]
        );
    }

    #[test]
    fn filter_keeps_all_formats_when_every_size_is_unknown() {
        let options = vec![option("first", ""), option("second", "")];

        let visible =
            apply_unknown_filesize_format_policy(options, UnknownFilesizeFormatPolicy::Filter);

        assert_eq!(
            visible
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
    }

    #[test]
    fn filter_keeps_only_sized_formats_when_any_size_is_available() {
        let options = vec![
            option("exact-or-approximate", "~10.0 MiB"),
            option("unknown", ""),
            option("another-sized", "12.0 MiB"),
        ];

        let visible =
            apply_unknown_filesize_format_policy(options, UnknownFilesizeFormatPolicy::Filter);

        assert_eq!(
            visible
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["exact-or-approximate", "another-sized"]
        );
    }
}
