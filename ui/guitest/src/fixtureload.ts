/**
 * Issue #311 — versioned fixture loader (TS half).
 *
 * Loads the single fixture corpus shared with the Rust conformance
 * tests (`crates/ruviform-ui/tests/ipc_conformance.rs` reads the same
 * files). Loading is fail-closed: an unrecognized `v`, `suite`, or a
 * case field outside the suite vocabulary rejects the whole file —
 * fixture drift must fail loudly, never pass silently.
 *
 * Format authority: `fixtures/README.md`.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

export const FIXTURES_DIR = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "fixtures",
);

export type Suite =
  | "ipc-intents"
  | "ipc-events"
  | "ipc-data"
  | "binding-messages"
  | "scenario-binding"
  | "scenario-host";

const SUITES: readonly string[] = [
  "ipc-intents",
  "ipc-events",
  "ipc-data",
  "binding-messages",
  "scenario-binding",
  "scenario-host",
];

const TOP_FIELDS = new Set(["v", "suite", "id", "cases", "windows", "setup", "in", "expect", "seed"]);

export class FixtureError extends Error {}

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

export interface LoadedFile {
  readonly suite: Suite;
  readonly id: string;
  readonly cases: readonly Record<string, unknown>[];
}

/**
 * Read + structurally validate one fixture file. `rel` is the path
 * under `fixtures/` (e.g. `ipc/intents.v1.json`). Returns the raw
 * cases; suite-specific case shape is asserted by the caller's
 * per-case validator before use.
 */
export function loadFixtureFile(rel: string): LoadedFile {
  const path = join(FIXTURES_DIR, rel);
  const text = readFileSync(path, "utf8");
  let doc: unknown;
  try {
    doc = JSON.parse(text);
  } catch (e) {
    throw new FixtureError(`${rel}: not valid JSON: ${String(e)}`);
  }
  if (!isObj(doc)) throw new FixtureError(`${rel}: top level must be an object`);
  for (const k of Object.keys(doc)) {
    if (!TOP_FIELDS.has(k)) {
      throw new FixtureError(`${rel}: unknown top-level field ${k}`);
    }
  }
  if (doc.v !== 1) {
    throw new FixtureError(`${rel}: unsupported fixture version ${String(doc.v)}`);
  }
  if (typeof doc.suite !== "string" || !SUITES.includes(doc.suite)) {
    throw new FixtureError(`${rel}: unknown suite ${String(doc.suite)}`);
  }
  if (typeof doc.id !== "string" || doc.id.length === 0) {
    throw new FixtureError(`${rel}: missing file id`);
  }
  const suite = doc.suite as Suite;
  if (suite === "scenario-binding" || suite === "scenario-host") {
    if (!Array.isArray(doc.in)) {
      throw new FixtureError(`${rel}: scenario file must carry an "in" array`);
    }
    return { suite, id: doc.id, cases: [] };
  }
  if (!Array.isArray(doc.cases)) {
    throw new FixtureError(`${rel}: "cases" must be an array`);
  }
  const seen = new Set<string>();
  for (const [i, c] of doc.cases.entries()) {
    if (!isObj(c) || typeof c.id !== "string") {
      throw new FixtureError(`${rel}: case ${i} has no string id`);
    }
    if (!seen.add(c.id)) {
      throw new FixtureError(`${rel}: duplicate case id ${c.id}`);
    }
  }
  return { suite, id: doc.id, cases: doc.cases };
}

export interface LoadedScenario {
  readonly suite: "scenario-binding" | "scenario-host";
  readonly id: string;
  readonly seed: number | null;
  readonly doc: Record<string, unknown>;
}

/** Load a `scenario-*` file with the full `setup`/`in`/`expect`/`seed`
 *  payload preserved. */
export function loadScenarioFile(rel: string): LoadedScenario {
  const path = join(FIXTURES_DIR, rel);
  const text = readFileSync(path, "utf8");
  let doc: unknown;
  try {
    doc = JSON.parse(text);
  } catch (e) {
    throw new FixtureError(`${rel}: not valid JSON: ${String(e)}`);
  }
  if (!isObj(doc)) throw new FixtureError(`${rel}: top level must be an object`);
  for (const k of Object.keys(doc)) {
    if (!TOP_FIELDS.has(k)) {
      throw new FixtureError(`${rel}: unknown top-level field ${k}`);
    }
  }
  if (doc.v !== 1) {
    throw new FixtureError(`${rel}: unsupported fixture version ${String(doc.v)}`);
  }
  if (doc.suite !== "scenario-binding" && doc.suite !== "scenario-host") {
    throw new FixtureError(`${rel}: not a scenario suite (${String(doc.suite)})`);
  }
  if (typeof doc.id !== "string" || doc.id.length === 0) {
    throw new FixtureError(`${rel}: missing file id`);
  }
  if (!isObj(doc.setup)) {
    throw new FixtureError(`${rel}: scenario file must carry a "setup" object`);
  }
  if (!Array.isArray(doc.in)) {
    throw new FixtureError(`${rel}: scenario file must carry an "in" array`);
  }
  if (doc.seed !== undefined && doc.seed !== null && typeof doc.seed !== "number") {
    throw new FixtureError(`${rel}: "seed" must be a number or null`);
  }
  return {
    suite: doc.suite,
    id: doc.id,
    seed: (doc.seed as number | undefined) ?? null,
    doc,
  };
}

/**
 * Assert that `v` is an object carrying only `allowed` fields and
 * containing every `required` field. Returns the object or throws.
 * The shared shape check both the fixture loader and the wire
 * validators need — vocabulary outside `allowed` must never be read.
 */
export function requireFields(
  what: string,
  v: unknown,
  required: readonly string[],
  allowed: readonly string[],
): Record<string, unknown> {
  if (!isObj(v)) throw new FixtureError(`${what}: expected object`);
  for (const k of Object.keys(v)) {
    if (!allowed.includes(k)) {
      throw new FixtureError(`${what}: unknown field ${k}`);
    }
  }
  for (const k of required) {
    if (!(k in v)) throw new FixtureError(`${what}: missing field ${k}`);
  }
  return v;
}
