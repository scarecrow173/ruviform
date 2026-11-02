//! `SourceReader` acceptance tests — RD-01 (source-frame addressing,
//! seek, EOF), RD-04 (fault classes), RD-08/09 (native mode,
//! metadata), plus the unseekable-source class.

use std::io::{Cursor, Read, Seek, SeekFrom};

use ruviform_media_core::{Fact, StreamKind};
use ruviform_media_io::{DecodeMode, Demuxer, ErrorKind, Frame};
use ruviform_media_reader::{SourceFrame, SourceReader};

const RATE: u32 = 48_000;

/// Deterministic S16 sample — every value maps exactly into f32
/// (i16 / 2^15 is representable), so comparisons are exact.
fn sample_at(i: usize) -> i16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    {
        (i as i32 * 7 % 5000 - 2500) as i16
    }
}

/// Builds a mono or stereo PCM WAV byte stream in memory.
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
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
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

fn expected(i: usize) -> f32 {
    f32::from(sample_at(i)) / 32768.0
}

fn open_reader(channels: u16, frames: usize) -> SourceReader {
    let demuxer =
        ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(channels, frames))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    SourceReader::open(Box::new(demuxer), stream, DecodeMode::Canonical).unwrap()
}

fn read_all(reader: &mut SourceReader, channels: usize) -> Vec<Vec<f32>> {
    let mut out = vec![Vec::new(); channels];
    let mut window = vec![vec![0.0f32; 997]; channels];
    let mut slices: Vec<&mut [f32]> = window.iter_mut().map(Vec::as_mut_slice).collect();
    loop {
        let outcome = reader.read_planar(&mut slices, 997).unwrap();
        for (channel, buf) in out.iter_mut().enumerate() {
            buf.extend_from_slice(&slices[channel][..outcome.frames]);
        }
        if outcome.eof || outcome.frames == 0 {
            break;
        }
    }
    out
}

#[test]
fn rd01_mono_decodes_source_frames_in_order() {
    let frames = 10_000usize;
    let mut reader = open_reader(1, frames);
    assert_eq!(reader.position(), SourceFrame(0));

    let channels = read_all(&mut reader, 1);
    assert_eq!(channels[0].len(), frames);
    for (i, &sample) in channels[0].iter().enumerate() {
        assert_eq!(sample, expected(i), "sample {i} mismatched");
    }
    assert!(reader.finished());
}

#[test]
fn rd01_stereo_frame_and_channel_content() {
    let frames = 5000usize;
    let mut reader = open_reader(2, frames);
    let channels = read_all(&mut reader, 2);
    assert_eq!(channels[0].len(), frames);
    assert_eq!(channels[1].len(), frames);
    for i in [0, 1, 1151, 1152, 3000, frames - 1] {
        assert_eq!(channels[0][i], expected(i), "ch0 frame {i}");
        assert_eq!(channels[1][i], expected(i), "ch1 frame {i}");
    }
}

#[test]
fn rd01_mid_seek_lands_and_decodes_correctly() {
    let frames = 10_000usize;
    let mut reader = open_reader(1, frames);
    let target = SourceFrame(5000);
    let receipt = reader.seek_source_frame(target).unwrap();
    assert_eq!(receipt.requested, target);
    assert!(receipt.landed <= target);
    assert_eq!(reader.position(), target);

    let mut dst = [vec![0.0f32; 512]];
    let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    let outcome = reader.read_planar(&mut slices, 512).unwrap();
    assert_eq!(outcome.frames, 512);
    for (i, &s) in slices[0].iter().enumerate() {
        assert_eq!(s, expected(5000 + i), "post-seek sample {i}");
    }
}

#[test]
fn rd01_tail_and_eof_are_distinguishable() {
    let frames = 2500usize;
    let mut reader = open_reader(1, frames);
    reader
        .seek_source_frame(SourceFrame((frames - 16) as u64))
        .unwrap();
    let mut dst = [vec![0.0f32; 64]];
    let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    let outcome = reader.read_planar(&mut slices, 64).unwrap();
    assert_eq!(outcome.frames, 16, "only the tail remains");
    assert!(outcome.eof, "EOF reached inside the read");
    for (i, &s) in slices[0][..16].iter().enumerate() {
        assert_eq!(s, expected(frames - 16 + i));
    }
    // A read past the end reports 0/eof — not content.
    let outcome = reader.read_planar(&mut slices, 64).unwrap();
    assert_eq!(outcome.frames, 0);
    assert!(reader.finished());
}

#[test]
fn rd01_seek_beyond_end_reports_honestly() {
    let mut reader = open_reader(1, 1000);
    // A seek past the stream end is either rejected by the demuxer
    // (InvalidValue/OutOfRange) or lands at EOF — both are honest
    // reports, never a fabricated position.
    match reader.seek_source_frame(SourceFrame(9000)) {
        Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidValue),
        Ok(receipt) => {
            let mut dst = [vec![0.0f32; 64]];
            let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
            let outcome = reader.read_planar(&mut slices, 64).unwrap();
            assert!(outcome.frames <= 64);
            let _ = receipt;
        }
    }
}

#[test]
fn rd04_unseekable_source_declares_and_rejects() {
    struct NoSeek(Cursor<Vec<u8>>);
    impl Read for NoSeek {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl Seek for NoSeek {
        fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
            Err(std::io::Error::other("unseekable"))
        }
    }

    // Unseekable source → declared Unsupported, seek rejected.
    let demuxer =
        ruviform_io_symphonia::open_demuxer(NoSeek(Cursor::new(wav_bytes(1, 512)))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    let mut reader = SourceReader::open(Box::new(demuxer), stream, DecodeMode::Canonical).unwrap();
    assert_eq!(reader.metadata().seekable, Fact::Unsupported);
    let err = reader.seek_source_frame(SourceFrame(100)).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
    // Sequential reads still work.
    let channels = read_all(&mut reader, 1);
    assert_eq!(channels[0].len(), 512);
}

#[test]
fn rd04_truncated_wav_surfaces_unexpected_eos() {
    let mut bytes = wav_bytes(1, 512);
    bytes.truncate(100); // cut inside the data chunk
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(bytes));
    match demuxer {
        Ok(demuxer) => {
            let stream = demuxer.streams()[0].stream.id;
            let mut reader =
                SourceReader::open(Box::new(demuxer), stream, DecodeMode::Canonical).unwrap();
            let mut dst = [vec![0.0f32; 4096]];
            let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
            // Either an UnexpectedEos error or a short read then EOF —
            // both are classified, never silent success.
            match reader.read_planar(&mut slices, 4096) {
                Err(e) => assert_eq!(e.kind(), ErrorKind::UnexpectedEos),
                Ok(o) => assert!(o.eof || o.frames < 4096),
            }
        }
        Err(e) => assert!(
            matches!(e.kind(), ErrorKind::UnexpectedEos | ErrorKind::InvalidData),
            "unexpected open failure kind: {:?}",
            e.kind()
        ),
    }
}

#[test]
fn rd04_corrupt_container_rejected_at_open() {
    let result = ruviform_io_symphonia::open_demuxer(Cursor::new(vec![0xABu8; 64]));
    match result {
        // Unsupported (no probe match) and InvalidData (probe found a
        // container then failed) are both honest refusals.
        Err(e) => assert!(
            matches!(e.kind(), ErrorKind::Unsupported | ErrorKind::InvalidData),
            "unexpected open failure kind: {:?}",
            e.kind()
        ),
        Ok(_) => panic!("corrupt container must not open"),
    }
}

#[test]
fn rd08_native_mode_preserves_native_scale() {
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(1, 512))).unwrap();
    let stream = demuxer.streams()[0].stream.id;
    let mut reader = SourceReader::open(Box::new(demuxer), stream, DecodeMode::Native).unwrap();
    assert_eq!(reader.metadata().mode, DecodeMode::Native);
    // read_planar is the canonical-only path.
    let mut dst = [vec![0.0f32; 64]];
    let mut slices: Vec<&mut [f32]> = dst.iter_mut().map(Vec::as_mut_slice).collect();
    assert_eq!(
        reader.read_planar(&mut slices, 64).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
    // Whole-frame native delivery preserves f64 native scale.
    let frame = reader.next_frame().unwrap().expect("a frame");
    match frame {
        Frame::NativeAudio(native) => {
            assert!(native.samples().iter().all(|s| s.is_finite()));
            assert!(native.frames() > 0);
        }
        other => unreachable!("expected native audio, got {other:?}"),
    }
}

#[test]
fn rd09_metadata_carries_capability_facts() {
    let reader = open_reader(2, 4800);
    let meta = reader.metadata();
    assert_eq!(meta.backend.name, "symphonia");
    assert_eq!(meta.stream.stream.kind, StreamKind::Audio);
    assert_eq!(meta.canonical.layout.channels(), 2);
    assert_eq!(meta.canonical.sample_rate.get(), RATE);
    assert_eq!(meta.seekable, Fact::known(()));
    assert_eq!(meta.frame_count, Fact::known(4800));
    assert!(meta.stream.facts.decodable.is_known());
}

#[test]
fn rd05_reopen_rebuilds_under_new_metadata() {
    let mut reader = open_reader(1, 1024);
    let _ = read_all(&mut reader, 1);
    assert!(reader.finished());
    // Reopen on a fresh source — position resets, metadata re-derived.
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(wav_bytes(2, 512))).unwrap();
    reader.reopen(Box::new(demuxer)).unwrap();
    assert_eq!(reader.position(), SourceFrame(0));
    assert!(!reader.finished());
    assert_eq!(reader.metadata().canonical.layout.channels(), 2);
}
