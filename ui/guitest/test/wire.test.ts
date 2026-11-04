/**
 * Issue #311 — `rvf.ipc.v1` + `rvf://data` fixture-driven conformance.
 *
 * The same corpus drives the Rust golden lane
 * (`crates/ruviform-ui/tests/ipc_conformance.rs`) — every case here is
 * a direct mirror of what Rust accepts/rejects, so both sides process
 * identical wire shapes with identical meaning.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { loadFixtureFile } from "../src/fixtureload.ts";
import {
  dataUrl,
  decodeEvent,
  parseIntent,
  serveData,
} from "../src/ipcwire.ts";

function utf8Len(s: string): number {
  return new TextEncoder().encode(s).length;
}

/** `bodyGen: {kind:"pad-intent", padTo:N}` — a valid intent padded to
 *  exactly N bytes via the `params.pad` string. */
function padIntent(padTo: number): string {
  const head = '{"v":1,"kind":"intent","id":"i-pad","op":"sys.ping","params":{"pad":"';
  const tail = '"}}';
  const n = padTo - utf8Len(head) - utf8Len(tail);
  assert.ok(n >= 0, `padTo ${padTo} smaller than frame`);
  return head + "x".repeat(n) + tail;
}

describe("ipc-intents fixtures", () => {
  const { cases } = loadFixtureFile("ipc/intents.v1.json");
  assert.ok(cases.length > 0);
  for (const c of cases) {
    it(String(c.id), () => {
      const gen = c.bodyGen as { kind?: string; padTo?: number } | undefined;
      const body =
        gen?.kind === "pad-intent"
          ? padIntent(gen.padTo!)
          : (c.body as string);
      const r = parseIntent(body);
      const e = c.expect as Record<string, unknown>;
      assert.equal(r.ok, e.accept, `${c.id}: accept mismatch`);
      if (e.accept === true) {
        assert.ok(r.ok);
        if (e.id !== undefined) assert.equal(r.intent.id, e.id);
        if (e.op !== undefined) assert.equal(r.intent.op, e.op);
        if ("params" in e) assert.deepEqual(r.intent.params, e.params);
      } else {
        assert.ok(!r.ok);
        assert.equal(r.code, e.code);
      }
    });
  }
});

describe("ipc-events fixtures", () => {
  const { cases } = loadFixtureFile("ipc/events.v1.json");
  for (const c of cases) {
    it(String(c.id), () => {
      const r = decodeEvent(c.wire);
      const e = c.expect as Record<string, unknown>;
      assert.equal(r.ok, e.accept, `${c.id}: accept mismatch`);
      if (e.accept !== true) {
        assert.ok(!r.ok);
        assert.equal(r.code, e.code);
        return;
      }
      assert.ok(r.ok);
      assert.equal(r.event.class, e.class);
      if (e.correlation !== undefined) {
        const corr =
          r.event.class === "result"
            ? r.event.id
            : r.event.class === "job"
              ? r.event.job
              : r.event.class === "notify"
                ? r.event.seq
                : r.event.class === "lifecycle"
                  ? r.event.state
                  : undefined;
        assert.equal(corr, e.correlation, `${c.id}: correlation`);
      }
      if (e.ok !== undefined && r.event.class === "result") {
        assert.equal(r.event.ok, e.ok);
      }
      if (e.state !== undefined && r.event.class === "job") {
        assert.equal(r.event.state, e.state);
      }
      if (e.code !== undefined) {
        const err =
          r.event.class === "result" ||
          r.event.class === "job" ||
          r.event.class === "error"
            ? r.event.error
            : undefined;
        assert.equal(err?.code, e.code, `${c.id}: error code`);
      }
    });
  }
});

describe("ipc-data fixtures", () => {
  const { cases } = loadFixtureFile("ipc/data.v1.json");
  for (const c of cases) {
    it(String(c.id), () => {
      const e = c.expect as Record<string, unknown>;
      if (e.url !== undefined) {
        const call = c.call as { name: string; offset?: number; len?: number };
        assert.equal(dataUrl(call.name, call.offset, call.len), e.url);
        return;
      }
      const reg = c.registry as {
        maxChunk: number;
        sources: { name: string; size: number }[];
      };
      const req = c.request as {
        name: string;
        offset?: number;
        len?: number;
        rawQuery?: string;
        host?: string;
      };
      const sources = new Map(reg.sources.map((s) => [s.name, s.size]));
      const query =
        req.rawQuery ??
        `offset=${req.offset ?? 0}` +
          (req.len !== undefined ? `&len=${req.len}` : "");
      const r = serveData(sources, BigInt(reg.maxChunk), req.name, query);
      assert.equal(r.status, e.status, `${c.id}: status`);
      if (e.status === 200) {
        assert.equal(r.bodyLen, e.bodyLen, `${c.id}: bodyLen`);
        assert.equal(r.total, e.total, `${c.id}: total`);
        assert.equal(r.headers["Content-Type"], "application/octet-stream");
        assert.equal(r.headers["Access-Control-Allow-Origin"], "*");
        assert.equal(r.headers["Cache-Control"], "no-store");
        assert.equal(r.headers["X-Rvf-Total"], String(e.total));
      } else {
        assert.equal(r.errorClass, e.class, `${c.id}: errorClass`);
      }
    });
  }
});
