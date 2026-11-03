/**
 * `RvfVizElement` — shared Custom-Element base for the Reference Viz
 * renderers (Issue #260, §13 framework-neutral surface).
 *
 * Owns the drawing chrome and lifecycle — nothing data-semantic:
 * - stacked canvases (plot + overlay) sized by `setSize`/`ResizeObserver`
 *   with device-pixel-ratio backing stores,
 * - backend creation via `createVizBackend` (preference attribute
 *   `backend="auto|canvas2d|webgl2"`), fallback + context-loss swap to
 *   Canvas2D, and the `rvf-backend` event as the capability record,
 * - VizStyle resolution (`collectVizTokens` → `resolveVizStyle`) with a
 *   MutationObserver on ancestor inline styles so a theme swap only
 *   repaints — never re-queries data (§12.5),
 * - the `status` part + `role="img"` canvas aria-label as the non-color
 *   state channel (stale/unsupported/loss are text, not just hue),
 * - draw scheduling (`drawSoon` → rAF/microtask) and per-element stats.
 *
 * Data flow (subscribe/query, model building) stays in subclasses.
 */

import { RvfElement } from "../../../components/src/element.ts";
import { tv } from "../../../components/src/tokens.ts";
import { createVizBackend, type BackendPreference, type VizBackend } from "../backend.ts";
import type { VizLayout } from "../layout.ts";
import { computeLayout } from "../layout.ts";
import type { DrawOp } from "../ops.ts";
import {
  resolveVizStyle,
  collectVizTokens,
  type ResolvedVizStyle,
  type VizStyleDiagnostic,
  type VizStyleInput,
} from "../style.ts";

export const RVF_BACKEND_EVENT = "rvf-backend";
export const RVF_VIEWPORT_EVENT = "rvf-viewport";
export const RVF_SELECT_EVENT = "rvf-select";

export interface VizStats {
  /** Completed draws since connect. */
  readonly framesDrawn: number;
  /** DrawOps in the last frame. */
  readonly lastOpCount: number;
  /** Wall time of the last scene+execute (ms, `performance.now`). */
  readonly lastDrawMs: number;
}

export abstract class RvfVizElement extends RvfElement {
  static observedAttributes = ["label", "backend", "orientation"];

  protected plotCanvas_: HTMLCanvasElement | undefined;
  protected overlayCanvas_: HTMLCanvasElement | undefined;
  private status_: HTMLElement | undefined;
  private resizeObs: ResizeObserver | undefined;
  private themeObs: MutationObserver | undefined;
  private backend_: VizBackend | null | undefined;
  private backendKind_ = "";
  /** The plot canvas may be pinned to WebGL mode (a successful
   * getContext("webgl2") even when the executor then failed) — a
   * subsequent getContext("2d") returns null forever, so fallback must
   * swap in a fresh canvas node. */
  private glAttempted_ = false;
  private canvasSwapped_ = false;
  private cssWidth = 0;
  private cssHeight = 0;
  private styleDirty = true;
  private style_: ResolvedVizStyle | undefined;
  private diagnostics_: VizStyleDiagnostic[] = [];
  private drawQueued = false;
  private stats_: VizStats = { framesDrawn: 0, lastOpCount: 0, lastDrawMs: 0 };
  private appearance_: VizStyleInput | undefined;
  private statusText = "";
  private statusReason = "";

  // ── typed properties ──────────────────────────────────────────────

  /** Paint overrides layered over the resolved token theme. */
  get appearance(): VizStyleInput | undefined {
    return this.appearance_;
  }
  set appearance(a: VizStyleInput | undefined) {
    this.appearance_ = a === undefined ? undefined : { ...a };
    this.styleDirty = true;
    this.drawSoon();
  }

  /** Backend preference (`backend` attribute): `auto` tries WebGL2 then
   * Canvas2D; explicit choices degrade to Canvas2D with a recorded
   * reason rather than rendering nothing. */
  get backendPreference(): BackendPreference {
    const v = this.getAttribute("backend");
    return v === "canvas2d" || v === "webgl2" ? v : "auto";
  }
  set backendPreference(p: BackendPreference) {
    this.setAttribute("backend", p);
  }

  /** The backend actually in use (after fallback), when created. */
  get backendKind(): string {
    return this.backend_?.kind ?? "none";
  }

  /** Diagnostics from the last VizStyle resolution. */
  get styleDiagnostics(): readonly VizStyleDiagnostic[] {
    return this.diagnostics_;
  }

  /** Draw counters — fixture-derived perf evidence (last frame only). */
  get stats(): VizStats {
    return this.stats_;
  }

  /** Current canvas CSS size (set via `setSize`/ResizeObserver). */
  get size(): { readonly width: number; readonly height: number } {
    return { width: this.cssWidth, height: this.cssHeight };
  }

  // ── lifecycle ─────────────────────────────────────────────────────

  override connectedCallback(): void {
    super.connectedCallback();
    this.upgradeProperty("appearance");
    this.attachObservers();
    this.onElementConnect();
    this.drawSoon();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback(); // runs the cleanup list
    this.onElementDisconnect();
  }

  /** Subclass hook: source subscribe, initial query, pump start. */
  protected onElementConnect(): void {}
  /** Subclass hook: unsubscribe/abort (runs after shared cleanups). */
  protected onElementDisconnect(): void {}

  protected template(): string {
    return `
      <div class="rvf-viz" part="root">
        <canvas class="plot" part="plot-canvas" role="img"></canvas>
        <canvas class="overlay" part="overlay-canvas" aria-hidden="true"></canvas>
        <div class="status" part="status" role="status" aria-live="polite"></div>
      </div>`;
  }

  protected stylesheet(): string {
    return `
      :host {
        box-sizing: border-box;
        display: inline-block;
        font-family: ${tv("font.family")};
        font-size: ${tv("font.size")};
        color: ${tv("text")};
        -webkit-tap-highlight-color: transparent;
        user-select: none;
      }
      :host([hidden]) { display: none; }
      .rvf-viz { position: relative; display: block; line-height: 0; }
      canvas.plot { position: relative; display: block; }
      canvas.overlay {
        position: absolute; inset: 0;
        pointer-events: none;
      }
      .status {
        position: absolute; right: 4px; bottom: 4px;
        line-height: 1.2; pointer-events: none;
        color: ${tv("viz.stale")};
        background: ${tv("viz.background")};
        padding: 0 3px;
      }
      .status:empty { display: none; }
    `;
  }

  protected afterRender(): void {
    this.plotCanvas_ =
      this.root.querySelector<HTMLCanvasElement>("canvas.plot") ?? undefined;
    this.overlayCanvas_ =
      this.root.querySelector<HTMLCanvasElement>("canvas.overlay") ?? undefined;
    this.status_ =
      this.root.querySelector<HTMLElement>(".status") ?? undefined;
    const plot = this.plotCanvas_;
    if (plot !== undefined) {
      plot.addEventListener("webglcontextlost", (e) => {
        e.preventDefault();
        this.onContextLost();
      });
    }
    // Fresh shadow nodes: re-apply the pending size + backend state.
    if (this.cssWidth > 0 || this.cssHeight > 0) {
      this.syncCanvasSize();
      this.backend_ = undefined;
    }
    this.glAttempted_ = false;
    this.canvasSwapped_ = false;
    this.onCanvasesReady();
  }

  /** Subclass hook for binding canvas listeners after (re)render. */
  protected onCanvasesReady(): void {}

  protected attributeChangedCallback(
    name: string,
    oldValue: string | null,
    newValue: string | null,
  ): void {
    if (oldValue === newValue) return;
    if (name === "backend") {
      this.backend_ = undefined; // recreate on next draw
      this.drawSoon();
      return;
    }
    if (name === "orientation") {
      this.drawSoon();
      return;
    }
    this.syncPresentation();
  }

  protected override syncPresentation(): void {
    this.syncStatus();
  }

  // ── sizing ────────────────────────────────────────────────────────

  /**
   * Set the drawing-surface size in CSS px (ResizeObserver entry point
   * and the deterministic test hook). Resizes the backing stores by the
   * current device-pixel ratio and repaints — a resize is a viewport
   * change, not a data invalidation.
   */
  setSize(width: number, height: number): void {
    if (width === this.cssWidth && height === this.cssHeight) return;
    const w = Number.isFinite(width) ? width : 0;
    const h = Number.isFinite(height) ? height : 0;
    this.cssWidth = Math.max(0, w);
    this.cssHeight = Math.max(0, h);
    this.syncCanvasSize();
    this.backend_ = undefined; // executors hold w/h/dpr — recreate
    this.onResize();
    this.drawSoon();
  }

  /** Push the current css size × dpr onto the canvas backing stores
   * (fresh shadow nodes and resizes both land here). */
  private syncCanvasSize(): void {
    const dpr = this.dpr();
    for (const c of [this.plotCanvas_, this.overlayCanvas_]) {
      if (c === undefined) continue;
      c.width = Math.round(this.cssWidth * dpr);
      c.height = Math.round(this.cssHeight * dpr);
      c.style.width = `${this.cssWidth}px`;
      c.style.height = `${this.cssHeight}px`;
    }
  }

  /** Subclass hook: a size change may need a re-query (viewport px). */
  protected onResize(): void {}

  protected dpr(): number {
    const w = this.ownerDocument?.defaultView;
    const d = w?.devicePixelRatio;
    return typeof d === "number" && Number.isFinite(d) && d > 0 ? d : 1;
  }

  private attachObservers(): void {
    this.addCleanup(() => {
      this.resizeObs?.disconnect();
      this.resizeObs = undefined;
      this.themeObs?.disconnect();
      this.themeObs = undefined;
    });
    if (typeof ResizeObserver !== "undefined" && this.resizeObs === undefined) {
      this.resizeObs = new ResizeObserver((entries) => {
        const r = entries[0]?.contentRect;
        if (r !== undefined) this.setSize(r.width, r.height);
      });
      this.resizeObs.observe(this);
    }
    // Theme swaps arrive as inline-style writes on ancestor scopes
    // (`<rvf-theme>`) or this element (instance `tokens`). Watch the
    // composed chain; any change only dirties the resolved style.
    if (typeof MutationObserver !== "undefined" && this.themeObs === undefined) {
      this.themeObs = new MutationObserver(() => {
        this.styleDirty = true;
        this.drawSoon();
      });
      for (
        let node: Element | null = this;
        node !== null;
        node = composedParent(node)
      ) {
        this.themeObs.observe(node, {
          attributes: true,
          attributeFilter: ["style"],
        });
      }
    }
  }

  // ── drawing ───────────────────────────────────────────────────────

  /** Resolve (and cache) the effective VizStyle: ambient theme tokens
   * + `appearance` overrides. A repaint is the ONLY consequence of a
   * style change — data paths are untouched. */
  protected resolvedStyle(): ResolvedVizStyle {
    if (this.styleDirty || this.style_ === undefined) {
      const { style, diagnostics } = resolveVizStyle(
        collectVizTokens(this),
        this.appearance_,
      );
      this.style_ = style;
      this.diagnostics_ = diagnostics;
      this.styleDirty = false;
    }
    return this.style_;
  }

  /** Coalesced async draw (rAF when available, else microtask). */
  protected drawSoon(): void {
    if (this.drawQueued || !this.isConnected) return;
    this.drawQueued = true;
    const go = () => {
      this.drawQueued = false;
      this.drawNow();
    };
    const raf = this.ownerDocument?.defaultView?.requestAnimationFrame;
    if (typeof raf === "function") raf.call(this.ownerDocument.defaultView, go);
    else queueMicrotask(go);
  }

  /** Synchronous draw entry — also the deterministic test hook. */
  drawNow(): void {
    if (!this.isConnected) return;
    if (this.cssWidth <= 0 || this.cssHeight <= 0) return;
    const t0 =
      typeof performance !== "undefined" ? performance.now() : Date.now();
    const style = this.resolvedStyle();
    const layout = this.layout(style);
    const ops = this.sceneOps(layout, style);
    const backend = this.ensureBackend();
    if (ops === null || backend === null) {
      // DOM-level unsupported status (no drawable surface at all). A
      // subclass-provided statusText (e.g. "no data") wins; otherwise
      // the backend is the thing that's missing.
      this.setStatus(
        this.statusText || "canvas unavailable",
        "unsupported",
      );
      return;
    }
    backend.execute(ops);
    this.stats_ = {
      framesDrawn: this.stats_.framesDrawn + 1,
      lastOpCount: ops.length,
      lastDrawMs:
        (typeof performance !== "undefined" ? performance.now() : Date.now()) -
        t0,
    };
    this.onDrawn();
  }

  /** Hook after a completed frame (status text refresh, etc.). */
  protected onDrawn(): void {}

  /** The scene ops for one frame; `null` → status-only frame
   * (unsupported). Subclasses supply the model → scene call. */
  protected abstract sceneOps(
    layout: VizLayout,
    style: ResolvedVizStyle,
  ): DrawOp[] | null;

  /** Layout params — subclasses with different gutter needs override. */
  protected layout(style: ResolvedVizStyle): VizLayout {
    return computeLayout({
      width: this.cssWidth,
      height: this.cssHeight,
      yAxis: true,
      xAxis: true,
      style,
    });
  }

  private ensureBackend(): VizBackend | null {
    if (this.backend_ !== undefined) return this.backend_;
    const plot = this.plotCanvas_;
    if (plot === undefined) return null;
    const pref = this.backendPreference;
    if (pref !== "canvas2d") this.glAttempted_ = true;
    this.backend_ = createVizBackend(
      { plot, overlay: this.overlayCanvas_ ?? null },
      pref,
      { width: this.cssWidth, height: this.cssHeight, dpr: this.dpr() },
    );
    // Canvas context modes are exclusive: a WebGL2 attempt that reached
    // getContext("webgl2") pins this canvas even when executor setup
    // then failed — the 2D fallback gets null from getContext("2d")
    // forever. Swap in a fresh canvas node once and retry Canvas2D.
    if (this.backend_ === null && this.glAttempted_ && !this.canvasSwapped_) {
      this.canvasSwapped_ = true;
      this.replacePlotCanvas();
      this.backend_ = createVizBackend(
        {
          plot: this.plotCanvas_ as HTMLCanvasElement,
          overlay: this.overlayCanvas_ ?? null,
        },
        "canvas2d",
        { width: this.cssWidth, height: this.cssHeight, dpr: this.dpr() },
      );
    }
    if (this.backend_ !== null) this.canvasSwapped_ = false;
    const kind = this.backend_?.kind ?? "none";
    if (kind !== this.backendKind_) {
      this.backendKind_ = kind;
      this.emit(RVF_BACKEND_EVENT, {
        kind,
        fallback: this.backend_?.fallback ?? null,
        capabilities: this.backend_?.capabilities ?? null,
      });
    }
    return this.backend_;
  }

  /** Replace the plot canvas node — the only way back from a pinned
   * WebGL context mode. Rebinds subclass listeners via
   * {@link onCanvasesReady} and re-applies size/status. */
  private replacePlotCanvas(): void {
    const old = this.plotCanvas_;
    if (old === undefined) return;
    const c = document.createElement("canvas");
    c.className = "plot";
    c.setAttribute("part", "plot-canvas");
    c.setAttribute("role", "img");
    old.replaceWith(c);
    this.plotCanvas_ = c;
    this.glAttempted_ = false;
    this.syncCanvasSize();
    this.syncStatus();
    c.addEventListener("webglcontextlost", (e) => {
      e.preventDefault();
      this.onContextLost();
    });
    this.onCanvasesReady();
  }

  /** GL context loss → swap to Canvas2D with a recorded reason and keep
   * drawing (precision/selection/gap positions survive — ops are
   * backend-neutral). A restored context does not auto-return to GL;
   * the swap is sticky for the element's lifetime (documented). */
  private onContextLost(): void {
    this.glAttempted_ = true; // this canvas is pinned to GL mode
    this.replacePlotCanvas();
    this.backend_ = createVizBackend(
      {
        plot: this.plotCanvas_ as HTMLCanvasElement,
        overlay: this.overlayCanvas_ ?? null,
      },
      "canvas2d",
      { width: this.cssWidth, height: this.cssHeight, dpr: this.dpr() },
    );
    if (this.backend_ !== null) {
      this.backendKind_ = "canvas2d";
      this.canvasSwapped_ = false;
      this.emit(RVF_BACKEND_EVENT, {
        kind: "canvas2d",
        fallback: { requested: "webgl2", reason: "webglcontextlost" },
        capabilities: this.backend_.capabilities,
      });
      this.drawSoon();
    } else {
      this.backendKind_ = "none";
      this.setStatus("canvas unavailable", "unsupported");
    }
  }

  // ── status / aria ─────────────────────────────────────────────────

  /** Non-color state channel: DOM status text + `unsupported` attr +
   * canvas aria-label suffix. `reason` empty clears the status. */
  protected setStatus(reason: string, kind?: "stale" | "unsupported"): void {
    if (reason === this.statusText && (kind ?? "") === this.statusReason) {
      return;
    }
    this.statusText = reason;
    this.statusReason = kind ?? "";
    this.setBoolAttr("unsupported", kind === "unsupported");
    this.syncStatus();
  }

  private syncStatus(): void {
    if (this.status_ !== undefined) {
      this.status_.textContent = this.statusText;
    }
    const plot = this.plotCanvas_;
    if (plot !== undefined) {
      const label = this.getAttribute("label") ?? this.localName;
      plot.setAttribute(
        "aria-label",
        this.statusText === "" ? label : `${label} — ${this.statusText}`,
      );
    }
  }

  protected emit(type: string, detail: unknown): void {
    this.dispatchEvent(
      new CustomEvent(type, { detail, bubbles: true, composed: true }),
    );
  }
}

function composedParent(el: Element): Element | null {
  const parent = el.parentElement;
  if (parent !== null) return parent;
  const root = el.getRootNode();
  return root instanceof ShadowRoot ? root.host : null;
}
