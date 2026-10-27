---
id: "DESIGN-034Kx6CVMPDNgzKhJwfgSp"
type: "design"
status: "approved"
title: "共通基盤・Observability・Diagnostics・Debug とクレート分割の設計方針"
created: "2026-10-22"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/101"
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
    - "../adr/audio-engine-snapshot-swap.md"
    - "../adr/rt-assert-no-alloc-thread-check.md"
    - "rt-verification-lane-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "overview.md"
    - "plugin-isolation.md"
  refines:
    - "ruviform-architecture-design-baseline.md"
  refined-by: []
  derives-from:
    - "ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/rt-foundation-primitives-implementation.md"
  derived-by:
    - "foundation-observability-minimal-api.md"
    - "issue-122-shared-platform-process-ipc-eval.md"
    - "../plans/implement-issue-104-rt-observability-poc.md"
    - "../test-specs/rt-observability-poc-tests.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/17"
    - "https://github.com/scarecrow173/ruviform/pull/100"
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

Ruviform の基礎ライブラリを DAW・再生ホスト・CLAP/VST3 プラグイン制作／ホスト・DSP・オフライン解析・将来の動画編集／NLE から独立利用できるように維持しつつ、Memory・Sync・Logging・Profiling・Metrics・Diagnostics・Debug 等の横断的な基盤を段階的に拡張できるよう、責務境界・依存方向・分割判断の設計方針を定義する。一次ソースは Issue [#101](https://github.com/scarecrow173/ruviform/issues/101)。

**本書で確定を目指すのは、責務境界・依存方向・段階的なクレート分割の判断規則であり、個別のクレート新設・公開 API・依存ライブラリ・既存実装の移動を確定するものではない。** `ruviform-foundation`、`ruviform-observability`、`ruviform-debug`、`ruviform-platform` 等は、下記で明示した場合を除き**構成候補の名前**であり、実装済みのクレートとして扱わない。新クレートの新設、新しい公開 API、バックエンド追加、既存実装の移動は本書の確定範囲外であり、必要に応じて後続 Issue で個別に判断する。

既存の [全体設計ベースライン](ruviform-architecture-design-baseline.md)（approved）と accepted ADR（特に [RT 契約](../adr/realtime-contract-and-verification-policy.md)、[RT プリミティブ実装](../adr/rt-foundation-primitives-implementation.md)、[グラフ交換](../adr/audio-engine-snapshot-swap.md)）を暗黙に上書きしない。既存の配置・依存制約・採用済み方式と矛盾する変更が必要になった場合は、差分を明示し影響分析と必要な ADR 改訂を別途行う。コード・workspace 構成・Issue は本書だけでは変更しない。

## 状態区分の凡例

本書では各論点を以下の 5 区分で管理し、候補と確定事項を混同しない。

- **方針**: 本書の設計方針として採用する事項。後続の設計・実装 Issue はこれを前提に進める。
- **構成候補**: 責務領域・クレート名・モジュール名の候補。名前の提示は採用や実装の確定を意味しない。
- **保留**: 判断材料が不足する、または後続 Issue・PoC に権限が属する事項。再着手の条件を併記する。
- **要 PoC**: 採否が PoC・ベンチマーク・互換性検証の結果に依存する事項。判断材料と採否基準を併記する。
- **実装済み**: 既存のクレート・API・accepted ADR で確定済みの事項。原則として維持し、変更には影響分析を要する。

## 設計原則

1. **責務境界と依存方向を先に設計する。** 汎用処理契約・ドメインモデル・RT 特性の所有者を明確にし、利用側から契約を定義する側へ依存させる。
2. **レイヤー・クレート・モジュールを区別する。** レイヤーは依存方向とポリシーを、クレートは独立したコンパイル・依存・公開境界を、モジュールは同一クレート内の凝集と可視性を担う。1 対 1 対応は要求しない。
3. **モジュール先行・必要時のクレート抽出。** まず責務別モジュールとして設計し、独立性を確認してクレート化する。コード行数・将来の予想だけを分割理由にしない。ただし OS、外部依存、RT 契約、安全性監査、独立配布の条件が異なれば小さくても早期分離を検討する。
4. **共通基盤は寄せ集めの置き場にしない。** 単に二箇所で使われることだけで Foundation に移さない。共通化は意味・不変条件・所有権・失敗ポリシーが一致する範囲に限る。
5. **既存の `core` 系と `math` は尊重する。** `ruviform-media-core`、`ruviform-audio-core`、`ruviform-video-core`、`ruviform-plugin-core` はドメイン固有の基礎であり、汎用 `foundation` と区別する。`ruviform-math` は独立維持を基本とし、巨大 `ruviform-core` の再導入は採用しない（Issue #17 で仮置き `ruviform-core` は分割・置換済み）。
6. **RT 安全性は API・所有権・ライフサイクル全体の契約。** 基礎キューの非ブロッキング性だけで任意のペイロードの Drop や利用側コードまで RT-safe とみなさない。
7. **軽量な記録と重い観測処理を分離する。** RT からの記録は事前準備済み・容量有界・非ブロッキングで、API 単位に検証したものだけに限定する。ログ整形・集約・可視化・出力は非 RT 側で扱う。
8. **外部ライブラリを再利用し、独自抽象は必要な保証に限定する。** `std`・`core`・`alloc`、既存の Rust クレートを重複実装しない。依存採用にはライセンス、MSRV、保守性、ターゲット、unsafe、ビルドコストの評価が必要。不要な独自ロガーや汎用 Platform／IPC 抽象を先行実装しない。
9. **本書は原則の正本、具体的な配置・実装は個別設計と ADR で確定する。** 未確定の API や機能を実装済みと記述しない。

設計の考え方は Clean Architecture の依存方向・Ports & Adapters と整合するが、これはアプリケーションのビジネスルールを中心に置く構成そのものではない。Ruviform では再利用可能な処理契約、ドメインモデル、RT 特性を保護するために適用する。

## 全体構造の目標（概念上）

```text
Applications / Hosts / Integrations        （利用側。本設計の境界外）
        │
        ▼
┌─ Engines / 拡張層（Plugin・UI）/ Optional platform adapters ─┐
│ Engines:   ruviform-audio・ruviform-video                     │
│ 拡張層:    ruviform-plugin-core → ruviform-plugin-host・      │
│            ruviform-plugin-export（未実装）→                  │
│            ruviform-plugin-clap・ruviform-plugin-vst3         │
│            ruviform-ui（未実装）→                             │
│            ruviform-ui-webview（未実装）                      │
│ Adapters:  ruviform-audio-device・ruviform-gpu（未実装）・    │
│            ruviform-io-*（ruviform-io-symphonia 等）・        │
│            ruviform-ui-webview（未実装）                      │
└──┬───────────────────────────────────────────────────────────┘
   ▼ Domain foundations
┌──────────────────────────────────────────────────────────────┐
│ ruviform-audio-core・ruviform-video-core                     │
│   → ruviform-dsp・ruviform-video-proc・ruviform-media-io     │
│   → ruviform-audio-analysis                                  │
└──┬───────────────────────────────────────────────────────────┘
   ▼ Primitives（最下位）
┌──────────────────────────────────────────────────────────────┐
│ ruviform-math・ruviform-rt（最下位・相互依存なし）           │
│   → ruviform-media-core・ruviform-exec                       │
└──────────────────────────────────────────────────────────────┘

構成候補（構成仮説であり baseline 層内の確定依存位置ではない）:
  ruviform-foundation      collections / memory / sync /
                           identity / telemetry
  ruviform-observability   logging / profiling / metrics /
                           diagnostics / debug probes /
                           rt_bridge / export
  ruviform-debug・ruviform-platform・ruviform-process・
  ruviform-ipc
```

上図は [全体設計ベースライン](ruviform-architecture-design-baseline.md) の層構造・層内順序に厳密に合わせた概念図であり、Cargo DAG の全エッジを表すものではない。縦方向は依存方向（上位 → 下位）を示し、`ruviform-math`・`ruviform-rt` は最下位・相互依存なし（baseline 確定済み）である。下段の構成候補は領域の分類を示す構成仮説であり baseline 層に混在させる確定依存位置ではない。`ruviform-observability` と `ruviform-exec` のような候補と既存クレートの上下関係は固定せず、`ruviform-rt → ruviform-foundation` のような新規依存は既定路線ではない（後述「依存方向と既存契約との整合」参照）。ポートを定義する所有者と、実装を提供するアダプタを個別に指定する。ポート定義が上位層にある場合の依存も baseline の規則どおり合法である（例: `ruviform-ui-webview` → `ruviform-ui` は WebView ホスト境界の定義元への依存として許容）。図中の `（未実装）` は baseline クレート表で計画済みだがコード棚卸し時点でクレートとして存在しない名前（`ruviform-plugin-export`・`ruviform-ui`・`ruviform-ui-webview`・`ruviform-gpu`）を示し、実装済みクレートおよび本書の構成候補名と区別して読むこと。

### 領域ごとの構成候補と採否状態

| 領域 | 担う機能の候補 | 所有しないもの | 状態区分 |
| --- | --- | --- | --- |
| `ruviform-foundation` | メディア非依存のデータ構造、メモリ・同期・識別子・軽量テレメトリの契約（`collections`・`memory`・`sync`・`identity`・`telemetry`） | OS プロセス制御、重いログ出力、メディア・プラグインの意味 | **構成候補**（新設時期・API は**保留**） |
| `ruviform-observability` | ログ設定、計測の集約、Profiling、Metrics、Diagnostics の記録と出力、軽量 Debug probe・RT → 非 RT 観測レコード転送（`rt_bridge`・構成候補上の所有者）・export | RT パス内での任意フォーマット・I/O、ドメイン固有エラー型の所有 | **構成候補**（実際の分割・バックエンドは**保留**） |
| `ruviform-debug` 等 | 不変条件検査、内部状態 inspection、構造化 dump。Graph Inspector／Capture & Replay／Crash Investigation は将来候補 | 他クレートへの必須依存としての組込、常時実行を前提とする機能 | **構成候補**（重い機能の独立クレート化は**保留**） |
| `ruviform-platform` / `ruviform-process` / `ruviform-ipc` 等 | プロセス管理・共有メモリ・IPC・共通 OS 資源の抽象と寿命管理 | 音声デバイス等のドメイン固有 API、プラグイン固有プロトコル | **構成候補**（独立クレートか機能別モジュールかは**要 PoC**。Issue #76 等の要件／PoC を受けて判断） |
| `ruviform-rt`・`ruviform-exec` | RT 契約、非 RT 実行、退避・スナップショット、スレッド優先度 | 汎用 OS サービス、重い観測・出力 | **実装済み**（既存契約を維持。汎用部品の抽出は**要 PoC**） |
| `*-core`・`ruviform-math`・DSP 等 | 各ドメインの型・処理・実行計画・数値演算 | 汎用 Foundation の雑多な再実装 | **実装済み**（原則として既存の責務境界を維持） |

## Foundation 候補の内訳

各モジュールは内部境界を保ち、相互に必要以上の `pub(crate)` 参照を作らず、独立抽出可能な API を意識する。以下は設計対象の一覧であり、モジュールの実装を直ちに要求しない。

| モジュール候補 | 主な責務 | 初期の設計上の注意 |
| --- | --- | --- |
| `collections` | 固定容量配列・リング・事前確保コンテナ | `arrayvec` / `heapless` 等の既存クレート（採用候補・未決定）を優先・比較。汎用コンテナであることだけを理由に、その `Drop` の RT 安全性までは保証しない |
| `memory` | アラインメント、事前確保、Storage/Pool/Arena、容量・使用量の計測点、所有権・枯渇・終了時の契約 | `std::alloc` を優先。グローバルアロケータの設定をライブラリから強制しない。音声／動画バッファのドメイン上の意味は各ドメインに残す |
| `sync` | プロセス内 SPSC・必要に応じ bounded queue、cache padding 等 | 既存 `ruviform-rt` の `rtrb` ラップを検証。退避 Drop・スナップショット寿命管理はキューと別の RT 契約。プロセス間共有メモリはさらに別契約 |
| `identity` | 世代付き ID・スロット等の汎用ハンドル構成部品 | `PluginHandle`（`ruviform-plugin-host`）や `StreamId`（`ruviform-media-core`）・`ParamId`（`ruviform-plugin-core`）等のドメイン別型を不必要に統合しない |
| `telemetry` | 固定サイズのイベントレコード、カウンタ、記録用リング、取りこぼし（オーバーフロー）の観測など、出力先に依存しない軽量記録機構 | ログフォーマット・Subscriber・export・ファイル I/O を含めない |

既存 `ruviform-rt` の `alloc_counter`（`#[cfg(test)]` 限定のテスト専用・内部計測アロケータ）を含め、Memory／Telemetry／RT 検証の所有者や重複はコード棚卸し済みである（後述「既存実装の棚卸し」節参照）。現時点で移動先を確定しない（**要 PoC**）。既存 ADR・公開 API・RT 契約との互換性と RT コストを検証してから判断する。

### Memory と所有権

事前確保した Pool/Arena を提供する場合は、構築・借用・返却・再利用・破棄・枯渇・シャットダウンを別フェーズとして仕様化する。`release` が固定時間であってもユーザーの `T: Drop` が RT で安全とは限らない。音声サンプルや動画フレームの意味を持つ Pool/Buffer はドメイン側が所有し、汎用 Storage のみ Foundation で共通化する。

`#[global_allocator]` のようなプロセス全体の設定は利用アプリケーション・ホストに委ねる。ライブラリが暗黙に差し替えない。

### Sync とプロセス間通信

`foundation::sync` のキューは原則 **同一プロセス内**のメモリ所有権・同期を扱う。共有メモリ上のレイアウト、相対オフセット、ABI、クラッシュ後の回収、プロセス間通知は別途 System Services / IPC の所有とする。キューが wait-free であっても、容量不足時の返却値の Drop、スナップショット参照の最終解放、終了処理などは別の契約が必要。

既存 `ruviform-rt` の SPSC（`channel`）を Foundation へ移動するかは**要 PoC**。汎用利用者の実在、RT 側互換性、依存 DAG、公開 API の影響を評価してから決定する。コード棚卸しでは `channel`／`retire_channel`／`slot` にプロダクション利用者が存在せず（`rt` の外部利用は `RtContext`（audio・exec）・`Processor`／`RtProcessor`（exec）・`MasterClock`（video）のみ）、「汎用利用者の実在」の条件は現時点で満たされないため、現状のデフォルトは移動しない。内部では `slot` は `retire_channel` 上に、`retire_channel` は `channel` 上に構築されているため、`channel` 単独の Foundation 移動は `retire_channel`／`slot` の同時移動か、非既定の `ruviform-rt → foundation` 依存を意味する。`ruviform-rt` の退役オブジェクト回収（`retire_channel`）とスナップショット交換（`slot`）は RT 特有の契約を維持する。

## Observability・Diagnostics・Debug の責務区分

これらは同義ではない。まず機能上の概念として区別し、初期は `ruviform-observability`（構成候補）のモジュール候補として管理する。重い Debug ツールの別クレート化は必要になってから決める。

| 領域 | 主な目的 | 所有する内容 | RT 上の扱い |
| --- | --- | --- | --- |
| **Logging** | 事象を記録・出力する | 重要度、構造化イベント、コンテキスト、フィルタ、出力先 | メッセージ生成・整形・出力は非 RT |
| **Profiling** | 実行時間・リソース消費の原因を分析する | span・計測区間・サンプリング・タイムライン・外部 profiler 連携 | RT 計測は独立検証した軽量記録のみ |
| **Metrics** | 状態を数値・時系列で監視する | xrun、処理時間、レイテンシ、キュー飽和、Drop 退避、プール使用量等 | 事前確保済みカウンタ／レコードを使用 |
| **Diagnostics** | 健全性・異常の状態や原因を説明する | 構造化診断、事象 ID、severity、context、health check・診断レポート | RT では固定容量の事実記録のみ。解析・説明は非 RT |
| **Debug** | 開発・検証・不具合再現を支援する | 不変条件検査、Graph/Session inspection、状態 dump、capture/replay、検証用 hooks | 明示的な Debug モードと局所コスト評価。重い処理は禁止 |

**Diagnostics と Error の違い:** ドメインの `Result<T, E>` / `Error` は、その API の契約を所有するクレートが定義する。Diagnostics は失敗の原因・経緯・動作状況を構造化して収集・説明する仕組みであり、すべての Error を Foundation の巨大 enum に統合しない。

**Diagnostics と Metrics の違い:** Metrics は計数・測定・集約、Diagnostics は問題の意味づけ・原因の切り分け・必要なコンテキストを扱う。

**Logging と Diagnostics の違い:** Logging は構造化イベントの記録・フィルタ・出力を担当し、Diagnostics はその事象への意味付け（severity・原因分析・health 判定）を担当する。イベントスキーマは発行側ドメインが所有し、Logging は意味を解釈しない。

**Logging と Metrics の違い:** Metrics は計数・集約した数値系列を所有する。Logging のイベント記録から metric を導出する場合、その導出・集約は Metrics 側の責務とし、Logging は生イベントの記録に留める。

**Debug と Diagnostics の違い:** Diagnostics は本番動作中にも必要な健全性情報を含むが、Debug は追加コストのある内部状態の検査・検証・再現支援を含み得る。重い Debug 機能を全利用者への必須依存にしない。

### Observability の候補モジュール

```text
ruviform-observability [構成候補]
├── logging          # tracing 等への統合 / sink / filtering
├── profiling        # duration / span / external profiler adapters
├── metrics          # counters / gauges / histograms / aggregation
├── diagnostics      # structured diagnostics / health / reports
├── debug            # lightweight debug probes / invariant-report integration
├── rt_bridge        # bounded RT records -> non-RT collector
└── export           # console / file / trace / external integrations
```

外部ライブラリ候補：`tracing`、`tracing-subscriber`、`metrics`、`tracy-client` 等は**採用候補であり未決定**。採用は個別 PoC とライセンス・MSRV・依存木・RT コスト評価による。Ruviform が独自の Log/Span/Metrics フレームワークを重複実装しない。**ライブラリの側でグローバル Subscriber・Logger・Recorder を暗黙に初期化・変更しない（方針）。** 利用側が初期化を所有するため、最終的な初期化と出力設定は Host/Application 側が所有する。

### Debug の検討対象

- **Invariant validation:** Graph の接続、Buffer の範囲、タイムスタンプ単調性、世代 ID、スレッド契約など。検査が高コストなら非 RT の検証 API として公開する。
- **State inspection / dump:** `GraphPlan`・`Session`（`ruviform-audio`）、Plugin 状態・パラメータ、使用メモリ・キュー、トランスポートを構造化して読み取る。所有権を壊さず、スナップショットの取得は非 RT 制御経路とする。
- **Event capture / deterministic replay:** 実際に再現性が必要になったドメインから設計する。壁時計・スレッド間競合・外部プラグイン・I/O の非決定性を明示し、「記録したら必ず完全再現できる」とは主張しない。
- **RT contract probes:** alloc/dealloc 検出、容量超過、最悪実行時間の観測、Drop 退避など。RT 本番経路と CI 検証レーン（`rt_` プレフィックス・`assert_no_alloc` 測定窓）の範囲を区別する。
- **Crash / hang investigation:** 非 RT 側でのバックトレース、子プロセス異常終了情報、必要なら dump 連携。クラッシュ処理を RT コールバックへ混入させない。
- **Privacy / security:** 状態 dump や記録に音声素材、パス、プラグイン状態、秘密情報が含まれ得るため、収集範囲と保存を明示 opt-in とする。

汎用 Debug ランタイムを初期必須基盤として固定するのではなく、共通の診断イベント形式・検査ポートを先に揃える。Graph Inspector 等のドメイン固有ツールは利用ドメインに置き、共通レンダラや開発支援が実際に必要になれば `ruviform-debug` 等を抽出する（**保留**）。

## RT / 非 RT 観測データフロー

```text
RT callback / DSP / plugin
   ├── prepared atomic counter
   └── prepared fixed-size record → bounded non-blocking handoff
                   │                        (overflow は記録・集計)
                   ▼
            non-RT collector
                   │
         ┌─────────┼──────────┐
         ▼         ▼          ▼
       Logging   Metrics   Profiling
                   │
             Diagnostics / Debug UI
```

- 既存 RT 契約（[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)）の禁止・制限区分に従う。RT 経路では alloc/dealloc を**禁止**し、ブロッキング操作・ファイル/ネットワーク I/O・無制限の待機/リトライ・実行時間が有界でない処理（可変長で上限不明の処理を含む）を**制限**する。加えて本書の観測文脈では、任意の文字列フォーマット・動的ログ整形・Subscriber／コールバック経由の未検証処理を、API 単位の検証なしに RT 経路へ持ち込まない（**本書独自の厳格化**であり、ADR の禁止・制限区分を緩める差分ではない）。`tracing` のマクロや非同期出力方式であっても、その呼び出し経路を検証せず RT-safe としない。
- イベントの時刻取得が syscall、初回初期化、ロック、計測オーバーヘッドを伴うかは対象プラットフォーム別に検証する。必要な記録領域・バッファは RT 区間前に準備する。
- キュー満杯時の**レコードの破棄／集約／取りこぼし数の計測**は API ごとに方針を決める。特に固定サイズの `Copy` レコードの破棄と、所有オブジェクトの `Drop` は区別する。任意の `T: Drop` や最後の `Arc` 参照解放が RT 側で動作しないことは別途検証する。取りこぼし数を非 RT から観測できることを重視する。
- 計測無効時と有効時の双方で RT 実行時間・レイテンシを評価する。OS レベルの hard RT を保証するものではない。
- RT → 非 RT の有界観測レコード転送機構は、構成候補上 `ruviform-observability` の `rt_bridge` が所有する（`foundation::sync` は汎用 SPSC・bounded queue プリミティブの提供候補、`foundation::telemetry` はレコード schema・カウンタの所有候補）。実配置は後続の個別設計で確定する。

## 既存実装の棚卸し（コード調査）

Issue #101 の後続として workspace 18 クレート（棚卸し時点の数）に対し実施したコード棚卸しの結果を、各領域の判断材料として記録する。以下は実装の有無・重複・配置に関する事実の要約であり、採否・移動・新設の確定ではない。

### Memory / Collections

- 割り込み計測（alloc 検出）は 3 系統が存在する: `ruviform-rt` の `alloc_counter`（`#[cfg(test)]` 限定・`pub(crate)`・スレッドローカル計数の `GlobalAlloc`、テスト専用）、`assert_no_alloc` の dev 依存（math・media-core・dsp・audio-analysis・audio-device・plugin-clap・plugin-vst3・plugin-host・ruviform-audio の 9 クレート）、`ruviform-audio-device` の `play_sine` example 内の private `CountingAlloc`＋`AtomicUsize`。example 側の機構は、`assert_no_alloc` のスレッドローカル測定窓がバックエンド所有のコールバックスレッドを観測できないという、検証レーンの既知のカバレッジ隙間を補うために存在する。
- 汎用 Pool / Arena / Storage モジュールは存在しない。`arrayvec`・`heapless`・`smallvec` 系の依存もなく、ポインタのアラインメント検証は plugin-clap・plugin-vst3 の 2 adapter が個別に実装している（`ruviform-audio-device`・`ruviform-io-symphonia` 等には存在しない）。「prepare で確保・process は借用」は dsp・audio-analysis・adapter の scratch 型に共通するワークスペース規約であり、共有コードではない。

### Sync

- `ruviform-rt` の `channel`・`retire_channel`・`slot` はプロダクション利用者を持たない（`rt` の外部利用は `RtContext`（audio・exec）・`Processor`／`RtProcessor`（exec）・`MasterClock`（video）のみ）。内部依存連鎖として `slot` は `retire_channel` 上に、`retire_channel` は `channel` 上に構築されている（`retire.rs`・`snapshot.rs` の内部 import 参照）。
- スナップショット交換は 2 系統が併存する: `rt::slot` と、`ruviform-audio` の `PlanSwap`（`plan.rs`・`RwLock`＋`try_read`＋`Arc` キャッシュ）。後者は [音声グラフ交換 ADR](../adr/audio-engine-snapshot-swap.md) が raw `arc-swap`／`RwLock` を比較した上で選んだ意図的な分岐であり、同 ADR は当時利用可能だった `rt::slot` 自体を比較対象に含めていない（Sources 末尾の注記参照）。
- 有界キューはこれらに加え `ruviform-exec` の `offline` 内部 `sync_channel`（bounded MPSC パイプライン）があり計 3 形状。`Atomic*` 型名の出現は crates/ 全体で約 75 箇所（数え方: `AtomicUsize`・`AtomicU64`・`AtomicBool` 等の型名を grep・コメント／テスト含む。`src/` のみなら 46 箇所）。いずれも所有者ローカルであり、共有カウンタ型・cache-line padding は存在しない。

### Identity

- 世代付きハンドルは `PluginHandle`（`ruviform-plugin-host` の `{slot, generation}`）のみ。`StreamId`・`ParamId` はレジストリ newtype、`DeviceId` は opaque `String` であり、セマンティクスが異なるため統合は偽の抽象化となる（「Foundation 候補の内訳」表の `identity` 注意と同旨）。

### Telemetry / 観測レコード

- RT 経路の飽和・取りこぼし観測カウンタとしては `retire_channel` 1 組あたり 1 本の共有 `Arc<AtomicUsize>` のみであり、`Retirer::overflows`・`DropCollector::overflows`・`SnapshotWriter::retire_overflows` の 3 アクセサが同一カウンタを読む。その他に所有者ローカルな prepared atomic として `Progress`（`ruviform-exec`・`completed`／`total` の `Arc<AtomicU64>`）、`MasterClock::tick`（`ruviform-rt`・`AtomicU64`）、plugin の param mirror（plugin-clap の `gain_plain_bits: AtomicU64`・`AtomicLifecycle`、plugin-vst3 の `tail: AtomicU32`）等が存在する。
- 固定サイズ Copy レコードの既存先例として、`Event`・`TransportInfo`・`ProcessConfig`・`ProcessContext`（いずれも plugin-core）と `TransportSnapshot`（media-core）の計 5 型が `reserved: [u64; 2]` 拡張フィールドを持つ（plugin-core の規約に media-core の `TransportSnapshot` が加わる形）。テレメトリのリング・レコード受け渡し機構は未実装であり、cpal `StreamInstant`→`MediaTime` マッピング（`ruviform-audio-device`）が RT 経路唯一のタイムスタンプ実装である。
- 命名衝突の注意: `rt` の `Snapshot*` プリミティブと、ドメイン側の "snapshot"（`TransportSnapshot`・`LevelMeter` のスナップショット・`RtPlan` 等）は別概念である。テレメトリレコードの schema 設計時は用語を区別する。

### Observability / Diagnostics / Debug

- 棚卸し時点の全 18 クレートの `Cargo.toml` に observability 系依存（`log`・`tracing`・`metrics` 等）は存在しない。
- 型付きエラーによる Diagnostics が既存の慣用形である（media-io `ErrorKind`、`DeviceError`（Xrun 含む）、`HostError`、`OfflineError`、media-core `Error`）。dump／inspect／capture／replay に相当する実装は存在しない。
- 既存の RT 契約 probe: `alloc_counter`、`assert_no_alloc` による `rt_` 検証レーン、CLAP thread-check（`is_audio_thread`）、retire overflow カウンタ。

### Process / IPC / Platform

- `ruviform-exec` の OS 接地面はスレッド優先度機構（Windows MMCSS・macOS time-constraint・Linux `SCHED_FIFO`。`audio_thread_priority` は非公開の実装詳細）と `OfflineDriver` のみであり、プロセス管理・IPC・共有メモリの実装は workspace に存在しない。なお [全体設計ベースライン](ruviform-architecture-design-baseline.md) L240・L352・L392 の「スレッドプール／ワーカープール」表記は Issue #24 の実装（`run_parallel` 呼び出しごとに `std::thread::scope` で生成される scoped worker）より前の記述であり、次回 baseline 改訂時の更新候補とする。
- プラグイン L1 は [plugin-isolation](plugin-isolation.md) 上の設計のみである。L0 のコマンド面は既にシリアライズ可能な plain data（`ruviform-plugin-host` の `command.rs`）であり、L1 化時にコマンド表現の再設計を要しない。

## 依存方向と既存契約との整合

- Foundation を新設する場合、メディアドメイン、`rt`、`exec`、Observability、Engine、`plugin-*`、OS adapter への逆依存を作らない。`core` / `alloc` / 必要な `std` と監査した最小の外部依存だけを持つ。共通化対象の意味・所有権・ライフサイクルを揃える。
- `ruviform-math` は独立維持し、Foundation との相互依存を作らない。共通数値ユーティリティの所有者は重複しないよう個別決定する。
- `ruviform-rt` は既存公開 API と実装契約を維持する。Foundation に抽出後の一方向依存は候補だが、既存 ADR の制約を先に変更せず実装しない。
- Observability 側が Foundation の軽量プリミティブを利用することは候補。ただし Foundation が重いログ出力やプロファイラを強制依存に持たず、Observability に依存してはならない。
- Domain Foundations（`*-core`）は外部ログ出力 backend を必須依存にせず、必要ならイベント発行やポート型に限定する。実際の集約・可視化は Host/Application 側が選ぶ。
- `ruviform-exec` のスレッド優先度機構（`promote_current_thread_to_realtime` 等）は当面維持する。OS に依存する共通サービスが複数箇所で必要になれば、`platform` または機能別 system crate を比較する。
- [全体設計ベースライン](ruviform-architecture-design-baseline.md) は現状、`ruviform-rt` と `ruviform-math` を最下位・相互独立としている。`ruviform-rt → ruviform-foundation` のような新しい依存は**既定路線ではない**。導入する場合は依存 DAG の再設計、公開 API／RT 契約の評価、関連 approved design・accepted ADR の必要な改訂を先に行う。
- [RT 基盤プリミティブ ADR](../adr/rt-foundation-primitives-implementation.md) で選定された `rtrb`／`arc-swap`（いずれも `ruviform-rt` の非公開実装詳細）と `ruviform-rt` の責務、`ruviform-exec` のスレッド優先度機構は、そのまま維持する。共通化・移動は別途判断する。
- 音声デバイスは `ruviform-audio-device`、メディア I/O backend は `ruviform-io-*`、GPU 等は各 adapter が所有する。OS のコードというだけで全て `platform` へ集約しない。Issue #76 のプラグイン L1 固有の protocol／lifecycle と汎用プロセス・共有メモリ基盤は別に検討する。
- Plugin L1 のプロセス管理・共有メモリと、プラグイン固有コマンド・ライフサイクルは分ける。プロセス内 SPSC をプロセス間共有メモリ用キューとして無条件に再利用しない。
- 依存の循環・逆転、重い optional dependency の全利用者への伝播、Cargo features による統合で意図しない機能が有効化されることを禁止・検証する。分割後の公開パス・型の互換性も評価し、可能なら re-export で互換性を維持するが、自動的な完全互換は前提としない。

## クレート分割の判断基準

モジュールからクレートへの抽出は**行数の閾値ではなく次の根拠**によって判断する。コード量は補助指標とし、契約・依存境界の違いが重要なら小さなモジュールも早期に分離する。

| 分割判断の軸 | 分割を支持する具体的状況 |
| --- | --- |
| 独立利用 | 単体の public API だけで意味があり、別の利用者が存在する |
| 依存・可搬性 | 独立させると OS / FFI / 重い外部依存 / `no_std` 条件を分離できる |
| 契約 | RT・非 RT、安全性、メモリ所有権、失敗モードの保証が異なる |
| 開発・互換性 | 独立した API 進化・検証・バージョニング・公開周期が必要 |
| 安全性 | unsafe / ABI / 共有メモリの監査範囲を隔離できる |
| ビルド | Cargo check / incremental / CI の実測で改善がある |

分割しない条件：実装詳細のためだけに `pub` を増やす、常に同時変更になる、依存 DAG が複雑化するだけ、利用者も保証契約も未定、という場合。モジュール分割・可視性の改善で十分ならクレートを増やさない。

既存モジュールの抽出時は公開パスを可能な範囲で re-export して互換性を保持する。ただし公開型・trait の所有元や features、MSRV が変わる影響は別途検証する。`cargo-semver-checks`、ターゲット別テスト、RT 検証、Cargo 依存木・ビルド性能の測定を候補とする。

## 適用例と設計上の検証項目

1. **Audio xrun / plugin timeout:** RT 側はカウンタまたは固定レコードを送出し、非 RT が原因・時刻・関連ノード・経過を Diagnostics として組み立てる。Graph 遅延の意味づけは audio engine が所有する。
2. **Memory Pool exhaustion:** Pool の容量・使用数の計測点は Foundation 候補、実際の audio/video buffer の意味とフォールバックはドメイン側が所有する。RT 側でログ文面を作成しない。
3. **Plugin L1 crash:** L1 は [plugin-isolation](plugin-isolation.md) の設計上の将来構成であり現行未実装。子プロセスの exit／監視は System Services / L1 側、設計上の `Failed` 状態遷移は plugin-host、共有プロセス資源回収と診断レポート作成は非 RT とする。
4. **Graph inspection:** 音声・動画の固有状態は各 engine 側に保持し、スナップショットを取得してから非 RT の Debug tool で図示・保存する。

## 状態一覧（方針／構成候補／保留／要 PoC／実装済み）

| 論点 | 区分 | 方針／確定条件 |
| --- | --- | --- |
| 小さな汎用基盤を先にモジュール設計し、必要時にクレート抽出 | **方針** | 採用。行数だけで機械的に分割しない |
| Foundation と Domain Core の分離 | **方針** | 採用。巨大な `ruviform-core` へ統合しない |
| Logging / Profiling / Metrics / Diagnostics / Debug の責務区分 | **方針** | 採用。重い出力と低コスト記録は分離 |
| RT からの観測は事前準備済み・容量有界・非ブロッキングの記録に限定 | **方針** | 採用。API 単位に検証したものだけを RT に置く |
| ライブラリはグローバル Subscriber・Logger・Recorder を暗黙に初期化・変更しない | **方針** | 採用。利用側が初期化を所有し、最終的な初期化と出力設定は Host/Application 側が担う |
| `ruviform-foundation`・`ruviform-observability`・`ruviform-debug`・`ruviform-platform`・`ruviform-process`・`ruviform-ipc` のクレート名と内部モジュール構成 | **構成候補** | 候補名であり実装済みではない。新設時期・最小 API・依存 DAG は後続設計で確定 |
| `ruviform-foundation`・`ruviform-observability` の新設・API 範囲 | **保留** | 実際に必要な型と依存関係を設計してから Issue 化 |
| `channel`／`retire_channel`／`slot` の Foundation 移動可否 | **要 PoC** | 2 番目のプロダクション利用者の出現をトリガに、既存 ADR・公開 API・RT 契約と照合して互換性・効果を検証。内部依存連鎖により 3 プリミティブ同時の判断になる |
| Identity・Memory の汎用化要否 | **要 PoC** | 汎用利用者・実装の実在をトリガに、既存 ADR・公開 API・RT 契約と照合して判断（現状 Identity はドメイン別型のみ・Memory の汎用実装は非存在のため「移動」ではなく新規設計の要否判断） |
| `alloc_counter` の所有者 | **要 PoC** | test 専用枠を超える計測アロケータ需要の発生をトリガに判断（`#[cfg(test)]` ゲートのためプロダクション利用者トリガは構造的に不成立） |
| `tracing` / `tracing-subscriber` / `metrics` / `tracy-client` 等の採用 | **要 PoC** | **採用候補・未決定**。RT 記録経路・依存・MSRV・運用コストの評価 |
| Debug の独立クレート化、Capture/Replay・Crash Investigation 仕様 | **保留** | ドメインをまたぐ具体的な利用需要が生まれたら |
| 共通 `platform` / `ipc` / `process` の独立 | **要 PoC** | Issue #76 L1 の着手と 2 番目の OS サービス利用者の出現をトリガに、複数用途・OS 互換性・ABI・資源寿命を検証 |
| 既存 18 クレート（`ruviform-rt`・`ruviform-exec`・`ruviform-math`・`*-core` 等。棚卸し時点の数）と accepted ADR の契約 | **実装済み** | 維持。変更には影響分析と必要な改訂を行う |

## 適用・移行順序

1. **設計整合（実施済み）:** 棚卸し時点の既存 18 クレートと accepted ADR の依存/公開 API/実行契約の棚卸しは完了した（「既存実装の棚卸し」節参照）。本書の候補を実装済みと誤認しない。
2. **Foundation 候補の最小設計:** Memory / Sync / Identity / Telemetry の必要な共通操作と、既存実装を流用できる範囲を確定する。
3. **Observability 候補の最小設計:** Logging・Metrics・Diagnostics・Profiling・Debug probe の責務と、RT → 非 RT の記録境界を設計する。
4. **PoC と採否:** キュー容量・Drop・スナップショットの寿命、RT 計測コスト、外部依存/OS/ABI を検証する。
5. **実装・移行:** 利益が確認できるモジュール/クレートだけ実装し、re-export・契約テスト・CI・依存 DAG 検証を伴って段階的に移行する。

新規クレート追加、公開 API の変更、accepted ADR を変更する際は、実装 Issue または ADR で判断根拠・影響・検証範囲を記録する。

## 後続 Issue と再着手条件

棚卸し結果を踏まえた未確定事項のトリアージを記録する。

- **起票済み**（本書の方針に基づき直ちに着手可能な設計・検証タスク）
  - [#103](https://github.com/scarecrow173/ruviform/issues/103) — [design] Foundation / Observability 候補モジュールの新設時期・最小 API・依存 DAG の設計（「適用・移行順序」の 2・3 に相当）。
  - [#104](https://github.com/scarecrow173/ruviform/issues/104) — [poc] RT 観測経路のコスト計測と方式評価（prepared counter・固定サイズレコード・`rt_bridge` 候補の RT 実行時間・レイテンシ評価、オーバーフロー方針（破棄／集約／計測）・タイムスタンプ取得方法、`tracing`／`metrics`／`tracy-client` 等バックエンド採否評価（依存・ライセンス・MSRV・RT コスト））。
- **延期**（再着手条件を明示して保留）
  - `channel`／`retire_channel`／`slot` の Foundation 移動可否 — トリガ: 2 番目のプロダクション利用者の出現（内部依存連鎖により 3 プリミティブ同時の判断になる）。
  - Identity・Memory の汎用化要否 — トリガ: 汎用利用者・実装の実在（現状 Identity はドメイン別型のみ・Memory の汎用実装は非存在のため「移動」ではなく新規設計の要否判断）。
  - `alloc_counter` の所有者決定 — トリガ: test 専用枠を超える計測アロケータ需要（`#[cfg(test)]` ゲートのためプロダクション利用者トリガは構造的に不成立）。
  - `ruviform-debug` の独立クレート化 — トリガ: ドメインをまたぐ具体的な利用需要。
  - `platform` / `process` / `ipc` の独立クレート化 — トリガ: Issue #76 L1 の作業着手と 2 番目の OS サービス利用者の出現。
  - L1 の汎用プロセス基盤とプラグイン固有境界の切り分け — Issue #76 の着手時に解決する。

## 作業時に参照する条件

`AGENTS.md` から本書を参照するのは、共通 Memory/Sync/Collections/Identity/Telemetry、Logging/Profiling/Metrics/Diagnostics/Debug、RT 計測経路、IPC/OS、クレート分割、依存 DAG・API 所有者・feature 境界を新設・変更・レビューするときとする。無関係な変更時に全読を義務化しない。

## Sources

- [Ruviform architecture design baseline](ruviform-architecture-design-baseline.md)
- [RT 契約と検証方針](../adr/realtime-contract-and-verification-policy.md)
- [RT 基盤プリミティブ実装方式](../adr/rt-foundation-primitives-implementation.md)
- [音声グラフ交換 ADR](../adr/audio-engine-snapshot-swap.md)
- [RT 検証機構の選定（assert_no_alloc 標準化・thread-check 意味論）](../adr/rt-assert-no-alloc-thread-check.md)
- [RT 検証レーン設計](rt-verification-lane-design.md)
- [Plugin isolation 設計](plugin-isolation.md)
- [Issue #101](https://github.com/scarecrow173/ruviform/issues/101)、[Issue #76](https://github.com/scarecrow173/ruviform/issues/76)、[Issue #17](https://github.com/scarecrow173/ruviform/issues/17)、[PR #100（未マージの参考ドラフト）](https://github.com/scarecrow173/ruviform/pull/100)
- [Bevy utilities](https://github.com/bevyengine/bevy/tree/main/crates/bevy_utils)、[Logging](https://github.com/bevyengine/bevy/tree/main/crates/bevy_log)、[Diagnostics](https://github.com/bevyengine/bevy/tree/main/crates/bevy_diagnostic)
- [Cargo Features](https://doc.rust-lang.org/cargo/reference/features.html)、[Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- 注記: [音声グラフ交換 ADR](../adr/audio-engine-snapshot-swap.md) は raw `arc-swap`／`RwLock` の比較を行っているが、当時既に利用可能だった `ruviform-rt` の `slot`（スナップショット交換プリミティブ）自体は検討対象に含めていない。将来の ADR 改訂時に補完を検討する（本書では ADR 本文は変更しない）。
