/**
 * Issue #261 — bounded latest-wins coalescing for continuous previews.
 *
 * Rules (Issue #257 §4/§5, Issue #261 contract):
 * - Only `preview` commands coalesce, and only within a gesture — the
 *   continuous-control semantics that make latest-wins safe.
 * - `begin` / `commit` / `cancel` / `action` are NEVER dropped or
 *   reordered: a gesture's first and last commands always land, and
 *   terminal notifications survive any preview volume.
 * - A buffered preview dies when its gesture's `commit`/`cancel`
 *   arrives — the terminal supersedes every tentative value before it.
 * - Coalescing is *measured*: {@link IntentQueue.stats} records how
 *   many previews were folded so the bound is observable in tests and
 *   diagnostics.
 */

import type { GestureId } from "../../core/src/index.ts";
import type { CommandIntent } from "./contract.ts";

export interface IntentQueueStats {
  /** Intents pushed since the queue was created. */
  readonly received: number;
  /** Intents handed to the dispatch port. */
  readonly emitted: number;
  /** Previews folded into a newer preview of the same gesture. */
  readonly coalesced: number;
  /** Previews dropped because their gesture terminated first. */
  readonly superseded: number;
}

type Slot = CommandIntent | undefined;

/**
 * Ordered intent buffer. Non-preview commands append strictly;
 * previews replace the still-buffered preview of the same gesture.
 */
export class IntentQueue {
  private queue: Slot[] = [];
  /** gestureId → index of its buffered preview (index never shifts —
   * removals tombstone instead). */
  private readonly pending = new Map<GestureId, number>();
  private received_ = 0;
  private emitted_ = 0;
  private coalesced_ = 0;
  private superseded_ = 0;

  get stats(): IntentQueueStats {
    return {
      received: this.received_,
      emitted: this.emitted_,
      coalesced: this.coalesced_,
      superseded: this.superseded_,
    };
  }

  get size(): number {
    return this.queue.length;
  }

  push(cmd: CommandIntent): void {
    this.received_ += 1;
    if (cmd.kind === "preview") {
      const at = this.pending.get(cmd.gestureId);
      if (at !== undefined) {
        this.queue[at] = cmd;
        this.coalesced_ += 1;
        return;
      }
      this.pending.set(cmd.gestureId, this.queue.length);
      this.queue.push(cmd);
      return;
    }
    if (cmd.kind === "commit" || cmd.kind === "cancel") {
      const at = this.pending.get(cmd.gestureId);
      if (at !== undefined) {
        this.queue[at] = undefined;
        this.pending.delete(cmd.gestureId);
        this.superseded_ += 1;
      }
    }
    this.queue.push(cmd);
  }

  /** Drain everything, in order. Tombstones are skipped. */
  drain(): CommandIntent[] {
    if (this.queue.length === 0) return [];
    const out: CommandIntent[] = [];
    for (const slot of this.queue) {
      if (slot !== undefined) out.push(slot);
    }
    this.queue = [];
    this.pending.clear();
    this.emitted_ += out.length;
    return out;
  }

  /** Drop everything without emitting (detach/teardown path). */
  purge(): void {
    this.queue = [];
    this.pending.clear();
  }
}
