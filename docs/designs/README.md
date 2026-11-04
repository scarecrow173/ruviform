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
| DESIGN-034LFikCuBIoaD4mWBEjIU | ruviform-video-proc 色変換カーネル設計（f32 linear working space + ICC 入出力経路） | approved | [color-conversion-kernels.md](./color-conversion-kernels.md) |
| DESIGN-034KScKaiUBXz4qKJZqM3h | ruviform-video-proc コンポジット/遷移カーネル設計（blend/affine/基本遷移初期セット） | approved | [composite-transition-kernels.md](./composite-transition-kernels.md) |
| DESIGN-034JZDNRML000000000003 | DenormalFlush ノード設計 | approved | [denormal-flush-node.md](./denormal-flush-node.md) |
| DESIGN-034IddXFCRKDIW01WXkSla | Rust 開発基盤の設計（最小ワークスペース・CIプロファイル・手動CI） | approved | [dev-foundation.md](./dev-foundation.md) |
| DESIGN-034JPvz9ieX8KoPbwDcvaX | dispatch 固定コスト分離の計測設計 | approved | [dispatched-overhead-method.md](./dispatched-overhead-method.md) |
| DESIGN-034JcqP2KiWsdes88fgvtt | DSP/SIMD 比較ベンチマーク PoC 詳細設計 | approved | [dsp-simd-poc.md](./dsp-simd-poc.md) |
| DESIGN-034KNBPbww1kvzVUF4Ab1H | ruviform-edit 設計（NLE 編集データモデル + 評価射影） | approved | [edit-data-model.md](./edit-data-model.md) |
| DESIGN-034Mclz5y96MrTg7R2BNCx | エンジン/利用側へのプラグイン統合（RT endpoint 所有分離 + グラフ接着） | approved | [engine-plugin-host-integration.md](./engine-plugin-host-integration.md) |
| DESIGN-034KQXr3w4dYgKO79dklG9 | Foundation/Observability 候補モジュールの最小 API・依存 DAG・新設時期の設計 | approved | [foundation-observability-minimal-api.md](./foundation-observability-minimal-api.md) |
| DESIGN-034MIeiBWXpLjwgA1yolIT | L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP + VST3 host-side bridge — Issue #114） | approved | [issue-114-real-adapter-l1-e2e.md](./issue-114-real-adapter-l1-e2e.md) |
| DESIGN-034N3J1C5vystrbJl69R1q | L1 GUI — プロセス境界越しの親ウィンドウ埋め込みの採否設計（Issue #115） | approved | [issue-115-l1-gui-cross-process-parenting.md](./issue-115-l1-gui-cross-process-parenting.md) |
| DESIGN-034NNxNMrrk6OoIGfX18rk | L1 supervisor（自動再起動 util 層）設計（Issue #116） | approved | [issue-116-supervisor.md](./issue-116-supervisor.md) |
| DESIGN-034Qa7Kf2NpXw9RtYbHc5d | L1 bundle 子プロセス（1 子プロセス N インスタンス）設計（Issue #117） | approved | [issue-117-bundle-child-process.md](./issue-117-bundle-child-process.md) |
| DESIGN-034NVCweXy0wECErcpEcv8 | video/NONRT プラグインの L1 経路設計（Issue #118） | approved | [issue-118-video-l1.md](./issue-118-video-l1.md) |
| DESIGN-034O9RrQutLFWyaHJGOF3G | Issue 121 L1 diagnostics | approved | [issue-121-l1-diagnostics.md](./issue-121-l1-diagnostics.md) |
| DESIGN-034N3EYKKosV8OzCP7Y9tY | 共通 platform/process/ipc 基盤の抽出評価（Issue #122） | approved | [issue-122-shared-platform-process-ipc-eval.md](./issue-122-shared-platform-process-ipc-eval.md) |
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
| DESIGN-034QfjYdt3o2a4lijg4lwt | Issue #195 M2 Typed Graph Compiler・Bounded Buffers/PDC・RT Runtime・Stateful Ownership・Activation | approved | [issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md](./issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md) |
| DESIGN-C1EIkNXtQvzacfPU0a2bhv | Issue #196 M4 Typed Graph Plugin Adapter — 実装設計 | approved | [issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md](./issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md) |
| DESIGN-fl6bBJvoJRiJHSkk73mskx | Issue #197 Generic Media Reader / Prefetch / RT Consumer — structural design | approved | [issue-197-media-reader-prefetch-rt-consumer.md](./issue-197-media-reader-prefetch-rt-consumer.md) |
| DESIGN-aRsJILdLBlEulHcYpfH4eb | Issue #198 stateful sequential offline render — PreparedRender, sequential driver, staged output lifecycle | approved | [issue-198-sequential-offline-render.md](./issue-198-sequential-offline-render.md) |
| DESIGN-034QdHuPP1Uvvlkvbh5awX | Issue #219 Cross-Repo CI Gate 設計・credential 一次設定・証拠記録 runbook | approved | [issue-219-cross-repo-ci-gate-credential-runbook.md](./issue-219-cross-repo-ci-gate-credential-runbook.md) |
| DESIGN-034QolOnuF7kf2drxSsvvH | Issue #233 prefetch contract improvements — discard_pending drain, seek discard flag, reopen_at structural design | approved | [issue-233-media-reader-prefetch-improvements.md](./issue-233-media-reader-prefetch-improvements.md) |
| DESIGN-034QxcnAfFWswZmwSKPbKH | Issue #244 prefetch contract wart fixes — boundary-first emission, dead-generation sequence, End commit-once, seeded-fault coverage | approved | [issue-244-media-reader-prefetch-warts.md](./issue-244-media-reader-prefetch-warts.md) |
| DESIGN-034RoXjgYbKKZ3ZSazkSGi | Issue #276 汎用 Logging Adapter 設計 | approved | [issue-276-generic-logging-adapter.md](./issue-276-generic-logging-adapter.md) |
| DESIGN-034Rp1Um3ZXDWfRx8pCmuw | Issue #277 OBS-02 軽量Primitive・Observation Source契約: Gauge/Histogram・Descriptor・SourceId/Generation・Registrar/Lease・寿命 | approved | [issue-277-observability-core-source-contract.md](./issue-277-observability-core-source-contract.md) |
| DESIGN-034RTpdouoLLSINNczwa3c | Issue #279 OBS-04 Plugin Host 4 CounterのSource登録: Registrar注入・Descriptor・Lease寿命 | approved | [issue-279-plugin-host-counter-source-registration.md](./issue-279-plugin-host-counter-source-registration.md) |
| DESIGN-034JcpvK9cZEfkeEbBLBoz | Issue #28 UI 技術検証 PoC の設計（使い捨て検証ハーネスと計測・評価方式） | approved | [issue-28-ui-poc.md](./issue-28-ui-poc.md) |
| DESIGN-034RoR9SDtSG2iiZbvNGKT | Issue #295 ruviform-io-file — StagedFile type-state・PublishOptions・Receipt/Failure・PlatformFileOps/Mock 設計 | approved | [issue-295-io-file-staged-publish.md](./issue-295-io-file-staged-publish.md) |
| DESIGN-034RBFdth7UW7A6IS10Ha3 | Issue #296 ruviform-io-file — 実 OS PlatformFileOps アダプタ設計 (IO-IMP-02) | approved | [issue-296-io-file-os-backend.md](./issue-296-io-file-os-backend.md) |
| DESIGN-034RTd0bYqWgcxXWa7uVPE | Issue #297 ruviform-io-structured — 共通契約・Format別API・Backend選定設計 (IO-IMP-03) | approved | [issue-297-io-structured-json-yaml-toml.md](./issue-297-io-structured-json-yaml-toml.md) |
| DESIGN-eGBdnzbpDjWwMwTEldA9gI | Issue #300 ruviform-io-compression — Gzip・Raw Deflate 借用 Stream 設計 (IO-IMP-06) | approved | [issue-300-io-compression-gzip-deflate.md](./issue-300-io-compression-gzip-deflate.md) |
| DESIGN-dHUQ40Hk50tvziqjx7Kewj | Issue #307 attach 時 frame_base 検証 context — 実装設計 | approved | [issue-307-attach-frame-base-verified-context.md](./issue-307-attach-frame-base-verified-context.md) |
| DESIGN-034Rp1KqXNzTDTKRXTbqHx | Issue #310 framework-neutral Browser GUI Test Harness (Playwright・Showcase実DOM検証) | approved | [issue-310-browser-gui-test-harness.md](./issue-310-browser-gui-test-harness.md) |
| DESIGN-wrF61tPcejIc31wMBYJHok | Issue #311 RF-GT-02 ui/guitest — IPC conformance fixture・deterministic record/replay・fault matrix 設計 | approved | [issue-311-ipc-conformance-replay.md](./issue-311-ipc-conformance-replay.md) |
| DESIGN-034S312gtUiTestDsnDD44 | Issue #312 ruviform-ui-test — test-only GUI host lifecycle・secure control adapter 設計 | approved | [issue-312-test-gui-host-lifecycle.md](./issue-312-test-gui-host-lifecycle.md) |
| DESIGN-034RBJLryk98hb4Yyq5wYG | Issue #327 CI計測Baseline 計測契約・分類・比較手順 | approved | [issue-327-cross-repo-ci-baseline.md](./issue-327-cross-repo-ci-baseline.md) |
| DESIGN-yMdXfkOVFpXfZGKfVH7LjG | Issue #328 CI PR concurrency 設計 — group 解決・cancel 意味論・required checks 保護 | approved | [issue-328-ci-pr-concurrency.md](./issue-328-ci-pr-concurrency.md) |
| DESIGN-mcItTpQPSlr7FZ7BBqChaW | Issue #329 sccache 設計 — step 構成・env 解決・cache 正しさ・監査・無効化 | approved | [issue-329-sccache-compile-cache.md](./issue-329-sccache-compile-cache.md) |
| DESIGN-034RBIy0yxPWYhXdXUlsLi | Issue #330 CI-OPT-03 UI npm install 集約・mise task freshness 設計 | approved | [issue-330-ui-npm-ci-dedup.md](./issue-330-ui-npm-ci-dedup.md) |
| DESIGN-034RJ00iyqN3oWCf99pDXu | Issue #331 設計 — check-ci を lint-ci へ統合する等価性証明と適用 | approved | [issue-331-check-clippy-test-dedup.md](./issue-331-check-clippy-test-dedup.md) |
| DESIGN-EJ86GHTwsKgyexnCdTpSJ5 | Issue #332 CI-OPT-05 Playwright Chromium cache・準備分離設計 | approved | [issue-332-playwright-browser-cache.md](./issue-332-playwright-browser-cache.md) |
| DESIGN-oq5cpmBZrL3fUcNnuUzPxg | Issue #333 CI-OPT-06 Impact Classifier・集約 Required Gate・段階移行設計 | approved | [issue-333-impact-based-ci.md](./issue-333-impact-based-ci.md) |
| DESIGN-elUvrfyRcsMrmrESYqsxDn | Issue #336 SinkDegraded Clone 設計 | approved | [issue-336-sink-degraded-clone.md](./issue-336-sink-degraded-clone.md) |
| DESIGN-034RP4B7OViSh6O4YCu1rz | Issue #346 CI-OPT-06 選択実行有効化・ruleset required context 移行 runbook | approved | [issue-346-ruleset-migration-runbook.md](./issue-346-ruleset-migration-runbook.md) |
| DESIGN-034JO1BQYrzMQsaFmL3YkF | workspace MSRV 引上げの適用範囲と検証設計 | approved | [msrv-raise-design.md](./msrv-raise-design.md) |
| DESIGN-034JRarHKPb6eKlZqW9WeO | オフライン実行ドライバ設計 | approved | [offline-execution-driver.md](./offline-execution-driver.md) |
| DESIGN-034L2Cjm1Zjxj8oQl2jINm | OFX ホストアダプタ詳細設計（Issue #98） | approved | [ofx-host-adapter.md](./ofx-host-adapter.md) |
| DESIGN-034JZPGUI0000000000002 | プラグイン GUI PoC 設計（外部親 HWND + 外部メッセージループへの wry attach） | approved | [plugin-gui-poc.md](./plugin-gui-poc.md) |
| DESIGN-mEXH6f0uKRaa2mwCWE8mxC | ruviform-plugin-host 設計（L0 インプロセス・ハンドル/コマンド境界） | approved | [plugin-host-l0.md](./plugin-host-l0.md) |
| DESIGN-034JZLISO0000000000003 | プラグイン分離設計（境界・プロセス監視・RT 面制約） | approved | [plugin-isolation.md](./plugin-isolation.md) |
| DESIGN-034LNAalREIHyUQNVXLLin | プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー） | approved | [plugin-l1-out-of-process.md](./plugin-l1-out-of-process.md) |
| DESIGN-034IvBNBZvoikAgd6DJxdD | テンプレート由来ドキュメント・識別子のRuviform固有化の設計 | approved | [replace-template-derived-docs-with-ruviform.md](./replace-template-derived-docs-with-ruviform.md) |
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

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-iwcBlHQ0lcdHJGucocxHOo | 音楽時間モデル設計（小節/拍/PPQN・テンポマップ・プラグイン ABI 整合） | superseded | [music-time-model.md](./music-time-model.md) |
