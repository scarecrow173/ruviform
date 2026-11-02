/** Shared test helpers: gesture simulation + event collection. */

import type { InteractionEvent } from "../src/headless.ts";
import { RVF_GESTURE_EVENT } from "../src/events.ts";
import type { RvfElement } from "../src/element.ts";

/** Collect every `rvf-gesture` event detail seen on an element. */
export function collectGestures(el: Element): {
  events: InteractionEvent[];
  stop(): void;
} {
  const events: InteractionEvent[] = [];
  const listener = (e: Event) => {
    events.push((e as CustomEvent<InteractionEvent>).detail);
  };
  el.addEventListener(RVF_GESTURE_EVENT, listener);
  return { events, stop: () => el.removeEventListener(RVF_GESTURE_EVENT, listener) };
}

export function typesOf(events: readonly InteractionEvent[]): string[] {
  return events.map((e) => e.type);
}

/** PointerEvent init for the headless DOM. */
export function ptr(
  type: string,
  init: {
    clientX?: number;
    clientY?: number;
    pointerId?: number;
    pointerType?: string;
    button?: number;
    buttons?: number;
    bubbles?: boolean;
  } = {},
): Event {
  return new PointerEvent(type, {
    bubbles: init.bubbles ?? true,
    composed: true,
    clientX: init.clientX ?? 0,
    clientY: init.clientY ?? 0,
    pointerId: init.pointerId ?? 1,
    pointerType: init.pointerType ?? "mouse",
    button: init.button ?? 0,
    buttons: init.buttons ?? 1,
  });
}

export function key(
  type: string,
  init: { key: string; shiftKey?: boolean; bubbles?: boolean },
): Event {
  return new KeyboardEvent(type, {
    key: init.key,
    shiftKey: init.shiftKey ?? false,
    bubbles: init.bubbles ?? true,
    composed: true,
  });
}

/**
 * Give a part a usable geometry: happy-dom returns a zero rect, so
 * absolute-geometry controls need a real rectangle stubbed in.
 */
export function stubRect(
  el: Element | null | undefined,
  rect: { left?: number; top?: number; width?: number; height?: number } = {},
): void {
  if (el === null || el === undefined) return;
  const r = {
    left: rect.left ?? 0,
    top: rect.top ?? 0,
    width: rect.width ?? 100,
    height: rect.height ?? 100,
    right: (rect.left ?? 0) + (rect.width ?? 100),
    bottom: (rect.top ?? 0) + (rect.height ?? 100),
    x: rect.left ?? 0,
    y: rect.top ?? 0,
    toJSON: () => ({}),
  };
  (el as HTMLElement).getBoundingClientRect = () => r as DOMRect;
}

/** Query the shadow root of a custom element. */
export function part(el: RvfElement | Element, name: string): Element | null {
  const root = (el as { shadowRoot?: ShadowRoot }).shadowRoot;
  return root?.querySelector(`[part~="${name}"]`) ?? null;
}
