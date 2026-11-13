//! G08 bounded corpus replay (Issue #306): deterministic, seed-fixed,
//! budget-bounded mutation replay of writer-produced canonical bytes
//! through every reader surface `ruviform-io-e2e` composes —
//! structured (`json`/`yaml`/`toml`/`csv`/`xml`), compression
//! (`gzip`/`zstd` verified-EOF decode), and archive (`zip`/`tar`
//! verify/drain). **Opt-in only**: the test is `#[ignore]`d and runs
//! via `mise run test-io-e2e-corpus`; the normal `test-io-e2e` lane
//! never executes it.
//!
//! Bounds (all fixed constants below — no external input, no clock
//! dependence in generation):
//!
//! - `CORPUS_SEED`: fixed `u64`; per-format streams derive from it via
//!   splitmix64 — the same seed reproduces the identical corpus.
//! - `CASES_PER_FORMAT`: mutation cases per format (the unmodified
//!   seed is replayed once up front and must be accepted).
//! - `INPUT_CAP`: per-input byte ceiling handed to `InputLimits` /
//!   `DecodeLimits::max_compressed_bytes`.
//! - `DECODED_CAP`: decoded-byte ceiling for gzip/zstd streams.
//! - `archive_limits()`: shrunk `ArchiveReadLimits` for zip/tar.
//! - `DEADLINE`: whole-run wall-clock budget; exceeding it fails the
//!   test loudly (never a silent skip).
//!
//! Format exclusions (documented, not silently skipped): `deflate`
//! (not in io-e2e's compression feature set — io-compression's own
//! matrix lane is authoritative), WAV (Issue #305 golden/fault lane),
//! secure extraction and the `StagedFile` publish FS path (covered by
//! `negative_fuzz`/`fault_boundary`/`fault_seam`), and any sustained /
//! coverage-guided fuzzing (external gate, Deferred).
//!
//! Rules: G08-R1..R4 of `docs/test-specs/issue-306-fault-seam-fuzz.md`.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use ruviform_io_archive::ArchiveReadLimits;
use ruviform_io_archive::tar::{TarArchiveReader, TarArchiveWriter, TarEntryOptions};
use ruviform_io_archive::zip::{
    ZipArchiveReader, ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions,
};
use ruviform_io_compression::{DecodeLimits, DecodeScope, gzip, zstd};
use ruviform_io_structured::{InputLimits, csv, json, toml, xml, yaml};

/// Fixed corpus seed — changing it changes the corpus; record the new
/// value in `evidence/rf306/` when bumping.
const CORPUS_SEED: u64 = 0x5246_3346_555a_5a45; // "RF3FUZZE"

/// Mutation cases generated per format.
const CASES_PER_FORMAT: usize = 48;

/// Per-input compressed/encoded byte ceiling.
const INPUT_CAP: u64 = 256 * 1024;

/// Per-stream decoded byte ceiling (gzip/zstd).
const DECODED_CAP: u64 = 1024 * 1024;

/// Whole-run wall-clock budget — exceeding it fails the test.
const DEADLINE: Duration = Duration::from_secs(120);

/// splitmix64 — deterministic PRNG, no external dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish index in `0..bound` (`bound > 0`).
    fn below(&mut self, bound: usize) -> usize {
        debug_assert!(bound > 0);
        usize::try_from(self.next_u64() % bound as u64).unwrap_or(usize::MAX)
    }

    fn byte(&mut self) -> u8 {
        u8::try_from(self.next_u64() & u64::from(u8::MAX)).unwrap_or_default()
    }
}

/// One deterministic mutation of `seed`: byte flips, truncation,
/// extension with PRNG bytes, or a single-byte splice.
fn mutate(seed: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut out = seed.to_vec();
    match rng.below(4) {
        0 => {
            let flips = 1 + rng.below(4);
            for _ in 0..flips {
                if out.is_empty() {
                    break;
                }
                let i = rng.below(out.len());
                out[i] ^= 1u8 << rng.below(8);
            }
        }
        1 => {
            if out.len() > 1 {
                out.truncate(1 + rng.below(out.len() - 1));
            }
        }
        2 => {
            let extra = rng.below(64);
            out.extend((0..extra).map(|_| rng.byte()));
        }
        _ => {
            if out.len() > 2 {
                let src = rng.below(out.len());
                let dst = rng.below(out.len());
                out[dst] = out[src];
            }
        }
    }
    out
}

/// A replay outcome: `Ok` accepted, `Err(kind)` typed rejection.
type Replay = fn(&[u8]) -> Result<(), String>;

fn document_kind<T>(
    result: Result<T, ruviform_io_structured::DocumentError>,
) -> Result<(), String> {
    result.map(|_| ()).map_err(|e| format!("{:?}", e.kind()))
}

fn archive_kind<T>(result: Result<T, ruviform_io_archive::ArchiveError>) -> Result<(), String> {
    result.map(|_| ()).map_err(|e| format!("{:?}", e.kind()))
}

// ---------------------------------------------------------------------------
// Structured seeds — writer-produced canonical bytes.
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct Doc {
    name: String,
    count: u32,
    tags: Vec<String>,
}

fn doc() -> Doc {
    Doc {
        name: "corpus".to_string(),
        count: 7,
        tags: vec!["alpha".to_string(), "beta".to_string()],
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CsvRow {
    city: String,
    pop: u64,
}

fn seed_json() -> Vec<u8> {
    let mut out = Vec::new();
    json::to_writer(&mut out, &doc(), &json::JsonWriteOptions::default()).expect("json seed");
    out
}

fn seed_yaml() -> Vec<u8> {
    let mut out = Vec::new();
    yaml::to_writer(&mut out, &doc(), &yaml::YamlWriteOptions::new()).expect("yaml seed");
    out
}

fn seed_toml() -> Vec<u8> {
    let mut out = Vec::new();
    toml::to_writer(&mut out, &doc(), &toml::TomlWriteOptions::default()).expect("toml seed");
    out
}

fn seed_csv() -> Vec<u8> {
    let mut out = Vec::new();
    let rows = [
        CsvRow {
            city: "Boston".to_string(),
            pop: 4_628_910,
        },
        CsvRow {
            city: "Austin".to_string(),
            pop: 1_002_316,
        },
    ];
    csv::to_writer(&mut out, &rows, &csv::CsvWriteOptions::default()).expect("csv seed");
    out
}

fn seed_xml() -> Vec<u8> {
    // xml is a read-only event API — the seed is a fixed canonical
    // document, not writer output.
    br#"<?xml version="1.0"?><root><item id="1">alpha</item><item id="2">beta</item></root>"#
        .to_vec()
}

// ---------------------------------------------------------------------------
// Compression/archive seeds — writer-produced canonical bytes.
// ---------------------------------------------------------------------------

fn payload() -> &'static [u8] {
    br#"{"name":"corpus","count":7,"tags":["alpha","beta"]}"#
}

fn seed_gzip() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder =
            gzip::Encoder::new(&mut out, gzip::EncodeOptions::default()).expect("gzip encoder");
        encoder.write_all(payload()).expect("gzip payload");
        encoder.finish().expect("gzip finish");
    }
    out
}

fn seed_zstd() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder =
            zstd::Encoder::new(&mut out, zstd::EncodeOptions::default()).expect("zstd encoder");
        encoder.write_all(payload()).expect("zstd payload");
        encoder.finish().expect("zstd finish");
    }
    out
}

fn seed_zip() -> Vec<u8> {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    for (name, method, body) in [
        ("a.txt", ZipCompressionMethod::Deflate, &b"hello corpus"[..]),
        ("b.txt", ZipCompressionMethod::Stored, &b"second"[..]),
    ] {
        let mut entry = writer
            .start_entry(name, &ZipEntryOptions::new(method))
            .expect("zip entry");
        entry.write_all(body).expect("zip payload");
        entry.finish().expect("zip entry finish");
    }
    writer.finish().expect("zip finish").into_inner()
}

fn seed_tar() -> Vec<u8> {
    let mut writer = TarArchiveWriter::new(Vec::new());
    {
        let mut entry = writer
            .start_entry("a.txt", 12, &TarEntryOptions::default())
            .expect("tar entry");
        entry.write_all(b"hello corpus").expect("tar payload");
        entry.finish().expect("tar entry finish");
    }
    writer.finish().expect("tar finish")
}

// ---------------------------------------------------------------------------
// Replays — every path bounded by the constants above.
// ---------------------------------------------------------------------------

fn input_limits() -> InputLimits {
    InputLimits::new(INPUT_CAP)
}

fn decode_limits() -> DecodeLimits {
    DecodeLimits::new()
        .with_max_compressed_bytes(INPUT_CAP)
        .with_max_decoded_bytes(DECODED_CAP)
        .with_max_units(8)
}

fn archive_limits() -> ArchiveReadLimits {
    let mut limits = ArchiveReadLimits::default();
    limits.max_entries = 64;
    limits.max_metadata_bytes = 1024 * 1024;
    limits.max_path_bytes = 1024;
    limits.max_path_depth = 16;
    limits.max_entry_uncompressed_bytes = 256 * 1024;
    limits.max_total_uncompressed_bytes = 1024 * 1024;
    limits
}

fn replay_json(bytes: &[u8]) -> Result<(), String> {
    let mut input = bytes;
    let options = json::JsonReadOptions::new().with_limits(input_limits());
    document_kind(json::from_reader::<serde::de::IgnoredAny>(
        &mut input, &options,
    ))
}

fn replay_yaml(bytes: &[u8]) -> Result<(), String> {
    let mut input = bytes;
    let mut options = yaml::YamlReadOptions::default();
    options.limits = input_limits();
    document_kind(yaml::from_reader::<serde::de::IgnoredAny>(
        &mut input, &options,
    ))
}

fn replay_toml(bytes: &[u8]) -> Result<(), String> {
    let mut input = bytes;
    let mut options = toml::TomlReadOptions::default();
    options.limits = input_limits();
    document_kind(toml::from_reader::<serde::de::IgnoredAny>(
        &mut input, &options,
    ))
}

fn replay_csv(bytes: &[u8]) -> Result<(), String> {
    let mut input = bytes;
    let mut options = csv::CsvReadOptions::default();
    options.limits = input_limits();
    csv::from_reader::<CsvRow>(&mut input, &options)
        .map(|_| ())
        .map_err(|e| format!("{:?}", e.kind()))
}

fn replay_xml(bytes: &[u8]) -> Result<(), String> {
    let mut options = xml::XmlReadOptions::default();
    options.limits = input_limits();
    options.max_events = Some(20_000);
    let mut input = bytes;
    let mut reader = xml::XmlEventReader::new(&mut input, &options);
    let mut buffer = xml::XmlEventBuffer::new();
    loop {
        match reader.read_event(&mut buffer) {
            Ok(Some(_)) => {}
            Ok(None) => return Ok(()),
            Err(e) => return Err(format!("{:?}", e.kind())),
        }
    }
}

fn replay_gzip(bytes: &[u8]) -> Result<(), String> {
    let mut input = bytes;
    let options = gzip::DecodeOptions::new(DecodeScope::WholeInput, decode_limits());
    let mut decoder = gzip::Decoder::new(&mut input, options);
    let mut sink = Vec::new();
    decoder
        .read_to_end(&mut sink)
        .map_err(|e| format!("io:{:?}", e.kind()))?;
    decoder
        .finish()
        .map(|_| ())
        .map_err(|e| format!("{:?}", e.kind()))
}

fn replay_zstd(bytes: &[u8]) -> Result<(), String> {
    let mut input = bytes;
    let options = zstd::DecodeOptions::new(DecodeScope::WholeInput, decode_limits());
    let mut decoder = zstd::Decoder::new(&mut input, options);
    let mut sink = Vec::new();
    decoder
        .read_to_end(&mut sink)
        .map_err(|e| format!("io:{:?}", e.kind()))?;
    decoder
        .finish()
        .map(|_| ())
        .map_err(|e| format!("{:?}", e.kind()))
}

fn replay_zip(bytes: &[u8]) -> Result<(), String> {
    let mut reader = ZipArchiveReader::new(Cursor::new(bytes), archive_limits())
        .map_err(|e| format!("{:?}", e.kind()))?;
    archive_kind(reader.verify_all())
}

fn replay_tar(bytes: &[u8]) -> Result<(), String> {
    let mut reader = TarArchiveReader::new(Cursor::new(bytes), archive_limits());
    while let Some(mut entry) = reader.next_entry().map_err(|e| format!("{:?}", e.kind()))? {
        let mut sink = Vec::new();
        if entry.read_to_end(&mut sink).is_err() {
            // The entry reader embeds the typed ArchiveError inside the
            // io::Error; finish() re-surfaces it with its real kind.
            return archive_kind(entry.finish());
        }
        archive_kind(entry.finish())?;
    }
    archive_kind(reader.finish())
}

// ---------------------------------------------------------------------------
// Bounded replay driver.
// ---------------------------------------------------------------------------

struct FormatReport {
    name: &'static str,
    accepted: u64,
    rejected: u64,
    panicked: u64,
    kinds: BTreeMap<String, u64>,
}

/// Derives a per-format stream seed from the corpus seed and the
/// format name — deterministic across runs and platforms.
fn name_seed(name: &str) -> u64 {
    name.bytes().fold(CORPUS_SEED, |acc, b| {
        acc.wrapping_mul(0x0000_0100_0000_01B3) ^ u64::from(b)
    })
}

fn run_format(name: &'static str, seed: &[u8], replay: Replay, started: Instant) -> FormatReport {
    // The unmodified writer output must be accepted — the sanity
    // anchor that keeps "rejected" honest (a seed that cannot parse
    // would make every mutation meaningless).
    replay(seed).expect("seed corpus must be accepted by its reader");

    let mut rng = Rng::new(name_seed(name));
    let mut report = FormatReport {
        name,
        accepted: 0,
        rejected: 0,
        panicked: 0,
        kinds: BTreeMap::new(),
    };
    for _ in 0..CASES_PER_FORMAT {
        assert!(
            started.elapsed() < DEADLINE,
            "{name}: corpus replay deadline exceeded"
        );
        let input = mutate(seed, &mut rng);
        match catch_unwind(AssertUnwindSafe(|| replay(&input))) {
            Ok(Ok(())) => report.accepted += 1,
            Ok(Err(kind)) => {
                report.rejected += 1;
                *report.kinds.entry(kind).or_default() += 1;
            }
            Err(_) => report.panicked += 1,
        }
    }
    report
}

/// The whole bounded corpus replay — one `#[ignore]`d test so the
/// opt-in lane is a single deterministic entry point.
#[test]
#[ignore = "opt-in bounded corpus replay — run `mise run test-io-e2e-corpus`"]
fn corpus_replay_bounded() {
    let started = Instant::now();
    // Silence per-case panic noise; a real panic is counted, not
    // hidden — the hook is restored before the result is inspected.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let formats: [(&'static str, Vec<u8>, Replay); 9] = [
            ("json", seed_json(), replay_json),
            ("yaml", seed_yaml(), replay_yaml),
            ("toml", seed_toml(), replay_toml),
            ("csv", seed_csv(), replay_csv),
            ("xml", seed_xml(), replay_xml),
            ("gzip", seed_gzip(), replay_gzip),
            ("zstd", seed_zstd(), replay_zstd),
            ("zip", seed_zip(), replay_zip),
            ("tar", seed_tar(), replay_tar),
        ];
        let mut reports = Vec::with_capacity(formats.len());
        for (name, seed, replay) in &formats {
            reports.push(run_format(name, seed, *replay, started));
        }
        reports
    }));
    let _ = std::panic::take_hook();
    std::panic::set_hook(previous_hook);
    let reports = match outcome {
        Ok(reports) => reports,
        Err(payload) => std::panic::resume_unwind(payload),
    };

    let mut total_panicked = 0_u64;
    for report in &reports {
        total_panicked += report.panicked;
        eprintln!(
            "corpus-replay {:<5} cases={} accepted={} rejected={} panicked={} kinds={:?}",
            report.name,
            CASES_PER_FORMAT,
            report.accepted,
            report.rejected,
            report.panicked,
            report.kinds
        );
    }
    eprintln!(
        "corpus-replay done: seed={CORPUS_SEED:#018x} cases/format={CASES_PER_FORMAT} \
         input_cap={} decoded_cap={} deadline={DEADLINE:?} elapsed={:?}",
        INPUT_CAP,
        DECODED_CAP,
        started.elapsed()
    );
    assert_eq!(
        total_panicked, 0,
        "corpus replay must never panic — a panic is a defect, not a rejection"
    );
}
