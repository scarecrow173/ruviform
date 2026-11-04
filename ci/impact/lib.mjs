// lib.mjs — Issue #333 (CI-OPT-06) impact classification machinery.
//
// Stdlib-only (no npm dependencies), mirroring ci/metrics conventions.
// Everything here is a pure-ish function over the repo filesystem + git so
// `node --test` fixtures can exercise the real workspace.
//
// Canonical contract: docs/designs/issue-333-impact-based-ci.md

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { LANES, LANE_IDS, OWNER_RULES, PATH_CLASSES } from "./map.mjs";

// ---------- glob ----------

// Minimal forward-slash glob: `**` spans segments (a `**/` also matches zero
// segments), `*` within one segment, `?` single non-slash char.
export function globToRegExp(pattern) {
  let re = "";
  for (let i = 0; i < pattern.length; i++) {
    const ch = pattern[i];
    if (ch === "*") {
      if (pattern[i + 1] === "*") {
        const eatsSlash = pattern[i + 2] === "/";
        re += eatsSlash ? "(?:.*/)?" : ".*";
        i += eatsSlash ? 2 : 1;
      } else {
        re += "[^/]*";
      }
    } else if (ch === "?") {
      re += "[^/]";
    } else {
      re += /[.+^${}()|[\]\\]/.test(ch) ? `\\${ch}` : ch;
    }
  }
  return new RegExp(`^${re}$`);
}

const STATIC_RULES = PATH_CLASSES.map((r) => ({
  id: r.id,
  lanes: r.lanes,
  res: r.patterns.map(globToRegExp),
}));

export function toPosixPath(p) {
  return p.split(sep).join("/").replace(/^\.\//, "");
}

// ---------- git ----------

export function gitDiffFiles(root, base, head) {
  const out = execFileSync("git", ["diff", "--name-only", base, head], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  return out
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean)
    .map(toPosixPath);
}

// ---------- cargo workspace ----------

// Returns { ok, reason?, members: Map<name, dirPosix>, reverseDeps: Map<name, Set<name>> }
export function buildCrateGraph(root) {
  let meta;
  try {
    const out = execFileSync(
      "cargo",
      ["metadata", "--no-deps", "--format-version", "1"],
      { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
    );
    meta = JSON.parse(out);
  } catch {
    return { ok: false, reason: "cargo-metadata-failed" };
  }
  const memberIds = new Set(meta.workspace_members);
  const pkgs = meta.packages.filter((p) => memberIds.has(p.id));
  const names = new Set(pkgs.map((p) => p.name));
  const members = new Map();
  const reverseDeps = new Map();
  for (const p of pkgs) {
    const dir = toPosixPath(relative(root, dirname(p.manifest_path)));
    members.set(p.name, dir);
    for (const dep of p.dependencies) {
      // Member edges = path deps onto workspace members (any dep kind:
      // normal/dev/build all matter for verification coverage).
      if (dep.source === null && names.has(dep.name)) {
        if (!reverseDeps.has(dep.name)) reverseDeps.set(dep.name, new Set());
        reverseDeps.get(dep.name).add(p.name);
      }
    }
  }
  if (members.size === 0) return { ok: false, reason: "cargo-metadata-empty" };
  return { ok: true, members, reverseDeps };
}

export function transitiveClosure(roots, reverseDeps) {
  const seen = new Set(roots);
  const queue = [...roots];
  while (queue.length) {
    const cur = queue.shift();
    for (const up of reverseDeps.get(cur) ?? []) {
      if (!seen.has(up)) {
        seen.add(up);
        queue.push(up);
      }
    }
  }
  return seen;
}

// ---------- rt_ test scan ----------

function* walkFiles(dir) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return;
  }
  for (const e of entries) {
    const p = join(dir, e);
    let st;
    try {
      st = statSync(p);
    } catch {
      continue;
    }
    if (st.isDirectory()) {
      if (e !== "node_modules" && e !== "target" && e !== "dist") yield* walkFiles(p);
    } else {
      yield p;
    }
  }
}

// Conservative: any `fn rt_` token under tests/src/benches/examples marks the
// crate an rt_ contract-test holder (false hits only widen coverage).
export function rtTestCrates(root, members) {
  const out = new Set();
  for (const [name, dir] of members) {
    for (const sub of ["tests", "src", "benches", "examples"]) {
      const base = join(root, dir, sub);
      if (!existsSync(base)) continue;
      for (const f of walkFiles(base)) {
        if (!f.endsWith(".rs")) continue;
        if (readFileSync(f, "utf8").includes("fn rt_")) {
          out.add(name);
          break;
        }
      }
      if (out.has(name)) break;
    }
  }
  return out;
}

// ---------- mise task parsing ----------

// Extract `[tasks.<name>]` blocks; body = text until the next `[` header.
export function miseTaskBodies(root) {
  const text = readFileSync(join(root, "mise.toml"), "utf8");
  const bodies = new Map();
  for (const m of text.matchAll(/\[tasks\.([^\]\n]+)\]([^[]*)/g)) {
    bodies.set(m[1].trim(), m[2]);
  }
  return bodies;
}

const CARGO_P_ARG = /(?:^|\s)-p\s+([A-Za-z0-9_-]+)/g;

export function cargoPkgsOfTask(body) {
  const out = new Set();
  for (const m of body.matchAll(CARGO_P_ARG)) out.add(m[1]);
  return out;
}

// ---------- ui package graph ----------

// ui/<pkg> where <pkg> has package.json. Edges pkg -> sibling pkg are read
// from `../<sibling>/` tokens inside the package's own sources and tsconfigs.
export function buildUiGraph(root) {
  const uiDir = join(root, "ui");
  let pkgs;
  try {
    pkgs = readdirSync(uiDir).filter((d) => existsSync(join(uiDir, d, "package.json")));
  } catch {
    return { ok: false, reason: "ui-dir-unreadable", edges: new Map(), packages: new Set() };
  }
  const pkgSet = new Set(pkgs);
  const edges = new Map(pkgs.map((p) => [p, new Set()]));
  const REF = /\.\.\/([A-Za-z0-9_-]+)\//g;
  for (const pkg of pkgs) {
    const dir = join(uiDir, pkg);
    for (const f of walkFiles(dir)) {
      const isConfig = /^tsconfig.*\.json$/.test(f.split(sep).pop());
      const isTs = /\.(?:ts|tsx|mts|cts)$/.test(f);
      if (!isConfig && !isTs) continue;
      let text;
      try {
        text = readFileSync(f, "utf8");
      } catch {
        continue;
      }
      for (const m of text.matchAll(REF)) {
        if (pkgSet.has(m[1]) && m[1] !== pkg) edges.get(pkg).add(m[1]);
      }
    }
  }
  return { ok: true, packages: pkgSet, edges };
}

export function uiReverseDeps(edges) {
  const rev = new Map();
  for (const [pkg, deps] of edges) {
    for (const dep of deps) {
      if (!rev.has(dep)) rev.set(dep, new Set());
      rev.get(dep).add(pkg);
    }
  }
  return rev;
}

// ---------- lane owner derivation ----------

// Map<laneId, {owners: Set<crate>|null, source: string}>
// owners === null means "always expected" (derivation failed -> fail-open).
export function deriveLaneOwners(root, ctx) {
  const owners = new Map();
  const bodies = miseTaskBodies(root);
  for (const [lane, rule] of Object.entries(OWNER_RULES)) {
    if (rule.kind === "rt-scan") {
      const found = rtTestCrates(root, ctx.crateGraph.members);
      owners.set(lane, {
        owners: found.size ? found : null,
        source: found.size ? "rt-scan" : "rt-scan-empty-fallback",
      });
      continue;
    }
    if (rule.kind === "mise-task-pkgs") {
      const set = new Set();
      for (const task of rule.tasks) {
        const body = bodies.get(task);
        if (!body) continue;
        for (const pkg of cargoPkgsOfTask(body)) set.add(pkg);
      }
      owners.set(lane, {
        owners: set.size ? set : null,
        source: set.size ? `mise:${rule.tasks.join(",")}` : "mise-parse-empty-fallback",
      });
      continue;
    }
    owners.set(lane, { owners: null, source: "unknown-rule-fallback" });
  }
  return owners;
}

// ---------- context ----------

export function buildContext(root) {
  const problems = [];
  const crateGraph = buildCrateGraph(root);
  if (!crateGraph.ok) problems.push(crateGraph.reason);
  const uiGraph = buildUiGraph(root);
  if (!uiGraph.ok) problems.push(uiGraph.reason);
  const ctx = {
    crateGraph,
    uiGraph,
    uiReverse: uiReverseDeps(uiGraph.edges),
    problems,
  };
  if (crateGraph.ok) {
    ctx.laneOwners = deriveLaneOwners(root, ctx);
  } else {
    // No crate graph -> every owner lane degrades to always-expected.
    ctx.laneOwners = new Map(
      Object.keys(OWNER_RULES).map((l) => [l, { owners: null, source: "no-graph-fallback" }]),
    );
  }
  return ctx;
}

// ---------- path classification ----------

// Returns { classes: [..], crate?: name, uiPkg?: name }
export function classifyPath(path, ctx) {
  const classes = [];
  let crate = null;
  let uiPkg = null;

  let bestDir = null;
  if (ctx.crateGraph.ok) {
    for (const [name, dir] of ctx.crateGraph.members) {
      if (path.startsWith(dir + "/") && (!bestDir || dir.length > bestDir.length)) {
        bestDir = dir;
        crate = name;
      }
    }
  }
  if (crate) classes.push(`crate:${crate}`);

  if (path.startsWith("ui/")) {
    const seg = path.split("/")[1];
    if (ctx.uiGraph.ok && ctx.uiGraph.packages.has(seg)) {
      uiPkg = seg;
      classes.push(`ui-pkg:${seg}`);
    }
  }

  for (const rule of STATIC_RULES) {
    if (rule.res.some((re) => re.test(path))) classes.push(rule.id);
  }

  // A `crates/` or `ui/` path that belongs to no member/package is not
  // docs/poc — it is unclassifiable and must resolve to Full CI.
  const underDynamicDir = /^(crates|ui)\//.test(path);
  if (classes.length === 0 || (underDynamicDir && !crate && !uiPkg && !classes.includes("docs") && !classes.includes("poc"))) {
    classes.push("unknown");
  }
  return { classes, crate, uiPkg };
}

// ---------- lane resolution ----------

const FULL_CLASSES = new Set(["shared-config", "ci-config", "repo-config", "unknown"]);

export function resolveLanes(classifiedFiles, ctx) {
  const lanes = {};
  for (const id of LANE_IDS) {
    lanes[id] = {
      expected: false,
      oses: LANES[id].oses,
      contexts: LANES[id].contexts,
      requiredContexts: LANES[id].requiredContexts,
      observeJob: LANES[id].observeJob,
      features: LANES[id].features,
      reasons: [],
    };
  }
  const fullReasons = [];

  for (const f of classifiedFiles) {
    if (f.classes.some((c) => FULL_CLASSES.has(c))) {
      const cls = f.classes.find((c) => FULL_CLASSES.has(c));
      fullReasons.push(`${cls}:${f.path}`);
    }
  }
  if (fullReasons.length) {
    for (const id of LANE_IDS) lanes[id].expected = true;
    return { decision: "full", fullReasons, lanes };
  }

  const touched = new Set();
  const uiTouched = new Set();
  const reasonLog = { verify: new Set() };
  const addReason = (lane, r) => lanes[lane].reasons.push(r);

  for (const f of classifiedFiles) {
    for (const cls of f.classes) {
      if (cls.startsWith("crate:")) touched.add(f.crate);
      else if (cls.startsWith("ui-pkg:")) uiTouched.add(f.uiPkg);
      else if (cls === "docs" || cls === "poc") {
        lanes.verify.expected = true;
        reasonLog.verify.add(`${cls}:${f.path}`);
      }
    }
  }

  const impacted = ctx.crateGraph.ok
    ? transitiveClosure([...touched], ctx.crateGraph.reverseDeps)
    : new Set(touched);
  if (impacted.size) {
    lanes.verify.expected = true;
    reasonLog.verify.add(`crates:${[...impacted].sort().join(",")}`);
  }
  if (uiTouched.size) {
    const uiImpacted = transitiveClosure([...uiTouched], ctx.uiReverse);
    lanes["ui-verify"].expected = true;
    lanes["ui-verify"].reasons.push(`ui-packages:${[...uiImpacted].sort().join(",")}`);
  }

  // Lane owners: any impacted crate that owns a lane marks it expected; a
  // lane whose owner derivation failed is always expected (fail-open).
  for (const [lane, info] of ctx.laneOwners) {
    if (info.owners === null) {
      if (ctx.crateGraph.ok) {
        // Derivation broke -> the lane can never be proven unaffected.
        lanes[lane].expected = true;
        lanes[lane].reasons.push(`owner-derivation-failed:${info.source}`);
      }
      continue;
    }
    for (const crate of impacted) {
      if (info.owners.has(crate)) {
        lanes[lane].expected = true;
        lanes[lane].reasons.push(`owner:${crate}`);
      }
    }
  }

  for (const [lane, set] of Object.entries(reasonLog)) {
    for (const r of set) lanes[lane].reasons.push(r);
  }
  for (const id of LANE_IDS) lanes[id].reasons = [...new Set(lanes[id].reasons)].sort();
  return { decision: "selective", fullReasons, lanes, touched, uiTouched, impacted };
}
