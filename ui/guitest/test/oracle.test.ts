/**
 * Issue #378 — `ContractModel` late-created binding epoch.
 *
 * `ContractModel.addBinding` must mirror the repaired SDK
 * `ViewLink.registerBinding` contract (Issue #376 / PR #379): a binding
 * created on an already-attached link adopts the live view epoch —
 * `setContext` unconditionally, then `attach(linkGeneration)` →
 * `subscribe` — so its dispatches, subscription context, and final
 * state carry the same generation as pre-registered bindings.
 *
 * These tests drive `ContractModel` directly: the replay op vocabulary
 * has no mid-scenario binding creation, so the scenario lane cannot
 * reach this branch. The three cases mirror the late-created binding
 * cases in `ui/binding/test/link.test.ts`.
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
