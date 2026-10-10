//! Chapter metadata choices shared by desktop and browser downloads.
//! Splitting media belongs to the independent audio/range workflows, not these options.
use std::{fs, path::Path, process::Command};

use serde_json::Value;

pub(crate) const CHAPTER_OUTPUT_PREFIX: &str = "__YTDLPGUI_CHAPTERS__=";

pub(crate) fn apply_chapter_choices(command: &mut Command, write: bool, embed: bool) {
    command.arg("--no-split-chapters");
    command.arg(if embed {
        "--embed-chapters"
    } else {
        "--no-embed-chapters"
    });
    if write && !embed {
        command.arg("--print").arg(format!(
            "after_move:{CHAPTER_OUTPUT_PREFIX}%(.{{filepath,chapters,section_start,section_end}})j"
        ));
    }
}

pub(crate) fn write_chapter_sidecars(lines: &[String]) -> Result<(), String> {
    for payload in lines
        .iter()
        .filter_map(|line| line.trim().strip_prefix(CHAPTER_OUTPUT_PREFIX))
    {
        let info: Value = serde_json::from_str(payload)
            .map_err(|error| format!("Could not parse downloaded chapters: {error}"))?;
        let Some(chapters) = info
            .get("chapters")
            .and_then(Value::as_array)
            .filter(|v| !v.is_empty())
        else {
            continue;
        };
        let path = info
            .get("filepath")
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .map(Path::new)
            .ok_or("Downloaded chapter metadata has no media path.")?;
        if !path.is_file() {
            return Err(format!(
                "Chapter media file was not found: {}",
                path.display()
            ));
        }
        let offset = info
            .get("section_start")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let limit = info
            .get("section_end")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Chapters>\n  <EditionEntry>\n",
        );
        let mut count = 0;
        for chapter in chapters {
            let start = chapter
                .get("start_time")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite() && *v >= 0.0)
                .ok_or("Downloaded chapter has an invalid start time.")?;
            let end = chapter
                .get("end_time")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite() && *v >= start);
            if start >= limit || end.is_some_and(|end| end <= offset) {
                continue;
            }
            count += 1;
            xml.push_str(&format!("    <ChapterAtom>\n      <ChapterUID>{count}</ChapterUID>\n      <ChapterTimeStart>{}</ChapterTimeStart>\n", timestamp((start - offset).max(0.0))?));
            if let Some(end) = end {
                xml.push_str(&format!(
                    "      <ChapterTimeEnd>{}</ChapterTimeEnd>\n",
                    timestamp(end.min(limit) - offset)?
                ));
            }
            let title = chapter.get("title").and_then(Value::as_str).unwrap_or("");
            xml.push_str(&format!("      <ChapterDisplay><ChapterString>{}</ChapterString><ChapterLanguage>und</ChapterLanguage></ChapterDisplay>\n    </ChapterAtom>\n", escape_xml(title)));
        }
        if count == 0 {
            continue;
        }
        xml.push_str("  </EditionEntry>\n</Chapters>\n");
        let sidecar = path.with_extension("chapters.xml");
        fs::write(&sidecar, xml).map_err(|error| {
            format!(
                "Could not write chapter file {}: {error}",
                sidecar.display()
            )
        })?;
    }
    Ok(())
}

fn timestamp(seconds: f64) -> Result<String, String> {
    let nanos = seconds * 1_000_000_000.0;
    if !nanos.is_finite() || nanos < 0.0 || nanos >= u64::MAX as f64 {
        return Err("Downloaded chapter timestamp is out of range.".to_owned());
    }
    let nanos = nanos.round() as u64;
    let seconds = nanos / 1_000_000_000;
    Ok(format!(
        "{:02}:{:02}:{:02}.{:09}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60,
        nanos % 1_000_000_000
    ))
}

fn escape_xml(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            matches!(*c, '\t' | '\n' | '\r') || *c >= ' ' && !matches!(*c, '\u{FFFE}' | '\u{FFFF}')
        })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
