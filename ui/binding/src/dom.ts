/**
 * Issue #261 — the thin DOM binding adapter.
 *
 * This is the ONLY file that knows what a DOM-ish object looks like —
 * and only through the structural {@link EventTargetLike} /
 * {@link BoundControlLike} interfaces, so the module itself needs no
 * DOM lib and no framework. The adapter's job (Issue #257 §13):
 *
 * - forward `rvf-gesture` / `rvf-transport` CustomEvent intents into a
 *   {@link CommandBinding} — verbatim, never re-authorized or
 *   re-validated here (that is the consumer gateway's job),
 * - reflect {@link BindingViewState} back onto the element: `pending`
 *   while a receipt-expecting command is in flight, `invalid` on
 *   rejection/timeout, `unsupported` on declared incapability, and the
 *   controlled `value` write when an authoritative projection lands,
 * - revert the element to the last observed value on terminal failure
 *   (`revertOnReject`, default on) — a rejected or timed-out commit
 *   must not leave the failed intent posing as truth,
 * - stay loop-free: controlled `value` writes emit no intents (the
 *   #259 element contract), so UI → consumer → UI feedback cannot
 *   spawn duplicate commands; stale projections are already dropped
 *   upstream by the revision guard,
 * - stay idempotent: rebinding the same element+binding pair is a
 *   no-op, unbinding twice is a no-op, and unbind cancels the active
 *   gesture exactly once (the element's own detach may have already
 *   delivered it — the binding dedupes).
 *
 * Event-name constants mirror `ui/components/src/events.ts` — the
 * names are the published contract; re-declaring them here keeps this
 * package's source dependency-free (binding → core only).
 */

import type { InteractionEvent } from "../../core/src/index.ts";
import type { CommandBinding } from "./binding.ts";
import type { BindingViewState, StateProjection } from "./contract.ts";

/** `rvf-gesture` — detail is one headless `InteractionEvent`. */
export const RVF_GESTURE_EVENT = "rvf-gesture";
/** `rvf-transport` — detail is `{ action, controlId, sequence, source? }`. */
export const RVF_TRANSPORT_EVENT = "rvf-transport";
/** `rvf-viewport` — viz viewport intent (consumer-owned view state). */
export const RVF_VIEWPORT_EVENT = "rvf-viewport";
/** `rvf-select` — viz selection intent (consumer-owned view state). */
export const RVF_SELECT_EVENT = "rvf-select";
/** `rvf-backend` — viz backend/capability record. */
export const RVF_BACKEND_EVENT = "rvf-backend";

/** Structural minimum of `EventTarget` for this adapter. */
export interface DomEventLike {
  readonly type: string;
  readonly detail?: unknown;
}

export interface EventTargetLike {
  addEventListener(type: string, listener: (e: DomEventLike) => void): void;
  removeEventListener(type: string, listener: (e: DomEventLike) => void): void;
}

/**
 * Structural minimum of a boundable control element — the
 * `ui/components` `RvfControlElement` satisfies it, as do plain
 * framework wrappers that honour the same controlled-value contract.
 */
export interface BoundControlLike extends EventTargetLike {
  /** Controlled committed value — writes never emit intents. */
  value?: number;
  /** In-flight command display flag. */
  pending?: boolean;
  /** Rejected/invalid display flag. */
  invalid?: boolean;
  /** Unsupported-capability display flag. */
  unsupported?: boolean;
}

export interface TransportIntentLike {
  readonly action: string;
  readonly controlId?: string;
  readonly sequence?: number;
  readonly source?: string;
}

export type Unbind = () => void;

/** Default projection → element value: `ControlValue.plain`. */
function defaultProject<P>(p: StateProjection<P>): number | undefined {
  const v = p.value;
  if (typeof v === "number") return Number.isFinite(v) ? v : undefined;
  if (
    v !== null &&
    typeof v === "object" &&
    "plain" in (v as Record<string, unknown>)
  ) {
    const plain = (v as unknown as { plain: unknown }).plain;
    if (typeof plain === "number" && Number.isFinite(plain)) return plain;
  }
  return undefined;
}

export interface ControlAdapterOptions<P = unknown> {
  /** Map an accepted projection to the element's `value` domain.
   *  Default: `ControlValue.plain` (or a finite number). */
  readonly project?: (p: StateProjection<P>) => number | undefined;
  /** Write the last observed value back on reject/timeout/unsupported
   *  (default `true`) — the display never poses a failed intent as
   *  truth. */
  readonly revertOnReject?: boolean;
}

const bindings = new WeakMap<EventTargetLike, Map<object, Unbind>>();

function reflect<P>(
  el: BoundControlLike,
  s: BindingViewState<P>,
  opts: ControlAdapterOptions<P>,
  seen: { observedSeq: number },
): void {
  el.pending =
    s.phase === "pending" || s.phase === "accepted" || s.phase === "applied";
  el.unsupported = s.phase === "unsupported";
  el.invalid = s.phase === "rejected" || s.phase === "timeout";
  const project = opts.project ?? defaultProject;
  if (s.observedSeq !== seen.observedSeq) {
    seen.observedSeq = s.observedSeq;
    if (s.lastObserved !== undefined) {
      const v = project(s.lastObserved);
      if (v !== undefined) el.value = v;
    }
    return;
  }
  const failed =
    s.phase === "rejected" ||
    s.phase === "timeout" ||
    s.phase === "unsupported";
  if (failed && opts.revertOnReject !== false && s.lastObserved !== undefined) {
    const v = project(s.lastObserved);
    if (v !== undefined) el.value = v;
  }
}

/**
 * Bind one control element to a {@link CommandBinding}: `rvf-gesture`
 * intents flow to the binding; state reflections flow back. Idempotent
 * per (element, binding) pair — a StrictMode-style double-mount returns
 * the existing unbind instead of stacking listeners. The returned
 * unbind removes the listener, drops the state hook, and detaches the
 * binding (`unmount` → in-flight gesture cancelled exactly once).
 */
export function bindControlElement<P>(
  el: BoundControlLike,
  binding: CommandBinding<P>,
  options?: ControlAdapterOptions<P>,
): Unbind {
  const forEl = bindings.get(el);
  const existing = forEl?.get(binding);
  if (existing !== undefined) return existing;

  const opts: ControlAdapterOptions<P> = options ?? {};
  const seen = { observedSeq: -1 };
  const listener = (e: DomEventLike): void => {
    binding.event(e.detail as InteractionEvent);
  };
  // A prior unbind detached the binding — a fresh bind is the element
  // mounting again, so revive it (a no-op when already attached).
  binding.attach();
  el.addEventListener(RVF_GESTURE_EVENT, listener);
  const offState = binding.addStateListener((s) => reflect(el, s, opts, seen));
  reflect(el, binding.state, opts, seen);

  let live = true;
  const unbind: Unbind = () => {
    if (!live) return;
    live = false;
    el.removeEventListener(RVF_GESTURE_EVENT, listener);
    offState();
    binding.detach("unmount");
    bindings.get(el)?.delete(binding);
  };
  let map = bindings.get(el);
  if (map === undefined) {
    map = new Map();
    bindings.set(el, map);
  }
  map.set(binding, unbind);
  return unbind;
}

/**
 * Bind a transport/discrete element (`rvf-transport` intents) to a
 * binding — each activation becomes a non-droppable `action` command
 * tracked for a receipt.
 */
export function bindTransportElement<P>(
  el: EventTargetLike,
  binding: CommandBinding<P>,
): Unbind {
  const forEl = bindings.get(el);
  const existing = forEl?.get(binding);
  if (existing !== undefined) return existing;

  const listener = (e: DomEventLike): void => {
    const d = e.detail as TransportIntentLike | undefined;
    if (d === undefined || typeof d.action !== "string") return;
    binding.action(d.action, {
      ...(d.controlId !== undefined ? { controlId: d.controlId } : {}),
      ...(d.sequence !== undefined ? { sequence: d.sequence } : {}),
      ...(d.source !== undefined ? { source: d.source } : {}),
    });
  };
  el.addEventListener(RVF_TRANSPORT_EVENT, listener);

  let live = true;
  const unbind: Unbind = () => {
    if (!live) return;
    live = false;
    el.removeEventListener(RVF_TRANSPORT_EVENT, listener);
    bindings.get(el)?.delete(binding);
  };
  let map = bindings.get(el);
  if (map === undefined) {
    map = new Map();
    bindings.set(el, map);
  }
  map.set(binding, unbind);
  return unbind;
}

/** View-state intents a consumer listens to (`rvf-viewport` etc.) —
 *  these are not commands, just a typed listener bundle on the same
 *  thin adapter. */
export interface ViewIntentHandlers {
  readonly onViewport?: (detail: unknown) => void;
  readonly onSelect?: (detail: unknown) => void;
  readonly onBackend?: (detail: unknown) => void;
}

/** Attach typed listeners for the viz view-state events. Unbinding is
 *  idempotent; listeners are removed — never re-registered. */
export function bindViewIntents(
  el: EventTargetLike,
  handlers: ViewIntentHandlers,
): Unbind {
  const wired: Array<[string, (e: DomEventLike) => void]> = [];
  const add = (
    type: string,
    fn: ((detail: unknown) => void) | undefined,
  ): void => {
    if (fn === undefined) return;
    const listener = (e: DomEventLike): void => fn(e.detail);
    el.addEventListener(type, listener);
    wired.push([type, listener]);
  };
  add(RVF_VIEWPORT_EVENT, handlers.onViewport);
  add(RVF_SELECT_EVENT, handlers.onSelect);
  add(RVF_BACKEND_EVENT, handlers.onBackend);
  let live = true;
  return () => {
    if (!live) return;
    live = false;
    for (const [type, listener] of wired) {
      el.removeEventListener(type, listener);
    }
  };
}
