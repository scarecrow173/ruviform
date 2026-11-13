# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/designs`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-OVERVIEW | System Design Overview | draft | [overview.md](./overview.md) |
| DESIGN-L8xeY9I5pmnb61GPOcn3D0 | AGENTS.md 旧版復元 + 参照詳細の Progressive Disclosure 分離方式 | approved | [agents-verification-entry-split.md](./agents-verification-entry-split.md) |
| DESIGN-034Kq7iQj2mt7VuR8YP2vq | ruviform-audio-analysis 設計（レベル/スペクトル/波形ピーク） | approved | [audio-analysis.md](./audio-analysis.md) |
| DESIGN-034JcC9QK924kDbu4WhibC | オーディオデバイス境界 trait と ruviform-audio-device PoC の詳細設計 | approved | [audio-device-boundary-and-poc.md](./audio-device-boundary-and-poc.md) |
| DESIGN-034JROdBjynllTNAVDZwaI | 音声エンジン設計 | approved | [audio-engine.md](./audio-engine.md) |
| DESIGN-034JOoWdEHXnb8G1QyMzCm | chunk 分割時の per-chunk timestamp 合成設計 | approved | [chunk-timestamp-design.md](./chunk-timestamp-design.md) |
| DESIGN-034WJELx3H0UQk20mwM76W | Issue #594 設計 — ci.yml MSRV lane の構成と ci-summary 宣言 lane 登録 | approved | [ci-msrv-daily-lane.md](./ci-msrv-daily-lane.md) |
| DESIGN-MSdMxhOZx7fphrbdNWWeNB | Issue #598 設計 — ui-gpui-verify-macos lane の構成と Native Build/Link 棚卸し | approved | [ci-os-l1-gpui-macos.md](./ci-os-l1-gpui-macos.md) |
| DESIGN-034LFikCuBIoaD4mWBEjIU | ruviform-video-proc 色変換カーネル設計（f32 linear working space + ICC 入出力経路） | approved | [color-conversion-kernels.md](./color-conversion-kernels.md) |
| DESIGN-034KScKaiUBXz4qKJZqM3h | ruviform-video-proc コンポジット/遷移カーネル設計（blend/affine/基本遷移初期セット） | approved | [composite-transition-kernels.md](./composite-transition-kernels.md) |
| DESIGN-034JZDNRML000000000003 | DenormalFlush ノード設計 | approved | [denormal-flush-node.md](./denormal-flush-node.md) |
| DESIGN-034IddXFCRKDIW01WXkSla | Rust 開発基盤の設計（最小ワークスペース・CIプロファイル・手動CI） | approved | [dev-foundation.md](./dev-foundation.md) |
| DESIGN-034JPvz9ieX8KoPbwDcvaX | dispatch 固定コスト分離の計測設計 | approved | [dispatched-overhead-method.md](./dispatched-overhead-method.md) |
| DESIGN-034JcqP2KiWsdes88fgvtt | DSP/SIMD 比較ベンチマーク PoC 詳細設計 | approved | [dsp-simd-poc.md](./dsp-simd-poc.md) |
| DESIGN-034KNBPbww1kvzVUF4Ab1H | ruviform-edit 設計（NLE 編集データモデル + 評価射影） | approved | [edit-data-model.md](./edit-data-model.md) |
| DESIGN-034Mclz5y96MrTg7R2BNCx | エンジン/利用側へのプラグイン統合（RT endpoint 所有分離 + グラフ接着） | approved | [engine-plugin-host-integration.md](./engine-plugin-host-integration.md) |
| DESIGN-034KQXr3w4dYgKO79dklG9 | Foundation/Observability 候補モジュールの最小 API・依存 DAG・新設時期の設計 | approved | [foundation-observability-minimal-api.md](./foundation-observability-minimal-api.md) |
| DESIGN-034Vu9v1geiqmFittsRAEQ | RF-GT-05 CI offscreen lanes design | approved | [gt-05-ci-offscreen-lanes-design.md](./gt-05-ci-offscreen-lanes-design.md) |
| DESIGN-xYI287PLIiNGr8mOrzVr7c | IO-06 Gate matrix 実走証拠の記録設計 | approved | [io-06-gate-run-design.md](./io-06-gate-run-design.md) |
| DESIGN-034MIeiBWXpLjwgA1yolIT | L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP + VST3 host-side bridge — Issue #114） | approved | [issue-114-real-adapter-l1-e2e.md](./issue-114-real-adapter-l1-e2e.md) |
| DESIGN-034N3J1C5vystrbJl69R1q | L1 GUI — プロセス境界越しの親ウィンドウ埋め込みの採否設計（Issue #115） | approved | [issue-115-l1-gui-cross-process-parenting.md](./issue-115-l1-gui-cross-process-parenting.md) |
| DESIGN-034NNxNMrrk6OoIGfX18rk | L1 supervisor（自動再起動 util 層）設計（Issue #116） | approved | [issue-116-supervisor.md](./issue-116-supervisor.md) |
| DESIGN-034Qa7Kf2NpXw9RtYbHc5d | L1 bundle 子プロセス（1 子プロセス N インスタンス）設計（Issue #117） | approved | [issue-117-bundle-child-process.md](./issue-117-bundle-child-process.md) |
| DESIGN-034NVCweXy0wECErcpEcv8 | video/NONRT プラグインの L1 経路設計（Issue #118） | approved | [issue-118-video-l1.md](./issue-118-video-l1.md) |
| DESIGN-034O9RrQutLFWyaHJGOF3G | Issue 121 L1 diagnostics | approved | [issue-121-l1-diagnostics.md](./issue-121-l1-diagnostics.md) |
| DESIGN-034N3EYKKosV8OzCP7Y9tY | 共通 platform/process/ipc 基盤の抽出評価（Issue #122） | approved | [issue-122-shared-platform-process-ipc-eval.md](./issue-122-shared-platform-process-ipc-eval.md) |
| DESIGN-034udQraUSrCxCnhZza67E | Issue #127 Linux 実配布方式 設計 — GPUI 標準 Driver の runtime 依存・Capability Matrix・deb/AppImage/Flatpak 適合条件 | draft | [issue-127-linux-dist-design.md](./issue-127-linux-dist-design.md) |
| DESIGN-034R150f64Num3r1cKxVb2zQ | Issue #150 f64 高精度オフライン数値・DSP API（ruviform-numeric 新設） | approved | [issue-150-f64-offline-numeric.md](./issue-150-f64-offline-numeric.md) |
| DESIGN-034R152f64Corr3l4t10nXy | Issue #152 f64 相互相関・NCC・GCC-PHAT・overlap 相関 API（ruviform-numeric `corr` モジュール） | approved | [issue-152-f64-correlation.md](./issue-152-f64-correlation.md) |
| DESIGN-034R153StftBand5Xq9wPzLm | Issue #153 汎用 f64 帯域演算・STFT・スペクトル統計（ruviform-numeric 拡張） | approved | [issue-153-f64-stft-bands-spectral.md](./issue-153-f64-stft-bands-spectral.md) |
| DESIGN-WVN475nNdieOE0fyOxkhf | GUI 非依存 CLI/headless 実行基盤設計（Issue #155） | approved | [issue-155-headless-cli-foundation.md](./issue-155-headless-cli-foundation.md) |
| DESIGN-034P8EHh2IlZd14yNSOreO | Issue #156 再利用可能デスクトップ GUI ホスト基盤 | draft | [issue-156-gui-host-foundation.md](./issue-156-gui-host-foundation.md) |
| DESIGN-034R157VizPrimitivesWk4 | Issue #157 波形・スペクトログラム・タイムラインの汎用インタラクティブ表示基盤 | draft | [issue-157-viz-primitives.md](./issue-157-viz-primitives.md) |
| DESIGN-BdiZY3GtD44RrVDtpLFcFx | Issue #181 Multi-Window Host — per-Window event & lifecycle contract | approved | [issue-181-multi-window.md](./issue-181-multi-window.md) |
| DESIGN-iYLA7E6C3gMZXNqMyIqMw5 | Issue #182 Dock Tree / View Move / Layout Restore — reusable multi-window docking foundation | approved | [issue-182-dock-tree.md](./issue-182-dock-tree.md) |
| DESIGN-034QdReVJaFxlQTjmr5oy5 | Issue #184 Typed Graph IR: Ports, Buses, Connections, Validation, Revisions (M1) | approved | [issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md](./issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md) |
| DESIGN-034QhWjbVG7vgYciUpfE2b | Issue #185 Generic Probe: typed-runtime observation points, Off/Monitor/Capture execution, chunk-pool RT handoff, non-RT probe broker/collector | approved | [issue-185-generic-probe-typed-runtime-observation-rt-capture.md](./issue-185-generic-probe-typed-runtime-observation-rt-capture.md) |
| DESIGN-034QkB2r7d3Qh9nZ3zJ3rP | Issue #186 Parameter Control Events — detailed contract design | approved | [issue-186-param-control-events-detailed.md](./issue-186-param-control-events-detailed.md) |
| DESIGN-034QkAdKcICxz0yVcZ8PPZ | Issue #186 Parameter Control Events — overview | approved | [issue-186-param-control-events.md](./issue-186-param-control-events.md) |
| DESIGN-034Qx35fKutOnHPc9IRWC7 | Issue #187 Audio Input Port: input/duplex contract, capture timestamp/clock-domain/latency, RT-safe handoff PoC | approved | [issue-187-audio-input-port-capture-clock.md](./issue-187-audio-input-port-capture-clock.md) |
| DESIGN-Gy2lun1o6N1NDRI7tfSArM | Issue #188 Timed MIDI — detailed API and contracts | approved | [issue-188-timed-midi-detailed.md](./issue-188-timed-midi-detailed.md) |
| DESIGN-lnTibw26AboiFgtq2O9IsE | Issue #188 Timed MIDI — overview | approved | [issue-188-timed-midi.md](./issue-188-timed-midi.md) |
| DESIGN-034QfaKklvNoBLtP5JVkOj | Issue #189 M3 Rational Beat / Variable PPQN media-core design | approved | [issue-189-rational-beat-variable-ppqn-tempomap.md](./issue-189-rational-beat-variable-ppqn-tempomap.md) |
| DESIGN-034Qh2kLmNoPqRsTuVwX | Issue #190 Transport Span Scheduler — 実装設計 | approved | [issue-190-transport-span-scheduler.md](./issue-190-transport-span-scheduler.md) |
| DESIGN-034Qhua2OtuBq26hwS8LpX | Issue #191 PCM WAV output backend design | approved | [issue-191-pcm-wav-output-backend.md](./issue-191-pcm-wav-output-backend.md) |
| DESIGN-034UBX7Lbr3m4u4ZGzQpHA | Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本 | approved | [issue-192-synvelis-driven-target-architecture-migration.md](./issue-192-synvelis-driven-target-architecture-migration.md) |
| DESIGN-034WYfShwch3ZurjTQp3Sp | Issue #192 TA Invariant Map — 監査設計 | approved | [issue-192-ta-invariant-map-design.md](./issue-192-ta-invariant-map-design.md) |
| DESIGN-034UCMjL9ps9ffJkQqwDlR | Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 設計正本 | approved | [issue-193-migration-execution-plan.md](./issue-193-migration-execution-plan.md) |
| DESIGN-034QfjYdt3o2a4lijg4lwt | Issue #195 M2 Typed Graph Compiler・Bounded Buffers/PDC・RT Runtime・Stateful Ownership・Activation | approved | [issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md](./issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md) |
| DESIGN-C1EIkNXtQvzacfPU0a2bhv | Issue #196 M4 Typed Graph Plugin Adapter — 実装設計 | approved | [issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md](./issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md) |
| DESIGN-fl6bBJvoJRiJHSkk73mskx | Issue #197 Generic Media Reader / Prefetch / RT Consumer — structural design | approved | [issue-197-media-reader-prefetch-rt-consumer.md](./issue-197-media-reader-prefetch-rt-consumer.md) |
| DESIGN-aRsJILdLBlEulHcYpfH4eb | Issue #198 stateful sequential offline render — PreparedRender, sequential driver, staged output lifecycle | approved | [issue-198-sequential-offline-render.md](./issue-198-sequential-offline-render.md) |
| DESIGN-034QdHuPP1Uvvlkvbh5awX | Issue #219 Cross-Repo CI Gate 設計・credential 一次設定・証拠記録 runbook | approved | [issue-219-cross-repo-ci-gate-credential-runbook.md](./issue-219-cross-repo-ci-gate-credential-runbook.md) |
| DESIGN-034UF2Q9OH59VxiQZUfUgN | Issue #219 Cross-Repo 影響判定・CI Gate 運用設計 — Consumer 棚卸し・影響分類テンプレート・候補 Gate・切り分け/保留復旧・M0-M7 証拠可視化 | draft | [issue-219-cross-repo-impact-gate-operations.md](./issue-219-cross-repo-impact-gate-operations.md) |
| DESIGN-034QolOnuF7kf2drxSsvvH | Issue #233 prefetch contract improvements — discard_pending drain, seek discard flag, reopen_at structural design | approved | [issue-233-media-reader-prefetch-improvements.md](./issue-233-media-reader-prefetch-improvements.md) |
| DESIGN-034UdN3RWXD8Kk4elhwAtR | Issue #243 GPUI 実機 GUI 検証 Windows 実窓ライフサイクル証拠 — 設計 | approved | [issue-243-gpui-realwin-windows.md](./issue-243-gpui-realwin-windows.md) |
| DESIGN-034WZGgweiRw2qZcHmfIbr | Issue #243 — Window/Job 不変条件 Headless 検証設計 | approved | [issue-243-window-job-headless-design.md](./issue-243-window-job-headless-design.md) |
| DESIGN-034V4sf4ndYXHxJt8IBeXG | Issue #243 — Windows 実 OS multiwindow smoke 設計（部分スコープ・後続スライス） | approved | [issue-243-windows-multiwindow-smoke.md](./issue-243-windows-multiwindow-smoke.md) |
| DESIGN-034QxcnAfFWswZmwSKPbKH | Issue #244 prefetch contract wart fixes — boundary-first emission, dead-generation sequence, End commit-once, seeded-fault coverage | approved | [issue-244-media-reader-prefetch-warts.md](./issue-244-media-reader-prefetch-warts.md) |
| DESIGN-cJ0I0j6o66AAfUm2eE5q8y | Issue #251 Stage W closeout 設計 — 実機 lane 拡張と証拠 matrix | approved | [issue-251-stage-w-real-device-closeout-design.md](./issue-251-stage-w-real-device-closeout-design.md) |
| DESIGN-034Vj33PXgAsLoeNPU9c8f | Issue #253 plugin→host report の host-graph consumer 配線 | approved | [issue-253-hostgraph-reports.md](./issue-253-hostgraph-reports.md) |
| DESIGN-rf253osmidibackend001 | Issue #253 OS MIDI input backend design（WinMM adapter + Unsupported 境界） | approved | [issue-253-os-midi-backend.md](./issue-253-os-midi-backend.md) |
| DESIGN-034VEQoHhDL68K8qLl8w8r | Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane scaffold | approved | [issue-253-unsupported-transparency-design.md](./issue-253-unsupported-transparency-design.md) |
| DSN-rf253vst3midicanonpath001 | Issue #253 VST3 MIDI 正規経路 slice 設計（IMidiMapping snapshot + plugin→host reporting） | approved | [issue-253-vst3-midi-canonical-path.md](./issue-253-vst3-midi-canonical-path.md) |
| DESIGN-034UjQBuwtfaWhLg6ZZMBn | Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合設計 | approved | [issue-257-driver-neutral-ui-contract.md](./issue-257-driver-neutral-ui-contract.md) |
| DESIGN-034WZCcSruHEmlEMFX1Mlu | Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付け設計 | approved | [issue-275-obs-owner-map-design.md](./issue-275-obs-owner-map-design.md) |
| DESIGN-hXhz1NTWfHfPzPb5P4A5ff | Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計 | approved | [issue-275-observability-integration-design.md](./issue-275-observability-integration-design.md) |
| DESIGN-034RoXjgYbKKZ3ZSazkSGi | Issue #276 汎用 Logging Adapter 設計 | approved | [issue-276-generic-logging-adapter.md](./issue-276-generic-logging-adapter.md) |
| DESIGN-034Rp1Um3ZXDWfRx8pCmuw | Issue #277 OBS-02 軽量Primitive・Observation Source契約: Gauge/Histogram・Descriptor・SourceId/Generation・Registrar/Lease・寿命 | approved | [issue-277-observability-core-source-contract.md](./issue-277-observability-core-source-contract.md) |
| DESIGN-034UBq1SnAHmlvoFnohCoS | Issue #278 OBS-03 tracing_export slice 2: EventRecord export + Correlation unlinked + W3C Trace Context | approved | [issue-278-observability-tracing-export-records.md](./issue-278-observability-tracing-export-records.md) |
| DESIGN-034UprcXe8LevfKFoKevlN | Issue #278 OBS-03 tracing_export: ObservationBatch Exporter + FindingSink adapter | approved | [issue-278-observability-tracing-export.md](./issue-278-observability-tracing-export.md) |
| DESIGN-034RTpdouoLLSINNczwa3c | Issue #279 OBS-04 Plugin Host 4 CounterのSource登録: Registrar注入・Descriptor・Lease寿命 | approved | [issue-279-plugin-host-counter-source-registration.md](./issue-279-plugin-host-counter-source-registration.md) |
| DESIGN-034JcpvK9cZEfkeEbBLBoz | Issue #28 UI 技術検証 PoC の設計（使い捨て検証ハーネスと計測・評価方式） | approved | [issue-28-ui-poc.md](./issue-28-ui-poc.md) |
| DESIGN-fhvRxnPJKvnhRAlg7TbeKm | Issue #280 OBS-05 RT Record Bridge・ClockContext・欠落envelope・CorrelationBook設計 | approved | [issue-280-rt-record-bridge-clock-correlation.md](./issue-280-rt-record-bridge-clock-correlation.md) |
| DESIGN-034UydimCfynTXvl5UJT6x | Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計 | approved | [issue-284-io-02-structured-formats-serde-boundary.md](./issue-284-io-02-structured-formats-serde-boundary.md) |
| DESIGN-8C8VqGGcRx0CT4AfwBMnLR | Issue #285 IO-03 Compression — 設計正本照合・契約試験棚卸し 設計 | approved | [issue-285-io-03-compression-contract-audit.md](./issue-285-io-03-compression-contract-audit.md) |
| DESIGN-TUrjxcJLe7rvYr6up8mhf3 | Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計 | approved | [issue-285-io-03-compression-serde-boundary.md](./issue-285-io-03-compression-serde-boundary.md) |
| DESIGN-034UApwItFwQ4wBpfOs2EC | Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計 | approved | [issue-286-io-04-zip-tar-archive.md](./issue-286-io-04-zip-tar-archive.md) |
| DESIGN-034UtFFzzM3wAegGBSycbJ | Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計 | approved | [issue-288-io-06-cross-consumer-os-fs-security-gate.md](./issue-288-io-06-cross-consumer-os-fs-security-gate.md) |
| DESIGN-034T2dhkl03BuqPOGmNlmc | Issue #289 OBS-07 Collector/Metrics/Snapshot: 単一Poll Owner・Baseline/Discontinuity・有界履歴・Health | approved | [issue-289-observability-collector-metrics.md](./issue-289-observability-collector-metrics.md) |
| DESIGN-034UoQ3mxByzCnjgdAGpe1 | Issue #290 OBS-08 Profiling — allocation-probe 設計（CountingAllocator/AllocationScope） | approved | [issue-290-observability-allocation-probe.md](./issue-290-observability-allocation-probe.md) |
| DESIGN-ZAfKHGovgBPKLRo8KY7Ss7 | Issue #290 OBS-08 RT Timing — observability 内完結 slice | approved | [issue-290-observability-rt-timing.md](./issue-290-observability-rt-timing.md) |
| DESIGN-034Unxw6Jlg98IypxJkHfQ | Issue #291 OBS-09 Diagnostics: Evidence/Finding/Health/Report — 型で分ける事実と推定 | approved | [issue-291-observability-diagnostics.md](./issue-291-observability-diagnostics.md) |
| DESIGN-koNZAEijupozhje6vYRpDO | Issue #292 OBS-10 Debug Inspection・Invariant・Session・Probe連携 — observability 内完結 slice | approved | [issue-292-observability-debug-inspection.md](./issue-292-observability-debug-inspection.md) |
| DESIGN-DQg7hyyzqx49OBMHYx8lv8 | Issue #293 OBS-11 ObservabilityConfig + ObservationRunner 設計 | approved | [issue-293-observability-integration-runner.md](./issue-293-observability-integration-runner.md) |
| DESIGN-MxFLFDKeJa955vgMTzSni8 | Issue #294 consumer-verify → ci-issue-tracker 配線設計 — 既存 scope 機構の再利用・scope 別 job・reconcile 全 scope 化 | approved | [issue-294-consumer-verify-tracker-wiring.md](./issue-294-consumer-verify-tracker-wiring.md) |
| DESIGN-034S9rV6yB3dG7jL0nQ4sU | Issue #294 Cross-Consumer候補tuple Gate 設計 — 証拠契約・解決SHA捕捉・UI lane 段階接続・task 接続点 | proposed | [issue-294-cross-consumer-candidate-gate.md](./issue-294-cross-consumer-candidate-gate.md) |
| DESIGN-034UlaEev2MQ6gsnrNqias | Issue #294 daily candidate-tuple trigger 設計 — tuple-aware decide・証拠 artifact 回復・daily-ci job | approved | [issue-294-daily-tuple-trigger.md](./issue-294-daily-tuple-trigger.md) |
| DESIGN-RVQaQUltza40Kb1HlAt48X | Issue #294 GUI-CONTRACT lane 接続設計 — 候補 tree 取得・task probe・step/lane 分離 | approved | [issue-294-gui-contract-lane-wiring.md](./issue-294-gui-contract-lane-wiring.md) |
| DESIGN-034RoR9SDtSG2iiZbvNGKT | Issue #295 ruviform-io-file — StagedFile type-state・PublishOptions・Receipt/Failure・PlatformFileOps/Mock 設計 | approved | [issue-295-io-file-staged-publish.md](./issue-295-io-file-staged-publish.md) |
| DESIGN-034RBFdth7UW7A6IS10Ha3 | Issue #296 ruviform-io-file — 実 OS PlatformFileOps アダプタ設計 (IO-IMP-02) | approved | [issue-296-io-file-os-backend.md](./issue-296-io-file-os-backend.md) |
| DESIGN-034RTd0bYqWgcxXWa7uVPE | Issue #297 ruviform-io-structured — 共通契約・Format別API・Backend選定設計 (IO-IMP-03) | approved | [issue-297-io-structured-json-yaml-toml.md](./issue-297-io-structured-json-yaml-toml.md) |
| DESIGN-034k6xAsDaYM7wuYe0CKOF | Issue #298 ruviform-io-structured — CSV Module 設計（IO-IMP-04） | approved | [issue-298-io-structured-csv.md](./issue-298-io-structured-csv.md) |
| DESIGN-034S6K6h23VjXK8NquLQMx | Issue #299 ruviform-io-structured — XML Module 設計（IO-IMP-05） | approved | [issue-299-io-structured-xml.md](./issue-299-io-structured-xml.md) |
| DESIGN-eGBdnzbpDjWwMwTEldA9gI | Issue #300 ruviform-io-compression — Gzip・Raw Deflate 借用 Stream 設計 (IO-IMP-06) | approved | [issue-300-io-compression-gzip-deflate.md](./issue-300-io-compression-gzip-deflate.md) |
| DESIGN-K3VAKLOqVu5SxMRUaZCAso | Issue #301 ruviform-io-compression — Zstd Frame 借用 Stream 設計 (IO-IMP-07) | approved | [issue-301-io-compression-zstd.md](./issue-301-io-compression-zstd.md) |
| DESIGN-034RT7CvRTe3b8jPTFBHtq | Issue #302 ruviform-io-archive — Archive 共通型・ZIP Indexed Reader/Writer・検証パイプライン設計 | approved | [issue-302-io-archive-zip.md](./issue-302-io-archive-zip.md) |
| DESIGN-HR0Ud1VXoIpQCfUcosM6ib | Issue #303 ruviform-io-archive — TAR Streaming Reader/Pending・PAX/GNU・Writer 設計 | approved | [issue-303-io-archive-tar.md](./issue-303-io-archive-tar.md) |
| DESIGN-mH3gJLoQj3rdU7xRXX8eZD | Issue #304 ruviform-io-archive — Secure Extraction・Staging/Recovery・OS 別 Root Capability 設計 | approved | [issue-304-io-archive-extract.md](./issue-304-io-archive-extract.md) |
| DESIGN-034UvWqyRcaTQap2SH5FfT | Issue #305 IO E2E slice — WAV Byte Golden・Fault fixture 設計 | approved | [issue-305-io-e2e-wav-golden-fault.md](./issue-305-io-e2e-wav-golden-fault.md) |
| DESIGN-eVPQpm7tyMUS2gxitU5KtN | Issue #305 Media I/O 旧 StagedOutput 撤去・StagedFile 移行設計 | approved | [issue-305-media-io-staged-file-cutover.md](./issue-305-media-io-staged-file-cutover.md) |
| DESIGN-034WXOSFa35dv10W4LdOri | Issue #306 G07/G08 fault seam・corpus replay 設計 | approved | [issue-306-fault-seam-fuzz-design.md](./issue-306-fault-seam-fuzz-design.md) |
| DESIGN-ODmc6p5Cp7Kbq0u35OrAiR | Issue #306 汎用 I/O 合成 E2E — 試験専用クレート構成・境界試験設計 | approved | [issue-306-io-composition-e2e.md](./issue-306-io-composition-e2e.md) |
| DESIGN-fH3jK9mP2qW8rT5yU7iO4a | Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完 設計 | approved | [issue-306-io-e2e-cases.md](./issue-306-io-e2e-cases.md) |
| DESIGN-034UtzREeBZLceakda7vIS | Issue #306 IO E2E slice 2 — Fault/Fuzz fixture 設計 | approved | [issue-306-io-e2e-fault-fuzz.md](./issue-306-io-e2e-fault-fuzz.md) |
| DESIGN-CTGZSygUhyfXwQSdShQuap | Issue #306 IO E2E slice 3 — IO MSRV lane・DAG/RT 境界 audit 設計 | approved | [issue-306-io-msrv-dag-rt-audit.md](./issue-306-io-msrv-dag-rt-audit.md) |
| DESIGN-dHUQ40Hk50tvziqjx7Kewj | Issue #307 attach 時 frame_base 検証 context — 実装設計 | approved | [issue-307-attach-frame-base-verified-context.md](./issue-307-attach-frame-base-verified-context.md) |
| DESIGN-034Ui3n3nIZNFJKLyIv0Uf | Issue #308 — Probe再Bindingのstaged all-or-nothing設計 | approved | [issue-308-probe-rebind-atomic.md](./issue-308-probe-rebind-atomic.md) |
| DESIGN-034UwjzYYEPPjIvuXPl28f | Issue #309 PROBE-TIME slice — 複数Probeアライメント fixture 設計 | approved | [issue-309-probe-time-align-fixture.md](./issue-309-probe-time-align-fixture.md) |
| DESIGN-034Uja29JLpwyE6xff7quc | Issue #309 — Probe時間コンテキストと観測整列契約の設計（PROBE-TIME） | approved | [issue-309-probe-time-context-and-alignment.md](./issue-309-probe-time-context-and-alignment.md) |
| DESIGN-034Rp1KqXNzTDTKRXTbqHx | Issue #310 framework-neutral Browser GUI Test Harness (Playwright・Showcase実DOM検証) | approved | [issue-310-browser-gui-test-harness.md](./issue-310-browser-gui-test-harness.md) |
| DESIGN-wrF61tPcejIc31wMBYJHok | Issue #311 RF-GT-02 ui/guitest — IPC conformance fixture・deterministic record/replay・fault matrix 設計 | approved | [issue-311-ipc-conformance-replay.md](./issue-311-ipc-conformance-replay.md) |
| DESIGN-034S312gtUiTestDsnDD44 | Issue #312 ruviform-ui-test — test-only GUI host lifecycle・secure control adapter 設計 | approved | [issue-312-test-gui-host-lifecycle.md](./issue-312-test-gui-host-lifecycle.md) |
| DESIGN-34VOSINPDRV4REAL0SIN2 | Issue #313 — `RealOsPointerOrKeyboard` capability 設計（OS-input port on `RealOsGuiTest`） | approved | [issue-313-real-os-input-driver-design.md](./issue-313-real-os-input-driver-design.md) |
| DESIGN-34WIND0WEDTESTI0S2DE | Issue #313 — windowed `TestIo` 設計（neutral 語彙統一 + GPUI adapter） | approved | [issue-313-windowed-testio-design.md](./issue-313-windowed-testio-design.md) |
| DESIGN-034UkPTR1W1N5M0K3P02 | Issue #313 — Windows 実 OS SendInput ポインタ smoke 設計（部分スコープ・後続スライス） | approved | [issue-313-windows-pointer-smoke.md](./issue-313-windows-pointer-smoke.md) |
| DESIGN-034UjLB6Yn2hveYiujXFY0 | Issue #313 — Windows 実 OS SendInput キーボード smoke 設計（部分スコープ） | approved | [issue-313-windows-realos-smoke.md](./issue-313-windows-realos-smoke.md) |
| DESIGN-034Ug8WYFJ37a8hKlmcQaW | Issue #314 slice 3 — GPUI/OS capture backend の設計 | approved | [issue-314-golden-capture.md](./issue-314-golden-capture.md) |
| DESIGN-GKjcGBxFK0ZtzxCEKsh1e7 | Issue #314 slice 1 — headless Pixel Golden Utility コアの設計 | approved | [issue-314-golden-core.md](./issue-314-golden-core.md) |
| DESIGN-L3YSlsSU8u5EdRMDfpHhy6 | Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane の設計 | approved | [issue-314-golden-evidence.md](./issue-314-golden-evidence.md) |
| DESIGN-034UewQlz8GZ6RlexpCiXW | Issue #314 slice 2 — Golden Utility × Scenario Contract 能力契約接続の設計 | approved | [issue-314-golden-scenario.md](./issue-314-golden-scenario.md) |
| DESIGN-034Vg9PXhXAWj20QVdvHWj | Issue #314 slice 4 follow-up — real-scenario lane + matrix 詳細設計 | approved | [issue-314-real-scenario-lane.md](./issue-314-real-scenario-lane.md) |
| DESIGN-314rwBn7xK2pQwE5rTyUi | Issue #314 slice 4 residual — real-window adapter 詳細設計 | approved | [issue-314-real-window-adapter.md](./issue-314-real-window-adapter.md) |
| DESIGN-034V2ho95TYYvIaJsGmRSx | Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures の設計 | approved | [issue-314-semantic-oracle.md](./issue-314-semantic-oracle.md) |
| DESIGN-bgU7RV8JrPVEY31dlXpJCG | Issue #326 ci.yml dispatch 専用化 設計 — trigger 撤去・concurrency 簡素化・trigger 契約テスト | approved | [issue-326-ci-manual-only.md](./issue-326-ci-manual-only.md) |
| DESIGN-034RBJLryk98hb4Yyq5wYG | Issue #327 CI計測Baseline 計測契約・分類・比較手順 | approved | [issue-327-cross-repo-ci-baseline.md](./issue-327-cross-repo-ci-baseline.md) |
| DESIGN-yMdXfkOVFpXfZGKfVH7LjG | Issue #328 CI PR concurrency 設計 — group 解決・cancel 意味論・required checks 保護 | approved | [issue-328-ci-pr-concurrency.md](./issue-328-ci-pr-concurrency.md) |
| DESIGN-mcItTpQPSlr7FZ7BBqChaW | Issue #329 sccache 設計 — step 構成・env 解決・cache 正しさ・監査・無効化 | approved | [issue-329-sccache-compile-cache.md](./issue-329-sccache-compile-cache.md) |
| DESIGN-034RBIy0yxPWYhXdXUlsLi | Issue #330 CI-OPT-03 UI npm install 集約・mise task freshness 設計 | approved | [issue-330-ui-npm-ci-dedup.md](./issue-330-ui-npm-ci-dedup.md) |
| DESIGN-034RJ00iyqN3oWCf99pDXu | Issue #331 設計 — check-ci を lint-ci へ統合する等価性証明と適用 | approved | [issue-331-check-clippy-test-dedup.md](./issue-331-check-clippy-test-dedup.md) |
| DESIGN-EJ86GHTwsKgyexnCdTpSJ5 | Issue #332 CI-OPT-05 Playwright Chromium cache・準備分離設計 | approved | [issue-332-playwright-browser-cache.md](./issue-332-playwright-browser-cache.md) |
| DESIGN-oq5cpmBZrL3fUcNnuUzPxg | Issue #333 CI-OPT-06 Impact Classifier・集約 Required Gate・段階移行設計 | approved | [issue-333-impact-based-ci.md](./issue-333-impact-based-ci.md) |
| DESIGN-elUvrfyRcsMrmrESYqsxDn | Issue #336 SinkDegraded Clone 設計 | approved | [issue-336-sink-degraded-clone.md](./issue-336-sink-degraded-clone.md) |
| DESIGN-034RP4B7OViSh6O4YCu1rz | Issue #346 CI-OPT-06 選択実行有効化・ruleset required context 移行 runbook | approved | [issue-346-ruleset-migration-runbook.md](./issue-346-ruleset-migration-runbook.md) |
| DESIGN-034SmBuqxSVw74rlBganJD | Issue #360 設計 — heir 系テストの exit を post-mint へ message 順序で固定 | approved | [issue-360-heir-exit-post-mint-ordering.md](./issue-360-heir-exit-post-mint-ordering.md) |
| DESIGN-034SmuehTqDxp5K68M5dSG | Issue #366+#364 Full CI Stage 1 設計 — 選択実行撤去・3OS 常時実行・ci-summary fail-closed・expected_sha 契約 | approved | [issue-366-full-ci-stage1.md](./issue-366-full-ci-stage1.md) |
| DESIGN-LKKyu5XTONFK84RN45ZdM2 | Issue #367 共通 Daily Scheduler 設計 — 履歴駆動判定・重複抑止・bounded retry・dispatch SHA race 対策 | approved | [issue-367-daily-scheduler.md](./issue-367-daily-scheduler.md) |
| DESIGN-UGm4KjCwx7pEetENeTzIwB | Issue #368 CI failure Issue Tracker 設計 — 有効 run 判定・1 Issue 集約・完了順逆転・coverage close 条件 | approved | [issue-368-ci-issue-tracker.md](./issue-368-ci-issue-tracker.md) |
| DESIGN-StMgstUBs2Df2UPm1oy2GF | Issue #368 Tracker reconcile 設計 — event 非依存の同一判定器入口・別 workflow schedule/dispatch・mode 分離 | approved | [issue-368-tracker-reconcile.md](./issue-368-tracker-reconcile.md) |
| DESIGN-imYFBiewK6Cdfwg9TUbsx9 | Issue #377 設計 — registerQuery adopt-on-join | approved | [issue-377-query-binding-epoch.md](./issue-377-query-binding-epoch.md) |
| DESIGN-034UhiZJ7MAi32Sa1sVTWR | Issue #378 oracle late-create binding epoch 整合設計 | approved | [issue-378-oracle-late-create-epoch.md](./issue-378-oracle-late-create-epoch.md) |
| DESIGN-034T6S9KVWbAOq8lqyMak4 | Driver-neutral GUI Contract 詳細設計 — 依存方向・API移行表・DriverCapability・NativeDock Gate（Issue #384） | approved | [issue-384-driver-neutral-gui-contract.md](./issue-384-driver-neutral-gui-contract.md) |
| DESIGN-034T6SbNaGu3l7owAp7FCX | DTCG Interpreter/Resolver 設計 — alias・mode・override merge・typed ResolvedTheme（Issue #385） | approved | [issue-385-dtcg-resolved-theme.md](./issue-385-dtcg-resolved-theme.md) |
| DESIGN-novwjdV4MyUKuG9Y0bg014 | Issue #386 Driver-neutral GUI Host 契約・GPUI Driver adapter・最小 Window fixture | proposed | [issue-386-driver-neutral-gui-host.md](./issue-386-driver-neutral-gui-host.md) |
| DESIGN-034T6RWQr3Kn5Xp8bC7yF1 | Issue #387 Component/View Specs・Headless Behavior・GPUI Renderer 設計 | proposed | [issue-387-component-view-specs.md](./issue-387-component-view-specs.md) |
| DESIGN-rf388DockWindowing01 | Issue #388 slice 3 設計 — slot projection clip 修正と windowing PoC scenario | approved | [issue-388-native-dock-windowing-slice3.md](./issue-388-native-dock-windowing-slice3.md) |
| DESIGN-E2c3n3d0DhHGOrf8ZGHJrP | Issue #388 Driver-neutral NativeSurfaceSlot 契約設計（SurfaceSlotRegistry / SlotLease / port） | approved | [issue-388-native-surface-slot.md](./issue-388-native-surface-slot.md) |
| DESIGN-034WXSz6r3CcQjiLo08b8H | Issue #389 Parent Lease 公開 API 移行設計 | approved | [issue-389-parent-lease-design.md](./issue-389-parent-lease-design.md) |
| DESIGN-x7UlzjoyvZa1pu2ARRLOZZ | Issue #389 Driver-neutral PluginEditorSession 設計（state machine / typed HostPort / L0-L1 写像・wire negotiation 方針） | approved | [issue-389-plugin-editor-session.md](./issue-389-plugin-editor-session.md) |
| DESIGN-034UnxZxmELaLcIYblZFRI | Issue #389 Plugin Editor Session hardening 設計（deadline/clock・明示 degrade・GraphicsLost・DComp 記録） | approved | [issue-389-session-hardening.md](./issue-389-session-hardening.md) |
| DESIGN-8EX9crwtfdh9PhZtOJ2cfY | Issue #389 VST3/L0 実外部 Editor attach 設計（Vst3Plugin::open_gui/close_gui + Windows HWND attach） | approved | [issue-389-vst3-l0-editor-attach.md](./issue-389-vst3-l0-editor-attach.md) |
| DESIGN-034T9VaXD2KXqhGxaHawyH | Issue #390 Driver Contract Lab — boundary guard 検査設計・registry schema・実行接続点 | approved | [issue-390-driver-contract-lab.md](./issue-390-driver-contract-lab.md) |
| DESIGN-utKDN0iwqlQEVtqkiEUHJi | Issue #400 借用 sealed readback → 独立 decoder bounded 接続の設計 | approved | [issue-400-borrowed-readback-decoder-bridge.md](./issue-400-borrowed-readback-decoder-bridge.md) |
| DESIGN-z4LQPvCYdpedQfNlIZbQ7J | Issue #404/#405 NativeSurfaceSlot host 境界修復設計（clip 整合 + teardown reconcile） | draft | [issue-404-405-native-surface-host-boundary.md](./issue-404-405-native-surface-host-boundary.md) |
| DESIGN-034Tnjz1ofC4OXppkHjsgB | Issue #409 scoped re-root + bounded enumeration 設計 | approved | [issue-409-scoped-enumeration.md](./issue-409-scoped-enumeration.md) |
| DESIGN-034UwcZWs6jYHWBRpUdA8D | Issue #410 P2 — view ledger correlation hardening | approved | [issue-410-p2-view-ledger-correlation-hardening.md](./issue-410-p2-view-ledger-correlation-hardening.md) |
| DESIGN-034UmHwuYp52pp4RtddpwM | Issue #410 — view command ledger bounded retention & correlation semantics | approved | [issue-410-view-command-ledger-bounded-retention-correlation-semantics.md](./issue-410-view-command-ledger-bounded-retention-correlation-semantics.md) |
| DESIGN-NS1HOEsCLKLDYdKMhuZOSf | Issue #411 — cargo_metadata 依存系列選定設計（MSRV 1.89 収束） | approved | [issue-411-msrv-cargo-metadata-pin.md](./issue-411-msrv-cargo-metadata-pin.md) |
| DESIGN-pL8wRtYbN3xKcVhJ5mQs2E | Issue #414 — projection→controlled display 正規経路の設計 | approved | [issue-414-projection-controlled-display.md](./issue-414-projection-controlled-display.md) |
| DESIGN-034Uoh91RxVTOnim395q6R | Issue #418 — theme-codegen 設計（manifest 駆動の ResolvedTheme 静的型生成） | approved | [issue-418-theme-codegen.md](./issue-418-theme-codegen.md) |
| DESIGN-Kt4NzBw9QxLr6VfGc1PuDo | Issue #421 — Meter/TimeAxis GPUI 描画と parity fixture の設計 | proposed | [issue-421-meter-timeaxis-gpui-parity.md](./issue-421-meter-timeaxis-gpui-parity.md) |
| DESIGN-Z3hWk8NpR2tLq6VbYc4MdE | Issue #423 WebView 写像・focus capability 精緻化・GPUI 実窓検証の設計 | proposed | [issue-423-webview-dock-verify.md](./issue-423-webview-dock-verify.md) |
| DESIGN-rO8ZT7WxqPYGw0e0jZJx_A | Issue #426 設計 — mechanism の test 実在検査・GPUI lab binding・story matrix | proposed | [issue-426-gpui-lab-binding.md](./issue-426-gpui-lab-binding.md) |
| DESIGN-Fn7RtQk2VbXp9LwCs4HgJu | Issue #429 — Consumer 観測 seam（GpuiObserver）の設計 | proposed | [issue-429-consumer-observation-seam.md](./issue-429-consumer-observation-seam.md) |
| DESIGN-Wq8Ht3NcZ6RmLv1KbFg7Sd | Issue #431 実機 e2e・実窓 lifecycle 検証・OS close 駆動の設計 | proposed | [issue-431-webview-gpui-real-verify.md](./issue-431-webview-gpui-real-verify.md) |
| DESIGN-Kt8PwRz4LxMc1VqBn6HdSu | Issue #434 — Spectrogram ViewSpec kind の設計 | proposed | [issue-434-spectrogram-kind.md](./issue-434-spectrogram-kind.md) |
| DESIGN-Pw6NsJc4VhTq9BrXy1MaFe | Issue #435 — Label/Table ViewSpec kind と parity の設計 | approved | [issue-435-label-table-kinds.md](./issue-435-label-table-kinds.md) |
| DESIGN-Hv8ZsN3QcKr6YwTe2LmBd1 | Issue #436 — GpuiObserver 描画 plan（drawn）公開の設計 | proposed | [issue-436-observer-drawn.md](./issue-436-observer-drawn.md) |
| DESIGN-Jq3Lw8Yn5RtBe2Fk7MzUc4 | Issue #442 — GpuiObserver 描画 plan への Spectrogram / Label / Table 追加の設計 | approved | [issue-442-observer-drawn-spectrogram-label-table.md](./issue-442-observer-drawn-spectrogram-label-table.md) |
| DESIGN-Vc3Nh8PtRq5LkXw2MdBy7F | Issue #443 — Tile 列区間と count の上流契約の設計 | approved | [issue-443-tile-column-contract.md](./issue-443-tile-column-contract.md) |
| DESIGN-Hn4Wz8QeKs1TcVb6LrYg2D | Issue #448 — 行選択と command 起動の ViewSpec 契約の設計 | approved | [issue-448-table-selection-command.md](./issue-448-table-selection-command.md) |
| DESIGN-5mzN1XENwHq9qvfxBhzqdc | Issue #450 — Seek/Fader canvas のサイズ決定契約 | approved | [issue-450-control-canvas-size.md](./issue-450-control-canvas-size.md) |
| DESIGN-034Ue3PUgmQ7PIOzbrGa9K | Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回配線の設計 | approved | [issue-451-focus-next-wiring.md](./issue-451-focus-next-wiring.md) |
| DESIGN-Hm4Vq8ZsLw2KxTn6RcYb1G | Issue #451 FU-1 — Tabs のキーボード切替の設計 | approved | [issue-451-keyboard-tab-nav.md](./issue-451-keyboard-tab-nav.md) |
| DESIGN-Hk9Qz4WsNt6XrLc1VbYe8M | Issue #451 — Tabs pointer 切替と descriptor 更新の ViewSpec 契約の設計 | approved | [issue-451-tabs-pointer-descriptor.md](./issue-451-tabs-pointer-descriptor.md) |
| DESIGN-60vvc47vxcVx8vlRAGVd3v | Issue #453 — paint canvas のサイズ決定契約と kind 別監査 | approved | [issue-453-canvas-size-audit.md](./issue-453-canvas-size-audit.md) |
| DESIGN-Tm7Xp4QvKn1ZrWd6JbGe8F | Issue #456 — job 終端 receipt の routing 設計 | approved | [issue-456-job-command-receipt.md](./issue-456-job-command-receipt.md) |
| DESIGN-034TXijlXpfTBgjbwGLo16 | Issue #461 — canvas painter の paint batch（paint_layer）適用と kind 別監査 | approved | [issue-461-canvas-paint-layer.md](./issue-461-canvas-paint-layer.md) |
| DESIGN-034U1MKPf89ADLuIJkjzgP | Issue #461 残課題 — TimeAxis label の canvas 内共有 layer 描画 | approved | [issue-461-timeaxis-label-cost.md](./issue-461-timeaxis-label-cost.md) |
| DESIGN-034U0zRvA4RGErmyI4Tcad | Issue #463 — Spectrogram cell 色の不透明事前合成 | approved | [issue-463-spectrogram-intensity.md](./issue-463-spectrogram-intensity.md) |
| DESIGN-034Uc9jupOZFWHdV6h98Xu | Issue #464 — Consumer-neutral GUI Scenario Contract | approved | [issue-464-consumer-neutral-gui-scenario-contract.md](./issue-464-consumer-neutral-gui-scenario-contract.md) |
| DESIGN-GuVirdX8ynQSJId0tz7Dte | Issue #464 slice — Scenario Runner・Report 組み立て契約の設計 | approved | [issue-464-scenario-runner.md](./issue-464-scenario-runner.md) |
| DESIGN-rhqM5KfSVajqQ3vvtVDHol | Issue #465 Legacy WebView P0/P1 — 分類ポリシーと文書歴史化の設計 | approved | [issue-465-legacy-webview-p0p1.md](./issue-465-legacy-webview-p0p1.md) |
| DESIGN-034UaAirEnkd41uDSZZzUH | Issue #468 RootedDir canonical 前提の設計判断 | approved | [issue-468-rooteddir-canonical-premise.md](./issue-468-rooteddir-canonical-premise.md) |
| DESIGN-034UeuINTqCil2VtMHJxGx | Issue #478 Fader/Knob 縦寸法契約 — h_full 兄弟押し出しのレイアウト判断 | approved | [issue-478-fader-knob-height-contract.md](./issue-478-fader-knob-height-contract.md) |
| DESIGN-64fKOE68pKEdIkGIEgcsyW | Issue #479 — 値制御コンポーネントのキーボード操作の設計 | approved | [issue-479-keyboard-value-controls.md](./issue-479-keyboard-value-controls.md) |
| DESIGN-034UiRsPAtNVImhGaTw2TY | Issue #480 設計 — tab_focus の op 駆動解放 | approved | [issue-480-tab-focus-release.md](./issue-480-tab-focus-release.md) |
| DESIGN-eBMdc3IAyT8cNugckC1k2Q | Issue #490 oracle late-create query epoch 整合設計 | approved | [issue-490-oracle-addquery-epoch.md](./issue-490-oracle-addquery-epoch.md) |
| DESIGN-034VedvLujxpzyWOjxyoD4 | Issue #552 — SendInput preflight 設計（注入→実効 read-back・EnvironmentBlocked verdict） | approved | [issue-552-sendinput-preflight.md](./issue-552-sendinput-preflight.md) |
| DESIGN-5Zp45sCRwv11kYwJ4MGJBX | Issue #565 WASAPI-native input path 設計 — per-packet device_position を frame_position に写す | approved | [issue-565-wasapi-input-frame-position-design.md](./issue-565-wasapi-input-frame-position-design.md) |
| DESIGN-ubGFwmTpu82MiFMYl0RSVo | Issue #582 設計 — ci 3 lane 失敗の修正方針(冷キャッシュ fetch / libc pin / moxcms 0.9.2 境界適応) | approved | [issue-582-ci-lane-failures-design.md](./issue-582-ci-lane-failures-design.md) |
| DESIGN-034Wv6hcYDLZycrIcEQ7Ec | Issue #583 設計 — mr_boxington 回帰の原因確定と検証方針(設定変更不要の判断) | approved | [issue-583-consumer-verify-mise-design.md](./issue-583-consumer-verify-mise-design.md) |
| DESIGN-MnDOJoPVmxmAz1hdgVs3T3 | Issue #586 設計 — ir front matter 是正分類・index marker 配置・検証手順 | approved | [issue-586-impl-index-generation-design.md](./issue-586-impl-index-generation-design.md) |
| DESIGN-034WuVTmYfKSkp3sJ8biu2 | Issue #590 設計 — ir missing-section 補完方針・セクション記述規約・検証手順 | approved | [issue-590-ir-missing-sections-design.md](./issue-590-ir-missing-sections-design.md) |
| DESIGN-034JO1BQYrzMQsaFmL3YkF | workspace MSRV 引上げの適用範囲と検証設計 | approved | [msrv-raise-design.md](./msrv-raise-design.md) |
| DESIGN-034JRarHKPb6eKlZqW9WeO | オフライン実行ドライバ設計 | approved | [offline-execution-driver.md](./offline-execution-driver.md) |
| DESIGN-034L2Cjm1Zjxj8oQl2jINm | OFX ホストアダプタ詳細設計（Issue #98） | approved | [ofx-host-adapter.md](./ofx-host-adapter.md) |
| DESIGN-034JZPGUI0000000000002 | プラグイン GUI PoC 設計（外部親 HWND + 外部メッセージループへの wry attach） | approved | [plugin-gui-poc.md](./plugin-gui-poc.md) |
| DESIGN-mEXH6f0uKRaa2mwCWE8mxC | ruviform-plugin-host 設計（L0 インプロセス・ハンドル/コマンド境界） | approved | [plugin-host-l0.md](./plugin-host-l0.md) |
| DESIGN-034JZLISO0000000000003 | プラグイン分離設計（境界・プロセス監視・RT 面制約） | approved | [plugin-isolation.md](./plugin-isolation.md) |
| DESIGN-034LNAalREIHyUQNVXLLin | プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー） | approved | [plugin-l1-out-of-process.md](./plugin-l1-out-of-process.md) |
| DESIGN-034IvBNBZvoikAgd6DJxdD | テンプレート由来ドキュメント・識別子のRuviform固有化の設計 | approved | [replace-template-derived-docs-with-ruviform.md](./replace-template-derived-docs-with-ruviform.md) |
| DESIGN-034VnJlduDjKcQQfErsMwS | Issue #253 L1 wire report verb（drain_output_reports の L1 実装） | approved | [rf-midi-l1-report-verb-design.md](./rf-midi-l1-report-verb-design.md) |
| DESIGN-6GZ1DYjAInxUNz0dwJRwN7 | ruviform-io-file Rooted Read Capability 設計 | approved | [rooted-read-capability.md](./rooted-read-capability.md) |
| DESIGN-034JgsK9khgDsPqG04Sp6i | RT 基盤 API 設計（ruviform-rt プリミティブ・ruviform-exec スレッド優先度機構） | approved | [rt-foundation-api-design.md](./rt-foundation-api-design.md) |
| DESIGN-034JOU7C43HBPJBsnz8JQP | RT 検証レーン設計 | approved | [rt-verification-lane-design.md](./rt-verification-lane-design.md) |
| DESIGN-034IPynWe2V30FLubzQenb | Ruviform architecture design baseline | approved | [ruviform-architecture-design-baseline.md](./ruviform-architecture-design-baseline.md) |
| DESIGN-lzqhLAShvqF3efUlehuaUS | ruviform-dsp 設計（上位 DSP カーネル・math::dsp 層分担・prepare/process 分離） | approved | [ruviform-dsp.md](./ruviform-dsp.md) |
| DESIGN-Tl9CIdfNfmIADLgFIeaTOq | 第 2 ターゲット設計（NLE 編集モデル・OFX 範囲・色管理） | approved | [second-target-design.md](./second-target-design.md) |
| DESIGN-034Kx6CVMPDNgzKhJwfgSp | 共通基盤・Observability・Diagnostics・Debug とクレート分割の設計方針 | approved | [shared-foundations-and-modularization-policy.md](./shared-foundations-and-modularization-policy.md) |
| DESIGN-034JQxPUJjVzr5aekaU1B8 | SIMD dispatch 層設計（トークン繰り上げ + ルーティング + 公開 API 形） | approved | [simd-dispatch-layer.md](./simd-dispatch-layer.md) |
| DESIGN-20261101-synvelis-driven-ruviform-target-architecture | Synvelis要件を基準としたRuviform目標アーキテクチャと段階的Migration v0.7.1 | approved | [synvelis-driven-target-architecture-and-migration.md](./synvelis-driven-target-architecture-and-migration.md) |
| DESIGN-034JcePtzKFix4IuxyE5mD | 時間モデル拡張の設計（FrameRate/TimeBase 分離・Timecode 拡張・丸め誤差累積方針） | approved | [time-model-extension.md](./time-model-extension.md) |
| DESIGN-034JZVID00000000000030 | 動画エンジン設計（パイプライン・スケジューラ・セッション） | approved | [video-engine.md](./video-engine.md) |
| DESIGN-aYGxhiXLZHim70lzzr4olp | ViewLink.attach authoritative generation 採用設計 | approved | [viewlink-authoritative-generation.md](./viewlink-authoritative-generation.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-iwcBlHQ0lcdHJGucocxHOo | 音楽時間モデル設計（小節/拍/PPQN・テンポマップ・プラグイン ABI 整合） | superseded | [music-time-model.md](./music-time-model.md) |
