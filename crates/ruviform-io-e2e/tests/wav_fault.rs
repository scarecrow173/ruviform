//! WAV → `StagedFile` fault fixtures (RF305 slice).
//!
//! `wav_staged.rs` proves the happy-path hand-off; this file injects
//! the faults a real pipeline hits between `WavMuxer` and publish:
//! flush/write `Io` during finalize or mid-stream, cancellation
//! after seal, muxer `Drop` best-effort finalize, and the publish
//! outcome trichotomy (`Published` / `NotPublished` /
//! not-provably-unpublished). Every wrapper is a public-API-only
//! `MediaSink` over `&mut StagedFile` — no crate internals.
//!
//! Real-OS limits are honest: `Published`/`Indeterminate` *failure*
//! effects and post-publish directory-sync failures cannot be
//! induced through the public surface (constructors are
//! crate-private) — those classes stay `NotRun` here and are covered
//! by `ruviform-io-file`'s own mock suite.

mod common;

use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

use common::wav::{independent_decode, packet, s16_payload, s16_stereo_spec};
use common::{TestDir, children, exists, read};
use ruviform_io_file::{
    Durability, DurabilityEvidence, FileErrorKind, PublishEffect, PublishFailure,
    PublishFailureKind, PublishMode, PublishOptions, PublishPhase, PublishReceipt, StagedFile,
};
use ruviform_io_wav::WavMuxer;
use ruviform_media_io::{ContainerFormat, ErrorKind, Muxer};

/// `MediaSink` over `&mut StagedFile` that fails every `flush` with
/// `StorageFull` — the finalize stage's last step.
struct FlushFail<'a> {
    inner: &'a mut StagedFile,
}

impl Write for FlushFail<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::StorageFull,
            "injected flush failure",
        ))
    }
}

impl Seek for FlushFail<'_> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// `MediaSink` over `&mut StagedFile` that accepts at most `capacity`
/// bytes, then fails writes with `StorageFull` — the disk-full
/// injection point. `Write::write_all`'s partial-write loop turns a
/// mid-buffer exhaustion into an `Err`, exactly like a full disk.
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
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let remaining = self.capacity.saturating_sub(self.written);
        if remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "injected disk full",
            ));
        }
        let take = usize::try_from(remaining.min(u64::try_from(buf.len()).expect("len")))
            .expect("take fits usize");
        let n = self.inner.write(&buf[..take])?;
        self.written += u64::try_from(n).expect("written");
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl Seek for DiskFull<'_> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// Publish outcome trichotomy — the only honest classification a
/// consumer may act on. `NotPublished` is the *sole* bucket that may
/// be reported as "definitively not published"; `Published`,
/// `Indeterminate`, and any future `PublishEffect` variant all land
/// in the safe "cannot prove unpublished" bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Published,
    NotPublished,
    NotProvablyUnpublished,
}

fn classify(result: &Result<PublishReceipt, PublishFailure>) -> Outcome {
    match result {
        Ok(_) => Outcome::Published,
        Err(failure) => match failure.effect() {
            PublishEffect::NotPublished => Outcome::NotPublished,
            PublishEffect::Published | PublishEffect::Indeterminate => {
                Outcome::NotProvablyUnpublished
            }
            // `PublishEffect` is `#[non_exhaustive]` — unknown future
            // variants must never collapse into "unpublished".
            _ => Outcome::NotProvablyUnpublished,
        },
    }
}

/// Stages `payload` through `WavMuxer` on the staged file, runs the
/// checked finalize, releases the borrow via `into_sink`, seals, and
/// publishes with `options` — returning the publish result for the
/// classifier.
fn stage_finalize_seal_publish(
    mut staged: StagedFile,
    payload: Vec<u8>,
    options: PublishOptions,
) -> Result<PublishReceipt, PublishFailure> {
    let spec = s16_stereo_spec();
    {
        let mut muxer = WavMuxer::new(&mut staged, &[spec.stream_info()]).expect("muxer");
        muxer
            .write_packet(packet(0, 4, payload))
            .expect("write packet");
        muxer.finalize().expect("finalize");
        let _staged_ref = muxer.into_sink();
    }
    staged.seal().publish(options)
}

/// `finalize`'s trailing `flush` fails → `ErrorKind::Io`, no
/// `WavReceipt`, and the staged temp is recoverable through
/// `into_sink` for an honest abort — never a publish.
#[test]
fn finalize_flush_failure_is_io_and_never_publishes() {
    let dir = TestDir::new("wav-flush-fail");
    let dest = dir.dest("flush.wav");
    let spec = s16_stereo_spec();
    let payload = s16_payload(256, 2);

    let mut staged = StagedFile::create(&dest).expect("stage");
    let result = {
        let mut muxer =
            WavMuxer::new(FlushFail { inner: &mut staged }, &[spec.stream_info()]).expect("muxer");
        muxer
            .write_packet(packet(0, 4, payload))
            .expect("packet write must reach the sink");
        let outcome = muxer.finalize();
        assert!(
            muxer.receipt().is_none(),
            "failed finalize must not mint a receipt"
        );
        // Borrow recovery: the sink wrapper comes back out and its
        // drop releases `&mut StagedFile` for the abort below.
        let _sink = muxer.into_sink();
        outcome
    };
    let err = result.expect_err("flush failure must fail finalize");
    assert_eq!(err.kind(), ErrorKind::Io);

    staged.abort().expect("abort unsealed staged");
    assert!(!exists(&dest), "no destination may appear");
    assert_eq!(
        children(dir.path()).len(),
        0,
        "staging temp must be cleaned by abort"
    );
}

/// Disk full *during* `finalize`'s RIFF/data backpatch write — a
/// distinct injection point from flush: the size fields cannot be
/// patched, so the container would be corrupt even if it flushed.
#[test]
fn finalize_backpatch_write_failure_is_io() {
    let dir = TestDir::new("wav-backpatch-fail");
    let dest = dir.dest("full.wav");
    let spec = s16_stereo_spec();
    let payload = s16_payload(256, 2); // 1024 bytes, even → no pad
    // Header (44) + payload (1024) + 1 byte of the 4-byte riff-size
    // backpatch — the write_all loop then hits the wall.
    let capacity = 44 + u64::try_from(payload.len()).expect("len") + 1;

    let mut staged = StagedFile::create(&dest).expect("stage");
    let result = {
        let mut muxer = WavMuxer::new(DiskFull::new(&mut staged, capacity), &[spec.stream_info()])
            .expect("muxer");
        muxer
            .write_packet(packet(0, 4, payload))
            .expect("packet fits inside capacity");
        let outcome = muxer.finalize();
        assert!(muxer.receipt().is_none());
        let _sink = muxer.into_sink();
        outcome
    };
    let err = result.expect_err("backpatch write failure must fail finalize");
    assert_eq!(err.kind(), ErrorKind::Io);

    staged.abort().expect("abort");
    assert!(!exists(&dest));
    assert_eq!(children(dir.path()).len(), 0);
}

/// Disk full *mid-stream*: `write_packet` gets `ErrorKind::Io`, the
/// payload counter is untouched, and a follow-up `finalize` fails
/// the same way — an honest "not completed", never a partial
/// receipt.
#[test]
fn disk_full_during_packet_write_is_io_and_never_publishes() {
    let dir = TestDir::new("wav-write-fail");
    let dest = dir.dest("diskfull.wav");
    let spec = s16_stereo_spec();
    let payload = s16_payload(256, 2);
    // Room for the 44-byte header plus only 8 payload bytes.
    let capacity = 44 + 8;

    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        let mut muxer = WavMuxer::new(DiskFull::new(&mut staged, capacity), &[spec.stream_info()])
            .expect("muxer");
        let err = muxer
            .write_packet(packet(0, 4, payload))
            .expect_err("disk full must fail the packet write");
        assert_eq!(err.kind(), ErrorKind::Io);
        let err = muxer
            .finalize()
            .expect_err("finalize cannot succeed on a full disk");
        assert_eq!(err.kind(), ErrorKind::Io);
        assert!(muxer.receipt().is_none());
        // Drop runs best-effort finalize — its failure is silent by
        // design and must not be mistaken for completion.
    }
    staged.abort().expect("abort");
    assert!(!exists(&dest));
    assert_eq!(children(dir.path()).len(), 0);
}

/// Cancel after a *successful* finalize: the receipt exists, the
/// bytes are sealed — and the caller still chooses not to publish.
/// Abort removes the temp; nothing reaches the destination.
#[test]
fn abort_after_finalize_never_publishes() {
    let dir = TestDir::new("wav-cancel");
    let dest = dir.dest("cancel.wav");
    let payload = s16_payload(128, 2);

    let mut staged = StagedFile::create(&dest).expect("stage");
    let spec = s16_stereo_spec();
    {
        let mut muxer = WavMuxer::new(&mut staged, &[spec.stream_info()]).expect("muxer");
        muxer
            .write_packet(packet(0, 4, payload.clone()))
            .expect("packet");
        let receipt = muxer.finalize().expect("finalize");
        assert_eq!(
            receipt.total_bytes,
            u64::try_from(payload.len()).expect("len")
        );
        let _staged_ref = muxer.into_sink();
    }
    let temp = staged.temp_path().map(PathBuf::from);
    staged.seal().abort().expect("abort sealed");

    assert!(!exists(&dest), "cancel must not publish");
    if let Some(temp) = temp {
        assert!(!exists(&temp), "abort removes the staging temp");
    }
    assert_eq!(children(dir.path()).len(), 0);
}

/// `Drop` best-effort finalize is *not* completion evidence: with no
/// `finalize()` call there is no `WavReceipt`, yet on a healthy
/// filesystem the dropped muxer still backpatches a decodable file.
/// This fixture pins exactly that trap — the bytes verify, the
/// receipt never existed, and the honest path is still abort, not
/// publish.
#[test]
fn muxer_drop_best_effort_finalize_is_not_completion_evidence() {
    let dir = TestDir::new("wav-drop");
    let dest = dir.dest("drop.wav");
    let payload = s16_payload(128, 2);
    let spec = s16_stereo_spec();

    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        let mut muxer = WavMuxer::new(&mut staged, &[spec.stream_info()]).expect("muxer");
        muxer
            .write_packet(packet(0, 4, payload.clone()))
            .expect("packet");
        // No finalize(): `receipt()` stays `None` and `Drop` runs
        // `finalize_inner` best-effort — its result is unobservable.
        assert!(muxer.receipt().is_none());
    }

    // The incidental bytes *do* decode — which is precisely why
    // "drop produced a valid-looking file" must never be treated as
    // proof of completion. There is no receipt to gate publish on.
    let mut sealed = staged.seal();
    let decoded: Vec<u8> = {
        let mut readback = sealed.open_readback().expect("readback");
        ruviform_io_symphonia::with_borrowed_demuxer(&mut readback, |d| {
            let mut out = Vec::new();
            while let Some(pkt) = d.read_packet()? {
                out.extend_from_slice(&pkt.data);
            }
            Ok(out)
        })
        .expect("best-effort bytes decode on a healthy FS")
    };
    assert_eq!(decoded, payload);
    sealed.abort().expect("abort — no receipt, no publish");
    assert!(!exists(&dest));
    assert_eq!(children(dir.path()).len(), 0);
}

/// Successful publish classifies as `Published` — and the
/// destination holds the byte-exact WAV.
#[test]
fn publish_success_classifies_as_published() {
    let dir = TestDir::new("wav-pub-ok");
    let dest = dir.dest("ok.wav");
    let payload = s16_payload(256, 2);

    let result = stage_finalize_seal_publish(
        StagedFile::create(&dest).expect("stage"),
        payload.clone(),
        PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
    );
    assert_eq!(classify(&result), Outcome::Published);
    let receipt = result.expect("publish receipt");
    assert_eq!(receipt.durability(), DurabilityEvidence::FileSynced);
    assert_eq!(receipt.destination(), dest.as_path());
    let published = read(&dest).expect("published bytes");
    let (format, decoded) = independent_decode(&published).expect("decode");
    assert_eq!(format, ContainerFormat::Wav);
    assert_eq!(decoded, payload);
    assert_eq!(children(dir.path()), vec![dest.clone()]);
}

/// `CreateNew` conflict: the *only* real-OS failure effect this
/// slice can induce — `DestinationExists` at `PublishPhase::Publish`
/// with `NotPublished`, incumbent bytes preserved, retained temp
/// recovered through the owned handle.
#[test]
fn publish_conflict_classifies_as_not_published() {
    let dir = TestDir::new("wav-pub-conflict");
    let dest = dir.dest("contended.wav");

    // Incumbent wins the destination before our publish.
    std::fs::write(&dest, b"incumbent-wav").expect("seed destination");

    let result = stage_finalize_seal_publish(
        StagedFile::create(&dest).expect("stage"),
        s16_payload(64, 2),
        PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
    );
    assert_eq!(classify(&result), Outcome::NotPublished);

    let mut failure = result.expect_err("conflict must fail publish");
    assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
    assert_eq!(failure.phase(), PublishPhase::Publish);
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(read(&dest).as_deref(), Some(b"incumbent-wav".as_slice()));
    if let Some(handle) = failure.take_recovery_handle() {
        let _ = handle.cleanup();
    }
    assert_eq!(children(dir.path()), vec![dest.clone()]);
}

/// `Replace` publish over an existing destination also classifies
/// `Published` — mode coverage for the classifier.
#[test]
fn publish_replace_classifies_as_published() {
    let dir = TestDir::new("wav-pub-replace");
    let dest = dir.dest("replace.wav");
    std::fs::write(&dest, b"old-owner").expect("seed destination");

    let result = stage_finalize_seal_publish(
        StagedFile::create(&dest).expect("stage"),
        s16_payload(64, 2),
        PublishOptions::new(PublishMode::Replace, Durability::FileSync),
    );
    assert_eq!(classify(&result), Outcome::Published);
    let (format, _) =
        independent_decode(&read(&dest).expect("published")).expect("published wav decodes");
    assert_eq!(format, ContainerFormat::Wav);
    assert_eq!(children(dir.path()), vec![dest.clone()]);
}

/// `StagedFile::create` failure is a typed `FileError` before any
/// temp or destination exists — the earliest honest failure class.
#[test]
fn staged_create_failure_is_typed_and_residue_free() {
    let dir = TestDir::new("wav-create-fail");
    let dest = dir.dest("missing").join("out.wav");

    let err = StagedFile::create(&dest).expect_err("missing parent must fail");
    assert_eq!(err.kind(), FileErrorKind::Io);
    assert!(!exists(&dir.dest("missing")), "no directory may be created");
    assert_eq!(children(dir.path()).len(), 0);
}
