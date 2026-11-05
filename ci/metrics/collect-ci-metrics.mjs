#!/usr/bin/env node
// collect-ci-metrics.mjs — Issue #327 (CI-OPT-00) cross-repo CI baseline collector.
//
// Canonical measurement contract: docs/designs/issue-327-cross-repo-ci-baseline.md
// Data source: GitHub Actions REST API via ambient `gh` CLI (read-only).
// Stdlib-only. No npm dependencies.
//
// Usage:
//   node ci/metrics/collect-ci-metrics.mjs \
//     [--repo owner/name ...] [--runs N] [--out DIR] \
//     [--logs-run RUN_ID ...] [--workflow NAME]
//
// Defaults: repos = scarecrow173/{ruviform,audio-lens,synvelis}, runs = 20 per
// workflow, out = ./ci-metrics-out. --logs-run enables job-log cache analysis
// for specific run ids only (rate-limit conscious).
//
// Output: <out>/runs.jsonl (raw evidence: one JSON object per line, types
//   run | job | step | cache | required-contexts | repo-meta)
//         <out>/report.md  (derived tables; every number re-derivable from JSONL)

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// ---------- args ----------
const args = process.argv.slice(2);
if (args.includes("--help") || args.includes("-h")) {
  console.log(`Usage: node ci/metrics/collect-ci-metrics.mjs [options]

  --repo owner/name   target repo (repeatable; default: ruviform,audio-lens,synvelis)
  --runs N            runs per workflow to sample (default: 20)
  --workflow NAME     restrict to one workflow name
  --logs-run RUN_ID   also fetch job logs for this run id and parse cache
                      Hit/Miss/transfer size (repeatable; rate-limit heavy)
  --out DIR           output directory (default: ./ci-metrics-out)

Outputs <out>/runs.jsonl (raw records) and <out>/report.md (derived tables).`);
  process.exit(0);
}
const KNOWN_FLAGS = new Set(["--repo", "--runs", "--out", "--logs-run", "--workflow"]);
for (const a of args) {
  const flag = a.split("=", 1)[0];
  if (flag.startsWith("-") && !KNOWN_FLAGS.has(flag)) {
    console.error(`error: unknown flag ${flag} (see --help)`);
    process.exit(2);
  }
}
function argValues(flag) {
  const out = [];
  for (let i = 0; i < args.length; i++) {
    if (args[i] === flag && i + 1 < args.length) out.push(args[++i]);
    else if (args[i].startsWith(flag + "=")) out.push(args[i].slice(flag.length + 1));
  }
  return out;
}
const REPOS = argValues("--repo").length
  ? argValues("--repo")
  : ["scarecrow173/ruviform", "scarecrow173/audio-lens", "scarecrow173/synvelis"];
const RUNS_PER_WORKFLOW = Number(argValues("--runs")[0] ?? "20");
const OUT_DIR = argValues("--out")[0] ?? "ci-metrics-out";
const LOG_RUNS = new Set(argValues("--logs-run").map(String));
const WORKFLOW_FILTER = argValues("--workflow")[0] ?? null;

// ---------- gh api ----------
function ghJson(path) {
  try {
    const out = execFileSync("gh", ["api", path], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
    return JSON.parse(out);
  } catch (e) {
    const body = (e.stdout ?? "") + (e.stderr ?? "");
    const m = body.match(/HTTP (\d+)/);
    return { __error: true, status: m ? Number(m[1]) : null, message: body.slice(0, 300) };
  }
}
function ghText(path) {
  try {
    return execFileSync("gh", ["api", path], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  } catch {
    return null;
  }
}
function ghRaw(endpoint, path) {
  return ghJson(`${endpoint}?per_page=100${path ? "&" + path : ""}`);
}

// ---------- time ----------
const t = (s) => (s ? Date.parse(s) / 1000 : null);
const dur = (a, b) => (a != null && b != null ? Math.max(0, b - a) : null);

// ---------- lane classification (design doc normative table) ----------
const JOB_LANES = [
  [/^verify\b/, "rust-verify"],
  [/^ui-verify\b/, "ui-verify"],
  [/^webview-verify\b/, "webview-verify"],
  [/^l1-real-verify\b/, "l1-real-verify"],
  [/^gw10-verify\b/, "gw10-verify"],
  [/^rt-verify\b/, "rt-verify"],
  [/^msrv-verify\b/, "msrv-verify"],
  [/^icc-verify\b/, "icc-verify"],
  [/^bench\b/, "bench"],
  [/^dep-guard\b/, "dep-guard"],
  [/^gui-real\b/, "gui-real"],
  [/^(ruviform-access|preflight)\b/, "preflight"],
  [/^(evidence|summary|ci-summary)\b/, "evidence"],
];
function jobLane(name, workflowName) {
  if (workflowName === "consumer-impact-check") return "consumer-impact";
  if (workflowName === "consumer-verify") return "consumer-verify";
  const base = name.replace(/\s*\([^)]*\)\s*$/, "");
  for (const [re, lane] of JOB_LANES) if (re.test(base)) return lane;
  return "other";
}
const STEP_LANES = [
  [/^Set up job$|^Complete job$|^Post /i, "runner-overhead"],
  [/^Checkout\b|^Clone /i, "checkout"],
  [/cache/i, "cache-restore"],
  [/^Set up mise|Restore .*mise|mise-action|Install mise|Install .*toolchain|rustup/i, "mise"],
  [/npm ci|npm install|cargo fetch|Restore CI-required tools|Fetch deps|Fetch locked dependencies|Install .*headers|Install WebKitGTK|Install ALSA|Install SSH credential/i, "dep-fetch"],
  [/playwright/i, "playwright"],
  [/mise run (fmt|check|lint|test|build|verify)|cargo (check|clippy|test|build|fmt)|check-ci lane|smoke lane|check lane|Build headless CLI/i, "rust"],
  [/mise run \S*-ui-|tsc|node --test|npm --prefix/i, "ui"],
  [/summary|evidence|Upload|gate on|Guard|assert|Enforce|Require non-empty|not-run-is-not-pass|^Verify |Validate|Record real-GUI gate|Owner triage/i, "evidence"],
  [/mise run|Run |deploy-key|access|LFS|git |prek|config files|Cleanup credential|Measure disk|Pin ruviform/i, "verify-other"],
];
function stepLane(name) {
  for (const [re, lane] of STEP_LANES) if (re.test(name)) return lane;
  return "other";
}

// ---------- minimal YAML needs extraction ----------
// Parses `jobs:` top-level block: 2-space job ids, `needs:` within job block.
// Handles `needs: x` and `needs: [a, b]` only; reusable-workflow/dynamic cases
// are reported as unknown per fail-closed contract.
function parseJobNeeds(yaml) {
  const needs = {};
  const lines = yaml.split("\n");
  let inJobs = false;
  let cur = null;
  for (const line of lines) {
    if (/^jobs:\s*$/.test(line)) {
      inJobs = true;
      continue;
    }
    if (!inJobs) continue;
    if (/^\S/.test(line) && !/^jobs:/.test(line)) break; // left jobs block
    const jm = line.match(/^ {2}([A-Za-z0-9_-]+):\s*$/);
    if (jm) {
      cur = jm[1];
      needs[cur] = [];
      continue;
    }
    if (cur) {
      const nm = line.match(/^\s+needs:\s*(.+)$/);
      if (nm) {
        const v = nm[1].trim();
        if (v.startsWith("[")) needs[cur] = v.replace(/[[\]\s]/g, "").split(",").filter(Boolean);
        else needs[cur] = [v];
      }
    }
  }
  return needs;
}

// ---------- cache log parsing ----------
const CACHE_PATTERNS = {
  hit: [/Cache hit for restore-key:\s*(\S+)/, /Cache restored from key:\s*(\S+)/],
  miss: [/Cache not found for input keys:\s*(.*)/, /cache not found for\s*(\S*)/i, /Failed to restore/i],
  size: [/Cache Size:\s*~?[\d.]+\s*[A-Z]+\s*\((\d+)\s*B\)/],
  saved: [/Cache saved with key:\s*(\S+)/],
};
function analyzeJobLog(text) {
  const events = [];
  if (!text) return events;
  const tryKind = (line, patterns, kind, map) => {
    for (const re of patterns) {
      const m = line.match(re);
      if (m) {
        events.push(map(m));
        return;
      }
    }
  };
  for (const line of text.split("\n")) {
    tryKind(line, CACHE_PATTERNS.hit, "hit", (m) => ({ kind: "hit", key: m[1] }));
    tryKind(line, CACHE_PATTERNS.miss, "miss", (m) => ({ kind: "miss", key: (m[1] || "").trim() || null }));
    tryKind(line, CACHE_PATTERNS.size, "size-bytes", (m) => ({ kind: "size-bytes", bytes: Number(m[1]) }));
    tryKind(line, CACHE_PATTERNS.saved, "saved", (m) => ({ kind: "saved", key: m[1] }));
  }
  return events;
}

// ---------- critical path ----------
// needs DAG weighted longest path over job_exec_s. Unknown refs -> warn + null.
function criticalPath(jobs, needs) {
  const byId = {};
  for (const j of jobs) {
    const id = j.job_id;
    if (!byId[id]) byId[id] = [];
    byId[id].push(j);
  }
  const exec = (id) => Math.max(...(byId[id] || []).map((j) => j.job_exec_s ?? 0));
  const memo = {};
  const visiting = new Set();
  const unresolved = [];
  function cost(id) {
    if (id in memo) return memo[id];
    if (visiting.has(id)) {
      unresolved.push(`cycle:${id}`);
      return 0;
    }
    visiting.add(id);
    const deps = needs[id] || [];
    let base = 0;
    for (const d of deps) {
      if (!(d in needs)) {
        unresolved.push(`unknown-need:${d}<-${id}`);
        continue;
      }
      base = Math.max(base, cost(d));
    }
    visiting.delete(id);
    memo[id] = base + exec(id);
    return memo[id];
  }
  const ids = Object.keys(needs);
  const best = ids.length ? Math.max(...ids.map(cost)) : null;
  return { critical_path_s: best, warnings: unresolved };
}

// ---------- stats ----------
function percentile(sorted, p) {
  if (!sorted.length) return null;
  const idx = Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1));
  return sorted[idx];
}
function statsFor(values) {
  const v = values.filter((x) => x != null).sort((a, b) => a - b);
  if (!v.length) return { n: 0 };
  const out = { n: v.length, min: v[0], max: v[v.length - 1], values: v };
  if (v.length >= 10) {
    out.p50 = percentile(v, 50);
    out.p95 = percentile(v, 95);
  }
  return out;
}

// ---------- main ----------
const jsonl = [];
const emit = (obj) => jsonl.push(JSON.stringify(obj));
const report = { generated_at: new Date().toISOString(), repos: {} };
const otherNames = new Set();

for (const repo of REPOS) {
  const [owner, name] = repo.split("/");
  const rmeta = ghJson(`repos/${repo}`);
  if (rmeta.__error) {
    emit({ type: "repo-meta", repo, error: rmeta });
    report.repos[repo] = { error: rmeta.message };
    continue;
  }
  emit({ type: "repo-meta", repo, visibility: rmeta.visibility, default_branch: rmeta.default_branch });

  // required contexts (fail-closed: distinguish 404 from empty)
  const prot = ghJson(`repos/${repo}/branches/${rmeta.default_branch}/protection`);
  const rulesets = ghJson(`repos/${repo}/rulesets?includes_parents=true`);
  const req = {
    type: "required-contexts",
    repo,
    branch_protection: prot.__error ? { status: prot.status, note: prot.status === 404 ? "not protected" : "error" } : {
      strict: prot.required_status_checks?.strict ?? null,
      contexts: prot.required_status_checks?.contexts ?? [],
    },
    rulesets: Array.isArray(rulesets) ? rulesets.map((r) => ({ id: r.id, name: r.name, enforcement: r.enforcement })) : { error: rulesets },
  };
  emit(req);

  const wfs = ghJson(`repos/${repo}/actions/workflows`);
  const workflows = (wfs.workflows ?? []).filter((w) => w.state === "active" && (!WORKFLOW_FILTER || w.name === WORKFLOW_FILTER));
  const repoOut = { visibility: rmeta.visibility, required: req, workflows: {}, runStats: {} };

  for (const wf of workflows) {
    const runsResp = ghJson(`repos/${repo}/actions/workflows/${wf.id}/runs?per_page=${RUNS_PER_WORKFLOW}`);
    const runs = runsResp.workflow_runs ?? [];
    repoOut.workflows[wf.name] = { total: runsResp.total_count ?? runs.length, sampled: runs.length };
    // workflow YAML for needs (at default branch — documented approximation)
    let needs = {};
    const contents = ghJson(`repos/${repo}/contents/${encodeURIComponent(wf.path)}?ref=${rmeta.default_branch}`);
    if (contents.content) {
      needs = parseJobNeeds(Buffer.from(contents.content, "base64").toString("utf8"));
    }
    for (const run of runs) {
      const detail = ghJson(`repos/${repo}/actions/runs/${run.id}`);
      const jobsResp = ghRaw(`repos/${repo}/actions/runs/${run.id}/jobs`, "filter=all");
      const jobs = jobsResp.jobs ?? [];
      const runRec = {
        type: "run",
        repo,
        workflow: wf.name,
        id: run.id,
        event: run.event,
        conclusion: run.conclusion,
        status: run.status,
        run_attempt: detail.run_attempt ?? run.run_attempt,
        head_sha: run.head_sha,
        head_branch: run.head_branch,
        created_at: run.created_at,
        run_started_at: detail.run_started_at ?? null,
        updated_at: run.updated_at,
        workflow_wall_time_s: dur(t(run.created_at), t(run.updated_at)),
        workflow_queue_s: dur(t(run.created_at), t(detail.run_started_at)),
      };
      emit(runRec);
      let totalRunner = 0;
      let minStart = null;
      let maxEnd = null;
      const jobRecs = [];
      for (const job of jobs) {
        const base = job.name.replace(/\s*\([^)]*\)\s*$/, "");
        const jobRec = {
          type: "job",
          repo,
          workflow: wf.name,
          run_id: run.id,
          job_id: base,
          name: job.name,
          lane: jobLane(job.name, wf.name),
          labels: job.labels,
          runner_name: job.runner_name,
          conclusion: job.conclusion,
          status: job.status,
          started_at: job.started_at,
          completed_at: job.completed_at,
          job_wait_s: dur(t(detail.run_started_at), t(job.started_at)),
          job_exec_s: dur(t(job.started_at), t(job.completed_at)),
        };
        emit(jobRec);
        jobRecs.push(jobRec);
        if (jobRec.lane === "other") otherNames.add(`${repo}/${wf.name}/${job.name}`);
        if (jobRec.job_exec_s != null) totalRunner += jobRec.job_exec_s;
        if (job.started_at) minStart = minStart == null ? t(job.started_at) : Math.min(minStart, t(job.started_at));
        if (job.completed_at) maxEnd = maxEnd == null ? t(job.completed_at) : Math.max(maxEnd, t(job.completed_at));
        for (const step of job.steps ?? []) {
          const sRec = {
            type: "step",
            repo,
            run_id: run.id,
            job_name: job.name,
            number: step.number,
            name: step.name,
            lane: stepLane(step.name),
            conclusion: step.conclusion,
            step_s: step.conclusion === "skipped" ? 0 : dur(t(step.started_at), t(step.completed_at)),
          };
          emit(sRec);
          if (sRec.lane === "other") otherNames.add(`step:${repo}/${job.name}/${step.name}`);
        }
        if (LOG_RUNS.has(String(run.id))) {
          const log = ghText(`repos/${repo}/actions/jobs/${job.id}/logs`);
          const events = analyzeJobLog(log ?? "");
          const rec = {
            type: "cache",
            repo,
            run_id: run.id,
            job_name: job.name,
            log_fetched: log != null,
            events,
            verdict:
              log == null ? "unknown(no-log)" :
              events.some((e) => e.kind === "hit") ? "hit" :
              events.some((e) => e.kind === "miss") ? "miss" :
              events.length ? "saved-or-size-only" : "unknown(no-cache-lines)",
          };
          emit(rec);
          ((repoOut.details ??= {})[run.id] ??= { caches: [] }).caches ??= [];
          repoOut.details[run.id].caches.push(rec);
        }
      }
      const cp = criticalPath(jobRecs, needs);
      const laneTotals = {};
      for (const j of jobRecs) {
        laneTotals[j.lane] = (laneTotals[j.lane] ?? 0) + (j.job_exec_s ?? 0);
      }
      const stepLaneTotals = {};
      for (const job of jobs) {
        for (const step of job.steps ?? []) {
          const sl = stepLane(step.name);
          const sv = step.conclusion === "skipped" ? 0 : dur(t(step.started_at), t(step.completed_at)) ?? 0;
          stepLaneTotals[sl] = (stepLaneTotals[sl] ?? 0) + sv;
        }
      }
      const rm = {
        type: "run-metrics",
        repo,
        workflow: wf.name,
        run_id: run.id,
        event: run.event,
        conclusion: run.conclusion,
        exec_span_s: dur(minStart, maxEnd),
        total_runner_time_s: totalRunner,
        critical_path_s: cp.critical_path_s,
        critical_path_warnings: cp.warnings,
        jobs: jobRecs.length,
        lane_totals: laneTotals,
        step_lane_totals: stepLaneTotals,
      };
      emit(rm);
      const prev = (repoOut.details ??= {})[run.id] ?? {};
      repoOut.details[run.id] = { ...prev, run: runRec, metrics: rm, jobs: jobRecs };
    }
    // aggregate stats: success & attempt 1 only
    const ok = runs.filter((r) => r.conclusion === "success" && (r.run_attempt ?? 1) === 1);
    const walls = ok.map((r) => dur(t(r.created_at), t(r.updated_at)));
    repoOut.runStats[wf.name] = {
      total: runsResp.total_count ?? runs.length,
      sampled: runs.length,
      success_attempt1: ok.length,
      wall_time: statsFor(walls),
      by_event: Object.fromEntries(
        [...new Set(runs.map((r) => r.event))].map((e) => [e, runs.filter((r) => r.event === e).length]),
      ),
      by_conclusion: Object.fromEntries(
        [...new Set(runs.map((r) => r.conclusion))].map((c) => [c, runs.filter((r) => r.conclusion === c).length]),
      ),
    };
  }
  report.repos[repo] = repoOut;
}

mkdirSync(OUT_DIR, { recursive: true });
writeFileSync(join(OUT_DIR, "runs.jsonl"), jsonl.join("\n") + "\n");

// ---------- report.md ----------
const L = [];
L.push("# CI Baseline Metrics Report");
L.push("");
L.push(`generated: ${report.generated_at}`);
L.push(`repos: ${REPOS.join(", ")}`);
L.push(`runs per workflow: up to ${RUNS_PER_WORKFLOW}`);
L.push(`cache log analysis: ${LOG_RUNS.size ? `run ids ${[...LOG_RUNS].join(", ")}` : "disabled (cache = unknown)"}`);
L.push(`contract: docs/designs/issue-327-cross-repo-ci-baseline.md`);
L.push("");
for (const repo of REPOS) {
  const r = report.repos[repo];
  if (!r || r.error) {
    L.push(`## ${repo}`, "", `error: ${r?.error ?? "fetch failed"}`, "");
    continue;
  }
  L.push(`## ${repo}`);
  L.push("");
  const prot = r.required?.branch_protection;
  L.push("### Required contexts");
  L.push("");
  if (prot?.contexts) {
    L.push(`- branch_protection strict=${prot.strict} contexts(${prot.contexts.length}):`);
    for (const c of prot.contexts) L.push(`  - \`${c}\``);
  } else {
    L.push(`- branch_protection: ${prot?.note ?? "unknown"}`);
  }
  L.push(`- rulesets: ${Array.isArray(r.required?.rulesets) ? `${r.required.rulesets.length} entries` : "error"}`);
  L.push("");
  for (const [wf, st] of Object.entries(r.runStats)) {
    L.push(`### workflow \`${wf}\`  (total ${st.total}, sampled ${st.sampled}, success&attempt1 ${st.success_attempt1})`);
    L.push("");
    L.push(`- events: ${JSON.stringify(st.by_event)}  conclusions: ${JSON.stringify(st.by_conclusion)}`);
    const w = st.wall_time;
    if (w.n === 0) L.push(`- wall_time: no successful attempt-1 runs in sample`);
    else if (w.p50 != null) L.push(`- wall_time_s: n=${w.n} p50=${w.p50} p95=${w.p95} min=${w.min} max=${w.max}`);
    else L.push(`- wall_time_s: n=${w.n} values=${JSON.stringify(w.values)} (below p50 threshold; raw observations)`);
    L.push("");
    // per-run table for this workflow
    const runIds = Object.keys(r.details ?? {}).filter((id) => r.details[id].run.workflow === wf);
    if (runIds.length) {
      L.push("| run_id | event | concl | attempt | wall_s | queue_s | exec_span_s | critical_path_s | runner_total_s | jobs |");
      L.push("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
      for (const id of runIds) {
        const d = r.details[id];
        const fmt = (v) => (v == null ? "unknown" : v);
        L.push(
          `| ${id} | ${d.run.event} | ${d.run.conclusion} | ${d.run.run_attempt} | ${fmt(d.run.workflow_wall_time_s)} | ${fmt(d.run.workflow_queue_s)} | ${fmt(d.metrics.exec_span_s)} | ${fmt(d.metrics.critical_path_s)} | ${fmt(d.metrics.total_runner_time_s)} | ${d.metrics.jobs} |`,
        );
      }
      L.push("");
      // per-job table for the most recent successful run in this workflow
      const repId = runIds.find((id) => r.details[id].run.conclusion === "success");
      if (repId) {
        const d = r.details[repId];
        L.push(`#### representative run ${repId} — per-job timings`);
        L.push("");
        L.push("| job | lane | wait_s | exec_s | runner |");
        L.push("| --- | --- | --- | --- | --- |");
        for (const j of d.jobs) {
          L.push(`| ${j.name} | ${j.lane} | ${j.job_wait_s ?? "unknown"} | ${j.job_exec_s ?? "unknown"} | ${(j.labels || []).join("/")} |`);
        }
        L.push("");
        L.push(`- lane totals: ${JSON.stringify(d.metrics.lane_totals)}`);
        L.push(`- step lane totals: ${JSON.stringify(d.metrics.step_lane_totals)}`);
        if (d.metrics.critical_path_warnings?.length) {
          L.push(`- critical path warnings: ${JSON.stringify(d.metrics.critical_path_warnings)}`);
        }
        if (d.caches?.length) {
          L.push("- cache analysis (job log):");
          for (const c of d.caches) {
            const hits = c.events.filter((e) => e.kind === "hit").length;
            const misses = c.events.filter((e) => e.kind === "miss").length;
            const bytes = c.events.filter((e) => e.kind === "size-bytes").map((e) => e.bytes);
            L.push(`  - ${c.job_name}: verdict=${c.verdict} hits=${hits} misses=${misses} bytes=${bytes.join(",") || "-"}`);
          }
        }
        L.push("");
      }
    }
  }
}
if (otherNames.size) {
  L.push("## Unclassified (lane=other)");
  L.push("");
  for (const n of [...otherNames].sort()) L.push(`- \`${n}\``);
  L.push("");
}
L.push("## Metric definitions");
L.push("");
L.push("See docs/designs/issue-327-cross-repo-ci-baseline.md (normative). Raw rows in runs.jsonl.");
writeFileSync(join(OUT_DIR, "report.md"), L.join("\n") + "\n");

console.log(`wrote ${join(OUT_DIR, "runs.jsonl")} (${jsonl.length} rows)`);
console.log(`wrote ${join(OUT_DIR, "report.md")}`);
