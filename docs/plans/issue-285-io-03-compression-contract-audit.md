---
id: "PLAN-LwPSJRNM4HAuPalTuxW7pu"
type: "plan"
status: "approved"
title: "Issue #285 IO-03 Compression 契約試験棚卸し 実装計画"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/285"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-285-io-03-compression-contract-audit.md"
      - type: "file"
        path: "docs/designs/issue-285-io-03-compression-contract-audit.md"
      - type: "file"
        path: "docs/test-specs/issue-285-io-03-compression-contract-audit.md"
      - type: "file"
        path: "docs/plans/issue-285-io-03-compression-contract-audit.md"
      - type: "file"
        path: "docs/tasks/issue-285-t1-contract-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-285-io-03-compression-contract-audit.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-compression/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "docs/designs/issue-285-io-03-compression-serde-boundary.md"
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
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-285-io-03-compression-contract-audit.md"
    - "docs/designs/issue-285-io-03-compression-contract-audit.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-285-io-03-compression-contract-audit.md"
    - "docs/designs/issue-285-io-03-compression-contract-audit.md"
    - "docs/test-specs/issue-285-io-03-compression-contract-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-285-io-03-compression-serde-boundary.md"
    - "docs/plans/issue-300-io-compression-gzip-deflate.md"
    - "docs/plans/issue-301-io-compression-zstd.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-285-io-03-compression-contract-audit.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-285-io-03-compression-contract-audit.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #285 IO-03 Compression 契約試験棚卸し 実装計画 Implementation Plan

## Goal

Issue #285 受入条件の「設計正本照合」「ENC/DEC/SEC 契約試験の
棚卸しと不足分追加」を、headless で誠実に実行可能な範囲で
閉じる。`ruviform-io-compression` の既存試験を契約へ写像し、
failure-path・flush/finish/Drop 区別・連結単位境界・高圧縮率
budget の不足を `tests/` に追加して、mise lane で実測する。

## Scope

単一実装ストリーム・1 plan・1 task。変更は
`crates/ruviform-io-compression/tests/`（`common/mod.rs`・
`encoder_contract.rs`・`decoder_contract.rs`）と本 doc チェーン、
設計正本 `docs/designs/issue-285-io-03-compression-serde-boundary.md`
の snapshot/証拠索引更新に限定。`src/`・`Cargo.toml`・他
クレートは変更しない（照合で設計不整合が確認された場合のみ
最小修正を検討）。

## Upstream Documents

- [docs/specs/issue-285-io-03-compression-contract-audit.md](../specs/issue-285-io-03-compression-contract-audit.md)（approved）
- [docs/designs/issue-285-io-03-compression-contract-audit.md](../designs/issue-285-io-03-compression-contract-audit.md)（approved）
- [docs/test-specs/issue-285-io-03-compression-contract-audit.md](../test-specs/issue-285-io-03-compression-contract-audit.md)（検証正本）
- 正本設計: [spec](../specs/issue-285-io-03-compression-serde-boundary.md) /
  [design](../designs/issue-285-io-03-compression-serde-boundary.md) /
  [plan](../plans/issue-285-io-03-compression-serde-boundary.md) /
  [test-spec](../test-specs/issue-285-io-03-compression-serde-boundary.md) /
  [task](../tasks/issue-285-io-03-compression-serde-boundary.md)
- 実装 slice: `docs/plans|tasks|impl/ir/issue-300-*`・`issue-301-*`

## File Map

- `crates/ruviform-io-compression/tests/common/mod.rs` —
  `FailAfter.inner` 追加、`FailFlush`/`RecoveringWriter`/
  `FailingReader` fixture 追加。
- `crates/ruviform-io-compression/tests/encoder_contract.rs` —
  gzip/deflate/zstd 各 mod に R1〜R7 対応試験を追加。
- `crates/ruviform-io-compression/tests/decoder_contract.rs` —
  gzip/deflate/zstd 各 mod に R8〜R12 対応試験を追加。
- `docs/designs/issue-285-io-03-compression-serde-boundary.md` —
  snapshot 追記・証拠項目対応表へ新試験名を併記。
- `docs/{specs,designs,test-specs,plans,tasks,impl/ir}/
  issue-285-*contract-audit*` — 本 doc チェーン。

## Implementation Sequence

- [x] spec → design → test-spec → plan → task の doc chain 作成
- [x] `tests/common/mod.rs` に fault fixture を追加・強化
- [x] `tests/encoder_contract.rs` に R1〜R7 対応試験を追加
- [x] `tests/decoder_contract.rs` に R8〜R12 対応試験を追加
- [x] `cargo fmt` + 対象試験の実走で compile/pass 確認
- [x] `docs/designs/issue-285-io-03-compression-serde-boundary.md`
      の snapshot・証拠索引を更新
- [x] `mise run fmt-check`/`check-io-compression`/
      `lint-io-compression`/`test-io-compression`/`test-io-e2e`/
      `test-rt`/`test-io-msrv` 実走
- [ ] managed docs index 再生成（doc-maintenance apply）
- [ ] `git fetch origin main && git merge origin/main` で最新取込
- [ ] workspace `check`/`lint`/`test` 非回帰確認
- [ ] push・PR 作成（日本語・`Refs #285`・close しない）

## Dependencies and Risks

- 依存: なし（crate は main merge 済み・toolchain は mise pin）。
- Risk: fault fixture の write 回数仮定が backend でずれる —
  試験が fail して検出される設計（偽装 pass しない）。
- Risk: 追加試験が clippy `-D warnings` に抵触 — `lint-io-
  compression` で feature matrix 全組合せを検証済み。
- Risk: docs 更新が受入完了を誤認させる — spec/IR で
  NotRun/Unsupported を分離記録。

## Task Breakdown

| Task | 内容 | 依存 |
| --- | --- | --- |
| T1 | 契約試験棚卸し・fixture/試験追加・設計正本更新・検証 | — |

## Verification

- `mise run test-io-compression` — feature matrix 全組合せで
  exit 0（追加試験を含む）。
- `mise run check-io-compression`/`lint-io-compression` —
  全 feature 組合せで `-D warnings` 通過。
- `mise run fmt-check` — workspace 整形 pass。
- `mise run test-io-e2e` — 合成境界の非回帰。
- `mise run test-rt` — RT 契約非回帰。
- `mise run test-io-msrv` — rust@1.89.0 で io-compression
  `--features deflate` 補完を含め exit 0。
- `mise run check`/`lint`/`test`（workspace）— 非回帰。
- `doc-maintenance` apply 後の `doc-status` audit で新規
  blocking finding なし。

## Open Questions / Gaps

- なし（slice 範囲は spec で確定済み。性能・Fuzz・Consumer
  実測・#288 gate・OS matrix は NotRun/Unsupported として
  記録のみ）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker 内で直接実装）
- Independent work streams: none
- User approval required before dispatch: no（Orca dispatch 済み・
  review-protocol を承認として扱う）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 追加試験が test-spec R1〜R12 と 1:1 に対応し、外部観測可能な
  契約のみを assert しているか。
- `src/` 無変更の判断が「設計不整合なし」の照合結果と整合するか。
- 設計正本の更新が snapshot/証拠索引の追記に留まり、契約内容を
  変更していないか。
