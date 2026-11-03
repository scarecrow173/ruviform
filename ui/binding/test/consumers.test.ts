/**
 * Acceptance gate (Issue #261): Mock Consumer A (DAW control
 * consumer) and Mock Consumer B (offline analysis query consumer)
 * connect to the *same* `BindingPort` contract — no domain state is
 * imported into Ruviform, and both lifecycle contracts hold.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { CommandBinding } from "../src/binding.ts";
import { QueryBinding } from "../src/query.ts";
import { ViewLink } from "../src/link.ts";
import type { InteractionEvent } from "../src/index.ts";
import {
  flushSync,
  ManualTimers,
  MockAnalysisBackend,
  MockDawGateway,
} from "./mocks.ts";

const V = (plain: number) => ({ plain, normalized: plain });
/** The atomic pair an element emits for a programmatic `set` input. */
const beginE = (g: string, seq: number, plain: number): InteractionEvent => ({
  type: "begin",
  meta: { controlId: "f1", gestureId: g, sequence: seq, atomic: true },
  value: V(plain),
});
const commitE = (g: string, seq: number, plain: number): InteractionEvent => ({
  type: "commit",
  meta: { controlId: "f1", gestureId: g, sequence: seq, atomic: true },
  value: V(plain),
});

/** Combined mock port: dispatch + subscribe + bounded query. */
function makePort(): {
  port: {
    dispatch: MockDawGateway["dispatch"];
    subscribe: MockDawGateway["subscribe"];
    query: MockAnalysisBackend["query"];
    cancelQuery: MockAnalysisBackend["cancelQuery"];
  };
  daw: MockDawGateway;
  analysis: MockAnalysisBackend;
} {
  const daw = new MockDawGateway();
  daw.addTarget("track.vol", 0.2);
  const analysis = new MockAnalysisBackend();
  return {
    port: {
      dispatch: daw.dispatch.bind(daw),
      subscribe: daw.subscribe.bind(daw),
      query: analysis.query.bind(analysis),
      cancelQuery: analysis.cancelQuery.bind(analysis),
    },
    daw,
    analysis,
  };
}

describe("Mock Consumer A — DAW control consumer", () => {
  it("drives a full gesture → ack → projection round trip", () => {
    const { port, daw } = makePort();
    const binding = new CommandBinding<{ plain: number }>({
      target: "track.vol",
      port,
      schedule: new ManualTimers().schedule,
      flushWith: flushSync,
    });
    binding.event(beginE("g1", 1, 0.6));
    binding.event(commitE("g1", 1, 0.6));
    assert.equal(binding.state.phase, "pending");
    const rec = daw.received.at(-1);
    assert.equal(rec?.ctx.viewId, undefined); // standalone binding, no link
    // The consumer owns the receipt channel — wire it into ack.
    daw.deliver = (r) => binding.ack(r);
    daw.apply(rec!);
    binding.observe({ revision: 5, value: { plain: 0.6 } });
    assert.equal(binding.state.phase, "observed");
    assert.equal(daw.valueOf("track.vol"), 0.6);
  });

  it("view-linked binding carries view/window/generation identity", () => {
    const { port, daw } = makePort();
    const link = new ViewLink({ viewId: "mixer.strip.1", port });
    const binding = link.createBinding("track.vol", {
      flushWith: flushSync,
    });
    link.attach("main-window");
    binding.event(beginE("g1", 1, 0.4));
    binding.event(commitE("g1", 1, 0.4));
    const rec = daw.received.at(-1);
    assert.equal(rec?.ctx.viewId, "mixer.strip.1");
    assert.equal(rec?.ctx.windowId, "main-window");
    assert.equal(rec?.ctx.generation, 1);
  });
});

describe("Mock Consumer B — offline analysis query consumer", () => {
  it("issues bounded queries on the same port and resolves replies", async () => {
    const { port, analysis } = makePort();
    const link = new ViewLink({ viewId: "analysis.view", port });
    const query = link.createQuery();
    link.attach("analysis-window");
    const p = query.request({
      sourceId: "dataset-9",
      sourceRevision: 7,
      start: 0,
      end: 400,
      buckets: 512,
    });
    const req = analysis.queries.at(-1)!;
    assert.equal(req.viewId, "analysis.view");
    // Bounded per the rvf://data range-pull contract (16 MiB default).
    assert.ok(req.maxBytes !== undefined && req.maxBytes > 0);
    assert.ok(req.maxBytes <= 16 * 1024 * 1024);
    analysis.resolveQuery(req.queryId, { payload: { score: 0.98 } });
    const out = await p;
    assert.equal(out.ok, true);
    if (out.ok) assert.deepEqual(out.reply.payload, { score: 0.98 });
  });

  it("a dock transfer cancels in-flight analysis queries without touching jobs", async () => {
    const { port, analysis } = makePort();
    const link = new ViewLink({ viewId: "analysis.view", port });
    const query = link.createQuery();
    link.attach("win-1");
    const p = query.request({
      sourceId: "dataset-9",
      sourceRevision: 7,
      start: 0,
      end: 400,
      buckets: 512,
    });
    link.attach("win-2"); // dock transfer
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "cancelled");
    // Only the query channel was cancelled — the port has no job
    // surface at all, so nothing else could be touched.
    assert.equal(analysis.cancelledQueries.length, 1);
  });

  it("a vanilla-TS consumer and a link-managed consumer share the port", () => {
    const { port, daw } = makePort();
    // Vanilla consumer: bare CommandBinding, no link, no element.
    const vanilla = new CommandBinding({
      target: "track.vol",
      port,
      flushWith: flushSync,
    });
    // Link-managed consumer: logical view + window context.
    const link = new ViewLink({ viewId: "view-x", port });
    const linked = link.createBinding("track.vol", {
      flushWith: flushSync,
    });
    link.attach("w1");
    vanilla.event(beginE("g1", 1, 0.3));
    vanilla.event(commitE("g1", 1, 0.3));
    linked.event(beginE("g2", 2, 0.8));
    linked.event(commitE("g2", 2, 0.8));
    // Each atomic set produced a begin+commit pair.
    assert.equal(daw.received.length, 4);
    const commits = daw.received.filter((r) => r.intent.kind === "commit");
    assert.equal(commits.length, 2);
    assert.equal(commits[0]?.ctx.viewId, undefined);
    assert.equal(commits[1]?.ctx.viewId, "view-x");
  });
});
