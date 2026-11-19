---
id: "EVI-R2wVE4pn6ttJAZU8Iz6Pl1"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "プラグイン L1 の結合・E2E 検証記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/22", "https://github.com/scarecrow173/ruviform/issues/29", "https://github.com/scarecrow173/ruviform/issues/76", "https://github.com/scarecrow173/ruviform/issues/113", "https://github.com/scarecrow173/ruviform/issues/114", "https://github.com/scarecrow173/ruviform/issues/116", "https://github.com/scarecrow173/ruviform/issues/117", "https://github.com/scarecrow173/ruviform/issues/118", "https://github.com/scarecrow173/ruviform/issues/119", "https://github.com/scarecrow173/ruviform/issues/121", "https://github.com/scarecrow173/ruviform/issues/123", "https://github.com/scarecrow173/ruviform/issues/238", "https://github.com/scarecrow173/ruviform/issues/242", "https://github.com/scarecrow173/ruviform/issues/253", "https://github.com/scarecrow173/ruviform/issues/336", "https://github.com/scarecrow173/ruviform/issues/360", "https://github.com/scarecrow173/ruviform/issues/625", "https://github.com/scarecrow173/ruviform/issues/763"]
  informs: []
  explains: []
  supersedes: []
---

# プラグイン L1 の結合・E2E 検証記録

本書は、プラグイン L1（プロセス外プラグインを制御面・共有メモリ・RT 脚で扱う層）の結合・E2E 検証で得られた実測値と実行結果を記録する。記録のもとになった実装記録群は後に削除されたため、本書はその内容を単独で読める形に再構成したものである。検証は Windows 11 x64 上のローカル環境と、CI の `verify` ジョブの 3 OS matrix に分担された（Issue #76）。本書では測定値・観測された動作を事実として記し、解釈が必要な箇所は「解釈」と明示する。各事実は出典の Issue 番号で帰属させる。

## plugin-core の単体検証と RT フィルタの限界

plugin-core の検証では、`cargo test -p ruviform-plugin-core` で 29 単体テストと 19 doctest の計 48 テストが合格し、clippy は clean、`cargo metadata` による依存監査では依存が audio-core と media-core のみで `publish=[]` であることが確認された。規格中立性の grep では公開面に binding パスが出現しなかった（Issue #29）。

`rt_` 名フィルタは 4 テストを選択するが、アロケーションを観測できないため、この段階では文書化契約とレビュー対象にとどめ、Issue #22 の RT 検証レーン導入で実測化する方針を Risks に記録した（Issue #29）。Issue #22 の thread-check 検証では `mise run check`/`lint`/`test` が全パスし、test は 43 unit と 3 doctest だった。新規の `rt_process_off_audio_thread_is_defined_rejection` と `rt_process_on_audio_thread_passes_thread_check` は `rt_*` 命名で `test-rt` 対象となり pass した。拡張未提供時の degradation は既存の `host_instance()` を使うライフサイクル/process テスト群がカバーした（Issue #22）。

## Issue #76: L1 基盤・制御面・データ面と Full Gate

Issue #76 では、契約移設・Lifecycle Failed・wire codec を含む T1 が check/lint/test/test-plugin-core/lint-plugin-core の全 lane で exit 0 となり、test-plugin-core は lib 59・video_contract 8・doctests 26 など 95 pass、test-rt の実一致は 109 件だった。レビュー修正（generation 世代管理・CLAP STATE_FAILED・coverage 追加・doc link）はすべて mutation 検証済みである（Issue #76）。

制御面では、spawn・handshake・コマンド転送・クラッシュ検出を検証する制御面 E2E 20 テストが全 pass し、workspace は 55 suites だった。cargo tree で `ruviform-rt` と `shared_memory` の存在を確認した。レビュー修正（P1: Shared.child 共有と converge 側 force-kill、P2: driver-only shutdown_initiated、try_wait Err の kill+reap、reader detach、shm key retry、driver write 前 terminal check）は mutation 検証済みである。spawn の最悪時間は `3 × command_timeout`（spawn hand-off・`Hello` wait・`Scan` reply）として doc に明記した（Issue #76）。

データ面では、l1_proxy_e2e 34 テストが全 pass し、process 経路の gain 正当性・定義 2 ブロック遅延・crash/hang 時の pass-through・ハングなし・既存 20 テストの無退行を確認した。test-rt では `rt_l1_process_path_no_alloc` と `rt_l1_result_leg_try_ops` が実一致 2 件で、`rt_` フィルタで l1_proxy_e2e binary 内の 2 件が実選択され 32 件が filtered out された。レビュー修正（hostile dims publish withhold・stale plane zero-fill・stale-submit ゲート・atomic::fence 強化など）は適用済みである（Issue #76）。

Full Gate の実測では、`cargo test` が 55 binaries・1021 passed・0 failed（`l1_runner_e2e` 11・`l1_proxy_e2e` 42 を含む）、test-rt の実一致は 111、`prek run --all-files` は 7 hooks passed・1 skipped（Windows 非対象）、doc audit は blocking 0・findings []（documents 317）だった。補助の `cargo tree` で `ruviform-plugin-export`/`-host` の直接外部依存は `shared_memory` のみで、host は `ruviform-rt`+`shared_memory`、外部依存クリープはなかった。Windows ローカルと Rust 実証のみで、mac/Linux/CI は CI `verify` ジョブの 3 OS matrix に委譲し、未実施を主張しない方針を取った（Issue #76）。

## Issue #113: endpoint 機構・公開面・graph E2E と RT lane

Issue #113 では、L0 endpoint 機構（Shared slot + endpoint inner）について check/lint/test と `cargo test -p ruviform-plugin-host` の narrow gate が全 exit 0 となり、新規 L0 単体テスト（Shared dispatch・verb 到達・epoch 二重 attach 拒否・Failed 収束・busy bypass・poisoned destroy・plugin_mut None・lifecycle_state mirror のみ読み）が pass した。test-rt では新規 `rt_endpoint_process_no_alloc`/`rt_endpoint_busy_bypass_no_alloc` の 2 件が実一致した。独立レビュー 2 件（A=APPROVE・B=全攻撃面クリア）の確定修正（poison 起源 comment・dead_code expect tripwire・attach 順序 doc・verb starvation doc など）を適用した（Issue #113）。

L1 endpoint（leg 移出・arm channel・pending_arm・leg 返却・drain-before-early-return）では、lib 55 tests（新規 25 件）・l1_proxy_e2e 54 tests・workspace 55 binaries が全 pass した。test-rt では新規 `rt_l1_endpoint_process_no_alloc`/`rt_l1_endpoint_arm_drain_no_alloc`/`rt_l1_detached_process_no_alloc`/`rt_l1_arm_apply_no_alloc` の `rt_` 4 件が実一致した。review-driven 修正 7 件（wait-free drain 分割・non-RT drain re-arm・共有 Mutex<Producer<u64>> result channel・over-bound arm pass-through 収束・channels gate・Destroyed restore gate など）を適用した（Issue #113）。

endpoint 公開面では、`prepare_endpoint`/`latency_frames`/統一 defined-rejection 表/crate doc/stale 遅延式修正を実装し、新規テスト 10 件・doctest 1 件・e2e assertion 2 箇所を追加した。T2/T3 の `expect(dead_code)` 11 件を除去し（残存 2 件は `in_scratch` field と `arm_retire_overflows` で意図的）、test は 55 binaries・1105 tests 全 pass、`cargo doc` は 0 warnings、doc audit は blocking 0 だった（Issue #113）。

graph E2E と RT lane では、`process_audio` shell・`plugin_graph_e2e.rs` 13 tests・dev-dep 配線を実装し、`cargo test -p ruviform-plugin-host` は 142 pass（lib 72 + l1_proxy_e2e 55 + plugin_graph_e2e 13 + doctest 2）、test-rt 実一致は 129 だった。`cargo tree -p ruviform-audio` で plugin 非出現、`--edges normal` で `ruviform-audio` 非出現を確認した。`rt_` フィルタは substring match のため `shared_start_failure_rolls_back_to_activated` が副次一致する計数クセも記録した（Issue #113）。

Full Gate 15 コマンドは全実実行で全 exit 0 だった。`test` は 56 binaries・1125 passed・0 failed、`test-rt` は実一致 129・0 failed、`cargo doc` は `-D warnings` 下で warning 0 だった。既知 flake 2 件（`hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded`）は 1 発目から pass で未発生だった。11 AC と actions・test-spec・実 test 名の写像表を記録し、plan を `completed`・3 test-spec を `approved` へ同期した（Issue #113）。

## Issue #114: CLAP/VST3 bridge・runner smoke・opt-in lane

CLAP bridge では `host.rs`（1,377 行）・`ClapLibrary`/`ClapPluginFactory`/`ClapPlugin`/`ClapHostError`/`BridgeHandlers`/`fixture_lib_name` を実装し unit test 17 件を追加した。`cargo test -p ruviform-plugin-clap --features host` は 63 unit test 全 pass で、`rt_process_drives_fixture_block_allocation_free`（`assert_no_alloc` 窓内で `process_host` 駆動）を含んだ。敵対的レビュー確定修正 7 件と output bounce deviation を全反映した（Issue #114）。

VST3 bridge では `host.rs`（2,214 行）・`Vst3Library`/`Vst3PluginFactory`/`Vst3Plugin`/`Vst3HostError`・COM host objects・`Phase` enum lifecycle を実装し unit test 16 件を追加した。`cargo test -p ruviform-plugin-vst3 --features host` は 47 unit test 全 pass だった。敵対的レビュー確定修正 8 件（LoadGuard・cast 分岐・restartComponent・debug_assert・continousTimeSamples・count caps・param hoist・transport coverage）を全適用した。`cargo tree` で `host` off 時に `libloading`/`ruviform-plugin-export` が non-optional 木に出ないことも確認した（Issue #114）。

runner 直接起動 smoke では、両 runner bin を実装し、shell 実測で引数なし起動が exit 2（`missing --ruviform-shm-key` usage reject = fixture load 通過の証拠）、`RUVIFORM_L1_REAL_PLUGIN_DIR=/nonexistent` が stderr diagnostic + exit 1（pre-`run` defined rejection）となることを pin した。runner unit test は各 2 件（override verbatim join・sibling 解決 + fixture 実在 assert）だった。敵対的レビューは clean verdict（確定修正 0・informational 2 件は spec-accepted）だった（Issue #114）。

opt-in lane は `test-l1-real-plugin`（build → `--lib --bins` → `--ignored`）・`lint-l1-real-plugin`・`l1-real-verify` ci job の 3 段で配線した。lane 実測では `--lib --bins` が CLAP lib 63+bin 2・VST3 lib 47+bin 2、`--ignored` が CLAP 3+VST3 3（`real_adapter_l1_full_series`/`kill_reports_failed_once`/`missing_runner_is_ipc_rejection` 各 ×2・実子プロセス）だった。敵対的レビュー確定修正 6 件（`--lib --bins` 化・delivered floor・macOS note・Scan comment・cast_unsigned・GUI verb comment）を全反映した（Issue #114）。

Full Gate 19 コマンドは全実実行で全 exit 0 だった。`test` は 58 binaries・1125 passed・0 failed、`test-rt` は実一致 129（Issue #113 計測と同一で無退行）、opt-in lane 3 段全 green、`cargo doc` は `-D warnings` 下で warning 0、runner 直接起動 smoke で exit 2/1 を再実測した。8 AC と actions・test-spec・実 test の写像表を記録し spec `proposed → implemented`・plan `completed`・3 test-spec `approved` へ同期した（Issue #114）。

## Issue #116: supervisor core・E2E・fault flags

supervisor core（watch/poll state machine・attempt replay・checkpoint bound・host binding）では `--lib` supervisor 33/33 pass だった。composite.rs/l1//host.rs/wire.rs は merge-base fc02103 対比で byte-level 無 diff（AC1/AC4 の機械的証明）で、supervisor.rs の use は lib.rs 公開面のみだった。T4 gate で test 60 binaries・1294 passed・test-rt 137 一致 0 failed・`cargo doc` warning 0 だった（Issue #116）。

supervisor E2E では l1_supervisor_e2e 22 tests 全 pass で、restarted_dedicated/bundled・checkpoint 4 分類（Applied/NotRequested/Skipped/PluginFailed burn）・stable_window リセット回帰・zero_max_attempts・inert_policy・crash-loop bounded GaveUp・bundle burst cap 繰越・attempt 失敗全段・lingering・adopt_failed_handle・wrong_host・plain_host・`rt_l1_supervised_process_no_alloc` を含んだ。l1_proxy_e2e 100/100 は無 diff green で supervisor 非混入の証拠となった（Issue #116）。

fault flags（`--fail-restore`/`--state-unsupported`/`--fail-initialize`/`--fail-start-processing`）は Flags struct・parse_flags・argv doc table に追加し、T3 E2E で全系発火を実子経路で検証した（checkpoint_unsupported_skips・checkpoint_pluginfailed_burns・fail_initialize_attempt_cleans_created・fail_start_processing_attempt_unwinds_activated 全 pass）。flag 未指定時は l1_proxy_e2e 100/100 無退行だった（Issue #116）。

Full Gate は全コマンド初回 exit 0（補正・rerun 不要）だった。`test` は 60 binaries・1294 passed・0 failed、`test-rt` は実一致 137・0 failed（Issue #117 計測 132 から +5）、opt-in lane 3 段全 green、`cargo doc` は `-D warnings` 下で初回から warning 0 だった。AC1–AC10 と R-V1–R-V6 の写像表を記録し spec `draft → implemented`・plan `completed`・2 test-spec `approved` へ同期した（Issue #116）。

## Issue #117: wire v2 codec・route pool・bundle child

wire v2 codec では、wire codec unit test（round-trip・scope 行列・InstanceLimit 両方向）・l1_runner_e2e MiniHost v2+registry bundle E2E・l1_proxy_e2e dedicated v2 経路が全 pass した。`--ruviform-` の code site 残存なしを grep で確認し（doc-level 言及は T2/T5 の範囲）、WIRE_VERSION=2・WIRE_MAX_INSTANCES=64・WIRE_INSTANCE_NONE=u32::MAX が codec に存在することを確認した（Issue #117）。

route pool と隔離選択面では、`Route::OutOfProcess` struct variant + pool・`OutOfProcessIsolation` 公開面について `cargo test -p ruviform-plugin-host --lib` の pool/isolation unit test が pass し、`cargo doc -p ruviform-plugin-host --no-deps` は warning 0 だった（Issue #117）。

bundle child では、l1_proxy_e2e に bundle E2E（1子N instance・(instance,seq) 相関 no crosstalk・kill_child fan-out・InstanceLimit 伝搬・shutdown-on-empty・Failed+live child の Destroy・mid-DestroyInstance 死亡・dedicated 非回帰・rt_ bundle）を追加し 99 tests pass した。l1_runner_e2e は 21 tests（e2e_create_instance_unknown_plugin_id 追加）、rt_ bundle テストが `assert_no_alloc` で pass した（Issue #117）。

Full Gate 12 コマンドを実施したところ、`cargo doc` が初回 exit 101 となった。`pub mod shm` の doc comment 内 8 intra-doc link（`[`HEADER_LEN`]` 等）が `shm` 子モジュールの非公開パスへ裸名で張られており rustdoc が解決不能だったもので、merge-base `d71bdd0` からの pre-existing 欠陥であり、本 gate が plugin-core を対象に初実施で検出した。`[`shm::…`]` 修飾へ doc-only 補正して exit 0・warning 0（before/after 両 log 保存）とした。`test` は 59 binaries・1239 passed・0 failed、`test-rt` は実一致 132・0 failed、opt-in lane 3 段全 green（v2 handshake 経由）だった（Issue #117）。

## Issue #118: video wire v3・video host surface・video runner・video E2E

wire v3 codec では、全 11 video 動詞+4 Reply の round-trip・scope 違反行列（video 動詞に instance=NONE/≥64→Malformed）・VideoProcessConfig/VideoFrameDesc/ColorInfo/PlainParamValue/PlainParamInfoOwned 拒否行列・InvalidValue 両方向 pin・Hello{version:2}/capabilities 非ゼロ拒否・bounded decode・vshm byte math const test が pass した。既存 audio suite が WIRE_VERSION=3 のまま green で、audio 無退行の T1 時点証拠となった（Issue #118）。

video host surface では、`video()` façade・Route/Slot video arm・descriptors kind-tag・create_video_instance・Creating ゲート・request_timed・demux 合成 frame・mirror 不変の unit test が pass し、既存 audio suite（l1_proxy_e2e）は無変更 green だった（Issue #118）。

video runner と erased surface では、erase_video・run_video・VideoEntry・vshm attach・reference bin・runner E2E を実装し、l1_video_runner_e2e（MiniHost video runner E2E — InvalidHandle flagged reply・dup CreateVideoInstance→Err(Ipc)・InstanceLimit・attach 拒否行列・stash config 再生・param 先行検査・panic 正規化・cross-kind・inbound BlockIn drop・argv `--ruviform-*`→exit 2）が pass した。既存 audio runner E2E（l1_runner_e2e）無変更 green、plugin-core erase_video adapter unit test も pass した（Issue #118）。

video fixture と公開面 E2E では、l1-test-video-worker（hand-rolled wire child + spec.args argv fault matrix）と tests/l1_video_e2e.rs を新設した。l1_video_e2e は happy-path（lifecycle・render pixel oracle・pts/duration/color pass-through・params round-trip・ParamInfo(None)）・cross-kind suite（kind filter Scan・DuplicatePluginId・Unsupported/EndpointUnavailable）・Failed parity（子死亡/InstanceFailed/create 中死亡/Err reply mirror 不変/semantic violation→Err(Ipc)/inert BlockIn/post-Shutdown/obligation (a)(c)）・bound 拒否・render_timeout 分離・real runner 2 正面を pass した。`cargo test -p ruviform-plugin-export --test l1_video_runner_e2e` も pass した（Issue #118）。

Full Gate では fmt-check/check/lint/test/test-rt/bench-smoke/`cargo doc` warning 0/prek/doc audit blocking:0 が全 pass だった。`mise run test` は既知負荷依存 flake 2 件（`hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded` — `command_timeout=500ms` の spawn+handshake budget 競合）で初回・2 回目・3 回目 exit 101、4 回目で exit 0（41 binaries + merged doctests・1436 passed・0 failed）だった。切り分けとして suite standalone 120/120 × 2・孤立 rerun 2/2 pass・`l1-test-worker` は Issue #118 範囲 zero-diff・末尾 crate 個別実行で全緑を確認した。`l1_video_e2e` 33/33・`l1_video_runner_e2e` 20/20・test-rt 実一致 139・opt-in lane 3 段全 green だった。T1–T5 task status residual（6/6 task が `todo` 残存）を T6 で修正し incident として記録した（Issue #118）。

## Issue #119: wire バージョン不一致の専用テスト

Hello version の厳密一致・reserved/capabilities 非ゼロ・`WIRE_MAX_*`・shm magic/形状は既存テストで固定済みで、欠落は runner shm `wire_version` の専用テストのみだった。`create_segment` を `create_segment_with(key, magic, wire_version)` へ一般化し、`e2e_shm_header_wire_version_mismatch`（`WIRE_VERSION + 1` → Hello 送出後に非ゼロ終了）を追加した。製品コード変更はなく、version override 機構は製品側に追加せず fixture 破損で検証した。runner E2E は 12 tests 全 pass で、fmt-check/check/lint/test/test-rt 全 exit 0、doc audit は blocking 0・findings [] だった（Issue #119）。

## Issue #121/#123: FailureCause 配線と L1 内部クリーンアップ

FailureCause 配線では、`cargo test -p ruviform-plugin-host` で 182 lib test pass（`converge_cause_is_first_observed_per_instance` 等の分類単体テスト含む）、`l1_proxy_e2e` で 130 件 pass（通知 payload・exactly-once 無退行）だった。変更面は compile-caught 前提で `converge_child`/`converge_instance` の cause 記録・notify payload 型追従（`l1_proxy_e2e.rs` harness 含む）・`Shared`/`InstShared` literal 14 箇所の field 追加を実施した（Issue #121）。

L1 内部クリーンアップ（frame-I/O dedup・runner_program 解決・gui_open 除去・audio-core doc 修正）では、fmt-check/check/lint/test/test-rt/`cargo doc` 全 exit 0 だった。初回 `mise run test` で `hang_converges_on_command_timeout` と `wedged_child_write_converges_and_teardown_is_bounded` が `Err(Ipc)` で flake したが、本 change 起因でなく既知の load-sensitive 経路であり、孤立 rerun 各 pass・suite 再実行で全 green を確認した（wire doctest ×3 含む）。lint で l1-test-worker main() の too_many_lines 106/100 を局所 `#[allow]`+理由コメントで解消し、`rg gui_open crates/`=0 件・`cargo doc` warning 0 を確認した。`composite_e2e::runner_bin_serves_real_gain_plugin` pass が cross-package runner fallback + 共有 framing の実動作を直接検証した（Issue #123）。

## Issue #238/#242/#360: flake 解析と反復 green

Issue #238 では、Windows 11 x64 で `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` を default/16/1 threads で 7 連続実行し全て 172/172 PASS（10.7〜21.3 s）だった。`mise run test` workspace 全量 112 suite 全 `test result: ok` だった。review-protocol の Reviewer A/B 並行 dispatch は両者 Issue Candidate ゼロ、Judge は ACCEPT WITH NON-BLOCKING NOTES（barrier の wire-order 証明・bundled 共有 stdin 網羅・hung 構造的上界を双方が独立検証）だった。残留 `thread::sleep` は bounded-poll wait helper ×5 のみで「正しい形」と判断された（Issue #238）。

Issue #242 では、残存 3 suite の sleep 監査で真の wall-clock ペーシングは `l1_supervisor_e2e.rs` の `settle_notification()`（50 ms × 7 call site）のみと判明した。`l1_video_e2e.rs`/`l1_runner_e2e.rs` は変更なしで、残る sleep は全て deadline 有界 poll 内の retry backoff で wall-clock ペーシングは存在しなかった。検証は `l1_supervisor_e2e` 23/23 PASS ×4 default + threads=1/16 各 1 回、`l1_video_e2e` 36/36 ×10、`l1_runner_e2e` 21/21 ×4 だった。`l1_video_e2e` 初回のみ 31 pass/5 fail（worker binary cold-spawn 一過性、以降 10 連続 green）で、merge `2a1b309` 後も fmt-check + supervisor 23/23 を再確認した（Issue #242）。

Issue #360 では、heir 系 4 サイト（destroy_detaches_reader_when_heir_holds_stdout・leave_heir_yields_exit_observed_and_bounded_drain・heir_stderr_inside_grace_lands_in_record・heir_stderr_after_drain_stop_is_never_read）修正後、`cargo test -p ruviform-plugin-host --test l1_proxy_e2e heir` を 6 回連続 green（SpawnFailed 非観測、Windows 11 x64 / Rust 1.99.0）とした。`mise run fmt-check`/`lint`/`test`（147 suite 全 green）も exit 0 だった（Issue #360）。

## Issue #625/#763: CI e2e handshake race と flaky e2e oracle

Issue #625 では、run 43540652567 の verify (ubuntu-latest) 失敗を一次ログで確認した（e2e_process.rs:325 で expected handshake refusal, got Io(BrokenPipe)、他 17 test pass、他必須 job success）。2613b238..origin/main に既修正なし（5 commit は crates/ruviform-ui-test/ 未触）を確認し、doc-chain 作成→driver.rs 実装→driver 内 unit test で分類契約を決定的に固定した。fmt-check/check/lint/test `--test e2e_process`/test/test-full 全 Pass、review-protocol は blocker なし ACCEPT WITH NON-BLOCKING NOTES（P3 指摘 2 件は非 blocking）だった。`mise run test ruviform-ui-test` は新規 unit test 7 件を含め pass で、fake FailWriter/FailReader/exit 済み child fixture で error 経路を決定的に再現した（Issue #625）。

Issue #763 では、両 adapter crate（ruviform-plugin-test-clap/vst3）の l1_real_test_plugin_e2e.rs を counter delta 分類へ改修し、対応する test-spec の R7 を substituted-output semantics に更新・Risk 節を更新した。canonical lane（fmt-check/check/lint/test 両 crate・check/lint/test-l1-real-plugin・prek）全 exit 0 で、test-l1-real-plugin は N≥3・内 ≥1 は loop+load lane で実測記録した（Issue #763）。

## Issue #253: MIDI 経路の検証と実機 lane の NotRun 記録

Issue #253 は MIDI 関連の検証を多数含む。unsupported 透過と実機 lane scaffold では、mock.rs/input.rs/status.rs/tests の UT・ST・TX 系と `#[ignore]` E2E（enumerate→skip→open→複数 read→close→StreamInvalidated）を追加し、fmt-check/check/lint/test/test-rt 全 Pass + MSRV 補助（rust@1.89.0）+ review-protocol 実施済みだった。実機 lane の実行結果または NotRun+理由は専用の evidence 記録に残した（Issue #253）。

virtual MIDI e2e verifier では、`os_endpoint_midi_e2e.rs` を新設し、実 WinMM backend→name-match loopback pair→InputTx::pump→InputStage→attach→PluginNode→GraphRuntime→out-of-process VST3 test_instrument→L1 PCM oracle の合成 e2e を `#[ignore]`-gated で実装した。l1-runner feature compile・clippy `-D warnings`・canonical mise lane 全 exit 0 で、補助実走 `--ignored` は exit 0 で verdict NotRun—NOT_PROVISIONED を記録した。pair 存在時の Pass 経路は本 slice では未実走で、oracle は scripted 先例と fixture 定数から導出したが、実 WinMM stamp 粒度・loopback 転送遅延でのフレーク可能性は provisioning 環境での初回実走まで残ると記録した（Issue #253）。

VST3 MIDI 正規経路 slice では plugin-core drain_output_reports contract・midi.rs の ungate + MidiAssignmentTable/control_assignment/routed_assignment + fill_event_list midi_map・host.rs の IMidiMapping 解決・writable output queue・report drain・fixture/controller IMidiMapping・cap.rs param_reporting→Declared・E2E/unit tests（MM-01..05, PR-01..05）・IR + canonical 検証 lane を実施した。report verb 実装では `fmt-check`/`check`/`lint`/`test`/`test-rt`/`test-plugin-core`/`lint-plugin-core`/`verify-ci` の全 lane が exit 0 で、`PluginHost::drain_output_reports` の L1 audio slot が `Drained` + canonical `Event` を返し、旧 `Unavailable` 期待の graph E2E を置換した。実 OS MIDI・実デバイス・実 DAW・consumer 消費・capability 昇格は `NotRun`/`Unsupported` として明示記録された（Issue #253）。

host-graph では Windows 11 x64 rust 1.99.0 で fmt-check/check/lint/test/test-rt/lint-l1-real-plugin/test-l1-real-plugin 全 PASS、`cargo test --workspace` 200 `test result: ok`・失敗 0（新規 unit 4 件 + e2e 2 件）、test-rt 155 ok で RT lane 非退行だった。review-protocol で test-spec DR-03 の verb-interleave 例に対する coverage gap を CONFIRMED P2 として検出し、`buses` verb を processing と drain の間に挟む assert を plain-slot test に追加して解消した。他に CONFIRMED P0/P1 はなかった（Issue #253）。

fixture では fmt-check/check/lint/test（workspace 192 test-result ok）/test-rt/lint-l1-real-plugin/test-l1-real-plugin 全 PASS、`cargo test -p ruviform-plugin-vst3` default 39 pass・`--all-features` 65 pass だった。MSRV は専用 task が対象外のため `mise exec rust@1.89.0` で scoped 実行し pass。実 OS MIDI backend・実デバイス検証・Issue #114 パターンの opt-in lane・実 DAW 往復は全て NotRun として正直に記録した（fixture cdylib の in-process L1 E2E のみ実走）（Issue #253）。

補助証拠の選定では、対象 SHA `b9eaf488` に対応する completed CI run が存在しないことを `gh run list --commit` = 0 件で実測し、Coordinator 裁定で `e93c438a` run `43428172820`（全 20 job success）を採用した。`e93c438a..b9eaf488`（11 files）・`b9eaf488..05c7fc3b`（15 files）・合算の 3 通り `git diff --name-only` で MIDI 影響 path が 0 件であることを検証済みと明記した。補足として `05c7fc3b` の dispatch run `43436082797`（conclusion=failure）でも MIDI 関連 lane（Windows/macOS verify・rt-verify・l1-real-verify×3）は全て success を確認した（Issue #253）。

job log 実測の test 集計では、midi-device lib 17+1ig、ports.rs 2+1ig/1+1ig/1+1ig、bridge midi_delivery 6・midi_wiring 15+1ig、plugin-vst3 41（default）/74（host）、plugin-clap 48/65、rt-verify strict `rt_` = 166 で `rt_midi_*` 2 件 ok、l1-real-verify の `imidi_mapping_*` 3 件 ok・ignored E2E は audio のみで MIDI coverage 0 だった。run `43436082797` は `verify (ubuntu-latest)` が prek `actionlint` hook の shellcheck 指摘で cargo 未到達 failure（05c7fc3b での Linux `test-full` は NotRun）+ `ci-summary` fail-closed failure で、main の CI 破損は別対応が必要と記録した（Issue #253）。

os-backend では fmt-check/check/lint/test/test-rt 全 PASS（workspace）、`cargo check -p ruviform-midi-device --target x86_64-unknown-linux-gnu` で非 Windows cfg 経路の compile 確認（`mise run check-linux` は io-file/io-archive のみ対象のため crate 個別実施）、`cargo test -p ruviform-midi-device` は unit 17 pass + 1 ignored・integration 2 pass + 1 ignored・doctest 1 pass だった。実 MIDI デバイス検証・実 DAW 往復・macOS/Linux backend・MIDI2/UMP（`supports_ump=false` で非対応宣言、Issue #317）は全て NotRun/Unsupported として記録した（Issue #253）。

readme-closeout では Windows 11 x64・rustc 1.99.0 で fmt-check/check/lint/test/build 全 PASS。初回 `mise run test` で `l1_video_e2e` の 2 test が `SpawnFailed { cause: Timeout }` で、並行 worker ビルド負荷下の spawn timeout infra flaky（先行 slice 記録の `hang_converges_on_command_timeout` と同型・docs のみ変更で無関係）であり、単独再実行と全量再実行で Pass だった。`mise run test-midi-real-device` 実走で 3 つの `#[ignore]` test 全て pass で、本環境は MIDI input ポート非在のため honest skip（実測偽装なし）だった（Issue #253）。

unsupported-transparency では Windows 11 x64・rustc 1.99.0 で fmt-check/check/lint/test/test-rt 全 PASS。初回 `mise run test` で `hang_converges_on_command_timeout` が `SpawnFailed { cause: Timeout }` で spawn timeout の infra flaky（plugin-host は scope 外・diff なし）であり、単独・全量再実行で Pass だった。`cargo test -p ruviform-midi-device -- --ignored` と `-p ruviform-audio-plugin-bridge -- --ignored` は「no MIDI input ports — nothing to verify」で honest skip・exit 0 だった。Linux target の `cargo check --all-targets` で非 Windows cfg 経路も確認した（`MidiInputPort` import の cfg 分離で unused_imports を回避 — review で検出・修正済み）（Issue #253）。

virtual-midi-e2e の compile/clippy/fmt では `cargo check`/`clippy -D warnings`（`--features l1-runner --tests`、feature なしでも `#![cfg]` gate で空 compile）/`fmt --check` 全 Pass で、canonical mise lane（fmt-check/check/lint/test・l1-real-plugin 系・prek）は検証記録に残した。補助実走 `-- --ignored` の verdict も同記録に残し（pair 未 provision なら `NotRun`）、pair 存在時の Pass 経路は本 slice では未実走だった。dev-dep は必要最小限の `ruviform-midi-device` のみ（`ruviform-audio-device` は未追加）だった（Issue #253）。

vst3-canonical では `cargo test -p ruviform-plugin-vst3 --features host --lib` 74 PASS で、`imidi_mapping_routes_volume_cc_to_gain_param`（CC7→gain 配送）・`unmapped_cc_passes_through_without_param_routing`・`plugin_output_reports_drain_as_canonical_events`（双方向 report 往復）等を含んだ。mise run fmt-check/check/lint/test/test-rt/lint-l1-real-plugin/test-l1-real-plugin 全 PASS、`git merge origin/main` fast-forward 競合なしだった。実 OS MIDI backend（slice a）・opt-in 実デバイス/実 plugin 検証（slice d）・実 DAW 往復は NotRun、host-graph consumer 配線は当時未配線（後続 slice で対応）だった（Issue #253）。

## Issue #336: observability logging feature の補助証拠

Issue #336 T1 では crates/ruviform-observability/src/logging/build.rs の SinkDegraded に Clone を derive し、tests/logging.rs に clone 複製が sink_index/kind/reason を保持する最小アサーションを追加した。LoggingBuildResult/LoggingLayersResult は非 Clone のまま維持し、他 trait は追加しない acceptance を満たした（Issue #336）。

`logging` feature は既定 off・CI lane なしのため、feature 有効検証（`cargo test -p ruviform-observability --features logging` で 28 pass + doctest 5、clippy 同 feature clean）はローカル実施で「canonical = mise/CI ではない補助証拠」と位置づけられた（Issue #334/#335 と同構成）。canonical gate は `mise run fmt`/`check`/`lint`/`test` 全 pass で、feature 有効の自動検証を CI に載せるかは別途検討事項として follow-up とした（Issue #336）。

## Issue #76/#113 の deviation 記録

Issue #76 では、task doc が「自前 `unsafe` ゼロ」を想定していたが、`shared_memory` 0.12.4 の `as_slice`/`as_slice_mut` が `unsafe fn` であることが判明し、`shm.rs` の `bytes`/`bytes_mut` に scoped `#[expect(unsafe_code)]` ×2（SAFETY コメント付き）を局所許可として deviation 記録した。検証は `cargo test` で runner E2E 11 tests 全 pass、`cargo tree` で依存が `ruviform-plugin-core` + `shared_memory` のみであることを確認した（Issue #76）。

composite E2E では `l1_proxy_e2e` 42 tests 全 pass（新規 `composite_e2e` 8 件: L0+L1 混在・runner bin 結合・`DuplicatePluginId`・L1 kill→`InstanceFailed` exactly once・`Destroy` legality・stale handle non-cross-route・advisory `stop_processing` mirror・shadowed registration）だった。`test-rt` は `rt_l1_process_path_no_alloc`/`rt_l1_result_leg_try_ops` が実選択 2 件（0 件スキップではない）だった。deviation は `register_factory` infallible 維持（first-claimant shadowing）・`runner_program()` の `current_exe` → `target/<profile>` 解決・`Box<L1Instance>` large_enum_variant だった（Issue #76）。

Issue #113 T5 では全 gate green で、`test` で 55 test binaries 全 pass、`test-rt` で ruviform-audio の `rt_` 実一致 6 件（新規 `rt_plan_active_no_alloc`/`rt_plan_swap_retire_no_alloc`/`rt_plan_drop_retire_no_alloc`/`rt_fn_node_pull_no_alloc` + 既存 2 件）だった。`static_assertions` dev-dep で `GraphPlan: Send+Sync`・`RtPlan: Send`・`PlanSwap: Send+Sync`・`Session: Send` の正側 pin と `Rc`/`RefCell` の負側 pin を compile-time 証拠化した（`expect_send` 型関数では負側を表現不能なため採用）。deviation は `Cargo.lock` の `static_assertions` entry が T2 commit に既着で単一 file 分割不能のため本 commit diff に含まれないことだった（Issue #113）。

## 記録の範囲と限界

本書が記録する検証の多くは Windows 11 x64 ローカル環境で実施され、macOS/Linux/CI は CI `verify` ジョブの 3 OS matrix に委譲された。Windows ローカル + Rust 実証のみで、mac/Linux/CI は未実施を主張しない方針を取った（Issue #76）。

NotRun または deferred として記録された主な項目は次のとおりである。実 OS MIDI backend・実デバイス検証・実 DAW 往復・macOS/Linux backend・MIDI2/UMP（`supports_ump=false` で非対応宣言、Issue #317）・consumer 消費・capability 昇格は NotRun/Unsupported（Issue #253）。virtual MIDI e2e の pair 存在時の Pass 経路は未実走で、provisioning 環境での初回実走まで残る（Issue #253）。`real_midi_device_drives_input_tx_to_stage` は opt-in ignored のままで実機 Pass は主張しない（Issue #253）。Issue #625 では ubuntu 固有 EPIPE 実機再現・macOS/実 window/RT device lane・`*-msrv` が NotRun で、統合検証は merge 後対象 SHA の CI（Coordinator 確認）に委譲した（Issue #625）。Issue #336 の `logging` feature 有効検証はローカル補助証拠であり、CI lane なし・自動検証を CI に載せるかは follow-up（Issue #336）。Issue #253 の `05c7fc3b` での Linux `test-full` は NotRun（run `43436082797` の `verify (ubuntu-latest)` が prek `actionlint` hook の shellcheck 指摘で cargo 未到達 failure）（Issue #253）。

flake として記録された既知の負荷依存テストは `hang_converges_on_command_timeout` と `wedged_child_write_converges_and_teardown_is_bounded` で、`command_timeout=500ms` の spawn+handshake budget 競合によるものである。複数 Issue で初回または複数回 exit 101 の後に孤立 rerun・suite 再実行で全 green となるパターンが繰り返し記録されており、Issue #118/#123/#253 では infra flaky として切り分けられた（Issue #118, #123, #253）。
