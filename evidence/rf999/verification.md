# evidence/rf999 — ComponentSpec 任意値 text/number 入力 widget の検証記録

Issue #999 の実装記録。`crates/ruviform-ui` に `ComponentSpec::Input`（`"type": "input"`）・`InputValueSpec`（`{"kind":"text"|"integer"|"number","min"?,"max"?}`）・`ViewHost::submit_input`・`A11yRole::TextBox`/`SpinButton` を追加し、入力値を `CommandParams` 型付き profile で `Action` intent の `params` に載せる headless wire 契約を実装した。wire 契約正本は `docs/specs/rf999-input-widget.md`、設計判断は `docs/designs/rf999-input-widget.md`。

- OS: Windows（`G:\orca\workspaces\ruviform\rf999-input-widget` worktree）
- 対象 SHA（実走時点）: `feat/999-input-widget` @ `origin/main` = `6f2a3882`
- 方針: `POLICIES.md` §5 — 変更範囲に比例した lane。`*-full`・Feature Matrix・MSRV・3OS 横断は Daily/手動 Full CI へ委譲。

## 1. 検証 lane 実走記録

| 検証 | コマンド | OS | exit | 結果 |
| --- | --- | --- | --- | --- |
| フォーマット | `mise run fmt-check` | Windows | 0 | **Pass** |
| 型チェック（変更本体） | `mise run check ruviform-ui` | Windows | 0 | **Pass** |
| Lint（`-D warnings`） | `mise run lint ruviform-ui` | Windows | 0 | **Pass** |
| テスト（変更本体・新規 headless test 含む） | `mise run test ruviform-ui` | Windows | 0 | **Pass** — lib 105 tests 全 pass（新規 `view::tests::input_*` 6 件含む） |
| 直接下流の型チェック | `mise run check ruviform-ui-test` | Windows | 0 | **Pass** |
| GPUI adapter 非回帰（feature lane） | `mise run check-ui-gpui` / `mise run lint-ui-gpui` | Windows | 0 | **Pass** — `render.rs` の `other =>` arm で `"input"` は "unsupported component" 表示へ正直に降格（adapter render は別 slice） |
| webview adapter 非回帰（feature lane） | `mise run check-ui-webview` | Windows | 0 | **Pass** |
| 残る直接下流 | `mise run check ruviform-plugin-dock-test` / `mise run check ruviform-poc-native-dock` | Windows | 0 | **Pass** |
| 下流 contract test 非回帰（parity fixture・`ui_command_params` 等） | `mise run test ruviform-ui-test` | Windows | 0 | **Pass** — 全 test binary pass（補助証拠: lane 指定外の任意追加 lane） |
| コミットフック | `mise exec -- prek run --all-files` | Windows | 0 | **Pass** — 全 hook |

注記: `check-ui-gpui`/`lint-ui-gpui` 実行時に `proc-macro-error2 v2.0.1` の future-incompat warning が出るが、main でも出る既存の依存側警告であり本変更とは無関係（エラーではなく lane は Pass）。

## 2. 受入条件 ↔ 検証の対応表

| Issue #999 受入 | 検証 | 結果 |
| --- | --- | --- |
| AC1: `ComponentSpec` に任意値 text/number 入力 widget が追加され、wire 経由で typed な入力値を intent として送出できる | `view::tests::input_submit_emits_spec_target_action_and_typed_params` — mount → `submit_input("480")` → `drain_events` で `CommandIntent::Action{target:"syn.project", action:"ppqn", params:{"new_ppqn":480}}`（`Integer` 型付き・`expect_receipt`）を観測。`input_submit_number_and_text_kinds_emit_their_typed_params` で `Number(0.5)`・`Text` verbatim、`input_spec_serde_round_trip` で `"type":"input"`/`"value":{"kind":...}` の wire serde round-trip を確認 | **Pass** |
| AC2: 範囲制約（min/max）・不正入力の拒否が wire 契約で表現され、headless test で input widget → intent mint → op 到達・範囲外拒否が確認できる | intent mint→到達: 同上（`drain_events` 経由で consumer へ出る = 「op 到達」、op mapping は consumer 責務）。receipt round-trip: `apply_receipt` で `Pending→Accepted→Applied` phase 遷移を確認。範囲外拒否: `input_submit_rejects_out_of_range_and_unparseable_without_emitting` — `"0"`/`"32768"`（範囲外）・`"abc"`/`"480.5"`/`""`（parse 不能）・`"NaN"`/`"inf"`/`"1e400"`（非有限）・wrong kind/unknown key/stale generation が全て `InvalidInput`/`StaleGeneration`/`UnknownComponent` で intent 非発行。mount 時拒否: `input_specs_require_target_action_param_and_ordered_bounds` — 空 `target`/`action`/`param`・`min>max`・NaN bound が `InvalidSpec`。routing drop: `removed_input_drops_its_receipt_routing` | **Pass** |
| AC3: `mise run` の対象 lane が pass する | §1 の lane 表 | **Pass** |

## 3. 実施していない検証（NotRun と理由）

| 検証 | 状態 | 理由・後続 |
| --- | --- | --- |
| 実 GUI 受入（実 OS window・実描画・IME 実入力） | **NotRun** | 本 slice 対象外 — 別 gate（synvelis#114 / ruviform#631）。adapter の `Input` render 実装自体が未実装（別 slice） |
| GPUI/webview adapter での `Input` 描画・submit 配線・driver-lab parity fixture | **NotRun** | 別 slice の責務。新 `type` は下流 wildcard arm で "unsupported component" 表示に落ちることを本 slice の仕様として確認（`check-ui-gpui`/`lint-ui-gpui` が compile 非回帰を証明） |
| `check-full`/`lint-full`/`test-full`/`build-full`/`verify-full`・Feature Matrix・MSRV・3OS 横断 | **NotRun** | `POLICIES.md` §5 — 変更範囲比例の原則により Daily Full CI・手動 Full CI に委譲 |
| synvelis 側 `syn.project.ppqn` op での受入 | **NotRun** | consumer 責務（#944 D7 の分界 — params admission は consumer）。本 slice は intent に typed params が canonical JSON で載るところまで |
| docs index（`docs/**/README.md`）同期 | **NotRun** | 共通契約により Worker は docs index を編集・再生成しない — Coordinator が統合時に整備 |
