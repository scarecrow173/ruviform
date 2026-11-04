/**
 * Issue #311 — `ui/binding` contract message validators.
 *
 * Structural validation of the five contract message shapes
 * (`CommandIntent`, `CommandReceipt`, `StateProjection`,
 * `ViewportQuery`, `QueryReply`) against the fixture corpus's
 * `binding-messages` suite. Hand-rolled (no zod-style dependency —
 * spec §2) and fail-closed on required fields; extra unknown fields
 * are tolerated on read but never validated away.
 *
 * Stable rejection vocabulary: `missing-field` | `bad-kind` |
 * `bad-type` | `out-of-range`.
 */

import type {
  CommandIntent,
  CommandReceipt,
  QueryReply,
  StateProjection,
  ViewportQuery,
} from "../../binding/src/contract.ts";

export type RejectReason =
  | "missing-field"
  | "bad-kind"
  | "bad-type"
  | "out-of-range";

export type Validation<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly reason: RejectReason };

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function isNonEmptyStr(v: unknown): v is string {
  return typeof v === "string" && v.length > 0;
}

function isSeq(v: unknown): v is number {
  return typeof v === "number" && Number.isInteger(v);
}

function isRevision(v: unknown): v is number | string {
  return typeof v === "number" || typeof v === "string";
}

function isControlValue(v: unknown): boolean {
  return (
    isObj(v) &&
    typeof v.plain === "number" &&
    Number.isFinite(v.plain) &&
    typeof v.normalized === "number" &&
    Number.isFinite(v.normalized)
  );
}

function fail(reason: RejectReason): { ok: false; reason: RejectReason } {
  return { ok: false, reason };
}

const COMMAND_KINDS = new Set(["begin", "preview", "commit", "cancel", "action"]);
const RECEIPT_STATUSES = new Set([
  "accepted",
  "queued",
  "applied",
  "rejected",
  "unsupported",
]);

/** Shared intent base: target/controlId/sequence + optional
 *  gestureId/originView/atomic typing. */
function checkIntentBase(m: Record<string, unknown>): RejectReason | undefined {
  if (!("target" in m)) return "missing-field";
  if (!isNonEmptyStr(m.target)) return "bad-type";
  if (!("controlId" in m)) return "missing-field";
  if (typeof m.controlId !== "string") return "bad-type";
  if (!("sequence" in m)) return "missing-field";
  if (!isSeq(m.sequence)) return "bad-type";
  if ((m.sequence as number) < 0) return "out-of-range";
  if ("gestureId" in m && typeof m.gestureId !== "string") return "bad-type";
  if ("originView" in m && typeof m.originView !== "string") return "bad-type";
  if ("atomic" in m && typeof m.atomic !== "boolean") return "bad-type";
  return undefined;
}

export function validateCommandIntent(
  m: unknown,
): Validation<CommandIntent> {
  if (!isObj(m)) return fail("bad-type");
  if (!("kind" in m)) return fail("missing-field");
  if (typeof m.kind !== "string" || !COMMAND_KINDS.has(m.kind)) {
    return fail("bad-kind");
  }
  const base = checkIntentBase(m);
  if (base !== undefined) return fail(base);
  switch (m.kind) {
    case "begin":
    case "preview":
    case "commit": {
      if (!("gestureId" in m)) return fail("missing-field");
      if (!("value" in m)) return fail("missing-field");
      if (!isControlValue(m.value)) return fail("bad-type");
      if (
        m.kind === "preview" &&
        "coalescible" in m &&
        m.coalescible !== true
      ) {
        return fail("bad-type");
      }
      break;
    }
    case "cancel": {
      if (!("gestureId" in m)) return fail("missing-field");
      if (!("reason" in m)) return fail("missing-field");
      if (typeof m.reason !== "string") return fail("bad-type");
      break;
    }
    case "action": {
      if (!("action" in m)) return fail("missing-field");
      if (!isNonEmptyStr(m.action)) return fail("bad-type");
      if ("source" in m && typeof m.source !== "string") return fail("bad-type");
      break;
    }
  }
  return { ok: true, value: m as unknown as CommandIntent };
}

export function validateReceipt(m: unknown): Validation<CommandReceipt> {
  if (!isObj(m)) return fail("bad-type");
  if (!("commandId" in m)) return fail("missing-field");
  if (!isNonEmptyStr(m.commandId)) return fail("bad-type");
  if (!("status" in m)) return fail("missing-field");
  if (typeof m.status !== "string" || !RECEIPT_STATUSES.has(m.status)) {
    return fail("bad-kind");
  }
  if ("revision" in m && !isRevision(m.revision)) return fail("bad-type");
  if ("detail" in m && typeof m.detail !== "string") return fail("bad-type");
  return { ok: true, value: m as unknown as CommandReceipt };
}

export function validateProjection<P = unknown>(
  m: unknown,
): Validation<StateProjection<P>> {
  if (!isObj(m)) return fail("bad-type");
  if (!("revision" in m)) return fail("missing-field");
  if (!isRevision(m.revision)) return fail("bad-type");
  if (!("value" in m)) return fail("missing-field");
  if ("provenance" in m && typeof m.provenance !== "string") {
    return fail("bad-type");
  }
  return { ok: true, value: m as unknown as StateProjection<P> };
}

export function validateViewportQuery(
  m: unknown,
): Validation<ViewportQuery> {
  if (!isObj(m)) return fail("bad-type");
  for (const f of ["queryId", "sourceId", "viewId"] as const) {
    if (!(f in m)) return fail("missing-field");
    if (!isNonEmptyStr(m[f])) return fail("bad-type");
  }
  if (!("sourceRevision" in m)) return fail("missing-field");
  if (!isRevision(m.sourceRevision)) return fail("bad-type");
  if (!("generation" in m)) return fail("missing-field");
  if (!isSeq(m.generation)) return fail("bad-type");
  if ((m.generation as number) < 0) return fail("out-of-range");
  for (const f of ["start", "end"] as const) {
    if (!(f in m)) return fail("missing-field");
    if (typeof m[f] !== "number" || !Number.isFinite(m[f] as number)) {
      return fail("bad-type");
    }
  }
  if ((m.start as number) > (m.end as number)) return fail("out-of-range");
  if ("buckets" in m) {
    if (!isSeq(m.buckets)) return fail("bad-type");
    if ((m.buckets as number) <= 0) return fail("out-of-range");
  }
  if ("channels" in m) {
    if (!Array.isArray(m.channels) || !m.channels.every((c) => isSeq(c) && c >= 0)) {
      return fail("bad-type");
    }
  }
  if ("maxBytes" in m) {
    if (!isSeq(m.maxBytes)) return fail("bad-type");
    if ((m.maxBytes as number) <= 0) return fail("out-of-range");
  }
  if ("encoding" in m) {
    if (
      !Array.isArray(m.encoding) ||
      !m.encoding.every((e) => typeof e === "string")
    ) {
      return fail("bad-type");
    }
  }
  return { ok: true, value: m as unknown as ViewportQuery };
}

export function validateQueryReply<T = unknown>(
  m: unknown,
): Validation<QueryReply<T>> {
  if (!isObj(m)) return fail("bad-type");
  if (!("generation" in m)) return fail("missing-field");
  if (!isSeq(m.generation)) return fail("bad-type");
  if (!("sourceRevision" in m)) return fail("missing-field");
  if (!isRevision(m.sourceRevision)) return fail("bad-type");
  if (!("payload" in m)) return fail("missing-field");
  if ("bytes" in m) {
    if (!isSeq(m.bytes)) return fail("bad-type");
    if ((m.bytes as number) < 0) return fail("out-of-range");
  }
  return { ok: true, value: m as unknown as QueryReply<T> };
}
