---
id: "IMPL-034YaYGQObFpsF3n5KIqi3"
type: "impl"
status: "in-progress"
title: "Issue #631 GPUI Test Lab / Scenario Console GUI v1 実装記録"
created: "2026-11-15"
updated: "2026-11-16"
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
    - "docs/tasks/issue-631-test-lab.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-631-test-lab.md"
    - "docs/designs/issue-631-test-lab.md"
    - "docs/plans/issue-631-test-lab.md"
    - "docs/test-specs/issue-631-test-lab.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-631-test-lab.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-631-test-lab.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-631-gpui-test-lab-scenario-console-gui-v1.jsonl"
    rejected: []
---

# Issue #631 GPUI Test Lab / Scenario Console GUI v1 実装記録

## Summary

`crates/ruviform-plugin-dock-test` に feature `lab-gui` 限定の
`plugin-dock-lab` bin を追加し、HEADLESS_MATRIX + real_catalog の
統合カタログを実 GPUI window 内の Scenario Console として提示する
Test Lab GUI v1 を実装した。headless cell は window 内 worker
thread から `run_selected` で実行し、real-window row は実行せず
`test-ui-plugin-dock-real` へ委譲する fail-closed 表示に留める。
canonical headless console lane (`test-ui-plugin-dock-console`) と
opt-in windowed lane (`test-ui-plugin-dock-lab`) を `mise.toml` に
追加し、`test-ui-plugin-dock-real` の stale な description
（test_gain 前提・editor attach unsupported 前提）を #670 後の
実態に合わせて修正した。

## Context

docs/tasks/issue-631-test-lab.md の task 群を実装。上流は
docs/specs/issue-631-test-lab.md（AC）・docs/designs/issue-631-test-lab.md
（配置・投影・fail-closed 契約）・docs/plans/issue-631-test-lab.md
（file map・順序）・docs/test-specs/issue-631-test-lab.md
（検証規則）。driver-neutral scenario/catalog contract（#464）と
`ruviform-ui` ViewSpec 語彙・`GpuiDriver`（#386/#387 系）を再利用し、
case 定義の複写は行わない。

## Implementation

### Changes

- `crates/ruviform-plugin-dock-test/Cargo.toml`: feature `lab-gui`
  （default 無効）を追加 — `lab-gui = ["dep:ruviform-ui-gpui",
  "ruviform-ui-gpui/gpui"]`。`real-scenario`/`uitest` の
  scenario/test adapter 面は引かず、plain `gpui` feature の
  `GpuiDriver`/`ViewMount` のみを有効化する（lab は
  `GpuiDriver::run` の event callback を直接駆動するため
  job channel 不要）。`[[bin]] name = "plugin-dock-lab"` を
  `required-features = ["lab-gui"]` で登録。新規外部依存なし —
  feature 追加理由は Cargo.toml コメントに記録。
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-lab.rs`
  （新規・約 1170 行）:
  - `catalog_rows()`: `HEADLESS_MATRIX.project()` の `EntryProjection`
    と `real_catalog::REAL_CATALOG` から `CatalogRow` を構成。
    case/scenario/driver/required caps/runnable/missing/lane/
    real 判定を保持し、case 定義は複写しない。
  - `lab_spec()`: `ruviform-ui` の `ViewSpec`/`PanelSpec`/
    `LabelSpec`/`TableSpec`/`ButtonSpec` のみで UI を構成
    （catalog table は selectable、results table は参照専用、
    actions panel に Run selected / Run all headless / Quit）。
  - `CommandPlan` dispatch: table selection
    （`TABLE_SELECT_ACTION` + `{"row": n}`）→ `Select`、
    button action → `RunCase`/`RunAll`/`Quit`/`Inform`/`Reject`。
    real row の `RunCase` は `Inform`（`mise run
    test-ui-plugin-dock-real` への委譲文言）で実行しない。
    row index は `usize::try_from` + 範囲検査で fail-closed。
  - worker thread + `std::sync::mpsc`: scenario 実行を UI loop の
    外へ逃がし、完了時に `update_view` で results table を更新。
    run gate で二重起動を拒否。intent dispatch は receipt
    （applied/rejected）を発行。
  - `blocked_reports()`: window が `WindowLifecycle::Ready` に
    到達しない環境では全 catalog row を `NotRun` +
    `observed="environment_blocked"` で記録（run id
    `rf631-test-lab` で headless lane の `rf631-plugin-dock` と
    混同しない）。対話 session での driver-loop error は fail。
    引数誤用は exit 2、出力書込失敗は fail。
  - JSONL: `--out <path>` へ append、未指定は stdout。
    `lane="plugin-dock-lab"`、実行 case は catalog 本来の
    run id を保持。
  - `#[cfg(test)]` unit test 6 件: 引数 parse・catalog 網羅
    （headless 8 + real 4）・command dispatch（select/run/reject/
    real 委譲）・blocked report 生成・verdict 語彙・ViewSpec 構造。
- `mise.toml`:
  - `[tasks.test-ui-plugin-dock-console]` 新設 — canonical
    headless console lane。`--list`（capability projection、
    evidence ではない）+ `--run --out
    evidence/rf631/plugin-dock-console.jsonl`。
  - `[tasks.test-ui-plugin-dock-real]` description 修正 —
    `test_gain` 前提・editor attach unsupported 前提の stale
    文言を #670 後の実態（plugin-test-vst3 / test_editor 実
    IPlugView / child HWND OS 観測）へ更新。run block は不変。
  - `[tasks.test-ui-plugin-dock-lab]` 新設 — opt-in windowed
    lane。`cargo build -p ruviform-plugin-test-vst3` の後
    `cargo run --features lab-gui --bin plugin-dock-lab --
    --out evidence/rf631/plugin-dock-lab.jsonl`。対話 session
    前提・CI-required gate ではない旨を明記。
- `docs/development-environment.md`: 検証入口表に plugin-dock
  lane family（`test-ui-plugin-dock` / `-console` / `-real` /
  `-lab`）の行を追加。
- `evidence/rf631/plugin-dock-console.jsonl` /
  `plugin-dock-scenario.jsonl`: console lane / headless lane の
  再実行で更新（verdict 不変）。
- `evidence/rf631/plugin-dock-lab.jsonl`: lab GUI 実走の JSONL
  evidence（後述）。

### Decisions During Implementation

- **配置**: driver boundary guard が `ruviform-plugin-dock-test`
  を composition-root として扱うことを確認し、lab bin を同 crate
  に置いた。production crate への依存追加・boundary.json 変更は
  不要と判断。
- **catalog 投影**: `HEADLESS_MATRIX.project()` と
  `real_catalog::REAL_CATALOG` をそのまま投影し、GUI 側に case
  定義を持たない（spec の non-goal を遵守）。real row は
  `required` capability と delegation 文言を表示し、selectable
  table には載せるが `RunCase` は `Inform` に落とす。
- **run id 分離**: lab 産の blocked record は `rf631-test-lab`、
  実行 case report は catalog の `rf631-plugin-dock` を維持 —
  lane label `plugin-dock-lab` で JSONL 上も区別可能。
- **window サイズ**: 初回 dogfooding で run-all 完了後に results
  table が伸び actions panel が window 下端で clip されることを
  観測し、`logical_size` を (1100, 720) → (1100, 860) に修正
  （resizable=true は維持）。
- **PostMessage dogfooding**: GPUI content は UIA tree に出ない
  ため、実 window に対し Win32 `PostMessage(WM_LBUTTON*)` で
  client 座標 click を送り、`component_bounds` 相当の実測位置
  （screenshot 画素解析でボタン帯を同定）を使って select /
  run-selected / run-all / real-row reject / quit を実走確認した。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-631-gpui-test-lab-scenario-console-gui-v1.jsonl`
  — 実装中の観測記録（compile/clippy 反復、window clip 発見と
  修正、GUI 実走 dogfooding）を採用。設計判断の根拠として参照。

### Rejected

- なし（GUI automation を `orca computer` UIA 経路から Win32
  PostMessage 経路へ切替えたのは手法の変更であり、実験 log の
  採否対象となる代替実装ではない）。

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo check --locked --all-targets -p ruviform-plugin-dock-test --features lab-gui`: pass
- `cargo clippy --locked -p ruviform-plugin-dock-test --features lab-gui --all-targets --no-deps -- -D warnings`: pass（本 crate clean）
- `mise run lint ruviform-plugin-dock-test --features lab-gui`（canonical・deps 込み）: **fail (exit 101)** — path dep `ruviform-audio-test` の既存 `too_many_lines`（`stream.rs:885` `pump_input` 105/100）。同 crate 単独 clippy でも再現・`git diff HEAD` 無変更の upstream drift で本 slice 無関係。scope 外のため未対処、PR で報告
- `cargo test --locked -p ruviform-plugin-dock-test`（default）: pass
- `cargo test --locked -p ruviform-plugin-dock-test --features lab-gui`: pass（lab unit 7 + console 14 + plugin-dock 4）
- `cargo test --locked -p ruviform-plugin-dock-test --features real-dock`: pass
- `cargo check --locked --all-targets -p ruviform-plugin-dock-test --features real-dock`: pass
- `mise run test-ui-plugin-dock-console`: pass（exit 0）
- `mise run test-ui-plugin-dock-lab`: pass（exit 0 — window 起動 →
  Quit で clean exit を lane 経由で確認）
- `mise run check-ui-driver-boundary`: pass（dep-graph/signature/
  allowlist — `lab-gui` 追加後も境界違反なし）
- `mise run test-ui-plugin-dock`: verdict 不変（環境 field のみ差分）
- review-protocol self-review: CONFIRMED P1 1 件を修正 —
  `emit_blocked` が `fs::write` で `--out` を truncate し先行
  session の蓄積 evidence を失い得た → `append_jsonl` と同じ
  append mode に修正し `emit_blocked_appends_instead_of_
  truncating` 回帰 test を追加。P2/P3 は non-blocking として
  記録（下記 Risks）。
- GUI dogfooding（interactive Windows session、PostMessage click）:
  catalog 12 row 描画・row select・run-selected（`artifact` →
  pass）・run-all headless（8 case、verdict pass/unsupported/
  not_run を正直に記録）・real row 選択時の委譲（JSONL 非増加・
  非実行）・Quit による clean exit を確認。証拠は
  `evidence/rf631/plugin-dock-lab.jsonl` と
  `evidence/rf631/verification.md` の本 slice 節。
- workspace 全体・canonical `mise run lint` の clippy は
  無関係な既存 file（`crates/ruviform-audio-test/src/backend/
  stream.rs` の `too_many_lines`）で失敗 — path dep 経由で
  `plugin-dock-test` の canonical lint も連鎖 fail する。
  本 slice の scope 外・未変更。upstream での対処が必要
  （#671 系で混入した可能性）。

## Risks

- lab GUI は v1 で最小限の操作面のみ（filter/sort なし、
  results は直近 run のみ表示）。大量結果時の scroll/仮想化は
  未対応 — window は resizable で当面凌げる。
- PostMessage dogfooding は OS 依存の手動経路であり、CI での
  再現 lane ではない（opt-in manual lane の位置付けと一致）。
- `plugin-dock-lab` の `--out` 既定は stdout のため、lane 経由
  以外で起動すると evidence が console に流れる（仕様どおり）。
- review-protocol non-blocking notes: `finish_session` の
  `Ok(Err)` ready 分岐は現状到達不能（`ready_tx` に Err を送る
  経路がない）で setup 失敗時の error 詳細が stderr に出ない
  — evidence は blocked record で正直だが診断性は限定的。
  status 文言の "N case line(s)" は run header を含まず実際は
  N+1 行。`--out --flag` は flag を path として消費する
  （lane bin と同じ parse 慣行）。

## Follow-ups

- 実 window lane の定期的な手動 dogfooding 運用（本 slice の
  JSONL evidence を baseline とする）。
- results 表示の scroll/集約が必要になった時点で view 語彙側の
  拡張を検討（本 slice では要求しない）。
