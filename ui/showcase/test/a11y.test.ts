/**
 * A11y / state regression checks across the theme matrix (Issue #263):
 * keyboard contract, ARIA surface, disabled / readOnly / pending /
 * invalid states — exercised identically per theme column to prove
 * state semantics never fork on appearance. Screen-reader and
 * real-pixel checks are out of scope headlessly (see impl record).
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { SHOWCASE_THEMES, mountShowcase } from "../src/index.ts";
import {
  collectGestures,
  flushSync,
  key,
  typesOf,
} from "./helpers.ts";

describe("a11y + state — identical behavior per theme", () => {
  for (const theme of SHOWCASE_THEMES) {
    describe(`theme ${theme.id}`, () => {
      it("slider controls expose ARIA value attributes + keyboard gestures", () => {
        const h = mountShowcase(document.body, {
          flushWith: flushSync,
          themes: [theme.id],
          viz: false,
          deferredRows: false,
        });
        const col = h.columns[0]!;
        const mins: Record<string, string> = {
          knob: "0",
          fader: "-60",
          seekbar: "0",
        };
        for (const key_ of ["knob", "fader", "seekbar"]) {
          const el = col.controls.get(key_)! as HTMLElement & {
            descriptor?: unknown;
          };
          assert.equal(el.getAttribute("role"), "slider", key_);
          assert.equal(el.getAttribute("tabindex"), "0", key_);
          assert.equal(el.getAttribute("aria-valuemin"), mins[key_], key_);
          assert.ok(el.getAttribute("aria-valuemax") !== null, key_);
          assert.ok(el.getAttribute("aria-valuenow") !== null, key_);
          assert.ok(el.getAttribute("aria-valuetext") !== null, key_);
          const rec = collectGestures(el);
          el.dispatchEvent(key("keydown", { key: "ArrowRight" }));
          el.dispatchEvent(key("keyup", { key: "ArrowRight" }));
          rec.stop();
          // keyboard commit — atomic begin+commit pair.
          assert.deepEqual(typesOf(rec.events), ["begin", "commit"], key_);
          assert.equal(
            rec.events.every((e) => e.meta.source === "keyboard"),
            true,
            key_,
          );
        }
        h.unmount();
      });

      it("disabled blocks gestures + input, sets aria-disabled", () => {
        const h = mountShowcase(document.body, {
          flushWith: flushSync,
          themes: [theme.id],
          viz: false,
          deferredRows: false,
        });
        const col = h.columns[0]!;
        const el = col.controls.get("knob")! as HTMLElement & {
          disabled: boolean;
          dispatchInput(i: unknown): void;
        };
        el.disabled = true;
        assert.equal(el.getAttribute("aria-disabled"), "true");
        const rec = collectGestures(el);
        el.dispatchEvent(key("keydown", { key: "ArrowRight" }));
        el.dispatchInput({ type: "set", value: { plain: 1 } });
        rec.stop();
        // Contract: blocked input emits a LOCAL `unavailable` notice —
        // never a gesture command (intentFromGesture drops these).
        assert.ok(rec.events.length > 0, "expected unavailable notices");
        assert.equal(
          rec.events.every(
            (e) => e.type === "unavailable" && e.reason === "disabled",
          ),
          true,
        );
        assert.equal(
          h.gateway.received.length,
          0,
          "disabled control leaked a command to the gateway",
        );
        h.unmount();
      });

      it("readOnly blocks input, sets aria-readonly, still focusable", () => {
        const h = mountShowcase(document.body, {
          flushWith: flushSync,
          themes: [theme.id],
          viz: false,
          deferredRows: false,
        });
        const col = h.columns[0]!;
        const el = col.controls.get("fader")! as HTMLElement & {
          readOnly: boolean;
          dispatchInput(i: unknown): void;
        };
        el.readOnly = true;
        assert.equal(el.getAttribute("aria-readonly"), "true");
        assert.equal(el.getAttribute("tabindex"), "0");
        const rec = collectGestures(el);
        el.dispatchInput({ type: "set", value: { plain: 1 } });
        rec.stop();
        assert.ok(rec.events.length > 0, "expected unavailable notices");
        assert.equal(
          rec.events.every(
            (e) => e.type === "unavailable" && e.reason === "readOnly",
          ),
          true,
        );
        assert.equal(
          h.gateway.received.length,
          0,
          "readOnly control leaked a command to the gateway",
        );
        h.unmount();
      });

      it("pending / invalid surface aria-busy / aria-invalid + attrs", () => {
        const h = mountShowcase(document.body, {
          flushWith: flushSync,
          themes: [theme.id],
          viz: false,
          deferredRows: false,
        });
        const col = h.columns[0]!;
        const el = col.controls.get("knob")! as HTMLElement & {
          pending: boolean;
          invalid: boolean;
        };
        el.pending = true;
        assert.equal(el.getAttribute("aria-busy"), "true");
        assert.equal(el.hasAttribute("pending"), true);
        el.pending = false;
        el.invalid = true;
        assert.equal(el.getAttribute("aria-invalid"), "true");
        assert.equal(el.hasAttribute("invalid"), true);
        h.unmount();
      });

      it("numeric input exposes a spinbutton with value attrs", () => {
        const h = mountShowcase(document.body, {
          flushWith: flushSync,
          themes: [theme.id],
          viz: false,
          deferredRows: false,
        });
        const col = h.columns[0]!;
        const el = col.controls.get("numeric")!;
        const spin = el.shadowRoot!.querySelector('[role="spinbutton"]');
        assert.ok(spin !== null, "spinbutton input missing");
        assert.equal(spin.getAttribute("aria-valuemin"), "-60");
        assert.equal(spin.getAttribute("aria-valuemax"), "6");
        assert.ok(spin.getAttribute("aria-valuenow") !== null);
        assert.ok(spin.getAttribute("aria-valuetext") !== null);
        h.unmount();
      });

      it("range selector exposes two focusable slider thumbs", () => {
        const h = mountShowcase(document.body, {
          flushWith: flushSync,
          themes: [theme.id],
          viz: false,
          deferredRows: false,
        });
        const col = h.columns[0]!;
        const el = col.controls.get("range")!;
        for (const edge of ["lo", "hi"]) {
          const thumb = el.shadowRoot!.querySelector(
            `[part~="thumb-${edge}"]`,
          );
          assert.ok(thumb !== null, `thumb-${edge} missing`);
          assert.equal(thumb.getAttribute("role"), "slider", edge);
          assert.equal(thumb.getAttribute("tabindex"), "0", edge);
          assert.ok(thumb.getAttribute("aria-valuenow") !== null, edge);
          assert.ok(thumb.getAttribute("aria-valuetext") !== null, edge);
        }
        h.unmount();
      });

      it("transport buttons carry labels + aria-disabled capability", () => {
        const h = mountShowcase(document.body, {
          flushWith: flushSync,
          themes: [theme.id],
          viz: false,
          deferredRows: false,
        });
        const col = h.columns[0]!;
        const el = col.controls.get("transport")! as HTMLElement & {
          disabled: boolean;
        };
        const play = el.shadowRoot!.querySelector('[part~="play"]');
        assert.ok(play !== null, "play button missing");
        assert.ok(play.getAttribute("aria-label") !== null);
        assert.equal(play.getAttribute("aria-disabled"), "false");
        el.disabled = true;
        assert.equal(play.getAttribute("aria-disabled"), "true");
        h.unmount();
      });

      it("toggle exposes role=switch + aria-checked", () => {
        const h = mountShowcase(document.body, {
          flushWith: flushSync,
          themes: [theme.id],
          viz: false,
          deferredRows: false,
        });
        const col = h.columns[0]!;
        const t = col.controls.get("toggle")! as HTMLElement & {
          checked: boolean;
          dispatchInput(i: unknown): void;
        };
        assert.equal(t.getAttribute("role"), "switch");
        assert.equal(t.getAttribute("aria-checked"), "false");
        t.dispatchInput({ type: "set", value: { plain: 1 } });
        assert.equal(t.getAttribute("aria-checked"), "true");
        h.unmount();
      });
    });
  }

  it("reduced-motion is honored in the shadow stylesheets", () => {
    // happy-dom lacks media queries; assert the rule exists in each
    // control's static CSS so theme columns all carry it.
    const h = mountShowcase(document.body, {
      flushWith: flushSync,
      themes: ["reference-light"],
      viz: false,
      deferredRows: false,
    });
    const col = h.columns[0]!;
    for (const key_ of ["knob", "fader", "seekbar", "toggle"]) {
      const el = col.controls.get(key_)!;
      const styles = el.shadowRoot!.querySelectorAll("style");
      const css = [...styles].map((s) => s.textContent ?? "").join("\n");
      assert.match(css, /prefers-reduced-motion/, key_);
    }
    h.unmount();
  });
});
