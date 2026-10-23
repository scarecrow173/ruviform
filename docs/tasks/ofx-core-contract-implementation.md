---
id: "TASK-034L6ycSsF8VW32LLoNxXr"
type: "task"
status: "done"
title: "Implement video plugin core contract"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
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
    - "docs/plans/implement-issue-98-ofx-host-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-98-ofx-host-adapter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/video-plugin-plain-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Core contract implementation

## Work

approved plan `implement-issue-98-ofx-host-adapter.md` の **actions 1/2/3/4 を 1 task の ordered substeps として実行**（14 action 群のまま・1 commit/action ではなく buildable unit 単位）。

### A1 — contract tests + 環境（既存 core pkg reuse）

- 既存 `ruviform-plugin-core` manifest/lib を reuse（新規 target・公開 API scaffold 置き換えなし）
- `mise.toml` に plan 記載の確定 TOML どおり `tasks.test-plugin-core`・`tasks.lint-plugin-core` を追加
- `tests/video_contract.rs` + doctests: `PluginDescriptorView`/`view()` pure borrow（compile-fail owner-lifetime は real borrow error）・`PlainParamValue::Double(1)` vs `0` stable Copy・`Double(NaN)` 構築可・`VideoProcessConfig` 非ゼロ dims/Copy getters・generic consumer が OFX 型なしで trait 実装コンパイル
- 構造的 presence は `rg -q`（`PluginDescriptorView`/`PlainParamValue`/`VideoProcessConfig`/`VideoPluginInstance`・2 mise task 見出し）で確認 — コンパイル証明ではない

### A2 — red 記録

- `mise run test-plugin-core` 実行・失敗原因を記録。**新公開 API 不在の `E0432` 等は正当な red**（テストは A1 で存在済み — 欠落は契約 API であり test file/wrong package/command/environment の欠落ではない）。既 green なら既存カバレッジ記録・人工欠陥を作らない（A3 スキップ根拠になる）

### A3 — implement

- `src/descriptor.rs`（`PluginDescriptorView<'a>`+`view()` pure borrow・`Box::leak` なし）・`src/plain_param.rs`（`PlainParamValue`/`PlainParamInfo` design 正確）・`src/video.rs`（`VideoProcessConfig`/`VideoPluginInstance` 全署名・全操作 NONRT）・`src/lib.rs` exports・`Cargo.toml`（plugin-core→video-core edge）・crate README・`crates/README.md`・baseline DAG/ledger は edge 実追加時のみ
- 旧 `param.rs`・audio event・wire・normalizer（C1 含む）は不変

### A4 — green + lint

- `mise run test-plugin-core` exit 0・`mise run lint-plugin-core` exit 0（実 log 保存）

## Completion Gate

targeted proof（上記実 log）+ lead review + **意味のある core commit** の後にのみこの task は完了（implementation-flow の commit gate — 1 substep ごとの commit ではなく、この task が一つの buildable unit）。IR は最初のコード前に起票。

## Verification

- [x] `mise run test-plugin-core` — exit 0
- [x] `mise run lint-plugin-core` — exit 0
- **evidence**: `t1-final-green.log`（32 unit + 8 integration + 21 doctest + 2 compile_fail・exit 0）・`t1-final-lint.log`（exit 0）・`probe-view.log`/`probe-trait.log`（E0515 実 borrow error・exit 1）。doc compile-fail は green 後に real borrow error を示す（unresolved symbol ではない）
