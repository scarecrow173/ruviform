/**
 * Accessibility assertions (Issue #310).
 *
 * DOM-level a11y contract checks: role, keyboard reachability
 * (tabindex), and the ARIA value mapping the components publish —
 * `aria-valuemin/max/now/valuetext` for sliders, `aria-disabled`,
 * `aria-readonly`, `aria-invalid`, `aria-busy`. These are semantic
 * DOM checks; an automated pass here is not a screen-reader pass
 * (per the design — OS assistive tech is #313's boundary).
 */

import { expect } from "@playwright/test";
import type { ControlHandle } from "./control.ts";

/** The element's effective ARIA role on its a11y target. */
export async function expectRole(
  c: ControlHandle,
  role: string,
): Promise<void> {
  const r = await c.attr("role");
  expect(r, `${c.selector} role should be "${role}"`).toBe(role);
}

/** The element is keyboard-focusable (managed `tabindex="0"`). */
export async function expectTabbable(c: ControlHandle): Promise<void> {
  expect(await c.attr("tabindex"), `${c.selector} tabindex`).toBe("0");
}

/** `aria-valuemin/max/now` and `aria-valuetext` reflect `descriptor`. */
export async function expectSliderAria(
  c: ControlHandle,
  opts: { min?: number; max?: number; now?: number; text?: string },
): Promise<void> {
  if (opts.min !== undefined) {
    expect(await c.attr("aria-valuemin")).toBe(String(opts.min));
  }
  if (opts.max !== undefined) {
    expect(await c.attr("aria-valuemax")).toBe(String(opts.max));
  }
  if (opts.now !== undefined) {
    expect(await c.attr("aria-valuenow")).toBe(String(opts.now));
  }
  if (opts.text !== undefined) {
    expect(await c.attr("aria-valuetext")).toBe(opts.text);
  }
}

/** Focus actually lands on the control (`document.activeElement`). */
export async function expectFocused(c: ControlHandle): Promise<void> {
  const active = await c.host.evaluate((el) => {
    const ae = el.ownerDocument.activeElement;
    return ae === el || (ae !== null && el.contains(ae));
  });
  expect(active, `${c.selector} should hold focus`).toBe(true);
}
