---
id: "DESIGN-SHARED-FOUNDATIONS-20261023"
type: "design"
status: "approved"
title: "共通基盤・Observability・Diagnostics・Debug とクレート分割の設計方針"
created: "2026-10-23"
updated: "2026-10-23"
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
  depends-on:
    - "ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/rt-foundation-primitives-implementation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "overview.md"
    - "../adr/audio-engine-snapshot-swap.md"
    - "plugin-isolation.md"
  refines:
    - "ruviform-architecture-design-baseline.md"
  refined-by: []
  derives-from:
    - "ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://doc.rust-lang.org/cargo/reference/features.html"
    - "https://doc.rust-lang.org/cargo/reference/workspaces.html"
    - "https://github.com/bevyengine/bevy/tree/main/crates/bevy_utils"
    - "https://github.com/bevyengine/bevy/tree/main/crates/bevy_log"
    - "https://github.com/bevyengine/bevy/tree/main/crates/bevy_diagnostic"
  defers: []
  deferred-by: []
---

# 共通基盤・Observability・Diagnostics・Debug とクレート分割の設計方針

## 目的と位置づけ

Ruviform の基礎ライブラリを DAW・再生ホスト・プラグイン制作・音声解析・動画処理・NLE から独立利用できるように維持しつつ、横断的な機能を段階的に拡張できる基盤設計を定義する。

**本書で確定するのは、責務境界・依存方向・段階的なクレート分割の判断規則であり、個別のクレート新設・API・依存ライブラリの採用を確定するものではない。** `ruviform-foundation`、`ruviform-observability`、`ruviform-debug`、`ruviform-platform` 等は、下記で明示した場合を除き**候補名・将来構成**であり、実装済みのクレートとして扱わない。

既存の [全体設計ベースライン](ruviform-architecture-design-baseline.md) と accepted ADR（特に [RT 契約](../adr/realtime-contract-and-verification-policy.md)、[RT プリミティブ実装](../adr/rt-foundation-primitives-implementation.md)、[グラフ交換](../adr/audio-engine-snapshot-swap.md)）を上書きしない。既存の配置・依存制約・採用済み方式を変更する際は影響分析と必要な ADR の改訂を行う。コード・workspace 構成・Issue は本書だけでは変更しない。

## 設計原則

1. **レイヤー・クレート・モジュールを区別する。** レイヤーは依存方向とポリシーを、クレートは独立したコンパイル・依存・公開境界を、モジュールは同一クレート内の凝集と可視性を表す。1 対 1 対応は要求しない。
2. **依存は、上位の利用側から安定した下位の契約へ向ける。** ポート（trait・値型）とバックエンドを分離し、基礎クレートがメディアエンジン・プラグインホスト・GUI・外部デバイスに逆依存しない。
3. **まず責務別モジュールとして設計し、独立性を確認してクレート化する。** コード行数・将来の予想だけを分割理由にしない。ただし OS、外部依存、RT 契約、安全性監査、独立配布の条件が異なれば小さくても早期分離を検討する。
4. **共通基盤は寄せ集めの置き場にしない。** 単に二箇所で使われることだけで Foundation に移さない。共通化は意味・不変条件・所有権・失敗ポリシーが一致する範囲に限る。
5. **既存の `core` 系と `math` は尊重する。** `ruviform-media-core`、`ruviform-audio-core`、`ruviform-video-core`、`ruviform-plugin-core` はドメイン固有の基礎であり、汎用 `foundation` と区別する。`ruviform-math` は独立維持を基本とし、巨大 `ruviform-core` の再導入は採用しない。
6. **RT 安全性は API・所有権・ライフサイクル全体の契約。** 基礎キューの非ブロッキング性だけで任意のペイロードの Drop や利用側コードまで RT-safe とみなさない。
7. **観測と出力を分離する。** 計測・障害情報の低コスト記録は可能な限り独立させ、フォーマット・集約・I/O・外部ツール連携は非 RT 側で行う。
8. **外部ライブラリを再利用し、独自抽象は必要な保証に限定する。** `std`・`core`・`alloc`、既存の Rust クレートを重複実装しない。依存採用にはライセンス、MSRV、保守性、ターゲット、unsafe、ビルドコストの評価が必要。
9. **本書は原則の正本、具体的な配置・実装は個別設計と ADR で確定する。** 未確定の API や機能を実装済みと記述しない。

設計の考え方は Clean Architecture の依存方向・Ports & Adapters と整合するが、これはアプリケーションのビジネスルールを中心に置く構成そのものではない。Ruviform では再利用可能な処理契約、ドメインモデル、RT 特性を保護するために適用する。

## 全体構造の目標（概念上）

```text
Applications / Hosts / Integrations
    │
    ├── Engines (audio / video / edit 等)
    ├── Plugin host / UI / media I/O / device / GPU adapters
    │
    ├── Domain foundations
    │     media-core / audio-core / video-core / dsp / video-proc /
    │     audio-analysis / plugin-core / media-io
    │
    ├── Execution contracts and services
    │     rt / exec
    │
    ├── Observability & developer support [候補]
    │     observability (logging / profiling / metrics /
    │                    diagnostics / debug probes)
    │     debug tools [必要になれば別クレート]
    │
    └── Generic foundations
          math [既存・独立]
          foundation [候補: collections / memory / sync /
                      identity / telemetry]
```

上図は階層の概念図であり Cargo DAG の全エッジを表すものではない。`observability` と `exec` は必ずしも上下関係に固定しない。ポートを定義する下位の所有者と、実装を提供する上位のアダプタを個別に指定する。

### クレートの役割と状態

| 領域 | 所有する責務 | 所有しないもの | 状態 |
| --- | --- | --- | --- |
| `ruviform-foundation` | メディア非依存のデータ構造、メモリ・同期・識別子・軽量テレメトリの契約 | OS プロセス制御、重いログ出力、メディア・プラグインの意味 | **新設候補** |
| `ruviform-observability` | ログ設定、計測の集約、Profiling、Metrics、Diagnostics の記録と出力、軽量 Debug probe の統合 | RT パス内での任意フォーマット・I/O、ドメイン固有のエラー型の所有 | **新設候補** |
| `ruviform-debug` 等 | 状態検査、グラフ/タイムライン可視化、記録・再現等の重い開発支援 | Foundation の必須依存、常時実行が必須な処理 | **必要時の抽出候補** |
| `ruviform-platform` / `ruviform-process` / `ruviform-ipc` 等 | 共通 OS 資源・プロセス間通信の抽象と寿命管理 | 音声デバイス等のドメイン固有 API、プラグイン固有プロトコル | **需要・PoC 後に判断** |
| `ruviform-math` | 数値・SIMD 処理 | 汎用メモリ管理・ログ・OS | **既存維持** |
| `ruviform-rt` | `Processor` / `RtProcessor`、`RtContext`、共有クロック、RT 固有の回収・スナップショット契約 | 汎用 OS サービス、重い観測・出力 | **既存維持・必要時に内部抽出を検討** |
| `ruviform-exec` | オフライン駆動・非 RT 実行・既存のスレッド優先度 | 汎用メモリコレクション | **既存維持** |
| `*-core` / DSP / engines / adapters | 各ドメインの型・処理・実行計画・外部実装 | 汎用 Foundation の雑多な再実装 | **既存境界を原則維持** |

## Foundation のモジュール候補

各モジュールは内部境界を保ち、相互に必要以上の `pub(crate)` 参照を作らず、独立抽出可能な API を意識する。以下は設計対象の一覧であり、モジュールの実装を直ちに要求しない。

| モジュール | 主な責務 | 初期の設計上の注意 |
| --- | --- | --- |
| `collections` | 固定容量配列・リング・事前確保コンテナ | `arrayvec` / `heapless` 等を比較。コンテナ単独の Drop まで RT 保証しない |
| `memory` | アラインメント、事前確保、Storage/Pool/Arena、メモリ使用量の計測点 | `std::alloc` を優先。グローバルアロケータをフレームワークが強制しない |
| `sync` | プロセス内 SPSC・必要に応じ bounded queue、cache padding 等 | 既存 `ruviform-rt` の `rtrb` ラップを検証。プロセス間共有メモリとは別契約 |
| `identity` | 世代付きスロット、汎用ハンドル構成部品 | `PluginHandle` や `NodeId` のドメイン別 newtype は維持 |
| `telemetry` | 固定サイズのイベントレコード、Atomic Counter、記録用リング、オーバーフロー観測 | 文字列整形、Subscriber、export、ファイル I/O を含めない |

### Memory と所有権

事前確保した Pool/Arena を提供する場合は、構築・借用・返却・再利用・破棄・枯渇・シャットダウンを別フェーズとして仕様化する。`release` が固定時間であってもユーザーの `T: Drop` が RT で安全とは限らない。音声サンプルや動画フレームの意味を持つ Pool/Buffer はドメイン側が所有し、汎用 Storage のみ Foundation で共通化する。

`#[global_allocator]` のようなプロセス全体の設定は利用アプリケーション・ホストに委ねる。ライブラリが暗黙に差し替えない。

### Sync とプロセス間通信

`foundation::sync` のキューは原則 **同一プロセス内**のメモリ所有権・同期を扱う。共有メモリ上のレイアウト、相対オフセット、ABI、クラッシュ後の回収、プロセス間通知は別途 System Services / IPC の所有とする。キューが wait-free であっても、容量不足時の返却値の Drop、スナップショット参照の最終解放、終了処理などは別の契約が必要。

既存 `ruviform-rt::channel` を Foundation へ移動するのは**保留**。汎用利用者の実在、RT 側互換性、依存 DAG、公開 API の影響を評価してから決定する。`ruviform-rt` の退役オブジェクト回収とスナップショットは RT 特有の契約を維持する。

## Observability・Diagnostics・Debug の責務区分

これらは同義ではない。まず機能上の概念として区別し、初期は `ruviform-observability` のモジュール候補として管理する。重い Debug ツールの別クレート化は必要になってから決める。

| 領域 | 目的 | 所有する内容 | RT 上の扱い |
| --- | --- | --- | --- |
| **Logging** | 人間・運用者向けの事象の記録 | 重要度、コンテキスト、フィルタ、出力 | メッセージ生成・整形・出力は非 RT |
| **Profiling** | どこで時間・リソースを消費したか | span・計測区間・サンプル・タイムライン・外部 profiler 連携 | RT 計測は独立検証した軽量記録のみ |
| **Metrics** | 集計可能な状態・傾向の監視 | xrun、遅延、キュー飽和、Drop 退避、プール使用量等 | 事前確保済みカウンタ/レコードを使用 |
| **Diagnostics** | 異常の原因・健全性・対処可能性の提示 | 構造化診断、事象 ID、severity、context、状態チェック・診断レポート | RT では固定容量の事実記録のみ。解析・説明は非 RT |
| **Debug** | 開発・不具合再現・内部状態の検証 | invariant checks、Graph/Session inspection、状態 dump、capture/replay、検証用 hooks | 明示的な Debug モードと局所コスト評価。重い処理は禁止 |

**Diagnostics と Error の違い:** ドメインの `Result<T, E>` / `Error` は、その API の契約を所有するクレートが定義する。Diagnostics は失敗の原因・経緯・動作状況を構造化して収集・説明する仕組みであり、すべての Error を Foundation の巨大 enum に統合しない。

**Diagnostics と Metrics の違い:** Metrics は値の集計、Diagnostics は異常の解釈・原因の切り分け・必要なコンテキストを扱う。

**Debug と Diagnostics の違い:** Diagnostics は本番動作中にも必要な健全性情報を含むが、Debug には高コストな内部状態の検査・検証・再現支援を含む。Debug 専用機能に本番利用者を強制依存させない。

### Observability の候補モジュール

```text
ruviform-observability [候補]
├── logging          # tracing 等への統合 / sink / filtering
├── profiling        # duration / span / external profiler adapters
├── metrics          # counters / gauges / histograms / aggregation
├── diagnostics      # structured diagnostics / health / reports
├── debug            # lightweight debug probes / invariant-report integration
├── rt_bridge        # bounded RT records -> non-RT collector
└── export           # console / file / trace / external integrations
```

候補ライブラリ：`tracing`、`tracing-subscriber`、`metrics`、`tracy-client` 等。採用は個別 PoC とライセンス・MSRV・依存木・RT コスト評価による。Ruviform が独自の Log/Span/Metrics フレームワークを重複実装しない。**ライブラリの側でグローバル Subscriber・Logger・Recorder を勝手に初期化・変更しない。** 最終的な初期化と出力設定は Host/Application 側が所有する。

### Debug の検討対象

- **Invariant validation:** Graph の接続、Buffer の範囲、タイムスタンプ単調性、世代 ID、スレッド契約など。検査が高コストなら非 RT の検証 API として公開する。
- **State inspection / dump:** GraphPlan、Session、Plugin 状態・パラメータ、使用メモリ・キュー、トランスポートを構造化して読み取る。所有権を壊さず、スナップショットの取得は非 RT 制御経路とする。
- **Event capture / deterministic replay:** 実際に再現性が必要になったドメインから設計する。壁時計・スレッド間競合・外部プラグイン・I/O の非決定性を明示し、「記録したら必ず完全再現できる」とは主張しない。
- **RT contract probes:** alloc/dealloc 検出、容量超過、最悪実行時間の観測、Drop 退避など。RT 本番経路と CI 検証レーンの範囲を区別する。
- **Crash / hang investigation:** 非 RT 側でのバックトレース、子プロセス異常終了情報、必要なら dump 連携。クラッシュ処理を RT コールバックへ混入させない。
- **Privacy / security:** 状態 dump や記録に音声素材、パス、プラグイン状態、秘密情報が含まれ得るため、収集範囲と保存を明示 opt-in とする。

汎用 Debug ランタイムを初期必須基盤として固定するのではなく、共通の診断イベント形式・検査ポートを先に揃える。Graph Inspector 等のドメイン固有ツールは利用ドメインに置き、共通レンダラや開発支援が実際に必要になれば `ruviform-debug` 等を抽出する。

## RT / 非 RT 計測データフロー

```text
RT callback / plugin DSP
   ├── atomic counter / prepared fixed-size event
   └── bounded, no-wait handoff (overflow は記録・集計)
                   │
                   ▼
            non-RT collector
                   │
         ┌─────────┼──────────┐
         ▼         ▼          ▼
       Logging   Metrics   Profiling
                   │
             Diagnostics / Debug UI
```

RT 経路では alloc/dealloc、blocking I/O、任意の文字列フォーマット、可変長で上限不明の処理、サブスクライバ/コールバック経由の未知の処理を禁止する。`tracing` のマクロや非同期出力方式であっても、その呼び出し経路を検証せず RT-safe としない。

イベントの時刻取得が syscall、初回初期化、ロック、計測オーバーヘッドを伴うかはターゲット別に検証する。必要な記録領域・バッファは RT 区間前に準備する。記録が満杯なら、**ドロップ / 集約 / 失敗記録を API ごとに定義**し、RT 側でのブロッキングや暗黙のメモリ解放を避ける。取りこぼし数を非 RT から観測できることを重視する。計測有効・無効の双方でレイテンシを評価する。

## 依存境界

- `foundation` はメディアドメイン、`rt`、`exec`、`observability`、`plugin-*`、OS adapter に依存しない。`core` / `alloc` / 必要な `std` と監査した最小の外部依存だけを持つ。
- `math` は独立維持し、Foundation との相互依存を作らない。共通数値ユーティリティの所有者は重複しないよう個別決定する。
- `rt` は既存公開 API と実装契約を維持する。Foundation に抽出後の一方向依存は候補だが、既存 ADR の制約を先に変更せず実装しない。
- `observability` は Foundation の軽量データを利用してよい。ただし Foundation が Observability に依存してはならない。
- Domain Foundations は外部ログ出力 backend を必須依存にせず、必要ならイベント発行やポート型に限定する。実際の集約・可視化は Host/Application 側が選ぶ。
- `exec` にあるスレッド優先度は当面維持する。OS に依存する共通サービスが複数箇所で必要になれば、`platform` または機能別 system crate を比較する。
- 音声デバイスは `audio-device`、メディア I/O backend は `io-*`、GPU 等は各 adapter が所有する。OS コードだからという理由だけで全て `platform` へ移さない。
- Plugin L1 のプロセス管理・共有メモリと、プラグイン固有コマンド・ライフサイクルは分ける。プロセス内 SPSC をプロセス間共有メモリ用キューとして無条件に再利用しない。
- 依存の循環・逆転、重い optional dependency の全利用者への伝播、feature 統合による意図しない機能有効化を禁止・検証する。

## クレート抽出の条件

モジュールからクレートへの抽出は**行数の閾値ではなく次の根拠**によって判断する。

| 分割判断の軸 | 分割を支持する具体的状況 |
| --- | --- |
| 独立利用 | 単体の public API だけで意味があり、別の利用者が存在する |
| 依存・可搬性 | 独立させると OS / FFI / 重い外部依存 / `no_std` 条件を分離できる |
| 契約 | RT・非 RT、安全性、メモリ所有権、失敗モードが異なる |
| 開発・互換性 | 独立した API 進化・検証・バージョニングが必要 |
| 安全性 | unsafe / ABI / 共有メモリの監査範囲を隔離できる |
| ビルド | Cargo check / incremental / CI の実測で改善がある |

分割しない条件：実装詳細のためだけに `pub` を増やす、常に同時変更になる、依存 DAG が複雑化するだけ、利用者も保証契約も未定、という場合。モジュール分割・可視性の改善で十分ならクレートを増やさない。

既存モジュールの抽出時は公開パスを可能な範囲で re-export して互換性を保持する。ただし公開型・trait の所有元や features、MSRV が変わる影響は別途検証する。`cargo-semver-checks`、ターゲット別テスト、RT 検証、Cargo 依存木・ビルド性能の測定を候補とする。

## 適用例と設計上の検証項目

1. **Audio xrun / plugin timeout:** RT 側はカウンタまたは固定レコードを送出し、非 RT が原因・時刻・関連ノード・経過を Diagnostics として組み立てる。Graph 遅延の意味づけは audio engine が所有する。
2. **Memory Pool exhaustion:** Pool の容量・使用数の計測点は Foundation 候補、実際の audio/video buffer の意味とフォールバックはドメイン側が所有する。RT 側でログ文面を作成しない。
3. **Plugin L1 crash:** Child の exit/監視は System Services / L1 側、`Failed` 状態遷移は plugin-host、共有プロセス資源回収と診断レポート作成は非 RT とする。
4. **Graph inspection:** 音声・動画の固有状態は各 engine 側に保持し、スナップショットを取得してから非 RT の Debug tool で図示・保存する。

## 決定／保留／要 PoC

| 論点 | 区分 | 方針／確定条件 |
| --- | --- | --- |
| 小さな汎用基盤を先にモジュール設計し、必要時にクレート抽出 | **設計方針** | 採用。行数だけで機械的に分割しない |
| Foundation と Domain Core の分離 | **設計方針** | 採用。巨大な `ruviform-core` へ統合しない |
| Logging / Profiling / Metrics / Diagnostics / Debug の責務区分 | **設計方針** | 採用。重い出力と低コスト記録は分離 |
| `ruviform-foundation`・`ruviform-observability` の新設・API 範囲 | **保留** | 実際に必要な型と依存関係を設計してから Issue 化 |
| SPSC・Identity・Memory の既存実装からの移動 | **要 PoC / 互換性検討** | 既存 ADR・公開 API・RT 契約と照合 |
| `tracing` / `metrics` / profiler backend 採用 | **要 PoC** | RT 記録経路・依存・MSRV・運用コストの評価 |
| Debug の独立クレート化、Capture/Replay 仕様 | **保留** | ドメインをまたぐ具体的な利用需要が生まれたら |
| 共通 `platform` / `ipc` / `process` の独立 | **要 PoC** | L1 を含む複数用途、OS 互換性、ABI・資源寿命を検証 |

## 適用・移行順序

1. **設計整合:** 既存 18 クレートと accepted ADR の依存/公開 API/実行契約を棚卸しする。本書の候補を実装済みと誤認しない。
2. **Foundation 候補の最小設計:** Memory / Sync / Identity / Telemetry の必要な共通操作と、既存実装を流用できる範囲を確定する。
3. **Observability 候補の最小設計:** Logging・Metrics・Diagnostics・Profiling・Debug probe の責務と、RT → 非 RT の記録境界を設計する。
4. **PoC と採否:** キュー容量・Drop・スナップショットの寿命、RT 計測コスト、外部依存/OS/ABI を検証する。
5. **実装・移行:** 利益が確認できるモジュール/クレートだけ実装し、re-export・契約テスト・CI・依存 DAG 検証を伴って段階的に移行する。

新規クレート追加、公開 API の変更、accepted ADR を変更する際は、実装 Issue または ADR で判断根拠・影響・検証範囲を記録する。

## 作業時に参照する条件

`AGENTS.md` から本書を参照するのは、共通 Memory/Sync/Collections/Identity、Logging/Profiling/Metrics/Diagnostics/Debug、RT 計測、IPC/OS、クレート分割、依存 DAG・API 所有者・feature 境界を変更・新設・レビューするときとする。無関係の変更時に全読を義務化しない。

## Sources

- [Ruviform architecture design baseline](ruviform-architecture-design-baseline.md)
- [RT 契約と検証方針](../adr/realtime-contract-and-verification-policy.md)
- [RT 基盤プリミティブ実装方式](../adr/rt-foundation-primitives-implementation.md)
- [音声グラフ交換 ADR](../adr/audio-engine-snapshot-swap.md)
- [Bevy utilities](https://github.com/bevyengine/bevy/tree/main/crates/bevy_utils)、[Logging](https://github.com/bevyengine/bevy/tree/main/crates/bevy_log)、[Diagnostics](https://github.com/bevyengine/bevy/tree/main/crates/bevy_diagnostic)
- [Cargo Features](https://doc.rust-lang.org/cargo/reference/features.html)、[Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
