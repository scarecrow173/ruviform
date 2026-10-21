---
id: "IMPL-034JfEIKHmfeyIZETbwKfD"
type: "impl"
status: "completed"
title: "Create video-proc crate skeleton"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/26"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/create-video-proc-crate-skeleton.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/create-video-proc-crate-skeleton.md"
  implements:
    - "docs/tasks/create-video-proc-crate-skeleton.md"
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

# Create video-proc crate skeleton

## Summary

task `TASK-034JcCRLsFJ7j8zPBi8mXY`（plan `PLAN-034JcCFGe6ILSTWLgUIjAn` Phase B）として、新規ワークスペースメンバー `crates/ruviform-video-proc` を骨格のみで新設した（spec AC5・AC10）。カーネル実装は含まず、crate doc に責務・許可依存層・RT 制約・二相カーネル形状・GPU 計算カーネル境界ポートの所有宣言（trait 定義は ruviform-gpu Issue へ移管）・将来の `Processor` trait への additive 接続余地を記録した。

## Context

- 実施タスク: [../../tasks/create-video-proc-crate-skeleton.md](../../tasks/create-video-proc-crate-skeleton.md)
- 上流: [plan](../../plans/implement-issue-26-video-foundation.md) Phase B（B-1〜B-3）と検証 V-2、[spec](../../specs/implement-video-foundation-api.md) AC5・AC10 と In Scope のクレート新設規定、[ADR](../../adr/video-proc-kernel-api-and-color-hdr-scope.md) D1-b（二相カーネル形状）・D4-b（GPU ポート所有宣言のみ）・D6-a（deps = video-core のみ・`ruviform-rt`・同順位依存なし）・D7-a（media-core `Error` は実使用時のみ = T3a で追加）、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md) クレート責務表 A.2 の video-proc 行・層内順序・「契約型・境界ポートの所有」節
- 契約保証の正本: [test-spec](../../test-specs/video-foundation-api.md)

## Implementation

### Changes

- `crates/ruviform-video-proc/Cargo.toml`（新規）— `ruviform-video-core` 同形: `version`/`edition`/`rust-version`/`license`/`publish` を workspace 継承、`[lints] workspace = true`、deps は `ruviform-video-core = { path = "../ruviform-video-core" }` のみ
- `crates/ruviform-video-proc/src/lib.rs`（新規）— crate doc: 責務（色変換・スケーリング・合成の基礎処理カーネル + GPU 計算カーネル境界ポート所有、codec・demux/mux・I/O 非保持）・許可依存層（Primitives + Domain foundations `ruviform-video-core`。同順位 dsp/media-io 依存禁止・media-core は `Error`/`Result` 実使用時のみ追加・`ruviform-rt` 依存なし）・Kernel API shape（非 RT `prepare`/コンストラクタ → alloc-free `process(&mut self, src, dst)`。`RtContext` 引数・`Processor` impl なし、将来は additive に trait impl 追加）・RT constraints（`process` 系は RT 適合設計だが保証は API 単位の `**RT contract:**` ラベルで明示、init/prepare は非 RT）・GPU port 節（所有宣言 + trait 定義は ruviform-gpu Issue へ移管）+ `version()`（`**RT contract:**` ラベル付き）+ doctest + smoke test 2 本 + `#!` allow 群（video-core 同形・コメントはカーネル名向けに適応）
- `Cargo.toml`（ルート）— `members` に `"crates/ruviform-video-proc"` を video-core 直後に追加。メンバーコメントを層内順序（Domain foundations = audio-core・video-core データ型 → video-proc 等の処理・境界、同順位相互依存禁止）と Issue #26 の path 依存設定に同期
- `Cargo.lock` — `cargo check` により video-proc エントリ（deps = video-core）を自動記録

### Decisions During Implementation

- **media-core 依存は追加しない**（ADR D6-a/D7-a どおり）: `Error`/`Result` の実使用は T3a のカーネル実装から。骨格時点では未使用依存を持たない
- **依存配線の smoke test を追加**: `version_is_not_empty` の定型に加え、`video_core_dependency_is_reachable` で唯一の path 依存が解決されカーネルの入出力型（`PixelFormat` 等）が到達可能であることを確認。テスト名は `rt_` substring フィルタに非該当（`rt_` を含まない命名 — RT 契約テストは T3 系で `rt_` プレフィックス付きで追加される）
- **`#!` allow コメントの適応**: video-core の「public names echo their module」文言を将来のカーネル再エクスポート（`yuv_to_rgb::YuvToRgb` 等）向けに書き換え。`publish = false` の literal 構築ポリシーと enum `#[non_exhaustive]` 規則はそのまま転記
- **contract source の記述**: タスク指定の baseline L197/L179 参照を、行番号ではなく節名（crate table A.2 の video-proc 行・「契約型・境界ポートの所有」の GPU 境界行・Domain foundations 層内順序）で引用 — 現行ファイルでは video-proc 行は L202 付近にあり、行番号引用よりドリフト耐性があるため

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 型チェック | `mise run check`（`cargo check --workspace --all-targets`） | exit 0・video-proc を含む全メンバー clean |
| テスト | `mise exec -- cargo test -p ruviform-video-proc` | 2 unit + 1 doctest 全合格 |
| lint | `mise exec -- cargo clippy -p ruviform-video-proc --all-targets -- -D warnings` | 0 warnings |
| 配線 | `mise exec -- cargo metadata --format-version 1 --no-deps` | members に video-proc・deps は `ruviform-video-core` のみ・全 7 パッケージ `publish: []` |
| 整形 | `mise run fmt-check` | 差分なし |
| docs 監査 | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` | 0 findings |
| IR 監査 | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | 0 findings |

## Risks

- `RtContext` を要求しない `process` 形状は「RT であること」の型レベル強制を持たない — 誤用防止は `**RT contract:**` doc 契約とレビューに依存（ADR D1-b Consequences と一致）
- GPU ポート trait 不在の間は GPU 経路が未接続のまま — 宣言のみで、境界は ruviform-gpu Issue が始まるまで形骸化リスクを許容（ADR D4-b の採用済みトレードオフ）

## Follow-ups

- T3a（[implement-yuv-to-rgb-kernel.md](../../tasks/implement-yuv-to-rgb-kernel.md)）: `ruviform-media-core` path 依存追加（`Error`/`Result` 使用）+ `coef.rs`/`yuv_to_rgb.rs` カーネル実装と `lib.rs` 再エクスポート
- T3b（[implement-rgb-to-yuv-and-f16-kernels.md](../../tasks/implement-rgb-to-yuv-and-f16-kernels.md)）: `RgbToYuv`・`Rgba8 ↔ RgbaF16` カーネル
- T4（[sync-docs-issue-26.md](../../tasks/sync-docs-issue-26.md)）: baseline 採否表・`AGENTS.md`・`README.md` の video-proc 反映と総合検証
