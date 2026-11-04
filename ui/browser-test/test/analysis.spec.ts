/**
 * Analysis-shaped consumer fixture (Issue #310 AC3+4 — "Consumer B").
 *
 * `mountAnalysisFixture`: waveform over a bounded-query source +
 * meter/spectrum over the snapshot port + a QueryBinding on the
 * DemoAnalysisBackend. Semantically verifiable: bounded query shape,
 * NaN gap regions (explicit cue, never zero-filled), stale replies
 * never painted as current, viewport pan emitting `rvf-viewport` +
 * re-querying, and long-form revision bumps re-pulling data.
 */

import { test, expect } from "../src/index.ts";
import {
  colorAffinity,
  control,
  drawAll,
  tokenValue,
  vizStat,
} from "../src/index.ts";

const WF = 'rvf-waveform[data-viz="waveform"]';

interface QueryRec {
  start: number;
  end: number;
  buckets: number;
  channels?: number[];
}

test("waveform issues a bounded query sized to the plot budget", async ({
  rvf,
}) => {
  const session = await rvf.open("analysis");
  await drawAll(session);
  await session.waitForState((api) => api.queries().length > 0, {
    label: "first waveform query",
  });

  const queries = (await session.queries()) as QueryRec[];
  const q = queries[0];
  // Bounded: buckets ≈ plot px (300 css), never an unbounded raw pull.
  expect(q?.buckets).toBeGreaterThan(0);
  expect(q?.buckets).toBeLessThanOrEqual(400);
  expect(q?.end).toBeGreaterThan(q?.start ?? 0);
});

test("NaN dropout is an explicit gap cue — never a zero-filled line", async ({
  rvf,
}) => {
  const session = await rvf.open("analysis");
  const wf = control(session, WF);
  await drawAll(session);
  await session.waitForState((api) => api.queries().length > 0, {
    label: "waveform view queried",
  });
  await drawAll(session); // paint the arrived view

  const stroke = await tokenValue(wf.host, "--rvf-viz-waveform-stroke");
  const gapColor = await tokenValue(wf.host, "--rvf-viz-gap");
  const bg = await tokenValue(wf.host, "--rvf-viz-background");
  expect(stroke).not.toBe("");
  expect(gapColor).not.toBe("");
  expect(bg).not.toBe("");

  // Probe in *plot* space: DEMO_GAP spans 45–55% of the data extent
  // and the viewport fits the full extent, so the dropout maps to
  // plot-x .45–.55 of the channel-0 lane (top lane of two).
  // Strokes are anti-aliased composites over the plot backdrop —
  // affinity, not exact color, is the "is this ink here" oracle.
  const lane0 = { y0: 0.05, y1: 0.42 };
  const gapBand = { x0: 0.47, x1: 0.53 };
  const dataBand = { x0: 0.15, x1: 0.4 };

  const dataStroke = await colorAffinity(
    wf, { ...dataBand, ...lane0 }, stroke, [bg, gapColor], "plot",
  );
  expect(dataStroke.readable).toBe(true);
  // The series stroke paints through the data region…
  expect(
    dataStroke.matching,
    "series stroke must be painted through the data region",
  ).toBeGreaterThan(0);

  // …but never through the gap — a zero-fill bug would continue the
  // polyline at y=0 across the dropout.
  const gapStroke = await colorAffinity(
    wf, { ...gapBand, ...lane0 }, stroke, [bg, gapColor], "plot",
  );
  expect(
    gapStroke.matching,
    `series stroke must not cross the NaN gap (got ${gapStroke.matching}px)`,
  ).toBe(0);

  // The gap is marked, not silently empty: the dedicated gap-cue
  // color (hatch strokes) is present inside the dropout.
  const gapCue = await colorAffinity(
    wf, { ...gapBand, ...lane0 }, gapColor, [bg, stroke], "plot",
  );
  expect(
    gapCue.matching,
    "explicit gap cue must be painted inside the dropout",
  ).toBeGreaterThan(0);
});

test("stale reply is discarded — diagnostics count it, view never claims it", async ({
  rvf,
}) => {
  const session = await rvf.open("analysis");

  await session.issueQuery({
    sourceId: "analysis.wf",
    sourceRevision: 0,
    start: 0,
    end: 1,
    buckets: 32,
  });
  await session.waitForState((api) => api.pendingQueries() > 0, {
    label: "query parked",
  });
  // The source advanced while the reply was in flight → stale.
  await session.setBackendRevision(9);
  const qid = ((await session.backendQueries()) as Array<{ queryId: string }>)
    .at(-1)?.queryId;
  expect(qid).toBeTruthy();
  await session.resolveQuery(qid ?? "", { payload: { answer: 1 } });
  await session.waitForState((api) => api.queryOutcomes().length > 0, {
    label: "outcome recorded",
  });

  const outcomes = (await session.queryOutcomes()) as Array<{
    ok: boolean;
    error?: { kind: string };
  }>;
  expect(outcomes.at(-1)?.ok).toBe(false);
  expect(outcomes.at(-1)?.error?.kind).toBe("stale");
  const d = await session.queryDiagnostics();
  expect(d["stale"]).toBe(1);
  expect(d["resolved"]).toBe(0);
});

test("rejected reply is an error outcome, not data", async ({ rvf }) => {
  const session = await rvf.open("analysis");
  await session.issueQuery({
    sourceId: "analysis.wf",
    sourceRevision: 0,
    start: 0,
    end: 1,
    buckets: 16,
  });
  await session.waitForState((api) => api.pendingQueries() > 0);
  const qid = ((await session.backendQueries()) as Array<{ queryId: string }>)
    .at(-1)?.queryId;
  await session.rejectQuery(qid ?? "");
  await session.waitForState((api) => api.queryOutcomes().length > 0);
  const outcomes = (await session.queryOutcomes()) as Array<{
    ok: boolean;
    error?: { kind: string };
  }>;
  expect(outcomes.at(-1)?.error?.kind).toBe("error");
});

test("pointer pan emits rvf-viewport and re-queries the new span", async ({
  rvf,
}) => {
  const session = await rvf.open("analysis");
  await drawAll(session);
  await session.waitForState((api) => api.queries().length > 0, {
    label: "layout + first query",
  });
  const wf = control(session, WF);
  const before = (await session.queries()).length;

  await wf.dragBegin(0.5, "plot-canvas");
  await wf.dragMoveTo(0.3, 3);
  await wf.dragEnd();

  const evts = await session.events("rvf-viewport");
  expect(evts.length).toBeGreaterThan(0);
  const detail = evts.at(-1)?.detail as { viewport?: { start: number } };
  // Dragging left pans the viewport forward into the data (start > 0).
  expect(detail?.viewport?.start).toBeGreaterThan(0);
  // The pan invalidated the view — a fresh bounded query was issued.
  const after = (await session.queries()) as QueryRec[];
  expect(after.length).toBeGreaterThan(before);
  expect(after.at(-1)?.start).toBeGreaterThan(0);
});

test("source revision bump re-queries; the stale view is never drawn as current", async ({
  rvf,
}) => {
  const session = await rvf.open("analysis");
  await drawAll(session);
  await session.waitForState((api) => api.queries().length > 0, {
    label: "first query",
  });
  const before = (await session.queries()).length;
  const meta0 = (await session.waveformMeta("waveform")) as {
    revision: number;
  };
  expect(meta0.revision).toBe(0);

  // Long-form publish: new samples, new revision.
  await session.replaceWaveform(
    "waveform",
    Array.from({ length: 4096 }, (_, i) => Math.sin(i / 20)),
  );
  const meta1 = (await session.waveformMeta("waveform")) as {
    revision: number;
    extent: number;
  };
  expect(meta1.revision).toBe(1);
  expect(meta1.extent).toBeGreaterThan(0);

  const after = (await session.queries()) as QueryRec[];
  expect(after.length).toBeGreaterThan(before);
  await drawAll(session);
  const st = await vizStat(session, "waveform");
  expect(st.framesDrawn).toBeGreaterThan(0);
  expect(st.statusText).not.toContain("stale");
});
