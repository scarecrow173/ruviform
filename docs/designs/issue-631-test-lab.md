---
id: "DESIGN-034YaXW1QInVHHaV5zshvW"
type: "design"
status: "approved"
title: "Issue #631 GPUI Test Lab / Scenario Console GUI v1 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-631-test-lab.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-464-scenario-runner.md"
    - "docs/designs/issue-631-scenario-console.md"
    - "docs/designs/issue-631-real-window-dock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-631-test-lab.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-631-test-lab.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 GPUI Test Lab / Scenario Console GUI v1 設計

## Context

`plugin-dock-console`（headless CLI）と `plugin-dock-real-scenario`
（real-window lane）は merge 済みで、catalog 投影・selector 実行・
fail-closed 意味論が確立している。本 slice はその catalog を **実
GPUI Window 上の GUI** として dogfood する consumer を追加し、
同時に mise lane 簿記を整える。

境界の前提:

- `ruviform-plugin-dock-test` は `boundary.json` で
  `composition-root` 登録済み — plugin 層と UI 層の両方を見られる
  唯一の consumer crate であり、lab app をここに置けば
  boundary.json 変更・Coordinator 承認は不要。
- `ruviform-ui-gpui` の `GpuiDriver`/`ViewMount` は `gpui` feature
  配下。`plugin-dock-test` は既に optional dep として保持しており、
  `real-dock` feature が `real-scenario` を有効化する先例がある。
- `set_ui_jobs`/`UiJob` は `real-scenario`/`uitest` feature 専用で
  `lab-gui` では使えない — lab は `GpuiDriver::run` の
  `on_event` コールバックを直接駆動する設計にする
  （`RealOsGuiTest` と同じ app-thread + channel 構成を自前で持つ）。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test/Cargo.toml`: `lab-gui` feature
  （`dep:ruviform-ui-gpui` + `ruviform-ui-gpui/gpui`）と
  `plugin-dock-lab` bin（`required-features = ["lab-gui"]`）。
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-lab.rs`:
  windowed GUI v1 の全実装（bin 内に閉じ、lib 公開面は増やさない）。
- `mise.toml`: `test-ui-plugin-dock-console`・
  `test-ui-plugin-dock-lab` 新設、`test-ui-plugin-dock-real`
  description/コメントの `test_editor` 実態への修正。
- `docs/development-environment.md` lane 一覧、
  `evidence/rf631/verification.md` 節追記、doc chain/impl 記録。

### Out of Scope

- spec/design/plan/task/test-spec/impl 各 doc の index
  （`docs/**/README.md`）再生成 — Coordinator 統合時。
- catalog projection API の拡張（`ruviform-ui-test/src/scenario/`
  変更なし — `project()`/`cells()`/`real_catalog` で v1 要件を満たす）。
- real lane 実行の GUI 導線・real case の GUI 実行。
- `Cargo.lock`/`Cargo.toml` ルート・`boundary.json` の変更。
  `lab-gui` は既存 optional dep `ruviform-ui-gpui` の feature 有効化
  のみで新規 dependency なし（`gpui` feature は ui-gpui 側の既存
  feature であり、依存木は `real-dock` と同系統）。

## Components and Boundaries

- **`plugin-dock-lab` bin（composition root 内・唯一の新規コード）**
  - 引数: `--out <path>`（省略時 stdout）。selector 引数なし —
    実行対象は GUI の Table 選択 + Run ボタンで決まる。
  - catalog 投影: `HEADLESS_MATRIX.project()`（8 cell）+
    `real_catalog::REAL_CATALOG`（4 row）を起動時に 1 度だけ
    `CatalogRow` に写像。`runnable`/`missing`/`session_required`/
    `lane` を列として保持 — verdict 語彙は投影段階で出さない。
  - ViewSpec: 単一 `Panel` root（title = window 名）。children:
    status `Label`、catalog 注記 `Label`、catalog `Table`
    （`selection.target = "lab.catalog"`）、results 注記 `Label`、
    results `Table`（selection なし）、evidence `Label`、actions
    `Panel`（`Button`×3: run-selected / run-all / quit）。
    Tabs は v1 では使わない（単一画面で要件を満たし、tab-select
    intent 処理を増やさない — KISS）。
  - ViewHost: `AttachEpochAllocator` + `ViewHost::new(view_id,
    epochs.mint())` + `host.mount(spec)`（`real_dock.rs` L139-143
    と同型）。`ViewMount { host, theme: None }`。
  - Threading: `GpuiDriver::new()` → `create_view_window(&spec,
    mount)` → `driver.run(tick, on_event)` を専用 app thread で
    起動（`RealOsGuiTest::run` と同構成）。`on_event` は UI thread
    上で `DriverEvent` を受け、`Arc<Mutex<LabState>>` を更新して
    `update_view` で spec を差し替える。
  - scenario 実行: `lab.run-selected`/`lab.run-all` intent で
    `std::thread` worker を spawn → `run_selected(&env, keep)` →
    結果 `Vec<ScenarioReport>` を `mpsc` で UI thread へ。UI thread
    は `Tick` ごとに `try_recv` して results table 更新 +
    `render_jsonl` を `--out` へ append。実行中は status Label が
    `running` を示し、二重起動は拒否（status で gate）。
  - intent 処理: `WindowLifecycle::Action(intent)` で
    `intent.op == COMMAND_INTENT_OP` を確認し `params.intent` を
    `CommandIntent` に deserialize。`CommandIntent::Action` の
    `action` 名で dispatch:
    - `"select"`（TABLE_SELECT_ACTION、target `lab.catalog`）→
      `params.row` を selected index に記録。
    - `"lab.run-selected"` → selected row が headless cell なら
      `run_selected` をその case のみに絞って worker 起動。
      real row なら実行せず status に「real lane は
      `mise run test-ui-plugin-dock-real` へ委譲」を表示。
    - `"lab.run-all"` → headless 全 cell（`keep = |_,_| true`）。
    - `"lab.quit"` → `drv.exit()`。
    - 未知 action → `post_event` で rejected receipt。
    - 処理した intent には `Event::result_ok(command_id,
      {"status":"applied"})` を `drv.post_event` — receipt 契約に
      従い button/tab の pending を残さない。
  - Ready handshake: app thread は `create_view_window` 完了後に
    setup-ready を送り、`WindowLifecycle::Ready` を別 channel で
    main thread へ転送。main thread は timeout（15s、
    `RealOsGuiTest::COMMAND_TIMEOUT` と同値）で `Ready` を待つ:
    - `Ready` → interactive session 確定 → app thread に join
      （window close/quit で `exit` → `run` 復帰 → join 完了 →
      exit 0）。
    - timeout/起動失敗 → `environment_blocked` の blocked
      `ScenarioReport` 群（catalog 全 12 row、`NotRun`）を
      `render_jsonl` で `--out`（または stdout）へ出力し exit 0。
      app thread は process 終了で回収（`RealOsGuiTest` の
      join-on-every-path は body 型 session 向けで、永続 GUI では
      timeout 時に join 不能 — main 復帰 = process 終了で thread を
      畳む。blocked 判定自体は「Ready 未到達」という観測事実のみ）。
  - blocked report 構築: `ScenarioReport::new(run_id, scenario,
    case, driver)` + `required`/`verdict=NotRun`/`detail`/
    `observed={"observed":"environment_blocked"}`/
    `environment={os,arch,scale_factor:null,repo_sha,
    environment_kind:"deterministic-backend",lane:"plugin-dock-lab"}`
    を bin 内で組み立てる（`finalize`/`case_environment` は
    `pub(crate)` で bin から見えないため、同形 JSON を bin 側で
    構成 — `real_dock::blocked_reports` と同じ正直形）。
    `run_id` は lab 専用 `"rf631-test-lab"`。
- **`ruviform-ui`/`ruviform-ui-gpui`**: 変更なし（consumer のみ）。
- **`ruviform-ui-test`**: 変更なし（projection API 拡張は Deferred）。

## Data and Control Flow

```
main()
 ├─ args parse (--out) / LaneEnvironment 構築 (repo_sha, PLUGIN_PATH_ENV)
 ├─ catalog_rows() = HEADLESS_MATRIX.project() + REAL_CATALOG → Vec<CatalogRow>
 ├─ driver = GpuiDriver::new()
 ├─ host = ViewHost::new(view, epochs.mint()); host.mount(lab_spec(&state))
 ├─ window = driver.create_view_window(&WindowSpec{title:"Ruviform Test Lab — plugin-dock", size 1100x720}, ViewMount{host,None})
 ├─ spawn app thread: driver.run(tick, on_event)   ── Ready → ready_tx
 ├─ main: ready_rx.recv_timeout(15s)
 │    ├─ Ready → join(app) → exit 0
 │    └─ timeout/err → write blocked JSONL → exit 0
 │
on_event (UI thread, per DriverEvent)
 ├─ Windowed{Ready} → ready_tx.send(Ok)（1 回のみ）
 ├─ Windowed{Destroyed} → drv.exit()
 ├─ Windowed{Action(intent)} → dispatch_command → state 変更 → update_view + post_event receipt
 ├─ Windowed{CloseRequested} → respond_close Approve（default に任せる）
 └─ Tick → worker_rx.try_recv → 結果あれば state 更新 + update_view + JSONL append
```

scenario worker thread（Run command ごとに spawn）:
`run_selected(&env.clone(), keep)` → `mpsc::Sender<Vec<ScenarioReport>>`
→ UI thread が受領して `render_jsonl(RUN_ID, "plugin-dock-lab",
repo_sha, &reports)` を `--out` に append（OpenOptions::append —
対話 session 内の複数 run を時系列で残す）。

## Non-Functional Constraints

- **RT contract**: non-RT only — 全て UI thread/worker thread の
  scenario 実行であり audio RT 経路に触れない。
- **verdict honesty**: catalog projection に verdict を出さない・
  headless 結果を real 証拠に昇格しない・real row は実行しない・
  environment_blocked は `NotRun` で fail-closed。
- **driver boundary**: GPUI 型は bin 内に閉じる。公開 API 変更なし、
  `boundary.json` 変更なし、`check-ui-driver-boundary` green が
  証拠。
- **opt-in**: `lab-gui` は非 default feature、bin は
  `required-features` 付き — default build・`cargo test` 既定経路・
  CI に GPUI backend を引き込まない。mise lane も manual/opt-in。
- **依存**: 新規 crates.io dependency なし（`gpui` は ui-gpui の
  既存 optional dep；serde_json/ruviform-ui-test 等は既存 dep）。
  feature 追加は test-only crate 内で `real-dock` 先例と同型 —
  PR に理由を記録する。

## Risks and Trade-offs

- **GPUI app thread の join 不能（blocked timeout 時）**: platform
  loop が生きたまま Ready が来ない場合、main 復帰で process 終了
  → thread は OS が回収。証拠（blocked JSONL）は main thread が
  書き終えてから終了するため欠損しない。`RealOsGuiTest` の
  join-on-every-path を模倣すると永続 session で timeout 時に
  hang するため、意図的に process-exit 路線を取る。
- **Ready が session-0 等で誤って発火する可能性**: driver の
  `Ready` 契約（visible+interactive）を信頼する — real lane の
  session-startup 判定と同じ信頼水準。実 Window での対話検証は
  manual lane の性質上、実走証拠は `evidence/rf631/` の実測のみ。
- **intent の serde 形状への結合**: `params.intent` は
  `rvf.ui.command.v1` の既存 wire 形（`render.rs` L749-754）。
  `CommandIntent` deserialize 失敗は rejected receipt + status
  表示で fail-closed。
- **大きな catalog 表**: v1 は 12 row の flat table で十分。
  pagination/仮想化は Deferred。

## Deferred Design Concerns

- richer catalog projection（filter/sort/group・per-driver live
  query）| v1 は `project()`/`cells()` の flat listing で要件充足 |
  listing 拡大時 | spec の Deferred Design Concerns と同一 |
- GUI → real lane 実行 launcher | 2 つ目 real executor 禁止 |
  real catalog 拡充 slice | lane 直接実行で代替可能 |
- 複数 window / window 管理 UI | v1 は単一 window | multi-window
  scenario slice | `MultiWindow` capability の case が出た時点 |
- spec の差分 update 最適化 | `update_view` は全 spec 差し替えで
  十分（12+ row 規模）| 性能問題が観測された時点 |

## Verification Notes

- [ ] `cargo check/test/lint -p ruviform-plugin-dock-test`（default）
  で headless 非回帰。
- [ ] `--features real-dock` で real lane 非回帰。
- [ ] `--features lab-gui` で bin が build され、bin 内 unit test
  （catalog row 写像・blocked report・spec 組立の pure 部分）が
  headless で通る。
- [ ] `mise run test-ui-plugin-dock-console` /
  `test-ui-plugin-dock-lab` / `test-ui-plugin-dock` /
  `test-ui-scenario-contract` / `check-ui-driver-boundary` /
  `prek run --all-files`。
- [ ] lab lane の interactive 実走は manual — headless host では
  `environment_blocked` → `NotRun` を実測記録する。
