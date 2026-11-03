/**
 * Headless DOM environment for `node --test` (Issue #261).
 *
 * Same discipline as ui/components + ui/viz: happy-dom supplies the DOM
 * the DOM-adapter tests run against — Custom Elements, composed
 * CustomEvents. It is a TEST-ONLY dependency: nothing under `src/`
 * imports it, and the src tree needs no DOM at all (the adapter works
 * on structural `EventTargetLike` objects).
 *
 * Node globals we must NOT overwrite (Node's own implementations stay
 * authoritative): timers, `process`, `console`, `URL`, `structuredClone`.
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
export function mount(
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
