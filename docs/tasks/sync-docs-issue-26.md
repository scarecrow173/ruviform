---
id: "TASK-034JcD43sDKFAlPxYuAWmj"
type: "task"
status: "done"
title: "Issue #26 ドキュメント同期（baseline 採否表・AGENTS・README）と総合検証"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-26-video-foundation.md"
  implemented-by:
    - "docs/impl/ir/sync-docs-issue-26.md"
  depends-on:
    - "docs/plans/implement-issue-26-video-foundation.md"
    - "docs/tasks/document-video-core-ownership-contract.md"
    - "docs/tasks/create-video-proc-crate-skeleton.md"
    - "docs/tasks/implement-yuv-to-rgb-kernel.md"
    - "docs/tasks/implement-rgb-to-yuv-and-f16-kernels.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/video-foundation-api.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #26 ドキュメント同期（baseline 採否表・AGENTS・README）と総合検証

## Work

Issue #26 着地（video-core 契約文書化 + `ruviform-video-proc` 新設 + 色変換カーネル実装）に伴うドキュメント同期と総合検証（spec AC11、plan Phase E-1/E-2/E-3、検証 V-5）。doc-only 変更でありコードは触らない。

- `docs/designs/ruviform-architecture-design-baseline.md`: 採否表の「色変換・HDR の対応範囲」行を 要PoC → 決定 へ移行（Issue #26・ADR D2-a 縮小版/D3-a の決定内容を根拠列に記録し、行は 決定 ブロックへ移動）。動画データモデル節の 要PoC 箇条から「色変換・HDR の対応範囲」を外し、決定内容を 決定 箇条へ追加（個別 codec/container・decoder zero-copy は 要PoC のまま残す）
- `docs/plans/implement-issue-26-video-foundation.md`: 歴史的記録を維持したまま、D-2・Q3 の stale な前提（primitive `f16` が MSRV 1.85 で使用可）に実績注記を追記 — `x86_64-pc-windows-msvc` では pinned 1.99.0 でも unstable（E0658・rust-lang/rust#116909）であり、出荷実装は `f16::from_f64` とビット一致のソフトウェア binary16 コーデック（T3b IR 参照）
- `AGENTS.md`: ワークスペース構成節を 6 → 7 クレートへ更新（video-proc 追加・Issue #26 新設・カーネル実装済み・video-proc → video-core・media-core の path 依存）、「公開 API 未確定」を 4 クレート以外に限定、`test-rt` 行の「RT テスト未実装」記述を実装済みへ同期
- `README.md`: 冒頭・実在リスト・未確定リスト・RT 契約段落・`crates/` 説明の5箇所を同期
- 総合検証スイート（E-3）を実行し、本タスクの Verification に結果を記録する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-26-video-foundation.md](../plans/implement-issue-26-video-foundation.md) — 本タスクの Phase E（E-1/E-2/E-3）と検証 V-5 の正本
- [../specs/implement-video-foundation-api.md](../specs/implement-video-foundation-api.md) — AC11（docs 同期 + 検証）
- [../adr/video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md) — `ADR-034JcidaiOOip5EhNPOnDI`。採否表へ転記する決定内容（D2-a 縮小版・D3-a）の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — 採否表・動画データモデル節の更新対象（approved baseline。本 Issue が決定した分類のみ移動）
- [../test-specs/video-foundation-api.md](../test-specs/video-foundation-api.md) — 契約保証の正本（`verified-by`）
- [../impl/ir/implement-rgb-to-yuv-and-f16-kernels.md](../impl/ir/implement-rgb-to-yuv-and-f16-kernels.md) — primitive `f16` 不成立とソフトウェア binary16 コーデック採用の実績記録（plan 注記の参照先）

## Dependencies

- T1（[document-video-core-ownership-contract.md](document-video-core-ownership-contract.md)）・T2（[create-video-proc-crate-skeleton.md](create-video-proc-crate-skeleton.md)）・T3a（[implement-yuv-to-rgb-kernel.md](implement-yuv-to-rgb-kernel.md)）・T3b（[implement-rgb-to-yuv-and-f16-kernels.md](implement-rgb-to-yuv-and-f16-kernels.md)）の完了（全て `done`）
- 本タスクは Issue #26 の最終タスクであり、後続タスクをブロックしない

## Affected Files

- `docs/designs/ruviform-architecture-design-baseline.md` — 採否表 1 行の分類移動 + 動画データモデル節の採否箇条同期
- `docs/plans/implement-issue-26-video-foundation.md` — D-2・Q3 への実績注記（historical record 維持の追記のみ）
- `AGENTS.md` — ワークスペース構成段落・`test-rt` 行
- `README.md` — 冒頭・実在リスト・未確定リスト・RT 契約段落・`crates/` 行
- `docs/impl/ir/sync-docs-issue-26.md`（新規）・`docs/impl/ir/README.md`（索引行追加）
- 本タスクドキュメント（本文記入・status 遷移）・`docs/tasks/README.md`（status 列同期）

## Done When

- [x] AC11: baseline 採否表の「色変換・HDR の対応範囲」行が 決定 に更新され（根拠に Issue #26・ADR の決定内容）、動画データモデル節の 要PoC 箇条から同項目が外れて 決定 箇条へ移っている。他の 要PoC/保留 項目・無関係な記述は不変
- [x] plan D-2・Q3 の primitive `f16` 前提に、windows-msvc での不成立とソフトウェア binary16 コーデック採用の実績注記が追記されている（歴史的記録の書き換えなし）
- [x] `AGENTS.md`・`README.md` のクレート一覧が `crates/ruviform-video-proc` を含む実態と一致し、「公開 API 未確定」「RT テスト未実装」等の stale 記述が同期されている
- [x] ルート `Cargo.toml` のメンバーコメントが実態と整合（変更不要を確認）
- [x] impl record `docs/impl/ir/sync-docs-issue-26.md` が completed で存在し、本タスクの `implemented-by`・status・両 README が同期済み

## Verification

- [x] `mise run fmt-check` — 差分なし
- [x] `mise run check` — ワークスペース型チェック exit 0
- [x] `mise run lint` — 0 warnings
- [x] `mise run test` — ワークスペース全テスト合格
- [x] `mise run test-rt` — video-proc の意図した 8 件を含む 18 件の `rt_` テストが選択・合格（#18 由来の 10 件を含む・誤検出なし）
- [x] `mise run bench` — ベンチマーク未実装・実質 no-op で PASS
- [x] `mise exec -- cargo metadata --format-version 1 --no-deps` — video-proc の依存辺は `ruviform-video-core` + `ruviform-media-core` のみ（`ruviform-rt`・同順位依存なし）、全パッケージ `publish: []`
- [x] `mise exec -- prek run --all-files` — 全フック Passed
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — 0 findings
- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 0 findings
