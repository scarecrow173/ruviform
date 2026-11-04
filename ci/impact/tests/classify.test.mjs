// classify.test.mjs — Issue #333 (CI-OPT-06) classifier contract tests.
// Fixture expectations are pinned to the real workspace DAG (they are
// integration tests against the live repo by design — a workspace change
// that legitimately alters an expectation requires updating the fixture).

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { LANE_IDS } from "../map.mjs";
import { buildContext, classifyPath, resolveLanes } from "../lib.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const ctx = buildContext(ROOT);

function lanesFor(paths) {
  const classified = paths.map((p) => ({ path: p, ...classifyPath(p, ctx) }));
  return resolveLanes(classified, ctx);
}

function expectedSet(res) {
  return new Set(LANE_IDS.filter((id) => res.lanes[id].expected));
}

test("context: cargo metadata graph loads with members", () => {
  assert.equal(ctx.crateGraph.ok, true, ctx.problems.join(","));
  assert.ok(ctx.crateGraph.members.size >= 30);
  assert.ok(ctx.uiGraph.packages.has("core"));
});

test("context: lane owner derivation is non-empty for every lane", () => {
  for (const [lane, info] of ctx.laneOwners) {
    assert.notEqual(info.owners, null, `${lane} owner derivation failed (${info.source})`);
    assert.ok(info.owners.size > 0, `${lane} has no owners`);
  }
});

test("math crate change -> verify, rt-verify, bench, l1-real-verify", () => {
  const res = lanesFor(["crates/ruviform-math/src/lib.rs"]);
  assert.equal(res.decision, "selective");
  assert.deepEqual(
    expectedSet(res),
    new Set(["verify", "rt-verify", "bench", "l1-real-verify"]),
  );
  // math <- dsp <- audio-analysis <- audio <- plugin-host <- clap/vst3
  assert.ok(res.impacted.has("ruviform-plugin-clap"), "l1 lane owners must be reached transitively");
});

test("plugin-core change -> verify + rt-verify + l1-real-verify", () => {
  const res = lanesFor(["crates/ruviform-plugin-core/src/lib.rs"]);
  assert.deepEqual(
    expectedSet(res),
    new Set(["verify", "rt-verify", "l1-real-verify"]),
  );
});

test("video-proc change -> verify + rt-verify + icc-verify", () => {
  const res = lanesFor(["crates/ruviform-video-proc/src/lib.rs"]);
  assert.deepEqual(
    expectedSet(res),
    new Set(["verify", "rt-verify", "icc-verify"]),
  );
});

test("ruviform-ui crate change -> verify + ui-verify", () => {
  const res = lanesFor(["crates/ruviform-ui/src/lib.rs"]);
  assert.deepEqual(expectedSet(res), new Set(["verify", "ui-verify"]));
});

test("ui/core package change -> ui-verify only (no verify)", () => {
  const res = lanesFor(["ui/core/src/index.ts"]);
  assert.deepEqual(expectedSet(res), new Set(["ui-verify"]));
});

test("ui/browser-test package.json change -> ui-verify only", () => {
  const res = lanesFor(["ui/browser-test/package.json"]);
  assert.deepEqual(expectedSet(res), new Set(["ui-verify"]));
});

test("docs-only change -> verify only", () => {
  for (const p of ["docs/specs/x.md", "AGENTS.md", "crates/README.md"]) {
    const res = lanesFor([p]);
    assert.deepEqual(expectedSet(res), new Set(["verify"]), p);
  }
});

test("poc change -> verify only", () => {
  const res = lanesFor(["poc/ui/package.json"]);
  assert.deepEqual(expectedSet(res), new Set(["verify"]));
});

test("shared/CI/repo config and unknown paths -> Full CI", () => {
  const fullCases = [
    "Cargo.toml",
    "Cargo.lock",
    "mise.toml",
    "mise.lock",
    ".github/workflows/ci.yml",
    ".pre-commit-config.yaml",
    "ci/impact/map.mjs",
    ".gitignore",
    "apm.yml",
    "vendor/newdir/file.txt",
    "ui/README.md",
    "crates/not-a-member/src/lib.rs",
  ];
  for (const p of fullCases) {
    const res = lanesFor([p]);
    assert.equal(res.decision, "full", p);
    assert.equal(expectedSet(res).size, LANE_IDS.length, p);
    assert.ok(res.fullReasons.length > 0, `full reason recorded for ${p}`);
  }
});

test("mixed docs + Cargo.lock -> Full CI", () => {
  const res = lanesFor(["docs/specs/x.md", "Cargo.lock"]);
  assert.equal(res.decision, "full");
});

test("non-workspace crate dir is unknown -> full", () => {
  const r = classifyPath("crates/something-new/build.rs", ctx);
  assert.ok(r.classes.includes("unknown"));
});

test("lane output carries oses/features/reasons", () => {
  const res = lanesFor(["crates/ruviform-math/src/lib.rs"]);
  for (const id of LANE_IDS) {
    assert.ok(Array.isArray(res.lanes[id].oses));
    assert.ok(Array.isArray(res.lanes[id].features));
    assert.ok(Array.isArray(res.lanes[id].reasons));
  }
  assert.deepEqual(res.lanes["ui-verify"].oses, [
    "ubuntu-latest",
    "windows-latest",
    "macos-latest",
  ]);
  assert.ok(res.lanes["bench"].reasons.length > 0);
});

test("CLI: --files produces decision JSON + GITHUB_OUTPUT", () => {
  const dir = mkdtempSync(join(tmpdir(), "impact-"));
  const filesPath = join(dir, "files.txt");
  const outPath = join(dir, "decision.json");
  const ghOut = join(dir, "ghout.txt");
  writeFileSync(filesPath, "crates/ruviform-video-proc/src/lib.rs\n");
  const stdout = execFileSync(
    process.execPath,
    [join(ROOT, "ci/impact/classify.mjs"), "--files", filesPath, "--out", outPath, "--repo-root", ROOT],
    { env: { ...process.env, GITHUB_OUTPUT: ghOut }, encoding: "utf8" },
  );
  const decision = JSON.parse(stdout);
  assert.equal(decision.decision, "selective");
  assert.equal(decision.lanes["icc-verify"].expected, true);
  const gh = readFileSync(ghOut, "utf8");
  assert.match(gh, /decision=selective/);
  assert.match(gh, /lane_icc_verify=true/);
});

test("CLI: --all forces full decision", () => {
  const stdout = execFileSync(
    process.execPath,
    [join(ROOT, "ci/impact/classify.mjs"), "--all", "--repo-root", ROOT],
    { encoding: "utf8" },
  );
  const decision = JSON.parse(stdout);
  assert.equal(decision.decision, "full");
  assert.equal(decision.files.length, 0);
  for (const id of LANE_IDS) assert.equal(decision.lanes[id].expected, true);
});
