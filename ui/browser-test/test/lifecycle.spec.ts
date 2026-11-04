/**
 * Lifecycle / remount contract (Issue #310 AC2).
 *
 * Full `unmount()` → `mount()` cycles on the DAW fixture — the
 * StrictMode-equivalent scenario: teardown must release subscriptions
 * and in-flight gestures exactly once, remount must restore a working
 * element without double-subscribing, double-dispatching commands, or
 * leaking the previous gateway's listeners.
 */

import { test, expect } from "../src/index.ts";
import {
  control,
  gestureEvents,
  commandsFor,
  drain,
} from "../src/index.ts";

const FADER = 'rvf-fader[data-control="fader"]';

test("unmount unsubscribes every view subscription exactly once", async ({
  rvf,
}) => {
  const session = await rvf.open("lifecycle");
  const subs = (await session.subscribeCalls()) as unknown[];
  expect(subs.length).toBeGreaterThan(0);

  await session.unmount();

  // Every subscription the mount opened is released — the count
  // pairs 1:1 with the fresh gateway's subscribeCalls.
  expect(await session.unsubscribed()).toBe(subs.length);
  // The fixture is gone from the DOM — not merely hidden.
  await expect(session.page.locator(FADER)).toHaveCount(0);
});

test("remount restores a working element — one gesture, one command", async ({
  rvf,
}) => {
  const session = await rvf.open("lifecycle");
  await session.unmount();
  await session.mount();

  const fader = control(session, FADER);
  await fader.drag(0.2, 0.6, { steps: 3 });

  const evts = await gestureEvents(session, "mixer.vol");
  const begins = evts.filter(
    (e) => (e.detail as { type: string }).type === "begin",
  );
  // Exactly one begin for this gesture — a stacked listener or a
  // resurrected binding would double it.
  expect(begins).toHaveLength(1);
  const commits = evts.filter(
    (e) => (e.detail as { type: string }).type === "commit",
  );
  expect(commits).toHaveLength(1);

  // Fresh gateway per mount — its log holds this gesture's intents
  // exactly once (no replayed history from the old mount).
  const cmds = await commandsFor(session, { controlId: "mixer.vol" });
  const commitCmds = cmds.filter((c) => c.intent.kind === "commit");
  expect(commitCmds).toHaveLength(1);

  await drain(session, "apply");
  expect(await session.valueOf("track.vol")).toBeCloseTo(-20.4, 0);
});

test("second remount cycle also cleans up (no accumulating listeners)", async ({
  rvf,
}) => {
  const session = await rvf.open("lifecycle");
  await session.unmount();
  await session.mount();
  const subs = (await session.subscribeCalls()) as unknown[];
  await session.unmount();
  expect(await session.unsubscribed()).toBe(subs.length);
});
