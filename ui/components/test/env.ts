/**
 * Headless DOM environment for `node --test` (Issue #259).
 *
 * happy-dom supplies the DOM the components run against — Custom
 * Elements, Shadow DOM, `::part`, composed CustomEvents, Pointer/
 * Keyboard events. It is a TEST-ONLY dependency: the shipped components
 * use nothing but standard browser APIs.
 *
 * Node globals we must NOT overwrite (Node's own implementations stay
 * authoritative): timers, `process`, `console`, `URL`, `structuredClone`.
 * Everything the component layer touches is installed explicitly below.
 */

import { Window } from "happy-dom";

const window = new Window({ url: "http://localhost/" });

const GLOBAL_KEYS = [
  "window",
  "self",
  "document",
  "customElements",
  "getComputedStyle",
  "Document",
  "DocumentFragment",
  "Node",
  "Element",
  "HTMLElement",
  "HTMLInputElement",
  "HTMLButtonElement",
  "SVGElement",
  "ShadowRoot",
  "MutationObserver",
  "Event",
  "EventTarget",
  "CustomEvent",
  "MouseEvent",
  "PointerEvent",
  "KeyboardEvent",
  "WheelEvent",
  "InputEvent",
  "FocusEvent",
  "CompositionEvent",
] as const;

const w = window as unknown as Record<string, unknown>;
const g = globalThis as unknown as Record<string, unknown>;

for (const key of GLOBAL_KEYS) {
  if (w[key] !== undefined) g[key] = w[key];
}

// Older happy-dom versions may not expose every event class; fall back
// to the closest available constructor so tests can still dispatch them.
g.PointerEvent ??= w.MouseEvent;
g.KeyboardEvent ??= w.Event;
g.CompositionEvent ??= w.Event;
g.FocusEvent ??= w.Event;
g.WheelEvent ??= w.Event;
g.InputEvent ??= w.Event;

export const dom = { window };

/** Fresh element of a registered tag, attached to the document. */
export function mount<K extends keyof HTMLElementTagNameMap | string>(
  tag: string,
  attrs: Record<string, string> = {},
): HTMLElement {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
  document.body.append(el);
  return el;
}

/** Detach an element (triggers disconnectedCallback → lifecycle cleanup). */
export function unmount(el: Element): void {
  el.remove();
}
