---
id: "IMPL-722vst3LifecycleAudit07"
type: "impl"
status: "completed"
title: "Issue #722 production VST3 adapter lifecycle 監査 — 実装記録"
created: "2026-11-19"
updated: "2026-11-19"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-722-vst3-lifecycle-audit.md"
    - "docs/plans/issue-722-vst3-lifecycle-audit.md"
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
    - "docs/designs/issue-722-vst3-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/component.rs"
      - type: "file"
        path: "ci/plugin-abi-validator/run.mjs"
      - type: "file"
        path: "evidence/rf722/lifecycle-audit.md"
      - type: "file"
        path: "evidence/rf722/abi-validator/run.json"
      - type: "file"
        path: "evidence/rf722/abi-validator/vst3-validator.txt"
      - type: "file"
        path: "evidence/rf722/abi-validator-fixture/run.json"
      - type: "file"
        path: "docs/impl/ir/issue-722-vst3-lifecycle-audit.md"
    modified: []
---

# Issue #722 実装記録

## 実施内容

### `crates/ruviform-plugin-vst3/src/component.rs`

1. **`terminate` 非終端化**（IPluginBase, :513-525）:
   `LifecycleEvent::Destroy` をバリデーションとして遷移し、成功時に
   `state.lifecycle = Lifecycle::new()`（`Created`）と
   `state.config = None` を実行して `kResultOk` を返す。
   fixture `ruviform-plugin-test-vst3` と同一形。`Activated`/
   `Processing` からの terminate は従来どおり `kResultFalse`。
2. **`setupProcessing` 受理 state 拡大**（IAudioProcessor, :721-736）:
   `Activated` のみ → `Initialized | Activated`（SDK 文書
   `[UI-thread & (Initialized | Connected)]`・`SuspendResumeTest`
   の `setActive(true)` 前呼び出しに整合）。`Created`/`Processing`/
   終端は `kResultFalse` を維持。sample size・frame 数・sample rate・
   `prepare` 失敗の拒否と scratch/`ProcessConfig` 保存は不変。
3. **doc 同期**: 冒頭 lifecycle mapping table（:10-17）と両関数の
   doc comment を新契約に更新。
4. **unit test 追加**（`#[cfg(test)] mod tests`）:
   - `terminate_allows_reinitialize`: initialize→activate→deactivate→
     terminate→再 initialize→activate→setupProcessing→
     setProcessing→stop→deactivate→terminate のフル往復
   - `setup_processing_accepted_while_initialized`: Created 拒否・
     Initialized 受理・Processing 拒否・Activated 再受理・
     terminate 後拒否・再 initialize 後受理

### `ci/plugin-abi-validator/run.mjs`

- `--vst3-package <name>` オプション追加（CLI のみ、env なし）
- `vst3NamesFromDll(dllPath)` を新規 export: dll stem の `_`→`-`
  で package 名導出、bundle 名 = `package + ".vst3"`
- `stageVst3Bundle(dllPath, stageRoot, bundleName = "ruviform-plugin-test-vst3.vst3")`:
  bundle 名引数追加（既定値で後方互換）
- `normalizeVst3Leg`/`runVst3Leg` に `name` 引数追加
- `buildFixtureArtifacts(opts, vst3Package)`: build 対象を
  `ruviform-plugin-test-clap` + 導出 package に（fixture 既定時は
  従来と同一対象）
- run report の `artifacts` に `vst3Package`/`vst3Bundle` を記録、
  leg の plugin ラベルを bundle 名に
- fail-closed 契約・NotRun 評価は不変。USAGE/ヘッダコメント更新

### `crates/ruviform-plugin-vst3/src/controller.rs` — 変更なし

監査の結果、`terminate`（:72-74）が無条件 `kResultOk` で状態を持たない
trivial 実装であり、VST3 `IEditController::terminate` の非終端性に対し
reset 対象がないため契約を満たすと判断。変更不要。

### `crates/ruviform-plugin-vst3/src/host.rs` — 変更なし

teardown 順序 `setProcessing(false)` → `setActive(false)` →
`terminate` は新契約下でも有効。`config = None` により terminate 後の
processing 再開経路はない。編集範囲外のため doc comment の陳腐化は
未修正（挙動影響なし）。

## TDD 記録

1. **Red**: 新規 2 テストを追加後 `mise run test
   ruviform-plugin-vst3` → exit 101。`terminate_allows_reinitialize`
   が 2 回目 `initialize` で `kResultFalse`、
   `setup_processing_accepted_while_initialized` が `Initialized` での
   `setupProcessing` に `kResultFalse` — 両欠陥を契約レベルで検出。
2. **Green**: 実装修正後 `mise run test ruviform-plugin-vst3` →
   44 tests pass・exit 0。

## 検証結果（全て実測）

環境: Windows (win32/x64)・rustc 1.99.1 (58cf8f9c3 2026-10-13)・
Node v26.12.1・VST3 SDK 3.8.1 validator・clap-validator 0.4.1。
ベース: `7124f0b24915ce18a139d71267582c54da7d903a`。

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | pass |
| `mise run check ruviform-plugin-vst3` | 0 | pass |
| `mise run lint ruviform-plugin-vst3` | 0 | clippy `-D warnings` pass |
| `mise run test ruviform-plugin-vst3` | 0 | 44 tests pass |
| `mise run test-l1-real-plugin` | 0 | pass（下記 flaky 注記） |
| `node --test ci/plugin-abi-validator/tests/` | 0 | 18 tests pass |
| `mise run test-plugin-abi-validator`（引数なし） | 1 | verdict NotRun — validator パス未指定の fail-closed 設計どおり。実走は明示パス指定で実施 |
| `node run.mjs --clap-validator … --vst3-validator …`（fixture 既定） | 0 | verdict Passed — CLAP 5 legs + VST3 fixture 235/235。`evidence/rf722/abi-validator-fixture/` |
| `node run.mjs … --vst3-dll target/debug/ruviform_plugin_vst3.dll` | 0 | verdict Passed — **production bundle `ruviform-plugin-vst3.vst3`: 47 passed, 0 failed**。`[Terminate/Initialize]`/`[Suspend/Resume]`/`[Valid State Transition]`/`[Bus Consistency]` 等全 suite `[Succeeded]`。`evidence/rf722/abi-validator/` |

### test-l1-real-plugin flaky 注記

1 回目の `mise run test-l1-real-plugin` は exit 101 — 失敗は
`crates/ruviform-plugin-test-clap` の
`l1_real_test_plugin_e2e::real_test_plugin_l1_instrument_note_off_releases_to_silence`
（CLAP fixture の note-off release tail が exact silence でない、
DSP レンダリング系）。本変更（`ruviform-plugin-vst3` + run.mjs）とは
無関係の crate・経路。単体再実行で pass、レーン全体の再実行でも
pass（exit 0）を確認 — flaky と判断。production adapter の
`l1_real_adapter_e2e`（clap/vst3 両方・計 6 tests）は両回とも pass。

## review-protocol 結果

実装 diff・doc chain・evidence を対象にレビュー実施。blocker なし
（詳細は PR 本文）。

## 残課題・範囲外メモ

- `host.rs` の doc comment が fixture 旧契約（setupProcessing は
  Activated 必須）を参照する記述のまま — 挙動影響なし・編集範囲外の
  ため未修正。別 Issue 候補。
- `evidence/rf630/abi-validator-verification.md` の
  「controller.rs:72 の terminate が `Destroy` を遷移」は現行コードと
  食い違い（実測: 無条件 `kResultOk`）— rf630 証拠ファイルは本 slice
  の編集範囲外のため修正せず、`evidence/rf722/lifecycle-audit.md` に
  実測を記録。
- production `ruviform-plugin-clap` adapter の同型 lifecycle 監査は
  本 Issue の対象外（Coordinator へ別 Issue 候補として報告済み）。
- docs index（`docs/**/README.md`）は未同期 — Coordinator が統合時に
  再生成。
