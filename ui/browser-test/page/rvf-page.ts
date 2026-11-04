/**
 * `installRvfPage` — the in-browser half of the harness (Issue #310).
 *
 * Runs inside the served page: records every `rvf-*` CustomEvent at
 * the document level (the components emit `bubbles`+`composed`, so a
 * document listener sees them after they cross the shadow boundary),
 * wraps the fixture gateway/backend so commands, receipts,
 * subscriptions, and bounded queries are inspectable, and exposes the
 * whole thing as `window.__rvf` (see `src/protocol.ts` for the shared
 * contract — imported here as types only, so the emitted module has
 * no runtime dependency on the Node side).
 *
 * Determinism: nothing here uses a timer. Receipts move only when
 * `drain(mode)` scripts them, and viz frames advance only through
 * `drawAll()` — a test never waits on rAF or setTimeout for state
 * that the harness itself can reach.
 *
 * Remount safety: document listeners are bound exactly once into a
 * swappable sink — a lifecycle fixture that calls `installRvfPage`
 * again on remount swaps the sink instead of stacking recorders.
 */

import type {
  DrainMode,
  IssueQueryInput,
  RecordedRvfEvent,
  RvfPageApi,
  RvfRecordedReceipt,
  WaveformSourceLike,
} from "../src/protocol.ts";

/** The `rvf-*` event types recorded at the document level. */
export const RECORDED_EVENT_TYPES = [
  "rvf-gesture",
  "rvf-transport",
  "rvf-viewport",
  "rvf-select",
  "rvf-backend",
] as const;

/** Structural view of the demo control gateway (showcase/daw). */
interface ControlGatewayLike {
  received: Array<{ intent: { controlId?: string }; ctx: unknown }>;
  subscribeCalls?: unknown[];
  unsubscribed?: number;
  deliver: (rec: never, receipt: never) => void;
  valueOf?(target: string): unknown;
  revisionOf?(target: string): number;
}

/** The mount handle's receipt methods take one received record. */
interface ReceiptScripter {
  apply(rec: never): void;
  accept(rec: never): void;
  reject(rec: never, detail?: string): void;
  unsupported(rec: never): void;
}

/** Structural view of the demo analysis backend (analysis page). */
interface AnalysisBackendLike {
  received: unknown[];
  queries: unknown[];
  cancelledQueries: string[];
  pendingQueries: number;
  currentRevision: number | string;
  resolveQuery(id: string, reply: Record<string, unknown>): void;
  rejectQuery(id: string, err?: unknown): void;
}

/** Structural view of the page's QueryBinding (analysis page). */
interface QueryBindingLike {
  request(input: IssueQueryInput): Promise<unknown>;
  readonly diagnostics: unknown;
}

/** Viz element surface used by `drawAll`/`stats`. */
export interface VizLike extends HTMLElement {
  autopump?: boolean;
  pumpNow?(): void;
  drawNow(): void;
  setSize(width: number, height: number): void;
  stats?: { framesDrawn: number; lastOpCount: number; lastDrawMs: number };
  backendKind?: string;
}

export interface RvfPageOptions {
  /** Control-gateway fake (showcase/daw). Omit for backend-only pages. */
  readonly gateway?: ControlGatewayLike;
  /** Analysis-backend fake (analysis page). */
  readonly backend?: AnalysisBackendLike;
  /** The view's QueryBinding (analysis page). */
  readonly query?: QueryBindingLike;
  /** Receipt scripter — usually the mount handle itself. */
  readonly receipts?: ReceiptScripter;
  /** Waveform sources registered for query-log + replace scripting. */
  readonly waveformSources?: Readonly<Record<string, WaveformSourceLike>>;
  /** Viz elements registered for `drawAll`/`stats` (key → element). */
  readonly viz?: Readonly<Record<string, VizLike>>;
  /** Lifecycle hook — the page's own mount/unmount (default no-op). */
  readonly mount?: () => void;
  readonly unmount?: () => void;
}

function sanitize(detail: unknown): unknown {
  try {
    return JSON.parse(JSON.stringify(detail ?? null)) as unknown;
  } catch {
    return { unserializable: Object.prototype.toString.call(detail) };
  }
}

function controlIdOf(detail: unknown): string | undefined {
  if (detail !== null && typeof detail === "object") {
    const d = detail as { meta?: { controlId?: string }; controlId?: string };
    return d.meta?.controlId ?? d.controlId;
  }
  return undefined;
}

/** Live sink the bound-once listeners write into. */
interface EventSink {
  events: RecordedRvfEvent[];
  seq: number;
}

let sink: EventSink | undefined;
let listenersBound = false;

function bindListeners(): void {
  if (listenersBound) return;
  listenersBound = true;
  for (const type of RECORDED_EVENT_TYPES) {
    document.addEventListener(type, (e) => {
      const s = sink;
      if (s === undefined) return;
      const detail = (e as CustomEvent).detail;
      const cid = controlIdOf(detail);
      s.events.push({
        seq: ++s.seq,
        type,
        detail: sanitize(detail),
        bubbles: e.bubbles,
        composed: e.composed,
        ...(cid !== undefined ? { controlId: cid } : {}),
      });
    });
  }
}

/**
 * Install `window.__rvf` for a mounted fixture page. Call once, after
 * the fixture's own mount wiring (the gateway `deliver` route is
 * wrapped, not replaced, so receipt routing to bindings is preserved
 * while every delivered receipt is also recorded). Re-calling on a
 * remount re-points the API at the new fixture handles.
 */
export function installRvfPage(opts: RvfPageOptions): RvfPageApi {
  sink = { events: [], seq: 0 };
  bindListeners();
  const receipts: RvfRecordedReceipt[] = [];
  const queries: unknown[] = [];
  const queryOutcomes: unknown[] = [];

  const gw = opts.gateway;
  if (gw !== undefined) {
    // Wrap — never replace — the mount-installed deliver route so the
    // receipts log sees every receipt while binding ack keeps working.
    const inner = gw.deliver.bind(gw);
    gw.deliver = ((rec: { intent: { controlId?: string }; ctx: { commandId: string } }, receipt: { status: string; detail?: string; commandId: string }) => {
      receipts.push({
        commandId: receipt.commandId,
        controlId: rec.intent.controlId ?? "",
        status: receipt.status,
        ...(receipt.detail !== undefined ? { detail: receipt.detail } : {}),
      });
      inner(rec as never, receipt as never);
    }) as ControlGatewayLike["deliver"];
  }

  for (const src of Object.values(opts.waveformSources ?? {})) {
    const orig = src.query.bind(src);
    src.query = (q: never, signal?: AbortSignal) => {
      queries.push(sanitize(q));
      return orig(q, signal);
    };
  }

  const api: RvfPageApi = {
    get ready() {
      return true;
    },
    events: (type) => {
      const s = sink;
      if (s === undefined) return [];
      return s.events.filter(
        (e) => type === undefined || e.type === type,
      );
    },
    clear: () => {
      const s = sink;
      if (s !== undefined) {
        s.events.length = 0;
        s.seq = 0;
      }
      receipts.length = 0;
      queries.length = 0;
      queryOutcomes.length = 0;
    },
    commands: () =>
      sanitize(gw?.received ?? opts.backend?.received ?? []) as unknown[],
    receipts: () => receipts.slice(),
    drain: (mode: DrainMode) => {
      const scripter = opts.receipts;
      const pending = gw?.received ?? [];
      if (scripter === undefined || gw === undefined) return 0;
      const drained = pending.splice(0, pending.length);
      for (const rec of drained) {
        scripter[mode](rec as never);
      }
      return drained.length;
    },
    subscribeCalls: () => sanitize(gw?.subscribeCalls ?? []) as unknown[],
    unsubscribed: () => gw?.unsubscribed ?? 0,
    valueOf: (target) => sanitize(gw?.valueOf?.(target)),
    revisionOf: (target) => gw?.revisionOf?.(target) ?? 0,
    queries: () => queries.slice(),
    drawAll: () => {
      for (const el of Object.values(opts.viz ?? {})) {
        el.pumpNow?.();
        el.drawNow();
      }
    },
    stats: () => {
      const out: Record<string, unknown> = {};
      for (const [key, el] of Object.entries(opts.viz ?? {})) {
        out[key] = {
          framesDrawn: el.stats?.framesDrawn ?? 0,
          lastOpCount: el.stats?.lastOpCount ?? 0,
          backendKind: el.backendKind ?? "none",
          statusText:
            el.shadowRoot?.querySelector(".status")?.textContent ?? "",
        };
      }
      return out;
    },
    setVizSize: (key, width, height) => {
      opts.viz?.[key]?.setSize(width, height);
    },
    backendQueries: () =>
      sanitize(opts.backend?.queries ?? []) as unknown[],
    issueQuery: (input) => {
      const qb = opts.query;
      if (qb === undefined) {
        queryOutcomes.push({
          ok: false,
          error: { kind: "unsupported", detail: "no query binding" },
        });
        return;
      }
      void qb.request(input).then((o) => queryOutcomes.push(sanitize(o)));
    },
    queryOutcomes: () => queryOutcomes.slice(),
    queryDiagnostics: () =>
      sanitize(opts.query?.diagnostics ?? {}) as Record<string, number>,
    resolveQuery: (id, reply = {}) => opts.backend?.resolveQuery(id, reply),
    rejectQuery: (id) => opts.backend?.rejectQuery(id),
    pendingQueries: () => opts.backend?.pendingQueries ?? 0,
    cancelledQueries: () => opts.backend?.cancelledQueries.slice() ?? [],
    setBackendRevision: (rev) => {
      if (opts.backend !== undefined) opts.backend.currentRevision = rev;
    },
    replaceWaveform: (key, samples) => {
      opts.waveformSources?.[key]?.replace(samples);
    },
    waveformMeta: (key) =>
      sanitize(opts.waveformSources?.[key]?.meta()) as
        | Record<string, unknown>
        | undefined,
    prop: (sel, name) =>
      sanitize(
        (document.querySelector(sel) as Record<string, unknown> | null)?.[
          name
        ],
      ),
    setProp: (sel, name, value) => {
      const el = document.querySelector(sel) as Record<
        string,
        unknown
      > | null;
      if (el !== null) el[name] = value;
    },
    mount: () => opts.mount?.(),
    unmount: () => opts.unmount?.(),
  };
  window.__rvf = api;
  return api;
}
