//! `SourceReader` over `with_borrowed_demuxer` — the canonical
//! consumer shape for a borrowed sealed readback (Issue #400):
//! `f` receives `&mut dyn Demuxer`, boxes the reborrow into
//! `SourceReader::open(Box::new(d), …)` and runs the existing
//! canonical pump — all inside the scope.

use std::io::Cursor;

use ruviform_media_core::{Fact, MediaTime, TimeBase};
use ruviform_media_io::{DecodeMode, Demuxer, Error, ErrorKind};
use ruviform_media_reader::{SourceFrame, SourceReader};

const RATE: u32 = 48_000;

fn sample_at(i: usize) -> i16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    {
        (i as i32 * 7 % 5000 - 2500) as i16
    }
}

fn wav_bytes(channels: u16, frames: usize) -> Vec<u8> {
    let bits = 16_u16;
    let block_align = channels * (bits / 8);
    let data_len = u32::try_from(frames * usize::from(block_align)).unwrap();
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        for _ in 0..channels {
            bytes.extend_from_slice(&sample_at(i).to_le_bytes());
        }
    }
    bytes
}

/// Canonical pump — the same window pull an owned `SourceReader`
/// runs, draining to `eof`.
fn read_all(reader: &mut SourceReader<'_>, channels: usize) -> Vec<Vec<f32>> {
    const WINDOW: usize = 997;
    let mut out = vec![Vec::new(); channels];
    let mut window = vec![vec![0.0f32; WINDOW]; channels];
    loop {
        let mut slices: Vec<&mut [f32]> = window.iter_mut().map(Vec::as_mut_slice).collect();
        let outcome = reader.read_planar(&mut slices, WINDOW).unwrap();
        for (plane, win) in out.iter_mut().zip(slices.iter()) {
            plane.extend_from_slice(&win[..outcome.frames]);
        }
        if outcome.eof {
            break;
        }
    }
    out
}

/// `&mut dyn Demuxer` satisfies `Demuxer` (the media-io reborrow
/// impl), so `Box::new(d)` is a `Box<dyn Demuxer + 'b>` and
/// `SourceReader::open` accepts it — the whole canonical pipeline
/// runs inside the callback with no encoded copy, and yields output
/// identical to the owned path.
#[test]
fn source_reader_over_borrowed_demuxer_matches_owned() {
    let frames = 3_000usize;
    let channels = 2usize;

    // Owned oracle: SourceReader over an owned symphonia demuxer.
    let owned_demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(
        u16::try_from(channels).unwrap(),
        frames,
    )))
    .unwrap();
    let stream = owned_demuxer.streams()[0].stream.id;
    let mut owned_reader =
        SourceReader::open(Box::new(owned_demuxer), stream, DecodeMode::Canonical).unwrap();
    let expected = read_all(&mut owned_reader, channels);

    // Borrowed path: the same reader built inside `f` over the scoped
    // `&mut dyn Demuxer` reborrow.
    let mut source = Cursor::new(wav_bytes(u16::try_from(channels).unwrap(), frames));
    let borrowed = ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| {
        let stream = d.streams()[0].stream.id;
        let mut reader = SourceReader::open(Box::new(d), stream, DecodeMode::Canonical)?;
        Ok(read_all(&mut reader, channels))
    })
    .expect("borrowed SourceReader decode");

    assert_eq!(
        borrowed, expected,
        "borrowed canonical output must match owned"
    );
    assert_eq!(borrowed[0].len(), frames);
    assert!((borrowed[0][123] - f32::from(sample_at(123)) / 32768.0).abs() < 1e-6);
}

/// `seek_source_frame` inside the callback seeks through the proxy —
/// the receipt reports the real landing — and `next_frame`-driven
/// reads resume from there. A second reader opened on the same
/// scoped demuxer re-decodes the stream head (reopen shape).
#[test]
fn seek_and_reopen_inside_callback() {
    let frames = 2_000usize;
    let target = frames / 2;
    let mut source = Cursor::new(wav_bytes(1, frames));
    let (landed, tail, head) = ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| {
        let stream = d.streams()[0].stream.id;
        let mut reader = SourceReader::open(Box::new(&mut *d), stream, DecodeMode::Canonical)?;
        // The reborrow impl forwards `seekable` honestly — a default
        // `NotDeclared` would be a lie for this WAV source.
        assert!(
            matches!(reader.metadata().seekable, Fact::Known(())),
            "seekable must be forwarded through the reborrow"
        );
        let receipt = reader.seek_source_frame(SourceFrame(u64::try_from(target).unwrap()))?;
        let tail = read_all(&mut reader, 1);
        drop(reader);
        // Reopen shape: `SourceReader::reopen` takes a demuxer
        // positioned at the stream start, so seek the scoped demuxer
        // back through the proxy and open a fresh reader over it.
        let base =
            TimeBase::sample_rate(RATE).map_err(|_| Error::from_kind(ErrorKind::InvalidValue))?;
        d.seek(MediaTime::new(0, base))?;
        let mut reopened = SourceReader::open(Box::new(&mut *d), stream, DecodeMode::Canonical)?;
        let head = read_all(&mut reopened, 1);
        Ok((usize::try_from(receipt.landed.0).unwrap(), tail, head))
    })
    .expect("seek + reopen inside callback");

    assert!(landed <= target, "landing must not pass the target");
    // The meaningful property is the *content*: the post-seek tail
    // decodes the stream's second half exactly — sample `target + k`
    // at tail index `k`.
    assert_eq!(tail[0].len(), frames - target);
    for k in [0usize, 100, 999] {
        assert!(
            (tail[0][k] - f32::from(sample_at(target + k)) / 32768.0).abs() < 1e-6,
            "tail[{k}] must be source sample {}",
            target + k
        );
    }
    assert_eq!(
        head[0].len(),
        frames,
        "reopened reader decodes the whole stream"
    );
}

/// Early termination: `f` returns an error after reading a prefix —
/// the serve loop exits and the call returns that error.
#[test]
fn early_reader_error_terminates_scope() {
    let mut source = Cursor::new(wav_bytes(1, 1_000));
    let result: ruviform_media_io::Result<()> =
        ruviform_io_symphonia::with_borrowed_demuxer(&mut source, |d| {
            let stream = d.streams()[0].stream.id;
            let mut reader = SourceReader::open(Box::new(d), stream, DecodeMode::Canonical)?;
            let mut window = [vec![0.0f32; 64]];
            let mut slices: Vec<&mut [f32]> = window.iter_mut().map(Vec::as_mut_slice).collect();
            let _ = reader.read_planar(&mut slices, 64)?;
            Err(Error::from_kind(ErrorKind::InvalidValue))
        });
    assert_eq!(
        result.expect_err("sentinel").kind(),
        ErrorKind::InvalidValue
    );
}
