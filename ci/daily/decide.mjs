#!/usr/bin/env node
// decide.mjs — Issue #367 common daily CI scheduler decision engine.
//
// Decides, once per day (UTC schedule) or on manual re-check, whether the
// current main HEAD already has a valid Full CI result in the GitHub
// Actions run history of the target workflow (default `ci.yml`). If not,
// it dispatches that workflow on `ref: main` with `expected_sha` set to
// the observed SHA — the Issue #366/#367 dispatch contract enforced by
// `ci-summary` against the measured common checkout SHA.
//
// The ONLY state authority is the GitHub Actions run history itself: no
// dedicated DB, no repository variables. Every difficulty (eventual
// consistency, retention expiry, reruns, dispatch/landing races) is
// absorbed by strict matching plus fail-closed classification — a result
// may only be recorded as verified when the API reports a completed
// `success` run whose head_sha/head_branch/event all match.
//
// Strict matching (spec R2): an attempt "on" SHA S is a `ci.yml` run with
// head_sha === S AND head_branch === mainRef AND event ∈ FULL_CI_EVENTS
// ({push, workflow_dispatch, schedule} — pull_request runs point at the
// PR head/branch and never qualify). Scheduler runs, other workflows,
// other branches, other SHAs, and PR runs contribute nothing.
//
// Retry budget (spec R4): auto dispatches for S are counted as runs with
// event === "workflow_dispatch" whose ORIGIN is automation. Per the API
// contract, `actor` is the run's original initiator and `triggering_actor`
// is the actor of the LATEST (re)run attempt — so a bot-dispatched run
// rerun by a human keeps actor=github-actions[bot] while
// triggering_actor=<human>. Counting triggering_actor alone would refund
// the budget on every human rerun, so a run counts as auto when EITHER
// provenance field names an auto actor, and a run with no identifiable
// actor at all counts too (an undercounted budget retries forever —
// overcounting merely dispatches less, the safe side). Human
// `workflow_dispatch` runs do not consume the budget.
// run status/conclusion/run_attempt always describe the run's LATEST
// attempt, matching the "latest attempt on the SHA" decision rule.
//
// Usage:
//   GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo node ci/daily/decide.mjs
//       [--workflow ci.yml] [--main-ref main] [--api-base URL]
//       [--max-auto-dispatches 2] [--max-pages 10]
//       [--confirm-timeout-ms 120000] [--confirm-interval-ms 15000]
//       [--report FILE] [--summary FILE] [--fixture FILE] [--dry-run]
//
// Exit codes: 0 every completed outcome (including budget-exhausted /
// hold-unknown / history-truncated / main-moved / dispatched-unconfirmed
// — the decision ran to completion and the unverified state is reported
// via warnings), 1 on `error` (the scheduler could not complete its
// decision — API failure, malformed response, unreadable fixture), 2 usage.

import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// Events that represent a Full CI execution of the target workflow.
// pull_request runs are excluded structurally (head_branch is the PR
// source branch, head_sha is the PR head). Unknown future events are
// allowlist-external and can never contribute a false verification.
export const FULL_CI_EVENTS = ["push", "workflow_dispatch", "schedule"];
// GitHub-documented non-terminal statuses; anything else is treated as
// unknown and holds the decision rather than guessing.
export const IN_FLIGHT_STATUSES = ["queued", "waiting", "in_progress", "pending", "requested"];
export const TERMINAL_STATUS = "completed";
// GitHub-documented terminal conclusions. A completed run whose
// conclusion is null or outside this set is classified "unknown" —
// holding the decision rather than dispatching on an unrecognised state.
export const KNOWN_CONCLUSIONS = ["success", "failure", "neutral", "cancelled", "skipped", "timed_out", "action_required", "startup_failure", "stale"];

const SHA_RE = /^[0-9a-f]{40}$/;
const MAX_ATTEMPTS_IN_REPORT = 20;

class ApiError extends Error {
  constructor(message, status = null, body = null) {
    super(message);
    this.status = status;
    this.body = body;
  }
}

// ---------------------------------------------------------------------------
// Decision core (pure)
// ---------------------------------------------------------------------------

function isFullCiRun(run, mainRef) {
  return (
    run != null &&
    typeof run === "object" &&
    run.head_branch === mainRef &&
    FULL_CI_EVENTS.includes(run.event) &&
    typeof run.head_sha === "string" &&
    SHA_RE.test(run.head_sha)
  );
}

// Latest-attempt recency. `created_at` never moves (original creation);
// `run_started_at` is the start of the run's LATEST attempt, so a rerun
// moves it forward even though created_at stays old (verified on the
// real API: run 41958976200 attempt 2, created 03:01 / started 03:07).
// `updated_at` is excluded from the ordering key — it also moves on
// non-attempt bookkeeping — but is used as an ambiguity probe below:
// a run with recent activity that produced no newer attempt is
// unexplained evidence and must block a success claim, not be ignored.
function attemptTime(r) {
  let best = null;
  for (const key of ["created_at", "run_started_at"]) {
    const t = Date.parse(r?.[key] ?? "");
    if (!Number.isNaN(t) && (best == null || t > best)) best = t;
  }
  return best; // null = unorderable
}
function activityTime(r) {
  let best = attemptTime(r);
  const t = Date.parse(r?.updated_at ?? "");
  if (!Number.isNaN(t) && (best == null || t > best)) best = t;
  return best;
}

// Newest-first ordering: latest-attempt recency desc, then run id desc
// (higher id was created later when timestamps tie). Runs with no
// parseable timestamps sort last — they are unorderable, not newest.
function newestFirst(a, b) {
  const ta = attemptTime(a);
  const tb = attemptTime(b);
  if (ta !== tb) {
    if (ta == null) return 1;
    if (tb == null) return -1;
    return tb - ta;
  }
  return (b.id ?? 0) - (a.id ?? 0);
}

function actorOf(run) {
  return run?.triggering_actor?.login ?? run?.actor?.login ?? null;
}

function isSuccess(r) {
  return r != null && r.status === TERMINAL_STATUS && r.conclusion === "success";
}

// Coarse fail-closed classification of one run's latest-attempt state.
function classifyRun(r) {
  if (r.status !== TERMINAL_STATUS) {
    return IN_FLIGHT_STATUSES.includes(r.status) ? "in-flight" : "unknown";
  }
  if (!KNOWN_CONCLUSIONS.includes(r.conclusion)) return "unknown";
  return r.conclusion === "success" ? "success" : "terminal-non-success";
}

// Retry-budget accounting (see header): automation-origin dispatches.
// actor = original initiator, triggering_actor = latest attempt's
// triggerer — a human rerun of a bot dispatch does not refund budget.
// A workflow_dispatch run with no identifiable actor counts too
// (undercounting the budget is the unsafe direction).
function isAutoDispatch(run, autoActors) {
  if (run?.event !== "workflow_dispatch") return false;
  const actor = run.actor?.login ?? null;
  const triggerer = run.triggering_actor?.login ?? null;
  if (actor == null && triggerer == null) return true;
  return autoActors.includes(actor) || autoActors.includes(triggerer);
}

// Evaluates the current main SHA against the run history.
//   decide({ mainSha, runs, mainRef, autoActors, maxAutoDispatches })
//   -> { decision, reason, latest, inFlight, autoDispatches, latestVerified }
// decision ∈ skip-verified | suppress-in-flight | hold-unknown |
//            budget-exhausted | dispatch
export function decide({ mainSha, runs, mainRef = "main", autoActors = ["github-actions[bot]"], maxAutoDispatches = 2 }) {
  const attempts = (runs ?? [])
    .filter((r) => isFullCiRun(r, mainRef) && r.head_sha === mainSha)
    .sort(newestFirst);
  const latest = attempts[0] ?? null;
  const inFlight = attempts.filter((r) => classifyRun(r) === "in-flight");
  const autoDispatches = attempts.filter((r) => isAutoDispatch(r, autoActors));

  const verifiedPool = (runs ?? [])
    .filter((r) => isFullCiRun(r, mainRef) && isSuccess(r))
    .sort(newestFirst);
  const latestVerified = verifiedPool[0] ?? null;

  let decision;
  let reason;
  if (inFlight.length > 0) {
    // Something is still running for this SHA — never double-dispatch.
    decision = "suppress-in-flight";
    reason = `${inFlight.length} in-flight run(s) on ${mainSha.slice(0, 12)}`;
  } else if (latest != null && classifyRun(latest) === "unknown") {
    // The newest attempt reports a status or conclusion we cannot
    // classify — hold rather than guess success or dispatch a duplicate.
    decision = "hold-unknown";
    reason = `unrecognised state "${latest.status}"/"${latest.conclusion ?? "null"}" on latest run ${latest.id}`;
  } else if (latest != null && isSuccess(latest)) {
    // A success claim is only sound if `latest` is provably the newest
    // effective attempt on the SHA. A tied or unorderable non-success
    // run, or one with newer unexplained activity (updated_at past the
    // latest attempt start — e.g. a rerun not yet reflected in
    // run_started_at), makes the "latest" ambiguous: hold, never
    // record a false verified.
    const top = attemptTime(latest);
    const contender = attempts.find((r) => {
      if (r === latest || isSuccess(r)) return false;
      const at = attemptTime(r);
      if (top == null || at == null || at === top) return true;
      const act = activityTime(r);
      return act != null && act > top;
    });
    if (contender != null) {
      decision = "hold-unknown";
      reason = `latest-attempt ordering ambiguous: run ${contender.id} contests success run ${latest.id}`;
    } else {
      decision = "skip-verified";
      reason = `run ${latest.id} completed success on ${mainSha.slice(0, 12)}`;
    }
  } else {
    const base = latest == null ? "no-history" : `latest:${latest.status}/${latest.conclusion ?? "null"}`;
    if (autoDispatches.length >= maxAutoDispatches) {
      decision = "budget-exhausted";
      reason = `${base}; ${autoDispatches.length} auto dispatches >= limit ${maxAutoDispatches}`;
    } else {
      decision = "dispatch";
      reason = `${base}; auto dispatches ${autoDispatches.length}/${maxAutoDispatches}`;
    }
  }

  return { decision, reason, attempts, latest, inFlight, autoDispatches, latestVerified };
}

// ---------------------------------------------------------------------------
// GitHub API client + fixture client
// ---------------------------------------------------------------------------

export function createGitHubClient({ token, repo, apiBase = "https://api.github.com", fetchImpl = fetch }) {
  const base = apiBase.replace(/\/+$/, "");
  async function request(method, path, body) {
    let res;
    try {
      res = await fetchImpl(`${base}${path}`, {
        method,
        headers: {
          Authorization: `Bearer ${token}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
          "User-Agent": "ruviform-daily-ci",
        },
        body: body == null ? undefined : JSON.stringify(body),
      });
    } catch (e) {
      throw new ApiError(`github api ${method} ${path} failed: ${e.message}`);
    }
    const text = await res.text();
    if (!res.ok) {
      throw new ApiError(`github api ${method} ${path} -> ${res.status}`, res.status, text.slice(0, 500));
    }
    if (!text) return null;
    try {
      return JSON.parse(text);
    } catch (e) {
      throw new ApiError(`github api ${method} ${path} invalid JSON: ${e.message}`);
    }
  }

  return {
    // heads/<mainRef> must resolve to a commit sha — validate before use.
    async getRefSha(ref) {
      const data = await request("GET", `/repos/${repo}/git/ref/heads/${ref}`);
      const sha = data?.object?.sha;
      if (typeof sha !== "string" || !SHA_RE.test(sha)) {
        throw new ApiError(`ref heads/${ref} did not resolve to a commit sha`);
      }
      return sha;
    },
    // Paginate the workflow run history. The server-side `branch` filter is
    // an optimisation only — strict matching lives in decide(). A 200
    // body without the workflow_runs array is a malformed response —
    // an error, never "empty history" (empty history dispatches).
    async listRuns({ workflow, branch, perPage = 100, maxPages = 10 }) {
      const all = [];
      for (let page = 1; page <= maxPages; page++) {
        const data = await request(
          "GET",
          `/repos/${repo}/actions/workflows/${workflow}/runs?branch=${encodeURIComponent(branch)}&per_page=${perPage}&page=${page}`,
        );
        if (data == null || !Array.isArray(data.workflow_runs)) {
          throw new ApiError(`runs list page ${page} malformed: missing workflow_runs array`);
        }
        if (data.workflow_runs.some((r) => r == null || typeof r !== "object")) {
          throw new ApiError(`runs list page ${page} malformed: non-object run entry`);
        }
        const pageRuns = data.workflow_runs;
        all.push(...pageRuns);
        if (pageRuns.length < perPage) return { runs: all, truncated: false };
      }
      return { runs: all, truncated: true };
    },
    // POST dispatches returns 204 with no body; a non-2xx throws.
    async dispatch({ workflow, ref, inputs }) {
      await request("POST", `/repos/${repo}/actions/workflows/${workflow}/dispatches`, { ref, inputs });
    },
  };
}

// Deterministic offline client driven by a fixture JSON:
//   { "refReads":  [sha, ...],           // consumed per getRefSha; repeats last
//     "listCalls": [[[run...]], ...],    // one entry per listRuns call; each is
//                                        // an array of pages; repeats last
//     "dispatchCalls": [{"error": {...}} | {}],
//     "dispatchError": {...},            // shorthand for a single failing call
//     "refError": {...}, "listError": {...},
//     "truncated": false }
export function createFixtureClient(fixture) {
  const calls = { refReads: 0, listCalls: 0, dispatches: [] };
  const refReads = fixture.refReads?.length ? fixture.refReads : [fixture.mainSha ?? null];
  const listCalls = fixture.listCalls?.length ? fixture.listCalls : [fixture.runs ? [fixture.runs] : [[]]];
  const dispatchCalls = fixture.dispatchCalls ?? (fixture.dispatchError ? [{ error: fixture.dispatchError }] : []);
  return {
    calls,
    async getRefSha() {
      if (fixture.refError) throw new ApiError(`fixture refError: ${fixture.refError.message ?? "injected"}`);
      const sha = refReads[Math.min(calls.refReads++, refReads.length - 1)];
      if (typeof sha !== "string" || !SHA_RE.test(sha)) {
        throw new ApiError(`fixture refRead ${calls.refReads - 1} is not a 40-hex sha`);
      }
      return sha;
    },
    async listRuns() {
      if (fixture.listError) throw new ApiError(`fixture listError: ${fixture.listError.message ?? "injected"}`);
      const group = listCalls[Math.min(calls.listCalls++, listCalls.length - 1)];
      return { runs: group.flat(), truncated: fixture.truncated === true };
    },
    async dispatch({ ref, inputs }) {
      const entry = dispatchCalls.length
        ? dispatchCalls[Math.min(calls.dispatches.length, dispatchCalls.length - 1)]
        : null;
      calls.dispatches.push({ ref, inputs });
      if (entry?.error) throw new ApiError(`fixture dispatchError: ${entry.error.message ?? "injected"}`, entry.error.status ?? null);
    },
  };
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

function attemptRow(r) {
  return {
    id: r.id ?? null,
    event: r.event ?? null,
    status: r.status ?? null,
    conclusion: r.conclusion ?? null,
    runAttempt: r.run_attempt ?? null,
    headSha: r.head_sha ?? null,
    createdAt: r.created_at ?? null,
    actor: actorOf(r),
    url: r.html_url ?? null,
  };
}

// Executes the full scheduler decision flow and returns the outcome plus
// the report object. Never throws for expected conditions — API/fixture
// failures become the `error` outcome.
export async function execute(cfg) {
  const warnings = [];
  const report = {
    schemaVersion: 1,
    kind: "daily-ci-decision",
    repository: cfg.repo ?? null,
    workflow: cfg.workflow,
    mainRef: cfg.mainRef,
    observedShas: [],
    decisionSha: null,
    outcome: null,
    reason: null,
    latestVerified: null,
    attemptsOnDecisionSha: [],
    autoDispatchCount: null,
    maxAutoDispatches: cfg.maxAutoDispatches,
    historyTruncated: false,
    dispatch: null,
    warnings,
    generatedAt: cfg.now(),
  };
  const done = (outcome, reason, extra = {}) => {
    report.outcome = outcome;
    report.reason = reason;
    return { outcome, exitCode: 0, report, warnings, ...extra };
  };

  try {
    let sha = await cfg.client.getRefSha(cfg.mainRef);
    report.observedShas.push(sha);
    let listed = await cfg.client.listRuns({ workflow: cfg.workflow, branch: cfg.mainRef, maxPages: cfg.maxPages });
    let d = decide({
      mainSha: sha,
      runs: listed.runs,
      mainRef: cfg.mainRef,
      autoActors: cfg.autoActors,
      maxAutoDispatches: cfg.maxAutoDispatches,
    });

    const recordDecision = () => {
      report.decisionSha = sha;
      if (d.latestVerified != null) {
        report.latestVerified = {
          headSha: d.latestVerified.head_sha,
          updatedAt: d.latestVerified.updated_at ?? null,
          url: d.latestVerified.html_url ?? null,
        };
      }
      report.attemptsOnDecisionSha = (d.attempts ?? []).slice(0, MAX_ATTEMPTS_IN_REPORT).map(attemptRow);
      report.autoDispatchCount = d.autoDispatches.length;
    };
    // Truncated history is fail-closed on BOTH sides: unseen older pages
    // could hide auto dispatches (retry-budget undercount) or a rerun on
    // this SHA whose recent activity would contest a success claim. No
    // outcome that asserts "verified" or fires a dispatch is safe on
    // incomplete data — hold, warn, and let ops widen DAILY_MAX_PAGES.
    const truncatedHold = () => {
      report.historyTruncated = true;
      recordDecision();
      warnings.push(`run history truncated at ${cfg.maxPages} page(s); cannot certify the verification state — widen DAILY_MAX_PAGES or re-run manually`);
      return done("history-truncated", `run history truncated at ${cfg.maxPages} page(s); held underlying decision "${d.decision}"`);
    };
    report.historyTruncated = listed.truncated;
    if (listed.truncated) return truncatedHold();

    // Re-check the ref right before dispatching so a moving main never
    // gets a run stamped with a stale expected_sha. Bounded: if main keeps
    // moving we defer to the next run instead of chasing it.
    let rechecks = 0;
    while (d.decision === "dispatch") {
      const current = await cfg.client.getRefSha(cfg.mainRef);
      if (current === sha) break;
      if (report.observedShas[report.observedShas.length - 1] !== current) report.observedShas.push(current);
      if (++rechecks > cfg.maxRechecks) {
        warnings.push(`main moved ${rechecks} time(s) during decision; deferring to next run`);
        return done("main-moved", `main moved past ${sha.slice(0, 12)} during decision`);
      }
      sha = current;
      listed = await cfg.client.listRuns({ workflow: cfg.workflow, branch: cfg.mainRef, maxPages: cfg.maxPages });
      d = decide({
        mainSha: sha,
        runs: listed.runs,
        mainRef: cfg.mainRef,
        autoActors: cfg.autoActors,
        maxAutoDispatches: cfg.maxAutoDispatches,
      });
      if (listed.truncated) return truncatedHold();
    }

    recordDecision();
    if (d.decision !== "dispatch") {
      if (d.decision === "budget-exhausted" || d.decision === "hold-unknown") warnings.push(d.reason);
      return done(d.decision, d.reason);
    }
    if (cfg.dryRun) {
      return done("dry-run", `would dispatch ${cfg.workflow} on ${cfg.mainRef} with expected_sha=${sha} (${d.reason})`);
    }

    // Dispatch, then confirm a new workflow_dispatch run actually appeared.
    // The dispatch POST returns 204 with no run id, so the new run is
    // identified as a workflow_dispatch run on mainRef whose id was not in
    // the pre-dispatch set. A simultaneous manual dispatch on the same SHA
    // is indistinguishable — harmless: it is still "a dispatch for this
    // SHA" and its landed head_sha is reported.
    const preDispatchIds = new Set(
      (listed.runs ?? [])
        .filter((r) => r.event === "workflow_dispatch" && r.head_branch === cfg.mainRef)
        .map((r) => r.id),
    );
    const requestedAt = cfg.now();
    await cfg.client.dispatch({ workflow: cfg.workflow, ref: cfg.mainRef, inputs: { expected_sha: sha } });
    report.dispatch = { requestedAt, expectedSha: sha, ref: cfg.mainRef, confirmedRun: null };

    const maxPolls = Math.max(1, Math.ceil(cfg.confirmTimeoutMs / cfg.confirmIntervalMs));
    let confirmed = null;
    for (let poll = 0; poll < maxPolls && confirmed == null; poll++) {
      await cfg.sleep(cfg.confirmIntervalMs);
      const relisted = await cfg.client.listRuns({ workflow: cfg.workflow, branch: cfg.mainRef, maxPages: cfg.maxPages });
      confirmed =
        relisted.runs.find(
          (r) => r.event === "workflow_dispatch" && r.head_branch === cfg.mainRef && !preDispatchIds.has(r.id),
        ) ?? null;
    }

    if (confirmed == null) {
      warnings.push(`dispatch accepted but no new run observed within ${cfg.confirmTimeoutMs}ms (eventual consistency); next scheduled run re-evaluates`);
      return done("dispatched-unconfirmed", `dispatched expected_sha=${sha.slice(0, 12)} but run not observed yet`);
    }
    report.dispatch.confirmedRun = attemptRow(confirmed);
    if (confirmed.head_sha !== sha) {
      warnings.push(
        `dispatched run ${confirmed.id} landed on ${confirmed.head_sha} not ${sha}; ci-summary's expected_sha check fails that run — the expected SHA is NOT verified`,
      );
      return done(
        "dispatch-landed-mismatch",
        `dispatched run landed on ${confirmed.head_sha.slice(0, 12)} != expected ${sha.slice(0, 12)}`,
      );
    }
    return done("dispatch-confirmed", `dispatched run ${confirmed.id} on ${sha.slice(0, 12)}`);
  } catch (e) {
    report.outcome = "error";
    report.reason = e.message;
    report.error = { message: e.message, status: e.status ?? null, body: e.body ?? null };
    return { outcome: "error", exitCode: 1, report, warnings };
  }
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

function fmtSha(sha) {
  return sha == null ? "-" : `\`${sha.slice(0, 12)}\``;
}

export function markdownSummary(report) {
  const lines = [
    `## daily-ci outcome: ${report.outcome}`,
    "",
    `- reason: \`${report.reason ?? "-"}\``,
    `- decision sha: ${fmtSha(report.decisionSha)}  observed: ${report.observedShas.map((s) => `\`${s.slice(0, 12)}\``).join(", ") || "-"}`,
    `- latest verified: ${fmtSha(report.latestVerified?.headSha)} @ ${report.latestVerified?.updatedAt ?? "-"}${report.latestVerified?.url ? `  (${report.latestVerified.url})` : ""}`,
    `- auto dispatches on decision sha: ${report.autoDispatchCount ?? "-"} / ${report.maxAutoDispatches}`,
    `- history truncated: ${report.historyTruncated}`,
  ];
  if (report.dispatch != null) {
    const c = report.dispatch.confirmedRun;
    lines.push(
      `- dispatch: expected ${fmtSha(report.dispatch.expectedSha)} on \`${report.dispatch.ref}\` @ ${report.dispatch.requestedAt}`,
      c == null
        ? `- confirmed run: not observed within poll window`
        : `- confirmed run: [${c.id}](${c.url})  landed ${fmtSha(c.headSha)}  status \`${c.status}\`/\`${c.conclusion}\`  attempt ${c.runAttempt ?? "-"}`,
    );
  }
  if (report.attemptsOnDecisionSha.length > 0) {
    lines.push(
      "",
      "| run | event | status | conclusion | attempt | created_at | actor | url |",
      "| --- | --- | --- | --- | --- | --- | --- | --- |",
      ...report.attemptsOnDecisionSha.map(
        (r) =>
          `| ${r.id ?? "-"} | ${r.event ?? "-"} | ${r.status ?? "-"} | ${r.conclusion ?? "-"} | ${r.runAttempt ?? "-"} | ${r.createdAt ?? "-"} | ${r.actor ?? "-"} | ${r.url ?? "-"} |`,
      ),
    );
  }
  if (report.warnings.length > 0) {
    lines.push("", "### warnings", "", ...report.warnings.map((w) => `- ${w}`));
  }
  lines.push("");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function parsePositiveInt(value, flag) {
  const n = Number.parseInt(value, 10);
  if (!Number.isFinite(n) || n < 0) throw new Error(`invalid value for ${flag}: ${value}`);
  return n;
}

export function resolveConfig(argv = process.argv, env = process.env) {
  const cfg = {
    repo: env.GITHUB_REPOSITORY ?? null,
    token: env.GITHUB_TOKEN ?? null,
    apiBase: env.DAILY_API_BASE ?? "https://api.github.com",
    workflow: env.DAILY_TARGET_WORKFLOW ?? "ci.yml",
    mainRef: env.DAILY_MAIN_REF ?? "main",
    maxAutoDispatches: 2,
    maxPages: 10,
    maxRechecks: 2,
    autoActors: (env.DAILY_AUTO_ACTORS ?? "github-actions[bot]").split(",").map((s) => s.trim()).filter(Boolean),
    confirmTimeoutMs: 120_000,
    confirmIntervalMs: 15_000,
    reportPath: env.DAILY_REPORT ?? "daily-ci-report.json",
    summaryPath: env.GITHUB_STEP_SUMMARY ?? null,
    fixture: null,
    dryRun: env.DRY_RUN === "true" || env.DRY_RUN === "1",
  };
  if (env.DAILY_MAX_AUTO_DISPATCHES != null) cfg.maxAutoDispatches = parsePositiveInt(env.DAILY_MAX_AUTO_DISPATCHES, "DAILY_MAX_AUTO_DISPATCHES");
  if (env.DAILY_MAX_PAGES != null) cfg.maxPages = parsePositiveInt(env.DAILY_MAX_PAGES, "DAILY_MAX_PAGES");
  if (env.DAILY_MAX_RECHECKS != null) cfg.maxRechecks = parsePositiveInt(env.DAILY_MAX_RECHECKS, "DAILY_MAX_RECHECKS");
  if (env.DAILY_CONFIRM_TIMEOUT_MS != null) cfg.confirmTimeoutMs = parsePositiveInt(env.DAILY_CONFIRM_TIMEOUT_MS, "DAILY_CONFIRM_TIMEOUT_MS");
  if (env.DAILY_CONFIRM_INTERVAL_MS != null) cfg.confirmIntervalMs = parsePositiveInt(env.DAILY_CONFIRM_INTERVAL_MS, "DAILY_CONFIRM_INTERVAL_MS");

  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--workflow") cfg.workflow = next();
    else if (a === "--main-ref") cfg.mainRef = next();
    else if (a === "--api-base") cfg.apiBase = next();
    else if (a === "--repo") cfg.repo = next();
    else if (a === "--max-auto-dispatches") cfg.maxAutoDispatches = parsePositiveInt(next(), a);
    else if (a === "--max-pages") cfg.maxPages = parsePositiveInt(next(), a);
    else if (a === "--max-rechecks") cfg.maxRechecks = parsePositiveInt(next(), a);
    else if (a === "--auto-actors") cfg.autoActors = next().split(",").map((s) => s.trim()).filter(Boolean);
    else if (a === "--confirm-timeout-ms") cfg.confirmTimeoutMs = parsePositiveInt(next(), a);
    else if (a === "--confirm-interval-ms") cfg.confirmIntervalMs = parsePositiveInt(next(), a);
    else if (a === "--report") cfg.reportPath = next();
    else if (a === "--summary") cfg.summaryPath = next();
    else if (a === "--fixture") cfg.fixture = next();
    else if (a === "--dry-run") cfg.dryRun = true;
    else if (a === "--help" || a === "-h") {
      return { help: true };
    } else {
      throw new Error(`unknown argument ${a}`);
    }
  }
  return cfg;
}

export async function main(argv = process.argv, env = process.env) {
  let cfg;
  try {
    cfg = resolveConfig(argv, env);
  } catch (e) {
    console.error(`error: ${e.message}`);
    process.exit(2);
    return;
  }
  if (cfg.help) {
    console.log(
      "Usage: node ci/daily/decide.mjs [--workflow ci.yml] [--main-ref main] [--api-base URL] [--repo owner/repo] [--max-auto-dispatches N] [--max-pages N] [--max-rechecks N] [--auto-actors a,b] [--confirm-timeout-ms N] [--confirm-interval-ms N] [--report FILE] [--summary FILE] [--fixture FILE] [--dry-run]",
    );
    process.exit(0);
    return;
  }

  let client;
  let sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  if (cfg.fixture != null) {
    let fixture;
    try {
      fixture = JSON.parse(readFileSync(cfg.fixture, "utf8"));
    } catch (e) {
      console.error(`error: cannot read fixture ${cfg.fixture}: ${e.message}`);
      process.exit(2);
      return;
    }
    client = createFixtureClient(fixture);
    sleep = async () => {}; // fixture polls are instant
  } else {
    if (!cfg.token) {
      console.error("error: GITHUB_TOKEN is required (or use --fixture/--dry-run for offline evaluation)");
      process.exit(2);
      return;
    }
    if (!cfg.repo) {
      console.error("error: GITHUB_REPOSITORY is required (or pass --repo owner/repo)");
      process.exit(2);
      return;
    }
    client = createGitHubClient({ token: cfg.token, repo: cfg.repo, apiBase: cfg.apiBase });
  }

  const { exitCode, report, warnings } = await execute({
    client,
    repo: cfg.repo ?? "fixture/fixture",
    workflow: cfg.workflow,
    mainRef: cfg.mainRef,
    autoActors: cfg.autoActors,
    maxAutoDispatches: cfg.maxAutoDispatches,
    maxPages: cfg.maxPages,
    maxRechecks: cfg.maxRechecks,
    confirmTimeoutMs: cfg.confirmTimeoutMs,
    confirmIntervalMs: cfg.confirmIntervalMs,
    dryRun: cfg.dryRun,
    sleep,
    now: () => new Date().toISOString(),
  });

  for (const w of warnings) console.log(`::warning::${w}`);
  const table = markdownSummary(report);
  process.stdout.write(table + "\n");
  if (cfg.reportPath) writeFileSync(cfg.reportPath, JSON.stringify(report, null, 2) + "\n");
  const summaryPath = cfg.summaryPath;
  if (summaryPath) appendFileSync(summaryPath, table + "\n");
  process.exit(exitCode);
}

const isMain = process.argv[1] != null && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  await main();
}
