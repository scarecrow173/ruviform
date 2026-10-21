---
id: "IMPL-034JCku503rJpSOXR7XpSc"
type: "impl"
status: "completed"
title: "RT 基盤ドキュメント同期と総合検証"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/sync-rt-foundation-docs.md"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-exec/src/lib.rs"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/tasks/sync-rt-foundation-docs-and-verify.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/sync-rt-foundation-docs-and-verify.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/implement-rt-snapshot-exchange.md"
    - "docs/tasks/implement-exec-thread-priority.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-rt-foundation.md"
    - "docs/designs/rt-foundation-api-design.md"
    - "docs/adr/rt-foundation-primitives-implementation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-channel-and-snapshot-contracts.md"
    - "docs/test-specs/rt-context-and-processor-contract.md"
    - "docs/test-specs/thread-priority-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# RT 基盤ドキュメント同期と総合検証

## Summary

plan `PLAN-034JxNegJktSAJZ2LuO0N8` Phase F（タスク 6/6・最終）。rt
プリミティブ実装（タスク 1–4）と exec スレッド優先度機構（タスク 5）の
完了を受け、crate doc・AGENTS.md・README.md・ルート `Cargo.toml`
コメント・`mise.toml` コメントを実装済み状態へ同期し、規約監査
（`#[non_exhaustive]`・`**RT contract:**` ラベル・doctest・`rt_` 命名・
`unsafe`・外部型非露出）とワークスペース総合検証スイートを全件グリーンで
完了した。spec `SPEC-034Jcr3dOT5Uxp4znj42II` の Acceptance Criteria 15 項・
design `DESIGN-034JgsK9khgDsPqG04Sp6i` の不変条件 1–13 をすべて確認。

## Context

task `TASK-034JxObKHc0v3kUhnQiePv`
（`sync-rt-foundation-docs-and-verify.md`）。T1–T5 で `ruviform-rt`
全 API（error/context/processor/spsc/retire/snapshot/clock、deps
`rtrb` 0.4 + `arc-swap` 1）と `ruviform-exec` の
`thread_priority.rs`（dep `audio_thread_priority` 0.38
`default-features = false`）が実装済み。本タスクはドキュメント同期と
監査・検証のみ — 実装コードの挙動変更なし。

## Implementation

### Changes

- `crates/ruviform-rt/src/lib.rs`（crate doc 最終化）: 責務列に
  「fixed-capacity, pre-allocated handoff structures」を追記（spec の
  「事前確保型受け渡し構造」の wording に合わせる）。Allowed dependency
  layers の external-crates 例を `(e.g. rtrb)` から実態どおり
  `(`rtrb`, `arc-swap`)` へ更新。RT contract 節（全公開 API に
  `RT-suitable` / `non-RT only` の `**RT contract:**` ラベル）と
  Contract source（baseline + RT 契約 ADR + 実装方式 ADR）は T1–T4 で
  実装済み表記になっていたため確認のみ。
- `crates/ruviform-exec/src/lib.rs`（crate doc 最終化）: RT constraints
  節に「全公開 API が `**RT contract:**` ラベルを持つ（値型・version は
  `RT-suitable`、優先度機構は `non-RT only`）」の一文を追加し
  house style（media-core/rt の RT contract 節）と揃えた。thread_priority
  責務記述・「Holds no RT primitives」・Contract source（baseline +
  実装方式 ADR）は T5 で実装済み表記になっていたため確認のみ。
- `AGENTS.md`: ワークスペース構成の「math・rt・exec は引き続きスケルトン
  （rt は `RtContext` のみ実装済み）」を実装済み状態へ更新（rt・exec を
  スケルトンから外し Issue #21 の実装内容を明記、math のみスケルトン継続、
  「上記3クレート以外」を「上記5クレート以外」へ）。`test-rt` 表行の
  「現時点では RT テスト未実装」注記を実装済み（Issue #18・#21 の `rt_`
  テスト）へ更新。
- `README.md`: 冒頭の実装済み括弧書き・Status 節のクレート一覧行・
  「決定済み・未確定」の未確定行・RT 契約段落（「`Processor`/`RtContext`
  を用いるランタイム API の実装は未着手です」→ Issue #21 で実装済み）・
  Repository layout の `crates/` 行の計 5 箇所を同期。
- `Cargo.toml`: members 直上コメントの「Phase 1 基盤スケルトン
  （Issue #17）」を現状態（実装済み 5 クレート + math のみスケルトン）へ
  更新。
- `mise.toml`（task Affected Files 外・機会修正）: test-rt/bench 直上
  コメントの「No RT tests or benchmarks exist at this point」が Issue #18
  時点から既に陳腐化していたため、RT テスト実在・bench 未実装の現状態へ
  修正。
- 規約監査（全件パス、詳細は Validation）: 4 公開 enum の enum レベル
  `#[non_exhaustive]`、全公開 API の `**RT contract:**` ラベル + doctest、
  非 RT 識別子の `rt_` 部分文字列非含有、test-scoped allocator 以外の
  `unsafe` 不在、`rtrb`/`arc-swap`/`audio_thread_priority` の外部型非露出。

### Decisions During Implementation

- `mise.toml` コメントは task の Affected Files に未記載だが、「No RT
  tests exist」が虚偽記述のまま残ると AGENTS.md の更新と矛盾するため
  doc-sync スコープ内の最小修正として扱った（コメントのみ、タスク定義は
  不変更）。
- 外部型非露出は `cargo doc` ではなく公開シグネチャレビュー（全 `pub`
  アイテム列挙 + `rtrb`/`arc_swap`/`audio_thread_priority` の grep ヒットが
  private フィールド・`use`・関数本体・doc コメントのみである確認）で実施
  — task は `cargo doc` / レビューの選択式であり、シグネチャレビューは
  同等以上のカバレッジを持つ。
- `cargo test --workspace rt_` は name フィルタ指定時に doctest ステージを
  実行しない（Rust 1.99 / edition 2024 のマージ doctest 動作）ことを実測で
  確認。`--doc` 指定での選択集合も別途確認し、`rt_` を含む doctest 名は
  media-core の `convert_exact` ×2 のみ（Issue #18 記録済みの既知制約、
  test-rt レーンには混入しない）。

## Related Experiments

### Adopted

なし — ドキュメント同期と監査タスクであり、実験的な方式選択はない。

### Rejected

なし。

## Validation

実施環境: Windows x64・Git Bash・mise pin の Rust 1.99.0。

規約監査:

- 公開 enum（`grep -rn "pub enum" crates/ruviform-rt/src
  crates/ruviform-exec/src`）: `Error`（error.rs:30）・`TryPushError<T>`
  （spsc.rs:285）・`TryPopError`（spsc.rs:341）・`PriorityError`
  （thread_priority.rs:202）の 4 件 — すべて直上に enum レベル
  `#[non_exhaustive]`（error.rs:28・spsc.rs:283・spsc.rs:339・
  thread_priority.rs:200）。
- `**RT contract:**` ラベル: 両クレート合計 64 箇所。全公開 API（trait
  `Processor`/`RtProcessor`・`RtContext`/`MasterClock`・`channel`/
  `Producer`/`Consumer`/`TryPushError`/`TryPopError`・`retire_channel`/
  `Retirer`/`DropCollector`・`slot`/`SnapshotWriter`/`SnapshotReader`/
  `SnapshotGuard`・`Error`/`Result`・`version()`・`ThreadPriorityParams`/
  `promote_*`/`demote_*`/`ThreadPriorityGuard`/`PriorityError`）にラベル +
  `# Examples` doctest あり（`cargo test` で全 doctest 合格）。
- `rt_` 命名: `grep -rn "fn assert_\|fn start_\|fn insert_" crates/` は
  0 件。テストヘルパーは `expect_send`/`expect_send_sync`/`flagged`/`base`/
  `pts` — `rt_` 部分文字列なし。`cargo test --workspace rt_` の選択集合は
  29 単体テストで全件 `rt_*`（audio-core 3・media-core 4・rt 19・
  video-core 3。exec 0・math 0）。doctest 側の `rt_` 含有は
  `--doc` 実行で media-core `convert_exact` ×2 のみ（公開 API 名
  `conve-rt_-exact` 由来の既知制約 — Issue #18 IR 記録済み。name フィルタ
  指定では doctest ステージ自体が走らないため test-rt レーンへは混入しない）。
- `unsafe`（`grep -rn "unsafe"` 両 src）: `alloc_counter.rs` の
  `#[expect(unsafe_code)]` + SAFETY コメント付き test-scoped counting
  allocator（`#[cfg(test)]` mod）のみ。他ゼロ。
- 外部型非露出（`grep -rn "rtrb\|arc_swap\|audio_thread_priority"` 両 src）:
  ヒットは doc コメント・`use`・private フィールド（`inner:
  rtrb::Producer<T>`、`handle: Option<audio_thread_priority::
  RtPriorityHandle>` 等）・関数本体のみ。全 `pub` シグネチャのレビューで
  外部型・エラー・feature 名の露出なし。

検証スイート（全件グリーン）:

| Check | コマンド | 結果 |
| --- | --- | --- |
| fmt-check | `mise run fmt-check` | PASS（差分なし） |
| check | `mise run check` | PASS |
| lint | `mise run lint` | PASS・全 6 クレート 0 warnings（`-D warnings` ゲート下） |
| test | `mise run test` | PASS・単体 115（audio 32・exec 3・math 1・media 30・rt 26・video 23）+ doctest 97（audio 20・exec 7・math 1・media 24・rt 39・video 6）= 212 合格・0 失敗 |
| test-rt | `mise run test-rt` | PASS・29 `rt_*` 単体テストのみ選択・全合格（exec 0 件選択 — `rt_` 非含有命名どおり） |
| bench | `mise run bench` | PASS（ベンチマーク未実装・入口として成功） |
| prek | `mise exec -- prek run --all-files` | 全フック Passed（1 件は no-files Skipped） |
| metadata | `mise exec -- cargo metadata --format-version 1 --no-deps` | 全 6 パッケージ `publish` = `[]`。rt の依存に `rtrb` ^0.4・`arc-swap` ^1、exec に `audio_thread_priority` ^0.38・`uses_default_features: false`・features `[]` と記録 |
| rt_ 選択目視 | `mise exec -- cargo test --workspace rt_` | 選択 29 件は全て `rt_*` RT 契約テスト。非 RT 単体テスト・doctest の混入なし |

design 不変条件 1–13（`rt-foundation-api-design.md`）と spec Acceptance
Criteria 15 項の全件確認済み — 詳細は当該文書と各実装 IR
（`ruviform-rt.md`・`spsc.md`・`retire.md`・`snapshot.md`・
`thread-priority.md`）を参照。

## Risks

- `cargo test --workspace rt_` の部分文字列フィルタは精度限界を持つ既知
  制約（Issue #18 IR 記録）: 公開 API 名 `convert_exact` の doctest 名が
  `rt_` を含む。現行では name フィルタ指定時に doctest ステージが実行
  されないため実害なし。厳密化は RT 検証 Issue 側の検討事項。
- `alloc_counter.rs` の `#[expect(unsafe_code)]` は design 不変条件 11 が
  想定した「局所許可も要求しない」からの唯一の逸脱 — spec が許容した
  std のみ test-scoped counting allocator は `unsafe impl GlobalAlloc` を
  必然とし、SAFETY コメント付きの局所許可として規約内に収まる。

## Follow-ups

- Issue #21（全 6 タスク）の完了。後続は engines・audio-device・
  オフライン実行・RT 検証の各 Issue（design の Out of Scope / spec の
  Deferred Design Concerns に記録済み）。
- `test-rt` フィルタの厳密化（プレフィックスマッチまたはレーン分離）は
  RT 検証 Issue で検討（Issue #18 IR からの継続フォローアップ）。
