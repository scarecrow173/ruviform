---
id: "IMPL-034UoQCq3m1rqajAlcen87"
type: "impl"
status: "in-progress"
title: "Issue #418 — theme-codegen 実装記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
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
      - type: "file"
        path: "docs/specs/issue-418-theme-codegen.md"
      - type: "file"
        path: "docs/designs/issue-418-theme-codegen.md"
      - type: "file"
        path: "docs/plans/issue-418-theme-codegen.md"
      - type: "file"
        path: "docs/tasks/issue-418-theme-codegen.md"
      - type: "file"
        path: "docs/test-specs/issue-418-theme-codegen.md"
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
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "crates/ruviform-ui-theme/src/generated.rs"
  implements:
    - "docs/tasks/issue-418-theme-codegen.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-418-theme-codegen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-418-theme-codegen.md"
    - "docs/designs/issue-418-theme-codegen.md"
    - "docs/plans/issue-418-theme-codegen.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-418-theme-codegen.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #418 — theme-codegen 実装記録

## Summary

`ruviform-ui-theme` に manifest 駆動の build-time codegen を実装した。
`codegen/themes.json` が宣言する各 theme について `theme-codegen` bin が
`TokenDocument::from_json` + `ThemeResolver::resolve` を生成時に 1 回実行し、
結果の `ResolvedTheme` を `src/generated.rs` の型付き Rust リテラルとして
emit する。resolve 規則の第二実装は持たず、意味解釈の正本は runtime
pipeline のみ（spec §スコープ・design §設計判断どおり）。

## Context

- Task: `docs/tasks/issue-418-theme-codegen.md`（`normal-plan-task`）
- Plan: `docs/plans/issue-418-theme-codegen.md`（approved）
- Spec: `docs/specs/issue-418-theme-codegen.md` / Design:
  `docs/designs/issue-418-theme-codegen.md` / Test spec:
  `docs/test-specs/issue-418-theme-codegen.md`（いずれも approved）
- 前提: Issue #385 の `ThemeResolver`/`ResolvedTheme` pipeline（PR #417
  squash `0dacf22c` で main 統合済み）。codegen はその公開 API のみを使う。

## Implementation

### Changes

- `src/bin/theme_codegen.rs` — CLI（`--manifest`/`--out`/`--check`）。
  manifest は `serde_json::Value` で strict parse（未知 key・型不一致・
  空配列はエラー）。document path は manifest ファイルのある dir 相対で
  解決し、`ThemeResolver::resolve` の結果だけを emit する。token path →
  識別子は ASCII 英数字 snake_case 正規化 + 先頭数字 `n` 前置 + keyword
  末尾 `_`。衝突・予約語（`resolved`/`new`/`default`/`clone`/
  `resolved_theme`/`into_resolved_theme`）・module 名衝突は決定的エラー。
  `#[non_exhaustive]` enum の未知 variant は明示エラーで fail-closed。
  出力は rustfmt 子プロセスで正規化（rustfmt 不在は明示エラー・決定性の
  ためフォールバックなし）。`--check` は書き込まず byte 比較。
- `codegen/themes.json` — 3 theme: `audiolens-dark`・`synvelis-dark`
  （ui-test product fixture）+ `rvf-primitive`（crate 内 fixture。
  全 8 `ResolvedValue` variant・alias・mode・rounding を網羅するために
  追加）。
- `src/generated.rs` — 生成物（commit 対象）。module ごとに
  `pub struct Theme`（typed field + private `resolved: ResolvedTheme`）、
  `MODE` const、`new()`/`Default`/`resolved_theme()`/`into_resolved_theme()`、
  token ごとの typed accessor。f64/u64 リテラルは clippy canonical の
  数字区切りで emit。
- `src/lib.rs` — `pub mod generated;`（`//` コメント。`///` + `//!` 併用は
  `clippy::mixed_attributes_style` 違反のため）。
- `Cargo.toml` — `[[bin]] name = "theme-codegen"`（新規依存なし）。
- `tests/codegen.rs` — 6 test: 繰り返し生成 byte-identical・committed
  鮮度（`--check`）・manifest 全 theme の generated/runtime
  `to_normalized_json()` 一致・accessor 型検査・invalid manifest /
  存在しない document で非 0 終了かつ部分出力なし。
- `mise.toml` — `codegen-theme` / `codegen-theme-check` task 追加のみ。
- `README.md`・`docs/development-environment.md` — codegen 節・task 表・
  runtime/static 使い分け方針を記述。

### Decisions During Implementation

- **in-crate bin を選択**: `ResolvedTheme::new` は `pub(crate)` のため、
  生成コードは同一 crate 内 module として emit する必要がある。新規
  workspace crate は task 制約で不可のため `src/bin/` + `src/generated.rs`
  の組合せとした。
- **rustfmt 子プロセス正規化**: emitter が rustfmt canonical 形を手計算で
  再現するのは脆弱なため、生成テキストを `rustfmt --edition 2024` の
  stdin/stdout に通す。mise pin toolchain の component であり、未導入は
  明示エラー（canonical= mise 経路を前提）。
- **field 名 `theme` → `resolved`**: `struct_field_names`（field 名が
  struct 名で始まる）回避 + 予約語衝突の明確化のため改名。
- **`rvf-primitive` を manifest に追加**: 当初の 2 product fixture は
  color/dimension のみで、FontFamily 等の emitter 経路が未検証だった。
  crate 内 fixture（全 variant 網羅）を第 3 theme として追加し、生成
  コードの全 variant コンパイルと runtime 一致を end-to-end で担保した。
- **数字区切り literal**: `unreadable_literal`/`inconsistent_digit_grouping`
  対策として f64 は小数部を左 3 桁区切り・整数部/指数部と u64
  fingerprint を右 3 桁区切りで emit（値は不変）。
- **`#[allow(clippy::too_many_lines)]` は生成 `new()` のみに局所付与**:
  token 数に比例して必然的に長くなる機械生成テーブルであり、他 lint の
  抑制は行わない（`mixed_attributes_style`/`struct_field_names`/
  `unreadable_literal`/`inconsistent_digit_grouping` は emitter 側の
  設計で根本解消した）。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

<!-- なし -->

## Validation

- `mise run codegen-theme` / `mise run codegen-theme-check` — 生成・鮮度
  byte 比較ともに成功。
- `cargo clippy -p ruviform-ui-theme --all-targets -- -D warnings` — clean
  （補助証拠。canonical は `mise run lint`）。
- `cargo test -p ruviform-ui-theme --test codegen` — 6 test 全 pass
  （補助証拠。canonical は `mise run test`）。
- `cargo fmt -p ruviform-ui-theme` 適用後も `--check` が byte 一致を維持。

## Risks

- rustfmt の version 差で生成 bytes が変わり得る。toolchain は
  `mise.toml` で pin されており、toolchain 更新時は `mise run
  codegen-theme` で再生成が必要（`--check` が検出する）。
- manifest に新規 `ResolvedValue` variant（enum 追加）や新文書が入ると
  codegen が明示エラーで停止する。これは意図した fail-closed であり、
  対応は emitter 拡張として行う。

## Follow-ups

- Driver/Component 層からの生成 theme 利用は Issue #386/#387 の範囲。
- Figma import・DTCG 型拡張は本 Issue の対象外（spec §対象外）。
