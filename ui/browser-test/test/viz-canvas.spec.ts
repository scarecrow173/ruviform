/**
 * Canvas2D / WebGL2 capability + DPR/resize evidence (Issue #310 AC3).
 *
 * The viz stack selects a real backend in a real browser — the
 * waveform is pinned `canvas2d` (pixel probes need getImageData);
 * meter/spectrum stay `auto` (whatever the browser picks is the
 * capability record). Either way the evidence is recorded in
 * `stats()`/`evidence.json` — never silently faked.
 */

import { test, expect } from "../src/index.ts";
import {
  control,
  drawAll,
  expectDrawOps,
  expectNonBlank,
  vizStats,
} from "../src/index.ts";

const WF = 'rvf-waveform[data-viz="waveform"]';

test("viz backend is real and draws ops — Canvas2D pixels are non-blank", async ({
  rvf,
}) => {
  const session = await rvf.open("analysis");
  await drawAll(session);
  await session.waitForState((api) => api.queries().length > 0, {
    label: "first waveform query",
  });
  await drawAll(session); // paint the arrived view + snapshots

  const stats = await vizStats(session);
  for (const key of ["waveform", "meter", "spectrum"]) {
    const st = stats[key];
    expect(st, `viz "${key}" registered`).toBeTruthy();
    // Real browser: canvas2d or webgl2 — never the headless fake.
    expect(["canvas2d", "webgl2"]).toContain(st?.backendKind);
    await expectDrawOps(session, key);
  }

  // The waveform element is pinned to Canvas2D — its plot pixels are
  // directly readable, and non-blank after real data arrives.
  const wf = control(session, WF);
  const probe = await expectNonBlank(wf);
  expect(probe.readable).toBe(true);
});

test("DPR sizing: backing store is css size × devicePixelRatio", async ({
  rvf,
}) => {
  // DPR=1 baseline (config default).
  const session = await rvf.open("analysis");
  const wf = control(session, WF);
  const dims = await wf.part("plot-canvas").evaluate((el) => ({
    w: (el as HTMLCanvasElement).width,
    h: (el as HTMLCanvasElement).height,
    dpr: (el as HTMLCanvasElement).ownerDocument.defaultView
      ?.devicePixelRatio,
  }));
  expect(dims.w).toBe(Math.round(300 * (dims.dpr ?? 1)));
  expect(dims.h).toBe(Math.round(120 * (dims.dpr ?? 1)));
});

test("resize re-queries the waveform (plot px is the LOD budget)", async ({
  rvf,
}) => {
  const session = await rvf.open("analysis");
  await drawAll(session);
  await session.waitForState((api) => api.queries().length > 0, {
    label: "first query",
  });
  const before = (await session.queries()) as Array<{ buckets: number }>;
  const bucketsBefore = before.at(-1)?.buckets;

  // `onResize` invalidates against the still-current layout, so the
  // new-budget query is issued once the resized layout is live — a
  // second size change then triggers a fresh, differently-sized query.
  await session.setVizSize("waveform", 420, 120);
  await drawAll(session);
  await session.setVizSize("waveform", 440, 120);
  await session.waitForState(
    (api, n) => api.queries().length > n,
    [before.length],
    { label: "re-query after resize" },
  );
  const after = (await session.queries()) as Array<{ buckets: number }>;
  expect(after.at(-1)?.buckets).not.toBe(bucketsBefore);

  await drawAll(session);
  const st = await vizStats(session);
  expect(st["waveform"]?.framesDrawn).toBeGreaterThan(0);
});

test.describe("dpr2 context", () => {
  test.use({ deviceScaleFactor: 2 });
  test("canvas backing store scales with devicePixelRatio=2", async ({
    rvf,
  }) => {
    const session = await rvf.open("analysis");
    const wf = control(session, WF);
    const dims = await wf.part("plot-canvas").evaluate((el) => ({
      w: (el as HTMLCanvasElement).width,
      dpr: (el as HTMLCanvasElement).ownerDocument.defaultView
        ?.devicePixelRatio,
    }));
    expect(dims.dpr).toBe(2);
    expect(dims.w).toBe(600);
  });
});
