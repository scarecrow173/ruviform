//! `test_gain`/`test_editor` artifact resolution and binary hashing.
//!
//! Resolution order (the lane's explicit-path-first contract):
//!
//! 1. `explicit` — the value of the format's plugin-path env var
//!    ([`PLUGIN_PATH_ENV`] for VST3, [`PLUGIN_PATH_ENV_CLAP`] for
//!    CLAP) when the lane or a test injects one (a file path, used
//!    verbatim);
//! 2. `current_exe()`'s `deps/` directory — where cargo places both
//!    test binaries and cdylib artifacts;
//! 3. `current_exe()`'s own directory — where `cargo build` places
//!    the cdylib for `target/<profile>/` binaries.
//!
//! The file name follows the shared fixture convention
//! (`{DLL_PREFIX}ruviform_plugin_test_{vst3,clap}{DLL_SUFFIX}` —
//! [`fixture_lib_name`]/[`fixture_lib_name_clap`]).
//!
//! A missing artifact is *not* an error here —
//! [`resolve_artifact`]/[`resolve_artifact_named`] return `Err` with
//! the searched paths so the caller can record an honest `NotRun`
//! (fail-closed) instead of silently skipping.
//!
//! **RT contract:** non-RT only — filesystem I/O.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Environment variable naming an explicit VST3 plugin-binary path —
/// the lane's escape hatch for out-of-tree fixture layouts.
/// Shell-only env injection, never an argv contract.
pub const PLUGIN_PATH_ENV: &str = "RUVIFORM_PLUGIN_DOCK_TEST_VST3";

/// Environment variable naming an explicit CLAP plugin-binary path —
/// the CLAP lane's mirror of [`PLUGIN_PATH_ENV`] (Issue #630 AC4 CLAP
/// leg). Shell-only env injection, never an argv contract.
pub const PLUGIN_PATH_ENV_CLAP: &str = "RUVIFORM_PLUGIN_DOCK_TEST_CLAP";

/// The workspace-built VST3 fixture cdylib file name:
/// `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`.
#[must_use]
pub fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_vst3{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// The workspace-built CLAP fixture cdylib file name:
/// `{DLL_PREFIX}ruviform_plugin_test_clap{DLL_SUFFIX}` (Issue #630
/// AC4 CLAP leg — symmetric to [`fixture_lib_name`]).
#[must_use]
pub fn fixture_lib_name_clap() -> String {
    format!(
        "{}ruviform_plugin_test_clap{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// Candidate artifact paths in resolution order. `explicit` (the
/// format's plugin-path env var value) wins verbatim; otherwise the
/// test/bin executable's `deps/` then its own directory are searched
/// for `lib_name`.
#[must_use]
pub fn candidate_paths_named(explicit: Option<&Path>, lib_name: &str) -> Vec<PathBuf> {
    if let Some(path) = explicit {
        return vec![path.to_path_buf()];
    }
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("deps").join(lib_name));
        candidates.push(dir.join(lib_name));
    }
    candidates
}

/// [`candidate_paths_named`] for the VST3 fixture name —
/// [`fixture_lib_name`].
#[must_use]
pub fn candidate_paths(explicit: Option<&Path>) -> Vec<PathBuf> {
    candidate_paths_named(explicit, &fixture_lib_name())
}

/// Resolves the `lib_name` fixture artifact to an existing file.
///
/// # Errors
/// `Err(detail)` listing every searched path when no candidate is a
/// file — the caller maps this to `NotRun` (fail-closed).
pub fn resolve_artifact_named(explicit: Option<&Path>, lib_name: &str) -> Result<PathBuf, String> {
    let candidates = candidate_paths_named(explicit, lib_name);
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
        "{lib_name} artifact not found; searched: {searched}"
    ))
}

/// Resolves the VST3 `test_gain`/`test_editor` artifact — the
/// [`fixture_lib_name`] specialization of
/// [`resolve_artifact_named`].
///
/// # Errors
/// `Err(detail)` listing every searched path when no candidate is a
/// file — the caller maps this to `NotRun` (fail-closed).
pub fn resolve_artifact(explicit: Option<&Path>) -> Result<PathBuf, String> {
    resolve_artifact_named(explicit, &fixture_lib_name())
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
