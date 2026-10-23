---
id: "PLAN-034KPgI4Qmc4o0yAzYskID"
type: "plan"
status: "approved"
title: "Issue #96 実施計画 — ruviform-edit 新設"
created: "2026-10-23"
updated: "2026-10-23"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/edit-data-model.md"
  derived-by:
    - "docs/tasks/edit-crate-and-model.md"
    - "docs/tasks/edit-evaluation-and-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/edit-data-model-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #96 実施計画 — ruviform-edit 新設

## Goal

`ruviform-edit` クレートを新設し、NLE 非破壊編集データモデル
（`Timeline`/`Track`/`Clip` + mutation）と評価射影（`evaluate` →
`Evaluation`/`ActiveClip`）を実装する。

## Scope

単一実装ストリーム — 1 クレート新設。spec/design の Out of Scope
（engine 構造体生成・undo/redo・UI・素材管理・直列化・トランジション・
再生速度変更）は含めない。

## Upstream Documents

- docs/specs/edit-data-model.md
- docs/adr/second-target-editing-model.md

## Design Inputs

- docs/designs/edit-data-model.md（approved — 型表・mutation 完全検証
  ルールセット・評価算術経路が確定済み）

## File Map

- `Cargo.toml` — workspace `members` へ `crates/ruviform-edit` 追加
- `crates/ruviform-edit/Cargo.toml` — `publish.workspace = true` 継承・
  `ruviform-media-core` path 依存のみ
- `crates/ruviform-edit/src/lib.rs` — crate doc（責務・許可依存層・
  RT 契約）+ re-export
- `crates/ruviform-edit/src/model.rs` — `Timeline`/`Track`/`Clip`/`ClipSpec`/
  `TrackKind`/`ClipAttributes`/`TrackId`/`ClipId`/`ClipSource` + mutation
- `crates/ruviform-edit/src/eval.rs` — `Evaluation`/`ActiveClip`/`evaluate`
- `crates/ruviform-edit/src/error.rs` — `Error`/`Result`
- `crates/ruviform-edit/README.md` — crate README（責務・許可依存層・RT 制約）
- `crates/README.md` — 索引行追加

## Implementation Sequence

- [x] 1. クレート骨格 — `crates/ruviform-edit` 新設。Cargo.toml は workspace
  継承（`version`/`edition`/`rust-version`/`license`/`publish`/`lints`）、
  依存は `ruviform-media-core` path のみ（audio-core/video-core は spec で
  許可されているが、最小属性セットでは media-core 時間型が主依存 —
  実需が出た時点で追加する YAGNI 規約）。workspace `members` 登録。
  `lib.rs` crate doc に責務（データ + 評価のみ）・許可依存層・RT 契約
  （全 API 非 RT — `evaluate` の確保を含み RT パスからの呼出し禁止）を明記
- [x] 2. データモデル型 + mutation — `error.rs`（`#[non_exhaustive]` `Copy`
  `Error`/`Result`）→ `model.rs`（ID newtype・`TrackKind`・`ClipAttributes`・
  `Clip`/`ClipSpec`/`Track`/`Timeline` + accessor + `add_track`/`remove_track`/
  `insert_clip`/`remove_clip`/`move_clip`/`trim_clip`/`enabled`）。全 mutation で
  design の完全検証ルールセットを再実行（正 duration・`checked_end` 厳密性と
  base-collapse・`range.duration() <= source_range.duration()`・kind 整合・
  属性サニティ・`*NotFound`）と `range` の timeline base 正規化を集約
- [x] 3. 評価射影 — `eval.rs`。`Timeline::evaluate(at)` が design の算術経路
  （`at` の 1 回変換・半開包含・後開始/同開始後挿入優先・`source_time` 写像・
  到達可能残区間・`TimeOverflow`）で `Evaluation`/`ActiveClip` を返す
- [x] 4. テスト — test-spec 全ケース（mutation・検証拒否・評価規則・accessor）
- [x] 5. 文書クローズ — crate README・`crates/README.md` 索引行・検証列・
  doc 監査・チェーン frontmatter 更新

## Dependencies and Risks

- `ruviform-media-core` の時間型（`MediaTime`/`MediaDuration`/`MediaRange`/
  `TimeBase`/`Rounding`）の既存 API（`convert`/`checked_end`/`checked_add`・
  cross-base 比較）をそのまま利用 — media-core 本体の変更なし
- engine（`ruviform-audio`/`ruviform-video`）・他 foundations への依存禁止 —
  射影の中立性を依存構造で保証する spec 要件

## Task Breakdown

- docs/tasks/edit-crate-and-model.md — 手順 1–2（骨格 + データモデル型・mutation）
- docs/tasks/edit-evaluation-and-closeout.md — 手順 3–5（評価 + テスト +
  文書クローズ。edit-crate-and-model に依存）

## Verification

- [x] test-spec 全ケースパス（`mise run test`）
- [x] `mise run fmt-check`/`check`/`lint` パス
- [x] 依存が `ruviform-media-core` のみ（engine・他 foundations なし —
  Cargo.toml 確認）
- [x] doc 監査チェーン起因 findings 0（`doc_status.js audit`）

## Open Questions / Gaps

- なし — design の Deferred Design Concerns（トラック属性拡張・速度変更・
  区間評価・直列化等）は本計画の範囲外として確定済み

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（task 2 件・依存直列のため分割不要）
- Independent work streams: none（edit-evaluation-and-closeout は
  edit-crate-and-model に依存）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- mutation の完全検証ルールセット（base-collapse の range/source_range 両側・
  cross-base 正確比較）と評価の丸め回数固定（`at` 1 回 + `checked_add` 最大
  2 回）がレビュー焦点
