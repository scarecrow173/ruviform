---
id: "PLAN-034KV6r7RDBR7CSvMSIB8e"
type: "plan"
status: "completed"
title: "Issue #104 RT 観測経路コスト計測 PoC の実施計画"
created: "2026-10-23"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/104"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/observability_poc.rs"
      - type: "file"
        path: "crates/ruviform-rt/benches/rt_observability_poc.rs"
      - type: "file"
        path: "docs/impl/exp/rt-observability-poc-<yyyymmdd>.md"
      - type: "file"
        path: "docs/impl/exp/rt-observability-poc.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-<yyyymmdd>-<seq>.jsonl"
      - type: "file"
        path: "docs/impl/ir/rt-observability-poc.md"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/designs/foundation-observability-minimal-api.md"
      - type: "file"
        path: "docs/specs/implement-rt-foundation.md"
      - type: "file"
        path: "docs/adr/rt-foundation-primitives-implementation.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements: []
  implemented-by:
    - "docs/tasks/rt-observability-bench-harness.md"
    - "docs/tasks/rt-observability-poc-run-and-report.md"
  depends-on:
    - "docs/designs/foundation-observability-minimal-api.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "docs/designs/rt-verification-lane-design.md"
    - "docs/adr/rt-assert-no-alloc-thread-check.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  derived-by:
    - "docs/tasks/rt-observability-bench-harness.md"
    - "docs/tasks/rt-observability-poc-run-and-report.md"
  verifies: []
  verified-by:
    - "docs/test-specs/rt-observability-poc-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
  defers: []
  deferred-by: []
---

# Issue #104 RT 観測経路コスト計測 PoC の実施計画 Implementation Plan

## Goal

Issue #104 の PoC として、設計 `DESIGN-034KQXr3w4dYgKO79dklG9`（[foundation-observability-minimal-api](../designs/foundation-observability-minimal-api.md)）が「要 PoC」と判定を据え置いた項目の判断材料となる計測データを取得する。`ruviform-rt` に実験専用の計測ハーネス（`#[doc(hidden)]` PoC scaffolding + criterion ベンチ）を構築し、(1) prepared atomic counter の有効/無効時 RT コスト、(2) `rt_bridge` 形状レコード経路とオーバーフロー方針（破棄/集約/計測）の API 単位評価、(3) タイムスタンプ取得コスト、(4) `tracing`/`tracing-subscriber`/`metrics`/`tracy-client` の採否評価資料（依存・ライセンス・MSRV・RT コスト）、(5) `assert_no_alloc` スレッドローカル測定窓がバックエンド所有スレッドを観測できないカバレッジ隙間の定量実証、を experiment log・レポート・impl record として記録し、各「要 PoC」項目への verdict 推奨をフィードバックする。

## Scope

単一の実装ストリームとして扱う。ハーネス実装（T1）→ 計測実行・記録・判定フィードバック（T2）は同一クリティカルパス上の工程であり、「Issue #104 の計測データ一式」という単一のリリース意図を共有するため plan 分割はしない。

範囲外（Issue #104 の範囲外と一致 + 本 plan 固有の限定）:

- foundation / observability 実装クレートの新設（本 PoC の scaffolding は実験 API であり本番 API を確定しない）
- グローバル Subscriber / Logger / Recorder の導入（ライブラリ側で暗黙に初期化・変更しない。バックエンド評価はマイクロプローブ・資料収集の範囲）
- `tracing`/`tracing-subscriber`/`metrics`/`tracy-client` のワークスペース `Cargo.toml` / `Cargo.lock` への追加（評価は workspace 外スクラッチクレートまたはメタデータ調査で実施）
- 実デバイスストリーム上の計測（cpal `StreamInstant` は実コールバックでしか取得できない。`instant_to_media_time` と同形の変換数学を std `Instant` 上で計測する代理計測とし、実機経路の言及は `ruviform-audio-device` の既存実装への参照記録に留める）
- 設計 `DESIGN-034KQXr3w4dYgKO79dklG9` の「要 PoC」判定自体の変更（本 PoC が供給するのは判定材料の計測データと推奨 verdict の記録であり、採否・新設判断は trigger Issue 側で行う）

## Upstream Documents

- [GitHub Issue #104](https://github.com/scarecrow173/ruviform/issues/104) — 一次ソース（`relations.source`）。計測対象 5 項目・範囲外・依存関係の正本
- [GitHub Issue #101](https://github.com/scarecrow173/ruviform/issues/101) / [GitHub Issue #103](https://github.com/scarecrow173/ruviform/issues/103) — 上流 Issue チェーン（references）

## Design Inputs

- [../designs/foundation-observability-minimal-api.md](../designs/foundation-observability-minimal-api.md) — `DESIGN-034KQXr3w4dYgKO79dklG9`（approved）。`rt_bridge` 候補の形状（`bridge<R: Copy + Send>(capacity) -> (RtRecorder<R>, RecordSink<R>)`、`record` drop-on-full + 共有オーバーフローカウンタ、`drain` bounded）、Counter プリミティブ化対象の選定ルール（`retire_channel` の共有 `Arc<AtomicUsize>` 3 アクセサが参照形状）、固定サイズ Copy レコードの `reserved: [u64; 2]` 規約、タイムスタンプ取得方法を本 PoC に委譲した旨、「要 PoC」項目（telemetry / rt_bridge / metrics / profiling / logging）の trigger 条件の正本
- [../designs/shared-foundations-and-modularization-policy.md](../designs/shared-foundations-and-modularization-policy.md) — `DESIGN-034Kx6CVMPDNgzKhJwfgSp`（approved）。「RT / 非 RT 観測データフロー」節（prepared atomic counter + prepared fixed-size record + bounded non-blocking handoff + overflow 記録・集計 → 非 RT collector の形）と「適用・移行順序」4（PoC と採否）の正本。RT 経路に任意の文字列整形・動的ログ成形・未検証 Subscriber コールバックを置かない厳格化条件の根拠
- [../designs/rt-verification-lane-design.md](../designs/rt-verification-lane-design.md) + [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md) — `assert_no_alloc` 測定窓の RT 検証レーン設計（カバレッジ隙間の実証対象となる測定機構の正本）
- 計測先例: `crates/ruviform-rt/src/retire.rs`（leak-on-full + 共有 `Arc<AtomicUsize>`、`Retirer::overflows` / `DropCollector::overflows` / `SnapshotWriter::retire_overflows` の 3 アクセサ）、`crates/ruviform-audio-device/src/stream.rs`（`StreamInstant::as_nanos()` → `MediaTime` の RT 経路唯一のタイムスタンプ実装）、`crates/ruviform-audio-device/examples/play_sine.rs`（`CountingAlloc` — バックエンド所有スレッドを観測する process-wide 計数アロケータの既存先例）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-rt/src/observability_poc.rs` | `#[doc(hidden)] pub mod observability_poc` の PoC scaffolding: `Counter`/`CounterReader`（telemetry スケッチ形状の共有 `Arc<AtomicUsize>` 計数ペア）、`bridge<R: Copy + Send>(capacity) -> (RtRecorder<R>, RecordSink<R>)`（rt_bridge スケッチの実験実装。drop-on-full + 共有オーバーフローカウンタ + bounded drain、リングは既存 private dep `rtrb` 上に構築）、固定サイズ Copy レコード `ObsRecord`（`reserved: [u64; 2]` 規約）、オーバーフロー方針の計測バリアント（破棄/集約/計測）、`#[cfg(test)]` による self-consistency テストとカバレッジ隙間実証テスト（crate 既存の `alloc_counter` を利用） | pure std + crate 内依存のみ・`unsafe` 0・pub enum なし。モジュール doc に「実験 API・本番 RT 契約の対象外・#103 判定材料の計測用 scaffolding であり本番 API を確定しない」旨を明記 |
| `crates/ruviform-rt/src/lib.rs` | `#[doc(hidden)] pub mod observability_poc;` の追加 | crate doc・公開 re-export は変更しない |
| `crates/ruviform-rt/benches/rt_observability_poc.rs` | criterion 計測ハーネス: counter コスト列・rt_bridge record/drain/overflows 列・オーバーフロー方針バリアント列・既存プリミティブ基準列（`channel` try_push/try_pop・`retire`/`overflows`・`MasterClock::tick`/`publish_tick`・`slot` publish/read）・タイムスタンプ取得列。`BenchmarkGroup` 時間予算設定・環境メタデータ出力・quick-run filter 規約 | `[[bench]] harness = false`。ナノ秒級 atomic 操作はバッチ反復で SN 比を確保（methodology 節参照） |
| `crates/ruviform-rt/Cargo.toml` | `[dev-dependencies] criterion = { version = "0.8.2", default-features = false }` と `[[bench]] name = "rt_observability_poc", harness = false` の追加 | `[dependencies]` は現行（`arc-swap`・`rtrb`）のまま維持。バックエンド候補クレートは dev-dep にも追加しない |
| `Cargo.lock` | criterion dev-dep 追加に伴う再生成 | `mise run check` 経由で再生成 |
| `mise.toml` | `bench-smoke` の対象に `ruviform-rt` を追加（`cargo bench -p ruviform-math -p ruviform-rt -- --test`）+ タスク記述更新 | `bench`（`cargo bench --workspace`）は変更不要・自動で新ベンチを含む |
| `AGENTS.md` | 検証入口対応表 `bench-smoke` 行・`bench` 行の対象記述を実態に更新 | CI `bench` ジョブは `mise run bench-smoke` を呼ぶため ci.yml 変更不要 |
| `docs/impl/exp/rt-observability-poc-<yyyymmdd>.md` | 計測レポート（環境メタデータ・判定表・バックエンド評価表・カバレッジ隙間実証・#103「要 PoC」項目への verdict 対応表） | 生成時に日付確定 |
| `docs/impl/exp/rt-observability-poc.jsonl` | impl-doc イベントログ（`new_experiment_log.js` 生成。start/observation/decision/validation/summary） | 生成時に確定 |
| `docs/impl/exp/data/rt-observability-poc-<yyyymmdd>-<seq>.jsonl` | 生計測データ（1 行 = 1 計測点。LF・末尾改行） | `data/` サブディレクトリ配置（impl-doc のイベントログ監査は exp/ 直下のフラット `*.jsonl` のみ検証するため — dsp-simd-poc と同じ構造判断） |
| `docs/impl/ir/rt-observability-poc.md` | impl record（実施内容・検証実績・experiment log 参照・採否推奨の記録） | 生成時に確定 |
| `target/rt-observability-backend-probe/` | バックエンド候補のマイクロコストプローブ用スクラッチクレート（`[workspace]` 空テーブルで workspace から切離） | git 管理外。成果は数値のみレポートへ転記 |
| `docs/designs/foundation-observability-minimal-api.md`・`docs/designs/shared-foundations-and-modularization-policy.md` | `derived-by` に本 plan・test-spec を追記済み（doc chain 作成時） | 判定内容自体の変更は行わない |

ファイル一覧に未確定要素はない。experiment log・レポート・impl record のファイル名は生成時に確定するが、上記の計画名を既定とする。

## Implementation Sequence

### Phase A: 計測ハーネス実装（Task T1 — [rt-observability-bench-harness](../tasks/rt-observability-bench-harness.md)）

- [x] A-1. dev-dep 導入前のコンパイルコスト baseline を記録: `mise run check`（`cargo check --workspace --all-targets`）の所要時間を計測し、T2 の dev-dep 前後比較・レポート補助データの基準値として記録する — **実績: 導入前 baseline の検証可能な記録は未保持。before/after 差分と推移的依存数は未検証として記録し、既存 criterion 再利用・dev-dep 限定を理由に計測欠落の逸脱を受理**
- [x] A-2. `crates/ruviform-rt/Cargo.toml` に `[dev-dependencies]`（`criterion = { version = "0.8.2", default-features = false }` — ruviform-math と同一 pin 方針: caret 指定・`Cargo.lock` を再現性の錨・`default-features = false` でコンパイルコスト抑制）と `[[bench]] name = "rt_observability_poc", harness = false` を追加（`[dependencies]` は変更しない）。dev-dep 追加理由は Cargo.toml インラインコメントに記録
- [x] A-3. `src/observability_poc.rs` を新規作成: `Counter`/`CounterReader` 計数ペア、`bridge`（`rtrb::RingBuffer<R>` を利用し drop-on-full + 共有 `Arc<AtomicUsize>` オーバーフローカウンタ + `drain` bounded + 両端 `overflows()`）、`ObsRecord` 固定サイズ Copy レコード、オーバーフロー方針バリアント API（`record` drop 系 / aggregate 系 / count 系）、タイムスタンプ取得 helper（`Instant` → epoch 相対 nanos → `i64` 変換 + レコード格納 — `instant_to_media_time` と同形の数学）。`unsafe` 0・pub enum なし・pure std + crate 内依存（`rtrb`）のみ。`lib.rs` に `#[doc(hidden)] pub mod observability_poc;` を追加し、モジュール doc に実験 API 宣言を記載
- [x] A-4. `#[cfg(test)]` テストを `src/observability_poc.rs` に実装: (a) self-consistency — crate の `alloc_counter` で `record`/`drain`/`overflows`/`Counter` 操作・タイムスタンプ helper の測定窓内 alloc/dealloc が 0 であることを検証（fixture 確保は窓外）、(b) カバレッジ隙間実証 — `alloc_counter` 窓を呼び出し側スレッドで開いて alloc する spawned thread を join しても呼び出し側 `count()` が 0 のまま（隙間）であること、spawned thread 内側で窓を開けばイベントが観測できること（窓設置スレッドのみ観測可能）を検証・標準出力に 3 組データを出す、(c) scaffolding 動作 — record 成功/drop-on-full 計数/drain bounded/方針バリアントの意味論
- [x] A-5. `benches/rt_observability_poc.rs` を実装（下記「計測マトリクス」節の列構成どおり）。`criterion_group!`/`criterion_main!`・`BenchmarkId` 規約・`iter_batched`/手動バッチ反復・削減時間設定（warm_up/measurement/sample — 全量目安 ≈10–20 分）・run 冒頭に OS・CPU 型名・コア数・rustc 版・criterion 版・測定対象クレート版を標準出力
- [x] A-6. `mise.toml` の `bench-smoke` を `cargo bench -p ruviform-math -p ruviform-rt -- --test` に更新（タスク記述も実態に合わせる）し、`AGENTS.md` の検証入口対応表 `bench`/`bench-smoke` 行を更新。`mise run bench-smoke` スモーク完走と `mise exec -- cargo bench -p ruviform-rt -- <filter>` の quick-run filter を確認

### Phase B: 計測実行・記録・判定フィードバック（Task T2 — [rt-observability-poc-run-and-report](../tasks/rt-observability-poc-run-and-report.md)）

- [x] B-1. Windows x64 主計測環境で全量ベンチを実行（`mise run bench` = `cargo bench --workspace`）。2–3 run で分散を記録し「run → `estimates.json` 集約 → 次 run」の順序で実施。環境メタデータ（OS・CPU 型名・コア数・rustc 版・criterion 版・負荷状況）を記録
- [x] B-2. `estimates.json` から `docs/impl/exp/data/rt-observability-poc-<yyyymmdd>-<seq>.jsonl`（1 行 = 1 計測点、LF・末尾改行）を生成し、A-4(b) のカバレッジ隙間実証データ（呼出側窓=0・spawned 窓=N・spawned 側逆方向確認）を記録する
- [x] B-3. バックエンド評価を実施: `tracing`/`tracing-subscriber`/`metrics`/`tracy-client` の依存木・ライセンス・MSRV・保守状況の調査表（crates.io/docs.rs/cargo metadata 由来）+ 実施可能な範囲のマイクロコストプローブ（`target/rt-observability-backend-probe/` のスクラッチクレートで「global sink 未設定時の呼出コスト」を計測。workspace の `Cargo.toml`/`Cargo.lock` に一切追加しない）
- [x] B-4. experiment log（`new_experiment_log.js`）とレポート `docs/impl/exp/rt-observability-poc-<yyyymmdd>.md` を作成: 環境メタデータ・計測マトリクス判定表（counter on/off 差分・方針別コスト・タイムスタンプ取得コスト・既存プリミティブ基準値）・バックエンド評価表・カバレッジ隙間実証・**#103「要 PoC」項目への verdict 対応表**（telemetry / rt_bridge / metrics / profiling / logging それぞれに対し、計測データが示す推奨方向と未解決点を記録）
- [x] B-5. `docs/impl/ir/rt-observability-poc.md` を作成し、doc 監査・最終検証スイート（V-6）を実施

### 計測マトリクス（ハーネス列構成の正本）

| グループ | 計測列 | 観測対象 | 判定への供給 |
| --- | --- | --- | --- |
| `counter_equal_work` | `disabled` / `flag/off` / `flag/on` / `always` — 全列が同一 base workload（`wrapping_add`+`black_box`）を行い、計装 step（ランタイムフラグ付き / 無条件 `fetch_add`）の有無のみが変わる equal-work バリアント列（初回実装の非 equal-work `counter` 群はレビュー指摘で改訂）| prepared counter の有効/無効コストと、ランタイムフラグ経路の残存コスト（同一ワーク上の差分評価） | telemetry「要 PoC」: Counter 採否・デフォルト計装可否 |
| `rt_bridge` | `record`（非満杯）/ `record`（満杯 = drop+計数パス）/ `drain`（空・部分・全量）/ `overflows`（read） | rt_bridge 形状のレコード経路 RT コストと、飽和時コスト差 | rt_bridge「要 PoC」: drop-on-full + 計測方針の実測裏付け |
| `overflow_policy_nonzero` | drop / aggregate / count の 3 方針バリアント（同一満杯シナリオ。aggregate は非ゼロ値を fold する記録を使用 — ゼロ値 fold では方針コストを判別できないレビュー指摘の是正）+ `retire_channel` の leak-on-full 基準列 | 方針別の API 単位コスト（既存先例との比較） | rt_bridge 方針選択・将来 API 追加の方針材料 |
| `rt_primitives` | `channel` try_push/try_pop、`retire`/`overflows`、`MasterClock::tick`/`publish_tick`、`slot` publish/read | 既存 RT プリミティブの基準コスト（計装なし素コスト） | counter 計装の相対コストの分母・既存 API の計装余裕評価 |
| `timestamp` | `Instant::now()` / `Instant`→epoch 相対 nanos→`i64`→レコード格納（`instant_to_media_time` 同形）/ `SystemTime::now()`（対照列） | タイムスタンプ取得コストのプラットフォーム実測 | rt_bridge レコード schema のタイムスタンプ取得方法 verdict |
| `alloc_gap` | criterion ベンチ外 — A-4(b) のテストで定量実証（呼出側窓=0 / spawned 窓=N）| `assert_no_alloc` 型スレッドローカル窓のカバレッジ隙間 | RT 検証レーンの限界記録・将来の窓設置所有権設計材料 |

ナノ秒級の atomic 操作（`fetch_add`/`load` Relaxed ≈ 数 ns〜十数 ns）は criterion の測定分解能と同桁のため、単発測定ではなくバッチ反復（1 iteration あたり N 回操作 + `black_box` で最適化消去を抑止）で SN 比を確保する。「enabled/disabled」評価はランタイムフラグの有無でなく、同一操作の計装バリアント同士の run 内比較で評価する（run 内比率で環境ノイズを相殺 — dsp-simd-poc レポートと同じ評価形）。

## Dependencies and Risks

### Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| A（ハーネス: dev-dep + scaffolding + ベンチ + smoke 配線） | — | B |
| B（計測実行 + exp/ir 記録 + verdict フィードバック） | A | — |

- クリティカルパス: A → B（計測対象の scaffolding が無ければ計測できない直列構造。分割しない理由と一致）
- 外部前提: crates.io 上の `criterion` 0.8.2 の入手可能性、Windows x64 計測環境、バックエンド候補クレートのメタデータ（crates.io / docs.rs）参照可能性

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | データ損失 | — | なし | 変更は新規ファイル追加と設定追記のみで不可逆操作なし。rollback は git revert / ファイル除去で完結 |
| 2 | 破壊的変更（`#[doc(hidden)] pub` で実質公開面増加） | 中 | 低 | `publish = false` の内部クレートで外部契約化しない。実験 API 旨をモジュール doc に明記し、本番 API 確定は trigger Issue 側の判断に委ねる（simd_poc と同じ封じ込め形） |
| 3 | 計測分解能不足（atomic 操作が ns 級で criterion 分解能と同桁） | 高 | 中 | バッチ反復（N 回/iteration）+ run 内比率比較 + 複数 run 分散記録で対処。分解能を下回る差分は「検出限界以下」として正直に記録し、結論を捏造しない |
| 4 | Windows x64 計測ノイズ（turbo/省電力・負荷・タイマ分解能） | 高 | 中 | 2–3 run の分散・run 内比率評価・環境メタデータ（省電力モード・負荷状況）記録で緩和（dsp-simd-poc 先例どおり） |
| 5 | scaffolding が本番 API 形状と乖離し計測値の意味が弱まる | 中 | 中 | #103 設計のスケッチ（`bridge` シグネチャ・drop-on-full + 計測・`reserved: [u64; 2]`）を忠実に形状の正本とし、shape 変更は行わない。実験実装である旨をモジュール doc・レポートに明記 |
| 6 | バックエンド候補評価が机上調査のみで RT コストの実測が得られない | 中 | 中 | スクラッチクレートのマイクロプローブで「sink 未設定時の呼出コスト」だけでも実測を試みる。プローブ不可（API 形状・feature 前提等）ならその旨と理由を表に記録し、評価表の残り（deps/license/MSRV）で判断材料を構成する |
| 7 | スクラッチプローブが workspace に混入して `Cargo.lock` を汚染する | 低 | 中 | `target/`（git 管理外）配下に置き `[workspace]` 空テーブルで切離。コミット対象は `crates/ruviform-rt` の criterion dev-dep のみで、候補クレートは workspace の依存に一切入れない |
| 8 | `rtrb` 再利用で `record` の drop-on-full + 計数が既存 spsc と意味論重複する | 低 | 低 | `rtrb` は crate の既存 private dep であり利用は許容範囲。`RtRecorder` は「`R: Copy` レコード + drop 計数」の rt_bridge スケッチ固有の意味論を持ち、spsc channel（`T: Send` 任意型・`TryPushError` 返却）とは責務が異なるため別 scaffolding として実装する |
| 9 | dev-dep の MSRV 超過で `rustup run 1.89` 系検証が dev ターゲット込みで壊れる | 中 | 低 | MSRV 検証は `rustup run 1.89 cargo check -p ruviform-rt --lib --locked` に限定（dev-dep は `--lib` 範囲に影響しない — simd_poc と同じ封じ込め検証形）。`Cargo.lock` は先に mise pin ツールチェーン側で再生成 |

## Task Breakdown

| Task | 内容 | 対応 Phase | verified-by |
| --- | --- | --- | --- |
| T1 | 計測ハーネス実装: `ruviform-rt` の criterion dev-dep + `[[bench]]`、`#[doc(hidden)] pub mod observability_poc` scaffolding（counter ペア・bridge・方針バリアント・タイムスタンプ helper）、self-consistency + カバレッジ隙間実証テスト、計測マトリクス全列のベンチ実装、`bench-smoke`/`AGENTS.md` 更新 | A | [rt-observability-poc-tests](../test-specs/rt-observability-poc-tests.md) |
| T2 | 計測実行と記録: 全量計測（Windows x64・複数 run）・`estimates.json` 集約 → `data/` jsonl、バックエンド評価表 + スクラッチマイクロプローブ、experiment log + レポート（#103 verdict 対応表含む）・impl record・最終検証 | B | [rt-observability-poc-tests](../test-specs/rt-observability-poc-tests.md) |

コミット粒度は 1 タスク 1 コミットを目安とする（T1 は dev-dep+scaffolding とベンチ+smoke 配線で 2 コミットに分割してもよい）。

## Verification

| ID | 対象 | 検証コマンド / 確認内容 | 合格条件 |
| --- | --- | --- | --- |
| V-1 | Phase A 前半 | `mise run check` + `Cargo.toml` 目視 + `mise exec -- rustup run 1.89 cargo check -p ruviform-rt --lib --locked` | dev-dep 宣言・`[dependencies]` 不変・`harness = false`・`Cargo.lock` 再生成済み・MSRV check グリーン・baseline 計測値の記録あり |
| V-2 | Phase A scaffolding | `mise run check` / `mise run lint` / `mise run test` | グリーン。`src/observability_poc.rs` が pure std + crate 内依存のみ・`unsafe` 0（`unsafe_code` lint ゲート通過で検証）・pub enum なし・`#[doc(hidden)]` 露出のみ |
| V-3 | Phase A テスト | `mise run test`（`cargo test --workspace`） | self-consistency（測定窓内 alloc 0）・カバレッジ隙間実証（呼出側=0 かつ spawned 側で観測可能）・scaffolding 意味論テストがグリーン |
| V-4 | Phase A ベンチ | `mise run bench-smoke` + `mise exec -- cargo bench -p ruviform-rt -- <filter>` | スモーク完走（ruviform-math + ruviform-rt 両ハーネスが `--test` 1 反復で完走）・quick-run filter が機能・`estimates.json` 出力階層が `BenchmarkId` 規約どおり生成 |
| V-5 | Phase B 計測 | `mise run bench`（`cargo bench --workspace`、全量）+ experiment log レビュー | Windows x64 で計測完了。`docs/impl/exp/` にイベントログ jsonl・`data/` 生データ jsonl（LF・末尾改行）・環境メタデータつきレポート文書が存在。2–3 run の分散が記録される |
| V-6 | Phase B 文書・最終検証 | `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench-smoke` + `mise exec -- prek run --all-files` + `mise exec -- rustup run 1.89 cargo check -p ruviform-rt --lib --locked` + `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` + `git diff --name-only <base>...HEAD` | すべてグリーン。変更ファイルが `changes` 宣言スコープ内。impl record が experiment log を参照し、#103「要 PoC」項目への verdict 対応表がレポートに存在する |

## Rollback Strategy

| 対象 | Trigger | Procedure | Verification |
| --- | --- | --- | --- |
| PoC 変更全体 | verdict 採否に関わらず PoC 実装を取り下げる場合 | PoC コミット群の `git revert`。または新規ファイル（`src/observability_poc.rs`・`benches/`）の削除 + `Cargo.toml` / `lib.rs` / `mise.toml` / `AGENTS.md` の編集差分を `git checkout --` で戻し、`mise exec -- cargo generate-lockfile` で `Cargo.lock` を再生成 | `mise run check` / `lint` / `test` がグリーン、`Cargo.toml` に dev-dep・`[[bench]]` が残っていないこと |
| ドキュメントのみの戻し | experiment log / レポート / impl record の誤り | 文書の `superseded` 化または修正コミット（文書は履歴として残す運用のため削除しない） | `audit_docs.js --type all --json` クリーン |
| スクラッチプローブ | 不要化・誤生成 | `target/rt-observability-backend-probe/` を削除（git 管理外のため履歴影響なし） | 存在しないこと |

## Open Questions / Gaps

- `ObsRecord` の確定フィールド: 計測のために必要な最小形（kind 識別子 + `i64` タイムスタンプ + `u64` 値 + `reserved: [u64; 2]`）を既定とするが、#103 の「固定サイズ Copy レコード schema の共有要否は各ドメイン個別」の判定どおりドメイン固有 schema は設計しない
- バックエンド候補のマイクロプローブ実施可能性: `tracy-client` は feature/ランタイム前提があり「sink 未設定時コスト」の測定が成立しない可能性（成立しない場合は表中にその旨を記録し、deps/license/MSRV 評価のみに留める）。`tracing` は global subscriber 未設定時のマクロ展開コストが計測可能な見込み
- `SystemTime::now()` の対照列: RT 経路で使うべきでない参照点として計測するが、環境によっては `Instant::now()` との差が小さい可能性（その場合も実測として記録）
- カバレッジ隙間実証の配置: `alloc_counter`（crate 既存・thread-local）で実証する方針だが、`assert_no_alloc` そのものでの実証も同じ thread-local window 機構である旨をレポートに明記する。`assert_no_alloc` を dev-dep に追加して直接実証するかは T1 実装時の判断余地とする（追加する場合も dev-dep 1 本の範囲内）
- bench 実行環境の CPU: 計測時に環境メタデータとして記録（dsp-simd-poc と同じ Windows x64 主計測環境を想定）

## Self-Review

- [x] Issue #104 の計測対象 5 項目（counter on/off・オーバーフロー方針 API 単位・タイムスタンプ取得・バックエンド採否評価・alloc カバレッジ隙間）が Phase A/B・計測マトリクス・V-1〜V-6 に対応している
- [x] #103 設計の PoC-gated 項目（telemetry / rt_bridge / metrics / profiling / logging）へフィードバックする経路（レポートの verdict 対応表）が B-4 に明確化されている
- [x] 範囲外（新クレート・グローバル sink・バックエンドクレートの workspace 依存追加・実機計測・設計判定自体の変更）が Scope に明記されている
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は Open Questions / Gaps に記録し判断先を明記）
- [x] ファイルパス・relation・status・用語が Issue #104・#103 設計・#101 設計・test-spec と整合している

## Implementation Handoff

- Recommended mode: inline execution（直列 2 タスクの小規模 PoC であり、subagent dispatch の分割利益が小さい。T1 内の scaffolding 実装とベンチ実装は一続きの作業として扱う）
- Independent work streams: なし（T1 → T2 の直列）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー重点: (1) ハーネス配置 = `ruviform-rt` の `#[doc(hidden)] pub mod observability_poc` + `benches/`（simd_poc の ruviform-math 配置先例と同じ封じ込め形。新 PoC クレート新設や `ruviform-math` への同居ではなく、計測対象である RT プリミティブを所有する crate に置く判断の妥当性）、(2) 「enabled/disabled」評価をランタイムフラグではなく計装バリアント列の run 内比較で行う methodology（フラグ込み計装列も併設する三段評価の妥当性）、(3) `alloc_gap` を criterion 計測ではなくテスト実証 + レポート転記とした扱い（計測対象というよりカバレッジ性質の実証であるため）、(4) バックエンド評価を workspace 外スクラッチプローブに限定する境界（`Cargo.lock` 非汚染の担保）
- 明示的に残した前提: experiment log・レポート・impl record のファイル名は生成時に確定可（File Map の計画名を既定）。`ObsRecord` の確定フィールドは T1 実装時に最小形で確定する
- 上流参照: Issue #104（計測対象の正本）、design `DESIGN-034KQXr3w4dYgKO79dklG9`（rt_bridge 形状・Counter 選定・要 PoC verdict の正本）、design `DESIGN-034Kx6CVMPDNgzKhJwfgSp`（観測データフロー・PoC と採否の正本）、test-spec `TSPEC-034KV6hs4k3mZ6jVBgDaKf`（計測方法論の契約の正本）
