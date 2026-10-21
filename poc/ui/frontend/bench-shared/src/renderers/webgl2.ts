// WebGL2 バックエンド。
// ピーク列を TRIANGLE_STRIP（列ごとに [min,max] の頂点対、計 ~2,000 頂点）として
// 毎フレーム bufferData でアップロードして描画する（実際の音声 UI 相当の更新経路）。

import type { FrameData } from "../workload";
import type { Renderer } from "./types";

const VS = `#version 300 es
layout(location = 0) in vec2 a_pos;
void main() {
  gl_Position = vec4(a_pos, 0.0, 1.0);
}
`;

const FS = `#version 300 es
precision mediump float;
uniform vec4 u_color;
out vec4 out_color;
void main() {
  out_color = u_color;
}
`;

export class WebGl2Renderer implements Renderer {
  readonly backend = "webgl2" as const;
  private readonly gl: WebGL2RenderingContext;
  private readonly program: WebGLProgram;
  private readonly buffer: WebGLBuffer;
  private readonly verts: Float32Array;
  private readonly uColor: WebGLUniformLocation | null;

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl2", { alpha: false, antialias: true });
    if (!gl) {
      throw new Error("webgl2 context unavailable");
    }
    this.gl = gl;
    this.program = WebGl2Renderer.link(gl);
    this.buffer = gl.createBuffer()!;
    this.verts = new Float32Array(4000); // columns*2 頂点 * xy
    this.uColor = gl.getUniformLocation(this.program, "u_color");
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.disable(gl.DEPTH_TEST);
    gl.disable(gl.BLEND);
  }

  private static compile(
    gl: WebGL2RenderingContext,
    type: number,
    src: string
  ): WebGLShader {
    const sh = gl.createShader(type)!;
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      throw new Error(`shader compile failed: ${gl.getShaderInfoLog(sh)}`);
    }
    return sh;
  }

  private static link(gl: WebGL2RenderingContext): WebGLProgram {
    const vs = WebGl2Renderer.compile(gl, gl.VERTEX_SHADER, VS);
    const fs = WebGl2Renderer.compile(gl, gl.FRAGMENT_SHADER, FS);
    const prog = gl.createProgram()!;
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      throw new Error(`program link failed: ${gl.getProgramInfoLog(prog)}`);
    }
    return prog;
  }

  draw(frame: FrameData): void {
    const { gl, verts } = this;
    const { peaks, columns } = frame;
    // TRIANGLE_STRIP: 列ごとに (x,ymin) (x,ymax) の頂点対で塗りつぶし帯を構成。
    for (let i = 0; i < columns; i++) {
      const x = (i / (columns - 1)) * 2 - 1;
      verts[i * 4] = x;
      verts[i * 4 + 1] = peaks[i * 2];
      verts[i * 4 + 2] = x;
      verts[i * 4 + 3] = peaks[i * 2 + 1];
    }
    gl.clearColor(0.051, 0.067, 0.09, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(this.program);
    gl.uniform4f(this.uColor, 0.345, 0.651, 1.0, 1.0);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    gl.bufferData(gl.ARRAY_BUFFER, verts, gl.DYNAMIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, columns * 2);
  }
}

/** 機能検出用のコンテキスト有無プローブ（使い捨てオフスクリーン canvas）。 */
export function probeWebGl2(): boolean {
  try {
    const c = document.createElement("canvas");
    return c.getContext("webgl2") !== null;
  } catch {
    return false;
  }
}
