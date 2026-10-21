---
id: "TASK-034JcCQQD01mBg5JPjNiAS"
type: "task"
status: "done"
title: "video-core の所有権・drop・zero-copy 契約と RT ラベルの文書化"
created: "2026-10-20"
updated: "2026-10-20"
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
    - "docs/impl/ir/document-video-core-ownership-contract.md"
  depends-on:
    - "docs/plans/implement-issue-26-video-foundation.md"
  blocks:
    - "docs/tasks/implement-yuv-to-rgb-kernel.md"
    - "docs/tasks/sync-docs-issue-26.md"
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

# video-core の所有権・drop・zero-copy 契約と RT ラベルの文書化

## Work

`ruviform-video-core` の rustdoc に `SharedPlane`/`VideoFrame` の所有権・drop・zero-copy 契約と `**RT contract:**` ラベルを確定記述する（doc-only。API 挙動の変更・追加は行わない — introspection API は既定追加なしで、`plane_mut() == None` を共有状態の観測点として doc に明記する。plan Open Question Q1 の既定どおり）。

- `src/lib.rs` crate doc に所有権モデルを集約: `VideoFrame` の `Clone` は `Arc` 参照の共有（コピーではない）・`Arc::get_mut` 書込ゲート（共有中は読取専用、排他所有時のみ `plane_mut` が `Some`）・`VideoFrame` の drop は最後の `Arc` 解放で backing を dealloc するため RT パスで所有を drop しない（退役フレームは非 RT 層へ送出 = Drop 退避）・フレームプールは非 RT 層が所有し返却・所有権移転は明示的 API として後続設計（現行契約は「共有 backing の refcount 管理 + 排他時のみ可変」まで）
- `src/frame.rs`: `VideoFrame` 型 doc を精緻化 — `plane_mut() == None` を共有状態の観測点として明記（AC1）、drop/pool 契約（AC2）、`clone`/`drop` をカバーする `**RT contract:**` ラベル（AC4）。`from_shared_planes` の検証規則（plane 数・`stride >= row_bytes`・`data.len() >= stride*(h-1)+row_bytes`）と唯一の外部供給経路であることを明記（AC3）
- `src/plane.rs`: `SharedPlane` doc に offset-less 既知制約（plane 先頭オフセット表現不可 → media-io Issue で再評価）と供給経路の唯一性を記録。借用・共有系 API（`plane`/`plane_mut`/`row`/`row_mut`/`data`/`data_mut`/`SharedPlane`/`VideoFrame::clone`/drop/確保系）の `**RT contract:**` ラベルが alloc/dealloc 挙動をカバーすることを監査（AC4）
- `src/color.rs`: モジュール doc の「対応範囲は要PoC」記述を決定済み範囲（HDR = メタデータ搬送 + 非破壊。transfer 関数適用・トーンマッピングなし）へ更新し ADR を参照する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-26-video-foundation.md](../plans/implement-issue-26-video-foundation.md) — 本タスクの Phase A（A-1〜A-5）の正本
- [../specs/implement-video-foundation-api.md](../specs/implement-video-foundation-api.md) — AC1–AC4（本タスクの受け入れ条件）と Requirements の RT ラベル規則
- [../adr/video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md) — D5-i（現行所有モデル維持 + 契約文書化）・D3-a（HDR = メタデータ搬送 + 非破壊）の決定
- [../test-specs/video-foundation-api.md](../test-specs/video-foundation-api.md) — doc が保証する契約保証の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — 動画データモデル節・RT/非 RT 分離（Drop 退避）・video-core 責務行
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — `**RT contract:**` ラベルの API 単位保証規則

## Dependencies

- [../plans/implement-issue-18-data-model.md](../plans/implement-issue-18-data-model.md) 系の完了（`VideoFrame`/`SharedPlane`/`ColorInfo` の API 表面が実装済みであること）
- 本タスクは [implement-yuv-to-rgb-kernel.md](implement-yuv-to-rgb-kernel.md)・[sync-docs-issue-26.md](sync-docs-issue-26.md) をブロックする（`relations.blocks`）

## Affected Files

- `crates/ruviform-video-core/src/lib.rs` — crate doc に所有権モデル集約節を追加
- `crates/ruviform-video-core/src/frame.rs` — モジュール doc・`VideoFrame` 型 doc・`from_shared_planes`/`plane_mut` doc
- `crates/ruviform-video-core/src/plane.rs` — モジュール doc・`SharedPlane` doc
- `crates/ruviform-video-core/src/color.rs` — モジュール doc の要PoC記述更新
- `docs/impl/ir/document-video-core-ownership-contract.md`（新規）・`docs/impl/ir/README.md`（索引行追加）
- 本タスクドキュメント（本文記入・status 遷移）・`docs/tasks/README.md`（status 列同期）

## Done When

- [x] AC1: `Clone`=`Arc` 共有・`Arc::get_mut` 書込ゲート・`plane_mut()==None` が共有状態の観測点であることが doc に規定されている
- [x] AC2: drop=backing dealloc・RT パスで所有を drop しない（Drop 退避）・フレームプールの方向（非 RT 層所有・返却は明示 API＝後続設計）が doc に規定されている
- [x] AC3: `from_shared_planes` が唯一の外部供給経路として検証規則とともに明示され、`SharedPlane` の offset-less 制約が既知の制限として記録されている
- [x] AC4: 借用・共有・確保系 API（`plane`/`plane_mut`/`row`/`row_mut`/`data`/`data_mut`/`SharedPlane`/`VideoFrame::clone`/drop/確保系）に alloc/dealloc 挙動をカバーする `**RT contract:**` ラベルがある
- [x] `color.rs` モジュール doc が決定済み HDR 範囲（メタデータ搬送 + 非破壊、ADR 参照）に更新されている
- [x] API 追加なし（introspection 既定方針どおり `plane_mut()==None` を観測点として使用）。追加した場合は根拠を IR に記録
- [x] impl record `docs/impl/ir/document-video-core-ownership-contract.md` が completed で存在し、本タスクの `implemented-by` と `status: done`・`docs/tasks/README.md`・`docs/impl/ir/README.md` が同期済み

## Verification

- [x] `mise exec -- cargo test -p ruviform-video-core` — 23 unit + 6 doctest 合格
- [x] `mise exec -- cargo clippy -p ruviform-video-core --all-targets -- -D warnings` — 0 warnings
- [x] `mise exec -- cargo doc -p ruviform-video-core --no-deps` — doc 生成成功・0 warnings
- [x] `mise run fmt-check` — 差分なし
- [x] `grep -n "RT contract" crates/ruviform-video-core/src/*.rs` で借用・共有・確保系 API のラベル網羅を確認（lib 6・frame 16・plane 19・color 9・pixel 5）
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — 0 findings
- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 0 findings
