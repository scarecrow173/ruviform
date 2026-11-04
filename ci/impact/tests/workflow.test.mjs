// workflow.test.mjs — Issue #346 (CI-OPT-06 enforcement) wiring contract.
// The classification contract lives in classify/gate tests; this file pins
// the ci.yml structure that connects them: every lane must consume its
// `lane_*` impact output through the fail-open `if:` shape, `impact` must
// run unconditionally, and `ci-gate` must aggregate every lane + impact
// under `if: always()`. A dropped `if:` silently un-skips lanes; a wrong
// output name silently skips a required lane (gate fails closed, but the
// PR deadlocks on the still-required lane context until the ruleset
// migration completes) — both are caught here.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { LANE_IDS } from "../map.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const WORKFLOW = readFileSync(join(ROOT, ".github/workflows/ci.yml"), "utf8");

// Job blocks start at `  <id>:` (two-space indent) and end at the next key
// at the same indent or EOF. Step keys and comments never sit at that
// indent, so a line-anchored scan is sufficient for this flat job list.
function jobBlock(text, jobId) {
  const lines = text.split("\n");
  const start = lines.findIndex((l) => l === `  ${jobId}:`);
  assert.notEqual(start, -1, `job ${jobId} not found in ci.yml`);
  let end = lines.length;
  for (let i = start + 1; i < lines.length; i++) {
    if (/^  \S/.test(lines[i])) {
      end = i;
      break;
    }
  }
  return lines.slice(start, end).join("\n");
}

const laneOutput = (id) => `lane_${id.replaceAll("-", "_")}`;

test("every lane job needs impact and gates on its lane_* output fail-open", () => {
  for (const id of LANE_IDS) {
    const block = jobBlock(WORKFLOW, id);
    assert.match(block, /^    needs: \[impact\]$/m, `${id} must declare needs: [impact]`);
    assert.ok(
      block.includes(`if: always() && needs.impact.outputs.${laneOutput(id)} != 'false'`),
      `${id} must run unless classifier proved it unaffected (always() && != 'false')`,
    );
  }
});

test("impact job has no job-level if (always emits the expected set)", () => {
  const block = jobBlock(WORKFLOW, "impact");
  assert.doesNotMatch(block, /^    if:/m, "impact must not be conditionally skipped");
  for (const id of LANE_IDS) {
    assert.ok(
      block.includes(`      ${laneOutput(id)}:`),
      `impact job outputs must declare ${laneOutput(id)}`,
    );
  }
});

test("ci-gate aggregates every lane + impact under if: always()", () => {
  const block = jobBlock(WORKFLOW, "ci-gate");
  assert.match(block, /^    if: always\(\)$/m, "ci-gate must run on skipped/failed needs");
  const needsLine = block.match(/^    needs: \[([^\]]+)\]/m);
  assert.ok(needsLine, "ci-gate needs list not found");
  const needs = needsLine[1].split(",").map((s) => s.trim());
  for (const id of ["impact", ...LANE_IDS]) {
    assert.ok(needs.includes(id), `ci-gate needs must include ${id}`);
  }
});
