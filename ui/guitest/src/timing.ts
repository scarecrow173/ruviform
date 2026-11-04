/**
 * Issue #311 — deterministic scheduling primitives for replay.
 *
 * Same contract shape as the binding tests' manual timers: a logical
 * clock (`now`) plus an `advance(ms)` that fires due timers in schedule
 * order. Wall-clock time is never read — replay determinism (AC-2)
 * requires every timer decision to come from the injected clock.
 */

export class ManualTimers {
  now = 0;
  private seq = 0;
  private readonly timers = new Map<number, { fn: () => void; at: number }>();

  readonly schedule = (fn: () => void, ms: number): (() => void) => {
    const id = ++this.seq;
    this.timers.set(id, { fn, at: this.now + ms });
    return () => {
      this.timers.delete(id);
    };
  };

  /** Advance the clock; due timers fire in schedule order. */
  advance(ms: number): void {
    this.now += ms;
    for (;;) {
      let dueId: number | undefined;
      for (const [id, t] of this.timers) {
        if (t.at <= this.now && (dueId === undefined || id < dueId)) {
          dueId = id;
        }
      }
      if (dueId === undefined) return;
      const t = this.timers.get(dueId);
      this.timers.delete(dueId);
      t?.fn();
    }
  }

  get pending(): number {
    return this.timers.size;
  }
}

/** Deferred flush — collects flush requests for manual draining. */
export function deferredFlush(): {
  readonly flushWith: (fn: () => void) => void;
  readonly pending: () => number;
  readonly runAll: () => void;
} {
  const fns: Array<() => void> = [];
  return {
    flushWith: (fn) => fns.push(fn),
    pending: () => fns.length,
    runAll: () => {
      for (const fn of fns.splice(0)) fn();
    },
  };
}
