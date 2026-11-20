//! `ruviform-plugin-dock-test` — test-only plugin-dock scenario
//! harness (Issue #631 first slice, RF-VTEST-04).
//!
//! This crate is the consumer-layer composition root the plugin↔UI
//! dependency rule requires: `ruviform-plugin-*` and `ruviform-ui-*`
//! crates may not depend on each other, so only a consumer can see
//! both sides (precedent: `ruviform-poc-native-dock`). It drives the
//! **real** `test_gain` VST3 cdylib (`ruviform-plugin-test-vst3`)
//! through the production host path (`Vst3Library` → `InProcessHost`
//! → `PluginEditorSessions`/`InProcessEditorOps`) and reports every
//! case through the driver-neutral `ruviform-ui-test` scenario
//! contract (`ScenarioSpec` → `run_scenario` → `ScenarioReport`).
//!
//! ## Verdict honesty (the point of the slice)
//!
//! - `load`/`discover`/`hash`/host-lifecycle steps that really
//!   executed against the binary report `Pass`.
//! - Editor attach goes through the real session/ops path; the
//!   current `test_gain` fixture has no editor (`createView` → null →
//!   `HostError::Unsupported`), so attach reports `Unsupported` with
//!   that detail — never `Pass`.
//! - Dock pixel checks report `NotRun`: no editor surface exists to
//!   capture and no `VisualCapture` driver is present.
//! - A missing artifact fails closed: every artifact-dependent case
//!   reports `NotRun`, never a silent skip or fake `Pass`.
//!
//! Fake slot / semantic view-mount success is contract conformance
//! only — it is never promoted to Native Dock or GUI-L2 evidence.
//!
//! The opt-in `real-dock` feature adds the Issue #631 real-window
//! lane (`real_dock`): the same dock spec mounted into a real OS
//! window through `RealOsGuiTest`, recording the `NativeSlot` →
//! surface `lease_parent` → `ParentSlotLease` → `InProcessEditorOps`
//! redeem → `session.open(Docked)` chain with `environment_kind`
//! `real-window`. Its `real-editor-attach` case drives the `test_editor`
//! fixture's real `IPlugView` and OS-observes the child HWND lifecycle
//! (existence/parent/visibility under the container, `IsWindow==0`
//! after detach); `real-dcomp-pixel` (Issue #532) additionally
//! captures the docked child's screen-composited pixels to pin the
//! known GPUI `DComp` topmost occlusion as an honest `Fail` until a
//! compositor fix lands; `real-dcomp-foreground` (Issue #532
//! foreground-probe slice) mutates the GPUI root's z-order/
//! foreground in bounded, restored stages to bisect external HWND
//! stacking from compositor-internal occlusion;
//! `real-dcomp-present` (Issue #532 presentation-probe slice)
//! observes — without mutating — the cover inventory,
//! presentation attributes, `PrintWindow` flag variants, and
//! screen-vs-suspect pixel fingerprints that discriminate the
//! remaining hit-test-transparent / child-presentation /
//! capture-blind-spot candidates; `real-dcomp-uncovered`
//! (Issue #532 uncovered-baseline slice) applies the foreground
//! probe's staged mutations to establish a no-foreign-cover state,
//! verifies it through hit-test roles, then re-observes hits and
//! screen pixels across a bounded settle curve plus post-settle
//! capture variants to decide whether the residual no-green is a
//! pre-paint frame or compositor-internal occlusion;
//! `real-dcomp-bandescape` (Issue #532 band-escape slice) mutates
//! the *container's* presentation relationship — a forced repaint,
//! then `WS_EX_LAYERED`+`LWA_ALPHA`, then a
//! `SetParent(GetDesktopWindow())` reparent onto the baseline
//! screen rect — and re-observes hit-tests and screen green at
//! each stage to bisect which escape route (minimal layered
//! container vs reparented owned popup) can free the docked
//! child's pixels from the presentation band, then restores and
//! verifies parent/exstyle/rect. The catalog
//! also
//! carries the AC1 remainder
//! rows — two-window focus transfer, real OS pointer/keyboard
//! injection, command-receipt settlement, live geometry/
//! scale-factor DPI observation, the armed window-close
//! lifecycle, and one honest `Unsupported` (`real-window-ime` —
//! `Ime` capability unclaimed, Issue #389) — plus the AC3 non-pixel
//! editor op rows (`real-editor-resize`/`real-editor-redock`, judged
//! by session events/HWND contract only, never pixels) and the
//! Issue #389 session lifecycle rows
//! (`real-session-terminate` — `Terminated{HostLost}` exactly-once,
//! child HWND teardown, host survival; `real-session-degraded` —
//! `open_or_degrade` `Degraded{Docked}` + `OpenOutcome::Degraded`;
//! `real-session-recovery` — refused open `Failed{Unsupported}` →
//! `recover(Docked)` `Recovered{Docked}`; `real-session-timeout` —
//! ZERO deadline policy + `tick_all` → `Failed{Timeout}`;
//! `real-editor-visibility` — the defined `set_visibility`
//! `Unsupported` refusal with `Attached` preserved;
//! `real-editor-input`/`real-editor-focus` — the `send_key`/
//! `set_focus` verbs judged by the fixture's OS-observable
//! `GetWindowTextW` event log; and the honest
//! `Unsupported` boundaries `real-session-crash`/`real-session-hang`
//! — process-killing/wedging fault modes the in-process lane cannot
//! survive).
//! `plugin-dock-real-scenario --blocked-probe` deterministically
//! proves the headless fail-closed mapping without opening a
//! session. It opens real OS windows, so it is never a
//! default feature and never a CI-required gate; a headless host
//! records `NotRun` + `environment_blocked`, never a fabricated
//! pass.
//!
//! **RT contract:** non-RT only. **Test-only:** no production crate
//! may depend on this crate; nothing here ships.

pub mod artifact;
pub mod jsonl;
#[cfg(feature = "real-dock")]
mod osgpu;
#[cfg(feature = "real-dock")]
mod osmut;
#[cfg(feature = "real-dock")]
mod osoccl;
#[cfg(feature = "real-dock")]
mod ospix;
#[cfg(feature = "real-dock")]
mod ospres;
pub mod probe;
pub mod real_catalog;
#[cfg(feature = "real-dock")]
pub mod real_dock;
pub mod scenario;

pub use artifact::{PLUGIN_PATH_ENV, fixture_lib_name, resolve_artifact, sha256_hex};
pub use probe::PluginHostProbe;
pub use scenario::{HEADLESS_MATRIX, LaneEnvironment, run_all, run_selected};
