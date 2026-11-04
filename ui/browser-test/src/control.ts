/**
 * `control` — real pointer/keyboard/focus drivers over a mounted
 * Ruviform element (Issue #310).
 *
 * Playwright locators pierce shadow DOM, so a spec names the element
 * (`rvf-fader[data-control="fader"]`) and the gesture surface inside
 * its shadow (`[part~="track"]`, `button[data-action="play"]`, …)
 * without knowing the shadow structure beyond the published `part`
 * contract.
 *
 * Drivers map normalized `[0,1]` positions onto the surface's real
 * bounding box — the same geometry the components' own input adapter
 * uses — so a "drag to 0.75" means 75% of the track, not pixels.
 */

import type { Locator } from "@playwright/test";
import type { HarnessSession } from "./harness.ts";

/** A mounted Ruviform element a spec is driving. */
export interface ControlHandle {
  /** The host element locator (`page.locator(sel)`). */
  readonly host: Locator;
  /** Locator for a node inside the element's shadow (`[part~="x"]`). */
  part(part: string): Locator;
  /**
   * One absolute-mode drag: press at `from`, move to `to` in `steps`
   * moves, release — the real pointerdown/move/up sequence the
   * element's gesture adapter sees (begin → preview×N → commit).
   */
  drag(from: number, to: number, opts?: DragOptions): Promise<void>;
  /** Vertical-drag relative mode (knob): press at center, drag by `dyPx`. */
  dragBy(dyPx: number, opts?: DragOptions): Promise<void>;
  /** Begin an absolute-mode drag without committing (pointer stays down). */
  dragBegin(at: number, surface?: string): Promise<void>;
  /** Continue an in-progress drag (`dragBegin` must have run). */
  dragMoveTo(at: number, steps?: number): Promise<void>;
  /** Release the pointer (commit) after `dragBegin`/`dragMoveTo`. */
  dragEnd(): Promise<void>;
  /**
   * One wheel tick over the control (`deltaY` sign = direction).
   * Hovers the gesture surface — the wheel listener lives on the
   * track/dial part, not the host center.
   */
  wheel(
    deltaY: number,
    opts?: { shiftKey?: boolean; surface?: string },
  ): Promise<void>;
  /** Double-click the gesture surface (reset gesture). */
  dblclick(): Promise<void>;
  /** Focus the element's keyboard target, then press `key`. */
  key(key: string): Promise<void>;
  /** Focus the element's keyboard target (`Tab`-reachable contract). */
  focus(): Promise<void>;
  /** Typed property read/write on the host element. */
  prop<T = unknown>(name: string): Promise<T>;
  setProp(name: string, value: unknown): Promise<void>;
  /** Attribute read/write on the host element. */
  attr(name: string): Promise<string | null>;
  setAttr(name: string, value: string | null): Promise<void>;
  /** CSS selector used to build this handle. */
  readonly selector: string;
}

export interface DragOptions {
  /** Intermediate move events (default 3 — at least one preview). */
  readonly steps?: number;
  /** Shadow part to drag on (default `"track"`, knob uses `"dial"`). */
  readonly surface?: string;
}

const DEFAULT_SURFACE = "track";

function normalizedPoint(
  box: { x: number; y: number; width: number; height: number },
  n: number,
): { x: number; y: number } {
  // Track geometry maps n=1 to the far edge along the control's main
  // axis. Value controls in the showcase are vertical (fader) or
  // horizontal (seekbar/range) — pick the longer axis, which is the
  // gesture axis for both reference skins.
  if (box.height >= box.width) {
    return { x: box.x + box.width / 2, y: box.y + box.height * (1 - n) };
  }
  return { x: box.x + box.width * n, y: box.y + box.height / 2 };
}

/** Build a {@link ControlHandle} for the host matched by `sel`. */
export function control(
  session: HarnessSession,
  sel: string,
): ControlHandle {
  const page = session.page;
  const host = page.locator(sel);
  let activeSurface = DEFAULT_SURFACE;

  const part = (p: string): Locator =>
    host.locator(`[part~="${p}"]`).first();

  const surfaceBox = async (surface: string) => {
    const el = part(surface);
    await el.waitFor({ state: "attached" });
    const box = await el.boundingBox();
    if (box === null) {
      throw new Error(`control(${sel}): part "${surface}" has no box`);
    }
    return box;
  };

  const handle: ControlHandle = {
    host,
    selector: sel,
    part,

    async drag(from, to, opts = {}) {
      const surface = opts.surface ?? DEFAULT_SURFACE;
      const box = await surfaceBox(surface);
      const steps = Math.max(1, opts.steps ?? 3);
      const p0 = normalizedPoint(box, from);
      const p1 = normalizedPoint(box, to);
      await page.mouse.move(p0.x, p0.y);
      await page.mouse.down();
      for (let i = 1; i <= steps; i++) {
        await page.mouse.move(
          p0.x + ((p1.x - p0.x) * i) / steps,
          p0.y + ((p1.y - p0.y) * i) / steps,
        );
      }
      await page.mouse.up();
    },

    async dragBy(dyPx, opts = {}) {
      const surface = opts.surface ?? "dial";
      const box = await surfaceBox(surface);
      const cx = box.x + box.width / 2;
      const cy = box.y + box.height / 2;
      const steps = Math.max(1, opts.steps ?? 3);
      await page.mouse.move(cx, cy);
      await page.mouse.down();
      for (let i = 1; i <= steps; i++) {
        await page.mouse.move(cx, cy + (dyPx * i) / steps);
      }
      await page.mouse.up();
    },

    async dragBegin(at, surface = DEFAULT_SURFACE) {
      activeSurface = surface;
      const box = await surfaceBox(surface);
      const p = normalizedPoint(box, at);
      await page.mouse.move(p.x, p.y);
      await page.mouse.down();
    },

    async dragMoveTo(at, steps = 1) {
      // Re-resolve the active surface's box so the driver works across
      // theme/resize changes mid-gesture.
      const box = await surfaceBox(activeSurface);
      const p = normalizedPoint(box, at);
      await page.mouse.move(p.x, p.y, { steps: Math.max(1, steps) });
    },

    async dragEnd() {
      await page.mouse.up();
    },

    async wheel(deltaY, opts = {}) {
      const box = await surfaceBox(opts.surface ?? DEFAULT_SURFACE);
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      if (opts.shiftKey === true) await page.keyboard.down("Shift");
      try {
        await page.mouse.wheel(0, deltaY);
      } finally {
        if (opts.shiftKey === true) await page.keyboard.up("Shift");
      }
    },

    async dblclick() {
      const el = part(DEFAULT_SURFACE);
      await el.waitFor({ state: "attached" });
      const box = await el.boundingBox();
      if (box === null) throw new Error(`control(${sel}): no surface box`);
      await page.mouse.dblclick(
        box.x + box.width / 2,
        box.y + box.height / 2,
      );
    },

    async key(key) {
      await handle.focus();
      await page.keyboard.press(key);
    },

    async focus() {
      // The host element carries tabindex="0" (managed by the a11y
      // contract) — focusing the host is the keyboard entry point.
      await host.focus();
    },

    prop<T = unknown>(name: string): Promise<T> {
      return session.prop(sel, name) as Promise<T>;
    },
    setProp(name: string, value: unknown): Promise<void> {
      return session.setProp(sel, name, value);
    },
    attr(name: string): Promise<string | null> {
      return host.getAttribute(name);
    },
    async setAttr(name: string, value: string | null): Promise<void> {
      await host.evaluate(
        (el, [n, v]) => {
          if (v === null) el.removeAttribute(n);
          else el.setAttribute(n, v);
        },
        [name, value] as const,
      );
    },
  };

  return handle;
}
