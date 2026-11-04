/**
 * Command / Receipt inspection (Issue #310).
 *
 * The page-side gateway fake records every dispatched command intent
 * (`commands()`) and every delivered receipt (`receipts()`); receipts
 * move only under `drain(mode)` scripting. These helpers assert the
 * contract: one intent per user action, exactly one receipt per
 * command, no fabricated truth while a receipt is outstanding.
 */

import { expect } from "@playwright/test";
import type { HarnessSession } from "./harness.ts";
import type { DrainMode, RvfRecordedReceipt } from "./protocol.ts";

/** Serializable view of one `gateway.received` record. */
export interface RecordedCommand {
  readonly intent: {
    readonly kind?: string;
    readonly target?: string;
    readonly controlId?: string;
    readonly value?: { plain?: number } | unknown;
    readonly action?: string;
    readonly sequence?: number;
  };
  readonly ctx: {
    readonly commandId?: string;
    readonly viewId?: string;
    readonly generation?: number;
  };
}

/** Typed view of the `commands()` log. */
export async function commands(
  session: HarnessSession,
): Promise<RecordedCommand[]> {
  return (await session.commands()) as RecordedCommand[];
}

/** Commands for one `intent.controlId` / `intent.target`. */
export async function commandsFor(
  session: HarnessSession,
  filter: { controlId?: string; target?: string; kind?: string },
): Promise<RecordedCommand[]> {
  const all = await commands(session);
  return all.filter(
    (c) =>
      (filter.controlId === undefined ||
        c.intent.controlId === filter.controlId) &&
      (filter.target === undefined || c.intent.target === filter.target) &&
      (filter.kind === undefined || c.intent.kind === filter.kind),
  );
}

/** Receipts for one controlId, in delivery order. */
export async function receiptsFor(
  session: HarnessSession,
  controlId: string,
): Promise<RvfRecordedReceipt[]> {
  const all = await session.receipts();
  return all.filter((r) => r.controlId === controlId);
}

/** Drain `gateway.received` through the scripted receipt mode. */
export function drain(
  session: HarnessSession,
  mode: DrainMode,
): Promise<number> {
  return session.drain(mode);
}

/**
 * Assert the command log for `controlId` contains each expected kind
 * exactly once (dedup oracle — a re-entrant binding or double-attached
 * element would show up as a second identical intent).
 */
export async function expectNoDuplicateCommands(
  session: HarnessSession,
  controlId: string,
  expectedKinds: readonly string[],
): Promise<RecordedCommand[]> {
  const cmds = await commandsFor(session, { controlId });
  const kinds = cmds.map((c) => c.intent.kind ?? c.intent.action ?? "?");
  for (const kind of expectedKinds) {
    const n = kinds.filter((k) => k === kind).length;
    expect(n, `intent "${kind}" for ${controlId} must appear exactly once (got ${JSON.stringify(kinds)})`).toBe(1);
  }
  return cmds;
}

/**
 * Assert every delivered receipt for `controlId` is unique per
 * commandId — the exactly-once delivery side of the contract.
 */
export async function expectExactlyOnceReceipts(
  session: HarnessSession,
  controlId: string,
): Promise<RvfRecordedReceipt[]> {
  const receipts = await receiptsFor(session, controlId);
  const ids = receipts.map((r) => r.commandId);
  expect(new Set(ids).size, `receipt commandIds must be unique: ${JSON.stringify(ids)}`).toBe(ids.length);
  return receipts;
}
