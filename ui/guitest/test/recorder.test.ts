/**
 * Issue #311 — `IpcRecorder` contract: opt-in, allowlisted, bounded.
 *
 * - disabled: `record`/`wrapPort` are transparent (zero observable
 *   effect, no allocation growth);
 * - enabled: records direction + correlation metadata only, JSONL
 *   round-trips through `parseRecording`;
 * - bounded: count/byte overflow emits one `sys/dropped` marker and
 *   counts every further record — never silently;
 * - direction: binding→consumer calls are `out`, consumer-originated
 *   replies are `in`.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { IpcRecorder, verifyJsonl } from "../src/recorder.ts";
import { parseRecording } from "../src/jsonl.ts";
import { ScriptedDawBackend } from "../src/consumers.ts";

describe("IpcRecorder", () => {
  it("disabled recorder is a transparent no-op", () => {
    const rec = new IpcRecorder({ enabled: false });
    const port = new ScriptedDawBackend();
    rec.record("in", "gesture", { controlId: "c" });
    const wrapped = rec.wrapPort(port);
    assert.equal(rec.size, 0);
    assert.equal(rec.dropped, 0);
    assert.equal(rec.toJsonl(), "");
    assert.equal(wrapped, port, "disabled wrapPort must return the same port");
  });

  it("records allowlisted metadata only, JSONL round-trips", () => {
    const rec = new IpcRecorder({ enabled: true });
    const port = rec.wrapPort(new ScriptedDawBackend());
    port.dispatch(
      {
        kind: "commit",
        target: "mixer.ch1.fader",
        controlId: "fader-1",
        sequence: 3,
        gestureId: "g-1",
        value: { plain: 0.5, normalized: 0.5 },
      },
      {
        commandId: "cmd-1",
        expectReceipt: true,
        generation: 0,
        issuedAt: 0,
      },
    );
    const text = rec.toJsonl();
    assert.ok(verifyJsonl(text).ok);
    const parsed = parseRecording(text);
    assert.ok(parsed.ok);
    const r = parsed.records[0]!;
    assert.equal(r.dir, "out");
    assert.equal(r.type, "dispatch");
    assert.equal(r.fields.commandId, "cmd-1");
    assert.equal(r.fields.kind, "commit");
    assert.equal(r.fields.gestureId, "g-1");
    // value payload must never reach the recording
    assert.equal("value" in r.fields, false);
    assert.equal(JSON.stringify(text).includes("normalized"), false);
  });

  it("subscribe records out; delivered projections are recorded in by the caller", () => {
    const rec = new IpcRecorder({ enabled: true });
    const backend = new ScriptedDawBackend();
    backend.addTarget("t", 0);
    const port = rec.wrapPort(backend);
    const off = port.subscribe!(
      "t",
      (p) => {
        rec.record("in", "projection", {
          target: "t",
          revision: p.revision,
        });
      },
      { viewId: "view-1", generation: 0 },
    );
    off?.();
    const parsed = parseRecording(rec.toJsonl());
    assert.ok(parsed.ok);
    const dirs = parsed.records.map((r) => `${r.dir}:${r.type}`);
    assert.deepEqual(dirs, [
      "out:subscribe",
      "in:projection",
      "out:unsubscribe",
    ]);
  });

  it("capacity bounds emit one sys/dropped marker then count", () => {
    const rec = new IpcRecorder({ enabled: true, maxEntries: 4 });
    for (let i = 0; i < 10; i += 1) {
      rec.record("in", "gesture", { controlId: `c${i}` });
    }
    assert.equal(rec.dropped, 7); // 10 attempts - 3 kept - 1 marker
    const parsed = parseRecording(rec.toJsonl());
    assert.ok(parsed.ok);
    const types = parsed.records.map((r) => r.type);
    assert.equal(types.at(-1), "dropped");
    assert.equal(parsed.records.at(-1)!.dir, "sys");
    assert.equal(parsed.records.filter((r) => r.type === "dropped").length, 1);
  });
});
