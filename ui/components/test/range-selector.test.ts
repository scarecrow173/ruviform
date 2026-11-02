/** RangeSelector tests: two-edge gesture contract + ARIA. */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import "./env.ts";
import { unmount } from "./env.ts";
import { collectGestures, key, part, ptr, stubRect, typesOf } from "./helpers.ts";
import {
  registerRvfElements,
  type RvfRangeSelector,
} from "../src/index.ts";

registerRvfElements();

const DESCRIPTOR = { min: 0, max: 100, defaultValue: 20 } as const;

function makeRange(attrs: Record<string, string> = {}): RvfRangeSelector {
  const el = document.createElement("rvf-range-selector") as RvfRangeSelector;
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
  el.descriptor = { ...DESCRIPTOR };
  document.body.append(el);
  return el;
}

describe("rvf-range-selector", () => {
  it("exposes two slider thumbs with role + edge labels", () => {
    const el = makeRange({ label: "Trim" });
    const lo = part(el, "thumb-lo") as HTMLElement;
    const hi = part(el, "thumb-hi") as HTMLElement;
    assert.equal(lo.getAttribute("role"), "slider");
    assert.equal(hi.getAttribute("role"), "slider");
    assert.equal(lo.getAttribute("aria-label"), "Trim minimum");
    assert.equal(hi.getAttribute("aria-label"), "Trim maximum");
    assert.equal(lo.getAttribute("aria-valuenow"), "0");
    assert.equal(hi.getAttribute("aria-valuenow"), "100");
    unmount(el);
  });

  it("initial span covers the full range", () => {
    const el = makeRange();
    assert.equal(el.valueLow, 0);
    assert.equal(el.valueHigh, 100);
    unmount(el);
  });

  it("thumb drag emits per-edge events with distinct controlIds", () => {
    const el = makeRange({ "control-id": "sel" });
    const lo = part(el, "thumb-lo");
    stubRect(part(el, "track"), { left: 0, width: 100 });
    const { events } = collectGestures(el);
    lo?.dispatchEvent(ptr("pointerdown", { clientX: 0 }));
    lo?.dispatchEvent(ptr("pointermove", { clientX: 30 }));
    lo?.dispatchEvent(ptr("pointerup", { clientX: 30 }));
    assert.deepEqual(
      typesOf(events).filter((t) => t === "begin" || t === "commit"),
      ["begin", "commit"],
    );
    assert.ok(events.every((e) => e.meta.controlId === "sel-lo"));
    assert.equal(el.valueLow, 30);
    unmount(el);
  });

  it("edges cannot cross: hi drag clamps at lo", () => {
    const el = makeRange();
    el.valueLow = 40;
    const hi = part(el, "thumb-hi");
    stubRect(part(el, "track"), { left: 0, width: 100 });
    const { events } = collectGestures(el);
    hi?.dispatchEvent(ptr("pointerdown", { clientX: 80 }));
    hi?.dispatchEvent(ptr("pointermove", { clientX: 10 }));
    hi?.dispatchEvent(ptr("pointerup", { clientX: 10 }));
    assert.ok(el.valueHigh >= 40);
    unmount(el);
  });

  it("track click jumps the nearer edge atomically", () => {
    const el = makeRange({ "control-id": "sel" });
    el.valueLow = 10;
    el.valueHigh = 90;
    const track = part(el, "track");
    stubRect(track, { left: 0, width: 100 });
    const { events } = collectGestures(el);
    track?.dispatchEvent(ptr("pointerdown", { clientX: 20 }));
    // nearer to lo(10): |20-10|=10 < |20-90|=70
    assert.ok(events.some((e) => e.type === "commit"));
    assert.ok(events.every((e) => e.meta.controlId === "sel-lo"));
    assert.equal(el.valueLow, 20);
    unmount(el);
  });

  it("per-thumb keyboard adjusts each edge independently", () => {
    const el = makeRange();
    el.valueLow = 30;
    const lo = part(el, "thumb-lo");
    const hi = part(el, "thumb-hi");
    lo?.dispatchEvent(key("keydown", { key: "ArrowLeft" }));
    assert.ok(el.valueLow < 30);
    hi?.dispatchEvent(key("keydown", { key: "ArrowRight" }));
    assert.equal(el.valueHigh, 100); // already at max → stays clamped
    unmount(el);
  });

  it("controlled valueLow/valueHigh writes emit nothing", () => {
    const el = makeRange();
    const { events } = collectGestures(el);
    el.valueLow = 25;
    el.valueHigh = 75;
    assert.equal(events.length, 0);
    const lo = part(el, "thumb-lo") as HTMLElement;
    const hi = part(el, "thumb-hi") as HTMLElement;
    assert.equal(lo.getAttribute("aria-valuenow"), "25");
    assert.equal(hi.getAttribute("aria-valuenow"), "75");
    unmount(el);
  });
});
