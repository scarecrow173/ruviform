/**
 * Showcase mount evidence (Issue #263): the Reference Skin alone
 * displays and operates every delivered control + viz — no product
 * CSS/Canvas hand-writing is a dependency. DOM-level snapshot and
 * geometry checks run headlessly; pixel-perfect raster is out of
 * scope (recorded as unverified in the impl record).
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  SHOWCASE_CONTROLS,
  SHOWCASE_DEFERRED_VIZ,
  SHOWCASE_THEMES,
  SHOWCASE_VIZ,
  mountShowcase,
  pumpDemoOnce,
} from "../src/index.ts";
import {
  Fake2DContext,
  installFakeCanvas,
} from "./env.ts";
import {
  domFingerprint,
  flushMicrotasks,
  flushSync,
  part,
} from "./helpers.ts";

describe("mountShowcase — Reference Skin alone", () => {
  it("mounts the full matrix under every theme column", () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    assert.equal(h.columns.length, SHOWCASE_THEMES.length);
    for (const col of h.columns) {
      assert.equal(col.controls.size, SHOWCASE_CONTROLS.length);
      assert.equal(col.viz.size, SHOWCASE_VIZ.length);
      for (const spec of SHOWCASE_CONTROLS) {
        const el = col.controls.get(spec.key)!;
        assert.equal(el.isConnected, true);
        assert.ok(
          el.shadowRoot !== null && part(el, "root") !== null,
          `${spec.key} has no rendered shadow root`,
        );
      }
      for (const key of SHOWCASE_VIZ) {
        const el = col.viz.get(key)!;
        assert.equal(el.isConnected, true);
        assert.ok(part(el, "status") !== null, `${key} has no status part`);
        assert.ok(
          part(el, "plot-canvas") !== null,
          `${key} has no plot canvas`,
        );
      }
      // Deferred viz rows are honest placeholders, not fake elements.
      for (const d of SHOWCASE_DEFERRED_VIZ) {
        const cell = col.scope.querySelector(`[data-deferred="${d.key}"]`);
        assert.ok(cell !== null, `missing deferred cell ${d.key}`);
        assert.match(cell.textContent ?? "", /deferred/);
      }
    }
    h.unmount();
  });

  it("paints each viz element on the fake 2D context after sizing", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const h = mountShowcase(document.body, {
        flushWith: flushSync,
        themes: ["reference-light"],
        autopump: false,
      });
      const col = h.columns[0]!;
      for (const key of SHOWCASE_VIZ) {
        const el = col.viz.get(key)! as HTMLElement & {
          setSize(w: number, h: number): void;
          drawNow(): void;
        };
        el.setSize(320, 160);
      }
      pumpDemoOnce(col.sources, 1, 0.25);
      await flushMicrotasks();
      for (const key of SHOWCASE_VIZ) {
        const el = col.viz.get(key)! as HTMLElement & {
          drawNow(): void;
          pumpNow?(): void;
          stats: { framesDrawn: number; lastOpCount: number };
        };
        el.pumpNow?.();
        el.drawNow();
        // Per-element: the draw path ran AND emitted draw ops.
        assert.ok(
          el.stats.framesDrawn >= 1,
          `${key} never drew a frame`,
        );
        assert.ok(
          el.stats.lastOpCount > 0,
          `${key} drew zero ops`,
        );
      }
      const ctx = fake.contexts.get("2d") as Fake2DContext | undefined;
      assert.ok(ctx !== undefined, "no 2D context was ever requested");
      h.unmount();
    } finally {
      fake.restore();
    }
  });

  it("theme swap repaints without re-querying the waveform source", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const h = mountShowcase(document.body, {
        flushWith: flushSync,
        themes: ["reference-light"],
        autopump: false,
      });
      const col = h.columns[0]!;
      const wf = col.viz.get("waveform")! as HTMLElement & {
        setSize(w: number, h: number): void;
        drawNow(): void;
        source?: unknown;
      };
      // Spy BEFORE first paint so the assignment query is observable;
      // the swap must not add queries on top of that baseline.
      let queries = 0;
      const src = col.sources.waveform;
      const wrapped = {
        meta: () => src.meta(),
        subscribe: src.subscribe?.bind(src),
        query: (q: never, s?: AbortSignal) => {
          queries += 1;
          return src.query(q, s);
        },
      };
      wf.source = wrapped;
      wf.setSize(320, 160);
      await flushMicrotasks();
      wf.drawNow();
      // The first layout arms the deferred bounded query — it issues on
      // a microtask AFTER drawNow returns.
      await flushMicrotasks();
      const baseline = queries;
      assert.ok(baseline > 0, "no baseline waveform query was issued");
      col.scope.theme = { base: "dark" };
      await flushMicrotasks();
      wf.drawNow();
      assert.equal(
        queries,
        baseline,
        "theme swap re-queried the waveform source (paint/data leak)",
      );
      h.unmount();
    } finally {
      fake.restore();
    }
  });

  it("initial projections reflect onto bound controls (basic operation)", () => {
    const h = mountShowcase(document.body, {
      flushWith: flushSync,
      autopump: false,
    });
    const col = h.columns[0]!;
    // subscribe() pushed the initial projection — the binding OBSERVED
    // it (proving the subscribe→observe path), and the adapter
    // reflected it onto the element (not just its own default).
    const fader = col.controls.get("fader")! as HTMLElement & {
      value?: number;
    };
    const binding = col.bindings.get("fader")!;
    assert.equal(binding.state.phase, "observed");
    const observed = binding.state.lastObserved?.value as
      | { plain: number }
      | undefined;
    assert.equal(observed?.plain, -12);
    assert.equal(fader.value, -12);
    // A later authoritative projection moves the element — the reflect
    // path is live, not a default coincidence.
    h.gateway.pushProjection(`demo.${col.theme.id}.fader`, {
      revision: 9,
      value: { plain: -30 },
    });
    assert.equal(fader.value, -30);
    h.unmount();
  });

  it("DOM fingerprint baseline — the semantic surface is stable", () => {
    const h = mountShowcase(document.body, {
      flushWith: flushSync,
      themes: ["reference-light"],
      viz: false,
      deferredRows: false,
    });
    const col = h.columns[0]!;
    const fp = domFingerprint(col.controls.get("fader")!);
    // Baseline: semantic parts/roles/ARIA only (no classes/styles).
    const expected = [
      "<rvf-fader aria-label=Vol aria-orientation=vertical",
      "aria-valuemax=6 aria-valuemin=-60",
      "data-control=fader role=slider tabindex=0",
      "<div part=root>",
      "<slot name=label>",
      "part=track",
      "part=fill",
      "part=thumb",
    ];
    for (const frag of expected) {
      assert.ok(
        fp.includes(frag),
        `fingerprint missing fragment: ${frag}\n---\n${fp}`,
      );
    }
    // Knob + transport semantic surfaces carry their own baselines.
    const knobFp = domFingerprint(col.controls.get("knob")!);
    for (const frag of [
      "<rvf-knob aria-label=Gain",
      "role=slider",
      "part=arc",
      "part=readout",
      "<slot name=label>",
    ]) {
      assert.ok(knobFp.includes(frag), `knob fingerprint missing: ${frag}`);
    }
    const trFp = domFingerprint(col.controls.get("transport")!);
    for (const frag of [
      "role=group",
      "data-action=play",
      "data-action=stop",
      "aria-label=Play",
      "<slot name=label>",
    ]) {
      assert.ok(trFp.includes(frag), `transport fingerprint missing: ${frag}`);
    }
    h.unmount();
  });
});
