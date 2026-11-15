//! Scenario catalog + driver projection (Issue #631 catalog slice —
//! the headless half of the Test Lab / Scenario Console).
//!
//! A catalog is a **function registry**, not an executor: each
//! [`ScenarioEntry`] names one case — its id, scenario id, declared
//! [`ScenarioCapability`] requirements, spec extras, and the `run`
//! function that produces the [`ScenarioReport`] through
//! [`super::runner::run_scenario`]. [`ScenarioMatrix`] groups entries
//! under [`DriverBinding`] columns so a lane binary, an integration
//! test, or a future console can enumerate the same cases without
//! duplicating the list.
//!
//! [`ScenarioMatrix::project`] is the driver-projection half: it
//! answers "would `run_scenario` pre-flight pass this entry on this
//! driver" from *declared* data only — it never executes a case and
//! never upgrades a verdict. Capability declarations stay honest:
//! a binding lists only what its driver actually provides, and an
//! entry's `required` is copied verbatim into `ScenarioSpec.required`
//! so the live pre-flight still decides.
//!
//! Nothing here is a JSON DSL, a second runner, or a verdict
//! authority — the design constraint from
//! `docs/designs/issue-464-scenario-runner.md` ("runner は 1 関数で
//! executor ではない") is preserved: every `run` is an ordinary Rust
//! function over [`super::SemanticGuiTest`]-compatible adapters, and
//! [`run_entry`]/[`run_entries`]/[`run_matrix`] only sequence the
//! `run_scenario` calls the entries already declare.
//!
//! **RT contract:** non-RT only.

use std::collections::BTreeSet;

use serde_json::Value;

use super::runner::ScenarioSpec;
use super::{CaseId, RunId, ScenarioCapability, ScenarioDriver, ScenarioId, ScenarioReport};

// ---------------------------------------------------------------------
// entry / spec extras
// ---------------------------------------------------------------------

/// The per-case [`ScenarioSpec`] fields a catalog entry customizes
/// beyond identity and required capabilities — environment facts and
/// an optional wall-clock budget.
///
/// `environment` is the *spec-level* declaration: it lands on
/// `ScenarioSpec.environment` (and transiently on the report inside
/// `run_scenario`), but consumers that finalize `report.environment`
/// post-run — the plugin-dock lanes do — overwrite it. Keep it
/// honest regardless: it is what a console lists before anything
/// runs.
#[derive(Debug, Clone)]
pub struct ScenarioSpecExtras {
    /// `environment` JSON recorded on the spec; `None` leaves the
    /// field unset.
    pub environment: Option<Value>,
    /// Wall-clock budget in milliseconds declared on the spec
    /// (`ScenarioSpec.timeout_ms`); `None` = undeclared.
    pub timeout_ms: Option<u64>,
}

impl ScenarioSpecExtras {
    /// No extras — the common case (no environment, no budget).
    #[must_use]
    pub const fn none() -> Self {
        Self {
            environment: None,
            timeout_ms: None,
        }
    }
}

/// One catalog row — a case's identity, capability declaration, spec
/// extras, and run function.
///
/// `C` is the consumer-defined lane context: everything `spec_extras`
/// and `run` need that is not the [`ScenarioSpec`] itself (lane
/// environment, shared artifact slots, a live session driver, ...).
/// The catalog never inspects `C` — it only threads `&C`/`&mut C`
/// through, which is what keeps the vocabulary driver-independent:
/// `ruviform-ui-test` names no adapter type, and GPUI/plugin-host
/// types stay inside the consumer crate that defines `C`.
///
/// `case`/`scenario` are `&'static str` so tables stay const-
/// constructible; [`ScenarioEntry::spec`] is the only place they
/// become [`CaseId`]/[`ScenarioId`], so a case's spec can never drift
/// from its declared identity.
#[derive(Debug)]
pub struct ScenarioEntry<C> {
    /// Case id — becomes [`CaseId`] on the spec/report.
    pub case: &'static str,
    /// Scenario id — becomes [`ScenarioId`] on the spec/report.
    pub scenario: &'static str,
    /// Capabilities the case requires, in declaration order. Copied
    /// verbatim into `ScenarioSpec.required` (the live pre-flight
    /// still decides) and used by [`ScenarioMatrix::project`].
    pub required: &'static [ScenarioCapability],
    /// Builds the per-case [`ScenarioSpecExtras`] from the lane
    /// context.
    pub spec_extras: fn(&C) -> ScenarioSpecExtras,
    /// Executes the case: receives the assembled [`ScenarioSpec`]
    /// and the lane context, routes through
    /// [`super::runner::run_scenario`], and returns the finalized
    /// [`ScenarioReport`]. Verdicts are the case body's own —
    /// catalog membership upgrades nothing.
    pub run: fn(&mut C, &ScenarioSpec) -> ScenarioReport,
}

// Manual impls: every field is `Copy` regardless of `C`, so derived
// bounds (`C: Copy`) would be spurious — they would forbid moving
// entries out of `static`/`const` tables for non-`Copy` lane
// contexts.
impl<C> Copy for ScenarioEntry<C> {}

impl<C> Clone for ScenarioEntry<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> ScenarioEntry<C> {
    /// Assembles the [`ScenarioSpec`] this entry declares under
    /// `run_id`: ids from `self`, `required` from the declaration,
    /// `environment`/`timeout_ms` from [`ScenarioEntry::spec_extras`].
    #[must_use]
    pub fn spec(&self, run_id: &RunId, ctx: &C) -> ScenarioSpec {
        let extras = (self.spec_extras)(ctx);
        let mut spec = ScenarioSpec::new(run_id.clone(), ScenarioId::new(self.scenario))
            .case(CaseId::new(self.case))
            .required(self.required.iter().copied().collect::<BTreeSet<_>>());
        spec.environment = extras.environment;
        spec.timeout_ms = extras.timeout_ms;
        spec
    }
}

// ---------------------------------------------------------------------
// driver binding / matrix / projection
// ---------------------------------------------------------------------

/// One driver column of a [`ScenarioMatrix`] — which
/// [`ScenarioDriver`] the bound cases run under, the capabilities it
/// honestly provides, and the entries themselves in run order.
#[derive(Debug)]
pub struct DriverBinding<'a, C> {
    /// Driver identity the bound cases' reports carry
    /// (`ScenarioReport.driver`). Declared for projection/display —
    /// the report's own `driver` always comes from the live adapter.
    pub driver: ScenarioDriver,
    /// Capabilities the driver honestly provides — the projection
    /// input for [`ScenarioMatrix::project`]. This is a *declaration*
    /// for "would `run_scenario` pre-flight pass", not a live query;
    /// consumers should keep it equal to what the adapter's
    /// `SemanticGuiTest::capabilities` reports.
    pub capabilities: &'static [ScenarioCapability],
    /// The entries this driver executes, in run order.
    pub cases: &'a [ScenarioEntry<C>],
}

/// A scenario catalog organized by driver binding — the single
/// source a lane binary, integration test, or future console
/// consumes.
///
/// `run_matrix` flattens bindings in order, so catalog order is
/// binding-major then case order — declare bindings in the order
/// reports should be produced.
#[derive(Debug)]
pub struct ScenarioMatrix<'a, C> {
    /// Driver bindings in run order.
    pub bindings: &'a [DriverBinding<'a, C>],
}

/// One projected cell of [`ScenarioMatrix::project`] — the answer to
/// "would this entry run on this driver", computed from declared
/// data only. Nothing executed; no verdict is implied beyond the
/// capability pre-flight mirror.
#[derive(Debug)]
pub struct EntryProjection<'a, C> {
    /// The catalog entry.
    pub entry: &'a ScenarioEntry<C>,
    /// The driver binding the entry projects onto.
    pub driver: ScenarioDriver,
    /// `true` when the binding's declared capabilities cover
    /// `entry.required` — the same answer `run_scenario`'s
    /// pre-flight gives on a live adapter claiming exactly these
    /// capabilities.
    pub runnable: bool,
    /// The first missing required capability in
    /// [`ScenarioCapability`]'s `Ord` order — the same capability
    /// [`super::SemanticGuiTest::require_capabilities`] would name —
    /// when `runnable` is `false`.
    pub missing: Option<ScenarioCapability>,
}

impl<'a, C> ScenarioMatrix<'a, C> {
    /// Every `(entry, binding)` cell in run order — bindings outer,
    /// cases inner.
    pub fn cells(
        &self,
    ) -> impl Iterator<Item = (&'a ScenarioEntry<C>, &'a DriverBinding<'a, C>)> + 'a {
        let bindings = self.bindings;
        bindings
            .iter()
            .flat_map(|binding| binding.cases.iter().map(move |entry| (entry, binding)))
    }

    /// Projects every cell onto its binding's declared capabilities —
    /// the console's "would this case run on this driver" listing,
    /// computed without executing anything.
    ///
    /// `missing` follows [`ScenarioCapability`]'s `Ord` (the
    /// `BTreeSet` ordering `require_capabilities` reports first),
    /// independent of the declaration order in `entry.required`.
    #[must_use]
    pub fn project(&self) -> Vec<EntryProjection<'a, C>> {
        self.cells()
            .map(|(entry, binding)| {
                let required: BTreeSet<ScenarioCapability> =
                    entry.required.iter().copied().collect();
                let missing = required
                    .iter()
                    .copied()
                    .find(|capability| !binding.capabilities.contains(capability));
                EntryProjection {
                    entry,
                    driver: binding.driver,
                    runnable: missing.is_none(),
                    missing,
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------
// execution projection — sequences the declared `run_scenario` calls
// ---------------------------------------------------------------------

/// Runs one catalog entry: assembles its [`ScenarioSpec`] via
/// [`ScenarioEntry::spec`] and hands spec + context to the entry's
/// `run`. This is the catalog's only execution path — it adds no
/// verdict logic of its own.
#[must_use]
pub fn run_entry<C>(entry: &ScenarioEntry<C>, run_id: &RunId, ctx: &mut C) -> ScenarioReport {
    let spec = entry.spec(run_id, ctx);
    (entry.run)(ctx, &spec)
}

/// [`run_entry`] over a flat entry slice, in order — for catalogs
/// that are a single driver column or session-scoped tables that
/// cannot be `&'static`.
#[must_use]
pub fn run_entries<C>(
    entries: &[ScenarioEntry<C>],
    run_id: &RunId,
    ctx: &mut C,
) -> Vec<ScenarioReport> {
    entries
        .iter()
        .map(|entry| run_entry(entry, run_id, ctx))
        .collect()
}

/// [`run_entries`] over a whole [`ScenarioMatrix`] — every binding's
/// cases in catalog order.
#[must_use]
pub fn run_matrix<C>(
    matrix: &ScenarioMatrix<'_, C>,
    run_id: &RunId,
    ctx: &mut C,
) -> Vec<ScenarioReport> {
    matrix
        .cells()
        .map(|(entry, _)| run_entry(entry, run_id, ctx))
        .collect()
}

// ---------------------------------------------------------------------
// tests — a mock context; no adapter needed
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::scenario::ScenarioVerdict;

    /// Mock lane context recording the specs each `run` received.
    #[derive(Default)]
    struct Ctx {
        specs: Vec<ScenarioSpec>,
    }

    fn no_extras(_: &Ctx) -> ScenarioSpecExtras {
        ScenarioSpecExtras::none()
    }

    fn claimed_extras(_: &Ctx) -> ScenarioSpecExtras {
        ScenarioSpecExtras {
            environment: Some(json!({ "claim": "test-claim" })),
            timeout_ms: Some(1_000),
        }
    }

    /// `run` double: records the spec and returns a canned Pass
    /// report carrying the spec's identity — enough to prove the
    /// catalog threads spec + context correctly.
    fn run_record(ctx: &mut Ctx, spec: &ScenarioSpec) -> ScenarioReport {
        ctx.specs.push(spec.clone());
        let mut report = ScenarioReport::new(
            spec.run_id.clone(),
            spec.scenario.clone(),
            spec.case.clone(),
            ScenarioDriver::Fake,
        );
        report.verdict = ScenarioVerdict::Pass;
        report
    }

    static ENTRY_A: ScenarioEntry<Ctx> = ScenarioEntry {
        case: "case-a",
        scenario: "scn-a",
        required: &[ScenarioCapability::SemanticAction],
        spec_extras: no_extras,
        run: run_record,
    };

    static ENTRY_B: ScenarioEntry<Ctx> = ScenarioEntry {
        case: "case-b",
        scenario: "scn-a",
        required: &[ScenarioCapability::VisualCapture],
        spec_extras: claimed_extras,
        run: run_record,
    };

    static MATRIX: ScenarioMatrix<'static, Ctx> = ScenarioMatrix {
        bindings: &[
            DriverBinding {
                driver: ScenarioDriver::Fake,
                capabilities: &[ScenarioCapability::SemanticAction],
                cases: &[ENTRY_A],
            },
            DriverBinding {
                driver: ScenarioDriver::PluginHostProbe,
                capabilities: &[],
                cases: &[ENTRY_B],
            },
        ],
    };

    #[test]
    fn spec_assembles_identity_required_and_extras() {
        let ctx = Ctx::default();
        let run_id = RunId::new("run-1");

        let spec = ENTRY_A.spec(&run_id, &ctx);
        assert_eq!(spec.run_id, run_id);
        assert_eq!(spec.scenario, ScenarioId::new("scn-a"));
        assert_eq!(spec.case, Some(CaseId::new("case-a")));
        assert_eq!(
            spec.required,
            BTreeSet::from([ScenarioCapability::SemanticAction])
        );
        assert!(spec.environment.is_none());
        assert!(spec.timeout_ms.is_none());

        let spec = ENTRY_B.spec(&run_id, &ctx);
        assert_eq!(spec.environment, Some(json!({ "claim": "test-claim" })));
        assert_eq!(spec.timeout_ms, Some(1_000));
    }

    #[test]
    fn project_marks_missing_capability_in_ord_order() {
        // ENTRY_B requires VisualCapture; the Fake binding declares
        // only SemanticAction → not runnable, missing named.
        let projections = MATRIX.project();
        assert_eq!(projections.len(), 2);

        assert!(projections[0].runnable);
        assert_eq!(projections[0].missing, None);
        assert_eq!(projections[0].driver, ScenarioDriver::Fake);
        assert_eq!(projections[0].entry.case, "case-a");

        assert!(!projections[1].runnable);
        assert_eq!(
            projections[1].missing,
            Some(ScenarioCapability::VisualCapture)
        );
        assert_eq!(projections[1].driver, ScenarioDriver::PluginHostProbe);
    }

    #[test]
    fn run_entries_executes_in_order_with_built_specs() {
        let mut ctx = Ctx::default();
        let run_id = RunId::new("run-1");
        let entries = [ENTRY_A, ENTRY_B];
        let reports = run_entries(&entries, &run_id, &mut ctx);

        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].case, Some(CaseId::new("case-a")));
        assert_eq!(reports[1].case, Some(CaseId::new("case-b")));
        assert!(
            reports
                .iter()
                .all(|report| report.verdict == ScenarioVerdict::Pass)
        );
        // Each `run` saw exactly the spec `entry.spec` assembles.
        assert_eq!(ctx.specs.len(), 2);
        assert_eq!(ctx.specs[0].case, Some(CaseId::new("case-a")));
        assert_eq!(ctx.specs[1].timeout_ms, Some(1_000));
    }

    #[test]
    fn run_matrix_flattens_bindings_in_order() {
        let mut ctx = Ctx::default();
        let run_id = RunId::new("run-1");
        let reports = run_matrix(&MATRIX, &run_id, &mut ctx);

        let cases: Vec<_> = reports
            .iter()
            .map(|report| report.case.as_ref().map(CaseId::as_str))
            .collect();
        assert_eq!(cases, vec![Some("case-a"), Some("case-b")]);
        assert_eq!(ctx.specs.len(), 2);
    }
}
