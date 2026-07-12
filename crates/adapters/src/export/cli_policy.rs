use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

pub const REQUIRED_SPINE_PATCH: &str = "4.2.43";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpineCliPolicy {
    /// Must be the path selected by the user. The application never searches PATH.
    pub executable: PathBuf,
    pub user_confirmed_professional_license: bool,
    /// Reserved for a future OS-enforced network sandbox. This runner only accepts `false`.
    pub network_granted_for_operation: bool,
    pub expected_patch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSpineCli {
    canonical_executable: PathBuf,
    executable_sha256: String,
}

impl ValidatedSpineCli {
    pub fn canonical_executable(&self) -> &Path {
        &self.canonical_executable
    }

    pub fn executable_sha256(&self) -> &str {
        &self.executable_sha256
    }

    /// A non-reversible path identifier suitable for receipts and diagnostics.
    pub fn path_token(&self) -> String {
        sha256_bytes(path_comparison_key(&self.canonical_executable).as_bytes())
    }
}

impl SpineCliPolicy {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_selected_executable().map(|_| ())
    }

    pub fn validate_selected_executable(&self) -> Result<ValidatedSpineCli, String> {
        if !self.user_confirmed_professional_license {
            return Err("user must confirm a legal Spine Professional license".into());
        }
        if self.network_granted_for_operation {
            return Err(
                "network-enabled Spine CLI execution is unsupported; configure 4.2.43 locally"
                    .into(),
            );
        }
        if self.expected_patch != REQUIRED_SPINE_PATCH {
            return Err(format!("only Spine CLI {REQUIRED_SPINE_PATCH} is allowed"));
        }
        validate_local_absolute_path_shape(&self.executable)?;
        if self
            .executable
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| !value.eq_ignore_ascii_case("spine.com"))
            .unwrap_or(true)
        {
            return Err("user must select the Spine.com executable by its exact file name".into());
        }
        reject_reparse_components(&self.executable)?;
        let metadata = fs::metadata(&self.executable)
            .map_err(|error| format!("selected Spine.com is unavailable: {error}"))?;
        if !metadata.is_file() {
            return Err("selected Spine.com path is not a regular file".into());
        }
        let canonical = fs::canonicalize(&self.executable)
            .map_err(|error| format!("cannot canonicalize selected Spine.com: {error}"))?;
        if path_comparison_key(&canonical) != path_comparison_key(&self.executable) {
            return Err("selected Spine.com path is not canonical".into());
        }
        reject_reparse_components(&canonical)?;
        let executable_sha256 = sha256_file(&canonical)?;
        Ok(ValidatedSpineCli {
            canonical_executable: canonical,
            executable_sha256,
        })
    }
}

pub(crate) fn validate_local_absolute_path_shape(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("path must be absolute".into());
    }
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
    {
        return Err("path must be lexically canonical and contain no dot components".into());
    }
    #[cfg(windows)]
    {
        use std::path::Prefix;
        let Some(Component::Prefix(prefix)) = path.components().next() else {
            return Err("path must use a local Windows drive".into());
        };
        if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) {
            return Err("UNC and device paths are not accepted for Spine CLI operations".into());
        }
        if path.to_string_lossy().contains(':') {
            let text = path.to_string_lossy();
            let drive_colon_only = text
                .char_indices()
                .filter_map(|(index, value)| (value == ':').then_some(index))
                .collect::<Vec<_>>();
            let expected = if text.starts_with(r"\\?\") { 5 } else { 1 };
            if drive_colon_only != [expected] {
                return Err("alternate data stream path syntax is not accepted".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn reject_reparse_components(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        for component in path.ancestors() {
            if component.as_os_str().is_empty() || !component.exists() {
                continue;
            }
            let metadata = fs::symlink_metadata(component)
                .map_err(|error| format!("cannot inspect path component: {error}"))?;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err("reparse points are not accepted for Spine CLI paths".into());
            }
        }
    }
    #[cfg(not(windows))]
    for component in path.ancestors() {
        if component.as_os_str().is_empty() || !component.exists() {
            continue;
        }
        if fs::symlink_metadata(component)
            .map_err(|error| format!("cannot inspect path component: {error}"))?
            .file_type()
            .is_symlink()
        {
            return Err("symbolic links are not accepted for Spine CLI paths".into());
        }
    }
    Ok(())
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("cannot open file for hashing: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot hash file: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn path_comparison_key(path: &Path) -> String {
    let value = path.to_string_lossy().replace('/', "\\");
    #[cfg(windows)]
    {
        value
            .strip_prefix(r"\\?\")
            .unwrap_or(&value)
            .to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    value
}

/// Legacy manifest-shape check. This does not establish process provenance. New publication code
/// must additionally call `SpineCliOperationReport::authorizes_proprietary_output` with the bytes'
/// SHA-256 before accepting a proprietary artifact.
pub fn proprietary_output_allowed(
    path: &Path,
    producing_operation_id: Option<&str>,
    observed_patch: Option<&str>,
) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["atlas", "spine", "skel"].contains(&extension.as_str()) {
        return true;
    }
    producing_operation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        && observed_patch == Some(REQUIRED_SPINE_PATCH)
}
