/**
 * `rvf-*` CustomEvent inspection + assertions (Issue #310).
 *
 * The page-side recorder (`page-src/rvf-page.ts`) captures every
 * component intent CustomEvent at the document level — these helpers
 * turn that log into the semantic oracles the acceptance criteria
 * need: exactly-one gesture termination, intact event-init flags
 * (`detail`/`bubbles`/`composed` crossing the shadow boundary), and a
 * monotonic per-control sequence.
 */

import { expect } from "@playwright/test";
import type { HarnessSession } from "./harness.ts";
import type { RecordedRvfEvent } from "./protocol.ts";

/** `rvf-gesture` detail shape (headless InteractionEvent verbatim). */
export interface GestureDetail {
  readonly type: string;
  readonly meta?: {
    readonly controlId: string;
    readonly target?: string;
    readonly originView?: string;
    readonly gestureId?: string;
    readonly sequence: number;
    readonly source?: string;
    /** `true` on the synthesized begin/commit pair of an atomic input. */
    readonly atomic?: boolean;
  };
  readonly value?: unknown;
  readonly reason?: string;
}

/** Recorded `rvf-gesture` events, optionally for one controlId. */
export async function gestureEvents(
  session: HarnessSession,
  controlId?: string,
): Promise<RecordedRvfEvent[]> {
  const evts = await session.events("rvf-gesture");
  if (controlId === undefined) return evts;
  return evts.filter((e) => e.controlId === controlId);
}

/** Typed detail view of one recorded event. */
export function gestureDetail(e: RecordedRvfEvent): GestureDetail {
  return e.detail as GestureDetail;
}

/** Gesture event types in occurrence order. */
export function gestureTypes(evts: RecordedRvfEvent[]): string[] {
  return evts.map((e) => gestureDetail(e).type);
}

/**
 * Assert a gesture's recorded `rvf-gesture` stream is exactly
 * `begin` → `preview{0..N}` → one terminal (`commit`/`cancel`):
 * same gestureId end-to-end, non-decreasing `meta.sequence`, and the
 * terminal is the gesture's last meaningful event. Events the
 * contract emits *after* the terminal for the same id (a stale-id
 * `rejected`/`noop`, e.g. a pointer released after Escape-cancel)
 * are ignored — they are terminal-adjacent, not a second terminal.
 * `expectPreview` bounds preview count (`true` = ≥1, `false` = 0).
 */
export function expectGestureSequence(
  evts: RecordedRvfEvent[],
  opts: {
    terminal: "commit" | "cancel";
    from?: number;
    controlId?: string;
    expectPreview?: boolean;
  },
): void {
  const seq = evts.map((e) => gestureDetail(e));
  const termIdx = seq.findIndex(
    (s) => s.type === "commit" || s.type === "cancel",
  );
  expect(
    termIdx,
    `gesture stream has no terminal: ${JSON.stringify(seq.map((s) => s.type))}`,
  ).toBeGreaterThanOrEqual(0);
  // Everything up to and including the first terminal is the gesture;
  // anything later for the same id is terminal-adjacent.
  const slice = seq.slice(0, termIdx + 1);
  expect(
    slice.length,
    `gesture stream: ${JSON.stringify(seq.map((s) => s.type))}`,
  ).toBeGreaterThanOrEqual(2);
  const begin = slice[0];
  expect(begin?.type).toBe("begin");
  const gid = begin?.meta?.gestureId;
  expect(gid, "begin must carry a gestureId").toBeTruthy();

  const mid = slice.slice(1, -1);
  for (const m of mid) {
    expect(
      m.type,
      `non-terminal event must be preview: ${JSON.stringify(seq.map((s) => s.type))}`,
    ).toBe("preview");
    expect(m.meta?.gestureId).toBe(gid);
  }
  const previews = mid.filter((s) => s.type === "preview");
  if (opts.expectPreview === true) expect(previews.length).toBeGreaterThan(0);
  if (opts.expectPreview === false) expect(previews.length).toBe(0);

  const terminal = slice[slice.length - 1];
  expect(terminal?.type).toBe(opts.terminal);
  expect(terminal?.meta?.gestureId).toBe(gid);

  // Exactly-one termination across the whole gesture id — later
  // events for the same id may be `rejected`/`noop`, never a second
  // terminal.
  const gidEvents = seq.filter((s) => s.meta?.gestureId === gid);
  const terminals = gidEvents.filter(
    (s) => s.type === "commit" || s.type === "cancel",
  );
  expect(terminals.length, "exactly one terminal").toBe(1);

  if (opts.controlId !== undefined) {
    expect(begin?.meta?.controlId).toBe(opts.controlId);
    expect(terminal?.meta?.controlId).toBe(opts.controlId);
  }

  const sequences = gidEvents
    .map((s) => s.meta?.sequence)
    .filter((s): s is number => s !== undefined);
  const sorted = [...sequences].sort((a, b) => a - b);
  expect(
    sequences,
    `sequence must be non-decreasing: ${JSON.stringify(sequences)}`,
  ).toEqual(sorted);
}

/**
 * Assert every recorded event crossed the shadow boundary as a
 * bubbling composed CustomEvent carrying a non-null `detail` — the
 * published `rvf-*` event-init contract.
 */
export function expectComposedInit(evts: RecordedRvfEvent[]): void {
  expect(evts.length).toBeGreaterThan(0);
  for (const e of evts) {
    expect(e.bubbles, `${e.type} must bubble`).toBe(true);
    expect(e.composed, `${e.type} must be composed`).toBe(true);
    expect(e.detail, `${e.type} must carry detail`).not.toBeNull();
  }
}

/** `rvf-transport` detail shape. */
export interface TransportDetail {
  readonly action: string;
  readonly controlId: string;
  readonly sequence: number;
  readonly source?: string;
}

/** Recorded `rvf-transport` events for one controlId. */
export async function transportEvents(
  session: HarnessSession,
  controlId?: string,
): Promise<RecordedRvfEvent[]> {
  const evts = await session.events("rvf-transport");
  if (controlId === undefined) return evts;
  return evts.filter((e) => e.controlId === controlId);
}

export function transportDetail(e: RecordedRvfEvent): TransportDetail {
  return e.detail as TransportDetail;
}
