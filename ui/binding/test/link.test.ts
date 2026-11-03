/**
 * ViewLink / LinkRegistry tests (Issue #261 multiwindow contract):
 * a logical view keeps its id across Dock moves and closes; detach
 * cancels gestures + subscriptions + in-flight queries, and never
 * touches Project/Engine/Job.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { LinkRegistry, ViewLink } from "../src/link.ts";
import type { InteractionEvent } from "../src/index.ts";
import {
  flushSync,
  ManualTimers,
  MockAnalysisBackend,
  MockDawGateway,
} from "./mocks.ts";

const V = { plain: 0.5, normalized: 0.5 };
const begin = (g: string, seq: number): InteractionEvent => ({
  type: "begin",
  meta: { controlId: "f1", gestureId: g, sequence: seq, target: "track.vol" },
  value: V,
});
const commit = (g: string, seq: number): InteractionEvent => ({
  type: "commit",
  meta: { controlId: "f1", gestureId: g, sequence: seq, target: "track.vol" },
  value: V,
});

function setup(): {
  gateway: MockDawGateway;
  link: ViewLink;
} {
  const gateway = new MockDawGateway();
  gateway.addTarget("track.vol", 0.2);
  const link = new ViewLink({ viewId: "mixer.strip.1", port: gateway });
  return { gateway, link };
}

describe("ViewLink attach/detach", () => {
  it("attach subscribes bindings with view/window/generation context", () => {
    const { gateway, link } = setup();
    link.createBinding("track.vol", { flushWith: flushSync });
    link.attach("window-A");
    assert.equal(gateway.subscribeCalls.length, 1);
    assert.equal(gateway.subscribeCalls[0]?.ctx.viewId, "mixer.strip.1");
    assert.equal(gateway.subscribeCalls[0]?.ctx.windowId, "window-A");
    assert.equal(gateway.subscribeCalls[0]?.ctx.generation, 1);
    // The immediate projection lands as the view's first truth.
  });

  it("re-attaching the same window is a no-op", () => {
    const { gateway, link } = setup();
    link.createBinding("track.vol", { flushWith: flushSync });
    link.attach("window-A");
    link.attach("window-A");
    assert.equal(gateway.subscribeCalls.length, 1);
    assert.equal(link.generation, 1);
  });

  it("dock move keeps the logical id, bumps generation, resubscribes", () => {
    const { gateway, link } = setup();
    link.createBinding("track.vol", { flushWith: flushSync });
    link.attach("window-A");
    link.attach("window-B"); // implicit dock-transfer
    assert.equal(link.viewId, "mixer.strip.1");
    assert.equal(link.generation, 2);
    assert.equal(link.windowId, "window-B");
    assert.equal(gateway.subscribeCalls.length, 2);
    assert.equal(gateway.subscribeCalls[1]?.ctx.windowId, "window-B");
    assert.equal(gateway.subscribeCalls[1]?.ctx.generation, 2);
    assert.equal(gateway.unsubscribed, 1);
  });

  it("detach releases subscriptions and cancels in-flight queries", async () => {
    const backend = new MockAnalysisBackend();
    const gateway = new MockDawGateway();
    gateway.addTarget("track.vol", 0.2);
    const port = {
      dispatch: gateway.dispatch.bind(gateway),
      subscribe: gateway.subscribe.bind(gateway),
      query: backend.query.bind(backend),
      cancelQuery: backend.cancelQuery.bind(backend),
    };
    const link = new ViewLink({ viewId: "v1", port });
    const binding = link.createBinding("track.vol", { flushWith: flushSync });
    const query = link.createQuery();
    link.attach("window-A");
    const p = query.request({
      sourceId: "ds",
      sourceRevision: 1,
      start: 0,
      end: 10,
    });
    binding.event(begin("g1", 1));
    link.detach("dock-transfer");
    // Gesture cancelled through the port…
    assert.equal(gateway.received.at(-1)?.intent.kind, "cancel");
    // …subscription released…
    assert.equal(gateway.unsubscribed, 1);
    // …query cancelled on both channels.
    assert.equal((await p).ok, false);
    assert.equal(backend.cancelledQueries.length, 1);
    // …and the view state goes stale, never pretending otherwise.
    assert.equal(binding.state.phase, "stale");
  });

  it("a window move never sends Project/Engine/Job commands", () => {
    const { gateway, link } = setup();
    const binding = link.createBinding("track.vol", { flushWith: flushSync });
    link.attach("window-A");
    binding.event(begin("g1", 1));
    binding.event(commit("g1", 2));
    const before = gateway.received.length;
    link.attach("window-B");
    link.detach("explicit");
    const after = gateway.received.slice(before);
    // Only the bound target's own gesture lifecycle may appear —
    // no job/project/engine verbs exist on this port at all.
    for (const r of after) {
      assert.equal(r.intent.target, "track.vol");
      assert.notEqual(r.intent.kind, "action");
    }
  });

  it("double detach is a no-op", () => {
    const { gateway, link } = setup();
    link.createBinding("track.vol", { flushWith: flushSync });
    link.attach("window-A");
    link.detach("unmount");
    link.detach("unmount");
    assert.equal(gateway.unsubscribed, 1);
  });

  it("a projection pushed between detach and re-attach is dropped", () => {
    const { gateway, link } = setup();
    const binding = link.createBinding<{ plain: number }>("track.vol", {
      flushWith: flushSync,
    });
    link.attach("window-A");
    binding.observe({ revision: 1, value: { plain: 1 } });
    link.detach("dock-transfer");
    binding.observe({ revision: 2, value: { plain: 2 } });
    assert.equal(binding.diagnostics.staleProjections, 1);
    assert.equal(binding.state.lastObserved?.value.plain, 1);
  });
});

describe("LinkRegistry", () => {
  it("keeps the logical view id stable across windows", () => {
    const gateway = new MockDawGateway();
    const registry = new LinkRegistry();
    const a = registry.attachView("view-7", "w1", gateway);
    registry.moveView("view-7", "w2");
    const b = registry.get("view-7");
    assert.equal(a, b);
    assert.equal(b?.viewId, "view-7");
    assert.equal(b?.windowId, "w2");
    assert.equal(b?.generation, 2);
  });

  it("closeView detaches and drops the entry", () => {
    const gateway = new MockDawGateway();
    const registry = new LinkRegistry();
    const link = registry.attachView("view-9", "w1", gateway);
    registry.closeView("view-9");
    assert.equal(link.attached, false);
    assert.equal(registry.get("view-9"), undefined);
  });

  it("detachView keeps the entry for a later re-attach", () => {
    const gateway = new MockDawGateway();
    const registry = new LinkRegistry();
    registry.attachView("view-3", "w1", gateway);
    registry.detachView("view-3", "dock-transfer");
    assert.ok(registry.get("view-3") !== undefined);
    registry.attachView("view-3", "w4", gateway);
    assert.equal(registry.get("view-3")?.windowId, "w4");
  });
});
