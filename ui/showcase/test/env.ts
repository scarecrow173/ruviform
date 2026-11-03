/**
 * Headless DOM environment for `@ruviform/ui-showcase` tests
 * (`node --test`).
 *
 * Same contract as `ui/viz/test/env.ts` (Issue #260): happy-dom
 * supplies Custom Elements / Shadow DOM / events — a TEST-ONLY
 * dependency. happy-dom does not implement `HTMLCanvasElement`
 * 2D/WebGL contexts — `installFakeCanvas` stubs `getContext` with
 * recording fake contexts so viz backend selection, op execution, and
 * fallback paths run headlessly (no GPU claims — see package docs).
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
  "HTMLCanvasElement",
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
  "AbortController",
  "DOMException",
  "queueMicrotask",
] as const;

const w = window as unknown as Record<string, unknown>;
const g = globalThis as unknown as Record<string, unknown>;

for (const key of GLOBAL_KEYS) {
  if (w[key] !== undefined) g[key] = w[key];
}

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

/** Detach an element (triggers disconnectedCallback → cleanup). */
export function unmount(el: Element): void {
  el.remove();
}

// ── fake canvas contexts ────────────────────────────────────────────

export interface RecordedCall {
  readonly method: string;
  readonly args: readonly unknown[];
}

/** A recording 2D-context fake — implements the API surface the
 * Canvas2DExecutor touches; every call is logged for assertions. */
export class Fake2DContext {
  readonly calls: RecordedCall[] = [];
  private fillStyle_: unknown = "";
  private strokeStyle_: unknown = "";
  lineWidth: unknown = 1;
  font: unknown = "";
  textAlign: unknown = "";
  textBaseline: unknown = "";

  get fillStyle(): unknown {
    return this.fillStyle_;
  }
  set fillStyle(v: unknown) {
    this.fillStyle_ = v;
    this.rec("set:fillStyle", v);
  }
  get strokeStyle(): unknown {
    return this.strokeStyle_;
  }
  set strokeStyle(v: unknown) {
    this.strokeStyle_ = v;
    this.rec("set:strokeStyle", v);
  }

  private rec(method: string, ...args: unknown[]): void {
    this.calls.push({ method, args });
  }

  /** Distinct values assigned to `fillStyle` (theme-change evidence). */
  get fillStyles(): readonly unknown[] {
    return this.calls
      .filter((c) => c.method === "set:fillStyle")
      .map((c) => c.args[0]);
  }

  save(): void {
    this.rec("save");
  }
  restore(): void {
    this.rec("restore");
  }
  setTransform(...a: unknown[]): void {
    this.rec("setTransform", ...a);
  }
  clearRect(...a: unknown[]): void {
    this.rec("clearRect", ...a);
  }
  fillRect(...a: unknown[]): void {
    this.rec("fillRect", ...a);
  }
  strokeRect(...a: unknown[]): void {
    this.rec("strokeRect", ...a);
  }
  beginPath(): void {
    this.rec("beginPath");
  }
  closePath(): void {
    this.rec("closePath");
  }
  moveTo(...a: unknown[]): void {
    this.rec("moveTo", ...a);
  }
  lineTo(...a: unknown[]): void {
    this.rec("lineTo", ...a);
  }
  rect(...a: unknown[]): void {
    this.rec("rect", ...a);
  }
  clip(): void {
    this.rec("clip");
  }
  fill(): void {
    this.rec("fill");
  }
  stroke(): void {
    this.rec("stroke");
  }
  fillText(...a: unknown[]): void {
    this.rec("fillText", ...a);
  }

  count(method: string): number {
    return this.calls.filter((c) => c.method === method).length;
  }
}

/** Minimal WebGL2 fake — records calls, reports shader/program status
 * success, exposes the constants the executor reads. */
export class FakeGL {
  readonly calls: RecordedCall[] = [];
  private nextId = 1;

  constructor() {
    const names = [
      "TRIANGLES",
      "TRIANGLE_STRIP",
      "TRIANGLE_FAN",
      "LINES",
      "ARRAY_BUFFER",
      "DYNAMIC_DRAW",
      "VERTEX_SHADER",
      "FRAGMENT_SHADER",
      "COMPILE_STATUS",
      "LINK_STATUS",
      "DEPTH_TEST",
      "BLEND",
      "SCISSOR_TEST",
      "SRC_ALPHA",
      "ONE_MINUS_SRC_ALPHA",
      "COLOR_BUFFER_BIT",
      "FLOAT",
    ];
    names.forEach((n, i) => {
      (this as Record<string, unknown>)[n] = i + 1;
    });
  }

  private rec(method: string, ...args: unknown[]): unknown {
    this.calls.push({ method, args });
    return undefined;
  }

  createShader(): object {
    this.rec("createShader");
    return { id: this.nextId++ };
  }
  shaderSource(...a: unknown[]): void {
    this.rec("shaderSource", ...a);
  }
  compileShader(...a: unknown[]): void {
    this.rec("compileShader", ...a);
  }
  getShaderParameter(): boolean {
    this.rec("getShaderParameter");
    return true;
  }
  getShaderInfoLog(): string {
    return "";
  }
  createProgram(): object {
    this.rec("createProgram");
    return { id: this.nextId++ };
  }
  attachShader(...a: unknown[]): void {
    this.rec("attachShader", ...a);
  }
  linkProgram(...a: unknown[]): void {
    this.rec("linkProgram", ...a);
  }
  getProgramParameter(): boolean {
    this.rec("getProgramParameter");
    return true;
  }
  getProgramInfoLog(): string {
    return "";
  }
  useProgram(...a: unknown[]): void {
    this.rec("useProgram", ...a);
  }
  createBuffer(): object {
    this.rec("createBuffer");
    return { id: this.nextId++ };
  }
  bindBuffer(...a: unknown[]): void {
    this.rec("bindBuffer", ...a);
  }
  bufferData(...a: unknown[]): void {
    this.rec("bufferData", ...a);
  }
  getAttribLocation(): number {
    this.rec("getAttribLocation");
    return 0;
  }
  getUniformLocation(): object {
    this.rec("getUniformLocation");
    return { id: this.nextId++ };
  }
  vertexAttribPointer(...a: unknown[]): void {
    this.rec("vertexAttribPointer", ...a);
  }
  enableVertexAttribArray(...a: unknown[]): void {
    this.rec("enableVertexAttribArray", ...a);
  }
  uniform2f(...a: unknown[]): void {
    this.rec("uniform2f", ...a);
  }
  uniform4f(...a: unknown[]): void {
    this.rec("uniform4f", ...a);
  }
  viewport(...a: unknown[]): void {
    this.rec("viewport", ...a);
  }
  scissor(...a: unknown[]): void {
    this.rec("scissor", ...a);
  }
  enable(...a: unknown[]): void {
    this.rec("enable", ...a);
  }
  disable(...a: unknown[]): void {
    this.rec("disable", ...a);
  }
  blendFunc(...a: unknown[]): void {
    this.rec("blendFunc", ...a);
  }
  clearColor(...a: unknown[]): void {
    this.rec("clearColor", ...a);
  }
  clear(...a: unknown[]): void {
    this.rec("clear", ...a);
  }
  drawArrays(...a: unknown[]): void {
    this.rec("drawArrays", ...a);
  }

  count(method: string): number {
    return this.calls.filter((c) => c.method === method).length;
  }
}

export interface FakeCanvasOptions {
  /** `"2d"`, `"webgl2"`, `"both"`, or `"none"`. */
  readonly contexts?: "2d" | "webgl2" | "both" | "none";
}

export interface FakeCanvas {
  readonly contexts: Map<string, object>;
  restore(): void;
}

/**
 * Stub `HTMLCanvasElement.prototype.getContext` for the test file's
 * duration. Returns handles to inspect the created contexts.
 */
export function installFakeCanvas(opts: FakeCanvasOptions = {}): FakeCanvas {
  const mode = opts.contexts ?? "both";
  const contexts = new Map<string, object>();
  const proto = (
    window as unknown as {
      HTMLCanvasElement: { prototype: HTMLCanvasElement };
    }
  ).HTMLCanvasElement.prototype;
  const original = proto.getContext;
  proto.getContext = function (
    this: HTMLCanvasElement,
    kind: string,
  ): object | null {
    const want =
      (mode === "both" && (kind === "2d" || kind === "webgl2")) ||
      (mode === "2d" && kind === "2d") ||
      (mode === "webgl2" && kind === "webgl2");
    if (!want) return null;
    const key = `${kind}`;
    let c = contexts.get(key);
    if (c === undefined) {
      c = kind === "2d" ? new Fake2DContext() : new FakeGL();
      contexts.set(key, c);
    }
    return c;
  } as typeof original;
  return {
    contexts,
    restore() {
      proto.getContext = original;
    },
  };
}
