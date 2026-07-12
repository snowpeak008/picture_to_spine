use super::{
    atlas_manifest::atlas_input_bytes,
    materialize::materialize_approved_png,
    psd::{PsdLayer, minimal_psd_bytes},
    rig_ir::rig_ir_bytes,
    spine42::spine_json_bytes,
};
use f2s_application::export::{
    preflight::{preflight, validate_relative_png_path},
    publish_snapshot::PublishSnapshot,
};
use f2s_domain::{
    ACTION_KEYS,
    canonical::canonical_bytes,
    motion::prompt::{PromptEntry, PromptPack},
};
use image::{ImageFormat, ImageReader};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    path::{Component, Path, PathBuf},
};

const TARGET_SPINE_PATCH: &str = "4.2.43";
const CONTRACT_VERIFIED: &str = "CONTRACT_VERIFIED";
const EXPORTED_UNVERIFIED: &str = "EXPORTED_UNVERIFIED";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentBytes {
    pub attachment_id: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportCommit {
    pub directory: PathBuf,
    pub status: String,
    pub checksums: BTreeMap<String, String>,
    pub external_editor_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactInventoryEntry {
    path: String,
    role: &'static str,
    format: &'static str,
    status: &'static str,
    note: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChecksumPolicy {
    algorithm: &'static str,
    manifest_path: &'static str,
    coverage: &'static str,
    self_excluded: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompatibilityManifest<'a> {
    schema_version: &'static str,
    export_id: &'a str,
    project_revision: u64,
    capability_id: &'a str,
    target_patch: &'static str,
    package_status: &'static str,
    contract_status_meaning: &'static str,
    spine_editor_round_trip_status: &'static str,
    release_ready: bool,
    prompt_pack_sha256: String,
    checksum_policy: ChecksumPolicy,
    artifacts: Vec<ArtifactInventoryEntry>,
    intentionally_not_produced: [&'static str; 3],
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn ensure_tree_not_reparse(root: &Path, target: &Path) -> Result<(), String> {
    if !target.starts_with(root) {
        return Err("export path escaped approved root".into());
    }
    let mut cursor = root.to_path_buf();
    if metadata_is_reparse(&fs::symlink_metadata(&cursor).map_err(|e| e.to_string())?) {
        return Err("export root cannot be a reparse point".into());
    }
    if let Ok(relative) = target.strip_prefix(root) {
        for component in relative.components() {
            let Component::Normal(part) = component else {
                return Err("export target contains traversal".into());
            };
            cursor.push(part);
            if cursor.exists()
                && metadata_is_reparse(&fs::symlink_metadata(&cursor).map_err(|e| e.to_string())?)
            {
                return Err("reparse point inside export tree".into());
            }
        }
    }
    Ok(())
}

fn write_file(root: &Path, relative: &str, bytes: &[u8]) -> Result<String, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|v| !matches!(v, Component::Normal(_)))
    {
        return Err("writer received unsafe relative path".into());
    }
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        ensure_tree_not_reparse(root, parent)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn valid_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_prompt_pack(prompt_pack: &PromptPack) -> Result<(), String> {
    prompt_pack.validate()?;
    if prompt_pack.pack_id.trim().is_empty()
        || prompt_pack.revision == 0
        || !valid_lower_sha256(&prompt_pack.motion_revision_hash)
    {
        return Err("PromptPack identity or motion hash is invalid".into());
    }
    let allowed = ACTION_KEYS.iter().copied().collect::<BTreeSet<_>>();
    let mut covered = BTreeSet::new();
    let mut asset_ids = BTreeSet::new();
    for entry in &prompt_pack.entries {
        if !allowed.contains(entry.action_key.as_str())
            || entry.asset_spec_id.trim().is_empty()
            || entry.pose_key.trim().is_empty()
            || entry.positive.trim().is_empty()
            || entry.negative.trim().is_empty()
            || !asset_ids.insert(entry.asset_spec_id.as_str())
        {
            return Err("PromptPack contains an invalid or duplicate entry".into());
        }
        covered.insert(entry.action_key.as_str());
    }
    if covered != allowed {
        return Err("PromptPack must cover the canonical ten-action set".into());
    }
    Ok(())
}

fn prompt_pack_json_bytes(prompt_pack: &PromptPack) -> Result<Vec<u8>, String> {
    validate_prompt_pack(prompt_pack)?;
    canonical_bytes(prompt_pack).map_err(|error| error.to_string())
}

fn normalized_prompt(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn inline_code(value: &str) -> String {
    value.replace(['\r', '\n'], " ").replace('`', "\\`")
}

fn fenced_text(value: &str) -> String {
    let value = normalized_prompt(value);
    let longest_run = value
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest_run.saturating_add(1).max(3));
    format!("{fence}text\n{value}\n{fence}\n")
}

fn prompt_entry_order(entry: &PromptEntry) -> (usize, &str, &str) {
    (
        ACTION_KEYS
            .iter()
            .position(|action| *action == entry.action_key)
            .unwrap_or(usize::MAX),
        entry.pose_key.as_str(),
        entry.asset_spec_id.as_str(),
    )
}

fn prompt_pack_markdown_bytes(prompt_pack: &PromptPack) -> Result<Vec<u8>, String> {
    validate_prompt_pack(prompt_pack)?;
    let mut entries = prompt_pack.entries.iter().collect::<Vec<_>>();
    entries.sort_by_key(|entry| prompt_entry_order(entry));
    let mut markdown = format!(
        "# AI Action Keyframe Prompt Pack\n\n- Pack ID: `{}`\n- Revision: {}\n- Style revision: {}\n- Style SHA-256: `{}`\n- Motion SHA-256: `{}`\n- Provider profile: `{}`\n- Network calls made: {}\n\n",
        inline_code(&prompt_pack.pack_id),
        prompt_pack.revision,
        prompt_pack.style_revision,
        prompt_pack.style_sha256,
        prompt_pack.motion_revision_hash,
        inline_code(&prompt_pack.provider_profile),
        prompt_pack.network_calls_made,
    );
    for (index, entry) in entries.into_iter().enumerate() {
        markdown.push_str(&format!(
            "## Entry {}\n\n- Action: `{}`\n- Pose: `{}`\n- Asset spec: `{}`\n\nPositive prompt:\n\n{}\nNegative prompt:\n\n{}\n",
            index + 1,
            inline_code(&entry.action_key),
            inline_code(&entry.pose_key),
            inline_code(&entry.asset_spec_id),
            fenced_text(&entry.positive),
            fenced_text(&entry.negative),
        ));
    }
    Ok(markdown.into_bytes())
}

fn inventory_entry(
    path: impl Into<String>,
    role: &'static str,
    format: &'static str,
    note: &'static str,
) -> ArtifactInventoryEntry {
    ArtifactInventoryEntry {
        path: path.into(),
        role,
        format,
        status: CONTRACT_VERIFIED,
        note,
    }
}

fn compatibility_manifest_bytes(
    snapshot: &PublishSnapshot,
    prompt_pack: &PromptPack,
) -> Result<Vec<u8>, String> {
    let mut artifacts = vec![
        inventory_entry(
            "rig-ir.json",
            "rig-intermediate-representation",
            "F2S Rig IR JSON",
            "Validated against the built-in Rig IR contract.",
        ),
        inventory_entry(
            "atlas-input-manifest.json",
            "atlas-packing-input",
            "F2S atlas input JSON",
            "Open packing input only; this file is not a Spine .atlas file.",
        ),
        inventory_entry(
            "character.spine.json",
            "spine-json",
            "Spine JSON 4.2.43",
            "Validated against the pinned serializer contract; no editor round trip is asserted.",
        ),
        inventory_entry(
            "character.psd",
            "layered-source",
            "minimal layered PSD",
            "Minimal built-in PSD profile, not a claim of full Photoshop feature support.",
        ),
        inventory_entry(
            "prompt-pack.json",
            "ai-keyframe-prompts",
            "canonical JSON",
            "Offline PromptPack source of truth.",
        ),
        inventory_entry(
            "prompt-pack.md",
            "ai-keyframe-prompts-readable",
            "Markdown",
            "Human-readable projection of prompt-pack.json.",
        ),
    ];
    let mut png_paths = snapshot
        .attachments
        .iter()
        .map(|attachment| attachment.logical_png_path.as_str())
        .collect::<Vec<_>>();
    png_paths.sort_unstable();
    artifacts.extend(png_paths.into_iter().map(|path| {
        inventory_entry(
            path,
            "approved-attachment",
            "PNG",
            "Hash and declared dimensions verified during package commit.",
        )
    }));
    artifacts.extend([
        inventory_entry(
            "compatibility-manifest.json",
            "compatibility-evidence",
            "canonical JSON",
            "Declares contract-level evidence without claiming Spine Editor verification.",
        ),
        inventory_entry(
            "checksums.sha256",
            "integrity-manifest",
            "SHA-256 text manifest",
            "Covers every package file except itself to avoid a recursive checksum.",
        ),
    ]);
    let manifest = CompatibilityManifest {
        schema_version: "1.1.0",
        export_id: &snapshot.export_id,
        project_revision: snapshot.project_revision,
        capability_id: &snapshot.capability_id,
        target_patch: TARGET_SPINE_PATCH,
        package_status: EXPORTED_UNVERIFIED,
        contract_status_meaning: "The built-in writer and its output contract passed; Spine Editor or runtime round-trip verification has not been performed.",
        spine_editor_round_trip_status: EXPORTED_UNVERIFIED,
        release_ready: false,
        prompt_pack_sha256: format!("{:x}", Sha256::digest(prompt_pack_json_bytes(prompt_pack)?)),
        checksum_policy: ChecksumPolicy {
            algorithm: "SHA-256",
            manifest_path: "checksums.sha256",
            coverage: "Every regular package file except checksums.sha256, including compatibility-manifest.json.",
            self_excluded: true,
        },
        artifacts,
        intentionally_not_produced: ["Spine .atlas", "Spine .spine", "Spine .skel"],
    };
    canonical_bytes(&manifest).map_err(|error| error.to_string())
}

fn validate_attachment_inputs(
    snapshot: &PublishSnapshot,
    attachments: &[AttachmentBytes],
) -> Result<(), String> {
    if snapshot.attachments.len() != attachments.len() {
        return Err("attachment byte set must exactly match the publish snapshot".into());
    }
    let mut declared_ids = BTreeSet::new();
    let mut supplied_ids = BTreeSet::new();
    for source in attachments {
        if !supplied_ids.insert(source.attachment_id.as_str()) {
            return Err("duplicate supplied attachment id".into());
        }
    }
    for attachment in &snapshot.attachments {
        if attachment.logical_png_path.contains('\\')
            || attachment.logical_png_path.chars().any(char::is_control)
            || !declared_ids.insert(attachment.attachment_id.as_str())
        {
            return Err("invalid or duplicate declared attachment".into());
        }
        let source = attachments
            .iter()
            .find(|source| source.attachment_id == attachment.attachment_id)
            .ok_or_else(|| format!("missing attachment bytes: {}", attachment.attachment_id))?;
        let reader = ImageReader::with_format(Cursor::new(&source.bytes), ImageFormat::Png);
        let dimensions = reader
            .into_dimensions()
            .map_err(|error| format!("invalid attachment PNG header: {error}"))?;
        if dimensions != (attachment.width, attachment.height) {
            return Err(format!(
                "attachment dimensions mismatch: {}",
                attachment.attachment_id
            ));
        }
    }
    if declared_ids != supplied_ids {
        return Err("attachment byte set contains undeclared ids".into());
    }
    Ok(())
}

fn render_checksum_manifest(sums: &BTreeMap<String, String>) -> String {
    sums.iter()
        .map(|(path, hash)| format!("{hash}  {path}\n"))
        .collect()
}

fn collect_package_files(root: &Path, directory: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = BTreeSet::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
            if metadata_is_reparse(&metadata) {
                return Err("reparse point found inside staged export".into());
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .components()
                    .map(|component| {
                        component
                            .as_os_str()
                            .to_str()
                            .ok_or_else(|| "export path must be Unicode".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join("/");
                files.insert(relative);
            } else {
                return Err("non-regular entry found inside staged export".into());
            }
        }
    }
    Ok(files)
}

fn verify_staged_package(staging: &Path, sums: &BTreeMap<String, String>) -> Result<(), String> {
    let expected_checksum_bytes = render_checksum_manifest(sums).into_bytes();
    let actual_checksum_bytes = fs::read(staging.join("checksums.sha256"))
        .map_err(|error| format!("cannot read checksum manifest: {error}"))?;
    if actual_checksum_bytes != expected_checksum_bytes {
        return Err("checksum manifest is not the canonical package checksum set".into());
    }
    for (path, expected_hash) in sums {
        let bytes = fs::read(staging.join(path))
            .map_err(|error| format!("cannot verify package artifact {path}: {error}"))?;
        let actual_hash = format!("{:x}", Sha256::digest(bytes));
        if &actual_hash != expected_hash {
            return Err(format!("package artifact hash mismatch: {path}"));
        }
    }
    let mut expected_files = sums.keys().cloned().collect::<BTreeSet<_>>();
    expected_files.insert("checksums.sha256".into());
    if collect_package_files(staging, staging)? != expected_files {
        return Err("staged export file inventory does not match its checksum manifest".into());
    }
    let compatibility: serde_json::Value = serde_json::from_slice(
        &fs::read(staging.join("compatibility-manifest.json"))
            .map_err(|error| format!("cannot read compatibility manifest: {error}"))?,
    )
    .map_err(|error| format!("invalid compatibility manifest: {error}"))?;
    if compatibility["targetPatch"] != TARGET_SPINE_PATCH
        || compatibility["packageStatus"] != EXPORTED_UNVERIFIED
        || compatibility["spineEditorRoundTripStatus"] != EXPORTED_UNVERIFIED
        || compatibility["releaseReady"] != false
    {
        return Err("compatibility manifest overstates or misstates package status".into());
    }
    let declared_files = compatibility["artifacts"]
        .as_array()
        .ok_or("compatibility manifest artifact inventory is missing")?
        .iter()
        .map(|artifact| {
            if artifact["status"] != CONTRACT_VERIFIED {
                return Err("compatibility artifact has an unsupported status".to_string());
            }
            artifact["path"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "compatibility artifact path is missing".to_string())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if declared_files != expected_files {
        return Err("compatibility artifact inventory does not match package files".into());
    }
    Ok(())
}

pub fn commit_open_export(
    snapshot: &PublishSnapshot,
    attachments: &[AttachmentBytes],
    psd_layers: &[PsdLayer],
    canvas: (u32, u32),
    prompt_pack: &PromptPack,
    export_root: &Path,
) -> Result<ExportCommit, String> {
    let report = preflight(snapshot);
    if !report.passed {
        return Err(format!(
            "export preflight failed: {}",
            report.errors.join(",")
        ));
    }
    validate_prompt_pack(prompt_pack)?;
    validate_attachment_inputs(snapshot, attachments)?;
    if psd_layers.len() != snapshot.attachments.len() {
        return Err("PSD layer set must exactly match exported attachments".into());
    }
    for (attachment, layer) in snapshot.attachments.iter().zip(psd_layers) {
        if (layer.width, layer.height) != (attachment.width, attachment.height)
            || layer.origin_x != 0
            || layer.origin_y != 0
        {
            return Err(format!(
                "PSD layer geometry differs from attachment {}",
                attachment.attachment_id
            ));
        }
    }
    fs::create_dir_all(export_root).map_err(|e| e.to_string())?;
    let root = export_root.canonicalize().map_err(|e| e.to_string())?;
    ensure_tree_not_reparse(&root, &root)?;
    let final_dir = root.join(&snapshot.export_id);
    let staging = root.join(format!(".{}.f2s-staging", snapshot.export_id));
    if final_dir.exists() || staging.exists() {
        return Err(
            "export or staging id already exists; immutable outputs are never overwritten".into(),
        );
    }
    fs::create_dir(&staging).map_err(|e| e.to_string())?;
    ensure_tree_not_reparse(&root, &staging)?;
    let result = (|| {
        let mut sums = BTreeMap::new();
        sums.insert(
            "rig-ir.json".into(),
            write_file(&staging, "rig-ir.json", &rig_ir_bytes(snapshot)?)?,
        );
        sums.insert(
            "atlas-input-manifest.json".into(),
            write_file(
                &staging,
                "atlas-input-manifest.json",
                &atlas_input_bytes(snapshot)?,
            )?,
        );
        sums.insert(
            "character.spine.json".into(),
            write_file(
                &staging,
                "character.spine.json",
                &spine_json_bytes(snapshot)?,
            )?,
        );
        sums.insert(
            "character.psd".into(),
            write_file(
                &staging,
                "character.psd",
                &minimal_psd_bytes(canvas.0, canvas.1, psd_layers)?,
            )?,
        );
        sums.insert(
            "prompt-pack.json".into(),
            write_file(
                &staging,
                "prompt-pack.json",
                &prompt_pack_json_bytes(prompt_pack)?,
            )?,
        );
        sums.insert(
            "prompt-pack.md".into(),
            write_file(
                &staging,
                "prompt-pack.md",
                &prompt_pack_markdown_bytes(prompt_pack)?,
            )?,
        );
        for attachment in &snapshot.attachments {
            validate_relative_png_path(&attachment.logical_png_path)?;
            let source = attachments
                .iter()
                .find(|v| v.attachment_id == attachment.attachment_id)
                .ok_or_else(|| format!("missing attachment bytes: {}", attachment.attachment_id))?;
            let path = staging.join(&attachment.logical_png_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                ensure_tree_not_reparse(&staging, parent)?;
            }
            let hash = materialize_approved_png(&source.bytes, &attachment.source_sha256, &path)?;
            sums.insert(attachment.logical_png_path.clone(), hash);
        }
        sums.insert(
            "compatibility-manifest.json".into(),
            write_file(
                &staging,
                "compatibility-manifest.json",
                &compatibility_manifest_bytes(snapshot, prompt_pack)?,
            )?,
        );
        let checksums = render_checksum_manifest(&sums);
        write_file(&staging, "checksums.sha256", checksums.as_bytes())?;
        verify_staged_package(&staging, &sums)?;
        Ok(sums)
    })();
    match result {
        Ok(sums) => {
            ensure_tree_not_reparse(&root, &staging)?;
            fs::rename(&staging, &final_dir).map_err(|e| e.to_string())?;
            ensure_tree_not_reparse(&root, &final_dir)?;
            Ok(ExportCommit {
                directory: final_dir,
                status: EXPORTED_UNVERIFIED.into(),
                checksums: sums,
                external_editor_status: EXPORTED_UNVERIFIED.into(),
            })
        }
        Err(error) => {
            if ensure_tree_not_reparse(&root, &staging).is_ok() {
                let _ = fs::remove_dir_all(&staging);
            }
            Err(error)
        }
    }
}
