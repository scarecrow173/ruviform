/**
 * Theme-matrix acceptance evidence (Issue #263):
 * - the same mock gesture/command scenario runs unchanged across every
 *   showcase theme column — appearance is data, never an API fork;
 * - token overrides, variant exchange, and slot exchange are data-only;
 * - a runtime theme swap repaints vars without changing event
 *   semantics.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  SHOWCASE_CONTROLS,
  SHOWCASE_THEMES,
  mountShowcase,
} from "../src/index.ts";
import type { InteractionEvent } from "../../components/src/index.ts";
import { RVF_TRANSPORT_EVENT } from "../../components/src/index.ts";
import { collectGestures, flushSync, typesOf } from "./helpers.ts";

/** One deterministic cross-theme input script per control kind —
 *  each kind's real public input path, never theme-conditional. */
function scriptedGesture(el: HTMLElement, key: string): void {
  const c = el as unknown as {
    dispatchInput?: (i: unknown) => void;
  };
  const g = `g-${key}`;
  switch (key) {
    case "toggle":
      c.dispatchInput!({ type: "set", value: { plain: 1 } });
      break;
    case "transport": {
      // `rvf-transport` intents come from the play button part.
      const play = el.shadowRoot!.querySelector('[part~="play"]')!;
      play.dispatchEvent(
        new MouseEvent("click", { bubbles: true, composed: true }),
      );
      break;
    }
    case "range": {
      // Two-edge control: ArrowRight on the `lo` thumb part.
      const lo = el.shadowRoot!.querySelector('[part~="thumb-lo"]')!;
      for (const type of ["keydown", "keyup"]) {
        lo.dispatchEvent(
          new KeyboardEvent(type, {
            key: "ArrowRight",
            bubbles: true,
            composed: true,
          }),
        );
      }
      break;
    }
    default:
      c.dispatchInput!({ type: "begin", gestureId: g, source: "pointer" });
      c.dispatchInput!({
        type: "preview",
        gestureId: g,
        value: { plain: 0.5 },
      });
      c.dispatchInput!({ type: "commit", gestureId: g });
  }
}

/** Collect `rvf-transport` intent details (discrete action channel). */
function collectTransportIntents(el: Element): {
  events: Record<string, unknown>[];
  stop(): void;
} {
  const events: Record<string, unknown>[] = [];
  const listener = (e: Event) =>
    events.push((e as CustomEvent).detail as Record<string, unknown>);
  el.addEventListener(RVF_TRANSPORT_EVENT, listener);
  return {
    events,
    stop: () => el.removeEventListener(RVF_TRANSPORT_EVENT, listener),
  };
}

/** Sanitize gesture details to the semantic surface (drop sequence
 *  ids, which legitimately differ across mounts). */
function sanitize(events: readonly InteractionEvent[]): unknown[] {
  return events.map((e) => ({
    type: e.type,
    controlId: e.meta.controlId.replace(/^[^.]+\./, ""),
    atomic: e.meta.atomic ?? false,
    value:
      "value" in e && e.value !== undefined
        ? { plain: e.value.plain }
        : undefined,
  }));
}

describe("theme matrix — same gesture script across all themes", () => {
  it("produces identical gesture event streams on every theme", async () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    const streams = new Map<string, unknown[]>();
    for (const col of h.columns) {
      for (const spec of SHOWCASE_CONTROLS) {
        const el = col.controls.get(spec.key);
        assert.ok(el !== undefined, `missing control ${spec.key}`);
        const k = `${col.theme.id}:${spec.key}`;
        if (spec.key === "transport") {
          const rec = collectTransportIntents(el);
          scriptedGesture(el, spec.key);
          rec.stop();
          streams.set(
            k,
            rec.events.map((d) => ({
              type: "action",
              action: d.action,
              controlId: String(d.controlId).replace(/^[^.]+\./, ""),
            })),
          );
        } else {
          const rec = collectGestures(el);
          scriptedGesture(el, spec.key);
          rec.stop();
          streams.set(k, sanitize(rec.events));
        }
      }
    }
    // Every column produced the same event stream for the same control —
    // compare per-control across themes (controlIds normalized above).
    for (const spec of SHOWCASE_CONTROLS) {
      const base = streams.get(`reference-light:${spec.key}`)!;
      assert.ok(
        base.length > 0,
        `vacuous parity: ${spec.key} emitted no events on reference-light`,
      );
      for (const theme of SHOWCASE_THEMES) {
        const got = streams.get(`${theme.id}:${spec.key}`);
        assert.deepEqual(
          got,
          base,
          `${spec.key} event stream differs under ${theme.id}`,
        );
      }
    }
    h.unmount();
  });

  it("each theme scope resolves a distinct token set (no-op themes detected)", () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    const seen = new Set<string>();
    for (const col of h.columns) {
      const interactive = col.scope.style.getPropertyValue(
        "--rvf-interactive",
      );
      const surface = col.scope.style.getPropertyValue("--rvf-surface");
      assert.notEqual(
        interactive,
        "",
        `theme ${col.theme.id} did not resolve --rvf-interactive`,
      );
      assert.notEqual(
        surface,
        "",
        `theme ${col.theme.id} did not resolve --rvf-surface`,
      );
      seen.add(`${interactive}|${surface}`);
    }
    // Every theme column must resolve a distinct var pair — a theme
    // silently collapsing into another theme's resolution fails here.
    assert.equal(
      seen.size,
      SHOWCASE_THEMES.length,
      `expected ${SHOWCASE_THEMES.length} distinct theme resolutions, got ${seen.size}`,
    );
    h.unmount();
  });

  it("no theme resolution produces token diagnostics", () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    for (const col of h.columns) {
      assert.deepEqual(
        [...col.scope.themeDiagnostics],
        [],
        `theme ${col.theme.id} reported diagnostics`,
      );
    }
    h.unmount();
  });
});

describe("token / variant / slot exchange is data-only", () => {
  it("instance token overrides land as inline vars and report diagnostics", () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    const col = h.columns[0]!;
    const fader = col.controls.get("fader")! as HTMLElement & {
      tokens?: unknown;
      tokenDiagnostics?: readonly { kind: string }[];
    };
    fader.tokens = { "fader.fill": "#123456" };
    assert.equal(
      fader.style.getPropertyValue("--rvf-fader-fill"),
      "#123456",
    );
    assert.deepEqual([...fader.tokenDiagnostics!], []);
    fader.tokens = { "fader.fill": "#123456", bogus: 1 };
    const diags = fader.tokenDiagnostics!;
    assert.equal(diags.length, 1);
    assert.equal(diags[0]!.kind, "unknown-token");
    h.unmount();
  });

  it("variant='compact' re-renders without a gesture-contract change", () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    const col = h.columns[0]!;
    const knob = col.controls.get("knob")! as HTMLElement;
    const rec = collectGestures(knob);
    knob.setAttribute("variant", "compact");
    scriptedGesture(knob, "knob");
    rec.stop();
    assert.deepEqual(typesOf(rec.events), ["begin", "preview", "commit"]);
    h.unmount();
  });

  it("label slot content overrides the attribute fallback", () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    const col = h.columns[0]!;
    const knob = col.controls.get("knob")!;
    const span = document.createElement("span");
    span.setAttribute("slot", "label");
    span.textContent = "Slotted";
    knob.append(span);
    const slot = knob.shadowRoot!.querySelector("slot[name=label]");
    assert.ok(slot !== null, "label slot missing");
    const assigned = (slot as HTMLSlotElement).assignedNodes();
    assert.ok(
      assigned.includes(span),
      "slotted label content not assigned",
    );
    h.unmount();
  });

  it("range edge gestures reach the gateway + receipts reflect per edge", () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    const col = h.columns[0]!;
    const el = col.controls.get("range")! as HTMLElement & {
      valueLow: number;
      valueHigh: number;
    };
    // ArrowRight on the lo thumb → <id>-lo gesture → lo-edge binding.
    const lo = el.shadowRoot!.querySelector('[part~="thumb-lo"]')!;
    for (const type of ["keydown", "keyup"]) {
      lo.dispatchEvent(
        new KeyboardEvent(type, {
          key: "ArrowRight",
          bubbles: true,
          composed: true,
        }),
      );
    }
    const rec = h.gateway.received.find(
      (r) =>
        r.intent.controlId === `${col.theme.id}.range-lo` &&
        r.intent.kind === "commit",
    );
    assert.ok(
      rec !== undefined,
      "range-lo commit never reached the gateway (edge controlId dropped)",
    );
    h.apply(rec!);
    assert.equal(
      el.valueLow,
      h.gateway.valueOf(`demo.${col.theme.id}.range-lo`),
      "applied lo-edge projection did not reflect onto valueLow",
    );
    // The hi edge stayed at its own target (no cross-edge leakage).
    assert.equal(
      el.valueHigh,
      h.gateway.valueOf(`demo.${col.theme.id}.range-hi`),
    );
    h.unmount();
  });

  it("a runtime base swap repaints vars while gesture semantics hold", () => {
    const h = mountShowcase(document.body, { flushWith: flushSync, autopump: false });
    const col = h.columns[0]!;
    const before = col.scope.style.getPropertyValue("--rvf-surface");
    col.scope.theme = { base: "dark" };
    const after = col.scope.style.getPropertyValue("--rvf-surface");
    assert.notEqual(before, after, "base swap did not repaint scope vars");
    const knob = col.controls.get("knob")!;
    const rec = collectGestures(knob);
    scriptedGesture(knob, "knob");
    rec.stop();
    assert.deepEqual(typesOf(rec.events), ["begin", "preview", "commit"]);
    h.unmount();
  });
});
