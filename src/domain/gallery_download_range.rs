//! Timeline intent. Frozen boundaries survive stop/restart; no pagination cursor.
use serde::{Deserialize, Serialize};

// gallery-dl usually emits naive UTC datetimes; normalize aware values as well.
const UTC_DATE: &str = "(date.replace(tzinfo=None) - (date.utcoffset() or timedelta()))";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GalleryDownloadRange {
    Posts {
        count: u32,
    },
    Recent {
        days: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        until: Option<u64>,
    },
    Since {
        date: String,
    },
    All,
}

impl Default for GalleryDownloadRange {
    fn default() -> Self {
        Self::Posts { count: 10 }
    }
}

impl GalleryDownloadRange {
    pub fn freeze(&mut self, now: u64) {
        if let Self::Recent { days, since, until } = self {
            since.get_or_insert(now.saturating_sub(u64::from(*days) * 86400));
            until.get_or_insert(now);
        }
    }

    pub fn post_filter(&self) -> Result<Option<String>, String> {
        match self {
            Self::Recent {
                days,
                since: Some(since),
                until: Some(until),
            } if *days > 0 && since <= until && *until <= 253402300799 => {
                Ok(Some(checked_date_filter(format!(
                    "({UTC_DATE} <= datetime.utcfromtimestamp({until})) and ({})",
                    timeline_date_start_filter(&format!("datetime.utcfromtimestamp({since})"))
                ))))
            }
            Self::Since { date } => {
                let (year, month, day) = parse_gallery_date(date)?;
                Ok(Some(checked_date_filter(timeline_date_start_filter(
                    &format!("datetime({year}, {month}, {day})"),
                ))))
            }
            Self::All => Ok(None),
            Self::Posts { count } if *count > 0 => Ok(None),
            _ => Err("Invalid gallery download range.".to_owned()),
        }
    }
}

fn checked_date_filter(condition: String) -> String {
    format!(
        "({condition}) if isinstance(locals().get('date'), datetime) else error('Timeline date metadata is missing or unsupported')"
    )
}

/// Instagram posts expose pinned IDs separately from the chronological feed.
/// An old pinned post is skipped, not a boundary. Unknown feeds/pin metadata
/// retain the full-scan filter; do not assume every extractor is date ordered.
fn timeline_date_start_filter(lower: &str) -> String {
    format!(
        "({UTC_DATE} >= {lower}) or (abort() if locals().get('category') == 'instagram' and locals().get('subcategory') == 'posts' and 'pinned' in locals() and not pinned else False)"
    )
}

pub fn parse_gallery_date(date: &str) -> Result<(u32, u32, u32), String> {
    let invalid = || "Invalid UTC date; expected YYYY-MM-DD.".to_owned();
    let parts = date.split('-').collect::<Vec<_>>();
    if parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
        || !parts
            .iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(invalid());
    }
    let y = parts[0].parse::<u32>().map_err(|_| invalid())?;
    let m = parts[1].parse::<u32>().map_err(|_| invalid())?;
    let d = parts[2].parse::<u32>().map_err(|_| invalid())?;
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let max = match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    if !(1..=9999).contains(&y) || d == 0 || d > max {
        return Err(invalid());
    }
    Ok((y, m, d))
}
