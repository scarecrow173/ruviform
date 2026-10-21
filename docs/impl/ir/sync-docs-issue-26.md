---
id: "IMPL-034JygdA2K2Y2qhRHaQZGP"
type: "impl"
status: "completed"
title: "Sync docs Issue 26"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/26"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/sync-docs-issue-26.md"
    modified:
      - type: "file"
        path: "docs/designs/ruviform-architecture-design-baseline.md"
      - type: "file"
        path: "docs/plans/implement-issue-26-video-foundation.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "docs/tasks/sync-docs-issue-26.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/sync-docs-issue-26.md"
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
    - "docs/impl/ir/implement-rgb-to-yuv-and-f16-kernels.md"
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

# Sync docs Issue 26

## Summary

task `TASK-034JcD43sDKFAlPxYuAWmj`（plan `PLAN-034JcCFGe6ILSTWLgUIjAn` Phase E）として、Issue #26 着地に伴うドキュメント同期と総合検証（spec AC11）を実施した。設計ベースライン採否表の「色変換・HDR の対応範囲」行を 要PoC → 決定 に移行し、動画データモデル節の採否箇条を同期。plan の stale な前提（primitive `f16` は MSRV 1.85 で使用可）に実績注記を追記し、`AGENTS.md`・`README.md` のクレート一覧を `ruviform-video-proc` 新設後の実態へ同期した。doc-only の変更でありコードは不変。総合検証スイート（fmt-check・check・lint・test・test-rt・bench・cargo metadata・prek・docs audit）は全て PASS。

## Context

- 実施タスク: [../../tasks/sync-docs-issue-26.md](../../tasks/sync-docs-issue-26.md)
- 上流: [plan](../../plans/implement-issue-26-video-foundation.md) Phase E（E-1/E-1b/E-2/E-3）と検証 V-5、[spec](../../specs/implement-video-foundation-api.md) AC11、[ADR](../../adr/video-proc-kernel-api-and-color-hdr-scope.md) D2-a 縮小版/D3-a（採否表へ転記する決定内容の正本）、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md)（approved — 本 Issue が決定した分類のみ移動）
- 契約保証の正本: [test-spec](../../test-specs/video-foundation-api.md)
- 前提: T1–T3b 全タスク `done`。primitive `f16` 不成立の実績記録は [implement-rgb-to-yuv-and-f16-kernels.md](implement-rgb-to-yuv-and-f16-kernels.md)（T3b IR）
- 制約: doc-only・git commit なし・baseline は approved 文体を維持・spec の `status` は `approved` のまま（spec→implemented 遷移は別ライフサイクルステップ）

## Implementation

### Changes

- `docs/designs/ruviform-architecture-design-baseline.md` — 採否表の「色変換・HDR の対応範囲」行を 要PoC ブロックから除去し、決定 ブロック末尾へ `決定` 行として移動（根拠列: Issue #26・video-proc ADR D2-a 縮小版/D3-a で確定 — `Yuv*p ↔ Rgba8`・`Rgba8 ↔ RgbaF16` スカラーカーネル、matrix Bt601/Bt709/Bt2020 × range Limited/Full・chroma resampling 込み、HDR = メタデータ搬送 + 非破壊で transfer 適用・トーンマップなし）。動画データモデル節の採否ブロックでは 要PoC 箇条から同項目を除去し 決定 箇条へ追加（個別 codec/container・decoder zero-copy は 要PoC のまま）。他の要PoC/保留行・「後続実装 Issue の提案一覧」の 動画基盤 Issue 記述等の無関係箇所は不変
- `docs/plans/implement-issue-26-video-foundation.md` — D-2 と Q3 に実績注記を追記（歴史的記録は書き換えず付記のみ）: primitive `f16` は `x86_64-pc-windows-msvc` で pinned ツールチェーン 1.99.0 時点でも unstable（E0658・rust-lang/rust#116909）であり、出荷実装は `f16::from_f64` と全入力ビット一致のソフトウェア IEEE 754 binary16 コーデック — LE binary16 ワイヤ契約は不変、T3b IR 参照
- `AGENTS.md` — ワークスペース構成段落を 6 → 7 クレートへ更新（`crates/ruviform-video-proc` を Domain foundations に追加、Issue #26 新設・色変換カーネル 4 種実装済み、video-proc → video-core・media-core の path 依存を記録）、「上記3クレート以外の公開 API は引き続き未確定」を「上記4クレート以外」へ訂正、`test-rt` 行の「現時点では RT テスト未実装」を video-proc `rt_` テスト実装済みへ同期
- `README.md` — 冒頭説明・実在リスト（Phase 1 基盤6クレート→7クレート）・未確定リスト（確定 API に video-proc カーネルを追加）・RT 契約段落（RT ラベル付与対象に video-proc カーネル API を追加）・`crates/` 行の5箇所を同期
- `docs/tasks/sync-docs-issue-26.md` — Work/Upstream/Dependencies/Affected Files/Done When/Verification 記入・`status: done`・`implemented-by` 設定、`docs/tasks/README.md` の status 列を `doc_maintenance.js apply` で再生成、`docs/impl/ir/README.md` に索引行を追加
- ルート `Cargo.toml` — メンバーコメントが既に実態どおり（`video-proc` メンバー・依存方向記述）のため変更なし

### Decisions During Implementation

- **採否表の行は 要PoC ブロック内で書き換えず 決定 ブロックへ移動**: 表は分類でグルーピングされているため、分類変更は行の物理移動を伴うのが構造整合的 — 行は 決定 ブロック末尾（`スカラー参照実装必須` 行の直後）に置き、次フォーラム列は「動画基盤 Issue（実装済み）」として決定・実装のフォーラムが同一 Issue だったことを記録
- **plan の stale 前提は注記で訂正し本文は保持**: plan は approved の歴史的記録なので、D-2・Q3 の既定文はそのまま残し「実績注記」として不成立理由（windows-msvc での E0658）と代替実装（ソフトウェア binary16 コーデック）を付記 — Risk Register の「f16 は primitive `f16`」記述も同じ前提を持つが、注記の重複を避け D-2/Q3 のみへの付記に留めた
- **README の RT 契約段落は事実訂正のみ**: 「`Processor`/`RtContext` を用いるランタイム API の実装は未着手」は video-proc 実装後も真（カーネルは `RtContext` 非依存の二相形状）なので、ラベル付与対象の列挙のみ更新
- **`test-rt` 行の「RT テスト未実装」記述を同期**: `mise run test-rt` が video-proc の 8 件（+ 既存の #18 由来 10 件）を選択する実態と食い違っていたため訂正 — 「API 未確定」系の stale 記述の一環として範囲内で処理

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64（`x86_64-pc-windows-msvc`、rustc 1.99.0）・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `mise run fmt-check` | PASS・差分なし |
| 型チェック | `mise run check` | PASS・exit 0 |
| lint | `mise run lint` | PASS・0 warnings |
| テスト | `mise run test` | PASS・129 unit + 62 doctest = 191 合格・0 失敗 |
| RT レーン | `mise run test-rt` | PASS・18 件選択（audio-core 3・media-core 4・video-core 3・video-proc 8）— video-proc の意図した 8 件（rgb_to_yuv 2・rgba_f16 4・yuv_to_rgb 2）を含み誤検出なし |
| ベンチ | `mise run bench` | PASS（ベンチマーク未実装・実質 no-op） |
| 配線 | `mise exec -- cargo metadata --format-version 1 --no-deps` | video-proc deps = `ruviform-media-core` + `ruviform-video-core` のみ（`ruviform-rt`・同順位依存なし）・全 7 パッケージ `publish: []` |
| prek | `mise exec -- prek run --all-files` | 全フック Passed（windows-names は no files で Skipped） |
| docs 監査 | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` | 0 findings（65 files） |
| IR 監査 | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | 0 findings（24 files） |

## Risks

- baseline・plan は approved 記録のため、今回の分類移動・注記は最小限の追記に留めた — 採否表の「次のフォーラム」列が「動画基盤 Issue（実装済み）」となり他行の「実装 Issue（…）」形式とやや異なるが、決定と実装が同一 Issue で完結した事実の表現として許容
- ルート `Cargo.toml` のメンバーコメントは「Issue #26 で video-proc → video-core を設定済み」とのみ記し media-core 依存を列挙していない（不正確ではないが不完全）— brief の「accurate なら変更しない」指示に従い据え置き、別途報告事項とする
- Risk Register 3 の「f16 は primitive `f16`（追加依存なし）」は注記未付与の stale 記述として残る — D-2/Q3 の注記で代替実装は記録済み（歴史的記録の最小注記方針）

## Follow-ups

- spec `SPEC-034JcicHlRidBgcK1wPtj2` の `status` 遷移（approved→implemented）は別ライフサイクルステップの責務（plan の範囲外規定どおり）
- `mise run test-rt` の substring フィルタ精度限界（Issue #18 IR で記録済みの既知制約）は継続 — 現状の選択集合に誤検出なし
- Issue #41 系の残存論点（`SharedPlane` offset 拡張・フレームプール・`Processor` trait 接続時の `ruviform-rt` 依存追加）は media-io・RT 基盤・gpu 各 Issue 側で処理
