/**
 * Issue #258 — headless value contract.
 *
 * A {@link ValueDescriptor} is pure, JSON-serializable data: it never carries
 * functions across a boundary. Local callbacks (custom mapping, formatter,
 * parser) are supplied separately as {@link ValuePorts} when a
 * {@link ValueMapper} is constructed.
 *
 * Three value domains are kept distinct:
 * - normalized: unitless position in `[0, 1]` — what pointer geometry resolves
 *   to and what wire-level parameter protocols exchange.
 * - plain: the product-facing value in `[min, max]` with its own unit
 *   (dB, Hz, seconds, …).
 * - display: formatted text plus explicit special states (e.g. `−∞ dB` for
 *   amplitude zero). Display text is never fed back into control values.
 *
 * Finite-input rule: `NaN`/`±Infinity` inputs are always rejected, never
 * stored or silently propagated. Finite out-of-range input is clamped or
 * rejected according to the descriptor's explicit {@link OutOfRangePolicy};
 * this contract intentionally does not inherit
 * `ruviform-plugin-core::ParamValue`'s fixed clamp rule — product adapters
 * convert explicitly.
 */

/** How a finite input outside the declared range is handled. */
export type OutOfRangePolicy = "clamp" | "reject";

/** The domain a `step`/`fineStep` quantum is expressed in. */
export type StepDomain = "plain" | "normalized";

/** Unit label carried through to display text (e.g. `"dB"`, `"Hz"`). */
export type UnitLabel = string;

/**
 * Local-only custom mapping between plain and normalized domains.
 *
 * Contract (verified by {@link ValueMapper.verifyMapping}): both functions
 * are monotonic non-decreasing, finite on the whole domain, satisfy
 * `toNormalized(min)≈0` / `toNormalized(max)≈1`, and are mutually inverse
 * within tolerance. Function values never cross a serialization boundary.
 */
export interface CustomValueMapping {
  toNormalized(plain: number): number;
  fromNormalized(normalized: number): number;
}

/**
 * Mapping declared in the descriptor. `"custom"` only names the mapping —
 * the implementation is injected via {@link ValuePorts.custom}; the optional
 * `id` lets a consumer resolve which of its registered mappings is meant.
 */
export type ValueMappingSpec =
  | { readonly kind: "linear" }
  | { readonly kind: "log" }
  | { readonly kind: "custom"; readonly id?: string };

/** Explicit display state that has no finite plain representation. */
export type DisplaySpecial = "negative-infinity";

/** Formatted display value: text plus an optional typed special state. */
export interface DisplayValue {
  readonly text: string;
  readonly special?: DisplaySpecial;
}

/** Local formatter port. May return plain text or a typed {@link DisplayValue}. */
export type ValueFormatter = (plain: number) => string | DisplayValue;

/**
 * Local parser port for text (NumericInput) entry. Returns the parsed plain
 * value; returning a non-finite value is reported as a non-finite input.
 */
export type ValueParser = (text: string) => number;

/** Local-only function ports for a {@link ValueMapper}. Never serialized. */
export interface ValuePorts {
  readonly custom?: CustomValueMapping;
  readonly format?: ValueFormatter;
  readonly parse?: ValueParser;
}

/**
 * Serializable control-value descriptor.
 *
 * Invariants checked by {@link validateDescriptor}:
 * - `min`, `max`, `defaultValue`, `step`, `fineStep` are finite;
 * - `min <= max`; `defaultValue` lies in `[min, max]` when present;
 * - `step`/`fineStep` are positive when present;
 * - `mapping: "log"` requires `min > 0` (non-positive endpoints are rejected —
 *   a piecewise zero-crossing policy is out of scope for this contract);
 * - `mapping: "custom"` requires a {@link ValuePorts.custom} implementation;
 * - a degenerate `min === max` range is only legal when the control is an
 *   explicit constant (`constant`) or `readOnly`.
 */
export interface ValueDescriptor {
  readonly min: number;
  readonly max: number;
  readonly defaultValue?: number;
  readonly step?: number;
  readonly stepDomain?: StepDomain;
  readonly fineStep?: number;
  readonly fineStepDomain?: StepDomain;
  readonly mapping?: ValueMappingSpec;
  readonly outOfRange?: OutOfRangePolicy;
  readonly unit?: UnitLabel;
  readonly readOnly?: boolean;
  readonly disabled?: boolean;
  readonly constant?: boolean;
}

/** Structural violation found by {@link validateDescriptor}. */
export type DescriptorIssueKind =
  | "non-finite-field"
  | "range-inverted"
  | "default-out-of-range"
  | "step-invalid"
  | "fine-step-invalid"
  | "log-non-positive-endpoint"
  | "custom-mapping-missing"
  | "degenerate-not-explicit";

export interface DescriptorIssue {
  readonly kind: DescriptorIssueKind;
  /** Field name when the violation is attributable to one. */
  readonly field?: string;
  readonly detail: string;
}

/** Error thrown by {@link ValueMapper} on an invalid descriptor. */
export class DescriptorValidationError extends Error {
  readonly issues: readonly DescriptorIssue[];
  constructor(issues: readonly DescriptorIssue[]) {
    super(`invalid ValueDescriptor: ${issues.map((i) => i.detail).join("; ")}`);
    this.name = "DescriptorValidationError";
    this.issues = issues;
  }
}

/** Why a runtime value input was not accepted. */
export type ValueFailureKind = "non-finite" | "out-of-range" | "invalid-mapping";

export interface ValueFailure {
  readonly kind: ValueFailureKind;
  readonly detail: string;
}

export type ValueResult =
  | { readonly ok: true; readonly normalized: number; readonly plain: number }
  | { readonly ok: false; readonly error: ValueFailure };

export type ScalarResult =
  | { readonly ok: true; readonly value: number }
  | { readonly ok: false; readonly error: ValueFailure };

/** Violation reported by {@link ValueMapper.verifyMapping}. */
export type MappingViolationKind =
  | "non-monotonic"
  | "endpoint-mismatch"
  | "non-finite-output"
  | "output-out-of-range"
  | "non-invertible";

export interface MappingViolation {
  readonly kind: MappingViolationKind;
  readonly detail: string;
}

/** Default number of probe points used by {@link ValueMapper.verifyMapping}. */
export const DEFAULT_VERIFY_SAMPLES = 64;

/** Relative tolerance used by {@link ValueMapper.verifyMapping}. */
export const DEFAULT_VERIFY_TOLERANCE = 1e-6;

/**
 * Validate a descriptor's declarative invariants. An empty result means the
 * descriptor is structurally valid; runtime mapping checks for `custom`
 * mappings live in {@link ValueMapper.verifyMapping}.
 */
export function validateDescriptor(d: ValueDescriptor): DescriptorIssue[] {
  const issues: DescriptorIssue[] = [];
  const finiteField = (name: string, v: number | undefined): v is number => {
    if (v === undefined) return false;
    if (!Number.isFinite(v)) {
      issues.push({
        kind: "non-finite-field",
        field: name,
        detail: `${name} must be finite, got ${v}`,
      });
      return false;
    }
    return true;
  };

  const minOk = finiteField("min", d.min);
  const maxOk = finiteField("max", d.max);
  if (minOk && maxOk) {
    if (d.min > d.max) {
      issues.push({
        kind: "range-inverted",
        detail: `min (${d.min}) must be <= max (${d.max})`,
      });
    } else if (d.min === d.max && !(d.readOnly === true || d.constant === true)) {
      issues.push({
        kind: "degenerate-not-explicit",
        detail:
          "degenerate range (min === max) must be marked readOnly or constant",
      });
    }
  }

  if (finiteField("defaultValue", d.defaultValue) && minOk && maxOk && d.min <= d.max) {
    const dv = d.defaultValue;
    if (dv < d.min || dv > d.max) {
      issues.push({
        kind: "default-out-of-range",
        detail: `defaultValue (${dv}) must lie within [${d.min}, ${d.max}]`,
      });
    }
  }

  if (d.step !== undefined) {
    if (!Number.isFinite(d.step) || d.step <= 0) {
      issues.push({
        kind: "step-invalid",
        field: "step",
        detail: `step must be a positive finite number, got ${d.step}`,
      });
    }
  }
  if (d.fineStep !== undefined) {
    if (!Number.isFinite(d.fineStep) || d.fineStep <= 0) {
      issues.push({
        kind: "fine-step-invalid",
        field: "fineStep",
        detail: `fineStep must be a positive finite number, got ${d.fineStep}`,
      });
    }
  }

  const mapping = d.mapping ?? { kind: "linear" as const };
  if (mapping.kind === "log" && minOk && d.min <= 0) {
    issues.push({
      kind: "log-non-positive-endpoint",
      field: "min",
      detail: `log mapping requires min > 0, got ${d.min}`,
    });
  }

  return issues;
}

const normalizeDisplay = (out: string | DisplayValue): DisplayValue =>
  typeof out === "string" ? { text: out } : out;

const trimFloat = (v: number): string => {
  // Display-only rounding: collapse binary-noise digits without quantizing
  // the underlying value (rounding happens exclusively at this boundary).
  if (!Number.isFinite(v)) return String(v);
  const s = String(v);
  if (!s.includes("e") && !s.includes("E") && s.length <= 14) return s;
  const r = Math.abs(v) >= 1 ? v.toPrecision(12) : v.toPrecision(6);
  return String(Number(r));
};

/**
 * Typed value mapping for one control. Constructing a mapper validates the
 * descriptor; every conversion re-validates inputs as finite.
 */
export class ValueMapper {
  readonly descriptor: ValueDescriptor;
  private readonly custom?: CustomValueMapping;
  private readonly formatFn?: ValueFormatter;
  private readonly parseFn?: ValueParser;
  private readonly kind: "linear" | "log" | "custom";
  private readonly rangePolicy: OutOfRangePolicy;
  private readonly logMin: number = 0;
  private readonly logSpan: number = 0;

  constructor(descriptor: ValueDescriptor, ports?: ValuePorts) {
    const issues = validateDescriptor(descriptor);
    const mapping = descriptor.mapping ?? { kind: "linear" as const };
    if (mapping.kind === "custom" && ports?.custom === undefined) {
      issues.push({
        kind: "custom-mapping-missing",
        detail:
          'mapping "custom" requires a ValuePorts.custom implementation',
      });
    }
    if (issues.length > 0) throw new DescriptorValidationError(issues);

    this.descriptor = descriptor;
    this.kind = mapping.kind;
    this.rangePolicy = descriptor.outOfRange ?? "clamp";
    if (ports?.custom !== undefined) this.custom = ports.custom;
    if (ports?.format !== undefined) this.formatFn = ports.format;
    if (ports?.parse !== undefined) this.parseFn = ports.parse;
    if (this.kind === "log") {
      this.logMin = Math.log(descriptor.min);
      this.logSpan = Math.log(descriptor.max) - this.logMin;
    }
  }

  get isDegenerate(): boolean {
    return this.descriptor.min === this.descriptor.max;
  }

  /** Plain-domain value corresponding to the descriptor default (or `min`). */
  defaultPlain(): number {
    return this.descriptor.defaultValue ?? this.descriptor.min;
  }

  /** Whether an explicit `defaultValue` was declared. */
  hasDefault(): boolean {
    return this.descriptor.defaultValue !== undefined;
  }

  private fail(kind: ValueFailureKind, detail: string): ScalarResult {
    return { ok: false, error: { kind, detail } };
  }

  private applyRangePolicyPlain(plain: number): ScalarResult {
    const { min, max } = this.descriptor;
    if (plain < min || plain > max) {
      if (this.rangePolicy === "reject") {
        return this.fail(
          "out-of-range",
          `plain value ${plain} outside [${min}, ${max}] under "reject" policy`,
        );
      }
      return { ok: true, value: Math.min(max, Math.max(min, plain)) };
    }
    return { ok: true, value: plain };
  }

  private applyRangePolicyNormalized(n: number): ScalarResult {
    if (n < 0 || n > 1) {
      if (this.rangePolicy === "reject") {
        return this.fail(
          "out-of-range",
          `normalized value ${n} outside [0, 1] under "reject" policy`,
        );
      }
      return { ok: true, value: Math.min(1, Math.max(0, n)) };
    }
    return { ok: true, value: n };
  }

  /**
   * Call a port mapping function. Port callbacks that throw are treated as
   * `invalid-mapping` defects, never propagated into control dispatch.
   */
  private rawToNormalized(plain: number): number {
    const { min, max } = this.descriptor;
    switch (this.kind) {
      case "linear":
        return this.isDegenerate ? 0 : (plain - min) / (max - min);
      case "log":
        return (Math.log(plain) - this.logMin) / this.logSpan;
      case "custom":
        try {
          return this.custom!.toNormalized(plain);
        } catch {
          return Number.NaN;
        }
    }
  }

  private rawToPlain(n: number): number {
    const { min, max } = this.descriptor;
    switch (this.kind) {
      case "linear":
        return this.isDegenerate ? min : min + n * (max - min);
      case "log":
        return Math.exp(this.logMin + n * this.logSpan);
      case "custom":
        try {
          return this.custom!.fromNormalized(n);
        } catch {
          return Number.NaN;
        }
    }
  }

  /**
   * Relative tolerance for mapping-output overshoot. Float noise from a
   * well-formed mapping may exceed the range by a hair; a mapping that
   * overshoots by more than this is a defect.
   */
  private outputEpsilon(): number {
    const { min, max } = this.descriptor;
    return 1e-9 * Math.max(Math.abs(min), Math.abs(max), 1);
  }

  /**
   * Validate a mapping output against its target domain, clamping
   * epsilon-scale overshoot and flagging real violations as
   * `invalid-mapping`.
   */
  private checkNormalizedOutput(n: number, forPlain: number): ScalarResult {
    if (!Number.isFinite(n)) {
      return this.fail(
        "invalid-mapping",
        `mapping produced non-finite normalized value for plain ${forPlain}`,
      );
    }
    if (n < -1e-9 || n > 1 + 1e-9) {
      return this.fail(
        "invalid-mapping",
        `mapping produced out-of-range normalized value ${n} for plain ${forPlain}`,
      );
    }
    return { ok: true, value: Math.min(1, Math.max(0, n)) };
  }

  private checkPlainOutput(plain: number, forNormalized: number): ScalarResult {
    const { min, max } = this.descriptor;
    if (!Number.isFinite(plain)) {
      return this.fail(
        "invalid-mapping",
        `mapping produced non-finite plain value for normalized ${forNormalized}`,
      );
    }
    const eps = this.outputEpsilon();
    if (plain < min - eps || plain > max + eps) {
      return this.fail(
        "invalid-mapping",
        `mapping produced out-of-range plain value ${plain} for normalized ${forNormalized}`,
      );
    }
    return { ok: true, value: Math.min(max, Math.max(min, plain)) };
  }

  /**
   * Convert a plain-domain input to both domains. Non-finite input is
   * rejected; finite out-of-range input follows the descriptor policy; a
   * non-finite or out-of-`[0,1]` mapping output is an `invalid-mapping`
   * defect, not a user error.
   */
  toNormalized(plain: number): ValueResult {
    if (!Number.isFinite(plain)) {
      return {
        ok: false,
        error: { kind: "non-finite", detail: `plain input ${plain} is not finite` },
      };
    }
    const ranged = this.applyRangePolicyPlain(plain);
    if (!ranged.ok) return { ok: false, error: ranged.error };
    const checked = this.checkNormalizedOutput(
      this.rawToNormalized(ranged.value),
      ranged.value,
    );
    if (!checked.ok) return { ok: false, error: checked.error };
    return { ok: true, normalized: checked.value, plain: ranged.value };
  }

  /** Convert a normalized `[0,1]` input to both domains. */
  toPlain(normalized: number): ValueResult {
    if (!Number.isFinite(normalized)) {
      return {
        ok: false,
        error: {
          kind: "non-finite",
          detail: `normalized input ${normalized} is not finite`,
        },
      };
    }
    const ranged = this.applyRangePolicyNormalized(normalized);
    if (!ranged.ok) return { ok: false, error: ranged.error };
    const checked = this.checkPlainOutput(
      this.rawToPlain(ranged.value),
      ranged.value,
    );
    if (!checked.ok) return { ok: false, error: checked.error };
    return { ok: true, normalized: ranged.value, plain: checked.value };
  }

  /**
   * Quantize a plain value to the declared step when `stepDomain` is
   * `"plain"` (the default). Steps anchor at `min`. When the descriptor
   * declares a normalized-domain step, plain values pass through unchanged —
   * quantization happens in the declared domain only (no double conversion).
   */
  snapPlain(plain: number): number {
    const { step, stepDomain = "plain" } = this.descriptor;
    if (step === undefined || stepDomain !== "plain") return plain;
    return this.quantizePlain(plain, step);
  }

  /**
   * Quantize a normalized value to the declared step when `stepDomain` is
   * `"normalized"`. Steps anchor at `0`.
   */
  snapNormalized(n: number): number {
    const { step, stepDomain = "plain" } = this.descriptor;
    if (step === undefined || stepDomain !== "normalized") return n;
    return this.quantizeNormalized(n, step);
  }

  private quantizePlain(plain: number, quantum: number): number {
    const { min, max } = this.descriptor;
    const snapped = min + Math.round((plain - min) / quantum) * quantum;
    return Math.min(max, Math.max(min, snapped));
  }

  private quantizeNormalized(n: number, quantum: number): number {
    const snapped = Math.round(n / quantum) * quantum;
    return Math.min(1, Math.max(0, snapped));
  }

  /**
   * Effective adjustment quantum and the domain it applies in. `fine`
   * selects `fineStep`/`fineStepDomain` when declared, then falls back to
   * `step`; with no declared step at all the fallback is a normalized
   * 1% quantum (0.1% in fine mode).
   */
  private effectiveStep(fine: boolean): { quantum: number; domain: StepDomain } {
    if (fine && this.descriptor.fineStep !== undefined) {
      return {
        quantum: this.descriptor.fineStep,
        domain:
          this.descriptor.fineStepDomain ??
          this.descriptor.stepDomain ??
          "plain",
      };
    }
    if (this.descriptor.step !== undefined) {
      return {
        quantum: this.descriptor.step,
        domain: this.descriptor.stepDomain ?? "plain",
      };
    }
    return { quantum: fine ? 0.001 : 0.01, domain: "normalized" };
  }

  /**
   * Resolve a plain input to a consistent `{plain, normalized}` pair.
   * Order: range policy → snap in the declared step domain → convert.
   * With a normalized-domain step the typed value converts, snaps, and
   * converts back so the emitted pair is always step-consistent.
   */
  resolvePlain(plain: number): ValueResult {
    if (!Number.isFinite(plain)) {
      return {
        ok: false,
        error: { kind: "non-finite", detail: `plain input ${plain} is not finite` },
      };
    }
    const ranged = this.applyRangePolicyPlain(plain);
    if (!ranged.ok) return { ok: false, error: ranged.error };
    const snapped = this.snapPlain(ranged.value);
    const converted = this.toNormalized(snapped);
    if (!converted.ok) return converted;
    const resnapped = this.snapNormalized(converted.normalized);
    if (resnapped === converted.normalized) return converted;
    return this.toPlain(resnapped);
  }

  /**
   * Resolve a normalized input to a consistent `{plain, normalized}` pair.
   * Order: range policy → snap → convert → snap in the declared step
   * domain → convert back. A normalized-domain input on a control with a
   * plain-domain step lands on the plain step grid (pointer drags snap
   * to steps), not on raw geometry.
   */
  resolveNormalized(n: number): ValueResult {
    if (!Number.isFinite(n)) {
      return {
        ok: false,
        error: {
          kind: "non-finite",
          detail: `normalized input ${n} is not finite`,
        },
      };
    }
    const ranged = this.applyRangePolicyNormalized(n);
    if (!ranged.ok) return { ok: false, error: ranged.error };
    const snapped = this.snapNormalized(ranged.value);
    const converted = this.toPlain(snapped);
    if (!converted.ok) return converted;
    const resnapped = this.snapPlain(converted.plain);
    if (resnapped === converted.plain) return converted;
    return this.toNormalized(resnapped);
  }

  /**
   * Relative adjustment by an integral number of effective steps from a
   * normalized position. The arithmetic runs in the declared step domain —
   * a plain-domain step is added to the plain value and converted once —
   * and the result lands on the quantum grid (anchored at `min` for plain,
   * at `0` for normalized) before the range policy applies.
   */
  adjustNormalized(from: number, steps: number, fine: boolean): ValueResult {
    if (!Number.isFinite(from) || !Number.isFinite(steps)) {
      return {
        ok: false,
        error: {
          kind: "non-finite",
          detail: `adjust input (from=${from}, steps=${steps}) is not finite`,
        },
      };
    }
    const { quantum, domain } = this.effectiveStep(fine);
    if (this.isDegenerate) return this.toNormalized(this.descriptor.min);
    if (domain === "normalized") {
      const ranged = this.applyRangePolicyNormalized(from + steps * quantum);
      if (!ranged.ok) return { ok: false, error: ranged.error };
      return this.toPlain(this.quantizeNormalized(ranged.value, quantum));
    }
    const fromPlain = this.rawToPlain(from);
    if (!Number.isFinite(fromPlain)) {
      return {
        ok: false,
        error: {
          kind: "invalid-mapping",
          detail: `mapping produced non-finite plain value for normalized ${from}`,
        },
      };
    }
    const ranged = this.applyRangePolicyPlain(fromPlain + steps * quantum);
    if (!ranged.ok) return { ok: false, error: ranged.error };
    return this.toNormalized(this.quantizePlain(ranged.value, quantum));
  }

  /**
   * Format a plain-domain value for display. A throwing formatter port is
   * degraded to the default text rather than propagated into UI code.
   */
  format(plain: number): DisplayValue {
    if (this.formatFn !== undefined) {
      try {
        return normalizeDisplay(this.formatFn(plain));
      } catch {
        // fall through to the default text
      }
    }
    const { unit } = this.descriptor;
    const text = trimFloat(plain);
    return { text: unit === undefined ? text : `${text} ${unit}` };
  }

  /**
   * Parse text input into a plain value (finite-validated, un-snapped).
   * The default parser mirrors the default formatter: a trailing unit
   * suffix is stripped so the displayed `"45 %"` round-trips, and empty
   * input is a non-finite failure — `Number("") === 0` would silently
   * commit zero for a cleared field. A throwing parser port reports a
   * non-finite result.
   */
  parse(text: string): ScalarResult {
    let parsed: number;
    try {
      if (this.parseFn !== undefined) {
        parsed = this.parseFn(text);
      } else {
        let t = text.trim();
        const { unit } = this.descriptor;
        if (unit !== undefined && t.endsWith(unit)) {
          t = t.slice(0, t.length - unit.length).trim();
        }
        parsed = t === "" ? Number.NaN : Number(t);
      }
    } catch {
      return this.fail(
        "non-finite",
        `text input ${JSON.stringify(text)} parser threw`,
      );
    }
    if (!Number.isFinite(parsed)) {
      return this.fail("non-finite", `text input ${JSON.stringify(text)} did not parse to a finite value`);
    }
    return { ok: true, value: parsed };
  }

  /**
   * Probe the effective mapping for contract violations: monotonicity,
   * endpoint mapping, finite outputs, and invertibility within `tolerance`.
   * Deterministic; intended for consumer setup checks and contract tests.
   */
  verifyMapping(options?: {
    samples?: number;
    tolerance?: number;
  }): MappingViolation[] {
    const samples = Math.max(8, Math.floor(options?.samples ?? DEFAULT_VERIFY_SAMPLES));
    const tolerance = options?.tolerance ?? DEFAULT_VERIFY_TOLERANCE;
    const violations: MappingViolation[] = [];
    const { min, max } = this.descriptor;
    if (this.isDegenerate) return violations;

    const rangeEps = this.outputEpsilon();
    let prev = -Infinity;
    for (let i = 0; i <= samples; i += 1) {
      const n = i / samples;
      const plain = this.rawToPlain(n);
      if (!Number.isFinite(plain)) {
        violations.push({
          kind: "non-finite-output",
          detail: `fromNormalized(${n}) produced ${plain}`,
        });
        continue;
      }
      if (plain < min - rangeEps || plain > max + rangeEps) {
        violations.push({
          kind: "output-out-of-range",
          detail: `fromNormalized(${n}) produced ${plain} outside [${min}, ${max}]`,
        });
      }
      if (plain < prev - Math.abs(prev) * tolerance - tolerance) {
        violations.push({
          kind: "non-monotonic",
          detail: `fromNormalized decreased at normalized ${n} (${prev} -> ${plain})`,
        });
      }
      prev = plain;
    }

    const atMin = this.rawToPlain(0);
    const atMax = this.rawToPlain(1);
    const scale = Math.max(Math.abs(min), Math.abs(max), 1);
    if (Number.isFinite(atMin) && Math.abs(atMin - min) > tolerance * scale) {
      violations.push({
        kind: "endpoint-mismatch",
        detail: `fromNormalized(0) = ${atMin}, expected ≈${min}`,
      });
    }
    if (Number.isFinite(atMax) && Math.abs(atMax - max) > tolerance * scale) {
      violations.push({
        kind: "endpoint-mismatch",
        detail: `fromNormalized(1) = ${atMax}, expected ≈${max}`,
      });
    }

    for (let i = 0; i <= samples; i += 1) {
      const n = i / samples;
      const plain = this.rawToPlain(n);
      if (!Number.isFinite(plain)) continue;
      const back = this.rawToNormalized(plain);
      if (!Number.isFinite(back)) {
        violations.push({
          kind: "non-finite-output",
          detail: `toNormalized(fromNormalized(${n})=${plain}) produced ${back}`,
        });
        continue;
      }
      if (Math.abs(back - n) > Math.max(tolerance, tolerance * Math.abs(n))) {
        violations.push({
          kind: "non-invertible",
          detail: `roundtrip at normalized ${n} returned ${back}`,
        });
      }
    }
    return violations;
  }
}

/**
 * Formatter for controls whose plain domain is amplitude (or any positive
 * linear quantity shown in decibels). Amplitude `<= 0` — or below
 * `negInfBelowDb` — renders as the explicit `"negative-infinity"` special
 * state (`−∞ dB`); the finite control value itself is never `-Infinity`.
 */
export function decibelFormatter(options?: {
  decimals?: number;
  unit?: string;
  negInfBelowDb?: number;
}): ValueFormatter {
  const decimals = options?.decimals ?? 2;
  const unit = options?.unit ?? "dB";
  const negInfBelowDb = options?.negInfBelowDb ?? -Infinity;
  return (plain: number): string | DisplayValue => {
    if (!Number.isFinite(plain)) return String(plain);
    if (!(plain > 0) || 20 * Math.log10(plain) < negInfBelowDb) {
      return { text: `−∞ ${unit}`, special: "negative-infinity" };
    }
    return `${(20 * Math.log10(plain)).toFixed(decimals)} ${unit}`;
  };
}
