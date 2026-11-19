---
id: "KNOW-qMB0pQVvI4hAwfV9bwNVNh"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "typed graph・コンパイラ・オフラインレンダ"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/21", "https://github.com/scarecrow173/ruviform/issues/24", "https://github.com/scarecrow173/ruviform/issues/25", "https://github.com/scarecrow173/ruviform/issues/27", "https://github.com/scarecrow173/ruviform/issues/113", "https://github.com/scarecrow173/ruviform/issues/184", "https://github.com/scarecrow173/ruviform/issues/187", "https://github.com/scarecrow173/ruviform/issues/195", "https://github.com/scarecrow173/ruviform/issues/196", "https://github.com/scarecrow173/ruviform/issues/198"]
  informs: []
  explains: []
  supersedes: []
---

# typed graph・コンパイラ・オフラインレンダ

この文書は、RT 基盤プリミティブ・pull グラフ engine・typed graph IR・コンパイラと実行ランタイム・plugin 接続・オフラインレンダまで、オーディオ処理系の設計と実装から得られた再利用可能な知見を記録する。出典となる実装記録は削除済みのため、この文書は単独で読めるように書く。

各知見には出典の Issue 番号を付記する。実測値や観測された挙動は事実として記し、解釈を含む記述はその旨を明示して区別する。

## RT 基盤プリミティブと RT 契約の規約

### 依存の選定と配置

RT 基盤は、SPSC リングに `rtrb`（依存ゼロ・オーディオ用途の実績）、スナップショット交換に `arc-swap`、スレッド優先度に `audio_thread_priority`（`default-features=false` で dbus 依存を排除）を private 依存として採用した。ringbuf の自前実装・AtomicPtr による RCU・left-right などの lock-free 構造は `unsafe` を要してワークスペース規約と衝突するため、いずれも却下された (Issue #21)。

遅延ドロップは basedrop を使わず、SPSC 上に自前の retire channel を構成した。channel 満杯時は drop-on-full を拒否し、`core::mem::forget` による leak-on-full とする。RT 側で drop・dealloc・blocking を起こさないための選択である。`arc-swap` 単体では、reader 側の last-drop で dealloc が RT スレッド上で走る経路が残るため、専用 guard 型と retire channel の組合せで防いだ (Issue #21)。

配置は `ruviform-exec` とした。`ruviform-rt` は Primitives 層の「OS 固有 API・デバイス依存を持たない」制約に反するため置けなかった (Issue #21)。`audio_thread_priority` は MPL-2.0 であり、公開前のライセンスレビュー要確認項目として記録されている (Issue #21)。

### スレッド優先度の self-promotion モデル

thread_priority 機構は self-promotion 呼び出しモデルを取る。exec 管理スレッドは spawn 時にセットアップし、デバイスコールバック由来のスレッドは初回コールバックの preamble（RT 契約区間の開始前）で呼ぶ。active な RT 契約区間内からの呼び出しは契約違反である。`default-features=false` 構成では、rtkit・CAP_SYS_NICE・RLIMIT_RTPRIO のない非特権 Linux で典型的に `Err` が返る。これは仕様上の失敗として文書化され、`Result` で表出してパニックしない best-effort とした (Issue #21)。

guard の `Send`/`Sync` は実装上の修正点だった。upstream の handle から構造導出したままでは、unix で guard を別スレッドへ move でき、その Drop が upstream の `assert!(pthread_self() == pthread_id)` に衝突して safe API から panic し得た。`PhantomData<*const ()>` フィールドを追加し、全プラットフォームで `!Send`/`!Sync` に強制して、demotion の calling-thread セマンティクスを型で強制した (Issue #21)。

### RT 契約ラベルと `rt_` 命名規約

全公開 API には `RT contract: RT-suitable`（alloc/dealloc なし・ノンブロッキング・有界）または `RT contract: non-RT only` のラベルを付ける規約で、保証範囲は API 単位であり、OS レベルのハードリアルタイム保証は提供しない。`channel`・`retire_channel`・`slot`・`promote_*`・`demote_*`・`DropCollector::drain`・`SnapshotWriter::{publish,collect}` など確保や syscall を伴う構築系はすべて non-RT only と分類される (Issue #21)。

`RtContext` は公開 ctor・公開 field・モードフラグを持たない単一の opaque 型であり、mint は `#[doc(hidden)] pub` で駆動側の入口点のみが使う。強制は規約とコードレビューに依り、型レベルの完全な抑止ではない。この弱体化は ADR に記録済みである (Issue #21)。

RT 契約テストは `cargo test --workspace rt_` の部分文字列フィルタで選ばれる。非 RT のテスト・doctest・ヘルパー名は `rt_` という部分文字列を含まない命名とし、`assert_*`・`start_*` は `rt_` を含むため使えない（`expect_*`・`spawn_*` などを使う）。実際、`shared_start_failure_rolls_back_to_activated` が `start_` 内の `rt_` に一致して RT テストの実一致件数に混入した例が記録されている。0 件一致のスキップを成功と偽らないため、実一致件数を記録する運用とした (Issue #21, Issue #113)。

## オフライン実行ドライバの決定論と Send 境界

`ruviform-exec` のオフライン driver は `Processor`/`RtProcessor` をチャンク列へ逐次・並列に駆動する (Issue #24)。決定論の保証はシグネチャ分割で表現した。逐次実行の `run` は Send 境界を持たず非 Send processor を許容し、並列の `run_parallel` のみ `Send` を必須とする。`run_parallel` は固定 `std::thread` プール・有界 `sync_channel`・`BTreeMap` による出力順序の再構成で実装し、有界キューが投入側ブロックによるバックプレッシャーを提供する。各ワーカーは `setup.clone()` で prepare し、prepare 失敗は集約し、投入ループで cancel を確認する (Issue #24)。

ワーカーのパニックは `OfflineError::WorkerPanicked` へ写像する。`OfflineConfig`/`OfflineError` は `#[non_exhaustive]` で `RT contract: non-RT only` を明記し、`RtContext::mint()` の発行はドライバ内部に限定した (Issue #24)。

## audio engine の pull グラフとスナップショット差替

`ruviform-audio` は pull グラフ・ミキサー・スナップショット差替を実装する engine crate である。`Session::open_output` が `OutputStream` callback → `RtContext::mint` → `plan.graph.process` を配線する。公開 API に adapter 型を出さない方針のため `ruviform-audio-device` には依存せず、テストではモックで代替する (Issue #25)。

実行中の RT コールバックが参照するグラフの差替方式は、`RwLock<Arc<Plan>>` と RT 側 `try_read` の組合せを採用した。RT コールバックは `try_read` でスナップショットを取得し、競合時は直前に cache した `Arc` clone を使い続ける。最大 1 ブロックの遅延で吸収することを契約として明記する。退役した Plan の `Arc` は非 RT 側が保持して自然に drop する。RT スレッド上での `Arc` clone/drop が参照カウントのみの allocation-free であることは事前に確認済みである (Issue #25)。

代替案はいずれも却下された。`arc-swap` は真の lock-free だが外部依存の追加と検証コストを理由に見送り（実測でボトルネックが確認された場合に再検討）、自前 RCU は `unsafe` と検証コストの大きさで、SPSC コマンドによる差替適用は不変プランを非 RT 側でしか構築できないため不適合とされた (Issue #25)。

## video engine の構成と seek 順序

`ruviform-video` は `VideoStage`/`FramePipeline`（pipeline）、`SyncDecision`/`SyncThresholds`/`FrameScheduler`（sync）、demux・decode・pipeline を調停して seek を扱う `VideoSession`（session）の構成とされた。seek は `demux.seek` → `decoder.flush` → `scheduler.reset` の順序実行が要件である。なお、この Issue の記録は計画段階のもので、判定表テスト・検証列・doc 監査の実行結果は確認されていない (Issue #27)。

## ruviform-graph の Graph IR と Validator

`ruviform-graph` は、typed multi-bus の Port/Connection Graph IR・非 RT の Validator・`GraphProject`/`StableHandleMap` を持つ crate として新設された。workspace の構造層では domain foundations 後・engine 前に置く。全て additive で既存 crate は不変のため、rollback は `git revert` で完結する。Compiler・Buffer・PDC・RT 実行・plugin 実接続は後続 Issue へ延期された (Issue #184)。

通常依存は `ruviform-audio-core` のみとし、`ruviform-media-core` への直接依存は crate-local の error 型採用によって回避した（推移的にのみ到達する）。これは設計からの意図的な相違として記録されている。`LatencyStatus` はこの段階では capability 記録のみで、runtime の意味は後続段階へ延期された (Issue #184)。

Validator は規則 1-7 を実装し、cycle 判定は部分グラフではなく全グラフの SCC で行う。`StableHandleMap` は Kahn のトポロジカルソートに `BTreeMap` の tie-break を組み合わせて決定的順序を保証し、`GraphProject` は requested → validate → published の revision 順で発行する。RT 境界として、`ruviform_rt`・`RtContext`・`unsafe` が crate 内に 0 件であることが検証項目化されている (Issue #184)。

実装時の実測では、`ChannelLayout::channels()` の戻り型は `u32` で、`AudioFormat` は `PartialEq`/`Eq`/`Hash` を実装済みのため field 比較は不要だった (Issue #184)。

## audio 入力 contract と clock 相関

### 入力ストリームの additive 拡張と API 互換

`Device` は入力側を additive に拡張し、`DeviceEnumerator::input_devices()`/`open_input_stream()`、`InputStream`、`InputDataCallback`、`InputTimestamp`、`StreamLatency`、`ClockDiscontinuity` などを追加した (Issue #187)。

public API の source 互換について、判断基準が確立されている。`OutputStream` に追加した `clock_domain()`/`output_latency()` は default method とし、trait への required method 追加で既存の全実装が壊れることを避けた。一方、pub field 構造体である `OutputCallbackInfo` への field 追加は source-breaking になるため見送り、入力側は metadata を集約した別構造体 `InputCallbackInfo` を新設した (Issue #187)。

`Device` trait への required method 追加は implementor を壊すが caller は壊さない、という非対称性がある。この変更は publish=false の未発表 crate という前提のもと、公開 enum の `#[non_exhaustive]` policy と同じ枠組みで許容すると判断し、workspace 内の全実装を同一の変更で更新した。cpal の型・エラー・feature を公開 API に露出させない方針も併せて確認された (Issue #187)。

### 独立 stream と clock_domain 相関

duplex は `DuplexStream` のような専用型で表現せず、独立した `InputStream`/`OutputStream` を `clock_domain()` の一致（かつ `UNKNOWN` 非該当）で相関させる設計を採用した。記録された不採用の根拠は、実 backend capability の実証がなく専用型の導入は早すぎることである。入出力が常に対で存在する前提を型に固定しない判断だとも読める（解釈）。単一 backend handle が入力と出力の双方を開く cpal のモデルとも一致する (Issue #187)。

same-clock の規則は「`ClockDomainId` が等値、かつ非 `UNKNOWN`」であり、`UNKNOWN == UNKNOWN` は同一 clock の主張にしない。`ClockDomainId` は media-core の型で method を追加できないため、規則は free function `same_clock_domain()` に集約し、API doc と mock テストの双方から参照させて解釈の分岐を防いだ (Issue #187)。

### timestamp と欠損値の扱い

`InputTimestamp` は `callback: MediaTime`（host 観測時刻）と `capture: Option<MediaTimestamp>`（デバイス実時刻）を分離し、capture 欠損時に callback 時刻で代用しないことを型で強制した。`capture_stamp()` は `capture.time.base() != DEVICE_CLOCK_TIMEBASE` の場合に `None` を返し、foreign-base の tick を無断で relabel することを防ぐ。`frame_position` も callback 回数から合成せず `Option<u64>` のまま保持し、backend が報告しない場合は `None` とする。断絶は `DeviceError::ClockDiscontinuity` で error callback 通知される (Issue #187)。

latency は `StreamLatency{frames, coverage, provenance}` を `Option` で報告する。coverage は HardwareOnly/HardwareAndBuffering/Unknown、provenance は Carried/Derived/Estimated を取る。裸の frame 数は buffer 含有を隠して二重計上を招くため採用しない、という根拠が記録されている (Issue #187)。

### untrusted boundary での正規化と mock PoC

cpal の `supported_*_configs` が返す `ErrorKind::UnsupportedOperation`（その方向の能力がないこと）は contract 上の `Ok(vec![])` へ正規化し、query 自体の失敗である `Err` と区別する。これは adapter の untrusted-boundary 責務として review で確認された (Issue #187)。

mock input backend の PoC は scripted fake で、callback 内で view から pre-allocated `CaptureBlock` pool へ bounded copy し、`filled` SPSC へ `try_push` する handoff を採用した。満杯時は block も panic もせず overflow 計数、pool 枯渇時は drop せず次 block を再利用し、非 RT 側の drain が内容・順序・frame_position の連続性を検査して pool へ recycle する。error callback の consumer pattern は bounded SPSC sink で、`DeviceError` は `Copy` + `&'static str` で allocation-free に保った (Issue #187)。

mock は port shape と handoff pattern の実装可能性のみを証明し、cpal 実装を証明しない。このため cpal adapter の `open_input_stream` は `Err(DeviceError::Unsupported)` を返して capture を偽装せず、`clock_domain`/`frame_position`/`input_latency` などの実値は `UNKNOWN`/`None`/`Ok(None)` を維持して値を捏造しない。`DuplexStream`・異 clock drift 補正・engine への統合は後続 Issue へ延期された (Issue #187)。

## PluginHost と PluginEndpoint の所有分離

### 非 RT 制御側と RT 処理側の型分離

plugin を pull グラフから per-block で駆動する構造は、non-RT 制御側の `PluginHost`（`!Send`/`!Sync`）と、RT 側 owner へ 1 度だけ move される `Send` な per-instance `PluginEndpoint` への所有分離で実現されている。endpoint 面は `PluginHost` コンポジットのみが持ち、standalone の `InProcessHost` は direct-call のみである。公開面は `prepare_endpoint`（non-RT・allocation あり）と `latency_frames` で、`PluginEndpoint::process` が RT leg（Send・try-only・block 経路 no alloc）を担う。engine は plugin 型を名指さず、host は engine 型を名指さない (Issue #113)。

L0（in-process）plugin は opt-in の sendable 化と instance の Plain/Shared スロット enum 化で対応し、Shared スロットは `Arc<Mutex<SendCore>>`・状態 mirror・attach 世代の atomic で構成される。全 verb は entry 事前チェック → match lock → lock 内での mirror 再読の 3 段 lock 形態を取り、attach は epoch の CAS で行う。L1（out-of-process）plugin は instance が持つ `Option<RtLeg>` の take で leg を移出し、cap 2 の arm channel（generation 単調 apply）と cap 1 の leg 返却 channel で制御する。video の plugin instance は全操作が non-RT で endpoint 経路は存在しない (Issue #113)。

`latency_frames` は L0 で `Some(0)`（direct call は pipeline latency を加えない）、L1 で `Some(lookahead_blocks × activated max_frames)`、未 Activate・無効 handle で `None` を返す (Issue #113)。

### `process_audio` の処理順序とエラー契約

`PluginEndpoint::process_audio` は (1) bound clip（channels・frames が bound 内か検査し、違反は `Err(Prepare(UnsupportedConfig))` で `out` 不変更）、(2) in-bus-first 規則の実ブロック寸法 view を scratch から切出して `out` の data を channel-major で複写、(3) `inner.process` の順序に固定される。uniform copy-in（stage-in）は仕様で確定した v1 規則で、plugin が同一バッファで in-place 処理する最適化は adapter 契約の拡張を要するため延期された。`Err(Ipc)` は L1 内部で事前 gate により `Err(Prepare(UnsupportedConfig))` へ写像され、公開面に出さない (Issue #113)。

エラー時に `out` を不変更とする契約を前提に、transform 位置では上流が bypass し、source 位置では consuming 側が事前に `out` を無音で埋める必須慣行が定められている。stale data ではなく無音を出力にするための措置である (Issue #113)。

### RT 経路で destructor を走らせないパターン

Issue #113 の実装では、RT 経路で destructor・dealloc・blocking を起こさないためのパターンが複数確立されている。

- `PlanSwap::swap` は冒頭で退役物を drain する（drain-at-entry）。publish 前に回収するため、destructor が slot に触れても publish は未開始である。
- retire channel 満杯時は `mem::forget` の leak-on-full とし、`retired_overflows`/`arm_retire_overflows` で観測する。`Retirer`・`PlanVault` も同じ方針である。
- `SnapshotGuard`/`RtPlan::active` の返却は `Option<&GraphPlan>` の借用で `Arc` clone を返さない。返却した `Arc` が RT 側で retain され、最後の refcount drop の destructor が callback thread 上で走る経路を静的に防ぐためである。
- `drain_endpoint_channels` は wait-free の半分（resend・restore のみ・destructor ゼロ）と non-RT の半分（collector drain・re-arm・field clear）に分割し、RT `process` 経路で ring の free や `Arc` の destructor を走らせない。
- `Session` の field 宣言順（stream → swap）が「collector が retirer より後に死ぬ」drop order を保証する。宣言順が drop order を決めるという依存関係は doc に明記されている。

Send 証拠は `unsafe impl` なしの構造型とし、`static_assertions` の `assert_impl_all!`/`assert_not_impl_any!`（負側 `!Send` の固定）と型関数で担保する。`expect_not_send` 型の関数では `!Send` を compile 時に表明できないため、負側には `assert_not_impl_any!` が必要である。enum variant は型ではないため、対象は具象の L0/L1 endpoint 型とする (Issue #113)。

### 検証手法と計数上の注意

- dev-dep 隠蔽の監査は両面で行う。`cargo tree` に `--no-dev` flag は存在しないため、`cargo tree -p ruviform-audio` で plugin 系 crate が出現しないことと、`cargo tree -p ruviform-plugin-host --edges normal` で normal edges に `ruviform-audio` が出現しないことの両方を確認した (Issue #113)。同じ方式で、`ruviform-audio-device` の normal deps に `ruviform-rt` が含まれないこと（dev-dep のみ）を確認している (Issue #187)。
- 未配線 API を `#[expect(dead_code)]` で留める場合、unit test がそれを exercise する間は bare `#[expect]` が unfulfilled expectation で lint gate に失敗する。`#[cfg_attr(not(test), expect(dead_code))]` で非 test build に限って expect する形にすると、配線後に lint が自動的に再燃する自己消去型の tripwire になる (Issue #113)。
- 設計上の記述修正が複数箇所に分散する場合は、修正後に `grep` で旧記述の残余が 0 件であることを機械確認する。遅延式の修正では fix site 2 箇所を同時に直し、`× block_frames` の残余 0 件を grep で再確認した (Issue #113)。
- mock の `DeviceEnumerator`/`Device`/`OutputStream` が callback を capture してテストスレッドから手動駆動する方式は、cpal・実デバイスに依存せずに実 `Session::open` 経路を通せる。L1 e2e は crate 内の `CARGO_BIN_EXE_*` で子プロセスを spawn する (Issue #113)。
- L1 e2e の pacing・assert は既存慣行を継承し、5ms/block の real-time-scale pace・`matched >= 16` の閾値（CI 高負荷の headroom）・silence または gained input のみ許可する defined-sample 判定で構成する。この閾値と pace は CI 高負荷下で flake し得る既知リスクとして記録されている (Issue #113)。
- main への merge 後は `cargo generate-lockfile` を経て全検証を再実行する順序が、複数 Issue で共通の workflow として記録されている (Issue #113)。

## typed ランタイムと Compiler

### 3 層構成と RT 契約

typed module は、ruviform-graph の ValidatedGraph を非 RT の `Compiler` で immutable な `PreparedPlan` にコンパイルし、`RtRuntime` が snapshot slot 経由の last-good plan を実行する 3 層構成である。`GraphRuntime`（非 RT）が compile・generation 付き install/remove command・publish・retire 収集を所有する。RT callback が触れる arena・delay ring・processor registry は全て construction 時に確保済みで、warm-up 後は no-alloc・no-block・no-OS-I/O・no-destructor を達成している。旧来の AudioNode/Graph ベースの経路は無変更で、golden fixture 下に共存させた (Issue #195)。

### stateful ownership の採否プロセス

processor の stateful ownership は、同一 MockDSP で Plan-owned と Stable Slot の 2 案を比較する PoC を integration test として実測し、ADR に採否を記録して確定した。両案が不適格な場合は測定データ付きで Blocked 報告とし、妥協採用を禁止する規律が明文化されている (Issue #195)。

採用したのは Stable Runtime Slot 所有で、processor instance が registry の stable slot に残り plan generation を跨いで state が継続する。実測では reuse 時に state が 1→3 へ進み install は 1 回のみだった。Plan-owned state は plan 更新ごとに instance を再構築して state が reset されることを実測で確認し、却下した（比較用の `PrepareOutcome::New` は残置）(Issue #195)。

### Compiler の pass 分割とエラー設計

`Compiler` の pass は「processor 割当（NodeFactory → Reuse/Boundary/New・stable pslot・removal 検出・allocator 解放）→ port→slot binding と deterministic schedule → accumulated latency と merge 点の遅延補償（PDC）delay-line（`max(inputs accum) − accum[edge]`）→ aliasing audit → boundary input/output table」の順に分割される。pass ごとに責務が一意に決まるため、拒否理由を `CompileError` の variant（`UnsupportedEdgeDelay`/`MissingProcessor`/`BufferAlias`/`SlotOverflow`/`DelayLineOverflow`/`DelayOverflow`/`ProcessorOverflow`/`Factory`/`Inconsistent`）で個別に識別できる。u32→u16 の channel 変換は checked で行う (Issue #195)。

buffer 側の `AlignedSamples` は over-allocate+offset で power-of-two alignment（≥64B）を非 RT 確保・`unsafe` なしで実現し、`DelayLine` は per-channel write cursor の ring で `process_add` が bounded・no-alloc である (Issue #195)。

### RtRuntime の単一 owner 保証と fault 計数

`RtRuntime` は `Send`/`!Sync` の単一 owner であり、`SnapshotGuard` の `reader` borrow と callback が mutate する registry/seq を `RefCell<RtState>` で型レベルに分離した。RT 側は SPSC + atomics + `RefCell` のみを使い、`Mutex` は test/mock に限定し、`RtContext` の capability 契約を全 RT API で保持する。borrowed IO の同時 borrow を避けるため、`NodeIo` は per-access materialize 型に限定した (Issue #195)。

producer slot の `stamp != seq` を consumer materialize で検出した uninitialized-read は、zero-fill degrade + fault 計数として扱い、stale data を返さない。generation 不一致の command/event も遮断して計数し、`SharedStatus` が `stale_events`/`stale_commands`/`uninit_reads`/`retire_overflows` などを atomic に公開する。`Drop` で全 deactivate と retire を行う (Issue #195)。

### plan-retire の一本化と leak-on-full の解釈

plan の退役処理は `SnapshotWriter::collect` に一本化した。`SnapshotGuard::drop` が自身の `Retirer` 経由で `Arc` を退避するため、runtime 側に独自の plan-retire channel を作ると二重経路になる。`plan_retire_overflows` は reader 側の guard-drop 飽和を観測する指標であり、publish で displaced された `Arc` は writer の local pending queue に行くためこの channel を満たさないことが契約で確認済みである (Issue #195)。

`retire_overflows`/`plan_retire_overflows` は leak-on-full の累積 signal であり、飽和が継続すると leaked memory の増加を意味する。capacity と `collect` cadence の tuning は driver 側の責務とされた (Issue #195)。

### 受入項目の分類

受入項目（EX/TA）は verified・明示 unverified・N/A に分類した。fixture が未整備の項目（fanout・sidechain/aux）は unverified と明示し、後続 issue で補完する方針を記録している。test-spec の matrix 節を正本として更新した (Issue #195)。

## plugin bridge と bus 列挙

### bridge crate の配置と scheduler の owner

plugin instance を typed graph へ接着する新規 crate `ruviform-audio-plugin-bridge` は利用側層に置いた。plugin-host 内に置く案は plugin-host→audio の依存を dev-dep に留める制約に反し、ruviform-audio 内は engine が plugin に依存する逆向き、plugin-core 内は foundations が graph/audio に依存する逆向きとなるため、いずれも却下された (Issue #196)。

`TransportScheduler` は `PluginNode`（RT 側）に内蔵し、非 RT の `PluginNodeControl` が SPSC control channel で command/plan/clock/restart を stage する。非 RT の制御側に scheduler を置く案は、callback の EngineFrame が RT 側の所有となるため制御側の先読み配送が必要になり、exact apply とレイテンシを壊すとして却下された (Issue #196)。

`Arc<TransportPlan>` の RT drop は `PlanVault`（node 内 keep-alive + retire channel + leak-on-full）で防ぐ。media-core に retire hook を追加する案は、層を跨ぐ API 変更で局所的でないため見送られた (Issue #196)。

### bus role の型付けと capability matrix

`BusDescriptor.main: bool` を `role: BusRole`（`#[non_exhaustive]` Main/Aux/Sidechain/Send/Return）に置き換えた。公開 API の破壊的変更だが、実利用が doc 例のみで blast radius が小さいため受容し、`is_main()` helper で旧意味は保持した。`BusRole` を plugin-core に置くのは、VST3/CLAP の bus 意味論を plugin 側が知っている方が正しく、bridge への推測移譲は検証不能を増やすためである (Issue #196)。

capability は `AdapterCapability`/`capability_for(Backend)` が、backend（L0/L1/CLAP/VST3）× 機能軸（bus enumeration・multi-bus・aux/sidechain・events・in-block transport・min_frames・state・crash isolation）を Verified/Declared/Unknown/Unavailable/Unsupported の 5 水準で宣言する。`Unknown`/`Declared`/`Unavailable` は採用済みの `Verified` と見なさない。実測では L0 が `Verified`、L1 の bus enumeration が `Unavailable`（プロセス間通信の verb が存在しないため推測で埋めない）である。CLAP の in-block transport は実 plugin の証拠がなく `Declared` 止まりである (Issue #196)。

### PluginNode の bus 処理と event 座標

`PluginNode` は `GraphProcessor` として `NodeIo` の input/output を `[offset, offset+frames)` に windowed する stack view を構築する（no alloc・no unsafe）。未接続の enabled input は silence scratch、disabled bus は empty view で bus の順位を保持し、silent slab を render せず implicit downmix も行わない。`MAX_BUSES=MAX_CHANNELS=16` は wire の bound と一致する (Issue #196)。

event の `frame_offset` は process call 開始相対で、callback→span→process の座標変換は専用関数のみが行い、`build_call_event_list` が process 開始相対 offset の正本である。adapter に独自の変換を持たせない設計である。same-frame の causal order は `source_sequence` の安定順を保持し、generation 不一致 event は遮断、overflow は fault+event drop とする。`NodeEvent`（u32+f32）は transport plan/command を表現できないため使わない (Issue #196)。

`TransportScheduler` には bounded な `pop_receipt`/`pop_fault`（capacity bounded・RT-safe）を追加し、RT-adjacent な report draining を bounded に保った。`Plugin::buses()`/`ErasedPlugin::buses()`（default `&[]` = 未宣言）の列挙は非 RT 経路である (Issue #196)。

### legacy 経路の保持

bridge は additive で、旧来の単一 bus `PluginEndpoint::process_audio` 経路は rollback 経路として無変更で保持する。削除しないことは仕様の Out of Scope にも明記されている (Issue #196)。

## オフラインレンダドライバ

### 配置と capability gate

render driver は `ruviform-exec` が layer baseline の制約で media 型を見られないため、`ruviform-audio::render`（engine 層）に配置した。`PreparedRender::compile` が render ごとに fresh な `GraphRuntime`/`RtRuntime`+`PreparedPlan` を生成し、processor instance は live runtime と独立で、plan・capabilities・geometry・sample rate・range は preparation で freeze される (Issue #198)。

exact rendering は全 installed processor の `OfflineRenderSupport` が `Verified` であることを要求し、`Unknown`/`Declared` は preflight で `PreflightError::OfflineUnsupported` として拒否する。近似で代替しない設計である。`DeliveryPolicy::Exact` が必須で degraded/approximate は reject され、`ruviform-exec` の既存 chunk `OfflineDriver` の保証範囲は広げていない (Issue #198)。

### preroll・PDC・tail

emit math は `position − total_latency + sample_offset` とし、warm-up frame は処理するが emit しない。emission window は `[range_start, range_end + tail)` である (Issue #198)。

`TailSpec::{None,Known,Estimated}` により、undefined tail を呼び出し側が明示的に選択する。receipt の `TailEvidence` が使用した variant を報告するため silent truncation は起きない。ただし `Estimated` は plugin の decay curve をモデル化しないという限界がある (Issue #198)。

### staged output と失敗安全

output lifecycle は feeder → `RtRuntime::process` → boundary-slot capture → `Encoder` → `Muxer` → 独立 reopen `verify(temp, facts)` → `publish` の順である。`StagedOutput` の temp sibling を使い、全失敗経路で destination を untouched に保つ。muxer には owned `try_clone` した `File`（shared file description）を渡すため、publish の `sync_all`/`rename` が finalized bytes を見る。`CancellationToken` は全 physical call と drain/finalize/verify/publish の境界で check し、`Cancelled{phase}` を typed error として返す (Issue #198)。

### 所有 layer でのバグ修正

render tests が根本原因を発見したバグとして、`RtRuntime::input_writer` が slot の logical `channels` を pending plan から設定していなかった問題がある。従来は `channels == 0` のまま `activate_generation` が最初の `process` 内で走るまで待つため、最初の callback での channels-gated feed が silently drop されていた。driver 側で workaround せず、所有 layer である runtime 側で修正した (Issue #198)。

## 横断的な契約と不変条件

- 時刻や位置の欠損を代替値で埋めない。`InputTimestamp` の capture/callback 分離と `frame_position` の `Option` 保持 (Issue #187)、uninitialized-read の zero-fill degrade + stale data 不返却 (Issue #195)、`TailSpec` の明示選択と `TailEvidence` による報告 (Issue #198) が同じ方針の現れである。
- capability は実装が保証できる範囲だけを報告する。`BusReport::Unavailable` (Issue #196)、`OfflineRenderSupport::Verified` の要求 (Issue #198)、`open_input_stream` の `Err(Unsupported)` と未報告値の `None`/`UNKNOWN` 維持 (Issue #187) が該当する。
- RT 経路は bounded かつ allocation-free・destructor-free に保つ。SPSC の bounded queue と leak-on-full 観測 (Issue #21, Issue #187, Issue #195)、借用返却による `Arc` 非返却と drain の wait-free/non-RT 分割 (Issue #113)、`Mutex` の test/mock 限定 (Issue #195)、`DeviceError` の `Copy` + `&'static str` (Issue #187)、bounded な `pop_receipt`/`pop_fault` (Issue #196) が該当する。
- 拒否は typed error で原因を個別に識別する。`CompileError` の variant 分割 (Issue #195)、`Err(Ipc)` の `Prepare(UnsupportedConfig)` への写像 (Issue #113)、`PreflightError::OfflineUnsupported`・`Cancelled{phase}` (Issue #198)、`OfflineError::WorkerPanicked` (Issue #24) が該当する。
- 修正はバグの所有 layer で行い、呼び出し側で workaround しない。plan-retire の `SnapshotWriter::collect` への一本化 (Issue #195) と `input_writer` の channels 設定修正 (Issue #198) が該当する。
- 公開 API の破壊は blast radius を評価して判断する。未発表 crate での trait method 追加 (Issue #187) と `BusDescriptor.main` → `role` 置換 (Issue #196) は影響範囲が小さいとして受容され、pub field 構造体への field 追加は source-breaking として回避された (Issue #187)。
- RT 契約テストの選択は命名規約（`rt_` 部分文字列フィルタ）に依存するため、非 RT 側の命名にも制約が及ぶ (Issue #21, Issue #113)。

## 未確定・保留事項

- Issue #184/#187/#195/#196/#198 の plan は検証チェックボックスが未チェック（approved/in-progress）のままで、実行結果はこれらの計画文書内に記録されていない。
- Issue #24/#25/#27 の実装記録・計測値レベルの記録は確認できていない。Issue #27 は検証が全て未チェックの計画段階の記録であり、実装の有無も未確認である。
- Issue #187 の実機検証（cpal `build_input_stream` plumbing・実 HW latency 計測・実同一 clock duplex 駆動・`map_stamp_to_frame` の engine frame 写像）は「実機未検証」と明記されたまま後続へ延期されており、その後の検証状況は不明。`test-rt` mise タスクが wrapper exit code 1 を返す状況も「別途確認要」として残っており、解消状況は不明である。
- Issue #113 の macOS/Linux 実証と CI の 3 OS matrix ジョブはローカル Windows のみの実証で未実行と記録されている。
- Issue #196 の CI-profile lanes（verify-ci・test-l1-real-plugin・lint-l1-real-plugin）は dev lanes のみ green で CI 未実行。CLAP/VST3 の実規格 lane は回帰のみで fixture は単一 stereo のまま、multi-bus は L0 mock plugin での検証に留まる。bridge 側は VaultExceeded/EndpointError の report を追加したが、L1 crash matrix 全体の再検証はしていない。
- Issue #198 の非 Windows プラットフォーム検証は未実施である。
- 各 Issue の ADR・design・spec・test-spec の本文は直接確認していない。Issue #195 の ownership 案の比較基準や Issue #198 の `TailSpec`/`OfflineRenderSupport` の契約根拠など、一次根拠の詳細は実装記録の要約経由である。
- Issue #184 の serde/永続化・delayed-cycle 実行はスコープ外と明示され、CI は workflow_dispatch 手動のみで未トリガと記録されている。
- deferred 項目として、`TailSpec::Estimated` は plugin の decay curve をモデル化しない (Issue #198)、`process_audio` の in-place 最適化は adapter 契約の拡張待ち (Issue #113)、`EndpointBounds` への `min_frames` 追加は別 Issue 扱い (Issue #196)、`DuplexStream`・異 clock drift 補正・engine 統合は未着手 (Issue #187)。
