/**
 * Shared test fixtures (Issue #261): manual timers plus two mock
 * consumers — a DAW-style control gateway and an analysis-style
 * query/subscription backend — that both implement the SAME
 * {@link BindingPort} contract (acceptance gate 1).
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
} from "../src/index.ts";

// ── deterministic timers ────────────────────────────────────────────

export class ManualTimers {
  now = 0;
  private seq = 0;
  private readonly timers = new Map<number, { fn: () => void; at: number }>();

  readonly schedule = (fn: () => void, ms: number): (() => void) => {
    const id = ++this.seq;
    this.timers.set(id, { fn, at: this.now + ms });
    return () => {
      this.timers.delete(id);
    };
  };

  /** Advance the clock; due timers fire in schedule order. */
  advance(ms: number): void {
    this.now += ms;
    for (;;) {
      let dueId: number | undefined;
      for (const [id, t] of this.timers) {
        if (t.at <= this.now && (dueId === undefined || id < dueId)) {
          dueId = id;
        }
      }
      if (dueId === undefined) return;
      const t = this.timers.get(dueId);
      this.timers.delete(dueId);
      t?.fn();
    }
  }

  get pending(): number {
    return this.timers.size;
  }
}

/** Synchronous flush — every push dispatches immediately. */
export const flushSync = (fn: () => void): void => fn();

/** Deferred flush — collects flush requests for manual draining. */
export function deferredFlush(): {
  readonly flushWith: (fn: () => void) => void;
  readonly pending: () => number;
  readonly runAll: () => void;
} {
  const fns: Array<() => void> = [];
  return {
    flushWith: (fn) => fns.push(fn),
    pending: () => fns.length,
    runAll: () => {
      for (const fn of fns.splice(0)) fn();
    },
  };
}

// ── recording helpers ───────────────────────────────────────────────

export interface ReceivedCommand {
  readonly intent: CommandIntent;
  readonly ctx: DispatchContext;
}

// ── Mock Consumer A — DAW-style control gateway ─────────────────────

interface DawTarget {
  plain: number;
  revision: number;
}

/**
 * DAW-side gateway: owns the parameter store, validates targets,
 * applies commits (bumping the domain revision), publishes
 * projections to subscribed views, and reports receipts through the
 * test-wired `deliver` outbox. Nothing here is automatic — the test
 * scripts exactly when receipts and projections move, which is what
 * delayed/reordered/duplicate scenarios need.
 */
export class MockDawGateway implements BindingPort {
  readonly received: ReceivedCommand[] = [];
  readonly subscribeCalls: Array<{
    readonly target: string;
    readonly ctx: SubscriptionContext;
  }> = [];
  unsubscribed = 0;
  /** Wired by the test to `binding.ack`. */
  deliver: (receipt: CommandReceipt) => void = () => {};

  private readonly store = new Map<string, DawTarget>();
  private readonly listeners = new Map<
    string,
    Set<ProjectionListener<{ plain: number }>>
  >();
  private readonly revisions = new Map<string, number>();

  /** Declare a controllable target with an initial value/revision. */
  addTarget(target: string, plain: number): void {
    this.store.set(target, { plain, revision: 0 });
    this.revisions.set(target, 0);
  }

  valueOf(target: string): number | undefined {
    return this.store.get(target)?.plain;
  }

  revisionOf(target: string): number {
    return this.revisions.get(target) ?? 0;
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
  accept(commandId: CommandId): void {
    this.deliver({ commandId, status: "accepted" });
  }

  /** Gateway-side: mark the command applied — applies the commit value
   *  to the store and publishes the new revision to subscribers. */
  apply(received: ReceivedCommand): void {
    const { intent, ctx } = received;
    if (intent.kind === "commit") {
      const t = this.store.get(intent.target);
      if (t !== undefined) {
        t.plain = intent.value.plain;
        t.revision += 1;
        this.revisions.set(intent.target, t.revision);
      }
    }
    this.deliver({ commandId: ctx.commandId, status: "applied" });
    this.publish(intent.target);
  }

  /** Gateway-side: reject/unsupported. */
  reject(commandId: CommandId, detail = "rejected by gateway"): void {
    this.deliver({ commandId, status: "rejected", detail });
  }

  unsupported(commandId: CommandId): void {
    this.deliver({ commandId, status: "unsupported" });
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

  lastCommandId(): CommandId | undefined {
    return this.received.at(-1)?.ctx.commandId;
  }

  commandIds(): CommandId[] {
    return this.received.map((r) => r.ctx.commandId);
  }
}

// ── Mock Consumer B — offline analysis query/subscription backend ───

interface PendingQuery {
  readonly request: ViewportQuery;
  readonly resolve: (r: QueryReply<unknown>) => void;
  readonly reject: (e: unknown) => void;
}

/**
 * Analysis-side backend: bounded queries with scriptable outcomes plus
 * the same subscribe/dispatch halves of the port. Every query parks
 * until the test resolves/rejects it — the deterministic way to make
 * delayed, reordered, and cancelled replies.
 */
export class MockAnalysisBackend implements BindingPort {
  readonly received: ReceivedCommand[] = [];
  readonly queries: ViewportQuery[] = [];
  readonly cancelledQueries: QueryId[] = [];
  readonly subscribeCalls: SubscriptionContext[] = [];
  /** Revision the source currently reports — bump to make in-flight
   *  replies stale. */
  currentRevision: number | string = 0;

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

  query<T>(request: ViewportQuery, signal?: AbortSignal): Promise<QueryReply<T>> {
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

  /** Test-side: resolve a parked query. */
  resolveQuery(queryId: QueryId, reply: Partial<QueryReply<unknown>>): void {
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

  /** Test-side: reject a parked query. */
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
