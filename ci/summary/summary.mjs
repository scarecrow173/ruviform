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
// Usage:
//   NEEDS_JSON='{...}' node ci/summary/summary.mjs [--report <path>] [--summary <path>]
//       [--head <sha>] [--expected-sha <sha>] [--event <name>] [--run-url <url>]
// or
//   node ci/summary/summary.mjs --needs <needs.json> ...
//
// `expected_sha` is the Issue #366/#367 dispatch contract: when non-empty it
// must equal the checked-out HEAD sha, otherwise the run verified a
// different revision than the caller expected and the summary fails.

import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
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

const HONORED_RESULTS = ["success", "failure", "cancelled", "skipped"];

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

export function evaluate({ needs, expectedSha = null, headSha = null }) {
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

  const expected = typeof expectedSha === "string" && expectedSha.trim() ? expectedSha.trim() : null;
  if (expected != null) {
    if (headSha == null || headSha === "") {
      reasons.push(`head-sha-unknown:expected=${expected}`);
    } else if (expected !== headSha) {
      reasons.push(`expected-sha-mismatch:${expected}!=${headSha}`);
    }
  }

  return {
    verdict: reasons.length > 0 ? "fail" : "pass",
    rows,
    reasons,
    expectedSha: expected,
    headSha: headSha ?? null,
  };
}

function markdownTable(result, meta) {
  const lines = [
    `## ci-summary verdict: ${result.verdict.toUpperCase()}`,
    "",
    `- event: \`${meta.event ?? "?"}\`  head: \`${result.headSha ?? "?"}\`  expected: \`${result.expectedSha ?? "-"}\``,
    `- run: ${meta.runUrl ?? "?"}`,
    "",
    "| job | declared | result | verdict | note |",
    "| --- | --- | --- | --- | --- |",
    ...result.rows.map(
      (r) => `| ${r.name} | ${r.declared} | ${r.result} | ${r.verdict} | ${r.note || "-"} |`,
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

export function main(argv = process.argv, env = process.env) {
  const args = {
    needs: null,
    report: null,
    summary: null,
    head: env.HEAD_SHA ?? null,
    expectedSha: env.EXPECTED_SHA ?? null,
    event: env.GITHUB_EVENT_NAME ?? null,
    runUrl: env.RUN_URL ?? null,
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--needs") args.needs = next();
    else if (a === "--report") args.report = next();
    else if (a === "--summary") args.summary = next();
    else if (a === "--head") args.head = next();
    else if (a === "--expected-sha") args.expectedSha = next();
    else if (a === "--event") args.event = next();
    else if (a === "--run-url") args.runUrl = next();
    else if (a === "--help" || a === "-h") {
      console.log("Usage: node ci/summary/summary.mjs [--needs <file>] [--report <file>] [--summary <file>] [--head <sha>] [--expected-sha <sha>] [--event <name>] [--run-url <url>]  (needs JSON also read from $NEEDS_JSON)");
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

  const result = evaluate({ needs, expectedSha: args.expectedSha, headSha: args.head });

  const report = {
    schemaVersion: 1,
    kind: "ci-summary",
    event: args.event,
    headSha: result.headSha,
    expectedSha: result.expectedSha,
    runUrl: args.runUrl,
    verdict: result.verdict,
    reasons: result.reasons,
    jobs: result.rows,
    generatedAt: new Date().toISOString(),
  };
  if (args.report) writeFileSync(args.report, JSON.stringify(report, null, 2) + "\n");

  const table = markdownTable(result, { event: args.event, runUrl: args.runUrl });
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
