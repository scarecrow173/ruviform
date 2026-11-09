---
id: "TASK-034Uokf4hj4eTVzf7L9rQd"
type: "task"
status: "completed"
title: "Issue #418 — theme-codegen 実装"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/418"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-theme/src/bin/theme_codegen.rs"
      - type: "file"
        path: "crates/ruviform-ui-theme/codegen/themes.json"
      - type: "file"
        path: "crates/ruviform-ui-theme/src/generated.rs"
      - type: "file"
        path: "crates/ruviform-ui-theme/tests/codegen.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-theme/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-theme/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-theme/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-418-theme-codegen.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-418-theme-codegen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-418-theme-codegen.md"
    - "docs/designs/issue-418-theme-codegen.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-418-theme-codegen.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #418 — theme-codegen 実装

## Work

`ruviform-ui-theme` に manifest 駆動の codegen を実装する: `src/bin/theme_codegen.rs`（CLI + strict manifest parse + `ThemeResolver::resolve` + emit）、`codegen/themes.json`（synvelis-dark / audiolens-dark fixture 入力）、`src/generated.rs`（commit する生成モジュール）、`tests/codegen.rs`（byte-identical 決定性・committed 鮮度・実行時 resolve 一致）、`Cargo.toml` の `[[bin]]`、`lib.rs` の `pub mod generated;`、`mise.toml` の `codegen-theme` / `codegen-theme-check`、README・development-environment.md の文書化。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-418-theme-codegen.md`
- `docs/specs/issue-418-theme-codegen.md`
- `docs/designs/issue-418-theme-codegen.md`
- `docs/test-specs/issue-418-theme-codegen.md`

## Dependencies

- `docs/specs/issue-385-dtcg-resolved-theme.md` の `ThemeResolver`/`ResolvedTheme` 公開 API（main 統合済み・前提）

## Affected Files

- `crates/ruviform-ui-theme/`（bin・codegen manifest・generated module・tests・Cargo.toml・lib.rs・README）
- `mise.toml`（codegen タスク追加のみ）
- `docs/development-environment.md`（タスク対応表）

## Done When

- [x] `mise run codegen-theme` で `src/generated.rs` が生成・更新される
- [x] `mise run codegen-theme-check` が生成物の鮮度を byte 比較で検査する
- [x] `cargo test -p ruviform-ui-theme` が決定性・鮮度・実行時一致テストを含めて green
- [x] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` が green
- [x] 実行時 resolve との使い分けが README/design doc に記述される

## Verification

- [x] `mise run codegen-theme` → `mise run codegen-theme-check`（一致）
- [x] `cargo test -p ruviform-ui-theme --test codegen`（補助証拠・canonical は `mise run test`）
- [x] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test`
