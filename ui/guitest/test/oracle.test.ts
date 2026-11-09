/**
 * Issue #378 — `ContractModel` late-created binding epoch.
 * Issue #490 — `ContractModel` late-created query epoch.
 *
 * `ContractModel.addBinding` must mirror the repaired SDK
 * `ViewLink.registerBinding` contract (Issue #376 / PR #379): a binding
 * created on an already-attached link adopts the live view epoch —
 * `setContext` unconditionally, then `attach(linkGeneration)` →
 * `subscribe` — so its dispatches, subscription context, and final
 * state carry the same generation as pre-registered bindings.
 *
 * `ContractModel.addQuery` must mirror the repaired SDK
 * `ViewLink.registerQuery` contract (Issue #377 / PR #489): a query
 * model created on an already-attached link adopts the live view epoch
 * via `attach(linkGeneration)`, so issued queries mint under the same
 * generation a pre-registered query model holds after `attach`.
 *
 * These tests drive `ContractModel` directly: the replay op vocabulary
 * has no mid-scenario binding/query creation, so the scenario lane
 * cannot reach these branches. The binding cases mirror the
 * late-created binding cases in `ui/binding/test/link.test.ts`; the
 * query cases mirror its late-created query cases, observed through
 * the oracle's query surface (`queries` / `queryOutcomes` /
 * `queryDiagnostics`) — queries hold no subscription, so the issued
 * query's recorded generation is the epoch-adoption evidence.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ContractModel } from "../src/oracle.ts";
import type { InteractionEvent } from "../../core/src/index.ts";

const V = { plain: 0.5, normalized: 0.5 };

const begin = (
  controlId: string,
  target: string,
  g: string,
  seq: number,
): InteractionEvent => ({
  type: "begin",
  meta: { controlId, gestureId: g, sequence: seq, target },
  value: V,
});
const commit = (
  controlId: string,
  target: string,
  g: string,
  seq: number,
): InteractionEvent => ({
  type: "commit",
  meta: { controlId, gestureId: g, sequence: seq, target },
  value: V,
});

function setup(): ContractModel {
  return new ContractModel("view-1", "daw", 0, {
    maxInflight: 4,
    defaultMaxBytes: 1024,
  });
}

describe("ContractModel late-created bindings", () => {
  const lateCases: ReadonlyArray<{
    readonly name: string;
    readonly windowId: string;
    readonly generation: number;
    readonly attach: (model: ContractModel) => void;
  }> = [
    {
      name: "late-created binding adopts the initial attach epoch",
      windowId: "window-A",
      generation: 1,
      attach: (model) => model.attach("window-A"),
    },
    {
      name: "late-created binding adopts the dock-transfer epoch",
      windowId: "window-B",
      generation: 2,
      attach: (model) => {
        model.attach("window-A");
        model.attach("window-B");
      },
    },
    {
      name: "late-created binding adopts the re-attach epoch",
      windowId: "window-A",
      generation: 2,
      attach: (model) => {
        model.attach("window-A");
        model.detach("unmount");
        model.attach("window-A");
      },
    },
  ];

  for (const c of lateCases) {
    it(c.name, () => {
      const model = setup();
      model.addBinding("f1", "track.vol");
      c.attach(model);
      model.addBinding("f2", "track.pan");

      const b2 = model.binding("f2");
      assert.ok(b2 !== undefined);

      // The late binding adopted the live view epoch — the same state
      // a pre-registered binding holds after `attach`.
      let tr = model.traceResult();
      assert.equal(tr.link.attached, true);
      assert.equal(tr.link.windowId, c.windowId);
      assert.equal(tr.finalStates["f1"]?.generation, c.generation);
      assert.equal(tr.finalStates["f2"]?.generation, c.generation);

      // Its subscription was recorded under the adopted epoch with
      // the link's current context.
      const f2Subs = tr.subscriptions.filter((s) => s.binding === "f2");
      assert.equal(f2Subs.length, 1);
      assert.equal(f2Subs[0]?.target, "track.pan");
      assert.equal(f2Subs[0]?.viewId, "view-1");
      assert.equal(f2Subs[0]?.windowId, c.windowId);
      assert.equal(f2Subs[0]?.generation, c.generation);

      // The scripted `daw` backend pushes the store's current
      // projection on subscribe — it lands as the binding's first
      // observed truth under the adopted epoch.
      assert.equal(tr.finalStates["f2"]?.phase, "observed");

      // Dispatches carry the adopted epoch and the link context.
      b2.onInteractionEvent(begin("f2", "track.pan", "g1", 1));
      b2.onInteractionEvent(commit("f2", "track.pan", "g1", 2));
      model.flush();
      tr = model.traceResult();
      const f2Dispatched = tr.dispatched.filter((d) => d.binding === "f2");
      assert.equal(f2Dispatched.length, 2);
      for (const d of f2Dispatched) {
        assert.equal(d.generation, c.generation);
        assert.equal(d.viewId, "view-1");
        assert.equal(d.windowId, c.windowId);
      }
      const commitRec = f2Dispatched.find((d) => d.kind === "commit");
      assert.ok(commitRec !== undefined);
      assert.equal(commitRec.expectReceipt, true);

      // Receipts correlate under the adopted epoch — a live-epoch
      // receipt is never classified stale.
      model.ack({ commandId: commitRec.commandId, status: "accepted" });
      model.ack({ commandId: commitRec.commandId, status: "applied" });
      model.projection("track.pan", 7);
      tr = model.traceResult();
      assert.equal(tr.finalStates["f2"]?.phase, "observed");
      assert.equal(tr.bindingDiagnostics["f2"]?.staleReceipts, 0);
      assert.equal(tr.bindingDiagnostics["f2"]?.staleProjections, 0);

      // Detach releases every subscription — including the late
      // binding's — and is idempotent.
      model.detach("unmount");
      tr = model.traceResult();
      assert.equal(tr.link.attached, false);
      assert.equal(tr.link.activeSubscriptions, 0);
      const unsubs = tr.unsubscriptions.map((u) => u.binding);
      assert.ok(unsubs.includes("f1"));
      assert.ok(unsubs.includes("f2"));
      assert.equal(tr.finalStates["f2"]?.attached, false);
      const unsubCount = unsubs.length;
      model.detach("unmount");
      tr = model.traceResult();
      assert.equal(tr.unsubscriptions.length, unsubCount);

      // Detached pushes are counted, never dispatched; detached
      // projections are stale.
      b2.onInteractionEvent(begin("f2", "track.pan", "g2", 3));
      model.projection("track.pan", 99);
      tr = model.traceResult();
      assert.equal(
        tr.dispatched.filter((d) => d.binding === "f2").length,
        2,
      );
      assert.equal(tr.bindingDiagnostics["f2"]?.detachedIntents, 1);
      assert.equal(tr.bindingDiagnostics["f2"]?.staleProjections, 1);

      // A late-created binding is managed like any other: the next
      // attach transition re-adopts the new epoch.
      model.attach("window-C");
      tr = model.traceResult();
      assert.equal(tr.finalStates["f2"]?.generation, c.generation + 1);
      assert.equal(tr.finalStates["f2"]?.attached, true);
      assert.equal(tr.link.windowId, "window-C");
    });
  }
});

function querySetup(): ContractModel {
  return new ContractModel("view-1", "analysis", 0, {
    maxInflight: 4,
    defaultMaxBytes: 1024,
  });
}

const QUERY_INPUT = {
  sourceId: "ds",
  sourceRevision: 1,
  start: 0,
  end: 10,
};

describe("ContractModel late-created queries", () => {
  const lateQueryCases: ReadonlyArray<{
    readonly name: string;
    readonly windowId: string;
    readonly generation: number;
    readonly attach: (model: ContractModel) => void;
  }> = [
    {
      name: "late-created query adopts the initial attach epoch",
      windowId: "window-A",
      generation: 1,
      attach: (model) => model.attach("window-A"),
    },
    {
      name: "late-created query adopts the dock-transfer epoch",
      windowId: "window-B",
      generation: 2,
      attach: (model) => {
        model.attach("window-A");
        model.attach("window-B");
      },
    },
    {
      name: "late-created query adopts the re-attach epoch",
      windowId: "window-A",
      generation: 2,
      attach: (model) => {
        model.attach("window-A");
        model.detach("unmount");
        model.attach("window-A");
      },
    },
  ];

  for (const c of lateQueryCases) {
    it(c.name, () => {
      const model = querySetup();
      c.attach(model);
      model.addQuery();

      // The late-created query model adopted the live view epoch —
      // the same state a pre-registered query model holds after
      // `attach` (`registerQuery` contract).
      let tr = model.traceResult();
      assert.equal(tr.link.attached, true);
      assert.equal(tr.link.windowId, c.windowId);
      assert.equal(tr.queryDiagnostics["view-1"]?.generation, c.generation);

      // Issued queries mint under the adopted epoch (queries hold no
      // subscription — the issued rec's generation is the evidence).
      model.query("qa", QUERY_INPUT);
      tr = model.traceResult();
      const issued = tr.queries.at(-1);
      assert.equal(issued?.viewId, "view-1");
      assert.equal(issued?.generation, c.generation);

      // A reply echoing the adopted epoch resolves — never stale.
      model.resolveQuery("qa", {
        generation: c.generation,
        sourceRevision: QUERY_INPUT.sourceRevision,
      });
      tr = model.traceResult();
      assert.equal(tr.queryOutcomes["qa"], "ok");
      assert.equal(tr.queryDiagnostics["view-1"]?.resolved, 1);
      assert.equal(tr.queryDiagnostics["view-1"]?.stale, 0);

      // The stale guard is preserved: a foreign-epoch echo is stale.
      model.query("qb", QUERY_INPUT);
      model.resolveQuery("qb", {
        generation: c.generation + 100,
        sourceRevision: QUERY_INPUT.sourceRevision,
      });
      tr = model.traceResult();
      assert.equal(tr.queryOutcomes["qb"], "stale");
      assert.equal(tr.queryDiagnostics["view-1"]?.stale, 1);

      // Detach cancels in-flight work and bumps the epoch.
      model.query("qc", QUERY_INPUT);
      model.detach("unmount");
      tr = model.traceResult();
      assert.equal(tr.link.attached, false);
      assert.equal(tr.queryOutcomes["qc"], "cancelled");
      assert.equal(tr.queryDiagnostics["view-1"]?.inflight, 0);

      // The next attach re-adopts the new link epoch — the same
      // convergence a pre-registered query model reaches.
      model.attach("window-C");
      tr = model.traceResult();
      assert.equal(
        tr.queryDiagnostics["view-1"]?.generation,
        c.generation + 1,
      );

      model.query("qd", QUERY_INPUT);
      model.resolveQuery("qd", {
        generation: c.generation + 1,
        sourceRevision: QUERY_INPUT.sourceRevision,
      });
      tr = model.traceResult();
      assert.equal(tr.queries.at(-1)?.generation, c.generation + 1);
      assert.equal(tr.queryOutcomes["qd"], "ok");
    });
  }

  it("a query registered before attach adopts the epoch at attach", () => {
    const model = querySetup();
    model.addQuery();
    model.attach("window-A");
    model.attach("window-B"); // dock-transfer → generation 2

    // The pre-registered path is unchanged: `ContractModel.attach`
    // distributes the live epoch to the query model.
    model.query("qa", QUERY_INPUT);
    let tr = model.traceResult();
    assert.equal(tr.queries.at(-1)?.generation, 2);
    assert.equal(tr.queryDiagnostics["view-1"]?.generation, 2);

    model.resolveQuery("qa", {
      generation: 2,
      sourceRevision: QUERY_INPUT.sourceRevision,
    });
    tr = model.traceResult();
    assert.equal(tr.queryOutcomes["qa"], "ok");
  });
});
