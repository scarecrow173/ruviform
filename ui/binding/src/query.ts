/**
 * Issue #261 — `QueryBinding`: bounded viewport/data queries for one
 * logical view.
 *
 * Wraps the `query` / `cancelQuery` half of the consumer's
 * {@link BindingPort} (Issue #257 §6.1):
 *
 * - every request carries `queryId` + the view's `viewId` + the current
 *   view `generation` — a `ViewLink`-managed binding adopts the link's
 *   live epoch both at `attach` and at registration on an attached
 *   link — plus the caller's `sourceRevision` — a reply that
 *   fails either identity check is discarded as `stale`, never applied;
 * - in-flight queries are bounded (`maxInflight`); overflow cancels the
 *   oldest — a viewport query is by nature superseded by the next one;
 * - transfer is bounded (`maxBytes`, default the `rvf://data` 16 MiB
 *   range-pull limit); an over-budget reply is `over-budget`, not data;
 * - `cancel` rides both the `AbortSignal` and the port's explicit
 *   `cancelQuery` channel (`rvf.ipc.v1`-style), so either mechanism
 *   terminates the request;
 * - `detach` bumps the generation and cancels everything in flight —
 *   replies for a dead view epoch resolve `stale`/`cancelled` and can
 *   never land in a rebound view.
 *
 * Outcomes are returned, not thrown — `stale` and `cancelled` are
 * normal control flow, not exceptions.
 */

import type {
  BindingPort,
  QueryId,
  QueryOutcome,
  QueryReply,
  ViewportQuery,
} from "./contract.ts";
import { DEFAULT_QUERY_MAX_BYTES } from "./contract.ts";

export const DEFAULT_MAX_INFLIGHT = 4;

export interface ViewportQueryInput {
  readonly sourceId: string;
  readonly sourceRevision: number | string;
  readonly start: number;
  readonly end: number;
  readonly buckets?: number;
  readonly channels?: readonly number[];
  readonly maxBytes?: number;
  readonly encoding?: readonly string[];
}

export interface QueryBindingOptions {
  readonly viewId: string;
  readonly port: Pick<BindingPort, "query" | "cancelQuery">;
  /** Bound on simultaneous in-flight queries (default 4). */
  readonly maxInflight?: number;
  /** Default `maxBytes` for requests that omit it (default 16 MiB). */
  readonly defaultMaxBytes?: number;
  /**
   * Optional resolve-time probe returning the source's *current*
   * revision — mirrors the element-side double check: a publish
   * between issue and resolve makes the answer stale even when the
   * reply itself echoes the requested revision.
   */
  readonly revisionProbe?: (
    sourceId: string,
  ) => number | string | undefined;
}

interface Inflight {
  readonly controller: AbortController;
  readonly generation: number;
  readonly request: ViewportQuery;
}

export interface QueryDiagnostics {
  readonly issued: number;
  readonly resolved: number;
  readonly stale: number;
  readonly cancelled: number;
  readonly overBudget: number;
  readonly errors: number;
  readonly evicted: number;
  readonly inflight: number;
  readonly generation: number;
}

function isAbort(err: unknown): boolean {
  return (
    err instanceof Error &&
    (err.name === "AbortError" || err.message === "aborted")
  );
}

export class QueryBinding {
  readonly viewId: string;
  private readonly port: Pick<BindingPort, "query" | "cancelQuery">;
  private readonly maxInflight: number;
  private readonly defaultMaxBytes: number;
  private readonly probe: QueryBindingOptions["revisionProbe"];

  private readonly inflight = new Map<QueryId, Inflight>();
  private readonly order: QueryId[] = [];
  private generation_ = 0;
  private seq = 0;
  private issued_ = 0;
  private resolved_ = 0;
  private stale_ = 0;
  private cancelled_ = 0;
  private overBudget_ = 0;
  private errors_ = 0;
  private evicted_ = 0;

  constructor(options: QueryBindingOptions) {
    this.viewId = options.viewId;
    this.port = options.port;
    this.maxInflight =
      options.maxInflight !== undefined && options.maxInflight > 0
        ? Math.floor(options.maxInflight)
        : DEFAULT_MAX_INFLIGHT;
    this.defaultMaxBytes = options.defaultMaxBytes ?? DEFAULT_QUERY_MAX_BYTES;
    this.probe = options.revisionProbe;
  }

  get generation(): number {
    return this.generation_;
  }

  get diagnostics(): QueryDiagnostics {
    return {
      issued: this.issued_,
      resolved: this.resolved_,
      stale: this.stale_,
      cancelled: this.cancelled_,
      overBudget: this.overBudget_,
      errors: this.errors_,
      evicted: this.evicted_,
      inflight: this.inflight.size,
      generation: this.generation_,
    };
  }

  /**
   * Issue a bounded query. The returned promise always resolves to a
   * {@link QueryOutcome} — `stale` (generation/revision mismatch),
   * `cancelled` (abort/cancel/detach), `over-budget`, `unsupported`
   * (port has no `query`), or `error` (port failure).
   */
  async request<T = unknown>(
    input: ViewportQueryInput,
  ): Promise<QueryOutcome<T>> {
    if (this.port.query === undefined) {
      return {
        ok: false,
        error: { kind: "unsupported", detail: "port exposes no query" },
      };
    }
    while (this.inflight.size >= this.maxInflight) {
      const oldest = this.order[0];
      if (oldest === undefined) break;
      this.cancelOne(oldest);
      this.evicted_ += 1;
    }
    const generation = this.generation_;
    this.seq += 1;
    const queryId = `q-${this.viewId}-${this.seq}`;
    const request: ViewportQuery = {
      queryId,
      viewId: this.viewId,
      generation,
      sourceId: input.sourceId,
      sourceRevision: input.sourceRevision,
      start: input.start,
      end: input.end,
      maxBytes: input.maxBytes ?? this.defaultMaxBytes,
      ...(input.buckets !== undefined ? { buckets: input.buckets } : {}),
      ...(input.channels !== undefined ? { channels: input.channels } : {}),
      ...(input.encoding !== undefined ? { encoding: input.encoding } : {}),
    };
    const controller = new AbortController();
    this.inflight.set(queryId, { controller, generation, request });
    this.order.push(queryId);
    this.issued_ += 1;

    let reply: QueryReply<T>;
    try {
      reply = (await this.port.query(request, controller.signal)) as QueryReply<T>;
    } catch (err: unknown) {
      this.finish(queryId);
      if (isAbort(err) || controller.signal.aborted) {
        this.cancelled_ += 1;
        return { ok: false, error: { kind: "cancelled", cause: err } };
      }
      this.errors_ += 1;
      return {
        ok: false,
        error: { kind: "error", detail: String(err), cause: err },
      };
    }
    const rec = this.inflight.get(queryId);
    if (rec === undefined) {
      // Cancelled while the port was still resolving — nothing lands.
      this.cancelled_ += 1;
      return { ok: false, error: { kind: "cancelled" } };
    }
    this.finish(queryId);
    if (
      reply.generation !== generation ||
      generation !== this.generation_ ||
      reply.sourceRevision !== request.sourceRevision ||
      (this.probe !== undefined &&
        this.probe(request.sourceId) !== undefined &&
        this.probe(request.sourceId) !== request.sourceRevision)
    ) {
      this.stale_ += 1;
      return { ok: false, error: { kind: "stale" } };
    }
    if (
      reply.bytes !== undefined &&
      request.maxBytes !== undefined &&
      reply.bytes > request.maxBytes
    ) {
      this.overBudget_ += 1;
      return { ok: false, error: { kind: "over-budget" } };
    }
    this.resolved_ += 1;
    return { ok: true, reply };
  }

  /** Cancel one in-flight query — `AbortSignal` plus the port's
   *  explicit cancel channel when it has one. */
  cancel(queryId: QueryId): void {
    this.cancelOne(queryId);
  }

  /** Cancel everything in flight (detach / close path). */
  cancelAll(): void {
    for (const id of [...this.order]) this.cancelOne(id);
  }

  /** Bump the view epoch — or adopt the epoch the {@link ViewLink}
   *  supplies — so in-flight becomes stale and new issues mint under
   *  the view generation. */
  attach(epoch?: number): void {
    this.generation_ = epoch ?? this.generation_ + 1;
    this.cancelAll();
  }

  /** Detach/close: same epoch bump plus cancellation — replies for the
   *  dead generation resolve `cancelled`/`stale`, never applied. */
  detach(): void {
    this.attach();
  }

  private cancelOne(queryId: QueryId): void {
    const rec = this.inflight.get(queryId);
    if (rec === undefined) return;
    this.inflight.delete(queryId);
    const at = this.order.indexOf(queryId);
    if (at >= 0) this.order.splice(at, 1);
    rec.controller.abort();
    this.port.cancelQuery?.(queryId);
  }

  private finish(queryId: QueryId): void {
    this.inflight.delete(queryId);
    const at = this.order.indexOf(queryId);
    if (at >= 0) this.order.splice(at, 1);
  }
}
