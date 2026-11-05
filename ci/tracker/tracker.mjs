#!/usr/bin/env node
// tracker.mjs — Issue #368 CI failure Issue Tracker.
//
// Watches `workflow_run` completions of the `ci` workflow and keeps at most
// ONE open failure issue per scope (repo × workflow × branch). It is the
// Issue-side counterpart of `ci/summary/summary.mjs`: where ci-summary
// fail-closes the run, the tracker records that a main Full CI failure
// exists and closes the issue only when a NEWER success covers the failed
// revision.
//
// Trust model (Issue #368 implementation 2): this script runs from the
// default branch inside a privileged `workflow_run` job. It never checks
// out, downloads, or executes the completed run's code or artifacts — the
// only inputs are the workflow_run event payload and GitHub REST metadata
// (issues, jobs, compare). Every externally-sourced value written into an
// issue body, comment, or the job summary is sanitized/allowlisted.
//
// Ordering: GitHub run ids increase monotonically, so "the latest valid
// Full CI run" is ordered by (run id, run attempt) — completion-order
// inversions cannot let an older run overwrite newer state, and a rerun
// (same run id, higher attempt) still counts as newer evidence. Closing
// additionally requires the compare API to prove the failed sha is
// identical to or an ancestor of the success sha, so an old or divergent
// success can never close a newer failure.
//
// The GitHub client is injectable: `run()` and `plan()` are fully
// deterministic under a mock client, and all tests are offline.
//
// Usage:
//   TRACKER_EVENT_PATH=event.json GH_TOKEN=*** node ci/tracker/tracker.mjs \
//       [--repo owner/name] [--branch main] [--workflow ci] \
//       [--label ci-failure] [--report out.json] [--summary out.md]

import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// ---------------------------------------------------------------- scope ---

export const TRACKER_LABEL = "ci-failure";
export const LABEL_COLOR = "d93f0b";
export const ISSUE_TITLE = "[CI failure][main] Full CI failed";
export const MARKER_TAG = "ci-issue-tracker";
export const HISTORY_LIMIT = 10;

// Events whose completed `ci` runs count as valid main Full CI executions:
// `push` (main) and `workflow_dispatch` (manual + the Issue #367 daily
// scheduler). `pull_request`/`merge_group`/`schedule`/anything else is
// skipped — the allowlist is deliberate so a new trigger added to ci.yml
// cannot silently become tracked evidence.
export const ALLOWED_EVENTS = ["push", "workflow_dispatch"];

// Conclusions that mean "the run finished without verifying" — the failure
// side of the tracker. `timed_out`/`startup_failure` never verified either.
// Everything else (`cancelled`, `skipped`, `neutral`, `stale`,
// `action_required`, null, unknown) is operational: not success, and not a
// reason to open a failure issue.
export const FAILURE_CONCLUSIONS = ["failure", "timed_out", "startup_failure"];
export const HONORED_CONCLUSIONS = [
  "success",
  "failure",
  "timed_out",
  "startup_failure",
  "cancelled",
  "skipped",
  "neutral",
  "stale",
  "action_required",
];

// ------------------------------------------------------------ sanitize ---

const SHA_RE = /^[0-9a-f]{40}$/;
const ISO_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})$/;

export function sanitizeSha(v) {
  return typeof v === "string" && SHA_RE.test(v) ? v : null;
}

export function sanitizeUrl(v) {
  if (typeof v !== "string" || v.length > 500) return null;
  if (!v.startsWith("https://github.com/")) return null;
  // printable ASCII only — no control characters or whitespace tricks.
  if (!/^[\x21-\x7e]+$/.test(v)) return null;
  return v;
}

// Free-form text taken from API metadata (job names, branch names, OS
// labels). Strip control characters, `|` (table injection), and backticks;
// bound the length.
export function sanitizeText(v, max = 100) {
  if (typeof v !== "string") return "";
  const cleaned = v.replace(/[\x00-\x1f\x7f|`]/g, "").trim();
  return cleaned.length > max ? `${cleaned.slice(0, max)}…` : cleaned;
}

export function sanitizeConclusion(v) {
  return typeof v === "string" && HONORED_CONCLUSIONS.includes(v) ? v : "unknown";
}

export function sanitizeRunId(v) {
  return typeof v === "number" && Number.isSafeInteger(v) && v > 0 ? v : null;
}

function sanitizeIso(v) {
  return typeof v === "string" && ISO_RE.test(v) ? v : null;
}

// ----------------------------------------------------------- classify ----

export function defaultScope(repo) {
  return {
    repo,
    workflow: "ci",
    branch: "main",
    allowedEvents: [...ALLOWED_EVENTS],
    label: TRACKER_LABEL,
  };
}

// A completed run is a valid Full CI target only when it ran on this
// repository's `main` branch via an allowed event. Anything else — fork
// runs, PR auxiliary runs, other branches, non-completed actions — is a
// hard `skip` and must not touch issues.
export function classifyRun(payload, scope) {
  const wr = payload?.workflow_run;
  const skip = (reason) => ({ kind: "skip", reason });
  if (payload?.action !== "completed") return skip(`action:${payload?.action ?? "missing"}`);
  if (wr == null || typeof wr !== "object") return skip("no-workflow_run");
  if (wr.head_repository?.full_name !== scope.repo) {
    return skip(`repo:${sanitizeText(wr.head_repository?.full_name, 200) || "unknown"}`);
  }
  if (wr.head_branch !== scope.branch) {
    return skip(`branch:${sanitizeText(wr.head_branch) || "unknown"}`);
  }
  if (!scope.allowedEvents.includes(wr.event)) {
    return skip(`event:${sanitizeText(wr.event) || "unknown"}`);
  }
  const id = sanitizeRunId(wr.id);
  const sha = sanitizeSha(wr.head_sha);
  if (id == null || sha == null) return skip("malformed-run-metadata");

  const raw = typeof wr.conclusion === "string" ? wr.conclusion : null;
  const conclusion = sanitizeConclusion(raw);
  const run = {
    id,
    sha,
    url: sanitizeUrl(wr.html_url),
    conclusion: raw,
    event: wr.event,
    attempt: sanitizeRunId(wr.run_attempt) ?? 1,
    updatedAt: sanitizeIso(wr.updated_at),
  };
  if (conclusion === "success") return { kind: "success", run };
  if (FAILURE_CONCLUSIONS.includes(conclusion)) return { kind: "failure", run };
  return { kind: "operational", run };
}

// ------------------------------------------------------------- marker ----

const MARKER_RE = new RegExp(`^\\s*<!--\\s*${MARKER_TAG}:([\\s\\S]*?)-->`);

// The stable identity: a JSON marker as the first HTML comment of the issue
// body, carrying scope + machine state. Issues carrying the label but no
// parseable marker (or a marker for a different scope) are foreign and must
// never be modified.
export function renderMarker(scope, state) {
  const json = JSON.stringify({
    v: 1,
    scope: { repo: scope.repo, workflow: scope.workflow, branch: scope.branch },
    state,
  })
    // Angle brackets only occur inside JSON string literals, so escaping
    // them keeps the JSON valid while making `-->` / `<!--` sequences
    // impossible — a hostile job name can never break out of the comment.
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e");
  return `<!-- ${MARKER_TAG}:${json} -->`;
}

export function parseMarker(body) {
  if (typeof body !== "string") return null;
  const m = MARKER_RE.exec(body);
  if (m == null) return null;
  let parsed;
  try {
    parsed = JSON.parse(m[1]);
  } catch {
    return null;
  }
  if (parsed?.v !== 1 || parsed.scope == null || parsed.state == null) return null;
  const { scope, state } = parsed;
  if (
    typeof scope.repo !== "string" ||
    typeof scope.workflow !== "string" ||
    typeof scope.branch !== "string"
  ) {
    return null;
  }
  const failure = state.failure;
  if (
    !Number.isSafeInteger(state.latestRunId) ||
    failure == null ||
    !Number.isSafeInteger(failure.runId) ||
    !SHA_RE.test(failure.sha ?? "")
  ) {
    return null;
  }
  return { scope, state };
}

// Run ordering key: (id, attempt). A rerun keeps the same id with a higher
// attempt, so lexicographic comparison treats "re-run failed jobs" as
// newer evidence than the attempt that failed.
export function isNewerRun(run, ref) {
  return run.id > ref.runId || (run.id === ref.runId && run.attempt > (ref.attempt ?? 1));
}

export function scopeMatches(markerScope, scope) {
  return (
    markerScope.repo === scope.repo &&
    markerScope.workflow === scope.workflow &&
    markerScope.branch === scope.branch
  );
}

// ------------------------------------------------------------ body -------

function jobRows(jobs) {
  if (!Array.isArray(jobs) || jobs.length === 0) return ["(no job data)"];
  return jobs.map(
    (j) => `| ${sanitizeText(j.name) || "?"} | ${sanitizeConclusion(j.conclusion)} |`,
  );
}

function historyRows(history) {
  return (history ?? []).map(
    (h) =>
      `| ${h.runId} | \`${sanitizeSha(h.sha) ?? "?"}\` | ${sanitizeConclusion(h.conclusion)} | ${sanitizeText(h.kind)} | ${sanitizeIso(h.at) ?? "?"} |`,
  );
}

// Human-readable body. Everything dynamic is sanitized before it lands —
// the marker line carries the machine state and stays decoration-free.
export function renderBody(scope, state) {
  const f = state.failure;
  const lines = [
    renderMarker(scope, state),
    "",
    `## CI failure tracker — \`${sanitizeText(scope.branch)}\` × \`${sanitizeText(scope.workflow)}\``,
    "",
    "Managed by `.github/workflows/ci-issue-tracker.yml` (Issue #368).",
    "Do not edit the leading marker comment — it identifies this issue to the tracker.",
    "",
    "### Status: OPEN — unresolved Full CI failure",
    "",
    "| field | value |",
    "| --- | --- |",
    `| latest failure run | ${sanitizeUrl(f.runUrl) ?? `run id ${f.runId}`} (run id ${f.runId}) |`,
    `| failed revision | \`${sanitizeSha(f.sha) ?? "?"}\` |`,
    `| conclusion | \`${sanitizeConclusion(f.conclusion)}\` |`,
    `| first detected | ${sanitizeIso(f.firstAt) ?? "?"} (revision \`${sanitizeSha(f.firstSha) ?? "?"}\`) |`,
    `| last detected | ${sanitizeIso(f.lastAt) ?? "?"} |`,
  ];
  if (state.operationalNote) {
    lines.push("", `> ${sanitizeText(state.operationalNote, 300)}`);
  }
  lines.push(
    "",
    "#### Jobs in latest failing run",
    "",
    "| job | conclusion |",
    "| --- | --- |",
    ...jobRows(f.jobs),
    "",
    "#### Recent observed runs",
    "",
    "| run id | sha | conclusion | kind | at |",
    "| --- | --- | --- | --- | --- |",
    ...historyRows(state.history),
    "",
    "#### Triage",
    "",
    "1. Open the latest failing run and inspect the failing jobs.",
    "2. Land a fix on `main` or re-run `ci` via `workflow_dispatch`.",
    "3. This issue closes automatically only when a **newer** `ci` run succeeds",
    "   at a revision covering the failed one. Old-run or divergent successes",
    "   do not close it.",
    "",
    "#### Operational policy",
    "",
    "- `cancelled`/`skipped`/unknown conclusions are neither failure nor",
    "  success — they are recorded here as an operational note only.",
    "- The tracker never reopens manually closed issues; a failure after a",
    "  manual close opens a new issue.",
    "- While GitHub Actions itself is down the tracker cannot run — check the",
    "  last green `ci` run and re-dispatch manually after recovery.",
    "",
  );
  return lines.join("\n");
}

// --------------------------------------------------------------- plan ----

function historyPush(state, entry) {
  const history = [...(state.history ?? []), entry];
  return history.slice(-HISTORY_LIMIT);
}

function baseState(run, jobs) {
  return {
    latestRunId: run.id,
    latestAttempt: run.attempt,
    failure: {
      sha: run.sha,
      runId: run.id,
      attempt: run.attempt,
      runUrl: run.url,
      conclusion: run.conclusion,
      firstSha: run.sha,
      firstAt: run.updatedAt,
      lastAt: run.updatedAt,
      jobs: jobs ?? [],
    },
    history: [
      { kind: "failure", runId: run.id, attempt: run.attempt, sha: run.sha, conclusion: run.conclusion, at: run.updatedAt },
    ],
    operationalNote: null,
  };
}

// `plan` returns the action list for one classified run against the managed
// issues that already exist. `compareStatus` is consulted only for the
// success path and is null otherwise — the caller must not fetch it eagerly
// (an unused compare call is a wasted API request on every failure event).
export function plan(classified, { canonical, duplicates, jobs = null, compareStatus = null }) {
  const actions = duplicates.map((d) => ({
    type: "close-duplicate",
    issue: d.number,
    canonical: canonical?.number ?? null,
  }));
  const run = classified.run;
  const state = canonical?.marker?.state ?? null;

  if (classified.kind === "failure") {
    if (canonical == null) {
      actions.push({ type: "create", state: baseState(run, jobs) });
    } else if (isNewerRun(run, state.failure)) {
      const next = {
        latestRunId: Math.max(state.latestRunId, run.id),
        latestAttempt: Math.max(state.latestAttempt ?? 1, run.attempt),
        failure: {
          sha: run.sha,
          runId: run.id,
          attempt: run.attempt,
          runUrl: run.url,
          conclusion: run.conclusion,
          firstSha: state.failure.firstSha,
          firstAt: state.failure.firstAt,
          lastAt: run.updatedAt,
          jobs: jobs ?? [],
        },
        history: historyPush(state, {
          kind: "failure",
          runId: run.id,
          attempt: run.attempt,
          sha: run.sha,
          conclusion: run.conclusion,
          at: run.updatedAt,
        }),
        operationalNote: null,
      };
      actions.push({ type: "update", issue: canonical.number, state: next });
    } else {
      actions.push({
        type: "noop",
        reason: `stale-failure:run-${run.id}a${run.attempt}<=failure-${state.failure.runId}a${state.failure.attempt ?? 1}`,
      });
    }
    return actions;
  }

  if (classified.kind === "success") {
    if (canonical == null) {
      actions.push({ type: "noop", reason: "success:no-open-tracker-issue" });
      return actions;
    }
    if (!isNewerRun(run, state.failure)) {
      actions.push({
        type: "noop",
        reason: `stale-success:run-${run.id}a${run.attempt}<=failure-${state.failure.runId}a${state.failure.attempt ?? 1}`,
      });
      return actions;
    }
    if (compareStatus === "identical" || compareStatus === "ahead") {
      actions.push({
        type: "close-recovered",
        issue: canonical.number,
        recovery: { sha: run.sha, runId: run.id, runUrl: run.url },
      });
    } else if (isNewerRun(run, { runId: state.latestRunId, attempt: state.latestAttempt })) {
      const next = {
        ...state,
        latestRunId: run.id,
        latestAttempt: run.attempt,
        history: historyPush(state, {
          kind: "success-not-covering",
          runId: run.id,
          attempt: run.attempt,
          sha: run.sha,
          conclusion: run.conclusion,
          at: run.updatedAt,
        }),
        operationalNote: `Newer success run ${run.id} does not cover the failed revision (compare:${compareStatus ?? "unavailable"}) — issue stays open.`,
      };
      actions.push({ type: "update", issue: canonical.number, state: next });
    } else {
      actions.push({ type: "noop", reason: `success-not-covering:${compareStatus}` });
    }
    return actions;
  }

  // operational — cancelled/skipped/neutral/stale/action_required/unknown:
  // never a success (no close) and never a failure (no new issue).
  if (canonical == null) {
    actions.push({ type: "noop", reason: `operational:${classified.run.conclusion ?? "null"}` });
  } else if (isNewerRun(run, { runId: state.latestRunId, attempt: state.latestAttempt })) {
    const next = {
      ...state,
      latestRunId: run.id,
      latestAttempt: run.attempt,
      history: historyPush(state, {
        kind: "operational",
        runId: run.id,
        attempt: run.attempt,
        sha: run.sha,
        conclusion: run.conclusion,
        at: run.updatedAt,
      }),
      operationalNote: `Latest observed run ${run.id} concluded \`${sanitizeConclusion(run.conclusion)}\` — verification pending retry; counted as neither failure nor success.`,
    };
    actions.push({ type: "update", issue: canonical.number, state: next });
  } else {
    actions.push({ type: "noop", reason: `operational-stale:run-${run.id}` });
  }
  return actions;
}

// --------------------------------------------------------------- apply ---

async function applyActions(actions, scope, client) {
  const applied = [];
  for (const a of actions) {
    if (a.type === "noop") {
      applied.push({ ...a });
      continue;
    }
    if (a.type === "close-duplicate") {
      await client.addComment(
        a.issue,
        `Superseded by #${a.canonical} — one open CI failure issue per scope. Closing as duplicate.`,
      );
      await client.closeIssue(a.issue);
      applied.push({ ...a });
      continue;
    }
    if (a.type === "create") {
      const created = await client.createIssue({
        title: ISSUE_TITLE,
        body: renderBody(scope, a.state),
        labels: [scope.label],
      });
      applied.push({ ...a, issue: created?.number ?? null });
      continue;
    }
    if (a.type === "update") {
      await client.updateIssue(a.issue, {
        title: ISSUE_TITLE,
        body: renderBody(scope, a.state),
      });
      applied.push({ ...a });
      continue;
    }
    if (a.type === "close-recovered") {
      await client.addComment(
        a.issue,
        [
          `Recovered: newer \`ci\` run ${a.recovery.runId} succeeded at revision`,
          `\`${a.recovery.sha}\` covering the failed revision.`,
          a.recovery.runUrl ? `Run: ${a.recovery.runUrl}` : null,
        ]
          .filter(Boolean)
          .join(" "),
      );
      await client.closeIssue(a.issue);
      applied.push({ ...a });
      continue;
    }
    throw new Error(`unknown action type ${a.type}`);
  }
  return applied;
}

// ----------------------------------------------------------------- run ---

// Orchestrator: classify → ensure label → list managed issues → fetch what
// the plan needs (jobs for failures, compare only for a closing candidate)
// → apply. Every API error propagates — the tracker job must fail and stay
// diagnosable rather than swallow a permission or outage problem.
export async function run({ payload, client, scope }) {
  const classified = classifyRun(payload, scope);
  const result = { scope, classified, canonical: null, actions: [], applied: [], mainHead: null };
  if (classified.kind === "skip") {
    result.actions = [{ type: "noop", reason: classified.reason }];
    result.applied = result.actions;
    return result;
  }

  await client.ensureLabel(scope.label, LABEL_COLOR);
  const listed = await client.listOpenIssues(scope.label);
  const managed = [];
  for (const issue of listed ?? []) {
    if (issue?.pull_request != null) continue;
    const marker = parseMarker(issue?.body ?? "");
    if (marker != null && scopeMatches(marker.scope, scope)) {
      managed.push({ number: issue.number, marker });
    }
  }
  managed.sort((a, b) => a.number - b.number);
  const canonical = managed[0] ?? null;
  const duplicates = managed.slice(1);
  result.canonical = canonical?.number ?? null;

  try {
    result.mainHead = sanitizeSha((await client.getBranchHead(scope.branch))?.sha ?? null);
  } catch {
    // Diagnostic-only — absence of the branch tip must not block tracking.
    result.mainHead = null;
  }

  const needsJobs = classified.kind === "failure";
  const jobs = needsJobs
    ? (await client.getRunJobs(classified.run.id)).map((j) => ({
        // sanitize at the input boundary: these values end up both in the
        // rendered table and inside the marker JSON state.
        name: sanitizeText(j?.name),
        conclusion: sanitizeConclusion(j?.conclusion),
      }))
    : null;

  const needCompare =
    classified.kind === "success" &&
    canonical != null &&
    isNewerRun(classified.run, canonical.marker.state.failure);
  const compareStatus = needCompare
    ? await client.compareCommits(canonical.marker.state.failure.sha, classified.run.sha)
    : null;

  result.actions = plan(classified, { canonical, duplicates, jobs, compareStatus });
  result.applied = await applyActions(result.actions, scope, client);
  return result;
}

// ------------------------------------------------------------ gh client --

// Thin `gh api` wrapper — gh CLI is preinstalled on every GitHub-hosted
// runner and authenticates via GH_TOKEN. JSONL outputs (`--paginate --jq`)
// are parsed line-wise so pagination never needs JSON-array juggling.
export function makeGhClient({ repo, gh = "gh", execFile = execFileSync } = {}) {
  function api(args) {
    return execFile(gh, ["api", ...args], { encoding: "utf8" });
  }
  function apiJson(args) {
    return JSON.parse(api(args));
  }
  function apiJsonLines(args) {
    return api(args)
      .split("\n")
      .filter((l) => l.trim() !== "")
      .map((l) => JSON.parse(l));
  }
  return {
    async ensureLabel(name, color) {
      try {
        api([`repos/${repo}/labels/${encodeURIComponent(name)}`]);
        return;
      } catch {
        // missing label — create below; any other error surfaces at POST.
      }
      api([`repos/${repo}/labels`, "-f", `name=${name}`, "-f", `color=${color}`]);
    },
    async listOpenIssues(label) {
      return apiJsonLines([
        `repos/${repo}/issues?labels=${encodeURIComponent(label)}&state=open&per_page=100`,
        "--paginate",
        "--jq",
        ".[]",
      ]);
    },
    async getRunJobs(runId) {
      return apiJsonLines([
        `repos/${repo}/actions/runs/${runId}/jobs?per_page=100`,
        "--paginate",
        "--jq",
        ".jobs[]",
      ]);
    },
    async compareCommits(base, head) {
      const out = apiJson([`repos/${repo}/compare/${base}...${head}`]);
      const status = out?.status;
      if (!["identical", "ahead", "behind", "diverged"].includes(status)) {
        throw new Error(`compare returned unexpected status: ${status}`);
      }
      return status;
    },
    async getBranchHead(branch) {
      const out = apiJson([`repos/${repo}/branches/${encodeURIComponent(branch)}`]);
      return { sha: out?.commit?.sha ?? null };
    },
    async createIssue({ title, body, labels }) {
      const args = [`repos/${repo}/issues`, "-f", `title=${title}`, "-f", `body=${body}`];
      for (const l of labels) args.push("-f", `labels[]=${l}`);
      return apiJson(args);
    },
    async updateIssue(number, { title, body }) {
      return apiJson([
        `repos/${repo}/issues/${number}`,
        "-X",
        "PATCH",
        "-f",
        `title=${title}`,
        "-f",
        `body=${body}`,
      ]);
    },
    async addComment(number, body) {
      return apiJson([`repos/${repo}/issues/${number}/comments`, "-f", `body=${body}`]);
    },
    async closeIssue(number) {
      return apiJson([`repos/${repo}/issues/${number}`, "-X", "PATCH", "-f", "state=closed"]);
    },
  };
}

// ---------------------------------------------------------------- main ---

function summaryLines(result) {
  const c = result.classified;
  const lines = [
    `## ci-issue-tracker`,
    "",
    `- scope: \`${result.scope.repo}\` × \`${result.scope.workflow}\` × \`${result.scope.branch}\``,
    `- run: ${c.run ? `id ${c.run.id} · sha \`${c.run.sha}\` · conclusion \`${c.run.conclusion}\` · ${c.run.url ?? "?"}` : "-"}`,
    `- classified: \`${c.kind}\`${c.reason ? ` (${c.reason})` : ""}`,
    `- current main head: \`${result.mainHead ?? "?"}\``,
    `- canonical issue: ${result.canonical != null ? `#${result.canonical}` : "none"}`,
    "",
    "| action | detail |",
    "| --- | --- |",
    ...result.applied.map(
      (a) =>
        `| ${a.type} | ${a.issue != null ? `#${a.issue}` : ""} ${a.reason ?? ""}${a.recovery ? `recovered at ${a.recovery.sha}` : ""} |`.trimEnd(),
    ),
    "",
  ];
  return lines.join("\n");
}

export async function main(argv = process.argv, env = process.env) {
  const args = {
    event: env.TRACKER_EVENT_PATH ?? env.GITHUB_EVENT_PATH ?? null,
    repo: env.GITHUB_REPOSITORY ?? null,
    branch: "main",
    workflow: "ci",
    label: TRACKER_LABEL,
    report: null,
    summary: null,
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--event") args.event = next();
    else if (a === "--repo") args.repo = next();
    else if (a === "--branch") args.branch = next();
    else if (a === "--workflow") args.workflow = next();
    else if (a === "--label") args.label = next();
    else if (a === "--report") args.report = next();
    else if (a === "--summary") args.summary = next();
    else if (a === "--help" || a === "-h") {
      console.log(
        "Usage: node ci/tracker/tracker.mjs [--event <file>] [--repo <owner/name>] [--branch <b>] [--workflow <name>] [--label <l>] [--report <file>] [--summary <file>]  (event path also read from $TRACKER_EVENT_PATH/$GITHUB_EVENT_PATH)",
      );
      process.exit(0);
    } else {
      console.error(`error: unknown argument ${a}`);
      process.exit(2);
    }
  }
  if (args.event == null) {
    console.error("error: no event payload (pass --event <file> or set TRACKER_EVENT_PATH)");
    process.exit(2);
  }
  if (args.repo == null) {
    console.error("error: no repo (pass --repo owner/name or set GITHUB_REPOSITORY)");
    process.exit(2);
  }
  const payload = JSON.parse(readFileSync(args.event, "utf8"));
  const scope = {
    ...defaultScope(args.repo),
    branch: args.branch,
    workflow: args.workflow,
    label: args.label,
  };
  const client = makeGhClient({ repo: args.repo });
  const result = await run({ payload, client, scope });

  const report = {
    schemaVersion: 1,
    kind: "ci-issue-tracker",
    repo: args.repo,
    scope: { workflow: scope.workflow, branch: scope.branch, label: scope.label },
    classified: result.classified,
    canonical: result.canonical,
    mainHead: result.mainHead,
    actions: result.applied,
    generatedAt: new Date().toISOString(),
  };
  if (args.report) writeFileSync(args.report, JSON.stringify(report, null, 2) + "\n");

  const table = summaryLines(result);
  process.stdout.write(table + "\n");
  const summaryPath = args.summary ?? env.GITHUB_STEP_SUMMARY;
  if (summaryPath) appendFileSync(summaryPath, table + "\n");
}

const isMain =
  process.argv[1] != null &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main().catch((e) => {
    console.error(`error: ${e?.message ?? e}`);
    process.exit(1);
  });
}
