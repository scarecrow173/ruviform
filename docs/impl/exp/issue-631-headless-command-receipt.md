# Issue #631 headless-command-receipt case — 実装実験記録（2026-11-18）

`docs/tasks/issue-631-headless-command-receipt.md` の実装で
試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-631-headless-command-receipt.md`](../ir/issue-631-headless-command-receipt.md)、
イベントログは
[`issue-631-headless-command-receipt-exp.jsonl`](issue-631-headless-command-receipt-exp.jsonl)
を参照。

## 試行 1: headless lane での fixture mount 形態

- **目的**: receipt-expecting intent を mint する control を
  Fake driver で駆動する。
- **観測**: `plugin_dock_view_spec`（`view-mount` が使う
  `NativeSlot` spec）は command binding を持たない。
  `sv_fader_dock_fixture` は fader `fader-ch1` を持つ
  command-bound spec で、`scenario_contract.rs` の
  receipt/projection 先例および real lane の
  `command_receipt_body` と同じ flow を Fake driver で
  再現できる。`FakeGuiTest` は case ごとに fresh 生成される
  ため、`ViewInstanceId` 1 の採番は `view-mount` case と
  衝突しない。
- **結論**: `command_receipt_case` 内で
  `mount_view(sv_fader_dock_fixture().spec)` する形態を採用
  （real lane の第 4 window D に相当するが、Fake では case
  内 mount で十分 — ADR 記録）。

## 試行 2: helper の共有可否

- **観測**: `step`/`steps_outcome`/`component_snapshot_json`/
  `commit_for_gesture`/`commit_control_id` は real_dock.rs に
  実装済みだが、同ファイル全体が `#[cfg(feature = "real-dock")]`
  配下 — default build の scenario.rs からは名前解決できない。
  共有化には real_dock.rs の編集（本 slice の lease 外）が
  必要。
- **結論**: scenario.rs に同型 helper を局所実装（各 ~10-45
  行、lane 非依存の責務）。将来 3 つ目の lane が同 helper を
  必要とした時点で cfg 非依存 module への集約を検討する残件
  として ADR に記録。

## 試行 3: clippy `-D warnings` 適合

- **観測**: 初回 lint で 2 件 — `ignored_unit_patterns`
  （`mount_view` の extra closure が `|_|` で unit 結果を
  受けていた → `|()|` に修正、real lane と同じ対処）と
  `too_many_lines`（`fixture_run_reports_honest_verdicts` が
  receipt assert 追加で 114/100 行超過 →
  `assert_command_receipt` helper に抽出。`#[expect]` ではなく
  test fn の分割を選択）。
- **結論**: `unused-qualifications`（新 import で
  `ruviform_ui::view::ComponentKey` の完全修飾が不要化）も
  import 使用形に修正。fmt-check・3 feature 系の
  check/lint/test 全て green。

## 試行 4: evidence 再生成と lease 境界

- **観測**: `test-ui-plugin-dock-real-headless`（必須検証
  コマンド）は exit 0 で pass したが、`--blocked-probe` が
  `plugin-dock-real-headless.jsonl` を再生成し、差分は
  `repo.sha`/`repo_sha` のみ（real catalog 無変更のため
  verdict・case_seed は同一）。同ファイルは本 slice の lease
  外。
- **結論**: 検証自体は実走済みとして記録し、生成物は
  `git checkout` で復元 — lease 外ファイルを変更しない。
  `plugin-dock-console-list*.jsonl` 2 件は対応 mise task が
  無いため `cargo run --locked -- --list [--include-real]`
  の補助コマンドで再生成（verification.md に補助証拠として
  明記）。

## 試行 5: receipts populate の実測

- **観測**: `plugin-dock-scenario.jsonl` の `command-receipt`
  行に `receipts` が非空で記録 — `{"command_id":
  "view-1:a1:m1:fader-ch1:3","status":"applied","revision":1}`
  と `{"command_id":"view-1:a1:m1:fader-ch1:6",
  "status":"rejected","detail":"consumer refused the commit"}`。
  headless lane で初めて `receipts` が非空になった。
- **結論**: AC1 link（minted command_id 相関 → delivered
  event decode）と AC4 evidence populate が Fake driver で
  実走確認。`environment_kind=deterministic-backend`・
  `driver=fake` のみを名乗り、real-window/pixel/OS-level の
  主張なし — honest contract evidence。
