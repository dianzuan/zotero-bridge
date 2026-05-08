use std::path::Path;

use zotron_types::{
    machine_artifact_persist_plan, machine_artifact_store_root_for_platform, ArtifactStorePlatform,
    MachineArtifactKind, MachineArtifactStorage,
};

#[test]
fn machine_artifact_persist_plan_defaults_to_external_store_without_zotero_attachment_rpc() {
    let plan =
        machine_artifact_persist_plan("ITEMKEY", "ATTACHKEY", MachineArtifactKind::OcrRaw, false);

    assert_eq!(plan.storage, MachineArtifactStorage::ExternalStore);
    assert_eq!(
        plan.relative_path.to_string_lossy().replace('\\', "/"),
        "items/ITEMKEY/attachments/ATTACHKEY/zotron-ocr.raw.zip"
    );
    assert_eq!(plan.file_name, "zotron-ocr.raw.zip");
    assert_eq!(plan.zotero_attachment_title, None);
    assert!(
        !plan.should_call_zotero_attachments_add,
        "default OCR/RAG machine artifact persistence must not call Zotero attachments.add"
    );
}

#[test]
fn machine_artifact_persist_plan_requires_explicit_zotero_attachment_opt_in() {
    let plan = machine_artifact_persist_plan(
        "ITEMKEY",
        "ATTACHKEY",
        MachineArtifactKind::EmbeddingVectors,
        true,
    );

    assert_eq!(plan.storage, MachineArtifactStorage::ZoteroAttachment);
    assert_eq!(
        plan.relative_path.to_string_lossy().replace('\\', "/"),
        "items/ITEMKEY/attachments/ATTACHKEY/zotron-embed.npz"
    );
    assert_eq!(
        plan.zotero_attachment_title.as_deref(),
        Some("ITEMKEY.zotron-embed.npz")
    );
    assert!(
        plan.should_call_zotero_attachments_add,
        "Zotero attachment creation is allowed only when explicitly requested"
    );
}

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
