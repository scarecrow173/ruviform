/**
 * Issue #311 — `rvf.ipc.v1` + `rvf://data` wire conformance (TS half).
 *
 * A procedural, dependency-free mirror of the Rust contract surface:
 *
 * - {@link parseIntent} mirrors `ruviform_ui::ipc::parse_intent`
 *   (size cap → `message_too_large`; JSON shape / `v` / `kind` / op
 *   syntax → `malformed_message`; `params` defaults to null; unknown
 *   top-level fields tolerated per serde defaults).
 * - {@link decodeEvent} mirrors the Rust→frontend `Event` envelope:
 *   `{"v":1,"kind":...}` with the per-kind required fields; unknown
 *   fields tolerated (additive-extension rule, Issue #172).
 * - {@link dataUrl} mirrors the `BRIDGE_JS` `dataUrl` builder
 *   (per-segment `encodeURIComponent`, `offset` default 0, `len` only
 *   when given).
 * - {@link serveData} models the `rvf://data` scheme handler's wire
 *   contract (name segment rules, query parse, range clamps, status
 *   mapping, headers) so fixture expectations can be checked without a
 *   live WebView.
 *
 * No new protocol is defined here — every rule names its Rust
 * implementation in `crates/ruviform-ui` / `ruviform-ui-webview`.
 */

export const RVF_IPC_VERSION = 1;
export const MAX_INTENT_BYTES = 1024 * 1024;
export const MAX_OP_LEN = 128;
/** `rvf://data` default per-pull cap (DEFAULT_MAX_CHUNK). */
export const DEFAULT_MAX_CHUNK = 16 * 1024 * 1024;

export type IpcErrorCode = "malformed_message" | "message_too_large";

export interface DecodedIntent {
  readonly v: 1;
  readonly kind: "intent";
  readonly id: string;
  readonly op: string;
  readonly params: unknown;
}

export type IntentDecodeResult =
  | { readonly ok: true; readonly intent: DecodedIntent }
  | { readonly ok: false; readonly code: IpcErrorCode };

function utf8Len(s: string): number {
  return new TextEncoder().encode(s).length;
}

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function isU32(v: unknown): v is number {
  return (
    typeof v === "number" &&
    Number.isInteger(v) &&
    v >= 0 &&
    v <= 0xffffffff
  );
}

/** `ns.name`, segments non-empty, `[a-z0-9_.-]` only, ≤ MAX_OP_LEN —
 *  mirrors `ruviform_ui::ipc::valid_op`. */
export function validOp(op: string): boolean {
  if (op.length === 0 || op.length > MAX_OP_LEN) return false;
  if (!op.includes(".")) return false;
  for (const c of op) {
    const ok =
      (c >= "a" && c <= "z") ||
      (c >= "0" && c <= "9") ||
      c === "." ||
      c === "_" ||
      c === "-";
    if (!ok) return false;
  }
  return op.split(".").every((seg) => seg.length > 0);
}

/** Mirror of `parse_intent` — byte-length cap, JSON shape, version,
 *  kind, op syntax. `params` missing → `null` (serde default). */
export function parseIntent(body: string): IntentDecodeResult {
  if (utf8Len(body) > MAX_INTENT_BYTES) {
    return { ok: false, code: "message_too_large" };
  }
  let raw: unknown;
  try {
    raw = JSON.parse(body);
  } catch {
    return { ok: false, code: "malformed_message" };
  }
  if (
    !isObj(raw) ||
    !isU32(raw.v) ||
    typeof raw.kind !== "string" ||
    typeof raw.id !== "string" ||
    typeof raw.op !== "string"
  ) {
    return { ok: false, code: "malformed_message" };
  }
  if (raw.v !== RVF_IPC_VERSION) {
    return { ok: false, code: "malformed_message" };
  }
  if (raw.kind !== "intent") {
    return { ok: false, code: "malformed_message" };
  }
  if (!validOp(raw.op)) {
    return { ok: false, code: "malformed_message" };
  }
  return {
    ok: true,
    intent: {
      v: 1,
      kind: "intent",
      id: raw.id,
      op: raw.op,
      params: raw.params === undefined ? null : raw.params,
    },
  };
}

// ── events (Rust → frontend) ────────────────────────────────────────

export type EventClass = "result" | "job" | "notify" | "lifecycle" | "error";

export type JobState = "progress" | "finished" | "cancelled" | "failed";

export interface ErrorBodyWire {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
}

export type DecodedEvent =
  | { readonly class: "result"; readonly id: string; readonly ok: boolean; readonly data?: unknown; readonly error?: ErrorBodyWire }
  | { readonly class: "job"; readonly job: number; readonly state: JobState; readonly done?: number; readonly total?: number; readonly detail?: string; readonly data?: unknown; readonly error?: ErrorBodyWire }
  | { readonly class: "notify"; readonly seq: number; readonly data: unknown }
  | { readonly class: "lifecycle"; readonly state: string }
  | { readonly class: "error"; readonly error: ErrorBodyWire };

export type EventDecodeResult =
  | { readonly ok: true; readonly event: DecodedEvent }
  | { readonly ok: false; readonly code: "malformed_message" };

const JOB_STATES = new Set(["progress", "finished", "cancelled", "failed"]);

function parseErrorBody(v: unknown): ErrorBodyWire | undefined {
  if (!isObj(v)) return undefined;
  if (typeof v.code !== "string" || typeof v.message !== "string") {
    return undefined;
  }
  return {
    code: v.code,
    message: v.message,
    ...(v.details !== undefined ? { details: v.details } : {}),
  };
}

const malformed: EventDecodeResult = { ok: false, code: "malformed_message" };

/** Mirror of the `Event`/`Envelope` wire shape produced by
 *  `Event::to_json`. Unknown fields are tolerated (additive extension). */
export function decodeEvent(wire: unknown): EventDecodeResult {
  if (!isObj(wire)) return malformed;
  if (wire.v !== RVF_IPC_VERSION) return malformed;
  if (typeof wire.kind !== "string") return malformed;
  switch (wire.kind) {
    case "result": {
      if (typeof wire.id !== "string" || typeof wire.ok !== "boolean") {
        return malformed;
      }
      let error: ErrorBodyWire | undefined;
      if (wire.error !== undefined) {
        error = parseErrorBody(wire.error);
        if (error === undefined) return malformed;
      }
      return {
        ok: true,
        event: {
          class: "result",
          id: wire.id,
          ok: wire.ok,
          ...(wire.data !== undefined ? { data: wire.data } : {}),
          ...(error !== undefined ? { error } : {}),
        },
      };
    }
    case "job": {
      if (
        !(
          typeof wire.job === "number" &&
          Number.isInteger(wire.job) &&
          wire.job >= 0 &&
          Number.isSafeInteger(wire.job)
        )
      ) {
        return malformed;
      }
      if (typeof wire.state !== "string" || !JOB_STATES.has(wire.state)) {
        return malformed;
      }
      let error: ErrorBodyWire | undefined;
      if (wire.error !== undefined) {
        error = parseErrorBody(wire.error);
        if (error === undefined) return malformed;
      }
      return {
        ok: true,
        event: {
          class: "job",
          job: wire.job,
          state: wire.state as JobState,
          ...(typeof wire.done === "number" ? { done: wire.done } : {}),
          ...(typeof wire.total === "number" ? { total: wire.total } : {}),
          ...(typeof wire.detail === "string" ? { detail: wire.detail } : {}),
          ...(wire.data !== undefined ? { data: wire.data } : {}),
          ...(error !== undefined ? { error } : {}),
        },
      };
    }
    case "notify": {
      if (!(typeof wire.seq === "number" && Number.isInteger(wire.seq) && wire.seq >= 0)) {
        return malformed;
      }
      if (wire.data === undefined) return malformed;
      return { ok: true, event: { class: "notify", seq: wire.seq, data: wire.data } };
    }
    case "lifecycle": {
      if (typeof wire.state !== "string" || wire.state.length === 0) {
        return malformed;
      }
      return { ok: true, event: { class: "lifecycle", state: wire.state } };
    }
    case "error": {
      const error = parseErrorBody(wire.error);
      if (error === undefined) return malformed;
      return { ok: true, event: { class: "error", error } };
    }
    default:
      return malformed;
  }
}

// ── rvf://data ──────────────────────────────────────────────────────

/** Mirror of `BRIDGE_JS` `dataUrl`: each `/`-separated name segment is
 *  `encodeURIComponent`'d; `offset` defaults to 0; `len` only when
 *  given. */
export function dataUrl(name: string, offset?: number, len?: number): string {
  const q = new URLSearchParams({ offset: String(offset || 0) });
  if (len != null) q.set("len", String(len));
  const path = String(name).split("/").map(encodeURIComponent).join("/");
  return `rvf://data/${path}?${q}`;
}

/** Name rule from `serve_data`: non-empty, every `/`-separated segment
 *  non-empty and not `..`. `%2F` is NOT decoded (raw path). */
export function validDataName(rawPath: string): boolean {
  const name = rawPath.replace(/^\/+/, "");
  if (name.length === 0) return false;
  return !name.split("/").some((s) => s.length === 0 || s === "..");
}

/** Rust `u64::from_str` equivalent: optional leading `+`, ASCII digits
 *  only, ≤ u64::MAX. Whitespace, signs other than `+`, and junk fail. */
export function parseU64(s: string): bigint | undefined {
  if (!/^\+?[0-9]+$/.test(s)) return undefined;
  const v = BigInt(s);
  return v <= 0xffffffffffffffffn ? v : undefined;
}

export interface ParsedDataQuery {
  readonly offset: bigint;
  readonly len?: bigint;
}

/** Mirror of `parse_query`: `offset`/`len` as u64 (offset default 0);
 *  unknown keys ignored; a pair without `=` fails; empty pairs skip. */
export function parseDataQuery(
  query: string | undefined,
): { ok: true; value: ParsedDataQuery } | { ok: false } {
  let offset = 0n;
  let len: bigint | undefined;
  if (query === undefined) return { ok: true, value: { offset } };
  for (const pair of query.split("&")) {
    if (pair.length === 0) continue;
    const eq = pair.indexOf("=");
    if (eq < 0) return { ok: false };
    const key = pair.slice(0, eq);
    const value = pair.slice(eq + 1);
    if (key === "offset") {
      const v = parseU64(value);
      if (v === undefined) return { ok: false };
      offset = v;
    } else if (key === "len") {
      const v = parseU64(value);
      if (v === undefined) return { ok: false };
      len = v;
    }
    // unknown keys: ignored (contract)
  }
  return { ok: true, value: { offset, ...(len !== undefined ? { len } : {}) } };
}

export type DataErrorClass =
  | "bad_name"
  | "bad_params"
  | "out_of_range"
  | "not_found"
  | "read_failed";

export interface DataResponse {
  readonly status: number;
  readonly bodyLen: number;
  readonly total?: number;
  readonly errorClass?: DataErrorClass;
  readonly headers: Record<string, string>;
}

export interface DataSourceSpec {
  readonly name: string;
  readonly size: number;
}

/**
 * Model of `serve_data` in `ruviform-ui-webview/src/scheme.rs` against a
 * byte-source map. Returns the wire response contract: status, body
 * length (`min(len, total-offset, maxChunk)`), `X-Rvf-Total`, and the
 * fixed headers. Read-failure injection is out of scope here (the Rust
 * lane covers `500`).
 */
export function serveData(
  sources: ReadonlyMap<string, number>,
  maxChunk: bigint,
  rawPath: string,
  query: string | undefined,
): DataResponse {
  const headers: Record<string, string> = {};
  if (!validDataName(rawPath)) {
    return { status: 400, bodyLen: 0, errorClass: "bad_name", headers };
  }
  const name = rawPath.replace(/^\/+/, "");
  const parsed = parseDataQuery(query);
  if (!parsed.ok) {
    return { status: 400, bodyLen: 0, errorClass: "bad_params", headers };
  }
  const size = sources.get(name);
  if (size === undefined) {
    return { status: 404, bodyLen: 0, errorClass: "not_found", headers };
  }
  const total = BigInt(size);
  const { offset, len } = parsed.value;
  if (offset > total) {
    return { status: 400, bodyLen: 0, errorClass: "out_of_range", headers };
  }
  const avail = total - offset;
  const want = (len ?? avail) < avail ? (len ?? avail) : avail;
  const body = want < maxChunk ? want : maxChunk;
  headers["Content-Type"] = "application/octet-stream";
  headers["Access-Control-Allow-Origin"] = "*";
  headers["Cache-Control"] = "no-store";
  headers["X-Rvf-Total"] = total.toString();
  return {
    status: 200,
    bodyLen: Number(body),
    total: Number(total),
    headers,
  };
}
