/**
 * Issue #261 — `CommandBinding`: the per-target binding session.
 *
 * One `CommandBinding` sits between a UI intent stream (headless
 * gesture events from `ui/core` via the DOM adapter, or direct calls)
 * and the consumer's {@link BindingPort.dispatch}. It owns:
 *
 * - **intent → command translation** (`contract.ts`) and the bounded
 *   preview coalescing queue (`coalesce.ts`),
 * - **receipt correlation** — per-command lifecycle with strict
 *   ordering: `accepted`/`queued` < `applied`; `rejected`/`unsupported`
 *   terminal. Delayed, reordered, and duplicate receipts are absorbed
 *   and counted, never applied twice and never regressed,
 * - **ack timeout** — a receipt-expecting command (`commit`, `action`)
 *   that outlives `ackTimeoutMs` turns `timeout`: an explicit failure
 *   state, not a fabricated commit,
 * - **projection guard** — `observe` accepts only projections that
 *   advance the stored revision; stale/delayed answers cannot overwrite
 *   a newer truth,
 * - **generations** — `detach` bumps the generation so receipts and
 *   projections minted under a dead view epoch are classified `stale`
 *   instead of driving a rebound view's display.
 *
 * What it does NOT own: authorization, undo, domain revision rules,
 * subscription handles (the {@link ViewLink} in `link.ts` re-subscribes
 * across window moves), and element state reflection (`dom.ts`).
 */

import type {
  CancelReason,
  ControlId,
  GestureId,
  InteractionEvent,
  LogicalTarget,
} from "../../core/src/index.ts";
import type {
  BindingPort,
  BindingViewState,
  CommandId,
  CommandIntent,
  CommandKind,
  CommandPhase,
  CommandReceipt,
  DispatchContext,
  ReceiptStatus,
  StateProjection,
} from "./contract.ts";
import { intentFromGesture } from "./contract.ts";
import { IntentQueue, type IntentQueueStats } from "./coalesce.ts";

export const DEFAULT_ACK_TIMEOUT_MS = 5000;
const TRACKED_LIMIT = 64;

/** `(fn, ms) → cancel?` — injectable timer for deterministic tests. */
export type ScheduleFn = (fn: () => void, ms: number) => (() => void) | undefined;

const defaultSchedule: ScheduleFn = (fn, ms) => {
  const h = setTimeout(fn, ms);
  return () => clearTimeout(h);
};

const defaultFlush: (fn: () => void) => void = (fn) => queueMicrotask(fn);

/** Lifecycle record for one dispatched non-droppable command. */
interface TrackedCommand {
  readonly commandId: CommandId;
  readonly kind: CommandKind;
  readonly generation: number;
  readonly expectReceipt: boolean;
  status: "in-flight" | ReceiptStatus;
  timedOut: boolean;
  cancelTimer: (() => void) | undefined;
}

/** Counters proving the "no misdelivery" contract — read in tests and
 * diagnostics rather than inferred. */
export interface BindingDiagnostics {
  readonly intents: number;
  readonly dispatched: number;
  readonly coalesced: number;
  readonly supersededPreviews: number;
  /** Receipts dropped: unknown commandId or dead generation. */
  readonly staleReceipts: number;
  /** Receipts that arrived out of order or duplicated. */
  readonly outOfOrderReceipts: number;
  /** Projections dropped by the revision guard or while detached. */
  readonly staleProjections: number;
  /** Intents dropped because the binding was detached. */
  readonly detachedIntents: number;
}

export interface CommandBindingOptions {
  readonly target: LogicalTarget;
  readonly port: Pick<BindingPort, "dispatch">;
  /** When set, gesture events for other controlIds are ignored —
   *  lets one binding sit on a multi-control event stream. */
  readonly controlId?: ControlId;
  readonly viewId?: string;
  readonly windowId?: string;
  readonly clock?: () => number;
  /** Timer impl — `setTimeout` by default. */
  readonly schedule?: ScheduleFn;
  /** How queued intents flush — `queueMicrotask` by default. */
  readonly flushWith?: (fn: () => void) => void;
  /** Ack bound for receipt-expecting commands; `<= 0` disables. */
  readonly ackTimeoutMs?: number;
  readonly onState?: (state: BindingViewState) => void;
}

/**
 * Compare projection revisions: numbers compare numerically; equal
 * identities of any other kind compare 0 and different identities are
 * treated as "advanced" (a new revision token always supersedes).
 * Consumers needing strict ordering on non-numeric revisions supply
 * their own compare through {@link CommandBindingOptions} — the default
 * only implements the safe subset.
 */
export function defaultRevisionCompare(
  a: number | string,
  b: number | string,
): number {
  if (typeof a === "number" && typeof b === "number") {
    return a === b ? 0 : a > b ? 1 : -1;
  }
  return a === b ? 0 : 1;
}

export class CommandBinding<P = unknown> {
  readonly target: LogicalTarget;
  private readonly port: Pick<BindingPort, "dispatch">;
  private readonly controlId_: ControlId | undefined;
  private viewId_: string | undefined;
  private windowId_: string | undefined;
  private readonly clock: () => number;
  private readonly schedule: ScheduleFn;
  private readonly flushWith: (fn: () => void) => void;
  private readonly ackTimeoutMs: number;
  private readonly revisionCompare: (a: number | string, b: number | string) => number;

  private readonly queue = new IntentQueue();
  private readonly tracked = new Map<CommandId, TrackedCommand>();
  private readonly stateListeners = new Set<(s: BindingViewState<P>) => void>();

  private commandSeq = 0;
  private generation_ = 0;
  private attached_ = true;
  private activeGesture_: GestureId | undefined;
  private headId_: CommandId | undefined;
  private lastObserved_: StateProjection<P> | undefined;
  private observedSeq_ = 0;
  private lastReceipt_: CommandReceipt | undefined;
  private flushScheduled = false;

  private intents_ = 0;
  private dispatched_ = 0;
  private staleReceipts_ = 0;
  private outOfOrderReceipts_ = 0;
  private staleProjections_ = 0;
  private detachedIntents_ = 0;

  constructor(options: CommandBindingOptions) {
    this.target = options.target;
    this.port = options.port;
    if (options.controlId !== undefined) this.controlId_ = options.controlId;
    if (options.viewId !== undefined) this.viewId_ = options.viewId;
    if (options.windowId !== undefined) this.windowId_ = options.windowId;
    this.clock = options.clock ?? (() => Date.now());
    this.schedule = options.schedule ?? defaultSchedule;
    this.flushWith = options.flushWith ?? defaultFlush;
    const t = options.ackTimeoutMs ?? DEFAULT_ACK_TIMEOUT_MS;
    this.ackTimeoutMs = t > 0 && Number.isFinite(t) ? t : 0;
    this.revisionCompare = defaultRevisionCompare;
    if (options.onState !== undefined) this.stateListeners.add(options.onState);
  }

  // ── context / lifecycle ───────────────────────────────────────────

  /** The link supplies view/window context on attach. */
  setContext(ctx: { viewId?: string; windowId?: string }): void {
    this.viewId_ = ctx.viewId;
    this.windowId_ = ctx.windowId;
  }

  get generation(): number {
    return this.generation_;
  }

  get attached(): boolean {
    return this.attached_;
  }

  /**
   * Re-attach after a detach. The dead epoch's head command is
   * forgotten (its receipts are already generation-stale); a fresh
   * subscription — driven by the link — re-reads truth. `epoch`, when
   * supplied by a {@link ViewLink}, adopts the view's generation so
   * dispatched commands carry the view epoch (contract §13); without
   * it the binding keeps its own detach counter.
   */
  attach(epoch?: number): void {
    if (epoch !== undefined) this.generation_ = epoch;
    if (this.attached_) return;
    this.attached_ = true;
    this.headId_ = undefined;
    this.activeGesture_ = undefined;
    this.emit();
  }

  /**
   * Detach (unmount / dock transfer / window close). Per contract the
   * in-flight gesture is CANCELLED — the cancel command still reaches
   * the consumer — while already-dispatched commands are left to the
   * consumer (a view move never retracts them). Queued intents,
   * pending previews, and ack timers die with the epoch.
   */
  detach(reason: CancelReason = "unmount"): void {
    if (!this.attached_) return;
    this.attached_ = false;
    this.queue.purge();
    if (this.activeGesture_ !== undefined) {
      const gestureId = this.activeGesture_;
      this.activeGesture_ = undefined;
      // Minted under the OLD generation: the cancel is the terminal
      // message *of* this epoch — the bump below marks its end.
      this.dispatchNow({
        kind: "cancel",
        target: this.target,
        controlId: this.controlId_ ?? this.target,
        sequence: 0,
        gestureId,
        reason,
      });
    }
    this.generation_ += 1;
    for (const cmd of this.tracked.values()) {
      cmd.cancelTimer?.();
      cmd.cancelTimer = undefined;
    }
    this.emit();
  }

  // ── intents ───────────────────────────────────────────────────────

  /** Feed one headless gesture event. Local-only event kinds
   *  (`unavailable`, `rejected`, `noop`) never reach the port. */
  event(e: InteractionEvent): void {
    if (this.controlId_ !== undefined && e.meta.controlId !== this.controlId_) {
      return;
    }
    const cmd = intentFromGesture(e, this.target);
    if (cmd === undefined) return;
    this.push(cmd);
  }

  /** Dispatch a discrete action command (transport, buttons). */
  action(
    action: string,
    meta?: { controlId?: ControlId; sequence?: number; source?: string },
  ): void {
    this.push({
      kind: "action",
      target: this.target,
      controlId: meta?.controlId ?? this.controlId_ ?? this.target,
      sequence: meta?.sequence ?? 0,
      action,
      ...(meta?.source !== undefined ? { source: meta.source } : {}),
    });
  }

  /** Feed a raw intent (non-gesture producers). */
  push(cmd: CommandIntent): void {
    this.intents_ += 1;
    if (!this.attached_) {
      this.detachedIntents_ += 1;
      return;
    }
    if (cmd.kind === "begin") this.activeGesture_ = cmd.gestureId;
    else if (cmd.kind === "commit" || cmd.kind === "cancel") {
      this.activeGesture_ = undefined;
    }
    this.queue.push(cmd);
    if (!this.flushScheduled) {
      this.flushScheduled = true;
      this.flushWith(() => {
        this.flushScheduled = false;
        this.flush();
      });
    }
    this.emit();
  }

  /** Flush queued intents synchronously (deterministic entry point). */
  flush(): void {
    for (const cmd of this.queue.drain()) this.dispatchNow(cmd);
    this.emit();
  }

  private dispatchNow(cmd: CommandIntent): void {
    this.commandSeq += 1;
    this.dispatched_ += 1;
    const commandId = `cmd-${this.commandSeq}`;
    const expectReceipt = cmd.kind === "commit" || cmd.kind === "action";
    if (cmd.kind !== "preview") {
      const rec: TrackedCommand = {
        commandId,
        kind: cmd.kind,
        generation: this.generation_,
        expectReceipt,
        status: "in-flight",
        timedOut: false,
        cancelTimer: undefined,
      };
      this.tracked.set(commandId, rec);
      if (this.tracked.size > TRACKED_LIMIT) {
        // Evict settled records first — receipt-expecting commands
        // still in flight must stay correlatable. Non-receipt commands
        // (`begin`/`cancel`) never settle via receipt, so they age out
        // on this bound instead of accumulating forever.
        for (const [id, c] of this.tracked) {
          if (this.tracked.size <= TRACKED_LIMIT) break;
          if (c.status === "in-flight" && c.expectReceipt) continue;
          this.tracked.delete(id);
        }
      }
      if (expectReceipt) this.headId_ = commandId;
      this.armTimeout(rec);
    }
    const ctx: DispatchContext = {
      commandId,
      generation: this.generation_,
      issuedAt: this.clock(),
      expectReceipt,
      ...(this.viewId_ !== undefined ? { viewId: this.viewId_ } : {}),
      ...(this.windowId_ !== undefined ? { windowId: this.windowId_ } : {}),
    };
    this.port.dispatch(cmd, ctx);
  }

  private armTimeout(cmd: TrackedCommand): void {
    if (!cmd.expectReceipt || this.ackTimeoutMs <= 0) return;
    cmd.cancelTimer = this.schedule(() => {
      cmd.cancelTimer = undefined;
      cmd.timedOut = true;
      this.emit();
    }, this.ackTimeoutMs);
  }

  // ── consumer feedback ─────────────────────────────────────────────

  /**
   * Feed one receipt from the consumer's ack channel. Ordering rules:
   * `accepted`/`queued` < `applied`, terminal = `rejected`/`unsupported`.
   * Out-of-order and duplicate receipts are absorbed and counted;
   * receipts for unknown commands or dead generations are classified
   * stale and can never move the display backwards.
   */
  ack(receipt: CommandReceipt): void {
    const cmd = this.tracked.get(receipt.commandId);
    if (cmd === undefined || cmd.generation !== this.generation_) {
      this.staleReceipts_ += 1;
      return;
    }
    const rank = (s: TrackedCommand["status"]): number =>
      s === "in-flight" ? 0 : s === "accepted" || s === "queued" ? 1 : s === "applied" ? 2 : 3;
    const next = rank(receipt.status);
    const cur = rank(cmd.status);
    if (next <= cur) {
      // Duplicate or regressed (applied→accepted, terminal→anything).
      this.outOfOrderReceipts_ += 1;
      return;
    }
    cmd.status = receipt.status;
    if (next === 3 || next === 2) {
      // applied / rejected / unsupported end the wait — a timeout
      // already fired stays recorded in `timedOut`.
      cmd.cancelTimer?.();
      cmd.cancelTimer = undefined;
    }
    this.lastReceipt_ = receipt;
    this.emit();
  }

  /**
   * Feed the authoritative projection from the consumer's subscription.
   * Only a revision advance lands; equal/older revisions are stale and
   * dropped so a delayed answer can't resurrect an old truth.
   */
  observe(projection: StateProjection<P>): void {
    if (!this.attached_) {
      this.staleProjections_ += 1;
      return;
    }
    const last = this.lastObserved_;
    if (
      last !== undefined &&
      this.revisionCompare(projection.revision, last.revision) <= 0
    ) {
      this.staleProjections_ += 1;
      return;
    }
    this.lastObserved_ = projection;
    this.observedSeq_ += 1;
    // A fresh projection retires a *terminal* head: the newest truth
    // wins the display (the receipt stays in `lastReceipt`). A head
    // still awaiting confirmation keeps the phase pending-ish — the
    // projection cannot answer "did my command land" for it.
    const head = this.head();
    if (
      head !== undefined &&
      (head.status === "applied" ||
        head.status === "rejected" ||
        head.status === "unsupported")
    ) {
      this.headId_ = undefined;
    }
    this.emit();
  }

  // ── state surface ─────────────────────────────────────────────────

  private head(): TrackedCommand | undefined {
    return this.headId_ === undefined
      ? undefined
      : this.tracked.get(this.headId_);
  }

  private phase(): CommandPhase {
    if (!this.attached_) return "stale";
    if (this.activeGesture_ !== undefined) return "preview";
    const head = this.head();
    if (head !== undefined) {
      // The ack bound was crossed without an applied/terminal answer —
      // a distinct failure display, kept even while the phase map below
      // would otherwise read "accepted". A late `applied` still moves on.
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
    return this.lastObserved_ !== undefined ? "observed" : "idle";
  }

  get state(): BindingViewState<P> {
    const head = this.head();
    return {
      phase: this.phase(),
      lastObserved: this.lastObserved_,
      observedSeq: this.observedSeq_,
      lastReceipt: this.lastReceipt_,
      pendingCommandId: this.headId_,
      timedOut: head?.timedOut === true,
      generation: this.generation_,
      attached: this.attached_,
    };
  }

  get diagnostics(): BindingDiagnostics {
    const qs: IntentQueueStats = this.queue.stats;
    return {
      intents: this.intents_,
      // Includes queue-bypassed dispatches (the detach-time cancel).
      dispatched: this.dispatched_,
      coalesced: qs.coalesced,
      supersededPreviews: qs.superseded,
      staleReceipts: this.staleReceipts_,
      outOfOrderReceipts: this.outOfOrderReceipts_,
      staleProjections: this.staleProjections_,
      detachedIntents: this.detachedIntents_,
    };
  }

  addStateListener(fn: (s: BindingViewState<P>) => void): () => void {
    this.stateListeners.add(fn);
    return () => this.stateListeners.delete(fn);
  }

  private emit(): void {
    if (this.stateListeners.size === 0) return;
    const s = this.state;
    for (const fn of this.stateListeners) fn(s);
  }
}
