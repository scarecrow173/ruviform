//! Real-filesystem integration tests for the OS backend (IO-IMP-02).
//!
//! Everything here exercises the real `OsFileOps` path through the
//! public `StagedFile`/`SealedFile` API on a real filesystem (a unique
//! workdir under `std::env::temp_dir()`). Inducible failures
//! (existing destination, directory destination, missing parent,
//! exclusive-share/read-only directory) are exercised for real;
//! failures that cannot be induced safely in CI (mid-publish
//! ambiguity, transport-level indeterminacy) stay covered by the
//! fault-injection mock in `src/staged.rs`.
//!
//! The multi-process `CreateNew` contention test spawns copies of
//! this test binary which re-run `child_create_new_once` — a
//! `#[ignore]`d entry point — coordinated by a gate file.

use ruviform_io_file::{
    Durability, DurabilityEvidence, FileErrorKind, PublishEffect, PublishFailureKind, PublishMode,
    PublishOptions, PublishPhase, StagedFile, TempState,
};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

/// A unique, real filesystem workdir for one test.
fn workdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rvf-ospub-{tag}-{}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    fs::create_dir_all(&dir).expect("workdir");
    dir
}

/// Removes the workdir (and any leftover files) on drop.
struct DirGuard(PathBuf);

impl DirGuard {
    fn new(dir: PathBuf) -> Self {
        Self(dir)
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn opts(mode: PublishMode, durability: Durability) -> PublishOptions {
    PublishOptions::new(mode, durability)
}

fn staged_at(dest: &Path, payload: &[u8]) -> StagedFile {
    let mut staged = StagedFile::create(dest).expect("create");
    staged.write_all(payload).expect("write");
    staged
}

/// Temp files staged by the backend inside `dir`.
fn temp_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .expect("read_dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().contains(".rvf-tmp-"))
        })
        .collect()
}

// ------------------------------------------------------------------
// Happy-path publish + durability evidence
// ------------------------------------------------------------------

#[test]
fn publish_create_new_file_and_directory_sync() {
    let dir = workdir("full");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("out.bin");
    let sealed = staged_at(&dest, b"payload-1234").seal();
    let receipt = sealed
        .publish(opts(
            PublishMode::CreateNew,
            Durability::FileAndDirectorySync,
        ))
        .expect("publish");
    assert_eq!(
        receipt.durability(),
        DurabilityEvidence::FileAndDirectorySynced
    );
    assert_eq!(receipt.temp_state(), TempState::Absent);
    assert_eq!(receipt.destination(), dest.as_path());
    assert_eq!(fs::read(&dest).unwrap(), b"payload-1234");
    assert!(temp_files(&dir).is_empty(), "no temp leftovers");
}

#[test]
fn publish_file_sync_reports_file_evidence() {
    let dir = workdir("filesync");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("f.bin");
    let receipt = staged_at(&dest, b"abc")
        .seal()
        .publish(opts(PublishMode::CreateNew, Durability::FileSync))
        .expect("publish");
    assert_eq!(receipt.durability(), DurabilityEvidence::FileSynced);
    assert_eq!(fs::read(&dest).unwrap(), b"abc");
}

#[test]
fn publish_best_effort_reports_no_evidence() {
    let dir = workdir("besteffort");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("be.bin");
    let receipt = staged_at(&dest, b"be")
        .seal()
        .publish(opts(PublishMode::Replace, Durability::BestEffort))
        .expect("publish");
    assert_eq!(receipt.durability(), DurabilityEvidence::None);
    assert_eq!(fs::read(&dest).unwrap(), b"be");
}

#[test]
fn publish_best_effort_visibility_still_atomic_primitive() {
    // BestEffort relaxes the *requirement*, not the mechanism: the
    // adapter still uses the atomic primitive, so the publish simply
    // succeeds.
    let dir = workdir("be-vis");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("bev.bin");
    let receipt = staged_at(&dest, b"v")
        .seal()
        .publish(
            opts(PublishMode::CreateNew, Durability::BestEffort)
                .with_visibility(ruviform_io_file::VisibilityRequirement::BestEffort),
        )
        .expect("publish");
    assert_eq!(receipt.durability(), DurabilityEvidence::None);
}

// ------------------------------------------------------------------
// Readback + write/seek surface
// ------------------------------------------------------------------

#[test]
fn readback_reads_staged_object_and_seeks() {
    let dir = workdir("readback");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("rb.bin");
    let mut staged = StagedFile::create(&dest).expect("create");
    staged.write_all(b"hello").expect("write");
    staged.seek(SeekFrom::Start(0)).expect("seek");
    staged.write_all(b"HE").expect("overwrite");
    let mut sealed = staged.seal();
    {
        let mut back = sealed.open_readback().expect("readback");
        let mut s = String::new();
        back.read_to_string(&mut s).expect("read");
        assert_eq!(s, "HEllo");
        let end = back.seek(SeekFrom::End(-1)).expect("seek end");
        assert_eq!(end, 4);
        let mut last = [0u8; 1];
        back.read_exact(&mut last).expect("read last");
        assert_eq!(last, [b'o']);
    }
    sealed
        .publish(opts(PublishMode::Replace, Durability::BestEffort))
        .expect("publish");
    assert_eq!(fs::read(&dest).unwrap(), b"HEllo");
}

#[test]
fn sparse_write_beyond_end_zero_fills() {
    let dir = workdir("sparse");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("sp.bin");
    let mut staged = StagedFile::create(&dest).expect("create");
    staged.write_all(b"A").expect("write");
    staged.seek(SeekFrom::Start(4)).expect("seek");
    staged.write_all(b"B").expect("write");
    staged
        .seal()
        .publish(opts(PublishMode::Replace, Durability::BestEffort))
        .expect("publish");
    assert_eq!(fs::read(&dest).unwrap(), [b'A', 0, 0, 0, b'B']);
}

// ------------------------------------------------------------------
// CreateNew / Replace semantics + inducible failures
// ------------------------------------------------------------------

#[test]
fn create_new_refuses_existing_destination() {
    let dir = workdir("exists");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("keep.bin");
    fs::write(&dest, b"original").expect("seed dest");
    let mut failure = staged_at(&dest, b"replacement")
        .seal()
        .publish(opts(PublishMode::CreateNew, Durability::FileSync))
        .expect_err("must refuse existing dest");
    assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.phase(), PublishPhase::Publish);
    assert_eq!(failure.durability(), DurabilityEvidence::FileSynced);
    assert_eq!(fs::read(&dest).unwrap(), b"original");

    // The retained temp must be inspectable and cleanly removable.
    let handle = failure.take_recovery_handle().expect("handle");
    assert_eq!(
        handle.inspect().expect("inspect").temp_state(),
        TempState::Retained
    );
    let temp = handle.info().temp_path().expect("temp path").to_path_buf();
    assert!(temp.exists());
    let receipt = handle.cleanup().expect("cleanup");
    assert_eq!(receipt.temp_state(), TempState::Absent);
    assert!(!temp.exists());
}

#[test]
fn replace_overwrites_existing_destination() {
    let dir = workdir("replace");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("r.bin");
    fs::write(&dest, b"old-content").expect("seed");
    let receipt = staged_at(&dest, b"new-content")
        .seal()
        .publish(opts(PublishMode::Replace, Durability::FileSync))
        .expect("publish");
    assert_eq!(receipt.durability(), DurabilityEvidence::FileSynced);
    assert_eq!(fs::read(&dest).unwrap(), b"new-content");
    assert_eq!(temp_files(&dir), Vec::<PathBuf>::new());
}

#[test]
fn replace_into_directory_is_definite_failure() {
    let dir = workdir("destdir");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("a-directory");
    fs::create_dir(&dest).expect("dest dir");
    let mut failure = staged_at(&dest, b"x")
        .seal()
        .publish(opts(PublishMode::Replace, Durability::BestEffort))
        .expect_err("cannot replace a directory");
    // A directory destination is a *definite* refusal — never
    // indeterminate.
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_ne!(failure.kind(), PublishFailureKind::DestinationExists);
    assert!(dest.is_dir(), "destination directory untouched");
    let handle = failure.take_recovery_handle().expect("handle");
    handle.cleanup().expect("cleanup");
}

#[test]
fn create_into_missing_parent_is_io_failure() {
    let dir = workdir("noparent");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("no-such-dir").join("f.bin");
    let err = StagedFile::create(&dest).expect_err("missing parent");
    assert_eq!(err.kind(), FileErrorKind::Io);
}

#[test]
fn abort_removes_temp() {
    let dir = workdir("abort");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("ab.bin");
    let staged = staged_at(&dest, b"abort me");
    let temp = staged.temp_path().expect("temp").to_path_buf();
    assert!(temp.exists());
    let receipt = staged.abort().expect("abort");
    assert_eq!(receipt.temp_state(), TempState::Absent);
    assert!(!temp.exists());
    assert!(!dest.exists());
}

#[test]
fn drop_without_publish_removes_temp() {
    let dir = workdir("drop");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("dr.bin");
    let temp = {
        let staged = staged_at(&dest, b"drop me");
        staged.temp_path().expect("temp").to_path_buf()
    };
    assert!(!temp.exists(), "drop best-effort cleanup ran");
    assert!(!dest.exists());
}

#[test]
fn publish_failure_source_is_io_error() {
    let dir = workdir("source");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("s.bin");
    fs::write(&dest, b"keep").expect("seed");
    let mut failure = staged_at(&dest, b"x")
        .seal()
        .publish(opts(PublishMode::CreateNew, Durability::BestEffort))
        .expect_err("refused");
    let source = std::error::Error::source(&failure).expect("source");
    assert!(
        source.downcast_ref::<std::io::Error>().is_some(),
        "failure source must expose the OS io::Error"
    );
    if let Some(h) = failure.take_recovery_handle() {
        let _ = h.cleanup();
    }
}

// ------------------------------------------------------------------
// OS-specific inducible failures
// ------------------------------------------------------------------

/// Windows: an exclusively-open destination cannot be replaced — the
/// OS-level sharing violation is a definite `NotApplied`.
#[cfg(windows)]
#[test]
fn replace_dest_held_exclusively_fails() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = workdir("share");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("held.bin");
    fs::write(&dest, b"keep").expect("seed");
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&dest)
        .expect("exclusive open");
    let mut failure = staged_at(&dest, b"new")
        .seal()
        .publish(opts(PublishMode::Replace, Durability::FileSync))
        .expect_err("exclusive dest must refuse");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.kind(), PublishFailureKind::Io);
    assert_eq!(failure.durability(), DurabilityEvidence::FileSynced);
    if let Some(h) = failure.take_recovery_handle() {
        let _ = h.cleanup();
    }
    drop(held);
    assert_eq!(fs::read(&dest).unwrap(), b"keep");
}

/// Unix: a read-only destination directory makes the publish a
/// definite `NotApplied` (EACCES), with the temp retained for
/// recovery. Skipped where the process bypasses permission checks
/// (root) — the probe write detects that and returns early.
#[cfg(unix)]
#[test]
fn replace_in_readonly_dir_is_definite_failure() {
    use std::os::unix::fs::PermissionsExt;
    let dir = workdir("rodir");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("ro.bin");
    fs::write(&dest, b"keep").expect("seed");
    let staged = staged_at(&dest, b"new");
    let mut perms = fs::metadata(&dir).unwrap().permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&dir, perms).unwrap();
    struct PermGuard(PathBuf);
    impl Drop for PermGuard {
        fn drop(&mut self) {
            let mut p = fs::metadata(&self.0).unwrap().permissions();
            p.set_mode(0o755);
            let _ = fs::set_permissions(&self.0, p);
        }
    }
    let pg = PermGuard(dir.clone());
    // Root bypasses directory permissions — verify the restriction
    // actually holds before relying on the failure.
    if fs::write(dir.join(".probe"), b"x").is_ok() {
        let _ = fs::remove_file(dir.join(".probe"));
        return;
    }
    let mut failure = staged
        .seal()
        .publish(opts(PublishMode::Replace, Durability::FileSync))
        .expect_err("readonly dir must refuse");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    drop(pg);
    let handle = failure.take_recovery_handle().expect("handle");
    handle.cleanup().expect("cleanup after restore");
    assert_eq!(fs::read(&dest).unwrap(), b"keep");
}

// ------------------------------------------------------------------
// Multi-process CreateNew contention
// ------------------------------------------------------------------

const CHILD_DEST: &str = "RVF_IOFILE_CHILD_DEST";
const CHILD_GATE: &str = "RVF_IOFILE_CHILD_GATE";
const CHILD_PAYLOAD: &str = "RVF_IOFILE_CHILD_PAYLOAD";
const CHILD_EXIT_PUBLISHED: i32 = 0;
const CHILD_EXIT_EXISTS: i32 = 2;
const CHILD_EXIT_OTHER: i32 = 9;

/// Child entry point for [`create_new_contention_multi_process`].
/// Stages + writes before the gate, then publishes once the gate
/// file appears so all processes contend on the same destination.
#[test]
#[ignore = "spawned by create_new_contention_multi_process; not a standalone test"]
fn child_create_new_once() {
    let dest = PathBuf::from(std::env::var(CHILD_DEST).expect("child dest"));
    let gate = PathBuf::from(std::env::var(CHILD_GATE).expect("child gate"));
    let payload: u8 = std::env::var(CHILD_PAYLOAD)
        .expect("child payload")
        .parse()
        .expect("u8");

    let Ok(mut staged) = StagedFile::create(&dest) else {
        std::process::exit(CHILD_EXIT_OTHER)
    };
    if staged.write_all(&[payload; 64]).is_err() {
        std::process::exit(CHILD_EXIT_OTHER);
    }

    let deadline = Instant::now() + Duration::from_secs(60);
    while !gate.exists() {
        if Instant::now() > deadline {
            std::process::exit(CHILD_EXIT_OTHER);
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    let result = staged
        .seal()
        .publish(opts(PublishMode::CreateNew, Durability::FileSync));
    match result {
        Ok(_) => std::process::exit(CHILD_EXIT_PUBLISHED),
        Err(mut failure) => {
            if failure.kind() != PublishFailureKind::DestinationExists {
                std::process::exit(CHILD_EXIT_OTHER);
            }
            // Recovery path: inspect + clean our own retained temp.
            if let Some(handle) = failure.take_recovery_handle()
                && handle.cleanup().is_err()
            {
                std::process::exit(CHILD_EXIT_OTHER);
            }
            std::process::exit(CHILD_EXIT_EXISTS);
        }
    }
}

/// AC1: N processes race a single `CreateNew` destination. Exactly
/// one publishes; every loser gets `DestinationExists` +
/// `NotPublished`; the destination is byte-for-byte the winner's.
#[test]
fn create_new_contention_multi_process() {
    const N: u8 = 6;
    let dir = workdir("contention");
    let _g = DirGuard::new(dir.clone());
    let dest = dir.join("contended.bin");
    let gate = dir.join("start.gate");
    let exe = std::env::current_exe().expect("current_exe");

    let mut children: Vec<std::process::Child> = (0..N)
        .map(|i| {
            Command::new(&exe)
                .args([
                    "--exact",
                    "child_create_new_once",
                    "--ignored",
                    "--test-threads=1",
                ])
                .env(CHILD_DEST, &dest)
                .env(CHILD_GATE, &gate)
                .env(CHILD_PAYLOAD, i.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn child")
        })
        .collect();

    // Wait until every child has staged its temp, then drop the gate.
    let deadline = Instant::now() + Duration::from_secs(60);
    while temp_files(&dir).len() < N as usize {
        assert!(
            Instant::now() < deadline,
            "children did not stage in time ({}/{} temps)",
            temp_files(&dir).len(),
            N
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    fs::write(&gate, b"go").expect("gate");

    let mut published = 0;
    let mut exists = 0;
    let mut other = 0;
    for child in &mut children {
        match child.wait().expect("wait").code() {
            Some(CHILD_EXIT_PUBLISHED) => published += 1,
            Some(CHILD_EXIT_EXISTS) => exists += 1,
            _ => other += 1,
        }
    }

    assert_eq!(published, 1, "exactly one process may publish");
    assert_eq!(
        exists,
        (N - 1) as usize,
        "all others must see DestinationExists"
    );
    assert_eq!(other, 0, "no other failure class allowed");

    let content = fs::read(&dest).expect("dest");
    assert_eq!(content.len(), 64);
    let winner = content[0];
    assert!(winner < N, "winner payload is one child's byte");
    assert!(content.iter().all(|&b| b == winner));

    assert_eq!(
        temp_files(&dir),
        Vec::<PathBuf>::new(),
        "all temps consumed or cleaned: {:?}",
        temp_files(&dir)
    );
}
