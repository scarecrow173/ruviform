// Issue #1064 contract tests for consumer manifest normalization. The
// regression fixture is Synvelis-shaped: root workspace dependencies plus
// crate-local direct git dependencies in nested workspace members.

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  RUVIFORM_GIT_URL_NEEDLE,
  normalizeManifestToml,
  normalizeWorkspace,
} from "../pin-ruviform-rev.mjs";

const SCRIPT = fileURLToPath(new URL("../pin-ruviform-rev.mjs", import.meta.url));
const REV = "a".repeat(40);
const OLD_REV = "b".repeat(40);
const RUVIFORM_URL = "https://github.com/scarecrow173/ruviform.git";

function write(repo, rel, text) {
  const path = join(repo, rel);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
}

function makeRepo(files) {
  const repo = mkdtempSync(join(tmpdir(), "rf1064-pin-"));
  for (const [rel, text] of Object.entries(files)) {
    write(repo, rel, text);
  }
  execFileSync("git", ["-C", repo, "init", "-q"]);
  execFileSync("git", ["-c", "core.autocrlf=false", "-C", repo, "add", "."]);
  return repo;
}

function synvelisShapeFiles() {
  return {
    "Cargo.toml": `[workspace]\nmembers = [\n  "crates/synvelis-player",\n  "crates/synvelis-script-*",\n]\nexclude = ["crates/synvelis-script-excluded"]\n\n[workspace.dependencies]\nruviform-io-file = { git = "${RUVIFORM_URL}" }\nruviform-media-core = { git = "${RUVIFORM_URL}", branch = "main", features = ["pcm"] }\nruviform-rt = { git = "${RUVIFORM_URL}", rev = "${OLD_REV}", features = ["rt"] }\n`,
    "crates/synvelis-player/Cargo.toml": `[package]\nname = "synvelis-player"\n\n[dependencies]\nruviform-plugin-export = { git = "${RUVIFORM_URL}" }\n`,
    "crates/synvelis-script-luau/Cargo.toml": `[package]\nname = "synvelis-script-luau"\n\n[dependencies]\nruviform-script = { git = "${RUVIFORM_URL}" }\nruviform-script-luau = { git = "${RUVIFORM_URL}", branch = "main" }\n`,
    "crates/synvelis-script-excluded/Cargo.toml": `[package]\nname = "excluded"\n\n[dependencies]\nruviform-script = { git = "${RUVIFORM_URL}" }\n`,
    "vendor/not-a-member/Cargo.toml": `[package]\nname = "vendored"\n\n[dependencies]\nruviform-script = { git = "${RUVIFORM_URL}" }\n`,
  };
}

function read(repo, rel) {
  return readFileSync(join(repo, rel), "utf8");
}

function relevantGitLines(text) {
  return text.split(/\r?\n/).filter((line) => line.includes(RUVIFORM_GIT_URL_NEEDLE) && !line.trimStart().startsWith("#"));
}

test("nested workspace member direct git dependencies are normalized with the root manifest", () => {
  const repo = makeRepo(synvelisShapeFiles());
  try {
    // Root-only normalization reproduces the regression shape: nested member
    // manifests still follow the default branch, so no single candidate SHA exists.
    const rootOnly = normalizeManifestToml(read(repo, "Cargo.toml"), REV, "Cargo.toml").text;
    assert.ok(rootOnly.includes(`rev = "${REV}"`));
    assert.ok(!read(repo, "crates/synvelis-player/Cargo.toml").includes(REV));
    assert.ok(!read(repo, "crates/synvelis-script-luau/Cargo.toml").includes(REV));

    const result = normalizeWorkspace(repo, REV);
    assert.deepEqual(result.changedFiles.sort(), [
      "Cargo.toml",
      "crates/synvelis-player/Cargo.toml",
      "crates/synvelis-script-luau/Cargo.toml",
    ].sort());
    assert.equal(result.scannedFiles.length, 3);

    for (const rel of result.scannedFiles) {
      const lines = relevantGitLines(read(repo, rel));
      assert.ok(lines.length > 0, `${rel} fixture must keep relevant dependencies`);
      for (const line of lines) {
        assert.ok(line.includes(`rev = "${REV}"`), `${rel}: ${line}`);
        assert.ok(!line.includes('branch = "main"'), `${rel}: ${line}`);
        assert.ok(!line.includes(OLD_REV), `${rel}: ${line}`);
      }
    }
    assert.ok(read(repo, "Cargo.toml").includes('features = ["pcm"]'));
    assert.ok(read(repo, "Cargo.toml").includes('features = ["rt"]'));

    // Tracked non-members are outside the consumer workspace and are not rewritten.
    assert.ok(!read(repo, "crates/synvelis-script-excluded/Cargo.toml").includes(REV));
    assert.ok(!read(repo, "vendor/not-a-member/Cargo.toml").includes(REV));
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});

test("cli prints scanned and changed manifests and exits zero", () => {
  const repo = makeRepo(synvelisShapeFiles());
  try {
    const out = execFileSync(process.execPath, [SCRIPT, "--repo", repo, "--rev", REV], {
      encoding: "utf8",
    });
    assert.match(out, /scanned 3 workspace manifest\(s\)/);
    assert.match(out, /changed 3 manifest\(s\)/);
    assert.match(out, /crates\/synvelis-player\/Cargo\.toml/);
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});

test("untracked files are outside the tracked-file boundary even when they contain ruviform git deps", () => {
  const repo = makeRepo({
    "Cargo.toml": `[workspace]\nmembers = []\n\n[workspace.dependencies]\nruviform-io-file = { git = "${RUVIFORM_URL}" }\n`,
  });
  try {
    write(repo, "crates/untracked/Cargo.toml", `[package]\nname = "untracked"\n\n[dependencies]\nruviform-script = { git = "${RUVIFORM_URL}" }\n`);
    const result = normalizeWorkspace(repo, REV);
    assert.deepEqual(result.scannedFiles, ["Cargo.toml"]);
    assert.ok(!read(repo, "crates/untracked/Cargo.toml").includes(REV));
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});

test("table-shaped git dependency is fail-closed instead of silently skipped", () => {
  const repo = makeRepo({
    "Cargo.toml": `[workspace]\nmembers = ["crates/member"]\n`,
    "crates/member/Cargo.toml": `[package]\nname = "member"\n\n[dependencies.ruviform-script]\ngit = "${RUVIFORM_URL}"\n`,
  });
  try {
    assert.throws(() => normalizeWorkspace(repo, REV), /unsupported ruviform git dependency shape/);
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});

test("multiline inline-table git dependency is fail-closed instead of silently skipped", () => {
  const repo = makeRepo({
    "Cargo.toml": `[workspace]\nmembers = []\n\n[workspace.dependencies]\nruviform-io-file = {\n  git = "${RUVIFORM_URL}",\n  features = ["pcm"],\n}\n`,
  });
  try {
    assert.throws(() => normalizeWorkspace(repo, REV), /unsupported ruviform git dependency shape/);
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});

test("tag-pinned git dependency is fail-closed because existing semantics do not rewrite tags", () => {
  assert.throws(
    () => normalizeManifestToml(`[dependencies]\nruviform-x = { git = "${RUVIFORM_URL}", tag = "v1" }\n`, REV, "Cargo.toml"),
    /tag/,
  );
});

test("comments and non-ruviform git dependencies are ignored", () => {
  const input = `# ruviform-example = { git = "${RUVIFORM_URL}" }\n[dependencies]\nother = { git = "https://github.com/example/ruviform.git" }\nruviform-x = { git = "${RUVIFORM_URL}" } # candidate pin\n`;
  const out = normalizeManifestToml(input, REV, "Cargo.toml").text;
  assert.ok(out.startsWith(`# ruviform-example = { git = "${RUVIFORM_URL}" }`));
  assert.ok(out.includes('other = { git = "https://github.com/example/ruviform.git" }'));
  assert.ok(out.includes(`rev = "${REV}" } # candidate pin`));
});

test("workspace member pattern that matches no tracked manifest fails closed", () => {
  const repo = makeRepo({
    "Cargo.toml": `[workspace]\nmembers = ["crates/missing"]\n`,
  });
  try {
    assert.throws(() => normalizeWorkspace(repo, REV), /matched no tracked Cargo\.toml/);
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});

test("cli exits non-zero on unsupported shapes", () => {
  const repo = makeRepo({
    "Cargo.toml": `[workspace]\nmembers = []\n\n[workspace.dependencies]\nruviform-io-file = {\n  git = "${RUVIFORM_URL}",\n}\n`,
  });
  try {
    assert.throws(
      () => execFileSync(process.execPath, [SCRIPT, "--repo", repo, "--rev", REV], { encoding: "utf8", stdio: "pipe" }),
      /Command failed/,
    );
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});
