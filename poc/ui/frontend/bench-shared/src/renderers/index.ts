// バックエンド選択 + WebGPU 機能検出フォールバック（design AC2 正本）。
// - canvas2d / webgl2: 直接作成。失敗は throw（計測不能 = 実験失敗）。
// - webgpu          : navigator.gpu → requestAdapter → requestDevice の順に検出し、
//                     失敗時は WebGL2 へフォールバック（fallback 自体は失敗としない。
//                     `fallback.to`/`reason` を done メッセージ経由で metrics に残す）。

import { Canvas2DRenderer } from "./canvas2d";
import { probeWebGl2, WebGl2Renderer } from "./webgl2";
import { WebGpuRenderer } from "./webgpu";
import type { BackendName, RendererResult } from "./types";

export type { BackendName, Renderer, RendererResult } from "./types";

/** 要求バックエンドの Renderer を生成する（webgpu のみ async 検出）。 */
export async function createRenderer(
  requested: BackendName,
  canvas: HTMLCanvasElement
): Promise<RendererResult> {
  const caps = {
    navigator_gpu:
      typeof navigator !== "undefined" &&
      "gpu" in (navigator as Navigator & { gpu?: unknown }),
    webgl2: probeWebGl2(),
    webgpu_adapter: null as string | null,
  };

  if (requested === "webgpu") {
    try {
      const { renderer, adapterName } = await WebGpuRenderer.create(canvas);
      caps.webgpu_adapter = adapterName;
      return { renderer, used: "webgpu", fallback: null, caps };
    } catch (e) {
      const reason = e instanceof Error ? e.message : String(e);
      // WebGL2 フォールバック。これも失敗したら致命的（throw → bench_error）。
      const renderer = new WebGl2Renderer(canvas);
      return {
        renderer,
        used: "webgl2",
        fallback: { to: "webgl2", reason },
        caps,
      };
    }
  }
  if (requested === "webgl2") {
    return {
      renderer: new WebGl2Renderer(canvas),
      used: "webgl2",
      fallback: null,
      caps,
    };
  }
  return {
    renderer: new Canvas2DRenderer(canvas),
    used: "canvas2d",
    fallback: null,
    caps,
  };
}
