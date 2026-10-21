---
id: "IMPL-034JftzSzGUrGW76DpIyVM"
type: "impl"
status: "completed"
title: "Document video-core ownership contract"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/26"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-video-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/frame.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/plane.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/color.rs"
      - type: "file"
        path: "docs/tasks/document-video-core-ownership-contract.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/document-video-core-ownership-contract.md"
  implements:
    - "docs/tasks/document-video-core-ownership-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-26-video-foundation.md"
    - "docs/specs/implement-video-foundation-api.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Document video-core ownership contract

## Summary

task `TASK-034JcCQQD01mBg5JPjNiAS`（plan `PLAN-034JcCFGe6ILSTWLgUIjAn` Phase A）として、`ruviform-video-core` の rustdoc に `SharedPlane`/`VideoFrame` の所有権・drop・zero-copy 契約と `**RT contract:**` ラベルを確定記述した（spec AC1–AC4）。doc-only の変更であり API の追加・挙動変更は行っていない（introspection API は既定どおり未追加 — `plane_mut() == None` を共有状態の観測点として明記）。

## Context

- 実施タスク: [../../tasks/document-video-core-ownership-contract.md](../../tasks/document-video-core-ownership-contract.md)
- 上流: [plan](../../plans/implement-issue-26-video-foundation.md) Phase A（A-1〜A-5）、[spec](../../specs/implement-video-foundation-api.md) AC1–AC4 と Requirements の RT ラベル規則、[ADR](../../adr/video-proc-kernel-api-and-color-hdr-scope.md) D5-i（現行所有モデル維持 + 契約文書化）・D3-a（HDR = メタデータ搬送 + 非破壊）、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md) 動画データモデル節・RT/非 RT 分離（Drop 退避）、[RT ADR](../../adr/realtime-contract-and-verification-policy.md)
- 契約保証の正本: [test-spec](../../test-specs/video-foundation-api.md)

## Implementation

### Changes

- `src/lib.rs` — crate doc に `## Ownership and lifecycle` 節を追加: `Clone`=`Arc` 共有（コピーではない）・`Arc::get_mut` 書込ゲート（共有中は読取専用、`plane_mut()==None` が観測点・他の introspection API は持たない）・drop=最後の `Arc` 解放で backing dealloc → RT パスで所有を drop しない・退役は非 RT 層へ送出（Drop 退避）・フレームプールは非 RT 層所有で返却・所有権移転は明示 API（後続設計）・`from_shared_planes` が唯一の zero-copy 供給経路・`SharedPlane` offset-less 既知制約。`## RT contract` 節に `Clone`/`Drop` の non-RT 規則を追記し、Contract source に本 ADR を追加。`MediaTime`/`MediaDuration`/`Error`/`Result` の intra-doc link を media-core 実パスへ修正（unresolved link warning 解消）
- `src/frame.rs` — モジュール doc に drop=dealloc → Drop 退避の一文を追加。`VideoFrame` 型 doc に `# Ownership`（clone=共有・排他所有でのみ可変・`plane_mut()==None` が共有状態の観測点）と `# Drop and frame pools`（最後の `Arc` で dealloc・RT で所有を drop しない・プールは非 RT 層所有・返却は後続設計の明示 API）を追加し、clone/drop をカバーする `**RT contract:**` ラベルを型レベルに付与。`from_shared_planes` doc を「sole zero-copy supply path」+ offset-less 制約（media-io で再評価）+ 検証規則（plane 数・`stride >= row_bytes`・`data.len() >= stride*(h-1)+row_bytes`・過剰 backing は許容し view が trim）へ精緻化。`plane_mut` doc に「in-range index で `None` = 共有中の観測シグナル」を明記
- `src/plane.rs` — モジュール doc に `SharedPlane`+`from_shared_planes` が唯一の供給経路である旨を追加。`SharedPlane` doc に offset-less 制約（plane 先頭オフセット表現不可 → 再包装が必要・media-io Issue で再評価・ADR D5-i 参照）・唯一経路・過剰 backing 許容を記録し、RT ラベルを drop の dealloc までカバー。`PlaneMut` doc の `[`VideoFrame`]` リンクを `crate::VideoFrame` 実パスへ修正（unresolved link warning 解消）
- `src/color.rs` — モジュール doc の「対応範囲は `要PoC`」記述を決定済み範囲へ更新: HDR = メタデータ搬送 + 非破壊（`ColorInfo`/`HdrMetadata` はフレームに付随して運ばれ、変換カーネルは `matrix`/`range` の code-value 演算のみ適用し `primaries`/`transfer`/`hdr` を無変更で伝播。transfer 関数適用・トーンマッピングなし）。`Unspecified` の既定解釈（matrix→Bt601・range→Limited）と出力 `matrix`/`range` の更新規則を記載し、ADR D3 を参照

### Decisions During Implementation

- **introspection API は追加しない**（plan Q1 既定どおり）: 「共有状態が観測できる」は `plane_mut() == None` を観測点として crate doc・型 doc・メソッド doc の 3 層で明記した。契約文書化に追加 API は不要と判断 — `is_shared()`/`is_exclusively_owned()` 相当は既存の観測点で表現可能かつ D5-i の最小追加条件を満たさないため
- **`clone`/`drop` の RT ラベルは `VideoFrame` 型 doc に集約**: `Clone` は derive、`Drop` は暗黙のため per-impl doc を置けない。型レベルの `**RT contract:**` で「`Clone`=refcount bump・`Drop`=最後の `Arc` で dealloc、いずれも non-RT」を規定（AC4）
- **pre-existing の unresolved intra-doc links（`MediaTime`/`MediaDuration`/`Error`/`Result`/`VideoFrame`）を併せて修正**: doc-only タスクの検証 `cargo doc` で warning 0 を達成するため、同じ doc ブロック内の壊れたリンクを実パス指定に修正
- **`要PoC` 文言の扱い**: `color.rs` は決定済み範囲へ置換（タスク要件）。`frame.rs`/`plane.rs` の「decoder mechanism は `要PoC`」は依然 media-io Issue の割当として正しいため、所在を明記して維持

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `mise run fmt-check` | 差分なし |
| テスト | `mise exec -- cargo test -p ruviform-video-core` | 23 unit + 6 doctest 全合格 |
| lint | `mise exec -- cargo clippy -p ruviform-video-core --all-targets -- -D warnings` | clean・0 warnings |
| rustdoc | `mise exec -- cargo doc -p ruviform-video-core --no-deps` | 生成成功・0 warnings（pre-existing 4 warnings を解消） |
| RT ラベル監査 | `grep -c "RT contract"` on `src/*.rs` | lib 6・frame 16・plane 19・color 9・pixel 5。全公開 API（`plane`/`plane_mut`/`row`/`row_mut`/`data`/`data_mut`/`SharedPlane`/`VideoFrame` clone/drop/確保系）をカバー |

## Risks

- `VideoFrame::clone`/`drop` の RT 規則は型 doc 上の契約であり型レベル強制ではない — 誤用防止は doc 契約とレビューに依存（`RtContext` capability による強制は `ruviform-rt` 側の責務・ADR Consequences と一致）
- `plane_mut() == None` は「共有中 or index 範囲外」の合成シグナル。doc は in-range index に限定して明記済み

## Follow-ups

- `SharedPlane` offset・`PlaneBacking`・decoder zero-copy 具体方式・フレームプールの返却/所有権移転 API は media-io 実装 Issue の要 PoC（ADR D5-i の移管どおり）
- video-proc カーネル側の `prepare`/`process` RT ラベル・`Unspecified` 既定規則の適用は T3a/T3b タスクで color.rs モジュール doc の決定範囲と整合させる
