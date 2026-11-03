/**
 * Ruviform-owned demo data (Issue #263).
 *
 * Deterministic synthetic fixtures shared by the standalone showcase
 * page and the headless tests — no randomness, no consumer product
 * data, no analysis state. "Fake" is the point: every dataset is small,
 * reproducible, and exercises the contract surface (NaN gaps, snapshot
 * sequences, bounded queries) rather than realism.
 *
 * All analysis (LOD/FFT/measurement) stays behind the `*DataSource`
 * ports — these factories only produce already-reduced demo views.
 */

import {
  ManualSnapshotSource,
  MemoryWaveformSource,
  type BinAxis,
  type MeterDataSource,
  type MeterFrame,
  type SpectrumDataSource,
  type SpectrumFrame,
  type SourceMeta,
} from "../../viz/src/index.ts";

export const DEMO_SAMPLE_RATE = 48000;
/** 2 s stereo fixture — bounded so tests stay fast and deterministic. */
export const DEMO_WAVEFORM_SECONDS = 2;
/** NaN dropout region in channel 0 (frames), as a fraction of length. */
export const DEMO_GAP = { start: 0.45, end: 0.55 } as const;

/**
 * Stereo f64 waveform: decaying sines + a deterministic NaN dropout in
 * channel 0 (gap semantics are data, not paint). Returns interleaved
 * `[frame*2+channel]` samples.
 */
export function makeDemoWaveformSamples(
  seconds = DEMO_WAVEFORM_SECONDS,
  sampleRate = DEMO_SAMPLE_RATE,
): Float64Array {
  const n = Math.floor(seconds * sampleRate);
  const out = new Float64Array(n * 2);
  const gapStart = Math.floor(n * DEMO_GAP.start);
  const gapEnd = Math.floor(n * DEMO_GAP.end);
  for (let i = 0; i < n; i++) {
    const decay = Math.exp(-i / n / 1.5);
    const inGap = i >= gapStart && i < gapEnd;
    out[i * 2] = inGap
      ? Number.NaN
      : 0.6 * Math.sin((2 * Math.PI * 220 * i) / sampleRate) * decay;
    out[i * 2 + 1] =
      0.45 * Math.sin((2 * Math.PI * 330 * i) / sampleRate) * decay;
  }
  return out;
}

/** Reference waveform source over the demo samples (id "demo-wf"). */
export function makeDemoWaveformSource(
  id = "demo-wf",
  sampleRate = DEMO_SAMPLE_RATE,
): MemoryWaveformSource {
  return new MemoryWaveformSource(
    makeDemoWaveformSamples(DEMO_WAVEFORM_SECONDS, sampleRate),
    2,
    { axis: "seconds", sampleRate },
    id,
  );
}

/** A demo snapshot port plus its manual `push` pump handle. */
export type DemoMeterSource = MeterDataSource & {
  push(f: MeterFrame): void;
};
export type DemoSpectrumSource = SpectrumDataSource & {
  push(f: SpectrumFrame): void;
};

/** Meter snapshot source with the channel-count meta the port needs. */
export function makeDemoMeterSource(meta?: Partial<SourceMeta>): DemoMeterSource {
  const base = new ManualSnapshotSource<MeterFrame>({
    id: meta?.id ?? "demo-meter",
    revision: meta?.revision ?? "r1",
  });
  return {
    meta: () => ({ ...base.meta(), channels: 2 }),
    snapshot: () => base.snapshot(),
    subscribe: (fn) => base.subscribe(fn),
    push: (f) => base.push(f),
  };
}

export const DEMO_SPECTRUM_BINS = 240;
export const DEMO_SPECTRUM_AXIS: BinAxis = {
  kind: "uniform",
  firstHz: 0,
  stepHz: DEMO_SAMPLE_RATE / 2 / DEMO_SPECTRUM_BINS,
};

/** Spectrum snapshot source with the bins/axis meta the port needs. */
export function makeDemoSpectrumSource(
  meta?: Partial<SourceMeta>,
): DemoSpectrumSource {
  const base = new ManualSnapshotSource<SpectrumFrame>({
    id: meta?.id ?? "demo-spectrum",
    revision: meta?.revision ?? "r1",
  });
  return {
    meta: () => ({
      ...base.meta(),
      bins: DEMO_SPECTRUM_BINS,
      axis: DEMO_SPECTRUM_AXIS,
      sampleRate: DEMO_SAMPLE_RATE,
    }),
    snapshot: () => base.snapshot(),
    subscribe: (fn) => base.subscribe(fn),
    push: (f) => base.push(f),
  };
}

/** Deterministic demo meter frame — envelope follows a slow cycle. */
export function makeDemoMeterFrame(seq: number, t: number): MeterFrame {
  const env = 0.5 + 0.4 * Math.sin(2 * Math.PI * 0.4 * t);
  return {
    sequence: seq,
    channels: [
      {
        peak: Math.min(1.2, env * 1.1),
        rms: env * 0.7,
        // Held peak tracks the instantaneous peak with the same
        // headroom — it may never sit below `peak`.
        heldPeak: Math.min(1.2, env * 1.15),
        clip: env > 0.98,
      },
      { peak: env * 0.85, rms: env * 0.55 },
    ],
    timestamp: t * 1000,
  };
}

/**
 * Deterministic demo spectrum frame — a moving log-bump with periodic
 * NaN bins (gap ticks are data, not paint).
 */
export function makeDemoSpectrumFrame(seq: number, t: number): SpectrumFrame {
  const mags = new Float64Array(DEMO_SPECTRUM_BINS);
  for (let i = 0; i < DEMO_SPECTRUM_BINS; i++) {
    mags[i] =
      i % 37 === 0
        ? Number.NaN
        : Math.max(
            0,
            Math.min(
              1,
              Math.exp(-Math.pow(Math.log2(((i + 1) / DEMO_SPECTRUM_BINS) * 40), 2)) *
                (0.5 + 0.4 * Math.sin(2 * Math.PI * 0.4 * t)) +
                0.05 * Math.abs(Math.sin(i / 7 + t * 3)),
            ),
          );
  }
  return { sequence: seq, magnitudes: mags, timestamp: t * 1000 };
}

/** Paired demo snapshot sources for one showcase column/fixture. */
export interface DemoSnapshotSources {
  readonly meter: DemoMeterSource;
  readonly spectrum: DemoSpectrumSource;
}

export function makeDemoSnapshotSources(): DemoSnapshotSources {
  return { meter: makeDemoMeterSource(), spectrum: makeDemoSpectrumSource() };
}

/**
 * Deterministic pump step: pushes the `seq`-th demo frames at time `t`
 * (seconds). Tests drive it directly; the showcase page calls it from
 * `requestAnimationFrame`.
 */
export function pumpDemoOnce(
  sources: DemoSnapshotSources,
  seq: number,
  t: number,
): void {
  sources.meter.push(makeDemoMeterFrame(seq, t));
  sources.spectrum.push(makeDemoSpectrumFrame(seq, t));
}
