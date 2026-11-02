/**
 * Headless consumer fixtures for Issue #258: the same interaction contract
 * driving a Synvelis-style control surface (DAW fader/knob/seekbar) and an
 * AudioLens-style analysis-parameter panel. No DOM or renderer is involved —
 * the fixtures feed typed inputs straight into the controllers.
 */

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  InteractionControl,
  InteractionDispatcher,
  decibelFormatter,
  type ControlValue,
  type InteractionEvent,
} from "../src/index.ts";

const types = (e: InteractionEvent[]) => e.map((x) => x.type);
const last = (e: InteractionEvent[]) => e[e.length - 1]!;

// --- Synvelis-style control surface ----------------------------------------

/**
 * Channel-strip fixture: a gain fader (amplitude domain, custom taper,
 * decibel display incl. the −∞dB special state), a pan knob, and a seekbar.
 */
function synvelisChannelStrip() {
  const dispatcher = new InteractionDispatcher();
  dispatcher.register({
    controlId: "ch1/gain",
    target: "track/ch1/gain",
    originView: "mixer",
    descriptor: {
      min: 0,
      max: 2,
      defaultValue: 1,
      mapping: { kind: "custom", id: "fader-taper" },
      unit: "dB",
    },
    ports: {
      custom: {
        // Fader taper: plain = 2·n², normalized = sqrt(amp/2).
        toNormalized: (p) => Math.sqrt(p / 2),
        fromNormalized: (n) => 2 * n * n,
      },
      format: decibelFormatter(),
    },
  });
  dispatcher.register({
    controlId: "ch1/pan",
    target: "track/ch1/pan",
    originView: "mixer",
    descriptor: { min: -1, max: 1, defaultValue: 0, step: 0.1, fineStep: 0.01 },
  });
  dispatcher.register({
    controlId: "transport/seek",
    target: "transport/position",
    originView: "arrangement",
    descriptor: { min: 0, max: 3600, unit: "s" },
  });
  return dispatcher;
}

test("Synvelis fixture: fader drag with taper, then −∞ dB at silence", () => {
  const d = synvelisChannelStrip();
  const strip = d.control("ch1/gain")!;

  // Drag the fader halfway up: plain = 2·0.5² = 0.5 amplitude ≈ −6.02 dB.
  const drag = [
    ...d.dispatch("ch1/gain", {
      type: "begin",
      gestureId: "ptr-1",
      source: "pointer",
    }),
    ...d.dispatch("ch1/gain", {
      type: "preview",
      gestureId: "ptr-1",
      value: { normalized: 0.5 },
    }),
    ...d.dispatch("ch1/gain", { type: "commit", gestureId: "ptr-1" }),
  ];
  assert.deepEqual(types(drag), ["begin", "preview", "commit"]);
  const committed = (last(drag) as { value: ControlValue }).value;
  assert.ok(Math.abs(committed.plain - 0.5) < 1e-12);
  assert.equal(committed.normalized, 0.5);
  assert.equal(strip.mapper.format(committed.plain).text, "-6.02 dB");

  // Drag to the bottom: amplitude 0 displays the explicit −∞ dB state.
  d.dispatch("ch1/gain", {
    type: "begin",
    gestureId: "ptr-2",
    source: "pointer",
  });
  d.dispatch("ch1/gain", {
    type: "preview",
    gestureId: "ptr-2",
    value: { normalized: 0 },
  });
  d.dispatch("ch1/gain", { type: "commit", gestureId: "ptr-2" });
  const bottom = strip.mapper.format(strip.getCommittedValue().plain);
  assert.deepEqual(bottom, { text: "−∞ dB", special: "negative-infinity" });
});

test("Synvelis fixture: wheel burst on pan + keyboard fine adjust", () => {
  const d = synvelisChannelStrip();

  // Wheel burst: three ticks in one gesture, committed at the end.
  d.dispatch("ch1/pan", {
    type: "adjust",
    steps: 1,
    gestureId: "wheel-1",
    source: "wheel",
  });
  d.dispatch("ch1/pan", {
    type: "adjust",
    steps: 1,
    gestureId: "wheel-1",
    source: "wheel",
  });
  const end = d.dispatch("ch1/pan", { type: "commit", gestureId: "wheel-1" });
  assert.equal(end[0]!.type, "commit");
  assert.ok(
    Math.abs((end[0] as { value: ControlValue }).value.plain - 0.2) < 1e-12,
  );

  // Keyboard: ArrowRight adjust, then Shift+fine ArrowLeft.
  d.dispatch("ch1/pan", { type: "adjust", steps: 1, source: "keyboard" });
  const fine = d.dispatch("ch1/pan", {
    type: "adjust",
    steps: -1,
    source: "keyboard",
    modifiers: { fine: true },
  });
  const committed = (last(fine) as { value: ControlValue }).value.plain;
  assert.ok(Math.abs(committed - 0.29) < 1e-12);

  // Double-click-style reset returns to the declared default.
  const reset = d.dispatch("ch1/pan", { type: "reset", source: "pointer" });
  assert.equal((last(reset) as { value: ControlValue }).value.plain, 0);
});

test("Synvelis fixture: seekbar click commits position; mid-drag unmount cancels", () => {
  const d = synvelisChannelStrip();

  // Click on the seekbar: begin at position, release commits it.
  const ev = d.dispatch("transport/seek", {
    type: "begin",
    gestureId: "seek-1",
    source: "pointer",
    at: { plain: 1250 },
  });
  assert.equal((ev[0] as { value: ControlValue }).value.plain, 1250);
  d.dispatch("transport/seek", { type: "commit", gestureId: "seek-1" });
  assert.equal(
    d.control("transport/seek")!.getCommittedValue().plain,
    1250,
  );

  // Drag interrupted by a dock move: cancel, no position change.
  d.dispatch("transport/seek", {
    type: "begin",
    gestureId: "seek-2",
    source: "pointer",
  });
  d.dispatch("transport/seek", {
    type: "preview",
    gestureId: "seek-2",
    value: { plain: 3000 },
  });
  const ev2 = d.dispatch("transport/seek", {
    type: "detach",
    reason: "dock-transfer",
  });
  assert.equal(ev2[0]!.type, "cancel");
  assert.equal(
    (ev2[0] as { reason: string }).reason,
    "dock-transfer",
  );
  assert.equal(
    d.control("transport/seek")!.getCommittedValue().plain,
    1250,
  );
});

// --- AudioLens-style analysis parameter panel -------------------------------

/**
 * Analysis-parameter fixture: a threshold in dBFS with 0.5 steps, a
 * log-mapped frequency, a readOnly diagnostic output, and a parameter the
 * current adapter cannot drive (unsupported).
 */
function audioLensParamPanel() {
  const dispatcher = new InteractionDispatcher();
  dispatcher.register({
    controlId: "analysis/threshold",
    target: "analysis/job/thresholdDbfs",
    originView: "param-inspector",
    descriptor: {
      min: -120,
      max: 0,
      defaultValue: -60,
      step: 0.5,
      unit: "dBFS",
    },
    ports: { parse: (t) => Number(t.replace(/dBFS?$/i, "").trim()) },
  });
  dispatcher.register({
    controlId: "analysis/min-frequency",
    target: "analysis/job/minFreqHz",
    originView: "param-inspector",
    descriptor: { min: 20, max: 20000, mapping: { kind: "log" }, unit: "Hz" },
  });
  dispatcher.register({
    controlId: "analysis/observed-floor",
    target: "analysis/result/noiseFloor",
    originView: "param-inspector",
    descriptor: { min: -144, max: 0, unit: "dBFS", readOnly: true },
  });
  dispatcher.register({
    controlId: "analysis/unsupported",
    target: "analysis/job/backend-only",
    originView: "param-inspector",
    descriptor: { min: 0, max: 1 },
    unsupported: true,
  });
  return dispatcher;
}

test("AudioLens fixture: typed text input parses and snaps to the step grid", () => {
  const d = audioLensParamPanel();
  const param = d.control("analysis/threshold")!;

  // NumericInput-style commit: parse text, then set as an atomic gesture.
  const parsed = param.mapper.parse("-45.37 dBFS");
  assert.ok(parsed.ok);
  const ev = d.dispatch("analysis/threshold", {
    type: "set",
    value: { plain: parsed.value },
    source: "keyboard",
  });
  assert.deepEqual(types(ev), ["begin", "commit"]);
  // Snapped to the 0.5 grid anchored at -120.
  const committed = (last(ev) as { value: ControlValue }).value.plain;
  assert.equal(committed, -45.5);
});

test("AudioLens fixture: log frequency knob roundtrips display text", () => {
  const d = audioLensParamPanel();
  const freq = d.control("analysis/min-frequency")!;

  d.dispatch("analysis/min-frequency", {
    type: "set",
    value: { normalized: 0.5 },
  });
  const plain = freq.getCommittedValue().plain;
  // log mapping midpoint: sqrt(20·20000) = 632.45…
  assert.ok(Math.abs(plain - Math.sqrt(20 * 20000)) / plain < 1e-9);
  assert.ok(freq.mapper.format(plain).text.endsWith("Hz"));
});

test("AudioLens fixture: readOnly and unsupported params notify, not noop", () => {
  const d = audioLensParamPanel();

  const ro = d.dispatch("analysis/observed-floor", {
    type: "set",
    value: { plain: -80 },
  });
  assert.equal(ro[0]!.type, "unavailable");
  assert.equal((ro[0] as { reason: string }).reason, "readOnly");

  const un = d.dispatch("analysis/unsupported", {
    type: "begin",
    gestureId: "g1",
    source: "pointer",
  });
  assert.equal(un[0]!.type, "unavailable");
  assert.equal((un[0] as { reason: string }).reason, "unsupported");

  // Committed values stay put — notification ≠ silent apply.
  assert.equal(
    d.control("analysis/observed-floor")!.getCommittedValue().plain,
    -144,
  );
});

test("AudioLens fixture: reject-policy param refuses out-of-range edits", () => {
  const d = new InteractionDispatcher();
  d.register({
    controlId: "analysis/confidence",
    target: "analysis/job/minConfidence",
    descriptor: { min: 0, max: 1, outOfRange: "reject" },
  });
  const ev = d.dispatch("analysis/confidence", {
    type: "set",
    value: { plain: 1.4 },
  });
  assert.equal(ev[0]!.type, "rejected");
  assert.equal((ev[0] as { reason: string }).reason, "out-of-range");
  assert.equal(
    d.control("analysis/confidence")!.getCommittedValue().plain,
    0,
  );
});

// --- Remount / strict-mode equivalent (#263 shared fixture) -----------------

test("re-mount: detach then re-register preserves the gesture contract", () => {
  const d = new InteractionDispatcher();
  const reg = () =>
    d.register({
      controlId: "ch1/gain",
      target: "track/ch1/gain",
      descriptor: { min: 0, max: 2, defaultValue: 1 },
    });
  reg();

  // Simulate a strict-mode unmount mid-gesture.
  d.dispatch("ch1/gain", {
    type: "begin",
    gestureId: "g-strict",
    source: "pointer",
  });
  const detached = d.unregister("ch1/gain");
  assert.equal(detached[0]!.type, "cancel");

  // Re-mounted adapter re-registers and drives the same contract.
  reg();
  const ev = d.dispatch("ch1/gain", {
    type: "set",
    value: { normalized: 0.75 },
  });
  assert.deepEqual(types(ev), ["begin", "commit"]);
  assert.equal(d.control("ch1/gain")!.getCommittedValue().normalized, 0.75);
});
