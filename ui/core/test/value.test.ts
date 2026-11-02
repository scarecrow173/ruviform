import { test } from "node:test";
import assert from "node:assert/strict";

import {
  DescriptorValidationError,
  ValueMapper,
  decibelFormatter,
  validateDescriptor,
  type CustomValueMapping,
  type DescriptorIssueKind,
  type ValueDescriptor,
} from "../src/index.ts";
import { mulberry32, range } from "./helpers.ts";

const LINEAR: ValueDescriptor = { min: -72, max: 6 };
const LOG_FREQ: ValueDescriptor = {
  min: 20,
  max: 20000,
  mapping: { kind: "log" },
};

/** Smooth fader-style custom mapping: plain = amp^2 shape over [0, 2]. */
const AMP_FADER: CustomValueMapping = {
  toNormalized: (plain) => Math.sqrt(Math.max(0, plain) / 2),
  fromNormalized: (n) => 2 * n * n,
};

test("descriptor validation accepts a minimal valid descriptor", () => {
  assert.deepEqual(validateDescriptor({ min: 0, max: 1 }), []);
});

test("descriptor validation reports every structural violation", () => {
  const cases: Array<[ValueDescriptor, DescriptorIssueKind]> = [
    [{ min: Number.NaN, max: 1 }, "non-finite-field"],
    [{ min: 0, max: Number.POSITIVE_INFINITY }, "non-finite-field"],
    [{ min: 5, max: 1 }, "range-inverted"],
    [{ min: 0, max: 1, defaultValue: 2 }, "default-out-of-range"],
    [{ min: 0, max: 1, step: 0 }, "step-invalid"],
    [{ min: 0, max: 1, step: -2 }, "step-invalid"],
    [{ min: 0, max: 1, step: Number.NaN }, "step-invalid"],
    [{ min: 0, max: 1, fineStep: -0.1 }, "fine-step-invalid"],
    [
      { min: 0, max: 20000, mapping: { kind: "log" } },
      "log-non-positive-endpoint",
    ],
    [{ min: 7, max: 7 }, "degenerate-not-explicit"],
  ];
  for (const [desc, expected] of cases) {
    const kinds = validateDescriptor(desc).map((i) => i.kind);
    assert.ok(
      kinds.includes(expected),
      `expected ${expected} in ${JSON.stringify(kinds)}`,
    );
  }
});

test("degenerate range is legal as explicit constant or readOnly", () => {
  assert.deepEqual(
    validateDescriptor({ min: 7, max: 7, constant: true }),
    [],
  );
  assert.deepEqual(
    validateDescriptor({ min: 7, max: 7, readOnly: true }),
    [],
  );
});

test("constructor throws DescriptorValidationError with issues", () => {
  assert.throws(
    () => new ValueMapper({ min: 2, max: 1 }),
    (e: unknown) =>
      e instanceof DescriptorValidationError &&
      e.issues.some((i) => i.kind === "range-inverted"),
  );
});

test("custom mapping requires a port implementation", () => {
  assert.throws(
    () => new ValueMapper({ min: 0, max: 2, mapping: { kind: "custom" } }),
    (e: unknown) =>
      e instanceof DescriptorValidationError &&
      e.issues.some((i) => i.kind === "custom-mapping-missing"),
  );
});

test("linear mapping roundtrips and pins endpoints (seeded)", () => {
  const m = new ValueMapper(LINEAR);
  const rand = mulberry32(0x258);
  for (let i = 0; i < 500; i += 1) {
    const n = rand();
    const r = m.resolveNormalized(n);
    assert.ok(r.ok);
    assert.ok(Math.abs(r.normalized - n) < 1e-12);
    const back = m.resolvePlain(r.plain);
    assert.ok(back.ok);
    assert.ok(Math.abs(back.normalized - n) < 1e-12);
  }
  const lo = m.resolveNormalized(0);
  const hi = m.resolveNormalized(1);
  assert.ok(lo.ok && hi.ok);
  assert.equal(lo.plain, -72);
  assert.equal(hi.plain, 6);
});

test("log mapping roundtrips and pins endpoints (seeded)", () => {
  const m = new ValueMapper(LOG_FREQ);
  const rand = mulberry32(0xbeef);
  for (let i = 0; i < 500; i += 1) {
    const n = rand();
    const r = m.resolveNormalized(n);
    assert.ok(r.ok);
    const back = m.resolvePlain(r.plain);
    assert.ok(back.ok);
    assert.ok(Math.abs(back.normalized - n) < 1e-9);
  }
  const lo = m.resolveNormalized(0);
  const hi = m.resolveNormalized(1);
  assert.ok(lo.ok && hi.ok);
  assert.ok(Math.abs(lo.plain - 20) < 1e-9);
  assert.ok(Math.abs(hi.plain - 20000) < 1e-6);
  assert.equal(m.verifyMapping().length, 0);
});

test("custom mapping roundtrips and passes verifyMapping (seeded)", () => {
  const m = new ValueMapper(
    { min: 0, max: 2, mapping: { kind: "custom", id: "amp-fader" } },
    { custom: AMP_FADER },
  );
  const rand = mulberry32(0xfade);
  for (let i = 0; i < 500; i += 1) {
    const n = rand();
    const r = m.resolveNormalized(n);
    assert.ok(r.ok);
    const back = m.resolvePlain(r.plain);
    assert.ok(back.ok);
    assert.ok(Math.abs(back.normalized - n) < 1e-12);
  }
  assert.equal(m.verifyMapping().length, 0);
});

test("verifyMapping detects non-monotonic and non-invertible mappings", () => {
  const bad: CustomValueMapping = {
    toNormalized: (p) => p,
    fromNormalized: (n) => (n < 0.5 ? 1 - n : n), // non-monotonic, non-invertible
  };
  const m = new ValueMapper(
    { min: 0, max: 1, mapping: { kind: "custom" } },
    { custom: bad },
  );
  const kinds = m.verifyMapping().map((v) => v.kind);
  assert.ok(kinds.includes("non-monotonic"));
  assert.ok(kinds.includes("non-invertible"));
});

test("verifyMapping detects endpoint mismatch", () => {
  const off: CustomValueMapping = {
    toNormalized: (p) => (p - 1) / 9,
    fromNormalized: (n) => 1 + n * 9, // maps [0,1] to [1,10], not [0,10]
  };
  const m = new ValueMapper(
    { min: 0, max: 10, mapping: { kind: "custom" } },
    { custom: off },
  );
  const kinds = m.verifyMapping().map((v) => v.kind);
  assert.ok(kinds.includes("endpoint-mismatch"));
});

test("non-finite inputs are rejected in every entry point", () => {
  const m = new ValueMapper(LINEAR);
  for (const bad of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
    const a = m.resolvePlain(bad);
    const b = m.resolveNormalized(bad);
    const c = m.toNormalized(bad);
    const d = m.toPlain(bad);
    assert.ok(!a.ok && a.error.kind === "non-finite");
    assert.ok(!b.ok && b.error.kind === "non-finite");
    assert.ok(!c.ok && c.error.kind === "non-finite");
    assert.ok(!d.ok && d.error.kind === "non-finite");
  }
  const parsed = m.parse("not-a-number");
  assert.ok(!parsed.ok && parsed.error.kind === "non-finite");
  const adj = m.adjustNormalized(Number.NaN, 1, false);
  assert.ok(!adj.ok && adj.error.kind === "non-finite");
});

test("finite out-of-range clamps by default and rejects under reject policy", () => {
  const clamping = new ValueMapper(LINEAR);
  const c = clamping.resolvePlain(100);
  assert.ok(c.ok && c.plain === 6 && c.normalized === 1);
  const c2 = clamping.resolveNormalized(1.5);
  assert.ok(c2.ok && c2.normalized === 1);

  const rejecting = new ValueMapper({ ...LINEAR, outOfRange: "reject" });
  const r = rejecting.resolvePlain(100);
  assert.ok(!r.ok && r.error.kind === "out-of-range");
  const r2 = rejecting.resolveNormalized(-0.1);
  assert.ok(!r2.ok && r2.error.kind === "out-of-range");
  // In-range values are unaffected.
  assert.ok(rejecting.resolvePlain(0).ok);
});

test("plain-domain step anchors at min and snaps idempotently (seeded)", () => {
  const m = new ValueMapper({ min: -72, max: 6, step: 6 });
  const rand = mulberry32(0x51e9);
  for (let i = 0; i < 300; i += 1) {
    const v = range(rand, -72, 6);
    const snapped = m.snapPlain(v);
    // Snapped value sits on the min + k*step grid.
    const k = Math.round((snapped - -72) / 6);
    assert.ok(Math.abs(snapped - (-72 + k * 6)) < 1e-9);
    assert.ok(snapped >= -72 && snapped <= 6);
    assert.equal(m.snapPlain(snapped), snapped); // idempotent
  }
});

test("normalized-domain step quantizes normalized, not plain", () => {
  const m = new ValueMapper({
    min: 0,
    max: 1,
    step: 0.25,
    stepDomain: "normalized",
  });
  assert.equal(m.snapNormalized(0.26), 0.25);
  assert.equal(m.snapNormalized(0.4), 0.5);
  // Plain values are untouched by a normalized-domain step.
  assert.equal(m.snapPlain(0.26), 0.26);
  const r = m.resolveNormalized(0.26);
  assert.ok(r.ok);
  assert.equal(r.normalized, 0.25);
});

test("step in plain domain applies to resolved values (seeded)", () => {
  const m = new ValueMapper({ min: 0, max: 10, step: 0.5 });
  const rand = mulberry32(0xa11);
  for (let i = 0; i < 200; i += 1) {
    const r = m.resolvePlain(range(rand, 0, 10));
    assert.ok(r.ok);
    const k = Math.round(r.plain / 0.5);
    assert.ok(Math.abs(r.plain - k * 0.5) < 1e-9);
  }
});

test("adjustNormalized moves by step quanta and honors fine step", () => {
  const m = new ValueMapper({ min: 0, max: 10, step: 0.5, fineStep: 0.1 });
  const up = m.adjustNormalized(0.5, 1, false);
  assert.ok(up.ok && up.plain === 5.5);
  const down = m.adjustNormalized(0.5, -2, false);
  assert.ok(down.ok && down.plain === 4);
  const fine = m.adjustNormalized(0.5, 1, true);
  assert.ok(fine.ok && Math.abs(fine.plain - 5.1) < 1e-12);
});

test("adjustNormalized without a declared step uses the normalized fallback grid", () => {
  const m = new ValueMapper({ min: 0, max: 1 });
  const r = m.adjustNormalized(0.5, 1, false);
  assert.ok(r.ok && Math.abs(r.normalized - 0.51) < 1e-12);
  const f = m.adjustNormalized(0.5, 1, true);
  assert.ok(f.ok && Math.abs(f.normalized - 0.501) < 1e-12);
});

test("plain-domain step adjustment works under a log mapping", () => {
  const m = new ValueMapper({
    min: 20,
    max: 20000,
    step: 10,
    stepDomain: "plain",
    mapping: { kind: "log" },
  });
  const base = m.resolvePlain(1000);
  assert.ok(base.ok);
  const up = m.adjustNormalized(base.normalized, 1, false);
  assert.ok(up.ok && up.plain === 1010);
});

test("degenerate range maps to a single normalized endpoint", () => {
  const m = new ValueMapper({ min: 7, max: 7, constant: true });
  const r = m.resolvePlain(7);
  assert.ok(r.ok && r.normalized === 0 && r.plain === 7);
  const r2 = m.resolveNormalized(0.5);
  assert.ok(r2.ok && r2.plain === 7);
});

test("default resolution: explicit default else min", () => {
  const withDefault = new ValueMapper({ min: -1, max: 1, defaultValue: 0.25 });
  assert.equal(withDefault.defaultPlain(), 0.25);
  assert.ok(withDefault.hasDefault());
  const without = new ValueMapper({ min: -1, max: 1 });
  assert.equal(without.defaultPlain(), -1);
  assert.ok(!without.hasDefault());
});

test("format: unit suffix, custom formatter, display-only rounding", () => {
  const m = new ValueMapper({ min: 0, max: 1, unit: "Hz" });
  assert.equal(m.format(0.5).text, "0.5 Hz");
  const noisy = new ValueMapper({ min: 0, max: 1 });
  // Float noise is collapsed at the display boundary only.
  assert.equal(noisy.format(0.1 + 0.2).text, "0.3");
  const custom = new ValueMapper(
    { min: 0, max: 1 },
    { format: (p) => ({ text: `~${p.toFixed(1)}~` }) },
  );
  assert.deepEqual(custom.format(0.5), { text: "~0.5~" });
});

test("decibelFormatter renders amplitude 0 as explicit negative-infinity", () => {
  const fmt = decibelFormatter();
  const negInf = fmt(0);
  assert.deepEqual(negInf, { text: "−∞ dB", special: "negative-infinity" });
  // Non-positive plain inputs also produce the special state.
  const alsoNeg = fmt(-0.5);
  assert.deepEqual(alsoNeg, { text: "−∞ dB", special: "negative-infinity" });
  const unity = fmt(1);
  assert.deepEqual(unity, "0.00 dB");
  const six = fmt(2);
  assert.equal(typeof six, "string");
  assert.ok((six as string).startsWith("6.02"));
});

test("decibelFormatter honors negInfBelowDb threshold", () => {
  const fmt = decibelFormatter({ negInfBelowDb: -120 });
  const tiny = fmt(Math.pow(10, -7)); // -140 dB
  assert.deepEqual(tiny, { text: "−∞ dB", special: "negative-infinity" });
  const loud = fmt(Math.pow(10, -3)); // -60 dB
  assert.deepEqual(loud, "-60.00 dB");
});

test("parse uses custom parser and validates finiteness", () => {
  const m = new ValueMapper(
    { min: -120, max: 0 },
    { parse: (t) => Number(t.replace("dB", "").trim()) },
  );
  const ok = m.parse("-45.5 dB");
  assert.ok(ok.ok && ok.value === -45.5);
  const bad = m.parse("loud");
  assert.ok(!bad.ok && bad.error.kind === "non-finite");
});

test("non-finite custom-mapping output is an invalid-mapping defect", () => {
  const broken: CustomValueMapping = {
    toNormalized: () => Number.NaN,
    fromNormalized: () => 0,
  };
  const m = new ValueMapper(
    { min: 0, max: 1, mapping: { kind: "custom" } },
    { custom: broken },
  );
  const r = m.resolvePlain(0.5);
  assert.ok(!r.ok && r.error.kind === "invalid-mapping");
});

test("out-of-range mapping outputs are invalid-mapping defects, not committed", () => {
  const overshoot: CustomValueMapping = {
    toNormalized: () => 1.5,
    fromNormalized: () => 500,
  };
  const m = new ValueMapper(
    { min: 0, max: 1, mapping: { kind: "custom" } },
    { custom: overshoot },
  );
  const a = m.resolvePlain(0.5);
  assert.ok(!a.ok && a.error.kind === "invalid-mapping");
  const b = m.resolveNormalized(0.5);
  assert.ok(!b.ok && b.error.kind === "invalid-mapping");
  const kinds = m.verifyMapping().map((v) => v.kind);
  assert.ok(kinds.includes("output-out-of-range"));
});

test("throwing port callbacks degrade instead of propagating", () => {
  const throwing: CustomValueMapping = {
    toNormalized: () => {
      throw new Error("boom");
    },
    fromNormalized: () => {
      throw new Error("boom");
    },
  };
  const m = new ValueMapper(
    { min: 0, max: 1, mapping: { kind: "custom" } },
    {
      custom: throwing,
      format: () => {
        throw new Error("fmt");
      },
      parse: () => {
        throw new Error("parse");
      },
    },
  );
  const r = m.resolvePlain(0.5);
  assert.ok(!r.ok && r.error.kind === "invalid-mapping");
  // Formatter falls back to default text instead of throwing.
  assert.equal(m.format(0.5).text, "0.5");
  const p = m.parse("x");
  assert.ok(!p.ok && p.error.kind === "non-finite");
});

test("log mapping stays finite near a zero-adjacent lower endpoint", () => {
  const m = new ValueMapper({
    min: 1e-4,
    max: 1e4,
    mapping: { kind: "log" },
  });
  const rand = mulberry32(0x109);
  for (let i = 0; i < 300; i += 1) {
    const n = rand();
    const r = m.resolveNormalized(n);
    assert.ok(r.ok, `resolveNormalized(${n}) failed`);
    const back = m.resolvePlain(r.plain);
    assert.ok(back.ok);
    assert.ok(Math.abs(back.normalized - n) < 1e-9);
  }
  const lo = m.resolveNormalized(0);
  assert.ok(lo.ok && Math.abs(lo.plain - 1e-4) < 1e-12);
  assert.equal(m.verifyMapping().length, 0);
});

test("normalized input on a plain-stepped control lands on the step grid", () => {
  const m = new ValueMapper({ min: 0, max: 10, step: 0.5 });
  // Raw geometry would give 0.26 → 2.6; the plain-domain step snaps it.
  const r = m.resolveNormalized(0.26);
  assert.ok(r.ok);
  const k = Math.round(r.plain / 0.5);
  assert.ok(Math.abs(r.plain - k * 0.5) < 1e-9);
});

test("plain input on a normalized-stepped control lands on the step grid", () => {
  const m = new ValueMapper({
    min: 0,
    max: 1,
    step: 0.25,
    stepDomain: "normalized",
  });
  const r = m.resolvePlain(0.26);
  assert.ok(r.ok);
  assert.equal(r.normalized, 0.25);
});
