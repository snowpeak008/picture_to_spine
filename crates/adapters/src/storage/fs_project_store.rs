use super::ntfs_atomic::{recover_atomic_target, write_atomic};
use f2s_application::ports::ProjectStore;
use f2s_domain::{canonical::canonical_bytes, storage::ProjectHead};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const INTEGRITY_SCHEMA_VERSION: &str = "1.0.0";

struct IntegrityConfig {
    key_id: String,
    key: [u8; 32],
}

impl Drop for IntegrityConfig {
    fn drop(&mut self) {
        // Best-effort process-memory hygiene. DPAPI protects the persisted copy;
        // the unwrapped HMAC key is only needed while this store value is alive.
        self.key.fill(0);
    }
}

/// Filesystem-backed project store.
///
/// `new` intentionally retains the legacy unsigned behavior for tests and
/// migration tooling. Production callers must use `new_with_integrity_key`.
pub struct FsProjectStore {
    root: PathBuf,
    integrity: Option<IntegrityConfig>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HeadMacPayload<'a> {
    schema_version: &'a str,
    project_id: &'a str,
    head_revision: u64,
    manifest_sha256: &'a str,
    previous_manifest_sha256: &'a Option<String>,
    previous_head_mac: &'a Option<String>,
    key_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IntegrityAnchor {
    schema_version: String,
    project_id: String,
    highest_revision: u64,
    manifest_sha256: String,
    head_mac: String,
    key_id: String,
    anchor_mac: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnchorMacPayload<'a> {
    schema_version: &'a str,
    project_id: &'a str,
    highest_revision: u64,
    manifest_sha256: &'a str,
    head_mac: &'a str,
    key_id: &'a str,
}

impl FsProjectStore {
    /// Construct the unsigned legacy store. This mode does not provide
    /// tamper or rollback detection and must not be used by the production
    /// desktop host.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            integrity: None,
        }
    }

    /// Construct a fail-closed production store using a caller-managed local
    /// 256-bit integrity key. The host is responsible for loading this key
    /// from DPAPI CurrentUser protected storage and for keeping `key_id`
    /// stable for the lifetime of the project store.
    pub fn new_with_integrity_key(
        root: impl AsRef<Path>,
        key_id: impl Into<String>,
        key: [u8; 32],
    ) -> Result<Self, String> {
        let key_id = key_id.into();
        if key_id.is_empty()
            || key_id.len() > 128
            || !key_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err("integrity key id must be 1-128 safe ASCII characters".into());
        }
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            integrity: Some(IntegrityConfig { key_id, key }),
        })
    }

    fn project(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty() || id.contains("..") || id.contains(['/', '\\']) {
            return Err("invalid project id".into());
        }
        Ok(self.root.join(id))
    }

    fn anchor_path(&self, project_id: &str) -> Result<PathBuf, String> {
        self.project(project_id)?;
        Ok(self
            .root
            .join("security")
            .join("anchors")
            .join(format!("{project_id}.json")))
    }

    fn revision_path(&self, project_id: &str, revision: u64) -> Result<PathBuf, String> {
        Ok(self
            .project(project_id)?
            .join("revisions")
            .join(format!("{revision}.json")))
    }

    fn signed_revision_path(&self, project_id: &str, revision: u64) -> Result<PathBuf, String> {
        Ok(self
            .project(project_id)?
            .join("revisions")
            .join(format!("{revision}.head.json")))
    }

    fn read_head_file(&self, project_id: &str) -> Result<Option<ProjectHead>, String> {
        let path = self.project(project_id)?.join("head.json");
        recover_atomic_target(&path)?;
        if !path.try_exists().map_err(|error| error.to_string())? {
            return Ok(None);
        }
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| format!("invalid project head: {error}"))
    }

    fn verify_head_mac(&self, head: &ProjectHead, config: &IntegrityConfig) -> Result<(), String> {
        if head.key_id.as_deref() != Some(config.key_id.as_str()) {
            return Err("project head integrity key id mismatch".into());
        }
        if !is_lower_hex_sha256(&head.manifest_sha256) {
            return Err("project head manifest hash is invalid".into());
        }
        if head
            .previous_head_sha256
            .as_deref()
            .is_some_and(|value| !is_lower_hex_sha256(value))
            || head
                .previous_head_mac
                .as_deref()
                .is_some_and(|value| !is_lower_hex_sha256(value))
        {
            return Err("project head chain hash is invalid".into());
        }
        let supplied = decode_hex_32(
            head.head_mac
                .as_deref()
                .ok_or("unsigned project head rejected by integrity store")?,
        )?;
        let expected = head_mac(head, config)?;
        if !constant_time_eq_32(&supplied, &expected) {
            return Err("project head integrity MAC mismatch".into());
        }
        Ok(())
    }

    fn verify_revision_chain(
        &self,
        current: &ProjectHead,
        config: &IntegrityConfig,
    ) -> Result<(), String> {
        let mut cursor = current.clone();
        loop {
            self.verify_head_mac(&cursor, config)?;
            let sidecar_path =
                self.signed_revision_path(&cursor.project_id, cursor.head_revision)?;
            let sidecar_bytes = fs::read(&sidecar_path).map_err(|error| {
                format!(
                    "signed revision sidecar missing or unreadable ({}): {error}",
                    sidecar_path.display()
                )
            })?;
            let sidecar: ProjectHead = serde_json::from_slice(&sidecar_bytes)
                .map_err(|error| format!("invalid signed revision sidecar: {error}"))?;
            if !constant_time_head_eq(&sidecar, &cursor)? {
                return Err("project head does not match immutable signed revision".into());
            }

            let revision_path = self.revision_path(&cursor.project_id, cursor.head_revision)?;
            let revision_bytes = fs::read(&revision_path).map_err(|error| {
                format!(
                    "immutable project revision missing or unreadable ({}): {error}",
                    revision_path.display()
                )
            })?;
            let revision_hash = sha256_hex(&revision_bytes);
            if !constant_time_hex_eq(&revision_hash, &cursor.manifest_sha256)? {
                return Err("immutable project revision manifest hash mismatch".into());
            }

            if cursor.head_revision == 0 {
                if cursor.previous_head_sha256.is_some() || cursor.previous_head_mac.is_some() {
                    return Err("initial project head contains a previous chain link".into());
                }
                break;
            }

            let previous_revision = cursor.head_revision - 1;
            let previous_path = self.signed_revision_path(&cursor.project_id, previous_revision)?;
            let previous_bytes = fs::read(&previous_path).map_err(|error| {
                format!(
                    "previous signed revision sidecar missing ({}): {error}",
                    previous_path.display()
                )
            })?;
            let previous: ProjectHead = serde_json::from_slice(&previous_bytes)
                .map_err(|error| format!("invalid previous signed revision sidecar: {error}"))?;
            self.verify_head_mac(&previous, config)?;
            if previous.project_id != cursor.project_id
                || previous.head_revision != previous_revision
                || cursor.previous_head_sha256.as_deref() != Some(previous.manifest_sha256.as_str())
                || cursor.previous_head_mac.as_deref() != previous.head_mac.as_deref()
            {
                return Err("project revision chain is broken or forked".into());
            }
            cursor = previous;
        }
        Ok(())
    }

    fn load_anchor(
        &self,
        project_id: &str,
        config: &IntegrityConfig,
    ) -> Result<Option<IntegrityAnchor>, String> {
        let path = self.anchor_path(project_id)?;
        recover_atomic_target(&path)?;
        if !path.try_exists().map_err(|error| error.to_string())? {
            return Ok(None);
        }
        let bytes = fs::read(&path).map_err(|error| error.to_string())?;
        let anchor: IntegrityAnchor = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid project integrity anchor: {error}"))?;
        if anchor.schema_version != INTEGRITY_SCHEMA_VERSION
            || anchor.project_id != project_id
            || anchor.key_id != config.key_id
            || !is_lower_hex_sha256(&anchor.manifest_sha256)
            || !is_lower_hex_sha256(&anchor.head_mac)
        {
            return Err("project integrity anchor identity or format mismatch".into());
        }
        let supplied = decode_hex_32(&anchor.anchor_mac)?;
        let expected = anchor_mac(&anchor, config)?;
        if !constant_time_eq_32(&supplied, &expected) {
            return Err("project integrity anchor MAC mismatch".into());
        }
        Ok(Some(anchor))
    }

    fn read_verified_signed_head(
        &self,
        project_id: &str,
        revision: u64,
        config: &IntegrityConfig,
    ) -> Result<ProjectHead, String> {
        let path = self.signed_revision_path(project_id, revision)?;
        let bytes = fs::read(&path).map_err(|error| {
            format!(
                "signed revision sidecar missing or unreadable ({}): {error}",
                path.display()
            )
        })?;
        let head: ProjectHead = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid signed revision sidecar: {error}"))?;
        if head.project_id != project_id || head.head_revision != revision {
            return Err("signed revision sidecar identity mismatch".into());
        }
        self.verify_revision_chain(&head, config)?;
        Ok(head)
    }

    fn anchor_matches_head(
        anchor: &IntegrityAnchor,
        head: &ProjectHead,
        config: &IntegrityConfig,
    ) -> Result<bool, String> {
        let Some(head_mac_value) = head.head_mac.as_deref() else {
            return Ok(false);
        };
        Ok(anchor.schema_version == INTEGRITY_SCHEMA_VERSION
            && anchor.project_id == head.project_id
            && anchor.highest_revision == head.head_revision
            && anchor.key_id == config.key_id
            && constant_time_hex_eq(&anchor.manifest_sha256, &head.manifest_sha256)?
            && constant_time_hex_eq(&anchor.head_mac, head_mac_value)?)
    }

    fn head_directly_extends(base: &ProjectHead, candidate: &ProjectHead) -> bool {
        candidate.project_id == base.project_id
            && base.head_revision.checked_add(1) == Some(candidate.head_revision)
            && candidate.previous_head_sha256.as_deref() == Some(base.manifest_sha256.as_str())
            && candidate.previous_head_mac.as_deref() == base.head_mac.as_deref()
            && candidate.key_id == base.key_id
    }

    fn build_anchor(
        head: &ProjectHead,
        config: &IntegrityConfig,
    ) -> Result<IntegrityAnchor, String> {
        let mut anchor = IntegrityAnchor {
            schema_version: INTEGRITY_SCHEMA_VERSION.into(),
            project_id: head.project_id.clone(),
            highest_revision: head.head_revision,
            manifest_sha256: head.manifest_sha256.clone(),
            head_mac: head
                .head_mac
                .clone()
                .ok_or("sealed project head has no MAC")?,
            key_id: config.key_id.clone(),
            anchor_mac: String::new(),
        };
        anchor.anchor_mac = encode_hex(&anchor_mac(&anchor, config)?);
        Ok(anchor)
    }

    /// Publishes a head that has already been written as an immutable, valid
    /// signed sidecar. The high-water anchor deliberately moves first. A
    /// process death between these writes therefore leaves an anchor-ahead
    /// state that can only roll the head forward to the exact signed sidecar.
    fn publish_signed_head(
        &self,
        head: &ProjectHead,
        config: &IntegrityConfig,
    ) -> Result<(), String> {
        self.verify_revision_chain(head, config)?;
        let anchor = Self::build_anchor(head, config)?;
        write_atomic(
            &self.anchor_path(&head.project_id)?,
            &canonical_bytes(&anchor).map_err(|error| error.to_string())?,
        )?;
        write_atomic(
            &self.project(&head.project_id)?.join("head.json"),
            &canonical_bytes(head).map_err(|error| error.to_string())?,
        )
    }

    fn recover_signed_successors(
        &self,
        mut current: ProjectHead,
        config: &IntegrityConfig,
    ) -> Result<ProjectHead, String> {
        loop {
            let Some(next_revision) = current.head_revision.checked_add(1) else {
                return Ok(current);
            };
            let sidecar = self.signed_revision_path(&current.project_id, next_revision)?;
            if !sidecar.try_exists().map_err(|error| error.to_string())? {
                return Ok(current);
            }
            let candidate =
                self.read_verified_signed_head(&current.project_id, next_revision, config)?;
            if !Self::head_directly_extends(&current, &candidate) {
                return Err("signed recovery sidecar does not extend the current head".into());
            }
            self.publish_signed_head(&candidate, config)?;
            current = candidate;
        }
    }

    fn load_secure_head(
        &self,
        project_id: &str,
        config: &IntegrityConfig,
    ) -> Result<Option<ProjectHead>, String> {
        let head = self.read_head_file(project_id)?;
        let anchor = self.load_anchor(project_id, config)?;
        let current = match (head, anchor) {
            (None, None) => {
                // A revision without a signed sidecar is an uncommitted orphan.
                // A genesis sidecar, however, is an authenticated commit that
                // died before either projection was published.
                let genesis_path = self.signed_revision_path(project_id, 0)?;
                if !genesis_path
                    .try_exists()
                    .map_err(|error| error.to_string())?
                {
                    return Ok(None);
                }
                if self
                    .signed_revision_path(project_id, 1)?
                    .try_exists()
                    .map_err(|error| error.to_string())?
                {
                    return Err(
                        "project high-water projections were deleted; refusing genesis rollback"
                            .into(),
                    );
                }
                let genesis = self.read_verified_signed_head(project_id, 0, config)?;
                self.publish_signed_head(&genesis, config)?;
                genesis
            }
            (None, Some(anchor)) => {
                let anchored =
                    self.read_verified_signed_head(project_id, anchor.highest_revision, config)?;
                if !Self::anchor_matches_head(&anchor, &anchored, config)? {
                    return Err("integrity anchor does not match its signed revision".into());
                }
                self.publish_signed_head(&anchored, config)?;
                anchored
            }
            (Some(head), None) => {
                self.verify_revision_chain(&head, config)?;
                if head.head_revision != 0
                    || self
                        .signed_revision_path(project_id, 1)?
                        .try_exists()
                        .map_err(|error| error.to_string())?
                {
                    return Err(
                        "project integrity anchor deletion or high-water rollback detected".into(),
                    );
                }
                self.publish_signed_head(&head, config)?;
                head
            }
            (Some(head), Some(anchor)) => {
                self.verify_revision_chain(&head, config)?;
                if head.head_revision == anchor.highest_revision {
                    if !Self::anchor_matches_head(&anchor, &head, config)? {
                        return Err(
                            "project head fork detected at the integrity high-water revision"
                                .into(),
                        );
                    }
                    head
                } else if head.head_revision.checked_sub(1) == Some(anchor.highest_revision) {
                    // Compatibility recovery for the former head-before-anchor
                    // ordering. The old anchor must itself identify a valid
                    // signed base, and the head must extend it exactly once.
                    let base = self.read_verified_signed_head(
                        project_id,
                        anchor.highest_revision,
                        config,
                    )?;
                    if !Self::anchor_matches_head(&anchor, &base, config)?
                        || !Self::head_directly_extends(&base, &head)
                    {
                        return Err("head-ahead recovery chain is invalid".into());
                    }
                    self.publish_signed_head(&head, config)?;
                    head
                } else if anchor.highest_revision.checked_sub(1) == Some(head.head_revision) {
                    let anchored = self.read_verified_signed_head(
                        project_id,
                        anchor.highest_revision,
                        config,
                    )?;
                    if !Self::anchor_matches_head(&anchor, &anchored, config)?
                        || !Self::head_directly_extends(&head, &anchored)
                    {
                        return Err("anchor-ahead recovery chain is invalid".into());
                    }
                    self.publish_signed_head(&anchored, config)?;
                    anchored
                } else {
                    return Err(
                        "project head and integrity anchor differ by more than one revision".into(),
                    );
                }
            }
        };
        self.recover_signed_successors(current, config).map(Some)
    }

    fn validate_compare_and_swap(
        requested: &ProjectHead,
        current: Option<&ProjectHead>,
    ) -> Result<(), String> {
        match current {
            None => {
                if requested.head_revision != 0 || requested.previous_head_sha256.is_some() {
                    return Err(
                        "initial project commit must be revision 0 with no predecessor".into(),
                    );
                }
            }
            Some(current) => {
                let expected_revision = current
                    .head_revision
                    .checked_add(1)
                    .ok_or("project revision overflow")?;
                if requested.head_revision != expected_revision
                    || requested.previous_head_sha256.as_deref()
                        != Some(current.manifest_sha256.as_str())
                {
                    return Err("project head compare-and-swap conflict".into());
                }
            }
        }
        Ok(())
    }

    fn commit_legacy(&self, head: &ProjectHead, manifest: &[u8]) -> Result<ProjectHead, String> {
        let current = self.read_head_file(&head.project_id)?;
        Self::validate_compare_and_swap(head, current.as_ref())?;
        if sha256_hex(manifest) != head.manifest_sha256 {
            return Err("project manifest does not match requested head hash".into());
        }
        let revision = self.revision_path(&head.project_id, head.head_revision)?;
        write_immutable(&revision, manifest, false)?;
        write_atomic(
            &self.project(&head.project_id)?.join("head.json"),
            &serde_json::to_vec(head).map_err(|error| error.to_string())?,
        )?;
        Ok(head.clone())
    }

    fn prepare_secure_revision_slot(
        &self,
        project_id: &str,
        revision: u64,
        manifest: &[u8],
    ) -> Result<(), String> {
        let revision_path = self.revision_path(project_id, revision)?;
        if !revision_path
            .try_exists()
            .map_err(|error| error.to_string())?
        {
            return Ok(());
        }
        let sidecar_path = self.signed_revision_path(project_id, revision)?;
        if sidecar_path
            .try_exists()
            .map_err(|error| error.to_string())?
        {
            return Err("signed revision slot already exists; refusing replacement".into());
        }
        let existing = fs::read(&revision_path).map_err(|error| error.to_string())?;
        if existing == manifest {
            // Safe retry after a death between revision and sidecar writes.
            return Ok(());
        }

        // A differently hashed revision without a signed sidecar was never a
        // committed project state. Preserve it for incident inspection, but
        // remove it from the immutable revision namespace before retrying.
        let quarantine = self
            .project(project_id)?
            .join("recovery")
            .join("unsigned-orphan-revisions");
        fs::create_dir_all(&quarantine).map_err(|error| error.to_string())?;
        let isolated = quarantine.join(format!(
            "revision-{revision}-{}-{}.json",
            sha256_hex(&existing),
            Uuid::new_v4().simple()
        ));
        fs::rename(&revision_path, &isolated).map_err(|error| {
            format!(
                "cannot isolate unsigned orphan revision {} -> {}: {error}",
                revision_path.display(),
                isolated.display()
            )
        })
    }

    fn commit_secure(
        &self,
        head: &ProjectHead,
        manifest: &[u8],
        config: &IntegrityConfig,
    ) -> Result<ProjectHead, String> {
        if head.key_id.is_some() || head.previous_head_mac.is_some() || head.head_mac.is_some() {
            return Err("caller must not supply a project head integrity seal".into());
        }
        if !is_lower_hex_sha256(&head.manifest_sha256)
            || sha256_hex(manifest) != head.manifest_sha256
        {
            return Err("project manifest does not match requested head hash".into());
        }

        let current = self.load_secure_head(&head.project_id, config)?;
        if let Some(current) = current.as_ref() {
            if head.schema_version == current.schema_version
                && head.project_id == current.project_id
                && head.head_revision == current.head_revision
                && head.manifest_sha256 == current.manifest_sha256
                && head.previous_head_sha256 == current.previous_head_sha256
            {
                // Idempotent retry after recovery published the exact signed
                // revision before the caller observed the original success.
                return Ok(current.clone());
            }
        }
        Self::validate_compare_and_swap(head, current.as_ref())?;

        let mut sealed = head.clone();
        sealed.key_id = Some(config.key_id.clone());
        sealed.previous_head_mac = current.as_ref().and_then(|value| value.head_mac.clone());
        sealed.head_mac = Some(encode_hex(&head_mac(&sealed, config)?));

        self.prepare_secure_revision_slot(&sealed.project_id, sealed.head_revision, manifest)?;
        let revision = self.revision_path(&sealed.project_id, sealed.head_revision)?;
        write_immutable(&revision, manifest, true)?;
        let sidecar = self.signed_revision_path(&sealed.project_id, sealed.head_revision)?;
        write_immutable(
            &sidecar,
            &canonical_bytes(&sealed).map_err(|error| error.to_string())?,
            true,
        )?;
        self.publish_signed_head(&sealed, config)?;
        Ok(sealed)
    }
}

impl ProjectStore for FsProjectStore {
    fn load_head(&self, project_id: &str) -> Result<Option<ProjectHead>, String> {
        match &self.integrity {
            Some(config) => self.load_secure_head(project_id, config),
            None => self.read_head_file(project_id),
        }
    }

    fn commit_head(&self, head: &ProjectHead, manifest: &[u8]) -> Result<ProjectHead, String> {
        if head.project_id.is_empty() {
            return Err("project head has no project id".into());
        }
        match &self.integrity {
            Some(config) => self.commit_secure(head, manifest, config),
            None => self.commit_legacy(head, manifest),
        }
    }
}

fn head_mac(head: &ProjectHead, config: &IntegrityConfig) -> Result<[u8; 32], String> {
    let key_id = head
        .key_id
        .as_deref()
        .ok_or("project head has no integrity key id")?;
    let payload = HeadMacPayload {
        schema_version: &head.schema_version,
        project_id: &head.project_id,
        head_revision: head.head_revision,
        manifest_sha256: &head.manifest_sha256,
        previous_manifest_sha256: &head.previous_head_sha256,
        previous_head_mac: &head.previous_head_mac,
        key_id,
    };
    let bytes = canonical_bytes(&payload).map_err(|error| error.to_string())?;
    Ok(hmac_sha256(&config.key, &bytes))
}

fn anchor_mac(anchor: &IntegrityAnchor, config: &IntegrityConfig) -> Result<[u8; 32], String> {
    let payload = AnchorMacPayload {
        schema_version: &anchor.schema_version,
        project_id: &anchor.project_id,
        highest_revision: anchor.highest_revision,
        manifest_sha256: &anchor.manifest_sha256,
        head_mac: &anchor.head_mac,
        key_id: &anchor.key_id,
    };
    let bytes = canonical_bytes(&payload).map_err(|error| error.to_string())?;
    Ok(hmac_sha256(&config.key, &bytes))
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut normalized_key = [0_u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        normalized_key[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalized_key[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0x36_u8; BLOCK_SIZE];
    let mut outer_pad = [0x5c_u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        inner_pad[index] ^= normalized_key[index];
        outer_pad[index] ^= normalized_key[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer.finalize().into()
}

fn write_immutable(path: &Path, bytes: &[u8], allow_identical: bool) -> Result<(), String> {
    let parent = path.parent().ok_or("immutable target has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    if path.try_exists().map_err(|error| error.to_string())? {
        return verify_existing_immutable(path, bytes, allow_identical);
    }

    // Never stream bytes directly into the immutable namespace. A process
    // death can leave only this unreferenced temporary file; the revision or
    // signed sidecar becomes visible only after all bytes have been synced.
    let temp = parent.join(format!(".f2s-immutable-{}.tmp", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;

        if path.try_exists().map_err(|error| error.to_string())? {
            return verify_existing_immutable(path, bytes, allow_identical);
        }
        fs::rename(&temp, path).map_err(|error| error.to_string())
    })();
    if temp.try_exists().unwrap_or(false) {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn verify_existing_immutable(
    path: &Path,
    bytes: &[u8],
    allow_identical: bool,
) -> Result<(), String> {
    if !allow_identical {
        return Err("revision already exists".into());
    }
    let existing = fs::read(path).map_err(|read_error| read_error.to_string())?;
    if existing == bytes {
        Ok(())
    } else {
        Err("immutable project revision already exists with different content".into())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], String> {
    let bytes = value.as_bytes();
    if bytes.len() != 64 {
        return Err("integrity MAC must be 64 lowercase hexadecimal characters".into());
    }
    let mut decoded = [0_u8; 32];
    for (index, pair) in bytes.chunks_exact(2).enumerate() {
        decoded[index] = (decode_hex_nibble(pair[0])? << 4) | decode_hex_nibble(pair[1])?;
    }
    Ok(decoded)
}

fn decode_hex_nibble(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err("integrity MAC must use lowercase hexadecimal".into()),
    }
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn constant_time_eq_32(left: &[u8; 32], right: &[u8; 32]) -> bool {
    let mut difference = 0_u8;
    for index in 0..32 {
        difference |= left[index] ^ right[index];
    }
    difference == 0
}

fn constant_time_hex_eq(left: &str, right: &str) -> Result<bool, String> {
    let left = decode_hex_32(left)?;
    let right = decode_hex_32(right)?;
    Ok(constant_time_eq_32(&left, &right))
}

fn constant_time_head_eq(left: &ProjectHead, right: &ProjectHead) -> Result<bool, String> {
    let left = canonical_bytes(left).map_err(|error| error.to_string())?;
    let right = canonical_bytes(right).map_err(|error| error.to_string())?;
    let left_hash: [u8; 32] = Sha256::digest(left).into();
    let right_hash: [u8; 32] = Sha256::digest(right).into();
    Ok(constant_time_eq_32(&left_hash, &right_hash))
}

#[cfg(test)]
mod tests {
    use super::{encode_hex, hmac_sha256};

    #[test]
    fn hmac_sha256_matches_rfc_4231_vectors() {
        let cases = [
            (
                vec![0x0b; 20],
                b"Hi There".to_vec(),
                "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
            ),
            (
                b"Jefe".to_vec(),
                b"what do ya want for nothing?".to_vec(),
                "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            ),
            (
                vec![0xaa; 20],
                vec![0xdd; 50],
                "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
            ),
        ];
        for (key, message, expected) in cases {
            assert_eq!(encode_hex(&hmac_sha256(&key, &message)), expected);
        }
    }
}
