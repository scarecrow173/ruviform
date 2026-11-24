# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/discovery`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DSC-154-al-media-matrix | AudioLens al-media サポートマトリクス抽出と Ruviform 境界との差分 | final | [al-media-support-matrix.md](./al-media-support-matrix.md) |
| DISC-034JcqEc2b3EHNgH9CRLaC | RT 基盤プリミティブの実装方式評価（SPSC・退避ドロップ・スナップショット交換・スレッド優先度） | draft | [evaluate-rt-foundation-primitives.md](./evaluate-rt-foundation-primitives.md) |
| DISC-034IO8gG0n5xCqh66w8KUp | Issue #1 詳細設計に向けた要求確定とエコシステム調査 | draft | [explore-architecture-design-baseline.md](./explore-architecture-design-baseline.md) |
| DISC-034JcicdqiAQzbUKFHiCbc | Issue #26 に向けた動画基盤 API・色変換/HDR 対応範囲の調査 | draft | [explore-video-foundation-api-scope.md](./explore-video-foundation-api-scope.md) |
| DISC-034NMZseZM94vtNrYx24rM | L1 自動再起動・supervisor ポリシーの探索（Issue #116） | active | [issue-116-supervisor.md](./issue-116-supervisor.md) |
| DISC-034NiVjJSeI0gfYX87KMjH | L1 bundle 子プロセス（1 子プロセス N インスタンス）の探索（Issue #117） | active | [issue-117-bundle-child-process-discovery.md](./issue-117-bundle-child-process-discovery.md) |
| DISC-034NV8A2eMJbUhGFvQgL3W | Issue #118 video/NONRT プラグインの L1 経路 | active | [issue-118-video-l1.md](./issue-118-video-l1.md) |
| DISC-034O7hCIm5rGi3tA4rmxuc | Issue #121 L1 診断・可観測性（underrun メトリクス・子 stderr 配管・失敗原因 payload） | active | [issue-121-diagnostics.md](./issue-121-diagnostics.md) |
| DISC-034QdFl4ZIWzDfKWh0Cg90 | Issue #184 型付き複数Bus/Port・Graph IRの現行実装棚卸しとcrate配置比較 | active | [issue-184-bus-port-graph-ir-crate.md](./issue-184-bus-port-graph-ir-crate.md) |
| DISC-034QhIlaOBN3t7jVxA4sW7 | Issue #185 Generic Probe: Node/Port observation capability, selective monitoring, RT-safe bounded PCM capture — existing-surface inventory and design option survey | active | [issue-185-generic-probe-observation-inventory.md](./issue-185-generic-probe-observation-inventory.md) |
| DISC-034WYeBHWSdbvryUnNKkus | Issue #192 TA-001〜012 Invariant ↔ 現行API/Issue/L1テスト対応表 + 依存非循環性監査 | active | [issue-192-ta-invariant-map.md](./issue-192-ta-invariant-map.md) |
| DISC-034Q6prdW5pMq5zW9uTb3z | Issue #194 M0 互換性・依存・検証 Baseline 実測監査 | active | [issue-194-m0-baseline-audit.md](./issue-194-m0-baseline-audit.md) |
| DISC-034QfjLwjBVqddpxivwOTg | Issue #195 M2 Typed Graph Audio Compiler・Buffer/PDC・Stateful RT Ownership・Plan Activation の現行棚卸し | active | [issue-195-m2-typed-graph-compiler-runtime-audit.md](./issue-195-m2-typed-graph-compiler-runtime-audit.md) |
| DISC-034QdBDFXQ7CpGCCYpN94k | Issue #219 両 Consumer 依存形態・検証基盤棚卸しと Cross-Repo CI 前提実測 | draft | [issue-219-consumer-cross-repo-ci.md](./issue-219-consumer-cross-repo-ci.md) |
| DISC-034JbXZSba3SvOavRFfs66 | Issue #28 UI 技術検証 PoC の事前調査 | draft | [issue-28-ui-poc.md](./issue-28-ui-poc.md) |
| DISCOVERY-034T6RWRk7Pj3Qm9vN0xF4 | Issue #387 Driver-neutral Component/View Specs + Headless Behavior + GPUI Renderer 棚卸 | proposed | [issue-387-component-view-inventory.md](./issue-387-component-view-inventory.md) |
| DISC-034T9T71FKLbmcJ4jmuUcD | Issue #390 Driver Contract Lab 既存資産棚卸し（Fake/Boundary/Dock/Headless 現行 coverage と gap） | final | [issue-390-driver-contract-lab-inventory.md](./issue-390-driver-contract-lab-inventory.md) |
| DISC-034YnFUCO4ivyDKz4EYxVt | Issue #532 dcomp-occlusion discovery — 遮蔽層診断の探索結果 | approved | [issue-532-dcomp-occlusion.md](./issue-532-dcomp-occlusion.md) |
| DISC-034YxCYwvosUuX47paEbNO | Issue #532 foreground-probe discovery — 変異 probe の探索結果 | approved | [issue-532-foreground-probe.md](./issue-532-foreground-probe.md) |
| DISC-fZWC8q6tfUQG3VuH1L9Eoz | Issue #532 presentation-probe discovery — 残余 3 候補の判別設計探索 | approved | [issue-532-presentation-probe.md](./issue-532-presentation-probe.md) |
| DISC-rf532sltcmp0stn1t3rp | Issue #532 スロット合成 PoC — fork-free 介入 2 方式の実測と Option-A 判定 | resolved | [issue-532-slot-compositing.md](./issue-532-slot-compositing.md) |
| DISC-034ZaaUncoveredBaseline01 | Issue #532 uncovered-baseline bisect — 調査記録 | approved | [issue-532-uncovered-baseline.md](./issue-532-uncovered-baseline.md) |
| DISC-034XC9utDSmNZnzNq2DOds | Issue #629 Windows virtual audio/MIDI endpoint survey and capability probe | draft | [issue-629-virtual-endpoint-survey.md](./issue-629-virtual-endpoint-survey.md) |
| DSC-630abiValidatorRerun01 | Issue #630 外部 ABI validator 再実走 slice — post-#735 surface の調査と declared-expectation 設計判断 | final | [issue-630-abi-validator-rerun.md](./issue-630-abi-validator-rerun.md) |
| DSC-630abiValidator01 | Issue #630 外部 ABI validator slice — provisioning 調査と validator 指摘の診断 | final | [issue-630-abi-validator.md](./issue-630-abi-validator.md) |
| DSC-630clapEditorVerbs01 | Issue #630 CLAP editor verb 実バイナリ coverage — slice 調査記録 | final | [issue-630-clap-editor-verbs.md](./issue-630-clap-editor-verbs.md) |
| DISC-034ZCLAPED1sl1ceM3kQ2x | Issue #630 CLAP test_editor slice — exploration notes | completed | [issue-630-clap-test-editor.md](./issue-630-clap-test-editor.md) |
| DSC-630presetDiscoveryReeval01 | Issue #630 preset-discovery 外部検証再評価 — upstream fix pin 調査 | final | [issue-630-preset-discovery-reeval.md](./issue-630-preset-discovery-reeval.md) |
| DSC-630presetSpawnFault01 | Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 調査記録 | final | [issue-630-preset-spawn-fault.md](./issue-630-preset-spawn-fault.md) |
| DSC-630acInventory01 | Issue #630 umbrella AC disposition census — AC 行 verdict 写像・残件分類の調査記録 | final | [issue-630-ac-inventory.md](./issue-630-ac-inventory.md) |
| DISC-Kd82mQxVnR4tYw6pLc3sJa | Issue #631 real-command-receipt case | draft | [issue-631-command-receipt.md](./issue-631-command-receipt.md) |
| DISC-x7Kp2mQwR9vTn4bYc6dF | Issue #631 AC4 evidence-fields 完成 slice | draft | [issue-631-evidence-fields.md](./issue-631-evidence-fields.md) |
| DISC-g9Hv3kLm6Qp2rT8wXc5Nb | Issue #631 AC4 'OS and GPU' display-adapter field | draft | [issue-631-gpu-field.md](./issue-631-gpu-field.md) |
| DISC-Mx41kYpNvT8wQc6rLd9sJh | Issue #631 headless-command-receipt case | draft | [issue-631-headless-command-receipt.md](./issue-631-headless-command-receipt.md) |
| DISC-EQytwTTrhPQ8wwzjh4tGK3 | Issue #631 real-window-close/dpi catalog flip | draft | [issue-631-real-window-catalog-flip.md](./issue-631-real-window-catalog-flip.md) |
| DISC-034YJ08C2FLjqUQpNN0PcG | Issue #631 RealOsGuiTest window-close/DPI 観測面 | draft | [issue-631-window-surface.md](./issue-631-window-surface.md) |
| DSC-631acInventory01 | Issue #631 umbrella AC disposition census — AC 行 verdict 写像・残件分類の調査記録 | final | [issue-631-ac-inventory.md](./issue-631-ac-inventory.md) |
| DSC-720tupleV201 | Issue #720 consumer-verify-tuple contract v1→v2 bump — 現状 pin 箇所の棚卸しと設計論点の調査 | final | [issue-720-consumer-tuple-v2.md](./issue-720-consumer-tuple-v2.md) |
| DSC-722vst3LifecycleAudit01 | Issue #722 production VST3 adapter lifecycle 監査 — test fixture 同型欠陥の診断 | final | [issue-722-vst3-lifecycle-audit.md](./issue-722-vst3-lifecycle-audit.md) |
| DSC-726clapValidatorPanic01 | Issue #726 clap-validator 0.4.1 param-fuzz panic — 根因所在と upstream 報告要否の調査 | final | [issue-726-clap-validator-panic.md](./issue-726-clap-validator-panic.md) |
| DSC-732clapLifecycleAudit01 | Issue #732 production CLAP adapter lifecycle 監査 — VST3同型欠陥の診断 | final | [issue-732-clap-lifecycle-audit.md](./issue-732-clap-lifecycle-audit.md) |
| DSC-630singleComponent01 | Issue #630 single-component slice — VST3 `test_stateful_single` 調査記録 | final | [issue-630-single-component.md](./issue-630-single-component.md) |
| DSC-630consumerOracle01 | Issue #630 AC7 consumer-oracle slice — 調査記録 | final | [issue-630-consumer-oracle.md](./issue-630-consumer-oracle.md) |
| DSC-630clapOutputDrain01 | Issue #842 CLAP output-drain slice — 調査記録 | final | [issue-630-clap-output-drain.md](./issue-630-clap-output-drain.md) |
| DISC-034ZaaBandEscapeProbe01 | Issue #532 band-escape probe — 調査記録 | approved | [issue-532-band-escape.md](./issue-532-band-escape.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DISC-034LEXAPQpsGvnYeoKEahA | 色変換カーネル（f32 linear working space + ICC 入出力経路）の探索（Issue #99） | resolved | [color-conversion-kernels.md](./color-conversion-kernels.md) |
| DISC-034Jc7DRXwhHU9BTMwYJe2 | DSP/SIMD バックエンド比較 PoC の探索（Issue #19） | resolved | [dsp-simd-poc-issue-19.md](./dsp-simd-poc-issue-19.md) |
| DISC-034MbMMsIDqdQqkkTzFSTf | エンジン/利用側へのプラグイン統合（plugin-host ノード化 + L1 選択配線）の探索（Issue #113） | resolved | [engine-plugin-host-integration.md](./engine-plugin-host-integration.md) |
| DISC-034Jc8wchRH37hqfhNLRPJ | オーディオデバイスバックエンド評価（cpal 等）の探索 | resolved | [evaluate-audio-device-backends.md](./evaluate-audio-device-backends.md) |
| DISC-034JccH31pBpKOSW5fst0g | プラグイン基盤 binding 方式の評価（Issue #29） | resolved | [evaluate-plugin-binding-options.md](./evaluate-plugin-binding-options.md) |
| DISC-034MHDcCcgTMOTFraQb90l | L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP/VST3/OFX）の探索（Issue #114） | resolved | [issue-114-real-adapter-l1-e2e.md](./issue-114-real-adapter-l1-e2e.md) |
| DISC-fqJPHPj2JQlpssrbP4erm6 | Issue #120 re-evaluation analysis — trigger #3 fired (per-child 3rd thread) | resolved | [issue-120-reaper-iomux-reeval-analysis.md](./issue-120-reaper-iomux-reeval-analysis.md) |
| DISC-034P8EGzRky80ILbGeYdKp | Issue #156 GUI ホスト基盤: PoC 棚卸しと本番化ギャップ | resolved | [issue-156-gui-host-foundation-inventory.md](./issue-156-gui-host-foundation-inventory.md) |
| DISC-hUXakPlCDliaq86pqk2UE3 | Issue #181 Multi-Window Host — existing API inventory and gap analysis | resolved | [issue-181-multi-window-inventory.md](./issue-181-multi-window-inventory.md) |
| DISC-GLEiDNdU7qC07pVNVGO6O1 | Issue #182 Dock Tree / View Move / Layout Restore — existing API inventory and gap analysis | resolved | [issue-182-dock-tree-inventory.md](./issue-182-dock-tree-inventory.md) |
| DISC-034QkuJnI4etlHHuplmcbe | Issue #186 Parameter Control Events — existing API inventory and gap analysis | resolved | [issue-186-param-control-events-inventory.md](./issue-186-param-control-events-inventory.md) |
| DISC-qiDOIoAYQ5bcRQ5rpis6kI | Issue #188 Timed MIDI — existing API inventory and gap analysis | resolved | [issue-188-timed-midi-inventory.md](./issue-188-timed-midi-inventory.md) |
| DISC-034Qg9A5RpGIykEqJYBwFA | Issue #190 Generic Transport Span Scheduler / Applied Receipt / Clock-Plan Generation contract | resolved | [issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md](./issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md) |
| DISC-034QhqgG9EF5ipLnFJv0Ka | Issue #191 PCM WAV encode/mux backend — crate placement, output contract, safe publish | resolved | [issue-191-pcm-wav-output-backend.md](./issue-191-pcm-wav-output-backend.md) |
| DISC-U91vIxxv8vbdgKjs4t9moz | Issue #196 M4 Typed Graph Plugin Adapter — multi-bus/stateful lifecycle/process offset precision | resolved | [issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md](./issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md) |
| DISC-uYfaNCLgYERA7zX0pHelL0 | Issue #197 Generic Media Reader / Decoder・Prefetch・RT Playback Consumer 境界 — inventory, crate placement, contract shape | resolved | [issue-197-media-reader-prefetch-rt-consumer.md](./issue-197-media-reader-prefetch-rt-consumer.md) |
| DISC-ubO5TxPGQDZSaLVHth3jpg | Issue #198 Stateful Sequential Offline Render・Frame/Preroll/PDC/Tail・Safe Output Receipt — inventory, placement, contract shape | resolved | [issue-198-sequential-offline-render.md](./issue-198-sequential-offline-render.md) |
| DISC-034Qohljm0FMrizvY86YLe | Issue #233 media-reader Prefetch/Seek/Pump contract doc & feature improvement candidates — inventory, option analysis | resolved | [issue-233-media-reader-prefetch-contract-improvements.md](./issue-233-media-reader-prefetch-contract-improvements.md) |
| DISC-034Qx5mnsXSbsrFmJqSCM1 | Issue #244 media-reader prefetch contract residual warts — boundary-first fault emission, reopen-failure dead generation, End commit-once | resolved | [issue-244-media-reader-prefetch-contract-warts.md](./issue-244-media-reader-prefetch-contract-warts.md) |
| DISC-XxEhtltezK4yxgxXo5SeB7 | Issue #294 consumer oracle lanes — consumer-verify 両 verify job への oracle lane 配線 現状監査 | resolved | [issue-294-consumer-oracle-lanes.md](./issue-294-consumer-oracle-lanes.md) |
| DISC-034S9kMn2pQ7rT4vX8yB1w | Issue #294 Cross-Consumer候補Gate — consumer-verify.yml 現状監査と要件ギャップ | resolved | [issue-294-cross-consumer-candidate-gate.md](./issue-294-cross-consumer-candidate-gate.md) |
| DISC-034XBVWijUovcdqO2rYXzx | Issue #294 virtual-first evidence schema — consumer-verify Case schema v2 現状監査 | resolved | [issue-294-virtual-first-evidence-schema.md](./issue-294-virtual-first-evidence-schema.md) |
| DISC-UsHnmyyijB53IvR586RquN | Issue #311 RF-GT-02 IPC schema conformance / deterministic record-replay / fault matrix — contract inventory | resolved | [issue-311-ipc-conformance-inventory.md](./issue-311-ipc-conformance-inventory.md) |
| DISC-034S312gtUiTestDiscAA11 | Issue #312 test-only GUI host lifecycle / secure control adapter — inventory and option survey | resolved | [issue-312-test-gui-host-lifecycle.md](./issue-312-test-gui-host-lifecycle.md) |
| DISC-034RBJK2HlLAlKCKmHP5Au | Issue #327 3リポジトリ横断CI計測Baseline 前提棚卸し | resolved | [issue-327-cross-repo-ci-baseline.md](./issue-327-cross-repo-ci-baseline.md) |
| DISC-034RC37YxqeDsq3Kad550u | Issue #330 CI-OPT-03 UI npm ci 重複・dist 依存 DAG 棚卸し | resolved | [issue-330-ui-npm-ci-inventory.md](./issue-330-ui-npm-ci-inventory.md) |
| DISC-034RIZZ08rxXFAvXxiesKc | Issue #331 check/clippy/test 重複評価の調査・実測 | resolved | [issue-331-check-clippy-test-dedup.md](./issue-331-check-clippy-test-dedup.md) |
| DISC-k7Pjkn8XoebpncwaTSISdf | Issue #332 CI-OPT-05 Playwright Chromium 準備・計測分離の棚卸し | resolved | [issue-332-playwright-browser-prep-inventory.md](./issue-332-playwright-browser-prep-inventory.md) |
| DISC-3qnA5E8SxxIxxEfA9LSI1k | Issue #333 CI-OPT-06 変更影響ベース検証 DAG・Fail-Closed Required Gate の棚卸し | resolved | [issue-333-impact-classifier-inventory.md](./issue-333-impact-classifier-inventory.md) |
| DISC-im2uskbeNbGlhxBg1mtsTR | Issue #386 Driver-neutral GUI Host: 現行契約棚卸しと WebView 前提分離案 | resolved | [issue-386-driver-neutral-gui-host-inventory.md](./issue-386-driver-neutral-gui-host-inventory.md) |
| DISC-c2G2365EomTupKRjR3zoqt | Issue #388/#389 Native Plugin Dock 現行実装監査（GuiParentToken / L1 Unsupported / wire / Dock 契約の棚卸し） | resolved | [issue-388-389-native-dock-current-state.md](./issue-388-389-native-dock-current-state.md) |
| DISC-034YtTASQQBBWA8Mb9PpLe | Issue #389 editor-session real-session-terminate 調査記録 | resolved | [issue-389-editor-session.md](./issue-389-editor-session.md) |
| DISC-rf389-ime-verbs-01 | Issue #389 adapter editor verbs 調査記録 | resolved | [issue-389-ime-verbs.md](./issue-389-ime-verbs.md) |
| DISC-rf389-real-session-lifecycle-01 | Issue #389 real session lifecycle remainder 調査記録 | resolved | [issue-389-real-session-lifecycle.md](./issue-389-real-session-lifecycle.md) |
| DISC-PZXzzHRXfnvatIchgHgLEJ | Issue #465 Legacy WebView 基盤の棚卸し — 撤去対象・移行先・consumer 影響 | resolved | [issue-465-legacy-webview-inventory.md](./issue-465-legacy-webview-inventory.md) |
| DISC-034WZDbDyQpBcWspeahHje | Issue #532 GPUI DComp topmost 合成制約 — 現行 L1 証拠照合・再開条件・方式 A/B/C 調査手順監査 | resolved | [issue-532-dcomp-native-dock.md](./issue-532-dcomp-native-dock.md) |
| DISC-034XAXQbwZKvh8lVqSB9dy | Issue #532 GPUI 0.2.2 DComp public-surface 監査 — 方式A/B/C 技術入力 | resolved | [issue-532-gpui-dcomp-surface.md](./issue-532-gpui-dcomp-surface.md) |
| DISC-034ZaaChildInvDiagnosis01 | Issue #532 child-level inventory + capture-plane 診断 — 調査記録 | approved | [issue-532-child-inventory.md](./issue-532-child-inventory.md) |
| DISC-034ZaaRasterfix0000001 | Issue #532 raster-fix oracle — 実測調査記録 | approved | [issue-532-rasterfix.md](./issue-532-rasterfix.md) |
| DISC-034YaXLgv895taUPDbx0B6 | Issue #631 GPUI Test Lab / Scenario Console GUI v1 + mise lane 簿記 — 調査記録 | resolved | [issue-631-test-lab.md](./issue-631-test-lab.md) |
| DISC-01M7WJTWYSRTVAMEGHYE8WA4F4 | Issue #631 editor input/focus verb 面 — EditorSessionOps 拡張と fixture の OS-observable 効果チャンネル | resolved | [issue-631-editor-input.md](./issue-631-editor-input.md) |
| DISC-qxX0PvQy0Qu34AwzZ69G5e | Issue #692 s2 Luau analyzer backend — 調査記録 | resolved | [issue-692-luau-backend.md](./issue-692-luau-backend.md) |
| DISC-034ZkLmNpQrStUvWxYzKl6 | Issue #692 ruviform-script port crate — 調査記録 | resolved | [issue-692-script-port.md](./issue-692-script-port.md) |
| DISC-uyGbmYQxvwiFeiFkGWSPO7 | Issue #716 runtime ScriptBackend — 調査記録 | resolved | [issue-716-runtime-backend.md](./issue-716-runtime-backend.md) |
| DISC-034YZUNuibc7hwIC9PXWm9 | Issue #725 EnvironmentKind serde 語彙 gap — consumer 影響評価と移行方針（Phase A） | resolved | [issue-725-serde-vocab-phase-a.md](./issue-725-serde-vocab-phase-a.md) |
| DISC-034Z7dvDpqTYOW3pCKWH8I | Issue #725 EnvironmentKind serde 語彙 gap 解消 Phase B — emit flip | resolved | [issue-725-serde-vocab-phase-b.md](./issue-725-serde-vocab-phase-b.md) |
| DISC-034Z78u4glu94S3wActSDq | Issue #744 bounded non-RT JobExecutor — 実装 slice の探索・設計確定 | resolved | [issue-744-job-executor.md](./issue-744-job-executor.md) |
| DISC-034ZhjBMAgFhy2Ry7D4Z5S | Issue #759 plugin-clap host.rs f64 as u32 cast — clippy lint lane 破損の診断と境界契約 | resolved | [issue-759-clippy-cast.md](./issue-759-clippy-cast.md) |
| DISC-034L2cDWe52zQHUbLyygTw | OFX ホストアダプタ境界とバインディング方針の探索（Issue #98） | resolved | [ofx-host-adapter-boundary-and-bindings.md](./ofx-host-adapter-boundary-and-bindings.md) |
| DISC-034LNekQmplu83WT5PUMzs | プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー）の探索（Issue #76） | resolved | [plugin-l1-out-of-process.md](./plugin-l1-out-of-process.md) |
| DISC-rf768staleOracle01 | Issue #768 l1_fault / l1_stateful E2E staleness oracle 監査（#763 同 class）— findings | final | [issue-768-staleness-oracle.md](./issue-768-staleness-oracle.md) |
| DISC-rf841spawnDeadline | Issue #841 L1 spawn/handshake deadline flake（SpawnFailed{Timeout}）— 定量・分類ルール・対処方針 | resolved | [issue-841-spawn-deadline-flake.md](./issue-841-spawn-deadline-flake.md) |
| DISC-034ZbbClipPart000000001 | Issue #839 GPUI fork clip-partition fix — 実測調査記録 | approved | [issue-839-gpui-dcomp.md](./issue-839-gpui-dcomp.md) |
| DISC-034ZccDcompResidual01 | Issue #532 DComp residual — 根治 fix 統合後の残存 Fail class 切り分け記録 | approved | [issue-532-dcomp-residual.md](./issue-532-dcomp-residual.md) |
| DISC-rf855popupKeyToggle | Issue #855 popup_key_toggles_plugin_owned_window 非 Windows 決定的 defect — 原因分類記録 | approved | [issue-855-popup-key-ci.md](./issue-855-popup-key-ci.md) |
| DISC-rf857PixelOcclusion | Issue #857 pixel-occlusion — real-dcomp-pixel lane 自身 top-level window 遮蔽（class B）調査記録 | approved | [issue-857-pixel-occlusion.md](./issue-857-pixel-occlusion.md) |
| DISC-rf868hostDirtyNotice | Issue #868 production host dirty/mark_dirty 通知経路の棚卸しと実装要否の判断 | final | [issue-868-host-dirty-notification.md](./issue-868-host-dirty-notification.md) |
| DISC-01M7Y96P5AVRW761HK2XP77XF2 | Issue #631 real-editor-popup real-HWND harness — 調査記録 | resolved | [issue-631-real-popup-harness.md](./issue-631-real-popup-harness.md) |
| DISC-rf388-virtual-first-ac-01 | Issue #388 Virtual-first AC 照合 — canonical real-dock lane 証拠 inventory・case→AC 対応根拠・RT 非干渉 rationale | approved | [issue-388-virtual-first-ac.md](./issue-388-virtual-first-ac.md) |
| DISC-01KBZWINDOWIME0DISCOVERY00A1 | Issue #389 real-window-ime — ホスト IME 実測、IMM32 駆動経路、oracle channel と capability claim の確定 | resolved | [issue-389-window-ime.md](./issue-389-window-ime.md) |
| DISC-0351ujnK1guLRroTqT6pcS | Issue #872 L1 OOS editor host — docked 越境 vs child 所有 toplevel・動詞/bit 設計・fault 経路・isolation 選択の比較と却下理由 | resolved | [issue-872-l1-oos-plugin-editor-host.md](./issue-872-l1-oos-plugin-editor-host.md) |
| DISC-Rf767SbmtDrpXv4nQyZk8Hw2 | Issue #767 dropped_submits の周期的発生 — L1 submit 経路の drop 原因調査 | resolved | [issue-767-submit-drop.md](./issue-767-submit-drop.md) |
| DISC-rf873pluginGuiRequest | Issue #873 plugin→host GUI 要求チャンネル — 現状棚卸し | final | [issue-873-plugin-gui-request.md](./issue-873-plugin-gui-request.md) |
| DISC-rf874rtOracle | Issue #874 実 Window RT 非干渉 oracle 調査記録 | resolved | [issue-874-rt-oracle.md](./issue-874-rt-oracle.md) |
| DISC-Rf895DrpSbtCse7nQ2xKw4Y9 | Issue #895 dropped_submits の条件別分離 — carrier 選定と集約 counter 扱いの調査 | resolved | [issue-895-dropped-submits.md](./issue-895-dropped-submits.md) |
| DISC-952vEPdR1v3rSurVeyX01 | Issue #952 仮想 Audio/MIDI endpoint driver 方式調査 — signing/runner/WMS 一次根拠 | draft | [issue-952-virtual-endpoint-driver-survey.md](./issue-952-virtual-endpoint-driver-survey.md) |
