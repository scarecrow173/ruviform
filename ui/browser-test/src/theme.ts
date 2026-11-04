/**
 * Theme / Shadow-Part inspection (Issue #310).
 *
 * Themes are data (`rvf-theme.theme` = ThemeInput) resolved into CSS
 * custom properties on the scope — these helpers change the theme and
 * read the resolved token so a spec can prove a swap is visible in
 * the same way a consumer sees it (computed style), without pixel
 * oracles.
 */

import type { Locator } from "@playwright/test";
import type { HarnessSession } from "./harness.ts";
import type { ControlHandle } from "./control.ts";

/** Assign `rvf-theme.theme` (typed property, serialized ThemeInput). */
export async function setTheme(
  session: HarnessSession,
  scopeSel: string,
  theme: unknown,
): Promise<void> {
  await session.setProp(scopeSel, "theme", theme);
}

/** Read `theme` back off an `rvf-theme` scope. */
export function themeOf(
  session: HarnessSession,
  scopeSel: string,
): Promise<unknown> {
  return session.prop(scopeSel, "theme");
}

/**
 * Computed value of a CSS custom property (design token) resolved on
 * `locator`'s element — e.g. `--rvf-color-accent` after a theme swap.
 * Returns the trimmed value (may be "" for an unknown token).
 */
export async function tokenValue(
  locator: Locator,
  token: string,
): Promise<string> {
  return locator.evaluate(
    (el, name) =>
      getComputedStyle(el as HTMLElement).getPropertyValue(name).trim(),
    token,
  );
}

/** Shadow `part` contract probe — `el::part(name)` exists in the shadow. */
export async function partExists(c: ControlHandle, part: string): Promise<boolean> {
  return (await c.part(part).count()) > 0;
}

/**
 * A `::part()`-exposed node's computed style value — the sanctioned
 * cross-shadow styling boundary a consumer skin uses.
 */
export async function partStyleValue(
  c: ControlHandle,
  part: string,
  cssProp: string,
): Promise<string> {
  return c.part(part).evaluate(
    (el, prop) => getComputedStyle(el as HTMLElement).getPropertyValue(prop),
    cssProp,
  );
}
