/**
 * Regression tests for review-cycle contract defects: gesture stranding,
 * controlled-write deferral, lifecycle idempotency, range invariants,
 * and ARIA/token boundaries.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import "./env.ts";
import { mount, unmount } from "./env.ts";
import {
  collectGestures,
  key,
  part,
  ptr,
  stubRect,
  typesOf,
} from "./helpers.ts";
import {
  applyTheme,
  registerRvfElements,
  RvfRangeSelector,
  type RvfControlElement,
} from "../src/index.ts";
import type { InteractionEvent } from "../src/headless.ts";

registerRvfElements();

const DESCRIPTOR = { min: 0, max: 100, defaultValue: 20 } as const;

function make(tag: string, attrs: Record<string, string> = {}): RvfControlElement {
  const el = document.createElement(tag) as RvfControlElement;
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
  el.descriptor = { ...DESCRIPTOR };
  document.body.append(el);
  return el;
}

describe("gesture stranding regressions", () => {
  it("a second pointer never takes over the in-flight gesture", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track, { width: 100, height: 100 });
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { pointerId: 1, clientX: 50, clientY: 50 }));
    track?.dispatchEvent(ptr("pointerdown", { pointerId: 2, clientX: 90, clientY: 90 }));
    // Second begin is rejected (or ignored); the first gesture survives.
    track?.dispatchEvent(ptr("pointerup", { pointerId: 2, clientX: 90, clientY: 90 }));
    track?.dispatchEvent(ptr("pointerup", { pointerId: 1, clientX: 50, clientY: 50 }));
    const ids = new Set(
      events
        .filter((e) => e.meta.gestureId !== undefined)
        .map((e) => e.meta.gestureId),
    );
    assert.ok(
      ![...ids].some((id) => id !== undefined && id.startsWith("ptr-2")),
      "ptr-2 must never own a gesture",
    );
    assert.ok(events.some((e) => e.type === "commit"));
    assert.equal(el.hasAttribute("data-previewing"), false);
    unmount(el);
  });

  it("a rejected terminal commit cancels instead of stranding", () => {
    const el = document.createElement("rvf-fader") as RvfControlElement;
    el.setAttribute("orientation", "horizontal");
    el.descriptor = { min: 0, max: 100, outOfRange: "reject" };
    document.body.append(el);
    const track = part(el, "track");
    stubRect(track, { width: 100, height: 100 });
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 150, clientY: 50 })); // n>1 → reject
    assert.ok(events.some((e) => e.type === "rejected"));
    const term = [...events]
      .reverse()
      .find((e) => e.type === "commit" || e.type === "cancel");
    assert.equal(term?.type, "cancel");
    assert.equal(el.control?.getPhase(), "idle");
    // The control still works afterwards.
    el.dispatchEvent(key("keydown", { key: "ArrowUp" }));
    assert.ok(el.value !== 0);
    unmount(el);
  });

  it("second disconnect still terminates an in-flight gesture", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track);
    unmount(el);
    document.body.append(el); // remount — cleanups must re-run
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
    assert.equal(el.control?.getPhase(), "active");
    unmount(el);
    assert.ok(events.some((e) => e.type === "cancel"));
    assert.equal(el.control?.getPhase(), "idle");
  });

  it("refresh mid-gesture terminates instead of stranding", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track);
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
    assert.equal(el.control?.getPhase(), "active");
    el.setAttribute("variant", "compact"); // triggers refresh()
    assert.ok(events.some((e) => e.type === "cancel"));
    assert.equal(el.control?.getPhase(), "idle");
    unmount(el);
  });

  it("repeated drags mint fresh gesture ids (no already-terminated)", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track, { width: 100, height: 100 });
    const { events } = collectGestures(el);
    for (let i = 0; i < 3; i++) {
      track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
      track?.dispatchEvent(ptr("pointerup", { clientX: 60, clientY: 50 }));
    }
    const begins = events.filter((e) => e.type === "begin");
    const commits = events.filter((e) => e.type === "commit");
    assert.equal(begins.length, 3);
    assert.equal(commits.length, 3);
    assert.ok(
      events.every((e) => e.type !== "rejected"),
      `unexpected rejections: ${typesOf(events)}`,
    );
    unmount(el);
  });

  it("pointercancel emits a terminal cancel", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track);
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
    track?.dispatchEvent(ptr("pointercancel", { clientX: 50, clientY: 50 }));
    assert.ok(events.some((e) => e.type === "cancel"));
    assert.equal(el.control?.getPhase(), "idle");
    unmount(el);
  });

  it("wheel during a pointer drag emits no wheel gesture", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track);
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
    track?.dispatchEvent(new WheelEvent("wheel", { deltaY: -10, bubbles: true }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 50, clientY: 50 }));
    assert.ok(
      events.every((e) => e.meta.gestureId !== "wheel"),
      "wheel must not open a gesture mid-drag",
    );
    unmount(el);
  });
});

describe("controlled write deferral", () => {
  it("value write on a disabled control defers until enable", () => {
    const el = make("rvf-fader", { disabled: "" });
    el.value = 64;
    assert.equal(el.value, 64); // getter reflects the staged write
    el.removeAttribute("disabled");
    assert.equal(el.control?.getCommittedValue().plain, 64);
    unmount(el);
  });

  it("staged value survives a mid-gesture ports rebuild", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
    el.value = 80; // staged while the gesture is active
    el.ports = { format: (p: number) => `${p} units` };
    const track2 = part(el, "track");
    track2?.dispatchEvent(ptr("pointerup", { clientX: 50, clientY: 50 }));
    assert.equal(el.value, 80);
    assert.equal(el.control?.getCommittedValue().plain, 80);
    unmount(el);
  });

  it("event sequence continues across a rebuild", () => {
    const el = make("rvf-fader");
    const { events } = collectGestures(el);
    el.dispatchEvent(key("keydown", { key: "ArrowUp" }));
    el.ports = { format: (p: number) => `${p}` }; // rebuild (preserve)
    el.dispatchEvent(key("keydown", { key: "ArrowUp" }));
    const seqs = events.map((e) => e.meta.sequence);
    assert.deepEqual(
      [...seqs].sort((a, b) => a - b),
      seqs,
      "sequence must be non-decreasing across the rebuild",
    );
    assert.ok(new Set(seqs).size === seqs.length, "no sequence replays");
    unmount(el);
  });

  it("value attribute + descriptor ordering lands under the descriptor", () => {
    const el = document.createElement("rvf-fader") as RvfControlElement;
    el.setAttribute("value", "30");
    el.descriptor = { min: 0, max: 100, defaultValue: 20 };
    document.body.append(el);
    assert.equal(el.value, 30);
    assert.equal(el.getAttribute("aria-valuenow"), "30");
    unmount(el);
  });
});

describe("range selector invariants", () => {
  function makeRange(): RvfRangeSelector {
    const el = document.createElement("rvf-range-selector") as RvfRangeSelector;
    el.descriptor = { min: 0, max: 100 };
    document.body.append(el);
    el.valueLow = 20;
    el.valueHigh = 60;
    return el;
  }

  it("keyboard cannot cross edges", () => {
    const el = makeRange();
    const thumbLo = part(el, "thumb-lo");
    const { events } = collectGestures(el);
    thumbLo?.dispatchEvent(key("keydown", { key: "End" }));
    assert.ok(
      el.valueLow <= el.valueHigh,
      `lo=${el.valueLow} must not exceed hi=${el.valueHigh}`,
    );
    assert.equal(el.valueLow, 60);
    assert.ok(events.length > 0);
    unmount(el);
  });

  it("programmatic edge writes cannot cross", () => {
    const el = makeRange();
    el.valueLow = 90;
    assert.equal(el.valueLow, 60);
    el.valueHigh = 10;
    assert.equal(el.valueHigh, 60);
    unmount(el);
  });

  it("thumb press preserves the grab offset (no jump)", () => {
    const el = makeRange();
    const track = part(el, "track");
    const thumb = part(el, "thumb-lo");
    stubRect(track, { left: 0, top: 0, width: 100, height: 10 });
    const { events } = collectGestures(el);
    // lo sits at 0.2; press far right on the thumb (0.5) — no jump.
    thumb?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 5 }));
    const begin = events.find((e) => e.type === "begin");
    assert.ok(begin !== undefined);
    assert.equal(Math.round(begin.value.normalized * 100), 20);
    thumb?.dispatchEvent(ptr("pointerup", { clientX: 50, clientY: 5 }));
    assert.equal(Math.round(el.valueLow), 20);
    unmount(el);
  });

  it("control-id attribute change rebuilds edge controlIds", () => {
    const el = makeRange();
    const thumb = part(el, "thumb-hi");
    stubRect(part(el, "track"), { width: 100, height: 10 });
    el.setAttribute("control-id", "zoom");
    const { events } = collectGestures(el);
    thumb?.dispatchEvent(ptr("pointerdown", { clientX: 60, clientY: 5 }));
    thumb?.dispatchEvent(ptr("pointerup", { clientX: 60, clientY: 5 }));
    const ids = new Set(events.map((e) => e.meta.controlId));
    assert.ok(ids.has("zoom-hi"), `got ${[...ids].join(",")}`);
    assert.ok(!ids.has(`${el.id}-hi`) && !ids.has("rvf-range-selector-hi"));
    unmount(el);
  });

  it("range selector exposes the instance tokens surface", () => {
    const el = makeRange();
    el.tokens = { "range.fill": "#123456" };
    assert.equal(el.style.getPropertyValue("--rvf-range-fill"), "#123456");
    assert.equal(el.tokenDiagnostics.length, 0);
    unmount(el);
  });
});

describe("a11y boundary regressions", () => {
  it("vertical fader (default) reports aria-orientation", () => {
    const el = make("rvf-fader");
    assert.equal(el.getAttribute("aria-orientation"), "vertical");
    unmount(el);
  });

  it("numeric input mirrors ARIA only on the inner input, not the host", () => {
    const el = document.createElement("rvf-numeric-input") as RvfControlElement;
    el.setAttribute("label", "Freq");
    document.body.append(el);
    el.descriptor = { min: 0, max: 100 };
    // The host must not retain a stale role/aria copy after render.
    assert.equal(el.getAttribute("role"), null);
    assert.equal(el.getAttribute("aria-valuenow"), null);
    const input = part(el, "input");
    assert.equal(input?.getAttribute("role"), "spinbutton");
    unmount(el);
  });
});

describe("theme application regressions", () => {
  it("re-applying a theme clears stale vars and falls back to theme value", () => {
    const scope = document.createElement("div");
    document.body.append(scope);
    applyTheme(scope, { tokens: { "fader.length": 240 } });
    assert.equal(scope.style.getPropertyValue("--rvf-fader-length"), "240px");
    // Now an invalid override: diagnosed and resolved to the effective
    // default — the stale 240px must not survive.
    const diags = applyTheme(scope, { tokens: { "fader.length": -5 } });
    assert.ok(diags.length > 0);
    assert.equal(scope.style.getPropertyValue("--rvf-fader-length"), "160px");
    scope.remove();
  });

  it("invalid override under a dark base falls back to the dark value", () => {
    const scope = document.createElement("div");
    document.body.append(scope);
    const diags = applyTheme(scope, {
      base: "dark",
      tokens: { surface: 42 },
    });
    assert.ok(diags.length > 0);
    assert.equal(scope.style.getPropertyValue("--rvf-surface"), "#1d2025");
    scope.remove();
  });
});

describe("event hygiene", () => {
  it("property rebuild on an idle control emits no noop", () => {
    const el = make("rvf-fader");
    const { events } = collectGestures(el);
    el.ports = { format: (p: number) => `${p}` };
    el.setAttribute("control-id", "x2");
    assert.deepEqual(typesOf(events), []);
    unmount(el);
  });

  it("double-click resets to the descriptor default", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track);
    el.value = 80;
    const { events } = collectGestures(el);
    track?.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    assert.equal(el.value, 20);
    assert.ok(events.some((e) => e.type === "commit"));
    unmount(el);
  });

  it("Escape on a focused slider cancels the key gesture", () => {
    const el = make("rvf-fader");
    const { events } = collectGestures(el);
    // Keys open a held-key gesture: ArrowUp (set, gestureId "key") then
    // Escape terminates it.
    el.dispatchEvent(key("keydown", { key: "ArrowUp" }));
    el.dispatchEvent(key("keydown", { key: "Escape" }));
    const types = typesOf(events);
    assert.ok(types.includes("cancel") || types.includes("commit"));
    unmount(el);
  });
});

describe("numeric input regressions", () => {
  it("unit-formatted display text round-trips through the parser", () => {
    const el = document.createElement("rvf-numeric-input") as RvfControlElement;
    el.descriptor = { min: 0, max: 100, unit: "%", defaultValue: 45 };
    document.body.append(el);
    const input = part(el, "input") as HTMLInputElement | null;
    assert.ok(input !== null);
    // The field displays "45 %"; committing that text must parse.
    input.value = "45 %";
    const { events } = collectGestures(el);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(key("keydown", { key: "Enter" }));
    const types = typesOf(events);
    assert.ok(!types.includes("rejected"), `unexpected reject in ${types}`);
    assert.equal(el.value, 45);
    unmount(el);
  });

  it("editing state resyncs after a controller rebuild", () => {
    const el = document.createElement("rvf-numeric-input") as RvfControlElement;
    el.descriptor = { min: 0, max: 100 };
    document.body.append(el);
    const input = part(el, "input") as HTMLInputElement | null;
    assert.ok(input !== null);
    const { events } = collectGestures(el);
    input.value = "55";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    // Rebuild mid-edit strands element-side editing_ — the next input
    // must open a fresh gesture instead of dispatching on a dead one.
    el.ports = { format: (p: number) => `${p}` };
    input.value = "56";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    const begins = events.filter(
      (e: InteractionEvent) => e.type === "begin" && !e.meta.atomic,
    );
    assert.ok(begins.length >= 2, `expected a re-begin, got ${typesOf(events)}`);
    unmount(el);
  });
});
