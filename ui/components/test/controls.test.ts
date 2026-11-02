/** Reference-Skin control tests: semantics, gestures, a11y, lifecycle. */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import "./env.ts";
import { mount, unmount } from "./env.ts";
import { collectGestures, key, part, ptr, stubRect, typesOf } from "./helpers.ts";
import {
  registerRvfElements,
  RvfKnob,
  type RvfControlElement,
} from "../src/index.ts";

registerRvfElements();

const DESCRIPTOR = { min: 0, max: 100, defaultValue: 20 } as const;

/** Create a control with the descriptor set pre-connect (consumer path). */
function make<K extends string>(
  tag: K,
  attrs: Record<string, string> = {},
): RvfControlElement {
  const el = document.createElement(tag) as RvfControlElement;
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
  el.descriptor = { ...DESCRIPTOR };
  document.body.append(el);
  return el;
}

describe("registration", () => {
  it("defines all elements", () => {
    for (const tag of [
      "rvf-theme",
      "rvf-knob",
      "rvf-fader",
      "rvf-seekbar",
      "rvf-toggle",
      "rvf-numeric-input",
      "rvf-transport",
      "rvf-range-selector",
    ]) {
      assert.ok(customElements.get(tag) !== undefined, tag);
    }
  });
});

describe("common value-control contract", () => {
  for (const tag of ["rvf-knob", "rvf-fader", "rvf-seekbar"]) {
    it(`${tag}: role=slider + ARIA + label`, () => {
      const el = make(tag, { label: "Gain" });
      assert.equal(el.getAttribute("role"), "slider");
      assert.equal(el.getAttribute("aria-label"), "Gain");
      assert.equal(el.getAttribute("aria-valuemin"), "0");
      assert.equal(el.getAttribute("aria-valuemax"), "100");
      assert.equal(el.getAttribute("aria-valuenow"), "20");
      assert.equal(el.getAttribute("tabindex"), "0");
      unmount(el);
    });

    it(`${tag}: controlled value writes emit no intent events`, () => {
      const el = make(tag);
      const { events } = collectGestures(el);
      el.value = 64;
      assert.equal(el.value, 64);
      assert.equal(el.getAttribute("aria-valuenow"), "64");
      assert.equal(events.length, 0);
      unmount(el);
    });

    it(`${tag}: pointer drag emits begin→preview→commit`, () => {
      const el = make(tag);
      const track = part(el, "track") ?? part(el, "dial");
      stubRect(track, { left: 0, top: 0, width: 100, height: 100 });
      const { events } = collectGestures(el);
      track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
      track?.dispatchEvent(ptr("pointermove", { clientX: 60, clientY: 40 }));
      track?.dispatchEvent(ptr("pointerup", { clientX: 60, clientY: 40 }));
      assert.deepEqual(
        typesOf(events).filter((t) => t === "begin" || t === "commit"),
        ["begin", "commit"],
      );
      assert.ok(events.some((e) => e.type === "preview"));
      assert.equal(el.hasAttribute("data-previewing"), false);
      unmount(el);
    });

    it(`${tag}: keyboard arrows emit atomic begin+commit`, () => {
      const el = make(tag);
      const { events } = collectGestures(el);
      el.dispatchEvent(key("keydown", { key: "ArrowUp" }));
      assert.deepEqual(typesOf(events), ["begin", "commit"]);
      assert.ok(el.value > 20);
      el.dispatchEvent(key("keydown", { key: "Home" }));
      assert.equal(el.displayValue.plain, 0);
      unmount(el);
    });

    it(`${tag}: disabled suppresses gestures but not the control`, () => {
      const el = make(tag, { disabled: "" });
      const track = part(el, "track") ?? part(el, "dial");
      stubRect(track);
      const { events } = collectGestures(el);
      track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
      assert.ok(events.every((e) => e.type !== "commit"));
      assert.equal(el.value, 20);
      assert.equal(el.getAttribute("aria-disabled"), "true");
      assert.equal(el.hasAttribute("tabindex"), false);
      unmount(el);
    });

    it(`${tag}: out-of-range write reports rejected and flashes`, () => {
      const el = make(tag);
      const { events } = collectGestures(el);
      el.value = Number.NaN;
      assert.ok(events.some((e) => e.type === "rejected"));
      assert.ok(el.hasAttribute("data-rejected"));
      unmount(el);
    });
  }

  it("fader horizontal orientation maps X axis", () => {
    const el = make("rvf-fader", { orientation: "horizontal" });
    const track = part(el, "track");
    stubRect(track, { left: 0, top: 0, width: 200, height: 10 });
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 100, clientY: 5 }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 100, clientY: 5 }));
    const commit = events.find((e) => e.type === "commit");
    assert.ok(commit !== undefined);
    assert.equal(el.value, 50);
    assert.equal(el.getAttribute("aria-orientation"), "horizontal");
    unmount(el);
  });

  it("value write during gesture is deferred to gesture end", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track, { width: 100, height: 100 });
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
    assert.ok(el.hasAttribute("data-previewing"));
    el.value = 90; // deferred — gesture in flight
    track?.dispatchEvent(ptr("pointerup", { clientX: 50, clientY: 50 }));
    // consumer write lands after the gesture's own commit
    assert.equal(el.value, 90);
    const commits = events.filter((e) => e.type === "commit");
    assert.ok(commits.length >= 1);
    unmount(el);
  });

  it("disconnect mid-gesture emits a terminal cancel", () => {
    const el = make("rvf-fader");
    const track = part(el, "track");
    stubRect(track);
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50, clientY: 50 }));
    assert.equal(el.control?.getPhase(), "active");
    unmount(el);
    assert.ok(events.some((e) => e.type === "cancel"));
  });

  it("reconnect keeps the committed value and does not double-bind", () => {
    const el = make("rvf-fader");
    el.value = 42;
    unmount(el);
    document.body.append(el);
    const { events } = collectGestures(el);
    assert.equal(el.value, 42);
    el.dispatchEvent(key("keydown", { key: "ArrowDown" }));
    // single atomic pair — listeners re-bound once
    assert.deepEqual(typesOf(events), ["begin", "commit"]);
    unmount(el);
  });
});

describe("rvf-toggle", () => {
  it("role=switch flips on click with atomic begin+commit", () => {
    const el = make("rvf-toggle") as RvfControlElement & {
      checked: boolean;
    };
    assert.equal(el.getAttribute("role"), "switch");
    assert.equal(el.getAttribute("aria-checked"), "false");
    const { events } = collectGestures(el);
    el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    assert.equal(el.checked, true);
    assert.equal(el.getAttribute("aria-checked"), "true");
    assert.deepEqual(typesOf(events), ["begin", "commit"]);
    unmount(el);
  });

  it("Space and Enter flip", () => {
    const el = make("rvf-toggle") as RvfControlElement & { checked: boolean };
    el.dispatchEvent(key("keydown", { key: " " }));
    assert.equal(el.checked, true);
    el.dispatchEvent(key("keydown", { key: "Enter" }));
    assert.equal(el.checked, false);
    unmount(el);
  });

  it("controlled checked writes emit nothing", () => {
    const el = make("rvf-toggle") as RvfControlElement & { checked: boolean };
    const { events } = collectGestures(el);
    el.checked = true;
    assert.equal(el.checked, true);
    assert.equal(events.length, 0);
    unmount(el);
  });
});

describe("rvf-seekbar", () => {
  it("buffered indicator is display-only and clamped", () => {
    const el = make("rvf-seekbar") as RvfControlElement & {
      buffered: number;
    };
    el.buffered = 0.6;
    const b = part(el, "buffered") as HTMLElement;
    assert.equal(b.style.width, "60%");
    el.buffered = 2;
    assert.equal(el.buffered, 1);
    unmount(el);
  });
});

describe("a11y state attributes", () => {
  it("readonly/invalid/pending are separate, perceivable states", () => {
    const el = make("rvf-knob", { readonly: "", invalid: "", pending: "" });
    assert.equal(el.getAttribute("aria-readonly"), "true");
    assert.equal(el.getAttribute("aria-invalid"), "true");
    assert.equal(el.getAttribute("aria-busy"), "true");
    // readOnly still renders the current value, just inert
    assert.equal(el.getAttribute("aria-valuenow"), "20");
    const { events } = collectGestures(el);
    part(el, "dial")?.dispatchEvent(ptr("pointerdown", { clientY: 50 }));
    assert.ok(events.every((e) => e.type !== "commit"));
    unmount(el);
  });

  it("custom skin path: subclass keeps gesture + a11y machinery", () => {
    class DarkKnob extends RvfKnob {
      protected override stylesheet(): string {
        return ":host { background: #000; }";
      }
    }
    customElements.define("rvf-dark-knob", DarkKnob);
    const el = mount("rvf-dark-knob") as RvfControlElement;
    el.descriptor = { ...DESCRIPTOR };
    const { events } = collectGestures(el);
    el.dispatchEvent(key("keydown", { key: "ArrowUp" }));
    assert.deepEqual(typesOf(events), ["begin", "commit"]);
    unmount(el);
  });
});
