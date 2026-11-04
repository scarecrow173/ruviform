/**
 * `openHarness` — open a served fixture page in a deterministic
 * browser environment and hand back a typed session (Issue #310).
 *
 * Determinism contract:
 * - the page is served hermetically from the workspace via
 *   {@link installStaticServe} — no server process, no network;
 * - `page.clock` is installed and paused before navigation, so every
 *   in-page timer (wheel-idle commit, debounce, rAF) fires only when
 *   the test says so via {@link HarnessSession.advanceClock};
 * - readiness is a Node-side state poll, never a fixed sleep;
 * - viz frames advance only through {@link RvfPageApi.drawAll}.
 */

import type { Page } from "@playwright/test";
import { installStaticServe, RVF_ORIGIN, type ServeStats } from "./serve.ts";
import { waitFor, type WaitForOptions } from "./poll.ts";
import type {
  DrainMode,
  IssueQueryInput,
  RecordedRvfEvent,
  RvfPageApi,
  RvfRecordedReceipt,
} from "./protocol.ts";

/** Fixed epoch installed in every harness page — recorded in evidence. */
// The epoch every harness page sees as `Date.now()`. It must sit
// ahead of the machine's wall clock: `pauseAt` is forward-only, so
// the install→pause sequence installs at wall-now and fast-forwards
// to this epoch, landing pinned at exactly RVF_FIXED_NOW. A past
// epoch makes that fast-forward negative and throws.
export const RVF_FIXED_NOW = "2030-01-01T00:00:00.000Z";
/** Default seed recorded in evidence (demo data is seed-fixed). */
export const RVF_DEFAULT_SEED = "rvf-demo-v1";

export type HarnessPageName = "showcase" | "daw" | "analysis" | "lifecycle";

export interface OpenHarnessOptions {
  /** Install + pause `page.clock` (default `true`). */
  readonly clock?: boolean;
  /** Epoch installed when `clock` is on (default {@link RVF_FIXED_NOW}). */
  readonly now?: string;
  /** Readiness poll timeout (default 10 000 ms). */
  readonly readyTimeout?: number;
}

export class HarnessSession {
  /** The Playwright page (kept public — drivers use its locators). */
  readonly page: Page;
  /** Fixture page name under `/pages/`. */
  readonly pageName: HarnessPageName;
  /** Serve-layer stats (served/aborted request counts). */
  readonly serveStats: ServeStats;

  private constructor(
    page: Page,
    pageName: HarnessPageName,
    serveStats: ServeStats,
  ) {
    this.page = page;
    this.pageName = pageName;
    this.serveStats = serveStats;
  }

  /** Open `pageName` on `page` and wait for `window.__rvf.ready`. */
  static async open(
    page: Page,
    pageName: HarnessPageName,
    opts: OpenHarnessOptions = {},
  ): Promise<HarnessSession> {
    if (opts.clock !== false) {
      const now = opts.now ?? RVF_FIXED_NOW;
      // Install at wall-now, then fast-forward + pause at `now`. The
      // clock ticks in real time after install, so `pauseAt` must
      // target a later wall epoch — it lands pinned at exactly `now`,
      // and pending timers (wheel idle, rAF) fire only under
      // advanceClock. No page timers exist pre-navigation, so the
      // fast-forward itself fires nothing.
      await page.clock.install();
      await page.clock.pauseAt(now);
    }
    const serveStats = await installStaticServe(page);
    const session = new HarnessSession(page, pageName, serveStats);
    await page.goto(`${RVF_ORIGIN}/pages/${pageName}.html`);
    await waitFor(
      async () => {
        // `__rvf` is undefined until the page module installs it —
        // an api() call before then throws; that is "not ready", not
        // a test error.
        try {
          return (await session.api((api) => api.ready)) === true;
        } catch {
          return false;
        }
      },
      {
        timeout: opts.readyTimeout ?? 10_000,
        label: `harness page "${pageName}" ready`,
      },
    );
    return session;
  }

  /**
   * Call the page-side `__rvf` API. `fn` is serialized into
   * `page.evaluate` and invoked as `fn(api, ...args)` — closure
   * captures do NOT cross the boundary, so every value `fn` needs
   * must be passed through `args` (JSON-serializable).
   */
  api<T, A extends unknown[]>(
    fn: (api: RvfPageApi, ...args: A) => T,
    ...args: A
  ): Promise<T> {
    return this.page.evaluate(
      (packed: { src: string; args: unknown[] }) => {
        const api = (window as unknown as { __rvf?: RvfPageApi }).__rvf;
        if (api === undefined) {
          throw new Error("__rvf not installed on this page");
        }
        // The serialized callback is harness-owned test code, never
        // page input — new Function is the transport for spec-authored
        // page-side predicates, same contract as page.evaluate itself.
        const call = new Function(
          "api",
          "args",
          `"use strict"; return (${packed.src})(api, ...args);`,
        ) as (a: RvfPageApi, args: unknown[]) => T;
        return call(api, packed.args);
      },
      { src: fn.toString(), args },
    );
  }

  /** Advance the installed clock (fires due timers exactly once). */
  async advanceClock(ms: number | string): Promise<void> {
    await this.page.clock.runFor(ms);
  }

  // ── protocol pass-throughs ───────────────────────────────────────

  events(type?: string): Promise<RecordedRvfEvent[]> {
    return this.api((api, t) => api.events(t), type);
  }
  commands(): Promise<unknown[]> {
    return this.api((api) => api.commands());
  }
  receipts(): Promise<RvfRecordedReceipt[]> {
    return this.api((api) => api.receipts());
  }
  drain(mode: DrainMode): Promise<number> {
    return this.api((api, m) => api.drain(m), mode);
  }
  subscribeCalls(): Promise<unknown[]> {
    return this.api((api) => api.subscribeCalls());
  }
  unsubscribed(): Promise<number> {
    return this.api((api) => api.unsubscribed());
  }
  valueOf(target: string): Promise<unknown> {
    return this.api((api, t) => api.valueOf(t), target);
  }
  revisionOf(target: string): Promise<number> {
    return this.api((api, t) => api.revisionOf(t), target);
  }
  queries(): Promise<unknown[]> {
    return this.api((api) => api.queries());
  }
  drawAll(): Promise<void> {
    return this.api((api) => api.drawAll());
  }
  stats(): Promise<Record<string, unknown>> {
    return this.api((api) => api.stats());
  }
  setVizSize(key: string, width: number, height: number): Promise<void> {
    return this.api((api, k, w, h) => api.setVizSize(k, w, h), key, width, height);
  }
  backendQueries(): Promise<unknown[]> {
    return this.api((api) => api.backendQueries());
  }
  issueQuery(input: IssueQueryInput): Promise<void> {
    return this.api((api, i) => api.issueQuery(i), input);
  }
  queryOutcomes(): Promise<unknown[]> {
    return this.api((api) => api.queryOutcomes());
  }
  queryDiagnostics(): Promise<Record<string, number>> {
    return this.api((api) => api.queryDiagnostics());
  }
  resolveQuery(
    queryId: string,
    reply?: Record<string, unknown>,
  ): Promise<void> {
    return this.api((api, i, r) => api.resolveQuery(i, r), queryId, reply);
  }
  rejectQuery(queryId: string): Promise<void> {
    return this.api((api, i) => api.rejectQuery(i), queryId);
  }
  pendingQueries(): Promise<number> {
    return this.api((api) => api.pendingQueries());
  }
  cancelledQueries(): Promise<string[]> {
    return this.api((api) => api.cancelledQueries());
  }
  setBackendRevision(rev: number | string): Promise<void> {
    return this.api((api, r) => api.setBackendRevision(r), rev);
  }
  replaceWaveform(key: string, samples: number[]): Promise<void> {
    return this.api((api, k, s) => api.replaceWaveform(k, s), key, samples);
  }
  waveformMeta(key: string): Promise<Record<string, unknown> | undefined> {
    return this.api((api, k) => api.waveformMeta(k), key);
  }
  prop(sel: string, name: string): Promise<unknown> {
    return this.api((api, s, n) => api.prop(s, n), sel, name);
  }
  setProp(sel: string, name: string, value: unknown): Promise<void> {
    return this.api((api, s, n, v) => api.setProp(s, n, v), sel, name, value);
  }
  mount(): Promise<void> {
    return this.api((api) => api.mount());
  }
  unmount(): Promise<void> {
    return this.api((api) => api.unmount());
  }
  clear(): Promise<void> {
    return this.api((api) => api.clear());
  }

  /** Poll an `__rvf` probe until truthy (state-arrival wait). */
  waitForState<T>(
    probe: (api: RvfPageApi) => T,
    opts?: WaitForOptions,
  ): Promise<T>;
  waitForState<T, A extends unknown[]>(
    probe: (api: RvfPageApi, ...args: A) => T,
    args: A,
    opts?: WaitForOptions,
  ): Promise<T>;
  waitForState<T, A extends unknown[]>(
    probe: (api: RvfPageApi, ...args: A) => T,
    argsOrOpts?: A | WaitForOptions,
    opts: WaitForOptions = {},
  ): Promise<T> {
    if (Array.isArray(argsOrOpts)) {
      const args = argsOrOpts;
      return waitFor(() => this.api(probe, ...args), opts);
    }
    const unary = probe as unknown as (api: RvfPageApi) => T;
    return waitFor(() => this.api(unary), argsOrOpts ?? {});
  }
}

/** Open a harness session (see {@link HarnessSession.open}). */
export function openHarness(
  page: Page,
  pageName: HarnessPageName,
  opts: OpenHarnessOptions = {},
): Promise<HarnessSession> {
  return HarnessSession.open(page, pageName, opts);
}
