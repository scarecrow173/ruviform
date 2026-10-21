---
id: "DESIGN-034JO1BQYrzMQsaFmL3YkF"
type: "design"
status: "approved"
title: "workspace MSRV 引上げの適用範囲と検証設計"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/msrv-raise-for-fearless-simd.md"
    - "../adr/workspace-msrv-1-85-1-89-fearless-simd.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/msrv-raise-for-fearless-simd.md"
    - "../adr/workspace-msrv-1-85-1-89-fearless-simd.md"
    - "../adr/dsp-simd-backend-adoption.md"
  derived-by:
    - "../plans/implement-issue-52-msrv-raise.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/34"
    - "https://github.com/scarecrow173/ruviform/issues/53"
  defers: []
  deferred-by: []
---

# workspace MSRV 引上げの適用範囲と検証設計

## Context

採否 ADR 決定 2 で条件付き採用された `fearless_simd` 1.1.0 の本番昇格前提として、workspace `rust-version` を `1.85` から `1.89` に引き上げる（ADR `ADR-034JNYt58J8NlUkn6RfJMs`）。本設計は「どこを変更し、どう検証するか」の適用範囲と検証手順を固定する。宣言値の変更自体は一行だが、`rust-version` は workspace 全メンバーの継承値であり、ドキュメント言及の同期漏れや検証範囲の曖昧さが将来の混同を生む。

## Scope

### In Scope

- `Cargo.toml` `[workspace.package] rust-version` の `1.85` → `1.89` 変更と隣接コメントの更新（引上げ根拠を Issue #52/ADR に記す）
- `AGENTS.md`「Rust ワークスペースと暫定開発規約」節の `rust-version = "1.85"` 言及の同期（Issue #34 確定 → Issue #52 で 1.89 に改訂）
- `README.md` その他の MSRV 言及箇所の検索・同期（存在する場合）
- `cargo +1.89` 実測検証（範囲: `--workspace --all-targets`。let-chains を使う bench/test ターゲットを含めて成立確認）

### Out of Scope

- `fearless_simd` の `[dependencies]` 昇格（Issue #53）
- mise pin `rust = "1.99.0"` の変更（pin は開発ツールで MSRV ではない）
- CI への MSRV 専用ジョブ追加（ADR で不採用 — ローカル `cargo +<msrv>` 検証で十分）
- メンバー個別 `rust-version`（workspace 一括継承を維持 — ADR Option D 否決）

## Components and Boundaries

- `Cargo.toml` — `[workspace.package]` の `rust-version` が唯一の正本宣言値。メンバー `Cargo.toml` は `rust-version.workspace = true` で継承（確認して個別値の混入がないこと）
- `AGENTS.md` — 開発者契約ドキュメントとして MSRV を言及する唯一の正本（README・issue テンプレート等の二次言及は検索で洗い出し同期）
- 検証経路 — `rustup` で `1.89` toolchain を用いた `cargo +1.89 check`。mise pin とは別 toolchain で実行することで MSRV 宣言値の実測を担保

## Data and Control Flow

1. `rust-version = "1.89"` に変更しコメントを更新
2. `rustup toolchain install 1.89`（未インストールなら）して `cargo +1.89 check --workspace --all-targets` を実行
3. let-chains を使う bench/test ターゲットを含めて成立を確認（1.85 時点では `--lib` のみ成立だった既知制限が解消するかを観測）
4. `AGENTS.md`・その他 MSRV 言及を同期
5. mise pin（1.99.0）の既存検証（fmt-check/check/lint/test/test-rt）は変更なしで全パスすることを確認

## Non-Functional Constraints

- stable Rust のみ（nightly/beta を MSRV にしない — AGENTS.md 確定制約）
- workspace 一括 `rust-version` を維持（メンバー個別の分散 MSRV を作らない）
- `cargo +1.89 check` の範囲は `--all-targets` を含む（`--lib` のみでは bench/test ターゲットの実測が漏れる）

## Risks and Trade-offs

- **リスク**: `cargo +1.89 check --all-targets` で dev-dep の推移的要求が 1.89 を超える場合、本番昇格後に再度破れる → 対策: 引上げ時に all-targets で実測し、失敗時は値を再検討（ADR の値は最小必要値であり、実測で不足が判明したら次の要求値を取る）
- **トレードオフ**: 1.89 は最小適合値のため、将来の依存が 1.90+ を要求したら再度引上げ Issue が必要 — しかし最小値を取ることで利用側の互換幅を最大化し、再評価トリガは既に ADR に明記されている
- **既知制限の残置**: dev-dep の fresh `cargo +1.89` resolve は lockfile があれば問題ないが、ゼロからの resolve は dev-dep 範囲の適合版に依存する（ADR 決定 6 の既知記録を継承）

## Deferred Design Concerns

- `fearless_simd` 本番昇格時の `rust-version` 実測再検証 | 昇格は #53 スコープ | #53 実装時 | dev-dep → dependencies 移行後の all-targets MSRV 再確認が漏れる可能性
- CI の MSRV matrix ジョブ | 現行 `verify-ci` は mise pin で検証 | rust-version 再変更時 | MSRV 宣言値の逸脱を CI で検出しない

## Verification Notes

- [ ] `cargo +1.89 check --workspace --all-targets` が成功（let-chains 使用ターゲットを含む）
- [ ] `cargo +1.89 check -p ruviform-math --lib` が従来どおり成功（`--lib` 範囲の退行なし）
- [ ] `Cargo.toml` の `rust-version` が `1.89`、隣接コメントが Issue #52/ADR 参照に更新されている
- [ ] `AGENTS.md` の MSRV 言及が `1.89` に一致する
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が全パス（mise pin 1.99 側の既存検証を維持）
