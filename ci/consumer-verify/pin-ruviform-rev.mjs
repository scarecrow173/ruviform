#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const RUVIFORM_GIT_URL_NEEDLE = "scarecrow173/ruviform.git";

const GIT_DEP_RE = new RegExp(
  String.raw`\bgit\s*=\s*("[^"]*scarecrow173/ruviform\.git[^"]*"|'[^']*scarecrow173/ruviform\.git[^']*')`,
);

function assertCandidateRev(rev) {
  if (!/^[0-9a-f]{40}$/.test(rev)) {
    throw new Error(`candidate rev must be a 40-hex commit SHA (got: ${rev})`);
  }
}

function splitTomlComment(line) {
  let quote = null;
  let escaped = false;
  for (let i = 0; i < line.length; i += 1) {
    const ch = line[i];
    if (quote === '"') {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === '"') {
        quote = null;
      }
      continue;
    }
    if (quote === "'") {
      if (ch === "'") quote = null;
      continue;
    }
    if (ch === "#") return [line.slice(0, i), line.slice(i)];
    if (ch === '"' || ch === "'") quote = ch;
  }
  return [line, ""];
}

function shapeError(file, lineNo, reason, line) {
  return new Error(`${file}:${lineNo}: unsupported ruviform git dependency shape (${reason}): ${line.trim()}`);
}

function splitInlineTableEntries(text, file, lineNo, line) {
  const parts = [];
  let quote = null;
  let escaped = false;
  let squareDepth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (quote === '"') {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === '"') {
        quote = null;
      }
      continue;
    }
    if (quote === "'") {
      if (ch === "'") quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
    } else if (ch === "[") {
      squareDepth += 1;
    } else if (ch === "]") {
      squareDepth -= 1;
      if (squareDepth < 0) throw shapeError(file, lineNo, "unbalanced array in inline dependency table", line);
    } else if (ch === "{" || ch === "}") {
      throw shapeError(file, lineNo, "nested inline table", line);
    } else if (ch === "," && squareDepth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  if (quote || squareDepth !== 0) {
    throw shapeError(file, lineNo, "unterminated string or array in inline dependency table", line);
  }
  parts.push(text.slice(start));
  const entries = parts.map((part) => part.trim());
  if (entries.length > 0 && entries[entries.length - 1] === "") entries.pop();
  if (entries.some((entry) => entry === "")) {
    throw shapeError(file, lineNo, "empty entry in inline dependency table", line);
  }
  return entries;
}

function findInlineTableClose(line, open, file, lineNo) {
  let quote = null;
  let escaped = false;
  for (let i = open + 1; i < line.length; i += 1) {
    const ch = line[i];
    if (quote === '"') {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === '"') {
        quote = null;
      }
      continue;
    }
    if (quote === "'") {
      if (ch === "'") quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") quote = ch;
    if (!quote && ch === "}") return i;
  }
  throw shapeError(file, lineNo, "inline dependency table is not closed on the same line", line);
}

function normalizeKey(rawKey) {
  if ((rawKey.startsWith('"') && rawKey.endsWith('"')) || (rawKey.startsWith("'") && rawKey.endsWith("'"))) {
    return rawKey.slice(1, -1);
  }
  return rawKey;
}

function normalizeRelevantLine(code, rev, file, lineNo) {
  const firstEquals = code.indexOf("=");
  const dependencyKey = firstEquals < 0 ? "" : code.slice(0, firstEquals).trim();
  if (!/^"?[A-Za-z0-9_-]+"?$/.test(dependencyKey)) {
    throw shapeError(file, lineNo, "not a single-line dependency assignment", code);
  }
  const open = code.indexOf("{", firstEquals + 1);
  if (open < 0) {
    throw shapeError(file, lineNo, "only single-line inline-table dependencies are supported", code);
  }
  const close = findInlineTableClose(code, open, file, lineNo);
  const trailing = code.slice(close + 1);
  if (trailing.trim() !== "") {
    throw shapeError(file, lineNo, "unexpected tokens after inline dependency table", code);
  }

  const entries = splitInlineTableEntries(code.slice(open + 1, close), file, lineNo, code);
  const seen = new Set();
  let hasGit = false;
  let hasRev = false;
  const normalized = [];
  for (const entry of entries) {
    const equals = entry.indexOf("=");
    if (equals < 0) throw shapeError(file, lineNo, "entry without key/value separator", code);
    const key = normalizeKey(entry.slice(0, equals).trim());
    if (!key) throw shapeError(file, lineNo, "empty entry key", code);
    if (seen.has(key)) throw shapeError(file, lineNo, `duplicate dependency key '${key}'`, code);
    seen.add(key);
    if (key === "tag") {
      throw shapeError(file, lineNo, "tag-pinned dependencies are outside the rev/branch normalization contract", code);
    }
    if (key === "branch") continue;
    if (key === "rev") {
      hasRev = true;
      normalized.push(`rev = \"${rev}\"`);
      continue;
    }
    if (key === "git") {
      hasGit = true;
      if (!entry.includes(RUVIFORM_GIT_URL_NEEDLE)) {
        throw shapeError(file, lineNo, "git key does not contain the ruviform URL", code);
      }
    }
    normalized.push(entry.trim());
  }
  if (!hasGit) throw shapeError(file, lineNo, "missing git key", code);
  if (!hasRev) normalized.push(`rev = \"${rev}\"`);
  return `${code.slice(0, open + 1)} ${normalized.join(", ")} }${trailing}`;
}

function splitLinesKeepEnds(source) {
  const pieces = source.split(/(\r\n|\n|\r)/);
  const lines = [];
  for (let i = 0; i < pieces.length; i += 2) {
    lines.push([pieces[i], pieces[i + 1] ?? ""]);
  }
  return lines;
}

export function normalizeManifestToml(source, rev, file = "Cargo.toml") {
  assertCandidateRev(rev);
  let changedLines = 0;
  const output = [];
  const lines = splitLinesKeepEnds(source);
  for (let i = 0; i < lines.length; i += 1) {
    const [body, ending] = lines[i];
    const [code, comment] = splitTomlComment(body);
    if (!GIT_DEP_RE.test(code)) {
      output.push(body, ending);
      continue;
    }
    output.push(normalizeRelevantLine(code, rev, file, i + 1), comment, ending);
    changedLines += 1;
  }
  const text = output.join("");
  return { text, changed: text !== source, changedLines };
}

function parseTomlString(input, state, key) {
  const quote = input[state.pos];
  if (quote !== '"' && quote !== "'") {
    throw new Error(`workspace ${key} must be an array of strings`);
  }
  state.pos += 1;
  let value = "";
  while (state.pos < input.length) {
    const ch = input[state.pos];
    state.pos += 1;
    if (ch === quote) return value;
    if (quote === "'") {
      value += ch;
      continue;
    }
    if (ch !== "\\") {
      value += ch;
      continue;
    }
    if (state.pos >= input.length) throw new Error(`workspace ${key} contains an unterminated escape`);
    const esc = input[state.pos];
    state.pos += 1;
    if (esc === "\\" || esc === '"' || esc === "/") {
      value += esc;
    } else if (esc === "n") {
      value += "\n";
    } else if (esc === "t") {
      value += "\t";
    } else {
      throw new Error(`workspace ${key} contains unsupported string escape \\${esc}`);
    }
  }
  throw new Error(`workspace ${key} contains an unterminated string`);
}

function skipWhitespace(input, state) {
  while (state.pos < input.length && /\s/.test(input[state.pos])) state.pos += 1;
}

function parseStringArray(input, key) {
  const state = { pos: 0 };
  skipWhitespace(input, state);
  if (input[state.pos] !== "[") throw new Error(`workspace ${key} must be an array of strings`);
  state.pos += 1;
  const values = [];
  for (;;) {
    skipWhitespace(input, state);
    if (state.pos >= input.length) throw new Error(`workspace ${key} array is unterminated`);
    if (input[state.pos] === "]") {
      state.pos += 1;
      break;
    }
    values.push(parseTomlString(input, state, key));
    skipWhitespace(input, state);
    if (input[state.pos] === ",") {
      state.pos += 1;
      continue;
    }
    if (input[state.pos] === "]") {
      state.pos += 1;
      break;
    }
    throw new Error(`workspace ${key} array has an unexpected token`);
  }
  skipWhitespace(input, state);
  if (state.pos !== input.length) throw new Error(`workspace ${key} has tokens after the array`);
  return values;
}

function arrayLiteralEnd(input) {
  let quote = null;
  let escaped = false;
  let depth = 0;
  for (let i = 0; i < input.length; i += 1) {
    const ch = input[i];
    if (quote === '"') {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === '"') {
        quote = null;
      }
      continue;
    }
    if (quote === "'") {
      if (ch === "'") quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
    } else if (ch === "[") {
      depth += 1;
    } else if (ch === "]") {
      depth -= 1;
      if (depth === 0) return i;
      if (depth < 0) return -1;
    }
  }
  return -1;
}

function extractWorkspaceArray(rootToml, key) {
  const lines = splitLinesKeepEnds(rootToml);
  let inWorkspace = false;
  let found = null;
  for (let i = 0; i < lines.length; i += 1) {
    const [code] = splitTomlComment(lines[i][0]);
    const section = code.match(/^\s*\[([^\]]+)\]\s*$/);
    if (section) {
      inWorkspace = section[1].trim() === "workspace";
      continue;
    }
    if (!inWorkspace) continue;
    const match = code.match(new RegExp(`^\\s*${key}\\s*=\\s*(.*)$`));
    if (!match) continue;
    if (found !== null) throw new Error(`workspace ${key} is declared more than once`);
    let literal = match[1];
    let end = arrayLiteralEnd(literal);
    while (end < 0 && i + 1 < lines.length) {
      i += 1;
      const [nextCode] = splitTomlComment(lines[i][0]);
      literal += `\n${nextCode}`;
      end = arrayLiteralEnd(literal);
    }
    if (end < 0) throw new Error(`workspace ${key} array is unterminated`);
    found = parseStringArray(literal, key);
  }
  return found;
}

function normalizeMemberPattern(pattern, key) {
  let normalized = pattern.replace(/\\/g, "/");
  while (normalized.startsWith("./")) normalized = normalized.slice(2);
  if (normalized === "" || normalized.startsWith("/") || normalized.includes("..")) {
    throw new Error(`workspace ${key} pattern is outside the repository or empty: ${pattern}`);
  }
  return normalized.replace(/\/+$/, "");
}

function globToRegex(pattern) {
  let out = "^";
  for (let i = 0; i < pattern.length; i += 1) {
    const ch = pattern[i];
    if (ch === "*") {
      if (pattern[i + 1] === "*") {
        out += ".*";
        i += 1;
      } else {
        out += "[^/]*";
      }
    } else if (ch === "?") {
      out += "[^/]";
    } else {
      out += ch.replace(/[.+^${}()|[\]\\]/g, "\\$&");
    }
  }
  return new RegExp(`${out}$`);
}

function manifestDir(rel) {
  if (rel === "Cargo.toml") return "";
  return rel.slice(0, -"/Cargo.toml".length);
}

function listTrackedCargoTomls(repoRoot) {
  const out = execFileSync("git", ["-C", repoRoot, "ls-files", "-z", "--", "*Cargo.toml"], {
    encoding: "utf8",
  });
  return out
    .split("\0")
    .filter(Boolean)
    .sort();
}

export function workspaceManifests(repoRoot, trackedFiles = listTrackedCargoTomls(repoRoot)) {
  if (!trackedFiles.includes("Cargo.toml")) {
    throw new Error("root Cargo.toml is not tracked in the consumer checkout");
  }
  for (const rel of trackedFiles) {
    if (rel.startsWith("../") || rel.includes("/../") || rel.includes("\\")) {
      throw new Error(`tracked Cargo.toml path is outside the portable relative-path contract: ${rel}`);
    }
  }
  const rootToml = readFileSync(join(repoRoot, "Cargo.toml"), "utf8");
  const members = extractWorkspaceArray(rootToml, "members") ?? [];
  const excludes = (extractWorkspaceArray(rootToml, "exclude") ?? []).map((pattern) => normalizeMemberPattern(pattern, "exclude"));
  const dirs = new Map();
  for (const rel of trackedFiles) dirs.set(manifestDir(rel), rel);

  const selected = new Set(["Cargo.toml"]);
  for (const rawPattern of members) {
    const pattern = normalizeMemberPattern(rawPattern, "members");
    const isGlob = /[*?]/.test(pattern);
    const re = globToRegex(pattern);
    const matches = [];
    for (const [dir, rel] of dirs) {
      if (dir === "") continue;
      if (!re.test(dir)) continue;
      if (isGlob && excludes.some((exclude) => globToRegex(exclude).test(dir))) continue;
      matches.push(rel);
    }
    if (matches.length === 0) {
      throw new Error(`workspace member pattern '${rawPattern}' matched no tracked Cargo.toml`);
    }
    for (const rel of matches) selected.add(rel);
  }
  return [...selected].sort();
}

export function normalizeWorkspace(repoRoot, rev, trackedFiles = listTrackedCargoTomls(repoRoot)) {
  assertCandidateRev(rev);
  const scannedFiles = workspaceManifests(repoRoot, trackedFiles);
  const changedFiles = [];
  let changedLines = 0;
  for (const rel of scannedFiles) {
    const path = join(repoRoot, ...rel.split("/"));
    const before = readFileSync(path, "utf8");
    const result = normalizeManifestToml(before, rev, rel);
    if (result.text !== before) {
      writeFileSync(path, result.text);
      changedFiles.push(rel);
      changedLines += result.changedLines;
    }
  }
  return {
    scannedFiles,
    changedFiles,
    scannedCount: scannedFiles.length,
    changedCount: changedFiles.length,
    changedLines,
  };
}

function parseArgs(argv) {
  const args = { repo: process.cwd(), rev: "" };
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--repo") {
      i += 1;
      args.repo = argv[i] ?? "";
    } else if (argv[i] === "--rev") {
      i += 1;
      args.rev = argv[i] ?? "";
    } else {
      throw new Error(`unknown argument: ${argv[i]}`);
    }
  }
  if (!args.repo) throw new Error("--repo is required");
  if (!args.rev) throw new Error("--rev is required");
  return args;
}

function main(argv) {
  const args = parseArgs(argv);
  const result = normalizeWorkspace(args.repo, args.rev);
  console.log(`scanned ${result.scannedCount} workspace manifest(s); changed ${result.changedCount} manifest(s); normalized ${result.changedLines} dependency line(s)`);
  for (const rel of result.changedFiles) console.log(`normalized: ${rel}`);
}

const invokedAsScript = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedAsScript) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`error: ${error.message}`);
    process.exitCode = 1;
  }
}
