---
id: "TASK-jf1GeH9DzuEjHri0o8EqrT"
type: "task"
status: "done"
title: "Issue #692 s2 Luau analyzer backend — タスク"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
    - "docs/plans/issue-692-luau-backend.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-692-luau-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-692-luau-backend.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-692-luau-backend.md"
    - "docs/specs/issue-692-luau-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-692-luau-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-692-luau-backend.md"
  verified-by: []
  references:
    - "docs/plans/issue-692-luau-backend.md"
  defers: []
  deferred-by: []
---

# Issue #692 s2 Luau analyzer backend — タスク

- [x] `crates/ruviform-script-luau` 骨格作成（Cargo.toml/build.rs/
      native/vendor コピー/src 骨格/README）
- [x] `src/contract.rs` + `src/ffi.rs` 移植（shim status/JSON 定数、
      extern "C"、BridgeHandle/ShimString/CreateOptions、versions）
- [x] `src/analyzer.rs` 実装（LuauAnalyzerOptions/LuauAnalyzer/
      ScriptAnalyzer impl/JSON→中立型写像/position_offset/status→
      AnalyzerError 写像）
- [x] `native/rf_luau_bridge.{h,cpp}` 製品化（env 名改名・
      impl_version 更新・ABI 不変）
- [x] `build.rs` 製品化（vendor コンパイル対象・lib 名・rerun-if）
- [x] `src/ffi_boundary_tests.rs` 移植（invalid UTF-8/範囲外/
      oversize/completion disabled/version negotiation）
- [x] `tests/bridge.rs` 新規（製品契約統合テスト一式）
- [x] `tests/faults.rs` 新規（RF_LUAU_FAIL_* seam）
- [x] `tests/registry.rs` 新規（AnalyzerRegistry end-to-end +
      stub による fail-closed 行列）
- [x] workspace 配線（root Cargo.toml members・crates/README.md・
      Cargo.lock 更新）
- [x] `docs/adr/issue-692-luau-dependency-selection.md` 作成
- [x] `mise run fmt-check/check/lint/test ruviform-script-luau` +
      `prek run --all-files` 検証
- [x] `docs/impl/ir` + `docs/impl/exp` + `evidence/rf692` s2 記録
- [x] review-protocol 1 サイクル以上・blocker 0
