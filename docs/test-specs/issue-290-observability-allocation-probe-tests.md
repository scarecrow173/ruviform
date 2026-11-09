---
id: "TSPEC-034UoYH3LGuoCYBYsd5dXf"
type: "test-spec"
status: "approved"
title: "Issue #290 allocation-probe — CountingAllocator/AllocationScope 契約試験"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
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
  related:
    - "docs/plans/implement-issue-290-observability-allocation-probe.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-290-observability-allocation-probe.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-290-observability-allocation-probe.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #290 allocation-probe — CountingAllocator/AllocationScope 契約試験

## Purpose

`allocation-probe` feature が提供する `CountingAllocator` と
`AllocationScope` が、Issue #290 の安全条件（GlobalAlloc 契約維持・
allocator 経路の no-alloc/no-panic・TLS Scope のスレッド限定性）を
実装レベルで破らないことを保証する。将来の変更で計数・委譲・
スコープ意味論が静かに壊れた場合に検出するための契約であり、
機能撤去または上位仕様の supersede 時に廃止を検討する。

## Feature

開発・テスト用の計数アロケータと TLS 計測ウィンドウが、
`GlobalAlloc` の unsafe 契約とスレッドローカル計測の意味論を
正しく維持する。

## Rules

- Rule: `CountingAllocator` は 4 つの `GlobalAlloc` API
  （`alloc`/`alloc_zeroed`/`dealloc`/`realloc`）を全て内側 `A` の
  対応 API へ委譲し、default impl の分解（`alloc_zeroed`→`alloc`、
  `realloc`→`alloc`+`copy`+`dealloc`）に依存しない。
- Rule: `GlobalAlloc` の unsafe 契約を維持する — `layout` の
  non-zero size・align、null=失敗、realloc 失敗時の旧ブロック生存、
  dealloc の ptr/layout 一致。zero-size layout は契約上 UB のため
  有効入力として扱わない。
- Rule: 計数は飽和（u64 saturation）で、allocator 経路内で
  alloc・panic・tracing・blocking lock・再帰を起こさない。
  TLS アクセスは const 初期化 + `try_with` でスレッド終了時に
  「計数しない」へ縮退する。
- Rule: `AllocationScope` は呼出スレッドのみを計数し、ネスト時は
  全 enclosing scope が独立に計数する。ウィンドウは LIFO でなくても
  安全に unlink され、`mem::forget` は dangling を生じない
  （scope が再利用不可になるだけ）。
- Rule: `current_bytes`/`peak_bytes` は要求ベースの net live
  バイト数を追跡し、realloc は成功時のみ `new_size - old_size`
  を反映する。

## Examples

- Example: `alloc` 成功 — `CountingAllocator::<ProbeAlloc>` に
  `Layout::from_size_align(64, 8)` で `alloc` を呼ぶと、
  `alloc.calls == 1`・`alloc.succeeded == 1`・`alloc.bytes == 64`・
  `current_bytes == 64` となり、内側 `ProbeAlloc` の `alloc` が
  1 回呼ばれる。
- Example: `alloc` 失敗 — 巨大 layout で null が返ると
  `alloc.failed == 1`・`current_bytes` 不変・旧状態を保持する
  （`System` inner 使用時）。
- Example: `alloc_zeroed` — zero 化済み領域が返り、
  `alloc_zeroed` op として別計数され `alloc` op には計上されない。
- Example: `realloc` 成功/失敗 — 成功時は `realloc.calls == 1`・
  `current_bytes += new_size - old_size`、失敗時は `failed == 1` で
  `current_bytes` 不変かつ旧ブロックが dealloc 可能なまま。
- Example: TLS Scope ネスト — 外側 scope と内側 scope を enter した
  状態で割当を行うと、両 scope の `alloc.calls` が増える
  （内側 ⊂ 外側）。
- Example: TLS Scope 同時スレッド — thread A の scope が有効な間に
  thread B で割当を行っても thread A の scope カウンタは増えない
  （B 側に scope がなければ global stats のみ増える）。
- Example: 非 LIFO drop — scope1・scope2 の window を scope2→scope1
  の順（逆順）で drop してもチェーンが正しく除去され、以後の割当が
  どちらの scope にも計上されない。
- Example: スレッド終了 — window を `mem::forget` したスレッドが
  終了してもプロセスは panic/crash せず、他スレッドの計測に影響しない。
- Example: no-alloc — mock inner（`ProbeAlloc`）を使い
  `assert_no_alloc` 内で `CountingAllocator` の 4 API と
  `scope.enter()`/window drop を実行しても割当が検出されない。

## Guarantees

- `CountingAllocator` の計数値（calls/succeeded/failed/bytes/
  current_bytes/peak_bytes）は内側 `A` の実挙動と一致し、
  default impl 分解による二重計数・イベント消失がない。
- `AllocationScope` の計数はそのスレッドの window 区間に限定され、
  他スレッド・window 外の割当を含まない。
- allocator 経路と window enter/exit は alloc・panic・lock・
  tracing を含まない（`assert_no_alloc` + 静的検査で保証）。
- TLS チェーンの unlink は window/scope いずれの drop 経路でも
  冪等に機能し、ノードの free は必ず unlink 後に行われる。

## Non-goals

- `#[global_allocator]` として実際に install した場合の
  プロセス全体計測の正確性（host binary の責務・本クレートは
  install しない）。
- `System` RSS・実ヒープ占有・leak 件数の正確な計測
  （要求ベース計数であり同義ではない）。
- RT Timing・CPU sampling・Tracy 連携の検証（別 slice/Issue）。
- `ruviform-rt::alloc_counter`・`play_sine` との統合試験
  （共存判断は文書化のみ・既存 lane は不変）。

## Risk

- allocator 経路に alloc/panic/lock が混入すると、計装自体が
  計測対象を破壊する（再帰・デッドロック・計測不能）。
- TLS チェーンの unlink 不変条件が破れると dangling pointer で
  UB になる — `mem::forget`・非 LIFO drop・スレッド終了の全経路で
  検証が必須。
- realloc の失敗時 current_bytes を誤って減算すると live bytes が
  過小評価され、メモリリークの見逃しにつながる。
