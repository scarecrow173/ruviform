/**
 * Pure-math units: axis ticks/labels, viewport mapping, color parsing,
 * and the bounded LRU view cache — no DOM, no canvas.
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  formatTick,
  linearTicks,
  logTicks,
  niceStep,
} from "../src/axis.ts";
import { LruViewCache, waveformViewKey } from "../src/cache.ts";
import { parseCssColor } from "../src/color.ts";
import {
  viewport,
  vpClamp,
  vpPan,
  vpToPx,
  vpToUnit,
  vpUnitsPerPx,
  vpZoom,
} from "../src/viewport.ts";

describe("niceStep", () => {
  it("returns 1/2/5·10ⁿ steps >= raw", () => {
    assert.equal(niceStep(0.4), 0.5);
    assert.equal(niceStep(1), 1);
    assert.equal(niceStep(1.5), 2);
    assert.equal(niceStep(3), 5);
    assert.equal(niceStep(7), 10);
    assert.equal(niceStep(0.03), 0.05);
    assert.equal(niceStep(120), 200);
  });
  it("degenerate input → 1", () => {
    assert.equal(niceStep(0), 1);
    assert.equal(niceStep(-5), 1);
    assert.equal(niceStep(Number.NaN), 1);
  });
});

describe("linearTicks", () => {
  it("spaces majors by niceStep(unitsPerPx * minPx)", () => {
    // 1000px axis over [0, 10] → 0.01/px → step ≥ 0.56 → 1.
    const ticks = linearTicks(0, 10, 0.01, 56);
    assert.deepEqual(
      ticks.map((t) => t.value),
      [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    );
    assert.ok(ticks.every((t) => t.major));
  });
  it("starts on the first multiple ≥ min, ends before max", () => {
    const ticks = linearTicks(2.3, 8.7, 0.001, 56);
    // upp*56 = 0.056 → step 0.1 → 2.3 is itself a multiple → 2.3.
    assert.ok(Math.abs((ticks[0]?.value ?? 0) - 2.3) < 1e-9);
    assert.ok((ticks.at(-1)?.value ?? 99) < 8.7);
  });
  it("empty on degenerate ranges", () => {
    assert.deepEqual(linearTicks(5, 5, 1), []);
    assert.deepEqual(linearTicks(9, 5, 1), []);
    assert.deepEqual(linearTicks(0, 5, 0), []);
  });
});

describe("logTicks", () => {
  it("decades major, 2–9 sub-ticks minor", () => {
    const ticks = logTicks(20, 20000);
    const majors = ticks.filter((t) => t.major).map((t) => t.value);
    assert.deepEqual(majors, [100, 1000, 10000]);
    assert.ok(ticks.some((t) => t.value === 50 && !t.major));
    assert.ok(ticks.some((t) => t.value === 2000 && !t.major));
  });
  it("empty when minHz <= 0 (caller must clamp)", () => {
    assert.deepEqual(logTicks(0, 1000), []);
  });
});

describe("formatTick", () => {
  it("seconds adapts to step size", () => {
    assert.equal(formatTick(0, "seconds"), "0");
    assert.equal(formatTick(1.5, "seconds", 1), "1.5s");
    assert.equal(formatTick(0.25, "seconds", 0.5), "0.25s");
    assert.equal(formatTick(0.0005, "seconds", 0.001), "0.5ms");
    assert.equal(formatTick(75, "seconds", 60), "1:15");
  });
  it("hz → k for thousands; db → int", () => {
    assert.equal(formatTick(20000, "hz"), "20k");
    assert.equal(formatTick(440, "hz"), "440");
    assert.equal(formatTick(-18.3, "db"), "-18");
  });
  it("non-finite → em dash", () => {
    assert.equal(formatTick(Number.NaN, "hz"), "—");
  });
});

describe("viewport", () => {
  it("rejects degenerate input", () => {
    assert.equal(viewport(5, 5, 100), undefined);
    assert.equal(viewport(0, 10, 0), undefined);
    assert.equal(viewport(Number.NaN, 10, 100), undefined);
  });
  it("maps units↔px exactly at edges", () => {
    const vp = viewport(0, 10, 100);
    assert.ok(vp !== undefined);
    assert.equal(vpUnitsPerPx(vp), 0.1);
    assert.equal(vpToPx(vp, 0), 0);
    assert.equal(vpToPx(vp, 10), 100);
    assert.equal(vpToUnit(vp, 0), 0.05); // center of first px
  });
  it("zoom preserves the anchor's px position", () => {
    const vp = viewport(0, 10, 100);
    assert.ok(vp !== undefined);
    const z = vpZoom(vp, 2, 5);
    assert.ok(z !== undefined);
    assert.equal(vpToPx(z, 5), vpToPx(vp, 5));
    assert.ok(z.end - z.start < 10);
  });
  it("pan slides positions; clamp holds bounds", () => {
    const vp = viewport(0, 10, 100);
    assert.ok(vp !== undefined);
    const p = vpPan(vp, 50); // content right → earlier units
    assert.equal(p.start, -5);
    const c = vpClamp(p, 0, 20);
    assert.equal(c.start, 0);
    assert.equal(c.end, 10);
  });
});

describe("parseCssColor", () => {
  it("parses hex forms", () => {
    assert.deepEqual(parseCssColor("#f00"), { r: 1, g: 0, b: 0, a: 1 });
    const c = parseCssColor("#2f6fd0");
    assert.ok(c !== undefined);
    assert.equal(c.r, 0x2f / 255);
    assert.equal(c.a, 1);
    assert.equal(parseCssColor("#ff000080")?.a, 0x80 / 255);
  });
  it("parses rgb()/rgba() comma + space/slash forms", () => {
    assert.deepEqual(parseCssColor("rgb(255, 0, 0)"), {
      r: 1,
      g: 0,
      b: 0,
      a: 1,
    });
    assert.equal(parseCssColor("rgb(255 0 0 / 50%)")?.a, 0.5);
    assert.equal(parseCssColor("rgba(0,0,0,0.25)")?.a, 0.25);
    assert.equal(parseCssColor("rgb(100% 0% 0%)")?.r, 1);
  });
  it("transparent + unparseable", () => {
    assert.deepEqual(parseCssColor("transparent"), {
      r: 0,
      g: 0,
      b: 0,
      a: 0,
    });
    assert.equal(parseCssColor("not-a-color"), undefined);
    assert.equal(parseCssColor("var(--x)"), undefined);
  });
});

describe("LruViewCache", () => {
  it("stores, hits, and evicts by byte budget (LRU order)", () => {
    const c = new LruViewCache<string>(100);
    c.set("a", "A", 40);
    c.set("b", "B", 40);
    c.get("a"); // touch a → b is now oldest
    c.set("c", "C", 40);
    assert.equal(c.get("a"), "A");
    assert.equal(c.get("b"), undefined);
    assert.equal(c.get("c"), "C");
    assert.ok(c.bytes <= 100);
  });
  it("counts hits/misses", () => {
    const c = new LruViewCache<number>(10);
    c.set("x", 1, 4);
    c.get("x");
    c.get("y");
    assert.equal(c.hits, 1);
    assert.equal(c.misses, 1);
  });
  it("oversize entry evicts everything else but is stored", () => {
    const c = new LruViewCache<string>(10);
    c.set("a", "A", 5);
    c.set("big", "B", 50);
    assert.equal(c.get("a"), undefined);
    assert.equal(c.get("big"), "B");
  });
  it("rejects non-positive budget", () => {
    assert.throws(() => new LruViewCache<string>(0));
  });
});

describe("waveformViewKey", () => {
  it("covers identity+revision+geometry; no style fields", () => {
    const k1 = waveformViewKey("src", 3, -1, 0, 10, 200);
    const k2 = waveformViewKey("src", 4, -1, 0, 10, 200);
    assert.notEqual(k1, k2); // revision changes the key
    assert.equal(
      waveformViewKey("src", 3, -1, 0, 10, 200),
      k1, // deterministic
    );
    assert.ok(!k1.includes("fill")); // nothing paint-ish in the key
  });
});
