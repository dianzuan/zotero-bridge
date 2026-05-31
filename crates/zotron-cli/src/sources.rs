//! Source-plugin discovery, skill sync, and the transparent external-command
//! proxy. Core knows nothing about academic data sources: every `zotron-*`
//! binary on `PATH` is a plugin discovered at runtime via its `manifest`
//! subcommand. `zotron <name> [args]` that does not match a built-in
//! subcommand is forwarded to `zotron-<name>`.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use serde_json::Value;

use crate::output::format_json;

/// Per-plugin timeout for the `manifest` call.
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Built-in top-level subcommand names. Used for the custom "did you mean?"
/// fallback, which replaces clap's built-in suggestion (disabled once
/// `allow_external_subcommands` is on).
pub(crate) const BUILTIN_COMMANDS: &[&str] = &[
    "ping",
    "rpc",
    "push",
    "system",
    "search",
    "items",
    "collections",
    "notes",
    "settings",
    "tags",
    "export",
    "annotations",
    "ocr",
    "rag",
    "sources",
    "help",
];

/// Locate an executable named `binary_name` by scanning `$PATH` directories.
pub(crate) fn which(binary_name: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(binary_name);
            if full_path.is_file() {
                Some(full_path)
            } else {
                None
            }
        })
    })
}

/// Forward `zotron <name> [args]` to the `zotron-<name>` plugin binary,
/// inheriting stdin/stdout/stderr and propagating its exit code. When no such
/// binary exists on PATH, return an `UNKNOWN_COMMAND` error carrying a custom
/// fuzzy "did you mean?" over the built-in command names.
pub(crate) fn run_external_command(args: Vec<OsString>) -> Result<String, String> {
    let name = args
        .first()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "INVALID_ARGS: missing plugin name".to_string())?
        .to_string();

    let binary_name = format!("zotron-{name}");

    let binary_path = match which(&binary_name) {
        Some(path) => path,
        None => return Err(unknown_command_error(&name, &binary_name)),
    };

    let status = ProcessCommand::new(&binary_path)
        .args(&args[1..])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|err| format!("PLUGIN_ERROR: failed to execute {binary_name}: {err}"))?;

    if status.success() {
        // The plugin owns stdout; nothing more to print.
        Ok(String::new())
    } else {
        Err(format!(
            "PLUGIN_ERROR: {binary_name} exited with {}",
            status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string())
        ))
    }
}

/// Build the `UNKNOWN_COMMAND` error message, appending fuzzy suggestions when
/// the typed name is close to a built-in command.
pub(crate) fn unknown_command_error(name: &str, binary_name: &str) -> String {
    let mut message = format!(
        "UNKNOWN_COMMAND: unknown command '{name}'. \
         No plugin '{binary_name}' found on PATH."
    );
    let suggestions = fuzzy_suggestions(name, BUILTIN_COMMANDS);
    if !suggestions.is_empty() {
        message.push_str(" Did you mean: ");
        message.push_str(&suggestions.join(", "));
        message.push('.');
    }
    message
}

/// Return built-in command names close to `input`, ranked by edit distance.
/// A candidate is "close" when its Levenshtein distance is at most a third of
/// the longer string's length (min 1) — the same heuristic clap uses.
pub(crate) fn fuzzy_suggestions(input: &str, candidates: &[&str]) -> Vec<String> {
    let mut scored: Vec<(usize, &str)> = candidates
        .iter()
        .filter_map(|candidate| {
            let distance = levenshtein(input, candidate);
            let threshold = (input.len().max(candidate.len()) / 3).max(1);
            if distance <= threshold {
                Some((distance, *candidate))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    scored.into_iter().map(|(_, name)| name.to_string()).collect()
}

/// Iterative Levenshtein edit distance over Unicode scalar values.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// Scan every `PATH` directory for `zotron-*` executables and call each one's
/// `manifest` subcommand (5s timeout). The result is one JSON object per
/// plugin. Plugins that fail or time out are reported with an `error` field
/// rather than silently dropped.
pub(crate) fn discover_plugins() -> Vec<Value> {
    let paths = match env::var_os("PATH") {
        Some(p) => p,
        None => return vec![],
    };

    // Collect unique zotron-* binaries; the first hit on PATH wins.
    let mut seen = HashSet::new();
    let mut binaries: Vec<(String, PathBuf)> = Vec::new();

    for dir in env::split_paths(&paths) {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = match name.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            if name_str.starts_with("zotron-")
                && name_str != "zotron-"
                && !seen.contains(&name_str)
            {
                let path = entry.path();
                if path.is_file() {
                    seen.insert(name_str.clone());
                    binaries.push((name_str, path));
                }
            }
        }
    }

    binaries.sort_by(|a, b| a.0.cmp(&b.0));

    binaries
        .into_iter()
        .map(|(binary_name, path)| {
            let plugin_name = binary_name
                .strip_prefix("zotron-")
                .unwrap_or(&binary_name)
                .to_string();
            manifest_for(&plugin_name, &path)
        })
        .collect()
}

/// Run `<path> manifest` with a 5-second timeout and normalize the output into
/// a source descriptor. Always includes `name` and `binary`; on the happy path
/// also carries the manifest's `version`/`description`/`capabilities`.
fn manifest_for(plugin_name: &str, path: &Path) -> Value {
    let result = thread::scope(|s| {
        let handle = s.spawn(|| {
            ProcessCommand::new(path)
                .arg("manifest")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
        });
        let start = Instant::now();
        loop {
            if handle.is_finished() {
                return handle.join().ok().and_then(Result::ok);
            }
            if start.elapsed() > MANIFEST_TIMEOUT {
                return None;
            }
            thread::sleep(Duration::from_millis(25));
        }
    });

    match result {
        Some(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match serde_json::from_str::<Value>(stdout.trim()) {
                Ok(mut manifest) => {
                    if let Some(obj) = manifest.as_object_mut() {
                        obj.entry("name")
                            .or_insert_with(|| Value::String(plugin_name.to_string()));
                        obj.insert(
                            "binary".to_string(),
                            Value::String(path.display().to_string()),
                        );
                    }
                    manifest
                }
                Err(err) => error_source(plugin_name, path, format!("invalid manifest JSON: {err}")),
            }
        }
        Some(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error_source(
                plugin_name,
                path,
                format!("manifest command failed: {}", stderr.trim()),
            )
        }
        None => error_source(plugin_name, path, "manifest command timed out (5s)".to_string()),
    }
}

fn error_source(plugin_name: &str, path: &Path, error: String) -> Value {
    serde_json::json!({
        "name": plugin_name,
        "binary": path.display().to_string(),
        "status": "error",
        "error": error,
    })
}

/// `zotron sources` / `zotron sources list`: aggregate discovered plugins.
pub(crate) fn run_sources_list() -> Result<String, String> {
    let plugins = discover_plugins();
    format_json(&serde_json::json!({ "sources": plugins }))
}

/// Walk up from the running executable to find the repo's `plugin/skills/`
/// directory so `sources sync` works without an explicit `--skills-dir`.
fn auto_discover_skills_dir() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let mut dir = exe.parent()?;
    for _ in 0..12 {
        let candidate = dir.join("plugin").join("skills");
        if candidate.is_dir() {
            return Some(candidate);
        }
        dir = dir.parent()?;
    }
    None
}

/// `zotron sources sync`: for each discovered plugin, symlink its
/// `manifest.skill_dir` into `plugin/skills/<name>`; remove symlinks for
/// plugins no longer on PATH.
pub(crate) fn run_sources_sync(skills_dir_arg: &str) -> Result<String, String> {
    let skills_dir = if skills_dir_arg.is_empty() {
        auto_discover_skills_dir().ok_or_else(|| {
            "SYNC_ERROR: could not auto-discover plugin/skills/ directory. \
             Pass --skills-dir explicitly."
                .to_string()
        })?
    } else {
        PathBuf::from(skills_dir_arg)
    };

    if !skills_dir.is_dir() {
        return Err(format!(
            "SYNC_ERROR: skills directory does not exist: {}",
            skills_dir.display()
        ));
    }

    let plugins = discover_plugins();
    let mut linked = 0u64;
    let mut cleaned = 0u64;

    // Names of plugins that currently provide a valid skill_dir; only these
    // symlinks survive the cleanup pass below.
    let mut active_link_names = HashSet::new();

    for plugin in &plugins {
        let name = plugin["name"].as_str().unwrap_or_default();
        let skill_dir_str = plugin["skill_dir"].as_str().unwrap_or_default();
        if name.is_empty() || skill_dir_str.is_empty() {
            continue;
        }
        let skill_dir = PathBuf::from(skill_dir_str);
        if !skill_dir.is_dir() {
            continue;
        }

        let link_path = skills_dir.join(name);
        active_link_names.insert(name.to_string());

        // Already pointing at the right target? Count it and move on.
        if link_path.symlink_metadata().is_ok() {
            if let Ok(target) = fs::read_link(&link_path) {
                if target == skill_dir {
                    linked += 1;
                    continue;
                }
            }
            fs::remove_file(&link_path).map_err(|e| {
                format!(
                    "SYNC_ERROR: failed to remove existing entry at {}: {e}",
                    link_path.display()
                )
            })?;
        }

        symlink_dir(&skill_dir, &link_path)?;
        linked += 1;
    }

    // Remove dangling plugin symlinks: symlinks in skills_dir that point
    // outside it and no longer correspond to a plugin on PATH. Core skill
    // directories (setup/, zotero/) are real directories, not symlinks, so
    // they are never touched.
    if let Ok(entries) = fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !meta.file_type().is_symlink() {
                continue;
            }
            let entry_name = entry.file_name();
            let entry_name_str = match entry_name.to_str() {
                Some(s) => s,
                None => continue,
            };
            if active_link_names.contains(entry_name_str) {
                continue;
            }
            if let Ok(target) = fs::read_link(&path) {
                let target_abs = if target.is_absolute() {
                    target
                } else {
                    skills_dir.join(&target)
                };
                if !target_abs.starts_with(&skills_dir) {
                    let _ = fs::remove_file(&path);
                    cleaned += 1;
                }
            }
        }
    }

    format_json(&serde_json::json!({
        "ok": true,
        "linked": linked,
        "cleaned": cleaned,
        "skills_dir": skills_dir.display().to_string(),
    }))
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link).map_err(|e| {
        format!(
            "SYNC_ERROR: failed to symlink {} -> {}: {e}",
            link.display(),
            target.display()
        )
    })
}

#[cfg(not(unix))]
fn symlink_dir(_target: &Path, _link: &Path) -> Result<(), String> {
    Err("SYNC_ERROR: sources sync is only supported on Unix systems".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_basic_distances() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("ping", "ping"), 0);
        assert_eq!(levenshtein("pign", "ping"), 2);
        assert_eq!(levenshtein("serch", "search"), 1);
        assert_eq!(levenshtein("", "rag"), 3);
    }

    #[test]
    fn fuzzy_suggests_close_builtins() {
        // One transposition away from "search".
        let hits = fuzzy_suggestions("serch", BUILTIN_COMMANDS);
        assert!(hits.contains(&"search".to_string()), "got {hits:?}");
    }

    #[test]
    fn fuzzy_suggests_nothing_for_distant_input() {
        // "scholar" is not close to any built-in — it's a real plugin name.
        assert!(fuzzy_suggestions("scholar", BUILTIN_COMMANDS).is_empty());
    }

    #[test]
    fn fuzzy_ranks_by_edit_distance() {
        // "ite" is closest to "items" among built-ins.
        let hits = fuzzy_suggestions("item", BUILTIN_COMMANDS);
        assert_eq!(hits.first().map(String::as_str), Some("items"));
    }

    #[test]
    fn unknown_command_error_includes_suggestion() {
        let msg = unknown_command_error("serch", "zotron-serch");
        assert!(msg.starts_with("UNKNOWN_COMMAND:"), "got {msg}");
        assert!(msg.contains("zotron-serch"), "got {msg}");
        assert!(msg.contains("Did you mean"), "got {msg}");
        assert!(msg.contains("search"), "got {msg}");
    }

    #[test]
    fn unknown_command_error_omits_suggestion_when_none() {
        let msg = unknown_command_error("scholar", "zotron-scholar");
        assert!(msg.starts_with("UNKNOWN_COMMAND:"), "got {msg}");
        assert!(!msg.contains("Did you mean"), "got {msg}");
    }
}
