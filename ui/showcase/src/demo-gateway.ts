/**
 * Ruviform-owned demo gateways (Issue #263 — cross-consumer fixtures).
 *
 * Two generic {@link BindingPort} implementations shared by the
 * showcase page and the headless consumer fixtures:
 *
 * - {@link DemoControlGateway} — a DAW-*shaped* control consumer: a
 *   plain-value parameter store, scripted receipts, projection
 *   publish. Characteristic fixture for "Consumer A" scenarios
 *   (fader/transport/automation binding); it is NOT a product type.
 * - {@link DemoAnalysisBackend} — an analysis-*shaped* query consumer:
 *   parked bounded queries the caller resolves/rejects/cancels, plus
 *   the same subscribe/dispatch halves. Characteristic fixture for
 *   "Consumer B" scenarios (waveform/spectrum/metadata).
 *
 * Everything is deterministic: receipts and replies move only when the
 * script says so, which is exactly what delayed/reordered/stale/cancel
 * scenarios need. Domain truth (validation, undo, jobs) stays inside
 * these gateway fakes — the same boundary a real consumer keeps.
 */

import type {
  BindingPort,
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
} from "../../binding/src/index.ts";

export interface DemoReceivedCommand {
  readonly intent: CommandIntent;
  readonly ctx: DispatchContext;
}

/**
 * Control-side gateway fake: owns a `{plain, revision}` target store,
 * applies commits (bumping the domain revision), publishes projections
 * to subscribed views, and reports receipts through the wired
 * `deliver` outbox.
 */
export class DemoControlGateway implements BindingPort {
  readonly received: DemoReceivedCommand[] = [];
  readonly subscribeCalls: Array<{
    readonly target: string;
    readonly ctx: SubscriptionContext;
  }> = [];
  unsubscribed = 0;
  /** Wired by the fixture once — receipts are delivered WITH their
   *  command record so the fixture can resolve the owning binding by
   *  `intent.controlId` (per-binding `cmd-N` ids collide by design, so
   *  a bare `commandId` cannot route). */
  deliver: (rec: DemoReceivedCommand, receipt: CommandReceipt) => void =
    () => {};

  private readonly store = new Map<
    string,
    { plain: number; revision: number }
  >();
  private readonly listeners = new Map<
    string,
    Set<ProjectionListener<{ plain: number }>>
  >();

  /** Declare a controllable target with an initial value. Duplicate
   *  declarations throw — a shared gateway silently resetting a live
   *  target would cross-wire two mounted views. */
  addTarget(target: string, plain: number): void {
    if (this.store.has(target)) {
      throw new Error(`DemoControlGateway target already declared: ${target}`);
    }
    this.store.set(target, { plain, revision: 0 });
  }

  valueOf(target: string): number | undefined {
    return this.store.get(target)?.plain;
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
    const l = listener as ProjectionListener<{ plain: number }>;
    set.add(l);
    // Contract: push the current projection immediately on subscribe.
    const cur = this.store.get(target);
    if (cur !== undefined) {
      l({ revision: cur.revision, value: { plain: cur.plain } });
    }
    return () => {
      this.unsubscribed += 1;
      set.delete(l);
    };
  }

  /** Gateway-side: accept the command (ack "accepted"). */
  accept(rec: DemoReceivedCommand): void {
    this.deliver(rec, { commandId: rec.ctx.commandId, status: "accepted" });
  }

  /** Gateway-side: apply the commit value to the store, then ack
   *  "applied" and publish the new revision to subscribers. Non-commit
   *  records are acked "accepted" — only a commit mutates truth. */
  apply(rec: DemoReceivedCommand): void {
    const { intent, ctx } = rec;
    if (intent.kind === "commit") {
      const t = this.store.get(intent.target);
      if (t !== undefined) {
        t.plain = intent.value.plain;
        t.revision += 1;
      }
      this.deliver(rec, { commandId: ctx.commandId, status: "applied" });
      this.publish(intent.target);
      return;
    }
    this.deliver(rec, { commandId: ctx.commandId, status: "accepted" });
  }

  /** Gateway-side: reject/unsupported receipts. */
  reject(rec: DemoReceivedCommand, detail = "rejected by gateway"): void {
    this.deliver(rec, {
      commandId: rec.ctx.commandId,
      status: "rejected",
      detail,
    });
  }

  unsupported(rec: DemoReceivedCommand): void {
    this.deliver(rec, {
      commandId: rec.ctx.commandId,
      status: "unsupported",
    });
  }

  /** Publish the current projection of a target to its subscribers. */
  publish(target: string): void {
    const t = this.store.get(target);
    if (t === undefined) return;
    for (const l of this.listeners.get(target) ?? []) {
      l({ revision: t.revision, value: { plain: t.plain } });
    }
  }

  /** Push an arbitrary projection (stale/duplicate scenarios). */
  pushProjection(
    target: string,
    projection: StateProjection<{ plain: number }>,
  ): void {
    for (const l of this.listeners.get(target) ?? []) {
      l(projection);
    }
  }

  lastReceived(): DemoReceivedCommand | undefined {
    return this.received.at(-1);
  }
}

interface DemoPendingQuery {
  readonly request: ViewportQuery;
  readonly resolve: (r: QueryReply<unknown>) => void;
  readonly reject: (e: unknown) => void;
  readonly onAbort?: { readonly signal: AbortSignal; readonly fn: () => void };
}

/**
 * Analysis-side backend fake: bounded queries with scriptable outcomes
 * plus the same subscribe/dispatch halves of the port. Every query
 * parks until the fixture resolves/rejects it — the deterministic way
 * to make delayed, reordered, stale, and cancelled replies.
 */
export class DemoAnalysisBackend implements BindingPort {
  readonly received: DemoReceivedCommand[] = [];
  readonly queries: ViewportQuery[] = [];
  readonly cancelledQueries: QueryId[] = [];
  readonly subscribeCalls: SubscriptionContext[] = [];
  /** Revision the source currently reports — bump it to make
   *  in-flight replies stale. */
  currentRevision: number | string = 0;

  private readonly pending = new Map<QueryId, DemoPendingQuery>();
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
      const entry: DemoPendingQuery = {
        request,
        resolve: resolve as (r: QueryReply<unknown>) => void,
        reject,
        ...(signal !== undefined
          ? {
              onAbort: {
                signal,
                fn: () => {
                  this.pending.delete(request.queryId);
                  reject(new DOMException("aborted", "AbortError"));
                },
              },
            }
          : {}),
      };
      this.pending.set(request.queryId, entry);
      entry.onAbort?.signal.addEventListener("abort", entry.onAbort.fn, {
        once: true,
      });
    });
  }

  private settle(queryId: QueryId): DemoPendingQuery | undefined {
    const p = this.pending.get(queryId);
    if (p === undefined) return undefined;
    this.pending.delete(queryId);
    p.onAbort?.signal.removeEventListener("abort", p.onAbort.fn);
    return p;
  }

  cancelQuery(queryId: QueryId): void {
    this.cancelledQueries.push(queryId);
    this.settle(queryId)?.reject(new Error("cancelled by host"));
  }

  /** Fixture-side: resolve a parked query. The reply reports
   *  {@link currentRevision} — the source state it was built against —
   *  unless overridden, so bumping the field mid-flight yields the
   *  `stale` outcome a real backend produces. */
  resolveQuery(queryId: QueryId, reply: Partial<QueryReply<unknown>>): void {
    const p = this.settle(queryId);
    if (p === undefined) return;
    p.resolve({
      generation: reply.generation ?? p.request.generation,
      sourceRevision: reply.sourceRevision ?? this.currentRevision,
      payload: reply.payload ?? {},
      ...(reply.bytes !== undefined ? { bytes: reply.bytes } : {}),
    });
  }

  /** Fixture-side: reject a parked query. */
  rejectQuery(
    queryId: QueryId,
    err: unknown = new Error("backend error"),
  ): void {
    this.settle(queryId)?.reject(err);
  }

  get pendingQueries(): number {
    return this.pending.size;
  }
}
