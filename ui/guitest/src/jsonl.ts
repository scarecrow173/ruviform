/**
 * Issue #311 — recording exchange format (`*.v1.jsonl`).
 *
 * One record per line: `{"v":1,"tick":N,"dir":"in|out|sys","type":...}`.
 * `dir:"in"` records are the *input* column replay re-injects;
 * `dir:"out"` records are observations (never replay expectations —
 * the contract oracle is the judge); `dir:"sys"` carries recorder
 * markers (`dropped`).
 *
 * The field vocabulary is a fixed allowlist — records may carry only
 * direction/tick/correlation/sequence/schema version and selected
 * enumerated metadata. Value payloads, media, prompts, grants, tokens
 * and bulk binary never appear (redaction by omission, spec §4).
 */

export type RecordDir = "in" | "out" | "sys";

export const RECORD_VERSION = 1;

/**
 * Allowlisted record types → allowed field names (beyond the common
 * `v`/`tick`/`dir`/`type`). Field names are intentionally enumerated —
 * adding a field is a format decision, not an accident.
 */
export const RECORD_FIELDS: Readonly<Record<string, readonly string[]>> = {
  gesture: ["controlId", "gestureId", "inputType", "source"],
  action: ["name"],
  receipt: ["commandId", "status", "ref"],
  projection: ["target", "revision", "hasValue"],
  detach: ["reason"],
  attach: ["windowId"],
  closeView: [],
  query: ["as", "sourceId", "sourceRevision", "start", "end"],
  queryOutcome: ["as", "outcome"],
  dispatch: [
    "commandId",
    "kind",
    "binding",
    "target",
    "controlId",
    "sequence",
    "gestureId",
    "generation",
    "expectReceipt",
    "viewId",
    "windowId",
  ],
  subscribe: ["binding", "target", "viewId", "windowId", "generation"],
  unsubscribe: ["binding", "target"],
  phase: ["binding", "phase"],
  dropped: ["reason", "count"],
};

export interface RecordLine {
  readonly v: 1;
  readonly tick: number;
  readonly dir: RecordDir;
  readonly type: string;
  readonly fields: Readonly<Record<string, unknown>>;
}

const DIRS = new Set<RecordDir>(["in", "out", "sys"]);
const TRUNCATE = 128;

/** Sanitize one field value to the allowlisted scalar shapes. */
function sanitize(v: unknown): unknown {
  if (typeof v === "string") {
    return v.length > TRUNCATE ? v.slice(0, TRUNCATE) : v;
  }
  if (typeof v === "number" && Number.isFinite(v)) return v;
  if (typeof v === "boolean") return v;
  if (v === null) return null;
  return undefined;
}

/** Build one record line (JSON, stable key order). Unknown/unsanitizable
 *  field values are omitted — records carry selected metadata only. */
export function serializeRecord(
  tick: number,
  dir: RecordDir,
  type: string,
  fields: Record<string, unknown>,
): string {
  const allowed = RECORD_FIELDS[type];
  if (allowed === undefined) {
    throw new Error(`jsonl: unknown record type ${type}`);
  }
  const parts: string[] = [
    `"v":${RECORD_VERSION}`,
    `"tick":${tick}`,
    `"dir":${JSON.stringify(dir)}`,
    `"type":${JSON.stringify(type)}`,
  ];
  for (const k of allowed) {
    if (!(k in fields)) continue;
    const v = sanitize(fields[k]);
    if (v === undefined) continue;
    parts.push(`${JSON.stringify(k)}:${JSON.stringify(v)}`);
  }
  return `{${parts.join(",")}}`;
}

export type ParsedRecord =
  | { readonly ok: true; readonly record: RecordLine }
  | { readonly ok: false; readonly error: string };

/** Parse one JSONL line, enforcing the allowlist (fail-closed). */
export function parseRecordLine(line: string): ParsedRecord {
  let raw: unknown;
  try {
    raw = JSON.parse(line);
  } catch (e) {
    return { ok: false, error: `invalid JSON: ${String(e)}` };
  }
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    return { ok: false, error: "record must be an object" };
  }
  const r = raw as Record<string, unknown>;
  if (r.v !== RECORD_VERSION) {
    return { ok: false, error: `unsupported record version ${String(r.v)}` };
  }
  if (typeof r.tick !== "number" || !Number.isInteger(r.tick) || r.tick < 0) {
    return { ok: false, error: "bad tick" };
  }
  if (typeof r.dir !== "string" || !DIRS.has(r.dir as RecordDir)) {
    return { ok: false, error: `bad dir ${String(r.dir)}` };
  }
  if (typeof r.type !== "string" || !(r.type in RECORD_FIELDS)) {
    return { ok: false, error: `unknown record type ${String(r.type)}` };
  }
  const allowed = new Set<string>([
    "v",
    "tick",
    "dir",
    "type",
    ...RECORD_FIELDS[r.type]!,
  ]);
  const fields: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(r)) {
    if (["v", "tick", "dir", "type"].includes(k)) continue;
    if (!allowed.has(k)) {
      return { ok: false, error: `field ${k} outside allowlist for ${r.type}` };
    }
    fields[k] = v;
  }
  return {
    ok: true,
    record: {
      v: 1,
      tick: r.tick,
      dir: r.dir as RecordDir,
      type: r.type,
      fields,
    },
  };
}

/** Parse a whole `*.v1.jsonl` document (fail-closed per line). */
export function parseRecording(
  text: string,
): { ok: true; records: RecordLine[] } | { ok: false; error: string } {
  const records: RecordLine[] = [];
  const lines = text.split("\n").filter((l) => l.trim().length > 0);
  for (const [i, line] of lines.entries()) {
    const p = parseRecordLine(line);
    if (!p.ok) return { ok: false, error: `line ${i + 1}: ${p.error}` };
    records.push(p.record);
  }
  return { ok: true, records };
}
