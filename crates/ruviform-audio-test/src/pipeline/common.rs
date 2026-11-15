//! Shared Expected/Observed helpers for the pipeline legs.
//!
//! Expected PCM is assembled **independently** of every consumer under
//! test: [`expected_block`] walks the [`PcmScenario`] coverage and
//! delegates each covered run to [`crate::oracle::pcm_reference_block`]
//! (which re-implements the fixture spec without calling
//! `PcmFixture::fill`/`sample_at`); gaps contribute exact `0.0`
//! silence, matching the scripted backend's gap contract. Nothing here
//! calls production processing code — the observed side always comes
//! from the consumer boundary (`captured_output`, typed runtime
//! `output_reader`, `InputStage::collect_block`).

use crate::oracle::{PcmCompare, pcm_assert_eq, pcm_reference_block};
use crate::pcm::{PcmBlock, PcmFixture, PcmScenario};

/// Per-channel expected PCM for the absolute frame range
/// `[start_frame, start_frame + frames)` under `scenario`.
///
/// Returns `channels` vectors of `frames` samples each (planar
/// channel-major, matching [`PcmBlock::data`] layout). Frames not
/// covered by any [`crate::pcm::PcmSegment`] are `0.0` — the scripted
/// backend's documented gap behavior (silence + `Xrun` report).
///
/// **RT contract:** non-RT — allocates the expected vectors.
#[must_use]
pub fn expected_block(
    scenario: &PcmScenario,
    start_frame: u64,
    frames: u32,
    channels: u16,
) -> Vec<Vec<f32>> {
    let end = start_frame.saturating_add(u64::from(frames));
    (0..channels)
        .map(|channel| {
            let mut out = Vec::with_capacity(frames as usize);
            let mut cursor = start_frame;
            while cursor < end {
                if let Some(seg) = scenario.segment_at(cursor) {
                    let seg_end = seg.frame_start.saturating_add(seg.frame_len);
                    let run_end = seg_end.min(end);
                    let run_frames = u32::try_from(run_end - cursor).unwrap_or(u32::MAX);
                    out.extend(pcm_reference_block(
                        &seg.fixture,
                        cursor,
                        run_frames,
                        channel,
                    ));
                    cursor = run_end;
                } else {
                    // Gap: silence until the next segment or block end.
                    let next_start = scenario
                        .segments()
                        .iter()
                        .find(|s| s.frame_start > cursor)
                        .map_or(end, |s| s.frame_start.min(end));
                    let gap = usize::try_from(next_start - cursor).unwrap_or(usize::MAX);
                    out.resize(out.len() + gap, 0.0);
                    cursor = next_start;
                }
            }
            out
        })
        .collect()
}

/// Per-channel expected PCM for `[start_frame, start_frame + frames)`
/// under a single [`PcmFixture`] — the output leg's oracle (the graph
/// node renders one fixture at absolute positions).
///
/// **RT contract:** non-RT — allocates the expected vectors.
#[must_use]
pub fn expected_fixture_block(
    fixture: &PcmFixture,
    start_frame: u64,
    frames: u32,
    channels: u16,
) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|channel| pcm_reference_block(fixture, start_frame, frames, channel))
        .collect()
}

/// Scales every sample of per-channel `expected` by `gain` — the
/// independent expected side of a gain-stage transform. Computed in
/// `f32` exactly as [`crate::pipeline::input::GainNode`] multiplies,
/// so the comparison isolates pipeline wiring, not arithmetic order.
#[must_use]
pub fn scale_expected(expected: &[Vec<f32>], gain: f32) -> Vec<Vec<f32>> {
    expected
        .iter()
        .map(|channel| channel.iter().map(|s| s * gain).collect())
        .collect()
}

/// Compares planar observed PCM (`data`, `channels * frames`
/// channel-major) against per-channel `expected` slices under `mode`.
///
/// # Errors
///
/// [`String`] describing the first mismatch — same contract as
/// [`pcm_assert_eq`].
///
/// **RT contract:** non-RT — diagnostic path.
pub fn compare_planar(
    data: &[f32],
    frames: u32,
    channels: u16,
    expected: &[Vec<f32>],
    mode: PcmCompare,
) -> Result<f32, String> {
    if expected.len() != usize::from(channels) {
        return Err(format!(
            "expected channel count {} != observed {}",
            expected.len(),
            channels
        ));
    }
    let frames_usize = frames as usize;
    let mut max_diff = 0.0f32;
    for (ch, want) in expected.iter().enumerate() {
        let start = ch * frames_usize;
        let end = start + frames_usize;
        let got = data.get(start..end).ok_or_else(|| {
            format!(
                "observed data short: need {} samples, have {}",
                end,
                data.len()
            )
        })?;
        max_diff =
            max_diff.max(pcm_assert_eq(got, want, mode).map_err(|e| format!("channel {ch}: {e}"))?);
    }
    Ok(max_diff)
}

/// Compares a captured [`PcmBlock`] against per-channel expected
/// slices — the output leg's oracle check.
///
/// # Errors
///
/// [`String`] on channel-count or sample mismatch.
pub fn compare_pcm_block(
    block: &PcmBlock,
    expected: &[Vec<f32>],
    mode: PcmCompare,
) -> Result<f32, String> {
    compare_planar(&block.data, block.frames, block.channels, expected, mode)
}
