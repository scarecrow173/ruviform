/**
 * DAW-shaped consumer fixture (Issue #310 AC4 — "Consumer A").
 *
 * Same harness, different shape: `mountDawFixture` mounts a mixer
 * strip (fader → `track.vol`, automation knob → `send.0.level`,
 * transport → action intents) inside one `mixer.strip.1` view. The
 * fixture is consumer-shaped data only — no DAW semantics enter the
 * ruviform API.
 */

import { test, expect } from "../src/index.ts";
import {
  control,
  drain,
  expectComposedInit,
  expectGestureSequence,
  gestureEvents,
  transportDetail,
  transportEvents,
  commandsFor,
  expectExactlyOnceReceipts,
  expectNoDuplicateCommands,
} from "../src/index.ts";

const FADER = 'rvf-fader[data-control="fader"]';
const KNOB = 'rvf-knob[data-control="automation"]';
const TRANSPORT = 'rvf-transport[data-control="transport"]';

test("fader drag commits to track.vol exactly once", async ({ rvf }) => {
  const session = await rvf.open("daw");
  const fader = control(session, FADER);

  await fader.drag(0.2, 0.6, { steps: 4 });
  const evts = await gestureEvents(session, "mixer.vol");
  expectComposedInit(evts);
  expectGestureSequence(evts, { terminal: "commit", controlId: "mixer.vol" });
  await expectNoDuplicateCommands(session, "mixer.vol", ["begin", "commit"]);

  await drain(session, "apply");
  await expectExactlyOnceReceipts(session, "mixer.vol");
  // Absolute-mode commit: plain = −60 + 0.6·66 ≈ −20.4 dB.
  expect(await session.valueOf("track.vol")).toBeCloseTo(-20.4, 0);
});

test("automation knob: relative drag + fine wheel", async ({ rvf }) => {
  const session = await rvf.open("daw");
  const knob = control(session, KNOB);

  await knob.dragBy(-30, { steps: 3 }); // 30px up over 150px span → +0.2 normalized
  const evts = await gestureEvents(session, "mixer.send0");
  expectGestureSequence(evts, { terminal: "commit", controlId: "mixer.send0" });

  await drain(session, "apply");
  expect(await session.valueOf("send.0.level")).toBeCloseTo(0.7, 1);
});

test("transport buttons emit one action intent each (disabled action blocked)", async ({
  rvf,
}) => {
  const session = await rvf.open("daw");
  const t = control(session, TRANSPORT);

  await t.part("play").click();
  await t.part("stop").click();

  const evts = await transportEvents(session, "transport");
  expectComposedInit(evts);
  expect(evts.map((e) => transportDetail(e).action)).toEqual(["play", "stop"]);

  // Action intents are commands — one per activation, receipt-required.
  const cmds = await commandsFor(session, { controlId: "transport" });
  expect(cmds.map((c) => c.intent.action)).toEqual(["play", "stop"]);
  await drain(session, "accept");
  await expectExactlyOnceReceipts(session, "transport");

  // The fixture declares resume unsupported — its button is a real
  // <button disabled>, so a click produces no intent at all.
  await t.part("resume").click({ force: true });
  const after = await transportEvents(session, "transport");
  expect(after.map((e) => transportDetail(e).action)).toEqual(["play", "stop"]);
});
