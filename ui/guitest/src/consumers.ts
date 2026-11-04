/**
 * Issue #311 — consumer mock fixtures (the "両 Consumer" lane).
 *
 * Two scriptable `BindingPort` implementations covering both consumer
 * shapes named by the Issue:
 *
 * - {@link ScriptedDawBackend} — control-shaped (Synvelis-side):
 *   `dispatch`/`subscribe` centered, receipts driven explicitly through
 *   {@link ScriptedDawBackend.deliver}.
 * - {@link ScriptedAnalysisBackend} — query-shaped (AudioLens-side):
 *   `query`/`cancelQuery` centered; every query parks until the driver
 *   resolves/rejects it, which is what delayed/reordered/cancelled
 *   replies need.
 *
 * They carry contract shapes only — no product vocabulary (no Project/
 * Grant/Dataset domain truth). Domain ownership stays in real consumer
 * gateways; these exist so conformance/replay/fault-matrix lanes run
 * identically on both shapes (#294 evidence).
 */

import type {
  BindingPort,
  CommandId,
  CommandIntent,
  CommandReceipt,
  DispatchContext,
  ProjectionListener,
  QueryId,
  QueryReply,
  StateProjection,
  SubscriptionContext,
  Unsubscribe,
  ViewportQuery,
} from "../../binding/src/contract.ts";

export interface ReceivedCommand {
  readonly intent: CommandIntent;
  readonly ctx: DispatchContext;
}

interface Store {
  value: { plain: number; normalized: number };
  revision: number;
}

/** Control-shaped consumer: param store + subscription publish +
 *  scripted receipt channel. */
export class ScriptedDawBackend implements BindingPort {
  readonly received: ReceivedCommand[] = [];
  readonly subscribeCalls: Array<{
    readonly target: string;
    readonly ctx: SubscriptionContext;
  }> = [];
  unsubscribed = 0;
  /** Wired by the driver to the receiving binding's `ack`. */
  deliver: (receipt: CommandReceipt) => void = () => {};

  private readonly store = new Map<string, Store>();
  private readonly listeners = new Map<
    string,
    Set<ProjectionListener<{ plain: number; normalized: number }>>
  >();

  addTarget(target: string, initial: number): void {
    this.store.set(target, {
      value: { plain: initial, normalized: initial },
      revision: 0,
    });
  }

  valueOf(target: string): number | undefined {
    return this.store.get(target)?.value.plain;
  }

  revisionOf(target: string): number {
    return this.store.get(target)?.revision ?? 0;
  }

  dispatch(intent: CommandIntent, ctx: DispatchContext): void {
    this.received.push({ intent, ctx });
  }

  subscribe<P>(
    target: string,
    listener: ProjectionListener<P>,
    ctx: SubscriptionContext,
  ): Unsubscribe {
    this.subscribeCalls.push({ target, ctx });
    let set = this.listeners.get(target);
    if (set === undefined) {
      set = new Set();
      this.listeners.set(target, set);
    }
    const l = listener as ProjectionListener<{
      plain: number;
      normalized: number;
    }>;
    set.add(l);
    // Contract: push the current projection immediately on subscribe.
    const cur = this.store.get(target);
    if (cur !== undefined) {
      l({ revision: cur.revision, value: { ...cur.value } });
    }
    return () => {
      this.unsubscribed += 1;
      set.delete(l);
    };
  }

  accept(commandId: CommandId): void {
    this.deliver({ commandId, status: "accepted" });
  }

  queue(commandId: CommandId): void {
    this.deliver({ commandId, status: "queued" });
  }

  /** Mark `received` applied — applies commit values to the store and
   *  publishes the bumped revision. */
  apply(received: ReceivedCommand): void {
    const { intent, ctx } = received;
    if (intent.kind === "commit") {
      const t = this.store.get(intent.target);
      if (t !== undefined) {
        t.value = { ...intent.value };
        t.revision += 1;
      }
    }
    this.deliver({ commandId: ctx.commandId, status: "applied" });
    this.publish(intent.target);
  }

  reject(commandId: CommandId, detail = "rejected by gateway"): void {
    this.deliver({ commandId, status: "rejected", detail });
  }

  unsupported(commandId: CommandId): void {
    this.deliver({ commandId, status: "unsupported" });
  }

  publish(target: string): void {
    const t = this.store.get(target);
    if (t === undefined) return;
    for (const l of this.listeners.get(target) ?? []) {
      l({ revision: t.revision, value: { ...t.value } });
    }
  }

  /** Push an arbitrary projection (stale/duplicate scenarios). */
  pushProjection(
    target: string,
    projection: StateProjection<{ plain: number; normalized: number }>,
  ): void {
    for (const l of this.listeners.get(target) ?? []) {
      l(projection);
    }
  }

  lastCommandId(): CommandId | undefined {
    return this.received.at(-1)?.ctx.commandId;
  }

  commandIds(): CommandId[] {
    return this.received.map((r) => r.ctx.commandId);
  }
}

interface PendingQuery {
  readonly request: ViewportQuery;
  readonly resolve: (r: QueryReply<unknown>) => void;
  readonly reject: (e: unknown) => void;
}

/** Query-shaped consumer: every query parks until the driver
 *  resolves/rejects it; `cancelQuery` + AbortSignal both terminate. */
export class ScriptedAnalysisBackend implements BindingPort {
  readonly received: ReceivedCommand[] = [];
  readonly queries: ViewportQuery[] = [];
  readonly cancelledQueries: QueryId[] = [];
  readonly subscribeCalls: SubscriptionContext[] = [];
  /** Revision the source currently reports — `undefined` means "no
   *  live revision to probe against" (the probe check is skipped),
   *  which is what sources without tracking need. Set via the
   *  `setSourceRevision` op to make in-flight replies stale. */
  currentRevision: number | string | undefined = undefined;

  private readonly pending = new Map<QueryId, PendingQuery>();
  private readonly listeners = new Set<ProjectionListener<unknown>>();

  dispatch(intent: CommandIntent, ctx: DispatchContext): void {
    this.received.push({ intent, ctx });
  }

  subscribe<P>(
    _target: string,
    listener: ProjectionListener<P>,
    ctx: SubscriptionContext,
  ): Unsubscribe {
    this.subscribeCalls.push(ctx);
    const l = listener as ProjectionListener<unknown>;
    this.listeners.add(l);
    return () => {
      this.listeners.delete(l);
    };
  }

  publish(projection: StateProjection<unknown>): void {
    for (const l of this.listeners) l(projection);
  }

  query<T>(
    request: ViewportQuery,
    signal?: AbortSignal,
  ): Promise<QueryReply<T>> {
    this.queries.push(request);
    return new Promise<QueryReply<T>>((resolve, reject) => {
      this.pending.set(request.queryId, {
        request,
        resolve: resolve as (r: QueryReply<unknown>) => void,
        reject,
      });
      signal?.addEventListener("abort", () => {
        this.pending.delete(request.queryId);
        reject(new DOMException("aborted", "AbortError"));
      });
    });
  }

  cancelQuery(queryId: QueryId): void {
    this.cancelledQueries.push(queryId);
    this.pending.get(queryId)?.reject(new Error("cancelled by host"));
    this.pending.delete(queryId);
  }

  /** Driver-side: resolve a parked query. */
  resolveQuery(
    queryId: QueryId,
    reply: Partial<QueryReply<unknown>>,
  ): void {
    const p = this.pending.get(queryId);
    if (p === undefined) return;
    this.pending.delete(queryId);
    p.resolve({
      generation: reply.generation ?? p.request.generation,
      sourceRevision: reply.sourceRevision ?? p.request.sourceRevision,
      payload: reply.payload ?? {},
      ...(reply.bytes !== undefined ? { bytes: reply.bytes } : {}),
    });
  }

  /** Driver-side: reject a parked query. */
  rejectQuery(queryId: QueryId, err: unknown = new Error("backend error")): void {
    const p = this.pending.get(queryId);
    if (p === undefined) return;
    this.pending.delete(queryId);
    p.reject(err);
  }

  get pendingQueries(): number {
    return this.pending.size;
  }
}
