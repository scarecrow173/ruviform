//! Headless value / interaction / command-binding contract (Issue #387).
//!
//! Rust port of the canonical TypeScript semantics in
//! `ui/core/src/value.ts`, `ui/core/src/interaction.ts` and
//! `ui/binding/src/contract.ts`. The port carries the contract rules —
//! not the DSL or the DOM-side helpers.
//!
//! Three value domains stay distinct: `normalized` (`[0, 1]` pointer
//! geometry), `plain` (product value in `[min, max]` with a unit) and
//! `display` (formatted text, never fed back). `NaN`/`±Infinity`
//! inputs are always rejected, never stored.
//!
//! **RT contract:** non-RT only.

use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

// ── value contract ─────────────────────────────────────────────────

/// How a finite input outside the declared range is handled.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutOfRangePolicy {
    /// Clamp into the declared range.
    Clamp,
    /// Reject the input as `out-of-range`.
    Reject,
}

/// The domain a `step`/`fine_step` quantum is expressed in.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepDomain {
    /// Step is expressed in the plain (product) domain.
    Plain,
    /// Step is expressed in the normalized `[0, 1]` domain.
    Normalized,
}

/// Unit label carried through to display text (e.g. `"dB"`, `"Hz"`).
pub type UnitLabel = String;

/// Local-only custom mapping between plain and normalized domains.
///
/// Contract (verified by [`ValueMapper::verify_mapping`]): both
/// functions are monotonic non-decreasing, finite on the whole domain,
/// satisfy `to_normalized(min)≈0` / `to_normalized(max)≈1`, and are
/// mutually inverse within tolerance. Function values never cross a
/// serialization boundary.
pub trait CustomValueMapping: Send + Sync {
    /// Maps a plain-domain value to normalized `[0, 1]`.
    fn to_normalized(&self, plain: f64) -> f64;
    /// Maps a normalized `[0, 1]` value to the plain domain.
    fn to_plain(&self, normalized: f64) -> f64;
}

/// Mapping declared in the descriptor. `Custom` only names the mapping —
/// the implementation is injected via [`ValuePorts::custom`]; the
/// optional `id` lets a consumer resolve which of its registered
/// mappings is meant.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ValueMappingSpec {
    /// Linear map over `[min, max]`.
    Linear,
    /// Logarithmic map; requires `min > 0`.
    Log,
    /// Consumer-injected mapping via [`ValuePorts::custom`].
    Custom {
        /// Optional mapping identifier.
        id: Option<String>,
    },
}

/// Explicit display state that has no finite plain representation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisplaySpecial {
    /// `-inf dB`-style special state for amplitude zero.
    NegativeInfinity,
}

/// Formatted display value: text plus an optional typed special state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayValue {
    /// Display text.
    pub text: String,
    /// Typed special state when the value has no finite form.
    pub special: Option<DisplaySpecial>,
}

/// Local formatter port — text or a typed [`DisplayValue`].
pub type ValueFormatter = Arc<dyn Fn(f64) -> DisplayValue + Send + Sync>;
/// Local parser port for text entry (returns the parsed plain value).
pub type ValueParser = Arc<dyn Fn(&str) -> f64 + Send + Sync>;

/// Local-only function ports for a [`ValueMapper`]. Never serialized.
#[derive(Clone, Default)]
pub struct ValuePorts {
    /// Custom mapping implementation (required for `Custom` specs).
    pub custom: Option<Arc<dyn CustomValueMapping>>,
    /// Display formatter.
    pub format: Option<ValueFormatter>,
    /// Text parser.
    pub parse: Option<ValueParser>,
}

/// Serializable control-value descriptor.
///
/// Invariants checked by [`validate_descriptor`]:
/// - `min`, `max`, `default_value`, `step`, `fine_step` are finite;
/// - `min <= max`; `default_value` lies in `[min, max]` when present;
/// - `step`/`fine_step` are positive when present;
/// - `mapping: Log` requires `min > 0`;
/// - `mapping: Custom` requires a [`ValuePorts::custom`] implementation;
/// - a degenerate `min == max` range is only legal when the control is
///   an explicit `constant` or `read_only`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueDescriptor {
    /// Plain-domain minimum.
    pub min: f64,
    /// Plain-domain maximum.
    pub max: f64,
    /// Declared default (reset target / initial committed value).
    pub default_value: Option<f64>,
    /// Adjustment quantum in `step_domain`.
    pub step: Option<f64>,
    /// Domain `step` is expressed in (default `Plain`).
    pub step_domain: Option<StepDomain>,
    /// Fine-adjust quantum.
    pub fine_step: Option<f64>,
    /// Domain `fine_step` is expressed in (default = `step_domain`).
    pub fine_step_domain: Option<StepDomain>,
    /// Plain↔normalized mapping (default `Linear`).
    pub mapping: Option<ValueMappingSpec>,
    /// Out-of-range policy (default `Clamp`).
    pub out_of_range: Option<OutOfRangePolicy>,
    /// Unit label for display/parse round-trip.
    pub unit: Option<UnitLabel>,
    /// `read_only` controls report `UnavailableReason::ReadOnly`.
    pub read_only: bool,
    /// `disabled` controls report `UnavailableReason::Disabled`.
    pub disabled: bool,
    /// `constant` marks a deliberate degenerate range.
    pub constant: bool,
}

impl ValueDescriptor {
    /// Minimal descriptor for `min <= v <= max`.
    #[must_use]
    pub fn ranged(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            default_value: None,
            step: None,
            step_domain: None,
            fine_step: None,
            fine_step_domain: None,
            mapping: None,
            out_of_range: None,
            unit: None,
            read_only: false,
            disabled: false,
            constant: false,
        }
    }
}

/// Structural violation found by [`validate_descriptor`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorIssueKind {
    /// A numeric field is `NaN`/infinite.
    NonFiniteField,
    /// `min > max`.
    RangeInverted,
    /// `default_value` outside `[min, max]`.
    DefaultOutOfRange,
    /// `step` non-positive or non-finite.
    StepInvalid,
    /// `fine_step` non-positive or non-finite.
    FineStepInvalid,
    /// Log mapping with `min <= 0`.
    LogNonPositiveEndpoint,
    /// `Custom` mapping without an injected implementation.
    CustomMappingMissing,
    /// `min == max` without `read_only`/`constant`.
    DegenerateNotExplicit,
}

/// One descriptor violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorIssue {
    /// Violation class.
    pub kind: DescriptorIssueKind,
    /// Attributable field name when known.
    pub field: Option<String>,
    /// Human-readable detail.
    pub detail: String,
}

/// Error returned by [`ValueMapper::new`] on an invalid descriptor.
#[derive(Debug)]
pub struct DescriptorValidationError {
    /// All violations found.
    pub issues: Vec<DescriptorIssue>,
}

impl Display for DescriptorValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let details: Vec<String> = self.issues.iter().map(|i| i.detail.clone()).collect();
        write!(f, "invalid ValueDescriptor: {}", details.join("; "))
    }
}

impl std::error::Error for DescriptorValidationError {}

/// Why a runtime value input was not accepted.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueFailureKind {
    /// `NaN`/infinite input.
    NonFinite,
    /// Finite input outside the declared range under `Reject`.
    OutOfRange,
    /// Mapping produced a non-finite or out-of-domain output.
    InvalidMapping,
}

/// A runtime value failure.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueFailure {
    /// Failure class.
    pub kind: ValueFailureKind,
    /// Human-readable detail.
    pub detail: String,
}

/// Violation reported by [`ValueMapper::verify_mapping`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingViolationKind {
    /// Mapping is not monotonic non-decreasing.
    NonMonotonic,
    /// `to_plain` misses `min`/`max` at the endpoints.
    EndpointMismatch,
    /// Mapping produced a non-finite output.
    NonFiniteOutput,
    /// Mapping produced an output outside its target domain.
    OutputOutOfRange,
    /// `to_normalized` is not the inverse of `to_plain`.
    NonInvertible,
}

/// One mapping violation.
#[derive(Debug, Clone, PartialEq)]
pub struct MappingViolation {
    /// Violation class.
    pub kind: MappingViolationKind,
    /// Human-readable detail.
    pub detail: String,
}

/// Default probe count for [`ValueMapper::verify_mapping`].
pub const DEFAULT_VERIFY_SAMPLES: usize = 64;
/// Relative tolerance for [`ValueMapper::verify_mapping`].
pub const DEFAULT_VERIFY_TOLERANCE: f64 = 1e-6;

fn finite_field(name: &str, v: Option<f64>, issues: &mut Vec<DescriptorIssue>) -> bool {
    match v {
        None => false,
        Some(v) if !v.is_finite() => {
            issues.push(DescriptorIssue {
                kind: DescriptorIssueKind::NonFiniteField,
                field: Some(name.to_string()),
                detail: format!("{name} must be finite, got {v}"),
            });
            false
        }
        Some(_) => true,
    }
}

/// IEEE `==` on two finite floats — the canonical exact-equality
/// check (TypeScript `===`). Non-finite inputs never reach these
/// comparisons: every caller rejects them upstream.
#[allow(clippy::float_cmp)]
fn values_equal(a: f64, b: f64) -> bool {
    a == b
}

/// Validate a descriptor's declarative invariants. An empty result
/// means the descriptor is structurally valid.
#[must_use]
pub fn validate_descriptor(d: &ValueDescriptor) -> Vec<DescriptorIssue> {
    let mut issues = Vec::new();
    let min_ok = finite_field("min", Some(d.min), &mut issues);
    let max_ok = finite_field("max", Some(d.max), &mut issues);
    if min_ok && max_ok {
        if d.min > d.max {
            issues.push(DescriptorIssue {
                kind: DescriptorIssueKind::RangeInverted,
                field: None,
                detail: format!("min ({}) must be <= max ({})", d.min, d.max),
            });
        } else if values_equal(d.min, d.max) && !(d.read_only || d.constant) {
            issues.push(DescriptorIssue {
                kind: DescriptorIssueKind::DegenerateNotExplicit,
                field: None,
                detail: "degenerate range (min == max) must be marked readOnly or constant"
                    .to_string(),
            });
        }
    }
    if finite_field("default_value", d.default_value, &mut issues)
        && min_ok
        && max_ok
        && d.min <= d.max
    {
        let dv = d.default_value.unwrap_or(0.0);
        if dv < d.min || dv > d.max {
            issues.push(DescriptorIssue {
                kind: DescriptorIssueKind::DefaultOutOfRange,
                field: None,
                detail: format!(
                    "default_value ({dv}) must lie within [{}, {}]",
                    d.min, d.max
                ),
            });
        }
    }
    if let Some(step) = d.step
        && (!step.is_finite() || step <= 0.0)
    {
        issues.push(DescriptorIssue {
            kind: DescriptorIssueKind::StepInvalid,
            field: Some("step".to_string()),
            detail: format!("step must be a positive finite number, got {step}"),
        });
    }
    if let Some(fine) = d.fine_step
        && (!fine.is_finite() || fine <= 0.0)
    {
        issues.push(DescriptorIssue {
            kind: DescriptorIssueKind::FineStepInvalid,
            field: Some("fine_step".to_string()),
            detail: format!("fine_step must be a positive finite number, got {fine}"),
        });
    }
    let mapping = d.mapping.clone().unwrap_or(ValueMappingSpec::Linear);
    if mapping == ValueMappingSpec::Log && min_ok && d.min <= 0.0 {
        issues.push(DescriptorIssue {
            kind: DescriptorIssueKind::LogNonPositiveEndpoint,
            field: Some("min".to_string()),
            detail: format!("log mapping requires min > 0, got {}", d.min),
        });
    }
    issues
}

/// Display-only rounding: collapse binary-noise digits without
/// quantizing the underlying value.
fn trim_float(v: f64) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    let s = v.to_string();
    if !s.contains('e') && !s.contains('E') && s.len() <= 14 {
        return s;
    }
    let r = if v.abs() >= 1.0 {
        format!("{v:.12}")
    } else {
        format!("{v:.6}")
    };
    r.trim_end_matches('0').trim_end_matches('.').to_string()
}

type ValueResult = Result<(f64, f64), ValueFailure>; // (normalized, plain)
type ScalarResult = Result<f64, ValueFailure>;

/// Typed value mapping for one control. Construction validates the
/// descriptor; every conversion re-validates inputs as finite.
#[derive(Clone)]
pub struct ValueMapper {
    /// The validated descriptor.
    pub descriptor: ValueDescriptor,
    custom: Option<Arc<dyn CustomValueMapping>>,
    format_fn: Option<ValueFormatter>,
    parse_fn: Option<ValueParser>,
    kind: ValueMappingSpec,
    range_policy: OutOfRangePolicy,
    log_min: f64,
    log_span: f64,
}

impl ValueMapper {
    /// Validates `descriptor` and binds optional [`ValuePorts`].
    ///
    /// # Errors
    /// [`DescriptorValidationError`] when invariants fail or a `Custom`
    /// mapping lacks an implementation.
    pub fn new(
        descriptor: ValueDescriptor,
        ports: Option<&ValuePorts>,
    ) -> Result<Self, DescriptorValidationError> {
        let mut issues = validate_descriptor(&descriptor);
        let mapping = descriptor
            .mapping
            .clone()
            .unwrap_or(ValueMappingSpec::Linear);
        let custom = ports.and_then(|p| p.custom.clone());
        if matches!(mapping, ValueMappingSpec::Custom { .. }) && custom.is_none() {
            issues.push(DescriptorIssue {
                kind: DescriptorIssueKind::CustomMappingMissing,
                field: None,
                detail: "mapping \"custom\" requires a ValuePorts.custom implementation"
                    .to_string(),
            });
        }
        if !issues.is_empty() {
            return Err(DescriptorValidationError { issues });
        }
        let (log_min, log_span) = if mapping == ValueMappingSpec::Log {
            let lm = descriptor.min.ln();
            (lm, descriptor.max.ln() - lm)
        } else {
            (0.0, 0.0)
        };
        Ok(Self {
            range_policy: descriptor.out_of_range.unwrap_or(OutOfRangePolicy::Clamp),
            format_fn: ports.and_then(|p| p.format.clone()),
            parse_fn: ports.and_then(|p| p.parse.clone()),
            custom,
            kind: mapping,
            descriptor,
            log_min,
            log_span,
        })
    }

    /// `min == max`.
    #[must_use]
    pub fn is_degenerate(&self) -> bool {
        values_equal(self.descriptor.min, self.descriptor.max)
    }

    /// Plain-domain default (or `min` when none is declared).
    #[must_use]
    pub fn default_plain(&self) -> f64 {
        self.descriptor.default_value.unwrap_or(self.descriptor.min)
    }

    /// Whether an explicit `default_value` was declared.
    #[must_use]
    pub fn has_default(&self) -> bool {
        self.descriptor.default_value.is_some()
    }

    fn fail<T>(kind: ValueFailureKind, detail: String) -> Result<T, ValueFailure> {
        Err(ValueFailure { kind, detail })
    }

    fn apply_range_policy_plain(&self, plain: f64) -> ScalarResult {
        let (min, max) = (self.descriptor.min, self.descriptor.max);
        if plain < min || plain > max {
            if self.range_policy == OutOfRangePolicy::Reject {
                return Self::fail(
                    ValueFailureKind::OutOfRange,
                    format!("plain value {plain} outside [{min}, {max}] under \"reject\" policy"),
                );
            }
            return Ok(plain.clamp(min, max));
        }
        Ok(plain)
    }

    fn apply_range_policy_normalized(&self, n: f64) -> ScalarResult {
        if !(0.0..=1.0).contains(&n) {
            if self.range_policy == OutOfRangePolicy::Reject {
                return Self::fail(
                    ValueFailureKind::OutOfRange,
                    format!("normalized value {n} outside [0, 1] under \"reject\" policy"),
                );
            }
            return Ok(n.clamp(0.0, 1.0));
        }
        Ok(n)
    }

    fn raw_to_normalized(&self, plain: f64) -> f64 {
        let (min, max) = (self.descriptor.min, self.descriptor.max);
        match &self.kind {
            ValueMappingSpec::Linear => {
                if self.is_degenerate() {
                    0.0
                } else {
                    (plain - min) / (max - min)
                }
            }
            ValueMappingSpec::Log => (plain.ln() - self.log_min) / self.log_span,
            ValueMappingSpec::Custom { .. } => match &self.custom {
                Some(c) => c.to_normalized(plain),
                None => f64::NAN,
            },
        }
    }

    fn raw_to_plain(&self, n: f64) -> f64 {
        let min = self.descriptor.min;
        let max = self.descriptor.max;
        match &self.kind {
            ValueMappingSpec::Linear => {
                if self.is_degenerate() {
                    min
                } else {
                    min + n * (max - min)
                }
            }
            ValueMappingSpec::Log => (self.log_min + n * self.log_span).exp(),
            ValueMappingSpec::Custom { .. } => match &self.custom {
                Some(c) => c.to_plain(n),
                None => f64::NAN,
            },
        }
    }

    fn output_epsilon(&self) -> f64 {
        1e-9 * self
            .descriptor
            .min
            .abs()
            .max(self.descriptor.max.abs())
            .max(1.0)
    }

    fn check_normalized_output(n: f64, for_plain: f64) -> ScalarResult {
        if !n.is_finite() {
            return Self::fail(
                ValueFailureKind::InvalidMapping,
                format!("mapping produced non-finite normalized value for plain {for_plain}"),
            );
        }
        if !(-1e-9..=1.0 + 1e-9).contains(&n) {
            return Self::fail(
                ValueFailureKind::InvalidMapping,
                format!("mapping produced out-of-range normalized value {n} for plain {for_plain}"),
            );
        }
        Ok(n.clamp(0.0, 1.0))
    }

    fn check_plain_output(&self, plain: f64, for_normalized: f64) -> ScalarResult {
        if !plain.is_finite() {
            return Self::fail(
                ValueFailureKind::InvalidMapping,
                format!("mapping produced non-finite plain value for normalized {for_normalized}"),
            );
        }
        let eps = self.output_epsilon();
        if plain < self.descriptor.min - eps || plain > self.descriptor.max + eps {
            return Self::fail(
                ValueFailureKind::InvalidMapping,
                format!(
                    "mapping produced out-of-range plain value {plain} for normalized {for_normalized}"
                ),
            );
        }
        Ok(plain.clamp(self.descriptor.min, self.descriptor.max))
    }

    /// Convert a plain-domain input to both domains.
    pub fn to_normalized(&self, plain: f64) -> ValueResult {
        if !plain.is_finite() {
            return Self::fail(
                ValueFailureKind::NonFinite,
                format!("plain input {plain} is not finite"),
            );
        }
        let ranged = self.apply_range_policy_plain(plain)?;
        let n = Self::check_normalized_output(self.raw_to_normalized(ranged), ranged)?;
        Ok((n, ranged))
    }

    /// Convert a normalized `[0, 1]` input to both domains.
    pub fn to_plain(&self, normalized: f64) -> ValueResult {
        if !normalized.is_finite() {
            return Self::fail(
                ValueFailureKind::NonFinite,
                format!("normalized input {normalized} is not finite"),
            );
        }
        let ranged = self.apply_range_policy_normalized(normalized)?;
        let p = self.check_plain_output(self.raw_to_plain(ranged), ranged)?;
        Ok((ranged, p))
    }

    /// Quantize a plain value when `step_domain` is `Plain` (default).
    #[must_use]
    pub fn snap_plain(&self, plain: f64) -> f64 {
        match (self.descriptor.step, self.descriptor.step_domain) {
            (Some(step), None | Some(StepDomain::Plain)) => self.quantize_plain(plain, step),
            _ => plain,
        }
    }

    /// Quantize a normalized value when `step_domain` is `Normalized`.
    #[must_use]
    pub fn snap_normalized(&self, n: f64) -> f64 {
        match (self.descriptor.step, self.descriptor.step_domain) {
            (Some(step), Some(StepDomain::Normalized)) => Self::quantize_normalized(n, step),
            _ => n,
        }
    }

    fn quantize_plain(&self, plain: f64, quantum: f64) -> f64 {
        let (min, max) = (self.descriptor.min, self.descriptor.max);
        let snapped = min + ((plain - min) / quantum).round() * quantum;
        snapped.clamp(min, max)
    }

    fn quantize_normalized(n: f64, quantum: f64) -> f64 {
        let snapped = (n / quantum).round() * quantum;
        snapped.clamp(0.0, 1.0)
    }

    /// Effective adjustment quantum and the domain it applies in.
    fn effective_step(&self, fine: bool) -> (f64, StepDomain) {
        if fine && self.descriptor.fine_step.is_some() {
            return (
                self.descriptor.fine_step.unwrap_or(0.0),
                self.descriptor
                    .fine_step_domain
                    .or(self.descriptor.step_domain)
                    .unwrap_or(StepDomain::Plain),
            );
        }
        if let Some(step) = self.descriptor.step {
            return (
                step,
                self.descriptor.step_domain.unwrap_or(StepDomain::Plain),
            );
        }
        (if fine { 0.001 } else { 0.01 }, StepDomain::Normalized)
    }

    /// Resolve a plain input to a consistent `(normalized, plain)`
    /// pair: range policy → snap in the declared step domain → convert.
    pub fn resolve_plain(&self, plain: f64) -> ValueResult {
        if !plain.is_finite() {
            return Self::fail(
                ValueFailureKind::NonFinite,
                format!("plain input {plain} is not finite"),
            );
        }
        let ranged = self.apply_range_policy_plain(plain)?;
        let snapped = self.snap_plain(ranged);
        let (n, p) = self.to_normalized(snapped)?;
        let resnapped = self.snap_normalized(n);
        if values_equal(resnapped, n) {
            return Ok((n, p));
        }
        self.to_plain(resnapped)
    }

    /// Resolve a normalized input to a consistent pair (pointer drags
    /// snap to the declared step grid).
    pub fn resolve_normalized(&self, n: f64) -> ValueResult {
        if !n.is_finite() {
            return Self::fail(
                ValueFailureKind::NonFinite,
                format!("normalized input {n} is not finite"),
            );
        }
        let ranged = self.apply_range_policy_normalized(n)?;
        let snapped = self.snap_normalized(ranged);
        let (n2, p) = self.to_plain(snapped)?;
        let resnapped = self.snap_plain(p);
        if values_equal(resnapped, p) {
            return Ok((n2, p));
        }
        self.to_normalized(resnapped)
    }

    /// Relative adjustment by an integral number of effective steps
    /// from a normalized position.
    pub fn adjust_normalized(&self, from: f64, steps: f64, fine: bool) -> ValueResult {
        if !from.is_finite() || !steps.is_finite() {
            return Self::fail(
                ValueFailureKind::NonFinite,
                format!("adjust input (from={from}, steps={steps}) is not finite"),
            );
        }
        let (quantum, domain) = self.effective_step(fine);
        if self.is_degenerate() {
            return self.to_normalized(self.descriptor.min);
        }
        if domain == StepDomain::Normalized {
            let ranged = self.apply_range_policy_normalized(from + steps * quantum)?;
            return self.to_plain(Self::quantize_normalized(ranged, quantum));
        }
        let from_plain = self.raw_to_plain(from);
        if !from_plain.is_finite() {
            return Self::fail(
                ValueFailureKind::InvalidMapping,
                format!("mapping produced non-finite plain value for normalized {from}"),
            );
        }
        let ranged = self.apply_range_policy_plain(from_plain + steps * quantum)?;
        self.to_normalized(self.quantize_plain(ranged, quantum))
    }

    /// Format a plain value for display.
    #[must_use]
    pub fn format(&self, plain: f64) -> DisplayValue {
        if let Some(f) = &self.format_fn {
            return f(plain);
        }
        let text = trim_float(plain);
        let text = match &self.descriptor.unit {
            Some(unit) => format!("{text} {unit}"),
            None => text,
        };
        DisplayValue {
            text,
            special: None,
        }
    }

    /// Parse text input into a plain value (finite-validated,
    /// un-snapped). A trailing unit suffix is stripped so the displayed
    /// `"45 %"` round-trips; empty input is a non-finite failure.
    pub fn parse(&self, text: &str) -> ScalarResult {
        let parsed = if let Some(p) = &self.parse_fn {
            p(text)
        } else {
            let mut t = text.trim().to_string();
            if let Some(unit) = &self.descriptor.unit
                && t.ends_with(unit.as_str())
            {
                t.truncate(t.len() - unit.len());
                t = t.trim().to_string();
            }
            if t.is_empty() {
                f64::NAN
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        };
        if !parsed.is_finite() {
            return Self::fail(
                ValueFailureKind::NonFinite,
                format!("text input {text:?} did not parse to a finite value"),
            );
        }
        Ok(parsed)
    }

    /// Probe the effective mapping for contract violations:
    /// monotonicity, endpoint mapping, finite outputs, and
    /// invertibility within `tolerance`. Deterministic.
    ///
    /// `samples` is a small bound (≥8; callers pass ≤ a few thousand),
    /// far below the `f64` integer-precision limit.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn verify_mapping(
        &self,
        samples: Option<usize>,
        tolerance: Option<f64>,
    ) -> Vec<MappingViolation> {
        let samples = samples.unwrap_or(DEFAULT_VERIFY_SAMPLES).max(8);
        let tolerance = tolerance.unwrap_or(DEFAULT_VERIFY_TOLERANCE);
        let mut violations = Vec::new();
        let (min, max) = (self.descriptor.min, self.descriptor.max);
        if self.is_degenerate() {
            return violations;
        }
        let range_eps = self.output_epsilon();
        let mut prev = f64::NEG_INFINITY;
        for i in 0..=samples {
            let n = i as f64 / samples as f64;
            let plain = self.raw_to_plain(n);
            if !plain.is_finite() {
                violations.push(MappingViolation {
                    kind: MappingViolationKind::NonFiniteOutput,
                    detail: format!("to_plain({n}) produced {plain}"),
                });
                continue;
            }
            if plain < min - range_eps || plain > max + range_eps {
                violations.push(MappingViolation {
                    kind: MappingViolationKind::OutputOutOfRange,
                    detail: format!("to_plain({n}) produced {plain} outside [{min}, {max}]"),
                });
            }
            if plain < prev - prev.abs() * tolerance - tolerance {
                violations.push(MappingViolation {
                    kind: MappingViolationKind::NonMonotonic,
                    detail: format!("to_plain decreased at normalized {n} ({prev} -> {plain})"),
                });
            }
            prev = plain;
        }
        let at_min = self.raw_to_plain(0.0);
        let at_max = self.raw_to_plain(1.0);
        let scale = min.abs().max(max.abs()).max(1.0);
        if at_min.is_finite() && (at_min - min).abs() > tolerance * scale {
            violations.push(MappingViolation {
                kind: MappingViolationKind::EndpointMismatch,
                detail: format!("to_plain(0) = {at_min}, expected ≈{min}"),
            });
        }
        if at_max.is_finite() && (at_max - max).abs() > tolerance * scale {
            violations.push(MappingViolation {
                kind: MappingViolationKind::EndpointMismatch,
                detail: format!("to_plain(1) = {at_max}, expected ≈{max}"),
            });
        }
        for i in 0..=samples {
            let n = i as f64 / samples as f64;
            let plain = self.raw_to_plain(n);
            if !plain.is_finite() {
                continue;
            }
            let back = self.raw_to_normalized(plain);
            if !back.is_finite() {
                violations.push(MappingViolation {
                    kind: MappingViolationKind::NonFiniteOutput,
                    detail: format!("to_normalized(to_plain({n})={plain}) produced {back}"),
                });
                continue;
            }
            if (back - n).abs() > tolerance.max(tolerance * n.abs()) {
                violations.push(MappingViolation {
                    kind: MappingViolationKind::NonInvertible,
                    detail: format!("roundtrip at normalized {n} returned {back}"),
                });
            }
        }
        violations
    }
}

/// Formatter for controls whose plain domain is amplitude (or any
/// positive linear quantity shown in decibels). Amplitude `<= 0` — or
/// below `neg_inf_below_db` — renders as `NegativeInfinity`.
#[must_use]
pub fn decibel_formatter(decimals: usize, unit: &str, neg_inf_below_db: f64) -> ValueFormatter {
    let unit = unit.to_string();
    Arc::new(move |plain: f64| {
        if !plain.is_finite() {
            return DisplayValue {
                text: plain.to_string(),
                special: None,
            };
        }
        if plain <= 0.0 || 20.0 * plain.log10() < neg_inf_below_db {
            return DisplayValue {
                text: format!("−∞ {unit}"),
                special: Some(DisplaySpecial::NegativeInfinity),
            };
        }
        DisplayValue {
            text: format!("{:.decimals$} {unit}", 20.0 * plain.log10()),
            special: None,
        }
    })
}

// ── interaction contract ───────────────────────────────────────────

/// Opaque gesture identifier (adapter-assigned).
pub type GestureId = String;
/// Element-level control identifier.
pub type ControlId = String;
/// Logical command target (consumer-defined path).
pub type LogicalTarget = String;
/// View the input originated from.
pub type OriginView = String;
/// Per-control event sequence number (strictly increasing from 1).
pub type SequenceNumber = u64;
/// Optional adapter-assigned input ordering token.
pub type InputSeq = u64;

/// Where an input physically came from. Open union — consumers may
/// extend with arbitrary names.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InputSource(pub String);

impl InputSource {
    /// Pointer (mouse) input.
    pub const POINTER: &'static str = "pointer";
    /// Touch input.
    pub const TOUCH: &'static str = "touch";
    /// Pen input.
    pub const PEN: &'static str = "pen";
    /// Scroll-wheel input.
    pub const WHEEL: &'static str = "wheel";
    /// Keyboard input.
    pub const KEYBOARD: &'static str = "keyboard";
    /// Programmatic (non-user) input — the `set`/`adjust`/`reset`
    /// default.
    pub const PROGRAMMATIC: &'static str = "programmatic";
}

impl From<&str> for InputSource {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Why a gesture was cancelled. Open union.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CancelReason(pub String);

impl CancelReason {
    /// User pressed Escape.
    pub const ESCAPE: &'static str = "escape";
    /// Focus left the control.
    pub const LOST_FOCUS: &'static str = "lost-focus";
    /// Pointer capture was lost.
    pub const LOST_POINTER_CAPTURE: &'static str = "lost-pointer-capture";
    /// Owning view was unmounted.
    pub const UNMOUNT: &'static str = "unmount";
    /// View moved across a dock/surface boundary.
    pub const DOCK_TRANSFER: &'static str = "dock-transfer";
    /// Explicit cancel (default for `cancel` inputs).
    pub const EXPLICIT: &'static str = "explicit";
    /// Availability removed mid-gesture.
    pub const UNAVAILABLE: &'static str = "unavailable";
}

impl From<&str> for CancelReason {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Input modifiers that change how a value is resolved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputModifiers {
    /// Fine-adjust (e.g. Shift): selects `fine_step` or the fine fallback.
    pub fine: bool,
}

/// Jump target for [`ValueInput::Extreme`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Extreme {
    /// Descriptor minimum.
    Min,
    /// Descriptor maximum.
    Max,
    /// Descriptor default.
    Default,
}

/// How an input specifies its value.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ValueInput {
    /// Absolute plain-domain value.
    Plain(f64),
    /// Absolute normalized `[0, 1]` value.
    Normalized(f64),
    /// Relative adjustment in effective step quanta.
    Steps(f64),
    /// Jump to min/max/default.
    Extreme(Extreme),
}

/// Decodes an authoritative [`StateProjection`] payload into the
/// [`ValueInput`] a controlled display may apply — the canonical
/// projection→display contract (Issue #414).
///
/// The payload stays opaque business JSON until here; nothing is
/// interpreted as `f64` unconditionally. Accepted shapes:
///
/// - a finite JSON number → [`ValueInput::Plain`] (the product-value
///   domain — the same domain the `value` property carries in the
///   reference implementation);
/// - `{"plain": <finite number>}` → [`ValueInput::Plain`];
/// - `{"normalized": <finite number>}` → [`ValueInput::Normalized`].
///
/// Anything else — strings, nulls, nested objects, non-finite
/// numbers — yields `None`: the binding still records the projection
/// as `observed`, but it carries no displayable control value and the
/// controlled display is left untouched.
#[must_use]
pub fn projection_value_input(value: &serde_json::Value) -> Option<ValueInput> {
    fn finite(v: &serde_json::Value) -> Option<f64> {
        v.as_f64().filter(|n| n.is_finite())
    }
    match value {
        serde_json::Value::Number(_) => finite(value).map(ValueInput::Plain),
        serde_json::Value::Object(map) if map.len() == 1 => {
            if let Some(v) = map.get("plain") {
                finite(v).map(ValueInput::Plain)
            } else {
                map.get("normalized")
                    .and_then(finite)
                    .map(ValueInput::Normalized)
            }
        }
        _ => None,
    }
}

/// Typed inputs accepted by [`InteractionControl::dispatch`].
///
/// `Set`/`Adjust`/`Reset` are value-producing shorthand inputs:
/// without `gesture_id` they are *atomic* (a synthesized
/// `begin`+`commit` pair marked `atomic`); with `gesture_id` they
/// auto-begin when idle and act as `preview`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ControlInput {
    /// Begin a gesture (idle controls only).
    Begin {
        /// Adapter-assigned gesture id.
        gesture_id: GestureId,
        /// Physical input source.
        source: InputSource,
        /// Originating view override.
        origin_view: Option<OriginView>,
        /// Logical target override.
        target: Option<LogicalTarget>,
        /// Optional initial value.
        at: Option<ValueInput>,
        /// Input modifiers.
        modifiers: Option<InputModifiers>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
    /// Update the in-flight gesture's tentative value.
    Preview {
        /// Active gesture id.
        gesture_id: GestureId,
        /// New value.
        value: ValueInput,
        /// Input modifiers.
        modifiers: Option<InputModifiers>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
    /// Finish the gesture, promoting preview to committed.
    Commit {
        /// Active gesture id.
        gesture_id: GestureId,
        /// Optional final value.
        value: Option<ValueInput>,
        /// Input modifiers.
        modifiers: Option<InputModifiers>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
    /// Abort the gesture; committed value is untouched.
    Cancel {
        /// Active gesture id.
        gesture_id: GestureId,
        /// Cancel reason (default `explicit`).
        reason: Option<CancelReason>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
    /// Atomic or auto-beginning absolute set.
    Set {
        /// New value.
        value: ValueInput,
        /// Optional gesture id (auto-begin when idle).
        gesture_id: Option<GestureId>,
        /// Input source (default `programmatic`).
        source: Option<InputSource>,
        /// Input modifiers.
        modifiers: Option<InputModifiers>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
    /// Atomic or auto-beginning relative step adjustment.
    Adjust {
        /// Step count (sign = direction).
        steps: f64,
        /// Optional gesture id.
        gesture_id: Option<GestureId>,
        /// Input source.
        source: Option<InputSource>,
        /// Input modifiers.
        modifiers: Option<InputModifiers>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
    /// Atomic or auto-beginning reset to the declared default.
    Reset {
        /// Optional gesture id.
        gesture_id: Option<GestureId>,
        /// Input source.
        source: Option<InputSource>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
    /// Focus left the control — cancels the in-flight gesture.
    /// Lifecycle input: exempt from seq drops.
    FocusLost {
        /// When set, only terminates the gesture with this id.
        gesture_id: Option<GestureId>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
    /// Detach from the surface (unmount/dock move) — terminates the
    /// in-flight gesture per policy. Lifecycle input.
    Detach {
        /// When set, only terminates the gesture with this id.
        gesture_id: Option<GestureId>,
        /// Terminal policy override.
        policy: Option<DetachPolicy>,
        /// Reason (default `unmount`).
        reason: Option<CancelReason>,
        /// Ordering token.
        seq: Option<InputSeq>,
    },
}

/// Event metadata carried on every emitted event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionEventMeta {
    /// Emitting control.
    pub control_id: ControlId,
    /// Logical target (gesture-level or default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<LogicalTarget>,
    /// Origin view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_view: Option<OriginView>,
    /// Gesture id when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gesture_id: Option<GestureId>,
    /// Per-control monotonically increasing sequence (starts at 1).
    pub sequence: SequenceNumber,
    /// Input source that produced this event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<InputSource>,
    /// `true` on the synthesized pair of an atomic input.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub atomic: bool,
}

/// A control value in both domains; always finite and in range.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ControlValue {
    /// Plain-domain value.
    pub plain: f64,
    /// Normalized `[0, 1]` value.
    pub normalized: f64,
}

/// Explicit capability notification — distinct from benign `noop`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UnavailableReason {
    /// `descriptor.disabled` or runtime flag.
    Disabled,
    /// `descriptor.read_only` or runtime flag.
    ReadOnly,
    /// The control kind is unsupported by the renderer.
    Unsupported,
}

/// Contract-violating input — the input was rejected, state unchanged.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RejectReason {
    /// `NaN`/infinite value input.
    NonFiniteInput,
    /// Finite value outside the declared range under `Reject`.
    OutOfRange,
    /// Mapping produced an invalid output.
    InvalidMapping,
    /// Input is structurally invalid.
    InvalidInput,
    /// A different gesture is already active on this control.
    GestureAlreadyActive,
    /// Input references a terminated gesture.
    StaleGesture,
    /// The gesture was already terminated.
    AlreadyTerminated,
    /// `seq` is below the last seen input sequence.
    OutOfOrder,
    /// No control registered for the given id.
    UnknownControl,
}

/// Benign non-effect — distinct from `unavailable` and `rejected`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoopReason {
    /// No gesture is in flight.
    NoActiveGesture,
    /// `seq` duplicates the last seen input sequence.
    DuplicateInput,
    /// No `default_value` is declared for a reset.
    NoDefault,
}

/// Events emitted by [`InteractionControl`]. `Begin`, `Commit`,
/// `Cancel`, `Unavailable` and `Rejected` are non-droppable;
/// consecutive `Preview` events for the same control+gesture are
/// coalescible by a transport that guarantees order within the gesture.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InteractionEvent {
    /// Gesture began.
    Begin {
        /// Event metadata.
        meta: InteractionEventMeta,
        /// Initial value.
        value: ControlValue,
    },
    /// Tentative value update (coalescible).
    Preview {
        /// Event metadata.
        meta: InteractionEventMeta,
        /// Preview value.
        value: ControlValue,
    },
    /// Gesture committed; `value` is the new committed value.
    Commit {
        /// Event metadata.
        meta: InteractionEventMeta,
        /// Committed value.
        value: ControlValue,
    },
    /// Gesture cancelled; committed value unchanged.
    Cancel {
        /// Event metadata.
        meta: InteractionEventMeta,
        /// Cancel reason.
        reason: CancelReason,
    },
    /// Capability notification (`disabled`/`read_only`/`unsupported`).
    Unavailable {
        /// Event metadata.
        meta: InteractionEventMeta,
        /// Why the control is unavailable.
        reason: UnavailableReason,
    },
    /// Contract-violating input rejected.
    Rejected {
        /// Event metadata.
        meta: InteractionEventMeta,
        /// Rejection reason.
        reason: RejectReason,
        /// Human-readable detail.
        detail: Option<String>,
    },
    /// Benign non-effect.
    Noop {
        /// Event metadata.
        meta: InteractionEventMeta,
        /// Why nothing happened.
        reason: NoopReason,
    },
}

impl InteractionEvent {
    /// Event metadata.
    #[must_use]
    pub fn meta(&self) -> &InteractionEventMeta {
        match self {
            Self::Begin { meta, .. }
            | Self::Preview { meta, .. }
            | Self::Commit { meta, .. }
            | Self::Cancel { meta, .. }
            | Self::Unavailable { meta, .. }
            | Self::Rejected { meta, .. }
            | Self::Noop { meta, .. } => meta,
        }
    }

    /// `preview` is the only droppable kind.
    #[must_use]
    pub fn is_coalescible(&self) -> bool {
        matches!(self, Self::Preview { .. })
    }
}

/// Phase of the gesture state machine.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GesturePhase {
    /// No gesture in flight.
    Idle,
    /// A gesture is in flight.
    Active,
}

/// What happens to an in-flight gesture on `detach`
/// (unmount/dock move).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetachPolicy {
    /// Cancel the gesture (default).
    Cancel,
    /// Commit the preview value.
    Commit,
}

/// Default bound on terminated-gesture ids kept for stale detection.
pub const DEFAULT_TERMINATED_HISTORY: usize = 32;

/// Constructor options for [`InteractionControl`].
#[derive(Clone, Default)]
pub struct InteractionControlOptions {
    /// Control identifier.
    pub control_id: ControlId,
    /// Value descriptor (required — set before construction).
    pub descriptor: Option<ValueDescriptor>,
    /// Local function ports.
    pub ports: Option<ValuePorts>,
    /// Default logical target reported in event meta.
    pub target: Option<LogicalTarget>,
    /// Default origin view reported in event meta.
    pub origin_view: Option<OriginView>,
    /// `true` marks the control unsupported: every mutating input
    /// notifies.
    pub unsupported: bool,
    /// Terminal policy for `detach` when the input does not override it.
    pub detach_policy: Option<DetachPolicy>,
    /// Bound on the terminated-gesture-id memory used for stale
    /// detection.
    pub terminated_gesture_history: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Availability {
    Available,
    Disabled,
    ReadOnly,
    Unsupported,
}

impl Availability {
    fn reason(self) -> Option<UnavailableReason> {
        match self {
            Self::Available => None,
            Self::Disabled => Some(UnavailableReason::Disabled),
            Self::ReadOnly => Some(UnavailableReason::ReadOnly),
            Self::Unsupported => Some(UnavailableReason::Unsupported),
        }
    }
}

#[derive(Debug, Clone)]
struct GestureState {
    gesture_id: GestureId,
    source: InputSource,
    target: Option<LogicalTarget>,
    origin_view: Option<OriginView>,
    preview: ControlValue,
}

#[derive(Default)]
struct MetaPartial {
    gesture_id: Option<GestureId>,
    source: Option<InputSource>,
    atomic: bool,
    target: Option<LogicalTarget>,
    origin_view: Option<OriginView>,
}

fn failure_to_reject(f: &ValueFailure) -> RejectReason {
    match f.kind {
        ValueFailureKind::NonFinite => RejectReason::NonFiniteInput,
        ValueFailureKind::OutOfRange => RejectReason::OutOfRange,
        ValueFailureKind::InvalidMapping => RejectReason::InvalidMapping,
    }
}

/// Runtime availability overrides for [`InteractionControl::set_availability`].
#[derive(Debug, Clone, Copy, Default)]
pub struct AvailabilityFlags {
    /// Disabled override.
    pub disabled: bool,
    /// Read-only override.
    pub read_only: bool,
    /// Unsupported override.
    pub unsupported: bool,
}

/// Headless interaction controller for one control. Owns the gesture
/// state machine, resolves inputs through its [`ValueMapper`], and
/// emits the contract events. Survives detach/reattach: `detach`
/// terminates the in-flight gesture per policy while the object stays
/// usable for an adapter that re-subscribes.
#[derive(Clone)]
pub struct InteractionControl {
    /// Control identifier.
    pub control_id: ControlId,
    /// Typed value mapping.
    pub mapper: ValueMapper,
    availability: Availability,
    detach_policy: DetachPolicy,
    history_bound: usize,
    default_target: Option<LogicalTarget>,
    default_origin_view: Option<OriginView>,
    phase: GesturePhase,
    gesture: Option<GestureState>,
    committed_value: ControlValue,
    /// An authoritative write staged while a gesture is active —
    /// lands as `committed_value` when the gesture terminates. The
    /// local preview always wins mid-gesture; the last observed truth
    /// takes over afterwards (Issue #414).
    pending_external: Option<ControlValue>,
    sequence: SequenceNumber,
    last_input_seq: Option<InputSeq>,
    atomic_counter: u64,
    terminated: VecDeque<GestureId>,
}

impl InteractionControl {
    /// Constructs the control; the initial committed value is the
    /// descriptor default (or `min` when none is declared).
    ///
    /// # Panics
    /// When the descriptor is invalid — descriptors are validated data,
    /// so construct via [`InteractionControlOptions`] from a spec that
    /// already passed `validate_descriptor`.
    #[must_use]
    pub fn new(options: InteractionControlOptions) -> Self {
        let descriptor = options
            .descriptor
            .expect("InteractionControlOptions.descriptor is required");
        let mapper = ValueMapper::new(descriptor.clone(), options.ports.as_ref())
            .expect("descriptor must validate");
        let availability = if options.unsupported {
            Availability::Unsupported
        } else if descriptor.disabled {
            Availability::Disabled
        } else if descriptor.read_only {
            Availability::ReadOnly
        } else {
            Availability::Available
        };
        let initial = mapper
            .resolve_plain(mapper.default_plain())
            .unwrap_or((0.0, descriptor.min));
        Self {
            control_id: options.control_id,
            availability,
            detach_policy: options.detach_policy.unwrap_or(DetachPolicy::Cancel),
            history_bound: options
                .terminated_gesture_history
                .unwrap_or(DEFAULT_TERMINATED_HISTORY),
            default_target: options.target,
            default_origin_view: options.origin_view,
            phase: GesturePhase::Idle,
            gesture: None,
            committed_value: ControlValue {
                plain: initial.1,
                normalized: initial.0,
            },
            pending_external: None,
            sequence: 0,
            last_input_seq: None,
            atomic_counter: 0,
            terminated: VecDeque::new(),
            mapper,
        }
    }

    /// Current phase of the gesture state machine.
    #[must_use]
    pub fn phase(&self) -> GesturePhase {
        self.phase
    }

    /// Gesture id of the in-flight gesture, if any.
    #[must_use]
    pub fn active_gesture_id(&self) -> Option<&GestureId> {
        self.gesture.as_ref().map(|g| &g.gesture_id)
    }

    /// Last committed value — never modified by cancel.
    #[must_use]
    pub fn committed_value(&self) -> ControlValue {
        self.committed_value
    }

    /// Previewed value while active; the committed value while idle.
    #[must_use]
    pub fn value(&self) -> ControlValue {
        self.gesture
            .as_ref()
            .map_or(self.committed_value, |g| g.preview)
    }

    /// Applies an authoritative external value to the controlled
    /// display — the write half of the projection contract
    /// (Issue #414).
    ///
    /// `input` must be an absolute [`ValueInput::Plain`] or
    /// [`ValueInput::Normalized`]; the value resolves through the
    /// same [`ValueMapper`] as user input (range policy, step snap,
    /// domain mapping), so a projection can never store a value the
    /// descriptor would reject for a user. Relative/jump inputs
    /// (`Steps`/`Extreme`) are meaningless as absolute truth and are
    /// refused with `None`.
    ///
    /// While a gesture is active the resolved value is staged and
    /// lands as the committed value when the gesture terminates
    /// (commit, cancel, detach, focus loss): the local preview
    /// always wins mid-gesture. While idle it becomes the committed
    /// value immediately.
    ///
    /// The write emits no [`InteractionEvent`]s and consumes no input
    /// sequence — it reflects domain truth pushed by the consumer,
    /// never a command back to it, so the controlled-write loop
    /// stays closed. Returns the resolved [`ControlValue`] when the
    /// input was absolute and resolvable, `None` otherwise — an
    /// unresolvable value leaves the display untouched (the binding
    /// still records the projection as `observed`).
    pub fn apply_external(&mut self, input: ValueInput) -> Option<ControlValue> {
        if !matches!(input, ValueInput::Plain(_) | ValueInput::Normalized(_)) {
            return None;
        }
        let resolved = self
            .resolve_value_input(input, self.committed_value, false)
            .ok()?;
        if self.gesture.is_some() {
            self.pending_external = Some(resolved);
        } else {
            self.committed_value = resolved;
        }
        Some(resolved)
    }

    /// Change capability flags at runtime. Making a control
    /// unavailable mid-gesture cancels the in-flight gesture
    /// (`reason: "explicit"`) before the flag takes effect;
    /// subsequent mutating inputs then emit `unavailable`.
    pub fn set_availability(&mut self, flags: AvailabilityFlags) -> Vec<InteractionEvent> {
        let next = if flags.unsupported {
            Availability::Unsupported
        } else if flags.disabled {
            Availability::Disabled
        } else if flags.read_only {
            Availability::ReadOnly
        } else {
            Availability::Available
        };
        if self.availability == next {
            return Vec::new();
        }
        let mut events = Vec::new();
        if next.reason().is_some()
            && let Some(g) = self.gesture.take()
        {
            let meta = self.meta(Self::gesture_meta(&g));
            self.end_gesture(g);
            events.push(InteractionEvent::Cancel {
                meta,
                reason: CancelReason::from(CancelReason::EXPLICIT),
            });
        }
        self.availability = next;
        events
    }

    fn next_sequence(&mut self) -> SequenceNumber {
        self.sequence += 1;
        self.sequence
    }

    fn meta(&mut self, p: MetaPartial) -> InteractionEventMeta {
        InteractionEventMeta {
            control_id: self.control_id.clone(),
            target: p.target.or_else(|| self.default_target.clone()),
            origin_view: p.origin_view.or_else(|| self.default_origin_view.clone()),
            gesture_id: p.gesture_id,
            sequence: self.next_sequence(),
            source: p.source,
            atomic: p.atomic,
        }
    }

    fn gesture_meta(g: &GestureState) -> MetaPartial {
        MetaPartial {
            gesture_id: Some(g.gesture_id.clone()),
            source: Some(g.source.clone()),
            atomic: false,
            target: g.target.clone(),
            origin_view: g.origin_view.clone(),
        }
    }

    fn unavailable(
        &mut self,
        reason: UnavailableReason,
        gesture_id: Option<GestureId>,
        source: Option<InputSource>,
    ) -> InteractionEvent {
        let meta = self.meta(MetaPartial {
            gesture_id,
            source,
            ..MetaPartial::default()
        });
        InteractionEvent::Unavailable { meta, reason }
    }

    fn rejected(
        &mut self,
        reason: RejectReason,
        gesture_id: Option<GestureId>,
        detail: Option<String>,
    ) -> InteractionEvent {
        let meta = self.meta(MetaPartial {
            gesture_id,
            ..MetaPartial::default()
        });
        InteractionEvent::Rejected {
            meta,
            reason,
            detail,
        }
    }

    fn noop(&mut self, reason: NoopReason, gesture_id: Option<GestureId>) -> InteractionEvent {
        let meta = self.meta(MetaPartial {
            gesture_id,
            ..MetaPartial::default()
        });
        InteractionEvent::Noop { meta, reason }
    }

    fn end_gesture(&mut self, g: GestureState) {
        self.gesture = None;
        self.phase = GesturePhase::Idle;
        self.terminated.push_back(g.gesture_id);
        while self.terminated.len() > self.history_bound {
            self.terminated.pop_front();
        }
        // A staged authoritative write lands as the gesture ends —
        // commit/cancel paths assign `committed_value` before calling
        // here, so the last observed truth takes over uniformly.
        if let Some(v) = self.pending_external.take() {
            self.committed_value = v;
        }
    }

    fn resolve_value_input(
        &self,
        input: ValueInput,
        from: ControlValue,
        fine: bool,
    ) -> Result<ControlValue, ValueFailure> {
        let resolved = match input {
            ValueInput::Plain(p) => self.mapper.resolve_plain(p),
            ValueInput::Normalized(n) => self.mapper.resolve_normalized(n),
            ValueInput::Steps(s) => self.mapper.adjust_normalized(from.normalized, s, fine),
            ValueInput::Extreme(e) => {
                let target = match e {
                    Extreme::Min => self.mapper.descriptor.min,
                    Extreme::Max => self.mapper.descriptor.max,
                    Extreme::Default => self.mapper.default_plain(),
                };
                self.mapper.resolve_plain(target)
            }
        };
        resolved.map(|(normalized, plain)| ControlValue { plain, normalized })
    }

    fn check_seq(&mut self, seq: Option<InputSeq>) -> Option<InteractionEvent> {
        let seq = seq?;
        if let Some(last) = self.last_input_seq {
            if seq == last {
                return Some(self.noop(NoopReason::DuplicateInput, None));
            }
            if seq < last {
                return Some(self.rejected(
                    RejectReason::OutOfOrder,
                    None,
                    Some(format!("input seq {seq} < last seen {last}")),
                ));
            }
        }
        self.last_input_seq = Some(seq);
        None
    }

    /// Lifecycle inputs are exempt from seq drops: a late-delivered
    /// lifecycle event must still be evaluated, or a gesture could
    /// outlive the view that owns it.
    fn check_seq_lifecycle(&mut self, seq: Option<InputSeq>) {
        if let Some(seq) = seq
            && self.last_input_seq.is_none_or(|l| seq > l)
        {
            self.last_input_seq = Some(seq);
        }
    }

    fn check_availability(
        &mut self,
        gesture_id: Option<GestureId>,
        source: Option<InputSource>,
    ) -> Option<InteractionEvent> {
        self.availability
            .reason()
            .map(|r| self.unavailable(r, gesture_id, source))
    }

    fn gesture_for(&self, gesture_id: &GestureId) -> Option<&GestureState> {
        self.gesture
            .as_ref()
            .filter(|g| &g.gesture_id == gesture_id)
    }

    fn is_terminated(&self, gesture_id: &GestureId) -> bool {
        self.terminated.contains(gesture_id)
    }

    fn auto_gesture_id(&mut self) -> GestureId {
        self.atomic_counter += 1;
        format!("atomic-{}", self.atomic_counter)
    }

    /// Dispatch one typed input and return the emitted events in
    /// emission order. Every input produces at least one event —
    /// including explicit `unavailable`/`noop`/`rejected` notifications
    /// — so a transport can observe that an input arrived and what
    /// happened to it.
    pub fn dispatch(&mut self, input: ControlInput) -> Vec<InteractionEvent> {
        match input {
            ControlInput::FocusLost { gesture_id, seq } => {
                self.check_seq_lifecycle(seq);
                self.dispatch_focus_lost(gesture_id)
            }
            ControlInput::Detach {
                gesture_id,
                policy,
                reason,
                seq,
            } => {
                self.check_seq_lifecycle(seq);
                self.dispatch_detach(gesture_id, policy, reason)
            }
            input => {
                if let Some(e) = self.check_seq(input_seq(&input)) {
                    return vec![e];
                }
                match input {
                    ControlInput::Begin {
                        gesture_id,
                        source,
                        origin_view,
                        target,
                        at,
                        modifiers,
                        ..
                    } => {
                        self.dispatch_begin(gesture_id, source, origin_view, target, at, modifiers)
                    }
                    ControlInput::Preview {
                        gesture_id,
                        value,
                        modifiers,
                        ..
                    } => self.dispatch_preview(gesture_id, value, modifiers),
                    ControlInput::Commit {
                        gesture_id,
                        value,
                        modifiers,
                        ..
                    } => self.dispatch_commit(gesture_id, value, modifiers),
                    ControlInput::Cancel {
                        gesture_id, reason, ..
                    } => self.dispatch_cancel(gesture_id, reason),
                    ControlInput::Set {
                        value,
                        gesture_id,
                        source,
                        modifiers,
                        ..
                    } => self.dispatch_set_like(gesture_id, source, modifiers, value),
                    ControlInput::Adjust {
                        steps,
                        gesture_id,
                        source,
                        modifiers,
                        ..
                    } => self.dispatch_set_like(
                        gesture_id,
                        source,
                        modifiers,
                        ValueInput::Steps(steps),
                    ),
                    ControlInput::Reset {
                        gesture_id, source, ..
                    } => self.dispatch_set_like(
                        gesture_id,
                        source,
                        None,
                        ValueInput::Extreme(Extreme::Default),
                    ),
                    ControlInput::FocusLost { .. } | ControlInput::Detach { .. } => {
                        unreachable!()
                    }
                }
            }
        }
    }

    fn dispatch_begin(
        &mut self,
        gesture_id: GestureId,
        source: InputSource,
        origin_view: Option<OriginView>,
        target: Option<LogicalTarget>,
        at: Option<ValueInput>,
        modifiers: Option<InputModifiers>,
    ) -> Vec<InteractionEvent> {
        if let Some(e) = self.check_availability(Some(gesture_id.clone()), Some(source.clone())) {
            return vec![e];
        }
        if self.gesture_for(&gesture_id).is_some() {
            return vec![self.rejected(
                RejectReason::GestureAlreadyActive,
                Some(gesture_id.clone()),
                Some(format!("gesture {gesture_id} is already active")),
            )];
        }
        if self.is_terminated(&gesture_id) {
            return vec![self.rejected(
                RejectReason::AlreadyTerminated,
                Some(gesture_id.clone()),
                Some(format!("gesture {gesture_id} was already terminated")),
            )];
        }
        if let Some(active) = &self.gesture {
            // A different gesture is already active: reject the
            // newcomer explicitly; the in-flight gesture continues.
            let detail = format!(
                "control {} already has active gesture {}",
                self.control_id, active.gesture_id
            );
            return vec![self.rejected(
                RejectReason::GestureAlreadyActive,
                Some(gesture_id),
                Some(detail),
            )];
        }
        let mut initial = self.committed_value;
        if let Some(at) = at {
            let fine = modifiers.is_some_and(|m| m.fine);
            match self.resolve_value_input(at, self.committed_value, fine) {
                Ok(v) => initial = v,
                Err(e) => {
                    return vec![self.rejected(
                        failure_to_reject(&e),
                        Some(gesture_id),
                        Some(e.detail),
                    )];
                }
            }
        }
        let g = GestureState {
            gesture_id,
            source,
            target,
            origin_view,
            preview: initial,
        };
        let meta = self.meta(Self::gesture_meta(&g));
        self.gesture = Some(g.clone());
        self.phase = GesturePhase::Active;
        vec![InteractionEvent::Begin {
            meta,
            value: g.preview,
        }]
    }

    fn dispatch_preview(
        &mut self,
        gesture_id: GestureId,
        value: ValueInput,
        modifiers: Option<InputModifiers>,
    ) -> Vec<InteractionEvent> {
        if let Some(e) = self.check_availability(Some(gesture_id.clone()), None) {
            return vec![e];
        }
        let slot_state = if self.gesture_for(&gesture_id).is_some() {
            SlotState::Active
        } else if self.is_terminated(&gesture_id) {
            SlotState::Stale
        } else {
            SlotState::None
        };
        match slot_state {
            SlotState::None => {
                return vec![self.noop(NoopReason::NoActiveGesture, Some(gesture_id))];
            }
            SlotState::Stale => {
                return vec![self.rejected(
                    RejectReason::StaleGesture,
                    Some(gesture_id.clone()),
                    Some(format!("preview for terminated gesture {gesture_id}")),
                )];
            }
            SlotState::Active => {}
        }
        let fine = modifiers.is_some_and(|m| m.fine);
        let from = self
            .gesture
            .as_ref()
            .map_or(self.committed_value, |g| g.preview);
        let r = match self.resolve_value_input(value, from, fine) {
            Ok(v) => v,
            Err(e) => {
                // Invalid preview input is rejected without disturbing
                // the gesture.
                return vec![self.rejected(
                    failure_to_reject(&e),
                    Some(gesture_id),
                    Some(e.detail),
                )];
            }
        };
        if let Some(g) = &mut self.gesture {
            g.preview = r;
            let partial = Self::gesture_meta(g);
            let meta = self.meta(partial);
            return vec![InteractionEvent::Preview { meta, value: r }];
        }
        vec![self.noop(NoopReason::NoActiveGesture, Some(gesture_id))]
    }

    fn dispatch_commit(
        &mut self,
        gesture_id: GestureId,
        value: Option<ValueInput>,
        modifiers: Option<InputModifiers>,
    ) -> Vec<InteractionEvent> {
        if let Some(e) = self.check_availability(Some(gesture_id.clone()), None) {
            return vec![e];
        }
        let slot_state = if self.gesture_for(&gesture_id).is_some() {
            SlotState::Active
        } else if self.is_terminated(&gesture_id) {
            SlotState::Stale
        } else {
            SlotState::None
        };
        match slot_state {
            SlotState::None => {
                return vec![self.noop(NoopReason::NoActiveGesture, Some(gesture_id))];
            }
            SlotState::Stale => {
                return vec![self.rejected(
                    RejectReason::AlreadyTerminated,
                    Some(gesture_id.clone()),
                    Some(format!("commit for terminated gesture {gesture_id}")),
                )];
            }
            SlotState::Active => {}
        }
        if let Some(value) = value {
            let fine = modifiers.is_some_and(|m| m.fine);
            let from = self
                .gesture
                .as_ref()
                .map_or(self.committed_value, |g| g.preview);
            match self.resolve_value_input(value, from, fine) {
                Ok(v) => {
                    if let Some(g) = &mut self.gesture {
                        g.preview = v;
                    }
                }
                Err(e) => {
                    // An invalid commit value is rejected and the
                    // gesture stays active — the adapter may retry.
                    return vec![self.rejected(
                        failure_to_reject(&e),
                        Some(gesture_id),
                        Some(e.detail),
                    )];
                }
            }
        }
        let Some(g) = self.gesture.take() else {
            return vec![self.noop(NoopReason::NoActiveGesture, Some(gesture_id))];
        };
        self.committed_value = g.preview;
        let meta = self.meta(Self::gesture_meta(&g));
        let value = g.preview;
        self.end_gesture(g);
        vec![InteractionEvent::Commit { meta, value }]
    }

    fn dispatch_cancel(
        &mut self,
        gesture_id: GestureId,
        reason: Option<CancelReason>,
    ) -> Vec<InteractionEvent> {
        let slot_state = if self.gesture_for(&gesture_id).is_some() {
            SlotState::Active
        } else if self.is_terminated(&gesture_id) {
            SlotState::Stale
        } else {
            SlotState::None
        };
        match slot_state {
            SlotState::None => {
                return vec![self.noop(NoopReason::NoActiveGesture, Some(gesture_id))];
            }
            SlotState::Stale => {
                return vec![self.rejected(
                    RejectReason::AlreadyTerminated,
                    Some(gesture_id.clone()),
                    Some(format!("cancel for terminated gesture {gesture_id}")),
                )];
            }
            SlotState::Active => {}
        }
        let Some(g) = self.gesture.take() else {
            return vec![self.noop(NoopReason::NoActiveGesture, Some(gesture_id))];
        };
        let meta = self.meta(Self::gesture_meta(&g));
        self.end_gesture(g);
        // Committed state is intentionally untouched by cancel.
        vec![InteractionEvent::Cancel {
            meta,
            reason: reason.unwrap_or_else(|| CancelReason::from(CancelReason::EXPLICIT)),
        }]
    }

    /// `set`/`adjust`/`reset` handling. The value is resolved first,
    /// so an invalid input is a plain `rejected` and never half-starts
    /// a gesture.
    fn dispatch_set_like(
        &mut self,
        gesture_id: Option<GestureId>,
        source: Option<InputSource>,
        modifiers: Option<InputModifiers>,
        value: ValueInput,
    ) -> Vec<InteractionEvent> {
        let source = source.unwrap_or_else(|| InputSource::from(InputSource::PROGRAMMATIC));
        let fine = modifiers.is_some_and(|m| m.fine);
        if let Some(e) = self.check_availability(gesture_id.clone(), Some(source.clone())) {
            return vec![e];
        }
        if matches!(value, ValueInput::Extreme(Extreme::Default)) && !self.mapper.has_default() {
            return vec![self.noop(NoopReason::NoDefault, gesture_id)];
        }

        let Some(gesture_id) = gesture_id else {
            // Atomic input: a complete begin+commit pair in one dispatch.
            return self.dispatch_atomic_set(source, fine, value);
        };

        if self.is_terminated(&gesture_id) {
            return vec![self.rejected(
                RejectReason::AlreadyTerminated,
                Some(gesture_id.clone()),
                Some(format!("input for terminated gesture {gesture_id}")),
            )];
        }
        if self.gesture_for(&gesture_id).is_none() && self.gesture.is_some() {
            let active_id = self
                .gesture
                .as_ref()
                .map(|g| g.gesture_id.clone())
                .unwrap_or_default();
            let detail = format!(
                "control {} already has active gesture {}",
                self.control_id, active_id
            );
            return vec![self.rejected(
                RejectReason::GestureAlreadyActive,
                Some(gesture_id),
                Some(detail),
            )];
        }
        let base = self
            .gesture
            .as_ref()
            .map_or(self.committed_value, |g| g.preview);
        let r = match self.resolve_value_input(value, base, fine) {
            Ok(v) => v,
            Err(e) => {
                return vec![self.rejected(
                    failure_to_reject(&e),
                    Some(gesture_id),
                    Some(e.detail),
                )];
            }
        };
        let mut events = Vec::new();
        if self.gesture_for(&gesture_id).is_none() {
            let g = GestureState {
                gesture_id: gesture_id.clone(),
                source,
                target: None,
                origin_view: None,
                preview: self.committed_value,
            };
            let meta = self.meta(Self::gesture_meta(&g));
            let value = g.preview;
            self.gesture = Some(g);
            self.phase = GesturePhase::Active;
            events.push(InteractionEvent::Begin { meta, value });
        }
        if let Some(g) = &mut self.gesture {
            g.preview = r;
            let partial = Self::gesture_meta(g);
            let meta = self.meta(partial);
            events.push(InteractionEvent::Preview { meta, value: r });
        }
        events
    }

    /// `set`-like input without a gesture id: a complete
    /// begin+commit pair minted in one dispatch. Rejected while
    /// another gesture is active — it never half-starts a gesture.
    fn dispatch_atomic_set(
        &mut self,
        source: InputSource,
        fine: bool,
        value: ValueInput,
    ) -> Vec<InteractionEvent> {
        if let Some(active) = &self.gesture {
            let detail = format!(
                "control {} already has active gesture {}",
                self.control_id, active.gesture_id
            );
            return vec![self.rejected(RejectReason::GestureAlreadyActive, None, Some(detail))];
        }
        let r = match self.resolve_value_input(value, self.committed_value, fine) {
            Ok(v) => v,
            Err(e) => {
                return vec![self.rejected(failure_to_reject(&e), None, Some(e.detail))];
            }
        };
        let g = GestureState {
            gesture_id: self.auto_gesture_id(),
            source,
            target: self.default_target.clone(),
            origin_view: self.default_origin_view.clone(),
            preview: r,
        };
        let mut p = Self::gesture_meta(&g);
        p.atomic = true;
        let begin_meta = self.meta(p);
        let begin = InteractionEvent::Begin {
            meta: begin_meta,
            value: g.preview,
        };
        self.committed_value = g.preview;
        let mut p2 = Self::gesture_meta(&g);
        p2.atomic = true;
        let commit_meta = self.meta(p2);
        let value = g.preview;
        self.end_gesture(g);
        let commit = InteractionEvent::Commit {
            meta: commit_meta,
            value,
        };
        vec![begin, commit]
    }

    fn dispatch_focus_lost(&mut self, gesture_id: Option<GestureId>) -> Vec<InteractionEvent> {
        let Some(g) = self.gesture.take() else {
            return vec![self.noop(NoopReason::NoActiveGesture, gesture_id)];
        };
        if let Some(id) = &gesture_id
            && g.gesture_id != *id
        {
            self.gesture = Some(g);
            return vec![self.noop(NoopReason::NoActiveGesture, gesture_id)];
        }
        let meta = self.meta(Self::gesture_meta(&g));
        self.end_gesture(g);
        vec![InteractionEvent::Cancel {
            meta,
            reason: CancelReason::from(CancelReason::LOST_FOCUS),
        }]
    }

    fn dispatch_detach(
        &mut self,
        gesture_id: Option<GestureId>,
        policy: Option<DetachPolicy>,
        reason: Option<CancelReason>,
    ) -> Vec<InteractionEvent> {
        let Some(g) = self.gesture.take() else {
            return vec![self.noop(NoopReason::NoActiveGesture, gesture_id)];
        };
        if let Some(id) = &gesture_id
            && g.gesture_id != *id
        {
            self.gesture = Some(g);
            return vec![self.noop(NoopReason::NoActiveGesture, gesture_id)];
        }
        let policy = policy.unwrap_or(self.detach_policy);
        if policy == DetachPolicy::Commit {
            self.committed_value = g.preview;
            let meta = self.meta(Self::gesture_meta(&g));
            let value = g.preview;
            self.end_gesture(g);
            return vec![InteractionEvent::Commit { meta, value }];
        }
        let meta = self.meta(Self::gesture_meta(&g));
        self.end_gesture(g);
        vec![InteractionEvent::Cancel {
            meta,
            reason: reason.unwrap_or_else(|| CancelReason::from(CancelReason::UNMOUNT)),
        }]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Active,
    Stale,
    None,
}

fn input_seq(input: &ControlInput) -> Option<InputSeq> {
    match input {
        ControlInput::Begin { seq, .. }
        | ControlInput::Preview { seq, .. }
        | ControlInput::Commit { seq, .. }
        | ControlInput::Cancel { seq, .. }
        | ControlInput::Set { seq, .. }
        | ControlInput::Adjust { seq, .. }
        | ControlInput::Reset { seq, .. }
        | ControlInput::FocusLost { seq, .. }
        | ControlInput::Detach { seq, .. } => *seq,
    }
}

/// Registration record for [`InteractionDispatcher`].
pub type ControlRegistration = InteractionControlOptions;

/// Owns the [`InteractionControl`]s of a view/session and routes
/// inputs by `control_id`. Re-registering an existing `control_id`
/// detaches the old control first (in-flight gestures end per its
/// detach policy); the resulting events are returned so a transport
/// can deliver them.
#[derive(Default)]
pub struct InteractionDispatcher {
    controls: HashMap<ControlId, InteractionControl>,
    synthetic_seq: SequenceNumber,
}

impl InteractionDispatcher {
    /// Creates an empty dispatcher.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a control. An existing `control_id` is detached first.
    pub fn register(&mut self, options: ControlRegistration) -> Vec<InteractionEvent> {
        let events = self.unregister(&options.control_id);
        self.controls
            .insert(options.control_id.clone(), InteractionControl::new(options));
        events
    }

    /// Removes a control, terminating its gesture with `unmount`.
    pub fn unregister(&mut self, control_id: &str) -> Vec<InteractionEvent> {
        let Some(existing) = self.controls.remove(control_id) else {
            return Vec::new();
        };
        let mut existing = existing;
        existing.dispatch(ControlInput::Detach {
            gesture_id: None,
            policy: None,
            reason: Some(CancelReason::from(CancelReason::UNMOUNT)),
            seq: None,
        })
    }

    /// Detaches every control (unmount/dock transfer of the whole
    /// view). Deterministic: controls are visited in `control_id`
    /// order.
    pub fn detach_all(&mut self, reason: &str) -> Vec<InteractionEvent> {
        let mut ids: Vec<ControlId> = self.controls.keys().cloned().collect();
        ids.sort();
        let mut events = Vec::new();
        for id in ids {
            if let Some(c) = self.controls.get_mut(&id) {
                events.extend(c.dispatch(ControlInput::Detach {
                    gesture_id: None,
                    policy: None,
                    reason: Some(CancelReason::from(reason)),
                    seq: None,
                }));
            }
        }
        events
    }

    /// Looks up a control.
    #[must_use]
    pub fn control(&self, control_id: &str) -> Option<&InteractionControl> {
        self.controls.get(control_id)
    }

    /// Mutable lookup (for `set_availability`).
    pub fn control_mut(&mut self, control_id: &str) -> Option<&mut InteractionControl> {
        self.controls.get_mut(control_id)
    }

    /// Registered control ids.
    #[must_use]
    pub fn control_ids(&self) -> Vec<ControlId> {
        self.controls.keys().cloned().collect()
    }

    /// Dispatch an input to a control. Unknown `control_id` is reported
    /// as `rejected` `unknown-control` rather than dropped silently.
    pub fn dispatch(&mut self, control_id: &str, input: ControlInput) -> Vec<InteractionEvent> {
        let Some(c) = self.controls.get_mut(control_id) else {
            self.synthetic_seq += 1;
            return vec![InteractionEvent::Rejected {
                meta: InteractionEventMeta {
                    control_id: control_id.to_string(),
                    target: None,
                    origin_view: None,
                    gesture_id: None,
                    sequence: self.synthetic_seq,
                    source: None,
                    atomic: false,
                },
                reason: RejectReason::UnknownControl,
                detail: Some(format!("no control registered for {control_id}")),
            }];
        };
        c.dispatch(input)
    }
}

// ── command vocabulary ─────────────────────────────────────────────

/// Binding-minted command identifier; unique per binding instance.
pub type CommandId = String;

/// The dispatchable command kinds. `Preview` is the only droppable one.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandKind {
    /// Gesture began.
    Begin,
    /// Tentative value update.
    Preview,
    /// Gesture finished.
    Commit,
    /// Gesture aborted.
    Cancel,
    /// Discrete action.
    Action,
}

/// A command intent handed to the consumer's `dispatch` port. The
/// union is deliberately flat and serializable — no callbacks cross
/// the port. `Begin`/`Commit`/`Cancel`/`Action` are non-droppable;
/// `Preview` is coalescible under bounded latest-wins.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CommandIntent {
    /// Gesture began.
    Begin {
        /// Logical command target.
        target: LogicalTarget,
        /// Element-level control id.
        control_id: ControlId,
        /// Element-level event sequence.
        sequence: SequenceNumber,
        /// Gesture id.
        gesture_id: GestureId,
        /// Origin view.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_view: Option<OriginView>,
        /// Atomic-input flag.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        atomic: bool,
        /// Initial value.
        value: ControlValue,
    },
    /// Tentative value update (coalescible).
    Preview {
        /// Logical command target.
        target: LogicalTarget,
        /// Element-level control id.
        control_id: ControlId,
        /// Element-level event sequence.
        sequence: SequenceNumber,
        /// Gesture id.
        gesture_id: GestureId,
        /// Origin view.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_view: Option<OriginView>,
        /// Atomic-input flag.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        atomic: bool,
        /// Preview value.
        value: ControlValue,
    },
    /// Gesture committed.
    Commit {
        /// Logical command target.
        target: LogicalTarget,
        /// Element-level control id.
        control_id: ControlId,
        /// Element-level event sequence.
        sequence: SequenceNumber,
        /// Gesture id.
        gesture_id: GestureId,
        /// Origin view.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_view: Option<OriginView>,
        /// Atomic-input flag.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        atomic: bool,
        /// Committed value.
        value: ControlValue,
    },
    /// Gesture cancelled.
    Cancel {
        /// Logical command target.
        target: LogicalTarget,
        /// Element-level control id.
        control_id: ControlId,
        /// Element-level event sequence.
        sequence: SequenceNumber,
        /// Gesture id.
        gesture_id: GestureId,
        /// Origin view.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_view: Option<OriginView>,
        /// Atomic-input flag.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        atomic: bool,
        /// Cancel reason.
        reason: CancelReason,
    },
    /// Discrete action (e.g. a transport action). Non-droppable.
    Action {
        /// Logical command target.
        target: LogicalTarget,
        /// Element-level control id.
        control_id: ControlId,
        /// Element-level event sequence.
        sequence: SequenceNumber,
        /// Origin view.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_view: Option<OriginView>,
        /// Discrete action name.
        action: String,
        /// Optional provenance tag.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        /// Optional action parameters (e.g. the selected row index).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        params: Option<serde_json::Value>,
    },
}

impl CommandIntent {
    /// The command kind.
    #[must_use]
    pub fn kind(&self) -> CommandKind {
        match self {
            Self::Begin { .. } => CommandKind::Begin,
            Self::Preview { .. } => CommandKind::Preview,
            Self::Commit { .. } => CommandKind::Commit,
            Self::Cancel { .. } => CommandKind::Cancel,
            Self::Action { .. } => CommandKind::Action,
        }
    }
}

/// `preview` is the only kind a transport may coalesce; everything
/// else is a boundary or a discrete, possibly non-idempotent operation.
#[must_use]
pub fn is_droppable(cmd: &CommandIntent) -> bool {
    matches!(cmd, CommandIntent::Preview { .. })
}

/// Translate one headless [`InteractionEvent`] into a command intent.
/// Returns `None` for local-only events (`unavailable`, `rejected`,
/// `noop`) — those are element-contract notifications, never gateway
/// commands. `fallback_target` supplies the target when the event meta
/// carries none.
#[must_use]
pub fn intent_from_gesture(
    e: &InteractionEvent,
    fallback_target: Option<&LogicalTarget>,
) -> Option<CommandIntent> {
    let meta = e.meta();
    let target = meta.target.clone().or_else(|| fallback_target.cloned())?;
    let control_id = meta.control_id.clone();
    let sequence = meta.sequence;
    let origin_view = meta.origin_view.clone();
    let atomic = meta.atomic;
    match e {
        InteractionEvent::Begin { value, .. } => Some(CommandIntent::Begin {
            target,
            control_id,
            sequence,
            gesture_id: meta.gesture_id.clone()?,
            origin_view,
            atomic,
            value: *value,
        }),
        InteractionEvent::Preview { value, .. } => Some(CommandIntent::Preview {
            target,
            control_id,
            sequence,
            gesture_id: meta.gesture_id.clone()?,
            origin_view,
            atomic,
            value: *value,
        }),
        InteractionEvent::Commit { value, .. } => Some(CommandIntent::Commit {
            target,
            control_id,
            sequence,
            gesture_id: meta.gesture_id.clone()?,
            origin_view,
            atomic,
            value: *value,
        }),
        InteractionEvent::Cancel { reason, .. } => Some(CommandIntent::Cancel {
            target,
            control_id,
            sequence,
            gesture_id: meta.gesture_id.clone()?,
            origin_view,
            atomic,
            reason: reason.clone(),
        }),
        _ => None,
    }
}

/// What the consumer gateway reports about a dispatched command.
/// `Accepted`/`Queued` mean the gateway took responsibility; `Applied`
/// means the domain state was changed — neither implies `Observed`,
/// which arrives only through a [`StateProjection`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReceiptStatus {
    /// Gateway accepted the command.
    Accepted,
    /// Gateway queued the command.
    Queued,
    /// Domain state was changed.
    Applied,
    /// Terminal failure — command rejected.
    Rejected,
    /// Terminal failure — capability unsupported.
    Unsupported,
}

/// Ordering rank for receipt dedup: in-flight < `accepted`/`queued` <
/// `applied` < terminal.
fn receipt_rank(status: ReceiptStatus) -> u8 {
    match status {
        ReceiptStatus::Accepted | ReceiptStatus::Queued => 1,
        ReceiptStatus::Applied => 2,
        ReceiptStatus::Rejected | ReceiptStatus::Unsupported => 3,
    }
}

/// Ordering token for receipts and projections — number or string.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Revision {
    /// Numeric revision.
    Num(u64),
    /// String revision.
    Str(String),
}

/// A receipt from the consumer gateway about a dispatched command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandReceipt {
    /// The `command_id` the binding minted at dispatch.
    pub command_id: CommandId,
    /// Reported status.
    pub status: ReceiptStatus,
    /// Consumer domain revision after the transition.
    pub revision: Option<Revision>,
    /// Human-readable detail.
    pub detail: Option<String>,
}

/// Authoritative state pushed by the consumer. `revision` is the
/// ordering token the stale-response check runs on: a projection that
/// does not advance the last observed revision is dropped, so a delayed
/// answer can never overwrite a newer truth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateProjection<P> {
    /// Ordering token.
    pub revision: Revision,
    /// Projected value.
    pub value: P,
    /// Optional provenance tag (consumer-defined).
    pub provenance: Option<String>,
}

/// The per-binding display lifecycle. Distinct on purpose:
/// `preview` is a local tentative value, not a claim; `pending`/
/// `accepted`/`applied` describe gateway progress; `observed` is the
/// only phase that proves domain state; `rejected`/`unsupported`/
/// `timeout` are explicit terminal failures; `stale` marks a detached
/// binding whose late replies must not drive display.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandPhase {
    /// No gesture in flight and nothing awaiting confirmation.
    Idle,
    /// A gesture is active; the shown value is a local tentative value.
    Preview,
    /// A receipt-expecting command was dispatched; no ack yet.
    Pending,
    /// The gateway took the command (or queued it); still not applied.
    Accepted,
    /// The consumer reported the change applied.
    Applied,
    /// An authoritative projection landed — proves domain state.
    Observed,
    /// Explicit terminal failure; last observed value is the truth.
    Rejected,
    /// Explicit terminal failure — capability unsupported.
    Unsupported,
    /// No receipt inside the ack bound — explicit failure.
    Timeout,
    /// The binding is detached; late replies must not drive display.
    Stale,
}

/// The state a presentation layer reflects onto a control.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingViewState<P> {
    /// Display lifecycle phase.
    pub phase: CommandPhase,
    /// Latest accepted authoritative projection (revision-guarded).
    pub last_observed: Option<StateProjection<P>>,
    /// Monotonic counter of accepted projections.
    pub observed_seq: u64,
    /// Most recent receipt that passed dedup/ordering checks.
    pub last_receipt: Option<CommandReceipt>,
    /// The newest receipt-expecting command, when one exists.
    pub pending_command_id: Option<CommandId>,
    /// `true` when a pending command crossed the ack bound — kept set
    /// even if a late receipt later moves the phase on.
    pub timed_out: bool,
    /// Binding generation — bumped on every detach so late replies are
    /// attributable to a dead view epoch.
    pub generation: u64,
    /// Whether the binding is currently attached.
    pub attached: bool,
}

/// Context handed to the consumer on every dispatch.
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchContext {
    /// Binding-minted command id.
    pub command_id: CommandId,
    /// Logical view id (rendered form of [`crate::dock::ViewInstanceId`]
    /// or a consumer id).
    pub view_id: String,
    /// Binding generation at dispatch time.
    pub generation: u64,
    /// `true` when the binding expects a receipt (`commit` / `action`).
    pub expect_receipt: bool,
}

/// One dispatched command awaiting delivery to the consumer port.
#[derive(Debug, Clone, PartialEq)]
pub struct Dispatched {
    /// The intent.
    pub intent: CommandIntent,
    /// Dispatch context.
    pub context: DispatchContext,
}

#[derive(Debug, Clone)]
struct PendingRecord {
    command_id: CommandId,
    /// Binding generation at dispatch — receipts for records minted
    /// under a dead epoch classify stale and never move the display.
    generation: u64,
    /// `None` = in-flight; otherwise the last accepted receipt status.
    status: Option<ReceiptStatus>,
    expect_receipt: bool,
    deadline: Option<std::time::Instant>,
    /// The ack bound was crossed while the command was still awaiting
    /// confirmation — kept set even if a late receipt moves it on.
    expired: bool,
}

impl PendingRecord {
    fn rank(&self) -> u8 {
        self.status.map_or(0, receipt_rank)
    }
}

/// Bounded record set for receipt correlation: settled records evict
/// first, in-flight receipt-expecting records stay.
const MAX_PENDING_RECORDS: usize = 64;

/// Per-control command-binding state machine (Rust port of the
/// `ui/binding` `Binding` semantics). Mints `view-id:N` command ids,
/// tracks every non-preview command for generation-scoped receipt
/// classification, dedups receipts by rank, guards projections by
/// revision, and marks detach boundaries with a generation bump +
/// `stale` phase. `head` names the newest receipt-expecting command —
/// the display phase derives from it, the active gesture, and the
/// last accepted projection rather than being written per arrival.
///
/// The binding owns no clock: ack deadlines are caller-supplied
/// `Instant`s evaluated by [`CommandBinding::on_tick`], keeping the
/// contract deterministic for headless verification.
#[derive(Debug)]
pub struct CommandBinding<P = serde_json::Value> {
    view_id: String,
    generation: u64,
    attached: bool,
    counter: u64,
    records: VecDeque<PendingRecord>,
    outbox: VecDeque<Dispatched>,
    head: Option<CommandId>,
    active_gesture: Option<GestureId>,
    last_receipt: Option<CommandReceipt>,
    last_observed: Option<StateProjection<P>>,
    observed_seq: u64,
}

impl<P: Clone> CommandBinding<P> {
    /// Creates a binding for `view_id`.
    #[must_use]
    pub fn new(view_id: impl Into<String>) -> Self {
        Self {
            view_id: view_id.into(),
            generation: 0,
            attached: true,
            counter: 0,
            records: VecDeque::new(),
            outbox: VecDeque::new(),
            head: None,
            active_gesture: None,
            last_receipt: None,
            last_observed: None,
            observed_seq: 0,
        }
    }

    fn mint(&mut self) -> CommandId {
        self.counter += 1;
        // Namespaced by the binding id: receipts route back through
        // `Intent.id` alone, so command ids must be unique across the
        // bindings of one view.
        format!("{}:{}", self.view_id, self.counter)
    }

    /// Queues an intent for the consumer port and returns its minted
    /// `(command_id, expect_receipt)`. `commit`/`action` expect a
    /// receipt; lifecycle commands are delivered but do not require one.
    pub fn dispatch(
        &mut self,
        intent: CommandIntent,
        ack_deadline: Option<std::time::Instant>,
    ) -> (CommandId, bool) {
        let expect_receipt = matches!(
            intent,
            CommandIntent::Commit { .. } | CommandIntent::Action { .. }
        );
        // Gesture tracking drives the derived `preview` phase: a
        // begin marks the gesture active; commit/cancel end it.
        match &intent {
            CommandIntent::Begin { gesture_id, .. } => {
                self.active_gesture = Some(gesture_id.clone());
            }
            CommandIntent::Commit { .. } | CommandIntent::Cancel { .. } => {
                self.active_gesture = None;
            }
            _ => {}
        }
        let command_id = self.mint();
        // Every non-preview command is tracked so receipts can be
        // classified by generation; previews mint ids but stay
        // untracked, so a stray preview receipt reads as stale.
        if !matches!(intent, CommandIntent::Preview { .. }) {
            self.records.push_back(PendingRecord {
                command_id: command_id.clone(),
                generation: self.generation,
                status: None,
                expect_receipt,
                deadline: if expect_receipt { ack_deadline } else { None },
                expired: false,
            });
            // In-flight receipt-expecting records stay correlatable;
            // everything else ages out first under the bound. When
            // every record is still in flight the bound yields rather
            // than drop a command the consumer may still answer.
            while self.records.len() > MAX_PENDING_RECORDS {
                let Some(pos) = self
                    .records
                    .iter()
                    .position(|r| r.status.is_some() || !r.expect_receipt)
                else {
                    break;
                };
                self.records.remove(pos);
            }
            if expect_receipt {
                self.head = Some(command_id.clone());
            }
        }
        self.outbox.push_back(Dispatched {
            context: DispatchContext {
                command_id: command_id.clone(),
                view_id: self.view_id.clone(),
                generation: self.generation,
                expect_receipt,
            },
            intent,
        });
        (command_id, expect_receipt)
    }

    /// Drains queued dispatches for the consumer port.
    pub fn drain_outbox(&mut self) -> Vec<Dispatched> {
        self.outbox.drain(..).collect()
    }

    /// Applies a gateway receipt. Late receipts for settled commands —
    /// and any receipt while detached — are absorbed without effect.
    /// Receipts that out-rank the current record state advance it;
    /// equal/lower-rank receipts dedup.
    pub fn on_receipt(&mut self, receipt: &CommandReceipt) {
        if !self.attached {
            return;
        }
        let Some(rec) = self
            .records
            .iter_mut()
            .find(|r| r.command_id == receipt.command_id)
        else {
            // Unknown or never-tracked id — absorbed as stale; only a
            // tracked, generation-live record may move the display.
            return;
        };
        if rec.generation != self.generation {
            return; // dead-epoch reply — stale
        }
        if receipt_rank(receipt.status) <= rec.rank() {
            return; // dedup: equal/lower rank absorbed
        }
        rec.status = Some(receipt.status);
        self.last_receipt = Some(receipt.clone());
    }

    /// Applies an authoritative projection. A projection that does not
    /// advance `last_observed.revision` is dropped — a delayed answer
    /// can never overwrite a newer truth. A landed projection retires
    /// a *terminal* head; a head still awaiting confirmation keeps
    /// answering "did my command land" through [`Self::phase`].
    pub fn on_projection(&mut self, projection: &StateProjection<P>) -> bool {
        if !self.attached {
            return false;
        }
        let advances = match (&self.last_observed, &projection.revision) {
            (Some(prev), next) => match (&prev.revision, next) {
                (Revision::Num(a), Revision::Num(b)) => b > a,
                (Revision::Str(a), Revision::Str(b)) => b != a,
                // Cross-type revisions always advance.
                _ => true,
            },
            (None, _) => true,
        };
        if !advances {
            return false;
        }
        self.last_observed = Some(projection.clone());
        self.observed_seq += 1;
        if self.head_record().is_some_and(|r| r.rank() >= 2) {
            self.head = None;
        }
        true
    }

    /// Marks receipt-expecting records past `now` as expired — the
    /// derived phase reads `Timeout` while the head stays unresolved,
    /// and `expired` stays set even if a late receipt moves it on.
    pub fn on_tick(&mut self, now: std::time::Instant) {
        if !self.attached {
            return;
        }
        for rec in &mut self.records {
            if rec.expect_receipt
                && !rec.expired
                && rec.rank() < 2
                && rec.deadline.is_some_and(|d| now >= d)
            {
                rec.expired = true;
            }
        }
    }

    /// Detaches the binding (unmount/dock move): bumps the generation
    /// so every tracked record — kept for classification — goes stale,
    /// and the derived phase reads `stale`. Late receipts/projections
    /// are absorbed until [`Self::attach`].
    pub fn detach(&mut self) {
        if !self.attached {
            return;
        }
        self.attached = false;
        self.active_gesture = None;
        self.generation += 1;
    }

    /// Re-attaches a detached binding. The dead epoch's head is
    /// forgotten and the generation stays bumped, so replies minted
    /// under it remain attributable.
    pub fn attach(&mut self) {
        if self.attached {
            return;
        }
        self.attached = true;
        self.head = None;
        self.active_gesture = None;
    }

    /// The head record — the newest receipt-expecting command, kept
    /// until an advancing projection retires it or [`Self::attach`]
    /// forgets the dead epoch.
    fn head_record(&self) -> Option<&PendingRecord> {
        self.head
            .as_ref()
            .and_then(|id| self.records.iter().find(|r| &r.command_id == id))
    }

    /// The display lifecycle phase, derived from attachment, the
    /// active gesture, the head command, and the last accepted
    /// projection — matching the canonical `Binding.phase()` order.
    #[must_use]
    pub fn phase(&self) -> CommandPhase {
        if !self.attached {
            return CommandPhase::Stale;
        }
        if self.active_gesture.is_some() {
            return CommandPhase::Preview;
        }
        if let Some(rec) = self.head_record() {
            // The ack bound was crossed without an applied/terminal
            // answer — a distinct failure display, kept even while
            // the status map below would otherwise read "accepted".
            if rec.expired && rec.rank() <= 1 {
                return CommandPhase::Timeout;
            }
            return match rec.rank() {
                0 => CommandPhase::Pending,
                1 => CommandPhase::Accepted,
                2 => CommandPhase::Applied,
                _ => match rec.status {
                    Some(ReceiptStatus::Rejected) => CommandPhase::Rejected,
                    _ => CommandPhase::Unsupported,
                },
            };
        }
        if self.last_observed.is_some() {
            CommandPhase::Observed
        } else {
            CommandPhase::Idle
        }
    }

    /// Current binding view state.
    #[must_use]
    pub fn state(&self) -> BindingViewState<P> {
        BindingViewState {
            phase: self.phase(),
            last_observed: self.last_observed.clone(),
            observed_seq: self.observed_seq,
            last_receipt: self.last_receipt.clone(),
            pending_command_id: self.head.clone(),
            timed_out: self.head_record().is_some_and(|r| r.expired),
            generation: self.generation,
            attached: self.attached,
        }
    }

    /// The current generation.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether the binding is attached.
    #[must_use]
    pub fn attached(&self) -> bool {
        self.attached
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc() -> ValueDescriptor {
        ValueDescriptor {
            default_value: Some(50.0),
            ..ValueDescriptor::ranged(0.0, 100.0)
        }
    }

    fn control() -> InteractionControl {
        InteractionControl::new(InteractionControlOptions {
            control_id: "vol".to_string(),
            descriptor: Some(desc()),
            ..Default::default()
        })
    }

    #[test]
    fn mapper_linear_roundtrip() {
        let m = ValueMapper::new(desc(), None).expect("valid");
        let (n, p) = m.to_normalized(75.0).expect("ok");
        assert!((n - 0.75).abs() < 1e-9 && (p - 75.0).abs() < 1e-9);
        let (n2, p2) = m.to_plain(0.25).expect("ok");
        assert!((p2 - 25.0).abs() < 1e-9 && (n2 - 0.25).abs() < 1e-9);
    }

    #[test]
    fn mapper_clamps_and_rejects() {
        let m = ValueMapper::new(desc(), None).expect("valid");
        let (n, p) = m.to_normalized(150.0).expect("clamped");
        assert!((n - 1.0).abs() < 1e-9 && (p - 100.0).abs() < 1e-9);
        assert!(m.to_normalized(f64::NAN).is_err());
        let mut d = desc();
        d.out_of_range = Some(OutOfRangePolicy::Reject);
        let m = ValueMapper::new(d, None).expect("valid");
        assert_eq!(
            m.to_normalized(150.0).unwrap_err().kind,
            ValueFailureKind::OutOfRange
        );
    }

    #[test]
    fn mapper_step_and_adjust() {
        let mut d = desc();
        d.step = Some(10.0);
        let m = ValueMapper::new(d, None).expect("valid");
        let (_, p) = m.resolve_plain(53.0).expect("ok");
        assert!((p - 50.0).abs() < 1e-9);
        let (n, p) = m.adjust_normalized(0.5, 2.0, false).expect("ok");
        assert!((p - 70.0).abs() < 1e-9 && (n - 0.7).abs() < 1e-9);
    }

    #[test]
    fn mapper_parse_format() {
        let mut d = desc();
        d.unit = Some("dB".to_string());
        let m = ValueMapper::new(d, None).expect("valid");
        assert_eq!(m.format(12.5).text, "12.5 dB");
        assert!((m.parse("45 dB").expect("ok") - 45.0).abs() < 1e-9);
        assert!(m.parse("").is_err());
        assert!(m.parse("abc").is_err());
    }

    #[test]
    fn gesture_begin_preview_commit() {
        let mut c = control();
        let ev = c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        assert!(matches!(ev[0], InteractionEvent::Begin { .. }));
        let ev = c.dispatch(ControlInput::Preview {
            gesture_id: "g1".to_string(),
            value: ValueInput::Normalized(0.8),
            modifiers: None,
            seq: None,
        });
        assert!(matches!(ev[0], InteractionEvent::Preview { .. }));
        let ev = c.dispatch(ControlInput::Commit {
            gesture_id: "g1".to_string(),
            value: None,
            modifiers: None,
            seq: None,
        });
        match &ev[0] {
            InteractionEvent::Commit { value, .. } => {
                assert!((value.plain - 80.0).abs() < 1e-9);
            }
            _ => panic!("expected commit"),
        }
        assert_eq!(c.phase(), GesturePhase::Idle);
        assert!((c.committed_value().plain - 80.0).abs() < 1e-9);
    }

    #[test]
    fn gesture_terminal_aftermath() {
        let mut c = control();
        c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        c.dispatch(ControlInput::Commit {
            gesture_id: "g1".to_string(),
            value: None,
            modifiers: None,
            seq: None,
        });
        // Late preview on a terminated gesture → stale-gesture reject.
        let ev = c.dispatch(ControlInput::Preview {
            gesture_id: "g1".to_string(),
            value: ValueInput::Normalized(0.1),
            modifiers: None,
            seq: None,
        });
        assert!(matches!(
            ev[0],
            InteractionEvent::Rejected {
                reason: RejectReason::StaleGesture,
                ..
            }
        ));
        // Re-begin with the same id → already-terminated.
        let ev = c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        assert!(matches!(
            ev[0],
            InteractionEvent::Rejected {
                reason: RejectReason::AlreadyTerminated,
                ..
            }
        ));
    }

    #[test]
    fn gesture_overlap_rejected() {
        let mut c = control();
        c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        let ev = c.dispatch(ControlInput::Begin {
            gesture_id: "g2".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        assert!(matches!(
            ev[0],
            InteractionEvent::Rejected {
                reason: RejectReason::GestureAlreadyActive,
                ..
            }
        ));
        // In-flight gesture untouched.
        assert_eq!(c.active_gesture_id().map(String::as_str), Some("g1"));
    }

    #[test]
    fn detach_and_focus_lost_cancel() {
        let mut c = control();
        c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        let ev = c.dispatch(ControlInput::Detach {
            gesture_id: None,
            policy: None,
            reason: Some(CancelReason::from(CancelReason::DOCK_TRANSFER)),
            seq: None,
        });
        assert!(matches!(
            ev[0],
            InteractionEvent::Cancel { ref reason, .. } if reason.0 == CancelReason::DOCK_TRANSFER
        ));
        // Late input on the terminated gesture is rejected.
        let ev = c.dispatch(ControlInput::Preview {
            gesture_id: "g1".to_string(),
            value: ValueInput::Normalized(0.5),
            modifiers: None,
            seq: None,
        });
        assert!(matches!(ev[0], InteractionEvent::Rejected { .. }));
    }

    #[test]
    fn set_atomic_and_auto_begin() {
        let mut c = control();
        let ev = c.dispatch(ControlInput::Set {
            value: ValueInput::Plain(30.0),
            gesture_id: None,
            source: None,
            modifiers: None,
            seq: None,
        });
        assert!(matches!(ev[0], InteractionEvent::Begin { ref meta, .. } if meta.atomic));
        assert!(matches!(ev[1], InteractionEvent::Commit { ref meta, .. } if meta.atomic));
        assert!((c.committed_value().plain - 30.0).abs() < 1e-9);
        // Wheel-style burst: auto-begin then preview with same id.
        let ev = c.dispatch(ControlInput::Adjust {
            steps: 1.0,
            gesture_id: Some("wheel".to_string()),
            source: Some(InputSource::from(InputSource::WHEEL)),
            modifiers: None,
            seq: None,
        });
        assert!(matches!(ev[0], InteractionEvent::Begin { .. }));
        assert!(matches!(ev[1], InteractionEvent::Preview { .. }));
        assert_eq!(c.phase(), GesturePhase::Active);
    }

    #[test]
    fn unavailable_and_seq() {
        let mut d = desc();
        d.disabled = true;
        let mut c = InteractionControl::new(InteractionControlOptions {
            control_id: "dis".to_string(),
            descriptor: Some(d),
            ..Default::default()
        });
        let ev = c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: Some(1),
        });
        assert!(matches!(
            ev[0],
            InteractionEvent::Unavailable {
                reason: UnavailableReason::Disabled,
                ..
            }
        ));
        let mut c2 = control();
        let ev = c2.dispatch(ControlInput::Set {
            value: ValueInput::Plain(10.0),
            gesture_id: None,
            source: None,
            modifiers: None,
            seq: Some(5),
        });
        assert_ne!(ev, [] as [InteractionEvent; 0]);
        let ev = c2.dispatch(ControlInput::Set {
            value: ValueInput::Plain(20.0),
            gesture_id: None,
            source: None,
            modifiers: None,
            seq: Some(5),
        });
        assert!(matches!(
            ev[0],
            InteractionEvent::Noop {
                reason: NoopReason::DuplicateInput,
                ..
            }
        ));
        let ev = c2.dispatch(ControlInput::Set {
            value: ValueInput::Plain(20.0),
            gesture_id: None,
            source: None,
            modifiers: None,
            seq: Some(3),
        });
        assert!(matches!(
            ev[0],
            InteractionEvent::Rejected {
                reason: RejectReason::OutOfOrder,
                ..
            }
        ));
    }

    #[test]
    fn intent_translation() {
        let mut c = InteractionControl::new(InteractionControlOptions {
            control_id: "vol".to_string(),
            descriptor: Some(desc()),
            target: Some("mixer.ch1".to_string()),
            ..Default::default()
        });
        let ev = c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        let intent = intent_from_gesture(&ev[0], None).expect("intent");
        assert!(matches!(intent, CommandIntent::Begin { .. }));
        // noop/rejected/unavailable never translate.
        let noop = c.dispatch(ControlInput::Preview {
            gesture_id: "nope".to_string(),
            value: ValueInput::Normalized(0.5),
            modifiers: None,
            seq: None,
        });
        assert!(intent_from_gesture(&noop[0], None).is_none());
    }

    #[test]
    fn binding_receipt_rank_and_projection() {
        let mut b = CommandBinding::new("v1");
        let (id, expect) = b.dispatch(
            CommandIntent::Commit {
                target: "mixer.ch1".to_string(),
                control_id: "vol".to_string(),
                sequence: 1,
                gesture_id: "g1".to_string(),
                origin_view: None,
                atomic: false,
                value: ControlValue {
                    plain: 80.0,
                    normalized: 0.8,
                },
            },
            None,
        );
        assert!(expect);
        assert_eq!(b.phase(), CommandPhase::Pending);
        b.on_receipt(&CommandReceipt {
            command_id: id.clone(),
            status: ReceiptStatus::Accepted,
            revision: None,
            detail: None,
        });
        assert_eq!(b.phase(), CommandPhase::Accepted);
        // Lower-rank duplicate is absorbed.
        b.on_receipt(&CommandReceipt {
            command_id: id.clone(),
            status: ReceiptStatus::Accepted,
            revision: None,
            detail: None,
        });
        assert_eq!(b.phase(), CommandPhase::Accepted);
        b.on_receipt(&CommandReceipt {
            command_id: id,
            status: ReceiptStatus::Applied,
            revision: None,
            detail: None,
        });
        assert_eq!(b.phase(), CommandPhase::Applied);
        // Stale projection dropped; newer one lands as observed.
        let dropped = b.on_projection(&StateProjection {
            revision: Revision::Num(1),
            value: 80.0,
            provenance: None,
        });
        assert!(dropped);
        let dropped2 = b.on_projection(&StateProjection {
            revision: Revision::Num(1),
            value: 70.0,
            provenance: None,
        });
        assert!(!dropped2);
        assert_eq!(b.phase(), CommandPhase::Observed);
    }

    #[test]
    fn binding_detach_stale() {
        let mut b = CommandBinding::<f64>::new("v1");
        b.dispatch(
            CommandIntent::Commit {
                target: "t".to_string(),
                control_id: "c".to_string(),
                sequence: 1,
                gesture_id: "g".to_string(),
                origin_view: None,
                atomic: false,
                value: ControlValue {
                    plain: 1.0,
                    normalized: 1.0,
                },
            },
            None,
        );
        b.detach();
        assert_eq!(b.phase(), CommandPhase::Stale);
        assert_eq!(b.generation(), 1);
        b.on_receipt(&CommandReceipt {
            command_id: "v1:1".to_string(),
            status: ReceiptStatus::Applied,
            revision: None,
            detail: None,
        });
        assert_eq!(b.phase(), CommandPhase::Stale); // absorbed
        b.attach();
        assert!(b.attached());
        assert_eq!(b.phase(), CommandPhase::Idle);
    }

    fn commit_intent(seq: u64, gid: &str) -> CommandIntent {
        CommandIntent::Commit {
            target: "t".to_string(),
            control_id: "c".to_string(),
            sequence: seq,
            gesture_id: gid.to_string(),
            origin_view: None,
            atomic: false,
            value: ControlValue {
                plain: 1.0,
                normalized: 1.0,
            },
        }
    }

    #[test]
    fn binding_observed_payload_and_head_semantics() {
        let mut b = CommandBinding::<f64>::new("v1");

        // A projection before any command lands the payload and
        // surfaces it through `state().last_observed`.
        assert!(b.on_projection(&StateProjection {
            revision: Revision::Num(1),
            value: 0.5,
            provenance: Some("init".to_string()),
        }));
        assert_eq!(b.phase(), CommandPhase::Observed);
        assert_eq!(
            b.state().last_observed,
            Some(StateProjection {
                revision: Revision::Num(1),
                value: 0.5,
                provenance: Some("init".to_string()),
            })
        );

        // A receipt for a never-tracked id is absorbed — no
        // last_receipt write, no phase movement.
        b.on_receipt(&CommandReceipt {
            command_id: "v1:999".to_string(),
            status: ReceiptStatus::Rejected,
            revision: None,
            detail: None,
        });
        assert_eq!(b.state().last_receipt, None);
        assert_eq!(b.phase(), CommandPhase::Observed);

        // An in-flight commit keeps the phase pending-ish even when a
        // newer projection lands — the projection cannot answer "did
        // my command land" for it.
        let (id, expect) = b.dispatch(commit_intent(2, "g1"), None);
        assert!(expect);
        assert_eq!(b.phase(), CommandPhase::Pending);
        assert!(b.on_projection(&StateProjection {
            revision: Revision::Num(2),
            value: 0.9,
            provenance: None,
        }));
        assert_eq!(b.phase(), CommandPhase::Pending);
        assert_eq!(b.state().pending_command_id, Some(id.clone()));

        // `Applied` moves the head to applied; the projection still
        // holds, and the head retires only when a terminal head meets
        // an advancing projection — here already terminal, so the
        // next projection retires it and the phase reads observed.
        b.on_receipt(&CommandReceipt {
            command_id: id,
            status: ReceiptStatus::Applied,
            revision: None,
            detail: None,
        });
        assert_eq!(b.phase(), CommandPhase::Applied);
        assert!(b.on_projection(&StateProjection {
            revision: Revision::Num(3),
            value: 1.0,
            provenance: None,
        }));
        assert_eq!(b.phase(), CommandPhase::Observed);
        assert_eq!(b.state().pending_command_id, None);
        assert_eq!(b.state().last_observed.map(|p| p.value), Some(1.0));
    }

    #[test]
    fn binding_timeout_is_head_scoped() {
        let mut b = CommandBinding::<f64>::new("v1");
        let t0 = std::time::Instant::now();
        let (id, _) = b.dispatch(commit_intent(1, "g1"), Some(t0));
        b.on_tick(t0 + std::time::Duration::from_millis(1));
        assert_eq!(b.phase(), CommandPhase::Timeout);
        assert!(b.state().timed_out);
        // A late applied still moves the phase on; the expired flag
        // stays set on the head record.
        b.on_receipt(&CommandReceipt {
            command_id: id,
            status: ReceiptStatus::Applied,
            revision: None,
            detail: None,
        });
        assert_eq!(b.phase(), CommandPhase::Applied);
        assert!(b.state().timed_out);
    }

    #[test]
    fn external_write_commits_when_idle() {
        let mut c = control();
        // An absolute plain value resolves through the mapper and
        // becomes the committed display value immediately.
        let applied = c.apply_external(ValueInput::Plain(80.0)).expect("applied");
        assert!((applied.plain - 80.0).abs() < 1e-9);
        assert!((c.value().plain - 80.0).abs() < 1e-9);
        assert!((c.committed_value().plain - 80.0).abs() < 1e-9);
        // Normalized input resolves to the same committed pair.
        c.apply_external(ValueInput::Normalized(0.25))
            .expect("applied");
        assert!((c.value().plain - 25.0).abs() < 1e-9);
        assert!((c.value().normalized - 0.25).abs() < 1e-9);
        // Relative/jump inputs are not absolute truth — refused and
        // the display is untouched.
        assert!(c.apply_external(ValueInput::Steps(1.0)).is_none());
        assert!(
            c.apply_external(ValueInput::Extreme(Extreme::Max))
                .is_none()
        );
        assert!((c.value().plain - 25.0).abs() < 1e-9);
    }

    #[test]
    fn external_write_stages_during_gesture() {
        let mut c = control();
        c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        c.dispatch(ControlInput::Preview {
            gesture_id: "g1".to_string(),
            value: ValueInput::Normalized(0.7),
            modifiers: None,
            seq: None,
        });
        // Staged while active — the local preview keeps winning the
        // display and the committed value is not touched.
        c.apply_external(ValueInput::Plain(30.0)).expect("staged");
        assert!((c.value().plain - 70.0).abs() < 1e-9);
        assert!((c.committed_value().plain - 50.0).abs() < 1e-9);
        // A second staged write replaces the first — last wins.
        c.apply_external(ValueInput::Plain(20.0)).expect("staged");
        // Commit promotes the preview, then the staged truth lands
        // as the committed value — the last observed state displays.
        c.dispatch(ControlInput::Commit {
            gesture_id: "g1".to_string(),
            value: None,
            modifiers: None,
            seq: None,
        });
        assert!((c.value().plain - 20.0).abs() < 1e-9);
        assert!((c.committed_value().plain - 20.0).abs() < 1e-9);
    }

    #[test]
    fn external_write_lands_after_cancel() {
        let mut c = control();
        c.dispatch(ControlInput::Begin {
            gesture_id: "g1".to_string(),
            source: InputSource::from(InputSource::POINTER),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        });
        c.dispatch(ControlInput::Preview {
            gesture_id: "g1".to_string(),
            value: ValueInput::Normalized(0.7),
            modifiers: None,
            seq: None,
        });
        c.apply_external(ValueInput::Plain(30.0)).expect("staged");
        c.dispatch(ControlInput::Cancel {
            gesture_id: "g1".to_string(),
            reason: None,
            seq: None,
        });
        // Cancel never rewrote the committed preview; the staged
        // authoritative value lands instead, so the display shows
        // the last observed truth rather than the stale committed.
        assert!((c.value().plain - 30.0).abs() < 1e-9);
        assert!((c.committed_value().plain - 30.0).abs() < 1e-9);
    }

    #[test]
    fn external_write_honors_range_policy() {
        let mut d = desc();
        d.out_of_range = Some(OutOfRangePolicy::Reject);
        let mut c = InteractionControl::new(InteractionControlOptions {
            control_id: "vol".to_string(),
            descriptor: Some(d),
            ..Default::default()
        });
        // Out-of-range under `reject` is refused — display untouched.
        assert!(c.apply_external(ValueInput::Plain(150.0)).is_none());
        assert!((c.value().plain - 50.0).abs() < 1e-9);
        // The default clamp policy lands the same write clamped.
        let mut c2 = control();
        c2.apply_external(ValueInput::Plain(150.0))
            .expect("clamped");
        assert!((c2.value().plain - 100.0).abs() < 1e-9);
    }

    #[test]
    fn projection_value_input_shapes() {
        use serde_json::json;
        assert_eq!(
            projection_value_input(&json!(0.8)),
            Some(ValueInput::Plain(0.8))
        );
        assert_eq!(
            projection_value_input(&json!({"plain": 42.0})),
            Some(ValueInput::Plain(42.0))
        );
        assert_eq!(
            projection_value_input(&json!({"normalized": 0.25})),
            Some(ValueInput::Normalized(0.25))
        );
        for bad in [
            json!("loudness"),
            json!(null),
            json!(true),
            json!({"plain": "x"}),
            json!({"plain": 1.0, "normalized": 0.5}),
            json!({"other": 1.0}),
            json!([1.0]),
        ] {
            assert_eq!(projection_value_input(&bad), None, "{bad}");
        }
    }
}
