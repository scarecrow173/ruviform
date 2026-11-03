/**
 * QueryBinding tests (Issue #261): bounded viewport queries,
 * per-view generation, cancel, stale-reply discard, and the
 * rvf://data-style byte bound.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { QueryBinding } from "../src/query.ts";
import { DEFAULT_QUERY_MAX_BYTES } from "../src/contract.ts";
import { MockAnalysisBackend } from "./mocks.ts";

const INPUT = {
  sourceId: "dataset-1",
  sourceRevision: 3,
  start: 0,
  end: 100,
  buckets: 200,
};

function make(opts: {
  backend?: MockAnalysisBackend;
  maxInflight?: number;
  revisionProbe?: (id: string) => number | string | undefined;
}): { q: QueryBinding; backend: MockAnalysisBackend } {
  const backend = opts.backend ?? new MockAnalysisBackend();
  const q = new QueryBinding({
    viewId: "view-1",
    port: backend,
    ...(opts.maxInflight !== undefined ? { maxInflight: opts.maxInflight } : {}),
    ...(opts.revisionProbe !== undefined
      ? { revisionProbe: opts.revisionProbe }
      : {}),
  });
  return { q, backend };
}

describe("bounded viewport queries", () => {
  it("issues a query carrying identity + bounds and resolves the reply", async () => {
    const { q, backend } = make({});
    const p = q.request(INPUT);
    assert.equal(backend.queries.length, 1);
    const req = backend.queries[0];
    assert.equal(req?.viewId, "view-1");
    assert.equal(req?.sourceId, "dataset-1");
    assert.equal(req?.sourceRevision, 3);
    assert.equal(req?.generation, 0);
    assert.equal(req?.maxBytes, DEFAULT_QUERY_MAX_BYTES);
    backend.resolveQuery(req!.queryId, { payload: { bins: [1, 2] } });
    const out = await p;
    assert.equal(out.ok, true);
    if (out.ok) assert.deepEqual(out.reply.payload, { bins: [1, 2] });
  });

  it("a reply with a mismatched generation is discarded as stale", async () => {
    const { q, backend } = make({});
    const p = q.request(INPUT);
    const req = backend.queries[0]!;
    backend.resolveQuery(req.queryId, { generation: 999 });
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "stale");
    assert.equal(q.diagnostics.stale, 1);
  });

  it("a reply under a superseded view generation is discarded", async () => {
    const { q, backend } = make({});
    const p = q.request(INPUT);
    q.attach(); // view re-epoch — cancels + bumps generation
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "cancelled");
  });

  it("a reply with a stale source revision is discarded", async () => {
    const { q, backend } = make({});
    const p = q.request(INPUT);
    const req = backend.queries[0]!;
    backend.resolveQuery(req.queryId, { sourceRevision: 2 });
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "stale");
  });

  it("a source publish mid-flight makes the answer stale (probe)", async () => {
    const backend = new MockAnalysisBackend();
    const { q } = make({
      backend,
      revisionProbe: () => backend.currentRevision,
    });
    const p = q.request(INPUT);
    const req = backend.queries[0]!;
    backend.currentRevision = 4; // source moved during flight
    backend.resolveQuery(req.queryId, { sourceRevision: 3 });
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "stale");
  });

  it("an over-budget reply is refused, not delivered", async () => {
    const { q, backend } = make({});
    const p = q.request({ ...INPUT, maxBytes: 1024 });
    const req = backend.queries[0]!;
    backend.resolveQuery(req.queryId, { bytes: 2048 });
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "over-budget");
    assert.equal(q.diagnostics.overBudget, 1);
  });

  it("port errors surface as error outcomes", async () => {
    const { q, backend } = make({});
    const p = q.request(INPUT);
    const req = backend.queries[0]!;
    backend.rejectQuery(req.queryId, new Error("backend exploded"));
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "error");
  });

  it("a port without query reports unsupported", async () => {
    const q = new QueryBinding({ viewId: "v", port: {} });
    const out = await q.request(INPUT);
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "unsupported");
  });
});

describe("cancel + generation", () => {
  it("cancel aborts the signal and calls the port's cancel channel", async () => {
    const { q, backend } = make({});
    const p = q.request(INPUT);
    const req = backend.queries[0]!;
    q.cancel(req.queryId);
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "cancelled");
    assert.deepEqual(backend.cancelledQueries, [req.queryId]);
  });

  it("a late reply after cancel never lands", async () => {
    const { q, backend } = make({});
    const p = q.request(INPUT);
    const req = backend.queries[0]!;
    q.cancel(req.queryId);
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "cancelled");
  });

  it("detach cancels every in-flight query", async () => {
    const { q, backend } = make({});
    const p1 = q.request(INPUT);
    const p2 = q.request({ ...INPUT, end: 200 });
    assert.equal(backend.queries.length, 2);
    q.detach();
    const [o1, o2] = await Promise.all([p1, p2]);
    assert.equal(o1.ok, false);
    assert.equal(o2.ok, false);
    assert.equal(backend.cancelledQueries.length, 2);
    assert.equal(q.diagnostics.inflight, 0);
  });

  it("in-flight overflow evicts the oldest query", async () => {
    const { q, backend } = make({ maxInflight: 2 });
    const p1 = q.request(INPUT);
    const p2 = q.request({ ...INPUT, end: 200 });
    const p3 = q.request({ ...INPUT, end: 300 });
    assert.equal(backend.queries.length, 3);
    assert.equal(backend.cancelledQueries.length, 1);
    const out1 = await p1;
    assert.equal(out1.ok, false);
    if (!out1.ok) assert.equal(out1.error.kind, "cancelled");
    // Resolve the survivors — they still land.
    for (const req of backend.queries.slice(1)) {
      backend.resolveQuery(req.queryId, {});
    }
    assert.equal((await p2).ok, true);
    assert.equal((await p3).ok, true);
    assert.equal(q.diagnostics.evicted, 1);
  });
});
