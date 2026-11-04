/**
 * Element state assertions (Issue #310): disabled / readOnly /
 * pending / rejected / unsupported / invalid, checked as the pair the
 * component contract promises — the host attribute AND the ARIA
 * mapping, plus (where observable) that input does not take effect.
 */

import { expect } from "@playwright/test";
import type { ControlHandle } from "./control.ts";

/** `attr` is present with any value (boolean-attribute semantics). */
export async function expectFlagOn(
  c: ControlHandle,
  attr: string,
): Promise<void> {
  expect(await c.attr(attr), `${c.selector} missing "${attr}"`).not.toBeNull();
}

export async function expectFlagOff(
  c: ControlHandle,
  attr: string,
): Promise<void> {
  expect(await c.attr(attr), `${c.selector} has "${attr}"`).toBeNull();
}

/** `aria-*` attribute on the host equals `value`. */
export async function expectAria(
  c: ControlHandle,
  name: string,
  value: string,
): Promise<void> {
  expect(
    await c.attr(name),
    `${c.selector} ${name} should be ${value}`,
  ).toBe(value);
}

/** disabled — attribute + `aria-disabled="true"`. */
export async function expectDisabled(c: ControlHandle): Promise<void> {
  await expectFlagOn(c, "disabled");
  await expectAria(c, "aria-disabled", "true");
}

export async function expectEnabled(c: ControlHandle): Promise<void> {
  await expectFlagOff(c, "disabled");
  const v = await c.attr("aria-disabled");
  expect(v === null || v === "false").toBe(true);
}

/** readOnly — attribute + `aria-readonly="true"`. */
export async function expectReadOnly(c: ControlHandle): Promise<void> {
  await expectFlagOn(c, "readonly");
  await expectAria(c, "aria-readonly", "true");
}

/** pending — receipt in flight (`aria-busy="true"` mirrors it). */
export async function expectPending(c: ControlHandle): Promise<void> {
  await expectFlagOn(c, "pending");
  await expectAria(c, "aria-busy", "true");
}

export async function expectSettled(c: ControlHandle): Promise<void> {
  await expectFlagOff(c, "pending");
  const v = await c.attr("aria-busy");
  expect(v === null || v === "false").toBe(true);
}

/** rejected/invalid — `invalid` attr + `aria-invalid="true"`. */
export async function expectInvalid(c: ControlHandle): Promise<void> {
  await expectFlagOn(c, "invalid");
  await expectAria(c, "aria-invalid", "true");
}

export async function expectValid(c: ControlHandle): Promise<void> {
  await expectFlagOff(c, "invalid");
  const v = await c.attr("aria-invalid");
  expect(v === null || v === "false").toBe(true);
}

/** unsupported — declared incapability. */
export async function expectUnsupported(c: ControlHandle): Promise<void> {
  await expectFlagOn(c, "unsupported");
}
