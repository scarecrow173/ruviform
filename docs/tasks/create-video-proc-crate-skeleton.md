---
id: "TASK-034JcCRLsFJ7j8zPBi8mXY"
type: "task"
status: "done"
title: "ruviform-video-proc クレート骨格の新設"
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
    - "docs/impl/ir/create-video-proc-crate-skeleton.md"
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

# ruviform-video-proc クレート骨格の新設

## Work

新規ワークスペースメンバー `crates/ruviform-video-proc` を骨格として新設する（カーネル実装は含まない — T3a/T3b が追加する。spec AC5・AC10）。

- `crates/ruviform-video-proc/Cargo.toml`: `ruviform-video-core` 同形 — `version`/`edition`/`rust-version`/`license`/`publish` は workspace 継承（`publish.workspace = true` 等）、`[lints] workspace = true`。`[dependencies]` は `ruviform-video-core = { path = "../ruviform-video-core" }` のみ（`ruviform-media-core` は `Error`/`Result` 実使用時 = T3a で追加。ADR D6-a・D7-a）
- ルート `Cargo.toml`: `members` に `"crates/ruviform-video-proc"` を video-core 直後に追加し、層内順序の説明コメントを video-proc の位置（Domain foundations のデータ型 → 処理・境界）に合わせて同期
- `crates/ruviform-video-proc/src/lib.rs`: crate doc（責務 = 色変換・スケーリング・合成の基礎処理カーネル + GPU 計算カーネル境界ポートの所有、codec・I/O 非保持。許可依存層 = Primitives + Domain foundations `ruviform-video-core`、同順位 dsp/media-io への依存禁止。RT 制約 = `process` 系は RT 適合設計（alloc-free）だが保証は API 単位の `**RT contract:**` ラベルで個別明示、初期化/prepare 系は非 RT。GPU ポート所有宣言 + trait 定義は ruviform-gpu Issue へ移管（AC10・ADR D4-b）。将来の `ruviform-rt` `Processor` trait へ impl 追加で接続する後方互換の拡張余地（ADR D1-b））+ `version()` + doctest + smoke test + `#!` allow 群（video-core 同形）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-26-video-foundation.md](../plans/implement-issue-26-video-foundation.md) — 本タスクの Phase B（B-1〜B-3）と検証 V-2 の正本
- [../specs/implement-video-foundation-api.md](../specs/implement-video-foundation-api.md) — AC5（クレート配線）・AC10（GPU ポート宣言）と In Scope のクレート新設規定
- [../adr/video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md) — D1-b（二相カーネル形状・`Processor` 拡張余地）・D4-b（GPU ポート所有宣言のみ）・D6-a（配線規則）・D7-a（media-core `Error` は実使用時のみ）
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — クレート責務表 A.2 の video-proc 行・層内順序（L174 付近）・「契約型・境界ポートの所有」節の GPU 境界 API 行（L184 付近）
- [../test-specs/video-foundation-api.md](../test-specs/video-foundation-api.md) — 契約保証の正本（`verified-by`）

## Dependencies

- [../plans/implement-issue-18-data-model.md](../plans/implement-issue-18-data-model.md) 系の完了（依存先 `ruviform-video-core` の API 表面が実装済みであること）
- 本タスクは [implement-yuv-to-rgb-kernel.md](implement-yuv-to-rgb-kernel.md)・[sync-docs-issue-26.md](sync-docs-issue-26.md) をブロックする（`relations.blocks`）

## Affected Files

- `crates/ruviform-video-proc/Cargo.toml`（新規）— workspace 継承 + `ruviform-video-core` path 依存
- `crates/ruviform-video-proc/src/lib.rs`（新規）— crate doc・`version()`・smoke test
- `Cargo.toml` — `members` 追加（video-core 直後）+ メンバーコメント同期
- `Cargo.lock` — 新メンバー・依存の自動記録
- `docs/impl/ir/create-video-proc-crate-skeleton.md`（新規）・`docs/impl/ir/README.md`（索引行追加）
- 本タスクドキュメント（本文記入・status 遷移）・`docs/tasks/README.md`（status 列同期）

## Done When

- [x] AC5: `crates/ruviform-video-proc` が workspace `members` に存在し、依存は `ruviform-video-core` のみ（`ruviform-rt`・同順位 dsp/media-io への依存なし）。`publish.workspace`/`edition`/`rust-version`/`[lints] workspace = true` を継承している
- [x] AC10: crate doc に GPU 計算カーネル境界のポート所有宣言があり、trait 定義が ruviform-gpu Issue へ移管される旨が明記されている
- [x] crate doc が責務（色変換・スケーリング・合成の基礎処理カーネル、codec・I/O 非保持）・許可依存層・RT 制約（`process` 系 = RT 適合設計だが API 単位明示まで非保証、初期化 = 非 RT）・`Processor` 拡張余地（ADR D1-b）をカバーする
- [x] `version()` + doctest + smoke test が存在し、`#!` allow 群が video-core 同形である
- [x] 公開 enum・外部クレート依存・`unsafe`・`ruviform-rt` 依存・カーネルコードがない（骨格のみ）
- [x] impl record `docs/impl/ir/create-video-proc-crate-skeleton.md` が completed で存在し、本タスクの `implemented-by` と `status: done`・`docs/tasks/README.md`・`docs/impl/ir/README.md` が同期済み

## Verification

- [x] `mise run check` — ワークスペース型チェック（新メンバー含む）が exit 0
- [x] `mise exec -- cargo test -p ruviform-video-proc` — smoke test + doctest 合格
- [x] `mise exec -- cargo clippy -p ruviform-video-proc --all-targets -- -D warnings` — 0 warnings
- [x] `mise exec -- cargo metadata --format-version 1 --no-deps` — members に video-proc、deps が video-core のみ、全パッケージの `publish` が `[]`
- [x] `mise run fmt-check` — 差分なし
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — 0 findings
- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 0 findings
