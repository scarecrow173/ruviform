---
id: "EVI-kQO7FqexidthKuYpX3jNe4"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "typed graph・オフラインレンダの検証記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/21", "https://github.com/scarecrow173/ruviform/issues/24", "https://github.com/scarecrow173/ruviform/issues/25", "https://github.com/scarecrow173/ruviform/issues/113", "https://github.com/scarecrow173/ruviform/issues/184", "https://github.com/scarecrow173/ruviform/issues/187", "https://github.com/scarecrow173/ruviform/issues/195", "https://github.com/scarecrow173/ruviform/issues/196", "https://github.com/scarecrow173/ruviform/issues/198"]
  informs: []
  explains: []
  supersedes: []
---

# typed graph・オフラインレンダの検証記録

本書は、型付きグラフ（typed graph）、そのコンパイラ、およびオフライン描画系に関する実装の検証結果を集約した記録である。各節では Issue ごとに、何を検証し、どのような結果を観測し、どの判断に至ったかを記す。未実施または後続工程へ委譲された検証は、最終節にまとめる。

## exec スレッド優先度機構の検証 (Issue #21)

`ruviform-exec` に OS のリアルタイムスレッド優先度付与機構を non-RT 専用 API として実装し、API・テスト・依存面を検証した (Issue #21)。crate root から `promote_current_thread_to_realtime`、`demote_current_thread_from_realtime`、`ThreadPriorityParams`、`ThreadPriorityGuard`、`PriorityError` の 5 API を再公開する構成で、依存する `audio_thread_priority` 0.38 は `default-features = false` の private 依存として、with_dbus 経路とネイティブ libdbus への依存を排除した。

`cargo test -p ruviform-exec` で unit test 3 件と doctest 7 件が合格した (Issue #21)。`cargo test --workspace rt_` で exec クレートのテストが 0 件選択されることを確認した。これは RT レーンの名前フィルタ `rt_` に誤選択されないよう、テスト・モジュール・doctest・ヘルパー名に `rt_` の部分文字列を含めない命名規約を採った結果であり、`assert_*` と `start_*` は `rt_` を内包するため禁止し `expect_*`・`spawn_*` を使う規約とした。clippy は警告 0 件、`cargo tree` で dbus・simple_logger・nix が依存木に含まれないことを確認した。実行環境のコミット上限枯渇（os error 1455、並走する他エージェント群が原因）による一度の失敗があったが、`-j1` および既定並列での再実行で全件合格しており、コード由来ではないと記録された (Issue #21)。

## オフライン実行ドライバの検証 (Issue #24)

オフライン実行ドライバ（Issue #24）では、計画に定めた検証項目がすべて完了した。`run_parallel` が Setup: Clone+Send、Rt: Send、Io: Send の境界で並列駆動し、seq による再構成で出力順序が入力順序と一致する設計である。workers ∈ {1,2,4,8} で出力が逐次実行と一致するテスト、キャンセルの即時停止と `Cancelled` 返却、`Progress` の正確性、prepare 失敗の集約、ワーカーパニックのエラー写像の各テストが green だった (Issue #24)。

クローズ時には spec の受入条件と計画の検証項目を実測で確定し、`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke` と `prek run --all-files`、doc 監査がすべてパスした。当該チェーン起因の findings は 0 件で、`unsafe` コードがないことを確認した (Issue #24)。

## オーディオエンジン基盤の検証 (Issue #25)

`ruviform-audio` エンジンの新設（Issue #25）では、graph・mixer・plan の各モジュールと session のコールバック配線を実装し、計画の検証項目をすべて完了した。pull グラフがモックノード列を正しい順序で駆動すること、ミキサーが planar 入力を和算して出力へ書き込むことをテストで確認した。`rt_callback_no_alloc` は assert_no_alloc の監査窓内で allocation が 0 件、`plan_swap_under_run` はプラン差替後に新プランが使われ panic しないことを確認した (Issue #25)。

session 側では `Session::open_output` とコールバック配線を実装し、`RtContext::mint` の呼出し、`PlanHandle` の参照、RT 契約テストを完了させた。検証は `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` のパスと doc 監査 findings 0 件で確認した (Issue #25)。

## プラグイン endpoint 統合の検証 (Issue #113)

Issue #113 は plugin-core と plugin-host にまたがる endpoint 統合で、T1〜T7 のタスクとして段階的に実装・検証された。

最初の 2 タスクで wire 面と L0 endpoint の前提を実装した (Issue #113)。T1 では `HostError::EndpointUnavailable` を wire に載せない local-only variant として追加し、`encode`/`encode_host_error` が `Err(WireError::UnsupportedVariant)` で拒否して Ipc/Unsupported へ潰さない形とした。`encode`/`encode_host_error` を `Result<(), WireError>` 返却へ変更し、`encode_body` を抽出することで、全 Err 経路が encode の単一点で `out.truncate(start)` する all-or-nothing append とした。`PluginFactory::create_sendable`（default None の opt-in）と `erase_sendable`（`Erased<P>: Send` は `P: Send` の場合に限り成立し、Plugin/ErasedPlugin への全域 Send bound は置かない）、`AudioBufferView`/`AudioBufferViewMut` への `BufferView`/`BufferViewMut` 実装を加え、call site 4 件・doctest 3 件・inline helper を対応した。`cargo test -p ruviform-plugin-core` は 103 pass（lib 67 + video_contract 8 + doctest 26 + compile-fail 2）で、l1_proxy_e2e の spawn handshake flake を一度観測したが、本タスク非起因として再実行で pass と記録した。

T2 では `Instance` の `Plain`/`Shared` スロット化と共有セル（`Arc<Mutex<SendCore>>` + `AtomicU8` mirror + `AtomicU64` attach epoch）、`shared_verb` の 3 段 lock 形態（entry pre-check → lock/poison 収束 → in-CS mirror 再読）、`PluginEndpoint`/`EndpointBounds`/`EndpointInner`/`L0Endpoint` と epoch CAS attach・Drop 返却を実装した (Issue #113)。L0 inner process は mirror fast-skip → try_lock → in-lock 再読 → activated gate → catch_unwind の順で処理し、panic/Failed 時は fetch_max + 無音 + `Ok(CONTINUE)`、busy 時は `Err(EndpointUnavailable)` を返す。単体テスト 19 件を追加し、独立レビューは A=APPROVE・adversarial 側の全攻撃面をクリアした。`cargo test -p ruviform-plugin-host` は lib 30 + l1_proxy_e2e 42 + doctest 1 が全 pass で、test-rt で `rt_endpoint_process_no_alloc`/`rt_endpoint_busy_bypass_no_alloc` を含む実一致を確認した。

T3 では L1 endpoint の arm と drain の機構を実装した (Issue #113)。`L1Instance.rt` の `Option<RtLeg>` 化、`prepare_endpoint` の pinned 拒否順序（state → rt presence → bounds-before-take → take → endpoint_channel_setup）、`EndpointInner::L1(Box<L1Endpoint>)`、`RtLeg::arm_apply`（ring 束縛のみ reset・seq 空間の簿記は持続・displaced ring は Retirer へ）、`drain_endpoint_channels` の 2 層構成（wait-free 半分は resend と restore のみ、non-RT 半分は collector drain・re-arm・field clear）、Drop → 容量 1 の return channel を実装した。独立レビューの確定修正 F1〜F7 では、RT 側で destructor を走らせない wait-free drain 分割、restored leg の non-RT re-arm、`Mutex<Producer<u64>>` の共有 result channel（doorbell-less な drop seq に driver が synthesized mark_failed completion を push し、strict-alternation の wedge を解消）、over-bound arm の pass-through 収束、`channels == bound.channels` gate と min_frames の child 委譲、Destroyed restore gate、`arm_retire_overflows` 観測点を適用した。単体テスト 25 件 + rt_ 4 件を追加し、全 gate green とした。

T4 では public `PluginHost::prepare_endpoint`（handle → state → presence → uniform bounds の pinned 拒否 prelude と slot dispatch）と `PluginHost::latency_frames`（L0 は `Some(0)` の lifecycle gate、L1 は mirror gate、それ以外は `None`）を公開し、統一 defined-rejection 表を API doc 化して crate doc を完成させた (Issue #113)。stale だった遅延式を `lookahead_blocks × block_frames / sample_rate` から `lookahead_blocks × activated max_frames` に修正し、T2/T3 に残していた `expect(dead_code)` の橋 11 件を全除去した。レビュー確定修正 F1〜F5（latency_frames の mirror gate と `armed_latency_frames` 簿記 read の分離、`is_poisoned` union half、doc completeness、L1 bounds matrix 行、bounds-split の記録）を適用し、新規テスト 10 件 + doctest 1 件 + e2e assertion 2 箇所を追加した。`cargo test -p ruviform-plugin-host` は lib 65 + l1_proxy_e2e 55 + doctest 2、`cargo doc` は警告 0 件だった。

T5 では `FnMut(&mut AudioBufferViewMut, &RtContext) + Send` に対する `AudioNode` の blanket impl と、`RtPlan.retirer: Retirer<Arc<GraphPlan>>`・`PlanSwap` の `Arc<Mutex<DropCollector>>` による退役収集（non-RT entry ごとの drain、`drain_retired`/`retired_overflows`/`with_retire_capacity`）を実装した (Issue #113)。レビューで `active()` の返却型を `Option<Arc<GraphPlan>>` から `Option<&GraphPlan>`（cached slot の借用）へ修正した。返却した `Arc` が RT 側で retain されると、最後の refcount drop で `GraphPlan` の destructor が callback thread 上で走り得るため、借用返却でこの経路を静的に閉じた。新規テスト 8 件（rt_ 4 + unit 4）と static_assertions による Send 証拠を追加し、test-rt で ruviform-audio は新規 `rt_plan_*` 3 件・`rt_fn_node_*` 1 件・既存 2 件の計 6 件の実一致だった。

T6 では `PluginEndpoint::process_audio` を実装した (Issue #113)。順序固定の 3 段 shell（bound clip → per-block view + channel-major stage-in → inner.process で plugin が out へ直接書込み、copy-back なし）とし、新規 E2E suite 13 テストが mock の DeviceEnumerator/Device/OutputStream で実際の `Session::open` コールバックを捕捉して手動駆動する構成とした。E2E は L0/L1 の graph 実行（register_factory/register_out_of_process と `execute(HostCommand::Instantiate{plugin_id})` の実 API 経路から graph.push まで）、lifecycle、kill_child による Failed + 通知 once + 無音、latency（L1 = 2 × 512 = 1024）、退役 DropFlag、leg 還流、source-position の fill(0.0) をカバーする。`cargo test -p ruviform-plugin-host` は 142 passed（lib 72 + l1_proxy_e2e 55 + E2E 13 + doctest 2）、test-rt 実一致 129 件（plugin-host 内訳は lib 12 + l1_proxy_e2e 4 + E2E 1）だった。`cargo tree` の両面監査で、ruviform-audio の依存木に plugin が現れず、plugin-host の normal edges に ruviform-audio が現れないこと（dev-dep のみ）を確認した。deviation として 3 件を記録した。v1 surface に `set_param` が存在しないため verb 行を SaveState/RestoreState/StopProcessing/StartProcessing で代替した点、DropFlag probe が drain 時点では立たないため「drain 時点 !dropped・clean teardown 後 dropped」の 2 点 assert とした点、L1 e2e の pacing/matched>=16 を l1_proxy_e2e の慣行から継承した点である。

最終タスク T7 では Full Gate を 1 回実施し、fmt-check・check・lint・test・test-plugin-core・lint-plugin-core・test-rt・bench-smoke・build・prek・doc 監査・cargo tree 2 件・`cargo test -p ruviform-plugin-host`・cargo doc の 15 コマンドがすべて exit 0 だった (Issue #113)。`mise run test` は 56 test binaries・1125 passed・0 failed、`test-rt` は `rt_` name filter で実一致 129 tests・0 failed で、件数により 0 件スキップでないことを確認した。`cargo doc` は `RUSTDOCFLAGS="-D warnings"` 下で警告 0 件、`cargo tree` の両面監査で依存分離を再実証し、全ログを保存した。既知 flake 2 件（`hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded`）は初回から pass で未発生だった。実機実証は Windows のみで、macOS/Linux は CI の verify ジョブに委譲した。

## graph IR と Validator の検証 (Issue #184)

`crates/ruviform-graph` の新設（Issue #184）では、IR・Validator・決定的 topo を実装し、全検証が exit 0 だった。実装内容は、u64=1 起点の決定的 ID 採番と private `GraphSpec` を持つ `GraphBuilder`、domain の唯一の正として `PortFormat::domain()` を据える直交 port 属性、referential → direction → domain → format → layout/ChannelMap → capacity → required → 全グラフ SCC cycle 判定の階層でエラーを全件収集する `Validator`（`ValidationErrors`）、Kahn + BTreeMap tie-break による決定的 topo の双方向 `StableHandleMap`、requested/published revision を区別して失敗時に published を維持する `GraphProject` である。

検証では PoC の ex_001〜ex_006 全 6 件が pass した (Issue #184)。`cargo tree --locked` で通常依存が ruviform-audio-core のみ・循環 0 件、rg による検索で `ruviform_rt`・`RtContext`・`unsafe` が 0 match であることを確認した。merge commit 818e88e の取込み後に lockfile を再生成して全 gate を再実行し、全 exit 0 を確認した。

## オーディオ入力ポートの検証 (Issue #187)

Issue #187（T1 契約 → T2 cpal adapter → T3 mock PoC）では、mise の開発 lane がすべて pass した。`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` を実行し、`test` は 114 test binaries 全件 ok で、うち `mock_input_backend` の 12 テスト（deterministic capture・data integrity・delivered-info 一致・timestamp/frame position・clock-domain 等値規則・latency・error injection・bounded handoff・drop/in-flight 観測）が green だった (Issue #187)。`test-rt` は 80 行の test-result が全件 ok で、`rt_input_callback_handoff_allocates_nothing` と `rt_input_callback_queue_pressure_stays_bounded` を含んだ。`cargo tree --edges normal` で `ruviform-rt` が dev-dependency にのみ現れることを確認し、入力側の実装が RT クレートへ通常依存を持ち込んでいないことを実証した。レビュー verdict は ACCEPT WITH NON-BLOCKING NOTES で、P1/P2 指摘は全件修正済みだった (Issue #187)。

実機経路はこの Issue の範囲外とし、`open_input_stream` は `Err(DeviceError::Unsupported)` を返す実装に留め、未実装の経路を成功したかのように見せない判断を確認した (Issue #187)。

## typed コンパイラの G2 受入ゲート (Issue #195)

Issue #195 では、typed graph オーディオランタイムの受入ゲート G2-01〜G2-08 がすべて PASS した。Rust 1.99.0・origin/main の merge 後の最終検証で `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が全 PASS で、`cargo test -p ruviform-audio` の typed_compiler 11 件・typed_runtime 6 件・typed_activation 12 件・legacy_golden 5 件がすべて exit 0 だった (Issue #195)。

各ゲートの観測内容は次のとおりである (Issue #195)。G2-01 deterministic compile では、同一入力から byte 同一の plan が生成された。G2-02 invalid 拒否では、`CompileError` を個別に識別した。G2-03 PDC では、latency 3 と 0 の merge で fast path edge に delay=3 が付与され、frame0/2=2.0・frame3=13.0・total_latency=3 を観測した（graph-internal の PDC のみで、ADC/DAC は範囲外）。

残るゲートも個別に確認された (Issue #195)。G2-04 は ownership、G2-05 は fault 系（publish 失敗時の last-good 維持・retire 飽和の計数・stale 遮断）を確認した。G2-06 では frames 1/13/96/200/255 が受理され、max=256 を超える 257 は `FramesExceedMax` で拒否、0 frame は no-op だった。G2-07 は legacy 共存、G2-08 は EX/TA traceability を test 名で再現可能に記録した。

`assert_no_alloc` の監査では、warm-up 後の `rt.process` 区間で allocation が 0 件だった (Issue #195)。`SnapshotReader::read` 初回呼出しの debt node alloc は暖機として、契約どおり監査窓の外にあることを確認した。

## plugin bridge アダプタの E2E 検証 (Issue #196)

Issue #196（T1 plugin-core の bus role・列挙 → T2 media-core pop・host bus 列挙 → T3 bridge crate → T4 検証）では、`cargo test -p ruviform-audio-plugin-bridge` の 5 E2E テストが green だった (Issue #196)。`multi_bus_audio_routes_and_processes` は Main stereo + Sidechain mono の入力から Main stereo + Aux stereo の出力を生成し、per-channel の gain を検証した（M4-02）。`loop_100_min_128_zero_violating_calls` は 256F callback・Loop=100・min=128 の条件で [128,256] 外の call が 0 件で 100F の call がなく、`UndeliverableSpan` が audit されたことを確認した（M4-03）。`loop_200_partial_delivery_no_violations` は 200F の lap を legal な call 1 回で処理し、56F の remainder を audit した。`event_offsets_are_process_start_relative` は max=128 で 256 frame を 2 call に分割し、position 200 の event が call 2 の offset 72 に観測された（engine-relative ではなく process-start-relative、M4-04）。`capability_matrix_is_honest` は L0 が `Verified`、L1 の `bus_enumeration` が `Unavailable` であることを検証した（M4-01）。

開発 lane（`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`）は全緑だったが、CI-profile lane は未実行だった (Issue #196)。

## オフラインレンダの検証 (Issue #198)

Issue #198 では、`cargo test -p ruviform-audio --test render` の 7 テストが green だった (Issue #198)。`of01_block_size_invariance` は OnePole の stateful render を continuous 768 frame と 64/128/256 block 分割で実行し、emitted PCM が全 variant で continuous reference と一致することを確認した。`of02_midstream_preroll_pdc_tail` は range [200,600) で warm-up が処理されるが emit されないこと、emitted PCM が reference と一致すること、latency 96 で `position−96` の emit 整列、`TailSpec::Known(64)` で 576 frames・tail_evidence=Known を確認した。

残るテストも個別の項目を検証した (Issue #198)。of03 は events/loop/transport の順序、of04 は live 独立性と capability gate を検証した。`of05_wav_reopen_verify` は実 WavEncoder/WavMuxer から StagedOutput 経由で書き出した出力を ruviform-io-symphonia の demuxer・decoder で独立に reopen し、PCM が analytic one-pole 予測と一致することを確認した。`of06` の failure injection では Cancelled/Output/Finalize/Publish/Verify/DestinationExists がすべて型付きエラーで返り、destination が untouched で `.rvf-tmp-*` の残滓がゼロだった。

この render テストによって `RtRuntime::input_writer` の根本原因バグを発見した (Issue #198)。channels==0 のまま writer が hand out されており、最初の process 内の activate_generation まで channels-gated feed が通知なく捨てられ続けていた。修正は所有層であるランタイムで、pending plan から logical channels を configure する形で実施した。

workspace 規模の検証（OF-07、Windows 11 x64 で実施）では、`cargo test --workspace --locked` が 109 test targets・0 failures、`cargo test -p ruviform-exec --locked` で offline driver suite に回帰がないこと、`mise run fmt-check`/`check`/`lint`/`test`（106 targets）/`test-rt`（73 targets）/`verify-ci`（CI profile）がすべて green だったことを確認した (Issue #198)。consumer-impact は synvelis・audio-lens とも no impact と記録した。additive module であり、defaulted trait method により既存 impl は `Unknown` のまま維持されるためである。

## 記録の範囲と限界

以下の検証は、記録時点で未実行、範囲外、または後続工程への委譲として記録された。

- Issue #113: ローカル実証は Windows のみで、macOS/Linux を含む 3 OS matrix の CI verify ジョブは未実行。CI 実行時に結果を記録する運用とした。
- Issue #187: 実機 cpal `build_input_stream`、HW latency 実測、実機 same-clock duplex は範囲外で未検証。`test-rt` の mise タスクが wrapper exit code 1 を返す状況が別途確認要のリスクとして残っており、その後の解消状況は不明。
- Issue #195: EX-002 fanout と EX-004 sidechain/aux は fixture 未整備のため明示的に未検証。EX-013/014/015/017/018/019 と TA-001/002/003/005/009/010/011 は M2 の範囲外として N/A とした。
- Issue #196: CI-profile lane（`verify-ci`・`test-l1-real-plugin`・`lint-l1-real-plugin`）はその作業単位では未実行。
- Issue #198: Windows 11 x64 以外のプラットフォーム検証は未実施で、CI matrix に委譲した。
