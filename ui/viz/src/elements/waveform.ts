/**
 * `<rvf-waveform>` — Reference Viz waveform element (Issue #260).
 *
 * Owns the bounded query pipeline around `waveformScene`:
 *
 * - `source` is a {@link WaveformDataSource} port (meta + bounded
 *   `query` + optional streaming `subscribe`). A viewport change, a
 *   resize (LOD budget = plot px), or a source notification triggers a
 *   re-query; a style or overlay change never does (§12.5).
 * - Responses are checked against the meta revision captured at issue
 *   time **and** the meta revision at resolve time — a stale answer is
 *   discarded and re-queried once (bounded), then marked rather than
 *   drawn as current (§12.6).
 * - Views are cached in an `LruViewCache` keyed by
 *   source-id+revision+geometry — panning back is a paint, not a
 *   query. Style is never in a cache key.
 * - Pointer interaction emits `rvf-viewport` (pan/wheel-zoom) and
 *   `rvf-select` (Shift+drag selection). A drag-in-progress selection is
 *   drawn transiently; the authoritative selection stays with the
 *   consumer (`overlays`), per §12.4/§13.
 *
 * Nothing here knows DAW state: overlays, markers, and the selection
 * the consumer echoes back are positions, not semantics.
 */

import { LruViewCache, waveformViewKey } from "../cache.ts";
import type { VizLayout } from "../layout.ts";
import type { DrawOp } from "../ops.ts";
import {
  frameOps,
  staleOps,
  type OverlayState,
} from "../scene.ts";
import type {
  WaveformDataSource,
  WaveformView,
} from "../source.ts";
import type { ResolvedVizStyle } from "../style.ts";
import {
  viewport,
  vpClamp,
  vpPan,
  vpToUnit,
  vpZoom,
  type Viewport,
} from "../viewport.ts";
import { waveformScene } from "../waveform.ts";
import {
  RvfVizElement,
  RVF_SELECT_EVENT,
  RVF_VIEWPORT_EVENT,
} from "./viz-element.ts";

export interface WaveformRange {
  readonly min: number;
  readonly max: number;
}

const DEFAULT_CACHE_BYTES = 8 * 1024 * 1024;

export class RvfWaveform extends RvfVizElement {
  private source_: WaveformDataSource | undefined;
  private span_: { readonly start: number; readonly end: number } | undefined;
  private overlays_: OverlayState | undefined;
  private range_: WaveformRange = { min: -1, max: 1 };
  private channelLabels_: readonly string[] | undefined;
  private cache_ = new LruViewCache<WaveformView>(DEFAULT_CACHE_BYTES);
  private view_: WaveformView | undefined;
  private viewStale_ = false;
  private querySeq = 0;
  private abort_: AbortController | undefined;
  private unsubscribe_: (() => void) | undefined;
  private drag:
    | {
        readonly mode: "pan" | "select";
        readonly pointerId: number;
        /** Drag-start axis position (select mode). */
        readonly u0: number;
        lastX: number;
      }
    | undefined;
  private pendingSelect: { start: number; end: number } | undefined;
  private lastLayout_: VizLayout | undefined;
  private invalidateQueued_ = false;

  // ── typed properties ──────────────────────────────────────────────

  /** The bounded waveform port. Swapping aborts in-flight queries,
   * clears cached views (a new source id would miss anyway), and
   * re-queries the current span. */
  get source(): WaveformDataSource | undefined {
    return this.source_;
  }
  set source(s: WaveformDataSource | undefined) {
    if (s === this.source_) return;
    this.unsubscribe_?.();
    this.unsubscribe_ = undefined;
    this.abort_?.abort();
    this.source_ = s;
    this.view_ = undefined;
    this.viewStale_ = false;
    this.cache_.clear();
    if (s !== undefined) {
      this.unsubscribe_ = s.subscribe?.(() => this.onSourceChanged());
      this.invalidateView(false);
    }
  }

  /** Visible span in axis units (`{start,end}`, end > start). Unset →
   * fits the source extent on the next draw. Pixel width comes from the
   * plot rect (the LOD budget). */
  get viewport(): { readonly start: number; readonly end: number } | undefined {
    return this.span_;
  }
  set viewport(v: { readonly start: number; readonly end: number } | undefined) {
    this.span_ = v === undefined ? undefined : { ...v };
    this.invalidateView(false);
  }

  /** Consumer-supplied overlays (selection/playhead/markers) — repaint
   * only, never a re-query. */
  get overlays(): OverlayState | undefined {
    return this.overlays_;
  }
  set overlays(o: OverlayState | undefined) {
    this.overlays_ = o;
    this.drawSoon();
  }

  /** Amplitude range mapped to a lane (display mapping). */
  get range(): WaveformRange {
    return this.range_;
  }
  set range(r: WaveformRange) {
    this.range_ = { ...r };
    this.drawSoon();
  }

  /** Optional per-lane labels (top-left of each channel lane). */
  get channelLabels(): readonly string[] | undefined {
    return this.channelLabels_;
  }
  set channelLabels(l: readonly string[] | undefined) {
    this.channelLabels_ = l === undefined ? undefined : [...l];
    this.drawSoon();
  }

  /** Byte-budgeted view cache (inspectable for tests/diagnostics). */
  get viewCache(): LruViewCache<WaveformView> {
    return this.cache_;
  }

  // ── lifecycle ─────────────────────────────────────────────────────

  protected override onElementConnect(): void {
    this.upgradeProperty("source");
    this.upgradeProperty("viewport");
    this.upgradeProperty("overlays");
    this.upgradeProperty("range");
    this.upgradeProperty("channelLabels");
    if (this.source_ !== undefined) {
      // The setter may have subscribed already (pre-connect
      // assignment) — replace it rather than stacking a second
      // listener the cleanup list would never remove.
      this.unsubscribe_?.();
      this.unsubscribe_ = this.source_.subscribe?.(() =>
        this.onSourceChanged(),
      );
      this.invalidateView(false);
    }
    this.addCleanup(() => {
      this.unsubscribe_?.();
      this.unsubscribe_ = undefined;
      this.abort_?.abort();
      this.drag = undefined;
      this.pendingSelect = undefined;
    });
  }

  protected override onCanvasesReady(): void {
    const plot = this.plotCanvas_;
    if (plot === undefined) return;
    plot.addEventListener("pointerdown", (e) => this.onPointerDown(e));
    plot.addEventListener("pointermove", (e) => this.onPointerMove(e));
    plot.addEventListener("pointerup", (e) => this.onPointerUp(e));
    plot.addEventListener("pointercancel", (e) => {
      if (this.drag !== undefined && this.drag.pointerId !== e.pointerId) {
        return;
      }
      this.drag = undefined;
      this.pendingSelect = undefined;
      this.drawSoon();
    });
    plot.addEventListener("lostpointercapture", (e) => {
      // Capture lost without pointerup (browser reclaim, element
      // detached mid-gesture) — end the gesture rather than leaving
      // drag/pendingSelect armed with stale anchors.
      if (this.drag !== undefined && this.drag.pointerId === e.pointerId) {
        this.drag = undefined;
        this.pendingSelect = undefined;
        this.drawSoon();
      }
    });
    plot.addEventListener("wheel", (e) => this.onWheel(e));
  }

  protected override onResize(): void {
    // Plot px is the LOD budget — a resize is a new query, not a restyle.
    this.invalidateView(false);
  }

  // ── query pipeline ────────────────────────────────────────────────

  private effectiveSpan(metaExtent: number): { start: number; end: number } {
    if (!Number.isFinite(metaExtent) || metaExtent <= 0) {
      return { start: 0, end: 0 };
    }
    const s = this.span_;
    if (s !== undefined && s.end > s.start) {
      const end = Math.min(s.end, metaExtent);
      const start = Math.min(Math.max(s.start, 0), Math.max(0, end - 1e-9));
      return start < end ? { start, end } : { start: 0, end: metaExtent };
    }
    return { start: 0, end: metaExtent };
  }

  /** Issue a bounded query for the visible span. `retry` marks the
   * single allowed re-issue after a stale-response rejection — bounded
   * recursion, never a publish-storm loop. */
  private invalidateView(retry: boolean): void {
    const source = this.source_;
    if (source === undefined || !this.isConnected) return;
    // The LOD budget is the plot px, not the element width — before the
    // first layout there is no budget, so the query is deferred to the
    // first sceneOps call (which queues one after recording layout).
    const plotW = this.lastLayout_?.plot.w ?? 0;
    if (!(plotW > 0)) return;
    const meta = source.meta();
    const span = this.effectiveSpan(meta.extent);
    // Every invalidate supersedes whatever came before — a superseded
    // in-flight query must never land, even when this call resolves
    // from cache or finds nothing to query.
    const seq = ++this.querySeq;
    this.abort_?.abort();
    this.abort_ = undefined;
    if (!(span.end > span.start) || !Number.isFinite(span.end)) {
      this.view_ = undefined;
      this.viewStale_ = false;
      this.drawSoon();
      return;
    }
    const buckets = Math.max(1, Math.round(plotW));
    const key = waveformViewKey(
      meta.id,
      meta.revision,
      -1,
      span.start,
      span.end,
      buckets,
    );
    const cached = this.cache_.get(key);
    if (cached !== undefined) {
      this.view_ = cached;
      this.viewStale_ = false;
      this.setStatus("");
      this.drawSoon();
      return;
    }
    const ctl = new AbortController();
    this.abort_ = ctl;
    this.viewStale_ = this.view_ !== undefined;
    this.drawSoon();
    void source
      .query({ start: span.start, end: span.end, buckets }, ctl.signal)
      .then((view) => {
        if (ctl.signal.aborted || seq !== this.querySeq) return;
        this.abort_ = undefined; // terminal: no longer in flight
        // Stale-response rejection: the revision the view was produced
        // from must match both the issued meta and the *current* meta —
        // a publish between issue and resolve makes this answer stale.
        const now = source.meta().revision;
        if (view.revision !== meta.revision || view.revision !== now) {
          if (!retry) {
            this.invalidateView(true);
          } else {
            this.viewStale_ = true;
            this.drawSoon();
          }
          return;
        }
        this.view_ = view;
        this.viewStale_ = false;
        const bytes = view.channels.reduce(
          (n, ch) => n + ch.length * 4 * 8,
          0,
        );
        this.cache_.set(key, view, Math.max(1, bytes));
        this.setStatus("");
        this.drawSoon();
      })
      .catch((err: unknown) => {
        if (ctl.signal.aborted) return;
        this.abort_ = undefined;
        this.viewStale_ = true;
        this.setStatus(
          `source error: ${err instanceof Error ? err.message : String(err)}`,
        );
        this.drawSoon();
      });
  }

  private onSourceChanged(): void {
    // New data for the same source — the previous view is stale NOW
    // (drawn with the stale cue until the replacement lands).
    this.viewStale_ = this.view_ !== undefined;
    this.invalidateView(false);
  }

  // ── interaction ───────────────────────────────────────────────────

  private pxViewport(): Viewport | undefined {
    const l = this.lastLayout_;
    if (l === undefined || this.source_ === undefined) return undefined;
    const meta = this.source_.meta();
    const span = this.effectiveSpan(meta.extent);
    const vp = viewport(span.start, span.end, l.plot.w);
    if (vp === undefined) return undefined;
    return vpClamp(vp, 0, meta.extent);
  }

  private unitAt(clientX: number): number | undefined {
    const l = this.lastLayout_;
    const vp = this.pxViewport();
    if (l === undefined || vp === undefined) return undefined;
    const rect = this.plotCanvas_?.getBoundingClientRect();
    const x = clientX - (rect?.left ?? 0) - l.plot.x;
    return vpToUnit(vp, x);
  }

  private onPointerDown(e: PointerEvent): void {
    if (e.button !== 0 || this.source_ === undefined || this.drag !== undefined) {
      return; // one gesture at a time
    }
    const select = e.shiftKey;
    this.drag = {
      mode: select ? "select" : "pan",
      pointerId: e.pointerId,
      u0: select ? (this.unitAt(e.clientX) ?? Number.NaN) : Number.NaN,
      lastX: e.clientX,
    };
    if (select && Number.isFinite(this.drag.u0)) {
      this.pendingSelect = { start: this.drag.u0, end: this.drag.u0 };
    }
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
    e.preventDefault();
  }

  private onPointerMove(e: PointerEvent): void {
    const d = this.drag;
    const vp = this.pxViewport();
    if (d === undefined || d.pointerId !== e.pointerId || vp === undefined) {
      return;
    }
    if (d.mode === "pan") {
      const dx = e.clientX - d.lastX;
      if (dx !== 0) {
        d.lastX = e.clientX;
        const next = vpPan(vp, dx);
        const meta = this.source_?.meta();
        const clamped = meta === undefined ? next : vpClamp(next, 0, meta.extent);
        this.span_ = { start: clamped.start, end: clamped.end };
        this.emit(RVF_VIEWPORT_EVENT, {
          viewport: { start: clamped.start, end: clamped.end },
          source: "pan",
        });
        this.invalidateView(false);
      }
    } else {
      const u = this.unitAt(e.clientX);
      if (u !== undefined && Number.isFinite(d.u0)) {
        this.pendingSelect = { start: d.u0, end: u };
        this.drawSoon();
      }
    }
  }

  private onPointerUp(e: PointerEvent): void {
    const d = this.drag;
    if (d === undefined || d.pointerId !== e.pointerId) return;
    this.drag = undefined;
    if (d.mode === "select" && this.pendingSelect !== undefined) {
      const { start, end } = this.pendingSelect;
      this.pendingSelect = undefined;
      if (Number.isFinite(start) && Number.isFinite(end) && start !== end) {
        this.emit(RVF_SELECT_EVENT, {
          start: Math.min(start, end),
          end: Math.max(start, end),
        });
      }
      this.drawSoon();
    }
    void e;
  }

  private onWheel(e: WheelEvent): void {
    const vp = this.pxViewport();
    if (vp === undefined) return;
    e.preventDefault();
    const anchor = this.unitAt(e.clientX);
    if (anchor === undefined) return;
    const next = vpZoom(vp, e.deltaY < 0 ? 1.25 : 0.8, anchor);
    if (next === undefined) return;
    const meta = this.source_?.meta();
    const clamped = meta === undefined ? next : vpClamp(next, 0, meta.extent);
    this.span_ = { start: clamped.start, end: clamped.end };
    this.emit(RVF_VIEWPORT_EVENT, {
      viewport: { start: clamped.start, end: clamped.end },
      source: "wheel",
    });
    this.invalidateView(false);
  }

  // ── scene ─────────────────────────────────────────────────────────

  protected sceneOps(
    layout: VizLayout,
    style: ResolvedVizStyle,
  ): DrawOp[] | null {
    this.lastLayout_ = layout;
    const source = this.source_;
    if (source === undefined) {
      this.setStatus("no source");
      return [...frameOps(layout, style)];
    }
    const vp = this.pxViewport();
    if (vp === undefined) return null;
    const sel = this.pendingSelect ?? this.overlays_?.selection;
    const overlays: OverlayState | undefined =
      sel !== undefined || this.overlays_ !== undefined
        ? {
            ...this.overlays_,
            ...(sel !== undefined ? { selection: sel } : {}),
          }
        : undefined;
    const meta = source.meta();
    if (this.view_ === undefined) {
      // Deferred first query — the LOD budget (plot px) exists now.
      // Multiple draws in one tick queue at most one issue, and never
      // re-issue while a query is already in flight.
      const inFlight =
        this.abort_ !== undefined && !this.abort_.signal.aborted;
      if (!this.viewStale_ && !this.invalidateQueued_ && !inFlight) {
        this.invalidateQueued_ = true;
        queueMicrotask(() => {
          this.invalidateQueued_ = false;
          this.invalidateView(false);
        });
      }
      this.setStatus(this.viewStale_ ? "stale" : "");
      return [...frameOps(layout, style)];
    }
    const spu =
      meta.axis === "seconds" && meta.sampleRate !== undefined
        ? meta.sampleRate
        : meta.axis === "samples"
          ? 1
          : undefined;
    const ops: DrawOp[] = waveformScene(
      {
        viewport: vp,
        view: this.view_,
        range: this.range_,
        ...(overlays !== undefined ? { overlays } : {}),
        axisUnit: meta.axis,
        ...(spu !== undefined ? { samplesPerUnit: spu } : {}),
        ...(this.channelLabels_ !== undefined
          ? { channelLabels: this.channelLabels_ }
          : {}),
      },
      layout,
      style,
    );
    if (this.viewStale_) {
      ops.push(...staleOps(layout, style));
      this.setStatus("stale", "stale");
    } else {
      this.setStatus("");
    }
    return ops;
  }
}
