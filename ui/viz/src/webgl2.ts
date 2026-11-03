/**
 * WebGL2 executor — the GPU backend for plot-layer ops.
 *
 * Runs `rect`/`rectStroke`/`band`/`line`/`poly`/`hatch`/clip ops through
 * one solid-color shader (position attribute + resolution/color
 * uniforms). Strokes are tessellated into quads so line width is not
 * limited by the platform's 1px `LINES` limit. `text` is a declared
 * overlay-layer capability (§12.5) — a text op reaching this executor
 * is a contract violation and is reported in `notes`, not drawn.
 *
 * Context-loss is surfaced to the element, which falls back to Canvas2D
 * — the executor itself never recreates the context.
 */

import { bandXs, hatchLines, type VizExecutor } from "./canvas2d.ts";
import { parseCssColor } from "./color.ts";
import type { DrawOp } from "./ops.ts";

const VS = `#version 300 es
in vec2 a_pos;
uniform vec2 u_res;
void main() {
  vec2 clip = vec2(a_pos.x / u_res.x * 2.0 - 1.0,
                   1.0 - a_pos.y / u_res.y * 2.0);
  gl_Position = vec4(clip, 0.0, 1.0);
}
`;

const FS = `#version 300 es
precision mediump float;
uniform vec4 u_color;
out vec4 o_color;
void main() { o_color = u_color; }
`;

export class WebGl2Executor implements VizExecutor {
  readonly kind = "webgl2" as const;
  readonly notes: string[] = [];

  private readonly program: WebGLProgram;
  private readonly buffer: WebGLBuffer;
  private readonly uRes: WebGLUniformLocation | null;
  private readonly uColor: WebGLUniformLocation | null;
  private readonly gl: WebGL2RenderingContext;
  private readonly width: number;
  private readonly height: number;
  private readonly dpr: number;
  private verts: Float32Array = new Float32Array(4096);
  private n = 0;
  private clipStack: (readonly [number, number, number, number])[] = [];

  constructor(
    gl: WebGL2RenderingContext,
    width: number,
    height: number,
    dpr: number,
  ) {
    this.gl = gl;
    this.width = width;
    this.height = height;
    this.dpr = dpr;
    this.program = link(gl);
    this.buffer = gl.createBuffer() ?? fail("createBuffer");
    this.uRes = gl.getUniformLocation(this.program, "u_res");
    this.uColor = gl.getUniformLocation(this.program, "u_color");
    const aPos = gl.getAttribLocation(this.program, "a_pos");
    gl.useProgram(this.program);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);
    gl.uniform2f(this.uRes, width, height);
    gl.viewport(0, 0, width * dpr, height * dpr);
    gl.disable(gl.DEPTH_TEST);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  }

  execute(ops: readonly DrawOp[]): void {
    const { gl } = this;
    this.notes.length = 0;
    this.clipStack.length = 0;
    gl.viewport(0, 0, this.width * this.dpr, this.height * this.dpr);
    gl.useProgram(this.program);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    for (const op of ops) this.exec(op);
    if (this.clipStack.length > 0) {
      this.notes.push("clip-unbalanced: clipPush without clipPop");
      gl.disable(gl.SCISSOR_TEST);
      this.clipStack.length = 0;
    }
  }

  private exec(op: DrawOp): void {
    const { gl } = this;
    switch (op.kind) {
      case "rect":
        this.n = 0;
        this.vert(op.x, op.y);
        this.vert(op.x + op.w, op.y);
        this.vert(op.x + op.w, op.y + op.h);
        this.vert(op.x, op.y);
        this.vert(op.x + op.w, op.y + op.h);
        this.vert(op.x, op.y + op.h);
        this.draw(gl.TRIANGLES, op.fill);
        break;
      case "rectStroke":
        this.strokeRect(op.x, op.y, op.w, op.h, op.width, op.stroke);
        break;
      case "band": {
        const xs = bandXs(op);
        const n = Math.min(op.top.length, op.bottom.length, xs.length);
        if (n === 0) break;
        this.n = 0;
        for (let i = 0; i < n; i++) {
          this.push(xs[i] ?? 0, op.top[i] ?? 0, xs[i] ?? 0, op.bottom[i] ?? 0);
        }
        this.draw(gl.TRIANGLE_STRIP, op.fill, n * 2);
        break;
      }
      case "line":
        this.strokePolyline(op.pts, op.width, op.stroke, false);
        break;
      case "poly": {
        if (op.pts.length < 6) break;
        this.n = 0;
        for (let i = 0; i < op.pts.length; i += 2) {
          this.vert(op.pts[i] ?? 0, op.pts[i + 1] ?? 0);
        }
        this.draw(gl.TRIANGLE_FAN, op.fill, op.pts.length / 2);
        break;
      }
      case "hatch":
        // hatchLines extends segments well past the rect (the 2D path
        // relies on ctx.clip for bounds) — GL needs the same per-op
        // scissor, nested inside any ambient clipPush.
        this.pushClip(op.x, op.y, op.w, op.h);
        this.n = 0;
        for (const [x0, y0, x1, y1] of hatchLines(op)) {
          this.seg(x0, y0, x1, y1, op.width);
        }
        this.draw(gl.TRIANGLES, op.stroke);
        this.popClip();
        break;
      case "text":
        this.notes.push(
          "text op reached the WebGL2 executor — overlay partition violation",
        );
        break;
      case "clipPush":
        this.pushClip(op.x, op.y, op.w, op.h);
        break;
      case "clipPop":
        this.popClip();
        break;
    }
  }

  // ── geometry helpers ──────────────────────────────────────────────

  private grow(need: number): void {
    if (this.n + need <= this.verts.length) return;
    const next = new Float32Array(
      Math.max(this.verts.length * 2, this.n + need),
    );
    next.set(this.verts);
    this.verts = next;
  }

  private vert(x: number, y: number): void {
    this.grow(2);
    this.verts[this.n] = x;
    this.verts[this.n + 1] = y;
    this.n += 2;
  }

  private push(x0: number, y0: number, x1: number, y1: number): void {
    this.vert(x0, y0);
    this.vert(x1, y1);
  }

  /** Axis-aligned stroke box → four tessellated edge segments. */
  private strokeRect(
    x: number,
    y: number,
    w: number,
    h: number,
    width: number,
    color: string,
  ): void {
    this.n = 0;
    const t = width / 2;
    this.seg(x, y - t, x + w, y - t, width);
    this.seg(x + w, y - t, x + w, y + h + t, width);
    this.seg(x + w, y + h + t, x, y + h + t, width);
    this.seg(x, y + h + t, x, y - t, width);
    this.draw(this.gl.TRIANGLES, color);
  }

  private strokePolyline(
    pts: readonly number[],
    width: number,
    color: string,
    closed: boolean,
  ): void {
    const count = pts.length / 2;
    if (count < 2) return;
    this.n = 0;
    const end = closed ? count : count - 1;
    for (let i = 0; i < end; i++) {
      const j = (i + 1) % count;
      this.seg(
        pts[i * 2] ?? 0,
        pts[i * 2 + 1] ?? 0,
        pts[j * 2] ?? 0,
        pts[j * 2 + 1] ?? 0,
        width,
      );
    }
    this.draw(this.gl.TRIANGLES, color);
  }

  /** One stroked segment → two triangles (butt caps). */
  private seg(
    x0: number,
    y0: number,
    x1: number,
    y1: number,
    width: number,
  ): void {
    const dx = x1 - x0;
    const dy = y1 - y0;
    const len = Math.hypot(dx, dy) || 1;
    const nx = (-dy / len) * (width / 2);
    const ny = (dx / len) * (width / 2);
    this.vert(x0 + nx, y0 + ny);
    this.vert(x0 - nx, y0 - ny);
    this.vert(x1 - nx, y1 - ny);
    this.vert(x0 + nx, y0 + ny);
    this.vert(x1 - nx, y1 - ny);
    this.vert(x1 + nx, y1 + ny);
  }

  private draw(mode: number, color: string, count?: number): void {
    const { gl } = this;
    const c = parseCssColor(color);
    if (c === undefined) {
      this.notes.push(`color-unparseable: ${JSON.stringify(color)}`);
      gl.uniform4f(this.uColor, 0.5, 0.5, 0.5, 1);
    } else {
      gl.uniform4f(this.uColor, c.r, c.g, c.b, c.a);
    }
    gl.bufferData(gl.ARRAY_BUFFER, this.verts.subarray(0, this.n), gl.DYNAMIC_DRAW);
    gl.drawArrays(mode, 0, count ?? this.n / 2);
    this.n = 0;
  }

  // ── clip stack (GL scissor) ───────────────────────────────────────

  private pushClip(x: number, y: number, w: number, h: number): void {
    const { gl } = this;
    const top = this.clipStack[this.clipStack.length - 1];
    let r: readonly [number, number, number, number] = [x, y, w, h];
    if (top !== undefined) {
      const x0 = Math.max(x, top[0]);
      const y0 = Math.max(y, top[1]);
      const x1 = Math.min(x + w, top[0] + top[2]);
      const y1 = Math.min(y + h, top[1] + top[3]);
      r = [x0, y0, Math.max(0, x1 - x0), Math.max(0, y1 - y0)];
    }
    this.clipStack.push(r);
    this.applyClip(r);
  }

  private popClip(): void {
    this.clipStack.pop();
    const top = this.clipStack[this.clipStack.length - 1];
    if (top === undefined) {
      this.gl.disable(this.gl.SCISSOR_TEST);
    } else {
      this.applyClip(top);
    }
  }

  private applyClip(r: readonly [number, number, number, number]): void {
    const { gl, dpr } = this;
    gl.enable(gl.SCISSOR_TEST);
    const y = this.height - r[1] - r[3]; // CSS px y-down → GL y-up
    gl.scissor(r[0] * dpr, y * dpr, r[2] * dpr, r[3] * dpr);
  }
}

function fail(what: string): never {
  throw new Error(`webgl2: ${what} failed`);
}

function compile(
  gl: WebGL2RenderingContext,
  type: number,
  src: string,
): WebGLShader {
  const sh = gl.createShader(type) ?? fail("createShader");
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (gl.getShaderParameter(sh, gl.COMPILE_STATUS) !== true) {
    throw new Error(`webgl2 shader compile: ${gl.getShaderInfoLog(sh) ?? ""}`);
  }
  return sh;
}

function link(gl: WebGL2RenderingContext): WebGLProgram {
  const vs = compile(gl, gl.VERTEX_SHADER, VS);
  const fs = compile(gl, gl.FRAGMENT_SHADER, FS);
  const prog = gl.createProgram() ?? fail("createProgram");
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  gl.linkProgram(prog);
  if (gl.getProgramParameter(prog, gl.LINK_STATUS) !== true) {
    throw new Error(`webgl2 program link: ${gl.getProgramInfoLog(prog) ?? ""}`);
  }
  return prog;
}
