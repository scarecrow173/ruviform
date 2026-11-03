/**
 * `SnapshotVizElement` — shared element base for snapshot-pull
 * renderers (meter, spectrum; Issue #260 §12.4/§12.6).
 *
 * The measurement cycle lives behind the source port; the draw cycle
 * lives here. The pump is a *pull* on each draw tick — `snapshot()` is
 * contractually cheap and non-blocking — plus an immediate pull when
 * the source's optional `subscribe` notifies. Bookkeeping the contract
 * requires:
 *
 * - `sequence` gaps accumulate into `dropped` (loss is rendered, never
 *   hidden),
 * - a snapshot older than `staleAfterMs` flips `stale` — once, on the
 *   transition — driving the stale cue + DOM status,
 * - the pump is an interval/animation-frame loop ONLY while connected;
 *   disconnect stops it via the shared cleanup list,
 * - `autopump = false` + `pumpNow()` give tests a deterministic tick.
 */

import { RvfVizElement } from "./viz-element.ts";

export interface SnapshotFrame {
  readonly sequence: number;
  readonly dropped?: number;
}

export abstract class SnapshotVizElement<
  F extends SnapshotFrame,
> extends RvfVizElement {
  private lastSeq_: number | undefined;
  private lastAt_ = 0;
  private dropped_ = 0;
  private stale_ = false;
  private unsubscribe_: (() => void) | undefined;
  private snapshotSrc_:
    | { subscribe?(onUpdate: () => void): () => void }
    | undefined;
  private pumpId: number | undefined;
  private pumpIsRaf = false;
  private autopump_ = true;

  /** Injectable clock (ms) — tests drive stale aging deterministically. */
  clock: () => number = () => Date.now();

  /** ms without a new sequence before the frame is stale. */
  staleAfterMs = 1000;

  /** Latest complete frame (null until the first snapshot lands). */
  protected frame_: F | null = null;

  /** Whole-view stale flag (snapshot aged out). */
  get stale(): boolean {
    return this.stale_;
  }

  /** Frames lost since the stream began (sequence gaps + source-
   * reported drops — both count, deduplicated by the source contract:
   * `dropped` reports producer-side loss the sequence cannot see). */
  get dropped(): number {
    return this.dropped_;
  }

  /** Draw-cycle pump on/off (default on while connected). */
  get autopump(): boolean {
    return this.autopump_;
  }
  set autopump(on: boolean) {
    this.autopump_ = on;
    if (on) this.startPump();
    else this.stopPump();
  }

  /** Pull one snapshot from the port (cheap, non-blocking; null when
   * none has been produced yet). */
  protected abstract pull(): F | null;

  /** Rebind the source's update notification (or clear it). Called by
   * subclass source setters with the new port (its `subscribe`, when
   * present, drives an immediate extra pull + repaint). A source swap
   * resets the stream bookkeeping — sequence numbers from different
   * sources are incomparable, so carrying `lastSeq_`/`dropped_` over
   * would report phantom loss. */
  protected setSnapshotSource(
    src: { subscribe?(onUpdate: () => void): () => void } | undefined,
  ): void {
    this.snapshotSrc_ = src;
    this.unsubscribe_?.();
    this.unsubscribe_ = src?.subscribe?.(() => this.pumpNow());
    this.lastSeq_ = undefined;
    this.lastAt_ = 0;
    this.dropped_ = 0;
    this.stale_ = false;
  }

  protected override onElementConnect(): void {
    this.upgradeProperty("autopump");
    this.addCleanup(() => {
      this.stopPump();
      this.unsubscribe_?.();
      this.unsubscribe_ = undefined;
      this.frame_ = null;
      this.stale_ = false;
      this.lastSeq_ = undefined;
      this.dropped_ = 0;
    });
    // Rebind the notify subscription — disconnect's cleanup removed it
    // and only the pump restart below resumed pulls.
    this.unsubscribe_?.();
    this.unsubscribe_ = this.snapshotSrc_?.subscribe?.(() =>
      this.pumpNow(),
    );
    if (this.autopump_) this.startPump();
  }

  /** One pump tick — the deterministic test entry point. Pulls the
   * latest frame, accumulates loss, ages staleness, repaints when
   * either changed. */
  pumpNow(): void {
    let f: F | null;
    try {
      f = this.pull();
    } catch (err: unknown) {
      // A misbehaving source must not kill the pump — surface it as a
      // state and keep the tick alive for the next cycle.
      this.setStatus(
        `source error: ${err instanceof Error ? err.message : String(err)}`,
        "unsupported",
      );
      return;
    }
    const now = this.clock();
    if (
      f !== null &&
      Number.isFinite(f.sequence) &&
      f.sequence !== this.lastSeq_
    ) {
      if (this.lastSeq_ !== undefined && f.sequence > this.lastSeq_ + 1) {
        this.dropped_ += f.sequence - this.lastSeq_ - 1;
      }
      this.dropped_ += Number.isFinite(f.dropped) ? (f.dropped ?? 0) : 0;
      this.frame_ = f;
      this.lastSeq_ = f.sequence;
      this.lastAt_ = now;
      if (this.stale_) this.stale_ = false;
      this.drawSoon();
      return;
    }
    if (this.lastSeq_ !== undefined && !this.stale_) {
      if (now - this.lastAt_ > this.staleAfterMs) {
        this.stale_ = true;
        this.drawSoon();
      }
    }
  }

  private startPump(): void {
    if (this.pumpId !== undefined || !this.isConnected) return;
    const win = this.ownerDocument?.defaultView;
    const raf = win?.requestAnimationFrame;
    if (typeof raf === "function") {
      this.pumpIsRaf = true;
      const loop = () => {
        if (this.pumpId === undefined || !this.pumpIsRaf) return;
        this.pumpNow();
        this.pumpId = raf.call(win, loop) as number;
      };
      this.pumpId = raf.call(win, loop) as number;
    } else {
      this.pumpIsRaf = false;
      this.pumpId = setInterval(() => this.pumpNow(), 16) as unknown as number;
    }
  }

  private stopPump(): void {
    if (this.pumpId === undefined) return;
    const win = this.ownerDocument?.defaultView;
    if (this.pumpIsRaf && typeof win?.cancelAnimationFrame === "function") {
      win.cancelAnimationFrame(this.pumpId);
    } else {
      clearInterval(this.pumpId);
    }
    this.pumpId = undefined;
  }
}
