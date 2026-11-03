/** Shared test helpers: gesture collection, DOM events, async flush. */

import {
  RVF_GESTURE_EVENT,
  type InteractionEvent,
} from "../../components/src/index.ts";

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

/** Give an element a usable geometry (happy-dom returns zero rects). */
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

/** Query an element's shadow root for a `part` token. */
export function part(el: Element, name: string): Element | null {
  const root = (el as { shadowRoot?: ShadowRoot }).shadowRoot;
  return root?.querySelector(`[part~="${name}"]`) ?? null;
}

/** Flush the microtask queue twice (binding coalescing + async source
 *  query resolution both settle on microtasks). */
export async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

/** Synchronous flush — every binding push dispatches immediately. */
export const flushSync = (fn: () => void): void => fn();

/**
 * Deterministic structural fingerprint of a subtree — the DOM-level
 * snapshot baseline. Covers the semantic surface only: tag name,
 * `part`, `role`, `aria-*`, `data-*` markers, and text content.
 * Class names, inline styles, and private nesting are deliberately
 * excluded (they are not the public contract).
 */
export function domFingerprint(el: Element): string {
  const lines: string[] = [];
  const walk = (node: Element | ShadowRoot, depth: number): void => {
    if (node instanceof ShadowRoot) {
      for (const child of node.children) walk(child, depth);
      return;
    }
    // Stylesheets are internals, not the semantic contract.
    if (node.localName === "style" || node.localName === "script") return;
    const keep = (name: string): boolean =>
      name === "part" ||
      name === "role" ||
      name === "tabindex" ||
      name === "type" ||
      name === "name" ||
      name === "slot" ||
      name.startsWith("aria-") ||
      name.startsWith("data-");
    const attrs = node
      .getAttributeNames()
      .filter(keep)
      .sort()
      .map((n) => `${n}=${node.getAttribute(n) ?? ""}`)
      .join(" ");
    const text =
      node.children.length === 0 && node.textContent !== null
        ? node.textContent.trim()
        : "";
    lines.push(
      `${"  ".repeat(depth)}<${node.localName}${attrs === "" ? "" : ` ${attrs}`}>${text}`,
    );
    const shadow = (node as Element & { shadowRoot?: ShadowRoot })
      .shadowRoot;
    if (shadow !== undefined && shadow !== null) walk(shadow, depth + 1);
    for (const child of node.children) walk(child, depth + 1);
  };
  walk(el, 0);
  return lines.join("\n");
}
