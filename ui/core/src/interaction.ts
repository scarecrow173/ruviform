/**
 * Issue #258 — headless gesture/interaction contract.
 *
 * The state machine is per control and deliberately small:
 *
 * ```text
 * Idle --begin--> Active --preview (0..N)--> Active
 *                  |--commit (exactly once)--> Idle
 *                  └--cancel (exactly once)--> Idle
 * ```
 *
 * Ownership rules (from the #257 design):
 * - `gestureId`, `controlId`, `target`, `originView` are opaque identifiers
 *   assigned by the consumer/adapter; this module only carries them.
 * - Begin fixes the gesture's target — mid-gesture focus/selection changes
 *   never retarget.
 * - Preview is a local tentative value. Commit is the user's *intent to
 *   finish*, not proof that a controller/engine applied the value.
 * - Cancel discards the uncommitted preview and never rewrites committed
 *   state.
 * - Consecutive `preview` events for the same control+gesture may be
 *   coalesced by a transport (`coalescible: true`); `begin`, `commit`,
 *   `cancel`, `unavailable` and `rejected` are non-droppable.
 * - DOM pointer capture, key handling, focus and unmount hooks belong to the
 *   presentation adapter (Issue #259); this module only accepts the typed
 *   inputs those hooks produce.
 */

import {
  ValueMapper,
  type ValueDescriptor,
  type ValueFailure,
  type ValuePorts,
} from "./value.ts";

/** Opaque identifier owned by the consumer/adapter. */
export type GestureId = string;
/** Opaque control identifier owned by the consumer/adapter. */
export type ControlId = string;
/** Opaque logical command target (e.g. a product parameter id). */
export type LogicalTarget = string;
/** Opaque identifier of the view a gesture originated from. */
export type OriginView = string;
/** Monotonic event sequence assigned by the controller. */
export type SequenceNumber = number;
/** Optional adapter-assigned input ordering token. */
export type InputSeq = number;

/** Where an input physically came from. Open union — consumers may extend. */
export type InputSource =
  | "pointer"
  | "touch"
  | "pen"
  | "wheel"
  | "keyboard"
  | "programmatic"
  | (string & {});

/** Why a gesture was cancelled. Open union — consumers may extend. */
export type CancelReason =
  | "escape"
  | "lost-focus"
  | "lost-pointer-capture"
  | "unmount"
  | "dock-transfer"
  | "explicit"
  | (string & {});

/** Input modifiers that change how a value is resolved. */
export interface InputModifiers {
  /** Fine-adjust (e.g. Shift): selects `fineStep` or the fine fallback. */
  readonly fine?: boolean;
}

/**
 * How an input specifies its value.
 * - `plain` / `normalized`: absolute value in that domain.
 * - `steps`: relative adjustment in effective step quanta (see
 *   {@link ValueMapper.adjustNormalized}); sign gives direction.
 * - `extreme`: jump to `"min"`, `"max"`, or the descriptor default.
 */
export type ValueInput =
  | { readonly plain: number }
  | { readonly normalized: number }
  | { readonly steps: number }
  | { readonly extreme: "min" | "max" | "default" };

/**
 * Typed inputs accepted by {@link InteractionControl.dispatch}.
 *
 * `set`, `adjust` and `reset` are value-producing shorthand inputs:
 * - without `gestureId` they are *atomic*: they emit a complete
 *   `begin`+`commit` pair (marked `atomic: true`) in one dispatch;
 * - with `gestureId` they auto-begin when the control is idle and then act
 *   as a `preview`, which is how wheel bursts and held keys form a single
 *   gesture that an adapter later closes with `commit` or `cancel`.
 */
export type ControlInput =
  | {
      readonly type: "begin";
      readonly gestureId: GestureId;
      readonly source: InputSource;
      readonly originView?: OriginView;
      readonly target?: LogicalTarget;
      readonly at?: ValueInput;
      readonly modifiers?: InputModifiers;
      readonly seq?: InputSeq;
    }
  | {
      readonly type: "preview";
      readonly gestureId: GestureId;
      readonly value: ValueInput;
      readonly modifiers?: InputModifiers;
      readonly seq?: InputSeq;
    }
  | {
      readonly type: "commit";
      readonly gestureId: GestureId;
      readonly value?: ValueInput;
      readonly modifiers?: InputModifiers;
      readonly seq?: InputSeq;
    }
  | {
      readonly type: "cancel";
      readonly gestureId: GestureId;
      readonly reason?: CancelReason;
      readonly seq?: InputSeq;
    }
  | {
      readonly type: "set";
      readonly value: ValueInput;
      readonly gestureId?: GestureId;
      readonly source?: InputSource;
      readonly modifiers?: InputModifiers;
      readonly seq?: InputSeq;
    }
  | {
      readonly type: "adjust";
      readonly steps: number;
      readonly gestureId?: GestureId;
      readonly source?: InputSource;
      readonly modifiers?: InputModifiers;
      readonly seq?: InputSeq;
    }
  | {
      readonly type: "reset";
      readonly gestureId?: GestureId;
      readonly source?: InputSource;
      readonly seq?: InputSeq;
    }
  | {
      readonly type: "focus-lost";
      /** When set, only terminates the gesture with this id (a mismatched
       * id no-ops, so a late focus event can't kill a newer gesture). */
      readonly gestureId?: GestureId;
      readonly seq?: InputSeq;
    }
  | {
      readonly type: "detach";
      /** When set, only terminates the gesture with this id. */
      readonly gestureId?: GestureId;
      readonly policy?: "cancel" | "commit";
      readonly reason?: "unmount" | "dock-transfer" | (string & {});
      readonly seq?: InputSeq;
    };

/** Event metadata carried on every emitted event. */
export interface InteractionEventMeta {
  readonly controlId: ControlId;
  readonly target?: LogicalTarget;
  readonly originView?: OriginView;
  readonly gestureId?: GestureId;
  /** Controller-assigned, strictly increasing per control, starting at 1. */
  readonly sequence: SequenceNumber;
  /** The input source that produced this event, when applicable. */
  readonly source?: InputSource;
  /** `true` on the synthesized begin/commit pair of an atomic input. */
  readonly atomic?: boolean;
}

/** A control value in both domains; always finite and in range. */
export interface ControlValue {
  readonly plain: number;
  readonly normalized: number;
}

/**
 * Explicit conversion port between a control value and a consumer-owned
 * parameter representation `T` (for example a plugin parameter type).
 * This package never depends on `ruviform-plugin-core::ParamValue`;
 * consumers implement this adapter where the binding lives.
 */
export interface ParamValueAdapter<T> {
  toParamValue(value: ControlValue): T;
  fromParamValue(param: T): ControlValue;
}

/** Explicit capability notification — distinct from benign `noop`. */
export type UnavailableReason = "disabled" | "readOnly" | "unsupported";

/** Contract-violating input — the input was rejected, state is unchanged. */
export type RejectReason =
  | "non-finite-input"
  | "out-of-range"
  | "invalid-mapping"
  | "invalid-input"
  | "gesture-already-active"
  | "stale-gesture"
  | "already-terminated"
  | "out-of-order"
  | "unknown-control";

/** Benign non-effect — distinct from `unavailable` and `rejected`. */
export type NoopReason = "no-active-gesture" | "duplicate-input" | "no-default";

/**
 * Events emitted by {@link InteractionControl}. `begin`, `commit`, `cancel`,
 * `unavailable` and `rejected` are non-droppable; consecutive `preview`
 * events for the same control+gesture are `coalescible` by a transport that
 * guarantees order within the gesture.
 */
export type InteractionEvent =
  | {
      readonly type: "begin";
      readonly meta: InteractionEventMeta;
      readonly value: ControlValue;
    }
  | {
      readonly type: "preview";
      readonly meta: InteractionEventMeta;
      readonly value: ControlValue;
      readonly coalescible: true;
    }
  | {
      readonly type: "commit";
      readonly meta: InteractionEventMeta;
      readonly value: ControlValue;
    }
  | {
      readonly type: "cancel";
      readonly meta: InteractionEventMeta;
      readonly reason: CancelReason;
    }
  | {
      readonly type: "unavailable";
      readonly meta: InteractionEventMeta;
      readonly reason: UnavailableReason;
    }
  | {
      readonly type: "rejected";
      readonly meta: InteractionEventMeta;
      readonly reason: RejectReason;
      readonly detail?: string;
    }
  | {
      readonly type: "noop";
      readonly meta: InteractionEventMeta;
      readonly reason: NoopReason;
    };

export type GesturePhase = "idle" | "active";

/** What happens to an in-flight gesture on `detach` (unmount/dock move). */
export type DetachPolicy = "cancel" | "commit";

/** Default number of terminated gesture ids retained for stale detection. */
export const DEFAULT_TERMINATED_HISTORY = 32;

export interface InteractionControlOptions {
  readonly controlId: ControlId;
  readonly descriptor: ValueDescriptor;
  readonly ports?: ValuePorts;
  /** Default logical target reported in event meta. */
  readonly target?: LogicalTarget;
  /** Default origin view reported in event meta. */
  readonly originView?: OriginView;
  /** `true` marks the control unsupported: every mutating input notifies. */
  readonly unsupported?: boolean;
  /** Terminal policy for `detach` when the input does not override it. */
  readonly detachPolicy?: DetachPolicy;
  /** Bound on the terminated-gesture-id memory used for stale detection. */
  readonly terminatedGestureHistory?: number;
}

interface GestureState {
  readonly gestureId: GestureId;
  readonly source: InputSource;
  target?: LogicalTarget;
  originView?: OriginView;
  preview: ControlValue;
}

interface MetaPartial {
  gestureId?: GestureId;
  source?: InputSource;
  atomic?: boolean;
  target?: LogicalTarget;
  originView?: OriginView;
}

type Availability = "available" | UnavailableReason;

const failureToReject = (f: ValueFailure): RejectReason =>
  f.kind === "non-finite"
    ? "non-finite-input"
    : f.kind === "out-of-range"
      ? "out-of-range"
      : "invalid-mapping";

type ValueResolution =
  | { readonly ok: true; readonly value: ControlValue }
  | { readonly ok: false; readonly error: ValueFailure };

/**
 * Headless interaction controller for one control. Owns the gesture state
 * machine, resolves inputs through its {@link ValueMapper}, and emits the
 * contract events. The controller deliberately survives DOM detach/reattach:
 * `detach` terminates the in-flight gesture per policy while the object
 * stays usable for an adapter that re-subscribes.
 */
export class InteractionControl {
  readonly controlId: ControlId;
  readonly mapper: ValueMapper;
  private availability: Availability;
  private readonly detachPolicy: DetachPolicy;
  private readonly historyBound: number;
  private readonly defaultTarget?: LogicalTarget;
  private readonly defaultOriginView?: OriginView;

  private phase: GesturePhase = "idle";
  private gesture: GestureState | undefined;
  private committedValue: ControlValue;
  private sequence = 0;
  private lastInputSeq: InputSeq | undefined;
  private atomicCounter = 0;
  private readonly terminated: GestureId[] = [];

  constructor(options: InteractionControlOptions) {
    this.controlId = options.controlId;
    this.mapper = new ValueMapper(options.descriptor, options.ports);
    this.availability =
      options.unsupported === true
        ? "unsupported"
        : options.descriptor.disabled === true
          ? "disabled"
          : options.descriptor.readOnly === true
            ? "readOnly"
            : "available";
    this.detachPolicy = options.detachPolicy ?? "cancel";
    this.historyBound =
      options.terminatedGestureHistory ?? DEFAULT_TERMINATED_HISTORY;
    if (options.target !== undefined) this.defaultTarget = options.target;
    if (options.originView !== undefined)
      this.defaultOriginView = options.originView;

    // The initial committed value is the descriptor default (or `min` when
    // none is declared) — the value a control presents before any input.
    const resolved = this.mapper.resolvePlain(this.mapper.defaultPlain());
    // A validated descriptor's default always resolves to a finite value.
    const initial = resolved.ok
      ? resolved
      : { normalized: 0, plain: options.descriptor.min };
    this.committedValue = {
      plain: initial.plain,
      normalized: initial.normalized,
    };
  }

  /** Current phase of the gesture state machine. */
  getPhase(): GesturePhase {
    return this.phase;
  }

  /** Gesture id of the in-flight gesture, if any. */
  getActiveGestureId(): GestureId | undefined {
    return this.gesture?.gestureId;
  }

  /** Last committed value — never modified by cancel. */
  getCommittedValue(): ControlValue {
    return this.committedValue;
  }

  /** Previewed value while active; the committed value while idle. */
  getValue(): ControlValue {
    return this.gesture?.preview ?? this.committedValue;
  }

  /**
   * Change capability flags at runtime. Making a control unavailable
   * mid-gesture cancels the in-flight gesture (`reason: "explicit"`) before
   * the flag takes effect; subsequent mutating inputs then emit
   * `unavailable`. Re-enabling restores normal dispatch.
   */
  setAvailability(flags: {
    disabled?: boolean;
    readOnly?: boolean;
    unsupported?: boolean;
  }): InteractionEvent[] {
    const next: Availability =
      flags.unsupported === true
        ? "unsupported"
        : flags.disabled === true
          ? "disabled"
          : flags.readOnly === true
            ? "readOnly"
            : "available";
    if (this.availability === next) return [];
    const events: InteractionEvent[] = [];
    if (next !== "available" && this.gesture !== undefined) {
      const g = this.gesture;
      this.endGesture(g);
      events.push({
        type: "cancel",
        meta: this.meta(this.gestureMeta(g)),
        reason: "explicit",
      });
    }
    this.availability = next;
    return events;
  }

  private nextSequence(): SequenceNumber {
    this.sequence += 1;
    return this.sequence;
  }

  private meta(p: MetaPartial): InteractionEventMeta {
    const meta: {
      controlId: ControlId;
      sequence: SequenceNumber;
      gestureId?: GestureId;
      source?: InputSource;
      atomic?: boolean;
      target?: LogicalTarget;
      originView?: OriginView;
    } = {
      controlId: this.controlId,
      sequence: this.nextSequence(),
    };
    if (p.gestureId !== undefined) meta.gestureId = p.gestureId;
    if (p.source !== undefined) meta.source = p.source;
    if (p.atomic !== undefined) meta.atomic = p.atomic;
    const target = p.target ?? this.defaultTarget;
    if (target !== undefined) meta.target = target;
    const originView = p.originView ?? this.defaultOriginView;
    if (originView !== undefined) meta.originView = originView;
    return meta;
  }

  private gestureMeta(g: GestureState, atomic?: boolean): MetaPartial {
    const p: MetaPartial = { gestureId: g.gestureId, source: g.source };
    if (g.target !== undefined) p.target = g.target;
    if (g.originView !== undefined) p.originView = g.originView;
    if (atomic !== undefined) p.atomic = atomic;
    return p;
  }

  private unavailable(
    reason: UnavailableReason,
    gestureId?: GestureId,
    source?: InputSource,
  ): InteractionEvent {
    const p: MetaPartial = {};
    if (gestureId !== undefined) p.gestureId = gestureId;
    if (source !== undefined) p.source = source;
    return { type: "unavailable", meta: this.meta(p), reason };
  }

  private rejected(
    reason: RejectReason,
    gestureId?: GestureId,
    detail?: string,
  ): InteractionEvent {
    const p: MetaPartial = {};
    if (gestureId !== undefined) p.gestureId = gestureId;
    const event: {
      type: "rejected";
      meta: InteractionEventMeta;
      reason: RejectReason;
      detail?: string;
    } = { type: "rejected", meta: this.meta(p), reason };
    if (detail !== undefined) event.detail = detail;
    return event;
  }

  private noop(reason: NoopReason, gestureId?: GestureId): InteractionEvent {
    const p: MetaPartial = {};
    if (gestureId !== undefined) p.gestureId = gestureId;
    return { type: "noop", meta: this.meta(p), reason };
  }

  private endGesture(g: GestureState): void {
    this.gesture = undefined;
    this.phase = "idle";
    this.terminated.push(g.gestureId);
    if (this.terminated.length > this.historyBound) {
      this.terminated.splice(0, this.terminated.length - this.historyBound);
    }
  }

  private resolveValueInput(
    input: ValueInput,
    from: ControlValue,
    fine: boolean,
  ): ValueResolution {
    if ("plain" in input) {
      const r = this.mapper.resolvePlain(input.plain);
      return r.ok
        ? { ok: true, value: { plain: r.plain, normalized: r.normalized } }
        : r;
    }
    if ("normalized" in input) {
      const r = this.mapper.resolveNormalized(input.normalized);
      return r.ok
        ? { ok: true, value: { plain: r.plain, normalized: r.normalized } }
        : r;
    }
    if ("steps" in input) {
      const r = this.mapper.adjustNormalized(
        from.normalized,
        input.steps,
        fine,
      );
      return r.ok
        ? { ok: true, value: { plain: r.plain, normalized: r.normalized } }
        : r;
    }
    const target =
      input.extreme === "min"
        ? this.mapper.descriptor.min
        : input.extreme === "max"
          ? this.mapper.descriptor.max
          : this.mapper.defaultPlain();
    const r = this.mapper.resolvePlain(target);
    return r.ok
      ? { ok: true, value: { plain: r.plain, normalized: r.normalized } }
      : r;
  }

  private checkSeq(seq: InputSeq | undefined): InteractionEvent | undefined {
    if (seq === undefined) return undefined;
    if (this.lastInputSeq !== undefined) {
      if (seq === this.lastInputSeq) return this.noop("duplicate-input");
      if (seq < this.lastInputSeq) {
        return this.rejected(
          "out-of-order",
          undefined,
          `input seq ${seq} < last seen ${this.lastInputSeq}`,
        );
      }
    }
    this.lastInputSeq = seq;
    return undefined;
  }

  /**
   * Lifecycle inputs (`focus-lost`, `detach`) are exempt from the seq drop
   * rule: a late-delivered lifecycle event must still be evaluated, or a
   * gesture could outlive the view that owns it. The input sequence is
   * still consumed when newer so ordering information is preserved.
   */
  private checkSeqLifecycle(seq: InputSeq | undefined): void {
    if (seq === undefined) return;
    if (this.lastInputSeq === undefined || seq > this.lastInputSeq) {
      this.lastInputSeq = seq;
    }
  }

  private checkAvailability(
    gestureId?: GestureId,
    source?: InputSource,
  ): InteractionEvent | undefined {
    if (this.availability !== "available") {
      return this.unavailable(this.availability, gestureId, source);
    }
    return undefined;
  }

  /** Route a gestureId-carrying input by gesture state. */
  private gestureFor(
    gestureId: GestureId,
  ):
    | { state: "active"; g: GestureState }
    | { state: "stale" }
    | { state: "none" } {
    if (this.gesture !== undefined && this.gesture.gestureId === gestureId) {
      return { state: "active", g: this.gesture };
    }
    if (this.terminated.includes(gestureId)) return { state: "stale" };
    return { state: "none" };
  }

  private autoGestureId(): GestureId {
    this.atomicCounter += 1;
    return `atomic-${this.atomicCounter}`;
  }

  /**
   * Dispatch one typed input and return the emitted events in emission
   * order. Every input currently produces at least one event — including
   * explicit `unavailable` / `noop` / `rejected` notifications — so a
   * transport can observe that an input arrived and what happened to it.
   */
  dispatch(input: ControlInput): InteractionEvent[] {
    if (input.type === "focus-lost" || input.type === "detach") {
      this.checkSeqLifecycle(input.seq);
      return input.type === "focus-lost"
        ? this.dispatchFocusLost(input)
        : this.dispatchDetach(input);
    }

    const seqEvent = this.checkSeq(input.seq);
    if (seqEvent !== undefined) return [seqEvent];

    switch (input.type) {
      case "begin":
        return this.dispatchBegin(input);
      case "preview":
        return this.dispatchPreview(input);
      case "commit":
        return this.dispatchCommit(input);
      case "cancel":
        return this.dispatchCancel(input);
      case "set":
        return this.dispatchSetLike(input, input.value);
      case "adjust":
        return this.dispatchSetLike(input, { steps: input.steps });
      case "reset":
        return this.dispatchSetLike(input, { extreme: "default" });
    }
  }

  private dispatchBegin(
    input: Extract<ControlInput, { type: "begin" }>,
  ): InteractionEvent[] {
    const blocked = this.checkAvailability(input.gestureId, input.source);
    if (blocked !== undefined) return [blocked];

    const slot = this.gestureFor(input.gestureId);
    if (slot.state === "active") {
      return [
        this.rejected(
          "gesture-already-active",
          input.gestureId,
          `gesture ${input.gestureId} is already active`,
        ),
      ];
    }
    if (slot.state === "stale") {
      return [
        this.rejected(
          "already-terminated",
          input.gestureId,
          `gesture ${input.gestureId} was already terminated`,
        ),
      ];
    }
    if (this.gesture !== undefined) {
      // A different gesture is already active on this control: reject the
      // newcomer explicitly; the in-flight gesture continues untouched.
      return [
        this.rejected(
          "gesture-already-active",
          input.gestureId,
          `control ${this.controlId} already has active gesture ${this.gesture.gestureId}`,
        ),
      ];
    }

    let initial = this.committedValue;
    if (input.at !== undefined) {
      const r = this.resolveValueInput(
        input.at,
        this.committedValue,
        input.modifiers?.fine === true,
      );
      if (!r.ok) {
        return [
          this.rejected(
            failureToReject(r.error),
            input.gestureId,
            r.error.detail,
          ),
        ];
      }
      initial = r.value;
    }

    const g: GestureState = {
      gestureId: input.gestureId,
      source: input.source,
      preview: initial,
    };
    if (input.target !== undefined) g.target = input.target;
    if (input.originView !== undefined) g.originView = input.originView;
    this.gesture = g;
    this.phase = "active";
    return [
      { type: "begin", meta: this.meta(this.gestureMeta(g)), value: g.preview },
    ];
  }

  private dispatchPreview(
    input: Extract<ControlInput, { type: "preview" }>,
  ): InteractionEvent[] {
    const blocked = this.checkAvailability(input.gestureId);
    if (blocked !== undefined) return [blocked];

    const slot = this.gestureFor(input.gestureId);
    if (slot.state === "none") {
      return [this.noop("no-active-gesture", input.gestureId)];
    }
    if (slot.state === "stale") {
      return [
        this.rejected(
          "stale-gesture",
          input.gestureId,
          `preview for terminated gesture ${input.gestureId}`,
        ),
      ];
    }
    const r = this.resolveValueInput(
      input.value,
      slot.g.preview,
      input.modifiers?.fine === true,
    );
    if (!r.ok) {
      // Invalid preview input is rejected without disturbing the gesture.
      return [
        this.rejected(
          failureToReject(r.error),
          input.gestureId,
          r.error.detail,
        ),
      ];
    }
    slot.g.preview = r.value;
    return [
      {
        type: "preview",
        meta: this.meta(this.gestureMeta(slot.g)),
        value: slot.g.preview,
        coalescible: true,
      },
    ];
  }

  private dispatchCommit(
    input: Extract<ControlInput, { type: "commit" }>,
  ): InteractionEvent[] {
    const blocked = this.checkAvailability(input.gestureId);
    if (blocked !== undefined) return [blocked];

    const slot = this.gestureFor(input.gestureId);
    if (slot.state === "none") {
      return [this.noop("no-active-gesture", input.gestureId)];
    }
    if (slot.state === "stale") {
      return [
        this.rejected(
          "already-terminated",
          input.gestureId,
          `commit for terminated gesture ${input.gestureId}`,
        ),
      ];
    }
    if (input.value !== undefined) {
      const r = this.resolveValueInput(
        input.value,
        slot.g.preview,
        input.modifiers?.fine === true,
      );
      if (!r.ok) {
        // Strict contract: an invalid commit value is rejected and the
        // gesture stays active — the adapter may retry commit without a
        // value or cancel.
        return [
          this.rejected(
            failureToReject(r.error),
            input.gestureId,
            r.error.detail,
          ),
        ];
      }
      slot.g.preview = r.value;
    }
    const g = slot.g;
    this.committedValue = g.preview;
    this.endGesture(g);
    return [
      { type: "commit", meta: this.meta(this.gestureMeta(g)), value: g.preview },
    ];
  }

  private dispatchCancel(
    input: Extract<ControlInput, { type: "cancel" }>,
  ): InteractionEvent[] {
    const slot = this.gestureFor(input.gestureId);
    if (slot.state === "none") {
      return [this.noop("no-active-gesture", input.gestureId)];
    }
    if (slot.state === "stale") {
      return [
        this.rejected(
          "already-terminated",
          input.gestureId,
          `cancel for terminated gesture ${input.gestureId}`,
        ),
      ];
    }
    const g = slot.g;
    this.endGesture(g);
    // Committed state is intentionally untouched by cancel.
    return [
      {
        type: "cancel",
        meta: this.meta(this.gestureMeta(g)),
        reason: input.reason ?? "explicit",
      },
    ];
  }

  /**
   * `set`/`adjust`/`reset` handling. The value is resolved first, so an
   * invalid input is a plain `rejected` and never half-starts a gesture.
   */
  private dispatchSetLike(
    input: Extract<ControlInput, { type: "set" | "adjust" | "reset" }>,
    value: ValueInput,
  ): InteractionEvent[] {
    const source = input.source ?? "programmatic";
    const fine =
      "modifiers" in input && input.modifiers?.fine === true;
    const blocked = this.checkAvailability(input.gestureId, source);
    if (blocked !== undefined) return [blocked];

    if (
      "extreme" in value &&
      value.extreme === "default" &&
      !this.mapper.hasDefault()
    ) {
      return [this.noop("no-default", input.gestureId)];
    }

    if (input.gestureId === undefined) {
      if (this.gesture !== undefined) {
        return [
          this.rejected(
            "gesture-already-active",
            undefined,
            `control ${this.controlId} already has active gesture ${this.gesture.gestureId}`,
          ),
        ];
      }
      const r = this.resolveValueInput(value, this.committedValue, fine);
      if (!r.ok) {
        return [
          this.rejected(failureToReject(r.error), undefined, r.error.detail),
        ];
      }
      const g: GestureState = {
        gestureId: this.autoGestureId(),
        source,
        preview: r.value,
      };
      if (this.defaultTarget !== undefined) g.target = this.defaultTarget;
      const begin: InteractionEvent = {
        type: "begin",
        meta: this.meta(this.gestureMeta(g, true)),
        value: g.preview,
      };
      this.committedValue = g.preview;
      this.endGesture(g);
      const commit: InteractionEvent = {
        type: "commit",
        meta: this.meta(this.gestureMeta(g, true)),
        value: g.preview,
      };
      return [begin, commit];
    }

    const slot = this.gestureFor(input.gestureId);
    if (slot.state === "stale") {
      return [
        this.rejected(
          "already-terminated",
          input.gestureId,
          `input for terminated gesture ${input.gestureId}`,
        ),
      ];
    }
    if (slot.state === "none" && this.gesture !== undefined) {
      return [
        this.rejected(
          "gesture-already-active",
          input.gestureId,
          `control ${this.controlId} already has active gesture ${this.gesture.gestureId}`,
        ),
      ];
    }

    const base =
      slot.state === "active" ? slot.g.preview : this.committedValue;
    const r = this.resolveValueInput(value, base, fine);
    if (!r.ok) {
      return [
        this.rejected(
          failureToReject(r.error),
          input.gestureId,
          r.error.detail,
        ),
      ];
    }

    const events: InteractionEvent[] = [];
    let g: GestureState;
    if (slot.state === "none") {
      g = {
        gestureId: input.gestureId,
        source,
        preview: this.committedValue,
      };
      this.gesture = g;
      this.phase = "active";
      events.push({
        type: "begin",
        meta: this.meta(this.gestureMeta(g)),
        value: g.preview,
      });
    } else {
      g = slot.g;
    }
    g.preview = r.value;
    events.push({
      type: "preview",
      meta: this.meta(this.gestureMeta(g)),
      value: g.preview,
      coalescible: true,
    });
    return events;
  }

  private dispatchFocusLost(
    input: Extract<ControlInput, { type: "focus-lost" }>,
  ): InteractionEvent[] {
    if (this.gesture === undefined) return [this.noop("no-active-gesture")];
    if (
      input.gestureId !== undefined &&
      this.gesture.gestureId !== input.gestureId
    ) {
      return [this.noop("no-active-gesture", input.gestureId)];
    }
    const g = this.gesture;
    this.endGesture(g);
    return [
      {
        type: "cancel",
        meta: this.meta(this.gestureMeta(g)),
        reason: "lost-focus",
      },
    ];
  }

  private dispatchDetach(
    input: Extract<ControlInput, { type: "detach" }>,
  ): InteractionEvent[] {
    if (this.gesture === undefined) return [this.noop("no-active-gesture")];
    if (
      input.gestureId !== undefined &&
      this.gesture.gestureId !== input.gestureId
    ) {
      return [this.noop("no-active-gesture", input.gestureId)];
    }
    const g = this.gesture;
    const policy = input.policy ?? this.detachPolicy;
    if (policy === "commit") {
      this.committedValue = g.preview;
      this.endGesture(g);
      return [
        {
          type: "commit",
          meta: this.meta(this.gestureMeta(g)),
          value: g.preview,
        },
      ];
    }
    this.endGesture(g);
    return [
      {
        type: "cancel",
        meta: this.meta(this.gestureMeta(g)),
        reason: input.reason ?? "unmount",
      },
    ];
  }
}

/** Registration record for {@link InteractionDispatcher}. */
export type ControlRegistration = InteractionControlOptions;

/**
 * Owns the {@link InteractionControl}s of a view/session and routes inputs
 * by `controlId`. Re-registering an existing `controlId` detaches the old
 * control first (in-flight gestures end per its detach policy); the
 * resulting events are returned so a transport can deliver them.
 */
export class InteractionDispatcher {
  private readonly controls = new Map<ControlId, InteractionControl>();
  private syntheticSeq = 0;

  register(options: ControlRegistration): InteractionEvent[] {
    const events = this.unregister(options.controlId);
    this.controls.set(options.controlId, new InteractionControl(options));
    return events;
  }

  unregister(controlId: ControlId): InteractionEvent[] {
    const existing = this.controls.get(controlId);
    if (existing === undefined) return [];
    this.controls.delete(controlId);
    return existing.dispatch({ type: "detach", reason: "unmount" });
  }

  control(controlId: ControlId): InteractionControl | undefined {
    return this.controls.get(controlId);
  }

  /**
   * Dispatch an input to a control. Unknown `controlId` is reported as
   * `rejected` `unknown-control` rather than dropped silently.
   */
  dispatch(controlId: ControlId, input: ControlInput): InteractionEvent[] {
    const c = this.controls.get(controlId);
    if (c === undefined) {
      this.syntheticSeq += 1;
      return [
        {
          type: "rejected",
          meta: { controlId, sequence: this.syntheticSeq },
          reason: "unknown-control",
          detail: `no control registered for ${controlId}`,
        },
      ];
    }
    return c.dispatch(input);
  }
}
