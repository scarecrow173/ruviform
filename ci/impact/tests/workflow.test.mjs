// workflow.test.mjs — Issue #346 (CI-OPT-06 enforcement) wiring contract.
// The classification contract lives in classify/gate/observe tests; this
// file pins the ci.yml structure that connects them:
//   - every lane job consumes its `lane_*` impact output through the
//     fail-open `if:` shape and `impact` runs unconditionally;
//   - the three matrix lanes carry the PR-required OSes only — the macOS
//     legs live in physically separate `*-macos` jobs (Issue #333/#326 rev
//     2026-11-05) consumed by `macos-report`, never by `ci-gate`;
//   - `ci-gate` aggregates impact + every required lane under
//     `if: always()` and contains no `*-macos` need — macOS can starve
//     without deadlocking or cancelling a required context.
// A dropped `if:` silently un-skips lanes; a macOS leg back inside a gated
// need re-creates the queue-cancel deadlock this structure exists to kill.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { LANE_IDS, OBSERVE_LANES, OBSERVE_LANE_IDS } from "../map.mjs";

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

test("every required lane job needs impact and gates on its lane_* output fail-open", () => {
  for (const id of LANE_IDS) {
    const block = jobBlock(WORKFLOW, id);
    assert.match(block, /^    needs: \[impact\]$/m, `${id} must declare needs: [impact]`);
    assert.ok(
      block.includes(`if: always() && needs.impact.outputs.${laneOutput(id)} != 'false'`),
      `${id} must run unless classifier proved it unaffected (always() && != 'false')`,
    );
  }
});

test("required matrices carry ubuntu+windows only — no macOS inside gated jobs", () => {
  for (const id of ["verify", "ui-verify", "l1-real-verify"]) {
    const block = jobBlock(WORKFLOW, id);
    assert.match(
      block,
      /^        os: \[ubuntu-latest, windows-latest\]$/m,
      `${id} matrix must be exactly [ubuntu-latest, windows-latest]`,
    );
    assert.doesNotMatch(
      block,
      /macos-latest/,
      `${id} must not contain a macOS leg (it lives in ${OBSERVE_LANES[`${id}-macos`] ? `${id}-macos` : "a *-macos job"})`,
    );
  }
});

test("every *-macos observe job exists, needs impact, gates on the parent lane output", () => {
  for (const jobId of OBSERVE_LANE_IDS) {
    const parent = OBSERVE_LANES[jobId].lane;
    const block = jobBlock(WORKFLOW, jobId);
    assert.match(block, /^    needs: \[impact\]$/m, `${jobId} must declare needs: [impact]`);
    assert.ok(
      block.includes(`if: always() && needs.impact.outputs.${laneOutput(parent)} != 'false'`),
      `${jobId} must run unless classifier proved ${parent} unaffected`,
    );
    assert.match(block, /^    runs-on: macos-latest$/m, `${jobId} must run on macos-latest`);
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

test("ci-gate aggregates impact + required lanes under if: always() — no macOS needs", () => {
  const block = jobBlock(WORKFLOW, "ci-gate");
  assert.match(block, /^    if: always\(\)$/m, "ci-gate must run on skipped/failed needs");
  const needsLine = block.match(/^    needs: \[([^\]]+)\]/m);
  assert.ok(needsLine, "ci-gate needs list not found");
  const needs = needsLine[1].split(",").map((s) => s.trim());
  for (const id of ["impact", ...LANE_IDS]) {
    assert.ok(needs.includes(id), `ci-gate needs must include ${id}`);
  }
  for (const id of OBSERVE_LANE_IDS) {
    assert.ok(!needs.includes(id), `ci-gate needs must NOT include ${id}`);
  }
  assert.doesNotMatch(needsLine[0], /macos/i, "ci-gate needs must not reference any macOS job");
});

test("macos-report aggregates impact + every *-macos observe job under if: always()", () => {
  const block = jobBlock(WORKFLOW, "macos-report");
  assert.match(block, /^    if: always\(\)$/m, "macos-report must run on skipped/failed needs");
  const needsLine = block.match(/^    needs: \[([^\]]+)\]/m);
  assert.ok(needsLine, "macos-report needs list not found");
  const needs = needsLine[1].split(",").map((s) => s.trim());
  for (const id of ["impact", ...OBSERVE_LANE_IDS]) {
    assert.ok(needs.includes(id), `macos-report needs must include ${id}`);
  }
  for (const id of LANE_IDS) {
    assert.ok(!needs.includes(id), `macos-report must not wait on required lane ${id}`);
  }
});
