//! Health vocabulary and evaluation — domain-declared rules turned
//! into explicit [`HealthState`] transitions.
//!
//! [`HealthState`] has four values: `Healthy`, `Degraded`, `Unhealthy`,
//! and `Unknown`. `Unknown` is neither normal nor abnormal — it means
//! the input needed for assessment was missing, stale, valueless, or
//! otherwise unevaluable, and it never silently clears a known-bad
//! state.
//!
//! [`HealthRule`]s are declared per domain and bound to a
//! `MetricDescriptor::key`. Each evaluation walks a fixed order —
//! valueless → reduced validity → unevaluable signal → threshold
//! comparison — producing a *raw* verdict that then passes through
//! count-based [`Hysteresis`]: `enter_after` consecutive equal-level
//! bad verdicts confirm a transition, and `recover_after` consecutive
//! `Healthy` verdicts recover. A change in the matched
//! [`SourceIdentity`] (id **or** generation) resets the streak state
//! so a re-registered source never inherits the previous generation's
//! hysteresis.
//!
//! Domain meanings stay with the domain owner: nothing here treats a
//! counter increment as a warning, and warmup/normal-range semantics
//! declared in [`SourceSemantics`](crate::source::SourceSemantics)
//! remain the domain's call — the rule only sees the typed measurement.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::Instant;

use crate::metrics::{Measurement, MeasurementValidity, MetricValue};
use crate::source::{Confidentiality, SourceIdentity};

use super::finding::{DiagnosticCode, confidentiality_rank};

/// Assessed health of a domain or rule.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HealthState {
    /// All declared rules evaluated healthy.
    Healthy,
    /// Assessment impossible — missing, stale, valueless, or
    /// unevaluable input. Neither normal nor abnormal.
    Unknown,
    /// Degraded — at least one rule confirmed below healthy, or part
    /// of the declared observation surface is unknown.
    Degraded,
    /// Unhealthy — every known rule is unhealthy.
    Unhealthy,
}

impl HealthState {
    /// Ordering rank for escalation comparisons — `Healthy < Unknown
    /// < Degraded < Unhealthy`. `Unknown` ranks above `Healthy` so a
    /// persistently unevaluable rule surfaces as `Unknown` rather
    /// than staying silently `Healthy`, but below `Degraded`/`Unhealthy`
    /// so it never overwrites a known-bad state.
    pub(crate) const fn rank(self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Unknown => 1,
            Self::Degraded => 2,
            Self::Unhealthy => 3,
        }
    }
}

/// Which derived signal a [`HealthRule`] thresholds.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalKind {
    /// `Gauge::value` as `f64`.
    GaugeValue,
    /// `Counter::delta` — `None` (decrease/anchor failure) is
    /// unevaluable, never zero.
    CounterDelta,
    /// `Counter::rate_per_sec` — already `f64`.
    CounterRate,
}

/// Which side of the threshold is bad.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalDirection {
    /// Bad when `signal >= threshold`.
    Above,
    /// Bad when `signal <= threshold`.
    Below,
}

/// Count-based hysteresis: how many consecutive evaluations must
/// agree before a state change is confirmed.
///
/// Both counts must be at least 1 — validated when the rule is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hysteresis {
    /// Consecutive equal-level bad verdicts required to enter a
    /// non-`Healthy` state.
    pub enter_after: u32,
    /// Consecutive `Healthy` verdicts required to recover to
    /// `Healthy`.
    pub recover_after: u32,
}

/// A domain-declared health rule bound to one metric key.
///
/// All fields are declarative — the domain states what "bad" means;
/// the evaluator only applies thresholds, validity mappings, and
/// hysteresis. At least one of `degrade_at`/`unhealthy_at` must be
/// `Some`, all thresholds must be finite, and when both are present
/// `unhealthy_at` must lie on the worse side of `degrade_at` for the
/// declared [`SignalDirection`].
#[derive(Debug, Clone)]
pub struct HealthRule {
    /// Code attached to transition findings for this rule.
    pub code: DiagnosticCode,
    /// `MetricDescriptor::key` this rule evaluates. When several
    /// tracked series share the key, the worst verdict wins.
    pub metric_key: &'static str,
    /// Which signal to extract from the measurement value.
    pub signal: SignalKind,
    /// Which side of the threshold is bad.
    pub direction: SignalDirection,
    /// Threshold at which the verdict becomes at least `Degraded`.
    pub degrade_at: Option<f64>,
    /// Threshold at which the verdict becomes `Unhealthy` — must be
    /// on the worse side of `degrade_at` when both are set.
    pub unhealthy_at: Option<f64>,
    /// Verdict when the measurement's validity is reduced
    /// (`Stale`/`Partial`/`Discontinuous`).
    pub reduced_validity_is: HealthState,
    /// Verdict when no measurement matches `metric_key`.
    pub missing_is: HealthState,
    /// Verdict when the measurement exists but `value` is `None`
    /// (`Unavailable`/`Unsupported`).
    pub valueless_is: HealthState,
    /// Consecutive-evaluation counts for enter/recover.
    pub hysteresis: Hysteresis,
}

/// Why a [`RuleHealth`] holds its current state — the observable
/// input fact behind the verdict.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleInput {
    /// A numeric signal was extracted and thresholded.
    Signal(f64),
    /// No measurement matched the rule's metric key.
    Missing,
    /// The measurement exists but carries no value.
    Valueless(MeasurementValidity),
    /// The measurement's validity is reduced.
    ReducedValidity(MeasurementValidity),
    /// A measurement exists but the declared signal could not be
    /// extracted (e.g. counter delta suppressed).
    Unevaluable,
}

/// Health of one rule within a [`DomainHealth`].
#[derive(Debug, Clone)]
pub struct RuleHealth {
    /// The rule's diagnostic code.
    pub code: DiagnosticCode,
    /// The bound metric key.
    pub metric_key: &'static str,
    /// Identity of the measurement that produced the verdict, when
    /// one matched.
    pub source: Option<SourceIdentity>,
    /// Current confirmed state.
    pub state: HealthState,
    /// The input fact behind `state`.
    pub input: RuleInput,
    /// Confidentiality of the verdict's input — `Internal` when no
    /// measurement matched (the declared surface default).
    pub confidentiality: Confidentiality,
    /// When `state` last changed, if ever.
    pub last_transition_at: Option<Instant>,
}

/// Rolled-up health of one domain — the deterministic aggregate of
/// its [`RuleHealth`]s.
#[derive(Debug, Clone)]
pub struct DomainHealth {
    /// The domain label rules were registered under.
    pub domain: &'static str,
    /// Rolled-up state.
    pub state: HealthState,
    /// Strictest confidentiality among the domain's rules — the
    /// report disclosure filter withholds the whole entry when the
    /// request is below this.
    pub confidentiality: Confidentiality,
    /// Per-rule health, ordered by [`DiagnosticCode`].
    pub rules: Vec<RuleHealth>,
}

/// A confirmed rule-level state change.
#[derive(Debug, Clone, Copy)]
pub struct RuleTransition {
    /// The rule's code.
    pub code: DiagnosticCode,
    /// The bound metric key.
    pub metric_key: &'static str,
    /// Subject source of the verdict, if any.
    pub source: Option<SourceIdentity>,
    /// Previous confirmed state.
    pub from: HealthState,
    /// New confirmed state.
    pub to: HealthState,
    /// Evaluation instant the transition was confirmed at.
    pub at: Instant,
}

/// The outcome of one [`HealthEvaluator::evaluate`] call.
#[derive(Debug)]
pub struct DomainEvaluation {
    /// Rolled-up domain health after this evaluation.
    pub health: DomainHealth,
    /// Rule transitions confirmed by this evaluation, in rule order.
    pub transitions: Vec<RuleTransition>,
}

/// Per-domain health rule set with hysteresis state.
///
/// Non-RT only — evaluation allocates (`Vec`/`BTreeMap`) and is meant
/// to run on the collector's non-RT path.
#[derive(Debug)]
pub struct HealthEvaluator {
    domain: &'static str,
    rules: BTreeMap<DiagnosticCode, RuleState>,
}

#[derive(Debug)]
struct RuleState {
    rule: HealthRule,
    /// Last confirmed state.
    current: HealthState,
    /// Level of the in-progress bad streak (`Healthy` when none).
    run_level: HealthState,
    /// Consecutive count of `run_level` bad verdicts.
    run_count: u32,
    /// Consecutive `Healthy` verdicts.
    ok_count: u32,
    /// Identity of the source that produced the last verdict — a
    /// change (id or generation) resets the streak state.
    last_identity: Option<SourceIdentity>,
    /// Latest observed input fact.
    input: RuleInput,
    /// Confidentiality of the input that produced `input`.
    input_confidentiality: Confidentiality,
    /// When `current` last changed.
    last_transition_at: Option<Instant>,
}

/// Pre-hysteresis verdict for one rule — the worst candidate across
/// every series bound to the rule's metric key.
struct RawVerdict {
    raw: HealthState,
    input: RuleInput,
    identity: Option<SourceIdentity>,
    confidentiality: Confidentiality,
}

/// One candidate measurement evaluated to a pre-hysteresis verdict.
struct Candidate {
    raw: HealthState,
    input: RuleInput,
    identity: SourceIdentity,
    confidentiality: Confidentiality,
}

impl HealthEvaluator {
    /// Creates an empty evaluator for `domain`.
    #[must_use]
    pub fn new(domain: &'static str) -> Self {
        Self {
            domain,
            rules: BTreeMap::new(),
        }
    }

    /// The domain label.
    #[must_use]
    pub const fn domain(&self) -> &'static str {
        self.domain
    }

    /// Adds a rule. Re-adding an existing [`DiagnosticCode`] is an
    /// error — remove it first to re-declare.
    ///
    /// # Errors
    ///
    /// [`RuleError::InvalidHysteresis`] when either count is 0;
    /// [`RuleError::MissingThreshold`] when both thresholds are `None`;
    /// [`RuleError::NonFiniteThreshold`] on NaN/infinite thresholds;
    /// [`RuleError::ThresholdOrder`] when `unhealthy_at` is not on the
    /// worse side of `degrade_at`; [`RuleError::DuplicateCode`] when
    /// the code is already registered.
    pub fn add_rule(&mut self, rule: HealthRule) -> Result<(), RuleError> {
        validate_rule(&rule)?;
        if self.rules.contains_key(&rule.code) {
            return Err(RuleError::DuplicateCode);
        }
        self.rules.insert(
            rule.code,
            RuleState {
                rule,
                current: HealthState::Healthy,
                run_level: HealthState::Healthy,
                run_count: 0,
                ok_count: 0,
                last_identity: None,
                input: RuleInput::Missing,
                input_confidentiality: Confidentiality::Internal,
                last_transition_at: None,
            },
        );
        Ok(())
    }

    /// Removes the rule with `code`; returns whether one existed.
    pub fn remove_rule(&mut self, code: DiagnosticCode) -> bool {
        self.rules.remove(&code).is_some()
    }

    /// Removes all rules.
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// Number of declared rules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether no rules are declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Evaluates every rule against `measurements` at `now`, updates
    /// hysteresis state, and returns the rolled-up domain health plus
    /// confirmed transitions.
    ///
    /// When several series share a rule's `metric_key`, each is
    /// evaluated and the worst raw verdict wins (ties keep snapshot
    /// order — deterministic).
    pub fn evaluate(&mut self, measurements: &[Measurement], now: Instant) -> DomainEvaluation {
        let mut transitions = Vec::new();
        for state in self.rules.values_mut() {
            let verdict = evaluate_rule(&state.rule, measurements);
            let identity_changed = verdict
                .identity
                .is_some_and(|id| state.last_identity.is_some_and(|last| last != id));
            if identity_changed {
                // A re-registered or different source must not inherit
                // the previous generation's streaks. The confirmed
                // `current` state is *not* hysteresis state — it is
                // only cleared by a confirmed recovery streak, never
                // fabricated Healthy by a bookkeeping reset.
                state.run_level = HealthState::Healthy;
                state.run_count = 0;
                state.ok_count = 0;
            }
            state.last_identity = verdict.identity.or(state.last_identity);
            state.input = verdict.input;
            state.input_confidentiality = verdict.confidentiality;
            apply_hysteresis(state, verdict.raw, now, &mut transitions);
        }
        DomainEvaluation {
            health: self.current_health(),
            transitions,
        }
    }

    /// The rolled-up health of the current rule states — no
    /// evaluation, no mutation.
    #[must_use]
    pub fn current_health(&self) -> DomainHealth {
        let mut rules = Vec::with_capacity(self.rules.len());
        let mut confidentiality = Confidentiality::Public;
        for state in self.rules.values() {
            confidentiality = max_confidentiality(confidentiality, state.input_confidentiality);
            rules.push(RuleHealth {
                code: state.rule.code,
                metric_key: state.rule.metric_key,
                source: state.last_identity,
                state: state.current,
                input: state.input,
                confidentiality: state.input_confidentiality,
                last_transition_at: state.last_transition_at,
            });
        }
        DomainHealth {
            domain: self.domain,
            state: rollup(&rules),
            confidentiality,
            rules,
        }
    }
}

/// Evaluates one rule against all series bound to its metric key and
/// returns the worst raw verdict (first match wins ties).
fn evaluate_rule(rule: &HealthRule, measurements: &[Measurement]) -> RawVerdict {
    let mut best: Option<Candidate> = None;
    for m in measurements
        .iter()
        .filter(|m| m.descriptor.key == rule.metric_key)
    {
        let candidate = evaluate_measurement(rule, m);
        let replace = match &best {
            None => true,
            Some(b) => candidate.raw.rank() > b.raw.rank(),
        };
        if replace {
            best = Some(candidate);
        }
    }
    match best {
        None => RawVerdict {
            raw: rule.missing_is,
            input: RuleInput::Missing,
            identity: None,
            confidentiality: Confidentiality::Internal,
        },
        Some(c) => RawVerdict {
            raw: c.raw,
            input: c.input,
            identity: Some(c.identity),
            confidentiality: c.confidentiality,
        },
    }
}

/// Fixed per-measurement evaluation order: valueless → reduced
/// validity → unevaluable signal → threshold comparison.
fn evaluate_measurement(rule: &HealthRule, m: &Measurement) -> Candidate {
    let confidentiality = m.descriptor.semantics.confidentiality;
    if m.value.is_none() {
        return Candidate {
            raw: rule.valueless_is,
            input: RuleInput::Valueless(m.validity),
            identity: m.identity,
            confidentiality,
        };
    }
    if m.validity != MeasurementValidity::Valid {
        return Candidate {
            raw: rule.reduced_validity_is,
            input: RuleInput::ReducedValidity(m.validity),
            identity: m.identity,
            confidentiality,
        };
    }
    let Some(signal) = extract_signal(rule.signal, m) else {
        return Candidate {
            raw: HealthState::Unknown,
            input: RuleInput::Unevaluable,
            identity: m.identity,
            confidentiality,
        };
    };
    Candidate {
        raw: threshold_verdict(rule, signal),
        input: RuleInput::Signal(signal),
        identity: m.identity,
        confidentiality,
    }
}

/// Applies count-based hysteresis to `state`, appending any confirmed
/// transition to `transitions`.
fn apply_hysteresis(
    state: &mut RuleState,
    raw: HealthState,
    now: Instant,
    transitions: &mut Vec<RuleTransition>,
) {
    let h = state.rule.hysteresis;
    if raw == HealthState::Healthy {
        state.ok_count = state.ok_count.saturating_add(1).min(h.recover_after);
        state.run_count = 0;
        if state.current != HealthState::Healthy && state.ok_count >= h.recover_after {
            let from = state.current;
            state.current = HealthState::Healthy;
            state.last_transition_at = Some(now);
            transitions.push(RuleTransition {
                code: state.rule.code,
                metric_key: state.rule.metric_key,
                source: state.last_identity,
                from,
                to: HealthState::Healthy,
                at: now,
            });
        }
        return;
    }

    state.ok_count = 0;
    if raw == state.run_level {
        state.run_count = state.run_count.saturating_add(1).min(h.enter_after);
    } else {
        state.run_level = raw;
        state.run_count = 1;
    }
    if state.run_count < h.enter_after {
        return;
    }
    // Confirmed target: `Unknown` can raise a known-good state to
    // `Unknown` but never downgrades a known-bad one; `Degraded` /
    // `Unhealthy` move freely among the bad states once their streak
    // confirms (a sustained better-but-still-bad level downgrades).
    let target = if raw == HealthState::Unknown {
        if state.current.rank() > HealthState::Unknown.rank() {
            state.current
        } else {
            HealthState::Unknown
        }
    } else {
        raw
    };
    if target != state.current {
        let from = state.current;
        state.current = target;
        state.last_transition_at = Some(now);
        transitions.push(RuleTransition {
            code: state.rule.code,
            metric_key: state.rule.metric_key,
            source: state.last_identity,
            from,
            to: target,
            at: now,
        });
    }
}

/// Extracts the rule's declared signal as `f64`.
///
/// `u64 → f64` conversion is intentionally lossy above 2^53 — the same
/// display-grade precision bound the collector documents for
/// `rate_per_sec`. Thresholds are domain-declared comparisons, not
/// exact accounting.
#[expect(
    clippy::cast_precision_loss,
    reason = "threshold comparisons are display-grade; u64→f64 loss above 2^53 is documented and acceptable"
)]
fn extract_signal(kind: SignalKind, m: &Measurement) -> Option<f64> {
    match (kind, &m.value) {
        (SignalKind::GaugeValue, Some(MetricValue::Gauge { value, .. })) => Some(*value as f64),
        (SignalKind::CounterDelta, Some(MetricValue::Counter { delta, .. })) => {
            delta.map(|d| d as f64)
        }
        (SignalKind::CounterRate, Some(MetricValue::Counter { rate_per_sec, .. })) => *rate_per_sec,
        _ => None,
    }
}

/// Compares `signal` against the rule's thresholds — `Unhealthy`
/// first (it is always the worse side), then `Degraded`.
fn threshold_verdict(rule: &HealthRule, signal: f64) -> HealthState {
    if let Some(u) = rule.unhealthy_at
        && violates(rule.direction, signal, u)
    {
        return HealthState::Unhealthy;
    }
    if let Some(d) = rule.degrade_at
        && violates(rule.direction, signal, d)
    {
        return HealthState::Degraded;
    }
    HealthState::Healthy
}

fn violates(direction: SignalDirection, signal: f64, threshold: f64) -> bool {
    match direction {
        SignalDirection::Above => signal >= threshold,
        SignalDirection::Below => signal <= threshold,
    }
}

fn validate_rule(rule: &HealthRule) -> Result<(), RuleError> {
    if rule.hysteresis.enter_after == 0 || rule.hysteresis.recover_after == 0 {
        return Err(RuleError::InvalidHysteresis);
    }
    if rule.degrade_at.is_none() && rule.unhealthy_at.is_none() {
        return Err(RuleError::MissingThreshold);
    }
    for t in [rule.degrade_at, rule.unhealthy_at].into_iter().flatten() {
        if !t.is_finite() {
            return Err(RuleError::NonFiniteThreshold);
        }
    }
    if let (Some(d), Some(u)) = (rule.degrade_at, rule.unhealthy_at) {
        let ordered = match rule.direction {
            SignalDirection::Above => u >= d,
            SignalDirection::Below => u <= d,
        };
        if !ordered {
            return Err(RuleError::ThresholdOrder);
        }
    }
    Ok(())
}

/// Rolls rule states up to a domain state:
///
/// - no known (non-`Unknown`) rules → `Unknown`
/// - all known rules `Unhealthy` → `Unhealthy`
/// - any known `Unhealthy` (partial) or any `Degraded` → `Degraded`
/// - all known `Healthy` with `Unknown` mixed in → `Degraded`
///   (part of the declared surface is unobservable — never claimed
///   fully healthy)
/// - all `Healthy` → `Healthy`
fn rollup(rules: &[RuleHealth]) -> HealthState {
    let mut known = 0usize;
    let mut unhealthy = 0usize;
    let mut degraded = 0usize;
    let mut unknown = 0usize;
    for r in rules {
        match r.state {
            HealthState::Unhealthy => {
                known += 1;
                unhealthy += 1;
            }
            HealthState::Degraded => {
                known += 1;
                degraded += 1;
            }
            HealthState::Healthy => known += 1,
            HealthState::Unknown => unknown += 1,
        }
    }
    if known == 0 {
        return HealthState::Unknown;
    }
    if unhealthy == known {
        return HealthState::Unhealthy;
    }
    if unhealthy > 0 || degraded > 0 || unknown > 0 {
        return HealthState::Degraded;
    }
    HealthState::Healthy
}

fn max_confidentiality(a: Confidentiality, b: Confidentiality) -> Confidentiality {
    if confidentiality_rank(b) > confidentiality_rank(a) {
        b
    } else {
        a
    }
}

/// Errors adding a [`HealthRule`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleError {
    /// `enter_after` or `recover_after` was 0.
    InvalidHysteresis,
    /// Both `degrade_at` and `unhealthy_at` were `None`.
    MissingThreshold,
    /// A threshold was NaN or infinite.
    NonFiniteThreshold,
    /// `unhealthy_at` was not on the worse side of `degrade_at`.
    ThresholdOrder,
    /// A rule with this [`DiagnosticCode`] already exists.
    DuplicateCode,
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHysteresis => {
                write!(f, "hysteresis enter_after/recover_after must be >= 1")
            }
            Self::MissingThreshold => {
                write!(f, "a health rule requires degrade_at or unhealthy_at")
            }
            Self::NonFiniteThreshold => write!(f, "health thresholds must be finite"),
            Self::ThresholdOrder => {
                write!(f, "unhealthy_at must be on the worse side of degrade_at")
            }
            Self::DuplicateCode => write!(f, "a rule with this diagnostic code exists"),
        }
    }
}

impl Error for RuleError {}
