//! TeX utility functions: command scanning, path utilities, JSONL parsing.

use std::path::Path;

use crate::{CollectorError, SemanticEvent, SemanticEventV2};

/// A scanned TeX command with its name and argument.
#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub argument: String,
}

/// Parse semantic events from a JSONL sidecar file (v1 and v2 schema).
pub fn parse_semantic_events_jsonl(input: &str) -> Result<Vec<SemanticEvent>, CollectorError> {
    let mut events = Vec::new();
    let mut line_number = 0;

    for raw_line in input.lines() {
        line_number += 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Skip v2 schema header lines (e.g. {"schema":"semantic-event-v2","engine":"xelatex"})
        if trimmed.starts_with('{')
            && trimmed.contains("\"schema\"")
            && trimmed.contains("semantic-event-v2")
        {
            continue;
        }

        // v2 event lines have "source" and "macro" fields (no schema field in hook output).
        // v1 event lines are plain semantic events with no source/macro nesting.
        let looks_like_v2 = trimmed.starts_with('{')
            && trimmed.contains("\"source\":{")
            && trimmed.contains("\"macro\"");

        if looks_like_v2 {
            match serde_json::from_str::<SemanticEventV2>(trimmed) {
                Ok(v2) => events.push(v2.into_event()),
                // Some v2 events lack the schema field — fall back to v1
                Err(_) => {
                    let event = serde_json::from_str::<SemanticEvent>(trimmed).map_err(|_| {
                        CollectorError::Parse(format!(
                            "semantic sidecar line {line_number} is invalid JSON"
                        ))
                    })?;
                    events.push(event);
                }
            }
        } else {
            let event = serde_json::from_str::<SemanticEvent>(trimmed).map_err(|_| {
                CollectorError::Parse(format!(
                    "semantic sidecar line {line_number} is invalid JSON"
                ))
            })?;
            events.push(event);
        }
    }

    Ok(events)
}

/// Check if a path points to a TeX-like source file.
pub fn is_tex_like_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "tex" | "sty" | "cls" | "ltx"
            )
        })
        .unwrap_or(false)
}

/// Normalize a filesystem path to POSIX forward slashes.
pub fn path_to_posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Strip TeX comment lines (% to end of line, respecting \verb|%| and strings).
pub fn strip_tex_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.split_inclusive('\n') {
        let bytes = line.as_bytes();
        let mut end = line.len();
        for (idx, b) in bytes.iter().enumerate() {
            if *b == b'%' && !is_escaped(bytes, idx) {
                end = idx;
                break;
            }
        }
        out.push_str(&line[..end]);
        if line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn is_escaped(bytes: &[u8], idx: usize) -> bool {
    let mut count = 0usize;
    let mut cursor = idx;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        count += 1;
        cursor -= 1;
    }
    count % 2 == 1
}

/// Scan for TeX commands with their braced arguments.
pub fn scan_tex_commands(text: &str, commands: &[&str]) -> Vec<Command> {
    let mut found = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }

        let command_start = i + 1;
        let Some(command) = commands
            .iter()
            .find(|command| tex_command_matches(text, command_start, command))
        else {
            i += 1;
            continue;
        };

        let mut arg_pos = command_start + command.len();
        arg_pos = skip_tex_space_and_options(text, arg_pos);
        if text.as_bytes().get(arg_pos) != Some(&b'{') {
            i += command.len() + 1;
            continue;
        }

        if let Some(end) = doc_latex_reader::normalize::find_matching_brace(text, arg_pos) {
            found.push(Command {
                name: (*command).to_string(),
                argument: text[arg_pos + 1..end].trim().to_string(),
            });
            i = end + 1;
        } else {
            i += command.len() + 1;
        }
    }
    found
}

fn tex_command_matches(text: &str, command_start: usize, command: &str) -> bool {
    let end = command_start + command.len();
    if text.get(command_start..end) != Some(command) {
        return false;
    }
    let next = text.as_bytes().get(end).copied();
    !matches!(next, Some(b'a'..=b'z' | b'A'..=b'Z' | b'@'))
}

fn skip_tex_space_and_options(text: &str, mut pos: usize) -> usize {
    loop {
        while pos < text.len() && text.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }

        if text.as_bytes().get(pos) != Some(&b'[') {
            return pos;
        }

        let Some(end) = find_matching_bracket(text, pos) else {
            return pos;
        };
        pos = end + 1;
    }
}

fn find_matching_bracket(text: &str, open_index: usize) -> Option<usize> {
    if text.as_bytes().get(open_index) != Some(&b'[') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open_index;
    while i < text.len() {
        let b = text.as_bytes()[i];
        let escaped = is_escaped(text.as_bytes(), i);
        if b == b'[' && !escaped {
            depth += 1;
        } else if b == b']' && !escaped {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Split a citation argument into individual citation keys.
pub fn split_citation_keys(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Split a package/class name list (e.g., from \usepackage{geometry,hyperref})
/// into individual package names.
pub fn split_tex_name_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .collect()
}
