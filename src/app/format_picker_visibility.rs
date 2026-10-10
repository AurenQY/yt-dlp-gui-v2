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
