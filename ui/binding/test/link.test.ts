/**
 * ViewLink / LinkRegistry tests (Issue #261 multiwindow contract):
 * a logical view keeps its id across Dock moves and closes; detach
 * cancels gestures + subscriptions + in-flight queries, and never
 * touches Project/Engine/Job.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { LinkRegistry, ViewLink } from "../src/link.ts";
import { CommandBinding } from "../src/binding.ts";
import type {
  BindingPort,
  InteractionEvent,
  StateProjection,
} from "../src/index.ts";
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

describe("ViewLink late-created bindings", () => {
  const lateCases: ReadonlyArray<{
    readonly name: string;
    readonly windowId: string;
    readonly generation: number;
    readonly subscribeCalls: number;
    readonly unsubscribed: number;
    readonly attach: (link: ViewLink) => void;
  }> = [
    {
      name: "late-created binding adopts the initial attach epoch",
      windowId: "window-A",
      generation: 1,
      subscribeCalls: 2,
      unsubscribed: 2,
      attach: (link) => link.attach("window-A"),
    },
    {
      name: "late-created binding adopts the dock-transfer epoch",
      windowId: "window-B",
      generation: 2,
      subscribeCalls: 3,
      unsubscribed: 3,
      attach: (link) => {
        link.attach("window-A");
        link.attach("window-B");
      },
    },
    {
      name: "late-created binding adopts the re-attach epoch",
      windowId: "window-A",
      generation: 2,
      subscribeCalls: 3,
      unsubscribed: 3,
      attach: (link) => {
        link.attach("window-A");
        link.detach("unmount");
        link.attach("window-A");
      },
    },
  ];

  for (const c of lateCases) {
    it(c.name, () => {
      const { gateway, link } = setup();
      try {
        const existing = link.createBinding("track.vol", {
          flushWith: flushSync,
          ackTimeoutMs: 0,
        });
        c.attach(link);

        const observedGenerations: number[] = [];
        const late = link.createBinding<{ plain: number }>("track.vol", {
          flushWith: flushSync,
          ackTimeoutMs: 0,
          onState: (s) => {
            observedGenerations.push(s.generation);
          },
        });

        assert.ok(observedGenerations.length > 0);
        for (const g of observedGenerations) assert.equal(g, c.generation);

        assert.equal(link.windowId, c.windowId);
        assert.equal(link.generation, c.generation);
        assert.equal(existing.generation, link.generation);
        assert.equal(late.generation, link.generation);

        const lateSub = gateway.subscribeCalls.at(-1);
        assert.equal(lateSub?.target, "track.vol");
        assert.equal(lateSub?.ctx.viewId, "mixer.strip.1");
        assert.equal(lateSub?.ctx.windowId, c.windowId);
        assert.equal(lateSub?.ctx.generation, c.generation);
        assert.equal(gateway.subscribeCalls.length, c.subscribeCalls);
        assert.equal(link.linkStats.activeSubscriptions, 2);

        gateway.deliver = (r) => late.ack(r);
        late.event(begin("late", 1));
        late.event(commit("late", 2));

        const sent = gateway.received.find((r) => r.intent.kind === "commit");
        assert.ok(sent);
        assert.equal(sent.ctx.viewId, "mixer.strip.1");
        assert.equal(sent.ctx.windowId, c.windowId);
        assert.equal(sent.ctx.generation, c.generation);

        gateway.accept(sent.ctx.commandId);
        assert.equal(late.state.phase, "accepted");
        gateway.apply(sent);
        assert.equal(gateway.valueOf("track.vol"), 0.5);
        assert.equal(late.state.lastObserved?.value.plain, 0.5);
        assert.equal(late.diagnostics.staleReceipts, 0);

        link.detach("unmount");
        assert.equal(late.attached, false);
        assert.equal(link.linkStats.activeSubscriptions, 0);
        assert.equal(gateway.unsubscribed, c.unsubscribed);
        link.detach("unmount");
        assert.equal(gateway.unsubscribed, c.unsubscribed);
        assert.equal(gateway.subscribeCalls.length, c.subscribeCalls);

        gateway.pushProjection("track.vol", {
          revision: 100,
          value: { plain: 0.9 },
        });
        assert.equal(late.state.lastObserved?.value.plain, 0.5);
      } finally {
        link.detach("unmount");
      }
    });
  }
});

describe("ViewLink authoritative generation", () => {
  it("adopts the attach-response epoch across bindings, subscriptions, and wire context", () => {
    const { gateway, link } = setup();
    const early = link.createBinding("track.vol", {
      flushWith: flushSync,
      ackTimeoutMs: 0,
    });
    link.attach("window-A", 7);

    assert.equal(link.generation, 7);
    assert.equal(early.generation, 7);
    const earlySub = gateway.subscribeCalls.at(-1);
    assert.equal(earlySub?.ctx.viewId, "mixer.strip.1");
    assert.equal(earlySub?.ctx.windowId, "window-A");
    assert.equal(earlySub?.ctx.generation, 7);

    // A binding created after the attach adopts the same epoch.
    const late = link.createBinding("track.vol", {
      flushWith: flushSync,
      ackTimeoutMs: 0,
    });
    assert.equal(late.generation, 7);
    assert.equal(gateway.subscribeCalls.at(-1)?.ctx.generation, 7);

    gateway.deliver = (r) => late.ack(r);
    late.event(begin("g1", 1));
    late.event(commit("g1", 2));
    const sent = gateway.received.find((r) => r.intent.kind === "commit");
    assert.ok(sent);
    assert.equal(sent.ctx.viewId, "mixer.strip.1");
    assert.equal(sent.ctx.windowId, "window-A");
    assert.equal(sent.ctx.generation, 7);
    gateway.accept(sent.ctx.commandId);
    assert.equal(late.state.phase, "accepted");
    assert.equal(late.diagnostics.staleReceipts, 0);
  });

  it("re-synchronizes to a new epoch on detach + re-attach", () => {
    const { gateway, link } = setup();
    const binding = link.createBinding("track.vol", {
      flushWith: flushSync,
      ackTimeoutMs: 0,
    });
    link.attach("window-A", 7);
    link.detach("unmount");
    link.attach("window-A", 11);
    assert.equal(link.generation, 11);
    assert.equal(binding.generation, 11);
    assert.equal(gateway.subscribeCalls.at(-1)?.ctx.generation, 11);
    const late = link.createBinding("track.vol", {
      flushWith: flushSync,
      ackTimeoutMs: 0,
    });
    assert.equal(late.generation, 11);
  });

  it("queries mint and replay replies under the adopted epoch", async () => {
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
    const query = link.createQuery();
    link.attach("window-A", 7);
    const p = query.request({
      sourceId: "ds",
      sourceRevision: 1,
      start: 0,
      end: 10,
    });
    assert.equal(backend.queries[0]?.generation, 7);
    backend.resolveQuery(backend.queries[0]!.queryId, {});
    const outcome = await p;
    assert.equal(outcome.ok, true);
    // A new epoch cancels the old epoch's in-flight query.
    const q2 = query.request({
      sourceId: "ds",
      sourceRevision: 1,
      start: 0,
      end: 10,
    });
    link.attach("window-A", 9);
    assert.equal((await q2).ok, false);
    assert.equal(backend.cancelledQueries.length, 1);
    assert.equal(backend.queries[1]?.generation, 7);
    // Queries issued after the transition mint under the new epoch.
    const q3 = query.request({
      sourceId: "ds",
      sourceRevision: 1,
      start: 0,
      end: 10,
    });
    assert.equal(backend.queries[2]?.generation, 9);
    backend.resolveQuery(backend.queries[2]!.queryId, {});
    assert.equal((await q3).ok, true);
  });

  it("same window + same or omitted generation stays a no-op", () => {
    const { gateway, link } = setup();
    link.createBinding("track.vol", { flushWith: flushSync });
    link.attach("window-A", 7);
    link.attach("window-A", 7);
    link.attach("window-A");
    assert.equal(gateway.subscribeCalls.length, 1);
    assert.equal(link.linkStats.attaches, 1);
    assert.equal(link.generation, 7);
  });

  it("same window + different generation transitions the epoch atomically", () => {
    const { gateway, link } = setup();
    const binding = link.createBinding("track.vol", {
      flushWith: flushSync,
      ackTimeoutMs: 0,
    });
    link.attach("window-A", 7);
    binding.event(begin("g1", 1));
    link.attach("window-A", 9);
    // The in-flight gesture is cancelled under the OLD epoch — the
    // cancel is the terminal message of the dead generation.
    const cancel = gateway.received.find((r) => r.intent.kind === "cancel");
    assert.ok(cancel);
    assert.equal(cancel.intent.kind === "cancel" ? cancel.intent.reason : "", "unmount");
    assert.equal(cancel.ctx.generation, 7);
    // Old subscription released; exactly one new subscription under 9.
    assert.equal(gateway.unsubscribed, 1);
    assert.equal(gateway.subscribeCalls.length, 2);
    assert.equal(gateway.subscribeCalls[1]?.ctx.generation, 9);
    assert.equal(link.linkStats.activeSubscriptions, 1);
    assert.equal(link.generation, 9);
    assert.equal(binding.generation, 9);
    assert.equal(binding.attached, true);
  });

  it("a different window adopts the supplied epoch after dock-transfer", () => {
    const { gateway, link } = setup();
    const binding = link.createBinding("track.vol", {
      flushWith: flushSync,
      ackTimeoutMs: 0,
    });
    link.attach("window-A", 7);
    link.attach("window-B", 9);
    const cancel = gateway.received.find((r) => r.intent.kind === "cancel");
    // No gesture was in flight — dock-transfer detaches with nothing
    // to cancel; the epoch still re-subscribes under the new value.
    assert.equal(cancel, undefined);
    assert.equal(link.windowId, "window-B");
    assert.equal(link.generation, 9);
    assert.equal(binding.generation, 9);
    assert.equal(gateway.subscribeCalls.at(-1)?.ctx.windowId, "window-B");
    assert.equal(gateway.subscribeCalls.at(-1)?.ctx.generation, 9);
  });

  it("rejects invalid generations without touching link state", () => {
    const { gateway, link } = setup();
    link.createBinding("track.vol", { flushWith: flushSync });
    link.attach("window-A", 7);
    for (const bad of [-1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1]) {
      assert.throws(() => link.attach("window-A", bad), RangeError);
      assert.throws(() => link.attach("window-B", bad), RangeError);
    }
    assert.equal(link.attached, true);
    assert.equal(link.windowId, "window-A");
    assert.equal(link.generation, 7);
    assert.equal(gateway.subscribeCalls.length, 1);
    assert.equal(gateway.unsubscribed, 0);
    // Detached links reject atomically too.
    const detached = setup();
    for (const bad of [-1, 1.5, NaN]) {
      assert.throws(() => detached.link.attach("window-X", bad), RangeError);
    }
    assert.equal(detached.link.attached, false);
    assert.equal(detached.link.generation, 0);
    assert.equal(detached.gateway.subscribeCalls.length, 0);
  });

  it("omitted generation keeps the local increment", () => {
    const { link } = setup();
    link.attach("window-A", 7);
    link.detach("unmount");
    link.attach("window-A");
    assert.equal(link.generation, 8);
  });

  it("an externally registered binding adopts the live epoch and unsubscribes on unregister", () => {
    const { gateway, link } = setup();
    link.createBinding("track.vol", { flushWith: flushSync });
    link.attach("window-A", 7);

    const external = new CommandBinding<{ plain: number }>({
      target: "track.vol",
      port: gateway,
      flushWith: flushSync,
      ackTimeoutMs: 0,
    });
    const unregister = link.registerBinding(external);

    // Same state a pre-registered binding holds after attach: the
    // adopted epoch and a live subscription.
    assert.equal(external.generation, 7);
    assert.equal(external.attached, true);
    assert.equal(gateway.subscribeCalls.length, 2);
    assert.equal(gateway.subscribeCalls[1]?.ctx.generation, 7);
    assert.equal(link.linkStats.activeSubscriptions, 2);

    gateway.deliver = (r) => external.ack(r);
    external.event(begin("ext", 1));
    external.event(commit("ext", 2));
    const sent = gateway.received.find((r) => r.intent.kind === "commit");
    assert.ok(sent);
    assert.equal(sent.ctx.generation, 7);
    assert.equal(sent.ctx.viewId, "mixer.strip.1");
    assert.equal(sent.ctx.windowId, "window-A");

    // Unregister releases the subscription the link made at
    // registration — the binding is no longer link-managed.
    unregister();
    assert.equal(link.linkStats.activeSubscriptions, 1);
    assert.equal(gateway.unsubscribed, 1);
    link.attach("window-B", 9);
    assert.equal(gateway.subscribeCalls.length, 3);
    assert.equal(gateway.subscribeCalls[2]?.ctx.generation, 9);
    // Untouched by the transition: not re-attached, not re-subscribed.
    assert.equal(external.generation, 7);
  });

  it("a projection delivered to a dead epoch's listener is dropped at the link guard", () => {
    // Capture listeners so a racy port that still pushes to an
    // already-unsubscribed listener can be simulated.
    const gateway = new MockDawGateway();
    gateway.addTarget("track.vol", 0.2);
    const captured: Array<(p: StateProjection<unknown>) => void> = [];
    const port: BindingPort = {
      dispatch: (intent, ctx) => gateway.dispatch(intent, ctx),
      subscribe: (target, listener, ctx) => {
        captured.push(listener as (p: StateProjection<unknown>) => void);
        return gateway.subscribe(target, listener, ctx);
      },
    };
    const link = new ViewLink({ viewId: "v-dead", port });
    const binding = link.createBinding<{ plain: number }>("track.vol", {
      flushWith: flushSync,
      ackTimeoutMs: 0,
    });
    link.attach("window-A", 7);
    assert.equal(captured.length, 1);
    link.attach("window-A", 9);
    assert.equal(captured.length, 2);

    // A push reaching the epoch-7 listener after the epoch-9
    // transition is dropped before `observe` — the observed truth
    // (revision 0, pushed at subscribe) cannot be overwritten.
    captured[0]!({ revision: 100, value: { plain: 0.9 } });
    assert.equal(binding.state.lastObserved?.value.plain, 0.2);
    assert.equal(binding.state.observedSeq, 1);

    // The live epoch-9 listener still delivers.
    captured[1]!({ revision: 100, value: { plain: 0.9 } });
    assert.equal(binding.state.lastObserved?.value.plain, 0.9);
    assert.equal(binding.state.observedSeq, 2);
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
