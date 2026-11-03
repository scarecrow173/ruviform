/**
 * `<rvf-spectrum>` — Reference Viz spectrum element (Issue #260).
 *
 * Snapshot-pull renderer over {@link SpectrumDataSource}: the shared
 * pump pulls the latest `SpectrumFrame` on the draw cycle and
 * `spectrumScene` paints the dB curve + under-fill on a linear or log
 * frequency axis. `freq`/`db` are display mappings only — repaints,
 * never re-analysis (FFT stays behind the source port).
 *
 * `NaN` bins surface as axis gap ticks; stale frames get the hatch +
 * DOM status; sequence gaps accumulate into the loss badge.
 */

import type { VizLayout } from "../layout.ts";
import type { DrawOp } from "../ops.ts";
import { frameOps } from "../scene.ts";
import type { SpectrumDataSource, SpectrumFrame } from "../source.ts";
import type { ResolvedVizStyle } from "../style.ts";
import { spectrumScene, type SpectrumModel } from "../spectrum.ts";
import { SnapshotVizElement } from "./snapshot-element.ts";

export class RvfSpectrum extends SnapshotVizElement<SpectrumFrame> {
  private source_: SpectrumDataSource | undefined;
  private freq_: SpectrumModel["freq"] = {
    scale: "log",
    minHz: 20,
    maxHz: 20000,
  };
  private db_: SpectrumModel["db"] = {
    minDb: -90,
    maxDb: 0,
    reference: 1,
  };

  /** The spectrum snapshot port. */
  get source(): SpectrumDataSource | undefined {
    return this.source_;
  }
  set source(s: SpectrumDataSource | undefined) {
    if (s === this.source_) return;
    this.source_ = s;
    this.frame_ = null;
    this.setSnapshotSource(s);
    this.drawSoon();
  }

  /** Frequency window + scale — display mapping. `scale: "log"`
   * requires `minHz > 0`. */
  get freq(): SpectrumModel["freq"] {
    return this.freq_;
  }
  set freq(f: SpectrumModel["freq"]) {
    this.freq_ = { ...f };
    this.drawSoon();
  }

  /** dB window + full-scale reference — display mapping. */
  get db(): SpectrumModel["db"] {
    return this.db_;
  }
  set db(d: SpectrumModel["db"]) {
    this.db_ = { ...d };
    this.drawSoon();
  }

  protected override onElementConnect(): void {
    this.upgradeProperty("source");
    this.upgradeProperty("freq");
    this.upgradeProperty("db");
    super.onElementConnect();
  }

  protected pull(): SpectrumFrame | null {
    return this.source_?.snapshot() ?? null;
  }

  protected sceneOps(
    layout: VizLayout,
    style: ResolvedVizStyle,
  ): DrawOp[] | null {
    if (this.source_ === undefined) {
      this.setStatus("no source");
      return [...frameOps(layout, style)];
    }
    const info = this.source_.meta();
    const frame = this.frame_;
    const ops: DrawOp[] = spectrumScene(
      {
        magnitudes: frame?.magnitudes ?? [],
        axis: info.axis,
        freq: this.freq_,
        db: this.db_,
        stale: this.stale,
        dropped: this.dropped,
      },
      layout,
      style,
    );
    const bits: string[] = [];
    if (this.stale) bits.push("stale");
    else if (frame === null) bits.push("awaiting data");
    if (this.dropped > 0) bits.push(`${this.dropped} dropped`);
    this.setStatus(bits.join(", "), this.stale ? "stale" : undefined);
    return ops;
  }
}
