//! Aria2 console readouts, shared by desktop and Native Messaging downloads.
//! Parse status snapshots only; these never signal successful completion.

#[derive(Debug)]
pub(crate) struct Aria2Progress {
    pub percent: f32,
    pub downloaded: String,
    pub total: String,
    pub speed: String,
    pub connections: String,
    pub eta: Option<String>,
}

pub(crate) fn parse_aria2_progress(line: &str) -> Option<Aria2Progress> {
    let clean = strip_terminal_sequences(line);
    let mut records = Vec::new();
    for part in clean.split("[#").skip(1) {
        let body = part.split_once(']')?.0;
        records.push(parse_record(body)?);
    }
    let first = records.first()?;
    if records.len() == 1 {
        return Some(Aria2Progress {
            percent: first.percent.min(99.9),
            downloaded: first.downloaded.to_owned(),
            total: first.total.to_owned(),
            speed: format!("{}/s", first.speed),
            connections: first.connections.to_string(),
            eta: first.eta.map(str::to_owned),
        });
    }
    // A console line can contain several GIDs. Never use the last GID's
    // percentage for the whole snapshot: combine known byte totals instead.
    let downloaded = records
        .iter()
        .map(|r| size_bytes(r.downloaded))
        .collect::<Option<Vec<_>>>()?
        .iter()
        .sum::<f64>();
    let total = records
        .iter()
        .map(|r| size_bytes(r.total))
        .collect::<Option<Vec<_>>>()?
        .iter()
        .sum::<f64>();
    if total <= 0.0 {
        return None;
    }
    let speed = records
        .iter()
        .map(|r| size_bytes(r.speed))
        .collect::<Option<Vec<_>>>()?
        .iter()
        .sum::<f64>();
    Some(Aria2Progress {
        percent: ((downloaded / total * 100.0) as f32).clamp(0.0, 99.9),
        downloaded: format!("{downloaded:.0} B"),
        total: format!("{total:.0} B"),
        speed: format!("{speed:.0} B/s"),
        connections: records
            .iter()
            .map(|r| r.connections)
            .sum::<u64>()
            .to_string(),
        // Multiple files have independent estimates; do not invent one ETA.
        eta: None,
    })
}

struct Record<'a> {
    percent: f32,
    downloaded: &'a str,
    total: &'a str,
    speed: &'a str,
    connections: u64,
    eta: Option<&'a str>,
}

fn parse_record(body: &str) -> Option<Record<'_>> {
    let mut fields = body.split_whitespace();
    let gid = fields.next()?;
    if gid.len() != 6 || !gid.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let (sizes, percent) = fields.next()?.split_once('(')?;
    let (downloaded, total) = sizes.split_once('/')?;
    size_bytes(downloaded)?;
    size_bytes(total)?;
    let percent = percent.strip_suffix("%)")?.parse::<f32>().ok()?;
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return None;
    }
    let fields = fields.collect::<Vec<_>>();
    let field = |prefix| fields.iter().find_map(|s| s.strip_prefix(prefix));
    let speed = field("DL:")?;
    size_bytes(speed)?;
    Some(Record {
        percent,
        downloaded,
        total,
        speed,
        connections: field("CN:")?.parse().ok()?,
        eta: field("ETA:").filter(|s| !s.is_empty() && *s != "N/A"),
    })
}

fn size_bytes(value: &str) -> Option<f64> {
    let index = value.find(|c: char| !c.is_ascii_digit() && c != '.')?;
    let number = value[..index].parse::<f64>().ok()?;
    let multiplier = match &value[index..] {
        "B" => 1.0,
        "KiB" => 1024.0,
        "MiB" => 1024.0_f64.powi(2),
        "GiB" => 1024.0_f64.powi(3),
        "TiB" => 1024.0_f64.powi(4),
        "KB" => 1000.0,
        "MB" => 1_000_000.0,
        "GB" => 1_000_000_000.0,
        _ => return None,
    };
    let bytes = number * multiplier;
    (bytes.is_finite() && bytes >= 0.0).then_some(bytes)
}

fn strip_terminal_sequences(line: &str) -> String {
    let mut chars = line.chars().peekable();
    let mut clean = String::new();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for control in chars.by_ref() {
                if ('@'..='~').contains(&control) {
                    break;
                }
            }
        } else {
            clean.push(ch);
        }
    }
    clean
}
