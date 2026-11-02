import { test } from "node:test";
import assert from "node:assert/strict";

import {
  InteractionControl,
  InteractionDispatcher,
  type ControlValue,
  type InteractionEvent,
  type InteractionEventMeta,
} from "../src/index.ts";

const LINEAR = { min: 0, max: 100 };

function make(extra?: Partial<ConstructorParameters<typeof InteractionControl>[0]>) {
  return new InteractionControl({
    controlId: "knob-1",
    descriptor: LINEAR,
    target: "track/3/volume",
    originView: "mixer-a",
    ...extra,
  });
}

const types = (events: InteractionEvent[]) => events.map((e) => e.type);

function sequences(events: InteractionEvent[]): number[] {
  return events.map((e) => e.meta.sequence);
}

function begin(c: InteractionControl, id = "g1", at?: { normalized: number }) {
  return c.dispatch({
    type: "begin",
    gestureId: id,
    source: "pointer",
    originView: "mixer-a",
    ...(at === undefined ? {} : { at }),
  });
}

test("pointer drag: begin -> preview* -> commit, meta carried through", () => {
  const c = make();
  const all: InteractionEvent[] = [];
  all.push(...begin(c, "g-drag"));
  assert.equal(c.getPhase(), "active");
  assert.equal(c.getActiveGestureId(), "g-drag");

  all.push(
    ...c.dispatch({
      type: "preview",
      gestureId: "g-drag",
      value: { normalized: 0.25 },
    }),
  );
  all.push(
    ...c.dispatch({
      type: "preview",
      gestureId: "g-drag",
      value: { normalized: 0.8 },
    }),
  );
  assert.equal(c.getValue().plain, 80);

  all.push(...c.dispatch({ type: "commit", gestureId: "g-drag" }));
  assert.deepEqual(types(all), ["begin", "preview", "preview", "commit"]);
  assert.equal(c.getPhase(), "idle");
  assert.deepEqual(c.getCommittedValue(), { plain: 80, normalized: 0.8 });

  const meta: InteractionEventMeta[] = all.map((e) => e.meta);
  for (const m of meta) {
    assert.equal(m.controlId, "knob-1");
    assert.equal(m.gestureId, "g-drag");
    assert.equal(m.target, "track/3/volume");
    assert.equal(m.originView, "mixer-a");
  }
  const seq = sequences(all);
  assert.deepEqual(seq, [1, 2, 3, 4]);
  assert.equal(all[0]!.meta.source, "pointer");
  // Only previews are coalescible.
  assert.equal((all[1] as { coalescible?: boolean }).coalescible, true);
  assert.equal((all[0] as { coalescible?: boolean }).coalescible, undefined);
  assert.equal((all[3] as { coalescible?: boolean }).coalescible, undefined);
});

test("commit is exactly-once: second commit and cancel are already-terminated", () => {
  const c = make();
  begin(c);
  c.dispatch({ type: "commit", gestureId: "g1" });

  const again = c.dispatch({ type: "commit", gestureId: "g1" });
  assert.equal(again.length, 1);
  assert.equal(again[0]!.type, "rejected");
  assert.equal(
    (again[0] as { reason: string }).reason,
    "already-terminated",
  );

  const cancel = c.dispatch({ type: "cancel", gestureId: "g1" });
  assert.equal(cancel[0]!.type, "rejected");
  assert.equal(
    (cancel[0] as { reason: string }).reason,
    "already-terminated",
  );
  // Committed state untouched.
  assert.deepEqual(c.getCommittedValue(), { plain: 0, normalized: 0 });
});

test("commit after cancel is impossible; committed state stays", () => {
  const c = make();
  begin(c);
  c.dispatch({
    type: "preview",
    gestureId: "g1",
    value: { normalized: 0.9 },
  });
  const cancel = c.dispatch({ type: "cancel", gestureId: "g1", reason: "escape" });
  assert.equal(cancel[0]!.type, "cancel");
  assert.equal((cancel[0] as { reason: string }).reason, "escape");
  assert.equal(c.getPhase(), "idle");
  // Preview discarded; committed value unchanged.
  assert.deepEqual(c.getCommittedValue(), { plain: 0, normalized: 0 });
  assert.deepEqual(c.getValue(), { plain: 0, normalized: 0 });

  const late = c.dispatch({ type: "commit", gestureId: "g1" });
  assert.equal(late[0]!.type, "rejected");
});

test("duplicate begin on same control is explicitly rejected", () => {
  const c = make();
  begin(c, "g-first");
  const dup = c.dispatch({
    type: "begin",
    gestureId: "g-second",
    source: "touch",
  });
  assert.equal(dup[0]!.type, "rejected");
  assert.equal(
    (dup[0] as { reason: string }).reason,
    "gesture-already-active",
  );
  // The original gesture continues untouched.
  assert.equal(c.getActiveGestureId(), "g-first");
  const ok = c.dispatch({
    type: "preview",
    gestureId: "g-first",
    value: { normalized: 0.5 },
  });
  assert.equal(ok[0]!.type, "preview");

  const sameId = c.dispatch({
    type: "begin",
    gestureId: "g-first",
    source: "pointer",
  });
  assert.equal(
    (sameId[0] as { reason: string }).reason,
    "gesture-already-active",
  );
});

test("begin with a terminated gesture id is already-terminated", () => {
  const c = make();
  begin(c, "g1");
  c.dispatch({ type: "commit", gestureId: "g1" });
  const re = c.dispatch({ type: "begin", gestureId: "g1", source: "pointer" });
  assert.equal(
    (re[0] as { reason: string }).reason,
    "already-terminated",
  );
});

test("stale preview for terminated gesture is rejected; unknown id is noop", () => {
  const c = make();
  begin(c, "g1");
  c.dispatch({ type: "commit", gestureId: "g1" });

  const stale = c.dispatch({
    type: "preview",
    gestureId: "g1",
    value: { normalized: 0.5 },
  });
  assert.equal(stale[0]!.type, "rejected");
  assert.equal((stale[0] as { reason: string }).reason, "stale-gesture");

  const unknown = c.dispatch({
    type: "preview",
    gestureId: "never-seen",
    value: { normalized: 0.5 },
  });
  assert.equal(unknown[0]!.type, "noop");
  assert.equal(
    (unknown[0] as { reason: string }).reason,
    "no-active-gesture",
  );
});

test("invalid preview input is rejected without disturbing the gesture", () => {
  const c = make();
  begin(c);
  const bad = c.dispatch({
    type: "preview",
    gestureId: "g1",
    value: { normalized: Number.NaN },
  });
  assert.equal(bad[0]!.type, "rejected");
  assert.equal(
    (bad[0] as { reason: string }).reason,
    "non-finite-input",
  );
  assert.equal(c.getPhase(), "active");
  const good = c.dispatch({
    type: "preview",
    gestureId: "g1",
    value: { normalized: 0.4 },
  });
  assert.equal(good[0]!.type, "preview");
});

test("commit with invalid value is rejected; gesture stays active", () => {
  const c = make();
  begin(c);
  c.dispatch({
    type: "preview",
    gestureId: "g1",
    value: { normalized: 0.7 },
  });
  const bad = c.dispatch({
    type: "commit",
    gestureId: "g1",
    value: { plain: Number.POSITIVE_INFINITY },
  });
  assert.equal(bad[0]!.type, "rejected");
  assert.equal(c.getPhase(), "active");
  // A commit without a value lands the last preview.
  const ok = c.dispatch({ type: "commit", gestureId: "g1" });
  assert.equal(ok[0]!.type, "commit");
  assert.equal((ok[0] as { value: ControlValue }).value.plain, 70);
});

test("begin with `at` resolves the starting value (seekbar click)", () => {
  const c = make();
  const ev = begin(c, "g-seek", { normalized: 0.6 });
  assert.equal(ev[0]!.type, "begin");
  assert.equal((ev[0] as { value: ControlValue }).value.normalized, 0.6);
});

test("focus-lost cancels with reason lost-focus", () => {
  const c = make();
  begin(c);
  c.dispatch({
    type: "preview",
    gestureId: "g1",
    value: { normalized: 0.3 },
  });
  const ev = c.dispatch({ type: "focus-lost" });
  assert.equal(ev[0]!.type, "cancel");
  assert.equal((ev[0] as { reason: string }).reason, "lost-focus");
  assert.deepEqual(c.getCommittedValue(), { plain: 0, normalized: 0 });
  const idle = c.dispatch({ type: "focus-lost" });
  assert.equal(idle[0]!.type, "noop");
});

test("detach cancels by default, or commits under the commit policy", () => {
  const c = make();
  begin(c);
  c.dispatch({
    type: "preview",
    gestureId: "g1",
    value: { normalized: 0.65 },
  });
  const ev = c.dispatch({ type: "detach" });
  assert.equal(ev[0]!.type, "cancel");
  assert.equal((ev[0] as { reason: string }).reason, "unmount");
  assert.equal(c.getCommittedValue().plain, 0);

  const c2 = make({ detachPolicy: "commit" });
  begin(c2);
  c2.dispatch({
    type: "preview",
    gestureId: "g1",
    value: { normalized: 0.65 },
  });
  const ev2 = c2.dispatch({ type: "detach" });
  assert.equal(ev2[0]!.type, "commit");
  assert.equal((ev2[0] as { value: ControlValue }).value.normalized, 0.65);
  assert.equal(c2.getCommittedValue().normalized, 0.65);

  // Input-level policy override and custom reason.
  const c3 = make();
  begin(c3);
  const ev3 = c3.dispatch({
    type: "detach",
    policy: "commit",
    reason: "dock-transfer",
  });
  assert.equal(ev3[0]!.type, "commit");
});

test("controller survives detach: adapter re-attach resumes dispatch", () => {
  const c = make();
  begin(c);
  c.dispatch({ type: "detach" });
  // Same controller re-attached (strict-mode / remount equivalent).
  const ev = begin(c, "g-reattach");
  assert.equal(ev[0]!.type, "begin");
  c.dispatch({ type: "commit", gestureId: "g-reattach" });
  assert.equal(c.getPhase(), "idle");
});

test("atomic set emits a begin+commit pair flagged atomic", () => {
  const c = make();
  const ev = c.dispatch({
    type: "set",
    value: { plain: 42 },
    source: "keyboard",
  });
  assert.deepEqual(types(ev), ["begin", "commit"]);
  assert.equal(ev[0]!.meta.atomic, true);
  assert.equal(ev[1]!.meta.atomic, true);
  assert.equal(ev[0]!.meta.gestureId, ev[1]!.meta.gestureId);
  assert.equal((ev[1] as { value: ControlValue }).value.plain, 42);
  assert.deepEqual(c.getCommittedValue(), { plain: 42, normalized: 0.42 });
  assert.equal(c.getPhase(), "idle");
});

test("atomic adjust and reset honor steps/extreme and fine modifier", () => {
  const c = make({
    descriptor: { min: 0, max: 10, step: 0.5, fineStep: 0.1, defaultValue: 4 },
  });
  // The committed value starts at the declared default.
  assert.equal(c.getCommittedValue().plain, 4);
  const a = c.dispatch({ type: "adjust", steps: 2, source: "keyboard" });
  assert.equal((a[1] as { value: ControlValue }).value.plain, 5);
  const f = c.dispatch({
    type: "adjust",
    steps: 1,
    source: "keyboard",
    modifiers: { fine: true },
  });
  assert.ok(
    Math.abs((f[1] as { value: ControlValue }).value.plain - 5.1) < 1e-12,
  );
  const r = c.dispatch({ type: "reset" });
  assert.equal((r[1] as { value: ControlValue }).value.plain, 4);
});

test("reset without a declared default is an explicit noop", () => {
  const c = make(); // LINEAR has no defaultValue
  const ev = c.dispatch({ type: "reset" });
  assert.equal(ev[0]!.type, "noop");
  assert.equal((ev[0] as { reason: string }).reason, "no-default");
});

test("set with extreme min/max jumps to the range ends", () => {
  const c = make();
  const hi = c.dispatch({ type: "set", value: { extreme: "max" } });
  assert.equal((hi[1] as { value: ControlValue }).value.plain, 100);
  const lo = c.dispatch({ type: "set", value: { extreme: "min" } });
  assert.equal((lo[1] as { value: ControlValue }).value.plain, 0);
});

test("atomic input while a gesture is active is a conflict rejection", () => {
  const c = make();
  begin(c, "g-drag");
  const ev = c.dispatch({ type: "set", value: { plain: 50 } });
  assert.equal(ev[0]!.type, "rejected");
  assert.equal(
    (ev[0] as { reason: string }).reason,
    "gesture-already-active",
  );
  assert.equal(c.getActiveGestureId(), "g-drag");
});

test("gestureId-carrying adjust forms one wheel-burst gesture", () => {
  const c = make({ descriptor: { min: 0, max: 1, step: 0.05 } });
  const e1 = c.dispatch({
    type: "adjust",
    steps: 1,
    gestureId: "wheel-7",
    source: "wheel",
  });
  assert.deepEqual(types(e1), ["begin", "preview"]);
  assert.equal(c.getPhase(), "active");
  assert.equal((e1[1] as { value: ControlValue }).value.plain, 0.05);

  const e2 = c.dispatch({
    type: "adjust",
    steps: 2,
    gestureId: "wheel-7",
    source: "wheel",
  });
  assert.deepEqual(types(e2), ["preview"]);
  assert.ok(
    Math.abs((e2[0] as { value: ControlValue }).value.plain - 0.15) < 1e-12,
  );

  const e3 = c.dispatch({ type: "commit", gestureId: "wheel-7" });
  assert.equal(e3[0]!.type, "commit");
  assert.ok(
    Math.abs((e3[0] as { value: ControlValue }).value.plain - 0.15) < 1e-12,
  );
  assert.equal(c.getPhase(), "idle");
});

test("a wheel burst can also be cancelled mid-stream", () => {
  const c = make({ descriptor: { min: 0, max: 1, step: 0.05 } });
  c.dispatch({ type: "adjust", steps: 1, gestureId: "w", source: "wheel" });
  const ev = c.dispatch({
    type: "cancel",
    gestureId: "w",
    reason: "lost-pointer-capture",
  });
  assert.equal(ev[0]!.type, "cancel");
  assert.equal(
    (ev[0] as { reason: string }).reason,
    "lost-pointer-capture",
  );
  assert.equal(c.getCommittedValue().plain, 0);
});

test("disabled / readOnly / unsupported notify explicitly, unlike noop", () => {
  const disabled = make({ descriptor: { ...LINEAR, disabled: true } });
  // Capability flags in the descriptor take effect at construction.
  const d = disabled.dispatch({
    type: "begin",
    gestureId: "g1",
    source: "pointer",
  });
  assert.equal(d[0]!.type, "unavailable");
  assert.equal((d[0] as { reason: string }).reason, "disabled");

  const ro = make();
  ro.setAvailability({ readOnly: true });
  const r = ro.dispatch({ type: "set", value: { plain: 10 } });
  assert.equal(r[0]!.type, "unavailable");
  assert.equal((r[0] as { reason: string }).reason, "readOnly");

  const un = make({ unsupported: true });
  const u = un.dispatch({ type: "adjust", steps: 1 });
  assert.equal(u[0]!.type, "unavailable");
  assert.equal((u[0] as { reason: string }).reason, "unsupported");

  // A benign noop is a different, non-capability signal.
  const idle = make();
  const n = idle.dispatch({
    type: "preview",
    gestureId: "ghost",
    value: { normalized: 0.5 },
  });
  assert.equal(n[0]!.type, "noop");
});

test("disabling mid-gesture cancels explicitly, then notifies unavailable", () => {
  const c = make();
  begin(c);
  const ev = c.setAvailability({ disabled: true });
  assert.equal(ev.length, 1);
  assert.equal(ev[0]!.type, "cancel");
  assert.equal((ev[0] as { reason: string }).reason, "explicit");
  assert.equal(c.getPhase(), "idle");

  const next = c.dispatch({
    type: "begin",
    gestureId: "g2",
    source: "pointer",
  });
  assert.equal(next[0]!.type, "unavailable");
  assert.equal((next[0] as { reason: string }).reason, "disabled");

  c.setAvailability({ disabled: false });
  const back = c.dispatch({
    type: "begin",
    gestureId: "g3",
    source: "pointer",
  });
  assert.equal(back[0]!.type, "begin");
});

test("input seq detects duplicates and out-of-order delivery", () => {
  const c = make();
  c.dispatch({ type: "set", value: { plain: 10 }, seq: 5 });
  const dup = c.dispatch({ type: "set", value: { plain: 20 }, seq: 5 });
  assert.equal(dup[0]!.type, "noop");
  assert.equal(
    (dup[0] as { reason: string }).reason,
    "duplicate-input",
  );
  assert.equal(c.getCommittedValue().plain, 10); // not applied twice

  const late = c.dispatch({ type: "set", value: { plain: 30 }, seq: 3 });
  assert.equal(late[0]!.type, "rejected");
  assert.equal((late[0] as { reason: string }).reason, "out-of-order");
  assert.equal(c.getCommittedValue().plain, 10);

  const ok = c.dispatch({ type: "set", value: { plain: 30 }, seq: 6 });
  assert.equal(ok[1]!.type, "commit");
  assert.equal(c.getCommittedValue().plain, 30);
});

test("descriptor readOnly/disabled flags are honored from construction", () => {
  // Descriptor-declared flags flow through setAvailability-independent path:
  // the mapper keeps them, and the control treats declared disabled/readOnly
  // as unavailable from the start.
  const c = make({ descriptor: { ...LINEAR, readOnly: true } });
  const ev = c.dispatch({
    type: "begin",
    gestureId: "g1",
    source: "pointer",
  });
  assert.equal(ev[0]!.type, "unavailable");
  assert.equal((ev[0] as { reason: string }).reason, "readOnly");
});

test("event sequence is strictly increasing across a long session", () => {
  const c = make();
  const all: InteractionEvent[] = [];
  for (let i = 0; i < 5; i += 1) {
    const id = `g-${i}`;
    all.push(...c.dispatch({ type: "begin", gestureId: id, source: "pointer" }));
    all.push(
      ...c.dispatch({
        type: "preview",
        gestureId: id,
        value: { normalized: 0.1 * i },
      }),
    );
    all.push(...c.dispatch({ type: "commit", gestureId: id }));
  }
  const seq = sequences(all);
  for (let i = 1; i < seq.length; i += 1) {
    assert.ok(seq[i]! > seq[i - 1]!);
  }
});

test("lifecycle inputs survive out-of-order seq delivery", () => {
  const c = make();
  c.dispatch({ type: "set", value: { plain: 10 }, seq: 10 });
  begin(c, "g-live");
  // A detach carrying an older seq must still terminate the gesture —
  // dropping it would let the gesture outlive its view.
  const ev = c.dispatch({ type: "detach", seq: 3 });
  assert.equal(ev[0]!.type, "cancel");
  assert.equal(c.getPhase(), "idle");
});

test("keyed detach/focus-lost only terminate the matching gesture", () => {
  const c = make();
  begin(c, "g-new");
  // A stale detach keyed to an older gesture no-ops instead of killing
  // the current one (strict-mode remount ordering).
  const stale = c.dispatch({ type: "detach", gestureId: "g-old" });
  assert.equal(stale[0]!.type, "noop");
  assert.equal(c.getPhase(), "active");
  const staleFocus = c.dispatch({
    type: "focus-lost",
    gestureId: "g-old",
  });
  assert.equal(staleFocus[0]!.type, "noop");
  assert.equal(c.getPhase(), "active");
  // Keyed match does terminate.
  const match = c.dispatch({ type: "detach", gestureId: "g-new" });
  assert.equal(match[0]!.type, "cancel");
  assert.equal(c.getPhase(), "idle");
});

test("dispatcher routes by controlId and reports unknown controls", () => {
  const d = new InteractionDispatcher();
  d.register({
    controlId: "fader-1",
    descriptor: { min: 0, max: 2 },
    target: "track/1/gain",
  });
  const ok = d.dispatch("fader-1", {
    type: "set",
    value: { normalized: 0.5 },
  });
  assert.equal(ok[1]!.type, "commit");

  const missing = d.dispatch("nope", { type: "reset" });
  assert.equal(missing[0]!.type, "rejected");
  assert.equal(
    (missing[0] as { reason: string }).reason,
    "unknown-control",
  );
});

test("unregister detaches an in-flight gesture via the detach contract", () => {
  const d = new InteractionDispatcher();
  d.register({ controlId: "c1", descriptor: LINEAR });
  d.dispatch("c1", { type: "begin", gestureId: "g1", source: "pointer" });
  const ev = d.unregister("c1");
  assert.equal(ev.length, 1);
  assert.equal(ev[0]!.type, "cancel");
  assert.equal((ev[0] as { reason: string }).reason, "unmount");
});

test("re-registering a control detaches its in-flight gesture first", () => {
  const d = new InteractionDispatcher();
  d.register({ controlId: "c1", descriptor: LINEAR });
  d.dispatch("c1", { type: "begin", gestureId: "g1", source: "pointer" });
  const ev = d.register({ controlId: "c1", descriptor: LINEAR });
  assert.equal(ev.length, 1);
  assert.equal(ev[0]!.type, "cancel");
  // New registration works cleanly afterwards.
  const ok = d.dispatch("c1", { type: "set", value: { plain: 5 } });
  assert.equal(ok[1]!.type, "commit");
});
