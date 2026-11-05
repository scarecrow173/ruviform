#!/usr/bin/env node
// summary.mjs — Issue #366 Full CI aggregated fail-closed summary gate.
//
// Successor of the retired ci/impact machinery (Issue #333/#346 selective
// execution): with every lane running unconditionally there is no expected
// set to compare against — the verdict is a pure function of the `needs`
// context. Every declared lane job plus every extra `needs` entry must
// report `result === "success"`; failure, cancelled, skipped, a missing
// lane, or an unknown result is a Fail — never a pass.
//
// SHA evidence (rev 2026-11-05): three distinct concepts are kept separate —
//   * workflow revision (`github.workflow_sha`/`workflow_ref`): which ci.yml
//     version executed — recorded as evidence only.
//   * event sha (`github.sha`): the commit the run was triggered for.
//   * actual checkout sha (`git rev-parse HEAD`): what each lane really
//     checked out — measured per lane leg and shipped as `checkout-sha-*`
//     artifacts, plus the summary job's own measurement.
// Every checkout step pins `ref: github.sha` so all of these agree by
// construction; the evaluator still fails closed when any measured leg is
// missing/invalid or disagrees, because evidence must prove the run
// verified a single revision rather than assume it.
// `expected_sha` is the Issue #366/#367 dispatch contract: when non-empty
// it must equal that measured common checkout sha — never `github.sha`
// alone — otherwise the run verified a different revision than the caller
// expected and the summary fails.
//
// Usage:
//   NEEDS_JSON='{...}' CHECKOUT_DIR=checkout-shas node ci/summary/summary.mjs
//       [--report <path>] [--summary <path>] [--head <sha>]
//       [--expected-sha <sha>] [--event <name>] [--run-url <url>]
// or
//   node ci/summary/summary.mjs --needs <needs.json> --checkout-dir <dir> ...

import { appendFileSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

// The declared lane set the summary must observe in `needs`. Kept in sync
// with `.github/workflows/ci.yml` needs list: a lane removed from `needs`
// (or renamed without updating this list) shows up as `missing` and fails
// closed. A job present in `needs` but absent from this list is still
// evaluated — extra-needs rows use the same success rule.
export const REQUIRED_JOBS = [
  "verify",
  "verify-macos",
  "rt-verify",
  "icc-verify",
  "compression-verify",
  "io-structured-verify",
  "ui-verify",
  "ui-verify-macos",
  "bench",
  "l1-real-verify",
  "l1-real-verify-macos",
];

// Declared checkout-sha evidence legs. One artifact per matrix leg, named
// `checkout-sha-<name>.txt` in ci.yml; kept in sync with the workflow so a
// lane that ran green but failed to ship its measured sha is an evidence
// gap that fails closed — never silently absent.
export const REQUIRED_CHECKOUT_LEGS = [
  "verify-ubuntu-latest",
  "verify-windows-latest",
  "verify-macos",
  "rt-verify",
  "icc-verify",
  "compression-verify",
  "io-structured-verify",
  "ui-verify-ubuntu-latest",
  "ui-verify-windows-latest",
  "ui-verify-macos",
  "bench",
  "l1-real-verify-ubuntu-latest",
  "l1-real-verify-windows-latest",
  "l1-real-verify-macos",
];

const HONORED_RESULTS = ["success", "failure", "cancelled", "skipped"];
const SHA_RE = /^[0-9a-f]{40}$/;

function rowFor(name, need, declared) {
  if (need == null) {
    return {
      name,
      declared,
      result: "missing",
      verdict: "Fail",
      note: "job-not-run-or-renamed",
    };
  }
  const result = need.result ?? "unknown";
  if (!HONORED_RESULTS.includes(result)) {
    return { name, declared, result, verdict: "Fail", note: `job-result-unknown` };
  }
  if (result === "success") {
    return { name, declared, result, verdict: "Pass", note: declared ? "" : "undeclared-needs-entry" };
  }
  return { name, declared, result, verdict: "Fail", note: `job-${result}` };
}

function shaRowFor(name, sha, declared) {
  if (sha == null || sha === "") {
    return { name, declared, sha: null, verdict: "Fail", note: "checkout-sha-missing" };
  }
  if (!SHA_RE.test(sha)) {
    return { name, declared, sha, verdict: "Fail", note: "checkout-sha-invalid" };
  }
  return { name, declared, sha, verdict: "Pass", note: declared ? "" : "undeclared-checkout-leg" };
}

export function evaluate({ needs, expectedSha = null, headSha = null, eventSha = null, checkoutShas = null }) {
  const reasons = [];
  const rows = [];

  for (const job of REQUIRED_JOBS) {
    const row = rowFor(job, needs?.[job], true);
    rows.push(row);
    if (row.verdict === "Fail") reasons.push(`${job}:${row.note}`);
  }
  for (const [name, need] of Object.entries(needs ?? {})) {
    if (REQUIRED_JOBS.includes(name)) continue;
    const row = rowFor(name, need, false);
    rows.push(row);
    if (row.verdict === "Fail") reasons.push(`${name}:${row.note}`);
  }

  // Checkout-sha evidence: every declared leg plus every extra file must be
  // a valid sha. The lane legs' values define `laneCommon` — the single
  // revision the run actually verified. The summary job's own measured
  // checkout (headSha) and the event sha (github.sha) are corroborating
  // anchors that must agree with it — they corroborate, they do not define
  // the verified revision, because `github.sha` is an event property, not
  // a measurement of what was on disk.
  const shaRows = [];
  const legShas = new Set();
  for (const leg of REQUIRED_CHECKOUT_LEGS) {
    const row = shaRowFor(leg, checkoutShas?.[leg], true);
    shaRows.push(row);
    if (row.verdict === "Fail") reasons.push(`${leg}:${row.note}`);
    else legShas.add(row.sha);
  }
  for (const [name, sha] of Object.entries(checkoutShas ?? {})) {
    if (REQUIRED_CHECKOUT_LEGS.includes(name)) continue;
    const row = shaRowFor(name, sha, false);
    shaRows.push(row);
    if (row.verdict === "Fail") reasons.push(`${name}:${row.note}`);
    else legShas.add(row.sha);
  }

  const laneDistinct = [...legShas];
  const laneCommon = laneDistinct.length === 1 ? laneDistinct[0] : null;
  if (laneDistinct.length > 1) reasons.push(`lane-checkout-mismatch:${laneDistinct.join(",")}`);

  const head = typeof headSha === "string" && SHA_RE.test(headSha) ? headSha : null;
  if (head == null) {
    reasons.push("ci-summary-checkout:checkout-sha-missing");
  } else if (laneCommon != null && head !== laneCommon) {
    reasons.push(`summary-checkout-mismatch:${head}!=${laneCommon}`);
  }
  const event = typeof eventSha === "string" && SHA_RE.test(eventSha) ? eventSha : null;
  if (event != null && laneCommon != null && event !== laneCommon) {
    reasons.push(`event-checkout-mismatch:${event}!=${laneCommon}`);
  }

  const expected = typeof expectedSha === "string" && expectedSha.trim() ? expectedSha.trim() : null;
  if (expected != null) {
    if (laneCommon == null) {
      reasons.push(`expected-sha-unverifiable:${expected}`);
    } else if (expected !== laneCommon) {
      reasons.push(`expected-sha-mismatch:${expected}!=${laneCommon}`);
    }
  }

  return {
    verdict: reasons.length > 0 ? "fail" : "pass",
    rows,
    shaRows,
    reasons,
    expectedSha: expected,
    headSha: head,
    eventSha: event,
    commonSha: laneCommon,
  };
}

function markdownTable(result, meta) {
  const lines = [
    `## ci-summary verdict: ${result.verdict.toUpperCase()}`,
    "",
    `- event: \`${meta.event ?? "?"}\`  checkout: \`${result.commonSha ?? "?"}\`  expected: \`${result.expectedSha ?? "-"}\``,
    `- event-sha: \`${result.eventSha ?? "-"}\`  summary-checkout: \`${result.headSha ?? "-"}\`  workflow: \`${meta.workflowRef ?? "?"}@${meta.workflowSha ?? "?"}\``,
    `- run: ${meta.runUrl ?? "?"}`,
    "",
    "| job | declared | result | verdict | note |",
    "| --- | --- | --- | --- | --- |",
    ...result.rows.map(
      (r) => `| ${r.name} | ${r.declared} | ${r.result} | ${r.verdict} | ${r.note || "-"} |`,
    ),
    "",
    "| checkout leg | declared | sha | verdict | note |",
    "| --- | --- | --- | --- | --- |",
    ...result.shaRows.map(
      (r) => `| ${r.name} | ${r.declared} | ${r.sha ?? "-"} | ${r.verdict} | ${r.note || "-"} |`,
    ),
  ];
  if (result.reasons.length > 0) {
    lines.push("", "### fail reasons", "", ...result.reasons.map((r) => `- ${r}`));
  }
  lines.push("");
  return lines.join("\n");
}

function parseJsonText(text, what) {
  try {
    return JSON.parse(text);
  } catch (e) {
    throw new Error(`invalid JSON for ${what}: ${e.message}`);
  }
}

// Reads `checkout-sha-<leg>.txt` files produced by the per-lane
// `git rev-parse HEAD` evidence steps (fetched via
// actions/download-artifact `checkout-sha-*` pattern + merge-multiple).
// A missing/unreadable dir yields an empty map — every declared leg then
// fails closed as evidence-not-obtained.
function readCheckoutShas(dir) {
  const shas = {};
  if (dir == null) return shas;
  let names;
  try {
    names = readdirSync(dir);
  } catch {
    return shas;
  }
  for (const name of names) {
    const m = /^checkout-sha-(.+)\.txt$/.exec(name);
    if (m == null) continue;
    try {
      shas[m[1]] = readFileSync(join(dir, name), "utf8").trim();
    } catch {
      shas[m[1]] = "";
    }
  }
  return shas;
}

export function main(argv = process.argv, env = process.env) {
  const args = {
    needs: null,
    report: null,
    summary: null,
    head: env.HEAD_SHA ?? null,
    expectedSha: env.EXPECTED_SHA ?? null,
    eventSha: env.EVENT_SHA ?? null,
    checkoutDir: env.CHECKOUT_DIR ?? null,
    event: env.GITHUB_EVENT_NAME ?? null,
    runUrl: env.RUN_URL ?? null,
    workflowRef: env.WORKFLOW_REF ?? null,
    workflowSha: env.WORKFLOW_SHA ?? null,
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--needs") args.needs = next();
    else if (a === "--report") args.report = next();
    else if (a === "--summary") args.summary = next();
    else if (a === "--head") args.head = next();
    else if (a === "--expected-sha") args.expectedSha = next();
    else if (a === "--event-sha") args.eventSha = next();
    else if (a === "--checkout-dir") args.checkoutDir = next();
    else if (a === "--event") args.event = next();
    else if (a === "--run-url") args.runUrl = next();
    else if (a === "--workflow-ref") args.workflowRef = next();
    else if (a === "--workflow-sha") args.workflowSha = next();
    else if (a === "--help" || a === "-h") {
      console.log("Usage: node ci/summary/summary.mjs [--needs <file>] [--checkout-dir <dir>] [--report <file>] [--summary <file>] [--head <sha>] [--expected-sha <sha>] [--event-sha <sha>] [--event <name>] [--run-url <url>] [--workflow-ref <ref>] [--workflow-sha <sha>]  (needs JSON also read from $NEEDS_JSON; checkout dir also from $CHECKOUT_DIR)");
      process.exit(0);
    } else {
      console.error(`error: unknown argument ${a}`);
      process.exit(2);
    }
  }

  const needsText = args.needs != null ? readFileSync(args.needs, "utf8") : env.NEEDS_JSON;
  if (needsText == null) {
    console.error("error: no needs JSON (pass --needs <file> or set NEEDS_JSON)");
    process.exit(2);
  }
  const needs = parseJsonText(needsText, "needs");
  const checkoutShas = readCheckoutShas(args.checkoutDir);

  const result = evaluate({
    needs,
    expectedSha: args.expectedSha,
    headSha: args.head,
    eventSha: args.eventSha,
    checkoutShas,
  });

  const report = {
    schemaVersion: 2,
    kind: "ci-summary",
    event: args.event,
    headSha: result.headSha,
    eventSha: result.eventSha,
    expectedSha: result.expectedSha,
    commonSha: result.commonSha,
    workflowRef: args.workflowRef,
    workflowSha: args.workflowSha,
    runUrl: args.runUrl,
    verdict: result.verdict,
    reasons: result.reasons,
    jobs: result.rows,
    checkoutShas: result.shaRows,
    generatedAt: new Date().toISOString(),
  };
  if (args.report) writeFileSync(args.report, JSON.stringify(report, null, 2) + "\n");

  const table = markdownTable(result, {
    event: args.event,
    runUrl: args.runUrl,
    workflowRef: args.workflowRef,
    workflowSha: args.workflowSha,
  });
  process.stdout.write(table + "\n");
  const summaryPath = args.summary ?? env.GITHUB_STEP_SUMMARY;
  if (summaryPath) appendFileSync(summaryPath, table + "\n");

  process.exit(result.verdict === "fail" ? 1 : 0);
}

const isMain =
  process.argv[1] != null &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main();
}
