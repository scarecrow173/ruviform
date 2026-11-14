# Experiment Log Documents

Directory: `docs/impl/exp`

| File | 対応 AC / タスク | 内容 |
| --- | --- | --- |
| [ac1-host-experiment-wry-tao-webview.jsonl](./ac1-host-experiment-wry-tao-webview.jsonl) | AC1 / poc-ui-harness-host | wry+tao ホスト実験（埋め込みロード・IPC・リサイズ・フォーカス） |
| [ac2-render-canvas2d-webgl2-webgpu.jsonl](./ac2-render-canvas2d-webgl2-webgpu.jsonl) | AC2 / poc-render-backends | 描画バックエンド計測（canvas2d/webgl2/webgpu フレーム統計） |
| [ac3-surface-adjacent-child.jsonl](./ac3-surface-adjacent-child.jsonl) | AC3 / poc-native-surface | ネイティブサーフェス合成 adjacent-child モード |
| [ac3-surface-adjacent-parent.jsonl](./ac3-surface-adjacent-parent.jsonl) | AC3 / poc-native-surface | ネイティブサーフェス合成 adjacent-parent モード |
| [ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl](./ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl) | AC4 / poc-message-boundary | メッセージ境界計測（JSON RTT・バイナリ pull・evaluate_script push・併用） |
| [ac5-frontend-compare.jsonl](./ac5-frontend-compare.jsonl) | AC5 / poc-frontend-and-dist | フロントエンド比較（vanilla vs solid） |
| [ac6-dist-single-binary.jsonl](./ac6-dist-single-binary.jsonl) | AC6 / poc-frontend-and-dist | 配布検証（release 単一バイナリ・portable zip・文書評価整理） |
| [ac7-adr-finalize.jsonl](./ac7-adr-finalize.jsonl) | AC7 / poc-adr-finalize | 採否 ADR 確定（エビデンス集約・条項別 confirm/revise verdict） |
| [device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl](./device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl) | — / Issue #23 | オーディオデバイス PoC 実機実行ログ（list_devices・play_sine・WASAPI） |
| [dsp-simd-poc-benchmark-comparison.jsonl](dsp-simd-poc-benchmark-comparison.jsonl) | — / Issue #19 | DSP/SIMD バックエンド比較ベンチ計測 |
| [issue-57-high-load-webgpu-multi-webview.jsonl](./issue-57-high-load-webgpu-multi-webview.jsonl) | — / Issue #57 | UI PoC 後続課題（高負荷再計測・WebGPU 有効化・複数 WebView・wry #1762 追跡） |
| [linux-container-device-and-arch-verification.jsonl](./linux-container-device-and-arch-verification.jsonl) | — / Issue #60, #55, #57 | Linux 検証コンテナ実測（ALSA/PipeWire・host_unavailable・aarch64 NEON・AVX-512 試行・Xvfb UI） |
| [rt-observability-poc.jsonl](./rt-observability-poc.jsonl) | — / Issue #104 | RT 観測コスト計測 PoC（counter/rt_bridge/overflow policy/timestamp・バックエンド採否評価・alloc-gap 実証） |
| [issue-327-ci-baseline.jsonl](./issue-327-ci-baseline.jsonl) | — / Issue #327 (CI-OPT-00) | 3リポジトリ横断 CI 計測 Baseline（run/job/step timing・lane 分類・critical path・cache Hit/Miss・required checks 棚卸し） |

## Reports and raw data

- [issue-157-viz-bench-20261031.md](issue-157-viz-bench-20261031.md) — Issue #157 ruviform-viz criterion bench measurement report（波形 LOD 構築・ビューポート応答・STFT・スペクトログラムタイル・binary encode。生ログ: `data/issue-157-viz-bench.log`）
- [dsp-simd-poc-20261021.md](dsp-simd-poc-20261021.md) — Issue #19 DSP/SIMD PoC bench measurement report
- [rt-observability-poc-20261023.md](rt-observability-poc-20261023.md) — Issue #104 RT observability cost PoC measurement report (counter instrumentation, rt_bridge record/drain/overflow policies, RT primitive baselines, timestamp acquisition, backend evaluation, alloc coverage-gap demo, #103 verdict mapping)
- [issue-327-ci-baseline.md](issue-327-ci-baseline.md) — Issue #327 (CI-OPT-00) 3-repo CI 計測 Baseline report（required-verification 棚卸し・代表 run・critical path・総 Runner 時間・cache Cold/Warm・Before/After 手順。raw: `data/issue-327-ci-baseline-runs.jsonl`。計測契約: `docs/designs/issue-327-cross-repo-ci-baseline.md`。収集器: `ci/metrics/collect-ci-metrics.mjs`）
- [issue-332-playwright-browser-cache.md](issue-332-playwright-browser-cache.md) — Issue #332 (CI-OPT-05) Playwright Chromium cache Before/After report（browser setup Cold/Warm・download 有無・実 test 時間の分離計測・fallback/検証能力確認。raw: `data/issue-332-playwright-browser-runs.jsonl`）
- [issue-331-check-clippy-test-dedup.md](issue-331-check-clippy-test-dedup.md) — Issue #331 (CI-OPT-04) check/clippy/test 重複評価実測と verify lane check-ci 統合効果 report（check/clippy 同一ユニット集合 292・ws 二重 frontend 140 除去・baseline step 時間・注入実験等価性。raw: `data/issue-331-verify-step-runs.jsonl`）
- [issue-333-impact-based-ci.md](issue-333-impact-based-ci.md) — Issue #333 (CI-OPT-06) impact classifier + ci-gate Shadow Mode 観測記録（expected×actual 照合・full fallback 発火・偽陰性検出器の稼働状態・段階移行の残課題。raw: `data/issue-333-impact-decision-run-41835836190.json` + `data/issue-333-gate-report-run-41835836190.json`）
- [issue-192-ta-invariant-map.md](issue-192-ta-invariant-map.md) — Issue #192 TA Invariant Map 監査手法の試行記録（cargo metadata dep_kinds 解釈・宣言↔解決差分・dev-dep 循環誤認回避・生成 script 出力先・/tmp パス不整合の補正）
- [issue-346-selective-enforcement.md](issue-346-selective-enforcement.md) — Issue #346 (CI-OPT-06) 選択実行有効化 Before/After メトリクス記録（Shadow 期の wall/critical path/runner_total・macOS queue 滞留実測・After 未実測の明示と推定根拠・監視継続点。raw: `data/issue-346-enforcement-before-runs.jsonl`）
- [issue-193-migration-plan-update.md](issue-193-migration-plan-update.md) — Issue #193 migration plan update — Issue 状態 census と Snapshot 記述規則（3 repo Issue 状態の `gh` 実測・Blocked by 照合・版付き Snapshot の記述規則決定。証拠: `evidence/rf193/issue-state-census-2026-11-14.md`）
- [issue-532-dcomp-audit.md](issue-532-dcomp-audit.md) — Issue #532 DComp 監査文書チェーン 監査手法の試行記録（dep_kinds 解釈の再利用確認・CI 証拠 SHA 差分評価・Issue 依存状態の確認方法・生成 script 出力先と rename）
- `data/` — raw per-run measurement JSONL (`dsp-simd-poc-20261021-01.jsonl`, `-02.jsonl`; `rt-observability-poc-20261023-01.jsonl`, `-02.jsonl`, `-03.jsonl`; `rt-observability-poc-20261024-corrected-01.jsonl`, `-02.jsonl`, `-03.jsonl`; 1 line = 1 measurement point) plus corrected-run summary `rt-observability-poc-20261024-summary.json`, backend dependency snapshot `rt-observability-backend-metadata-20261024.json`, and capture script `rt-observability-capture.cjs`, plus `issue-327-ci-baseline-runs.jsonl` (Issue #327 cross-repo CI timing raw rows: 1 line = 1 record of run/job/step/run-metrics/cache/required-contexts) and `issue-332-playwright-browser-runs.jsonl` (Issue #332 browser-lane Before/After step timing raw rows: 1 line = 1 record of run/job/step) and `issue-331-verify-step-runs.jsonl` (Issue #331 verify-lane Before/After step timing raw rows: 1 line = 1 record of run/job/step) and `issue-333-impact-decision-run-41835836190.json` / `issue-333-gate-report-run-41835836190.json` (Issue #333 Shadow Mode artifact snapshots: impact-decision lane map and ci-gate verdict report for PR #345 run 41835836190) and `issue-334-shadow-classification-run-41840536142.jsonl` (Issue #334 Shadow Mode observation: selective decision, expected×actual lane verdicts and false-negative check for PR #347 run 41840536142; 1 line = 1 run summary or lane record) and `issue-335-shadow-classification-run-41848239157.jsonl` (Issue #335 Shadow Mode observation: selective decision, expected×actual lane verdicts and false-negative check for PR #348 run 41848239157; 1 line = 1 run summary or lane record) and `issue-346-enforcement-before-runs.jsonl` (Issue #346 selective-enforcement Before metrics raw rows: 1 line = 1 record of run/job/step/run-metrics for the last 15 ci runs before lane `if:` enablement). Not an impl-doc event log, so it lives in a subdirectory outside the flat `*.jsonl` event-log audit scope

<!-- doc-suite:generated-index -->

Directory: `docs/impl/exp`

| File |
| --- |
| [ac1-host-experiment-wry-tao-webview.jsonl](./ac1-host-experiment-wry-tao-webview.jsonl) |
| [ac2-render-canvas2d-webgl2-webgpu.jsonl](./ac2-render-canvas2d-webgl2-webgpu.jsonl) |
| [ac3-surface-adjacent-child.jsonl](./ac3-surface-adjacent-child.jsonl) |
| [ac3-surface-adjacent-parent.jsonl](./ac3-surface-adjacent-parent.jsonl) |
| [ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl](./ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl) |
| [ac5-frontend-compare.jsonl](./ac5-frontend-compare.jsonl) |
| [ac6-dist-single-binary.jsonl](./ac6-dist-single-binary.jsonl) |
| [ac7-adr-finalize.jsonl](./ac7-adr-finalize.jsonl) |
| [device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl](./device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl) |
| [dsp-simd-poc-benchmark-comparison.jsonl](./dsp-simd-poc-benchmark-comparison.jsonl) |
| [issue-327-ci-baseline.jsonl](./issue-327-ci-baseline.jsonl) |
| [issue-389-parent-lease-exp.jsonl](./issue-389-parent-lease-exp.jsonl) |
| [issue-558-script-contract.jsonl](./issue-558-script-contract.jsonl) |
| [issue-57-high-load-webgpu-multi-webview.jsonl](./issue-57-high-load-webgpu-multi-webview.jsonl) |
| [linux-container-device-and-arch-verification.jsonl](./linux-container-device-and-arch-verification.jsonl) |
| [plugin-gui-attach-e1-e2.jsonl](./plugin-gui-attach-e1-e2.jsonl) |
| [rt-observability-poc.jsonl](./rt-observability-poc.jsonl) |
