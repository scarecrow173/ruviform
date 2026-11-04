/**
 * Gesture contract on the real-DOM showcase (Issue #310 AC1).
 *
 * `rvf-fader` in the Reference Light column: pointer drag, wheel
 * burst (deterministic clock — the WHEEL_IDLE_MS commit is virtual),
 * keyboard atomic gestures, and the Escape-cancel path. Every gesture
 * ends in exactly one commit or cancel; the command log holds one
 * begin/commit pair per gesture; receipts ack each command once.
 */

import { test, expect } from "../src/index.ts";
import {
  control,
  drain,
  expectComposedInit,
  expectGestureSequence,
  gestureDetail,
  gestureEvents,
  commandsFor,
  expectExactlyOnceReceipts,
} from "../src/index.ts";

const FADER = 'rvf-fader[control-id="reference-light.fader"]';
const FADER_CID = "reference-light.fader";
const FADER_TARGET = "demo.reference-light.fader";

test("pointer drag → begin, previews, exactly one commit; value lands on drain(apply)", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.drag(0.1, 0.5, { steps: 3 });

  const evts = await gestureEvents(session, FADER_CID);
  expectComposedInit(evts);
  expectGestureSequence(evts, {
    terminal: "commit",
    controlId: FADER_CID,
    expectPreview: true,
  });

  // The gesture's begin carried the press position; previews moved;
  // the commit carries the final value (fader −60..+6 dB → ≈ −27).
  const seq = evts.map((e) => gestureDetail(e));
  const commit = seq.find((s) => s.type === "commit");
  const committed = (commit?.value as { plain?: number } | undefined)?.plain;
  expect(committed).toBeCloseTo(-27, 0);

  // Intents reached the gateway exactly once each (flush is
  // synchronous — no coalescing dropped previews mid-gesture).
  const cmds = await commandsFor(session, { controlId: FADER_CID });
  const kinds = cmds.map((c) => c.intent.kind);
  expect(kinds.filter((k) => k === "begin")).toHaveLength(1);
  expect(kinds.filter((k) => k === "commit")).toHaveLength(1);
  expect(kinds.filter((k) => k === "preview").length).toBeGreaterThan(0);

  // Pending while the commit awaits its receipt; applied on drain.
  expect(await fader.attr("pending")).not.toBeNull();
  const n = await drain(session, "apply");
  expect(n).toBe(cmds.length);
  await expectExactlyOnceReceipts(session, FADER_CID);
  expect(await fader.attr("pending")).toBeNull();
  expect(await session.valueOf(FADER_TARGET)).toBeCloseTo(-27, 0);
  expect(await session.revisionOf(FADER_TARGET)).toBe(1);
});

test("wheel burst → one gesture, committed after the idle window (virtual clock)", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.wheel(-120);
  await fader.wheel(-120);
  // Nothing commits until the 200 ms idle window — and the clock is
  // paused, so only advanceClock can fire it.
  let evts = await gestureEvents(session, FADER_CID);
  expect(evts.filter((e) => gestureDetail(e).type === "commit")).toHaveLength(0);

  await session.advanceClock(250);

  evts = await gestureEvents(session, FADER_CID);
  expectGestureSequence(evts, {
    terminal: "commit",
    controlId: FADER_CID,
  });
});

test("keyboard: each key press is one atomic begin+commit", async ({ rvf }) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.key("ArrowUp");
  let evts = await gestureEvents(session, FADER_CID);
  expectGestureSequence(evts, {
    terminal: "commit",
    controlId: FADER_CID,
    expectPreview: false,
  });
  const first = evts.map((e) => gestureDetail(e));
  expect(first[0]?.meta?.atomic).toBe(true);

  await fader.key("End");
  evts = await gestureEvents(session, FADER_CID);
  const last = evts.map((e) => gestureDetail(e)).at(-1);
  expect(last?.type).toBe("commit");
  expect((last?.value as { plain?: number } | undefined)?.plain).toBe(6);
});

test("Escape during a drag cancels; cancel never reaches the command log as a commit", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);

  await fader.dragBegin(0.1);
  await fader.dragMoveTo(0.6, 2);
  await fader.key("Escape");
  await fader.dragEnd(); // released after cancel — stale commit is rejected, never applied

  const evts = await gestureEvents(session, FADER_CID);
  expectGestureSequence(evts, {
    terminal: "cancel",
    controlId: FADER_CID,
    expectPreview: true,
  });

  // No commit intent: truth must not move.
  const cmds = await commandsFor(session, {
    controlId: FADER_CID,
    kind: "commit",
  });
  expect(cmds).toHaveLength(0);
  expect(await session.valueOf(FADER_TARGET)).toBe(-12);
});

test("a cancelled gesture leaves no pending state on the host", async ({
  rvf,
}) => {
  const session = await rvf.open("showcase");
  const fader = control(session, FADER);
  await fader.dragBegin(0.3);
  await fader.key("Escape");
  await fader.dragEnd();
  expect(await fader.attr("data-previewing")).toBeNull();
  expect(await fader.attr("pending")).toBeNull();
});
