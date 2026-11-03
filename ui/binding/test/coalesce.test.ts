/**
 * Coalescing contract tests (Issue #261 acceptance):
 * continuous previews fold latest-wins inside a gesture; every
 * non-droppable kind — begin, commit, cancel, action — survives in
 * order and is never dropped; the folding is measured.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { IntentQueue } from "../src/coalesce.ts";
import type { CommandIntent } from "../src/contract.ts";

const VALUE = { plain: 0.5, normalized: 0.5 };

function preview(gestureId: string, n: number): CommandIntent {
  return {
    kind: "preview",
    target: "t",
    controlId: "c",
    sequence: n,
    gestureId,
    value: { plain: n, normalized: n / 100 },
    coalescible: true,
  };
}

function begin(gestureId: string, n = 0): CommandIntent {
  return {
    kind: "begin",
    target: "t",
    controlId: "c",
    sequence: n,
    gestureId,
    value: VALUE,
  };
}

function commit(gestureId: string, n: number): CommandIntent {
  return {
    kind: "commit",
    target: "t",
    controlId: "c",
    sequence: n,
    gestureId,
    value: { plain: n, normalized: n / 100 },
  };
}

function cancel(gestureId: string, n: number): CommandIntent {
  return {
    kind: "cancel",
    target: "t",
    controlId: "c",
    sequence: n,
    gestureId,
    reason: "explicit",
  };
}

function action(n: number): CommandIntent {
  return {
    kind: "action",
    target: "transport",
    controlId: "tp",
    sequence: n,
    action: "play",
  };
}

describe("IntentQueue", () => {
  it("coalesces previews of one gesture to the latest", () => {
    const q = new IntentQueue();
    for (let i = 1; i <= 100; i++) q.push(preview("g1", i));
    const out = q.drain();
    assert.equal(out.length, 1);
    assert.equal(out[0]?.kind, "preview");
    if (out[0]?.kind === "preview") assert.equal(out[0].value.plain, 100);
    assert.equal(q.stats.received, 100);
    assert.equal(q.stats.coalesced, 99);
    assert.equal(q.stats.emitted, 1);
  });

  it("keeps previews of different gestures separate", () => {
    const q = new IntentQueue();
    q.push(preview("g1", 1));
    q.push(preview("g2", 2));
    q.push(preview("g1", 3));
    const out = q.drain();
    assert.equal(out.length, 2);
    assert.deepEqual(
      out.map((c) => (c.kind === "preview" ? c.value.plain : 0)),
      [3, 2],
    );
  });

  it("never drops or reorders begin/commit/cancel/action", () => {
    const q = new IntentQueue();
    q.push(begin("g1", 1));
    q.push(commit("g1", 2));
    q.push(begin("g2", 3));
    q.push(cancel("g2", 4));
    q.push(action(5));
    q.push(commit("g3", 6));
    const out = q.drain();
    assert.deepEqual(
      out.map((c) => c.kind),
      ["begin", "commit", "begin", "cancel", "action", "commit"],
    );
    assert.equal(q.stats.coalesced, 0);
  });

  it("a terminal supersedes its gesture's buffered preview", () => {
    const q = new IntentQueue();
    q.push(begin("g1", 1));
    q.push(preview("g1", 2));
    q.push(preview("g1", 3));
    q.push(commit("g1", 4));
    const out = q.drain();
    // The commit IS the final preview — the buffered ones die before it.
    assert.deepEqual(
      out.map((c) => c.kind),
      ["begin", "commit"],
    );
    assert.equal(q.stats.superseded, 1);
  });

  it("cancel supersedes buffered previews the same way", () => {
    const q = new IntentQueue();
    q.push(preview("g1", 1));
    q.push(preview("g1", 2));
    q.push(cancel("g1", 3));
    const out = q.drain();
    assert.deepEqual(out.map((c) => c.kind), ["cancel"]);
  });

  it("previews after a terminal start a fresh slot", () => {
    const q = new IntentQueue();
    q.push(preview("g1", 1));
    q.drain();
    q.push(preview("g1", 2));
    q.push(preview("g1", 3));
    const out = q.drain();
    assert.equal(out.length, 1);
    if (out[0]?.kind === "preview") assert.equal(out[0].value.plain, 3);
  });

  it("purge drops everything without emitting", () => {
    const q = new IntentQueue();
    q.push(begin("g1", 1));
    q.push(preview("g1", 2));
    q.push(commit("g1", 3));
    q.purge();
    assert.deepEqual(q.drain(), []);
    assert.equal(q.stats.emitted, 0);
  });

  it("an interleaved second gesture's preview survives the first's commit", () => {
    const q = new IntentQueue();
    q.push(begin("g1", 1));
    q.push(preview("g1", 2));
    q.push(preview("g2", 3));
    q.push(commit("g1", 4));
    const out = q.drain();
    assert.deepEqual(
      out.map((c) => (c.kind === "preview" ? `p${c.value.plain}` : c.kind)),
      ["begin", "p3", "commit"],
    );
  });
});
