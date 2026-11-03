/**
 * CommandBinding lifecycle tests (Issue #261):
 * intent → command translation, receipt ordering/dedup/timeout,
 * revision-guarded projections, and generation semantics across
 * detach/attach.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { CommandBinding } from "../src/binding.ts";
import type {
  BindingViewState,
  CommandIntent,
  InteractionEvent,
} from "../src/index.ts";
import {
  deferredFlush,
  flushSync,
  ManualTimers,
  MockDawGateway,
} from "./mocks.ts";

const V = (plain: number) => ({ plain, normalized: plain });

function gestureEvent(
  type: "begin" | "preview" | "commit" | "cancel",
  gestureId: string,
  seq: number,
  plain = 0.5,
): InteractionEvent {
  const meta = { controlId: "f1", gestureId, sequence: seq, target: "track.vol" };
  switch (type) {
    case "begin":
      return { type, meta, value: V(plain) };
    case "preview":
      return { type, meta, value: V(plain), coalescible: true };
    case "commit":
      return { type, meta, value: V(plain) };
    case "cancel":
      return { type, meta, reason: "explicit" };
  }
}

function makeBinding(opts: {
  gateway?: MockDawGateway;
  timers?: ManualTimers;
  flush?: ReturnType<typeof deferredFlush>;
  states?: BindingViewState<{ plain: number }>[];
}): {
  binding: CommandBinding<{ plain: number }>;
  gateway: MockDawGateway;
  timers: ManualTimers;
} {
  const gateway = opts.gateway ?? new MockDawGateway();
  const timers = opts.timers ?? new ManualTimers();
  const binding = new CommandBinding<{ plain: number }>({
    target: "track.vol",
    port: gateway,
    schedule: timers.schedule,
    flushWith: opts.flush?.flushWith ?? flushSync,
    ackTimeoutMs: 1000,
    ...(opts.states !== undefined
      ? {
          onState: (s) =>
            opts.states?.push(s as BindingViewState<{ plain: number }>),
        }
      : {}),
  });
  // The receipt channel is the consumer's: wire the gateway's outbox
  // into the binding's ack inlet.
  gateway.deliver = (r) => binding.ack(r);
  return { binding, gateway, timers };
}

describe("intent → command translation", () => {
  it("gesture events dispatch begin/preview/commit in order", () => {
    const { binding, gateway } = makeBinding({});
    binding.event(gestureEvent("begin", "g1", 1));
    binding.event(gestureEvent("preview", "g1", 2));
    binding.event(gestureEvent("commit", "g1", 3));
    assert.deepEqual(
      gateway.received.map((r) => r.intent.kind),
      ["begin", "preview", "commit"],
    );
    const commit = gateway.received.at(-1);
    assert.equal(commit?.intent.kind, "commit");
    if (commit?.intent.kind === "commit") {
      assert.equal(commit.intent.value.plain, 0.5);
      assert.equal(commit.intent.target, "track.vol");
      assert.equal(commit.intent.gestureId, "g1");
    }
    assert.equal(commit?.ctx.expectReceipt, true);
    assert.equal(binding.state.phase, "pending");
  });

  it("local-only events (unavailable/rejected/noop) never reach the port", () => {
    const { binding, gateway } = makeBinding({});
    binding.event({
      type: "unavailable",
      meta: { controlId: "f1", sequence: 1 },
      reason: "disabled",
    });
    binding.event({
      type: "rejected",
      meta: { controlId: "f1", sequence: 2 },
      reason: "out-of-range",
    });
    binding.event({
      type: "noop",
      meta: { controlId: "f1", sequence: 3 },
      reason: "no-active-gesture",
    });
    assert.equal(gateway.received.length, 0);
  });

  it("a cancel terminates the gesture — phase falls back", () => {
    const { binding, gateway } = makeBinding({});
    binding.event(gestureEvent("begin", "g1", 1));
    assert.equal(binding.state.phase, "preview");
    binding.event(gestureEvent("cancel", "g1", 2));
    assert.equal(binding.state.phase, "idle");
    assert.deepEqual(
      gateway.received.map((r) => r.intent.kind),
      ["begin", "cancel"],
    );
  });

  it("discrete actions dispatch as non-droppable receipt-expecting commands", () => {
    const { binding, gateway } = makeBinding({});
    binding.action("play", { controlId: "tp", sequence: 7, source: "pointer" });
    const r = gateway.received[0];
    assert.equal(r?.intent.kind, "action");
    if (r?.intent.kind === "action") assert.equal(r.intent.action, "play");
    assert.equal(r?.ctx.expectReceipt, true);
    assert.equal(binding.state.phase, "pending");
  });
});

describe("receipt lifecycle", () => {
  function committedBinding(): {
    binding: CommandBinding<{ plain: number }>;
    gateway: MockDawGateway;
    timers: ManualTimers;
    commandId: string;
  } {
    const { binding, gateway, timers } = makeBinding({});
    binding.event(gestureEvent("begin", "g1", 1));
    binding.event(gestureEvent("commit", "g1", 2));
    const commandId = gateway.lastCommandId();
    assert.ok(commandId !== undefined);
    return { binding, gateway, timers, commandId };
  }

  it("accepted → applied → observed are distinct phases", () => {
    const { binding, gateway, commandId } = committedBinding();
    assert.equal(binding.state.phase, "pending");
    gateway.deliver({ commandId, status: "accepted" });
    assert.equal(binding.state.phase, "accepted");
    gateway.apply(gateway.received.at(-1)!);
    assert.equal(binding.state.phase, "applied");
    binding.observe({ revision: 1, value: { plain: 0.5 } });
    assert.equal(binding.state.phase, "observed");
  });

  it("a reordered ack (applied before accepted) does not regress", () => {
    const { binding, gateway, commandId } = committedBinding();
    gateway.deliver({ commandId, status: "applied" });
    gateway.deliver({ commandId, status: "accepted" }); // late, stale ordering
    assert.equal(binding.state.phase, "applied");
    assert.equal(binding.diagnostics.outOfOrderReceipts, 1);
  });

  it("duplicate acks are absorbed once and counted", () => {
    const { binding, gateway, commandId } = committedBinding();
    gateway.deliver({ commandId, status: "accepted" });
    gateway.deliver({ commandId, status: "accepted" });
    gateway.deliver({ commandId, status: "accepted" });
    assert.equal(binding.state.phase, "accepted");
    assert.equal(binding.diagnostics.outOfOrderReceipts, 2);
  });

  it("a receipt for an unknown command is stale, never applied", () => {
    const { binding } = committedBinding();
    binding.ack({ commandId: "cmd-999", status: "applied" });
    assert.equal(binding.state.phase, "pending");
    assert.equal(binding.diagnostics.staleReceipts, 1);
  });

  it("reject is terminal — later acks cannot move it", () => {
    const { binding, gateway, commandId } = committedBinding();
    gateway.reject(commandId);
    assert.equal(binding.state.phase, "rejected");
    gateway.deliver({ commandId, status: "applied" });
    assert.equal(binding.state.phase, "rejected");
    assert.equal(binding.diagnostics.outOfOrderReceipts, 1);
  });

  it("unsupported is a distinct terminal phase", () => {
    const { binding, gateway, commandId } = committedBinding();
    gateway.unsupported(commandId);
    assert.equal(binding.state.phase, "unsupported");
  });

  it("ack timeout is an explicit failure, never a fabricated commit", () => {
    const { binding, timers } = committedBinding();
    timers.advance(1100);
    assert.equal(binding.state.phase, "timeout");
    assert.equal(binding.state.timedOut, true);
  });

  it("a late applied after timeout still lands — flagged timedOut", () => {
    const { binding, gateway, timers, commandId } = committedBinding();
    timers.advance(1100);
    assert.equal(binding.state.phase, "timeout");
    gateway.deliver({ commandId, status: "applied" });
    assert.equal(binding.state.phase, "applied");
    assert.equal(binding.state.timedOut, true);
  });

  it("timeout while queued shows timeout, not accepted", () => {
    const { binding, gateway, timers, commandId } = committedBinding();
    gateway.deliver({ commandId, status: "queued" });
    assert.equal(binding.state.phase, "accepted");
    timers.advance(1100);
    assert.equal(binding.state.phase, "timeout");
  });
});

describe("projection guard", () => {
  it("stale or duplicate revisions are dropped", () => {
    const { binding } = makeBinding({});
    binding.observe({ revision: 5, value: { plain: 1 } });
    assert.equal(binding.state.phase, "observed");
    binding.observe({ revision: 4, value: { plain: 2 } }); // older
    binding.observe({ revision: 5, value: { plain: 3 } }); // duplicate
    assert.equal(binding.state.lastObserved?.value.plain, 1);
    assert.equal(binding.state.observedSeq, 1);
    assert.equal(binding.diagnostics.staleProjections, 2);
  });

  it("different non-numeric revision tokens supersede", () => {
    const { binding } = makeBinding({});
    binding.observe({ revision: "r1", value: { plain: 1 } });
    binding.observe({ revision: "r2", value: { plain: 2 } });
    binding.observe({ revision: "r2", value: { plain: 3 } }); // dup dropped
    assert.equal(binding.state.lastObserved?.value.plain, 2);
    assert.equal(binding.state.observedSeq, 2);
  });

  it("a projection mid-gesture updates truth but keeps preview phase", () => {
    const { binding } = makeBinding({});
    binding.event(gestureEvent("begin", "g1", 1));
    binding.observe({ revision: 1, value: { plain: 9 } });
    assert.equal(binding.state.phase, "preview");
    assert.equal(binding.state.lastObserved?.value.plain, 9);
  });
});

describe("coalescing through the binding", () => {
  it("a preview burst is measured and bounded", () => {
    const flush = deferredFlush();
    const { binding, gateway } = makeBinding({ flush });
    binding.event(gestureEvent("begin", "g1", 1));
    flush.runAll();
    for (let i = 2; i <= 60; i++) {
      binding.event(gestureEvent("preview", "g1", i, i / 100));
    }
    // One flush request for the whole burst.
    assert.equal(flush.pending(), 1);
    flush.runAll();
    const previews = gateway.received.filter((r) => r.intent.kind === "preview");
    assert.equal(previews.length, 1);
    if (previews[0]?.intent.kind === "preview") {
      assert.equal(previews[0].intent.value.plain, 0.6);
    }
    assert.equal(binding.diagnostics.coalesced, 58);
    // Boundaries intact.
    assert.equal(
      gateway.received.filter((r) => r.intent.kind === "begin").length,
      1,
    );
  });

  it("commit/cancel are never coalesced away", () => {
    const flush = deferredFlush();
    const { binding, gateway } = makeBinding({ flush });
    binding.event(gestureEvent("begin", "g1", 1));
    binding.event(gestureEvent("preview", "g1", 2));
    binding.event(gestureEvent("commit", "g1", 3));
    flush.runAll();
    assert.deepEqual(
      gateway.received.map((r) => r.intent.kind),
      ["begin", "commit"],
    );
    assert.equal(binding.diagnostics.supersededPreviews, 1);
  });
});

describe("detach / attach", () => {
  it("detach cancels the in-flight gesture through the port", () => {
    const { binding, gateway } = makeBinding({});
    binding.event(gestureEvent("begin", "g1", 1));
    binding.event(gestureEvent("preview", "g1", 2));
    binding.detach("dock-transfer");
    const last = gateway.received.at(-1);
    assert.equal(last?.intent.kind, "cancel");
    if (last?.intent.kind === "cancel") {
      assert.equal(last.intent.reason, "dock-transfer");
      assert.equal(last.intent.gestureId, "g1");
    }
    assert.equal(binding.state.phase, "stale");
  });

  it("detach purges queued intents — nothing else is dispatched", () => {
    const flush = deferredFlush();
    const { binding, gateway } = makeBinding({ flush });
    binding.event(gestureEvent("begin", "g1", 1));
    binding.event(gestureEvent("preview", "g1", 2));
    binding.detach("unmount");
    flush.runAll();
    // Only the detach-time cancel reached the port.
    assert.deepEqual(
      gateway.received.map((r) => r.intent.kind),
      ["cancel"],
    );
  });

  it("receipts from the dead generation are stale", () => {
    const { binding, gateway, timers } = makeBinding({});
    binding.event(gestureEvent("begin", "g1", 1));
    binding.event(gestureEvent("commit", "g1", 2));
    const commandId = gateway.lastCommandId()!;
    binding.detach("unmount");
    binding.attach();
    binding.ack({ commandId, status: "applied" });
    assert.equal(binding.diagnostics.staleReceipts, 1);
    assert.equal(binding.state.phase, "idle");
    void timers;
  });

  it("projections while detached are dropped", () => {
    const { binding } = makeBinding({});
    binding.detach("unmount");
    binding.observe({ revision: 1, value: { plain: 1 } });
    assert.equal(binding.state.lastObserved, undefined);
    assert.equal(binding.diagnostics.staleProjections, 1);
  });

  it("detached intents are counted, not dispatched", () => {
    const { binding, gateway } = makeBinding({});
    binding.detach("unmount");
    binding.event(gestureEvent("begin", "g2", 1));
    binding.action("play");
    assert.equal(gateway.received.length, 0);
    assert.equal(binding.diagnostics.detachedIntents, 2);
  });

  it("detach with no active gesture emits nothing", () => {
    const { binding, gateway } = makeBinding({});
    binding.detach("unmount");
    assert.equal(gateway.received.length, 0);
  });

  it("re-attach resumes dispatch under the new epoch", () => {
    const { binding, gateway } = makeBinding({});
    binding.event(gestureEvent("begin", "g1", 1));
    binding.detach("dock-transfer");
    binding.attach();
    binding.event(gestureEvent("begin", "g2", 2));
    binding.event(gestureEvent("commit", "g2", 3));
    assert.deepEqual(
      gateway.received.map((r) => r.intent.kind),
      ["begin", "cancel", "begin", "commit"],
    );
  });
});

describe("state listeners", () => {
  it("onState fires on every transition", () => {
    const states: BindingViewState<{ plain: number }>[] = [];
    const { binding, gateway } = makeBinding({ states });
    binding.event(gestureEvent("begin", "g1", 1));
    binding.event(gestureEvent("commit", "g1", 2));
    gateway.deliver({ commandId: gateway.lastCommandId()!, status: "applied" });
    const phases = states.map((s) => s.phase);
    assert.ok(phases.includes("preview"));
    assert.ok(phases.includes("pending"));
    assert.ok(phases.includes("applied"));
  });
});
