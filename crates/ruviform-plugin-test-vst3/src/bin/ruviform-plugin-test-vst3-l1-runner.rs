//! `ruviform-plugin-test-vst3-l1-runner` — the test-fixture VST3 L1
//! child-process runner (Issue #630 L1 slice).
//!
//! Structural mirror of the adapter's
//! `ruviform-plugin-vst3-l1-runner`, kept inside the test crate so the
//! production adapter's compile-time-fixed fixture name and sources
//! stay untouched. The binary embeds no factories of its own: it
//! dlopens this crate's own cdylib
//! (`{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`) through the
//! `ruviform-plugin-vst3` `host` bridge, hands the scanned
//! `Vst3PluginFactory`s — the bridge scans only `"Audio Module
//! Class"` entries — to [`ruviform_plugin_export::run`], and serves
//! plugin instances over the L1 wire (one child hosting up to
//! `WIRE_MAX_INSTANCES` of them through `run`'s per-instance
//! registry). Wire v2's `run` takes no argv at all — the v1
//! `--ruviform-shm-key`/`--ruviform-plugin-id` bootstrap flags moved
//! into the `CreateInstance` payload, and any argument
//! (`--ruviform-*` included) is a usage rejection → exit 2 — so no
//! plugin-path or loader argument exists and `OutOfProcessSpec::args`
//! stays empty *except* for the spawn-fault flag below. `run` selects
//! a factory by `plugin_id` alone — embedding two factories with the
//! same id in one binary is an authoring error (the second is
//! unreachable).
//!
//! ## Spawn-fault injection (test-only)
//!
//! `OutOfProcessSpec::args` may carry one recognized flag,
//! `--ruviform-test-fault=MODE`, consumed by this binary *before*
//! `run` is reached — after the fixture cdylib has been resolved and
//! loaded, so the fault is genuinely post-spawn/pre-`Hello`:
//!
//! | mode | effect |
//! | ---- | ------ |
//! | `exit:N` | exit `N` after the fixture loads |
//! | `abort` | `std::process::abort()` after the fixture loads |
//! | `hang` | park forever after the fixture loads — the parent's `Hello` deadline decides |
//! | `stderr:N` | emit `N` marker bytes on stderr, then exit 3 |
//! | `bad-fixture` | resolve a guaranteed-missing fixture path so the normal load path fails (exit 1) |
//!
//! Any other argument — including an unrecognized `--ruviform-test-fault`
//! value — falls through to `run`, whose argv grammar rejects it as a
//! usage error (exit 2, still pre-`Hello`). `run`'s own contract is
//! untouched: the fault channel lives entirely in this binary.
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
//!
//! ## Test-only declaration
//!
//! This binary exists only under `--features l1-runner` and only to
//! serve the `#[ignore]`d L1 e2e lane — it is never a product
//! distribution artifact.

use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use ruviform_plugin_core::PluginFactory;
use ruviform_plugin_vst3::host::Vst3Library;

/// Optional env override for the fixture directory — the lane's
/// escape hatch for out-of-tree fixture layouts. Never an argv
/// extension: `run`'s argument grammar is untouched.
const PLUGIN_DIR_ENV: &str = "RUVIFORM_L1_REAL_PLUGIN_DIR";

/// The argv flag prefix this binary recognizes for spawn-fault
/// injection — see the module docs for the mode table.
const FAULT_ARG_PREFIX: &str = "--ruviform-test-fault=";

/// Marker bytes `stderr:N` repeats on the child's stderr — the
/// parent's diagnostics capture retains them verbatim.
const STDERR_MARKER: &[u8] = b"ruviform-test-vst3-spawn-fault-stderr\n";

fn main() -> ExitCode {
    let fault = SpawnFault::parse(env::args_os().skip(1));
    let resolved = if matches!(fault, Some(SpawnFault::BadFixture)) {
        missing_fixture_path()
    } else {
        fixture_path(env::var_os(PLUGIN_DIR_ENV))
    };
    let path = match resolved {
        Ok(path) => path,
        Err(reason) => {
            eprintln!("ruviform-plugin-test-vst3-l1-runner: {reason}");
            return ExitCode::FAILURE;
        }
    };
    let library = match Vst3Library::load(&path) {
        Ok(library) => library,
        Err(error) => {
            eprintln!(
                "ruviform-plugin-test-vst3-l1-runner: cannot load fixture {}: {error}",
                path.display()
            );
            return ExitCode::FAILURE;
        }
    };
    // A recognized fault fires only after the fixture loaded — the
    // child genuinely spawned and resolved its ABI library before
    // dying or wedging pre-`Hello`. `BadFixture` never reaches here:
    // its missing path already failed at `Vst3Library::load`.
    if let Some(fault) = fault {
        return fault.act();
    }
    ruviform_plugin_export::run(
        library
            .factories()
            .into_iter()
            .map(|factory| Box::new(factory) as Box<dyn PluginFactory>)
            .collect(),
    )
}

/// A pre-`Hello` fault this binary performs itself — the argv
/// channel `OutOfProcessSpec::args` carries into the child. Every
/// variant terminates or wedges the process before `run` is reached,
/// so no `Hello` frame is ever sent.
#[derive(Debug, PartialEq, Eq)]
enum SpawnFault {
    /// Exit with the given code once the fixture has loaded.
    Exit(u8),
    /// `std::process::abort()` once the fixture has loaded.
    Abort,
    /// Park forever once the fixture has loaded — the parent's `Hello`
    /// wait deadline (`command_timeout`) decides the outcome.
    Hang,
    /// Emit `n` marker bytes on stderr, then exit 3 — proves the
    /// parent's diagnostics capture retains child stderr.
    Stderr(u64),
    /// Resolve a guaranteed-missing fixture path so the ordinary
    /// resolution/load path fails with exit 1 — the
    /// invalid-fixture-path case.
    BadFixture,
}

impl SpawnFault {
    /// Scans `args` for the first `--ruviform-test-fault=MODE` flag.
    /// An unrecognized mode value yields `None`, so the argument set
    /// falls through to `run`'s usage rejection like any other argv.
    fn parse(args: impl IntoIterator<Item = OsString>) -> Option<Self> {
        for arg in args {
            let Some(value) = arg
                .to_str()
                .and_then(|arg| arg.strip_prefix(FAULT_ARG_PREFIX))
            else {
                continue;
            };
            return if value == "abort" {
                Some(Self::Abort)
            } else if value == "hang" {
                Some(Self::Hang)
            } else if value == "bad-fixture" {
                Some(Self::BadFixture)
            } else if let Some(code) = value.strip_prefix("exit:") {
                code.parse::<u8>().ok().map(Self::Exit)
            } else if let Some(bytes) = value.strip_prefix("stderr:") {
                bytes.parse::<u64>().ok().map(Self::Stderr)
            } else {
                None
            };
        }
        None
    }

    /// Performs the fault — every variant diverges from `run`.
    fn act(self) -> ExitCode {
        match self {
            Self::Exit(code) => ExitCode::from(code),
            Self::Abort => std::process::abort(),
            Self::Hang => loop {
                std::thread::sleep(Duration::from_secs(86_400));
            },
            Self::Stderr(bytes) => {
                let mut stderr = std::io::stderr().lock();
                let mut remaining = bytes;
                while remaining > 0 {
                    let chunk = usize::try_from(remaining.min(STDERR_MARKER.len() as u64))
                        .unwrap_or(STDERR_MARKER.len());
                    if stderr.write_all(&STDERR_MARKER[..chunk]).is_err() {
                        break;
                    }
                    remaining -= chunk as u64;
                }
                let _ = stderr.flush();
                ExitCode::from(3)
            }
            // `BadFixture` fails at `Vst3Library::load` before `act`.
            Self::BadFixture => ExitCode::FAILURE,
        }
    }
}

/// This crate's cdylib file name —
/// `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`. Local to the
/// test crate: the adapter's `fixture_lib_name()` is compile-time
/// fixed to the adapter fixture and must not be reused here.
fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_test_vst3{}",
        env::consts::DLL_PREFIX,
        env::consts::DLL_SUFFIX
    )
}

/// Resolves the fixture cdylib path: `dir_override` (the
/// `RUVIFORM_L1_REAL_PLUGIN_DIR` value when set) wins verbatim;
/// otherwise the runner executable's own directory. The file name is
/// this crate's own [`fixture_lib_name`] convention.
fn fixture_path(dir_override: Option<OsString>) -> Result<PathBuf, String> {
    let dir = match dir_override {
        Some(dir) => PathBuf::from(dir),
        None => exe_dir()?,
    };
    Ok(dir.join(fixture_lib_name()))
}

/// A sibling path that cannot exist — `<fixture>.missing` next to the
/// runner. `bad-fixture` mode routes the normal resolution here so
/// `Vst3Library::load` performs the real invalid-path rejection.
fn missing_fixture_path() -> Result<PathBuf, String> {
    exe_dir().map(|dir| dir.join(format!("{}.missing", fixture_lib_name())))
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
    /// file that `Vst3Library::load` then rejects (the exit-1 path).
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
            "fixture cdylib missing: {} — build with `cargo build -p ruviform-plugin-test-vst3 \
             --features l1-runner` first",
            path.display()
        );
    }

    /// The fault parser recognizes every documented mode and ignores
    /// unrelated argv — an unrecognized mode value yields `None` so
    /// the argument set falls through to `run`'s usage rejection.
    #[test]
    fn spawn_fault_parse_recognizes_documented_modes() {
        let args = |values: &[&str]| values.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(
            SpawnFault::parse(args(&["--ruviform-test-fault=exit:42"])),
            Some(SpawnFault::Exit(42))
        );
        assert!(matches!(
            SpawnFault::parse(args(&["--ruviform-test-fault=abort"])),
            Some(SpawnFault::Abort)
        ));
        assert!(matches!(
            SpawnFault::parse(args(&["--ruviform-test-fault=hang"])),
            Some(SpawnFault::Hang)
        ));
        assert!(matches!(
            SpawnFault::parse(args(&["--ruviform-test-fault=stderr:128"])),
            Some(SpawnFault::Stderr(128))
        ));
        assert!(matches!(
            SpawnFault::parse(args(&["--ruviform-test-fault=bad-fixture"])),
            Some(SpawnFault::BadFixture)
        ));
        // Unrelated argv and unrecognized mode values both yield None.
        assert_eq!(SpawnFault::parse(args(&["--other"])), None);
        assert_eq!(
            SpawnFault::parse(args(&["--ruviform-test-fault=bogus"])),
            None
        );
        assert_eq!(SpawnFault::parse(args(&[])), None);
    }
}
