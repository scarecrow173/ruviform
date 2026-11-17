---
id: "SPEC-722vst3LifecycleAudit02"
type: "spec"
status: "approved"
title: "Issue #722 production VST3 adapter lifecycle 契約監査と同型欠陥修正"
created: "2026-11-19"
updated: "2026-11-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/722"
    - "docs/discovery/issue-722-vst3-lifecycle-audit.md"
    - "docs/specs/issue-630-abi-validator.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-722-vst3-lifecycle-audit.md"
  implements:
    - "docs/discovery/issue-722-vst3-lifecycle-audit.md"
  implemented-by:
    - "docs/designs/issue-722-vst3-lifecycle-audit.md"
  depends-on:
    - "docs/specs/issue-630-abi-validator.md"
  blocks: []
---

# Issue #722 production VST3 adapter lifecycle 契約監査と同型欠陥修正 — spec

## 背景

Issue #630 の外部 ABI validator 実走で test fixture
`ruviform-plugin-test-vst3` の lifecycle 契約違反 3 件
（terminate 終端化・`setupProcessing` 受理 state 過狭・bus info
不一致）が検出され、fixture は修正済み。本 Issue は同じ監査を
production adapter `ruviform-plugin-vst3` に適用し、同型欠陥を
修正する。

## 要件

### R1: lifecycle 監査（AC1/AC4）

`ruviform-plugin-vst3` の lifecycle state machine を VST3 SDK 契約と
照合し、fixture で検出された 3 欠陥それぞれの有無を根拠行つきで
結論づける。結論は discovery 文書と `evidence/rf722/` に記録する。

### R2: terminate の非終端化

VST3 `terminate` は COM 破棄ではなく非終端でなければならない
（SDK terminf suite が同一オブジェクトで `initialize` を再実行する）。
法律的な `terminate`（`Deactivate` 済み、すなわち
`Initialized`/`Created`/`Failed` から）は、neutral `Destroy` を
バリデーションとして遷移した後、adapter 層で lifecycle を `Created`
にリセットし、negotiated `config` を破棄する。`Activated`/
`Processing` 中の `terminate` は従来どおり `kResultFalse` で拒否する。
core の neutral `Destroy` 終端契約は変更しない。

### R3: setupProcessing の受理 state 拡大

`IAudioProcessor::setupProcessing` は `Initialized` と `Activated`
の両方で受理する（SDK 文書 `[UI-thread & (Initialized | Connected)]`
および `SuspendResumeTest` の `setActive(true)` 前呼び出しに整合）。
`Created`（`initialize` 前）・`Processing`（処理中 renegotiation 不可）
・終端状態では `kResultFalse` で拒否する。`kSample64`・無効な
`maxSamplesPerBlock`（≤0 または `MAX_NEGOTIATED_FRAMES` 超過）・
無効 sample rate・`prepare` 失敗の拒否は維持する。

### R4: 単体テスト

fixture の修正テストと同型の往復テストを production adapter に追加する:

- `terminate` → 再 `initialize` → `setActive` → `setupProcessing` →
  `setProcessing` のフルサイクルが同一 COM オブジェクトで成功する
- `Initialized` 状態での `setupProcessing` 受理、`Processing` 中の
  拒否、`Activated` での再 `setupProcessing` 受理、`Created`/
  terminate 後の拒否、再 initialize 後の受理
- 既存の拒否テスト（activated 中 terminate・二重 activate・
  setup 未了での setProcessing・`kSample64`・過大 frame 数）を維持

### R5: ABI validator lane の production 対象化

`ci/plugin-abi-validator/run.mjs` を最小限汎用化し、
`--vst3-dll` で指定した production cdylib を `.vst3` bundle として
stage して VST3 SDK validator に掛けられるようにする。既定動作
（fixture bundle）は不変とし、fail-closed 契約を維持する。
production 実走証拠を `evidence/rf722/` に記録する。実走できない
場合は手順と理由を NotRun として正直に記録する。

### R6: 監査 evidence

`evidence/rf722/` に監査報告（3 欠陥の有無と根拠行）、validator
実走 or NotRun 証拠（SDK/validator version・コマンド・exit code・
出力）を記録する。

## 非目標

- `ruviform-plugin-core` の Lifecycle 機械の変更（`Destroy` 終端維持）
- `crates/ruviform-plugin-test-vst3/` への変更（fixture は修正済み）
- VST3 以外の plugin 形式（CLAP 等）の lifecycle 監査
- `host.rs` の機能変更（teardown 順序は新契約下でも有効）
- docs index（`docs/**/README.md`）の更新 — Coordinator が統合時に再生成

## 受入条件（Issue #722 転記）

- lifecycle state machine を VST3 SDK 契約と照合し 3 欠陥の有無を
  結論づける（evidence/報告に記録）
- 同型欠陥がある場合は修正し、ABI validator 実走で再検出しないことを
  確認する
- `mise run test` 系の正規検証が pass し、ABI validator lane の実走
  証拠を記録する
- 「同型欠陥なし」の結論の場合も根拠を記録する
