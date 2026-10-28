//! `ruviform-plugin-clap-l1-runner` — the CLAP-specific L1
//! child-process runner (Issue #114, task
//! `docs/tasks/real-adapter-runner-bins.md`).
//!
//! The binary is compile-time fixed: it embeds no factories of its
//! own. It dlopens the workspace-built CLAP fixture cdylib
//! (`{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}`) through the `host`
//! bridge, hands the scanned `ClapPluginFactory`s to
//! [`ruviform_plugin_export::run`], and serves plugin instances over
//! the L1 wire (one child hosting up to `WIRE_MAX_INSTANCES` of them
//! through `run`'s per-instance registry). Wire v2's `run` takes no
//! argv at all — the v1 `--ruviform-shm-key`/`--ruviform-plugin-id`
//! bootstrap flags moved into the `CreateInstance` payload, and any
//! argument (`--ruviform-*` included) is a usage rejection → exit 2 —
//! so no plugin-path or loader argument exists and
//! `OutOfProcessSpec::args` stays empty. `run` selects a factory by
//! `plugin_id` alone — embedding two factories with the same id in one
//! binary is an authoring error (the second is unreachable).
//!
//! ## Fixture resolution
//!
//! `RUVIFORM_L1_REAL_PLUGIN_DIR` (optional, shell-only env override
//! for out-of-tree fixture layouts — env injection, not an argv
//! extension) → unset: the runner executable's own directory
//! (`target/<profile>/`, or `deps/` for the unit-test binary). A
//! resolution or load failure is a defined rejection — a stderr
//! diagnostic + exit 1 before `run` is reached, so no `Hello` frame is
//! sent and the parent's spawn converges to `Err(HostError::Ipc)`
//! instead of hanging.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ruviform_plugin_clap::host::{ClapLibrary, fixture_lib_name};
use ruviform_plugin_core::PluginFactory;

/// Optional env override for the fixture directory — the lane's
/// escape hatch for out-of-tree fixture layouts. Never an argv
/// extension: `run`'s argument grammar is untouched.
const PLUGIN_DIR_ENV: &str = "RUVIFORM_L1_REAL_PLUGIN_DIR";

fn main() -> ExitCode {
    let path = match fixture_path(env::var_os(PLUGIN_DIR_ENV)) {
        Ok(path) => path,
        Err(reason) => {
            eprintln!("ruviform-plugin-clap-l1-runner: {reason}");
            return ExitCode::FAILURE;
        }
    };
    let library = match ClapLibrary::load(&path) {
        Ok(library) => library,
        Err(error) => {
            eprintln!(
                "ruviform-plugin-clap-l1-runner: cannot load fixture {}: {error}",
                path.display()
            );
            return ExitCode::FAILURE;
        }
    };
    ruviform_plugin_export::run(
        library
            .factories()
            .into_iter()
            .map(|factory| Box::new(factory) as Box<dyn PluginFactory>)
            .collect(),
    )
}

/// Resolves the fixture cdylib path: `dir_override` (the
/// `RUVIFORM_L1_REAL_PLUGIN_DIR` value when set) wins verbatim;
/// otherwise the runner executable's own directory. The file name is
/// the crate's shared convention — [`fixture_lib_name`].
fn fixture_path(dir_override: Option<OsString>) -> Result<PathBuf, String> {
    let dir = match dir_override {
        Some(dir) => PathBuf::from(dir),
        None => exe_dir()?,
    };
    Ok(dir.join(fixture_lib_name()))
}

/// The runner executable's parent directory — `target/<profile>/`
/// where cargo places both this binary and the fixture cdylib (and
/// `deps/` for the unit-test binary, which the cdylib is also emitted
/// into).
fn exe_dir() -> Result<PathBuf, String> {
    let exe = env::current_exe()
        .map_err(|error| format!("cannot resolve the runner executable path: {error}"))?;
    exe.parent().map(Path::to_path_buf).ok_or_else(|| {
        format!(
            "runner executable has no parent directory: {}",
            exe.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The env override is used verbatim — resolution is a plain join,
    /// so an override pointing at an empty dir lands on a nonexistent
    /// file that `ClapLibrary::load` then rejects (the exit-1 path).
    #[test]
    fn override_dir_resolves_inside_the_given_dir() {
        let path = fixture_path(Some(OsString::from("fixture-dir")))
            .expect("override resolution cannot fail");
        assert_eq!(path, Path::new("fixture-dir").join(fixture_lib_name()));
    }

    /// Default resolution lands next to the executable. This unit-test
    /// binary lives under `target/<profile>/deps/`, where cargo also
    /// emits the fixture cdylib — the missing fixture is a failure,
    /// never a silent skip (same convention as the `host` bridge
    /// tests).
    #[test]
    fn sibling_resolution_finds_the_workspace_fixture() {
        let path = fixture_path(None).expect("test binary lives under target/<profile>/deps");
        let exe_dir = env::current_exe()
            .expect("current_exe")
            .parent()
            .expect("test binary lives in deps/")
            .to_path_buf();
        assert_eq!(path, exe_dir.join(fixture_lib_name()));
        assert!(
            path.is_file(),
            "fixture cdylib missing: {} — build with `cargo build -p ruviform-plugin-clap \
             --features host` first",
            path.display()
        );
    }
}
