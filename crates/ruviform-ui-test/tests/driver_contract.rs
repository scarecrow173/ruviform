//! Issue #390 `DriverContractFixture` — UI-DVR case manifest の
//! 整合性検査と、現行 main で実行可能な semantic fixture
//! （UI-DVR-01/05/07/08 の `fake` layer）を提供する。
//!
//! `blocked`/`not_run` の binding は契約到着まで実行しない —
//! Fake semantic pass を GPUI/実 OS/Native Dock の成功と混同しない
//! （判定語彙 `pass/fail/not_run/unsupported/blocked`、
//! `RUVIFORM_DRIVER_LAB_EVIDENCE` 指定時に evidence.jsonl へ記録）。

use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use ruviform_ui::{
    DataRegistry, DemoService, DockTarget, DockWorkspace, HostEvent, UiRuntime, ViewInstanceId,
    WindowEvent, WindowHandle, WindowId,
};
use ruviform_ui_test::{HeadlessIo, TestApp, TestIo, driver_lab};

/// Sink app — event を飲み込む最小 `TestApp`（close-decision 等は
/// 本 fixture の検証対象外）。
struct App;

impl TestApp for App {
    fn on_event(&mut self, _wev: WindowEvent, _wh: &mut dyn WindowHandle) {}
}

fn manifest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn cases() -> driver_lab::CaseManifest {
    driver_lab::load_cases(&manifest_dir().join("fixtures/driver-lab/cases.json"))
        .expect("cases.json が load できること")
}

fn stories() -> driver_lab::StoryManifest {
    driver_lab::load_stories(&manifest_dir().join("fixtures/driver-lab/stories.json"))
        .expect("stories.json が load できること")
}

fn binding_status<'a>(manifest: &'a driver_lab::CaseManifest, case: &str, layer: &str) -> &'a str {
    manifest
        .cases
        .iter()
        .find(|c| c.id == case)
        .and_then(|c| c.bindings.iter().find(|b| b.layer == layer))
        .map_or("<missing>", |b| b.status.as_str())
}

/// manifest/story registry の構造整合性 + cargo-test mechanism の
/// 実在確認（package・test file・filter fn）。
#[test]
fn manifests_are_consistent() {
    let cases = cases();
    let stories = stories();
    let errors = driver_lab::validate_cases(&cases);
    assert!(errors.is_empty(), "cases.json violations: {errors:?}");
    let errors = driver_lab::validate_stories(&stories);
    assert!(errors.is_empty(), "stories.json violations: {errors:?}");

    let workspace_root = manifest_dir().join("../..").canonicalize().unwrap();
    let metadata = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .expect("workspace metadata");
    let errors = driver_lab::verify_mechanisms(&cases, &metadata, &workspace_root);
    assert!(errors.is_empty(), "mechanism violations: {errors:?}");

    // UI-DVR-01..09 が requiredCases で pin されていること。
    assert_eq!(cases.required_cases.len(), 9);
}

/// UI-DVR-01 fake: driver 非搭載の headless semantic fixture が
/// GUI toolkit なしで window lifecycle を回せること。
#[test]
fn ui_dvr_01_driverless_headless_smoke() {
    let mut io = HeadlessIo::new(App);
    assert!(io.windows().is_empty());
    let w = io
        .create_window(&ruviform_ui::WindowConfig::default())
        .expect("headless window 作成");
    assert_eq!(io.windows().len(), 1);
    let _ = w;
    driver_lab::emit_evidence(
        "UI-DVR-01",
        "fake",
        binding_status(&cases(), "UI-DVR-01", "fake"),
        "pass",
        "cargo-test:ruviform-ui-test/driver_contract",
        "driverless headless window lifecycle smoke",
    );
}

/// UI-DVR-05 fake: `DockWorkspace` の 2-phase move/rollback と
/// detach/reattach の `ViewInstanceId` 安定性（同一 oracle を
/// driver-lab registry 側でも検証）。
#[test]
fn ui_dvr_05_dock_two_phase_move_rollback() {
    let mut ws = DockWorkspace::new();
    let w1 = WindowId::new(1).unwrap();
    let w2 = WindowId::new(2).unwrap();
    ws.register_window(w1);
    ws.register_window(w2);
    let v1 = ViewInstanceId::new(1).unwrap();
    ws.attach_view(v1, DockTarget::Window(w1)).unwrap();
    assert_eq!(ws.owner_of(v1), Some(w1));

    // 2-phase: begin で in-transfer、commit で atomic に所有権切替。
    let token = ws
        .begin_move(v1, DockTarget::Window(w2))
        .expect("begin_move");
    assert_eq!(ws.owner_of(v1), Some(w1), "commit 前は source が owner");
    let report = ws.commit_move(token).expect("commit_move");
    assert_eq!(ws.owner_of(v1), Some(w2));
    assert_eq!(report.view, v1);

    // rollback: 転送中に destination window が消えると commit は
    // 失敗し、view は source を保持する。
    let w3 = WindowId::new(3).unwrap();
    ws.register_window(w3);
    let v2 = ViewInstanceId::new(2).unwrap();
    ws.attach_view(v2, DockTarget::Window(w1)).unwrap();
    let token = ws
        .begin_move(v2, DockTarget::Window(w3))
        .expect("begin_move");
    ws.remove_window(w3, ruviform_ui::WindowCloseDisposition::DetachViews)
        .unwrap();
    assert!(
        ws.commit_move(token).is_err(),
        "destination 喪失時は commit が失敗すること"
    );
    assert_eq!(ws.owner_of(v2), Some(w1), "rollback で source を保持");

    driver_lab::emit_evidence(
        "UI-DVR-05",
        "fake",
        binding_status(&cases(), "UI-DVR-05", "fake"),
        "pass",
        "cargo-test:ruviform-ui-test/driver_contract",
        "2-phase move/rollback + owner semantics",
    );
}

/// UI-DVR-07 fake: Capabilities/DeniedCapabilities が未対応を
/// 型付きで正直に報告すること。
#[test]
fn ui_dvr_07_capability_honest_report() {
    let io = HeadlessIo::new(App);
    let caps = io.capabilities();
    // RF-GT-04 の port 外 scope は常に false — 「未実装」と
    // 「port 外拒否」を型で区別する契約。
    assert!(!caps.denied.input_injection, "input_injection は port 外");
    assert!(!caps.denied.dom_access, "dom_access は port 外");
    // backend 非実装は広告しない — headless は実 screenshot を
    // 持たないので false を報告し、fixture 未登録時は fixture=false。
    assert!(!caps.screenshot, "headless は screenshot を広告しない");
    assert!(!caps.fixture, "fixture 未登録なら fixture=false を報告");
    assert_ne!(caps.clock_domains, [] as [String; 0]);
    assert_ne!(caps.observation, [] as [String; 0]);

    driver_lab::emit_evidence(
        "UI-DVR-07",
        "fake",
        binding_status(&cases(), "UI-DVR-07", "fake"),
        "pass",
        "cargo-test:ruviform-ui-test/driver_contract",
        "Capabilities/DeniedCapabilities honest report",
    );
}

/// UI-DVR-08 fake: window 関連 job の cancel と terminal state。
/// subscription teardown（JS bridge unsubscribe）は bridge 契約側の
/// scope で本 fixture の対象外（manifest に記録済み）。
#[test]
fn ui_dvr_08_teardown_cancels_jobs() {
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(DemoService::new())
        .expect("demo service");
    let w = WindowId::new(1).unwrap();

    // window-scoped job を dispatch（demo.sleep は JobSpec を返す）。
    let msg = r#"{"v":1,"kind":"intent","id":"dvr-08","op":"demo.sleep","params":{"ms":60000}}"#
        .to_string();
    let _ = rt.handle_window(w, HostEvent::Message(msg));

    // teardown: window の job が cancel され terminal state に至る。
    let cancelled = rt.cancel_window_jobs(w);
    assert!(cancelled >= 1, "window job が cancel されること");

    // job は cancel signal を観測して終端遷移する — poll で終端まで
    // 排出する（sleep job は 10ms 間隔で cancel を観測する）。
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut all = Vec::new();
    while rt.has_running_jobs() && std::time::Instant::now() < deadline {
        all.extend(rt.poll(Some(std::time::Duration::from_millis(50))));
    }
    all.extend(rt.drain_jobs());
    assert!(!rt.has_running_jobs(), "残存 job がないこと");
    assert!(
        all.iter().any(|e| matches!(
            e,
            ruviform_ui::Event::Job {
                state: ruviform_ui::JobState::Cancelled,
                ..
            }
        )),
        "cancel terminal event が発行されること: {all:?}"
    );

    driver_lab::emit_evidence(
        "UI-DVR-08",
        "fake",
        binding_status(&cases(), "UI-DVR-08", "fake"),
        "pass",
        "cargo-test:ruviform-ui-test/driver_contract",
        "window job cancel → terminal state",
    );
}
