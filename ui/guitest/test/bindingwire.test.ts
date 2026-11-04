/**
 * Issue #311 — `ui/binding` contract message fixtures.
 *
 * Every message shape gets the stable rejection vocabulary
 * (`missing-field`/`bad-kind`/`bad-type`/`out-of-range`), mirroring the
 * wire semantics both Rust hosts and TS bindings must agree on.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { loadFixtureFile } from "../src/fixtureload.ts";
import {
  validateCommandIntent,
  validateProjection,
  validateQueryReply,
  validateReceipt,
  validateViewportQuery,
  type Validation,
} from "../src/bindingwire.ts";

const ROUTES: Array<{
  prefix: string;
  cls: string;
  run: (m: unknown) => Validation<unknown>;
}> = [
  { prefix: "intent-", cls: "intent", run: validateCommandIntent },
  { prefix: "receipt-", cls: "receipt", run: validateReceipt },
  { prefix: "projection-", cls: "projection", run: validateProjection },
  { prefix: "viewport-query-", cls: "viewportQuery", run: validateViewportQuery },
  { prefix: "query-reply-", cls: "queryReply", run: validateQueryReply },
];

describe("binding-messages fixtures", () => {
  const { cases } = loadFixtureFile("binding/messages.v1.json");
  for (const c of cases) {
    it(String(c.id), () => {
      const id = String(c.id);
      const route = ROUTES.find((r) => id.startsWith(r.prefix));
      assert.ok(route, `${id}: no validator route`);
      const r = route.run(c.message);
      const e = c.expect as Record<string, unknown>;
      assert.equal(r.ok, e.accept, `${id}: accept mismatch`);
      if (e.accept === true) {
        assert.ok(r.ok);
        if (e.commandKind !== undefined) {
          assert.equal(
            (r.value as { kind?: string }).kind,
            e.commandKind,
            `${id}: kind`,
          );
        }
      } else {
        assert.ok(!r.ok);
        assert.equal(r.reason, e.reason, `${id}: reason`);
      }
    });
  }
});
