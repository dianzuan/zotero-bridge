//! Machine-artifact kinds and hidden per-PDF sidecar paths/IO.

use std::{env, fs, io, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

pub fn is_zotron_evidence_artifact(title: &str) -> bool {
    const SUFFIXES: [&str; 12] = [
        ".zotron-ocr.raw.zip",
        ".zotron-blocks.jsonl",
        ".zotron-chunks.jsonl",
        ".zotron-embed.npz",
        ".zotron-ocr.native.md",
        ".zotron-ocr.assets.json",
        "latest.raw.json",
        "latest.blocks.jsonl",
        "chunks.v1.jsonl",
        "vectors.jsonl",
        "latest.native.md",
        "latest.assets.json",
    ];
    SUFFIXES.iter().any(|suffix| title.ends_with(suffix))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MachineArtifactKind {
    OcrRaw,
    Blocks,
    Chunks,
    EmbeddingVectors,
    OcrNativeMarkdown,
    OcrNativeAssets,
}

impl MachineArtifactKind {
    pub fn file_name(self) -> &'static str {
        match self {
            Self::OcrRaw => "latest.raw.json",
            Self::Blocks => "latest.blocks.jsonl",
            Self::Chunks => "chunks.v1.jsonl",
            Self::EmbeddingVectors => "vectors.jsonl",
            Self::OcrNativeMarkdown => "latest.native.md",
            Self::OcrNativeAssets => "latest.assets.json",
        }
    }

    pub fn sidecar_relative_path(self) -> PathBuf {
        match self {
            Self::OcrRaw => PathBuf::from("ocr").join(self.file_name()),
            Self::Blocks => PathBuf::from("ocr").join(self.file_name()),
            Self::Chunks => PathBuf::from("chunks").join(self.file_name()),
            Self::EmbeddingVectors => PathBuf::from("embeddings").join(self.file_name()),
            Self::OcrNativeMarkdown => PathBuf::from("ocr").join(self.file_name()),
            Self::OcrNativeAssets => PathBuf::from("ocr").join(self.file_name()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineArtifactRecord {
    pub item_key: String,
    pub attachment_key: String,
    pub kind: MachineArtifactKind,
    pub relative_path: PathBuf,
    pub absolute_path: PathBuf,
}

pub fn machine_artifact_sidecar_relative_path(kind: MachineArtifactKind) -> PathBuf {
    PathBuf::from(".zotron").join(kind.sidecar_relative_path())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactStorePlatform {
    Linux,
    Macos,
    Windows,
    Other,
}

impl ArtifactStorePlatform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }
}

pub fn machine_artifact_store_root_for_platform(
    platform: ArtifactStorePlatform,
    zotron_artifact_store: Option<&Path>,
    xdg_data_home: Option<&Path>,
    appdata: Option<&Path>,
    userprofile: Option<&Path>,
    home: Option<&Path>,
) -> PathBuf {
    if let Some(path) = zotron_artifact_store {
        return path.to_path_buf();
    }

    match platform {
        ArtifactStorePlatform::Windows => {
            if let Some(path) = appdata {
                return path.join("Zotron").join("artifacts");
            }
            if let Some(path) = userprofile {
                return path
                    .join("AppData")
                    .join("Roaming")
                    .join("Zotron")
                    .join("artifacts");
            }
            if let Some(path) = home {
                return path
                    .join("AppData")
                    .join("Roaming")
                    .join("Zotron")
                    .join("artifacts");
            }
            PathBuf::from(".zotron").join("artifacts")
        }
        ArtifactStorePlatform::Macos => {
            if let Some(path) = home {
                return path
                    .join("Library")
                    .join("Application Support")
                    .join("Zotron")
                    .join("artifacts");
            }
            if let Some(path) = xdg_data_home {
                return path.join("zotron").join("artifacts");
            }
            PathBuf::from(".zotron").join("artifacts")
        }
        ArtifactStorePlatform::Linux | ArtifactStorePlatform::Other => xdg_data_home
            .map(|path| path.join("zotron").join("artifacts"))
            .or_else(|| {
                home.map(|path| {
                    path.join(".local")
                        .join("share")
                        .join("zotron")
                        .join("artifacts")
                })
            })
            .unwrap_or_else(|| PathBuf::from(".zotron").join("artifacts")),
    }
}

pub fn machine_artifact_store_root() -> PathBuf {
    let zotron_artifact_store = env::var_os("ZOTRON_ARTIFACT_STORE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let xdg_data_home = env::var_os("XDG_DATA_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let appdata = env::var_os("APPDATA")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let userprofile = env::var_os("USERPROFILE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let home = env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);

    machine_artifact_store_root_for_platform(
        ArtifactStorePlatform::current(),
        zotron_artifact_store.as_deref(),
        xdg_data_home.as_deref(),
        appdata.as_deref(),
        userprofile.as_deref(),
        home.as_deref(),
    )
}

pub fn machine_artifact_sidecar_absolute_path(
    attachment_storage_dir: impl AsRef<Path>,
    kind: MachineArtifactKind,
) -> PathBuf {
    attachment_storage_dir
        .as_ref()
        .join(machine_artifact_sidecar_relative_path(kind))
}

pub fn write_machine_artifact_sidecar(
    attachment_storage_dir: impl AsRef<Path>,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    bytes: &[u8],
) -> io::Result<MachineArtifactRecord> {
    let relative_path = machine_artifact_sidecar_relative_path(kind);
    let absolute_path = attachment_storage_dir.as_ref().join(&relative_path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&absolute_path, bytes)?;
    Ok(MachineArtifactRecord {
        item_key: item_key.to_string(),
        attachment_key: attachment_key.to_string(),
        kind,
        relative_path,
        absolute_path,
    })
}

pub fn read_machine_artifact_sidecar(
    attachment_storage_dir: impl AsRef<Path>,
    kind: MachineArtifactKind,
) -> io::Result<Vec<u8>> {
    fs::read(machine_artifact_sidecar_absolute_path(
        attachment_storage_dir,
        kind,
    ))
}

pub fn machine_artifact_exists_in_sidecar(
    attachment_storage_dir: impl AsRef<Path>,
    kind: MachineArtifactKind,
) -> bool {
    machine_artifact_sidecar_absolute_path(attachment_storage_dir, kind).exists()
}

pub fn machine_artifact_exists_for_item(
    store_root: impl AsRef<Path>,
    item_key: &str,
    kind: MachineArtifactKind,
) -> bool {
    let legacy_file_name = match kind {
        MachineArtifactKind::OcrRaw => "zotron-ocr.raw.zip",
        MachineArtifactKind::Blocks => "zotron-blocks.jsonl",
        MachineArtifactKind::Chunks => "zotron-chunks.jsonl",
        MachineArtifactKind::EmbeddingVectors => "zotron-embed.npz",
        MachineArtifactKind::OcrNativeMarkdown => "zotron-ocr.native.md",
        MachineArtifactKind::OcrNativeAssets => "zotron-ocr.assets.json",
    };
    let attachments_dir = store_root
        .as_ref()
        .join("items")
        .join(item_key)
        .join("attachments");
    let Ok(entries) = fs::read_dir(attachments_dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry
            .path()
            .join(legacy_file_name)
            .try_exists()
            .unwrap_or(false)
    })
}
