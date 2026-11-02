/**
 * DOM input → headless {@link ControlInput} wiring (Issue #259).
 *
 * This module only translates DOM events into typed inputs and hands them
 * to a {@link GestureEndpoint}; every value resolution and every state
 * transition lives in the headless `InteractionControl` — presentation
 * never re-implements gesture semantics. A single-control element creates
 * one endpoint; the range selector composes two (one per edge).
 */

import type {
  ControlInput,
  InteractionControl,
  InteractionEvent,
  InputSource,
} from "./headless.ts";

/**
 * One gesture target: a headless control plus the sink that forwards the
 * emitted events (as `rvf-gesture` CustomEvents and visual state). The
 * control is read lazily so descriptor-driven controller rebuilds do not
 * leave stale references behind.
 */
export interface GestureEndpoint {
  control(): InteractionControl | undefined;
  forward(events: InteractionEvent[]): void;
  /**
   * Optional input transform applied before every dispatch — composite
   * controls use it to enforce invariants (e.g. range edges may not
   * cross) uniformly across pointer, wheel, and keyboard paths.
   */
  preprocess?(input: ControlInput): ControlInput;
  /** Called when a terminal (commit/cancel) event was processed. */
  onTerminal?(): void;
}

/** How pointer geometry resolves to a normalized position. */
export interface GestureGeometry {
  /**
   * `absolute`: pointer position inside the track maps to `[0,1]`
   * (fader/seekbar/range — click jumps, drag follows).
   * `relative`: vertical drag delta maps to a normalized offset of
   * `1/dragSpanPx` per pixel (knob — grab anywhere, drag to adjust).
   */
  readonly mode: "absolute" | "relative";
  /** `absolute` mode: event → normalized position in `[0,1]` (unclamped). */
  readonly normalizedFromPoint?: (e: PointerEvent) => number;
  /**
   * `absolute` mode, default `false`: when `true` (thumb surfaces), the
   * press does not jump the value — the pointer's offset from the
   * control value at grab time is preserved through the drag.
   */
  readonly grabOffset?: boolean;
  /** `absolute` mode: optional final clamp on the computed position. */
  readonly clamp?: (n: number) => number;
  /** `relative` mode: pixel distance covering the full `[0,1]` range. */
  readonly dragSpanPx?: number;
  /**
   * Element to focus on pointer press — `preventDefault` on
   * `pointerdown` suppresses the browser's default focus, so the
   * keyboard contract would otherwise be unreachable after a mouse
   * interaction.
   */
  readonly focusTarget?: () => HTMLElement | undefined;
}

const DEFAULT_DRAG_SPAN_PX = 150;
/** Wheel-burst gestures commit after this quiet period (bounded timer). */
export const WHEEL_IDLE_MS = 200;

/** Handle returned by {@link attachGestureInput}. */
export interface GestureBinding {
  /**
   * Full teardown: clear timers and remove listeners. Use when the
   * surface node is being replaced (re-render) — a persistent surface
   * would otherwise accumulate duplicate dispatch chains.
   */
  detach(): void;
  /**
   * Suspend only the timer-bound wheel-burst commit. Use on element
   * disconnect: listeners stay bound because their shadow nodes persist
   * across a remount, and removing them would leave the reconnected
   * control dead.
   */
  suspend(): void;
}

const pointerSource = (e: PointerEvent): InputSource =>
  e.pointerType === "touch" || e.pointerType === "pen"
    ? e.pointerType
    : "pointer";

/**
 * Attach pointer + wheel + double-click gesture wiring to a track element.
 * All listeners live on `el` itself. `detach()` removes them (surface
 * replacement); `suspend()` clears only the wheel timer (element
 * disconnect — shadow nodes and their listeners survive a remount).
 */
export function attachGestureInput(
  el: Element,
  ep: GestureEndpoint,
  geometry: GestureGeometry,
): GestureBinding {
  let activePointerId: number | undefined;
  let activeGestureId: string | undefined;
  let dragStartY = 0;
  let dragStartNormalized = 0;
  let grabPointerN = 0;
  let gestureSeq = 0;
  let wheelGestureId: string | undefined;
  let wheelTimer: ReturnType<typeof setTimeout> | undefined;

  // Gesture ids are minted per physical gesture — the contract rejects a
  // terminated id as `already-terminated`, so reusing `ptr-<id>` or
  // `wheel` would break every second drag/wheel burst on the control.

  const dispatch = (input: ControlInput): void => {
    const c = ep.control();
    if (c === undefined) return;
    const events = c.dispatch(ep.preprocess?.(input) ?? input);
    ep.forward(events);
    if (events.some((e) => e.type === "commit" || e.type === "cancel")) {
      ep.onTerminal?.();
    }
  };

  /** If `gestureId` is still active, terminate it (commit rejected etc.). */
  const cancelIfActive = (id: string, reason: string): void => {
    const c = ep.control();
    if (c !== undefined && c.getActiveGestureId() === id) {
      dispatch({ type: "cancel", gestureId: id, reason });
    }
  };

  const commitWheel = (): void => {
    wheelTimer = undefined;
    if (wheelGestureId !== undefined) {
      const id = wheelGestureId;
      wheelGestureId = undefined;
      dispatch({ type: "commit", gestureId: id });
      // A rejected commit leaves the gesture active per the contract;
      // the burst is already over, so the adapter cancels to terminate.
      cancelIfActive(id, "commit-rejected");
    }
  };

  const absoluteN = (e: PointerEvent): number => {
    const raw = geometry.normalizedFromPoint?.(e) ?? 0;
    const n =
      geometry.grabOffset === true
        ? dragStartNormalized + (raw - grabPointerN)
        : raw;
    return geometry.clamp?.(n) ?? n;
  };

  const onPointerDown = (e: PointerEvent): void => {
    if (e.button !== 0 && e.pointerType !== "touch" && e.pointerType !== "pen") {
      return;
    }
    // Single-gesture contract: a second pointer never takes over — the
    // first pointer keeps ownership or its gesture strands forever.
    if (activePointerId !== undefined) return;
    e.preventDefault();
    geometry.focusTarget?.()?.focus();
    // A pointer takeover closes any in-flight wheel burst first.
    if (wheelTimer !== undefined) {
      clearTimeout(wheelTimer);
      commitWheel();
    }
    const id = `ptr-${e.pointerId}-${++gestureSeq}`;
    const base = {
      type: "begin" as const,
      gestureId: id,
      source: pointerSource(e),
    };
    if (
      geometry.mode === "absolute" &&
      geometry.normalizedFromPoint !== undefined &&
      geometry.grabOffset !== true
    ) {
      dispatch({
        ...base,
        at: { normalized: absoluteN(e) },
      });
    } else {
      dispatch(base);
    }
    // Only track the pointer when the gesture was accepted — a rejected
    // or unavailable begin must not leak previews/commits afterward.
    const c = ep.control();
    if (c === undefined || c.getActiveGestureId() !== id) return;
    activePointerId = e.pointerId;
    activeGestureId = id;
    dragStartY = e.clientY;
    dragStartNormalized = c.getValue().normalized;
    grabPointerN = geometry.normalizedFromPoint?.(e) ?? 0;
    try {
      el.setPointerCapture(e.pointerId);
    } catch {
      // capture unsupported/unavailable — moves on `el` still arrive
    }
  };

  const onPointerMove = (e: PointerEvent): void => {
    if (activePointerId !== e.pointerId || activeGestureId === undefined)
      return;
    e.preventDefault();
    let n: number;
    if (geometry.mode === "absolute") {
      n = absoluteN(e);
    } else {
      const span = geometry.dragSpanPx ?? DEFAULT_DRAG_SPAN_PX;
      n = dragStartNormalized + (dragStartY - e.clientY) / span;
    }
    dispatch({
      type: "preview",
      gestureId: activeGestureId,
      value: { normalized: n },
    });
  };

  const onPointerUp = (e: PointerEvent): void => {
    if (activePointerId !== e.pointerId || activeGestureId === undefined)
      return;
    activePointerId = undefined;
    const id = activeGestureId;
    activeGestureId = undefined;
    dispatch({
      type: "commit",
      gestureId: id,
      ...(geometry.mode === "absolute"
        ? { value: { normalized: absoluteN(e) } }
        : {}),
    });
    // A rejected commit leaves the gesture active per the contract; the
    // pointer is already released, so the adapter cancels to terminate.
    cancelIfActive(id, "commit-rejected");
  };

  const endPointer = (e: PointerEvent, reason: "lost-pointer-capture") => {
    if (activePointerId !== e.pointerId || activeGestureId === undefined)
      return;
    activePointerId = undefined;
    const id = activeGestureId;
    activeGestureId = undefined;
    dispatch({
      type: "cancel",
      gestureId: id,
      reason,
    });
  };

  const onWheel = (e: WheelEvent): void => {
    if (e.deltaY === 0 || activePointerId !== undefined) return;
    e.preventDefault();
    const gid = (wheelGestureId ??= `wheel-${++gestureSeq}`);
    dispatch({
      type: "adjust",
      steps: e.deltaY < 0 ? 1 : -1,
      gestureId: gid,
      source: "wheel",
      modifiers: { fine: e.shiftKey },
    });
    if (wheelTimer !== undefined) clearTimeout(wheelTimer);
    wheelTimer = setTimeout(commitWheel, WHEEL_IDLE_MS);
  };

  const onDblClick = (e: MouseEvent): void => {
    e.preventDefault();
    dispatch({ type: "reset", source: "pointer" });
  };

  const listeners: Array<readonly [string, EventListener, AddEventListenerOptions?]> = [
    ["pointerdown", (e) => onPointerDown(e as PointerEvent)],
    ["pointermove", (e) => onPointerMove(e as PointerEvent)],
    ["pointerup", (e) => onPointerUp(e as PointerEvent)],
    ["pointercancel", (e) => endPointer(e as PointerEvent, "lost-pointer-capture")],
    ["lostpointercapture", (e) => endPointer(e as PointerEvent, "lost-pointer-capture")],
    ["wheel", (e) => onWheel(e as WheelEvent), { passive: false }],
    ["dblclick", (e) => onDblClick(e as MouseEvent)],
  ];
  for (const [type, fn, opts] of listeners) {
    el.addEventListener(type, fn, opts);
  }

  const clearWheel = (): void => {
    if (wheelTimer !== undefined) {
      clearTimeout(wheelTimer);
      wheelTimer = undefined;
    }
    // An uncommitted wheel gesture is terminated by the control detach
    // that follows suspend; drop its id so the next burst mints fresh.
    wheelGestureId = undefined;
  };

  return {
    suspend: clearWheel,
    // detach() removes listeners too — a persistent gesture surface
    // (e.g. the host itself in a custom skin) outlives rebinds and would
    // otherwise accumulate duplicate dispatch chains per refresh.
    detach: () => {
      clearWheel();
      for (const [type, fn] of listeners) el.removeEventListener(type, fn);
    },
  };
}

/**
 * Shared keyboard contract for value controls: Arrow keys ±1 effective
 * step (Shift = fine), PageUp/PageDown ±10, Home/End min/max, Escape
 * cancels an in-flight gesture. Each key press is a complete atomic
 * intent (begin+commit) so held-key repeat stays predictable.
 * Returns `true` when the key was consumed (caller preventDefaults).
 */
export function dispatchValueKey(
  e: KeyboardEvent,
  ep: GestureEndpoint,
): boolean {
  const c = ep.control();
  if (c === undefined) return false;
  const fine = e.shiftKey;
  let input: ControlInput | undefined;
  switch (e.key) {
    case "ArrowUp":
    case "ArrowRight":
      input = { type: "adjust", steps: 1, source: "keyboard", modifiers: { fine } };
      break;
    case "ArrowDown":
    case "ArrowLeft":
      input = { type: "adjust", steps: -1, source: "keyboard", modifiers: { fine } };
      break;
    case "PageUp":
      input = { type: "adjust", steps: 10, source: "keyboard", modifiers: { fine } };
      break;
    case "PageDown":
      input = { type: "adjust", steps: -10, source: "keyboard", modifiers: { fine } };
      break;
    case "Home":
      input = { type: "set", value: { extreme: "min" }, source: "keyboard" };
      break;
    case "End":
      input = { type: "set", value: { extreme: "max" }, source: "keyboard" };
      break;
    case "Escape":
      if (c.getActiveGestureId() === undefined) return false;
      input = {
        type: "cancel",
        gestureId: c.getActiveGestureId() ?? "",
        reason: "escape",
      };
      break;
    default:
      return false;
  }
  const events = c.dispatch(ep.preprocess?.(input) ?? input);
  ep.forward(events);
  if (events.some((ev) => ev.type === "commit" || ev.type === "cancel")) {
    ep.onTerminal?.();
  }
  return true;
}
