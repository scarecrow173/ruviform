---
id: "PLAN-034JO2EWa8LuEEE4BFVZ9Q"
type: "plan"
status: "approved"
title: "Issue #52 workspace MSRV 引上げの実施（rust-version 1.85 → 1.89・ドキュメント同期・MSRV 実測検証）"
created: "2026-10-21"
updated: "2026-10-21"
owners: ["scarecrow173"]
test-spec-skip: "設定値とドキュメント言及の同期のみの変更であり、コード上の新規振る舞いを持たない。検証は cargo +1.89 check --workspace --all-targets の実測と既存 mise/prek 検証列で行うため test-spec は作成しない"
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
  implements:
    - "../specs/msrv-raise-for-fearless-simd.md"
  implemented-by:
    - "../tasks/apply-msrv-1-89-and-sync-docs.md"
    - "../tasks/verify-msrv-1-89-build.md"
  depends-on:
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
    - "../designs/msrv-raise-design.md"
    - "../adr/dsp-simd-backend-adoption.md"
  derived-by:
    - "../tasks/apply-msrv-1-89-and-sync-docs.md"
    - "../tasks/verify-msrv-1-89-build.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/34"
    - "https://github.com/scarecrow173/ruviform/issues/53"
  defers: []
  deferred-by: []
---

# Issue #52 workspace MSRV 引上げの実施（rust-version 1.85 → 1.89・ドキュメント同期・MSRV 実測検証） Implementation Plan

## Goal

ADR `ADR-034JNYt58J8NlUkn6RfJMs`（workspace MSRV 1.85 → 1.89）を `Cargo.toml` に適用し、MSRV 言及ドキュメントを同期して `cargo +1.89 check --workspace --all-targets` で実測検証する。`fearless_simd` の `[dependencies]` 昇格前提を解禁する（昇格自体は Issue #53）。

## Scope

1 つの実装ストリーム（設定変更 + ドキュメント同期 + 実測検証）。変更行数は小さいが、検証は `cargo +1.89` toolchain を使う別経路のため独立した検証タスクとする。

## Upstream Documents

- [../specs/msrv-raise-for-fearless-simd.md](../specs/msrv-raise-for-fearless-simd.md) — `SPEC-034JNYkUPfvyc1GQ5hXtHN`（approved）。要件と受入条件
- [../adr/workspace-msrv-1-85-1-89-fearless-simd.md](../adr/workspace-msrv-1-85-1-89-fearless-simd.md) — `ADR-034JNYt58J8NlUkn6RfJMs`（accepted）。値 1.89・適用範囲・検証基準の決定
- [../adr/dsp-simd-backend-adoption.md](../adr/dsp-simd-backend-adoption.md) — 決定 6（MSRV 引上げ Issue への送り）の正本

## Design Inputs

- [../designs/msrv-raise-design.md](../designs/msrv-raise-design.md) — `DESIGN-034JO1BQYrzMQsaFmL3YkF`（approved）。適用範囲・検証経路・既知制限の設計

## File Map

- `Cargo.toml` — `[workspace.package] rust-version` の宣言値変更と隣接コメント更新（根拠を Issue #52/ADR に記す）
- `AGENTS.md` — 「Rust ワークスペースと暫定開発規約」節の `rust-version = "1.85"` 言及を `1.89` に同期
- `README.md` 等 — MSRV 言及箇所を検索し存在すれば同期（`grep -rn "1\.85" .` で洗い出し）

## Implementation Sequence

- [x] `Cargo.toml` の `rust-version = "1.85"` を `rust-version = "1.89"` に変更し、コメントを「Issue #34 で 1.85 確定 → Issue #52 / ADR で 1.89 に引上げ（fearless_simd 本番昇格前提）」に更新
- [x] `cargo +1.89 check --workspace --all-targets` を実行し、let-chains 使用の bench/test ターゲットを含めて成立することを確認（`rustup toolchain install 1.89` を実施）
- [x] `cargo +1.89 check -p ruviform-math --lib` で従来成立していた範囲の退行がないことを確認
- [x] `AGENTS.md`・その他 MSRV 言及箇所を `1.89` に同期（`README.md`・`crates/ruviform-io-symphonia/Cargo.toml` コメントも同期）
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`prek run --all-files` で pin toolchain（1.99）側の既存検証が維持されることを確認

## Dependencies and Risks

- `rustup toolchain install 1.89` — MSRV 実測に別 toolchain が必要。未インストールなら実装中に取得
- dev-dep の fresh resolve は lockfile 運用で回避（ADR 決定 6 の既知記録）
- 引上げ後の all-targets MSRV が破れた場合、ADR の「実測で不足なら次の要求値を取る」対策に従い値を再検討

## Task Breakdown

- `apply-msrv-1-89-and-sync-docs` — `Cargo.toml` 変更 + `AGENTS.md`/他言及箇所同期
- `verify-msrv-1-89-build` — `cargo +1.89 check --all-targets`・`--lib` 実測 + mise 検証全量 + spec/ADR/design/plan ステータス更新・PR 準備

## Verification

- [x] `cargo +1.89 check --workspace --all-targets` 成功（let-chains ターゲット含む）
- [x] `cargo +1.89 check -p ruviform-math --lib` 成功
- [x] `Cargo.toml` `rust-version` が `1.89`・コメントに Issue #52/ADR 参照
- [x] `AGENTS.md` の MSRV 言及が `1.89` に一致
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`bench-smoke`・`prek run --all-files` 全パス
- [x] `audit_docs.js --type all --json` で本変更起因 findings 0（残存5件は wave-1 由来の既存非ブロッキング警告）

## Open Questions / Gaps

- `cargo +1.89 check --all-targets` で dev-dep の推移的要求が 1.89 を超えた場合のフォールバック値は ADR の再検討対象（発生したら値を上げて plan/ADR を修正）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（変更量小・検証がローカル toolchain 依存のため）
- Independent work streams: none（順序依存の 2 タスク）
- User approval required before dispatch: yes
- If approved: execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `cargo +1.89` 実測結果（特に all-targets の成立）が ADR の検証基準を満たすかを重点確認
