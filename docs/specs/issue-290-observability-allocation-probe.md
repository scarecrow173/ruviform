---
id: "SPEC-034UoMuIhsjFGaAbbUs8x0"
type: "spec"
status: "approved"
title: "Issue #290 OBS-08 Profiling — CountingAllocator/AllocationScope（allocation-probe slice）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
    - "docs/designs/rt-verification-lane-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-290-observability-allocation-probe-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/279"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/292"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #290 OBS-08 Profiling — CountingAllocator/AllocationScope（allocation-probe slice）

## What Should Be Built

`ruviform-observability` に **`profiling` モジュール**を新設し、Issue #290
（OBS-08）のうち **開発・テスト用 CountingAllocator + TLS AllocationScope**
の slice を実装する。`allocation-probe` feature（既定 OFF）で gate し、
feature 無効時は依存・コードともに完全に除外される。

- `CountingAllocator<A: GlobalAlloc>`: 任意の `GlobalAlloc` をラップする
  計数アダプタ。`alloc`/`alloc_zeroed`/`dealloc`/`realloc` の 4 API 全てを
  正しい Layout/Pointer/Alignment 契約で内側へ委譲し、要求件数・成功・
  失敗・要求バイト数・current/peak バイト数を計数する。
  `#[global_allocator]` の宣言は Host/Test Binary の責務とし、本クレートは
  型の提供のみを行う（ライブラリがグローバル登録を強制しない）。
- `AllocationScope` / `AllocationWindow`: TLS による**呼出スレッド限定**の
  有界計測ウィンドウ。ネスト・同時スレッド・スレッド終了時の挙動を
  明文化された契約として提供する。
- 計測値の読み出し型 `AllocationStats` / `OpStats`（要求/成功/失敗/
  バイト数の per-operation 内訳 + current/peak）。

本 slice は Issue #290 全体の一部であり、**RT Timing**（Clock/overrun/
Callback budget + Histogram 統合）と CPU sampling 手順の確定版は本 spec
では対象外とし、別 slice / 後続 Issue へ送る（下記 Out of Scope 参照）。

## Why Now

- Issue #290 は OBS-08 として承認済み（Design approved; implementation not
  started）。前提となる OBS-02 Source 契約（#277）・OBS-05 RT Bridge
  （#280）・OBS-07 Collector/Metrics（#289）は main に実装済みであり、
  profiling 層の土台が揃った。
- `foundation-observability-minimal-api`（approved）は `alloc_counter`
  所有者問題を「3 つ目のバイナリ需要または `#[cfg(test)]` を超えた計測
  アロケータ需要」で `observability` 側へ移すトリガを定めており、本 Issue
  がそのトリガに相当する。恒久的な置き場は install するバイナリ側だが、
  共有可能なのは `GlobalAlloc` アダプタ型のみ、という境界を守る。
- 現状 `ruviform-rt::alloc_counter`（test 限定・イベント数のみ・bytes
  なし）と `ruviform-audio-device/examples/play_sine.rs` の `CountingAlloc`
  が同種パターンの重複として実在する。本 slice はその棚卸し結果を文書化
  した上で、開発・テスト用途の正式な提供形を確定する。

## Users and Value

- **Host/Test Binary 実装者**: `#[global_allocator]` に
  `CountingAllocator<System>` を宣言するだけで、プロセス全体の
  alloc/dealloc/realloc を計数できる。テストでは `AllocationScope` で
  測定区間を区切り、「この区間で割り当てが起きない」等の RT 契約検証を
  型付きの計数値で行える。
- **RT/非 RT 計測の設計者**: TLS Scope が「呼出スレッドのみ」を保証する
  ため、別スレッドの RT callback を測るにはそのスレッドへ Window を
  設置する、という責務分界が明確になる。
- **OBS-11 Integration（#293）以降の Consumer**: feature OFF で出荷すれば
  オーバーヘッドゼロのまま、開発時のみ計装を有効化できる。

## Scope

### In Scope

- `crates/ruviform-observability/src/profiling/` 新設
  （`allocation-probe` feature で gate）:
  - `CountingAllocator<A: GlobalAlloc>` — `GlobalAlloc` 4 API
    （`alloc`/`alloc_zeroed`/`dealloc`/`realloc`）を全て内側 `A` へ
    委譲。計数は飽和カウンタ（u64 saturation）で、allocator 経路内で
    alloc/panic/tracing/重い lock/再帰を起こさない。
  - `AllocationStats`/`OpStats` — per-operation の calls/succeeded/
    failed/bytes と `current_bytes`/`peak_bytes` のスナップショット型。
  - `AllocationScope`/`AllocationWindow` — TLS のスレッドローカル計測
    ウィンドウ。ネスト（全 enclosing scope が計数）・同時スレッド
    （TLS 分離）・スレッド終了時（`try_with` フォールバック）・
    process-wide とのギャップを契約として明文化。
- `Cargo.toml` に `allocation-probe = []` feature 追加（外部依存ゼロ）。
- `lib.rs` への `#[cfg(feature = "allocation-probe")] pub mod profiling;`
  と doc bullet の additive 変更。
- 契約試験: 4 API の成功/失敗/zero/realloc/align 委譲、Layout 契約の
  維持、ネスト/同時スレッド/teardown/再オープン、allocator 経路の
  no-alloc・no-panic 性、`rt_` レーン非介入。
- 文書化: `ruviform-rt::alloc_counter`・`play_sine` example との
  移行/共存判断、計測 ON/OFF のオーバーヘッド、CPU sampling 手順と
  Tracy 等の任意 nonRT Layer の制約（設計文書に集約）。

### Out of Scope

- RT Timing（処理 Block/Callback/Plugin/Graph の実行時間計測、
  Clock Domain・overrun・Callback budget 意味論、Histogram 統合）—
  本 slice には含めず、後続 slice / Issue へ送る。
- CPU Profiler エンジン・独自 Heap Allocator・Allocation stack 追跡・
  Leak Analyzer・RSS/OS メモリ解析 — 外部 Profiler（samply/Tracy 等）の
  責務であり、本クレートは手順と制約の文書化のみ行う。
- `#[global_allocator]` のライブラリ側自動登録、Runtime 全
  Allocator 置換、Heap 再設計。
- `ruviform-rt::alloc_counter` の変更・移行そのもの（共存判断の
  文書化のみ。既存 RT test lane を保持する）。
- AudioLens/Synvelis での Enabled/Disabled 回帰評価、MSRV/feature
  matrix/OS ごとの #281 証拠 — cross-repo gate であり本 Issue の
  コード責務ではない（NotRun として記録）。
- `src/diagnostics/`（OBS-09・別 dispatch の領域）、`src/collector/`、
  `src/metrics/`、`src/source/`、他クレートの変更。

## Requirements

- `CountingAllocator<A>` は `alloc`/`alloc_zeroed`/`dealloc`/`realloc`
  の 4 `GlobalAlloc` API を全て override し、内側 `A` の対応 API へ
  委譲する。default 実装（`alloc_zeroed`→`alloc`+`write_bytes`、
  `realloc`→`alloc`+`copy`+`dealloc`）に依存しない — realloc が
  独立イベントとして計数されること、alloc_zeroed が zero 化を
  内側へ委譲することが契約。
- `GlobalAlloc` の unsafe 契約を維持する: Layout の non-zero size・
  align・isize 上限、返り値 null=失敗、realloc 失敗時に旧ブロックが
  生存すること、dealloc の ptr/layout 一致。委譲は SAFETY コメントで
  監査可能にする。
- 計数は要求件数・成功・失敗・要求バイト数を per-operation で記録し、
  `current_bytes`（net live requested bytes）と `peak_bytes` を追跡する。
  realloc は new_size を要求バイト数とし、成功時のみ
  `current += new_size - old_size`（失敗時は旧ブロック生存のため
  current 不変）。
- カウンタは飽和（u64 `saturating_add`）とし、allocator 経路内で
  alloc・panic・tracing 出力・blocking lock・再帰を起こさない。
  TLS アクセスは const 初期化 `thread_local!` + `try_with` で、
  スレッド終了時は「計数されない」へ縮退する（panic しない）。
- `AllocationScope` は呼出スレッドのみに計数を限定し、ネスト時は
  全 enclosing scope がそれぞれ独立に計数する。スコープは
  `&mut` borrow で排他し、ウィンドウの drop 順序が LIFO でなくても
  安全に unlink される。`mem::forget` 等で閉じられなかった
  ウィンドウはリークとして文書化し、dangling 参照を生じさせない。
- TLS Scope は process 全体の無割当保証を主張しない — 他スレッドの
  割当は process-wide カウンタ（`CountingAllocator` 自身の stats）
  のみが捕捉する。両者のギャップを文書化する。
- feature `allocation-probe` は既定 OFF で外部依存を持たない。
  feature 無効時に `profiling` モジュールとその試験はコンパイル
  対象外となる。
- 全公開 enum は `#[non_exhaustive]`。`unsafe` は `GlobalAlloc` impl
  と TLS raw pointer 復元の局所に限定し `#[expect(unsafe_code)]` +
  SAFETY コメントで監査可能にする。
- 既存の `Counter`/`Gauge`/`Histogram`・`record`/`rt_bridge`/
  `collector`/`source` の公開 API・RT 契約・性能・ビルドを回帰
  させない。`rt_` テストレーンの意味を変更しない。

## Acceptance Criteria

- [ ] AC-1: `CountingAllocator` の 4 `GlobalAlloc` API が内側 `A` の
  対応 API へ委譲され、成功/失敗（null 返却）/`alloc_zeroed` の
  zero 化/realloc（拡大・縮小・失敗時旧ブロック生存）/align を保つ
  試験がある。zero-size layout は `GlobalAlloc` 契約上 UB のため
  有効入力としては試験せず、契約の継承を文書化する。
- [ ] AC-2: `alloc_zeroed` が zero 化済み領域を返し、`realloc` が
  独立イベントとして計数され（default impl の alloc+dealloc 分解に
  ならない）、要求/成功/失敗/バイト数が `AllocationStats` に反映される。
- [ ] AC-3: `current_bytes`/`peak_bytes` が alloc/dealloc/realloc の
  増減を正しく追跡し、飽和カウンタが overflow で wrap しない。
- [ ] AC-4: `AllocationScope` のネスト（内側・外側双方が計数）、
  同時スレッド（TLS 分離で他スレッドを計数しない）、スレッド終了時
  （`try_with` 失敗で panic せず縮退）、ウィンドウの非 LIFO drop・
  再 enter が試験される。
- [ ] AC-5: allocator 経路（計数更新を含む 4 API）で alloc/panic/
  tracing/lock/再帰がないことが静的検査（コードレビュー・依存なし）
  と動的検査（`assert_no_alloc` レーンまたは同等）で示される。
- [ ] AC-6: `ruviform-rt::alloc_counter`・`play_sine` example との
  移行/共存判断、計測 ON/OFF のオーバーヘッド、CPU sampling 手順、
  Tracy 等の任意 nonRT Layer の制約が文書化される。
- [ ] AC-7: `mise run fmt-check`/`check`/`lint`/`test` が pass し、
  `cargo check/test -p ruviform-observability --features
  allocation-probe` が pass する。既存試験が回帰しない。
- [ ] AC-8: feature 無効の default build で `profiling` がコンパイル
  対象外であり、外部依存が追加されないことが `Cargo.toml`/`cargo
  tree` で確認できる。

## Deferred Design Concerns

- RT Timing（Clock/overrun/Callback budget + Histogram 統合）|
  本 slice の優先度は CountingAllocator 側の完結であり、RT Timing は
  Clock Domain 意味論・budget 定義・記録欠落の扱いを含む独立した設計が
  必要 | OBS-08 の後続 slice または別 Issue 起票 | 観測手段なしに RT
  性能を語れない状態が継続する
- CPU sampling 手順の実機検証・Tracy 連携 PoC | samply/Tracy は環境依存の
  外部ツールであり、本リポジトリの CI では検証不能 | #281 cross-consumer
  gate または専用 PoC Issue | 手順の陳腐化・誤った計測誘導
- Enabled/Disabled での AudioLens 分析精度・Synvelis 再生性能の回帰評価
  | cross-repo 実機評価であり #281 gate の責務 | #281 | feature ON 時の
  性能影響が未評価のまま出荷判断される

## Sources and Relations

- Issue #290（OBS-08）: 実装範囲・安全条件・完了条件の正本。
- Issue #275（Observability 設計正本）・#289（OBS-07 Collector）・
  #280（OBS-05 RT Bridge/Clock）: 前提となる実装済み基盤。
- `docs/designs/foundation-observability-minimal-api.md`: `alloc_counter`
  所有者判断・`profiling`/`debug` 候補のトリガと置き場の根拠。
- `docs/designs/rt-verification-lane-design.md`・
  `docs/adr/realtime-contract-and-verification-policy.md`・
  `docs/adr/rt-assert-no-alloc-thread-check.md`: RT 契約と
  `alloc_counter`/`assert_no_alloc` 併存ルール。
- `docs/adr/obs-05-observability-leaf-primitive-deps.md`: 観測系の
  依存方向制約（本 slice は外部依存を追加しない）。
