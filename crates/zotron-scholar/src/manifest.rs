use serde_json::json;

pub fn print_manifest() {
    let skill_dir = std::env::var("XDG_DATA_HOME")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
            format!("{home}/.local/share")
        })
        + "/zotron-scholar/skills/scholar";

    let manifest = json!({
        "name": "zotron-scholar",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "capabilities": ["search", "fetch", "import"],
        "skill_dir": skill_dir,
    });

    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}
