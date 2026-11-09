// issue-tracker-workflows-contract.test.mjs — Issue #294 contract test:
// consumer-verify failure tracking wiring.
//
// Pins the boundary between the two tracked workflows and the evaluator:
//   - ci-issue-tracker.yml watches BOTH `ci` and `consumer-verify`
//     workflow_run completions on main, with one job per scope gated on
//     the completed run's workflow name — a consumer-verify completion
//     can never be evaluated as `ci` evidence.
//   - ci-issue-tracker-reconcile.yml reconciles BOTH scopes, covering the
//     anti-recursion gap: daily-ci dispatches consumer-verify with
//     GITHUB_TOKEN, which fires no workflow_run event.
//   - The verdict mapping stays honest end to end: consumer evidence
//     statuses (pass/fail/not_run/unsupported/skipped) derive through
//     ci/consumer-verify/evidence.mjs, the workflow summary fail-closes
//     the run conclusion, and the tracker maps only failure conclusions
//     to failure — operational conclusions are neither pass nor fail.
//
// Textual YAML inspection only (stdlib, no yaml dep) — same convention as
// ci/daily/tests/ci-yml-contract.test.mjs.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { deriveStatus, OUTCOMES } from "../../consumer-verify/evidence.mjs";
import { FAILURE_CONCLUSIONS, kindOfConclusion } from "../tracker.mjs";

const WORKFLOWS_DIR = fileURLToPath(
  new URL("../../../.github/workflows/", import.meta.url),
);

function readWorkflow(name) {
  return readFileSync(join(WORKFLOWS_DIR, name), "utf8").replace(/^﻿/, "");
}

const stripComment = (line) => line.replace(/(^|\s)#.*$/, "");
const unquote = (s) => s.trim().replace(/^['"]|['"]$/g, "");

function onBlock(yamlText) {
  const lines = yamlText.split("\n");
  const start = lines.findIndex((l) => /^\s*['"]?on['"]?\s*:/.test(l));
  if (start < 0) return null;
  const end = lines.findIndex(
    (l, i) =>
      i > start && /^['"]?[A-Za-z][\w-]*['"]?\s*:/.test(l) && !/^\s/.test(l),
  );
  return {
    first: stripComment(lines[start]),
    body: lines.slice(start + 1, end < 0 ? lines.length : end),
  };
}

function topLevelItems(inner) {
  const parts = [];
  let depth = 0;
  let seg = "";
  for (const ch of inner) {
    if ("[{(".includes(ch)) depth++;
    if ("}])".includes(ch)) depth--;
    if (ch === "," && depth === 0) {
      parts.push(seg);
      seg = "";
      continue;
    }
    seg += ch;
  }
  if (seg.trim()) parts.push(seg);
  return parts;
}

const KEY_RE = /^\s*([\w-]+|'[^']+'|"[^"]+")\s*:/;

function directTriggers(block) {
  if (!block) return null;
  const triggers = [];
  let inline = block.first.replace(/^\s*['"]?on['"]?\s*:\s*/, "");
  const opens = (s) => (s.match(/[[{]/g) ?? []).length;
  const closes = (s) => (s.match(/[\]}]/g) ?? []).length;
  if (opens(inline) > closes(inline)) {
    for (const l of block.body) {
      inline += " " + stripComment(l).trim();
      if (opens(inline) <= closes(inline)) break;
    }
  }
  if (inline && inline !== "|" && inline !== ">") {
    if (inline.startsWith("[") || inline.startsWith("{")) {
      const inner = inline.slice(1, inline.lastIndexOf(inline[0] === "[" ? "]" : "}"));
      for (const item of topLevelItems(inner)) {
        const k = item.match(KEY_RE);
        triggers.push(unquote(k ? k[1] : item));
      }
    } else {
      triggers.push(unquote(inline));
    }
  }
  const content = block.body
    .map(stripComment)
    .filter((l) => /\S/.test(l));
  const indents = content.map((l) => l.match(/^ */)[0].length);
  const min = indents.length ? Math.min(...indents) : Infinity;
  for (const l of content) {
    if (l.match(/^ */)[0].length !== min) continue;
    const seq = l.match(/^\s*-\s*(\S+)/);
    if (seq) {
      triggers.push(unquote(seq[1]));
      continue;
    }
    const key = l.match(KEY_RE);
    if (key) triggers.push(unquote(key[1]));
  }
  return triggers;
}

// Extract the `jobs:` section and return job names at the `  <name>:` level.
function jobNames(yamlText) {
  const lines = yamlText.split("\n");
  const jobsStart = lines.findIndex((l) => /^jobs:\s*$/.test(l));
  if (jobsStart < 0) return [];
  const names = [];
  for (let i = jobsStart + 1; i < lines.length; i++) {
    const l = lines[i];
    if (/^\S/.test(l) && l.trim() !== "") break; // next top-level key
    const m = l.match(/^ {2}([\w-]+):\s*$/);
    if (m) names.push(m[1]);
  }
  return names;
}

// Return the raw lines of one job's block (from `  <name>:` to the next
// job or top-level key) with comments stripped for assertion.
function jobBlock(yamlText, name) {
  const lines = yamlText.split("\n");
  const jobsStart = lines.findIndex((l) => /^jobs:\s*$/.test(l));
  const start = lines.findIndex(
    (l, i) => i > jobsStart && l === `  ${name}:`,
  );
  if (start < 0) return null;
  const end = lines.findIndex(
    (l, i) => i > start && (/^ {2}[\w-]+:\s*$/.test(l) || /^\S/.test(l)),
  );
  return lines.slice(start, end < 0 ? lines.length : end);
}

const EVENT_YML = readWorkflow("ci-issue-tracker.yml");
const RECONCILE_YML = readWorkflow("ci-issue-tracker-reconcile.yml");
const CONSUMER_YML = readWorkflow("consumer-verify.yml");

// ------------------------------------------------------- event entry ----

test("contract: ci-issue-tracker.yml watches ci AND consumer-verify on main", () => {
  const block = onBlock(EVENT_YML);
  const triggers = directTriggers(block);
  assert.deepEqual(triggers, ["workflow_run"]);
  const wr = block.body.join("\n");
  // workflows: [ci, consumer-verify] — both names present in the filter.
  const wfLine = block.body.find((l) => /workflows:/.test(l));
  assert.ok(wfLine, "workflow_run.workflows filter missing");
  for (const name of ["ci", "consumer-verify"]) {
    assert.ok(
      wfLine.includes(name),
      `workflow_run.workflows must include '${name}'`,
    );
  }
  assert.ok(/types:\s*\[?completed/.test(wr), "types must be [completed]");
  assert.ok(/branches:\s*\[?main/.test(wr), "branches must be [main]");
  // The event entry stays free of every other trigger class.
  for (const banned of [
    "pull_request",
    "pull_request_target",
    "push",
    "schedule",
    "workflow_dispatch",
  ]) {
    assert.ok(
      !triggers.includes(banned),
      `ci-issue-tracker.yml must not declare ${banned}`,
    );
  }
});

test("contract: one gated job per tracked workflow scope", () => {
  const names = jobNames(EVENT_YML);
  assert.ok(names.includes("track"), "track job missing");
  assert.ok(
    names.includes("track-consumer-verify"),
    "track-consumer-verify job missing",
  );

  const ciJob = jobBlock(EVENT_YML, "track").join("\n");
  const cvJob = jobBlock(EVENT_YML, "track-consumer-verify").join("\n");

  // Each job is pinned to exactly its own workflow_run name.
  assert.match(ciJob, /if:\s*github\.event\.workflow_run\.name == 'ci'/);
  assert.match(
    cvJob,
    /if:\s*github\.event\.workflow_run\.name == 'consumer-verify'/,
  );

  // The ci job keeps the default scope (no --workflow override)…
  assert.ok(
    !/--workflow(\s|=)/.test(ciJob),
    "ci job must not pass --workflow (default scope is ci)",
  );
  // …and the consumer job pins BOTH identity args explicitly.
  assert.match(cvJob, /--workflow consumer-verify/);
  assert.match(cvJob, /--workflow-file consumer-verify\.yml/);

  // Both jobs run the evaluator self-test before touching issues, and
  // upload attempt-unique reports under distinct artifact names.
  for (const [label, job] of [
    ["track", ciJob],
    ["track-consumer-verify", cvJob],
  ]) {
    assert.match(job, /mise run test-ci-tracker/, `${label} lost self-test`);
    assert.match(job, /TRACKER_EVENT_PATH/, `${label} lost event path env`);
    assert.match(job, /actions: read/, `${label} lost actions:read`);
    assert.match(job, /issues: write/, `${label} lost issues:write`);
    assert.match(job, /if-no-files-found: warn/, `${label} lost report upload`);
  }
  assert.match(ciJob, /ci-issue-tracker-report-attempt-\$\{\{ github\.run_attempt \}\}/);
  assert.match(
    cvJob,
    /ci-issue-tracker-report-consumer-verify-attempt-\$\{\{ github\.run_attempt \}\}/,
  );
});

// ------------------------------------------------------ reconcile entry --

test("contract: reconcile entry stays schedule+dispatch only and covers both scopes", () => {
  const triggers = directTriggers(onBlock(RECONCILE_YML));
  assert.deepEqual(
    new Set(triggers),
    new Set(["schedule", "workflow_dispatch"]),
    "reconcile triggers must stay exactly {schedule, workflow_dispatch}",
  );

  // Both scopes reconcile in the same job — sequential invocations of the
  // same evaluator, never a second framework.
  assert.match(
    RECONCILE_YML,
    /--reconcile --report ci-issue-tracker-report\.json/,
  );
  assert.match(RECONCILE_YML, /--workflow consumer-verify/);
  assert.match(RECONCILE_YML, /--workflow-file consumer-verify\.yml/);
  assert.match(
    RECONCILE_YML,
    /--report ci-issue-tracker-report-consumer-verify\.json/,
  );
  // Failure of one scope must not starve the other, but must still fail
  // the job.
  assert.match(RECONCILE_YML, /\|\| rc=1/);
  assert.match(RECONCILE_YML, /exit "\$rc"/);
  // Shared serialization group with the event entry is unchanged.
  assert.match(
    RECONCILE_YML,
    /group: ci-issue-tracker-\$\{\{ github\.repository \}\}/,
  );
});

// ------------------------------------------------- verdict → tracker input
//
// The tracker consumes the workflow_run CONCLUSION, not evidence statuses.
// The honest chain is: consumer evidence status → lane/job result →
// consumer-verify summary fail-closed → workflow conclusion → tracker
// kind. Pin each link so a future change cannot silently turn
// not_run/unsupported into pass or failure evidence.

test("contract: consumer evidence statuses never collapse into pass/fail", () => {
  const lane = (outcome) => ({
    name: "lane",
    command: "x",
    feature: null,
    os: null,
    outcome,
    reason: null,
  });
  const base = {
    gate: "run",
    gateReason: null,
    shaMatch: true,
    lanes: [],
  };

  // Sanctioned mappings that MUST stay: preflight-blocked and tuple
  // mismatch are failures; a real lane failure is a failure.
  assert.equal(
    deriveStatus({ ...base, gate: "blocked", gateReason: "preflight" }).status,
    "fail",
  );
  assert.equal(
    deriveStatus({ ...base, shaMatch: false }).status,
    "fail",
  );
  assert.equal(
    deriveStatus({ ...base, lanes: [lane("fail")] }).status,
    "fail",
  );

  // not_run / unsupported must never be read as pass OR fail — they are
  // their own honest statuses.
  assert.equal(
    deriveStatus({ ...base, lanes: [lane("not_run")] }).status,
    "not_run",
  );
  assert.equal(
    deriveStatus({ ...base, lanes: [lane("unsupported")] }).status,
    "unsupported",
  );
  assert.equal(
    deriveStatus({ ...base, lanes: [lane("not_run"), lane("unsupported")] }).status,
    "unsupported",
  );
  assert.equal(deriveStatus({ ...base }).status, "not_run");
  assert.equal(
    deriveStatus({ ...base, gate: "not_run" }).status,
    "not_run",
  );

  // And the reverse direction: nothing but a real pass may produce pass —
  // skipped lanes alone still leave the consumer unverified.
  assert.equal(
    deriveStatus({ ...base, lanes: [lane("skipped")] }).status,
    "not_run",
  );
  assert.equal(
    deriveStatus({ ...base, lanes: [lane("pass"), lane("unsupported")] }).status,
    "pass",
  );

  // The lane outcome vocabulary stays closed — exactly the five honest
  // outcomes, nothing that could smuggle a pass/fail alias.
  assert.deepEqual([...OUTCOMES].sort(), [
    "fail",
    "not_run",
    "pass",
    "skipped",
    "unsupported",
  ]);
});

test("contract: tracker conclusion matrix keeps operational ≠ failure", () => {
  // The tracker sees only the workflow conclusion. `cancelled`, `skipped`,
  // `neutral`, etc. are operational — never failure evidence, never a
  // pass that could close an issue.
  for (const c of FAILURE_CONCLUSIONS) {
    assert.equal(kindOfConclusion(c), "failure", `${c} must be failure`);
  }
  for (const c of ["cancelled", "skipped", "neutral", "stale", "action_required", null]) {
    assert.equal(kindOfConclusion(c), "operational", `${c} must be operational`);
    assert.notEqual(kindOfConclusion(c), "success");
    assert.notEqual(kindOfConclusion(c), "failure");
  }
  assert.equal(kindOfConclusion("success"), "success");
});

test("contract: consumer-verify summary still fail-closes the run", () => {
  // The workflow conclusion the tracker consumes is produced by the
  // summary job. Pin the enforcement so a weakened summary cannot turn a
  // consumer failure into a `success` conclusion the tracker would treat
  // as recovery.
  const summaryIdx = CONSUMER_YML.indexOf("summary:");
  assert.ok(summaryIdx > 0, "consumer-verify.yml lost the summary job");
  const summary = CONSUMER_YML.slice(summaryIdx);
  assert.match(summary, /needs:/);
  assert.match(summary, /if:\s*always\(\)/);
  // Failure/cancelled/preflight-fail and the all-not-run guard must still
  // be referenced in the summary's fail-closed logic.
  assert.match(summary, /failure|cancelled/);
  assert.match(summary, /not_run|unsupported/);
});

test("contract: no other workflow file gained a tracker trigger", () => {
  // The tracker entry must stay the ONLY workflow_run consumer of these
  // scopes — a second watcher would double-file issues.
  for (const file of readdirSync(WORKFLOWS_DIR).filter((f) => f.endsWith(".yml"))) {
    if (file === "ci-issue-tracker.yml") continue;
    const triggers = directTriggers(onBlock(readWorkflow(file))) ?? [];
    assert.ok(
      !triggers.includes("workflow_run"),
      `${file} unexpectedly declares workflow_run`,
    );
  }
});
