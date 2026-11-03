/**
 * Issue #261 — generic Command/Query binding contract.
 *
 * This module is the wire-level agreement between UI intents and a
 * consumer-owned Command/Query gateway (Issue #257 §5):
 *
 * - **UI intents are not receipts.** A gesture `commit` is the user's
 *   intent to finish; only a consumer {@link CommandReceipt} or an
 *   authoritative {@link StateProjection} may assert that a side effect
 *   happened. This package never promotes an intent into a claim.
 * - **The port is generic.** `dispatch` / `subscribe` / `query` /
 *   `cancelQuery` is the whole capability set. Authorization, target
 *   validation, expected-revision checks, undo, and job ownership stay
 *   inside the consumer's gateway — they are never re-implemented here.
 * - **Delivery classes.** `begin` / `commit` / `cancel` / `action` are
 *   non-droppable; `preview` is coalescible under bounded latest-wins
 *   (see `coalesce.ts`). RT delivery precision is the consumer's
 *   concern (Ruviform #186/#190); this package adds no timing claims.
 * - **Viz queries are bounded.** `ViewportQuery` carries
 *   source/revision/viewport plus `maxBytes` (the `rvf://data` bound,
 *   16 MiB by default) and a per-view `generation`; stale-generation or
 *   stale-revision replies are discarded, never drawn.
 *
 * No DOM, UI framework, WebView, or renderer types appear here — the
 * contract is plain TypeScript so vanilla TS and framework consumers
 * share one port.
 */

import type {
  CancelReason,
  ControlId,
  ControlValue,
  GestureId,
  InteractionEvent,
  LogicalTarget,
  OriginView,
  SequenceNumber,
} from "../../core/src/index.ts";

// ── commands ────────────────────────────────────────────────────────

/** Binding-minted command identifier; unique per binding instance. */
export type CommandId = string;

/** The dispatchable command kinds. `preview` is the only droppable one. */
export type CommandKind = "begin" | "preview" | "commit" | "cancel" | "action";

interface CommandBase {
  /** Logical command target — the element's `target` or the binding's. */
  readonly target: LogicalTarget;
  /** Element-level control id from the headless event meta. */
  readonly controlId: ControlId;
  /** Element-level event sequence (per-element monotonic, Issue #259). */
  readonly sequence: SequenceNumber;
  readonly gestureId?: GestureId;
  readonly originView?: OriginView;
  /** `true` on the synthesized pair of an atomic programmatic input. */
  readonly atomic?: boolean;
}

/**
 * A command intent handed to the consumer's `dispatch` port. The union
 * is deliberately flat and serializable — no callbacks cross the port.
 */
export type CommandIntent =
  | (CommandBase & {
      readonly kind: "begin";
      readonly gestureId: GestureId;
      readonly value: ControlValue;
    })
  | (CommandBase & {
      readonly kind: "preview";
      readonly gestureId: GestureId;
      readonly value: ControlValue;
      readonly coalescible: true;
    })
  | (CommandBase & {
      readonly kind: "commit";
      readonly gestureId: GestureId;
      readonly value: ControlValue;
    })
  | (CommandBase & {
      readonly kind: "cancel";
      readonly gestureId: GestureId;
      readonly reason: CancelReason;
    })
  | (CommandBase & {
      readonly kind: "action";
      /** Discrete action name (e.g. a transport action). */
      readonly action: string;
      readonly source?: string;
    });

/** `preview` is the only kind a transport may coalesce; everything else
 * is a boundary or a discrete, possibly non-idempotent operation. */
export function isDroppable(cmd: CommandIntent): boolean {
  return cmd.kind === "preview";
}

/**
 * Translate one headless {@link InteractionEvent} into a command intent.
 * Returns `undefined` for local-only events (`unavailable`, `rejected`,
 * `noop`) — those are element-contract notifications, never gateway
 * commands. `fallbackTarget` supplies the target when the event meta
 * carries none; `undefined` result also when no target is resolvable.
 */
export function intentFromGesture(
  e: InteractionEvent,
  fallbackTarget: LogicalTarget | undefined,
): CommandIntent | undefined {
  const target = e.meta.target ?? fallbackTarget;
  if (target === undefined) return undefined;
  const base = {
    target,
    controlId: e.meta.controlId,
    sequence: e.meta.sequence,
    ...(e.meta.originView !== undefined
      ? { originView: e.meta.originView }
      : {}),
    ...(e.meta.atomic === true ? { atomic: true } : {}),
  };
  switch (e.type) {
    case "begin":
      if (e.meta.gestureId === undefined) return undefined;
      return { ...base, kind: "begin", gestureId: e.meta.gestureId, value: e.value };
    case "preview":
      if (e.meta.gestureId === undefined) return undefined;
      return {
        ...base,
        kind: "preview",
        gestureId: e.meta.gestureId,
        value: e.value,
        coalescible: true,
      };
    case "commit":
      if (e.meta.gestureId === undefined) return undefined;
      return { ...base, kind: "commit", gestureId: e.meta.gestureId, value: e.value };
    case "cancel":
      if (e.meta.gestureId === undefined) return undefined;
      return {
        ...base,
        kind: "cancel",
        gestureId: e.meta.gestureId,
        reason: e.reason,
      };
    default:
      return undefined;
  }
}

// ── receipts ────────────────────────────────────────────────────────

/**
 * What the consumer gateway reports about a dispatched command.
 * `accepted`/`queued` mean the gateway took responsibility; `applied`
 * means the domain state was changed — neither implies `observed`,
 * which arrives only through a {@link StateProjection}.
 */
export type ReceiptStatus =
  | "accepted"
  | "queued"
  | "applied"
  | "rejected"
  | "unsupported";

export interface CommandReceipt {
  /** The `commandId` the binding minted at dispatch. */
  readonly commandId: CommandId;
  readonly status: ReceiptStatus;
  /** Consumer domain revision after the reported transition. */
  readonly revision?: number | string;
  readonly detail?: string;
}

// ── projections ─────────────────────────────────────────────────────

/**
 * Authoritative state pushed by the consumer. `revision` is the
 * ordering token the stale-response check runs on: a projection that
 * does not advance the last observed revision is dropped, so a delayed
 * answer can never overwrite a newer truth.
 */
export interface StateProjection<P = unknown> {
  readonly revision: number | string;
  readonly value: P;
  /** Optional provenance tag for diagnostics (consumer-defined). */
  readonly provenance?: string;
}

export type ProjectionListener<P = unknown> = (
  projection: StateProjection<P>,
) => void;

/** Returns an `unsubscribe` callback (or `undefined` when the port
 * manages cleanup itself). */
export type Unsubscribe = () => void;

// ── display phases ──────────────────────────────────────────────────

/**
 * The per-binding display lifecycle. Distinct on purpose
 * (Issue #257 §5, Issue #261 contract):
 *
 * - `preview` — a gesture is active; the shown value is a local
 *   tentative value, not a claim about the domain.
 * - `pending` — a receipt-expecting command was dispatched; no ack yet.
 * - `accepted` — the gateway took it (or queued it); still not applied.
 * - `applied` — the consumer reported the change applied; the
 *   authoritative projection has not necessarily been observed.
 * - `observed` — an authoritative projection landed. This is the only
 *   phase that proves domain state.
 * - `rejected` / `unsupported` — explicit terminal failures; the last
 *   observed value is the truth, never the failed intent.
 * - `timeout` — no receipt inside the ack bound; an explicit failure,
 *   never a fabricated success.
 * - `stale` — the binding is detached; late receipts/projections must
 *   not drive display.
 * - `idle` — no gesture in flight and nothing awaiting confirmation.
 */
export type CommandPhase =
  | "idle"
  | "preview"
  | "pending"
  | "accepted"
  | "applied"
  | "observed"
  | "rejected"
  | "unsupported"
  | "timeout"
  | "stale";

/** The state a presentation layer reflects onto a control. */
export interface BindingViewState<P = unknown> {
  readonly phase: CommandPhase;
  /** Latest accepted authoritative projection (revision-guarded). */
  readonly lastObserved: StateProjection<P> | undefined;
  /** Monotonic counter of accepted projections — lets an adapter tell
   * "a new truth arrived" apart from "only the phase moved". */
  readonly observedSeq: number;
  /** Most recent receipt that passed the dedup/ordering checks. */
  readonly lastReceipt: CommandReceipt | undefined;
  /** The newest receipt-expecting command, when one exists. */
  readonly pendingCommandId: CommandId | undefined;
  /** `true` when the pending command crossed the ack bound — kept set
   * even if a late receipt later moves the phase on. */
  readonly timedOut: boolean;
  /** Binding generation — bumped on every detach so late replies are
   * attributable to a dead view epoch. */
  readonly generation: number;
  readonly attached: boolean;
}

// ── queries ─────────────────────────────────────────────────────────

/** Binding-minted query identifier. */
export type QueryId = string;

/**
 * A bounded viz/analysis read (Issue #257 §6.1). `generation` is the
 * view-epoch token: replies minted under a superseded generation are
 * discarded. `maxBytes` bounds the transfer — the `rvf://data`
 * range-pull bound (default {@link DEFAULT_QUERY_MAX_BYTES}).
 */
export interface ViewportQuery {
  readonly queryId: QueryId;
  readonly sourceId: string;
  readonly sourceRevision: number | string;
  readonly viewId: string;
  readonly generation: number;
  readonly start: number;
  readonly end: number;
  /** Resolution bound (e.g. plot px / LOD budget). */
  readonly buckets?: number;
  readonly channels?: readonly number[];
  readonly maxBytes?: number;
  /** Wire encodings the requester accepts (e.g. RVBW/RVST ids). */
  readonly encoding?: readonly string[];
}

export interface QueryReply<T = unknown> {
  /** Must echo `ViewportQuery.generation`. */
  readonly generation: number;
  /** Must echo `ViewportQuery.sourceRevision`. */
  readonly sourceRevision: number | string;
  readonly payload: T;
  /** Declared transfer size, checked against `maxBytes` when present. */
  readonly bytes?: number;
}

export type QueryErrorKind =
  | "cancelled"
  | "stale"
  | "over-budget"
  | "unsupported"
  | "error";

export interface QueryError {
  readonly kind: QueryErrorKind;
  readonly detail?: string;
  readonly cause?: unknown;
}

export type QueryOutcome<T = unknown> =
  | { readonly ok: true; readonly reply: QueryReply<T> }
  | { readonly ok: false; readonly error: QueryError };

/** Default transfer bound — the `rvf://data` range-pull limit. */
export const DEFAULT_QUERY_MAX_BYTES = 16 * 1024 * 1024;

// ── the port ────────────────────────────────────────────────────────

/** Context handed to the consumer on every dispatch. */
export interface DispatchContext {
  readonly commandId: CommandId;
  readonly viewId?: string;
  readonly windowId?: string;
  readonly generation: number;
  readonly issuedAt: number;
  /**
   * `true` when the binding expects a receipt (commit / discrete
   * action); lifecycle commands (`begin`/`cancel`) are delivered but do
   * not require one. A consumer may still report on them — the binding
   * correlates by `commandId` either way.
   */
  readonly expectReceipt: boolean;
}

/** Context handed to `subscribe` — re-issued per attach generation. */
export interface SubscriptionContext {
  readonly viewId: string;
  readonly windowId?: string;
  readonly generation: number;
}

/**
 * The consumer-side Command/Query gateway port — the single contract
 * both control-style (DAW) and query-style (analysis) consumers plug
 * into. Every member is optional except `dispatch`: a pure-read
 * consumer implements `query`/`cancelQuery`/`subscribe`; a pure-control
 * consumer implements `dispatch`/`subscribe`.
 *
 * Contract notes:
 * - `dispatch` must not throw for contract-valid commands; failures are
 *   reported as {@link CommandReceipt}s via the ack channel the
 *   consumer owns.
 * - `subscribe` should push the current projection immediately so a
 *   re-attaching view re-reads truth instead of reusing stale state.
 * - `query` must honor `signal` (reject on abort) and the `maxBytes`
 *   bound; `cancelQuery` is the explicit cancel channel for hosts whose
 *   wire protocol has one (`rvf.ipc.v1`-style).
 */
export interface BindingPort {
  dispatch(intent: CommandIntent, ctx: DispatchContext): void;
  subscribe?<P>(
    target: LogicalTarget,
    listener: ProjectionListener<P>,
    ctx: SubscriptionContext,
  ): Unsubscribe | undefined;
  query?<T>(query: ViewportQuery, signal?: AbortSignal): Promise<QueryReply<T>>;
  cancelQuery?(queryId: QueryId): void;
}
