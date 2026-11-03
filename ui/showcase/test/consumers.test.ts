/**
 * Cross-consumer gates (Issue #263): Synvelis-shaped (Consumer A) and
 * AudioLens-shaped (Consumer B) fixtures exercise only the public
 * APIs — no product types enter Ruviform interfaces. Commands/queries
 * flow through generic BindingPort contracts; receipt lifecycle,
 * bounded queries, and dock-transfer generation semantics are
 * asserted headlessly.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  mountAnalysisFixture,
  mountDawFixture,
} from "../src/index.ts";
import {
  collectGestures,
  flushMicrotasks,
  flushSync,
  typesOf,
} from "./helpers.ts";
import { installFakeCanvas } from "./env.ts";

describe("Consumer A — DAW-shaped fader/transport/automation", () => {
  it("fader gesture → gateway command → applied receipt → reflection", async () => {
    const fx = mountDawFixture(document.body, { flushWith: flushSync });
    const el = fx.fader as HTMLElement & {
      dispatchInput(i: unknown): void;
      value?: number;
      hasAttribute(n: string): boolean;
    };
    const gestures = collectGestures(fx.fader);
    el.dispatchInput({ type: "begin", gestureId: "g1", source: "pointer" });
    el.dispatchInput({
      type: "preview",
      gestureId: "g1",
      value: { plain: 0.7 },
    });
    el.dispatchInput({ type: "commit", gestureId: "g1" });
    assert.deepEqual(typesOf(gestures.events), [
      "begin",
      "preview",
      "commit",
    ]);
    gestures.stop();

    // Gateway saw the command and recorded envelope provenance.
    const rec = fx.gateway.received.find(
      (r) => r.intent.controlId === "mixer.vol" && r.intent.kind === "commit",
    );
    assert.ok(rec !== undefined, "no commit command received");
    assert.equal(rec.ctx.viewId, "mixer.strip.1");
    assert.equal(rec.ctx.windowId, "daw-window");
    assert.equal(rec.ctx.generation, 1);
    assert.equal(rec.intent.kind, "commit");

    // Deterministic apply → store update + applied receipt + the new
    // projection reflects onto the element (observed = proven truth).
    fx.apply(rec!);
    await flushMicrotasks();
    assert.equal(fx.gateway.valueOf("track.vol"), 0.7);
    assert.equal(el.value, 0.7, "element must reflect the projection");
    assert.equal(fx.faderBinding.state.phase, "observed");
    fx.unmount();
  });

  it("rejected receipt keeps control interactive + flags data-rejected", async () => {
    const fx = mountDawFixture(document.body, { flushWith: flushSync });
    const el = fx.automation as HTMLElement & {
      dispatchInput(i: unknown): void;
      hasAttribute(n: string): boolean;
    };
    el.dispatchInput({ type: "begin", gestureId: "g2", source: "pointer" });
    el.dispatchInput({ type: "commit", gestureId: "g2" });
    const rec = fx.gateway.received.find(
      (r) =>
        r.intent.controlId === "mixer.send0" && r.intent.kind === "commit",
    );
    assert.ok(rec !== undefined);
    fx.reject(rec!, "lane locked");
    await flushMicrotasks();
    // The adapter surfaces a rejected receipt as `invalid`/`aria-invalid`
    // — the truth stays the last observed value, not the failed intent.
    assert.equal(el.getAttribute("aria-invalid"), "true");
    assert.equal(el.hasAttribute("invalid"), true);
    assert.equal(fx.automationBinding.state.phase, "rejected");
    fx.unmount();
  });

  it("transport action intent is non-droppable + routed by controlId", () => {
    const fx = mountDawFixture(document.body, { flushWith: flushSync });
    const tr = fx.transport as HTMLElement & {
      dispatchEvent(e: Event): boolean;
      shadowRoot?: ShadowRoot;
    };
    const play = tr.shadowRoot!.querySelector(
      '[part~="play"]',
    ) as HTMLElement;
    assert.ok(play !== null, "play button part missing");
    play.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    const rec = fx.gateway.received.find(
      (r) =>
        r.intent.controlId === "transport" && r.intent.kind === "action",
    );
    assert.ok(rec !== undefined, "transport action not delivered");
    assert.equal(rec!.intent.kind, "action");
    fx.unmount();
  });

  it("dock transfer cancels an in-flight gesture (generation bump)", async () => {
    const fx = mountDawFixture(document.body, { flushWith: flushSync });
    const el = fx.fader as HTMLElement & {
      dispatchInput(i: unknown): void;
    };
    const gestures = collectGestures(fx.fader);
    el.dispatchInput({ type: "begin", gestureId: "g3", source: "pointer" });
    el.dispatchInput({
      type: "preview",
      gestureId: "g3",
      value: { plain: 0.4 },
    });
    // Move the view to another window mid-gesture — the binding must
    // send its contractual cancel to the gateway under the old epoch.
    fx.link.attach("daw-window-2");
    await flushMicrotasks();
    const cancelCmd = fx.gateway.received.find(
      (r) => r.intent.kind === "cancel" && r.intent.controlId === "mixer.vol",
    );
    assert.ok(
      cancelCmd !== undefined,
      "dock transfer did not dispatch a cancel command",
    );
    assert.equal(cancelCmd.ctx.generation, 1);
    assert.equal(fx.link.generation, 2);
    gestures.stop();
    fx.unmount();
  });
});

describe("Consumer B — analysis-shaped waveform/spectrum/metadata", () => {
  it("bounded waveform queries never exceed bucket budget", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const fx = mountAnalysisFixture(document.body, {
        flushWith: flushSync,
        autopump: false,
      });
      const wf = fx.waveform as HTMLElement & {
        setSize(w: number, h: number): void;
        drawNow(): void;
      };
      wf.setSize(400, 160);
      await flushMicrotasks();
      // The synthetic NaN dropout lives at 0.9–1.1 s on channel 0 —
      // bounded queries there must return count=0/NaN buckets while
      // channel 1 keeps real data (gaps are data, not paint).
      const gap = await fx.sources.waveform.query({
        start: 0.92,
        end: 1.08,
        buckets: 8,
      });
      if (gap.kind !== "bins") assert.fail("expected a bins view");
      const ch0 = gap.channels[0]!;
      const ch1 = gap.channels[1]!;
      assert.equal(ch0.length, 8);
      assert.equal(
        ch0.every((b) => b.count === 0 && Number.isNaN(b.min)),
        true,
        "expected NaN/empty buckets in the synthetic gap",
      );
      assert.equal(
        ch1.some((b) => b.count > 0 && Number.isFinite(b.min)),
        true,
        "channel 1 must keep real data through the ch0 gap",
      );
      fx.unmount();
    } finally {
      fake.restore();
    }
  });

  it("snapshot sources push deterministic frames verbatim", () => {
    const fx = mountAnalysisFixture(document.body, {
      flushWith: flushSync,
      autopump: false,
    });
    const meter = fx.sources.snapshots.meter;
    assert.equal(meter.snapshot(), null);
    meter.push({
      sequence: 1,
      timestamp: 100,
      channels: [
        { peak: 0.5, rms: 0.3 },
        { peak: 0.55, rms: 0.32 },
      ],
    });
    const snap = meter.snapshot();
    assert.equal(snap?.sequence, 1);
    // A sequence gap is preserved verbatim — loss stays visible data.
    meter.push({ sequence: 4, timestamp: 400, channels: [] });
    assert.equal(meter.snapshot()?.sequence, 4);
    fx.unmount();
  });

  it("metadata query → resolve → ok outcome; bounded by maxBytes", async () => {
    const fx = mountAnalysisFixture(document.body, {
      flushWith: flushSync,
      autopump: false,
    });
    // The backend agrees the source is at rev 1 — replies built against
    // it are in-sync (the resolveQuery default reports currentRevision).
    fx.backend.currentRevision = 1;
    const reqP = fx.query.request({
      sourceId: "analysis.wf",
      sourceRevision: 1,
      start: 0,
      end: 1,
      buckets: 16,
      maxBytes: 64 * 1024,
    });
    assert.equal(fx.backend.queries.length, 1);
    const q = fx.backend.queries[0]!;
    assert.equal(q.maxBytes, 64 * 1024);
    fx.backend.resolveQuery(q.queryId, {
      payload: { rows: [{ channel: 0, buckets: [] }], done: true },
      bytes: 128,
    });
    const out = await reqP;
    assert.equal(out.ok, true);
    fx.unmount();
  });

  it("query bound defaults to 16 MiB; over-budget replies fail", async () => {
    const fx = mountAnalysisFixture(document.body, {
      flushWith: flushSync,
      autopump: false,
    });
    fx.backend.currentRevision = 1;
    // No explicit bound → the binding fills its 16 MiB default.
    const defP = fx.query.request({
      sourceId: "analysis.wf",
      sourceRevision: 1,
      start: 0,
      end: 1,
      buckets: 8,
    });
    const dq = fx.backend.queries.at(-1)!;
    assert.equal(dq.maxBytes, 16 * 1024 * 1024);
    fx.backend.resolveQuery(dq.queryId, { payload: {}, bytes: 1 });
    assert.equal((await defP).ok, true);
    // A reply larger than the request's bound resolves over-budget.
    const overP = fx.query.request({
      sourceId: "analysis.wf",
      sourceRevision: 1,
      start: 0,
      end: 1,
      buckets: 8,
      maxBytes: 64,
    });
    const oq = fx.backend.queries.at(-1)!;
    fx.backend.resolveQuery(oq.queryId, { payload: {}, bytes: 4096 });
    const out = await overP;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "over-budget");
    fx.unmount();
  });

  it("a reply built against a newer source revision resolves stale", async () => {
    const fx = mountAnalysisFixture(document.body, {
      flushWith: flushSync,
      autopump: false,
    });
    fx.backend.currentRevision = 1;
    const reqP = fx.query.request({
      sourceId: "analysis.wf",
      sourceRevision: 1,
      start: 0,
      end: 1,
      buckets: 8,
    });
    // The source moved on while the query was parked — the reply now
    // reports a different sourceRevision than the request carried.
    fx.backend.currentRevision = 2;
    const q = fx.backend.queries.at(-1)!;
    fx.backend.resolveQuery(q.queryId, { payload: {} });
    const out = await reqP;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "stale");
    fx.unmount();
  });

  it("dock transfer cancels a parked metadata query", async () => {
    const fx = mountAnalysisFixture(document.body, {
      flushWith: flushSync,
      autopump: false,
    });
    const reqP = fx.query.request({
      sourceId: "analysis.wf",
      sourceRevision: 1,
      start: 0,
      end: 0.5,
      buckets: 8,
    });
    fx.link.attach("analysis-window-2");
    const out = await reqP;
    assert.equal(out.ok, false);
    if (!out.ok) {
      assert.equal(out.error.kind, "cancelled");
    }
    assert.equal(fx.link.generation, 2);
    fx.unmount();
  });
});
