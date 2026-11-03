/**
 * DOM adapter tests (Issue #261): `rvf-gesture`/`rvf-transport`
 * CustomEvents from real elements flow into the binding; receipts and
 * projections reflect back as pending/invalid/unsupported/value —
 * idempotently and loop-free.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import "./env.ts";
import { unmount } from "./env.ts";
import {
  registerRvfElements,
  type RvfControlElement,
  type RvfTransport,
} from "../../components/src/index.ts";
import { CommandBinding } from "../src/binding.ts";
import {
  bindControlElement,
  bindTransportElement,
  bindViewIntents,
} from "../src/dom.ts";
import { flushSync, ManualTimers, MockDawGateway } from "./mocks.ts";

registerRvfElements();

function makeFader(): RvfControlElement {
  const el = document.createElement("rvf-fader") as RvfControlElement;
  el.setAttribute("control-id", "f1");
  el.descriptor = { min: 0, max: 1, defaultValue: 0.2 };
  el.target = "track.vol";
  document.body.append(el);
  return el;
}

function makeBinding(
  gateway: MockDawGateway,
  timers = new ManualTimers(),
): CommandBinding<{ plain: number }> {
  const binding = new CommandBinding<{ plain: number }>({
    target: "track.vol",
    port: gateway,
    schedule: timers.schedule,
    flushWith: flushSync,
    ackTimeoutMs: 1000,
  });
  gateway.deliver = (r) => binding.ack(r);
  return binding;
}

describe("bindControlElement", () => {
  it("gesture intents from a real element reach the port", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    const unbind = bindControlElement(el, binding);
    el.dispatchInput({ type: "begin", gestureId: "g1", source: "pointer" });
    el.dispatchInput({ type: "commit", gestureId: "g1" });
    assert.deepEqual(
      gateway.received.map((r) => r.intent.kind),
      ["begin", "commit"],
    );
    const commit = gateway.received.at(-1);
    if (commit?.intent.kind === "commit") {
      assert.equal(commit.intent.target, "track.vol");
      assert.equal(commit.intent.controlId, "f1");
    }
    unbind();
    unmount(el);
  });

  it("a pending commit reflects on the element's pending flag", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    bindControlElement(el, binding);
    el.dispatchInput({ type: "begin", gestureId: "g1", source: "pointer" });
    el.dispatchInput({ type: "commit", gestureId: "g1" });
    assert.equal(el.pending, true);
    gateway.deliver({
      commandId: gateway.lastCommandId()!,
      status: "applied",
    });
    // applied keeps pending until the projection confirms…
    assert.equal(el.pending, true);
    binding.observe({ revision: 1, value: { plain: 0.5 } });
    assert.equal(el.pending, false);
    unmount(el);
  });

  it("an authoritative projection writes the controlled value — with no command loop", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    bindControlElement(el, binding);
    const before = gateway.received.length;
    binding.observe({ revision: 3, value: { plain: 0.7 } });
    assert.equal(el.value, 0.7);
    assert.equal(gateway.received.length, before);
    unmount(el);
  });

  it("a stale projection does not touch the element", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    bindControlElement(el, binding);
    binding.observe({ revision: 3, value: { plain: 0.7 } });
    binding.observe({ revision: 2, value: { plain: 0.1 } });
    assert.equal(el.value, 0.7);
    unmount(el);
  });

  it("a rejected commit reverts to the last observed value", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    bindControlElement(el, binding);
    binding.observe({ revision: 1, value: { plain: 0.2 } });
    el.dispatchInput({
      type: "set",
      value: { plain: 0.9 },
      source: "pointer",
    });
    // The local commit landed…
    assert.equal(el.value, 0.9);
    // …but the gateway refused it — the display reverts to truth.
    gateway.deliver({
      commandId: gateway.lastCommandId()!,
      status: "rejected",
    });
    assert.equal(el.value, 0.2);
    assert.equal(el.invalid, true);
    unmount(el);
  });

  it("a timeout marks the element invalid and reverts", () => {
    const timers = new ManualTimers();
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway, timers);
    bindControlElement(el, binding);
    binding.observe({ revision: 1, value: { plain: 0.2 } });
    el.dispatchInput({ type: "set", value: { plain: 0.9 }, source: "pointer" });
    timers.advance(1100);
    assert.equal(el.invalid, true);
    assert.equal(el.value, 0.2);
    unmount(el);
  });

  it("bind is idempotent — no duplicated listeners or commands", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    const u1 = bindControlElement(el, binding);
    const u2 = bindControlElement(el, binding);
    assert.equal(u1, u2);
    el.dispatchInput({ type: "set", value: { plain: 0.5 }, source: "pointer" });
    assert.deepEqual(
      gateway.received.map((r) => r.intent.kind),
      ["begin", "commit"],
    );
    unmount(el);
  });

  it("unbind is idempotent and stops the flow both ways", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    const unbind = bindControlElement(el, binding);
    unbind();
    unbind();
    el.dispatchInput({ type: "set", value: { plain: 0.5 }, source: "pointer" });
    assert.equal(gateway.received.length, 0);
    unmount(el);
  });

  it("a StrictMode-style rebind revives the detached binding", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    bindControlElement(el, binding)();
    assert.equal(binding.attached, false);
    bindControlElement(el, binding);
    assert.equal(binding.attached, true);
    el.dispatchInput({ type: "set", value: { plain: 0.4 }, source: "pointer" });
    assert.deepEqual(
      gateway.received.map((r) => r.intent.kind),
      ["begin", "commit"],
    );
    unmount(el);
  });

  it("element detach delivers the gesture cancel before unbind", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    bindControlElement(el, binding);
    el.dispatchInput({ type: "begin", gestureId: "g1", source: "pointer" });
    unmount(el); // element-level detach cancels the gesture
    assert.equal(gateway.received.at(-1)?.intent.kind, "cancel");
  });

  it("unbind while a gesture is active cancels it through the port", () => {
    const gateway = new MockDawGateway();
    const el = makeFader();
    const binding = makeBinding(gateway);
    const unbind = bindControlElement(el, binding);
    el.dispatchInput({ type: "begin", gestureId: "g1", source: "pointer" });
    unbind();
    assert.equal(gateway.received.at(-1)?.intent.kind, "cancel");
    unmount(el);
  });
});

describe("bindTransportElement", () => {
  it("transport clicks dispatch discrete action commands", () => {
    const gateway = new MockDawGateway();
    const el = document.createElement("rvf-transport") as RvfTransport;
    document.body.append(el);
    const binding = makeBinding(gateway);
    bindTransportElement(el, binding);
    const play = el.shadowRoot?.querySelector('[part~="play"]');
    play?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    const r = gateway.received.at(-1);
    assert.equal(r?.intent.kind, "action");
    if (r?.intent.kind === "action") {
      assert.equal(r.intent.action, "play");
      assert.equal(r.intent.controlId, "rvf-transport");
    }
    unmount(el);
  });
});

describe("bindViewIntents", () => {
  it("routes typed view events and unbinds them", () => {
    const seen: string[] = [];
    const el = document.createElement("div");
    const unbind = bindViewIntents(el as unknown as Parameters<typeof bindViewIntents>[0], {
      onViewport: () => seen.push("viewport"),
      onSelect: () => seen.push("select"),
    });
    const fire = (type: string) =>
      el.dispatchEvent(new CustomEvent(type, { detail: {} }));
    fire("rvf-viewport");
    fire("rvf-select");
    fire("rvf-backend");
    assert.deepEqual(seen, ["viewport", "select"]);
    unbind();
    fire("rvf-viewport");
    assert.deepEqual(seen, ["viewport", "select"]);
  });
});
