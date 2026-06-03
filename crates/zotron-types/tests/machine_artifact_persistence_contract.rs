use std::path::Path;

use zotron_types::{machine_artifact_store_root_for_platform, ArtifactStorePlatform};

#[test]
fn machine_artifact_store_root_uses_explicit_override_on_every_platform() {
    let root = machine_artifact_store_root_for_platform(
        ArtifactStorePlatform::Macos,
        Some(Path::new("/custom/zotron-artifacts")),
        Some(Path::new("/xdg")),
        None,
        None,
        Some(Path::new("/Users/alice")),
    );

    assert_eq!(root, Path::new("/custom/zotron-artifacts"));
}

#[test]
fn machine_artifact_store_root_follows_linux_data_dir_conventions() {
    let xdg = machine_artifact_store_root_for_platform(
        ArtifactStorePlatform::Linux,
        None,
        Some(Path::new("/home/alice/.local/state")),
        None,
        None,
        Some(Path::new("/home/alice")),
    );
    assert_eq!(
        xdg.to_string_lossy().replace('\\', "/"),
        "/home/alice/.local/state/zotron/artifacts"
    );

    let home = machine_artifact_store_root_for_platform(
        ArtifactStorePlatform::Linux,
        None,
        None,
        None,
        None,
        Some(Path::new("/home/alice")),
    );
    assert_eq!(
        home.to_string_lossy().replace('\\', "/"),
        "/home/alice/.local/share/zotron/artifacts"
    );
}

#[test]
fn machine_artifact_store_root_follows_macos_application_support_convention() {
    let root = machine_artifact_store_root_for_platform(
        ArtifactStorePlatform::Macos,
        None,
        Some(Path::new("/tmp/xdg")),
        None,
        None,
        Some(Path::new("/Users/alice")),
    );

    assert_eq!(
        root.to_string_lossy().replace('\\', "/"),
        "/Users/alice/Library/Application Support/Zotron/artifacts"
    );
}

#[test]
fn machine_artifact_store_root_follows_windows_roaming_appdata_convention() {
    let appdata = machine_artifact_store_root_for_platform(
        ArtifactStorePlatform::Windows,
        None,
        None,
        Some(Path::new(r"C:\Users\Alice\AppData\Roaming")),
        Some(Path::new(r"C:\Users\Alice")),
        Some(Path::new(r"C:\msys64\home\alice")),
    );
    assert_eq!(
        appdata.to_string_lossy().replace('\\', "/"),
        "C:/Users/Alice/AppData/Roaming/Zotron/artifacts"
    );

    let userprofile = machine_artifact_store_root_for_platform(
        ArtifactStorePlatform::Windows,
        None,
        None,
        None,
        Some(Path::new(r"C:\Users\Alice")),
        None,
    );
    assert_eq!(
        userprofile.to_string_lossy().replace('\\', "/"),
        "C:/Users/Alice/AppData/Roaming/Zotron/artifacts"
    );
}

#[test]
fn provider_native_markdown_extracts_layout_parsing_markdown() {
    use serde_json::json;
    use zotron_types::provider_native_markdown;

    let payload = json!({
        "errorCode": 0,
        "result": {
            "layoutParsingResults": [{
                "markdown": {
                    "text": "# 标题\n\n正文",
                    "images": {"figure_1.png": "data:image/png;base64,AAAA"}
                }
            }]
        }
    });

    assert_eq!(
        provider_native_markdown(&payload).as_deref(),
        Some("# 标题\n\n正文")
    );
}

#[test]
fn evidence_artifact_filter_covers_native_provider_sidecars() {
    use zotron_types::is_zotron_evidence_artifact;

    assert!(is_zotron_evidence_artifact("ITEM.zotron-ocr.native.md"));
    assert!(is_zotron_evidence_artifact("ITEM.zotron-ocr.assets.json"));
    assert!(is_zotron_evidence_artifact("latest.native.md"));
    assert!(is_zotron_evidence_artifact("latest.assets.json"));
}
