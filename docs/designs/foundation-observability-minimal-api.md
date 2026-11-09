---
id: "DESIGN-034KQXr3w4dYgKO79dklG9"
type: "design"
status: "approved"
title: "Foundation/Observability 候補モジュールの最小 API・依存 DAG・新設時期の設計"
created: "2026-10-23"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/103"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "shared-foundations-and-modularization-policy.md"
    - "ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/rt-foundation-primitives-implementation.md"
    - "../adr/rt-assert-no-alloc-thread-check.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "overview.md"
    - "rt-verification-lane-design.md"
    - "issue-275-observability-integration-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "shared-foundations-and-modularization-policy.md"
    - "../specs/implement-rt-foundation.md"
    - "../adr/rt-foundation-primitives-implementation.md"
  derived-by:
    - "../plans/implement-issue-104-rt-observability-poc.md"
    - "../test-specs/rt-observability-poc-tests.md"
    - "../discovery/issue-121-diagnostics.md"
    - "../specs/issue-121-diagnostics.md"
    - "../specs/issue-276-generic-logging-adapter.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/104"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/22"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  defers: []
  deferred-by: []
---

# Foundation/Observability 候補モジュールの最小 API・依存 DAG・新設時期の設計

## 目的と位置づけ

本書は [共通基盤・Observability・クレート分割の設計方針](shared-foundations-and-modularization-policy.md)（Issue #101・approved）「適用・移行順序」の 2（Foundation 候補の最小設計）・3（Observability 候補の最小設計）を実行し、Issue [#103](https://github.com/scarecrow173/ruviform/issues/103) の成果物として `foundation::{collections, memory, sync, identity, telemetry}` と `observability::{logging, profiling, metrics, diagnostics, debug, rt_bridge, export}` 候補モジュールの最小 API スケッチ・新設時期トリガ・依存 DAG を定める。

入力は同書「既存実装の棚卸し」節で確定した事実と、棚卸し済みサイトの直接のコード確認範囲（`offline.rs` 内部の `failed`/`CancellationToken`・`play_sine.rs` の static 群・テスト内 atomic・`GuiHandle`/`GuiParentToken` 等の個別確認）に限る。新たな棚卸し領域の探索や利用者の想定は追加しない。**本書は各候補の「もし抽出するなら」の最小形と「いつ抽出判断を起こすか」のトリガを設計するのであり、クレート新設・既存実装の移動・外部依存の採用を確定しない。** `ruviform-foundation`・`ruviform-observability`・そのモジュール名は引き続き構成候補であり、本書の API スケッチは実装コードではなく設計上の輪郭である。

RT 候補形状の前提（事前確保・bounded/non-blocking 通信・retire の取りこぼし観測）は既存 RT 基盤 [spec](../specs/implement-rt-foundation.md) と採用 [ADR](../adr/rt-foundation-primitives-implementation.md) から継承する。これらへの派生リンクは既存契約の根拠を明示するもので、PoC API の本番契約への昇格や既存 spec の範囲拡張を意味しない。

## 状態区分と判定ルール

判定区分は #101 設計書と同じ 5 区分（方針／構成候補／保留／要 PoC／実装済み）を使う。本書でのモジュール判定は次のルールに従う。

- **所要前提**: そのモジュールの存在を正当化する具体的な消費者・前提が棚卸し実態にあるか。ない場合は「現時点では不要」と明示し、想定利用者を創作しない。
- **新設時期トリガ**: 保留を解除して実装 Issue を起票する具体的な条件。トリガは観測可能な事象（利用者の出現・PoC 結果・別 Issue の着手）で書く。
- 棚卸しで実装・利用者が存在しない領域に「念のため」の API を先行設計しない（YAGNI）。スケッチはトリガ成立時の設計着手点を示すための最小限の輪郭に留める。

なお #101 で **要 PoC** と記録した項目（`sync` 移動・Identity/Memory 汎用化・`alloc_counter` 所有者）は、本書では PoC 照合をトリガ成立時の手続きに織り込んだ **保留** として再分類する（延期条件と判定の実質は同一）。

## Scope

### In Scope

- `foundation::{collections, memory, sync, identity, telemetry}` 候補モジュールの最小 API スケッチ・所要前提・新設時期トリガ
- `observability::{logging, profiling, metrics, diagnostics, debug, rt_bridge, export}` 候補モジュールの最小 API スケッチ・所要前提・新設時期トリガ
- 所有者ローカル Atomic（crates/ 全体で約 75 箇所・`src/` のみでは 46 箇所。棚卸し済み）のうち `Counter` プリミティブ化対象の選定
- 固定サイズ Copy レコード schema（`reserved: [u64; 2]` を持つ 5 型）の共有要否
- RT → 非 RT レコード受け渡し（`rt_bridge` 候補）の形状（bounded non-blocking handoff・overflow 記録）
- ドメインに残すものの明確化（ドメイン別 Identity 型・ドメイン意味を持つ Pool/Buffer 等）
- `alloc_counter`（`ruviform-rt`・`#[cfg(test)]` 限定）の所有者判断
- 新設時の依存 DAG と非既定依存（`ruviform-rt → foundation`）の制約明示
- 本設計から派生する後続 Issue の整理

### Out of Scope

- `ruviform-foundation`・`ruviform-observability` のクレート新設の実装自体
- `channel`／`retire_channel`／`slot` を含む既存実装の移動（別途トリガ管理: 2 番目のプロダクション利用者の出現）
- `tracing`／`metrics`／`tracy-client` 等の外部バックエンド採否（Issue #104 PoC の権限）
- 共通 `platform`/`process`/`ipc` 候補（Issue [#76](https://github.com/scarecrow173/ruviform/issues/76) 関連）— #101 の要 PoC 行の権限であり、本書では意図的に触れない
- baseline・accepted ADR の改訂自体（必要な改訂は後続 Issue で行う）

## Foundation 候補モジュールの最小設計

### 判定一覧

| モジュール候補 | 所要前提（棚卸し実態） | 判定 | 新設時期トリガ |
| --- | --- | --- | --- |
| `collections` | 汎用コンテナ実装・`arrayvec`/`heapless`/`smallvec` 依存ともに非存在。「prepare で確保・process は借用」はワークスペース規約であり共有コードではない | **保留**（現時点では不要） | 2 つ以上のドメインで同一不変条件の固定容量コンテナ需要が実在し、監査対象の外部クレートでは不足と判明したとき |
| `memory` | 汎用 Pool/Arena/Storage は非存在 | **保留**（現時点では不要。「移動」ではなく新規設計の要否判断） | 汎用利用者・実装の実在（#101 延期行と同一トリガ） |
| `sync` | `channel`/`retire_channel`/`slot` は外部プロダクション利用者ゼロ・内部依存連鎖あり | **保留**（移動は延期維持） | 2 番目のプロダクション利用者の出現（3 プリミティブ同時判断） |
| `identity` | 世代付きハンドルは `PluginHandle` のみ | **保留**（現時点では不要。ドメイン型の統合はしない） | 2 つ目の世代付きハンドル需要の出現 |
| `telemetry` | 飽和・取りこぼし観測カウンタの先例（retire 系共有 `Arc<AtomicUsize>`）と将来消費者（xrun 等）が存在。**Issue #121 で「observability 最初の実需要」を記録**: `ruviform-observability` クレート新設（writer/reader pair `Counter`/`CounterReader` — 共有 `AtomicU64` cell の writer-owned increment・reader-side read）が L1 diagnostics のデータ面 counter（`RtLeg` の underrun/pass-through/dropped-submit/`RESULT_FAILED` 計数）に初消費された（正本 `docs/designs/issue-121-l1-diagnostics.md`） | **構成候補として維持・要 PoC** | Issue #104 PoC 結果 + 2 つ目のドメイン計測点需要（**#121 L1 diagnostics が第1の実消費者として着地 — 第2ドメイン計測点は残りのトリガ**） |

### `collections`（保留）

最小スケッチ（抽出する場合の輪郭）: 固定容量配列・リングの選択は監査対象の外部クレート（`arrayvec`/`heapless` 系・採否未決定）の薄い facade または再エクスポートであり、独自実装を先行しない。`Drop` の RT 安全性はコンテナ型が保証せず利用側契約とする（#101「Foundation 候補の内訳」の注意と同旨）。

所要前提なし: 「prepare で確保・process は借用」の規約は dsp・audio-analysis・adapter の scratch 型に共有コードなしで実装済みであり、共通化の対象になる不変条件の重複は観測されていない。

### `memory`（保留）

最小スケッチ（新規設計する場合の輪郭）: `Storage`/`Arena`/`Pool` の契約型と容量・使用量の計測点。構築・借用・返却・再利用・破棄・枯渇・シャットダウンを別フェーズとして仕様化し、`release` が固定時間でも `T: Drop` の RT 安全性は保証しない（#101「Memory と所有権」）。`#[global_allocator]` の設定は利用側に委ね、ライブラリが強制しない。

所要前提なし: 汎用実装が非存在であるため「移動」ではなく新規設計の要否判断となる。音声／動画バッファの意味を持つ Pool/Buffer は対象外（ドメイン常駐・後述）。

### `sync`（保留・移動延期維持）

最小スケッチ（抽出する場合の輪郭）: `ruviform-rt` の既存公開面をほぼそのまま移す形が最小形 — `channel`/`Producer`/`Consumer`/`TryPushError`/`TryPopError`、`retire_channel`/`Retirer`/`DropCollector`、`slot`/`SnapshotWriter`/`SnapshotReader`/`SnapshotGuard`。cache padding 等の追加はこの段階では要求しない。

棚卸し事実に忠実な判定: `channel`/`retire_channel`/`slot` は外部プロダクション利用者を持たず（`rt` の外部利用は `RtContext`・`Processor`/`RtProcessor`・`MasterClock` のみ）、`slot` は `retire_channel` 上に、`retire_channel` は `channel` 上に構築されている（`spsc.rs`・`retire.rs`・`snapshot.rs`）。`channel` 単独の移動は 3 プリミティブ同時移動か、非既定の `ruviform-rt → foundation` 依存（後述「依存 DAG」）を意味するため、**移動は延期を維持する**。トリガは 2 番目のプロダクション利用者の出現であり、成立時は既存 ADR・公開 API・RT 契約との照合（要 PoC）を経て判断する。

### `identity`（保留）

最小スケッチ（新規設計する場合の輪郭）: `PluginHandle` と同型の `{ slot: u32, generation: u16 }` 世代付きハンドル構成部品。ドメイン型そのものではなく「世代付きハンドルを組み立てる部品」が最小単位。

所要前提なし: 世代付きハンドルは `PluginHandle`（`ruviform-plugin-host`）のみであり、`StreamId`・`ParamId` はレジストリ newtype、`DeviceId` は opaque `String` とセマンティクスが異なる。統合は偽の抽象化となるため行わない（#101「Identity」棚卸しと同旨）。トリガは 2 つ目の世代付きハンドル需要（例: 別のホスト境界が再利用スロットを管理する場合）の出現。

### `telemetry`（構成候補として維持・要 PoC）

Foundation 候補のうち唯一、具体的な将来消費者が見えているモジュール。最小スケッチ:

```rust
// スケッチ — 実装ではない。型名・シグネチャは輪郭の例示。
/// 非 RT 側から観測可能な共有カウンタ。Relaxed bookkeeping。
/// retire_channel の共有 `Arc<AtomicUsize>` 先例を一般化した形。
pub struct Counter /* (書き手側。clone で Reader と共有) */;
pub struct CounterReader /* (非 RT 観測側。load のみ) */;

/// RT → 非 RT 受け渡し用の固定サイズ Copy レコード規約。
/// `Copy + Send`、デストラクタを持たず、長さはコンパイル時固定。
/// 拡張は `reserved: [u64; 2]` 規約（後述「固定サイズ Copy レコード schema」）。
```

所要前提: `retire_channel` の共有 `Arc<AtomicUsize>` オーバーフローカウンタ（`Retirer::overflows`・`DropCollector::overflows`・`SnapshotWriter::retire_overflows` が同一カウンタを読む先例）と、#101「適用例」の audio xrun・キュー飽和・Drop 退避・プール使用量という Metrics の将来消費者。ただし現時点では共有カウンタ型は 1 先例のみであり、`ruviform-rt` への適用は非既定依存問題（後述）に阻まれるため、新設は Issue #104 PoC（RT 記録コスト・オーバーフロー方針の実測）と 2 つ目のドメイン計測点需要をトリガとする。

## Observability 候補モジュールの最小設計

> **2026-11-09 改訂（Issue #275 合意版）**: 本節の observability 候補モジュール判定（`logging`/`profiling`/`metrics`/`diagnostics`/`debug` の「保留」、`rt_bridge`/`export` の「構成候補・要 PoC」）と、後述の `ruviform-observability` 新設トリガは、Issue [#275](https://github.com/scarecrow173/ruviform/issues/275) の合意設計（2026-11-04）と OBS-01〜11 の main 上の実装により解決済みである。`ruviform-observability` は単一クレートとして `core`/`source`/`collector`/`diagnostics`/`debug`/`record`/`rt_bridge`/`rt_timing`/`profiling`/`tracing_export`/`logging`/`integration` モジュールを持ち、統合設計の正本は [Issue #275 Observability統合設計正本](issue-275-observability-integration-design.md)。本節の判定表とスケッチは当時の分析記録として保持し、本文は改変しない。

### 判定一覧

| モジュール候補 | 所要前提（棚卸し実態） | 判定 | 新設時期トリガ |
| --- | --- | --- | --- |
| `logging` | `log`/`tracing` 等の observability 依存は全 19 クレートに非存在 | **保留**（現時点では不要） | ドメインから構造化イベント出力の需要が実在しバックエンド PoC が済んだとき |
| `profiling` | 計測・profiler 連携の実装非存在 | **保留**（現時点では不要） | 実行時間・リソース原因分析の需要 + #104 PoC（tracy-client 等の採否評価を含む） |
| `metrics` | 型付きエラーと局所カウンタのみ。集約機構は非存在 | **保留**（将来消費者は既知・実装着手は #104 PoC 結果待ち） | 最初のメトリクス集約需要（例: ホストへの xrun/飽和レポート）+ #104 PoC |
| `diagnostics` | 型付きエラーが既存慣用形。dump/inspect/capture/replay 非存在 | **保留**（現時点では不要） | ドメインをまたぐ健全性・原因説明の需要の出現 |
| `debug` | RT 契約 probe（`alloc_counter`・`assert_no_alloc`・CLAP thread-check・retire overflow カウンタ）は実在するが検証用途に限定（CLAP thread-check のみ、非オーディオスレッド応答時の defined rejection として `process` 本番経路でも照会される） | **保留**（重い独立クレート化は延期維持） | ドメインをまたぐ具体的な検査・再現需要 |
| `rt_bridge` | bounded non-blocking handoff + overflow 記録の先例（`retire_channel`・`channel`）と、RT → 非 RT 観測レコード転送の需要が設計上既知 | **構成候補として維持・形状を本書で設計（要 PoC）** | 最初の RT → 非 RT 観測レコードパイプライン需要 + #104 PoC |
| `export` | 出力 backend 非存在 | **保留**（現時点では不要） | logging/metrics/diagnostics のいずれかが実在し出力先が必要になったとき |

### `logging`・`profiling`・`export`（保留）

最小スケッチ（必要になった場合の輪郭）:

- `logging`: 発行側ドメインが所有するイベント schema の emit 経路と sink ポート型のみ。`tracing` 等のバックエンド採否・Subscriber 初期化は利用側（Host/Application）の所有であり、ライブラリがグローバル Subscriber/Logger を暗黙に初期化・変更しない（#101 方針）。独自 Log/Span フレームワークは重複実装しない。
- `profiling`: span・計測区間のアダプタポート。RT 計測は独立検証した軽量記録のみ。`tracy-client` 等の採否は #104 PoC の権限。
- `export`: console/file/trace/外部連携のアダプタ。上流モジュールが実在してから設計する。

所要前提なし（現時点）: 全 19 クレートに observability 系依存が存在せず、構造化ログ・プロファイル出力の需要は棚卸しに現れていない。

### `metrics`（保留・将来消費者は既知）

最小スケッチ: 事前確保済みカウンタ（`foundation::telemetry` 候補の `Counter` 形またはドメイン内カウンタ）を読み出す集約・問合せ面。カウンタ値の所有は発行側（ドメインまたは telemetry）に残し、Metrics は計数・集約・時系列化を担い、ログ生イベントから metric を導出する場合もその導出は Metrics 側の責務（#101 責務区分）。

所要前提: 将来消費者（xrun・処理時間・キュー飽和・Drop 退避・プール使用量）は #101 で列挙済みだが、最初の集約パイプライン需要は未発生。トリガは最初のメトリクス集約需要と #104 PoC 結果。

### `diagnostics`（保留）

最小スケッチ: 構造化診断レコード（severity・事象 ID・context）と health check/レポートのポート。ドメインの `Error`/`Result` 型は所有しない（media-io `ErrorKind`・`DeviceError`・`HostError`・`OfflineError`・media-core `Error` の型付きエラー慣用形を維持し、巨大 enum への統合は行わない — #101「Diagnostics と Error の違い」）。失敗の意味付け・原因切り分け・経緯の収集のみを担う。

所要前提なし（現時点）: 型付きエラーによる診断が既存の慣用形として十分に機能しており、ドメインをまたぐ健全性説明の需要は棚卸しにない。

### `debug`（保留）

最小スケッチ: 不変条件検査・状態 dump のポート型と、RT 契約 probe（alloc/dealloc 検出・容量超過・実行時間観測）の検証用途インターフェース。重い検査は非 RT API として公開し、全利用者への必須依存にしない（#101「Debug の検討対象」）。`alloc_counter` の所有者判断は後述の独立節で行う。

所要前提なし（現時点）: dump/inspect/capture/replay の実装は非存在。ドメインをまたぐ利用需要の出現がトリガ（#101 延期行と同一）。

### `rt_bridge`（構成候補として維持・形状を設計）

唯一「形状の設計」が要求されるモジュール。詳細は「rt_bridge 候補の形状」節。判定としては実装着手は Issue #104 PoC と最初の実需要を待つ保留であり、API の輪郭と契約方針は本書内で確定する（公開 API の確定ではなく、#104 PoC 結果による見直しを許容する設計上の決定）。

## Counter プリミティブ化対象の選定

棚卸し済みの所有者ローカル Atomic（crates/ 全体約 75 箇所・`src/` のみ 46 箇所）のうち、プロダクション `src/` の実サイトを個別に判定する。**選定ルール: `Counter` 化対象は「書き手スレッドと別スレッドから読む必要のある計数値」（飽和・取りこぼし・完了数等の累積カウント）のみとする。** 値 mirror・状態機械・フラグ・時刻・テスト instrumentation は対象外。

| サイト | 実体 | 判定 |
| --- | --- | --- |
| `retire_channel` 共有カウンタ（`rt/src/retire.rs` の `Arc<AtomicUsize>`。`Retirer::overflows`・`DropCollector::overflows`・`rt/src/snapshot.rs` `SnapshotWriter::retire_overflows` の 3 アクセサが同一値を読む） | 飽和・取りこぼし計数 | **Counter プリミティブの参照形（適用候補）。** 既に「事前準備済みカウンタ + 非 RT 観測」の形状。ただし適用には `ruviform-rt → foundation` の非既定依存が必要となり、ADR 改訂が先行するため現状は所有者ローカル維持 |
| `ruviform-exec` `Progress`（`offline.rs` の `completed`/`total` `Arc<AtomicU64>`） | 非 RT 進捗計数 | **形状上の候補**（共有カウンタ読み出し）。`foundation::telemetry` が実在しない現時点では所有者（exec）ローカル維持。移行自体の効果も小さい |
| `ruviform-rt` `MasterClock::tick`（`clock.rs` の `AtomicU64`） | 時刻値の共有 | **対象外** — 累積カウントではなく最新値の公開（Release/Acquire の値共有）。Counter ではなくクロック契約 |
| `ruviform-exec` `CancellationToken`（`Arc<AtomicBool>`）・`offline.rs` 内部の `failed: Arc<AtomicBool>` | フラグ | **対象外** — 計数ではなく単発状態遷移。プリミティブ化しても契約が変わらない |
| plugin-clap `gain_plain_bits`（`adapt.rs` の `AtomicU64`）・plugin-vst3 `gain`（`controller.rs` の `AtomicU64`・`f64` bits）・`tail`（`component.rs` の `AtomicU32`） | パラメータ/値 mirror | **対象外** — カウントではなく RT 側が読む最新値の mirror。`latest-wins` 値共有であり集計対象でない |
| plugin-clap `AtomicLifecycle`（`adapt.rs` の `AtomicU8`） | 状態機械の原子エンコード | **対象外** — ライフサイクル状態の packed encoding であり計数でない。アダプタのドメイン固有機構 |
| `ruviform-audio-device` `play_sine.rs` の static カウンタ群（`ALLOC_EVENTS`/`DEALLOC_EVENTS`/`CALLBACKS`/`XRUNS` 等の `AtomicUsize` と `FIRST_CALLBACK_NS` 等の `AtomicI64`） | example 専用 instrumentation | **対象外（現状）** — Counter 参照形に最も近い使用形だが、example バイナリ固有の計測で共有需要がない。`CountingAlloc` については「alloc_counter の所有者」節 |
| テスト内 atomics（audio-core の `frames_seen`/`errors_seen`・plugin-clap の `is_audio`/`is_main`・plugin-vst3 の `point_calls`・rt の `DropFlag` 等） | test-only instrumentation | **対象外** — 検証用観測点でありプロダクションの共有契約でない |

結論: `Counter` プリミティブ化の実質的な適用候補は retire 系オーバーフローカウンタと `Progress` のみであり、両者とも現時点では所有者ローカル維持が正しい。`Counter` 型自体は `foundation::telemetry` 候補の一部として設計を留保し、既存サイトへの適用は `telemetry` 実在と `rt → foundation` 依存問題の解決を前提とする別 Issue とする。

## 固定サイズ Copy レコード schema の共有要否

棚卸し済みの 5 型（`TransportSnapshot`（media-core）、`Event`・`TransportInfo`・`ProcessConfig`・`ProcessContext`（いずれも plugin-core））はいずれも `reserved: [u64; 2]` の不透明拡張フィールドを持つ `Copy` レコードである。

**判定: 共有 schema 型は作らない。** 5 型はレコードの意味・読み手・進化経路がドメイン固有であり、共通基底型への統合は偽の抽象化となる（Identity の判定と同じ構造）。共有するのは型ではなく**規約**であり、本書で以下を記録する:

- 固定サイズ `Copy + Send` の値レコードは `reserved: [u64; 2]` の不透明拡張フィールドを末尾に持ち得る
- `reserved` はコンストラクタで `[0; 2]` に初期化し、公開 `reserved()` アクセサ経由でのみ読み出す。呼び出し側は内容を解釈しない
- 公開 enum 側の `#[non_exhaustive]` と同じく、追加フィールドを破壊的変更なく受け入れるためのワークスペース規約として扱う
- `rt_bridge` のレコード設計（後述）もこの規約に従う。共有基底型ではなく規約の再利用とする

命名衝突の注意（#101 棚卸しの記録を維持）: `rt` の `Snapshot*` プリミティブとドメイン側の "snapshot"（`TransportSnapshot`・`RtPlan` 等）は別概念であり、テレメトリレコードの schema 設計時は用語を区別する。

## rt_bridge 候補の形状

RT → 非 RT の有界・非ブロッキング観測レコード受け渡しの最小形を、既存 `retire_channel` 先例（固定容量リング + 共有 overflow カウンタ）を参照して設計する。実装ではなく形状と契約方針を本書内で確定する設計である。なお有界キューの棚卸し形状は他に `ruviform-exec` `offline` 内部の `sync_channel`（bounded MPSC）があるが、ブロッキング std プリミティブで overflow 計測を持たないため RT handoff の先例とはしない（#101 棚卸し）。

```rust
// スケッチ — 実装ではない。型名・シグネチャは輪郭の例示。
// 構築は非 RT のみ。capacity は構築時固定で、リング全体を事前確保する。
// R は Copy + Send + 'static: エンドポイントが RT/非 RT スレッドを跨ぐため
// （retire_channel の T: Send + 'static と同じ契約）。
fn bridge<R: Copy + Send + 'static>(capacity: usize) -> (RtRecorder<R>, RecordSink<R>);

// RT 側（作成は非 RT・メソッドのみ RT-suitable）:
impl RtRecorder<R> {
    /// wait-free・no-alloc・bounded。満杯時はレコードを破棄し
    /// 共有 overflow カウンタを +1（drop-on-full。後述の方針）。
    fn record(&mut self, rec: R);
    /// 取りこぼし累計。retire_channel の `overflows` と同じ共有カウンタ形。
    fn overflows(&self) -> usize;
}

// 非 RT 側:
impl RecordSink<R> {
    /// 1 呼出あたり capacity 上限の bounded drain。
    fn drain(&mut self, f: impl FnMut(R)) -> usize;
    fn overflows(&self) -> usize; // 非 RT 観測点
}
```

契約上の設計判断（本書内での確定事項。公開 API の確定ではなく、#104 PoC 結果・実装時設計による見直しを許容）:

- **オーバーフロー方針は drop-on-full + 計測。** `retire_channel` が leak-on-full（`mem::forget`）を選んだ理由は「満杯時に `T` のデストラクタ（dealloc を含み得る）を RT 側で走らせない」ためであり、`R: Copy` でデストラクタ・所有資源を持たない観測レコードにはその根拠が適用されない。破棄は許容するが取りこぼしは沈黙させず、共有カウンタで非 RT から観測可能にする（#101「取りこぼし数を非 RT から観測できることを重視する」）。破棄・集約・latest-wins の選択は API ごとの方針であり、`Consumer::pop_latest` の先例があるように受信側 latest-wins 変種は将来の API 追加で扱う。
- **容量は呼び出し側契約** — drain 周期あたりの最大レコード数をプロビジョンする（`retire_channel` の `capacity` 契約と同じ）。overflows の非ゼロ・増加は容量または drain 周期の不足を意味し、対処は運用側（容量増・drain 頻度増）であって API 変更ではない。
- **レコードは固定サイズ `Copy`** — `reserved: [u64; 2]` 規約に従う。可変長・`String`/`Vec` ペイロード・`Drop` を持つ型は RT 経路に乗せない。
- **タイムスタンプはレコード schema の一部とし、取得方法は #104 PoC に委ねる。** cpal `StreamInstant`→`MediaTime` マッピング（`ruviform-audio-device`）が RT 経路唯一の既存タイムスタンプ実装であり、時刻取得の syscall・初回初期化・ロック・計測オーバーヘッドは対象プラットフォーム別に検証する（#101「RT / 非 RT 観測データフロー」）。
- **基盤プリミティブの所有者は未確定のままとする** — リング実体は `foundation::sync` 候補の汎用 SPSC/bounded queue の提供範囲、レコード schema・カウンタは `foundation::telemetry` 候補、受け渡し機構の組立ては `observability::rt_bridge` 候補という #101 の構成候補上の分担を維持する。実配置は抽出トリガ成立時の個別設計で確定する。

## ドメインに残すもの

以下は Foundation/Observability 候補へ移さず、ドメインに常駐させる。

- **ドメイン別 Identity 型**: `PluginHandle`（`ruviform-plugin-host`・`{slot, generation}` 世代付き）、`StreamId`・`ParamId`（レジストリ newtype）、`DeviceId`（`ruviform-audio-core`・opaque `String`）。セマンティクスが異なり統合は偽の抽象化。`GuiHandle`・`GuiParentToken` も同様にドメイン境界の語彙として残す。
- **ドメイン意味を持つ Pool/Buffer**: 音声サンプル・動画フレームの Pool/Buffer の意味は各ドメインが所有する。汎用 Storage のみが抽出候補であり、その抽出自体は `memory` のトリガ待ち。
- **型付きエラー**: `HostError`・`DeviceError`・media-io `ErrorKind`・`OfflineError`・media-core `Error` 等は各 API の契約を所有するクレートに残す。Diagnostics は失敗の収集・説明を担い、Error 型を統合しない。
- **スナップショット交換の 2 系統**: `ruviform-audio` の `PlanSwap`/`RtPlan`（`plan.rs`・`RwLock`+`try_read`+`Arc` キャッシュ）は音声グラフ交換 ADR が選んだ意図的な分岐であり、`rt::slot` への統合はしない（同 ADR が `rt::slot` 自体を比較対象に含めていない点は #101 Sources 注記の既知事項であり、将来の ADR 改訂時の補完候補）。
- **クレート・バイナリ固有 instrumentation**: plugin の param mirror・`AtomicLifecycle`、Primitives 層 `exec` の `Progress`/`CancellationToken`、example の static カウンタは現行の所有者に残す。
- **ドメイン固有 Debug ツール**: Graph Inspector 等は利用ドメイン側に置く（#101「汎用 Debug ランタイムを初期必須基盤として固定しない」）。

## alloc_counter の所有者

`ruviform-rt` の `alloc_counter` は `#[cfg(test)]` 限定の private モジュール（`mod alloc_counter`）内にあり、`pub(crate)` 関数（`enable`/`disable`/`count`）経由でのみ到達するスレッドローカル計数 `GlobalAlloc` であり、テストバイナリ専用（#101 棚卸し・`alloc_counter.rs` 冒頭の契約記述どおり）。

**判定: `ruviform-rt` 内に留置する（#101 の延期行を維持）。** 再評価トリガは「test 専用枠を超える計測アロケータ需要」であり、`#[cfg(test)]` ゲートのためプロダクション利用者トリガは構造的に不成立する。理由と構造上の注意:

- `#[global_allocator]` はバイナリごとに 1 定義であり、ライブラリは「設定済みアロケータ」を提供できない。共有可能なのは `GlobalAlloc` アダプタ型（`System` への委譲 + 計数）のみで、install は各バイナリの権限。この非対称から、counting allocator の恒久的な置き場は原則「それを install するバイナリ」である。
- 同一パターン（手書きの計数 `GlobalAlloc` 実装）の実例は既に 2 つ存在する（rt の `alloc_counter` と `play_sine.rs` の `CountingAlloc`）が、テストバイナリと example バイナリの別バイナリであり相互消費できない。なお `assert_no_alloc` は各バイナリが外部クレートの機構を install する利用形であり、手書き計数アロケータの重複には数えない。重複は実在するが小さい — `alloc_counter.rs` は 84 行、`play_sine.rs` の counting 機構（static 群 + `GlobalAlloc` 転送 impl + install）は 55 行前後の単純な転送実装であり、共有の利益は小さい。
- トリガ成立時（3 つ目のバイナリでの同種需要、または `#[cfg(test)]` を超えた出荷計測機構の需要）の移動先候補は `observability::debug` 候補（RT 契約 probe の責務域）である。共有する場合も install しない `GlobalAlloc` 実装型の提供に限定し、グローバルアロケータ設定をライブラリから強制しない（#101 方針）。別の受け皿として dev-support 系クレートも比較対象となるが、現時点でその確定は行わない。

## 依存 DAG（新設時の方向）

候補クレートを実際に新設した場合に許容する依存方向を確定する。図は構成候補を既存層に置いた場合の許容エッジであり、新設自体を確定しない。

```text
既存（確定済み・変更なし）:
  Applications/Hosts → Engines/adapters → *-core → ruviform-media-core
  Primitives 層: ruviform-rt・ruviform-math（最下位・相互依存なし）、
    ruviform-media-core・ruviform-exec（同順位・相互依存禁止。baseline 確定）
    — 実在エッジは `*-core → ruviform-media-core`・`ruviform-exec → ruviform-rt` であり、
      ruviform-exec への依存側は現状存在しない

新設時に許容する方向:
  observability(候補) ──depends-on──▶ foundation(候補)   … 許容候補
  domain crates(*-core・engine・adapter) ──▶ foundation(候補)   … 下向き依存として許容
    （exec は Primitives 層のためここには含めない。rt 同様、新設時の依存可否は個別評価）
  domain crates ──▶ observability(候補) はイベント発行・ポート型に限定
                    （重い backend の必須依存は作らない）

禁止・非既定:
  foundation(候補) ──▶ observability(候補)              … 禁止（重い出力を基盤が強制しない）
  foundation(候補) ──▶ ドメイン・rt・exec・engine・plugin-*・adapter … 逆依存禁止
  foundation(候補) ⇄ ruviform-math                      … 相互依存禁止（math 独立維持）
  ruviform-rt ──▶ foundation(候補)                      … 非既定路線
```

- **`ruviform-rt → foundation` は既定路線ではない**（#101「依存方向と既存契約との整合」確定事項）。baseline は `ruviform-rt`・`ruviform-math` を最下位・相互独立としており、この依存を導入する場合は依存 DAG の再設計・公開 API/RT 契約評価・関連 approved design/accepted ADR の改訂が先行する。本設計はこの制約を前提に置き、rt 内プリミティブ（`channel`/`retire_channel`/`slot`・overflow カウンタ）への Foundation 側要素の適用はすべてこの非既定問題に従属する延期扱いとする。
- `observability → foundation` の依存は候補として許容する（Foundation が重いログ出力・profiler を強制依存に持たないことが条件）。
- `*-core` 等の Domain Foundations は observability backend を必須依存にせず、必要ならイベント発行やポート型に限定する。実際の集約・可視化は Host/Application 側が選ぶ（#101 方針）。
- 依存の循環・逆転、重い optional dependency の全利用者への伝播、Cargo features による意図しない統合の有効化は禁止・検証する（#101 確定事項の維持）。

## 後続 Issue 化の整理

本設計から派生する後続タスクと、その起票条件を整理する。**トリガ未成立のものは起票しない**（#101「後続 Issue と再着手条件」の管理方式を継続）。

- **起票済み・本設計へフィードバックするもの**
  - Issue [#104](https://github.com/scarecrow173/ruviform/issues/104)（[poc] RT 観測経路のコスト計測と方式評価）— prepared counter・固定サイズレコード・`rt_bridge` 候補の RT 実行時間/レイテンシ・オーバーフロー方針・タイムスタンプ取得方法・バックエンド採否を検証し、結果は本書の `telemetry`/`metrics`/`rt_bridge`/logging 系の「要 PoC」判定へ還流する。
- **トリガ成立時に起票するもの（現時点では起票しない）**
  - `ruviform-foundation` クレート新設 Issue — トリガ: いずれかの候補モジュールで本書の新設時期トリガ成立。
  - `ruviform-observability` クレート新設 Issue — トリガ: `rt_bridge` または logging/metrics/diagnostics の最初の実需要 + #104 PoC 結果。
  - `ruviform-rt → foundation` 依存導入のための DAG/ADR 改訂 Issue — トリガ: rt 内プリミティブを Foundation 側要素で扱う需要（`sync` 移動トリガまたは Counter 適用需要）の成立。依存導入の是非自体を判断する先行 Issue とする。
  - Counter プリミティブの既存サイト適用 Issue — トリガ: `foundation::telemetry` の実在 + 上記 DAG/ADR 問題の解決。
  - dev-instrumentation 共有（counting allocator の共有実装）Issue — トリガ: 3 つ目のバイナリでの同種需要、または `#[cfg(test)]` を超えた計測アロケータ需要。
- **本設計で完結するもの**
  - `reserved: [u64; 2]` のワークスペース規約の記録（共有型は作らない）
  - Counter プリミティブ化対象の選定基準（計数値のみ・mirror/flag/状態機械/時刻/test instrumentation は対象外）
  - `alloc_counter` の留置判断と移動先候補の記録

## Risks and Trade-offs

- **保留判定が続くリスク**: トリガ条件ベースの管理は、実需要が曖昧なまま低頻度で現れた場合に各サイトで局所実装が育つリスクを持つ。緩和として、本書は「もし抽出するなら」の最小形をモジュールごとに記録しており、トリガ成立時の設計着手点を固定済みとする。
- **`rt → foundation` 非既定依存の連鎖**: この制約が `sync` 移動と Counter 適用の双方を構造的に遅延させる。これは #101 で意図的に確定した制約であり、本設計は緩めない。解除には baseline・ADR 改訂を伴う別 Issue が要ることを「後続 Issue 化の整理」で明示した。
- **スケッチと実装の混同**: 本書のコード片は形状の例示であり公開 API 確定ではない。`foundation`/`observability` のクレート・モジュール名は引き続き構成候補であり、実装済みと誤認しない旨を目的節と各判定で明示した。

## Deferred Design Concerns

- `channel`/`retire_channel`/`slot` の Foundation 移動可否 | 外部プロダクション利用者ゼロ・内部依存連鎖あり | 2 番目のプロダクション利用者の出現 | 局所重複の育成
- `identity`/`memory`/`collections`/`logging`/`profiling`/`diagnostics`/`debug`/`export` の新設 | 所要前提なし（棚卸しに消費者・実装が存在しない） | 各判定表のトリガ成立 | 想定外の需要変化で再設計
- `telemetry`/`metrics`/`rt_bridge` の実装着手 | バックエンド採否・RT コストが未計測 | Issue #104 PoC 結果 + 最初の実需要 | 未検証方式の先行固定
- `alloc_counter` の共有化・移動 | バイナリ単位 install の構造的非対称・重複は小さい | 3 つ目のバイナリ需要または test 専用枠を超える需要 | 各バイナリでの局所コピー継続

## Verification Notes

- [x] 棚卸し事実との整合: `channel`/`retire_channel`/`slot` の外部プロダクション利用者ゼロ、`alloc_counter` の `#[cfg(test)]` 限定、`reserved` 5 型、Atomic サイトの所在地を crates/ 実コードと照合済み（observability 系依存の非存在は #101 棚卸し後に追加された `ruviform-edit` を含む全 19 クレートで確認）
- [x] `rt → foundation` が非既定依存である旨を依存 DAG 節に明示し、rt 内要素への Foundation 側適用をすべて従属延期扱いとした
- [x] #101 の延期行（sync 移動・Identity/Memory 汎用化・alloc_counter 所有者）のトリガを本書で維持・具体化した
- [x] `doc_status` audit で findings 0（frontmatter・index 整合）

## Sources

- [共通基盤・Observability・クレート分割の設計方針](shared-foundations-and-modularization-policy.md)（Issue #101・approved。棚卸し・責務区分・延期行の正本）
- [Ruviform architecture design baseline](ruviform-architecture-design-baseline.md)（層構造・依存方向の正本）
- [RT 契約と検証方針](../adr/realtime-contract-and-verification-policy.md)・[RT 基盤プリミティブ実装方式](../adr/rt-foundation-primitives-implementation.md)・[assert_no_alloc 標準化・thread-check 意味論](../adr/rt-assert-no-alloc-thread-check.md)・[音声グラフ交換 ADR](../adr/audio-engine-snapshot-swap.md)
- [RT 検証レーン設計](rt-verification-lane-design.md)（`alloc_counter`/`assert_no_alloc` 併存ルール）
- 棚卸し対象コード: `crates/ruviform-rt/src/{spsc,retire,snapshot,clock,alloc_counter}.rs`、`crates/ruviform-audio/src/plan.rs`、`crates/ruviform-plugin-host/src/command.rs`、`crates/ruviform-media-core/src/transport.rs`、`crates/ruviform-plugin-core/src/{event,processor,transport,param}.rs`、`crates/ruviform-audio-core/src/device.rs`、`crates/ruviform-exec/src/offline.rs`、`crates/ruviform-plugin-clap/src/adapt.rs`、`crates/ruviform-plugin-vst3/src/{component,controller}.rs`、`crates/ruviform-audio-device/examples/play_sine.rs`
- Issue: [#101](https://github.com/scarecrow173/ruviform/issues/101)・[#103](https://github.com/scarecrow173/ruviform/issues/103)・[#104](https://github.com/scarecrow173/ruviform/issues/104)・[#76](https://github.com/scarecrow173/ruviform/issues/76)・[#22](https://github.com/scarecrow173/ruviform/issues/22)
