---
id: "PLAN-034Uoj9ui4gTFQIKFt9GqA"
type: "plan"
status: "approved"
title: "Issue #418 — theme-codegen 実装計画"
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
    - "docs/specs/issue-418-theme-codegen.md"
    - "docs/designs/issue-418-theme-codegen.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-418-theme-codegen.md"
    - "docs/designs/issue-418-theme-codegen.md"
  derived-by:
    - "docs/tasks/issue-418-theme-codegen.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-418-theme-codegen.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #418 — theme-codegen 実装計画 Implementation Plan

## Goal

`ruviform-ui-theme` に manifest 駆動の DTCG→静的 Rust 型 codegen（`theme-codegen` bin + commit 済み生成モジュール + mise タスク + 決定性テスト）を実装する。

## Scope

単一実装ストリーム。`crates/ruviform-ui-theme` 内に完結し、他 crate のソース・workspace members・公開 API 契約には触れない。`mise.toml` へのタスク追加と `docs/` チェーンは Worker Spec が許可する共有ファイル変更。

## Upstream Documents

- `docs/specs/issue-418-theme-codegen.md`
- `https://github.com/scarecrow173/ruviform/issues/418`

## Design Inputs

- `docs/designs/issue-418-theme-codegen.md`（approved）

## File Map

- `crates/ruviform-ui-theme/src/bin/theme_codegen.rs` - CLI（`--manifest`/`--out`/`--check`）+ strict manifest parse + `ThemeResolver::resolve` 呼出 + Rust emit。lib 公開 API + 既存 `serde_json` のみ使用。
- `crates/ruviform-ui-theme/codegen/themes.json` - manifest 正本。synvelis-dark / audiolens-dark（`../ruviform-ui-test/fixtures/themes/` への相対 path）。
- `crates/ruviform-ui-theme/src/generated.rs` - 生成物（commit 対象・手編集禁止）。theme 毎に `pub mod <name>` + `pub struct Theme`（private typed fields + `theme: ResolvedTheme`）+ typed accessors + `resolved_theme()`/`into_resolved_theme()`/`MODE`。
- `crates/ruviform-ui-theme/src/lib.rs` - `pub mod generated;` 追加のみ。
- `crates/ruviform-ui-theme/Cargo.toml` - `[[bin]] name = "theme-codegen"` 追加（ui-test の `gui-boundary-guard` 前例に倣う rename）。
- `crates/ruviform-ui-theme/tests/codegen.rs` - byte-identical 決定性・committed 鮮度・生成型 vs 実行時 resolve の `to_normalized_json` 一致。
- `crates/ruviform-ui-theme/README.md` - codegen 節と実行時 resolve との使い分けを追記。
- `mise.toml` - `codegen-theme` / `codegen-theme-check` タスク追加。
- `docs/development-environment.md` - タスク対応表へ追記。

## Implementation Sequence

- [ ] `theme_codegen.rs` 実装（引数 parse → manifest parse → resolve → sanitize/emit → write/check）
- [ ] `Cargo.toml` に `[[bin]]` 追加、`codegen/themes.json` 作成
- [ ] `mise run codegen-theme` で `src/generated.rs` を生成し、rustfmt/clippy 適合を確認・調整
- [ ] `lib.rs` に `pub mod generated;` 追加
- [ ] `tests/codegen.rs` 実装（`CARGO_BIN_EXE_theme_codegen` 経由で bin を起動）
- [ ] `mise.toml` タスク + `docs/development-environment.md` + README 更新
- [ ] `docs/` チェーン（task/test-spec/impl ir）完成・status 更新

## Dependencies and Risks

- 依存: なし（新規 crate・外部依存なし）。`ResolvedTheme::new` は `pub(crate)` で crate 内生成からのみ利用 — crate 外 emit は本計画の範囲外。
- リスク: 生成コードが clippy pedantic / rustfmt に抵触 → emit 形式を調整（`#[must_use]`・`Default`・doc・`too_many_lines` のみ理由付き allow）。
- リスク: 識別子衝突 → 決定的エラーで fail-fast（fixture の token 命名は snake 互換で衝突しない見込み。実測で確認）。

## Task Breakdown

- T1: codegen bin + manifest + Cargo.toml `[[bin]]`（単一ファイル実装）
- T2: 生成物 emit + `src/generated.rs` + `lib.rs` 配線 + mise タスク
- T3: `tests/codegen.rs`（決定性・鮮度・実行時一致）+ README/docs 更新

## Verification

- [ ] `mise run codegen-theme` → 生成、`mise run codegen-theme-check` → 一致
- [ ] `cargo test -p ruviform-ui-theme`（codegen.rs 含む）green
- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` green
- [ ] 生成コードに Driver/Toolkit/DOM/wry 依存なし（crate 依存不変）

## Open Questions / Gaps

- なし（設計判断は design doc に確定済み。crate 外 emit は Deferred）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（Worker として単独実装）
- Independent work streams: none
- User approval required before dispatch: yes（Orca Worker 文脈では review-protocol レビューを承認代替として実施）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: 生成コードの lint/fmt 適合、`--check` の byte 比較契約、識別子 sanitize の決定性、実行時 resolve との値一致テストの網羅性
