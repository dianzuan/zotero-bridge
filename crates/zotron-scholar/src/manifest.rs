use serde_json::json;

pub fn print_manifest() {
    let skill_dir = find_skill_dir();

    let manifest = json!({
        "name": "scholar",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "capabilities": ["search", "fetch", "import"],
        "skill_dir": skill_dir,
    });

    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}

/// Locate the scholar skills directory.
///
/// Walk up from the binary location looking for the skills directory in two
/// layouts:
///   - Development (cargo workspace): `<ancestor>/crates/zotron-scholar/skills/scholar`
///   - Installed (`<prefix>/bin`):     `<ancestor>/share/zotron-scholar/skills/scholar`
///
/// Falls back to `$XDG_DATA_HOME/zotron-scholar/skills/scholar`.
fn find_skill_dir() -> String {
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.as_path();
        // Walk up at most 5 levels from the binary.
        for _ in 0..5 {
            dir = match dir.parent() {
                Some(p) => p,
                None => break,
            };

            let dev = dir
                .join("crates")
                .join("zotron-scholar")
                .join("skills")
                .join("scholar");
            if let Ok(canonical) = dev.canonicalize() {
                if canonical.is_dir() {
                    return canonical.to_string_lossy().to_string();
                }
            }

            let installed = dir
                .join("share")
                .join("zotron-scholar")
                .join("skills")
                .join("scholar");
            if let Ok(canonical) = installed.canonicalize() {
                if canonical.is_dir() {
                    return canonical.to_string_lossy().to_string();
                }
            }
        }
    }

    // Fallback: XDG data directory
    let base = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
        format!("{home}/.local/share")
    });
    format!("{base}/zotron-scholar/skills/scholar")
}
