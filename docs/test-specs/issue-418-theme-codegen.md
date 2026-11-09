---
id: "TSPEC-034UohaTnl4VwJLJCSfXgJ"
type: "test-spec"
status: "approved"
title: "Issue #418 — theme-codegen の検証契約"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/418"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-theme/tests/codegen.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-418-theme-codegen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-385-dtcg-resolved-theme.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-418-theme-codegen.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-418-theme-codegen.md"
    - "docs/designs/issue-418-theme-codegen.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #418 — theme-codegen の検証契約

## Purpose

`theme-codegen` が「決定的・実行時 resolve と値一致・lint/fmt 適合」の 3 契約を満たすことを、`crates/ruviform-ui-theme/tests/codegen.rs` と mise タスクで検証可能にする。本 spec は codegen 機構の退行（非決定的 emit・stale な commit 済み生成物・resolve との値乖離）を恒久的に検出することを目的とし、codegen が存在する限り退役しない。

## Feature

DTCG manifest 駆動の静的型 codegen — 同一入力に対し byte-identical な Rust モジュールを生成し、その値が `ThemeResolver::resolve` の出力と一致する。

## Rules

- **Rule 1: 決定性** — 同一 manifest・同一入力文書に対し、生成を繰り返しても出力バイト列が一致する（ソート順・数値表記・文字列エスケープが安定）。
- **Rule 2: 鮮度** — commit 済み `src/generated.rs` は最新の manifest + 入力文書からの再生成結果と byte 一致する（`--check` が exit 0）。手編集・入力変更後の未再生成を検出する。
- **Rule 3: 実行時一致** — 生成型 `Theme::new().resolved_theme().to_normalized_json()` が、同一 document 群を実行時 `ThemeResolver::resolve(docs, mode)` に通した `to_normalized_json()` と一致する（全 token・sources・mode を含む）。
- **Rule 4: 型付き表面** — 生成 module/struct/accessor が公開 API として存在し、`cargo check`/`clippy -D warnings`/`cargo fmt --check` を通過する（生成コード自体が workspace 検証対象）。
- **Rule 5: 失敗の正直さ** — manifest 不正・document 解決失敗・識別子衝突は生成時に非 0 終了で報告され、部分的な出力 file を残さない。

## Examples

- Given `codegen/themes.json`（synvelis-dark + audiolens-dark）, When `theme-codegen` を 2 回実行, Then 2 つの出力ファイルのバイト列が一致する。
- Given commit 済み `src/generated.rs`, When `theme-codegen --check` を実行, Then exit 0（一致）。`src/generated.rs` を 1 byte 改変すると exit 1。
- Given manifest の theme `synvelis-dark`/`dark`, When 生成型と実行時 resolve を比較, Then `to_normalized_json()` が一致し、`surface.bg` 等の accessor が `ResolvedColor` を返す。

## Guarantees

- 生成の決定性と commit 済み生成物の鮮度が `cargo test -p ruviform-ui-theme` で常時検証される。
- codegen が resolve を再実装していないこと（値一致テスト）が保証される。
- `mise run codegen-theme` / `codegen-theme-check` が入口として存在する（Issue 受入）。

## Non-goals

- 生成コードの網羅的 lint 抑制の有無の監査（`too_many_lines` の 1 箇所のみ設計で正当化済み）。
- crate 外 emit・大規模 token 集合の性能計測（Deferred）。
- Driver/Component 側での生成型の実使用（#386/#387 の領域）。

## Risk

- byte 比較テストが rustfmt/ツールチェーン差分に脆い → 生成テキストを rustfmt 規約で直接 emit し、pin toolchain（mise rust 1.99.0）でのみ検証する前提を明記。toolchain 更新時は再生成で吸収。
- `CARGO_BIN_EXE_*` 非対応環境 → cargo test の標準機構であり workspace 内で実績あり（`gui-boundary-guard` 前例は test では未使用だが bin target 規約は共通）。万一 env が無い場合は test が明示的に fail し、黙って skip しない。
