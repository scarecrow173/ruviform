---
id: "TSPEC-034KV6hs4k3mZ6jVBgDaKf"
type: "test-spec"
status: "approved"
title: "RT 観測コスト計測 PoC ハーネスの計測保証"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/104"
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
    - "../../AGENTS.md"
    - "docs/plans/implement-issue-104-rt-observability-poc.md"
    - "docs/designs/rt-verification-lane-design.md"
    - "docs/adr/rt-assert-no-alloc-thread-check.md"
    - "bench-tasks-and-ci-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  derived-by: []
  verifies:
    - "docs/designs/foundation-observability-minimal-api.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
  defers: []
  deferred-by: []
---

# RT 観測コスト計測 PoC ハーネスの計測保証

## Purpose

Issue #104 PoC の計測ハーネスが保証すべき計測の信頼性意図——「測定窓内の挙動のみを計測し、ハーネス自身が計測対象を汚染しない」「計測データが #103 設計の『要 PoC』判定に使える品質（分散記録・環境メタデータ・有効データ条件）を持つ」「alloc 検出カバレッジ隙間を機構レベルで実証する」——を、設計 [foundation-observability-minimal-api](../designs/foundation-observability-minimal-api.md)（rt_bridge 形状・Counter 選定・PoC-gated verdict）と [shared-foundations-and-modularization-policy](../designs/shared-foundations-and-modularization-policy.md)（RT/非 RT 観測データフロー・PoC と採否）から固定する。ハーネスが「計測は走るが数字が信用できない」状態でレポートだけが残る退行を防ぐ。廃止してよいのは、Issue #104 の計測成果物が #103 の要 PoC 判定へ反映完了した後に PoC ハーネス自体を除去するとき、または RT 観測の計測方法論を変更する ADR / 設計改訂が行われたとき。

## Feature

`ruviform-rt` 内の RT 観測コスト計測ハーネス: `#[doc(hidden)] pub mod observability_poc` の実験 scaffolding（`Counter`/`CounterReader` 計数ペア・`bridge` の `RtRecorder`/`RecordSink`・オーバーフロー方針バリアント・タイムスタンプ取得 helper）と、`benches/rt_observability_poc.rs` の criterion 計測列・`#[cfg(test)]` の self-consistency / カバレッジ隙間実証テストが、計測方法論契約を満たす形で動作すること。

## Rules

- **測定窓の純粋性**: ベンチの測定 iteration（criterion `iter` / `iter_batched` 等の被計測区間）とテストの計測窓は alloc/dealloc を行わない経路のみを対象とする。fixture 確保・`Vec` 準備・文字列整形などの allocate する準備処理は必ず窓外（`iter_batched` の setup 側・テストでは `alloc_counter::enable()` 前）に置く。self-consistency テストが scaffolding の RT 経路関数（`record`・`drain`・`overflows`・`Counter` 操作・タイムスタンプ helper）の窓内 alloc/dealloc = 0 を `alloc_counter` で検証する
- **ハーネス非攪拌**: scaffolding の計測対象 API は wait-free/no-alloc/bounded を維持し、計測列間の差分のみが変わるよう同一シナリオでバリアント比較する（counter 有効/無効の評価はランタイムフラグ単独ではなく「計装なし / フラグ付き / 無条件計装」のバリアント列比較）。`black_box` 等で最適化による対象消去を抑止する
- **有効データ条件**: 計測値は warm-up 済み・複数 sample の統計として記録し、criterion の出力（`estimates.json` の median/mean/CI）を 1 行 = 1 計測点の jsonl（LF・末尾改行あり、コミット可能・prek hook 適合）へ集約する。単発値・warm-up 前の値は採用しない
- **分散記録**: 2–3 run を実施し run 間分散を記録する。criterion の `estimates.json` は run ごとに上書きされるため「run 実行 → 集約 → 次 run」の順序を守る（bench-tasks-and-ci-smoke と同一規約）
- **環境メタデータ**: レポート冒頭に OS・CPU 型名・コア数・rustc 版・criterion 版・対象クレート版・負荷状況を記録する。絶対値は環境依存であり、判定には run 内相対比較（同一 run の基準列を分母にする比率）を用いる
- **カバレッジ隙間の実証**: `alloc_counter`（crate 既存の thread-local 測定窓 — `assert_no_alloc` と同じ機構）で、呼出側スレッドの窓が spawned thread 上の alloc/dealloc を観測しないこと（呼出側 count = 0 維持）と、spawned thread 内で窓を開けば観測できること（窓は設置スレッドのみ有効）の双方をテストで検証し、観測イベント数の 3 組（呼出側=0・spawned 側=N・N>0）を記録可能にする
- **バックエンド候補の非混入**: `tracing`/`tracing-subscriber`/`metrics`/`tracy-client` の評価は workspace の `Cargo.toml`/`Cargo.lock` へ追加しない。マイクロコストプローブは git 管理外スクラッチクレート（`target/` 配下・`[workspace]` 空テーブルで切離）で実施し、依存木・ライセンス・MSRV の表は公開メタデータ由来で構成する
- **cargo 呼出は mise 経由**: 計測・スモーク・部分計測（quick-run filter）は `mise run` / `mise exec --` 経由とし、裸 `cargo` を検証入口に使わない

## Examples

- Given: `observability_poc` scaffolding 実装済み。When: `mise run test` を実行する。Then: `record`/`drain`/`overflows`/`Counter`/タイムスタンプ helper の self-consistency テストが `alloc_counter` 窓内 0 イベントでグリーンになる
- Given: 呼出側スレッドで `alloc_counter::enable()` 後に alloc する thread を spawn・join。When: `count()` を読む。Then: 呼出側は 0 のまま（隙間の実証）。spawned thread 側で enable→alloc→count した場合は N>0 が返る（窓設置スレッドのみ観測可能）
- Given: `mise run bench-smoke`（`cargo bench -p ruviform-math -p ruviform-rt -- --test`）。When: 実行する。Then: ruviform-math・ruviform-rt 両方の全ベンチが計測なし 1 反復で完走し、ハーネス破損（panic・設定不備）があれば失敗する
- Given: `mise exec -- cargo bench -p ruviform-rt -- counter`。When: 実行する。Then: `counter` を名に含む計測列のみが計測される（quick-run filter 規約）
- Given: 2 回目のベンチ run 直前。When: 前回 run の `estimates.json` を確認する。Then: 次 run で上書きされるため、先に jsonl へ集約済みであることが必要となる
- Given: `docs/impl/exp/data/rt-observability-poc-*.jsonl` をコミット。When: `prek run --all-files` を実行する。Then: end-of-file-fixer / mixed-line-ending 系 hook を通過する（LF・末尾改行あり）
- Given: バックエンド評価の実施。When: workspace `Cargo.toml`・`Cargo.lock` を確認する。Then: `tracing`/`tracing-subscriber`/`metrics`/`tracy-client` が一切含まれていない（評価はスクラッチクレート・メタデータ由来）

## Guarantees

- 計測値が「測定窓内の対象操作のみ」のコストを表し、fixture 確保やハーネス自身の alloc に汚染されない（self-consistency テストがこれを機械検証する）
- scaffolding の RT 経路関数（`record`・`drain`・`overflows`・`Counter`・タイムスタンプ helper）が wait-free/no-alloc/bounded を維持し、#103 設計の rt_bridge スケッチ契約（drop-on-full + 共有オーバーフローカウンタ + bounded drain + `reserved: [u64; 2]` 規約）と形状が一致する
- counter 有効/無効の差分・オーバーフロー方針別コスト・タイムスタンプ取得コストが、同一 run 内の相対比較として解釈可能な形で記録される
- alloc 検出カバレッジ隙間が「呼出側窓=0 / spawned 側窓=N>0」の定量的実証として残り、`assert_no_alloc` 型スレッドローカル窓の機構的限界として再利用できる
- 計測結果が環境メタデータつき・hook 適合・コミット可能な形式で残り、#103 の要 PoC verdict（telemetry / rt_bridge / metrics / profiling / logging）の判断材料として再利用できる
- バックエンド候補評価が workspace 依存に混入せず、`Cargo.lock` を汚染しない

## Non-goals

- 計測値の性能判定・採否の最終判断（#103 設計の「要 PoC」項目の trigger Issue 側の責務。本 spec は「判定に使える計測データが得られること」を保証する）
- scaffolding の本番 API 確定・foundation/observability クレート新設・既存公開 API への Counter 適用（Issue #104 範囲外）
- `assert_no_alloc` の外部クレート実装の正しさ（上流ツールの責務。隙間の実証は機構＝スレッドローカル窓一般を対象とし、`alloc_counter` での実証が `assert_no_alloc` に移ることをレポートに明記する）
- criterion の統計処理・`estimates.json` スキーマの正しさ（上流ツールの責務）
- 実デバイスストリーム上の cpal `StreamInstant` 実測（実コールバックが必要。同形変換数学の代理計測に留め、実機経路は `ruviform-audio-device` 参照記録で担保）
- 他 OS / アーキでのベンチ再現（Windows x64 が主計測。バックエンド MSRV はメタデータ表で担保）
- CI ワークフローの手動ディスパッチ方針・`bench`/`test-rt` 等他タスクの検証内容（bench-tasks-and-ci-smoke・manual-ci-workflow・mise-verification-tasks の責務）

## Risk

- 測定窓の純粋性を欠くと、計測値がハーネスの alloc コストを含み「prepared counter / rt_bridge の RT コスト」が過大評価され、#103 の採否判定を誤方向に導く
- self-consistency を機械検証しないと「ハーネスが計測対象を汚染している」退化が検出されず、得られた数字の証拠性が失われる
- カバレッジ隙間を実証しないと「alloc 検出が効く」という誤前提が残り、将来 backend 所有スレッド上の RT 経路で未検出の alloc が混入する
- バックエンド候補を workspace 依存へ混入させると `Cargo.lock` が汚染され、評価のために実装が伴ってしまう（採否未定の依存をリポジトリが抱える状態）
- 環境メタデータ・分散記録を欠くと単発値の絶対値を採否根拠に誤用し、環境ノイズを実差分と取り違える
- バッチ反復なしの ns 級単発計測は criterion 分解能を下回り、「差なし」と「検出限界以下」を区別できない報告になる
