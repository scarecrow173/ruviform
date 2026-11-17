---
id: "TSPEC-722vst3LifecycleAudit06"
type: "test-spec"
status: "approved"
title: "Issue #722 production VST3 adapter lifecycle 監査 — test specification"
created: "2026-11-19"
updated: "2026-11-19"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
    - "docs/designs/issue-722-vst3-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-722-vst3-lifecycle-audit.md"
  implements:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
    - "docs/designs/issue-722-vst3-lifecycle-audit.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #722 test specification

## Feature: VST3 lifecycle 契約（production adapter）

### Rule TS-722-1: terminate は非終端でなければならない

**Example**: `initialize` → `setActive(true)` → `setActive(false)` →
`terminate` → `initialize`（再）→ `setActive(true)` →
`setupProcessing` → `setProcessing(true)` → `setProcessing(false)` →
`setActive(false)` → `terminate` — 全て `kResultOk`。
同一 COM オブジェクトでの再利用が可能であること（SDK terminf
suite の `terminate` → `initialize` 再実行に対応）。

### Rule TS-722-2: setupProcessing の受理 state

**Example**:

- `Created`（initialize 前）で `setupProcessing` → `kResultFalse`
- `Initialized` で `setupProcessing` → `kResultOk`
- `setActive(true)` → `setProcessing(true)` で `Processing` 中に
  `setupProcessing` → `kResultFalse`
- `setProcessing(false)` で `Activated` に戻った後の
  `setupProcessing` → `kResultOk`（処理停止後の renegotiation）
- `terminate` 後の `Created` で `setupProcessing` → `kResultFalse`
- 再 `initialize` 後の `Initialized` で `setupProcessing` → `kResultOk`

### Rule TS-722-3: 既存の拒否契約を維持する

- `Activated` 中の `terminate` → `kResultFalse`
- 二重 `setActive(true)` → `kResultFalse`
- `setupProcessing` 成功前の `setProcessing(true)` → `kResultFalse`
- `kSample64` の `setupProcessing` → `kResultFalse`
- `maxSamplesPerBlock` が `MAX_NEGOTIATED_FRAMES` 超過 → `kResultFalse`
- `process` が `Processing` 以外 → `kResultFalse`
- `setActive(false)` が `Processing` 中 → `kResultFalse`

### Rule TS-722-4: ABI validator 実走

production `ruviform_plugin_vst3.dll` を `.vst3` bundle として stage
し、Steinberg VST3 SDK 3.8.1 validator を実走する。terminf /
suspend-resume 系テストを含む suite で失敗が再検出されないこと。
fixture 既定 lane の回帰も確認する。

### Rule TS-722-5: run.mjs 契約

`node --test ci/plugin-abi-validator/tests/` が pass し、既定
（引数なし相当）で fixture bundle `ruviform-plugin-test-vst3.vst3`
が stage される後方互換を維持する。`--vst3-dll` に production dll
を渡すと package/bundle 名が stem から導出される。

## 検証コマンド対応

| Rule | 検証 |
| --- | --- |
| TS-722-1/2/3 | `cargo test -p ruviform-plugin-vst3`（`mise run test ruviform-plugin-vst3`）の新規 2 テスト + 既存拒否テスト |
| TS-722-1 補強 | `mise run test-l1-real-plugin` の `l1_real_adapter_e2e`（host bridge 経由の実 cdylib 実走） |
| TS-722-4 | `node ci/plugin-abi-validator/run.mjs --vst3-dll target/debug/ruviform_plugin_vst3.dll --evidence-dir evidence/rf722/abi-validator` |
| TS-722-5 | `node --test ci/plugin-abi-validator/tests/` + fixture 既定 lane 実走 |
