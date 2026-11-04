/**
 * `window.__rvf` — the page-side harness protocol (Issue #310, RF-GT-01).
 *
 * One interface describes everything a mounted verification page
 * exposes to the Node-side test. Node code reaches it via
 * `page.evaluate`; the page-side implementation lives in
 * `page/rvf-page.ts` (compiled to `dist/page/` and served to the
 * browser). The two sides share ONLY this type declaration — Node
 * sources never import `page/` (process boundary), and `page/`
 * imports this file with `import type` only, so the emitted browser
 * module has no runtime dependency on `src/`.
 */

/** One captured `rvf-*` CustomEvent, JSON-serializable. */
export interface RecordedRvfEvent {
  /** Monotonic capture order across all recorded event types. */
  readonly seq: number;
  /** DOM event type (`rvf-gesture`, `rvf-transport`, …). */
  readonly type: string;
  /** `event.detail`, structured-cloned into JSON-safe data. */
  readonly detail: unknown;
  /** The dispatch's `bubbles`/`composed` init flags. */
  readonly bubbles: boolean;
  readonly composed: boolean;
  /** `detail.meta.controlId ?? detail.controlId` when present. */
  readonly controlId?: string;
}

/** A gateway `received` record (`{intent, ctx}`), JSON-safe. */
export type RvfRecordedCommand = unknown;

/** A delivered receipt record (`{commandId, controlId, status, detail?}`). */
export interface RvfRecordedReceipt {
  readonly commandId: string;
  readonly controlId: string;
  readonly status: string;
  readonly detail?: string;
}

/** Receipt scripting mode for {@link RvfPageApi.drain}. */
export type DrainMode = "apply" | "accept" | "reject" | "unsupported";

/** Bounded-query request issued through the page's QueryBinding. */
export interface IssueQueryInput {
  readonly sourceId: string;
  readonly sourceRevision: number | string;
  readonly start: number;
  readonly end: number;
  readonly buckets?: number;
  readonly channels?: readonly number[];
  readonly maxBytes?: number;
}

/** Waveform source surface `replaceWaveform`/`waveformMeta` script. */
export interface WaveformSourceLike {
  meta(): unknown;
  replace(samples: ArrayLike<number>): void;
  query(q: never, signal?: AbortSignal): Promise<unknown>;
}

/**
 * The page-side API. Every method returns JSON-serializable data
 * (`page.evaluate` boundary). Surfaces that do not exist on a given
 * page return empty values — a page with no analysis backend answers
 * `backendQueries() === []` and `pendingQueries() === 0` rather than
 * throwing, so shared spec helpers need no page sniffing.
 */
export interface RvfPageApi {
  /** Set once the page module finished mounting. */
  readonly ready: boolean;

  // ── event log ────────────────────────────────────────────────────
  /** Captured `rvf-*` CustomEvents (optionally one type), in order. */
  events(type?: string): RecordedRvfEvent[];
  /** Drop every recorded log (events/commands/receipts/queries). */
  clear(): void;

  // ── gateway records ──────────────────────────────────────────────
  /** `gateway.received` verbatim (`{intent, ctx}` records). */
  commands(): RvfRecordedCommand[];
  /** Receipts delivered through `gateway.deliver`, in order. */
  receipts(): RvfRecordedReceipt[];
  /**
   * Drain `gateway.received` through the scripted receipt path:
   * `apply` applies commits to the store (ack `applied`, revision
   * bump, projection publish) and acks the rest `accepted`;
   * `accept`/`reject`/`unsupported` ack every record accordingly.
   * Returns the number of drained commands.
   */
  drain(mode: DrainMode): number;
  /** `gateway.subscribeCalls` log — the subscription dedup oracle. */
  subscribeCalls(): unknown[];
  /** `gateway.unsubscribed` counter — the leak oracle. */
  unsubscribed(): number;
  /** Current store value of a gateway target (`undefined` if none). */
  valueOf(target: string): unknown;
  /** Current store revision of a gateway target (0 if none). */
  revisionOf(target: string): number;

  // ── viz / bounded queries ────────────────────────────────────────
  /** Waveform source `query` invocations `{start,end,buckets,…}`. */
  queries(): unknown[];
  /** Deterministic viz tick: `pumpNow?.()` + `drawNow()` per viz el. */
  drawAll(): void;
  /** Per-viz stats: `{framesDrawn,lastOpCount,backendKind,statusText}`. */
  stats(): Record<string, unknown>;
  /**
   * `setSize(cssW, cssH)` on a registered viz element — the
   * resize/viewport-budget trigger (re-queries where the contract
   * requires, DPR × CSS px backing-store update).
   */
  setVizSize(key: string, width: number, height: number): void;

  // ── analysis backend (empty on pages without one) ────────────────
  /** `backend.queries` — the ViewportQuery log of a QueryBinding. */
  backendQueries(): unknown[];
  /**
   * Issue a bounded query through the page's QueryBinding — the
   * request lands in `backendQueries()` parked; the outcome lands in
   * `queryOutcomes()` once scripted via resolve/reject. Pages without
   * a QueryBinding record `{kind:"unsupported"}` as the outcome.
   */
  issueQuery(input: IssueQueryInput): void;
  /** Recorded `QueryOutcome`s of `issueQuery` calls, in order. */
  queryOutcomes(): unknown[];
  /** `query.diagnostics` counters (issued/resolved/stale/…). */
  queryDiagnostics(): Record<string, number>;
  /** Resolve a parked backend query (`Partial<QueryReply>`). */
  resolveQuery(queryId: string, reply?: Record<string, unknown>): void;
  /** Reject a parked backend query. */
  rejectQuery(queryId: string): void;
  /** `backend.pendingQueries` — queries parked awaiting resolution. */
  pendingQueries(): number;
  /** `backend.cancelledQueries` — query ids the host cancelled. */
  cancelledQueries(): string[];
  /** Script `backend.currentRevision` (stale-reply scripting). */
  setBackendRevision(rev: number | string): void;

  // ── waveform sources (long-form/stale scripting) ──────────────────
  /**
   * Replace a registered waveform source's samples — bumps the source
   * revision and notifies subscribers (the stale/long-form trigger).
   */
  replaceWaveform(key: string, samples: number[]): void;
  /** `source.meta()` of a registered waveform source. */
  waveformMeta(key: string): Record<string, unknown> | undefined;

  // ── typed DOM property helpers ───────────────────────────────────
  /** Read `document.querySelector(sel)[name]` (typed property). */
  prop(sel: string, name: string): unknown;
  /** Write `document.querySelector(sel)[name] = value`. */
  setProp(sel: string, name: string, value: unknown): void;

  // ── lifecycle (implemented on the lifecycle page) ────────────────
  /** Re-mount the page's fixture into the same host. */
  mount(): void;
  /** Unmount the page's fixture (element teardown runs). */
  unmount(): void;
}

declare global {
  interface Window {
    /** Installed by `installRvfPage` — undefined until the page is up. */
    __rvf?: RvfPageApi;
  }
}
