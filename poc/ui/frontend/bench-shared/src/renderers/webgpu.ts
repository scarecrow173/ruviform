// WebGPU バックエンド。
// `navigator.gpu` + adapter 要求の機能検出を必須とし、失敗時は呼び出し側が
// WebGL2 へフォールバックする（design AC2「フォールバック込み評価」）。
// ピーク列は TRIANGLE_STRIP（~2,000 頂点）を毎フレーム writeBuffer で更新して描画。
// WebGPU の DOM 型は TS lib.dom 未同梱のため @webgpu/types を参照する
// （本ファイルのみが GPU 型を使うため局所 reference で足りる）。

/// <reference types="@webgpu/types" />

import type { FrameData } from "../workload";
import type { Renderer } from "./types";

const WGSL = `
@vertex fn vs(@location(0) p: vec2f) -> @builtin(position) vec4f {
  return vec4f(p, 0.0, 1.0);
}
@fragment fn fs() -> @location(0) vec4f {
  return vec4f(0.345, 0.651, 1.0, 1.0);
}
`;

const VERTEX_COUNT = 2000; // columns*2
const BUFFER_BYTES = VERTEX_COUNT * 2 * 4; // xy * f32

export class WebGpuRenderer implements Renderer {
  readonly backend = "webgpu" as const;

  private constructor(
    private readonly device: GPUDevice,
    private readonly context: GPUCanvasContext,
    private readonly pipeline: GPURenderPipeline,
    private readonly buffer: GPUBuffer,
    private readonly verts: Float32Array<ArrayBuffer>
  ) {}

  /**
   * 機能検出 + 初期化。`navigator.gpu` 不在・adapter 要求失敗・
   * device 要求失敗・canvas context 取得失敗はすべて throw となり、
   * 呼び出し側の WebGL2 フォールバックへ回される。
   */
  static async create(
    canvas: HTMLCanvasElement
  ): Promise<{ renderer: WebGpuRenderer; adapterName: string | null }> {
    const nav = navigator as Navigator & { gpu?: GPU };
    if (!nav.gpu) {
      throw new Error("navigator.gpu unavailable");
    }
    const adapter = await nav.gpu.requestAdapter();
    if (!adapter) {
      throw new Error("webgpu adapter request returned null");
    }
    const adapterName = adapterNameOf(adapter);
    const device = await adapter.requestDevice();
    const context = canvas.getContext("webgpu") as GPUCanvasContext | null;
    if (!context) {
      throw new Error("webgpu canvas context unavailable");
    }
    const format = nav.gpu.getPreferredCanvasFormat();
    context.configure({ device, format, alphaMode: "opaque" });

    const module = device.createShaderModule({ code: WGSL });
    const pipeline = device.createRenderPipeline({
      layout: "auto",
      vertex: {
        module,
        entryPoint: "vs",
        buffers: [
          {
            arrayStride: 8,
            attributes: [{ shaderLocation: 0, offset: 0, format: "float32x2" }],
          },
        ],
      },
      fragment: { module, entryPoint: "fs", targets: [{ format }] },
      primitive: { topology: "triangle-strip" },
    });
    const buffer = device.createBuffer({
      size: BUFFER_BYTES,
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    });
    return {
      renderer: new WebGpuRenderer(
        device,
        context,
        pipeline,
        buffer,
        new Float32Array(VERTEX_COUNT * 2)
      ),
      adapterName,
    };
  }

  draw(frame: FrameData): void {
    const { device, context, pipeline, buffer, verts } = this;
    const { peaks, columns } = frame;
    const n = columns * 2;
    for (let i = 0; i < columns; i++) {
      const x = (i / (columns - 1)) * 2 - 1;
      verts[i * 4] = x;
      verts[i * 4 + 1] = peaks[i * 2];
      verts[i * 4 + 2] = x;
      verts[i * 4 + 3] = peaks[i * 2 + 1];
    }
    device.queue.writeBuffer(buffer, 0, verts, 0, n * 2);
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: context.getCurrentTexture().createView(),
          loadOp: "clear",
          clearValue: { r: 0.051, g: 0.067, b: 0.09, a: 1 },
          storeOp: "store",
        },
      ],
    });
    pass.setPipeline(pipeline);
    pass.setVertexBuffer(0, buffer);
    pass.draw(n);
    pass.end();
    device.queue.submit([encoder.finish()]);
  }
}

/** adapter 情報の証跡用文字列（GPUAdapterInfo は実装により欠落しうる）。 */
function adapterNameOf(adapter: GPUAdapter): string | null {
  const info = adapter.info as
    | { description?: string; device?: string; vendor?: string; architecture?: string }
    | undefined;
  if (!info) return null;
  const name = info.description ?? info.device;
  const suffix = [info.vendor, info.architecture]
    .filter((s): s is string => typeof s === "string" && s.length > 0)
    .join("/");
  if (!name && !suffix) return null;
  return name ? (suffix ? `${name} (${suffix})` : name) : `(${suffix})`;
}
