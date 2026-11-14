#!/usr/bin/env node
// evidence.mjs — Issue #294 cross-consumer candidate-tuple evidence tool.
//
// The consumer-verify workflow uses this script as the single authority
// for three jobs that must not drift between the audio-lens and synvelis
// lanes or between bash reimplementations:
//
//   1. classify-ref   — decide whether a consumer ref input is a 40-hex
//      commit SHA, a branch/tag name, or invalid (preflight validation and
//      the unified fetch+FETCH_HEAD checkout share this rule).
//   2. lane           — append one lane outcome record to a JSONL file.
//   3. finalize       — read the lane JSONL plus the tuple environment,
//      derive the consumer status fail-closed, and emit the structured
//      evidence JSON, the Job Summary markdown, and a `status=` line for
//      GITHUB_OUTPUT.
//
// Status derivation (fail-closed, priority order):
//   GATE=blocked                       -> fail     (preflight_failed)
//   GATE=not_run                       -> not_run  (gate reason, e.g. no access path)
//   expectedSha provided && resolved   -> fail     (tuple_mismatch — a run that
//     != expectedSha                             tested a different tuple must
//                                                not be recorded as the
//                                                expected tuple's evidence)
//   any lane outcome=fail              -> fail
//   >=1 lane pass, none fail           -> pass     (not_run/unsupported/skipped
//                                                lanes never count as pass but
//                                                are recorded in coverage)
//   lanes are only unsupported (>=1)   -> unsupported (capability-absent, e.g.
//                                        real-window host or macOS lanes that
//                                        cannot run on ubuntu-latest — honest
//                                        record, neither pass nor fail)
//   otherwise (empty / only not_run or -> not_run
//   skipped lanes)
//
// `unsupported` and `skipped` are produced only by statically-declared lane
// rows (capability-absent placeholders, or steps GitHub marked skipped
// because an upstream step in the same job failed). A lane that ran and
// failed is `fail` — failures are never reclassified as unsupported.
//
// Only an allowlist of environment variables is read (tuple, gate, run
// metadata). Secrets are never copied into evidence or output.
//
// Usage:
//   node ci/consumer-verify/evidence.mjs classify-ref <input>
//   node ci/consumer-verify/evidence.mjs lane --file lanes.jsonl \
//       --name "mise run check-ci" [--command "mise run check-ci"] \
//       [--feature workspace] [--os ubuntu-latest] \
//       --outcome pass|fail|not_run|unsupported|skipped [--reason ...]
//   node ci/consumer-verify/evidence.mjs finalize --file lanes.jsonl \
//       --out consumer-evidence-audiolens.json \
//       [--summary "$GITHUB_STEP_SUMMARY"] [--status-out "$GITHUB_OUTPUT"]
//
// Exit codes: 0 success, 1 error (bad env/IO), 2 usage.

import { appendFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const SHA_RE = /^[0-9a-f]{40}$/;
export const REF_RE = /^[A-Za-z0-9][A-Za-z0-9._/-]*$/;
export const REF_REJECT_RE = /(\.\.|^\/|\/$|\.lock$)/;
export const OUTCOMES = ["pass", "fail", "not_run", "unsupported", "skipped"];
export const CONTRACT = "consumer-verify-tuple/v1";
export const SCHEMA_VERSION = 2;
// Case schema v2 (Issue #294 first slice / #627 contract §2): the result
// axis stays `outcome` (requirement is orthogonal — never folded into one
// field). `skipped` remains a lane-vocabulary-only derived outcome.
export const LEVELS = ["OS-L1", "GUI-L2", "EXT-L3"];
export const ENVIRONMENT_KINDS = [
  "deterministic-backend",
  "virtual-os-endpoint",
  "test-plugin",
  "real-window",
  "physical-device",
];
export const REQUIREMENTS = ["Required", "Deferred", "NotApplicable"];
// Optional case-record fields (camelCase option keys; serialized snake_case):
// string-or-null; resolved_ruviform_sha must be a real 40-hex SHA when
// present (resolved value, not a declared one).
export const OPTIONAL_LANE_FIELDS = [
  "oracle",
  "fixtureVersion",
  "resolvedRuviformSha",
  "expected",
  "observed",
  "device",
  "endpoint",
  "plugin",
  "abi",
  "sdk",
  "arch",
  "osVer",
];

// ---------------------------------------------------------------------------
// classify-ref (pure)
// ---------------------------------------------------------------------------

export function classifyRef(input) {
  if (typeof input !== "string" || input.length === 0) return "invalid";
  if (SHA_RE.test(input)) return "sha";
  if (REF_RE.test(input) && !REF_REJECT_RE.test(input)) return "ref";
  return "invalid";
}

// ---------------------------------------------------------------------------
// lane record (pure validation + JSONL append in cmd)
// ---------------------------------------------------------------------------

function requireEnum(value, allowed, flag) {
  if (!allowed.includes(value)) {
    throw new Error(`${flag} must be one of ${allowed.join("/")} (got: ${value})`);
  }
}

function optionalString(value, key) {
  if (value == null) return null;
  if (typeof value !== "string") {
    throw new Error(`lane ${key} must be a string or null (got: ${typeof value})`);
  }
  return value;
}

export function makeLaneRecord({
  name,
  command,
  feature,
  os,
  outcome,
  reason,
  caseId,
  level,
  environmentKind,
  requirement,
  ...optional
}) {
  if (typeof name !== "string" || name.length === 0) {
    throw new Error("lane --name is required");
  }
  if (!OUTCOMES.includes(outcome)) {
    throw new Error(`lane --outcome must be one of ${OUTCOMES.join("/")} (got: ${outcome})`);
  }
  if (typeof caseId !== "string" || caseId.length === 0) {
    throw new Error("lane --case-id is required");
  }
  requireEnum(level, LEVELS, "lane --level");
  requireEnum(environmentKind, ENVIRONMENT_KINDS, "lane --environment-kind");
  requireEnum(requirement, REQUIREMENTS, "lane --requirement");
  const rec = {
    name,
    command: command ?? "-",
    feature: feature ?? "-",
    os: os ?? "-",
    outcome,
    reason: reason ?? null,
    case_id: caseId,
    level,
    environment_kind: environmentKind,
    requirement,
  };
  for (const key of OPTIONAL_LANE_FIELDS) {
    const snake = key.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
    let value = optionalString(optional[key], `--${snake.replace(/_/g, "-")}`);
    if (value === "") value = null;
    if (snake === "resolved_ruviform_sha" && value !== null && !SHA_RE.test(value)) {
      throw new Error(`lane --resolved-ruviform-sha must be a 40-hex SHA or empty (got: ${value})`);
    }
    rec[snake] = value;
  }
  for (const key of Object.keys(optional)) {
    if (!OPTIONAL_LANE_FIELDS.includes(key)) {
      throw new Error(`lane: unknown option --${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`);
    }
  }
  return rec;
}

// ---------------------------------------------------------------------------
// status derivation (pure)
// ---------------------------------------------------------------------------

export function deriveStatus({ gate, gateReason, lanes, shaMatch }) {
  if (gate === "blocked") {
    return { status: "fail", statusReason: gateReason ?? "preflight_failed" };
  }
  if (gate === "not_run") {
    return { status: "not_run", statusReason: gateReason ?? "not_run" };
  }
  if (shaMatch === false) {
    return { status: "fail", statusReason: "tuple_mismatch" };
  }
  const has = (o) => lanes.some((l) => l.outcome === o);
  if (has("fail")) {
    const first = lanes.find((l) => l.outcome === "fail");
    return {
      status: "fail",
      statusReason: first.reason ? `lane_fail: ${first.name} — ${first.reason}` : `lane_fail: ${first.name}`,
    };
  }
  if (has("pass")) return { status: "pass", statusReason: null };
  if (has("unsupported")) return { status: "unsupported", statusReason: "all lanes capability-unsupported" };
  return { status: "not_run", statusReason: "no lane produced evidence" };
}

export function coverage(lanes) {
  const c = { pass: 0, fail: 0, notRun: 0, unsupported: 0, skipped: 0 };
  for (const l of lanes) {
    if (l.outcome === "not_run") c.notRun += 1;
    else if (l.outcome in c) c[l.outcome] += 1;
  }
  return c;
}

// ---------------------------------------------------------------------------
// evidence assembly (pure)
// ---------------------------------------------------------------------------

export function buildEvidence({ env, lanes, now }) {
  const gate = env.GATE ?? "run";
  if (!["run", "not_run", "blocked"].includes(gate)) {
    throw new Error(`GATE must be run|not_run|blocked (got: ${gate})`);
  }
  const ruviformRev = env.RUVIFORM_REV ?? "";
  // On the run path the candidate identity must be a real SHA. On blocked /
  // not_run terminal paths (e.g. preflight rejected the input itself) the raw
  // value is still recorded — evidence of what was attempted, never dropped.
  if (gate === "run" && !SHA_RE.test(ruviformRev)) {
    throw new Error(`RUVIFORM_REV must be a 40-hex SHA (got: ${ruviformRev})`);
  }
  const name = env.CONSUMER_NAME ?? "";
  const repo = env.CONSUMER_REPO ?? "";
  const requestedRef = env.REQUESTED_REF ?? "";
  if (!name || !repo || !requestedRef) {
    throw new Error("CONSUMER_NAME, CONSUMER_REPO and REQUESTED_REF are required");
  }
  const resolvedSha = (env.RESOLVED_SHA ?? "") || null;
  if (gate === "run" && resolvedSha !== null && !SHA_RE.test(resolvedSha)) {
    throw new Error(`RESOLVED_SHA must be a 40-hex SHA or empty (got: ${env.RESOLVED_SHA})`);
  }
  const expectedSha = (env.EXPECTED_SHA ?? "") || null;
  if (gate === "run" && expectedSha !== null && !SHA_RE.test(expectedSha)) {
    throw new Error(`EXPECTED_SHA must be a 40-hex SHA or empty (got: ${env.EXPECTED_SHA})`);
  }
  // shaMatch: null when no expectation was supplied — "no check" is not a pass.
  const shaMatch = expectedSha === null ? null : resolvedSha === expectedSha;
  const { status, statusReason } = deriveStatus({ gate, gateReason: env.GATE_REASON ?? null, shaMatch, lanes });
  const server = env.GITHUB_SERVER_URL ?? "https://github.com";
  const repository = env.GITHUB_REPOSITORY ?? null;
  const runId = env.GITHUB_RUN_ID ?? null;
  const runUrl =
    server && repository && runId ? `${server}/${repository}/actions/runs/${runId}` : null;
  return {
    schemaVersion: SCHEMA_VERSION,
    kind: "consumer-verify-evidence",
    contract: CONTRACT,
    run: {
      workflow: env.GITHUB_WORKFLOW ?? "consumer-verify",
      runId,
      runAttempt: env.GITHUB_RUN_ATTEMPT ?? null,
      runUrl,
      event: env.GITHUB_EVENT_NAME ?? null,
      repository,
    },
    candidate: { ruviformRev },
    consumer: {
      name,
      repo,
      requestedRef,
      resolvedSha,
      expectedSha,
      shaMatch,
    },
    accessMode: env.ACCESS_MODE ?? "none",
    tuple: {
      ruviformRev,
      consumerResolvedSha: resolvedSha,
      contract: CONTRACT,
    },
    lanes,
    status,
    statusReason,
    coverage: coverage(lanes),
    generatedAt: now,
  };
}

// ---------------------------------------------------------------------------
// markdown summary (pure)
// ---------------------------------------------------------------------------

const esc = (v) => (v == null ? "-" : String(v));

export function markdownSummary(evidence) {
  const c = evidence.consumer;
  const match =
    c.shaMatch === null ? "(no expected sha)" : c.shaMatch ? "match" : "**MISMATCH**";
  const title = c.name === "audiolens" ? "AudioLens" : c.name === "synvelis" ? "Synvelis" : c.name;
  const lines = [
    `## ${title} — ruviform ${evidence.candidate.ruviformRev} (access: ${evidence.accessMode})`,
    "",
    "| tuple element | value |",
    "|---|---|",
    `| requested ref | ${c.requestedRef} |`,
    `| expected sha | ${esc(c.expectedSha)} |`,
    `| resolved sha | ${esc(c.resolvedSha)} |`,
    `| sha match | ${match} |`,
    `| status | ${evidence.status}${evidence.statusReason ? ` (${evidence.statusReason})` : ""} |`,
    "",
    "| lane | OS | feature | ruviform revision | 結果 | 許容例外 | Rollback |",
    "|---|---|---|---|---|---|---|",
  ];
  for (const l of evidence.lanes) {
    const res = l.reason ? `${l.outcome} (${l.reason})` : l.outcome;
    lines.push(`| ${l.name} | ${l.os} | ${l.feature} | ${evidence.candidate.ruviformRev} | ${res} | - | pin を旧 SHA へ revert |`);
  }
  if (evidence.lanes.length === 0) {
    lines.push(`| all | - | - | ${evidence.candidate.ruviformRev} | ${evidence.status}${evidence.statusReason ? ` (${evidence.statusReason})` : ""} | - | - |`);
  }
  return lines.join("\n") + "\n";
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next === undefined || next.startsWith("--")) {
        args[key] = true;
      } else {
        args[key] = next;
        i += 1;
      }
    } else {
      args._.push(a);
    }
  }
  return args;
}

// Fail-closed lane-record schema check shared by readLanes: rows written
// outside the `lane` command must satisfy the same vocabulary — a
// forged/unknown value must not count toward status.
function assertLaneRecord(rec, ctx) {
  const bad = (what) => {
    throw new Error(`lanes file ${ctx} fails lane record schema (${what})`);
  };
  if (rec === null || typeof rec !== "object" || Array.isArray(rec)) bad("not an object");
  if (typeof rec.name !== "string" || rec.name.length === 0) bad("name");
  if (!OUTCOMES.includes(rec.outcome)) bad("outcome");
  if (typeof rec.case_id !== "string" || rec.case_id.length === 0) bad("case_id");
  if (!LEVELS.includes(rec.level)) bad("level");
  if (!ENVIRONMENT_KINDS.includes(rec.environment_kind)) bad("environment_kind");
  if (!REQUIREMENTS.includes(rec.requirement)) bad("requirement");
  for (const snake of OPTIONAL_LANE_FIELDS.map((k) =>
    k.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`),
  )) {
    const v = rec[snake];
    if (v !== undefined && v !== null && typeof v !== "string") bad(snake);
    if (snake === "resolved_ruviform_sha" && typeof v === "string" && !SHA_RE.test(v)) {
      bad("resolved_ruviform_sha");
    }
  }
}

function readLanes(file) {
  if (!existsSync(file)) return [];
  return readFileSync(file, "utf8")
    .split("\n")
    .filter((l) => l.trim().length > 0)
    .map((l, i) => {
      let rec;
      try {
        rec = JSON.parse(l);
      } catch {
        throw new Error(`lanes file ${file} line ${i + 1} is not valid JSON`);
      }
      try {
        assertLaneRecord(rec, `${file} line ${i + 1}`);
      } catch (e) {
        throw e;
      }
      return rec;
    });
}

async function main() {
  const [cmd, ...rest] = process.argv.slice(2);
  const args = parseArgs(rest);

  if (cmd === "classify-ref") {
    const input = args._[0];
    if (input === undefined) {
      console.error("usage: evidence.mjs classify-ref <input>");
      process.exit(2);
    }
    process.stdout.write(classifyRef(input) + "\n");
    process.exit(0);
  }

  if (cmd === "lane") {
    try {
      const rec = makeLaneRecord({
        name: args.name,
        command: args.command,
        feature: args.feature,
        os: args.os,
        outcome: args.outcome,
        reason: args.reason,
        caseId: args["case-id"],
        level: args.level,
        environmentKind: args["environment-kind"],
        requirement: args.requirement,
        oracle: args.oracle,
        fixtureVersion: args["fixture-version"],
        resolvedRuviformSha: args["resolved-ruviform-sha"],
        expected: args.expected,
        observed: args.observed,
        device: args.device,
        endpoint: args.endpoint,
        plugin: args.plugin,
        abi: args.abi,
        sdk: args.sdk,
        arch: args.arch,
        osVer: args["os-ver"],
      });
      if (!args.file) {
        console.error("usage: lane --file <jsonl> --name <n> --outcome <o>");
        process.exit(2);
      }
      appendFileSync(args.file, JSON.stringify(rec) + "\n");
      process.exit(0);
    } catch (e) {
      console.error(`error: ${e.message}`);
      process.exit(1);
    }
    return;
  }

  if (cmd === "finalize") {
    if (!args.file || !args.out) {
      console.error("usage: finalize --file <jsonl> --out <json> [--summary md] [--status-out f]");
      process.exit(2);
    }
    try {
      const lanes = readLanes(args.file);
      const evidence = buildEvidence({ env: process.env, lanes, now: new Date().toISOString() });
      writeFileSync(args.out, JSON.stringify(evidence, null, 2) + "\n");
      if (args.summary) appendFileSync(args.summary, markdownSummary(evidence));
      if (args["status-out"]) {
        appendFileSync(args["status-out"], `status=${evidence.status}\n`);
      }
      process.stdout.write(`status=${evidence.status}\n`);
      process.exit(0);
    } catch (e) {
      console.error(`error: ${e.message}`);
      process.exit(1);
    }
    return;
  }

  console.error("usage: evidence.mjs classify-ref|lane|finalize ...");
  process.exit(2);
}

const isMain =
  process.argv[1] != null &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (isMain) {
  await main();
}
