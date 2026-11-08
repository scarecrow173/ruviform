//! Visual/golden verification connection for [`SemanticGuiTest`]
//! adapters (Issue #314 slice 2 — design
//! `docs/designs/issue-314-golden-scenario.md`).
//!
//! [`VisualGuiTest`] extends the semantic port with one adapter-owned
//! primitive — [`capture_frame`](VisualGuiTest::capture_frame) — and
//! one shared step — [`golden_check`](VisualGuiTest::golden_check) —
//! that turns a captured frame into a [`GoldenComparison`] through the
//! slice-1 [`crate::golden`] utility. The step is deliberately *not* a
//! [`GuiAction`]: it returns a verdict-carrying report, not `()`.
//!
//! Honesty contract (same rule as `SemanticGuiTest::capabilities`):
//!
//! - `capture_frame` MUST re-check the target's `attach_epoch`/
//!   `generation` pins first — a stale target is a [`ScenarioError`],
//!   never a verdict.
//! - `capture_frame` MUST return [`CaptureOutcome::Unsupported`] or
//!   [`CaptureOutcome::NotRun`] rather than fabricate pixels when the
//!   adapter/environment cannot capture. Neither current adapter
//!   (Fake, GPUI-simulated) claims [`ScenarioCapability::VisualCapture`]
//!   nor produces pixels — both return `Unsupported` until slice 3
//!   lands a real capture backend.
//! - A semantic or draw-plan observation is never promoted into pixel
//!   evidence: only `Captured` pixels reach `BaselineStore::compare`,
//!   and only a measured comparison yields `pass`/`fail`.
//!
//! **RT contract:** non-RT only.

use crate::golden::{
    self, BaselineStore, EnvironmentMeta, GoldenComparison, GoldenKey, GoldenVerdict, RgbaImage,
    emit_evidence,
};

use super::{ScenarioError, ScenarioReport, ScenarioTarget, ScenarioVerdict, SemanticGuiTest};

/// The honest outcome of one pixel-capture attempt.
///
/// This is a *measured fact*, not an error: contract violations
/// (stale/missing target) stay [`ScenarioError`] on the `Err` side of
/// [`VisualGuiTest::capture_frame`], while "no pixels exist" is data
/// the golden layer records as `unsupported`/`not_run`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureOutcome {
    /// Real RGBA8 pixels captured from the target's surface.
    Captured(RgbaImage),
    /// This adapter/environment cannot produce pixels at all —
    /// maps to golden verdict `unsupported`. `reason` is a
    /// machine-readable `snake_case` code (e.g.
    /// `fake_viewhost_has_no_pixel_pipeline`).
    Unsupported {
        /// Machine-readable reason code.
        reason: String,
    },
    /// A capture path exists but a prerequisite is unmet — maps to
    /// golden verdict `not_run`. `reason` is a machine-readable
    /// `snake_case` code.
    NotRun {
        /// Machine-readable reason code.
        reason: String,
    },
}

/// The visual/golden port — [`SemanticGuiTest`] plus pixel capture.
///
/// Implementors provide [`capture_frame`](Self::capture_frame); the
/// [`golden_check`](Self::golden_check) step is a provided method so
/// every adapter shares one compare/evidence path. All signature
/// types are driver-neutral — no GPUI/wry/OS handle type appears here.
pub trait VisualGuiTest: SemanticGuiTest {
    /// Attempts to capture `target`'s current pixels.
    ///
    /// Contract:
    ///
    /// - Re-check `target.attach_epoch`/`target.generation` first —
    ///   stale pins return [`ScenarioError::StaleEpoch`]/
    ///   [`ScenarioError::StaleGeneration`], exactly like every other
    ///   [`SemanticGuiTest`] operation.
    /// - Return [`CaptureOutcome::Unsupported`] or
    ///   [`CaptureOutcome::NotRun`] when pixels cannot be produced —
    ///   never fabricate an image and never return `Captured` for a
    ///   draw plan or semantic snapshot.
    ///
    /// # Errors
    /// [`ScenarioError`] for contract violations (stale/missing
    /// target) and adapter/driver failures.
    fn capture_frame(&mut self, target: &ScenarioTarget) -> Result<CaptureOutcome, ScenarioError>;

    /// One golden verification step for `target`: capture → baseline
    /// compare → evidence.
    ///
    /// The measured [`CaptureOutcome`] is authoritative — this method
    /// does *not* consult [`SemanticGuiTest::capabilities`], so a
    /// scenario that skipped `require_capabilities` still gets an
    /// honest `unsupported`/`not_run` report rather than fabricated
    /// pixels. Scenarios that want pre-flight should call
    /// `require_capabilities({ScenarioCapability::VisualCapture})`
    /// first; the two layers answer different questions (declared
    /// capability vs. measured outcome).
    ///
    /// - `Captured` → [`BaselineStore::compare`] — `pass`/`fail`/
    ///   `blocked` come only from a real pixel comparison.
    /// - `Unsupported`/`NotRun` → [`golden::unsupported`] /
    ///   [`golden::not_run`] report carrying the adapter's reason.
    /// - Store/tool failures surface as [`ScenarioError::Driver`].
    ///
    /// The `rvf.golden.evidence/v1` evidence line is emitted through
    /// the `RUVIFORM_GOLDEN_EVIDENCE` env gate (no-op when unset).
    /// The returned [`GoldenComparison`] keeps `diff_image` for the
    /// caller to persist — artifact storage stays the caller's job.
    ///
    /// # Errors
    /// Propagates [`capture_frame`](Self::capture_frame)'s
    /// [`ScenarioError`] (stale/missing target, driver failure) and
    /// maps [`GoldenError`] to [`ScenarioError::Driver`].
    fn golden_check(
        &mut self,
        target: &ScenarioTarget,
        key: &GoldenKey,
        environment: Option<&EnvironmentMeta>,
        store: &BaselineStore,
    ) -> Result<GoldenComparison, ScenarioError> {
        let comparison = match self.capture_frame(target)? {
            // `GoldenError` has no [`ScenarioError`] variant —
            // `ScenarioError` keeps `Clone + PartialEq` and
            // `GoldenError` carries `io::Error`. Tool-level golden
            // failures therefore surface as [`ScenarioError::Driver`]
            // with the golden error's display text.
            CaptureOutcome::Captured(actual) => store
                .compare(key, &actual, environment)
                .map_err(|e| ScenarioError::Driver(format!("golden store: {e}")))?,
            CaptureOutcome::Unsupported { reason } => GoldenComparison {
                report: golden::unsupported(key, environment, reason),
                diff_image: None,
            },
            CaptureOutcome::NotRun { reason } => GoldenComparison {
                report: golden::not_run(key, environment, reason),
                diff_image: None,
            },
        };
        let _ = emit_evidence(&comparison.report);
        Ok(comparison)
    }
}

impl From<GoldenVerdict> for ScenarioVerdict {
    /// The five-value vocabulary is identical — `pass`/`fail`/
    /// `not_run`/`unsupported`/`blocked` map name-for-name. Neither
    /// enum is upgraded or downgraded by the conversion.
    fn from(verdict: GoldenVerdict) -> Self {
        match verdict {
            GoldenVerdict::Pass => Self::Pass,
            GoldenVerdict::Fail => Self::Fail,
            GoldenVerdict::NotRun => Self::NotRun,
            GoldenVerdict::Unsupported => Self::Unsupported,
            GoldenVerdict::Blocked => Self::Blocked,
        }
    }
}

/// Folds one golden step into a [`ScenarioReport`] — the canonical
/// embedding shape for golden results (design D4/D6).
///
/// - `report.evidence` gains `golden:<canonical key>` so the report
///   references the exact baseline identity that was evaluated.
/// - `report.observed["golden"]` gains the verdict/reason/detail/
///   baseline/stats JSON — merged into an existing object, created
///   when `observed` is `None`, and left untouched when the consumer
///   already stored a non-object value (the evidence reference is
///   still recorded).
///
/// This helper deliberately does **not** touch `report.verdict`:
/// the scenario verdict is the consumer's overall judgment across
/// all steps. Fold [`ScenarioVerdict::from`]`(comparison.report.
/// verdict)` into the consumer's own verdict logic — a golden `pass`
/// does not prove semantic checks passed, and a golden `fail` should
/// fail the scenario but the policy is the consumer's.
pub fn record_golden_step(report: &mut ScenarioReport, comparison: &GoldenComparison) {
    let golden = &comparison.report;
    report
        .evidence
        .push(format!("golden:{}", golden.key.canonical()));
    let entry = serde_json::json!({
        "verdict": golden.verdict.as_str(),
        "reason": golden.reason.as_str(),
        "detail": golden.detail.as_str(),
        "baseline": golden.baseline.as_deref(),
        "stats": golden.stats.as_ref(),
        "artifacts": golden.artifacts.as_slice(),
    });
    match &mut report.observed {
        Some(serde_json::Value::Object(map)) => {
            map.insert("golden".to_string(), entry);
        }
        Some(_) => {}
        None => {
            report.observed = Some(serde_json::json!({ "golden": entry }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{CaseId, RunId, ScenarioDriver, ScenarioId};

    fn key() -> GoldenKey {
        GoldenKey {
            story_id: "S1".to_string(),
            theme: "dark".to_string(),
            state: "idle".to_string(),
            driver_class: "test".to_string(),
            os_class: "test-os".to_string(),
            gpu_class: None,
            scale_factor: 1.0,
            font_class: None,
            revision: "rev1".to_string(),
        }
    }

    fn unsupported_comparison() -> GoldenComparison {
        GoldenComparison {
            report: golden::unsupported(&key(), None, "no_capture_backend"),
            diff_image: None,
        }
    }

    #[test]
    fn golden_verdict_maps_name_for_name() {
        for (golden, scenario) in [
            (GoldenVerdict::Pass, ScenarioVerdict::Pass),
            (GoldenVerdict::Fail, ScenarioVerdict::Fail),
            (GoldenVerdict::NotRun, ScenarioVerdict::NotRun),
            (GoldenVerdict::Unsupported, ScenarioVerdict::Unsupported),
            (GoldenVerdict::Blocked, ScenarioVerdict::Blocked),
        ] {
            assert_eq!(ScenarioVerdict::from(golden), scenario);
        }
    }

    #[test]
    fn lane_status_maps_blocked_to_not_run() {
        assert_eq!(ScenarioVerdict::Pass.lane_status(), "pass");
        assert_eq!(ScenarioVerdict::Fail.lane_status(), "fail");
        assert_eq!(ScenarioVerdict::NotRun.lane_status(), "not_run");
        assert_eq!(ScenarioVerdict::Unsupported.lane_status(), "unsupported");
        // consumer-verify lane has no `blocked` — it is `not_run` +
        // reason (the #390 convention, codified here).
        assert_eq!(ScenarioVerdict::Blocked.lane_status(), "not_run");
    }

    #[test]
    fn record_golden_step_writes_canonical_evidence_and_observed() {
        let mut report = ScenarioReport::new(
            RunId::new("run-1"),
            ScenarioId::new("scenario-visual"),
            Some(CaseId::new("case-1")),
            ScenarioDriver::Fake,
        );
        report.verdict = ScenarioVerdict::Pass;
        let comparison = unsupported_comparison();
        let canonical = comparison.report.key.canonical();

        record_golden_step(&mut report, &comparison);

        assert_eq!(report.evidence, vec![format!("golden:{canonical}")]);
        // The helper records evidence, never decides the scenario verdict.
        assert_eq!(report.verdict, ScenarioVerdict::Pass);
        let observed = report.observed.as_ref().expect("observed recorded");
        assert_eq!(observed["golden"]["verdict"], "unsupported");
        assert_eq!(observed["golden"]["reason"], "no_capture_backend");
        assert!(observed["golden"]["detail"].is_string());
        assert!(observed["golden"].get("stats").is_some());
    }

    #[test]
    fn record_golden_step_merges_into_existing_observed_object() {
        let mut report = ScenarioReport::new(
            RunId::new("run-1"),
            ScenarioId::new("scenario-visual"),
            None,
            ScenarioDriver::Fake,
        );
        report.observed = Some(serde_json::json!({"semantic": {"ok": true}}));
        record_golden_step(&mut report, &unsupported_comparison());
        let observed = report.observed.as_ref().unwrap();
        assert_eq!(observed["semantic"]["ok"], true);
        assert_eq!(observed["golden"]["verdict"], "unsupported");
    }

    #[test]
    fn record_golden_step_preserves_non_object_observed() {
        let mut report = ScenarioReport::new(
            RunId::new("run-1"),
            ScenarioId::new("scenario-visual"),
            None,
            ScenarioDriver::Fake,
        );
        report.observed = Some(serde_json::json!("consumer-defined scalar"));
        record_golden_step(&mut report, &unsupported_comparison());
        // Consumer's non-object observed is not clobbered; the golden
        // evidence reference is still recorded.
        assert_eq!(
            report.observed.as_ref().unwrap(),
            &serde_json::json!("consumer-defined scalar")
        );
        assert_eq!(report.evidence.len(), 1);
        assert!(report.evidence[0].starts_with("golden:"));
    }
}
