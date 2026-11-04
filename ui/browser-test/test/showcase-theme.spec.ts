/**
 * Theme / Shadow-Part contract on the real DOM (Issue #310 AC1+2).
 *
 * The five showcase columns render the same fixture under different
 * `rvf-theme` scopes: token values must differ where the themes do,
 * a `theme` property swap must visibly re-resolve, and `::part()`
 * surfaces must exist inside every control shadow — the sanctioned
 * cross-shadow styling boundary.
 */

import { test, expect } from "../src/index.ts";
import {
  control,
  setTheme,
  themeOf,
  tokenValue,
  partExists,
  partStyleValue,
} from "../src/index.ts";

const LIGHT_SCOPE = 'rvf-theme[data-theme="reference-light"]';
const DARK_SCOPE = 'rvf-theme[data-theme="reference-dark"]';
const LIGHT_FADER = 'rvf-fader[control-id="reference-light.fader"]';
const LIGHT_KNOB = 'rvf-knob[control-id="reference-light.knob"]';
const DARK_FADER = 'rvf-fader[control-id="reference-dark.fader"]';

test("reference themes resolve different surface tokens; controls inherit", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const light = session.page.locator(LIGHT_SCOPE);
  const dark = session.page.locator(DARK_SCOPE);

  const lightSurface = await tokenValue(light, "--rvf-surface");
  const darkSurface = await tokenValue(dark, "--rvf-surface");
  expect(lightSurface).not.toBe("");
  expect(darkSurface).not.toBe("");
  expect(lightSurface).not.toBe(darkSurface);

  // The token resolves through the scope into the control's shadow
  // part — theme inheritance is what consumers skin through ::part().
  const lightFader = control(session, LIGHT_FADER);
  const trackLight = await partStyleValue(lightFader, "track", "background-color");
  const darkFader = control(session, DARK_FADER);
  const trackDark = await partStyleValue(darkFader, "track", "background-color");
  expect(trackLight).not.toBe(trackDark);
});

test("theme property swap re-resolves tokens in place", async ({ rvf }) => {
  const session = await rvf.open("showcase");
  const scope = session.page.locator(LIGHT_SCOPE);
  const before = await tokenValue(scope, "--rvf-surface");

  const theme = (await themeOf(session, LIGHT_SCOPE)) as {
    tokens?: Record<string, unknown>;
  };
  await setTheme(session, LIGHT_SCOPE, {
    ...theme,
    tokens: { ...(theme.tokens ?? {}), surface: "#112233" },
  });

  const after = await tokenValue(scope, "--rvf-surface");
  expect(after).not.toBe(before);
});

test("every published part surface exists inside the control shadows", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, LIGHT_FADER);
  for (const p of ["track", "fill", "thumb", "readout", "label"]) {
    expect(await partExists(fader, p), `fader part "${p}"`).toBe(true);
  }
  const knob = control(session, LIGHT_KNOB);
  for (const p of ["dial", "arc", "arc-track", "indicator", "readout"]) {
    expect(await partExists(knob, p), `knob part "${p}"`).toBe(true);
  }
});
