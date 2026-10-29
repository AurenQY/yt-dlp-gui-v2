//! Direct-tool preparation; the prepared command and private output owner travel together.
use super::*;
use crate::infrastructure::{
    output_policy::*,
    output_transaction::{OutputArtifacts, OutputTransaction},
};
use std::sync::{Arc, Mutex};

pub(super) fn prepare_plan(
    tools: &ToolPaths,
    request: &DownloadRequest,
) -> Result<(OutputPlan, Arc<Mutex<OutputTransaction>>), String> {
    let mut planned = request.clone();
    let mut automatic_basename = None;
    if request.output.transaction.is_none() {
        let mut context = request.output.context.clone();
        if context.source.is_empty() {
            context.source = request.url.clone();
        }
        let external = tools
            .output_config
            .as_ref()
            .and_then(|c| c.path.as_ref().map(|_| c.output.clone()))
            .filter(|v| v.0.is_some() || v.1.is_some());
        let needs_metadata = request.output.automatic && !context.resolved;
        if needs_metadata {
            context = resolve_context(
                tools,
                &request.url,
                request.use_cookies,
                request.playlist_index,
                &request.output.cancel,
            )?;
        }
        if !request.output.automatic && !request.output.literal_name.trim().is_empty() {
            planned.file_name = request.output.literal_name.clone();
        }
        if let Some((template, directory)) = external {
            if request.output_path.is_none() {
                if request.output.automatic && template.is_none() {
                    automatic_basename = Some(request.output.preferences.basename(&context)?);
                }
                if let Some(directory) = directory {
                    planned.output_dir = directory;
                }
                if let Some(template) = template.filter(|_| request.output.automatic) {
                    let relative = expand_external(
                        &template,
                        &context,
                        &determine_expected_container(request),
                    )?;
                    let path = Path::new(&relative);
                    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                        planned.output_dir = resolve_output_dir(&planned.output_dir)?
                            .join(parent)
                            .display()
                            .to_string();
                    }
                    planned.file_name = path
                        .file_name()
                        .ok_or("Output template has no filename")?
                        .to_string_lossy()
                        .into_owned();
                }
            }
        } else if request.output.automatic && request.output_path.is_none() {
            automatic_basename = Some(request.output.preferences.basename(&context)?);
        }
    }
    // The original planner retains the existing container/explicit extension
    // semantics. Only the final publication boundary changes ownership.
    if automatic_basename.is_some() {
        planned.file_name = "output".into();
    }
    let mut plan = if let Some(owner) = &request.output.transaction {
        owner
            .lock()
            .map_err(|_| "Output transaction unavailable")?
            .plan
            .clone()
            .ok_or("Output plan unavailable")?
    } else {
        build_output_plan(&planned)?
    };
    if let Some(basename) = automatic_basename {
        let extension = plan
            .final_output_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");
        plan.final_output_path = plan
            .final_output_path
            .with_file_name(format!("{basename}.{extension}"));
    }
    let owner = if let Some(owner) = &request.output.transaction {
        owner.clone()
    } else {
        let destination = plan
            .final_output_path
            .parent()
            .ok_or("Output has no directory")?;
        let stem = plan
            .final_output_path
            .file_stem()
            .ok_or("Output has no filename")?
            .to_string_lossy();
        Arc::new(Mutex::new(OutputTransaction::new_with_status(
            destination,
            &stem,
            direct_artifacts(tools, request, &plan),
            request.output.preferences.existing,
            &request.output.cancel,
            request.output.status.clone(),
        )?))
    };
    let work = {
        let mut guard = owner.lock().map_err(|_| "Output transaction unavailable")?;
        if guard.plan.is_none() {
            if request.download_section_args.len() > 1 {
                guard.include_sections(&request.output.cancel)?;
            }
            guard.plan = Some(plan.clone());
            guard.output_config = tools.output_config.clone();
        }
        guard.work.clone()
    };
    // Preserve templates only for the existing section-number expansion.
    let extension = plan
        .final_output_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");
    let output =
        output_base_name_with_section_number("output", request.download_section_args.len());
    let tool_extension = if plan.remux_extension.is_some() {
        "%(ext)s"
    } else {
        extension
    };
    plan.output_argument = output_argument_for_kind(
        request.target_kind,
        &work.join(format!("{output}.{tool_extension}")),
    );
    plan.final_output_path = work.join(format!("output.{extension}"));
    Ok((plan, owner))
}

fn direct_artifacts(
    tools: &ToolPaths,
    request: &DownloadRequest,
    plan: &OutputPlan,
) -> OutputArtifacts {
    let extension = plan
        .final_output_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    let thumbnail_is_embedded = request.embed_thumbnail && !extension.eq_ignore_ascii_case("webm");
    let extension = request
        .output
        .transcode
        .as_ref()
        .map(|s| match s.container_policy {
            crate::infrastructure::ContainerPolicy::Mp4 => "mp4",
            crate::infrastructure::ContainerPolicy::Mkv => "mkv",
            crate::infrastructure::ContainerPolicy::Mov => "mov",
            crate::infrastructure::ContainerPolicy::Auto => extension,
        })
        .unwrap_or(extension);
    let mut artifacts = OutputArtifacts::file(extension);
    if let Some(lang) = &request.subtitle_lang {
        if request.target_kind != DownloadTargetKind::Subtitle
            && !tools.has_explicit_config()
            && request.write_subtitles
            && !request.embed_subtitles
        {
            artifacts = artifacts.with_suffix(&format!("{lang}.srt"));
        }
    }
    if !tools.has_explicit_config() && request.write_thumbnail && !thumbnail_is_embedded {
        artifacts = artifacts.with_suffix("jpg");
    }
    if !tools.has_explicit_config() && request.write_chapters && !request.embed_chapters {
        artifacts = artifacts.with_suffix("chapters.xml");
    }
    if request.download_section_args.len() > 1 {
        artifacts.sections()
    } else {
        artifacts
    }
}

pub(super) fn external_output(
    options: &[ConfigOption],
) -> Result<(Option<String>, Option<String>), String> {
    let mut template = None;
    let mut directory = None;
    for option in options {
        let output = (option.name == "--output").then(|| option.values[0].as_str());
        if let Some(output) = output {
            if output.split_once(':').is_some_and(|(kind, _)| {
                kind.len() > 1 && kind.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            }) {
                return Err(format!(
                    "Unsupported typed external output template: {output}"
                ));
            }
            template = Some(output.to_owned());
        }
        let value = (option.name == "--paths").then(|| option.values[0].as_str());
        if let Some(value) = value {
            if let Some(value) = value.strip_prefix("home:") {
                directory = Some(value.to_owned());
            } else if !value.starts_with("temp:") {
                directory = Some(value.to_owned());
            }
        }
    }
    Ok((template, directory))
}

pub(super) fn music_destination(
    tools: &ToolPaths,
    output: &OutputRequest,
    url: &str,
    directory: &Path,
    literal: &str,
    use_cookies: bool,
) -> Result<(PathBuf, String), String> {
    let mut directory = directory.to_owned();
    let external = tools
        .output_config
        .as_ref()
        .and_then(|c| c.path.as_ref().map(|_| c.output.clone()));
    let needs_metadata = if let Some(template) = external.as_ref().and_then(|v| v.0.as_ref()) {
        let mut rest = template.as_str();
        let mut needed = false;
        while let Some(index) = rest.find('%') {
            rest = &rest[index + 1..];
            if let Some(next) = rest.strip_prefix('%') {
                rest = next;
                continue;
            }
            if let Some((key, next)) = rest.strip_prefix('(').and_then(|v| v.split_once(")s")) {
                needed |= output.context.needs_field(key);
                rest = next;
            } else {
                break;
            }
        }
        needed
    } else {
        output.preferences.needs_metadata(&output.context)
    };
    let context = if output.automatic && needs_metadata {
        resolve_context(tools, url, use_cookies, None, &output.cancel)?
    } else {
        output.context.clone()
    };
    let mut stem = if output.automatic {
        output.preferences.basename(&context)?
    } else {
        safe_stem(&split_file_name_parts(literal).0)
    };
    if let Some((template, home)) = external {
        if let Some(home) = home {
            directory = resolve_output_dir(&home)?;
        }
        if let Some(template) = template.filter(|_| output.automatic) {
            let relative = expand_external(&template, &context, "media")?;
            let path = Path::new(&relative);
            if let Some(parent) = path.parent() {
                directory = directory.join(parent);
            }
            stem = path
                .file_stem()
                .ok_or("External output template has no filename")?
                .to_string_lossy()
                .into_owned();
        }
    }
    Ok((directory, stem))
}

fn resolve_context(
    tools: &ToolPaths,
    url: &str,
    cookies: bool,
    index: Option<usize>,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<OutputNameContext, String> {
    let metadata = tools
        .analyze_output_naming(url, cookies, index, cancel)
        .map_err(|e| e.message)?;
    context_from_metadata(&metadata.json, url, index)
}

fn context_from_metadata(
    json: &serde_json::Value,
    url: &str,
    index: Option<usize>,
) -> Result<OutputNameContext, String> {
    let value = if let Some(entries) = json.get("entries").and_then(serde_json::Value::as_array) {
        let entry = index
            .and_then(|index| {
                entries.iter().find(|entry| {
                    entry
                        .get("playlist_index")
                        .and_then(serde_json::Value::as_u64)
                        == Some(index as u64)
                })
            })
            .or_else(|| (entries.len() == 1).then(|| &entries[0]))
            .ok_or("Could not resolve naming metadata for the selected playlist entry.")?;
        entry
    } else {
        json
    };
    Ok(OutputNameContext::from_json(value, url))
}

#[cfg(test)]
mod naming_metadata_tests {
    use super::*;
    #[test]
    fn selected_entry_and_genuinely_absent_fields() {
        let metadata = serde_json::json!({"title":"Playlist display placeholder","entries":[{"playlist_index":1,"title":"First","id":"one"},{"playlist_index":2,"title":"Second","id":"two"}]});
        let context = context_from_metadata(&metadata, "source", Some(2)).unwrap();
        assert_eq!(
            (context.title.as_str(), context.id.as_str()),
            ("Second", "two")
        );
        assert!(context.resolved);
        assert!(context_from_metadata(&metadata, "source", None).is_err());
        let absent = context_from_metadata(&serde_json::json!({}), "source", None).unwrap();
        assert!(absent.resolved);
        assert_eq!(
            OutputPreferences::default().basename(&absent).unwrap(),
            "Untitled"
        );
    }
}

pub(super) fn validate_protected_options(options: &[ConfigOption]) -> Result<(), String> {
    const UNSAFE: &[&str] = &[
        // Reject external sidecar combinations whose final group is unplanned.
        "--write-description",
        "--write-info-json",
        "--write-playlist-metafiles",
        "--write-thumbnail",
        "--write-all-thumbnails",
        "--write-subs",
        "--write-srt",
        "--write-auto-subs",
        "--write-automatic-subs",
        "--all-subs",
        "--embed-subs",
        "--embed-thumbnail",
        "--convert-thumbnails",
        "--convert-subs",
        "--write-link",
        "--write-url-link",
        "--write-webloc-link",
        "--write-desktop-link",
        "--write-comments",
        "--get-comments",
        "--keep-video",
        "--extract-audio",
        "--audio-format",
        "--remux-video",
        "--recode-video",
        "--merge-output-format",
        "--split-chapters",
        "--compat-options",
        "--exec",
        "--exec-before-download",
        "--postprocessor-args",
        "--ppa",
        "--use-postprocessor",
        "--use-postprocessors",
        "--external-downloader-args",
        "--downloader-args",
        "--print",
        "--print-to-file",
        "--batch-file",
        "--dump-pages",
        "--write-pages",
        "--download-archive",
        "--parse-metadata",
        "--replace-in-metadata",
        "--plugin-dirs",
        "--config-location",
        "--config-locations",
        "--alias",
    ];
    for parsed in options {
        let option = parsed.name.as_str();
        if option == "--preset-alias" && parsed.values.first().map(String::as_str) != Some("sleep")
        {
            return Err(format!(
                "Unsupported preset for protected output: {} {}. Select the app's audio/conversion operation or use supported individual config options.",
                parsed.spelling,
                parsed.values.first().map_or("", String::as_str)
            ));
        }
        if (option.starts_with("--") && UNSAFE.iter().any(|name| name.starts_with(option)))
            || (!option.starts_with("--")
                && (option.starts_with("-a")
                    || option.starts_with("-O")
                    || option.starts_with("-x")
                    || option.starts_with("-k")))
        {
            return Err(format!(
                "Unsupported option for protected output: {}",
                parsed.spelling
            ));
        }
        for name in ["--output", "--paths", "--load-info-json"] {
            let spelling = parsed.spelling.as_str();
            if spelling.starts_with("--") && name.starts_with(spelling) && name != spelling {
                return Err(format!("Unsupported abbreviated output option: {spelling}"));
            }
        }
        if option == "--load-info-json" {
            let path = parsed
                .values
                .first()
                .map(String::as_str)
                .ok_or("--load-info-json requires a file")?;
            let value: serde_json::Value = serde_json::from_slice(
                &fs::read(path)
                    .map_err(|e| format!("Could not inspect output metadata {path}: {e}"))?,
            )
            .map_err(|e| e.to_string())?;
            fn check(value: &serde_json::Value) -> bool {
                match value {
                    serde_json::Value::Object(map) => map.iter().all(|(key, value)| {
                        !matches!(
                            key.as_str(),
                            "filepath"
                                | "_filename"
                                | "__files_to_move"
                                | "requested_downloads"
                                | "entries"
                        ) && check(value)
                    }),
                    serde_json::Value::Array(values) => values.iter().all(check),
                    _ => true,
                }
            }
            if !check(&value) {
                return Err(format!(
                    "Unsupported output paths or entries in --load-info-json: {path}"
                ));
            }
        }
    }
    Ok(())
}

fn expand_external(
    template: &str,
    context: &OutputNameContext,
    extension: &str,
) -> Result<String, String> {
    let mut result = String::new();
    let mut rest = template;
    while let Some(index) = rest.find('%') {
        result.push_str(&rest[..index]);
        rest = &rest[index + 1..];
        if let Some(next) = rest.strip_prefix('%') {
            result.push('%');
            rest = next;
            continue;
        }
        let Some(next) = rest.strip_prefix('(') else {
            return Err("Unsupported percent expression in output template.".into());
        };
        let Some((key, next)) = next.split_once(")s") else {
            return Err("Unsupported output template formatting.".into());
        };
        let value = match key {
            "ext" => extension.to_owned(),
            "title" | "id" | "uploader" | "upload_date" => {
                render_template(&format!("{{{key}}}"), context)?
            }
            _ => {
                return Err(format!(
                    "Unsupported protected output template field: {key}"
                ));
            }
        };
        result.push_str(&value);
        rest = next;
    }
    result.push_str(rest);
    let path = Path::new(&result);
    if path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::Prefix(_)
            )
        })
    {
        return Err("Output template escapes its selected output folder.".into());
    }
    Ok(result)
}
