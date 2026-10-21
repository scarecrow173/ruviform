// @ui-poc/bench-shared — FW 非依存の共通部品（design「frontend/bench-shared」）。
// ワークロード生成・rAF フレーム統計・描画バックエンド・ipc ブリッジを提供する。

export { METER_CHANNELS, PEAK_COLUMNS, WorkloadGenerator } from "./workload";
export type { FrameData } from "./workload";
export { FrameRecorder, median, summarizeFrames } from "./stats";
export { createMeterHost } from "./meters";
export { installApi, postMessage } from "./ipc";
export { createRenderer } from "./renderers";
export type { BackendName, Renderer, RendererResult } from "./renderers";
export { BenchRunner, WARMUP_FRAMES } from "./runner";
export type { BenchElements, BenchState } from "./runner";
