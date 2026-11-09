//! Scenario execution assembly for [`SemanticGuiTest`] adapters
//! (Issue #464 runner slice).
//!
//! The base contract ([`super`]) gives adapters the port, target
//! resolution, capability vocabulary, and the [`ScenarioReport`]
//! skeleton — but leaves "declare a run, execute a scenario function,
//! collect a report" to each consumer. This module adds that thin
//! assembly layer as ordinary Rust functions, not an executor:
//!
//! - [`ScenarioSpec`] declares one run: scenario/case/run ids, the
//!   [`ScenarioCapability`] set the scenario requires, environment
//!   facts, and an optional wall-clock budget.
//! - [`ScenarioOutcome`] is what the scenario body returns — the
//!   consumer's verdict plus the evidence fields of
//!   [`ScenarioReport`].
//! - [`run_scenario`] performs capability pre-flight (a scenario
//!   whose requirements the adapter cannot meet never runs its
//!   body), executes the body, maps [`ScenarioError`] onto
//!   [`ScenarioVerdict`], and assembles the report.
//! - [`settle_until`] is the bounded wait primitive for
//!   receipt/revision conditions — at most `max_settles` settle
//!   steps, never a sleep.
//!
//! Nothing here is a second runner or DSL: the scenario itself stays
//! an ordinary Rust function over [`SemanticGuiTest`], and the same
//! [`run_scenario`] call works for the Fake adapter, the GPUI
//! simulated adapter, and any future real-OS adapter because every
//! signature type is driver-neutral.
//!
//! **RT contract:** non-RT only.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use ruviform_ui::behavior::CommandReceipt;
use serde_json::Value;

use super::{
    CaseId, RunId, ScenarioCapability, ScenarioError, ScenarioId, ScenarioReport, ScenarioTarget,
    ScenarioVerdict, SemanticGuiTest,
};

// ---------------------------------------------------------------------
// declaration / outcome / timeout record
// ---------------------------------------------------------------------

/// One scenario run's declaration — identity, capability
/// requirements, environment, and an optional wall-clock budget.
///
/// Build with [`ScenarioSpec::new`] plus the `#[must_use]` builder
/// methods, mirroring [`super::TargetQuery`]'s style.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioSpec {
    /// Execution run id.
    pub run_id: RunId,
    /// Scenario definition id.
    pub scenario: ScenarioId,
    /// Case id inside the scenario, when the run is case-scoped.
    pub case: Option<CaseId>,
    /// Capabilities the scenario requires — pre-flight checked
    /// against [`SemanticGuiTest::capabilities`] before the body
    /// runs. A missing capability yields an `Unsupported` report,
    /// never a partially executed scenario.
    pub required: BTreeSet<ScenarioCapability>,
    /// Environment facts (tested SHAs, OS, theme, ...).
    pub environment: Option<Value>,
    /// Wall-clock budget in milliseconds. When declared,
    /// [`run_scenario`] records a [`ScenarioTimeout`] on the report
    /// and downgrades a `Pass` verdict to `Fail` if the measured
    /// elapsed time exceeds the budget. The semantic port cannot
    /// preempt a synchronous call, so the budget is recorded and
    /// corrected after the fact — never enforced mid-call. Real-OS
    /// drivers enforce their own deadlines inside their adapters
    /// (Issue #313 scope).
    pub timeout_ms: Option<u64>,
}

impl ScenarioSpec {
    /// A spec for `scenario` under `run_id` — no case, no required
    /// capabilities, no environment, no timeout budget.
    #[must_use]
    pub fn new(run_id: RunId, scenario: ScenarioId) -> Self {
        Self {
            run_id,
            scenario,
            case: None,
            required: BTreeSet::new(),
            environment: None,
            timeout_ms: None,
        }
    }

    /// Scope the run to `case`.
    #[must_use]
    pub fn case(mut self, case: CaseId) -> Self {
        self.case = Some(case);
        self
    }

    /// Add `capability` to the required set.
    #[must_use]
    pub fn require(mut self, capability: ScenarioCapability) -> Self {
        self.required.insert(capability);
        self
    }

    /// Replace the required capability set.
    #[must_use]
    pub fn required(mut self, required: BTreeSet<ScenarioCapability>) -> Self {
        self.required = required;
        self
    }

    /// Record environment facts on the report.
    #[must_use]
    pub fn environment(mut self, environment: Value) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Declare a wall-clock budget in milliseconds.
    #[must_use]
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// What a scenario body reports back to [`run_scenario`].
///
/// The fields mirror the evidence half of [`ScenarioReport`]; the
/// runner copies them verbatim and never upgrades the verdict.
#[derive(Debug, Clone)]
pub struct ScenarioOutcome {
    /// The consumer's verdict — recorded as-is (subject only to the
    /// timeout downgrade in [`run_scenario`]). Choosing `Pass` is
    /// the scenario's own claim; the runner does not second-guess
    /// it beyond the budget rule.
    pub verdict: ScenarioVerdict,
    /// What the scenario expected (consumer-defined JSON).
    pub expected: Option<Value>,
    /// What was actually observed (consumer-defined JSON).
    pub observed: Option<Value>,
    /// Resolved targets the scenario touched.
    pub targets: Vec<ScenarioTarget>,
    /// Receipts the consumer posted back and the view accepted.
    pub receipts: Vec<CommandReceipt>,
    /// Evidence references (fixture paths, journal ids, ...).
    pub evidence: Vec<String>,
    /// Human-readable detail, especially for non-pass verdicts.
    pub detail: Option<String>,
}

impl ScenarioOutcome {
    /// An outcome carrying `verdict` and no evidence fields.
    #[must_use]
    pub fn new(verdict: ScenarioVerdict) -> Self {
        Self {
            verdict,
            expected: None,
            observed: None,
            targets: Vec::new(),
            receipts: Vec::new(),
            evidence: Vec::new(),
            detail: None,
        }
    }
}

/// Wall-clock accounting for one run — recorded evidence, not a
/// control input.
///
/// `timed_out` is computed from the exact measured [`Duration`],
/// while `elapsed_ms`/`budget_ms` are the millisecond-spelling
/// recorded on the report — sub-millisecond precision is not
/// preserved in the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioTimeout {
    /// Budget declared via [`ScenarioSpec::timeout_ms`].
    pub budget_ms: u64,
    /// Measured wall-clock elapsed time, truncated to
    /// milliseconds.
    pub elapsed_ms: u64,
    /// Whether the measured elapsed time exceeded `budget_ms`.
    pub timed_out: bool,
}

// ---------------------------------------------------------------------
// runner
// ---------------------------------------------------------------------

/// Runs one scenario against `gui` and assembles its
/// [`ScenarioReport`] — the shared, driver-neutral execution entry
/// point for consumer `#[test]` composition roots.
///
/// Sequence:
///
/// 1. Report skeleton: `run_id`/`scenario`/`case` from `spec`,
///    `driver`/`capabilities` read live from the adapter,
///    `required`/`environment` copied from `spec`.
/// 2. Pre-flight: `spec.required` must be a subset of
///    [`SemanticGuiTest::capabilities`]. On failure the body is
///    **not** executed and the report carries `Unsupported` (or
///    `Fail` for any non-`Unsupported` error) with the reason in
///    `detail`.
/// 3. Body: `Ok(ScenarioOutcome)` fields are copied verbatim —
///    the consumer's verdict stands. `Err(ScenarioError)` maps to
///    `Unsupported` for [`ScenarioError::Unsupported`] and `Fail`
///    for everything else, with the error display in `detail`.
/// 4. Timeout: when `spec.timeout_ms` is declared, the measured
///    elapsed time is recorded as [`ScenarioTimeout`] on **every**
///    path (pre-flight rejection included — an honest ~0ms record).
///    A `Pass` verdict with `timed_out == true` is downgraded to
///    `Fail`: a run that exceeded its budget cannot claim pass.
///
/// `cleanup` is never touched — the consumer/driver records it on
/// the returned report after teardown.
///
/// The body receives `&mut G` — `G` may be a concrete adapter or
/// `dyn SemanticGuiTest` (`?Sized`), so consumer scenario functions
/// taking either form work unchanged.
pub fn run_scenario<G: SemanticGuiTest + ?Sized>(
    gui: &mut G,
    spec: &ScenarioSpec,
    body: impl FnOnce(&mut G) -> Result<ScenarioOutcome, ScenarioError>,
) -> ScenarioReport {
    let started = Instant::now();
    let mut report = ScenarioReport::new(
        spec.run_id.clone(),
        spec.scenario.clone(),
        spec.case.clone(),
        gui.driver(),
    );
    report.capabilities = gui.capabilities();
    report.required.clone_from(&spec.required);
    report.environment.clone_from(&spec.environment);

    if let Err(error) = gui.require_capabilities(&spec.required) {
        report.verdict = verdict_for_error(&error);
        report.detail = Some(error.to_string());
        record_timeout(&mut report, spec, started);
        return report;
    }

    match body(gui) {
        Ok(outcome) => {
            report.verdict = outcome.verdict;
            report.expected = outcome.expected;
            report.observed = outcome.observed;
            report.targets = outcome.targets;
            report.receipts = outcome.receipts;
            report.evidence = outcome.evidence;
            report.detail = outcome.detail;
        }
        Err(error) => {
            report.verdict = verdict_for_error(&error);
            report.detail = Some(error.to_string());
        }
    }

    if let Some(timeout) = record_timeout(&mut report, spec, started)
        && timeout.timed_out
        && report.verdict == ScenarioVerdict::Pass
    {
        report.verdict = ScenarioVerdict::Fail;
        let timeout_detail = format!(
            "timeout budget {}ms exceeded (elapsed {}ms)",
            timeout.budget_ms, timeout.elapsed_ms
        );
        report.detail = Some(match report.detail.take() {
            Some(existing) => format!("{existing}; {timeout_detail}"),
            None => timeout_detail,
        });
    }
    report
}

/// The verdict an execution-path [`ScenarioError`] maps to:
/// `Unsupported` stays `unsupported`; every contract/driver
/// failure is `fail` — never silently `pass`/`not_run`.
fn verdict_for_error(error: &ScenarioError) -> ScenarioVerdict {
    match error {
        ScenarioError::Unsupported { .. } => ScenarioVerdict::Unsupported,
        _ => ScenarioVerdict::Fail,
    }
}

/// Records [`ScenarioTimeout`] on `report` when `spec` declared a
/// budget; returns the record for the caller's verdict correction.
fn record_timeout(
    report: &mut ScenarioReport,
    spec: &ScenarioSpec,
    started: Instant,
) -> Option<ScenarioTimeout> {
    let budget_ms = spec.timeout_ms?;
    let elapsed = started.elapsed();
    let timeout = ScenarioTimeout {
        budget_ms,
        elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        timed_out: elapsed > Duration::from_millis(budget_ms),
    };
    report.timeout = Some(timeout);
    Some(timeout)
}

// ---------------------------------------------------------------------
// bounded settle
// ---------------------------------------------------------------------

/// Bounded settle for receipt/revision conditions — the semantic
/// port's wait primitive (Issue #464 detail 7).
///
/// Evaluates `ready` (a read-only predicate over `gui`, typically
/// built on [`SemanticGuiTest::observe`]) before each settle step
/// and once more after the last allowed step. Returns `Ok(true)` as
/// soon as `ready` holds, `Ok(false)` when `max_settles` settle
/// steps were not enough, and propagates the first [`ScenarioError`]
/// from [`SemanticGuiTest::settle`]. There is no sleep and no
/// unbounded wait: `max_settles` is the entire budget.
///
/// `ready` receives `&G` — it can observe but cannot mutate, so a
/// wait condition can never become a hidden write path.
///
/// # Errors
/// Propagates [`SemanticGuiTest::settle`]'s [`ScenarioError`].
pub fn settle_until<G: SemanticGuiTest + ?Sized>(
    gui: &mut G,
    mut ready: impl FnMut(&G) -> bool,
    max_settles: usize,
) -> Result<bool, ScenarioError> {
    for _ in 0..max_settles {
        if ready(gui) {
            return Ok(true);
        }
        gui.settle()?;
    }
    Ok(ready(gui))
}

// ---------------------------------------------------------------------
// tests — a purpose-built mock port, no ViewHost needed
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use ruviform_ui::WindowId;
    use ruviform_ui::behavior::ControlInput;
    use ruviform_ui::ipc::Event;
    use ruviform_ui::view::{ComponentKey, ComponentSnapshot};

    use super::*;
    use crate::scenario::{MintedIntent, ScenarioDriver, ScenarioTarget, TargetQuery};

    /// Minimal [`SemanticGuiTest`] double: configurable capabilities,
    /// a settle counter, and an optional settle failure. Every
    /// operation that needs a real view returns a driver error — the
    /// runner tests exercise assembly logic, not `ViewHost` behavior.
    struct MockGui {
        capabilities: BTreeSet<ScenarioCapability>,
        settles: usize,
        settle_error: bool,
    }

    impl MockGui {
        fn new(capabilities: &[ScenarioCapability]) -> Self {
            Self {
                capabilities: capabilities.iter().copied().collect(),
                settles: 0,
                settle_error: false,
            }
        }
    }

    impl SemanticGuiTest for MockGui {
        fn driver(&self) -> ScenarioDriver {
            ScenarioDriver::Fake
        }

        fn capabilities(&self) -> BTreeSet<ScenarioCapability> {
            self.capabilities.clone()
        }

        fn windows(&self) -> Vec<WindowId> {
            Vec::new()
        }

        fn resolve_target(
            &mut self,
            _query: &TargetQuery,
        ) -> Result<ScenarioTarget, ScenarioError> {
            Err(ScenarioError::Driver("mock has no views".to_string()))
        }

        fn dispatch_input(
            &mut self,
            _target: &ScenarioTarget,
            _key: &ComponentKey,
            _input: ControlInput,
        ) -> Result<(), ScenarioError> {
            Err(ScenarioError::Driver("mock has no views".to_string()))
        }

        fn select_row(
            &mut self,
            _target: &ScenarioTarget,
            _key: &ComponentKey,
            _row: usize,
        ) -> Result<(), ScenarioError> {
            Err(ScenarioError::Driver("mock has no views".to_string()))
        }

        fn select_tab(
            &mut self,
            _target: &ScenarioTarget,
            _key: &ComponentKey,
            _index: usize,
        ) -> Result<(), ScenarioError> {
            Err(ScenarioError::Driver("mock has no views".to_string()))
        }

        fn invoke(
            &mut self,
            _target: &ScenarioTarget,
            _key: &ComponentKey,
        ) -> Result<(), ScenarioError> {
            Err(ScenarioError::Driver("mock has no views".to_string()))
        }

        fn observe(
            &self,
            _target: &ScenarioTarget,
            _key: &ComponentKey,
        ) -> Result<ComponentSnapshot, ScenarioError> {
            Err(ScenarioError::Driver("mock has no views".to_string()))
        }

        fn settle(&mut self) -> Result<(), ScenarioError> {
            self.settles += 1;
            if self.settle_error {
                Err(ScenarioError::Driver("settle failed".to_string()))
            } else {
                Ok(())
            }
        }

        fn drain_intents(&mut self) -> Vec<MintedIntent> {
            Vec::new()
        }

        fn post_event(
            &mut self,
            _target: &ScenarioTarget,
            _event: &Event,
        ) -> Result<(), ScenarioError> {
            Ok(())
        }
    }

    fn spec() -> ScenarioSpec {
        ScenarioSpec::new(RunId::new("run-1"), ScenarioId::new("scenario-runner"))
            .case(CaseId::new("case-1"))
            .environment(serde_json::json!({"os": "test"}))
    }

    #[test]
    fn preflight_unsupported_skips_body() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        let ran = Cell::new(false);
        let report = run_scenario(
            &mut gui,
            &spec().require(ScenarioCapability::VisualCapture),
            |_| {
                ran.set(true);
                Ok(ScenarioOutcome::new(ScenarioVerdict::Pass))
            },
        );

        assert!(!ran.get(), "body must not run on capability failure");
        assert_eq!(report.verdict, ScenarioVerdict::Unsupported);
        assert_eq!(report.driver, ScenarioDriver::Fake);
        assert!(report.required.contains(&ScenarioCapability::VisualCapture));
        assert!(
            report
                .capabilities
                .contains(&ScenarioCapability::SemanticAction)
        );
        assert!(report.environment.is_some());
        assert!(
            report
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("unsupported capability: VisualCapture"))
        );
    }

    #[test]
    fn preflight_met_runs_body() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        let report = run_scenario(
            &mut gui,
            &spec().require(ScenarioCapability::SemanticAction),
            |_| Ok(ScenarioOutcome::new(ScenarioVerdict::Pass)),
        );
        assert_eq!(report.verdict, ScenarioVerdict::Pass);
    }

    #[test]
    fn body_error_maps_to_verdict() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);

        let report = run_scenario(&mut gui, &spec(), |_| {
            Err(ScenarioError::StaleGeneration {
                expected: 1,
                got: 2,
            })
        });
        assert_eq!(report.verdict, ScenarioVerdict::Fail);
        assert!(report.detail.is_some());

        let report = run_scenario(&mut gui, &spec(), |_| {
            Err(ScenarioError::Unsupported {
                capability: ScenarioCapability::Ime,
            })
        });
        assert_eq!(report.verdict, ScenarioVerdict::Unsupported);
    }

    #[test]
    fn outcome_verdict_and_evidence_are_recorded() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        let report = run_scenario(&mut gui, &spec(), |_| {
            let mut outcome = ScenarioOutcome::new(ScenarioVerdict::NotRun);
            outcome.expected = Some(serde_json::json!({"phase": "applied"}));
            outcome.observed = Some(serde_json::json!({"phase": "pending"}));
            outcome.evidence.push("fixture:lab".to_string());
            outcome.detail = Some("prerequisite absent".to_string());
            Ok(outcome)
        });

        assert_eq!(report.verdict, ScenarioVerdict::NotRun);
        assert_eq!(
            report.expected,
            Some(serde_json::json!({"phase": "applied"}))
        );
        assert_eq!(
            report.observed,
            Some(serde_json::json!({"phase": "pending"}))
        );
        assert_eq!(report.evidence, vec!["fixture:lab".to_string()]);
        assert_eq!(report.detail.as_deref(), Some("prerequisite absent"));
        assert_eq!(report.case, Some(CaseId::new("case-1")));
    }

    #[test]
    fn timeout_exceeded_downgrades_pass() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        // A zero budget plus a body that measurably consumes time:
        // `timed_out` compares the exact Duration, so any nonzero
        // elapsed exceeds 0ms.
        let report = run_scenario(&mut gui, &spec().timeout_ms(0), |_| {
            let mut acc = 0u64;
            for i in 0..1_000_000u64 {
                acc = acc.wrapping_add(i);
            }
            std::hint::black_box(acc);
            Ok(ScenarioOutcome::new(ScenarioVerdict::Pass))
        });

        let timeout = report.timeout.expect("timeout recorded");
        assert_eq!(timeout.budget_ms, 0);
        assert!(timeout.timed_out, "elapsed must exceed a 0ms budget");
        assert_eq!(report.verdict, ScenarioVerdict::Fail);
        assert!(report.detail.is_some_and(|d| d.contains("timeout")));
    }

    #[test]
    fn timeout_within_budget_keeps_verdict() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        let report = run_scenario(&mut gui, &spec().timeout_ms(60_000), |_| {
            Ok(ScenarioOutcome::new(ScenarioVerdict::Pass))
        });

        let timeout = report.timeout.expect("timeout recorded");
        assert!(!timeout.timed_out);
        assert_eq!(report.verdict, ScenarioVerdict::Pass);
        assert!(report.detail.is_none());
    }

    #[test]
    fn no_budget_leaves_timeout_unset() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        let mut report = run_scenario(&mut gui, &spec(), |_| {
            Ok(ScenarioOutcome::new(ScenarioVerdict::Pass))
        });
        assert!(report.timeout.is_none());
        // cleanup stays consumer-owned: unset until the caller records it.
        report.cleanup = Some(serde_json::json!({"windows_closed": 1}));
        assert_eq!(
            report.cleanup,
            Some(serde_json::json!({"windows_closed": 1}))
        );
    }

    #[test]
    fn settle_until_returns_true_when_ready_holds() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        let calls = Cell::new(0usize);
        let ok = settle_until(
            &mut gui,
            |_| {
                calls.set(calls.get() + 1);
                calls.get() >= 2
            },
            5,
        )
        .expect("settle_until");
        assert!(ok);
        // ready held on the second evaluation — exactly one settle ran.
        assert_eq!(calls.get(), 2);
        assert_eq!(gui.settles, 1);
    }

    #[test]
    fn settle_until_is_bounded_and_reports_false() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        let calls = Cell::new(0usize);
        let ok = settle_until(
            &mut gui,
            |_| {
                calls.set(calls.get() + 1);
                false
            },
            3,
        )
        .expect("settle_until");
        assert!(!ok);
        assert_eq!(gui.settles, 3, "exactly max_settles settle steps");
        // max_settles loop checks + one final evaluation.
        assert_eq!(calls.get(), 4);
    }

    #[test]
    fn settle_until_initial_ready_needs_no_settle() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        let ok = settle_until(&mut gui, |_| true, 0).expect("settle_until");
        assert!(ok);
        assert_eq!(gui.settles, 0);
    }

    #[test]
    fn settle_until_propagates_settle_error() {
        let mut gui = MockGui::new(&[ScenarioCapability::SemanticAction]);
        gui.settle_error = true;
        let err = settle_until(&mut gui, |_| false, 3).expect_err("settle error");
        assert!(matches!(err, ScenarioError::Driver(_)));
        assert_eq!(gui.settles, 1, "stops at the first failing settle");
    }
}
