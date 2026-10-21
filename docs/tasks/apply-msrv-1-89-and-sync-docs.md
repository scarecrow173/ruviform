---
id: "TASK-034JO3X9ZpAvMoWiRIiMU4"
type: "task"
status: "done"
title: "MSRV 1.89 適用とドキュメント同期（Cargo.toml・AGENTS.md）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
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
  implements:
    - "../plans/implement-issue-52-msrv-raise.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-52-msrv-raise.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-52-msrv-raise.md"
    - "../adr/workspace-msrv-1-85-1-89-fearless-simd.md"
  derived-by:
    - "verify-msrv-1-89-build.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/52"
  defers: []
  deferred-by: []
---

# MSRV 1.89 適用とドキュメント同期（Cargo.toml・AGENTS.md）

## Work

ADR `ADR-034JNYt58J8NlUkn6RfJMs` の決定どおり、workspace `rust-version` を `1.85` から `1.89` に引き上げ、MSRV 言及ドキュメントを同期する。

1. `Cargo.toml` `[workspace.package]` の `rust-version = "1.85"` を `"1.89"` に変更し、隣接コメントを更新（Issue #34 で確定した 1.85 を Issue #52 / ADR で 1.89 に引上げ — `fearless_simd` 1.1.0 の MSRV 1.89 要求に起因）
2. メンバー `Cargo.toml` 全件で `rust-version.workspace = true` 継承であることを確認（個別値の混入がないこと）
3. `AGENTS.md`「Rust ワークスペースと暫定開発規約」節の `rust-version = "1.85"` 言及を `1.89` に同期
4. `grep` で `1.85` / `MSRV` / `rust-version` の言及を洗い出し、README 等他の二次言及があれば同期
5. `mise.toml` の `rust = "1.99.0"` pin は変更しない（開発ツール pin であり MSRV ではない）

`fearless_simd` の `[dependencies]` 昇格は本タスクの範囲外（Issue #53）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-52-msrv-raise.md](../plans/implement-issue-52-msrv-raise.md) — 実施計画
- [../adr/workspace-msrv-1-85-1-89-fearless-simd.md](../adr/workspace-msrv-1-85-1-89-fearless-simd.md) — 値と適用範囲の決定

## Dependencies

- 先行タスク: なし（チェーン先頭）
- ブロック対象: [verify-msrv-1-89-build.md](verify-msrv-1-89-build.md)

## Affected Files

- `Cargo.toml`
- `AGENTS.md`
- 検索で検出された MSRV 言及ファイル（存在する場合）

## Implementation Notes

- コメント更新は「なぜ 1.89 か」を将来の読者が追えるように Issue #52 と ADR ID を記す
- `Cargo.lock` の再生成は本タスクでは不要（`rust-version` 変更は resolve に影響しない）

## Done Criteria

- [x] `Cargo.toml` の `rust-version` が `1.89`、コメントに Issue #52 / ADR 参照がある
- [x] メンバー `Cargo.toml` に個別 `rust-version` が存在しない（全件 `workspace = true` 継承）
- [x] `AGENTS.md` の MSRV 言及が `1.89` に一致
- [x] 他の MSRV 言及箇所が検索で洗い出され同期済み（または非存在を確認）

## Verification

- [x] `mise run fmt-check`（toml/md 整形ルールに抵触しないこと）
- [x] `mise run check` — pin toolchain 側で宣言値変更が既存検証を破らないこと

## Completion Notes

同期した MSRV 言及箇所: `Cargo.toml`（宣言値 + コメント）、`AGENTS.md` L114、`README.md` L33、`crates/ruviform-io-symphonia/Cargo.toml` L32 コメント（symphonia 0.6 の MSRV 1.85 は workspace 1.89 で充足される記述に更新）。`poc/ui/RESULTS.md` の wgpu MSRV メモ（Issue #34 時点の衝突記録・Issue #57 追跡対象）と `docs/specs/dsp-simd-poc-adr.md` の前提記述は発行時点の記録のため変更しない。`mise.toml` の `rust = "1.99.0"` pin は据え置き。`cargo +1.89 check --workspace --all-targets` / `-p ruviform-math --lib` は先行実測で成功（詳細は verify タスクで記録）。
