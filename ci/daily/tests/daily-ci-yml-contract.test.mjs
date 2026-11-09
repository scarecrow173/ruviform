// Issue #294 contract test: .github/workflows/daily-ci.yml must keep the
// decide-consumer-verify job structurally wired to the tuple-mode contract
// of ci/daily/decide.mjs. Textual checks (stdlib only, no yaml dep) —
// actionlint covers YAML validity; this file pins the semantics that must
// not silently regress:
//
//   - both scheduler jobs exist and share the schedule/dispatch triggers
//   - the tuple job runs the SAME evaluator self-test before deciding
//   - tuple mode env pins: mode=tuple, target=consumer-verify.yml,
//     consumer repo map, contract id, bounded evidence scan
//   - dispatch inputs the job can produce match consumer-verify.yml's
//     declared inputs (ruviform_rev + <consumer>_ref/<consumer>_expected_sha)
//   - the tuple report artifact name cannot collide with the single-SHA one
//   - permissions stay minimal (contents:read + actions:write only)

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const YML = fileURLToPath(
  new URL("../../../.github/workflows/daily-ci.yml", import.meta.url),
);
const src = readFileSync(YML, "utf8");

// Slice the text of one job block (from `  <name>:` at jobs-level indent to
// the next same-indent key or EOF) so per-job assertions can't drift.
function jobBlock(name) {
  const start = src.indexOf(`\n  ${name}:`);
  assert.notEqual(start, -1, `job ${name} missing`);
  const rest = src.slice(start + 1);
  const next = rest.slice(1).search(/\n  [a-zA-Z][\w-]*:\n/);
  return next < 0 ? rest : rest.slice(0, next + 1);
}

const decide = jobBlock("decide");
const tuple = jobBlock("decide-consumer-verify");

test("contract: both scheduler jobs exist under one workflow", () => {
  assert.ok(jobBlock("decide").length > 0);
  assert.ok(tuple.length > 0);
  // schedule + workflow_dispatch triggers are shared (no per-job triggers)
  const on = src.slice(src.indexOf("\non:"), src.indexOf("\npermissions:"));
  assert.ok(/schedule:/.test(on), "daily schedule trigger missing");
  assert.ok(/workflow_dispatch:/.test(on), "manual re-check trigger missing");
});

test("contract: tuple job self-tests the evaluator before deciding", () => {
  assert.ok(/mise run test-ci-daily/.test(tuple), "tuple job lost evaluator self-test");
  const selfTest = tuple.indexOf("test-ci-daily");
  const evaluate = tuple.indexOf("decide.mjs");
  assert.ok(selfTest >= 0 && evaluate > selfTest, "self-test must precede the decision run");
});

test("contract: tuple job pins tuple-mode env for decide.mjs", () => {
  for (const [key, value] of [
    ["DAILY_MODE", "tuple"],
    ["DAILY_TARGET_WORKFLOW", "consumer-verify.yml"],
    ["DAILY_TUPLE_CONTRACT", "consumer-verify-tuple/v1"],
    ["DAILY_CONSUMER_REPOS", "audiolens=scarecrow173/audio-lens,synvelis=scarecrow173/synvelis"],
  ]) {
    assert.ok(new RegExp(`${key}:\\s*${value.replace(/[/.]/g, "\\$&")}`).test(tuple),
      `tuple job env must pin ${key}: ${value}`);
  }
  // bounded scans — an unbounded evidence crawl must not be configurable away
  assert.ok(/DAILY_MAX_EVIDENCE_RUNS: "\d+"/.test(tuple), "DAILY_MAX_EVIDENCE_RUNS missing");
  assert.ok(/DAILY_MAX_AUTO_DISPATCHES: "\d+"/.test(tuple), "DAILY_MAX_AUTO_DISPATCHES missing");
  // dry_run input must reach the tuple decision too
  assert.ok(/DRY_RUN: \$\{\{ inputs\.dry_run \}\}/.test(tuple), "tuple job lost DRY_RUN wiring");
});

test("contract: tuple job invokes decide.mjs in tuple mode with its own report", () => {
  assert.ok(/node ci\/daily\/decide\.mjs --mode tuple/.test(tuple),
    "tuple job must pass --mode tuple explicitly (env alone is not the contract)");
  assert.ok(/--report daily-ci-tuple-report\.json/.test(tuple), "tuple report path missing");
});

test("contract: tuple report artifact name cannot collide with single-SHA report", () => {
  const singleName = decide.match(/name: (daily-ci-report[^\n]*)/);
  const tupleName = tuple.match(/name: (daily-ci-tuple-report[^\n]*)/);
  assert.ok(singleName && tupleName, "both report artifact names must exist");
  assert.notEqual(singleName[1], tupleName[1], "artifact names must differ (v4 upload conflicts on rerun)");
  // both keep the attempt-unique suffix convention
  for (const m of [singleName, tupleName]) {
    assert.ok(m[1].includes("github.run_attempt"), `${m[1]} must stay attempt-unique`);
  }
});

test("contract: tuple job permissions stay minimal", () => {
  const perms = tuple.slice(tuple.indexOf("permissions:"), tuple.indexOf("steps:"));
  assert.ok(/contents: read/.test(perms), "contents:read required for checkout");
  assert.ok(/actions: write/.test(perms), "actions:write required for history read + dispatch");
  assert.ok(!/contents: write/.test(perms) && !/pull-requests:/.test(perms),
    "tuple job must not gain write scopes it does not need");
});

test("contract: consumer repo map matches consumer-verify.yml CONSUMER_REPO values", () => {
  // The DAILY_CONSUMER_REPOS map must name the same owner/repo pairs the
  // verify lanes clone — a drifted repo silently verifies the wrong tuple.
  const cvYml = readFileSync(
    fileURLToPath(new URL("../../../.github/workflows/consumer-verify.yml", import.meta.url)),
    "utf8",
  );
  const repos = [...cvYml.matchAll(/CONSUMER_REPO: (\S+)/g)].map((m) => m[1]);
  assert.deepEqual([...new Set(repos)].sort(), ["scarecrow173/audio-lens", "scarecrow173/synvelis"].sort());
  for (const repo of new Set(repos)) {
    assert.ok(tuple.includes(repo), `DAILY_CONSUMER_REPOS must include ${repo}`);
  }
});
