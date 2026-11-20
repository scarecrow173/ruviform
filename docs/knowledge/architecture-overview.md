---
id: "KNOW-0350CWtG3cUgzAB14TzdHc"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "システムアーキテクチャ概要"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/798"]
  informs: []
  explains: ["../designs/ruviform-architecture-design-baseline.md"]
  supersedes: []
---

# システムアーキテクチャ概要

本書は Ruviform ワークスペース全体のアーキテクチャを、コードの現状に基づいて説明する入門文書である。クレート構成と依存方向、主要レイヤの責務、主要ユースケースの制御・データフロー、横断的な不変条件を扱い、個別領域の詳細は `docs/knowledge/` 配下の領域別文書へ委ねる（末尾の「領域別 knowledge 文書への導線」参照）。設計の意図・経緯の正本は `docs/designs/ruviform-architecture-design-baseline.md`（Issue #1・approved）だが、本書の記述は `Cargo.toml` の依存宣言と `src/` の実装を根拠とする。

## Ruviform の位置づけ

Ruviform は Rust 製の再利用可能なマルチメディア共通基盤である。DAW・再生ホストを第 1 ターゲット、NLE・映像制作を第 2 ターゲットとし、製品アプリケーションは含まない。消費者は外部リポジトリ（AudioLens・Synvelis）であり、基礎クレートはエンジンやホストから独立して単独利用できることが前提である。

ワークスペースはルート `Cargo.toml` に 49 メンバーを持つ。`crates/` 配下の 48 クレートと、Luau FFI 隔離実験用の `ffi-poc`（`ffi-poc/`、本番採用は別ゲートで判断）である。全クレートは `publish = false` であり、配布・公開方針は未確定である。

## 層構造と依存方向

クレートは下から上への一方向依存だけを許可する層構造に編成されている。実際の `path` 依存は各クレートの `Cargo.toml` で検証できる。層の分類は `crates/README.md` の索引に従う。

```text
利用側層（consumer 側・composition root 含む）・テスト専用クレート
        │  ruviform-edit / ruviform-ui* / ruviform-viz / ruviform-script*
        │  ruviform-cli-sample / ruviform-audio-plugin-bridge / poc・test 系
        ▼
Engines          ruviform-audio        ruviform-video
        │
構造層           ruviform-graph（typed Graph IR・非 RT Validator）
        │
拡張層           ruviform-plugin-core → plugin-clap/-vst3/-ofx/-export
        │        ruviform-plugin-host（L0/L1）
        │        ruviform-ui（driver 中立 GUI 基盤）
        ▼
Domain foundations   ruviform-audio-core / ruviform-video-core
                     ruviform-video-proc / ruviform-dsp / ruviform-audio-analysis
        │
メディア I/O 層      ruviform-media-io（境界）+ io-symphonia/io-avi/io-wav
                     ruviform-media-reader（mixed-RT）
汎用 I/O 基盤        ruviform-io-file / io-compression / io-structured / io-archive
        │
Optional platform adapters   ruviform-audio-device / ruviform-midi-device
        ▼
Primitives       ruviform-math / ruviform-numeric / ruviform-rt
                 ruviform-media-core / ruviform-exec / ruviform-observability
```

依存方向のルールは次の通りであり、構造的に強制されている（`Cargo.toml` 検証済み）。

- 依存は上位から下位のみ。循環依存は存在しない。
- Optional platform adapters はポート trait の定義元（`ruviform-audio-core` 等）と Primitives にだけ依存する。foundations から adapters への逆方向依存は禁止。
- エンジンから拡張層（plugin・UI）への直接依存は禁止。実際、`ruviform-plugin-host` が `ruviform-audio` へ持つのは dev-dependency（graph E2E 試験用）だけであり、エンジンとプラグインホストを本番コードで結ぶのは利用側層の `ruviform-audio-plugin-bridge` だけである。
- plugin 系と ui 系の相互依存は禁止。両側を結線する composition root（`ruviform-poc-native-dock`、`ruviform-plugin-dock-test`）は利用側層に置かれる。
- テスト専用クレート（`ruviform-audio-test`、`ruviform-ui-test`、`ruviform-plugin-test-*`、`ruviform-plugin-dock-test`、`ruviform-io-e2e`）は本番 API を消費する側であり、本番クレートからこれらへの依存は存在しない。
- 外部ライブラリの型・feature 名をポート境界の公開 API に露出しない。`symphonia`、`cpal`、`clack`、`vst3`、`wry`/`tao`、`gpui`、`mlua` 等はすべて対応アダプタクレートの内部実装に閉じる。GUI・外部バックエンド依存は非既定 feature でゲートし、`cargo check --workspace` が headless 環境で通る構成を維持する。

### 層ごとの責務

| 層 | クレート | 責務 |
| --- | --- | --- |
| Primitives | `ruviform-rt` | RT 契約の定義（`Processor`/`RtProcessor` 分離・`RtContext`・`MasterClock`）と RT-safe プリミティブ（SPSC ring・retire channel・snapshot slot）。実装詳細の `arc-swap`/`rtrb` は非公開。 |
| Primitives | `ruviform-math` | f32 SIMD カーネル層。`SimdToken` を prepare 時に一度だけ作り、RT パスで dispatch を行わない。`fearless_simd` は非公開。 |
| Primitives | `ruviform-numeric` | f64 高精度オフライン数値・DSP（FFT・リサンプラ・相関・STFT・band 解析）。plan 構築は非 RT、処理 API は RT 適合のものを明示。 |
| Primitives | `ruviform-media-core` | 有理数時刻（`TimeBase`/`MediaTime`/`MediaDuration`/`MediaRange`）、`Fact<T>`・`TimestampProvenance`、トランスポート値型と `TransportScheduler`、音楽時間（テンポマップ・PPQN）、MIDI 値型、共通 `Error`。値型のみ・処理・I/O を持たない。 |
| Primitives | `ruviform-exec` | 非 RT 実行基盤（`OfflineDriver`・`OfflineJob`・`JobExecutor`・スレッド優先度機構）。`ruviform-rt` のみに依存。 |
| Primitives | `ruviform-observability` | 共有観測基盤（counter/gauge/histogram の writer-reader ペア、source 登録・寿命契約、固定長 `EventRecord`、RT→非 RT の `rt_bridge`）。`ruviform-rt`・`ruviform-media-core` への一方向依存。 |
| Domain foundations | `ruviform-audio-core` | 音声データモデル（正準 planar f32・`AudioBuffer`/view・`SampleRate`・`ChannelLayout`）と device ポート・`midi_input`/`midi_output` ポート trait の定義。 |
| Domain foundations | `ruviform-video-core` | 動画データモデル（`VideoFrame`・`Plane`・`PixelFormat`・色メタデータ）。`Arc` 共有 backing。 |
| Domain foundations | `ruviform-video-proc` | 色変換・コンポジット等のビデオ処理カーネル（`RgbaF32` linear working space・feature `icc`）。 |
| Domain foundations | `ruviform-dsp` | ドメイン型付き audio DSP カーネル（`Window`・`Biquad`・`FirFilter`・`Fft`・`LinearResampler`）。`ruviform-math` 経由で SIMD。 |
| Domain foundations | `ruviform-audio-analysis` | 解析層（`LevelMeter`・`SpectrumAnalyzer`・`WaveformPeaks`）。 |
| 汎用 I/O 基盤 | `ruviform-io-file` | `StagedFile`/`SealedFile` 型状態と分類済み publish 契約（`PublishReceipt`/`PublishFailure`・recovery handle）、rooted read capability。非 RT。 |
| 汎用 I/O 基盤 | `ruviform-io-compression` | 借用ストリーム圧縮（gzip/deflate/zstd・明示 `finish`・`VerifiedEnd`・累積 budget）。非 RT。 |
| 汎用 I/O 基盤 | `ruviform-io-structured` | Serde 型付き構造化ドキュメント I/O（JSON/YAML/TOML/CSV/XML・Strict EOF・byte budget）。非 RT。 |
| 汎用 I/O 基盤 | `ruviform-io-archive` | ZIP/TAR アーカイブ（索引読み・CRC/中央 Directory 検証・安全展開）。非 RT。 |
| メディア I/O | `ruviform-media-io` | demux/mux・decode/encode の 4 系統 trait 境界（`Demuxer`/`Decoder`/`Muxer`/`Encoder`）と `Packet`/`Frame` 通貨型。非 RT。 |
| メディア I/O backends | `ruviform-io-symphonia` / `ruviform-io-avi` / `ruviform-io-wav` | 入力 backend が 2 系統（symphonia 汎用・pure-Rust AVI）、出力 backend が 1 系統（pure-Rust PCM WAV の `Muxer`/`Encoder`）。 |
| メディア Reader | `ruviform-media-reader` | `SourceReader`（stream 単位の逐次 decode・source-frame 位置付け）+ `Prefetch`（bounded block pool と固定 SPSC ring による RT 供給）。mixed-RT。 |
| Optional adapters | `ruviform-audio-device` | cpal アダプタ。`audio-core` device ポートの `Box<dyn>` 実装。Windows では WASAPI 経路も持つ。 |
| Optional adapters | `ruviform-midi-device` | OS MIDI アダプタ。Windows は WinMM、他 OS は `Unsupported` を返す。 |
| 拡張層 | `ruviform-plugin-core` | 規格中立プラグイン基盤（`PluginProcessor`/`Plugin`/`PluginFactory`・`EventList`・`Lifecycle`・L1 wire codec・transport bridge）。音声 `process` は RT、動画は NONRT 契約。`ruviform-rt` には意図的に依存しない。 |
| 拡張層 | `ruviform-plugin-clap` / `ruviform-plugin-vst3` / `ruviform-plugin-ofx` | 規格アダプタ。CLAP/VST3 は薄い ABI 変換 + feature `host` で host 側 bridge と L1 runner bin。OFX は NONRT L0 の video ホストアダプタ。 |
| 拡張層 | `ruviform-plugin-export` | プラグイン側 L1 薄ランナー（子プロセスエントリ `run`/`run_video` + 参照 runner bin）。 |
| Host 層 | `ruviform-plugin-host` | 規格中立ホスト境界。`PluginHost` composite が L0（in-process）と L1（out-of-process・framed stdio + shared memory）を統一 `PluginHandle` で振り分ける。`PluginHost::video()` が NONRT video 面。 |
| 構造層 | `ruviform-graph` | 型付き multi-bus/port Graph IR + 全エラー蓄積の非 RT `Validator` + `GraphProject`/`StableHandleMap`。`ruviform-audio-core` のみに依存。 |
| Engines | `ruviform-audio` | オーディオエンジン。v1 pull グラフ（`Graph`/`Mixer`/`GraphPlan`/`PlanSwap`/`Session`）と typed ランタイム（`typed::Compiler`→`PreparedPlan`→`GraphRuntime`/`RtRuntime`）の 2 系統 + オフラインレンダ `render`。 |
| Engines | `ruviform-video` | ビデオエンジン（`FramePipeline`・A/V sync・seek 仲介）。`ruviform-graph` には依存しない。 |
| 利用側層 | `ruviform-audio-plugin-bridge` | ホストプラグインを typed グラフの `GraphProcessor` ノードへ写像するアダプタ（bus→port 射影・`PluginNode`/`PluginNodeControl`・MIDI input stage）。 |
| 利用側層 | `ruviform-edit` | NLE 非破壊編集データモデル + `Timeline::evaluate` 評価射影。`ruviform-media-core` のみに依存し engine 中立。現状ワークスペース内の消費者はいない。 |
| 利用側層 | `ruviform-ui` 系 | driver 中立 GUI 基盤（`WindowDriver` ポート・`ViewHost` 射影・dock・`native_slot`・`UiRuntime`・`rvf.ipc.v1`）+ adapter（`ui-webview`/`ui-gpui`）+ `ui-theme` + `ui-test`。 |
| 利用側層 | `ruviform-viz` | 可視化プリミティブ（波形 LOD・STFT spectrogram・timeline 座標・RVBW/RVST 転送形式）。GUI 非依存。 |
| 利用側層 | `ruviform-script` / `ruviform-script-luau` | 言語中立 script/analyzer 契約ポート + vendored Luau 実 backend/analyzer。 |
| 利用側層 | `ruviform-cli-sample` | headless CLI 利用例（`probe`/`run` の JSON 契約・`OfflineJob` 駆動の解析）。 |

## RT / 非 RT 分離の仕組み

この基盤のアーキテクチャを貫く最も重要な概念が、リアルタイム（RT）パスと非 RT パスの分離である。RT パスとは、オーディオデバイスのコールバックが実行されるスレッド上の処理であり、そこでは allocation・blocking・OS 呼出・無界の仕事が契約違反となる。

### `Processor` / `RtProcessor` の分離と `RtContext`

`ruviform-rt` が定義する `Processor::prepare` は非 RT のセットアップ段階で、allocation や I/O を行ってよく、戻り値として別オブジェクト（`RtProcessor`）を返す。`RtProcessor::process(&mut io, ctx: &RtContext)` が RT 半分で、「処理失敗は `Result` でなく io ドメインの状態で表す」という infallible 契約になっている。prepare 済みかどうかが型で表現される設計である。

`RtContext` はゼロサイズの capability トークンであり、`process` 系 API はすべて `&RtContext` を要求する。生成口は `RtContext::mint()` だけで `#[doc(hidden)]` に隠され、「driver エントリポイントだけが mint する」という運用契約になっている。これは型システムの強制ではなく慣例とレビューによる境界であり、現状の正当な mint 地点はデバイスコールバック、`OfflineDriver`、エンジンのオフラインレンダの 3 系統である。

### RT パスでデストラクタを走らせない仕組み

RT 上で参照が切り替わる構造は、退役オブジェクトの破棄を非 RT 側へ送る仕組みを共通して使う。

- `retire_channel` は deferred `Drop` のための片方向チャネルで、RT 側から `Retirer` へ押し込み、非 RT 側の `DropCollector` が回収する。容量超過時は drop せず `mem::forget` による leak-on-full を選択し、溢れた個数はカウンタで観測可能である（drop-on-full は RT 上でデストラクタが走るため却下された）。
- `slot` は `arc-swap` 上のスナップショット交換で、非 RT 側の `SnapshotWriter::publish` と RT 側の lock-free `SnapshotReader::read`（`SnapshotGuard` 経由で退役経路に接続）を提供する。RT スレッドでは contract 区間の前に一度 `read` してウォームアップする必要がある。
- `ruviform-audio` の `PlanSwap`/`RtPlan` は同じ発想を `RwLock::try_read` で実装した v1 版で、RT 側は成功時に `Arc` を `cached` に保持し、追い出された `Arc` は `Retirer` へ送る。読み取り失敗時は最後に成功した plan を使うため、グラフ更新は最大 1 ブロック遅れで届く。

### 世代（generation）による古い情報の排除

ハンドルと非同期コマンドには世代番号が付けられ、RT 側・受信側が古い世代を「数えて捨てる」規則が一貫している。`PluginHandle` の slot+generation、`RtRuntime` の `RuntimeCommand`/`NodeEvent`、prefetch の generation、`ParamDispatchLedger` の `BindingGeneration` が同じパターンである。部分的に失敗した publish は generation が合わないコマンドとして破棄され、最後に正常だった状態が維持される。

### `OfflineDriver` が同じ契約を駆動する

`ruviform-exec` の `OfflineDriver` は任意の `Processor`/`RtProcessor` をチャンク単位で駆動する非 RT driver であり、正当な `RtContext` の mint 地点である。つまり RT 用に書かれたプロセッサは、オフライン処理でも同じコードパスで実行される。`run` は呼び出しスレッドで逐次、`run_parallel` は呼び出しごとに scoped worker を spawn/join し、結果はシーケンス番号で再構成されるため逐次実行と一致する。キャンセルは `CancellationToken`（`Arc<AtomicBool>`）をチャンク境界で見る協調キャンセルのみで、強制中断は存在しない。

## 主要フロー

### リアルタイム音声出力（v1 グラフ）

`Session::open` が `ruviform_audio_core::device::Device` ポート（`Box<dyn>`）に設定とコールバックを渡し、`OutputStream` を開く。データコールバックは backend の RT スレッド上で `RtPlan::active` で現行 plan を取り、`RtContext::mint` して `GraphPlan::drive` を呼ぶ。`drive` は `Mutex<Graph>` を `try_lock` し、失敗時はそのブロックを無音にする（poison 後は永続的に無音——意図的な設計）。`Graph::pull` は登録順に各 `AudioNode::pull(&mut out, &ctx)` を呼び、`Mixer` が planar f32 を加算する。

デバイス境界では `ruviform-audio-device`（cpal）が開くときに一度だけ planar scratch を確保し、interleaved なデバイスバッファを `frames_capacity` で区切って `write_interleaved`/`read_interleaved` が変換する。コールバック内で再確保はなく、書き残した領域は必ずゼロ埋めされる。`Session` 自体はアダプタ型を知らず、ポート trait のみを話す。

### typed graph ランタイム

`ruviform-graph` の `GraphBuilder` が `NodeDef`/`PortDef`/`ConnDef` から ID を採番して `GraphSpec` を作り、`Validator::validate` が全ルール（端点方向・format 一致・multiplicity・参照整合・cycle 拒否）を検査した `ValidatedGraph` を返す。`ValidatedGraph` は必ず DAG であり、`StableHandleMap` が論理 ID と runtime index（Kahn 順）を双方向に写像する。

`ruviform-audio` の `typed::Compiler` は `ValidatedGraph` を不変の `PreparedPlan` にコンパイルし、`GraphRuntime::new` がスロット・delay line・プロセッサ登録域の RT arena を事前確保する。`GraphRuntime::publish` は install/remove コマンドを SPSC で送り plan `Arc` を publish する。RT 側の `RtRuntime::process` は generation が変わったときだけ command drain と入れ替えを行い、退役プロセッサは retire channel へ、新規は `activate` してから各 `ScheduledNode` を駆動する。プロデューサスロットの未書き込み検出（uninit read）は fault として数え、古いデータを読ませない。

### プラグイン統合

呼び出し側は `PluginHost` に factory を登録する。L0（`register_factory`）か L1（`register_out_of_process`）かは登録経路で決まり、`Instantiate { plugin_id }` 以降は統一 `PluginHandle` だけを見る。非 RT 操作（scan・ライフサイクル・state・GUI）は `HostCommand`/`HostReply` のコマンド面を通り、`process` はコマンドではなく別経路を通る設計である。

- L0: `InProcessHost` が `Box<dyn ErasedPlugin>`（または sendable 版の `Shared` スロット）を保持し、`process` は直接 `PluginProcessor::process` を呼ぶ。`try_lock` が `WouldBlock` のときは `EndpointUnavailable` を返して無音にせず、呼び出し側が bypass する。
- L1: 子プロセス + framed stdio control plane + shared_memory data plane。`RtLeg::process` は lookahead blocks 先のブロックに答える（既定 `2 × max_frames`、48 kHz・512 で約 21.3 ms）。underrun はエラーにせず最後に受信したブロックを通すか無音にする。子プロセスの live exit は全インスタンスを `Failed` に収束させ、`process` は silence + `Ok(CONTINUE)` を返す。

`PluginHost::prepare_endpoint` は `PluginEndpoint`（`Send`・move-once）を発行し、それを `ruviform-audio-plugin-bridge` の `attach` が `PluginNode`（typed ランタイムの `GraphProcessor`）に包む。`PluginNode::process` は SPSC の `NodeControl` drain、MIDI input stage、`TransportScheduler::run_callback`、物理呼出ごとの `EventList` 構築、`endpoint.process` の順で処理し、`latency()` がコンパイラの PDC（plugin delay compensation）入力になる。L0 と L1 は同じ `endpoint` 語彙で見えるため、グラフからは透過的である。

### メディア読込から RT 消費

ファイルからの音声供給は、`media-io` 境界（非 RT）を通り、`media-reader` の prefetch が RT へ届ける形になる。`SourceReader` は `Box<dyn Demuxer>` + `Box<dyn Decoder>` を持ち、`next_frame`/`read_planar`/`seek_source_frame` を source-frame 単位で提供する。`Prefetch` は `block_count` 個の `PcmBlock` プールと 2 本の固定容量 SPSC ring（data ring・free ring）を持ち、producer が非 RT で decode を溜め、`PrefetchConsumer::fill` が RT 端点で受け取る。free ring の容量はプールと同じため、消費側の返却 push は失敗しえず、RT での解放は発生しない。seek/reopen は generation を上げて `Boundary` マーカーを先頭に挿入し、消費側が世代を adopt する。underrun と eof は別の状態として報告される。

### オフラインレンダとジョブ実行

`ruviform-audio` の `render` は `RenderSink` が `callback_chunk` 単位で `TransportScheduler::run_callback` を駆動し、物理呼出ごとに `RtContext::mint` で `rt.process` を呼ぶ。出力は `AudioFrame` に包まれて `Encoder` → `Muxer` → `StagedFile` の publish に流れ、`PublishReceipt`/`PublishEffect` が「実際に publish されたか」の正直な証拠を返す。ジョブ実行の基盤は `ruviform-exec` にあり、`OfflineJob` は「1 job = 1 専用スレッド」、`JobExecutor` は明示的な bounded pool、進捗は atomic counter（正確）と bounded event channel（latest-wins・不可欠でないイベントは drop）の二面である。

`ruviform-cli-sample` はこの組み合わせを使う最小の利用側実装で、`probe`（demux で container/stream fact を列挙）と `run`（`OfflineJob` で decode → `OfflineDriver` で meter 駆動 → `AnalysisReport`）を提供し、JSON 契約と exit code を持つ。

### MIDI 入力

`ruviform-midi-device`（Windows は WinMM、他 OS は `Unsupported`）がドライバコールバックで事前確保 `VecDeque`（容量 1024・短い `Mutex`）へ積み、`MidiInput::read`（非 RT）が `TimedMidiInput` を取り出す。時刻刻印はアダプタが宣言した clock domain・timebase・provenance を正直に持ち、audio サンプル時計への写像は consumer 側の責務である。`ruviform-audio-plugin-bridge` の `InputTx::pump`（非 RT・engine block ごとに呼ぶ）が SPSC へ積み、RT 側の `InputStage` が現在公開されている `ClockAnchor` で `map_stamp_to_frame` して `EventList` へ載せる。overflow・unsupported・anchor 未解決はすべてカウントされる。

### GUI と Native Dock

GUI 層は driver 中立の `ruviform-ui` が語彙（`WindowDriver` ポート・`ViewSpec`/`ViewHost` 射影・`DockWorkspace`・`NativeSurfaceSlot` 契約・`rvf.ipc.v1` IPC schema・`UiRuntime` の service/job dispatch）を持ち、OS 依存は `ruviform-ui-webview`（wry/tao・feature `webview`）と `ruviform-ui-gpui`（GPUI 0.2.2・feature `gpui`）の adapter に閉じる。`UiRuntime` は `Service` に登録した op を dispatch し、job ワーカーの進捗は bounded mailbox（256）で latest-wins に合体して UI スレッドへ届く。

Native Dock（プラグインの実 native editor を host ウィンドウに埋め込む）は、plugin 系と ui 系が相互依存できないため composition root（`ruviform-poc-native-dock`/`ruviform-plugin-dock-test`）が結線する。`NativeSurfaceSlot` で slot を宣言 → `lease_parent` が `SlotLease`（generation 付き opaque token）を mint → `ParentSlotLease` に包んで plugin 側へ渡し → `PluginEditorSession` が `EditorSessionOps` ポート経由で open → `LeaseRedeemer` が ledger 照合して実コンテナの token に換え → plugin が子ウィンドウを作る、という順になる。lease payload を生ハンドルとして再解釈する経路は存在しない。

### Observability

`ruviform-observability` は counter/gauge/histogram の writer-reader ペア（`Arc<Atomic*>`・wait-free）を提供し、`source` 層が登録・寿命契約（`SourceId`/`SourceGeneration`/retire→final drain→removed）を、`record` 層が固定長 `EventRecord` を、`rt_bridge` が `ruviform-rt` の SPSC ring 上に RT→非 RT の記録経路（`RtRecorder::try_record`・満杯時は drop してカウント）を提供する。collector は pull-only で、subscription/コールバック API は設計上存在しない。

## 横断的な不変条件・契約

コード全体に繰り返し現れる契約は次の通りである。

- **RT 契約は API 単位で明記**: 各公開要素に `RT-suitable` か `non-RT only` のラベルがあり、`assert_no_alloc` を使う `rt_` 系レーンで測定される。`Processor::prepare` / `RtProcessor::process` の分離と `RtContext` による capability ゲーティングが型で表現され、mint 制限と「違反を型では防げない部分」は文書化された慣例である。
- **RT パスで destructor を走らせない**: retire channel・snapshot slot・`PlanVault`（transport plan の clone を node 寿命で保持）など、退役は非 RT 側へ送る。overflow は leak-on-full でカウンタ可視。
- **正直な capability・欠損の報告**: `Fact<T>`（`Known`/`NotDeclared`/`Unsupported`/`Malformed`）、`TimestampProvenance`（`Carried`/`Derived`/`Estimated`）、`PublishEffect`（`NotPublished`/`Published`/`Indeterminate`）、`ScenarioVerdict`（pass/fail/not_run/unsupported/blocked）など、「分からない・できない・失敗した」を型で返す。fake success は欠陥として扱われる。
- **完了証明は receipt のみ**: `finish()`/`seal()→publish()`/`finalize()` が完了の証拠であり、`Drop` の best-effort 処理は領収書にならない。
- **有界性・予算**: SPSC ring の容量、`ScheduleBudget`、`ArchiveReadLimits`、`InputLimits::max_bytes`、prefetch の `block_count`、mailbox サイズ等が定数で決められ、溢れは `Backpressured`/`LimitExceeded`/カウント付き drop として報告される。
- **世代・ID の意図的な stale 検出**: `PluginHandle`/`SurfaceSlotId`/`PluginEditorSessionId`/`SourceId`/`BindingGeneration` など generation 付き handle が多用され、古い参照は構造的に拒否される。
- **公開 enum は `#[non_exhaustive]`**: `workspace.lints.clippy.exhaustive_enums` で強制される。
- **`unsafe` は原則禁止**: `unsafe_code` が lint ゲートで実質 deny になり、必要箇所は SAFETY コメント付きの局所許可だけが残る。FFI/vendor 型・unsafe は adapter クレート内部に閉じる。
- **headless を保つ feature ゲーティング**: GUI・外部 backend 依存は非既定 feature で、`webview`/`gpui`/`host`/`l1-runner`/`icc`/`file` 等が opt-in。

## テスト・検証基盤の構成

検証方針の正本は `POLICIES.md`（OS-L1/GUI-L2/EXT-L3 の要求状態 Required/Deferred/NotApplicable と実行結果 Pass/Fail/NotRun/Unsupported の分離）である。コード側では次の構造になっている。

- **テスト専用クレートの分離**: `ruviform-audio-test`（VirtualClock・PCM fixture・MIDI trace・scripted backend・pipeline legs・独立 oracle）、`ruviform-ui-test`（`rvf.uitest.v1`・FakeHost/FakeDriver・scenario contract）、`ruviform-plugin-test-vst3`/`-clap`（実 ABI の test cdylib）、`ruviform-plugin-dock-test`（native dock scenario）、`ruviform-io-e2e`（層横断 I/O 試験）はいずれも本番クレートから依存されない葉側 consumer である。`ruviform-ui-test` の `gui-boundary-guard` が dep グラフと公開面の漏出を機械的に検査する。
- **deterministic backend と実 ABI の使い分け**: OS-L1 では決定論的 backend と fixture で契約・精度を試し、EXT-L3 では本番 OS API・実 Plugin ABI・実 Window を通す仮想環境を使う。Mock を実 OS API の Pass としない方針が verdict 型にも反映されている。
- **RT 検証**: `assert_no_alloc` を組み込んだ `rt_` 系テストと `mise` の検証タスクで、RT-suitable とラベルされた API の no-alloc/bounded を機械的に測る。
- **入口の統一**: `mise.toml` がツールとタスクの正本で、`check`/`lint`/`test`/`build` は対象 package 必須引数、`*-full` は全体検証の任意入口、`mise run setup` が APM・prek・Git LFS を復元する。

## 領域別 knowledge 文書への導線

`docs/knowledge/` の領域別文書は次の通りである。本書で概観した項目の詳細は該当文書を参照する。

| 文書 | 扱う範囲 |
| --- | --- |
| [rt-foundation-and-verification.md](./rt-foundation-and-verification.md) | `ruviform-rt` の契約・プリミティブ・RT 検証レーン |
| [media-core-data-model.md](./media-core-data-model.md) | 時間・transport・音楽時間・`Fact`/`Error` のデータモデル |
| [media-time-and-transport.md](./media-time-and-transport.md) | 時刻表現・トランスポート・`TransportScheduler` |
| [media-io-and-file-formats.md](./media-io-and-file-formats.md) | `media-io` 境界・各 io-* backend・`StagedFile` 等ファイル系基盤 |
| [typed-graph-and-offline-render.md](./typed-graph-and-offline-render.md) | `ruviform-graph` IR・typed ランタイム・オフラインレンダ |
| [job-execution.md](./job-execution.md) | `ruviform-exec` の `OfflineJob`/`OfflineDriver`/`JobExecutor` |
| [audio-device-and-midi.md](./audio-device-and-midi.md) | `ruviform-audio-device`・`ruviform-midi-device` の backend 設計 |
| [plugin-format-adapters.md](./plugin-format-adapters.md) | `ruviform-plugin-clap`/`vst3`/`ofx` の規格アダプタ |
| [observability-and-diagnostics.md](./observability-and-diagnostics.md) | `ruviform-observability` の設計と診断 |
| [gui-architecture-gpui-dock.md](./gui-architecture-gpui-dock.md) | `ruviform-ui*` の driver 中立アーキテクチャ・GPUI・native dock |
| [gui-view-contracts-and-testing.md](./gui-view-contracts-and-testing.md) | view 契約・widget・GUI テスト基盤 |
| [dsp-simd-and-kernels.md](./dsp-simd-and-kernels.md) | `ruviform-dsp`/`ruviform-math`/`ruviform-numeric` の数値・SIMD |
| [scripting-luau.md](./scripting-luau.md) | `ruviform-script`/`ruviform-script-luau` の Luau 基盤 |
| [ci-and-verification-lanes.md](./ci-and-verification-lanes.md) | CI レーン・ゲート・検証基盤の運用 |
| [development-environment-and-toolchain.md](./development-environment-and-toolchain.md) | mise・ツールチェーン・MSRV |
| [docs-operations-and-agent-workflow.md](./docs-operations-and-agent-workflow.md) | ドキュメント運用・エージェント作業基盤 |

## 調査方法と正本

本書は pstack の `how` スキルの手順に従い、読み取り専用の探索サブエージェントを RT 音声・プラグイン系、オフライン実行・グラフ系、メディアデータモデル・I/O 系、GUI・観測・スクリプト・テスト系の 4 系統に分けて起動し、その報告を統合して記述した。各報告は `Cargo.toml` の `path` 依存と `src/` の実装を根拠としており、設計ベースラインとの差異はコードを正とした。依存方向・層分類・RT 契約・主要フローは、記述時点のコードで直接検証した範囲に基づく。
