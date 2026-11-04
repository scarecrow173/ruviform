/**
 * Element states + a11y contract on the real DOM (Issue #310 AC1).
 *
 * disabled / readOnly / pending / rejected / unsupported are
 * *distinct perceivable states*: host attribute + the ARIA mapping the
 * a11y contract publishes, plus the behavioral half — input while
 * unavailable surfaces `unavailable`, never a fabricated commit.
 */

import { test, expect } from "../src/index.ts";
import {
  control,
  drain,
  gestureDetail,
  gestureEvents,
  expectDisabled,
  expectEnabled,
  expectReadOnly,
  expectPending,
  expectSettled,
  expectInvalid,
  expectValid,
  expectUnsupported,
  expectRole,
  expectTabbable,
  expectSliderAria,
  expectFocused,
} from "../src/index.ts";

const FADER = 'rvf-fader[control-id="reference-light.fader"]';
const FADER_CID = "reference-light.fader";

test("baseline: slider role, tabbable, ARIA value mapping", async ({ rvf }) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await expectRole(fader, "slider");
  await expectTabbable(fader);
  await expectSliderAria(fader, { min: -60, max: 6, now: -12 });
  await fader.focus();
  await expectFocused(fader);
});

test("disabled: attribute+aria, not tabbable, input is `unavailable` — no begin", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.setAttr("disabled", "");
  await expectDisabled(fader);
  // Disabled removes the managed tabindex — not keyboard-reachable.
  expect(await fader.attr("tabindex")).toBeNull();

  await fader.drag(0.2, 0.8, { steps: 2 });
  const evts = await gestureEvents(session, FADER_CID);
  const types = evts.map((e) => gestureDetail(e).type);
  expect(types).toContain("unavailable");
  expect(types).not.toContain("begin");
  expect(types).not.toContain("commit");
  const un = evts.find((e) => gestureDetail(e).type === "unavailable");
  expect((un && gestureDetail(un).reason) as string).toBe("disabled");

  await fader.setAttr("disabled", null);
  await expectEnabled(fader);
});

test("readOnly: attribute+aria, tabbable still, input is `unavailable`", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.setAttr("readonly", "");
  await expectReadOnly(fader);

  await fader.drag(0.2, 0.8, { steps: 2 });
  const evts = await gestureEvents(session, FADER_CID);
  const un = evts.find((e) => gestureDetail(e).type === "unavailable");
  expect(un, "input under readonly must surface unavailable").toBeTruthy();
  expect((un && gestureDetail(un).reason) as string).toBe("readOnly");

  await fader.setAttr("readonly", null);
});

test("pending while a commit awaits receipt; rejected receipt → invalid", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.drag(0.1, 0.4, { steps: 2 });
  await expectPending(fader);

  await drain(session, "reject");
  await expectSettled(fader);
  await expectInvalid(fader);

  // The rejected commit is reverted to the last observed value —
  // the failed intent never poses as truth.
  expect(await fader.prop("value")).toBe(-12);
});

test("unsupported receipt → unsupported state (distinct from invalid)", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.drag(0.1, 0.4, { steps: 2 });
  await drain(session, "unsupported");
  await expectUnsupported(fader);
  await expectSettled(fader);
});

test("unsupported declared on the element → input unavailable, tabbable", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.setAttr("unsupported", "");
  await expectUnsupported(fader);

  await fader.drag(0.2, 0.8, { steps: 2 });
  const evts = await gestureEvents(session, FADER_CID);
  const un = evts.find((e) => gestureDetail(e).type === "unavailable");
  expect(un).toBeTruthy();
  expect((un && gestureDetail(un).reason) as string).toBe("unsupported");

  await fader.setAttr("unsupported", null);
  await expectValid(fader);
});
