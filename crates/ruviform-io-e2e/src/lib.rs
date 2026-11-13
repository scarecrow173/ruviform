//! `ruviform-io-e2e` — composition E2E tests for the generic IO
//! crates (IO-IMP-12 / Issue #306).
//!
//! This crate is **test-only**: it publishes no API. Its `tests/`
//! directory exercises the same public surface `AudioLens` and
//! `Synvelis` consume, chaining the independently verified layer
//! contracts into end-to-end staged-publish flows:
//!
//! - `staged_lifecycle` — `StagedFile` → `seal` → `open_readback` →
//!   `publish` → reopen; `CreateNew` conflicts, `Replace` owner
//!   replacement, abort/drop temp cleanup.
//! - `structured_staged` — `json`/`csv` `write_file`/`read_file`
//!   staged convenience, encode/publish failure classification.
//! - `archive_compression_staged` — `TarArchiveWriter` →
//!   `gzip`/`zstd` `Encoder` → `&mut StagedFile`, verified-EOF gating.
//! - `extraction` — published ZIP/TAR → secure
//!   `ExtractionSession` staging trees, path/collision/budget refusal.
//! - `wav_staged` — `WavMuxer<&mut StagedFile>` → `finalize` →
//!   `seal` → readback independent decode → `publish` → reopen.
//! - `wav_golden` — byte-exact RIFF/WAVE goldens for the published
//!   destination (header layouts, wire formats, conversion edge
//!   samples, pad byte, classic RIFF 4 GiB boundary) built from the
//!   spec, never from `ruviform-io-wav` internals (Issue #305).
//! - `wav_fault` — finalize/write fault injection over
//!   `&mut StagedFile`, cancel/drop boundaries, and the publish
//!   outcome trichotomy classifier (Issue #305).
//! - `fault_seam` — publish classification contract tests driven
//!   through the public API only, with faults injected via
//!   `ruviform-io-file`'s opt-in `fault-injection` feature
//!   (`#[doc(hidden)]` mock facade): `Indeterminate`, post-publish
//!   directory-sync failure, `TempState::Unknown`, and per-phase
//!   `NotPublished` (Issue #306 G07).
//! - `corpus_replay` — `#[ignore]`d, opt-in bounded corpus replay:
//!   fixed-seed deterministic mutations of writer-canonical bytes
//!   replayed through every composed reader under byte/decoded/
//!   deadline budgets (`mise run test-io-e2e-corpus`, Issue #306 G08).
//!
//! # RT contract
//!
//! **Non-RT only** — every test performs blocking filesystem,
//! parser, and compression I/O. Nothing here may run on an audio
//! real-time callback.
#![forbid(unsafe_code)]
