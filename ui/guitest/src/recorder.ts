/**
 * Issue #311 — `IpcRecorder`: opt-in, allowlisted, bounded observation
 * of the `BindingPort` boundary.
 *
 * Contract (spec §4 / Issue acceptance AC-4):
 * - **opt-in**: `enabled` defaults to `false`; a disabled recorder's
 *   `record`/`wrapPort` are transparent no-ops — no allocation, no
 *   observable behavior change.
 * - **allowlisted**: only the enumerated `jsonl.ts` record types and
 *   field names can be written; unknown record types throw.
 * - **bounded**: `maxEntries`/`maxBytes` cap the artifact; the first
 *   overflow emits a single `sys/dropped` marker and every further
 *   record is counted in {@link dropped} — bounds are never crossed
 *   silently.
 * - **redaction by omission**: value payloads, media, prompts, grants,
 *   tokens, and bulk binary are never carried — the field vocabulary
 *   has no slot for them; strings truncate at 128 chars.
 */

import type {
  BindingPort,
  CommandIntent,
  CommandReceipt,
  DispatchContext,
  QueryId,
  QueryReply,
  StateProjection,
  SubscriptionContext,
  Unsubscribe,
  ViewportQuery,
} from "../../binding/src/contract.ts";
import {
  parseRecording,
  serializeRecord,
  type RecordDir,
} from "./jsonl.ts";

export interface IpcRecorderOptions {
  /** Default `false` — recording is opt-in. */
  readonly enabled?: boolean;
  /** Max record lines retained (default 4096). */
  readonly maxEntries?: number;
  /** Max total serialized bytes (default 256 KiB). */
  readonly maxBytes?: number;
}

export class IpcRecorder {
  readonly enabled: boolean;
  private readonly maxEntries: number;
  private readonly maxBytes: number;
  private readonly lines: string[] = [];
  private bytes = 0;
  private tick_ = 0;
  private dropped_ = 0;
  private markerEmitted = false;

  constructor(options: IpcRecorderOptions = {}) {
    this.enabled = options.enabled === true;
    this.maxEntries = options.maxEntries ?? 4096;
    this.maxBytes = options.maxBytes ?? 256 * 1024;
  }

  /** Number of records suppressed by the capacity bounds. */
  get dropped(): number {
    return this.dropped_;
  }

  /** Number of records retained. */
  get size(): number {
    return this.lines.length;
  }

  /**
   * Append one record. Only `dir`/`type` + allowlisted fields are
   * serialized; unknown types throw (vocabulary drift must fail).
   * Capacity overflow emits a single `sys/dropped` marker then counts.
   */
  record(dir: RecordDir, type: string, fields: Record<string, unknown>): void {
    if (!this.enabled) return;
    const line = serializeRecord(this.tick_, dir, type, fields);
    // Reserve the last entry for the eventual `sys/dropped` marker so
    // entries-bound overflow always lands an explicit marker. Byte
    // overflow can't be pre-reserved — the marker is emitted if it
    // still fits the byte budget.
    const needed = this.markerEmitted ? 1 : 2;
    if (
      this.lines.length + needed > this.maxEntries ||
      this.bytes + line.length + 1 > this.maxBytes
    ) {
      this.dropped_ += 1;
      if (!this.markerEmitted && this.lines.length < this.maxEntries) {
        this.markerEmitted = true;
        const marker = serializeRecord(this.tick_, "sys", "dropped", {
          reason: "capacity",
          count: this.dropped_,
        });
        if (this.bytes + marker.length + 1 <= this.maxBytes) {
          this.lines.push(marker);
          this.bytes += marker.length + 1;
        }
      }
      return;
    }
    this.lines.push(line);
    this.bytes += line.length + 1;
    this.tick_ += 1;
  }

  /** Serialize the retained records as `*.v1.jsonl` text. */
  toJsonl(): string {
    return this.lines.length === 0 ? "" : this.lines.join("\n") + "\n";
  }

  /**
   * Wrap a `BindingPort` so every boundary crossing is recorded.
   * A disabled recorder returns the port unchanged — zero observation
   * cost (AC-4: collection OFF is behavior-identical).
   */
  wrapPort(port: BindingPort): BindingPort {
    if (!this.enabled) return port;
    const rec = this;
    return {
      dispatch(intent: CommandIntent, ctx: DispatchContext): void {
        rec.record("out", "dispatch", {
          commandId: ctx.commandId,
          kind: intent.kind,
          target: intent.target,
          controlId: intent.controlId,
          sequence: intent.sequence,
          ...(intent.kind !== "action" && "gestureId" in intent
            ? { gestureId: intent.gestureId }
            : {}),
          generation: ctx.generation,
          expectReceipt: ctx.expectReceipt,
          ...(ctx.viewId !== undefined ? { viewId: ctx.viewId } : {}),
          ...(ctx.windowId !== undefined ? { windowId: ctx.windowId } : {}),
        });
        port.dispatch(intent, ctx);
      },
      ...(port.subscribe !== undefined
        ? {
            subscribe<P>(
              this: BindingPort,
              target: string,
              listener: (p: StateProjection<P>) => void,
              ctx: SubscriptionContext,
            ): Unsubscribe | undefined {
              rec.record("out", "subscribe", {
                target,
                viewId: ctx.viewId,
                generation: ctx.generation,
                ...(ctx.windowId !== undefined
                  ? { windowId: ctx.windowId }
                  : {}),
              });
              const off = port.subscribe!(target, listener, ctx);
              return off === undefined
                ? undefined
                : () => {
                    rec.record("out", "unsubscribe", { target });
                    off();
                  };
            },
          }
        : {}),
      ...(port.query !== undefined
        ? {
            async query<T>(
              this: BindingPort,
              query: ViewportQuery,
              signal?: AbortSignal,
            ): Promise<QueryReply<T>> {
              rec.record("out", "query", {
                as: query.queryId,
                sourceId: query.sourceId,
                sourceRevision: query.sourceRevision,
                start: query.start,
                end: query.end,
              });
              return port.query!(query, signal);
            },
          }
        : {}),
      ...(port.cancelQuery !== undefined
        ? {
            cancelQuery(queryId: QueryId): void {
              rec.record("out", "queryOutcome", {
                as: queryId,
                outcome: "cancel-request",
              });
              port.cancelQuery!(queryId);
            },
          }
        : {}),
    };
  }
}

/** Round-trip helper for tests: serialize then parse back. */
export function verifyJsonl(text: string): { ok: boolean; error?: string } {
  const r = parseRecording(text);
  return r.ok ? { ok: true } : { ok: false, error: r.error };
}
