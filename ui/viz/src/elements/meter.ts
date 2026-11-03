/**
 * `<rvf-meter>` — Reference Viz level-meter element (Issue #260).
 *
 * Snapshot-pull renderer over {@link MeterDataSource}: the shared pump
 * pulls the latest complete `MeterFrame` on the draw cycle, tracks
 * sequence-gap loss and stale aging, and `meterScene` paints peak /
 * RMS / held-peak / clip per channel with the dB scale as a pure
 * display mapping. `NaN` channels render as unavailable hatches — never
 * as live level; stale renders the hatch+badge, never as live data.
 *
 * `orientation="vertical"` (default) draws bars that grow up with
 * channels side-by-side; `"horizontal"` draws right-growing stacked
 * bars — the attribute changes structure, so it re-renders.
 */

import type { VizLayout } from "../layout.ts";
import type { DrawOp } from "../ops.ts";
import { frameOps } from "../scene.ts";
import type { MeterDataSource, MeterFrame } from "../source.ts";
import type { ResolvedVizStyle } from "../style.ts";
import {
  DEFAULT_METER_SCALE,
  meterScene,
  type MeterScale,
} from "../meter.ts";
import { SnapshotVizElement } from "./snapshot-element.ts";

export class RvfMeter extends SnapshotVizElement<MeterFrame> {
  private source_: MeterDataSource | undefined;
  private scale_: MeterScale = DEFAULT_METER_SCALE;

  /** The meter snapshot port. */
  get source(): MeterDataSource | undefined {
    return this.source_;
  }
  set source(s: MeterDataSource | undefined) {
    if (s === this.source_) return;
    this.source_ = s;
    this.frame_ = null;
    this.setSnapshotSource(s);
    this.drawSoon();
  }

  /** dB scale — display mapping only; repaints, never touches data.
   * Partial/misspelled shapes merge over the reference scale rather
   * than rendering NaN geometry. */
  get scale(): MeterScale {
    return this.scale_;
  }
  set scale(s: MeterScale) {
    this.scale_ = { ...DEFAULT_METER_SCALE, ...s };
    this.drawSoon();
  }

  protected override onElementConnect(): void {
    this.upgradeProperty("source");
    this.upgradeProperty("scale");
    super.onElementConnect();
  }

  protected pull(): MeterFrame | null {
    return this.source_?.snapshot() ?? null;
  }

  protected override attributeChangedCallback(
    name: string,
    oldValue: string | null,
    newValue: string | null,
  ): void {
    if (name === "orientation" && oldValue !== newValue) {
      this.drawSoon();
      return;
    }
    super.attributeChangedCallback(name, oldValue, newValue);
  }

  protected sceneOps(
    layout: VizLayout,
    style: ResolvedVizStyle,
  ): DrawOp[] | null {
    if (this.source_ === undefined) {
      this.setStatus("no source");
      return [...frameOps(layout, style)];
    }
    const ops = meterScene(
      {
        channels: this.frame_?.channels ?? [],
        scale: this.scale_,
        stale: this.stale,
        dropped: this.dropped,
        orientation:
          this.getAttribute("orientation") === "horizontal"
            ? "horizontal"
            : "vertical",
      },
      layout,
      style,
    );
    // staleOps/loss badge come from meterScene; the DOM status mirrors
    // (loss is text too — pixels are invisible to assistive tech).
    const bits: string[] = [];
    if (this.stale) bits.push("stale");
    else if (this.frame_ === null) bits.push("awaiting data");
    if (this.dropped > 0) bits.push(`${this.dropped} dropped`);
    this.setStatus(bits.join(", "), this.stale ? "stale" : undefined);
    return ops;
  }
}
