// evidence.test.mjs — Issue #294 cross-consumer tuple evidence contract
// tests. Every branch of the status-derivation matrix is pinned so a
// wrong-pass, wrong-skip, or tuple-mismatch laundering cannot ship silently.
// All tests are offline and deterministic (pure functions + tmp files).

import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  classifyRef,
  makeLaneRecord,
  deriveStatus,
  buildEvidence,
  markdownSummary,
  coverage,
  CONTRACT,
  SCHEMA_VERSION,
} from "../evidence.mjs";

const SCRIPT = fileURLToPath(new URL("../evidence.mjs", import.meta.url));
const SHA_A = "a".repeat(40);
const SHA_B = "b".repeat(40);
const SHA_C = "c".repeat(40);

function baseEnv(overrides = {}) {
  return {
    RUVIFORM_REV: SHA_A,
    CONSUMER_NAME: "synvelis",
    CONSUMER_REPO: "scarecrow173/synvelis",
    REQUESTED_REF: "main",
    RESOLVED_SHA: SHA_B,
    EXPECTED_SHA: "",
    GATE: "run",
    ...overrides,
  };
}

const lane = (outcome, extra = {}) =>
  makeLaneRecord({ name: `lane-${outcome}`, outcome, ...extra });

// ---------------------------------------------------------------------------
// classify-ref
// ---------------------------------------------------------------------------

test("classify-ref: 40-hex is sha", () => {
  assert.equal(classifyRef(SHA_A), "sha");
  assert.equal(classifyRef("0123456789abcdef0123456789abcdef01234567"), "sha");
});

test("classify-ref: branch/tag names are ref", () => {
  for (const r of ["main", "feature/x", "v1.2.3", "release/2026-11", "a-b_c.d"]) {
    assert.equal(classifyRef(r), "ref", r);
  }
});

test("classify-ref: rejects invalid refs", () => {
  for (const bad of [
    "",
    "..",
    "../x",
    "/abs",
    "trail/",
    "tag.lock",
    "white space",
    "-starts-with-dash",
    "semi;colon",
    "a b",
  ]) {
    assert.equal(classifyRef(bad), "invalid", bad);
  }
  // A 39-hex string is not a sha but is still a legal branch-name shape.
  assert.equal(classifyRef(SHA_A.slice(0, 39)), "ref");
  // A 40-hex string with an uppercase char is not a sha (lowercase only).
  assert.equal(classifyRef("A" + SHA_A.slice(1)), "ref");
});

// ---------------------------------------------------------------------------
// lane record validation
// ---------------------------------------------------------------------------

test("lane record: requires name and a known outcome", () => {
  assert.throws(() => makeLaneRecord({ name: "", outcome: "pass" }), /name/);
  assert.throws(() => makeLaneRecord({ name: "x", outcome: "green" }), /outcome/);
  const rec = makeLaneRecord({ name: "x", outcome: "unsupported", reason: "real-window host" });
  assert.equal(rec.outcome, "unsupported");
  assert.equal(rec.reason, "real-window host");
  assert.equal(rec.command, "-");
});

// ---------------------------------------------------------------------------
// status derivation matrix
// ---------------------------------------------------------------------------

test("status: gate=blocked -> fail (preflight_failed)", () => {
  const r = deriveStatus({ gate: "blocked", lanes: [], shaMatch: null });
  assert.equal(r.status, "fail");
  assert.equal(r.statusReason, "preflight_failed");
});

test("status: gate=not_run -> not_run even if lanes exist", () => {
  const r = deriveStatus({ gate: "not_run", gateReason: "no_access_path", lanes: [lane("pass")], shaMatch: null });
  assert.equal(r.status, "not_run");
  assert.equal(r.statusReason, "no_access_path");
});

test("status: shaMatch=false -> fail tuple_mismatch before lane evaluation", () => {
  const r = deriveStatus({ gate: "run", lanes: [lane("pass"), lane("pass")], shaMatch: false });
  assert.equal(r.status, "fail");
  assert.equal(r.statusReason, "tuple_mismatch");
});

test("status: any lane fail -> fail", () => {
  const r = deriveStatus({ gate: "run", lanes: [lane("pass"), lane("fail", { name: "test-ci" })], shaMatch: null });
  assert.equal(r.status, "fail");
  assert.match(r.statusReason, /test-ci/);
});

test("status: all pass -> pass", () => {
  const r = deriveStatus({ gate: "run", lanes: [lane("pass"), lane("pass")], shaMatch: true });
  assert.equal(r.status, "pass");
});

test("status: pass + unsupported/not_run/skipped lanes -> pass with coverage", () => {
  const lanes = [lane("pass"), lane("unsupported"), lane("not_run"), lane("skipped")];
  const r = deriveStatus({ gate: "run", lanes, shaMatch: null });
  assert.equal(r.status, "pass");
  const c = coverage(lanes);
  assert.deepEqual(c, { pass: 1, fail: 0, notRun: 1, unsupported: 1, skipped: 1 });
});

test("status: only unsupported lanes -> unsupported (neither pass nor fail)", () => {
  const r = deriveStatus({ gate: "run", lanes: [lane("unsupported"), lane("unsupported")], shaMatch: null });
  assert.equal(r.status, "unsupported");
});

test("status: empty / only not_run / only skipped -> not_run", () => {
  for (const lanes of [[], [lane("not_run")], [lane("skipped")], [lane("not_run"), lane("skipped")]]) {
    const r = deriveStatus({ gate: "run", lanes, shaMatch: null });
    assert.equal(r.status, "not_run", JSON.stringify(lanes));
  }
});

test("status: skipped can never launder to pass without a pass lane", () => {
  const r = deriveStatus({ gate: "run", lanes: [lane("skipped"), lane("unsupported")], shaMatch: null });
  assert.equal(r.status, "unsupported");
});

// ---------------------------------------------------------------------------
// buildEvidence: tuple + schema + fail-closed validation
// ---------------------------------------------------------------------------

test("evidence: requires 40-hex ruviform rev and consumer identity", () => {
  assert.throws(() => buildEvidence({ env: baseEnv({ RUVIFORM_REV: "abc" }), lanes: [], now: "t" }), /40-hex/);
  assert.throws(() => buildEvidence({ env: baseEnv({ CONSUMER_NAME: "" }), lanes: [], now: "t" }), /required/);
  assert.throws(() => buildEvidence({ env: baseEnv({ RESOLVED_SHA: "zzz" }), lanes: [], now: "t" }), /40-hex/);
  assert.throws(() => buildEvidence({ env: baseEnv({ EXPECTED_SHA: "123" }), lanes: [], now: "t" }), /40-hex/);
});

test("evidence: blocked/not_run paths tolerate an invalid candidate rev (evidence of what was attempted)", () => {
  const ev = buildEvidence({
    env: baseEnv({ GATE: "blocked", RUVIFORM_REV: "not-a-sha", RESOLVED_SHA: "" }),
    lanes: [],
    now: "t",
  });
  assert.equal(ev.candidate.ruviformRev, "not-a-sha");
  assert.equal(ev.status, "fail");
  const ev2 = buildEvidence({
    env: baseEnv({ GATE: "not_run", RUVIFORM_REV: "bogus", RESOLVED_SHA: "" }),
    lanes: [],
    now: "t",
  });
  assert.equal(ev2.status, "not_run");
});

test("evidence: shaMatch tri-state (null when no expected sha)", () => {
  const ev1 = buildEvidence({ env: baseEnv(), lanes: [lane("pass")], now: "2026-11-07T00:00:00Z" });
  assert.equal(ev1.consumer.shaMatch, null);
  const ev2 = buildEvidence({ env: baseEnv({ EXPECTED_SHA: SHA_B }), lanes: [lane("pass")], now: "t" });
  assert.equal(ev2.consumer.shaMatch, true);
  const ev3 = buildEvidence({ env: baseEnv({ EXPECTED_SHA: SHA_C }), lanes: [lane("pass")], now: "t" });
  assert.equal(ev3.consumer.shaMatch, false);
  assert.equal(ev3.status, "fail");
  assert.equal(ev3.statusReason, "tuple_mismatch");
});

test("evidence: resolved sha missing while expected provided -> fail-closed mismatch", () => {
  const ev = buildEvidence({ env: baseEnv({ EXPECTED_SHA: SHA_B, RESOLVED_SHA: "" }), lanes: [], now: "t" });
  assert.equal(ev.consumer.resolvedSha, null);
  assert.equal(ev.consumer.shaMatch, false);
  assert.equal(ev.status, "fail");
});

test("evidence: schema shape and contract identifier", () => {
  const ev = buildEvidence({
    env: baseEnv({
      GITHUB_RUN_ID: "123",
      GITHUB_RUN_ATTEMPT: "2",
      GITHUB_REPOSITORY: "scarecrow173/ruviform",
      GITHUB_SERVER_URL: "https://github.com",
      GITHUB_EVENT_NAME: "workflow_dispatch",
      ACCESS_MODE: "anonymous",
    }),
    lanes: [lane("pass"), lane("unsupported", { name: "webview", reason: "real-window" })],
    now: "2026-11-07T00:00:00Z",
  });
  assert.equal(ev.schemaVersion, SCHEMA_VERSION);
  assert.equal(ev.kind, "consumer-verify-evidence");
  assert.equal(ev.contract, CONTRACT);
  assert.equal(ev.tuple.ruviformRev, SHA_A);
  assert.equal(ev.tuple.consumerResolvedSha, SHA_B);
  assert.equal(ev.tuple.contract, CONTRACT);
  assert.equal(ev.run.runUrl, "https://github.com/scarecrow173/ruviform/actions/runs/123");
  assert.equal(ev.status, "pass");
  assert.deepEqual(ev.coverage, { pass: 1, fail: 0, notRun: 0, unsupported: 1, skipped: 0 });
  assert.equal(ev.generatedAt, "2026-11-07T00:00:00Z");
});

test("evidence: secret-shaped env values are not copied into output", () => {
  const env = baseEnv({
    CROSS_REPO_DEPLOY_KEY: "-----BEGIN OPENSSH " + "PRIVATE KEY-----xyz",
    GITHUB_TOKEN: "ghp_secret",
    ACCESS_MODE: "deploy_key",
  });
  const ev = buildEvidence({ env, lanes: [lane("pass")], now: "t" });
  const raw = JSON.stringify(ev);
  assert.ok(!raw.includes("PRIVATE KEY"));
  assert.ok(!raw.includes("ghp_secret"));
});

// ---------------------------------------------------------------------------
// markdown summary
// ---------------------------------------------------------------------------

test("summary: tuple table + lane table + mismatch marker", () => {
  const ev = buildEvidence({
    env: baseEnv({ EXPECTED_SHA: SHA_C }),
    lanes: [lane("fail", { name: "tuple-guard", reason: "tuple_mismatch: expected c..., resolved b..." })],
    now: "t",
  });
  const md = markdownSummary(ev);
  assert.match(md, /## Synvelis — ruviform a{40}/);
  assert.match(md, /\| expected sha \| c{40} \|/);
  assert.match(md, /\| sha match \| \*\*MISMATCH\*\* \|/);
  assert.match(md, /tuple-guard/);
  assert.match(md, /\| lane \| OS \| feature \| ruviform revision \| 結果 \|/);
});

test("summary: empty lanes emits status row, not an empty table", () => {
  const ev = buildEvidence({ env: baseEnv({ GATE: "not_run", GATE_REASON: "no access path", RESOLVED_SHA: "" }), lanes: [], now: "t" });
  const md = markdownSummary(ev);
  assert.equal(ev.status, "not_run");
  assert.match(md, /not_run \(no access path\)/);
});

// ---------------------------------------------------------------------------
// CLI end-to-end (lane append + finalize + status-out)
// ---------------------------------------------------------------------------

test("cli: lane appends JSONL and finalize emits JSON/summary/status", () => {
  const dir = mkdtempSync(join(tmpdir(), "cv-evidence-"));
  try {
    const lanesFile = join(dir, "lanes.jsonl");
    const out = join(dir, "evidence.json");
    const summary = join(dir, "summary.md");
    const statusOut = join(dir, "status.txt");
    execFileSync(process.execPath, [
      SCRIPT, "lane", "--file", lanesFile, "--name", "mise run check",
      "--command", "mise run check", "--os", "ubuntu-latest", "--outcome", "pass",
    ]);
    execFileSync(process.execPath, [
      SCRIPT, "lane", "--file", lanesFile, "--name", "webview lanes",
      "--outcome", "unsupported", "--reason", "real-window host",
    ]);
    execFileSync(process.execPath, [
      SCRIPT, "finalize", "--file", lanesFile, "--out", out,
      "--summary", summary, "--status-out", statusOut,
    ], {
      env: {
        ...process.env,
        ...baseEnv({ ACCESS_MODE: "anonymous" }),
      },
    });
    const ev = JSON.parse(readFileSync(out, "utf8"));
    assert.equal(ev.status, "pass");
    assert.equal(ev.lanes.length, 2);
    assert.equal(readFileSync(statusOut, "utf8").trim(), "status=pass");
    const md = readFileSync(summary, "utf8");
    assert.match(md, /mise run check/);
    assert.match(md, /unsupported \(real-window host\)/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("cli: classify-ref prints the class", () => {
  const out = execFileSync(process.execPath, [SCRIPT, "classify-ref", SHA_A]).toString().trim();
  assert.equal(out, "sha");
});

test("cli: finalize fails closed on bad env", () => {
  const dir = mkdtempSync(join(tmpdir(), "cv-evidence-"));
  try {
    const lanesFile = join(dir, "lanes.jsonl");
    assert.throws(() =>
      execFileSync(process.execPath, [
        SCRIPT, "finalize", "--file", lanesFile, "--out", join(dir, "e.json"),
      ], { env: { ...process.env, RUVIFORM_REV: "not-a-sha", CONSUMER_NAME: "x", CONSUMER_REPO: "y", REQUESTED_REF: "main", GATE: "run" } })
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("cli: finalize rejects forged lane rows (unknown outcome/schema)", () => {
  const dir = mkdtempSync(join(tmpdir(), "cv-evidence-"));
  try {
    const lanesFile = join(dir, "lanes.jsonl");
    writeFileSync(lanesFile, JSON.stringify({ name: "x", outcome: "green" }) + "\n");
    assert.throws(
      () =>
        execFileSync(process.execPath, [
          SCRIPT, "finalize", "--file", lanesFile, "--out", join(dir, "e.json"),
        ], { env: { ...process.env, ...baseEnv() } }),
      /schema/,
    );
    writeFileSync(lanesFile, '"just a string"\n');
    assert.throws(
      () =>
        execFileSync(process.execPath, [
          SCRIPT, "finalize", "--file", lanesFile, "--out", join(dir, "e.json"),
        ], { env: { ...process.env, ...baseEnv() } }),
      /schema/,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("evidence: blocked path tolerates non-hex EXPECTED_SHA (records raw input)", () => {
  const ev = buildEvidence({
    env: baseEnv({ GATE: "blocked", EXPECTED_SHA: "garbage", RESOLVED_SHA: "" }),
    lanes: [],
    now: "t",
  });
  assert.equal(ev.status, "fail");
  assert.equal(ev.consumer.expectedSha, "garbage");
  // run path still strict
  assert.throws(
    () => buildEvidence({ env: baseEnv({ EXPECTED_SHA: "garbage" }), lanes: [], now: "t" }),
    /40-hex/,
  );
});
