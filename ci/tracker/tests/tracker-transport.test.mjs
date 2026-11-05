import { test } from "node:test";
import assert from "node:assert/strict";
import { HISTORY_PAGE_SIZE, makeGhClient } from "../tracker.mjs";

const REPO = "scarecrow173/ruviform";
const MIB = 1024 * 1024;
const HISTORY_BASE = `repos/${REPO}/actions/workflows/ci.yml/runs?branch=main&exclude_pull_requests=true&per_page=${HISTORY_PAGE_SIZE}`;
const historyEndpoint = (page) => `${HISTORY_BASE}&page=${page}`;

function histRecord(id, pad = "", overrides = {}) {
  return {
    id,
    head_sha: "0123456789abcdef0123456789abcdef01234567",
    head_branch: "main",
    event: "push",
    conclusion: "failure",
    html_url: `https://github.com/scarecrow173/ruviform/actions/runs/${id}`,
    run_attempt: 1,
    updated_at: "2026-11-06T00:00:00Z",
    run_started_at: "2026-11-06T00:00:00Z",
    status: "completed",
    path: ".github/workflows/ci.yml",
    name: "ci",
    repository: { full_name: REPO, description: pad },
    head_repository: { full_name: REPO, fork: false },
    ...overrides,
  };
}

function histPage(ids, pad = "", overrides = {}) {
  return { body: { workflow_runs: ids.map((id) => histRecord(id, pad, overrides[id] ?? {})) } };
}

function enobufs() {
  const err = new Error("spawnSync gh ENOBUFS");
  err.code = "ENOBUFS";
  return err;
}

function fakeExecFile(pages) {
  const calls = [];
  let expectedPage = 0;
  const checkBuffer = (out, options) => {
    if (Buffer.byteLength(out) > (options.maxBuffer ?? MIB)) throw enobufs();
    return out;
  };
  const pageBody = (n) => {
    const r = pages[n - 1];
    assert.ok(r !== undefined, `unexpected page request: ${n}`);
    if (r.error) throw r.error;
    return r.raw !== undefined ? r.raw : JSON.stringify(r.body);
  };
  const execFile = (file, args, options = {}) => {
    assert.equal(file, "gh");
    assert.equal(args[0], "api");
    calls.push(args);
    if (args.includes("--paginate")) {
      assert.deepEqual(args.slice(2), ["--paginate", "--jq", ".workflow_runs[]"]);
      assert.equal(args[1], HISTORY_BASE);
      const lines = [];
      for (let n = 1; n <= pages.length; n++) {
        const envelope = JSON.parse(pageBody(n));
        const runs = envelope?.workflow_runs;
        if (!Array.isArray(runs)) {
          throw new Error(`jq error: Cannot iterate over ${runs === null ? "null" : typeof runs}`);
        }
        for (const rec of runs) lines.push(JSON.stringify(rec));
      }
      return checkBuffer(lines.join("\n"), options);
    }
    assert.deepEqual(args.slice(2), [], `unexpected extra gh api args: ${args.slice(2)}`);
    const m = /[?&]page=(\d+)$/.exec(args[1] ?? "");
    assert.ok(m, `expected &page= query in endpoint: ${args[1]}`);
    expectedPage += 1;
    assert.equal(Number(m[1]), expectedPage, "pages must be requested sequentially from 1");
    assert.equal(args[1], historyEndpoint(expectedPage));
    return checkBuffer(pageBody(expectedPage), options);
  };
  return { execFile, calls };
}

const ghClient = (pages) => {
  const { execFile, calls } = fakeExecFile(pages);
  return { client: makeGhClient({ repo: REPO, execFile }), calls };
};

test("transport: >1 MiB history is read page-by-page, all 82 raw records in order", async () => {
  const pad = "x".repeat(14000);
  const pages = [
    histPage(Array.from({ length: 30 }, (_, i) => 100 + i), pad),
    histPage(Array.from({ length: 30 }, (_, i) => 200 + i), pad, {
      205: { run_attempt: 3, status: "in_progress", conclusion: null, updated_at: "2026-11-06T00:30:00Z" },
    }),
    histPage(Array.from({ length: 22 }, (_, i) => 300 + i), pad, {
      300: { head_repository: { full_name: REPO, fork: false } },
    }),
  ];
  const legacyJsonl = pages.flatMap((p) => p.body.workflow_runs).map((r) => JSON.stringify(r)).join("\n");
  assert.ok(Buffer.byteLength(legacyJsonl) > MIB, "fixture must exceed the 1 MiB execFileSync default maxBuffer");
  for (const p of pages) {
    assert.ok(Buffer.byteLength(JSON.stringify(p.body)) <= MIB, "each page must fit within maxBuffer");
  }
  const { client, calls } = ghClient(pages);
  const runs = await client.getRunHistory({ workflowFile: "ci.yml", branch: "main" });
  assert.equal(runs.length, 82);
  assert.deepEqual(calls, [
    ["api", historyEndpoint(1)],
    ["api", historyEndpoint(2)],
    ["api", historyEndpoint(3)],
  ]);
  const flat = pages.flatMap((p) => p.body.workflow_runs);
  assert.deepEqual(runs, flat);
  const rerun = runs[flat.findIndex((r) => r.id === 205)];
  assert.equal(rerun.run_attempt, 3);
  assert.equal(rerun.status, "in_progress");
  assert.equal(rerun.conclusion, null);
  assert.equal(rerun.updated_at, "2026-11-06T00:30:00Z");
  assert.equal(runs[0].repository.full_name, REPO);
  assert.equal(runs[0].head_repository.fork, false);
  assert.equal(runs[0].path, ".github/workflows/ci.yml");
  assert.equal(runs[0].name, "ci");
});

test("transport: a legacy --paginate read of the same fixture dies with ENOBUFS", () => {
  const pad = "x".repeat(14000);
  const pages = [histPage(Array.from({ length: 30 }, (_, i) => 100 + i), pad), histPage(Array.from({ length: 30 }, (_, i) => 200 + i), pad), histPage(Array.from({ length: 22 }, (_, i) => 300 + i), pad)];
  const { execFile } = fakeExecFile(pages);
  assert.throws(
    () => execFile("gh", ["api", HISTORY_BASE, "--paginate", "--jq", ".workflow_runs[]"], { encoding: "utf8" }),
    (e) => e.code === "ENOBUFS",
  );
});

test("transport: a full final page still triggers one more page read before terminating", async () => {
  const pages = [
    histPage(Array.from({ length: 30 }, (_, i) => 100 + i)),
    { body: { workflow_runs: [] } },
  ];
  const { client, calls } = ghClient(pages);
  const runs = await client.getRunHistory({ workflowFile: "ci.yml", branch: "main" });
  assert.equal(runs.length, 30);
  assert.deepEqual(calls, [
    ["api", historyEndpoint(1)],
    ["api", historyEndpoint(2)],
  ]);
});

test("transport: an empty first page returns an empty history", async () => {
  const { client, calls } = ghClient([{ body: { workflow_runs: [] } }]);
  const runs = await client.getRunHistory({ workflowFile: "ci.yml", branch: "main" });
  assert.deepEqual(runs, []);
  assert.deepEqual(calls, [["api", historyEndpoint(1)]]);
});

test("transport: a mid-history API failure rejects instead of returning a prefix", async () => {
  const pages = [
    histPage(Array.from({ length: 30 }, (_, i) => 100 + i)),
    { error: new Error("HTTP 502: Bad Gateway") },
  ];
  const { client, calls } = ghClient(pages);
  await assert.rejects(client.getRunHistory({ workflowFile: "ci.yml", branch: "main" }), /HTTP 502/);
  assert.deepEqual(calls, [
    ["api", historyEndpoint(1)],
    ["api", historyEndpoint(2)],
  ]);
});

test("transport: malformed JSON on a later page rejects instead of returning a prefix", async () => {
  const pages = [
    histPage(Array.from({ length: 30 }, (_, i) => 100 + i)),
    { raw: '{"workflow_runs": [broken' },
  ];
  const { client } = ghClient(pages);
  await assert.rejects(client.getRunHistory({ workflowFile: "ci.yml", branch: "main" }), SyntaxError);
});

test("transport: an envelope without a workflow_runs array rejects", async () => {
  for (const bad of [{ total_count: 1 }, { workflow_runs: "nope" }, { workflow_runs: null }]) {
    const pages = [
      histPage(Array.from({ length: 30 }, (_, i) => 100 + i)),
      { body: bad },
    ];
    const { client } = ghClient(pages);
    await assert.rejects(client.getRunHistory({ workflowFile: "ci.yml", branch: "main" }));
  }
});

test("transport: a non-array envelope on the first page rejects", async () => {
  const { client } = ghClient([{ body: {} }]);
  await assert.rejects(client.getRunHistory({ workflowFile: "ci.yml", branch: "main" }));
});

const COMPARE_BASE = "128ba97cc4b7a0e4028565d4e8e1bfeda02bd8fa";
const COMPARE_HEAD = "669d6d88892dd61d166035aa8a3331def0311957";
const COMPARE_ENDPOINT = `repos/${REPO}/compare/${COMPARE_BASE}...${COMPARE_HEAD}`;

function compareFixture(status, pad = "p".repeat(6000)) {
  return {
    status,
    ahead_by: 1,
    behind_by: 2,
    total_commits: 3,
    commits: Array.from({ length: 60 }, (_, i) => ({
      sha: `${i}`.padStart(40, "0"),
      commit: { message: `commit ${i}\n\n${pad}` },
    })),
    files: Array.from({ length: 150 }, (_, i) => ({
      filename: `crates/crate-${i}/src/lib.rs`,
      status: "modified",
      additions: 10,
      deletions: 2,
      patch: `@@ -1,2 +1,10 @@\n+${pad}`,
    })),
  };
}

function fakeCompareExecFile(fixture) {
  const calls = [];
  const raw = fixture.raw !== undefined ? fixture.raw : JSON.stringify(fixture.body);
  const execFile = (file, args, options = {}) => {
    assert.equal(file, "gh");
    assert.equal(args[0], "api");
    calls.push({ file, args, options });
    if (fixture.error) throw fixture.error;
    let out = raw;
    const jq = args.indexOf("--jq");
    if (jq !== -1) {
      assert.equal(args[jq + 1], "{status: .status}");
      out = JSON.stringify({ status: JSON.parse(raw).status });
    }
    if (Buffer.byteLength(out) > (options.maxBuffer ?? MIB)) throw enobufs();
    return out;
  };
  return { execFile, calls };
}

const ghCompareClient = (fixture) => {
  const { execFile, calls } = fakeCompareExecFile(fixture);
  return { client: makeGhClient({ repo: REPO, execFile }), calls };
};

test("transport: a >1 MiB compare response is projected to {status} inside gh without ENOBUFS", async () => {
  const fixture = compareFixture("diverged");
  assert.ok(Buffer.byteLength(JSON.stringify(fixture)) > MIB, "fixture must exceed the 1 MiB execFileSync default maxBuffer");
  const { client, calls } = ghCompareClient({ body: fixture });
  const status = await client.compareCommits(COMPARE_BASE, COMPARE_HEAD);
  assert.equal(status, "diverged");
  assert.deepEqual(calls.map((c) => [c.file, ...c.args]), [
    ["gh", "api", COMPARE_ENDPOINT, "--jq", "{status: .status}"],
  ]);
  assert.equal(calls[0].options.maxBuffer, undefined);
});

test("transport: an unfiltered read of the same >1 MiB compare fixture dies with ENOBUFS", () => {
  const { execFile } = fakeCompareExecFile({ body: compareFixture("ahead") });
  assert.throws(
    () => execFile("gh", ["api", COMPARE_ENDPOINT], { encoding: "utf8" }),
    (e) => e.code === "ENOBUFS",
  );
});

test("transport: every valid compare status is returned verbatim", async () => {
  for (const expected of ["identical", "ahead", "behind", "diverged"]) {
    const { client } = ghCompareClient({ body: compareFixture(expected, "") });
    assert.equal(await client.compareCommits(COMPARE_BASE, COMPARE_HEAD), expected);
  }
});

test("transport: a missing, null, or unknown compare status rejects", async () => {
  for (const body of [{}, { status: null }, { status: "merged" }, { status: 42 }]) {
    const { client } = ghCompareClient({ body });
    await assert.rejects(client.compareCommits(COMPARE_BASE, COMPARE_HEAD));
  }
});

test("transport: malformed compare JSON rejects with SyntaxError", async () => {
  const { client } = ghCompareClient({ raw: '{"status": "ahead", "files": [broken' });
  await assert.rejects(client.compareCommits(COMPARE_BASE, COMPARE_HEAD), SyntaxError);
});

test("transport: a compare API failure rejects and propagates", async () => {
  const { client } = ghCompareClient({ error: new Error("HTTP 502: Bad Gateway") });
  await assert.rejects(client.compareCommits(COMPARE_BASE, COMPARE_HEAD), /HTTP 502/);
});
