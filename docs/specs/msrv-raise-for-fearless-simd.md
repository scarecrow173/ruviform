---
id: "SPEC-034JNYkUPfvyc1GQ5hXtHN"
type: "spec"
status: "approved"
title: "workspace MSRV 引上げ（fearless_simd 本番昇格前提）"
created: "2026-10-21"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/52"
  changes:
    added: []
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/implement-issue-52-msrv-raise.md"
  depends-on:
    - "../adr/dsp-simd-backend-adoption.md"
    - "dsp-simd-poc-adr.md"
    - "establish-rust-dev-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/rust-dev-foundation-toolchain-and-ci-profile.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/dsp-simd-backend-adoption.md"
    - "dsp-simd-poc-adr.md"
  derived-by:
    - "../adr/workspace-msrv-1-85-1-89-fearless-simd.md"
    - "../designs/msrv-raise-design.md"
    - "../plans/implement-issue-52-msrv-raise.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/34"
    - "https://github.com/scarecrow173/ruviform/issues/19"
    - "https://github.com/scarecrow173/ruviform/issues/53"
  defers: []
  deferred-by: []
---

# workspace MSRV 引上げ（fearless_simd 本番昇格前提）

## What Should Be Built

workspace の `rust-version` を `1.85` から採用バックエンド `fearless_simd` の要求する MSRV 以上に引き上げ、`fearless_simd` の `[dev-dependencies]` → `[dependencies]` 本番昇格（Issue #53 の前提）を解禁する。引上げ幅・対象範囲・ツールチェーン運用を確定し、`Cargo.toml`・`AGENTS.md`・README 等の MSRV 言及箇所を同期する。

## Why Now

採否 ADR [dsp-simd-backend-adoption.md](../adr/dsp-simd-backend-adoption.md) 決定 6 で、候補全員（`fearless_simd` 1.89・`multiversion` 1.86・`criterion` 1.86・`pulp` 推移的 ≈1.89）が現行 MSRV 1.85 を超過することが確定済み。`fearless_simd` の条件付き採用（決定 2）が結論されており、本番昇格の前提である本判断が Issue #53 をブロックしている。既知の制限として、PoC の bench/test ターゲットは edition 2024 let-chains（Rust 1.88+）を使用しており、dev-dep 抜きでも all-targets の MSRV は 1.85 では成立しないことが実測で確認済み（`--lib` 範囲のみ 1.85 適合）。

## Users and Value

- **開発者**: `fearless_simd` を `[dependencies]` へ昇格でき、Issue #53 の本番 SIMD dispatch 実装が着手可能になる
- **利用者・CI**: 引上げ後の MSRV でワークスペースが実際にビルドできることが検証され、MSRV 宣言値の信頼性が保たれる
- **保守**: `rust-version` の引上げ根拠と範囲が ADR として記録され、将来の再評価トリガ（`wgpu` 等の新依存）が明確になる

## Scope

### In Scope

- workspace `rust-version` の引上げ値の決定（`Cargo.toml` `[workspace.package]`）
- 引上げの適用範囲（workspace 全体 vs メンバー個別）の決定
- MSRV 言及箇所の同期（`Cargo.toml` コメント・`AGENTS.md`・README 等）
- 引上げ後の MSRV 実測検証（`cargo +<new-msrv> check` の範囲 — lib / all-targets いずれを検証基準とするかを含む）
- mise pin（`rust = "1.99.0"`）と MSRV の関係の明記

### Out of Scope

- `fearless_simd` の `[dependencies]` 昇格そのもの（Issue #53 のスコープ — 本 spec は MSRV 前提の解禁のみ）
- dispatch 層・カーネル API の設計・実装（Issue #53）
- CI ワークフローの MSRV matrix ジョブ追加（`verify-ci` は mise pin toolchain で検証する既存方針を維持。MSRV 専用 CI ジョブの要否は ADR で判断）
- mise pin toolchain（1.99.0）の変更 — pin は開発ツールであり MSRV 宣言値ではない

## Requirements

- `rust-version` の新値は `fearless_simd` 1.1.0 の要求（MSRV 1.89）を満たしつつ、過剰に上げすぎない（利用側環境の幅を維持）
- workspace 共通 `[workspace.package]` の `rust-version` として一括適用する（メンバー個別の分散 MSRV を作らない — `edition`/`rust-version` は workspace 継承が正本）
- AGENTS.md の「Rust ワークスペースと暫定開発規約」節の MSRV 言及（`rust-version = "1.85"`）を新値に同期する
- README や他の MSRV 言及箇所があれば同期する
- `cargo +<new-msrv> check --workspace --all-targets`（dev-dep 含む全ターゲット）または `--lib` 範囲での実測を行い、選択した検証基準と結果を記録する。既知の制限（bench/test ターゲットが let-chains 1.88+ を要求）は引上げ後の値で実際に解消するか確認する
- stable Rust のみ（nightly/beta を MSRV としない）

## Acceptance Criteria

- [x] `Cargo.toml` の `rust-version` が ADR で確定した値（1.89）に更新されている
- [x] `AGENTS.md` 等のドキュメント内 MSRV 言及が新値と一致する（`AGENTS.md`・`README.md`・`crates/ruviform-io-symphonia/Cargo.toml` コメントを同期）
- [x] `cargo +<new-msrv> check --workspace`（または選択した範囲）が成功し、結果が記録されている（実測: `cargo +1.89 check --workspace --all-targets`・`-p ruviform-math --lib` とも成功 — 結果は plan `PLAN-034JO2EWa8LuEEE4BFVZ9Q`・design `DESIGN-034JO1BQYrzMQsaFmL3YkF` に記録）
- [x] `fearless_simd` を `[dependencies]` に移した場合に MSRV 不足で失敗しないことが確認できる（宣言値 1.89 = `fearless_simd` 1.1.0 の要求値。昇格自体は Issue #53 スコープ）
- [x] MSRV 引上げの根拠・範囲・既知制限が ADR に記録され、後続の再評価トリガが明記されている（`ADR-034JNYt58J8NlUkn6RfJMs`）

## Deferred Design Concerns

- `fearless_simd` 本番昇格時の `rust-version` 不足再検証（`cargo +<msrv> check --all-targets` が dev-dep の新規 MSRV を含めて成立するか） | MSRV 引上げ時点では dev-dep のまま評価 | Issue #53 実装時 | 昇格後に all-targets MSRV が再度破れる可能性を残したままになる
- CI への MSRV 検証ジョブ追加 | 既存 `verify-ci` は mise pin toolchain で動く方針。MSRV ジョブは重複コストとの兼ね合い | rust-version 変更が再発した時点（新依存の MSRV 要求） | MSRV 宣言値の逸脱を CI で検出できず、リリース時に初めて判明する
- nightly `std::simd` との関係 | nightly 限定のため MSRV 判断の対象外 | `std::simd` stable 化 | 将来の最適解への移行パスを見逃す

## Sources and Relations

- 一次ソース: GitHub Issue [#52 [design] MSRV 引上げ判断（fearless_simd 本番昇格前提）](https://github.com/scarecrow173/ruviform/issues/52)（`relations.source`）。Issue #19 / PR #50 の DSP/SIMD PoC 採否 ADR 決定 6 で確定した前提課題として起票された。
- 派生元（`relations.derives-from`）: [dsp-simd-backend-adoption.md](../adr/dsp-simd-backend-adoption.md)（`ADR-034JEe7SAbksaycyrum16R` — 決定 6 で MSRV 引上げ Issue への送りと候補別 MSRV 一覧を確定）、[dsp-simd-poc-adr.md](dsp-simd-poc-adr.md)（spec「Deferred Design Concerns」で MSRV 引上げ Issue への移管を宣言）。
- 前提（`relations.depends-on`）: [establish-rust-dev-foundation.md](establish-rust-dev-foundation.md)（Issue #34 で `rust-version = "1.85"` を確定した基盤 spec — 本 spec がその値を改訂する）。
- 関連（`relations.related`）: [AGENTS.md](../../AGENTS.md)（stable-only・`unsafe` 原則禁止の確定制約）、[rust-dev-foundation-toolchain-and-ci-profile.md](../adr/rust-dev-foundation-toolchain-and-ci-profile.md)（toolchain pin と CI profile の決定 — mise pin は MSRV ではない）。
- 参考（`relations.references`）: Issue [#34](https://github.com/scarecrow173/ruviform/issues/34)（MSRV 1.85 確定の由来）、Issue [#19](https://github.com/scarecrow173/ruviform/issues/19)（DSP/SIMD PoC）、Issue [#53](https://github.com/scarecrow173/ruviform/issues/53)（本決定を前提とする本番昇格 Issue）。
