/**
 * Issue #311 — seeded fault-matrix lane.
 *
 * The seeded generators (`src/seed.ts`) produce fault scenarios:
 * receipt storms (reorder/duplicate/unknown id) and detach/reattach
 * interleaves with regressed projections. Each generated `in` list is
 * injected into the REAL binding objects and judged against the
 * independent ContractModel oracle — the same contract check as the
 * pinned scenario corpus, applied to arbitrary fault permutations.
 *
 * Covers AC: delay/reorder/dup ack, detach/reattach, stale revision,
 * unknown ids — plus seeded determinism (identical seed → identical
 * trace on two runs).
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { genAckStorm, genAttachStorm, mulberry32 } from "../src/seed.ts";
import { runScenario, type ScenarioDoc, type ScenarioSetup } from "../src/replay.ts";

const DAW_SETUP: ScenarioSetup = {
  consumer: "daw",
  viewId: "view-1",
  windowId: "win-1",
  ackTimeoutMs: 5000,
  targets: [{ control: "fader-1", target: "mixer.ch1.fader", initial: 0.5 }],
};

/**
 * genAckStorm refs are `cmd-N` (0-based) — minted commandIds are
 * `cmd-N` starting at 1, so the c-th generated ref maps to `cmd-(N+1)`
 * after `commands` minting actions precede the storm.
 */
function withMintedRefs(ops: { op: string; [k: string]: unknown }[], commands: number) {
  const ins: Record<string, unknown>[] = [];
  for (let c = 0; c < commands; c += 1) {
    ins.push({ op: "action", name: `storm.action-${c}` });
  }
  ins.push({ op: "flush" });
  for (const op of ops) {
    const next = { ...op } as Record<string, unknown>;
    const ref = String(next.ref ?? "");
    const m = /^cmd-(\d+)$/.exec(ref);
    if (m) next.ref = `cmd-${Number(m[1]) + 1}`;
    ins.push(next);
  }
  return ins;
}

describe("seeded fault matrix", () => {
  it("mulberry32 is deterministic per seed", () => {
    const a = mulberry32(42);
    const b = mulberry32(42);
    for (let i = 0; i < 20; i += 1) assert.equal(a(), b());
  });

  it("identical seed → bit-identical generated op list", () => {
    assert.deepEqual(genAckStorm(7, 4), genAckStorm(7, 4));
    assert.deepEqual(genAttachStorm(13, 3), genAttachStorm(13, 3));
    assert.notDeepEqual(genAckStorm(7, 4), genAckStorm(8, 4));
  });

  for (const seed of [1, 7, 42, 99]) {
    it(`ack storm seed=${seed}: real path matches oracle, deterministic`, async () => {
      const generated = genAckStorm(seed, 3);
      const doc = {
        v: 1,
        suite: "scenario-binding",
        id: `ack-storm-${seed}`,
        setup: DAW_SETUP,
        in: withMintedRefs(generated.in, 3),
        expect: {},
      } as unknown as ScenarioDoc;

      const { actual, expected } = await runScenario(doc);
      assert.deepStrictEqual(actual, expected, `seed ${seed} oracle mismatch`);

      const again = await runScenario(doc);
      assert.deepStrictEqual(again.actual, actual, `seed ${seed} not deterministic`);
    });
  }

  for (const seed of [3, 17, 55]) {
    it(`attach storm seed=${seed}: stale projections + reattach match oracle`, async () => {
      const generated = genAttachStorm(seed, 3);
      const doc = {
        v: 1,
        suite: "scenario-binding",
        id: `attach-storm-${seed}`,
        setup: DAW_SETUP,
        in: generated.in as unknown as Record<string, unknown>[],
        expect: {},
      } as unknown as ScenarioDoc;

      const { actual, expected } = await runScenario(doc);
      assert.deepStrictEqual(actual, expected, `seed ${seed} oracle mismatch`);

      const again = await runScenario(doc);
      assert.deepStrictEqual(again.actual, actual, `seed ${seed} not deterministic`);
    });
  }

  it("storm receipts never mint ghost commands and terminals stay terminal", async () => {
    // A storm with guaranteed unknown ids — unknown receipts must
    // classify stale, never mint new commands nor resurrect terminals.
    const generated = genAckStorm(7, 2);
    const doc = {
      v: 1,
      suite: "scenario-binding",
      id: "ack-storm-terminal",
      setup: DAW_SETUP,
      in: [
        ...withMintedRefs(generated.in, 2),
        // Explicit terminal sequence on the first minted command.
        { op: "receipt", status: "applied", ref: "cmd-1" },
        { op: "receipt", status: "rejected", ref: "cmd-1" },
        // Lower-rank echoes after terminal — must not reapply.
        { op: "receipt", status: "accepted", ref: "cmd-1" },
        { op: "receipt", status: "queued", ref: "cmd-1" },
      ] as Record<string, unknown>[],
      expect: {},
    } as unknown as ScenarioDoc;

    const { actual, expected } = await runScenario(doc);
    assert.deepStrictEqual(actual, expected);

    // Both receipts minted only by actions — storm must not invent ids.
    const minted = new Set(actual.dispatched.map((d) => d.commandId));
    assert.ok(minted.size === 2, `expected 2 minted commands, got ${minted.size}`);
    // Terminal state preserved: a final phase exists and the trace
    // equals the oracle's judgment (deep-equal above is the check).
    assert.ok(actual.phases.length > 0);
  });
});
