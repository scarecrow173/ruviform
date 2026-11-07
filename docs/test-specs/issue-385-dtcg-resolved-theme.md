---
id: "TSPEC-7kQ2mWcR9dLx4VfT1hZa8N"
type: "test-spec"
status: "approved"
title: "Issue #385 — DTCG 解釈器・ResolvedTheme の golden / 分類エラー検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/385"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-theme/tests/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-theme/tests/registry.rs"
      - type: "file"
        path: "crates/ruviform-ui-theme/tests/golden"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-theme/src/registry.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #385 — DTCG 解釈器・ResolvedTheme の検証契約

## Feature: 決定的な normalized 出力（golden）

### Rule: 同一入力は同一バイト列を返す
- Example: `valid-dark`（primitive + semantic + component、dark）の `to_normalized_json()` が `tests/golden/valid-dark.json` とバイト一致する。
- Example: 同じ文書集合を逆順で渡しても、同じ mode の出力は同一バイト列になる（`output_is_independent_of_input_order_and_repetition`）。

### Rule: Consumer override は同一 key・同一 mode scope のみに作用する
- Example: `consumer-override-dark`（base 文書 + `product-override.json`）が golden と一致し、`color.muted` のような override 対象外の key は base 値のまま（`consumer_override_scoped_to_same_key_and_mode`）。

### Rule: OS font 不在では fallback 列を改変しない
- Example: `font.brand` の fallback 列 `["Ruviform Brand Not Installed", "Inter", "sans-serif"]` が順序どおり保持され、`os-font-fallback-dark` golden と一致する。インストール有無の判定は Driver の責務（本 spec の範囲外）。

### Rule: DPI scale は ScaleContext のみを変える
- Example: `with_scale(scale_factor = 2.0, rem_base_px = 16.0)` 後も token 列は base と完全一致し、`dpi-scale-2x-dark` golden は scale 欄のみ異なる。

## Feature: 分類済みエラー（invalid 系）

### Rule: 不正値は document error として分類される
- Example: `err-invalid-value.json` の parse が `TokenError::InvalidValue` を返す。

### Rule: 循環・型不一致・mode 欠落は resolve 時に分類される
- Example: `err-cycle.json` → `ResolveError::AliasCycle`。
- Example: `err-type-mismatch.json` を override として積む → `ResolveError::TypeMismatch`。
- Example: `product-light-only.json`（`modes: ["light"]`）を dark で解決 → `ResolveError::MissingMode { document: "consumer-light-only", mode: "dark" }`。

## Feature: Theme Registry

### Rule: Registry は解決規則を持たず ThemeResolver へ委譲する
- Example: `ThemeRegistry::resolve("base", "dark")` の normalized 出力が `ThemeResolver::resolve` の直接出力と一致する（`registry_matches_direct_resolver_output`）。
- Example: 未登録名は `ResolveError::UnknownTheme { name }`。
- Example: 登録済み stack の mode 欠落は `MissingMode` としてそのまま返る。

## 対象外（本 spec では検証しない）
- OS フォントの実在判定・text metrics・DPI による実描画差 → Driver capability / 実機検証（#386/#390）。
- GPUI 等 Toolkit への mapping → Driver 側（本 crate は依存を持たない）。
