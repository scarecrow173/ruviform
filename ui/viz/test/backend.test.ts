/**
 * Backend tests — executors + selection/fallback contract.
 *
 * Headless scope: the Canvas2D executor drives a recording fake 2D
 * context; the WebGL2 executor drives a recording fake GL; backend
 * selection runs against stubbed `getContext`. Real GPU rendering is
 * NOT claimed here — geometry is verified at the op→call boundary, the
 * layer the contract actually owns.
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { createVizBackend } from "../src/backend.ts";
import { Canvas2DExecutor, bandXs } from "../src/canvas2d.ts";
import { WebGl2Executor } from "../src/webgl2.ts";
import { partitionOps, type DrawOp } from "../src/ops.ts";
import { Fake2DContext, FakeGL, installFakeCanvas } from "./env.ts";

const ops: DrawOp[] = [
  { kind: "rect", layer: "plot", x: 0, y: 0, w: 10, h: 10, fill: "#111111" },
  {
    kind: "rectStroke", layer: "plot", x: 0, y: 0, w: 10, h: 10,
    stroke: "#222222", width: 1,
  },
  {
    kind: "band", layer: "plot",
    span: { x0: 0, dx: 2 },
    top: [1, 2, 3], bottom: [4, 5, 6], fill: "#333333",
  },
  {
    kind: "band", layer: "plot",
    span: { xs: [0, 3, 9] },
    top: [1, 2, 3], bottom: [4, 5, 6], fill: "#333333",
  },
  {
    kind: "line", layer: "plot", pts: [0, 0, 5, 5, 10, 0],
    stroke: "#444444", width: 1.5,
  },
  {
    kind: "poly", layer: "plot", pts: [0, 0, 4, 0, 2, 4], fill: "#555555",
  },
  {
    kind: "hatch", layer: "plot", x: 0, y: 0, w: 20, h: 10,
    stroke: "#666666", width: 1, spacing: 4, angle: Math.PI / 4,
  },
  { kind: "clipPush", layer: "plot", x: 1, y: 1, w: 8, h: 8 },
  {
    kind: "text", layer: "overlay", x: 5, y: 5, text: "hi",
    fill: "#777777", font: "12px sans", align: "left", baseline: "top",
  },
  { kind: "clipPop", layer: "plot" },
];

describe("Canvas2DExecutor", () => {
  it("runs every op kind on the 2D context", () => {
    const ctx = new Fake2DContext();
    const ex = new Canvas2DExecutor(
      ctx as unknown as CanvasRenderingContext2D,
      100,
      50,
      2,
    );
    ex.execute(ops);
    assert.ok(ctx.count("fillRect") >= 1);
    assert.ok(ctx.count("strokeRect") >= 1);
    assert.ok(ctx.count("fill") >= 2); // bands + poly
    assert.ok(ctx.count("stroke") >= 2); // line + hatch
    assert.equal(ctx.count("fillText"), 1);
    assert.ok(ctx.count("clip") >= 2); // clipPush + hatch's own clip
    assert.equal(ctx.calls[0]?.method, "save");
    assert.equal(ctx.calls.at(-1)?.method, "restore");
  });

  it("bandXs covers uniform and explicit spans", () => {
    const u = ops.find((o) => o.kind === "band" && "x0" in o.span);
    const e = ops.find((o) => o.kind === "band" && "xs" in o.span);
    assert.ok(u !== undefined && u.kind === "band");
    assert.ok(e !== undefined && e.kind === "band");
    assert.deepEqual(bandXs(u), [0, 2, 4]);
    assert.deepEqual(bandXs(e), [0, 3, 9]);
  });
});

describe("WebGl2Executor", () => {
  it("translates ops into GL calls; text stays off GL", () => {
    const gl = new FakeGL();
    const ex = new WebGl2Executor(
      gl as unknown as WebGL2RenderingContext,
      100,
      50,
      1,
    );
    const { plot, overlay } = partitionOps(ops);
    ex.execute(plot);
    assert.ok(gl.count("drawArrays") >= 5);
    assert.ok(gl.count("scissor") >= 1); // clipPush → scissor
    assert.ok(gl.count("uniform4f") >= 5); // per-primitive color
    assert.deepEqual(overlay.map((o) => o.kind), ["text"]);
  });

  it("unparseable colors produce notes, not silent black", () => {
    const gl = new FakeGL();
    const ex = new WebGl2Executor(
      gl as unknown as WebGL2RenderingContext,
      100,
      50,
      1,
    );
    ex.execute([
      {
        kind: "rect", layer: "plot", x: 0, y: 0, w: 5, h: 5,
        fill: "var(--unresolvable)",
      },
    ]);
    assert.ok(ex.notes.some((n) => n.startsWith("color-unparseable")));
  });

  it("a stray overlay-layer text op is reported, not drawn", () => {
    const gl = new FakeGL();
    const ex = new WebGl2Executor(
      gl as unknown as WebGL2RenderingContext,
      100,
      50,
      1,
    );
    ex.execute(ops); // unpartitioned — text reaches GL executor
    assert.ok(ex.notes.some((n) => n.includes("text op")));
  });
});

describe("createVizBackend", () => {
  it("canvas2d preference → single-ctx executor, no overlay canvas", () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const canvas = document.createElement("canvas");
      const b = createVizBackend(
        { plot: canvas, overlay: null },
        "canvas2d",
        { width: 100, height: 50, dpr: 1 },
      );
      assert.ok(b !== null);
      assert.equal(b.kind, "canvas2d");
      assert.equal(b.needsOverlayCanvas, false);
      assert.equal(b.fallback, null);
      b.execute(ops);
    } finally {
      fake.restore();
    }
  });

  it("auto prefers webgl2, routes overlay ops to the 2D canvas", () => {
    const fake = installFakeCanvas({ contexts: "both" });
    try {
      const plot = document.createElement("canvas");
      const overlay = document.createElement("canvas");
      const b = createVizBackend(
        { plot, overlay },
        "auto",
        { width: 100, height: 50, dpr: 1 },
      );
      assert.ok(b !== null);
      assert.equal(b.kind, "webgl2");
      assert.equal(b.needsOverlayCanvas, true);
      assert.equal(b.capabilities.text, "overlay");
      b.execute(ops);
      const gl = fake.contexts.get("webgl2") as FakeGL;
      const ctx2d = fake.contexts.get("2d") as Fake2DContext;
      assert.ok(gl.count("drawArrays") >= 5);
      assert.equal(ctx2d.count("fillText"), 1); // overlay text → 2d
    } finally {
      fake.restore();
    }
  });

  it("webgl2 unavailable → falls back to canvas2d with a reason", () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const plot = document.createElement("canvas");
      const overlay = document.createElement("canvas");
      const b = createVizBackend(
        { plot, overlay },
        "webgl2",
        { width: 100, height: 50, dpr: 1 },
      );
      assert.ok(b !== null);
      assert.equal(b.kind, "canvas2d");
      assert.ok(b.fallback !== null);
      assert.equal(b.fallback.requested, "webgl2");
    } finally {
      fake.restore();
    }
  });

  it("no usable context → null (element's DOM status path)", () => {
    const fake = installFakeCanvas({ contexts: "none" });
    try {
      const canvas = document.createElement("canvas");
      const b = createVizBackend(
        { plot: canvas, overlay: canvas },
        "auto",
        { width: 100, height: 50, dpr: 1 },
      );
      assert.equal(b, null);
    } finally {
      fake.restore();
    }
  });
});
