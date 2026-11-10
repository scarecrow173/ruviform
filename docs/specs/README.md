# Architecture Decision Records

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
| SPEC-034udQra6CS6JQJJN4e0hG | Issue #127 Linux 実配布方式の設計 — GPUI 標準 Driver の runtime 依存・Capability Matrix・パッケージ形式適合条件 | proposed | [issue-127-linux-dist-design.md](./issue-127-linux-dist-design.md) |
| SPEC-034N9oLnrGISolGjoiaTx2 | Issue #131 OFX UAF residuals — non-catalog window leases + lifecycle hardening | approved | [issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md](./issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md) |
| SPEC-IM5TwWbpq1rrgX81HVk3bN | Issue #181 Multi-Window Host — per-Window event & lifecycle contract | proposed | [issue-181-multi-window.md](./issue-181-multi-window.md) |
| SPEC-BkHWcVnZOw9qbxHXdfNUzH | Issue #182 Dock Tree / View Move / Layout Restore — reusable multi-window docking foundation | proposed | [issue-182-dock-tree.md](./issue-182-dock-tree.md) |
| SPEC-034QdNpdBY3gkeQraIPKwP | Issue #184 Typed multi-bus Port/Connection Graph IR contract (Migration M1) | approved | [issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md](./issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md) |
| SPEC-034QhNRt6XTihijW6B8pOy | Issue #185 Generic Probe: Node/Port observation capability, Off/Monitor/Capture states, bounded RT-safe PCM capture, time/identity metadata, non-RT delivery contract | approved | [issue-185-generic-probe-rt-capture.md](./issue-185-generic-probe-rt-capture.md) |
| SPEC-034QkwrLNvylpSOCZVs2bC | Issue #186 Parameter Control Events — capability, delivery and observability contract | proposed | [issue-186-param-control-events.md](./issue-186-param-control-events.md) |
| SPEC-034QwZG7xkTZz51niLYsTZ | Issue #187 Audio Input/Duplex Port: input enumeration/open, capture timestamp/clock-domain/frame-position contract, latency report, RT-safe handoff | approved | [issue-187-audio-input-port-capture-clock.md](./issue-187-audio-input-port-capture-clock.md) |
| SPEC-bg9fb3WGAfULNZMmr9jTMc | Issue #188 Timed MIDI — input, timestamp and plugin delivery contract | proposed | [issue-188-timed-midi.md](./issue-188-timed-midi.md) |
| SPEC-034Qf9Ze8jsyjHppvPkQup | Issue #189 Rational Beat Position + Variable PPQN (Migration M3) | approved | [issue-189-rational-beat-variable-ppqn-m3.md](./issue-189-rational-beat-variable-ppqn-m3.md) |
| SPEC-034Qg9hT0w1z3B9vJkLmNo | Issue #190 Generic Transport Span Scheduler / Applied Receipt / Clock-Plan Generation contract | approved | [issue-190-transport-span-scheduler.md](./issue-190-transport-span-scheduler.md) |
| SPEC-034QhrD80l0ZtpqNTqFvHa | Issue #191 PCM WAV encode/mux output backend | approved | [issue-191-pcm-wav-output-backend.md](./issue-191-pcm-wav-output-backend.md) |
| SPEC-034UBU16WQsh5QzC5ak17G | Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本 | approved | [issue-192-synvelis-driven-target-architecture-migration.md](./issue-192-synvelis-driven-target-architecture-migration.md) |
| SPEC-034UCI2A3NOXf96MhNKdmi | Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 仕様 | approved | [issue-193-migration-execution-plan.md](./issue-193-migration-execution-plan.md) |
| SPEC-034QfjN7Wt7AJBbhXeK60o | Issue #195 Typed Graph Audio Compiler・Prepared Plan・Buffer/PDC・Stateful RT Ownership・Plan Activation (Migration M2) | approved | [issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md](./issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md) |
| SPEC-hmdguU023dLBLtBKCM9kfH | Issue #196 M4 Typed Graph Plugin Adapter — multi-bus/stateful lifecycle/process offset precision | approved | [issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md](./issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md) |
| SPEC-jhXBM8Vy5edyp0fsJ6Nqbb | Issue #197 Generic Media Reader / Decoder・Prefetch・RT Playback Consumer 境界 | approved | [issue-197-media-reader-prefetch-rt-consumer.md](./issue-197-media-reader-prefetch-rt-consumer.md) |
| SPEC-Yr26o4c8Q0cCi56S9vjvsF | Issue #198 Stateful Sequential Offline Render・Frame/Preroll/PDC/Tail・Safe Output Receipt | approved | [issue-198-sequential-offline-render.md](./issue-198-sequential-offline-render.md) |
| SPEC-034QdDDqIQgpFoBr9E1Z1T | Issue #219 両 Consumer 互換契約・影響判定・証拠要件と Cross-Repo CI Gate 仕様 | proposed | [issue-219-consumer-compat-contract.md](./issue-219-consumer-compat-contract.md) |
| SPEC-034QoidM48Z7Yl3Xtw57h2 | Issue #233 media-reader prefetch fill/seek/pump contract clarifications + additive discard/reopen features | approved | [issue-233-media-reader-prefetch-contract-improvements.md](./issue-233-media-reader-prefetch-contract-improvements.md) |
| SPEC-Kn4byTEKolEb9bRq7eawMU | Issue #238 l1_proxy_e2e Windows flake — 壁時計 sleep 同期を有界 readiness/barrier 待機へ置換する | approved | [issue-238-l1-proxy-e2e-flake.md](./issue-238-l1-proxy-e2e-flake.md) |
| SPEC-034UdM9XszyXZB0hZ8wvnc | Issue #243 GPUI 実機 GUI 検証 — Windows 実窓ライフサイクル証拠（部分スコープ） | proposed | [issue-243-gpui-realwin-windows.md](./issue-243-gpui-realwin-windows.md) |
| SPEC-034V4nfv1RJmI8TqYPEWcY | Issue #243 — Windows 実 OS multiwindow smoke（部分スコープ・後続スライス） | approved | [issue-243-windows-multiwindow-smoke.md](./issue-243-windows-multiwindow-smoke.md) |
| SPEC-034Qx7sstYyddu4xbj5Q7d | Issue #244 media-reader prefetch contract residual wart fixes — boundary-first fault emission, reopen-failure dead generation, End commit-once | approved | [issue-244-media-reader-prefetch-contract-warts.md](./issue-244-media-reader-prefetch-contract-warts.md) |
| SPEC-rf253vst3midicanonpath001 | Issue #253 VST3 MIDI 正規経路 slice（IMidiMapping CC→param 配送 + plugin→host reporting） | approved | [issue-253-vst3-midi-canonical-path.md](./issue-253-vst3-midi-canonical-path.md) |
| SPEC-034UjQv2KPJWujHxiCffIK | Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合 | approved | [issue-257-driver-neutral-ui-contract.md](./issue-257-driver-neutral-ui-contract.md) |
| SPEC-8EJ4gM4rl1pb5GPij5Nm0l | Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計 | approved | [issue-275-observability-integration-design.md](./issue-275-observability-integration-design.md) |
| SPEC-034RoXidyTvNC4Cd998l35 | Issue #276 汎用 Logging Adapter(observability::logging) | approved | [issue-276-generic-logging-adapter.md](./issue-276-generic-logging-adapter.md) |
| SPEC-034Rp1Lwq5BGjy5ucoWJ4s | Issue #277 OBS-02 軽量Primitive・Observation Source型・登録/寿命契約 | approved | [issue-277-observability-core-source-contract.md](./issue-277-observability-core-source-contract.md) |
| SPEC-034UBnaDfEZMAipNtQoTeu | Issue #278 OBS-03 tracing_export slice 2 — EventRecord export + Correlation unlinked + W3C Trace Context | approved | [issue-278-observability-tracing-export-records.md](./issue-278-observability-tracing-export-records.md) |
| SPEC-034UpqR38pxxqJjjHLaFBG | Issue #278 OBS-03 tracing_export — ObservationBatch Exporter + FindingSink adapter slice | approved | [issue-278-observability-tracing-export.md](./issue-278-observability-tracing-export.md) |
| SPEC-034RToefGzxcXIANSfMf7U | Issue #279 OBS-04 Plugin Host既存4 CounterのSource登録・統合 | approved | [issue-279-plugin-host-counter-source-registration.md](./issue-279-plugin-host-counter-source-registration.md) |
| SPEC-ML6zMi4szNWY33LanXM0JL | Issue #280 OBS-05 RT固定長Record Bridge・Clock・欠落・Correlation統合 | approved | [issue-280-rt-record-bridge-clock-correlation.md](./issue-280-rt-record-bridge-clock-correlation.md) |
| SPEC-034Uyb75wxxSWrPfyvFEgh | Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計 | approved | [issue-284-io-02-structured-formats-serde-boundary.md](./issue-284-io-02-structured-formats-serde-boundary.md) |
| SPEC-7TCHVfCUzxmF7YWrv1aS4M | Issue #285 IO-03 Compression — 設計正本照合・契約試験棚卸し slice | approved | [issue-285-io-03-compression-contract-audit.md](./issue-285-io-03-compression-contract-audit.md) |
| SPEC-7BYANBc86b8qnIn77CPmoE | Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計 | approved | [issue-285-io-03-compression-serde-boundary.md](./issue-285-io-03-compression-serde-boundary.md) |
| SPEC-034UAmZTk0TIdPDXB52i1O | Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計 | approved | [issue-286-io-04-zip-tar-archive.md](./issue-286-io-04-zip-tar-archive.md) |
| SPEC-034UtEnxrMMCVpVRENHQ19 | Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計 | approved | [issue-288-io-06-cross-consumer-os-fs-security-gate.md](./issue-288-io-06-cross-consumer-os-fs-security-gate.md) |
| SPEC-034T2dgEk6l5QpkecJOJYo | Issue #289 OBS-07 Collector・Metrics・履歴・Snapshot（単一クレート） | approved | [issue-289-observability-collector-metrics.md](./issue-289-observability-collector-metrics.md) |
| SPEC-034UoMuIhsjFGaAbbUs8x0 | Issue #290 OBS-08 Profiling — CountingAllocator/AllocationScope（allocation-probe slice） | approved | [issue-290-observability-allocation-probe.md](./issue-290-observability-allocation-probe.md) |
| SPEC-f0JtG8wUPC8ziWo3EERrZ2 | Issue #290 OBS-08 RT Timing — 事前準備・選択的記録の実行時間観測 slice | approved | [issue-290-observability-rt-timing.md](./issue-290-observability-rt-timing.md) |
| SPEC-034UnxeRRp2vX35gnordF9 | Issue #291 OBS-09 Diagnostics・Evidence・Finding・Health・Report | approved | [issue-291-observability-diagnostics.md](./issue-291-observability-diagnostics.md) |
| SPEC-tBdATMZtqYxn8MbVgZquYA | Issue #292 OBS-10 Debug Inspection・Invariant・Session・Probe連携（observability 内完結 slice） | approved | [issue-292-observability-debug-inspection.md](./issue-292-observability-debug-inspection.md) |
| SPEC-4IC1B6yvvsvwdDR85TSYNI | Issue #293 OBS-11 Host統合 Config + ObservationRunner 生命周期 slice（observability 内完結） | approved | [issue-293-observability-integration-runner.md](./issue-293-observability-integration-runner.md) |
| SPEC-ko4Wl9fIvU1e2cmiVYFmwp | Issue #294 consumer-verify → ci-issue-tracker 配線 — 失敗の Issue 追跡を既存 tracker scope で実現 | proposed | [issue-294-consumer-verify-tracker-wiring.md](./issue-294-consumer-verify-tracker-wiring.md) |
| SPEC-034S9nR4tW7xZ2cF5hJ8kM | Issue #294 Cross-Consumer候補tuple Gate — consumer-verify.yml 証拠契約・UI lane 段階拡張・候補連携接続点 | proposed | [issue-294-cross-consumer-candidate-gate.md](./issue-294-cross-consumer-candidate-gate.md) |
| SPEC-034Ul9sCnKK1pa64jCWkdO | Issue #294 daily candidate-tuple trigger — consumer-verify tuple 判定の daily-ci 組込み（tuple-aware decide・証拠 artifact 回復・bounded retry） | proposed | [issue-294-daily-tuple-trigger.md](./issue-294-daily-tuple-trigger.md) |
| SPEC-7DGwosr0bGXXRsfeRfhe1v | Issue #294 GUI-CONTRACT lane 接続 — ruviform 候補 tree 上の boundary guard / Fake driver semantic を consumer-verify へ | approved | [issue-294-gui-contract-lane-wiring.md](./issue-294-gui-contract-lane-wiring.md) |
| SPEC-034RoR8tcezgggBLlNk7od | Issue #295 ruviform-io-file — StagedFile 型状態・公開契約・Mock Publish (IO-IMP-01) | approved | [issue-295-io-file-staged-publish-contract.md](./issue-295-io-file-staged-publish-contract.md) |
| SPEC-034RBEkXlCcpgzIaiikzso | Issue #296 ruviform-io-file — 実 OS Atomic Publish・Durability・Recovery (IO-IMP-02) | approved | [issue-296-io-file-os-backend.md](./issue-296-io-file-os-backend.md) |
| SPEC-034RTbxCxbL7YmxxkxrMmt | Issue #297 ruviform-io-structured — JSON/YAML/TOML Serde型付き Reader/Writer・Strict EOF (IO-IMP-03) | approved | [issue-297-io-structured-json-yaml-toml.md](./issue-297-io-structured-json-yaml-toml.md) |
| SPEC-0340TuOY8fzYFyCTHKGsWs | Issue #298 ruviform-io-structured — CSV Record Streaming・Dialect・Typed Serde (IO-IMP-04) | approved | [issue-298-io-structured-csv.md](./issue-298-io-structured-csv.md) |
| SPEC-034S6JgQsu8LENsvnVQR6J | Issue #299 ruviform-io-structured — XML Event Streaming・Namespace・Secure Default (IO-IMP-05) | approved | [issue-299-io-structured-xml.md](./issue-299-io-structured-xml.md) |
| SPEC-iRf6rX6sS5vJpc9b5uOxEk | Issue #300 ruviform-io-compression — Gzip・Raw Deflate Stream 変換・Finish・Checksum (IO-IMP-06) | approved | [issue-300-io-compression-gzip-deflate.md](./issue-300-io-compression-gzip-deflate.md) |
| SPEC-GvsblrwUriPJe6B9kCJ118 | Issue #301 ruviform-io-compression — Zstd Frame 境界・Window/Skippable・Verification Receipt (IO-IMP-07) | approved | [issue-301-io-compression-zstd.md](./issue-301-io-compression-zstd.md) |
| SPEC-034RT7qvjzmpx6kytmM8O0 | Issue #302 ruviform-io-archive — ZIP Indexed Entry Reader/Writer・CRC/中央Directory検証 (IO-IMP-08) | approved | [issue-302-io-archive-zip.md](./issue-302-io-archive-zip.md) |
| SPEC-h6TZICrhpaneHHENSs2SP8 | Issue #303 ruviform-io-archive — TAR Streaming Entry/Pending・PAX/GNU・EOF検証 (IO-IMP-09) | approved | [issue-303-io-archive-tar.md](./issue-303-io-archive-tar.md) |
| SPEC-6UnMiBIsE4whNsJmSW8SQX | Issue #304 ruviform-io-archive — Secure Extraction・Root Capability・Path Policy・Staging Recovery (IO-IMP-10) | approved | [issue-304-io-archive-extract.md](./issue-304-io-archive-extract.md) |
| SPEC-034UvWaV1HSJ4d8QLZI8fF | Issue #305 IO E2E slice — WAV→StagedFile Byte Golden・Fault fixture | approved | [issue-305-io-e2e-wav-golden-fault.md](./issue-305-io-e2e-wav-golden-fault.md) |
| SPEC-P6IzbXqAJ2cgiMm8llJIIO | Issue #305 Media I/O 旧 StagedOutput 完全削除・WAV→StagedFile 移行 (IO-IMP-11) | approved | [issue-305-media-io-staged-file-cutover.md](./issue-305-media-io-staged-file-cutover.md) |
| SPEC-lYVG6Gr07aTIBC8BSS0WKW | Issue #306 汎用 I/O 合成 E2E — StagedFile/Structured/Compression/Archive/WAV 境界試験 (IO-IMP-12) | approved | [issue-306-io-composition-e2e.md](./issue-306-io-composition-e2e.md) |
| SPEC-034UtxqBI7DX73fpCfjAok | Issue #306 IO E2E slice 2 — Fault/Fuzz fixture・OS/FS 実測・CI handoff | approved | [issue-306-io-e2e-fault-fuzz.md](./issue-306-io-e2e-fault-fuzz.md) |
| SPEC-akGFe19N27efIRTF1LTQdg | Issue #306 IO E2E slice 3 — IO MSRV lane・DAG/RT 境界 audit 証跡 | approved | [issue-306-io-msrv-dag-rt-audit.md](./issue-306-io-msrv-dag-rt-audit.md) |
| SPEC-P7W4Dz6p2EQIQNPclAQG2U | Issue #307 PluginNode attach 時 frame_base の非 RT 型付き検証（verified clock context） | approved | [issue-307-attach-frame-base-verified-context.md](./issue-307-attach-frame-base-verified-context.md) |
| SPEC-034Ui3cZpzwuwGdf2aHCCv | Issue #308 — Generation切替時のProbe再Bindingを全件整合・部分失敗可観測にする | approved | [issue-308-probe-rebind-atomic.md](./issue-308-probe-rebind-atomic.md) |
| SPEC-034Uwjri6KbgwOU8k9kjh0 | Issue #309 PROBE-TIME slice — 複数Probe時間アライメント fixture 検証 | approved | [issue-309-probe-time-align-fixture.md](./issue-309-probe-time-align-fixture.md) |
| SPEC-034Uj8oH3kcr44vuCWxYrC | Issue #309 — 複数Probeの汎用Frame/Clock/Epoch/Latency整列可能性・欠落比較契約（PROBE-TIME） | approved | [issue-309-probe-time-frame-clock-epoch-alignment.md](./issue-309-probe-time-frame-clock-epoch-alignment.md) |
| SPEC-034Rp1AEJDKiDTpPq3H4Yz | Issue #310 framework-neutral Browser GUI Test Harness (Playwright・Showcase実DOM検証) | approved | [issue-310-browser-gui-test-harness.md](./issue-310-browser-gui-test-harness.md) |
| SPEC-3S6R25rgxNv9VL9etjWsH9 | Issue #311 RF-GT-02 Rust↔TypeScript IPC Schema Conformance・Deterministic Record/Replay・Fault Matrix | approved | [issue-311-ipc-conformance-replay.md](./issue-311-ipc-conformance-replay.md) |
| SPEC-034S312gtUiTestSpecBB22 | Issue #312 test-only GUI host lifecycle・secure control adapter・fixture/clock/observation 仕様 | approved | [issue-312-test-gui-host-lifecycle.md](./issue-312-test-gui-host-lifecycle.md) |
| SPEC-034UkPTR1W1N5M0K3P01 | Issue #313 — Windows 実 OS SendInput ポインタ smoke（部分スコープ・後続スライス） | approved | [issue-313-windows-pointer-smoke.md](./issue-313-windows-pointer-smoke.md) |
| SPEC-034UjKJM3JWo1yfqUTlRka | Issue #313 — Windows 実 OS SendInput キーボード smoke と OS Capability Matrix（部分スコープ） | approved | [issue-313-windows-realos-smoke.md](./issue-313-windows-realos-smoke.md) |
| SPEC-034Ug8RuRab8De5zcQCjTB | Issue #314 slice 3 — GPUI/OS capture backend と能力の正直な配線 | proposed | [issue-314-golden-capture.md](./issue-314-golden-capture.md) |
| SPEC-0k38sOnYLZQngydDDLygHu | Issue #314 slice 1 — headless Pixel Golden Utility コア | proposed | [issue-314-golden-core.md](./issue-314-golden-core.md) |
| SPEC-2v1KGNSaMXtSGSr7czOhiR | Issue #314 slice 5 — Golden 環境正規化・fixture registry 結合・Scenario runner 統合・証拠 lane | proposed | [issue-314-golden-evidence.md](./issue-314-golden-evidence.md) |
| SPEC-034UevQgR6YDvj7j5Urcrc | Issue #314 slice 2 — Golden Utility と Scenario Contract の能力契約接続 | proposed | [issue-314-golden-scenario.md](./issue-314-golden-scenario.md) |
| SPEC-034V2gKIAk9L49Yej6EXsG | Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures (AudioLens/Synvelis) | proposed | [issue-314-semantic-oracle.md](./issue-314-semantic-oracle.md) |
| SPEC-LVITTCzVKzuRSkMzcYwUmc | Issue #326 ci.yml を workflow_dispatch 専用化 — PR/push trigger 撤去（Daily Full CI 移行・RF Stage 4） | approved | [issue-326-ci-manual-only.md](./issue-326-ci-manual-only.md) |
| SPEC-034RBJKNinKQbKocpnqqM9 | Issue #327 3リポジトリ横断CI計測Baseline・非回帰基準 仕様 | approved | [issue-327-cross-repo-ci-baseline.md](./issue-327-cross-repo-ci-baseline.md) |
| SPEC-BbANBkZi3v9ZHthJOmnDCy | Issue #328 CI-OPT-01 PR Concurrency 制御・旧 revision CI キャンセル | approved | [issue-328-ci-pr-concurrency.md](./issue-328-ci-pr-concurrency.md) |
| SPEC-FuYJCOjgHlGKlqRskoLQGq | Issue #329 CI-OPT-02 sccache 先行導入・3OS Rustコンパイルキャッシュ効果検証 | approved | [issue-329-sccache-compile-cache.md](./issue-329-sccache-compile-cache.md) |
| SPEC-034RBIrH4H2hFNw7gfmgDa | Issue #330 CI-OPT-03 UI npm ci 重複排除・TypeScript検証パイプライン集約 | approved | [issue-330-ui-npm-ci-dedup.md](./issue-330-ui-npm-ci-dedup.md) |
| SPEC-034RIZZEiovcu0sAxuAPkX | Issue #331 CI-OPT-04 check/clippy/test 重複評価・再コンパイル削減 | approved | [issue-331-check-clippy-test-dedup.md](./issue-331-check-clippy-test-dedup.md) |
| SPEC-x9aakFSwqMZzdqGoq2uyc3 | Issue #332 CI-OPT-05 Playwright Chromium 準備・ブラウザキャッシュと実行時間の最適化 | approved | [issue-332-playwright-browser-cache.md](./issue-332-playwright-browser-cache.md) |
| SPEC-nZdlFyflADPeFBthK8sEBB | Issue #333 CI-OPT-06 変更影響ベース検証 DAG・Fail-Closed Required Gate | approved | [issue-333-impact-based-ci.md](./issue-333-impact-based-ci.md) |
| SPEC-AJKrUcOGJ76vbIWIZ065rr | Issue #336 SinkDegraded への Clone derive 追加 | approved | [issue-336-sink-degraded-clone.md](./issue-336-sink-degraded-clone.md) |
| SPEC-034SmylRtM9crIdjdInbvd | Issue #360 l1_proxy_e2e heir flake — mint 窓内 exit を post-mint へ因果順序付け | approved | [issue-360-l1-proxy-e2e-heir-flake.md](./issue-360-l1-proxy-e2e-heir-flake.md) |
| SPEC-034Smu6fnzDQaoN4box3qk | Issue #366+#364 Full CI 化 Stage 1 — 常時全 lane 実行・ci-summary fail-closed 集約・io-structured lane 新設 | approved | [issue-366-full-ci-stage1.md](./issue-366-full-ci-stage1.md) |
| SPEC-ElrrvM1k7RbACtLgfzKaxs | Issue #367 共通 Daily Scheduler — `daily-ci.yml` 軽量判定 + `ci.yml` 条件付き dispatch | approved | [issue-367-daily-scheduler.md](./issue-367-daily-scheduler.md) |
| SPEC-4KMssIOpNuvh9jGgKKfp7R | Issue #368 CI failure Issue Tracker — workflow_run 完了監視で main×ci の未解決障害を 1 Issue に集約 | approved | [issue-368-ci-issue-tracker.md](./issue-368-ci-issue-tracker.md) |
| SPEC-9R6tNeU4aiEuUseUczer9o | Issue #368 Tracker reconcile 入口 — GITHUB_TOKEN dispatch 完了の通知 gap を default-branch 定期/手動 reconcile で埋める | approved | [issue-368-tracker-reconcile.md](./issue-368-tracker-reconcile.md) |
| SPEC-jXnvZdJj5kDVqJywcaAgiy | Issue #377 — live ViewLink の新規 QueryBinding epoch 契約整合 | approved | [issue-377-query-binding-epoch.md](./issue-377-query-binding-epoch.md) |
| SPEC-034UhhQycxrrbYudGthyQi | Issue #378 oracle late-create binding epoch 整合 | approved | [issue-378-oracle-late-create-epoch.md](./issue-378-oracle-late-create-epoch.md) |
| SPEC-034T6RWP3Vz7KWTjv6TILE | Driver-neutral GUI Architecture — GPUI標準Driver方針と3repo移行正本（Issue #384） | approved | [issue-384-driver-neutral-gui-architecture.md](./issue-384-driver-neutral-gui-architecture.md) |
| SPEC-034T6SaLC8Kz85jG9rbdsJ | DTCG Design Token Interpreter — backend-neutral ResolvedTheme/Theme Registry（Issue #385） | approved | [issue-385-dtcg-resolved-theme.md](./issue-385-dtcg-resolved-theme.md) |
| SPEC-0cG0IEArkb6RGWXH0jtA3e | Issue #386 Driver-neutral GUI Host 契約・GPUI 標準 Driver adapter・最小 Window fixture | proposed | [issue-386-driver-neutral-gui-host.md](./issue-386-driver-neutral-gui-host.md) |
| SPEC-034T6RWQv8Hm2Lj4pD9sT6 | Issue #387 Driver-neutral Component/View Specs・Headless Behavior・GPUI Controls/Viz Renderer | proposed | [issue-387-component-view-specs.md](./issue-387-component-view-specs.md) |
| SPEC-rf388DockWindowing01 | Issue #388 NativeSurfaceSlot slice 3 — Windows/VST3/L0 ancestor clip・keyboard/focus routing・re-dock/float 2-phase の実測仕様 | approved | [issue-388-native-dock-windowing-slice3.md](./issue-388-native-dock-windowing-slice3.md) |
| SPEC-zrIXGa6QONDm4o1DVVUwaz | Issue #388 Driver-neutral NativeSurfaceSlot 契約（早期 Gate 設計・契約実装） | approved | [issue-388-native-surface-slot.md](./issue-388-native-surface-slot.md) |
| SPEC-ynejrtwiz2jIJrf3DwFtAK | Issue #389 Driver-neutral PluginEditorSession（state / typed HostPort / L0-L1 接続設計） | approved | [issue-389-plugin-editor-session.md](./issue-389-plugin-editor-session.md) |
| SPEC-034UnwHc80Nd7iT5pKN2Eg | Issue #389 Plugin Editor Session hardening — bounded timeout・明示 degrade・GPU cause・DComp 正直記録 | approved | [issue-389-session-hardening.md](./issue-389-session-hardening.md) |
| SPEC-034T9TZQ7ztGDAPvFaz9IT | Issue #390 Driver Contract Lab — UI-DVR fixture registry・境界 Guard・GPUI なし Fake・Native Component Lab harness | proposed | [issue-390-driver-contract-lab.md](./issue-390-driver-contract-lab.md) |
| SPEC-xCsjb8OsiMswabSn30uVVs | Issue #404/#405 NativeSurfaceSlot host 境界修復（clip 解除/再投影 + owner window teardown 伝播） | draft | [issue-404-405-native-surface-host-boundary.md](./issue-404-405-native-surface-host-boundary.md) |
| SPEC-034UwbURsJwbo65nV81E1T | Issue #410 P2 — view command ledger retention 検証と projection correlation floor | approved | [issue-410-p2-view-ledger-correlation-hardening.md](./issue-410-p2-view-ledger-correlation-hardening.md) |
| SPEC-034UmGsNtxrfkDz2KxieUt | Issue #410 — view command ledger の bounded retention と correlation semantics | approved | [issue-410-view-command-ledger-bounded-retention-correlation-semantics.md](./issue-410-view-command-ledger-bounded-retention-correlation-semantics.md) |
| SPEC-Y6JWBZ3CHy6qvkcyHzSK5H | Issue #411 — ruviform-ui-test の cargo_metadata 推移依存 MSRV 1.89 不整合を修復する | approved | [issue-411-msrv-cargo-metadata-pin.md](./issue-411-msrv-cargo-metadata-pin.md) |
| SPEC-kR4mNxQwY8vLpTzH2jFd6A | Issue #414 — authoritative projection を controlled display へ到達させる | approved | [issue-414-projection-controlled-display.md](./issue-414-projection-controlled-display.md) |
| SPEC-034Uoh0rqjKI3fjlaRV6Fj | Issue #418 — DTCG token から ResolvedTheme 相当の静的 Rust 型を生成する codegen | approved | [issue-418-theme-codegen.md](./issue-418-theme-codegen.md) |
| SPEC-Yq7KpWx3RtNm8LbVd2HaJe | Issue #421 — Meter/TimeAxis GPUI 描画と Knob/Toggle/Meter/TimeAxis の Fake/GPUI parity | proposed | [issue-421-meter-timeaxis-gpui-parity.md](./issue-421-meter-timeaxis-gpui-parity.md) |
| SPEC-Qm7RvT2kXc9LpW4dHs8NfA | Issue #423 WebView 側写像・GPUI Dock 実窓検証・focus capability 精緻化（#386 残 DoD） | proposed | [issue-423-webview-dock-verify.md](./issue-423-webview-dock-verify.md) |
| SPEC-bO4Bjvhj9IWEhVO0Vo3ghA | Issue #426 GPUI lab binding・Story 2 scale × 2 theme fixture・Pixel Golden 境界・Consumer GPUI 型不使用検査 | proposed | [issue-426-gpui-lab-binding.md](./issue-426-gpui-lab-binding.md) |
| SPEC-Wb3KqT8nLf2RvXc6HmPd9Y | Issue #429 — Consumer 観測 seam・残 Fake/GPUI 検証の公開 | proposed | [issue-429-consumer-observation-seam.md](./issue-429-consumer-observation-seam.md) |
| SPEC-Rk4Vw9TqLb2XcJ7NhPd5Ya | Issue #431 WebView 実機 e2e・GPUI 実窓 close veto/Dock 検証・legacy trait Phase 2 判断 | proposed | [issue-431-webview-gpui-real-verify.md](./issue-431-webview-gpui-real-verify.md) |
| SPEC-Vn3Qa7Lk2RwXe9TdYc5MbHg | Issue #434 — Spectrogram ViewSpec kind（解決済み bounded payload） | proposed | [issue-434-spectrogram-kind.md](./issue-434-spectrogram-kind.md) |
| SPEC-Qm4Wd7Lz1RbNc8Xk3TgHs5 | Issue #436 — GpuiObserver の描画 plan（drawn）公開 | proposed | [issue-436-observer-drawn.md](./issue-436-observer-drawn.md) |
| SPEC-Vd7Qk2LmRt9WxYc4NbHa3F | Issue #448 — 読み取り専用 Table の行選択と command 起動の ViewSpec 契約 | approved | [issue-448-table-selection-command.md](./issue-448-table-selection-command.md) |
| SPEC-LOaygozld2Yp16xXhm1Yx5 | Issue #450 — Seek/Fader の canvas が実窓で 0 サイズになり描画されない問題の是正 | approved | [issue-450-control-canvas-size.md](./issue-450-control-canvas-size.md) |
| SPEC-034Ue3khxUUmbPdFCx4Mbh | Issue #451 FU-1b — Tab/Shift-Tab によるフォーカス巡回の配線 | approved | [issue-451-focus-next-wiring.md](./issue-451-focus-next-wiring.md) |
| SPEC-Kb7Tn3QwRx9LpVc2Hy5Ud8 | Issue #451 FU-1 — Tabs のキーボード切替 | approved | [issue-451-keyboard-tab-nav.md](./issue-451-keyboard-tab-nav.md) |
| SPEC-Gt5Rw2KqZm8LxNv3TcYb7H | Issue #451 — Tabs ヘッダの pointer 切替と ViewSpec descriptor の mount 後更新契約 | approved | [issue-451-tabs-pointer-descriptor.md](./issue-451-tabs-pointer-descriptor.md) |
| SPEC-7snPYyix2JlpoyXvP0zKwf | Issue #453 — 残 canvas 要素（Waveform/Meter/Spectrogram/TimeAxis/NativeSlot）のサイズ規則監査と是正 | approved | [issue-453-canvas-size-audit.md](./issue-453-canvas-size-audit.md) |
| SPEC-Rk5Wq2TnBx8LmYc3VdHa9E | Issue #456 — job を伴う command の receipt を job 終端まで route する | approved | [issue-456-job-command-receipt.md](./issue-456-job-command-receipt.md) |
| SPEC-034TXb9sMUqVVNvWbTUXkU | Issue #461 — spec サイズ比例の canvas painter の O(n²) paint を paint_layer で解消 | approved | [issue-461-canvas-paint-layer.md](./issue-461-canvas-paint-layer.md) |
| SPEC-034U1Mw5EZAfhDMxQUUpbi | Issue #461 残課題 — TimeAxis label の O(n²) text paint を canvas 内の共有 layer で解消 | approved | [issue-461-timeaxis-label-cost.md](./issue-461-timeaxis-label-cost.md) |
| SPEC-034U0yFY8Tr771Gyy5U4yy | Issue #463 — Spectrogram cell の intensity(alpha)が実窓で反映されない | approved | [issue-463-spectrogram-intensity.md](./issue-463-spectrogram-intensity.md) |
| SPEC-034Uc80WBRvRmOb7dt2yeU | Issue #464 — Consumer-neutral GUI Scenario Contract | approved | [issue-464-consumer-neutral-gui-scenario-contract.md](./issue-464-consumer-neutral-gui-scenario-contract.md) |
| SPEC-Ghn42uyLNbGNp71h5dfWyP | Issue #464 slice — Scenario Runner・Report 組み立て契約（ruviform-ui-test） | approved | [issue-464-scenario-runner.md](./issue-464-scenario-runner.md) |
| SPEC-HvasimHE9mJVFN1ay6D68D | Issue #465 Legacy WebView 基盤の段階的廃止 P0/P1 — 文書歴史化と棚卸し | proposed | [issue-465-legacy-webview-p0p1.md](./issue-465-legacy-webview-p0p1.md) |
| SPEC-034Uay7qlGIC70AEeDcn7u | Issue #468 RootedDir containment の canonical 前提の設計判断 | proposed | [issue-468-rooteddir-canonical-premise.md](./issue-468-rooteddir-canonical-premise.md) |
| SPEC-z3dzJJAH4RMnLrkQfhkR0m | Issue #479 — Seek/Fader/Knob のキーボード操作（共通 key 規約） | approved | [issue-479-keyboard-value-controls.md](./issue-479-keyboard-value-controls.md) |
| SPEC-034UiRf9jEzvH2FbreqPsA | Issue #480 — Tabs strip の FocusHandle unmount 解放 | approved | [issue-480-tab-focus-release.md](./issue-480-tab-focus-release.md) |
| SPEC-1ZxAql6tPbxsqXhvmGvphr | Issue #490 oracle late-create query epoch 整合 | approved | [issue-490-oracle-addquery-epoch.md](./issue-490-oracle-addquery-epoch.md) |
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
| SPEC-OSFxdu6zk2jDbA6edHhE38 | ruviform-io-file Rooted Read Capability — dirfd/handle 相対 traversal・FollowWithinRoot/NoFollow・pin/identity | approved | [rooted-read-capability.md](./rooted-read-capability.md) |
| SPEC-034JOU6EjinMh0rhxE6yM9 | RT 検証基盤（allocation 検出・thread-check・検証文書） | approved | [rt-verification-lane.md](./rt-verification-lane.md) |
| SPEC-X9YOosDAuB8PX2Qv31thuB | ruviform-dsp 新設（上位 DSP カーネル層 — prepare/process 分離・RT 適合） | approved | [ruviform-dsp.md](./ruviform-dsp.md) |
| SPEC-5eEaW8lLmoyVPgocJnooYD | 第 2 ターゲット設計（NLE 範囲・OFX 等映像プラグイン規格・広範色管理） | approved | [second-target-design.md](./second-target-design.md) |
| SPEC-034JQubfZRC7nRmTsYvLiq | SIMD dispatch 層（prepare 固定 + カーネル/サイズ別ルーティング + 本番精度契約） | approved | [simd-dispatch-layer.md](./simd-dispatch-layer.md) |
| SPEC-034JZVID00000000000010 | 動画エンジン（フレームパイプライン・A/V 同期・シーク） | approved | [video-engine.md](./video-engine.md) |
| SPEC-BXaMmK5e7X2mLZAG4xFeZv | ViewLink authoritative generation 採用 — backend attach応答epochのSDK seam | approved | [viewlink-authoritative-generation.md](./viewlink-authoritative-generation.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| SPEC-R6DgKRt3JJqwgUhZfDMOhr | AGENTS.md の #350 過剰縮小の是正 — `### 検証入口` 等の参照詳細を Progressive Disclosure で docs/ へ分離 | implemented | [agents-verification-entry-split.md](./agents-verification-entry-split.md) |
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
| SPEC-0kIy1SFw7ffSK1AAi4BXHu | Issue #400 借用 sealed readback を独立 decoder へ bounded に接続する共通API | implemented | [issue-400-borrowed-readback-decoder-bridge.md](./issue-400-borrowed-readback-decoder-bridge.md) |
| SPEC-034Tnjp8f4gexhi9BsQuq1 | Issue #409 Rooted capability の再 root 化と停止可能な bounded directory 列挙 | implemented | [issue-409-scoped-enumeration.md](./issue-409-scoped-enumeration.md) |
| SPEC-Lk3RvQ8mTxWn5HcZb2GdYu | Issue #435 — Label/Table ViewSpec kind と Fake/GPUI parity | implemented | [issue-435-label-table-kinds.md](./issue-435-label-table-kinds.md) |
| SPEC-Hs7Kd2Mv9QpXc4Nw1ZrTb6 | Issue #442 — GpuiObserver の描画 plan に Spectrogram / Label / Table を追加 | implemented | [issue-442-observer-drawn-spectrogram-label-table.md](./issue-442-observer-drawn-spectrogram-label-table.md) |
| SPEC-Qm7Lr2TxVb9KdWn4HcYe8S | Issue #443 — Tile から SpectrogramColumn への写し替え規則の上流契約 | implemented | [issue-443-tile-column-contract.md](./issue-443-tile-column-contract.md) |
| SPEC-034UetYOiYEThOr1lvUH8t | Issue #478 Fader/Knob 縦寸法契約 — h_full 兄弟押し出しのレイアウト判断 | implemented | [issue-478-fader-knob-height-contract.md](./issue-478-fader-knob-height-contract.md) |
| SPEC-034JceNZQvAawWwMH4ct91 | 時間型拡張（FrameRate/TimeBase 意味分離・Timecode 拡張・丸め誤差累積方針） | implemented | [time-type-extension-framerate-timecode-rounding.md](./time-type-extension-framerate-timecode-rounding.md) |
| SPEC-034N3JBtBP4dPi8ob9HwDV | video-proc RT 検証レーンへの assert_no_alloc 適用（Issue #126） | implemented | [video-proc-rt-verification-lane.md](./video-proc-rt-verification-lane.md) |
