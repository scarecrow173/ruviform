/**
 * Data-source ports for the Reference Viz renderers (Issue #260 —
 * データ/描画契約).
 *
 * These ports mirror `crates/ruviform-viz` semantics into TypeScript:
 * - queries are **bounded** — a view never asks for the whole dataset;
 *   `buckets` caps the returned resolution (the LOD budget).
 * - every response carries the `revision` it was produced from, so a
 *   consumer can **reject stale answers** after a source swap or a
 *   revision race.
 * - `NaN`/non-finite marks missing/invalid data. Views propagate it;
 *   renderers draw gaps — never interpolate it away.
 * - cancellation rides the standard `AbortSignal`; unmounting a view
 *   aborts its in-flight query.
 * - metering/spectrum arrive as **snapshots** (the existing safe
 *   RT→UI non-RT delivery): the source owns the measurement cycle, the
 *   element pulls the latest complete frame on its own draw cycle, and
 *   sequence gaps / dropped counts / stale time are surfaced, not hidden.
 *
 * Decimation itself (LOD pyramids, STFT, FFT) stays in the Rust layer —
 * a bridge source serves already-reduced views over these ports. The
 * in-memory implementations here are reference fixtures (the
 * `SliceWaveform`/`VecSpectrum` equivalent), small enough for tests,
 * demos, and bounded in-memory datasets.
 */

import type { AxisUnit } from "./axis.ts";

/** Source identity + generation. `revision` changes whenever the data a
 * source serves may differ — replacing the source instance or mutating
 * it (streaming appends) must advance it. */
export interface SourceMeta {
  readonly id: string;
  readonly revision: number | string;
}

export type SourceAxisUnit = Extract<AxisUnit, "seconds" | "samples" | "frames">;

export interface WaveformInfo {
  /** Axis positions are in `axis` units; `extent` is the total span. */
  readonly axis: SourceAxisUnit;
  readonly extent: number;
  readonly channels: number;
  /** Capture rate — display metadata (tick formatting), never required
   * for geometry. */
  readonly sampleRate?: number;
}

export interface WaveformQuery {
  /** Viewport span in axis units, `[start, end)` (clamped to extent). */
  readonly start: number;
  readonly end: number;
  /** Upper bound of returned buckets per channel — the LOD budget,
   * normally the plot width in px. */
  readonly buckets: number;
  /** Channel indices to read (default: all). */
  readonly channels?: readonly number[];
}

/** One reduction bucket — same fields as `ruviform-viz` `LodBucket`.
 * `count` is the finite-sample count; `count === 0` marks a fully
 * missing bucket and `min`/`max`/`rms` are `NaN` (never zero-filled). */
export interface WaveformBucket {
  readonly min: number;
  readonly max: number;
  readonly rms: number;
  readonly count: number;
}

export interface WaveformBinsView {
  readonly kind: "bins";
  /** Revision the view was produced from (stale-response check). */
  readonly revision: number | string;
  /** Axis position of bucket 0's start. */
  readonly start: number;
  /** Axis units covered per bucket. */
  readonly unitsPerBucket: number;
  /** Per-channel bucket rows (`query.channels` order). */
  readonly channels: readonly (readonly WaveformBucket[])[];
}

export interface WaveformSamplesView {
  readonly kind: "samples";
  readonly revision: number | string;
  readonly start: number;
  readonly unitsPerSample: number;
  readonly channels: readonly ArrayLike<number>[];
}

export type WaveformView = WaveformBinsView | WaveformSamplesView;

/**
 * Bounded random-access waveform port. `query` must honor `signal`
 * (rejected promise on abort) and never return more than `q.buckets`
 * buckets per channel — the caller's memory bound is the contract.
 */
export interface WaveformDataSource {
  meta(): WaveformInfo & SourceMeta;
  query(q: WaveformQuery, signal?: AbortSignal): Promise<WaveformView>;
  /** Streaming sources notify appended/replaced data; the element
   * re-queries the visible span. Static sources omit this. */
  subscribe?(onChange: () => void): () => void;
}

/** One channel of a meter snapshot — linear amplitude domain
 * (`0` = silence, `>= 1` = full scale; `NaN` = unavailable). `heldPeak`
 * is a *source-provided* display-hold value; the peak-hold decay
 * behavior stays a source/display-owner decision, not data. */
export interface MeterChannel {
  readonly peak: number;
  readonly rms: number;
  readonly heldPeak?: number;
  readonly clip?: boolean;
}

export interface MeterFrame {
  /** Monotonically increasing snapshot id — gaps visualize loss. */
  readonly sequence: number;
  readonly channels: readonly MeterChannel[];
  /** Snapshots the source knows were dropped since the previous frame
   * (measurement-cycle loss — rendered, never hidden). */
  readonly dropped?: number;
  readonly timestamp?: number;
}

export interface MeterInfo {
  readonly channels: number;
}

/** Latest-complete-frame port: measurement cycle behind the port, draw
 * cycle in the element. `snapshot()` must be cheap and non-blocking. */
export interface SnapshotSource<F> {
  subscribe?(onUpdate: () => void): () => void;
  snapshot(): F | null;
}

export interface MeterDataSource extends SnapshotSource<MeterFrame> {
  meta(): SourceMeta & MeterInfo;
}

/** Bin → Hz axis, mirroring `ruviform-viz` `BinAxis`. */
export type BinAxis =
  | { readonly kind: "uniform"; readonly firstHz: number; readonly stepHz: number }
  | { readonly kind: "centers"; readonly hz: readonly number[] };

export function binAxisHz(axis: BinAxis, bin: number): number {
  if (axis.kind === "uniform") return axis.firstHz + bin * axis.stepHz;
  const hz = (axis as { hz?: unknown }).hz;
  return hz !== null &&
    typeof (hz as { length?: unknown }).length === "number"
    ? ((hz as ArrayLike<number>)[bin] ?? Number.NaN)
    : Number.NaN;
}

export interface SpectrumInfo {
  readonly bins: number;
  readonly axis: BinAxis;
  readonly sampleRate?: number;
}

export interface SpectrumFrame {
  readonly sequence: number;
  /** f64 magnitudes, `info.bins` long, frame-major order. `NaN` = gap. */
  readonly magnitudes: ArrayLike<number>;
  readonly timestamp?: number;
}

export interface SpectrumDataSource extends SnapshotSource<SpectrumFrame> {
  meta(): SourceMeta & SpectrumInfo;
}

// ── reference implementations ────────────────────────────────────────

/**
 * In-memory waveform source over an interleaved f64 buffer — the TS
 * `SliceWaveform` equivalent. Queries are bounded by `q.buckets`: when
 * the visible span holds at most one sample per bucket the raw samples
 * are returned, otherwise min/max/rms/count buckets are reduced on the
 * fly (NaN skipped, `count === 0` for all-missing buckets). `replace`
 * publishes a new revision and notifies subscribers — the long-form
 * offline and streaming-append flavors in one reference port.
 */
export class MemoryWaveformSource implements WaveformDataSource {
  private rev = 0;
  private readonly listeners = new Set<() => void>();
  private samples: ArrayLike<number>;
  private readonly ch: number;
  private readonly info_: { axis: SourceAxisUnit; sampleRate?: number };
  private readonly id_: string;

  constructor(
    samples: ArrayLike<number>,
    ch: number,
    info_: { axis: SourceAxisUnit; sampleRate?: number },
    id_ = "memory-waveform",
  ) {
    if (ch < 1) throw new Error("channels must be >= 1");
    if (samples.length % ch !== 0) {
      throw new Error("samples length must be a multiple of channels");
    }
    this.ch = ch;
    this.info_ = info_;
    this.id_ = id_;
    this.samples = samples;
  }

  private get rate(): number {
    return this.info_.axis === "seconds"
      ? (this.info_.sampleRate ?? 1)
      : 1;
  }

  private get frames(): number {
    return this.samples.length / this.ch;
  }

  meta(): WaveformInfo & SourceMeta {
    const { axis, sampleRate } = this.info_;
    return {
      id: this.id_,
      revision: this.rev,
      axis,
      extent: this.frames / this.rate,
      channels: this.ch,
      ...(sampleRate !== undefined ? { sampleRate } : {}),
    };
  }

  subscribe(onChange: () => void): () => void {
    this.listeners.add(onChange);
    return () => this.listeners.delete(onChange);
  }

  /** Publish new data — bumps `revision`, notifies subscribers. */
  replace(samples: ArrayLike<number>): void {
    if (samples.length % this.ch !== 0) {
      throw new Error("samples length must be a multiple of channels");
    }
    this.samples = samples;
    this.rev += 1;
    for (const fn of this.listeners) fn();
  }

  private sampleAt(frame: number, channel: number): number {
    return this.samples[frame * this.ch + channel] ?? Number.NaN;
  }

  query(q: WaveformQuery, signal?: AbortSignal): Promise<WaveformView> {
    if (signal?.aborted === true) {
      return Promise.reject(new DOMException("aborted", "AbortError"));
    }
    const rev = this.rev;
    const startF = Math.max(0, Math.floor(q.start * this.rate));
    const endF = Math.min(this.frames, Math.ceil(q.end * this.rate));
    const channels =
      q.channels === undefined || q.channels.length === 0
        ? Array.from({ length: this.ch }, (_, i) => i)
        : q.channels.filter((c) => c >= 0 && c < this.ch);
    const spanF = endF - startF;
    if (spanF <= 0 || q.buckets <= 0 || channels.length === 0) {
      return Promise.resolve({
        kind: "bins",
        revision: rev,
        start: q.start,
        unitsPerBucket: q.buckets > 0 ? (q.end - q.start) / q.buckets : 1,
        channels: channels.map(() => []),
      });
    }
    if (spanF <= q.buckets) {
      // Zoomed in: bounded raw samples — `spanF <= buckets` keeps the
      // memory bound identical to the bins path.
      return Promise.resolve({
        kind: "samples",
        revision: rev,
        start: startF / this.rate,
        unitsPerSample: 1 / this.rate,
        channels: channels.map((c) => {
          const out = new Float64Array(spanF);
          for (let f = 0; f < spanF; f++) out[f] = this.sampleAt(startF + f, c);
          return out;
        }),
      });
    }
    const perBucket = spanF / q.buckets;
    const start = startF / this.rate;
    return Promise.resolve({
      kind: "bins",
      revision: rev,
      start,
      unitsPerBucket: perBucket / this.rate,
      channels: channels.map((c) => {
        const buckets: WaveformBucket[] = new Array(q.buckets);
        for (let b = 0; b < q.buckets; b++) {
          const f0 = startF + Math.floor(b * perBucket);
          const f1 = Math.min(endF, startF + Math.floor((b + 1) * perBucket));
          let min = Number.NaN;
          let max = Number.NaN;
          let sumSq = 0;
          let count = 0;
          for (let f = f0; f < f1; f++) {
            const v = this.sampleAt(f, c);
            if (!Number.isFinite(v)) continue;
            if (count === 0) {
              min = v;
              max = v;
            } else {
              if (v < min) min = v;
              if (v > max) max = v;
            }
            sumSq += v * v;
            count += 1;
          }
          buckets[b] = {
            min,
            max,
            rms: count === 0 ? Number.NaN : Math.sqrt(sumSq / count),
            count,
          };
        }
        return buckets;
      }),
    });
  }
}

/**
 * Manual push-driven snapshot source — the reference impl for meter and
 * spectrum ports. `push` stores the latest complete frame and notifies;
 * `snapshot` is a cheap pull for the draw cycle. `sequence` gaps the
 * producer leaves in `frame.sequence` are preserved verbatim so loss is
 * visible to the view.
 */
export class ManualSnapshotSource<F> implements SnapshotSource<F> {
  private latest: F | null = null;
  private readonly listeners = new Set<() => void>();
  private readonly meta_: SourceMeta;

  constructor(meta_: SourceMeta) {
    this.meta_ = meta_;
  }

  meta(): SourceMeta {
    return this.meta_;
  }

  push(frame: F): void {
    this.latest = frame;
    for (const fn of this.listeners) fn();
  }

  snapshot(): F | null {
    return this.latest;
  }

  subscribe(onUpdate: () => void): () => void {
    this.listeners.add(onUpdate);
    return () => this.listeners.delete(onUpdate);
  }
}
