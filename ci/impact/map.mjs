// map.mjs — Issue #333 (CI-OPT-06) impact classification rule data.
//
// Canonical contract: docs/designs/issue-333-impact-based-ci.md
// ("変更クラス → 期待 lane 契約" section). This file is reviewable DATA:
// the classification semantics live here, the mechanics live in lib.mjs.
//
// Consumers (AudioLens #149 / Synvelis #148) reuse the contract shape —
// classes, lane verdict vocabulary, full-CI fallback — with their own map.

// Lane ids are the job ids in .github/workflows/ci.yml. `contexts` are the
// required-status-check context names the jobs emit (kept for audit output;
// the aggregated `ci-gate` job is the future single required context).
export const LANE_IDS = [
  "verify",
  "rt-verify",
  "icc-verify",
  "ui-verify",
  "l1-real-verify",
  "bench",
];

export const LANES = {
  verify: {
    oses: ["ubuntu-latest", "windows-latest", "macos-latest"],
    contexts: [
      "verify (ubuntu-latest)",
      "verify (windows-latest)",
      "verify (macos-latest)",
    ],
    features: [],
  },
  "rt-verify": {
    oses: ["ubuntu-latest"],
    contexts: ["rt-verify"],
    features: [],
  },
  "icc-verify": {
    oses: ["ubuntu-latest"],
    contexts: ["icc-verify"],
    features: ["icc"],
  },
  "ui-verify": {
    oses: ["ubuntu-latest", "windows-latest", "macos-latest"],
    contexts: [
      "ui-verify (ubuntu-latest)",
      "ui-verify (windows-latest)",
      "ui-verify (macos-latest)",
    ],
    features: ["webview"],
  },
  "l1-real-verify": {
    oses: ["ubuntu-latest", "windows-latest", "macos-latest"],
    contexts: [
      "l1-real-verify (ubuntu-latest)",
      "l1-real-verify (windows-latest)",
      "l1-real-verify (macos-latest)",
    ],
    features: ["host"],
  },
  bench: {
    oses: ["ubuntu-latest"],
    contexts: ["bench"],
    features: [],
  },
};

// Path classes evaluated against repo-relative forward-slash paths.
// `lanes: "full"` means: a single matching file forces Full CI — the class
// carries shared/global blast radius that cannot be scoped to one lane.
// Order does not matter; a file accumulates every matching class and the
// lane set is the union (any "full" class wins the decision).
export const PATH_CLASSES = [
  {
    id: "shared-config",
    lanes: "full",
    patterns: [
      "Cargo.toml",
      "Cargo.lock",
      "mise.toml",
      "mise.lock",
      ".mise/**",
      ".cargo/**",
      "rust-toolchain",
      "rust-toolchain.toml",
      "deny.toml",
    ],
    note: "workspace manifest/lockfile/toolchain — affects every lane",
  },
  {
    id: "ci-config",
    lanes: "full",
    patterns: [".github/**", ".pre-commit-config.yaml", "ci/**"],
    note: "CI workflows/scripts incl. this classifier — self-changes run full",
  },
  {
    id: "repo-config",
    lanes: "full",
    patterns: [
      ".gitattributes",
      ".gitignore",
      ".editorconfig",
      "apm.yml",
      "apm.lock.yaml",
      ".agents/**",
      ".claude/**",
    ],
    note: "shared repo/agent config — ambiguity resolves to full",
  },
  {
    id: "docs",
    lanes: ["verify"],
    patterns: ["docs/**", "*.md", "crates/*.md", "LICENSE", "LICENSE-*"],
    note: "only the prek language-agnostic hooks (in verify) can fail",
  },
  {
    id: "poc",
    lanes: ["verify"],
    patterns: ["poc/**"],
    note: "outside the cargo workspace — only prek hooks apply",
  },
];

// Dynamic classes handled by code (not static patterns):
//   crates/<dir>/**  -> class `crate:<pkg>`   (dir must host a member manifest)
//   ui/<pkg>/**      -> class `ui-pkg:<pkg>`  (dir must contain package.json)
// Anything unmatched -> class `unknown` -> Full CI.

// Lane owners: a lane becomes expected when a touched-or-impacted crate is in
// its owner set. Owner sets are machine-derived, not hand-maintained:
//   kind "mise-task-pkgs" — parse `cargo ... -p <crate>` args out of the named
//     mise tasks' `run` entries (single source for feature-lane coverage).
//   kind "rt-scan" — crates containing `fn rt_` under tests|src|benches|examples
//     (the `cargo test --workspace rt_` contract is name-prefix based).
// A derivation that yields an empty set is treated as failure and the lane
// degrades to "always expected" (fail-open ownership).
export const OWNER_RULES = {
  "rt-verify": { kind: "rt-scan" },
  "icc-verify": {
    kind: "mise-task-pkgs",
    tasks: ["test-video-proc-icc", "lint-video-proc-icc"],
  },
  "l1-real-verify": {
    kind: "mise-task-pkgs",
    tasks: ["test-l1-real-plugin", "lint-l1-real-plugin"],
  },
  "ui-verify": {
    kind: "mise-task-pkgs",
    tasks: ["check-ui-webview", "lint-ui-webview", "test-ui-webview"],
  },
  bench: { kind: "mise-task-pkgs", tasks: ["bench-smoke"] },
};

// Event classes: selective decisions exist only for pull_request diffs.
export const FULL_EVENTS = new Set(["push", "workflow_dispatch", "schedule", "local-all"]);

export const VERDICT = {
  PASS: "Pass",
  FAIL: "Fail",
  NOT_APPLICABLE: "NotApplicable",
  NOT_RUN: "NotRun",
  UNSUPPORTED: "Unsupported",
};
