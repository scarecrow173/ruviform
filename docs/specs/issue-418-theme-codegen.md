---
id: "SPEC-034Uoh0rqjKI3fjlaRV6Fj"
type: "spec"
status: "approved"
title: "Issue #418 — DTCG token から ResolvedTheme 相当の静的 Rust 型を生成する codegen"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/418"
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
  changes:
    added:
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
      - type: "file"
        path: "docs/impl/ir/issue-418-theme-codegen.md"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/418"
  derived-by:
    - "docs/designs/issue-418-theme-codegen.md"
    - "docs/plans/issue-418-theme-codegen.md"
    - "docs/test-specs/issue-418-theme-codegen.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/390"
  defers: []
  deferred-by: []
---

# Issue #418 — DTCG token から ResolvedTheme 相当の静的 Rust 型を生成する codegen

## What Should Be Built

Git 管理の DTCG token document 群（`rvf-dtcg-profile.v1`）を入力として、#385 の `ThemeResolver::resolve` を**生成時に実行**し、その `ResolvedTheme` を Rust の静的型として具現化する codegen 経路を `ruviform-ui-theme` に追加する。生成物は「解決済み token 集合を型付きアクセサとして持つ不変 snapshot を構築する Rust モジュール」であり、codegen の入口は mise タスクとして定義する。

## Why Now

- #385（PR #417, squash `0dacf22c`）で実行時 resolve パイプラインを実装した際、「大規模 token 集合のビルド時 codegen」は spec の Deferred Design Concerns として別 Issue 化が決まっており、本 Issue #418 がその追跡先である。
- token 集合が既知・固定の組み込み/出荷 Theme では、token 名と値をコンパイル時に確定した型として提供すると、Driver mapper や Component 側が文字列キー探索なしに型安全に参照できる。一方、Consumer が実行時に登録する動的 Theme は引き続き `ThemeRegistry`/`ThemeResolver` が担うため、両経路の整合方針を明文化する必要がある。

## Users and Value

- `ruviform-ui-theme` の消費側（Driver mapper・Component/View・将来の Consumer アプリ）: 既知 Theme の token を `theme.surface_bg()` のような型付き・タイポコンパイル時検出可能な形で取得できる。
- Theme 定義の保守者: token JSON を編集して codegen を再実行するだけで Rust 側の型付き表面が追従し、golden/byte 比較テストで生成漏れ・手編集を検出できる。

## Scope

### In Scope

- `ruviform-ui-theme` 内の codegen 入口: manifest（theme 名・mode・document 一覧）を読み、各 theme を `ThemeResolver::resolve` で解決して Rust モジュールを出力する CLI バイナリ。
- 生成物の commit 運用と、同一入力→同一出力の byte-identical 決定性を検証するテスト（golden/差分比較）。
- 生成コードが `cargo check` / `mise run lint` / `mise run test` を通ること（生成物自体が workspace の検証対象に入る）。
- 実行時 resolve（`ThemeRegistry`/`ThemeResolver`）と codegen の使い分け・値の同一性の文書化（crate README・design doc）。

### Out of Scope

- 既存 `ruviform-ui-theme` の実行時解決契約（parse→validate→alias→override→`ResolvedTheme`）の変更・再実装。
- Consumer 固有 Theme の中身の決定、Figma import、`typography` 等の v1 対象外 `$type` の拡張。
- crate 外への生成物配置（生成コードは `crate::` パスを前提とし、`ruviform-ui-theme` 内モジュールとして commit する。外部 crate への出力は公開コンストラクタ整備を伴う別 Issue）。
- 新規 workspace クレート・新規外部依存の追加。

## Requirements

- codegen の入口は `mise.toml` のタスクとして定義する（Issue 受入条件）。生成は決定的でなければならない: 同一 manifest・同一入力文書からは常に byte-identical な出力を生成する。
- 生成コードは workspace の lint（clippy pedantic + `-D warnings`）・fmt・test を無変更で通過する。
- 生成型の値は、同一入力を実行時 `ThemeResolver::resolve` に通した結果と一致しなければならない（codegen は resolve パイプラインを再実装せず、その出力を静的化する）。
- token path → Rust 識別子の写像は決定的かつ衝突検出つきとし、衝突・不正名は生成時エラーとする。
- 実行時 resolve との差異（解決タイミング・動的/静的の使い分け）が文書化される。

## Acceptance Criteria

- [ ] codegen の入口（CLI コマンドまたは build タスク）が mise タスクとして定義されている
- [ ] 生成コードが `cargo check` / `mise run lint` / `mise run test` を通る
- [ ] 実行時 resolve との差異がある場合、その扱い（使い分け・非推奨化等）が文書化されている
- [ ] 同一入力の繰り返し生成結果を比較し、生成の決定性を検証する（golden または byte-identical 比較テスト）

## Deferred Design Concerns

- crate 外 crate への生成物 emit（`ResolvedTheme` の公開 materialize API が前提）→ 実需時の別 Issue。現状は crate 内生成に限定。
- 大規模 token 集合での生成物サイズ・コンパイル時間の最適化 → プロファイル実測後。

## Sources and Relations

- Issue #418（codegen 追跡 Issue）、#385（ResolvedTheme パイプライン・本 Issue の前提）、#386（Driver Theme Mapping — 生成型の消費側）、#390（semantic/visual golden — 受入連携）。
- 実装設計: [issue-418-theme-codegen.md](../designs/issue-418-theme-codegen.md)。
- 前提 spec: [issue-385-dtcg-resolved-theme.md](issue-385-dtcg-resolved-theme.md)（Deferred Design Concerns「大規模 token 集合のビルド時 codegen」）。
