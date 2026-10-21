// 描画バックエンド共通型（FW 非依存）。

import type { FrameData } from "../workload";

/** `--backend` が取りうる値。 */
export type BackendName = "canvas2d" | "webgl2" | "webgpu";

/** バックエンド共通の描画 I/F。 */
export interface Renderer {
  /** 実際に使用しているバックエンド（fallback 後は要求と異なりうる）。 */
  readonly backend: BackendName;
  /** 1 フレーム分の波形ピーク列を描画する。 */
  draw(frame: FrameData): void;
}

/** バックエンド作成結果。fallback 発生時は理由を証跡として残す。 */
export interface RendererResult {
  renderer: Renderer;
  /** 実際に使われたバックエンド。 */
  used: BackendName;
  /** WebGPU → WebGL2 フォールバック発生時の記録。未発生なら null。 */
  fallback: { to: BackendName; reason: string } | null;
  /** 機能検出の生結果（証跡用）。 */
  caps: {
    navigator_gpu: boolean;
    webgl2: boolean;
    webgpu_adapter: string | null;
  };
}
