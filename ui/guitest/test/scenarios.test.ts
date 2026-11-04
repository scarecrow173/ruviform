/**
 * Issue #311 — scenario replay + oracle conformance.
 *
 * For every `scenarios/binding/*.v1.json` fixture:
 * 1. the real `InteractionControl`/`CommandBinding`/`QueryBinding`/
 *    `ViewLink` path runs the `in` op list;
 * 2. its observable trace must deep-equal the independent
 *    `ContractModel` oracle's expected trace (replaying recorded
 *    responses is never a PASS);
 * 3. the `expect` pins hold on the actual trace;
 * 4. a second identical run produces an identical trace (AC-2
 *    deterministic replay).
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { FIXTURES_DIR, loadScenarioFile } from "../src/fixtureload.ts";
import {
  checkExpects,
  runScenario,
  type ScenarioDoc,
} from "../src/replay.ts";

const DIR = join(FIXTURES_DIR, "scenarios", "binding");
const files = readdirSync(DIR)
  .filter((f) => f.endsWith(".v1.json"))
  .sort();

assert.ok(files.length > 0, "no binding scenario fixtures");

for (const file of files) {
  const rel = `scenarios/binding/${file}`;
  const loaded = loadScenarioFile(rel);
  const doc = loaded.doc as unknown as ScenarioDoc;

  describe(loaded.id, () => {
    it("trace deep-equals the independent oracle", async () => {
      const { actual, expected } = await runScenario(doc);
      assert.deepStrictEqual(actual, expected);
    });

    it("expect pins hold on the actual trace", async () => {
      const { actual } = await runScenario(doc);
      const firstControl = doc.setup.targets[0]?.control ?? "";
      const fails = checkExpects(actual, doc.expect ?? {}, firstControl);
      assert.deepEqual(fails, [], fails.join("\n"));
    });

    it("identical seed/clock/inputs → identical trace (determinism)", async () => {
      const a = await runScenario(doc);
      const b = await runScenario(doc);
      assert.deepStrictEqual(a.actual, b.actual);
    });
  });
}
