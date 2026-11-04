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
- `data/` — raw per-run measurement JSONL (`dsp-simd-poc-20261021-01.jsonl`, `-02.jsonl`; `rt-observability-poc-20261023-01.jsonl`, `-02.jsonl`, `-03.jsonl`; `rt-observability-poc-20261024-corrected-01.jsonl`, `-02.jsonl`, `-03.jsonl`; 1 line = 1 measurement point) plus corrected-run summary `rt-observability-poc-20261024-summary.json`, backend dependency snapshot `rt-observability-backend-metadata-20261024.json`, and capture script `rt-observability-capture.cjs`, plus `issue-327-ci-baseline-runs.jsonl` (Issue #327 cross-repo CI timing raw rows: 1 line = 1 record of run/job/step/run-metrics/cache/required-contexts). Not an impl-doc event log, so it lives in a subdirectory outside the flat `*.jsonl` event-log audit scope
