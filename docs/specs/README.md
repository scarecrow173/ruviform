# SPEC Documents

<!-- doc-suite:generated-index -->

Directory: `docs/specs`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| SPEC-034INYy2aDLxse9dy8MgGZ | Ruviform 設計ベースライン文書群の作成（アーキテクチャ・クレート構成・RT/オフライン実行モデル） | approved | [architecture-design-baseline-deliverables.md](./architecture-design-baseline-deliverables.md) |
| SPEC-034Kq7ipiNwhFkBrHtwlII | ruviform-audio-analysis 実装 spec | approved | [audio-analysis.md](./audio-analysis.md) |
| SPEC-034JROd7c0jBSZ1hnPEAg5 | 音声エンジン（pull グラフ・デバイスアダプタ統合・RT パス） | approved | [audio-engine.md](./audio-engine.md) |
| SPEC-034JOoOaExdxXgKrQ80l7x | chunk 分割時の OutputCallbackInfo timestamp 合成 | approved | [chunk-timestamp-synthesis.md](./chunk-timestamp-synthesis.md) |
| SPEC-034LEXC13iAgtRjaqGTPoF | ruviform-video-proc 色変換カーネル追加（linear working space + ICC 入出力経路） | proposed | [color-conversion-kernels.md](./color-conversion-kernels.md) |
| SPEC-034KScJM81H7da9TxzL0H5 | ruviform-video-proc コンポジット/遷移カーネル追加（blend/affine 初期セット） | approved | [composite-transition-kernels.md](./composite-transition-kernels.md) |
| SPEC-034JZDNRML000000000001 | denormal 対策（エンジン層: ソフトウェア flush 方針） | approved | [denormal-mitigation.md](./denormal-mitigation.md) |
| SPEC-034JPubrypO0IylcWkuqlz | fearless_dispatched 軽量カーネル劣後の原因分離（低負荷再計測） | approved | [dispatched-overhead-analysis.md](./dispatched-overhead-analysis.md) |
| SPEC-034KNvB6d6rMoH8b8xST22 | ruviform-edit 新設（NLE 非破壊編集データモデル + 評価射影） | approved | [edit-data-model.md](./edit-data-model.md) |
| SPEC-034MbMNCaEATFCOgSdtwkA | エンジン/利用側へのプラグイン統合（plugin-host ノード化 + L1 選択配線） | proposed | [engine-plugin-host-integration.md](./engine-plugin-host-integration.md) |
| SPEC-034IcwhwIFSNUNKtX7KK0Q | Rust 開発基盤の確立（mise/APM lockfile・最小ワークスペース・手動CI） | approved | [establish-rust-dev-foundation.md](./establish-rust-dev-foundation.md) |
| SPEC-034JcicHlRidBgcK1wPtj2 | 動画基盤 API の確定（video-core / video-proc・色変換・HDR 対応範囲） | approved | [implement-video-foundation-api.md](./implement-video-foundation-api.md) |
| SPEC-034O7hTzfdwtNrUQom8ISw | Issue #121 L1 診断・可観測性（underrun メトリクス・子 stderr 配管・失敗原因 payload） | approved | [issue-121-diagnostics.md](./issue-121-diagnostics.md) |
| SPEC-034N3QmyfzJRPM3AVpKQTn | Issue #124 OFX フォローアップ（catalog_open cross-window h 露出修正 + instance clip props clone） | approved | [issue-124-ofx-cross-window-h-exposure.md](./issue-124-ofx-cross-window-h-exposure.md) |
| SPEC-034N9oLnrGISolGjoiaTx2 | Issue #131 OFX UAF residuals — non-catalog window leases + lifecycle hardening | approved | [issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md](./issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md) |
| SPEC-034QdNpdBY3gkeQraIPKwP | Issue #184 Typed multi-bus Port/Connection Graph IR contract (Migration M1) | approved | [issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md](./issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md) |
| SPEC-034QhNRt6XTihijW6B8pOy | Issue #185 Generic Probe: Node/Port observation capability, Off/Monitor/Capture states, bounded RT-safe PCM capture, time/identity metadata, non-RT delivery contract | approved | [issue-185-generic-probe-rt-capture.md](./issue-185-generic-probe-rt-capture.md) |
| SPEC-034QkwrLNvylpSOCZVs2bC | Issue #186 Parameter Control Events — capability, delivery and observability contract | proposed | [issue-186-param-control-events.md](./issue-186-param-control-events.md) |
| SPEC-bg9fb3WGAfULNZMmr9jTMc | Issue #188 Timed MIDI — input, timestamp and plugin delivery contract | proposed | [issue-188-timed-midi.md](./issue-188-timed-midi.md) |
| SPEC-034Qf9Ze8jsyjHppvPkQup | Issue #189 Rational Beat Position + Variable PPQN (Migration M3) | approved | [issue-189-rational-beat-variable-ppqn-m3.md](./issue-189-rational-beat-variable-ppqn-m3.md) |
| SPEC-034Qg9hT0w1z3B9vJkLmNo | Issue #190 Generic Transport Span Scheduler / Applied Receipt / Clock-Plan Generation contract | approved | [issue-190-transport-span-scheduler.md](./issue-190-transport-span-scheduler.md) |
| SPEC-034QhrD80l0ZtpqNTqFvHa | Issue #191 PCM WAV encode/mux output backend | approved | [issue-191-pcm-wav-output-backend.md](./issue-191-pcm-wav-output-backend.md) |
| SPEC-034QfjN7Wt7AJBbhXeK60o | Issue #195 Typed Graph Audio Compiler・Prepared Plan・Buffer/PDC・Stateful RT Ownership・Plan Activation (Migration M2) | approved | [issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md](./issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md) |
| SPEC-hmdguU023dLBLtBKCM9kfH | Issue #196 M4 Typed Graph Plugin Adapter — multi-bus/stateful lifecycle/process offset precision | approved | [issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md](./issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md) |
| SPEC-jhXBM8Vy5edyp0fsJ6Nqbb | Issue #197 Generic Media Reader / Decoder・Prefetch・RT Playback Consumer 境界 | approved | [issue-197-media-reader-prefetch-rt-consumer.md](./issue-197-media-reader-prefetch-rt-consumer.md) |
| SPEC-Yr26o4c8Q0cCi56S9vjvsF | Issue #198 Stateful Sequential Offline Render・Frame/Preroll/PDC/Tail・Safe Output Receipt | approved | [issue-198-sequential-offline-render.md](./issue-198-sequential-offline-render.md) |
| SPEC-034QdDDqIQgpFoBr9E1Z1T | Issue #219 両 Consumer 互換契約・影響判定・証拠要件と Cross-Repo CI Gate 仕様 | proposed | [issue-219-consumer-compat-contract.md](./issue-219-consumer-compat-contract.md) |
| SPEC-034QoidM48Z7Yl3Xtw57h2 | Issue #233 media-reader prefetch fill/seek/pump contract clarifications + additive discard/reopen features | approved | [issue-233-media-reader-prefetch-contract-improvements.md](./issue-233-media-reader-prefetch-contract-improvements.md) |
| SPEC-034N3OsgUWx7hzkr8aicqq | Issue #76 フォローアップ: L1 内部クリーンアップ（frame-I/O dedup・runner_program 解決・gui_open 除去・audio-core doc 修正） | approved | [issue-76-followup-l1-internal-cleanup.md](./issue-76-followup-l1-internal-cleanup.md) |
| SPEC-034JNYkUPfvyc1GQ5hXtHN | workspace MSRV 引上げ（fearless_simd 本番昇格前提） | approved | [msrv-raise-for-fearless-simd.md](./msrv-raise-for-fearless-simd.md) |
| SPEC-ey1gakUyUmWw9lBalMOE79 | 音楽時間設計（小節/拍/PPQN・テンポマップ） | approved | [music-time-design.md](./music-time-design.md) |
| SPEC-034JRarfc0pLFZoZeGMVhn | オフライン実行ドライバ（チャンク駆動・進捗/キャンセル・決定論モード） | approved | [offline-execution-driver.md](./offline-execution-driver.md) |
| SPEC-034L2CiqIfMljZ5oNFbkx0 | OFX ホストアダプタ（Issue #98） | approved | [ofx-host-adapter.md](./ofx-host-adapter.md) |
| SPEC-034JZPGUI0000000000001 | プラグイン GUI PoC（規格準拠ネイティブビューへの WebView 埋め込み検証） | approved | [plugin-gui-poc.md](./plugin-gui-poc.md) |
| SPEC-aNDyfHu1ZkBojTHEltDl94 | ruviform-plugin-host 新設（L0 インプロセス・ハンドル/コマンド境界） | approved | [plugin-host-l0.md](./plugin-host-l0.md) |
| SPEC-034JZLISO0000000000001 | プラグイン分離方式設計（サンドボックス・プロセス分離） | approved | [plugin-isolation-design.md](./plugin-isolation-design.md) |
| SPEC-034LNeyep4t7pIzFlpPyTK | プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー） | proposed | [plugin-l1-out-of-process.md](./plugin-l1-out-of-process.md) |
| SPEC-034Ivc5nj1AE6P2PcZU6Ki | テンプレート由来のドキュメント・識別子をRuviform向けに置き換える | approved | [replace-template-derived-docs-with-ruviform.md](./replace-template-derived-docs-with-ruviform.md) |
| SPEC-034JOU6EjinMh0rhxE6yM9 | RT 検証基盤（allocation 検出・thread-check・検証文書） | approved | [rt-verification-lane.md](./rt-verification-lane.md) |
| SPEC-X9YOosDAuB8PX2Qv31thuB | ruviform-dsp 新設（上位 DSP カーネル層 — prepare/process 分離・RT 適合） | approved | [ruviform-dsp.md](./ruviform-dsp.md) |
| SPEC-5eEaW8lLmoyVPgocJnooYD | 第 2 ターゲット設計（NLE 範囲・OFX 等映像プラグイン規格・広範色管理） | approved | [second-target-design.md](./second-target-design.md) |
| SPEC-034JQubfZRC7nRmTsYvLiq | SIMD dispatch 層（prepare 固定 + カーネル/サイズ別ルーティング + 本番精度契約） | approved | [simd-dispatch-layer.md](./simd-dispatch-layer.md) |
| SPEC-034JZVID00000000000010 | 動画エンジン（フレームパイプライン・A/V 同期・シーク） | approved | [video-engine.md](./video-engine.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| SPEC-034Jc9QEqYLVfg1CXXTiPV | オーディオデバイス境界 trait と ruviform-audio-device PoC（バックエンド評価・採否） | implemented | [audio-device-backend-evaluation-poc.md](./audio-device-backend-evaluation-poc.md) |
| SPEC-034Jci19siJZexLs53Xh5d | DSP/SIMD 比較ベンチマーク PoC と採否 ADR | implemented | [dsp-simd-poc-adr.md](./dsp-simd-poc-adr.md) |
| SPEC-034J5FYaYfeL5zkzb4sIV4 | メディアデータモデル API の実装（media-core / audio-core / video-core） | implemented | [implement-media-data-model-api.md](./implement-media-data-model-api.md) |
| SPEC-034JcaMXubILS7j1FX5qob | メディア I/O 抽象境界と第一バックエンド PoC の実装 | implemented | [implement-media-io-abstraction-and-first-backend-poc.md](./implement-media-io-abstraction-and-first-backend-poc.md) |
| SPEC-034JcgdAKPNeh1YCFyprzh | プラグイン基盤の実装（ruviform-plugin-core + CLAP/VST3 アダプタ・binding 方式 PoC） | implemented | [implement-plugin-foundation.md](./implement-plugin-foundation.md) |
| SPEC-034Jcr3dOT5Uxp4znj42II | RT 基盤の実装（ruviform-rt プリミティブ・スレッド優先度機構・スナップショット交換） | implemented | [implement-rt-foundation.md](./implement-rt-foundation.md) |
| SPEC-034MHDt79C7KINncq66sE3 | L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP/VST3） | implemented | [issue-114-real-adapter-l1-e2e.md](./issue-114-real-adapter-l1-e2e.md) |
| SPEC-034NMZAKKGWufQ5vWwZNYe | Issue #116 L1 自動再起動・supervisor ポリシー | implemented | [issue-116-supervisor.md](./issue-116-supervisor.md) |
| SPEC-034NiVkp0Q1fpkaBoUGNY0 | Issue #117 L1 bundle 子プロセス（1 子プロセス N インスタンス） | implemented | [issue-117-bundle-child-process.md](./issue-117-bundle-child-process.md) |
| SPEC-034NV8AWA7qua5MfPc7q6i | Issue #118 video/NONRT プラグインの L1 経路 | implemented | [issue-118-video-l1.md](./issue-118-video-l1.md) |
| SPEC-034NhJTrJ1kCCldVF5XZKB | Issue #119 L1 wire バージョン方針（単一最新バージョン・交渉機構なし） | implemented | [issue-119-l1-wire-version-policy.md](./issue-119-l1-wire-version-policy.md) |
| SPEC-034N3OBemoWwFEpqzWlf7r | Issue #125 ICC LUT fixture coverage（intent 区別 pair + mft1/mft2/mAB 経路） | implemented | [issue-125-icc-lut-fixture-coverage.md](./issue-125-icc-lut-fixture-coverage.md) |
| SPEC-034N9j2YKqyKxZZZyUPKFe | Issue #130 media-core/dsp/audio-analysis assert_no_alloc レーン武装（AllocDisabler 宣言） | implemented | [issue-130-arm-assert-no-alloc-lane.md](./issue-130-arm-assert-no-alloc-lane.md) |
| SPEC-034TRm8kN4vWqZxYsJpCe | Issue #151 メディア観測情報・非破壊 PCM 取得契約の拡張 | implemented | [issue-151-media-observation-contract.md](./issue-151-media-observation-contract.md) |
| SPEC-034J6OCXHiP1izKRzbMmgz | Issue #18 フォローアップ（公開 enum 方針適用・時間演算修正・改善トリアージ） | implemented | [issue-18-followup-enum-and-time-fix.md](./issue-18-followup-enum-and-time-fix.md) |
| SPEC-034Jc9BJlI7ULEBWoIlMP2 | Issue #28 UI 技術検証 PoC（wry・描画バックエンド・フロントエンド FW・配布） | implemented | [issue-28-ui-poc.md](./issue-28-ui-poc.md) |
| SPEC-034JceNZQvAawWwMH4ct91 | 時間型拡張（FrameRate/TimeBase 意味分離・Timecode 拡張・丸め誤差累積方針） | implemented | [time-type-extension-framerate-timecode-rounding.md](./time-type-extension-framerate-timecode-rounding.md) |
| SPEC-034N3JBtBP4dPi8ob9HwDV | video-proc RT 検証レーンへの assert_no_alloc 適用（Issue #126） | implemented | [video-proc-rt-verification-lane.md](./video-proc-rt-verification-lane.md) |
