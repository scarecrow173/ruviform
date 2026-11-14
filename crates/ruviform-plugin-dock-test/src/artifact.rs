//! `test_gain` artifact resolution and binary hashing.
//!
//! Resolution order (the lane's explicit-path-first contract):
//!
//! 1. `explicit` — the value of [`PLUGIN_PATH_ENV`] when the lane or a
//!    test injects one (a file path, used verbatim);
//! 2. `current_exe()`'s `deps/` directory — where cargo places both
//!    test binaries and cdylib artifacts;
//! 3. `current_exe()`'s own directory — where `cargo build` places
//!    the cdylib for `target/<profile>/` binaries.
//!
//! The file name follows the shared fixture convention
//! [`fixture_lib_name`] (`{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`).
//!
//! A missing artifact is *not* an error here — [`resolve_artifact`]
//! returns `Err` with the searched paths so the caller can record an
//! honest `NotRun` (fail-closed) instead of silently skipping.
//!
//! **RT contract:** non-RT only — filesystem I/O.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Environment variable naming an explicit plugin-binary path — the
/// lane's escape hatch for out-of-tree fixture layouts. Shell-only
/// env injection, never an argv contract.
pub const PLUGIN_PATH_ENV: &str = "RUVIFORM_PLUGIN_DOCK_TEST_VST3";

/// The workspace-built `test_gain` cdylib file name:
/// `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`.
#[must_use]
pub fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_vst3{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// Candidate artifact paths in resolution order. `explicit` (the
/// [`PLUGIN_PATH_ENV`] value) wins verbatim; otherwise the test/bin
/// executable's `deps/` then its own directory are searched for
/// [`fixture_lib_name`].
#[must_use]
pub fn candidate_paths(explicit: Option<&Path>) -> Vec<PathBuf> {
    if let Some(path) = explicit {
        return vec![path.to_path_buf()];
    }
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("deps").join(fixture_lib_name()));
        candidates.push(dir.join(fixture_lib_name()));
    }
    candidates
}

/// Resolves the `test_gain` artifact to an existing file.
///
/// # Errors
/// `Err(detail)` listing every searched path when no candidate is a
/// file — the caller maps this to `NotRun` (fail-closed).
pub fn resolve_artifact(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let candidates = candidate_paths(explicit);
    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    let searched = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "test_gain artifact not found; searched: {searched}"
    ))
}

/// Lowercase hex SHA-256 of the artifact bytes — the digest recorded
/// in case evidence so a `Pass` is attributable to exact bytes.
///
/// # Errors
/// Propagates the `std::fs::read` I/O error.
pub fn sha256_hex(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}
