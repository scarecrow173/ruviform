/**
 * Issue #311 — `ContractModel`: the independent oracle for replay.
 *
 * This module encodes the *documented* contract rules of `ui/binding`
 * as a small observation model. It intentionally shares no code with
 * `ui/binding/src` (type imports only) — the point of replay is that
 * two independent readings of the contract must agree on every
 * scenario.
 *
 * Rules encoded (contract.ts / binding.ts / coalesce.ts / query.ts /
 * link.ts doc comments):
 *
 * - `preview` is the only coalescible kind — a buffered preview is
 *   replaced by a newer preview of the same gesture; a gesture's
 *   `commit`/`cancel` tombstones its buffered preview; all other kinds
 *   append strictly and are never dropped or reordered.
 * - Command ids mint `cmd-N` per binding in dispatch order;
 *   `expectReceipt` is `commit`|`action` only; non-preview commands
 *   are tracked (bounded by the 64-record eviction rule — settled
 *   records evict first, in-flight receipt-expecting ones stay).
 * - Receipt correlation: unknown commandId or dead generation → stale;
 *   rank `in-flight < accepted|queued < applied < terminal
 *   (rejected|unsupported)` — lower-or-equal rank is absorbed as
 *   out-of-order, never applied twice.
 * - `ackTimeoutMs` arms at dispatch; crossing it marks `timedOut` — an
 *   explicit failure flag, never a fabricated success. A later
 *   `applied`/terminal receipt still moves on.
 * - Projections land only on a revision *advance* (numeric compare;
 *   equal non-numeric identity is stale, different identity advances)
 *   and only while attached. A landed projection retires a terminal
 *   head; it cannot answer an in-flight one.
 * - `detach` cancels the in-flight gesture (minted under the *old*
 *   generation — the terminal message of the epoch), purges the
 *   queue, bumps the generation, and disarms the ack timers. Detached
 *   pushes are counted, never dispatched.
 * - `ViewLink.attach` bumps the view generation, sets context, and
 *   re-subscribes — a fresh projection is re-read rather than trusting
 *   the old window. Re-attach of the same window and detach of an
 *   already-detached link are no-ops. A binding created on an
 *   already-attached link is subscribed but does NOT adopt the view
 *   epoch (its generation stays its own counter until a detach cycle).
 * - `QueryBinding` mints `q-<viewId>-<n>` under the current view
 *   generation; replies whose generation, sourceRevision, or live
 *   revision probe disagree are `stale`; over `maxBytes` is
 *   `over-budget`; overflow beyond `maxInflight` cancels the oldest;
 *   `detach` bumps the generation and cancels everything in flight.
 */

import type {
  CommandIntent,
  CommandKind,
  CommandPhase,
  CommandReceipt,
  QueryErrorKind,
  QueryReply,
  ReceiptStatus,
} from "../../binding/src/contract.ts";
import type {
  CancelReason,
  GestureId,
  InteractionEvent,
  LogicalTarget,
} from "../../core/src/index.ts";

// ── shared trace surface (oracle expectations and driver observations
//    use the same shapes so equality is a plain deep-equal) ──────────

export interface DispatchRec {
  readonly binding: string;
  readonly commandId: string;
  readonly kind: CommandKind;
  readonly controlId: string;
  readonly target: string;
  readonly sequence: number;
  readonly gestureId?: string;
  readonly action?: string;
  readonly generation: number;
  readonly expectReceipt: boolean;
  readonly viewId?: string;
  readonly windowId?: string;
}

export interface PhaseRec {
  readonly binding: string;
  readonly phase: CommandPhase;
}

export interface SubscribeRec {
  readonly binding: string;
  readonly target: string;
  readonly viewId: string;
  readonly windowId?: string;
  readonly generation: number;
}

export interface UnsubscribeRec {
  readonly binding: string;
  readonly target: string;
}

export interface QueryIssueRec {
  readonly queryId: string;
  readonly viewId: string;
  readonly generation: number;
  readonly sourceId: string;
  readonly sourceRevision: number | string;
  readonly start: number;
  readonly end: number;
  readonly maxBytes?: number;
}

export interface BindingDiag {
  readonly intents: number;
  readonly dispatched: number;
  readonly coalesced: number;
  readonly supersededPreviews: number;
  readonly staleReceipts: number;
  readonly outOfOrderReceipts: number;
  readonly staleProjections: number;
  readonly detachedIntents: number;
}

export interface QueryDiag {
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

export interface LinkDiag {
  readonly attaches: number;
  readonly activeSubscriptions: number;
  readonly totalSubscriptions: number;
  readonly attached: boolean;
  readonly windowId?: string;
}

export interface BindingFinalState {
  readonly phase: CommandPhase;
  readonly timedOut: boolean;
  readonly attached: boolean;
  readonly generation: number;
}

export interface RunTrace {
  readonly dispatched: DispatchRec[];
  readonly phases: PhaseRec[];
  readonly subscriptions: SubscribeRec[];
  readonly unsubscriptions: UnsubscribeRec[];
  readonly queries: QueryIssueRec[];
  readonly queryOutcomes: Record<string, string>;
  readonly bindingDiagnostics: Record<string, BindingDiag>;
  readonly queryDiagnostics: Record<string, QueryDiag>;
  readonly link: LinkDiag;
  readonly finalStates: Record<string, BindingFinalState>;
}

// ── model internals ─────────────────────────────────────────────────

type Rank = 0 | 1 | 2 | 3;

function rank(s: "in-flight" | ReceiptStatus): Rank {
  if (s === "in-flight") return 0;
  if (s === "accepted" || s === "queued") return 1;
  if (s === "applied") return 2;
  return 3;
}

function revisionCompare(a: number | string, b: number | string): number {
  if (typeof a === "number" && typeof b === "number") {
    return a === b ? 0 : a > b ? 1 : -1;
  }
  return a === b ? 0 : 1;
}

interface TrackedModel {
  readonly commandId: string;
  readonly kind: CommandKind;
  readonly generation: number;
  readonly expectReceipt: boolean;
  status: "in-flight" | ReceiptStatus;
  timedOut: boolean;
  dueAt: number | undefined;
}

const TRACKED_LIMIT = 64;

class BindingModel {
  readonly id: string;
  readonly target: LogicalTarget;
  private commandSeq = 0;
  generation = 0;
  attached = true;
  private activeGesture: GestureId | undefined;
  private queue: Array<CommandIntent | undefined> = [];
  private pending = new Map<GestureId, number>();
  private readonly tracked = new Map<string, TrackedModel>();
  private headId: string | undefined;
  private lastObservedRev: number | string | undefined;
  private hasObserved = false;
  viewId: string | undefined;
  windowId: string | undefined;

  private intents_ = 0;
  private dispatched_ = 0;
  private coalesced_ = 0;
  private superseded_ = 0;
  private staleReceipts_ = 0;
  private outOfOrder_ = 0;
  private staleProjections_ = 0;
  private detached_ = 0;

  private readonly ackTimeoutMs: number;
  private readonly now: () => number;
  private readonly trace: TraceSink;
  private readonly onMint: (commandId: string, owner: BindingModel) => void;

  constructor(
    id: string,
    target: LogicalTarget,
    ackTimeoutMs: number,
    now: () => number,
    trace: TraceSink,
    onMint: (commandId: string, owner: BindingModel) => void,
  ) {
    this.id = id;
    this.target = target;
    this.ackTimeoutMs = ackTimeoutMs;
    this.now = now;
    this.trace = trace;
    this.onMint = onMint;
  }

  /** Contract mapping: interaction events → command intents.
   *  `unavailable`/`rejected`/`noop` are local-only — no command. */
  onInteractionEvent(e: InteractionEvent): void {
    const target = e.meta.target ?? this.target;
    const base = {
      target,
      controlId: e.meta.controlId,
      sequence: e.meta.sequence,
      ...(e.meta.originView !== undefined
        ? { originView: e.meta.originView }
        : {}),
      ...(e.meta.atomic === true ? { atomic: true } : {}),
    };
    let cmd: CommandIntent | undefined;
    switch (e.type) {
      case "begin":
        if (e.meta.gestureId === undefined) return;
        cmd = {
          ...base,
          kind: "begin",
          gestureId: e.meta.gestureId,
          value: e.value,
        };
        break;
      case "preview":
        if (e.meta.gestureId === undefined) return;
        cmd = {
          ...base,
          kind: "preview",
          gestureId: e.meta.gestureId,
          value: e.value,
          coalescible: true,
        };
        break;
      case "commit":
        if (e.meta.gestureId === undefined) return;
        cmd = {
          ...base,
          kind: "commit",
          gestureId: e.meta.gestureId,
          value: e.value,
        };
        break;
      case "cancel":
        if (e.meta.gestureId === undefined) return;
        cmd = {
          ...base,
          kind: "cancel",
          gestureId: e.meta.gestureId,
          reason: e.reason,
        };
        break;
      default:
        return;
    }
    this.push(cmd);
  }

  pushAction(action: string, source?: string): void {
    this.push({
      kind: "action",
      target: this.target,
      controlId: this.id,
      sequence: 0,
      action,
      ...(source !== undefined ? { source } : {}),
    });
  }

  /** `push`: count, detached-drop (no emit), gesture tracking, queue,
   *  emit. */
  push(cmd: CommandIntent): void {
    this.intents_ += 1;
    if (!this.attached) {
      this.detached_ += 1;
      return;
    }
    if (cmd.kind === "begin") this.activeGesture = cmd.gestureId;
    else if (cmd.kind === "commit" || cmd.kind === "cancel") {
      this.activeGesture = undefined;
    }
    this.queuePush(cmd);
    this.emit();
  }

  private queuePush(cmd: CommandIntent): void {
    if (cmd.kind === "preview") {
      const at = this.pending.get(cmd.gestureId);
      if (at !== undefined) {
        this.queue[at] = cmd;
        this.coalesced_ += 1;
        return;
      }
      this.pending.set(cmd.gestureId, this.queue.length);
      this.queue.push(cmd);
      return;
    }
    if (cmd.kind === "commit" || cmd.kind === "cancel") {
      const at = this.pending.get(cmd.gestureId);
      if (at !== undefined) {
        this.queue[at] = undefined;
        this.pending.delete(cmd.gestureId);
        this.superseded_ += 1;
      }
    }
    this.queue.push(cmd);
  }

  /** `flush`: drain in order, skipping tombstones; emit once even when
   *  empty. */
  flush(): void {
    for (const slot of this.queue) {
      if (slot !== undefined) this.dispatchNow(slot);
    }
    this.queue = [];
    this.pending.clear();
    this.emit();
  }

  private dispatchNow(cmd: CommandIntent): void {
    this.commandSeq += 1;
    this.dispatched_ += 1;
    const commandId = `cmd-${this.commandSeq}`;
    const expectReceipt = cmd.kind === "commit" || cmd.kind === "action";
    if (cmd.kind !== "preview") {
      const rec: TrackedModel = {
        commandId,
        kind: cmd.kind,
        generation: this.generation,
        expectReceipt,
        status: "in-flight",
        timedOut: false,
        dueAt:
          expectReceipt && this.ackTimeoutMs > 0
            ? this.now() + this.ackTimeoutMs
            : undefined,
      };
      this.tracked.set(commandId, rec);
      if (this.tracked.size > TRACKED_LIMIT) {
        for (const [id, c] of this.tracked) {
          if (this.tracked.size <= TRACKED_LIMIT) break;
          if (c.status === "in-flight" && c.expectReceipt) continue;
          this.tracked.delete(id);
        }
      }
      if (expectReceipt) this.headId = commandId;
    }
    this.onMint(commandId, this);
    this.trace.dispatched.push({
      binding: this.id,
      commandId,
      kind: cmd.kind,
      controlId: cmd.controlId,
      target: cmd.target,
      sequence: cmd.sequence,
      ...(cmd.kind !== "action" && "gestureId" in cmd
        ? { gestureId: cmd.gestureId }
        : {}),
      ...(cmd.kind === "action" ? { action: cmd.action } : {}),
      generation: this.generation,
      expectReceipt,
      ...(this.viewId !== undefined ? { viewId: this.viewId } : {}),
      ...(this.windowId !== undefined ? { windowId: this.windowId } : {}),
    });
  }

  /** Fire due ack timers in mint order; each emits once. */
  advanceTimers(): void {
    for (const rec of this.tracked.values()) {
      if (rec.dueAt !== undefined && rec.dueAt <= this.now()) {
        rec.dueAt = undefined;
        rec.timedOut = true;
        this.emit();
      }
    }
  }

  ack(receipt: CommandReceipt): void {
    const cmd = this.tracked.get(receipt.commandId);
    if (cmd === undefined || cmd.generation !== this.generation) {
      this.staleReceipts_ += 1;
      return;
    }
    const next = rank(receipt.status);
    if (next <= rank(cmd.status)) {
      this.outOfOrder_ += 1;
      return;
    }
    cmd.status = receipt.status;
    if (next === 3 || next === 2) {
      cmd.dueAt = undefined;
    }
    this.emit();
  }

  observe(revision: number | string): void {
    if (!this.attached) {
      this.staleProjections_ += 1;
      return;
    }
    if (
      this.hasObserved &&
      revisionCompare(revision, this.lastObservedRev!) <= 0
    ) {
      this.staleProjections_ += 1;
      return;
    }
    this.hasObserved = true;
    this.lastObservedRev = revision;
    const head = this.head();
    if (
      head !== undefined &&
      (head.status === "applied" ||
        head.status === "rejected" ||
        head.status === "unsupported")
    ) {
      this.headId = undefined;
    }
    this.emit();
  }

  detach(reason: string): void {
    if (!this.attached) return;
    this.attached = false;
    this.queue = [];
    this.pending.clear();
    if (this.activeGesture !== undefined) {
      const gestureId = this.activeGesture;
      this.activeGesture = undefined;
      this.dispatchNow({
        kind: "cancel",
        target: this.target,
        controlId: this.id,
        sequence: 0,
        gestureId,
        reason: reason as CancelReason,
      });
    }
    this.generation += 1;
    for (const rec of this.tracked.values()) rec.dueAt = undefined;
    this.emit();
  }

  attach(epoch?: number): void {
    if (epoch !== undefined) this.generation = epoch;
    if (this.attached) return;
    this.attached = true;
    this.headId = undefined;
    this.activeGesture = undefined;
    this.emit();
  }

  setContext(viewId: string | undefined, windowId: string | undefined): void {
    this.viewId = viewId;
    this.windowId = windowId;
  }

  private head(): TrackedModel | undefined {
    return this.headId === undefined
      ? undefined
      : this.tracked.get(this.headId);
  }

  phase(): CommandPhase {
    if (!this.attached) return "stale";
    if (this.activeGesture !== undefined) return "preview";
    const head = this.head();
    if (head !== undefined) {
      if (
        head.timedOut &&
        (head.status === "in-flight" ||
          head.status === "accepted" ||
          head.status === "queued")
      ) {
        return "timeout";
      }
      switch (head.status) {
        case "in-flight":
          return "pending";
        case "accepted":
        case "queued":
          return "accepted";
        case "applied":
          return "applied";
        case "rejected":
          return "rejected";
        case "unsupported":
          return "unsupported";
      }
    }
    return this.hasObserved ? "observed" : "idle";
  }

  private emit(): void {
    this.trace.phases.push({ binding: this.id, phase: this.phase() });
  }

  diagnostics(): BindingDiag {
    return {
      intents: this.intents_,
      dispatched: this.dispatched_,
      coalesced: this.coalesced_,
      supersededPreviews: this.superseded_,
      staleReceipts: this.staleReceipts_,
      outOfOrderReceipts: this.outOfOrder_,
      staleProjections: this.staleProjections_,
      detachedIntents: this.detached_,
    };
  }

  finalState(): BindingFinalState {
    return {
      phase: this.phase(),
      timedOut: this.head()?.timedOut === true,
      attached: this.attached,
      generation: this.generation,
    };
  }
}

interface InflightModel {
  readonly generation: number;
  readonly sourceId: string;
  readonly sourceRevision: number | string;
  readonly maxBytes: number;
}

class QueryModel {
  private seq = 0;
  generation = 0;
  private readonly inflight = new Map<string, InflightModel>();
  private readonly order: string[] = [];
  private readonly queryToAs = new Map<string, string>();
  private issued_ = 0;
  private resolved_ = 0;
  private stale_ = 0;
  private cancelled_ = 0;
  private overBudget_ = 0;
  private errors_ = 0;
  private evicted_ = 0;

  readonly viewId: string;
  private readonly maxInflight: number;
  private readonly defaultMaxBytes: number;
  private readonly probe:
    | ((sourceId: string) => number | string | undefined)
    | undefined;
  private readonly trace: TraceSink;
  private readonly outcomes: Map<string, string>;

  constructor(
    viewId: string,
    maxInflight: number,
    defaultMaxBytes: number,
    probe:
      | ((sourceId: string) => number | string | undefined)
      | undefined,
    trace: TraceSink,
    outcomes: Map<string, string>,
  ) {
    this.viewId = viewId;
    this.maxInflight = maxInflight;
    this.defaultMaxBytes = defaultMaxBytes;
    this.probe = probe;
    this.trace = trace;
    this.outcomes = outcomes;
  }

  /** `request`: evict-oldest while at cap, mint `q-<view>-<n>` under
   *  the current generation. */
  request(
    as: string,
    input: {
      sourceId: string;
      sourceRevision: number | string;
      start: number;
      end: number;
      maxBytes?: number;
    },
  ): string {
    while (this.inflight.size >= this.maxInflight && this.order.length > 0) {
      const oldest = this.order[0]!;
      this.cancelOne(oldest);
      this.evicted_ += 1;
    }
    this.seq += 1;
    const queryId = `q-${this.viewId}-${this.seq}`;
    const generation = this.generation;
    const maxBytes = input.maxBytes ?? this.defaultMaxBytes;
    this.inflight.set(queryId, {
      generation,
      sourceId: input.sourceId,
      sourceRevision: input.sourceRevision,
      maxBytes,
    });
    this.order.push(queryId);
    this.queryToAs.set(queryId, as);
    this.issued_ += 1;
    this.trace.queries.push({
      queryId,
      viewId: this.viewId,
      generation,
      sourceId: input.sourceId,
      sourceRevision: input.sourceRevision,
      start: input.start,
      end: input.end,
      maxBytes,
    });
    return queryId;
  }

  /** Classify rules: already-removed → cancelled; generation /
   *  sourceRevision / probe mismatch → stale; over maxBytes →
   *  over-budget; else ok. */
  resolve(as: string, queryId: string, reply: Partial<QueryReply>): void {
    const rec = this.inflight.get(queryId);
    if (rec === undefined) {
      this.cancelled_ += 1;
      this.outcomes.set(as, "cancelled");
      return;
    }
    this.finish(queryId);
    const replyGen = reply.generation ?? rec.generation;
    const replyRev = reply.sourceRevision ?? rec.sourceRevision;
    const probeRev = this.probe?.(rec.sourceId);
    if (
      replyGen !== rec.generation ||
      rec.generation !== this.generation ||
      replyRev !== rec.sourceRevision ||
      (probeRev !== undefined && probeRev !== rec.sourceRevision)
    ) {
      this.stale_ += 1;
      this.outcomes.set(as, "stale");
      return;
    }
    if (reply.bytes !== undefined && reply.bytes > rec.maxBytes) {
      this.overBudget_ += 1;
      this.outcomes.set(as, "over-budget");
      return;
    }
    this.resolved_ += 1;
    this.outcomes.set(as, "ok");
  }

  reject(as: string, queryId: string): void {
    const rec = this.inflight.get(queryId);
    if (rec === undefined) {
      this.cancelled_ += 1;
      this.outcomes.set(as, "cancelled");
      return;
    }
    this.finish(queryId);
    this.errors_ += 1;
    this.outcomes.set(as, "error");
  }

  private cancelOne(queryId: string): void {
    const rec = this.inflight.get(queryId);
    if (rec === undefined) return;
    this.finish(queryId);
    this.cancelled_ += 1;
    const as = this.queryToAs.get(queryId);
    if (as !== undefined) this.outcomes.set(as, "cancelled");
  }

  cancelAll(): void {
    for (const id of [...this.order]) this.cancelOne(id);
  }

  attach(epoch?: number): void {
    this.generation = epoch ?? this.generation + 1;
    this.cancelAll();
  }

  detach(): void {
    this.attach();
  }

  private finish(queryId: string): void {
    this.inflight.delete(queryId);
    const at = this.order.indexOf(queryId);
    if (at >= 0) this.order.splice(at, 1);
  }

  diagnostics(): QueryDiag {
    return {
      issued: this.issued_,
      resolved: this.resolved_,
      stale: this.stale_,
      cancelled: this.cancelled_,
      overBudget: this.overBudget_,
      errors: this.errors_,
      evicted: this.evicted_,
      inflight: this.inflight.size,
      generation: this.generation,
    };
  }
}

// ── driver-facing oracle ────────────────────────────────────────────

export interface TraceSink {
  readonly dispatched: DispatchRec[];
  readonly phases: PhaseRec[];
  readonly subscriptions: SubscribeRec[];
  readonly unsubscriptions: UnsubscribeRec[];
  readonly queries: QueryIssueRec[];
}

export type ConsumerKind = "daw" | "analysis";

/**
 * The oracle's view-level model: the documented `ViewLink` /
 * `LinkRegistry` lifecycle over the registered binding/query models,
 * plus the scripted-backend behavior the scenario's consumer declares
 * (a `daw` backend pushes its store's current projection on subscribe;
 * an `analysis` backend does not).
 */
export class ContractModel {
  private readonly trace: TraceSink = {
    dispatched: [],
    phases: [],
    subscriptions: [],
    unsubscriptions: [],
    queries: [],
  };
  private readonly bindings = new Map<string, BindingModel>();
  private readonly bindingsByTarget = new Map<string, BindingModel>();
  private readonly mintedBy = new Map<string, BindingModel>();
  private readonly store = new Map<string, { revision: number }>();
  private readonly queryAs = new Map<string, string>();
  private readonly queryOutcomes = new Map<string, string>();
  private queryModel: QueryModel | undefined;
  private linkGeneration = 0;
  private linkAttached = false;
  private linkWindow: string | undefined;
  private attachCount = 0;
  private activeSubs = 0;
  private totalSubs = 0;
  now = 0;

  private readonly viewId: string;
  private readonly consumer: ConsumerKind;
  private readonly ackTimeoutMs: number;
  private readonly queryConfig: {
    maxInflight: number;
    defaultMaxBytes: number;
    probe?: (sourceId: string) => number | string | undefined;
  };

  constructor(
    viewId: string,
    consumer: ConsumerKind,
    ackTimeoutMs: number,
    queryConfig: {
      maxInflight: number;
      defaultMaxBytes: number;
      probe?: (sourceId: string) => number | string | undefined;
    },
  ) {
    this.viewId = viewId;
    this.consumer = consumer;
    this.ackTimeoutMs = ackTimeoutMs;
    this.queryConfig = queryConfig;
  }

  /** Register a control binding (id = controlId) plus its declared
   *  store entry (initial revision 0). */
  addBinding(controlId: string, target: LogicalTarget): void {
    const b = new BindingModel(
      controlId,
      target,
      this.ackTimeoutMs,
      () => this.now,
      this.trace,
      (commandId, owner) => this.mintedBy.set(commandId, owner),
    );
    this.bindings.set(controlId, b);
    this.bindingsByTarget.set(target, b);
    this.store.set(target, { revision: 0 });
    // `createBinding` on an attached link: context + subscribe, but the
    // binding does NOT adopt the view epoch until its next detach cycle.
    if (this.linkAttached) {
      b.setContext(this.viewId, this.linkWindow);
      this.subscribe(b);
    }
  }

  addQuery(): void {
    if (this.queryModel !== undefined) return;
    this.queryModel = new QueryModel(
      this.viewId,
      this.queryConfig.maxInflight,
      this.queryConfig.defaultMaxBytes,
      this.queryConfig.probe,
      this.trace,
      this.queryOutcomes,
    );
  }

  binding(controlId: string): BindingModel | undefined {
    return this.bindings.get(controlId);
  }

  /** Route a receipt to the binding that minted the commandId; an
   *  unminted id arrives at the first binding (the port-edge owner the
   *  driver routes to) and classifies stale there. */
  ack(receipt: CommandReceipt): void {
    const owner =
      this.mintedBy.get(receipt.commandId) ??
      this.bindings.values().next().value;
    if (owner !== undefined) owner.ack(receipt);
  }

  /** Flush every binding's queue in registration order — the driver's
   *  `flush` op drains all pending commands under the deferred-flush
   *  harness. */
  flush(): void {
    for (const b of this.bindings.values()) b.flush();
  }

  private subscribe(b: BindingModel): void {
    this.totalSubs += 1;
    this.activeSubs += 1;
    this.trace.subscriptions.push({
      binding: b.id,
      target: b.target,
      viewId: this.viewId,
      ...(this.linkWindow !== undefined ? { windowId: this.linkWindow } : {}),
      generation: this.linkGeneration,
    });
    // Scripted-backend contract: a control-shaped consumer pushes its
    // store's current projection immediately on subscribe.
    if (this.consumer === "daw") {
      const cur = this.store.get(b.target);
      if (cur !== undefined) b.observe(cur.revision);
    }
  }

  /** ViewLink.attach contract. */
  attach(windowId: string): void {
    if (this.linkAttached && this.linkWindow === windowId) return;
    if (this.linkAttached) this.detach("dock-transfer");
    this.linkAttached = true;
    this.linkWindow = windowId;
    this.linkGeneration += 1;
    this.attachCount += 1;
    for (const b of this.bindings.values()) {
      b.setContext(this.viewId, windowId);
      b.attach(this.linkGeneration);
      this.subscribe(b);
    }
    this.queryModel?.attach(this.linkGeneration);
  }

  /** ViewLink.detach contract: subscriptions released first, then
   *  gesture cancels + queue purge + query cancellation. */
  detach(reason: string): void {
    if (!this.linkAttached) return;
    this.linkAttached = false;
    for (const b of this.bindings.values()) {
      this.trace.unsubscriptions.push({ binding: b.id, target: b.target });
      this.activeSubs = Math.max(0, this.activeSubs - 1);
      b.detach(reason);
    }
    this.queryModel?.detach();
  }

  /** LinkRegistry.closeView: detach("explicit") + drop the entry —
   *  `windowId` on the link survives (ViewLink never clears it on
   *  detach). */
  closeView(): void {
    this.detach("explicit");
  }

  get linkAttachedNow(): boolean {
    return this.linkAttached;
  }

  /** Consumer projection fan-out on `target`. */
  projection(target: string, revision: number | string): void {
    this.bindingsByTarget.get(target)?.observe(revision);
  }

  query(
    as: string,
    input: {
      sourceId: string;
      sourceRevision: number | string;
      start: number;
      end: number;
      maxBytes?: number;
    },
  ): string {
    if (this.queryModel === undefined) {
      throw new Error("oracle: no query model registered for this view");
    }
    const queryId = this.queryModel.request(as, input);
    this.queryAs.set(as, queryId);
    return queryId;
  }

  resolveQuery(as: string, reply: Partial<QueryReply>): void {
    const queryId = this.queryAs.get(as);
    if (this.queryModel === undefined || queryId === undefined) {
      this.queryOutcomes.set(as, "cancelled");
      return;
    }
    this.queryModel.resolve(as, queryId, reply);
  }

  rejectQuery(as: string): void {
    const queryId = this.queryAs.get(as);
    if (this.queryModel === undefined || queryId === undefined) {
      this.queryOutcomes.set(as, "cancelled");
      return;
    }
    this.queryModel.reject(as, queryId);
  }

  advance(ms: number): void {
    this.now += ms;
    for (const b of this.bindings.values()) b.advanceTimers();
  }

  /** Final observation snapshot — same shape as the driver's trace. */
  traceResult(): RunTrace {
    const bindingDiagnostics: Record<string, BindingDiag> = {};
    const finalStates: RunTrace["finalStates"] = {};
    for (const [id, b] of this.bindings) {
      bindingDiagnostics[id] = b.diagnostics();
      finalStates[id] = b.finalState();
    }
    const queryDiagnostics: Record<string, QueryDiag> = {};
    if (this.queryModel !== undefined) {
      queryDiagnostics[this.viewId] = this.queryModel.diagnostics();
    }
    return {
      dispatched: this.trace.dispatched,
      phases: this.trace.phases,
      subscriptions: this.trace.subscriptions,
      unsubscriptions: this.trace.unsubscriptions,
      queries: this.trace.queries,
      queryOutcomes: Object.fromEntries(this.queryOutcomes),
      bindingDiagnostics,
      queryDiagnostics,
      link: {
        attaches: this.attachCount,
        activeSubscriptions: this.activeSubs,
        totalSubscriptions: this.totalSubs,
        attached: this.linkAttached,
        ...(this.linkWindow !== undefined ? { windowId: this.linkWindow } : {}),
      },
      finalStates,
    };
  }
}

export type { QueryErrorKind };
