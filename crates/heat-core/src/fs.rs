use crate::error::HeatError;
use std::path::Path;

/// Ensure a directory exists, creating it if necessary.
pub fn ensure_dir(path: &Path) -> Result<(), HeatError> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| {
            HeatError::internal(
                "mkdir_failed",
                format!("Failed to create directory {}: {e}", path.display()),
            )
        })?;
    }
    Ok(())
}

/// Atomic write: write to sibling temp file, then rename.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), HeatError> {
    let tmp = sibling_tmp(path);
    std::fs::write(&tmp, data).map_err(|e| {
        HeatError::internal(
            "write_failed",
            format!("Failed to write {}: {e}", tmp.display()),
        )
    })?;
    std::fs::rename(&tmp, path).map_err(|e| {
        // Clean up temp file on rename failure
        let _ = std::fs::remove_file(&tmp);
        HeatError::internal(
            "rename_failed",
            format!(
                "Failed to rename {} -> {}: {e}",
                tmp.display(),
                path.display()
            ),
        )
    })
}

/// Atomic write with chmod 600 (key files).
pub fn atomic_write_secure(path: &Path, data: &[u8]) -> Result<(), HeatError> {
    atomic_write(path, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms).map_err(|e| {
            HeatError::internal(
                "chmod_failed",
                format!("Failed to set permissions on {}: {e}", path.display()),
            )
        })?;
    }
    Ok(())
}

/// Generate a sibling temp path: /dir/foo.toml -> /dir/.foo.toml.heat-tmp
fn sibling_tmp(path: &Path) -> std::path::PathBuf {
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("temp");
    let parent = path.parent().unwrap_or(Path::new("."));
    parent.join(format!(".{file_name}.heat-tmp"))
}
