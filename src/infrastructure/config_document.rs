//! Hierarchical sparse YAML with generated English comments.
use serde_yaml::{Mapping, Value};

use super::{AppConfig, DependencyTool};

#[path = "config_schema.rs"]
mod schema;

const HEADER: &str = "# Only settings that differ from the application defaults are stored.\n";

pub(super) fn decode(content: &str) -> Result<AppConfig, String> {
    let document: Value = serde_yaml::from_str(content).map_err(|e| e.to_string())?;
    if !document.is_null() && !document.is_mapping() {
        return Err("Settings must be a YAML mapping.".to_owned());
    }
    let mut snapshot = Value::Mapping(Mapping::new());
    let mut output_errors = Vec::new();
    for field in schema::FIELDS {
        let value = match read_path(&document, field.path) {
            Ok(value) => value,
            Err(error) if field.source.starts_with("output_") => {
                output_errors.push(format!(
                    "Invalid {}: {error} Using its safe default.",
                    field.path
                ));
                continue;
            }
            Err(error) => return Err(error),
        };
        if let Some(value) = value {
            if field.source.starts_with("output_") {
                let valid = match field.source {
                    "output_naming" => value.as_str().is_some_and(|v| {
                        matches!(v, "title" | "title_id" | "uploader_title_id" | "custom")
                    }),
                    "output_existing_file" => value
                        .as_str()
                        .is_some_and(|v| matches!(v, "keep_both" | "skip")),
                    "output_template" => value.as_str().is_some_and(|v| {
                        super::super::output_policy::render_template(v, &Default::default()).is_ok()
                    }),
                    _ => true,
                };
                if !valid {
                    output_errors.push(format!("Invalid {}; using its safe default.", field.path));
                    continue;
                }
            }
            insert_path(&mut snapshot, field.source, value.clone());
        }
    }
    let mut config: AppConfig = serde_yaml::from_value(snapshot).map_err(|e| e.to_string())?;
    if !output_errors.is_empty() {
        config.output_settings_error = Some(output_errors.join(" "));
    }
    Ok(config)
}

pub(super) fn encode(config: &AppConfig) -> Result<String, String> {
    let snapshot = serde_yaml::to_value(config).map_err(|e| e.to_string())?;
    let defaults = serde_yaml::to_value(AppConfig::default()).map_err(|e| e.to_string())?;
    let mut document = Value::Mapping(Mapping::new());
    for field in schema::FIELDS {
        let Some(value) = read_path(&snapshot, field.source)? else {
            continue;
        };
        if Some(value) == read_path(&defaults, field.source)?
            || is_standard_path(field.source, value)
        {
            continue;
        }
        insert_path(&mut document, field.path, value.clone());
    }
    let mut output = HEADER.to_owned();
    if document.as_mapping().is_some_and(Mapping::is_empty) {
        output.push_str("{}\n");
    } else {
        write_mapping(&mut output, document.as_mapping().unwrap(), "", 0)?;
    }
    Ok(output)
}

fn read_path<'a>(value: &'a Value, path: &str) -> Result<Option<&'a Value>, String> {
    let mut node = value;
    for part in path.split('.') {
        if node.is_null() {
            return Ok(None);
        }
        let mapping = node
            .as_mapping()
            .ok_or_else(|| format!("Expected a settings group at {path}."))?;
        let Some(child) = mapping.get(Value::String(part.to_owned())) else {
            return Ok(None);
        };
        node = child;
    }
    Ok(Some(node))
}

fn insert_path(root: &mut Value, path: &str, value: Value) {
    let mut node = root;
    let mut parts = path.split('.').peekable();
    while let Some(part) = parts.next() {
        let mapping = node.as_mapping_mut().expect("schema paths form mappings");
        let key = Value::String(part.to_owned());
        if parts.peek().is_none() {
            mapping.insert(key, value);
            return;
        }
        node = mapping
            .entry(key)
            .or_insert_with(|| Value::Mapping(Mapping::new()));
    }
}

fn is_standard_path(source: &str, value: &Value) -> bool {
    let standard = match source {
        "download_dir" => ".\\download",
        "cache_dir" => ".\\cache",
        "yt_dlp_path" => DependencyTool::YtDlp.default_portable_path(),
        "ffmpeg_path" => DependencyTool::Ffmpeg.default_portable_path(),
        "aria2c_path" => DependencyTool::Aria2c.default_portable_path(),
        "deno_path" => DependencyTool::Deno.default_portable_path(),
        _ => return false,
    };
    value.as_str().is_some_and(|path| path == standard)
}

fn write_mapping(
    output: &mut String,
    mapping: &Mapping,
    parent: &str,
    depth: usize,
) -> Result<(), String> {
    let indent = "  ".repeat(depth);
    for (key, value) in mapping {
        let name = key.as_str().ok_or("Settings keys must be strings.")?;
        let path = if parent.is_empty() {
            name.to_owned()
        } else {
            format!("{parent}.{name}")
        };
        let comment = schema::FIELDS
            .iter()
            .find(|field| field.path == path)
            .map(|field| field.comment)
            .or_else(|| {
                schema::GROUPS
                    .iter()
                    .find(|(p, _)| *p == path)
                    .map(|(_, c)| *c)
            });
        if let Some(comment) = comment {
            output.push_str(&format!("{indent}# {comment}\n"));
        }
        if let Some(children) = value.as_mapping().filter(|m| !m.is_empty()) {
            output.push_str(&format!("{indent}{name}:\n"));
            write_mapping(output, children, &path, depth + 1)?;
        } else {
            // Serialize the whole leaf entry to preserve YAML quoting and block scalars.
            let mut leaf = Mapping::new();
            leaf.insert(key.clone(), value.clone());
            let yaml = serde_yaml::to_string(&leaf).map_err(|e| e.to_string())?;
            for line in yaml.lines() {
                output.push_str(&indent);
                output.push_str(line);
                output.push('\n');
            }
        }
    }
    Ok(())
}
