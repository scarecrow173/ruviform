//! PCM fixture determinism + oracle tests (Issue #628).

use ruviform_audio_core::{AudioBufferViewMut, SampleRate};
use ruviform_audio_test::oracle::{
    PcmCompare, midi_expected_playthrough, pcm_assert_eq, pcm_bits_eq, pcm_reference_block,
};
use ruviform_audio_test::pcm::{PcmFixture, PcmPattern, PcmScenario, PcmSegment};
use ruviform_media_core::{MediaTime, TimeBase};

fn rate() -> SampleRate {
    SampleRate::new(48_000).unwrap()
}

fn fixture(pattern: PcmPattern, channels: u16) -> PcmFixture {
    PcmFixture::new(rate(), channels, pattern).unwrap()
}

#[test]
fn silence_and_constant_are_exact() {
    let silence = fixture(PcmPattern::Silence, 2);
    let mut buf = vec![1.0f32; 2 * 64];
    silence.fill(0, 64, &mut buf).unwrap();
    assert!(buf.iter().all(|s| *s == 0.0));

    let dc = fixture(PcmPattern::Constant { value: -0.25 }, 1);
    let mut buf = vec![0.0f32; 32];
    dc.fill(7, 32, &mut buf).unwrap();
    assert!(buf.iter().all(|s| s.to_bits() == (-0.25f32).to_bits()));
}

#[test]
fn impulse_hits_only_on_period_boundaries() {
    let f = fixture(
        PcmPattern::Impulse {
            period_frames: 4,
            amplitude: 0.5,
        },
        1,
    );
    let mut buf = vec![0.0f32; 10];
    f.fill(0, 10, &mut buf).unwrap();
    // frames 0,4,8 are impulses
    assert_eq!(buf[0], 0.5);
    assert_eq!(buf[4], 0.5);
    assert_eq!(buf[8], 0.5);
    assert!(
        buf.iter()
            .enumerate()
            .all(|(i, s)| { i % 4 == 0 || *s == 0.0 })
    );
}

#[test]
fn ramp_and_square_are_deterministic() {
    let ramp = fixture(
        PcmPattern::Ramp {
            amplitude: 1.0,
            period_frames: 4,
        },
        1,
    );
    // phase = (frame % 4)/4 → values -1, -0.5, 0, 0.5
    let got: Vec<f32> = (0..4).map(|i| ramp.sample_at(0, i)).collect();
    let want = [-1.0f32, -0.5, 0.0, 0.5];
    assert_eq!(got, want);

    let sq = fixture(
        PcmPattern::Square {
            amplitude: 0.75,
            period_frames: 4,
        },
        1,
    );
    let got: Vec<f32> = (0..4).map(|i| sq.sample_at(0, i)).collect();
    assert_eq!(got, [0.75, 0.75, -0.75, -0.75]);
}

#[test]
fn fill_is_position_independent() {
    // R2: fill(start=512) must equal the [512..1024) slice of a fill
    // from 0 — the same absolute frame always yields the same sample.
    for pattern in [
        PcmPattern::Sine {
            freq_hz: 440.0,
            amplitude: 0.8,
        },
        PcmPattern::Sweep {
            f0_hz: 100.0,
            f1_hz: 2_000.0,
            duration_frames: 4_096,
            amplitude: 0.5,
        },
        PcmPattern::Noise {
            seed: 0xDEAD_BEEF,
            amplitude: 0.9,
        },
        PcmPattern::Ramp {
            amplitude: 1.0,
            period_frames: 97,
        },
    ] {
        let f = fixture(pattern, 2);
        let mut a = vec![0.0f32; 2 * 1_024];
        f.fill(0, 1_024, &mut a).unwrap();
        let mut b = vec![0.0f32; 2 * 512];
        f.fill(512, 512, &mut b).unwrap();
        // Planar layout: b[ch][0..512] must equal a[ch][512..1024].
        for ch in 0..2usize {
            let got = &b[ch * 512..(ch + 1) * 512];
            let want = &a[ch * 1_024 + 512..ch * 1_024 + 1_024];
            assert!(
                pcm_bits_eq(got, want),
                "pattern {pattern:?} ch{ch} is not position-independent"
            );
        }
    }
}

#[test]
fn noise_is_seeded_and_per_channel() {
    let f = fixture(
        PcmPattern::Noise {
            seed: 7,
            amplitude: 1.0,
        },
        2,
    );
    // deterministic across calls
    assert_eq!(f.sample_at(0, 100), f.sample_at(0, 100));
    // channels differ (overwhelmingly likely; deterministic so stable)
    assert_ne!(f.sample_at(0, 100), f.sample_at(1, 100));
    // bounded
    for frame in 0..256u64 {
        for ch in 0..2 {
            assert!(f.sample_at(ch, frame).abs() < 1.0);
        }
    }
    // different seed → different stream
    let g = fixture(
        PcmPattern::Noise {
            seed: 8,
            amplitude: 1.0,
        },
        2,
    );
    assert_ne!(f.sample_at(0, 42), g.sample_at(0, 42));
}

#[test]
fn sine_stays_within_amplitude_and_is_deterministic() {
    let f = fixture(
        PcmPattern::Sine {
            freq_hz: 1_000.0,
            amplitude: 0.5,
        },
        1,
    );
    let mut buf = vec![0.0f32; 4_800];
    f.fill(0, 4_800, &mut buf).unwrap();
    assert!(buf.iter().all(|s| s.abs() <= 0.5 + 1e-6));
    // bit-identical regeneration
    let mut again = vec![0.0f32; 4_800];
    f.fill(0, 4_800, &mut again).unwrap();
    assert!(pcm_bits_eq(&buf, &again));
    // completes ~100 cycles at 1kHz/48k → endpoints near zero crossing
    assert!(buf[0].abs() < 0.01);
}

#[test]
fn invalid_fixtures_are_rejected() {
    assert!(PcmFixture::new(rate(), 0, PcmPattern::Silence).is_err());
    assert!(
        PcmFixture::new(
            rate(),
            1,
            PcmPattern::Impulse {
                period_frames: 0,
                amplitude: 1.0
            }
        )
        .is_err()
    );
    assert!(
        PcmFixture::new(
            rate(),
            1,
            PcmPattern::Sine {
                freq_hz: 48_000.0, // >= sample_rate
                amplitude: 1.0
            }
        )
        .is_err()
    );
    assert!(
        PcmFixture::new(
            rate(),
            1,
            PcmPattern::Sweep {
                f0_hz: -1.0,
                f1_hz: 10.0,
                duration_frames: 10,
                amplitude: 1.0
            }
        )
        .is_err()
    );
}

#[test]
fn fill_rejects_wrong_buffer_length() {
    let f = fixture(PcmPattern::Silence, 2);
    let mut buf = vec![0.0f32; 10];
    assert!(f.fill(0, 4, &mut buf).is_err()); // needs 8
    assert!(f.fill(0, 0, &mut []).is_ok());
}

#[test]
fn fill_view_matches_fill() {
    let f = fixture(
        PcmPattern::Sine {
            freq_hz: 440.0,
            amplitude: 0.5,
        },
        2,
    );
    let frames = 128usize;
    let mut data = vec![0.0f32; 2 * frames];
    let mut view = AudioBufferViewMut::try_new(&mut data, 2, frames).unwrap();
    f.fill_view(64, &mut view).unwrap();

    let mut want = vec![0.0f32; 2 * frames];
    f.fill(64, u32::try_from(frames).unwrap(), &mut want)
        .unwrap();
    assert!(pcm_bits_eq(&data, &want));
}

#[test]
fn scenario_segments_and_gaps() {
    let seg_a = PcmSegment {
        frame_start: 0,
        frame_len: 100,
        fixture: fixture(PcmPattern::Silence, 1),
    };
    let seg_b = PcmSegment {
        frame_start: 150, // gap 100..150 = dropped frames
        frame_len: 50,
        fixture: fixture(PcmPattern::Constant { value: 0.25 }, 1),
    };
    let sc = PcmScenario::new(vec![seg_b, seg_a]).unwrap(); // unsorted input ok
    assert_eq!(sc.segments()[0].frame_start, 0);
    assert!(!sc.is_contiguous());
    assert_eq!(sc.segment_at(50).unwrap().frame_start, 0);
    assert_eq!(sc.segment_at(120), None); // inside the gap
    assert_eq!(sc.segment_at(160).unwrap().frame_start, 150);

    // overlapping segments rejected
    let bad = PcmScenario::new(vec![
        PcmSegment {
            frame_start: 0,
            frame_len: 100,
            fixture: seg_a.fixture,
        },
        PcmSegment {
            frame_start: 50,
            frame_len: 10,
            fixture: seg_b.fixture,
        },
    ]);
    assert!(bad.is_err());
}

#[test]
fn block_carries_format_metadata() {
    let f = fixture(PcmPattern::Constant { value: 0.1 }, 2);
    let b = f.block(1_000, 8).unwrap();
    assert_eq!(b.start_frame, 1_000);
    assert_eq!(b.frames, 8);
    assert_eq!(b.channels, 2);
    assert_eq!(b.sample_rate.get(), 48_000);
    assert_eq!(b.data.len(), 16);
}

// ---- Oracle independence ----

#[test]
fn oracle_matches_generator_all_patterns() {
    let cases: Vec<(PcmPattern, PcmCompare)> = vec![
        (PcmPattern::Silence, PcmCompare::Exact),
        (PcmPattern::Constant { value: 0.3 }, PcmCompare::Exact),
        (
            PcmPattern::Impulse {
                period_frames: 8,
                amplitude: 0.9,
            },
            PcmCompare::Exact,
        ),
        (
            PcmPattern::Ramp {
                amplitude: 0.7,
                period_frames: 33,
            },
            PcmCompare::Exact,
        ),
        (
            PcmPattern::Square {
                amplitude: 0.4,
                period_frames: 16,
            },
            PcmCompare::Exact,
        ),
        (
            PcmPattern::Noise {
                seed: 0x1234_5678,
                amplitude: 0.6,
            },
            PcmCompare::Exact,
        ),
        (
            PcmPattern::Sine {
                freq_hz: 997.0,
                amplitude: 0.8,
            },
            PcmCompare::AbsEpsilon(1e-5),
        ),
        (
            PcmPattern::Sweep {
                f0_hz: 50.0,
                f1_hz: 8_000.0,
                duration_frames: 2_048,
                amplitude: 0.5,
            },
            PcmCompare::AbsEpsilon(1e-5),
        ),
    ];
    for (pattern, mode) in cases {
        let f = fixture(pattern, 2);
        let start = 1_234u64;
        let frames = 512u32;
        let mut got = vec![0.0f32; 2 * frames as usize];
        f.fill(start, frames, &mut got).unwrap();
        for ch in 0..2u16 {
            let want = pcm_reference_block(&f, start, frames, ch);
            let got_ch: Vec<f32> = (0..frames as usize)
                .map(|i| got[ch as usize * frames as usize + i])
                .collect();
            pcm_assert_eq(&got_ch, &want, mode)
                .unwrap_or_else(|e| panic!("pattern {pattern:?} ch{ch}: {e}"));
        }
    }
}

#[test]
fn oracle_literal_vectors() {
    // Fixed literal vectors pin both generator and oracle to known
    // values — catches spec drift on either side.
    let ramp = fixture(
        PcmPattern::Ramp {
            amplitude: 2.0,
            period_frames: 8,
        },
        1,
    );
    // phase 0,1/8,..7/8 → 2*(2p-1) = -2,-1.5,-1,-0.5,0,0.5,1,1.5
    let got: Vec<f32> = (0..8).map(|i| ramp.sample_at(0, i)).collect();
    assert_eq!(got, [-2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5]);

    let sq = fixture(
        PcmPattern::Square {
            amplitude: 1.0,
            period_frames: 2,
        },
        1,
    );
    assert_eq!(sq.sample_at(0, 0), 1.0);
    assert_eq!(sq.sample_at(0, 1), -1.0);
    assert_eq!(sq.sample_at(0, 2), 1.0);

    // silence literal
    assert_eq!(fixture(PcmPattern::Silence, 1).sample_at(0, u64::MAX), 0.0);
}

// ---- MIDI oracle smoke (full coverage in tests/midi.rs) ----

#[test]
fn midi_oracle_playthrough_orders_by_position_then_sequence() {
    let base = TimeBase::sample_rate(48_000).unwrap();
    let mk = |pos: i64, seq: u64, key: u8| ruviform_audio_test::midi::MidiCue {
        position: MediaTime::new(pos, base),
        source_sequence: seq,
        event: ruviform_media_core::MidiEvent {
            group: ruviform_media_core::MidiGroup::ZERO,
            channel: ruviform_media_core::MidiChannel::new(0).unwrap(),
            kind: ruviform_media_core::MidiKind::NoteOn {
                key: ruviform_media_core::NoteKey::new(key).unwrap(),
                velocity: 0x8000_0000,
            },
        },
    };
    // intentionally unordered input; same-position cues ordered by seq
    let cues = vec![mk(200, 9, 60), mk(100, 5, 62), mk(100, 2, 64)];
    let expected = midi_expected_playthrough(base, &cues, 1_000);
    assert_eq!(expected.len(), 3);
    assert_eq!(expected[0].frame.frame(), 100);
    assert_eq!(expected[0].source_sequence, 2); // seq 2 before seq 5
    assert_eq!(expected[1].source_sequence, 5);
    assert_eq!(expected[2].frame.frame(), 200);
}
