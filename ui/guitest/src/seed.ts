/**
 * Issue #311 — seeded fault-scenario generator.
 *
 * A deterministic PRNG (mulberry32) derives fault order/type/delay from
 * the seed; identical seeds produce bit-identical `in` op lists —
 * `assertDeterministic` and the fixtures' `seed` field rely on it.
 */

export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export interface ScenarioInOp {
  readonly op: string;
  readonly [k: string]: unknown;
}

export interface GeneratedScenario {
  readonly seed: number;
  readonly in: ScenarioInOp[];
}

const ACK_STATUSES = ["accepted", "queued", "applied"] as const;

function pick<T>(r: () => number, xs: readonly T[]): T {
  return xs[Math.floor(r() * xs.length)]!;
}

/**
 * Generate a receipt-storm scenario over `commands` sequential
 * receipt-expecting dispatches (commits or actions): each command gets
 * 1–3 receipts drawn from the ack vocabulary with a chance of
 * duplicates, reordered accepted-after-applied, and unknown commandIds
 * (`cmd-*` refs the driver resolves).
 *
 * `refKind`: `"concrete"` resolves refs to `cmd-N` ids the scenario
 * producer must guarantee exist; `"driver"` leaves symbolic refs
 * (`last`/`first`/`dead-epoch`/`unknown`) for the driver to resolve.
 */
export function genAckStorm(seed: number, commands = 3): GeneratedScenario {
  const r = mulberry32(seed);
  const ins: ScenarioInOp[] = [];
  for (let c = 0; c < commands; c += 1) {
    const n = 1 + Math.floor(r() * 3);
    const statuses: string[] = [];
    for (let i = 0; i < n; i += 1) {
      statuses.push(pick(r, ACK_STATUSES));
    }
    for (const s of statuses) {
      ins.push({ op: "receipt", status: s, ref: `cmd-${c}` });
    }
    // occasional duplicate of the last receipt
    if (r() < 0.4) {
      ins.push({
        op: "receipt",
        status: statuses[statuses.length - 1],
        ref: `cmd-${c}`,
      });
    }
    // occasional unknown command id (stale classification expected)
    if (r() < 0.3) {
      ins.push({
        op: "receipt",
        status: pick(r, ACK_STATUSES),
        ref: "unknown",
        commandId: `cmd-unknown-${Math.floor(r() * 1000)}`,
      });
    }
  }
  return { seed, in: ins };
}

/**
 * Generate a detach/reattach + projection interleave: `pairs`
 * detach/attach cycles with projections at seeded revisions (some
 * regressed to exercise the stale guard).
 */
export function genAttachStorm(
  seed: number,
  pairs = 2,
  target = "mixer.ch1.fader",
): GeneratedScenario {
  const r = mulberry32(seed);
  const ins: ScenarioInOp[] = [];
  let rev = 0;
  for (let i = 0; i < pairs; i += 1) {
    rev += 1 + Math.floor(r() * 3);
    ins.push({
      op: "projection",
      target,
      revision: rev,
      value: { plain: r(), normalized: r() },
    });
    if (r() < 0.35) {
      // regressed revision — must be stale-dropped
      ins.push({
        op: "projection",
        target,
        revision: Math.max(0, rev - 1 - Math.floor(r() * 2)),
        value: { plain: 0, normalized: 0 },
      });
    }
    ins.push({ op: "detach", reason: "dock-transfer" });
    ins.push({ op: "attach", windowId: `win-${i + 2}` });
  }
  return { seed, in: ins };
}
