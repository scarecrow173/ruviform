---
id: "DISC-034IO8gG0n5xCqh66w8KUp"
type: "discovery"
status: "draft"
title: "Issue #1 詳細設計に向けた要求確定とエコシステム調査"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/1"
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
    - "../specs/architecture-design-baseline-deliverables.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/ui-architecture-webview-hybrid.md"
    - "../adr/licensing-and-distribution-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  derived-by:
    - "../designs/ruviform-architecture-design-baseline.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/2"
    - "https://github.com/scarecrow173/ruviform/issues/3"
  defers: []
  deferred-by: []
---

# Issue #1 詳細設計に向けた要求確定とエコシステム調査

## Exploration Goal

本書は GitHub Issue [#1 [design] Ruviformの全体アーキテクチャ・クレート構成・RT/オフライン実行モデルを詳細設計する](https://github.com/scarecrow173/ruviform/issues/1) の briefing／探索フェーズの中間成果を記録する canonical ドキュメントである。記録対象は 2 系統:

- **(a) 対話で確定した要求・決定**: briefing 対話においてユーザーが DECISION として承認した内容（探索への入力としての制約）
- **(b) エコシステム調査結果**: Rust マルチメディアエコシステムを対象とした調査サブエージェントの生レポート（要約・構造化）

本書が回答する問い:

- Issue #1 の詳細設計に先立ち、どの要求・制約がユーザー承認済みで確定しているか
- エコシステム調査が示す制約・先行事例・選択肢は何か
- Issue #1 で設計案として具体化すべき論点（クレート DAG・データモデル・処理グラフ・DSP/SIMD・検証計画・ロードマップ）と、調査結果の対応関係はどうなるか

> **位置づけ**: 本書は探索の推論経路と一次情報を記録するものであり、設計上の最終決定を含めない。(a) の内容は「探索への入力としてユーザーが確定した要求・制約」であり、その記録先は並行作成された spec（[architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md) の Requirements B）と 6 件の ADR（`relations.related` 参照）である。調査に基づく採用候補はすべて「仮説」として記し、最終決定は ADR・design-doc に委ねる。

## Confirmed Requirements and Constraints（対話で確定した入力）

以下は briefing 対話でユーザーが DECISION として承認した内容である（逐語に近い形で転記）。Issue #1 の設計案はこれらを制約として満たす必要がある。

### ターゲットと提供物

- 第 1 ターゲット: DAW／再生ホスト／プラグイン制作。基礎クレートはエンジン・ホストから独立して単独利用可能であること（**汎用ライブラリ性は絶対条件**）。
- 第 2 ターゲット: NLE／映像制作級（動画再生・同期 → 編集・コンポジット）。
- プラグイン制作は後段追加ではなく第 1 ターゲット。

### メディア I/O

- 全 I/O はクレート内抽象境界の背後にバックエンドを置く。
- 外部ライブラリの型・エラー・機能フラグを公開 API に露出しない。
- 純 Rust 実装（symphonia 等の利用）も同じ境界経由の差替可能な実装詳細とする。
- コアは純 Rust。FFmpeg 等は分離クレート + opt-in。

### プラットフォーム

- Windows / macOS / Linux、x86_64 + aarch64。
- 基礎クレートは GPU 非依存。`wgpu` はエンジン層の opt-in。
- stable-only。
- モバイル・組込・wasm は排除しないが初期ターゲット外。

### リアルタイム（RT）契約

- RT 契約は Ruviform 内部だけでなく、アプリ／外部プラグインを含む RT クリティカルパス全体に適用する。ただしアプリ全体に制約は課さず、保証は自身の RT 対応実装に限定する。
- RT パスでは alloc／dealloc 禁止。ブロッキング・I/O・無制限待機／リトライ・非有界処理を制限。
- RT／非 RT を API とライフサイクルで分離。
- 検証可能な違反は CI で検出。静的に判定不可能なものは文書＋レビューで担保。CI 成功は完全証明ではない。
- OS ハードリアルタイム保証なし。

### プラグイン

- ホスト側・制作側の双方が第 1 ターゲット。
- CLAP／VST3 を同格とし、実装順序は技術的都合（同時必須ではない）。
- 共通処理基盤と規格固有アダプターを分離。RT 契約は双方に適用。
- Issue #1 で ABI 境界・ライフサイクル・分離方針を確定する。
- 映像プラグインは OFX が有力候補（範囲は別途設計）。
- プラグイン GUI 統合は双方向必須だが、独自 GUI を全プラグインに強制しない。
- VST3 SDK は 3.8 以降 MIT ライセンス。

### UI

- WebView ファースト・ハイブリッドを第 1 候補（HTML/CSS/JS・TS、フレームワーク不固定、PoC で検証）。
- 高頻度描画は DOM 更新に限定せず、WebView 内 Canvas2D／WebGL2／WebGPU・`wgpu` 等のネイティブ GPU・ネイティブサーフェス合成を検討する（固定順序・単一フォールバック階層ではない）。
- 理想構造は、描画バックエンド非依存で WebView 側から通常コンポーネント同様に使えること。描画方式とコンポーネントを分離。
- 波形ピーク・FFT 等の描画データは Rust 側で生成・キャッシュ。全コンポーネントに共通抽象を強制しない。
- プラグイン GUI にも同一方針（外側は規格準拠ネイティブウィンドウ）。
- UI 基盤は再利用構造。エンジンと UI を分離し、RT スレッドに UI 処理・ブロッキング同期を持ち込まない。

### ライセンス・公開

- 正式決定は保留（MIT or MIT OR Apache-2.0 が有力）。`license = "MIT"` は仮置きであり未決定を明確化する。
- Private 開発継続。公開可否はクレート単位で選択し、未決定クレートは原則 `publish = false`。
- 0.x 継続。1.0 は成熟度で判断。
- プラグイン ABI 適合性と内部 Rust API 安定性は分離管理。
- 依存のライセンス義務はクレート分離で解消しない。公開前ライセンスレビュー必須。

## Key Issues and Assumptions

Issue #1 で設計案として具体化すべき論点（未解決のままでは詳細設計を完了できないもの）:

- **クレート DAG**: Issue #1 のレイヤー案・クレート境界候補（`ruviform-math` / `ruviform-audio-core` / `ruviform-dsp` / `ruviform-audio-io` / `ruviform-video-*` / `ruviform-audio` / `ruviform-video` / `ruviform-gpu` / `ruviform-ui` / `ruviform-plugin-*` は仮称）から、実際の依存 DAG・公開 API 境界・optional platform adapters の依存方向を確定する。前提: 基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない。
- **データモデル**: 音声 sample/frame/buffer・interleaved/planar・チャンネルレイアウト、動画 pixel/frame/plane・stride・色空間／HDR メタデータ・フレーム時刻、音声サンプル時刻と動画 timebase／PTS の変換責務、A/V 同期・シーク・ドロップフレーム・VFR の所在。
- **処理グラフ・実行モデル**: RT 系統（コールバック駆動・事前確保・有界）とオフライン系統（バッチ・並列化・チャンク）の分離構造、初期化・グラフ再構成・codec 初期化の非 RT 区間配置、RT ↔ 非 RT 間の通信機構。
- **DSP／SIMD**: stable-only の前提で、SIMD の実装方式・ランタイム分岐・フォールバック・境界条件（アラインメント・端数・NaN/Inf・denormal・丸め差異）の設計と採否基準。
- **検証計画**: SIMD とスカラーリファレンスの比較方法、音声ドロップアウト／レイテンシ／動画性能の検証、RT 違反の CI 検出機構。
- **ロードマップ**: Phase 1（math＋基礎データ型）→ Phase 2（音声／動画基礎）→ Phase 3（エンジン・描画・UI）の精査と、#2 基盤への反映差分。

これらの論点に対し、エコシステム調査は選択肢・制約・先行事例を与える（次節）。「調査で有力候補が見つかった」ことと「採用決定」は区別し、後者は ADR・design-doc で確定する。

## Alternatives and Comparison

エコシステム調査（調査サブエージェントの生レポート）を領域別に構造化したもの。`Lean` は調査時点の傾向（仮説）であり採否決定ではない。

### オーディオデバイス I/O

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `cpal` | Rust オーディオデバイス I/O の事実標準。3 OS 対応実績 | メンテナ少数・資金問題あり（issue #903 / #981）。将来の保守リスク | yes（ただし抽象境界の背後に隔離が必須） |
| 自前 OS API 実装 | 依存ゼロ・完全制御 | Core Audio / WASAPI / ALSA 等の全面自前化は非現実的コスト | no |

- 含意: `cpal` のメンテ状況は「バックエンド抽象で差替可能にする」設計の正当化根拠。デバイス I/O も抽象境界経由の実装詳細とする仮説が有力。

### 音声 codec / コンテナ I/O

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `symphonia`（0.6.1、MPL-2.0） | 純 Rust decode の最有力。`forbid(unsafe)`。trait 分離設計が抽象境界の好モデル | MPL-2.0（弱コピーレフト。ファイル単位）。encode 範囲は限定的 | yes（純 Rust バックエンドの中心候補） |
| `ffmpeg-next`（9.x） | 広範な codec/container 対応。活発 | LGPL／GPL・nonfree は build-license feature で制御が必要。ネイティブ依存 | maybe（分離クレート + opt-in） |
| `hound` / `claxon` / `opus-pure` 等 | 軽量・純 Rust の形式個別クレート | 対応形式が個別・断片的 | maybe（形式単位の補完） |
| `rubato` | リサンプリング専用。RT alloc 回避を設計目標に明記 | 用途限定 | yes（リサンプリング候補） |

### 動画 codec / デコード

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 純 Rust: `rav1d`/`rav1e`（AV1）、`openh264`（H.264） | 純 Rust で実用域にあるのは AV1・H.264 に限定 | それ以外の形式は純 Rust では実用域に達していない。OpenH264 のソースビルドは MPEG-LA 義務移転の可能性（Cisco prebuilt のみ安全・要法務確認） | maybe（対応形式を限定） |
| FFmpeg 系バックエンド | 実質全形式をカバー | LGPL/GPL・配布条件の管理が必須 | yes（広範囲は FFmpeg 必須という現実） |
| `video-rs` / 後継 `rave` | 高水準ラッパー | `video-rs` は WIP 宣言、`rave` は未実用 | no |

### 編集・タイムライン（第 2 ターゲット）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| GES（GStreamer Editing Services） | 成熟した編集エンジン | 重量級・非スレッドセーフ。Ruviform の RT/データモデルと不整合 | no |
| 自前タイムライン基盤 | データモデル・RT 契約と整合 | 設計・実装コスト大 | yes（FilmCraft / Concat / kael_media_engines が "exact integer time" / "Rational time" パイプラインの参照例） |

### 色管理・タイムコード

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `moxcms` | 純 Rust ICC。SIMD 対応。活発 | ICC 範囲のみ（ACES 等の wider な色管理は別） | yes |
| OpenColorIO Rust バインディング | 映像制作級色管理の標準 | Rust バインディングは見当たらず空白と推測（未検証） | maybe（要調査） |
| `vtc` | SMPTE タイムコードで唯一の成熟クレート | 用途限定 | yes |

### GPU

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `wgpu`（v29/v30 系） | 非常に活発。3 OS + Web 対応。エンジン層 opt-in 方針と整合 | 3 ヶ月毎の破壊的更新（semver churn）。MSRV 1.87 | yes（backend 層でバージョン差を吸収する設計が必須） |

### WebView / UI 実装

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `wry`（0.57、Apache/MIT） | 活発。WebView2 / WKWebView / WebKitGTK を抽象化 | 素の `wry` はプラグイン UI として不足（後述） | yes（アプリ・ホスト側） |
| `wxp`（NovoNotes の WebView プラグイン UI 基盤 — プラグインホスト向けパッチ適用済み `wry` を同梱する上位フレームワーク） | 「プラグインホストのライフサイクル」パッチ保持の実績 | フォーク保守・ライセンス詳細は未検証 | maybe（プラグイン GUI で有力） |
| `nih-plug-webview` | WebView-in-plugin の実装例 | 実験段階 | maybe（参照用） |

- 描画下限: WebGL2 が 3 OS 共通の下限。WebGPU は Windows（WebView2）のみ信頼可、WebKitGTK は未整備。
- 配布リスク: Linux の WebKitGTK 実行時依存が配布上の最大リスク。
- 合成: macOS の WKWebView は layer-backed で合成しやすい。Windows は CompositionController が利用可能。

### ネイティブ GUI フレームワーク（比較・参照用）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `vizia` | オーディオプラグイン実績。Skia。`baseview` 内蔵 | エコシステムは egui より小規模 | maybe |
| `egui` | 最大エコシステム。即時モードで道具立てが豊富 | 即時モードの特性は要件適合を要検証 | maybe |
| `iced` | Elm 型アーキテクチャ | オーディオ用途の実績は薄い | maybe |
| `Slint` | 組込・デスクトップ実績 | GPLv3 / Royalty-free / Commercial の三重ライセンス（Royalty-free は帰属表示で無償プロプライエタリ可・組込除外） | no（ライセンス制約） |
| 自作 | DAW 級の描画要件を完全制御 | Meadowlark の失敗談（BillyDM: DAW 級 GUI は既存 FW 不足 → 自作 Yarrow + Firewheel）が示す通りコスト極大 | no（ただし既存 FW 不足の実例として記録） |

### プラグイン規格基盤

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `clack`（0.2.0、MIT OR Apache-2.0、MSRV 1.85） | plugin + host 双方対応の唯一の実用的 safe ラッパー。活発。`clack-extensions` が params/gui/thread-check/thread-pool 等を feature 分割提供 | CLAP のみ | yes（CLAP 第一候補） |
| `vst3`（coupler-rs、0.3.0、MIT OR Apache-2.0） | VST3 ラッパーとして最良。活発。SDK 3.8（2025-10-29 リリース）の MIT 化で GPLv3 `vst3-sys` の存在意義が低下 | VST3 のみ | yes（VST3 第一候補） |
| `truce` | 7 形式対応の上位フレームワーク | 独自ライセンス → コード流用不可・設計参照のみ | no（参照用） |
| `nice-plug`（NIH-plug 後継、ISC） | 上位フレームワークの設計参照 | experimental 自称 | no（参照用） |

### プラグイン GUI ウィンドウ

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `baseview`（0.4.1） | 復活・活発（prokopyl 参加）。遅延描画 + タイマー。プラグイン GUI の事実標準化 | ネイティブウィンドウ専用 | yes |
| WebView-in-plugin（`wxp` / `nih-plug-webview`） | UI 方針（WebView ハイブリッド）と整合 | ライフサイクル統合が課題。`wxp` は同梱するパッチ適用済み `wry` の保守に依存 | maybe |

### SIMD（stable-only 前提）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `fearless_simd`（Linebender） | safe API・dispatch 内蔵・AVX-512 対応 | 新しめのクレート。実績は要検証 | yes（第一候補） |
| `pulp` | safe SIMD の対抗馬。実績あり | API 設計・保守状況を要検証 | maybe |
| `multiversion` + `wide` / `std::arch` | 定石の組合せ | `std::arch` は `unsafe` カプセル化が必要 | maybe |

- `std::simd` は未安定のため stable-only 方針では使用不可。
- aarch64 NEON は baseline のためランタイム検出不要（x86_64 のみ検出・分岐が必要）。

### RT 実践パターン（ライブラリ・手法）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `assert_no_alloc` | RT パスの alloc 検査を CI に組込み可能（nice-plug / truce 統合例あり） | 検出のみで防止ではない | yes |
| `rtrb` | SPSC リングバッファの事実標準 | SPSC 限定（MPMC は別） | yes（RT→非RT 連絡の定石） |
| `basedrop` パターン | RT 側から重い Drop を非 RT 側へ逃がす | 版状況は未検証 | yes（パターンとして採用検討） |
| `audio_thread_priority` | MMCSS / time-constraint / SCHED_FIFO の優先度付与を抽象化 | OS ごとの効果差あり | yes |
| `clack-extensions` の thread-pool / thread-check | CLAP 規格準拠のスレッド契約をそのまま活用 | CLAP 依存 | yes（CLAP 実装時） |

### OFX（映像プラグイン）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `openfx` クレート | Rust バインディングが存在 | 極初期で実用不可 | no |
| 自前 sys 層 | OFX は ASWF 管理・BSD-3・純 C ABI（1.5.1）で C API が単純。技術的には容易 | 実装コストが必要 | maybe（第 2 ターゲット設計時に判断） |

### 時間・同期モデル

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 有理数／整数カウントを中核型にする | 長時間ドリフトなし。`vtc` / FilmCraft / Concat / kael_media_engines の 4 系統が同結論 | API がやや複雑 | yes（ほぼ確定的な方向） |
| f64 秒を中核型にする | 単純 | 長時間ドリフトで編集・同期用途に不適 | no |

- A/V sync: audio master clock + video PTS 追従（ffplay 型）が定石。
- sample-accurate イベント: プラグイン API が sample-offset を保持すれば透過可能。
- 音楽時間（小節/拍/PPQN）は自前設計が必要。

### ライセンス上の注意（設計制約）

- `symphonia` = MPL-2.0、FFmpeg = LGPL（GPL/nonfree は feature 制御）、`truce` = 独自（流用不可）、`Slint` = GPLv3/Royalty-free/Commercial の三重、`vst3-sys` = GPLv3、OpenH264 ソースビルド = MPEG-LA 義務移転の可能性（Cisco prebuilt のみ安全・要法務確認）、ASIO SDK = 独自ライセンス。

## Tentative Conclusions and Hypotheses

Issue #1 の設計論点と、エコシステム調査が示す制約・先行事例の対応。各項目は仮説であり、最終決定は ADR・design-doc で行う。

| Issue #1 設計論点 | エコシステム調査が示す制約・先行事例 | 仮説（傾向） |
| --- | --- | --- |
| クレート DAG | `cpal` のメンテ少数・資金問題、`wgpu` の 3 ヶ月毎 semver churn、FFmpeg のライセンス隔離 | 外部依存はすべて抽象境界の背後に隔離する方針が、調査事実によって強く正当化される。基礎クレートは純 Rust・依存最小を維持 |
| データモデル | 有理数／整数時刻が 4 系統の先行事例で同結論。`vtc` がタイムコード実装の実例 | 中核時刻型は有理数／整数カウント。f64 秒は UI 表示等の限定的用途に留める |
| 処理グラフ・実行モデル | A/V sync は audio master clock + PTS 追従が定石。RT→非RT は SPSC（`rtrb`）が標準。`basedrop` パターン。`audio_thread_priority` | RT スレッド = 事前確保 + SPSC 通知 + Drop 逃がし、オフライン = 同一カーネルのバッチ・並列実行、という二系統構造が有力 |
| DSP／SIMD | stable-only → `std::simd` 不可。`fearless_simd` が第一候補、`pulp` 対抗馬。aarch64 NEON は baseline | safe SIMD ラッパー（`fearless_simd` 等）+ スカラーリファレンスの二層構造。採否はベンチマーク基準で（要 PoC） |
| 検証計画 | `assert_no_alloc` の CI 統合例あり。`clack-extensions` thread-check。OS 別スレッド優先度は `audio_thread_priority` | RT 検証 = `assert_no_alloc` 系の CI 検査 + 静的不可分は文書・レビュー、という契約が実装可能な実例を持つ |
| プラグイン | `clack` + `vst3`（coupler）が両規格の最良ラッパー。`baseview` が GUI 事実標準。`wxp` が WebView-in-plugin の実績 | 規格固有アダプター = `clack` / `vst3`、共通基盤と分離（確定制約と整合）。GUI は `baseview` または WebView 系の二系統を許容 |
| UI | `wry` + WebGL2 下限。プラグインでは `wxp` 等のライフサイクル対策が必要。WebKitGTK 配布リスク | WebView ファースト方針は維持できるが、Linux 配布とプラグイン統合は要 PoC の最大リスク |
| ロードマップ | 動画の純 Rust 実用域は AV1/H.264 のみ → 第 2 ターゲットの広範対応は FFmpeg 系が前提 | Phase 2 の動画基礎では純 Rust 中心、広範形式は分離クレート FFmpeg 系という段階的構成が現実的 |

## Open Questions

調査サブエージェントからの未検証引継ぎ事項、および Issue #1 設計で残る問い:

- `truce` のライセンス条項詳細（設計参照の範囲を含む）
- `ringbuf` / `basedrop` の現行バージョン・保守状況
- WebKitGTK での WebGPU 可否（3 OS での WebGPU 下限確認）
- ACES / OpenColorIO の Rust バインディング（見当たらず空白と推測。要確認）
- `wxp` / `beamer`（helpermedia の AU/VST3 プラグインフレームワーク、`beamer-webview` 含む・MIT）/ `nih-plug-webview` のライセンス詳細
- ASIO SDK の配布条件（Windows オーディオ I/O で必要になる場合）
- `fundsp` / `dasp` 等の DSP ライブラリ層は未調査
- OpenH264 の Cisco prebuilt 以外の使用に伴う MPEG-LA 義務（要法務確認）
- 個別 codec/container の対応範囲（Issue #1 では抽象境界のみ設計し、個別形式の採否は後続実装 Issue に移管 — spec の Out of Scope / Deferred 表と整合）
- UI 具体技術（WebView 実装・描画バックエンド・フレームワーク）の選定 — 要 PoC
- OFX の対応範囲 — 第 2 ターゲット設計時に判断

## Promotion Candidates

- [x] spec-doc needed: Issue #1 の設計ベースライン成果物の要求を [architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md)（SPEC-034INYy2aDLxse9dy8MgGZ、proposed）として並行作成済み
- [x] adr-doc needed: 確定制約を領域別に記録した 6 件の ADR を並行作成済み（`relations.related`）: [media-io-backend-abstraction](../adr/media-io-backend-abstraction.md) / [platform-and-toolchain-targets](../adr/platform-and-toolchain-targets.md) / [realtime-contract-and-verification-policy](../adr/realtime-contract-and-verification-policy.md) / [plugin-architecture-clap-vst3](../adr/plugin-architecture-clap-vst3.md) / [ui-architecture-webview-hybrid](../adr/ui-architecture-webview-hybrid.md) / [licensing-and-distribution-policy](../adr/licensing-and-distribution-policy.md)
- [ ] 今後の昇格候補: Issue #1 で具体化する design-doc 群（クレート DAG・データモデル・実行モデル・検証計画）、UI/描画 PoC 結果に基づく ADR、OFX 対応範囲の設計、個別 codec/container 採否の記録

なお、spec・ADR 群は本 discovery と並行して作成されたため `relations.derived-by` ではなく `relations.related` でリンクしている。`resolved` への昇格は、本書の仮説・未解決問いが design-doc・後続 ADR へ移管された時点で行う。

## Sources and Relations

- 一次ソース: GitHub Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（`relations.source` / `relations.derives-from`）。目的・レイヤー案・横断設計契約・決定すべき事項・成果物・完了条件の正本。
- 入力 1（対話 DECISION）: 「Confirmed Requirements and Constraints」節の内容。ユーザー承認済みの要求・制約であり、記録先は spec [architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md) の Requirements B と 6 件の ADR。
- 入力 2（エコシステム調査）: 調査サブエージェントの生レポート。「Alternatives and Comparison」節に領域別で構造化して収録。`docs/research/` 相当の独立ファイルは存在しないため、本書が調査結果の canonical 記録を兼ねる。
- 関連ドキュメント（`relations.related`）: リポジトリルートの [AGENTS.md](../../AGENTS.md)（暫定開発規約・RT 意図・#1 委譲事項の所在）、spec [architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md)、および 6 件の ADR（本探索の確定制約を領域別に決定記録したもの）。
- 参照（`relations.references`）: Issue [#2](https://github.com/scarecrow173/ruviform/issues/2)（開発基盤、#1 の暫定的足場）、Issue [#3](https://github.com/scarecrow173/ruviform/issues/3)（README/AGENTS 固有化、#1 完了後の差分対象）。
