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

## Reports and raw data

- [dsp-simd-poc-20261021.md](dsp-simd-poc-20261021.md) — Issue #19 DSP/SIMD PoC bench measurement report
- `data/` — raw per-run measurement JSONL (`dsp-simd-poc-20261021-01.jsonl`, `-02.jsonl`; 1 line = 1 measurement point). Not an impl-doc event log, so it lives in a subdirectory outside the flat `*.jsonl` event-log audit scope
