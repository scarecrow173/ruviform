//! IO composition performance baseline (Issue #306 G05) — criterion
//! bench for the `ruviform-io-e2e` staged-publish composition paths.
//!
//! ## Pre-defined measurement contract (pinned before implementation)
//!
//! - **Format**: staged raw bytes (`StagedFile` write / seal /
//!   publish / readback) and TAR→`gzip`/`zstd` `Encoder`→
//!   `&mut StagedFile` composite encode. WAV / structured /
//!   secure-extraction paths are out of scope (G05 covers the IO
//!   composition representative paths only).
//! - **Input size**: `INPUT_BYTES = 8 MiB` staged payload;
//!   `TAR_TOTAL_BYTES = 8 MiB` across `TAR_ENTRIES = 4` entries of
//!   2 MiB each. 4 GiB is unsuitable for iterative measurement —
//!   that boundary is G01's lane (`tests/large_staged.rs`).
//! - **Feature / Profile**: `bench` profile (release-derived).
//!   io-e2e dev-dependency feature set applies as-is; note that
//!   `ruviform-io-file`'s `fault-injection` feature is unified into
//!   this build via the dev-dep — it adds `#[cfg]`-gated code only
//!   and does not alter the hot path.
//! - **FS / OS**: real filesystem under `std::env::temp_dir()`
//!   (environment-dependent — path printed in the env block and
//!   recorded in `evidence/rf306/`).
//! - **Baseline revision**: the git SHA of the measured tree is
//!   recorded in the evidence report by the runner (the bench prints
//!   package/criterion/rustc versions, not the SHA).
//! - **Regression thresholds (provisional, defined up front)**:
//!   same-machine, same-profile reruns should stay within **±25%**
//!   of the recorded baseline throughput; a larger deviation is an
//!   investigation trigger, not an automatic fail. `peak_alloc`
//!   has a hard sanity bound of **1 GiB** additional live heap per
//!   pipeline run — exceeding it indicates whole-input buffering or
//!   a leak-class regression and is a **Fail**. Consumer-specific
//!   thresholds (f64/RT/long-media/project-save) remain owned by the
//!   consumer Issues and are *not* replaced by this baseline.
//! - **Measurement variance**: criterion's mean/median/throughput/
//!   CV are transcribed into evidence; cells with high CV are
//!   recorded as noisy rather than hidden.
//! - **Unacceptable outcomes**: panic, `peak_alloc` bound violation,
//!   or environment-caused storage exhaustion (NotRun-equivalent,
//!   recorded honestly — never Pass/Fail disguised).
//!
//! ## Cells (`io_composition` group)
//!
//! Fixture setup (create/write/seal/seed) always runs **outside**
//! the timed window via `iter_batched(_ref)` +
//! `BatchSize::PerIteration`; each timed iteration is exactly one
//! routine call on a fresh fixture, and fixture teardown (temp
//! cleanup, destination removal where applicable) is likewise
//! outside the window except where noted.
//!
//! | cell               | timed routine                                        | throughput        |
//! |--------------------|------------------------------------------------------|-------------------|
//! | `staged_write`     | `write_all` of `INPUT_BYTES` pattern                 | Bytes(INPUT)      |
//! | `staged_seal`      | `seal()` type transition (no I/O — expected ≈ 0)     | —                 |
//! | `staged_publish`   | `publish(CreateNew, FileAndDirectorySync)` + `remove_file(dest)` (per-iter dest hygiene, included) | Bytes(INPUT) |
//! | `staged_readback`  | `open_readback` + chunked read + FNV-1a verify       | Bytes(INPUT)      |
//! | `tar_gzip_staged`  | TAR 4×2 MiB → `gzip::Encoder` → finish proofs        | Bytes(TAR total)  |
//! | `tar_zstd_staged`  | same via `zstd::Encoder`                             | Bytes(TAR total)  |
//!
//! `staged_seal` uses a 4 KiB fixture on purpose: `seal` is a pure
//! type transition whose cost must not depend on payload size, so a
//! small fixture keeps setup cheap while still measuring the real
//! call. Its expected result is ≈ 0 — the cell exists to *show* the
//! transition is free, not to hide it.
//!
//! ## Peak allocation probe
//!
//! `peak_alloc` is measured by a counting `GlobalAlloc` wrapping
//! `System`. Live-byte accounting runs for the whole bench process
//! (two relaxed atomics per alloc/dealloc pair — negligible next to
//! the ms-scale I/O being measured, and confined to this bench
//! binary). A window flag gates only the peak tracker: probes run
//! once per annotated cell *before* `b.iter*`, outside criterion's
//! timing, enabling the window for 3 pipeline repetitions and
//! printing `live_before` / `peak_delta` to stdout for evidence
//! transcription. Only **heap** allocations through the global
//! allocator are counted — mmap, OS buffers, and page cache are out
//! of scope and documented as such.
//!
//! ## Environment metadata
//!
//! Printed to stdout exactly once at first bench (same convention
//! as `ruviform-rt`'s `rt_observability_poc`): OS/arch, CPU model,
//! core count, rustc, criterion (read from `Cargo.lock`), crate
//! version, temp dir.
//!
//! Run: `mise run bench` (`cargo bench --workspace`). Filter:
//! `cargo bench -p ruviform-io-e2e -- io_composition/<cell>`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Once, OnceLock};
use std::time::Duration;

use criterion::{
    BatchSize, BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main,
    measurement::WallTime,
};
use ruviform_io_archive::tar::{TarArchiveWriter, TarEntryOptions};
use ruviform_io_compression::{gzip, zstd};
use ruviform_io_file::{Durability, PublishMode, PublishOptions, SealedFile, StagedFile};

/// Staged payload size for write/readback/publish cells: 8 MiB.
const INPUT_BYTES: u64 = 8 * 1024 * 1024;

/// TAR composite payload: 4 entries × 2 MiB = 8 MiB total.
const TAR_ENTRIES: usize = 4;
const TAR_ENTRY_BYTES: u64 = 2 * 1024 * 1024;

/// Small payload for the `seal` cell — seal cost is payload-size
/// independent, so a tiny fixture keeps per-iteration setup cheap.
const SEAL_FIXTURE_BYTES: u64 = 4 * 1024;

/// Chunk size for pattern generation / checksum — bounds heap.
const CHUNK: usize = 1024 * 1024;

/// Repetitions inside one peak-alloc probe window.
const PEAK_REPS: usize = 3;

/// FNV-1a 64-bit constants (std-only checksum, mirrors the G01 lane).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

// ---------------------------------------------------------------------------
// Deterministic payload + checksum helpers (self-contained: bench targets
// cannot reach `tests/common`; duplication is deliberate, deps stay zero).
// ---------------------------------------------------------------------------

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Fills `buf` with the position-dependent pattern for absolute
/// offsets `base..`; `base` must be 8-aligned (chunk bases always
/// are — same contract as the G01 lane).
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

fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Writes `size` pattern bytes to `w`, returning the stream FNV-1a.
fn write_pattern<W: Write>(w: &mut W, size: u64) -> u64 {
    let mut buf = vec![0_u8; CHUNK];
    let mut hash = FNV_OFFSET;
    let mut offset = 0_u64;
    let chunk_u64 = u64::try_from(CHUNK).expect("chunk fits u64");
    while offset < size {
        let n = usize::try_from((size - offset).min(chunk_u64)).expect("chunk fits usize");
        fill_pattern(&mut buf[..n], offset);
        w.write_all(&buf[..n]).expect("pattern write");
        hash = fnv1a_update(hash, &buf[..n]);
        offset += u64::try_from(n).expect("n fits u64");
    }
    w.flush().expect("pattern flush");
    hash
}

/// Chunked FNV-1a over exactly `size` bytes read from `r`.
fn checksum_stream<R: Read>(r: &mut R, size: u64) -> u64 {
    let mut buf = vec![0_u8; CHUNK];
    let mut hash = FNV_OFFSET;
    let mut total = 0_u64;
    let chunk_u64 = u64::try_from(CHUNK).expect("chunk fits u64");
    while total < size {
        let want = usize::try_from((size - total).min(chunk_u64)).expect("chunk fits usize");
        r.read_exact(&mut buf[..want]).expect("checksum read");
        hash = fnv1a_update(hash, &buf[..want]);
        total += u64::try_from(want).expect("want fits u64");
    }
    hash
}

// ---------------------------------------------------------------------------
// Bench workspace — a unique temp dir per process, removed on exit.
// ---------------------------------------------------------------------------

/// Unique bench temp dir under the system temp dir; recursive
/// cleanup on drop (mirrors `tests/common::TestDir`'s RAII shape —
/// bench targets cannot reach `tests/common`).
struct BenchDir {
    path: PathBuf,
}

impl BenchDir {
    fn new() -> Self {
        let base = std::env::temp_dir();
        for _ in 0..64 {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX));
            let path = base.join(format!(
                "rvf-io-e2e-bench-{}-{nanos:016x}",
                std::process::id()
            ));
            if std::fs::create_dir(&path).is_ok() {
                return Self { path };
            }
        }
        panic!(
            "could not create a unique bench temp dir under {}",
            base.display()
        );
    }

    fn dest(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for BenchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Process-wide bench workspace, created lazily on first use.
fn bench_dir() -> &'static BenchDir {
    static DIR: OnceLock<BenchDir> = OnceLock::new();
    DIR.get_or_init(BenchDir::new)
}

// ---------------------------------------------------------------------------
// Counting allocator — live-byte tracking for the peak-alloc probe.
// ---------------------------------------------------------------------------

static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static WINDOW_BASE: AtomicU64 = AtomicU64::new(0);
static WINDOW_PEAK: AtomicU64 = AtomicU64::new(0);
static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

/// `GlobalAlloc` wrapper delegating to `System` while tracking live
/// heap bytes. `realloc`/`alloc_zeroed` use the trait's provided
/// defaults (alloc+copy+dealloc), which keeps the counters balanced
/// without extra unsafe surface.
///
/// SAFETY: every call forwards to `System` with the caller's layout
/// unchanged; the atomics only observe sizes — no pointer provenance
/// or aliasing assumption is introduced.
struct CountingAlloc;

#[expect(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: identical forwarding contract as `System`.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let size = u64::try_from(layout.size()).expect("layout size fits u64");
            let live = LIVE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
            if WINDOW_OPEN.load(Ordering::Relaxed) {
                let excess = live.saturating_sub(WINDOW_BASE.load(Ordering::Relaxed));
                WINDOW_PEAK.fetch_max(excess, Ordering::Relaxed);
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: identical forwarding contract as `System`; the
        // size was counted by `alloc` under the same layout.
        unsafe { System.dealloc(ptr, layout) };
        let size = u64::try_from(layout.size()).expect("layout size fits u64");
        LIVE_BYTES.fetch_sub(size, Ordering::Relaxed);
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// Runs `f` `PEAK_REPS` times inside a peak-tracking window and
/// returns `(live_before, peak_delta)` — the maximum *additional*
/// live heap bytes observed above the window baseline. Called once
/// per annotated bench cell, before `b.iter*` — outside criterion
/// timing.
fn measure_peak<F: FnMut()>(mut f: F) -> (u64, u64) {
    let live_before = LIVE_BYTES.load(Ordering::Relaxed);
    WINDOW_BASE.store(live_before, Ordering::Relaxed);
    WINDOW_PEAK.store(0, Ordering::Relaxed);
    WINDOW_OPEN.store(true, Ordering::Relaxed);
    for _ in 0..PEAK_REPS {
        f();
    }
    WINDOW_OPEN.store(false, Ordering::Relaxed);
    (live_before, WINDOW_PEAK.load(Ordering::Relaxed))
}

fn print_peak_probe(cell: &str, live_before: u64, peak_delta: u64) {
    println!(
        "io_composition peak_alloc {cell}: live_before={live_before} \
         peak_delta={peak_delta} (heap only, reps={PEAK_REPS})"
    );
}

// ---------------------------------------------------------------------------
// Environment metadata — printed once, transcribed into evidence.
// ---------------------------------------------------------------------------

fn print_env_metadata() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        println!("=== io_composition bench environment ===");
        println!("os: {} {}", std::env::consts::OS, std::env::consts::ARCH);
        println!("cpu: {}", cpu_model());
        println!("cores: {}", core_count());
        println!("rustc: {}", rustc_version());
        println!("criterion: {}", criterion_version());
        println!("ruviform-io-e2e: {}", env!("CARGO_PKG_VERSION"));
        println!("temp_dir: {}", std::env::temp_dir().display());
    });
}

#[cfg(target_os = "windows")]
fn cpu_model() -> String {
    if let Ok(out) = std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0",
            "/v",
            "ProcessorNameString",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if line.contains("ProcessorNameString")
                && let Some((_, name)) = line.split_once("REG_SZ")
            {
                return name.trim().to_string();
            }
        }
    }
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(target_os = "linux")]
fn cpu_model() -> String {
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            if let Some((key, value)) = line.split_once(':')
                && key.trim() == "model name"
            {
                return value.trim().to_string();
            }
        }
    }
    "unknown".to_string()
}

#[cfg(target_os = "macos")]
fn cpu_model() -> String {
    if let Ok(out) = std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        && out.status.success()
    {
        return String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    "unknown".to_string()
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn cpu_model() -> String {
    "unknown".to_string()
}

fn core_count() -> String {
    std::thread::available_parallelism()
        .map_or_else(|_| "unknown".to_string(), |n| n.get().to_string())
}

fn rustc_version() -> String {
    std::process::Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string()))
        .arg("--version")
        .output()
        .map_or_else(
            |_| "unknown".to_string(),
            |out| String::from_utf8_lossy(&out.stdout).trim().to_string(),
        )
}

/// criterion version actually linked — read from the workspace
/// `Cargo.lock` (reproducibility anchor), same as the ruviform-rt
/// observability bench.
fn criterion_version() -> String {
    let lock = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let Ok(text) = std::fs::read_to_string(lock) else {
        return "unknown".to_string();
    };
    let mut found = false;
    for line in text.lines().map(str::trim) {
        if found && let Some(rest) = line.strip_prefix("version = ") {
            return rest.trim_matches('"').to_string();
        }
        found = line == "name = \"criterion\"";
    }
    "unknown".to_string()
}

// ---------------------------------------------------------------------------
// Shared fixture builders.
// ---------------------------------------------------------------------------

fn publish_options() -> PublishOptions {
    PublishOptions::new(PublishMode::CreateNew, Durability::FileAndDirectorySync)
}

/// Creates a staged file at `dest` inside the bench dir.
fn stage(dest: &Path) -> StagedFile {
    StagedFile::create(dest).expect("stage")
}

/// Writes `size` pattern bytes into `staged`, seals, and returns
/// `(SealedFile, expected_hash)` — fixture setup for readback.
fn sealed_with_payload(dir: &BenchDir, name: &str, size: u64) -> (SealedFile, u64) {
    let mut staged = stage(&dir.dest(name));
    let hash = write_pattern(&mut staged, size);
    (staged.seal(), hash)
}

/// TAR composite into any `Write` sink: 4 entries of deterministic
/// pattern payloads, all completion proofs (`entry.finish`,
/// `tar.finish`) inside the timed routine. Returns the sink.
fn tar_entries_into<W: Write>(sink: W) -> W {
    let mut buf = vec![0_u8; CHUNK];
    let chunk_u64 = u64::try_from(CHUNK).expect("chunk fits u64");
    let mut tar = TarArchiveWriter::new(sink);
    for entry_idx in 0..TAR_ENTRIES {
        let name = format!("entry-{entry_idx}.bin");
        let mut entry = tar
            .start_entry(&name, TAR_ENTRY_BYTES, &TarEntryOptions::default())
            .expect("tar entry");
        let mut written = 0_u64;
        while written < TAR_ENTRY_BYTES {
            let n =
                usize::try_from((TAR_ENTRY_BYTES - written).min(chunk_u64)).expect("chunk fits");
            let base =
                u64::try_from(entry_idx).expect("entry idx fits u64") * TAR_ENTRY_BYTES + written;
            fill_pattern(&mut buf[..n], base);
            entry.write_all(&buf[..n]).expect("entry payload");
            written += u64::try_from(n).expect("n fits u64");
        }
        entry.finish().expect("entry finish");
    }
    tar.finish().expect("tar finish")
}

// ---------------------------------------------------------------------------
// Benchmark cells.
// ---------------------------------------------------------------------------

fn io_group<'a>(c: &'a mut Criterion, name: &str) -> BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(10);
    group
}

fn bench_staged_write(c: &mut Criterion) {
    print_env_metadata();
    let mut group = io_group(c, "io_composition");
    group.throughput(Throughput::Bytes(INPUT_BYTES));

    // One-shot peak probe (outside criterion timing): full pipeline
    // call including fixture creation — the write cell's heap
    // footprint is dominated by the 1 MiB chunk buffer.
    let (live_before, peak_delta) = measure_peak(|| {
        let dir = bench_dir();
        let mut staged = stage(&dir.dest("peak-write.bin"));
        black_box(write_pattern(&mut staged, INPUT_BYTES));
    });
    print_peak_probe("staged_write", live_before, peak_delta);

    group.bench_function("staged_write", |b| {
        let dir = bench_dir();
        b.iter_batched_ref(
            || stage(&dir.dest("write.bin")),
            |staged| {
                black_box(write_pattern(staged, INPUT_BYTES));
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_staged_seal(c: &mut Criterion) {
    print_env_metadata();
    let mut group = io_group(c, "io_composition");

    group.bench_function("staged_seal", |b| {
        let dir = bench_dir();
        b.iter_batched(
            || {
                let mut staged = stage(&dir.dest("seal.bin"));
                black_box(write_pattern(&mut staged, SEAL_FIXTURE_BYTES));
                staged
            },
            |staged| black_box(staged.seal()),
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_staged_publish(c: &mut Criterion) {
    print_env_metadata();
    let mut group = io_group(c, "io_composition");
    group.throughput(Throughput::Bytes(INPUT_BYTES));

    group.bench_function("staged_publish", |b| {
        let dir = bench_dir();
        let mut counter = 0_u64;
        b.iter_batched(
            || {
                counter += 1;
                // Unique destination per iteration, bound at create —
                // CreateNew must never see a leftover from the
                // previous run, and the routine removes exactly the
                // path publish wrote.
                let dest = dir.dest(&format!("published-{counter}.bin"));
                let mut staged = stage(&dest);
                black_box(write_pattern(&mut staged, INPUT_BYTES));
                (staged.seal(), dest)
            },
            |(sealed, dest)| {
                let receipt = sealed.publish(publish_options()).expect("publish");
                black_box(receipt);
                // Per-iteration destination hygiene — included in the
                // timed window (documented overhead, small vs fsync).
                std::fs::remove_file(&dest).expect("remove published dest");
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_staged_readback(c: &mut Criterion) {
    print_env_metadata();
    let mut group = io_group(c, "io_composition");
    group.throughput(Throughput::Bytes(INPUT_BYTES));

    group.bench_function("staged_readback", |b| {
        let dir = bench_dir();
        b.iter_batched_ref(
            || sealed_with_payload(dir, "readback.bin", INPUT_BYTES),
            |(sealed, expected)| {
                let mut readback = sealed.open_readback().expect("readback");
                let hash = checksum_stream(&mut readback, INPUT_BYTES);
                assert_eq!(hash, *expected, "readback checksum mismatch");
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_tar_gzip(c: &mut Criterion) {
    print_env_metadata();
    let mut group = io_group(c, "io_composition");
    group.throughput(Throughput::Bytes(
        u64::try_from(TAR_ENTRIES).expect("entries fit u64") * TAR_ENTRY_BYTES,
    ));

    // One-shot peak probe (outside criterion timing).
    let (live_before, peak_delta) = measure_peak(|| {
        let dir = bench_dir();
        let mut staged = stage(&dir.dest("peak-tgz.bin"));
        let mut encoder =
            gzip::Encoder::new(&mut staged, gzip::EncodeOptions::default()).expect("encoder");
        tar_entries_into(&mut encoder);
        encoder.finish().expect("gzip finish");
    });
    print_peak_probe("tar_gzip_staged", live_before, peak_delta);

    group.bench_function("tar_gzip_staged", |b| {
        let dir = bench_dir();
        b.iter_batched_ref(
            || stage(&dir.dest("tgz.bin")),
            |staged| {
                let mut encoder =
                    gzip::Encoder::new(staged, gzip::EncodeOptions::default()).expect("encoder");
                tar_entries_into(&mut encoder);
                encoder.finish().expect("gzip finish");
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_tar_zstd(c: &mut Criterion) {
    print_env_metadata();
    let mut group = io_group(c, "io_composition");
    group.throughput(Throughput::Bytes(
        u64::try_from(TAR_ENTRIES).expect("entries fit u64") * TAR_ENTRY_BYTES,
    ));

    group.bench_function("tar_zstd_staged", |b| {
        let dir = bench_dir();
        b.iter_batched_ref(
            || stage(&dir.dest("tzst.bin")),
            |staged| {
                let mut encoder =
                    zstd::Encoder::new(staged, zstd::EncodeOptions::default()).expect("encoder");
                tar_entries_into(&mut encoder);
                encoder.finish().expect("zstd finish");
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_staged_write,
    bench_staged_seal,
    bench_staged_publish,
    bench_staged_readback,
    bench_tar_gzip,
    bench_tar_zstd
);
criterion_main!(benches);
