//! G01 real-4GiB boundary lane (Issue #306) — **opt-in only**.
//!
//! Every test in this file is `#[ignore]`d: the normal
//! `test-io-e2e` lane (`cargo test -p ruviform-io-e2e`) never runs
//! it. The single entry point is
//!
//! ```sh
//! mise run test ruviform-io-e2e --test large_staged -- --ignored --nocapture
//! ```
//!
//! What it verifies on the real filesystem through the public API
//! only (`StagedFile`/`SealedFile`/`PublishOptions`/`PublishReceipt`/
//! `PublishFailure`/`RecoveryHandle` — no production changes, no new
//! dependencies):
//!
//! - boundary sizes **4GiB−1 / 4GiB / 4GiB+1**;
//! - per size: `CreateNew` round-trip (write → seal → readback
//!   checksum → publish → reopen checksum), `Replace` round-trip,
//!   `CreateNew` conflict → `RecoveryHandle` cleanup, mid-write
//!   `StorageFull` fault → abort, and post-seal temp delete →
//!   recovery `TempState::Absent`;
//! - payload integrity via chunked FNV-1a (std only) computed during
//!   write and re-verified on sealed readback and post-publish
//!   reopen;
//! - **capacity honesty**: a preflight probe (`set_len` + tail-byte
//!   write + `sync_data`) checks the temp drive before any case
//!   runs, and a genuine `StorageFull` observed mid-run stops the
//!   remaining cases as **`NotRun`** — never disguised as Pass/Fail.
//!   Skip reasons (required bytes, observed result, temp path) are
//!   printed to stdout for transcription into `evidence/rf306/`.
//!
//! Execution model: one `#[ignore]`d test drives every size and
//! scenario **sequentially**, each in its own `TestDir`, so peak
//! disk usage stays at roughly one case (≈4GiB+1 + margin) and no
//! two 4GiB files coexist. Memory stays bounded: payload is
//! generated and checksummed in 4MiB chunks.
//!
//! WAV RIFF boundary note: the classic RIFF 4GiB boundary is already
//! pinned byte-exact by `wav_golden.rs` (discard sink, no real
//! file). A real-file WAV 4GiB variant is intentionally **`NotRun`**
//! here — see `docs/designs/issue-306-io-lane-completion.md`.

mod common;

use std::error::Error as _;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use common::{TestDir, children, exists};
use ruviform_io_file::{
    Durability, FileError, PublishEffect, PublishFailureKind, PublishMode, PublishOptions,
    PublishPhase, StagedFile, TempState,
};

/// 1 GiB in bytes.
const GIB: u64 = 1 << 30;

/// Boundary sizes under test: 4GiB−1 / 4GiB / 4GiB+1.
const SIZES: [u64; 3] = [4 * GIB - 1, 4 * GIB, 4 * GIB + 1];

/// Extra headroom the preflight probe demands beyond the largest
/// payload — NTFS journal/slack, incumbent seeds, and rename
/// co-existence margin. Conservative on purpose: a passing probe
/// must imply no case dies of capacity later.
const PROBE_MARGIN: u64 = 256 * 1024 * 1024;

/// Chunk size for payload generation and checksum passes — bounds
/// heap use regardless of file size.
const CHUNK: usize = 4 * 1024 * 1024;

/// FNV-1a 64-bit offset basis / prime (std-only checksum, no deps).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// splitmix64-style bit mixer — deterministic pattern source.
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Fills `buf` with the deterministic position-dependent pattern
/// for absolute file offsets `base..base+buf.len()`. `base` must be
/// a multiple of 8 so 8-byte words land on aligned positions; chunk
/// bases always are (CHUNK and every written length accumulate from
/// offset 0 in multiples of 8 except the final tail, which is filled
/// byte-wise).
fn fill_pattern(buf: &mut [u8], base: u64) {
    debug_assert_eq!(base % 8, 0, "pattern base must be 8-aligned");
    let (words, tail) = buf.as_chunks_mut::<8>();
    let tail_base = base + u64::try_from(words.len() * 8).expect("index fits u64");
    for (j, word) in words.iter_mut().enumerate() {
        let pos = base + u64::try_from(j * 8).expect("index fits u64");
        word.copy_from_slice(&mix64(pos).to_le_bytes());
    }
    for (k, byte) in tail.iter_mut().enumerate() {
        *byte = mix64(tail_base + u64::try_from(k).expect("index fits u64")).to_le_bytes()[0];
    }
}

/// FNV-1a over `bytes` continuing from `hash`.
fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Writes `size` bytes of pattern to `w`, returning the FNV-1a of
/// the whole stream. I/O errors propagate as `io::Result` so callers
/// decide whether a `StorageFull` is expected (fault fixture) or an
/// honest `NotRun` (capacity).
fn write_pattern<W: Write>(w: &mut W, size: u64) -> io::Result<u64> {
    let mut buf = vec![0_u8; CHUNK];
    let mut hash = FNV_OFFSET;
    let mut offset = 0_u64;
    while offset < size {
        let n = usize::try_from((size - offset).min(CHUNK as u64)).expect("chunk fits usize");
        fill_pattern(&mut buf[..n], offset);
        w.write_all(&buf[..n])?;
        hash = fnv1a_update(hash, &buf[..n]);
        offset += u64::try_from(n).expect("n fits u64");
    }
    w.flush()?;
    Ok(hash)
}

/// Reads `r` to EOF in chunks, returning the FNV-1a checksum and
/// asserting the total byte count equals `expected`.
fn checksum_stream<R: Read>(r: &mut R, expected: u64, ctx: &str) -> io::Result<u64> {
    let mut buf = vec![0_u8; CHUNK];
    let mut hash = FNV_OFFSET;
    let mut total = 0_u64;
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash = fnv1a_update(hash, &buf[..n]);
        total += u64::try_from(n).expect("read fits u64");
    }
    assert_eq!(total, expected, "{ctx}: byte count mismatch");
    Ok(hash)
}

/// `io::Result` → `NotRun` on storage exhaustion, panic on any other
/// I/O error (an honest failure, not a skip).
fn check_io<T>(result: io::Result<T>, ctx: &str) -> Result<T, NotRun> {
    match result {
        Ok(value) => Ok(value),
        Err(err) if is_storage_full(&err) => Err(NotRun::storage_full(ctx, &err)),
        Err(err) => panic!("{ctx}: unexpected io error: {err:?}"),
    }
}

/// `FileError` → `NotRun` when the wrapped `io::Error` reports
/// storage exhaustion (`StagedFile::create` / `open_readback` take
/// this path — they allocate/consume disk just like writes do),
/// panic on any other error.
fn check_file<T>(result: Result<T, FileError>, ctx: &str) -> Result<T, NotRun> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            let storage_full = err
                .source()
                .and_then(|source| source.downcast_ref::<io::Error>())
                .is_some_and(is_storage_full);
            if storage_full {
                Err(NotRun::storage_full(ctx, &err))
            } else {
                panic!("{ctx}: unexpected file error: {err:?}")
            }
        }
    }
}

/// Whether an io error means the filesystem ran out of space —
/// `ErrorKind::StorageFull` covers `ENOSPC`/`ERROR_DISK_FULL`; the
/// raw codes are checked too for mapping drift across platforms.
fn is_storage_full(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::StorageFull || matches!(err.raw_os_error(), Some(28 | 112))
}

/// Honest skip marker: observed capacity exhaustion. Carries the
/// reason printed to stdout and transcribed into evidence.
struct NotRun {
    reason: String,
}

impl NotRun {
    fn storage_full(ctx: &str, err: &(dyn std::fmt::Display + '_)) -> Self {
        Self {
            reason: format!("{ctx}: storage full — {err}"),
        }
    }
}

/// Preflight capacity probe: tries to actually allocate `needed`
/// bytes in `dir` via `set_len` + tail-byte write + `sync_data`,
/// then removes the probe. `Ok` means the largest case can plausibly
/// run; `Err` carries the io error for the `NotRun` report.
fn probe_capacity(dir: &Path, needed: u64) -> io::Result<()> {
    let probe = dir.join(format!(".rvf-g01-probe-{}", std::process::id()));
    let result = (|| {
        let mut file = File::create(&probe)?;
        file.set_len(needed)?;
        file.seek(SeekFrom::Start(needed - 1))?;
        file.write_all(&[0xA5])?;
        file.sync_data()?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&probe);
    result
}

/// `MediaSink`-style `Write` wrapper over `&mut StagedFile` that
/// accepts at most `capacity` payload bytes, then fails with
/// `StorageFull` — the mid-write disk-full injection point (same
/// shape as `tests/wav_fault.rs::DiskFull`; test-side only, the
/// production API is unchanged).
struct DiskFull<'a> {
    inner: &'a mut StagedFile,
    capacity: u64,
    written: u64,
}

impl DiskFull<'_> {
    fn new(inner: &mut StagedFile, capacity: u64) -> DiskFull<'_> {
        DiskFull {
            inner,
            capacity,
            written: 0,
        }
    }
}

impl Write for DiskFull<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let remaining = self.capacity.saturating_sub(self.written);
        if remaining == 0 {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "injected disk full",
            ));
        }
        let take = usize::try_from(remaining.min(u64::try_from(buf.len()).expect("len")))
            .expect("take fits usize");
        let n = self.inner.write(&buf[..take])?;
        self.written += u64::try_from(n).expect("written");
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn options(mode: PublishMode) -> PublishOptions {
    PublishOptions::new(mode, Durability::FileAndDirectorySync)
}

/// Scenario A — `CreateNew` round-trip at the boundary size:
/// write → seal → readback checksum → publish → reopen checksum.
fn scenario_create_new_roundtrip(size: u64) -> Result<(), NotRun> {
    let dir = TestDir::new("g01-create-new");
    let dest = dir.dest("payload.bin");

    let mut staged = check_file(StagedFile::create(&dest), "stage")?;
    let expected = check_io(write_pattern(&mut staged, size), "write")?;
    let mut sealed = staged.seal();

    // Scoped so the readback borrow ends before `publish` consumes
    // the sealed file.
    let sealed_hash = {
        let mut readback = check_file(sealed.open_readback(), "readback")?;
        check_io(checksum_stream(&mut readback, size, "readback"), "readback")?
    };
    assert_eq!(sealed_hash, expected, "sealed readback checksum mismatch");

    let receipt = sealed
        .publish(options(PublishMode::CreateNew))
        .expect("publish");
    assert_eq!(receipt.destination(), dest.as_path());

    let mut file = check_io(File::open(&dest), "reopen published")?;
    let reopened = check_io(checksum_stream(&mut file, size, "reopen"), "reopen")?;
    assert_eq!(reopened, expected, "published bytes checksum mismatch");
    assert_eq!(check_io(file.metadata(), "metadata")?.len(), size);
    assert_eq!(children(dir.path()), vec![dest.clone()]);
    Ok(())
}

/// Scenario B — `Replace` publish over an incumbent destination at
/// the boundary size, with reopen verification.
fn scenario_replace_roundtrip(size: u64) -> Result<(), NotRun> {
    let dir = TestDir::new("g01-replace");
    let dest = dir.dest("payload.bin");
    check_io(std::fs::write(&dest, b"incumbent-owner"), "seed incumbent")?;

    let mut staged = check_file(StagedFile::create(&dest), "stage")?;
    let expected = check_io(write_pattern(&mut staged, size), "write")?;
    let receipt = staged
        .seal()
        .publish(options(PublishMode::Replace))
        .expect("replace publish");
    assert_eq!(receipt.destination(), dest.as_path());

    let mut file = check_io(File::open(&dest), "reopen published")?;
    let reopened = check_io(checksum_stream(&mut file, size, "reopen"), "reopen")?;
    assert_eq!(reopened, expected, "published bytes checksum mismatch");
    assert_eq!(check_io(file.metadata(), "metadata")?.len(), size);
    assert_eq!(children(dir.path()), vec![dest.clone()]);
    Ok(())
}

/// Scenario C — `CreateNew` conflict at the boundary size: the
/// publish fails `DestinationExists`/`NotPublished`, the retained
/// staging temp is recovered through `RecoveryHandle::cleanup`, and
/// the incumbent destination stays untouched.
fn scenario_create_new_conflict_recovery(size: u64) -> Result<(), NotRun> {
    let dir = TestDir::new("g01-conflict");
    let dest = dir.dest("payload.bin");
    check_io(std::fs::write(&dest, b"incumbent-owner"), "seed incumbent")?;

    let mut staged = check_file(StagedFile::create(&dest), "stage")?;
    check_io(write_pattern(&mut staged, size), "write")?;
    let sealed = staged.seal();

    let mut failure = sealed
        .publish(options(PublishMode::CreateNew))
        .expect_err("create-new must refuse an existing destination");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
    assert_eq!(failure.phase(), PublishPhase::Publish);
    assert_eq!(
        check_io(std::fs::read(&dest), "read incumbent")?,
        b"incumbent-owner",
        "conflict must preserve incumbent bytes"
    );

    let handle = failure
        .take_recovery_handle()
        .expect("failed publish retains its recovery handle");
    let temp = handle
        .info()
        .temp_path()
        .map(PathBuf::from)
        .expect("os backend reports the temp path");
    assert!(exists(&temp), "temp retained for recovery at {size} bytes");
    handle.cleanup().expect("cleanup removes the retained temp");
    assert!(!exists(&temp));
    assert_eq!(children(dir.path()), vec![dest.clone()]);
    Ok(())
}

/// Scenario D — mid-write `StorageFull` fault at the boundary size:
/// the injected disk-full wrapper fails `write_all` partway, and
/// `abort` leaves neither temp nor destination behind.
fn scenario_mid_write_fault_abort(size: u64) -> Result<(), NotRun> {
    let dir = TestDir::new("g01-fault");
    let dest = dir.dest("payload.bin");

    let mut staged = check_file(StagedFile::create(&dest), "stage")?;
    let capacity = size / 3;
    let result = {
        let mut limited = DiskFull::new(&mut staged, capacity);
        write_pattern(&mut limited, size)
    };
    let err = result.expect_err("injected disk full must fail the write");
    assert_eq!(err.kind(), io::ErrorKind::StorageFull);

    staged.abort().expect("abort unsealed staged");
    assert!(!exists(&dest), "faulted stage must not publish");
    assert_eq!(
        children(dir.path()),
        Vec::<PathBuf>::new(),
        "abort leaves no residue"
    );
    Ok(())
}

/// Scenario E — post-seal temp delete at the boundary size: the
/// externally removed staging temp makes publish fail
/// `NotPublished`, and the recovery handle reports
/// `TempState::Absent` on cleanup (the `staged_lifecycle`
/// temp-delete contract at 4GiB scale).
fn scenario_post_seal_temp_delete(size: u64) -> Result<(), NotRun> {
    let dir = TestDir::new("g01-temp-delete");
    let dest = dir.dest("payload.bin");

    let mut staged = check_file(StagedFile::create(&dest), "stage")?;
    check_io(write_pattern(&mut staged, size), "write")?;
    let sealed = staged.seal();
    let temp = sealed
        .temp_path()
        .map(PathBuf::from)
        .expect("os backend exposes the temp path");

    check_io(std::fs::remove_file(&temp), "external temp delete")?;

    let mut failure = sealed
        .publish(PublishOptions::new(
            PublishMode::CreateNew,
            Durability::BestEffort,
        ))
        .expect_err("publish cannot succeed without its temp file");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert!(!exists(&dest), "failed publish must not create destination");

    let handle = failure
        .take_recovery_handle()
        .expect("failed publish carries its recovery handle");
    let receipt = handle.cleanup().expect("cleanup of absent temp succeeds");
    assert_eq!(receipt.temp_state(), TempState::Absent);
    assert!(!exists(&temp));
    assert_eq!(children(dir.path()), Vec::<PathBuf>::new());
    Ok(())
}

/// One G01 scenario: drives `size` bytes through a boundary case.
/// `Err(NotRun)` is the honest capacity skip — never Pass/Fail.
type Scenario = fn(u64) -> Result<(), NotRun>;

const SCENARIOS: &[(&str, Scenario)] = &[
    ("create_new_roundtrip", scenario_create_new_roundtrip),
    ("replace_roundtrip", scenario_replace_roundtrip),
    (
        "create_new_conflict_recovery",
        scenario_create_new_conflict_recovery,
    ),
    ("mid_write_fault_abort", scenario_mid_write_fault_abort),
    ("post_seal_temp_delete", scenario_post_seal_temp_delete),
];

/// The whole G01 lane — one `#[ignore]`d test so the opt-in entry
/// point is a single deterministic, sequential run.
#[test]
#[ignore = "opt-in real-4GiB boundary lane — run `mise run test ruviform-io-e2e --test large_staged -- --ignored --nocapture`"]
fn large_staged_4gib_boundary() {
    // Largest payload + headroom — the preflight probe must cover the
    // worst case, not an average.
    let required = 4 * GIB + 1 + PROBE_MARGIN;
    let temp_dir = std::env::temp_dir();
    eprintln!(
        "G01 lane: sizes={SIZES:?} required_probe_bytes={required} temp_dir={}",
        temp_dir.display()
    );

    // Preflight: actually allocate the largest case's footprint once.
    // Failure means the environment cannot host the lane — every
    // case is NotRun, reported honestly, never Pass/Fail.
    {
        let probe_dir = TestDir::new("g01-preflight");
        match probe_capacity(probe_dir.path(), required) {
            Ok(()) => eprintln!("G01 preflight: probe allocation OK"),
            Err(err) => {
                eprintln!(
                    "G01 preflight: NOT RUN — probe failed: {err:?} \
                     (required={required} bytes, temp_dir={})",
                    temp_dir.display()
                );
                return;
            }
        }
    }

    for &size in &SIZES {
        for &(id, scenario) in SCENARIOS {
            match scenario(size) {
                Ok(()) => eprintln!("G01 {size} {id}: PASS"),
                Err(not_run) => {
                    let reason = not_run.reason;
                    eprintln!("G01 {size} {id}: NOT RUN — {reason}");
                    eprintln!(
                        "G01: remaining cases NOT RUN — storage exhausted \
                         (temp_dir={}, required_probe_bytes={required})",
                        temp_dir.display()
                    );
                    return;
                }
            }
        }
    }
    eprintln!("G01 lane: all cases PASS");
}
