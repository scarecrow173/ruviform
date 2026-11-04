/**
 * State-arrival polling (Issue #310).
 *
 * The harness never fixed-sleeps: every wait is "poll until the state
 * arrived or the deadline passed". `waitFor` runs a Node-side poll —
 * the page's own timers may be virtualized by `page.clock`, so the
 * poll cadence lives in Node where it cannot be frozen.
 */

export interface WaitForOptions {
  /** Overall deadline in ms (default 5000). */
  readonly timeout?: number;
  /** Poll interval in ms (default 25). */
  readonly interval?: number;
  /** Label used in the timeout error. */
  readonly label?: string;
}

const sleep = (ms: number): Promise<void> =>
  new Promise((r) => setTimeout(r, ms));

/**
 * Poll `probe` until it returns a truthy value (returned) or
 * `timeout` elapses (throws with `label` and the last result).
 */
export async function waitFor<T>(
  probe: () => Promise<T>,
  opts: WaitForOptions = {},
): Promise<T> {
  const timeout = opts.timeout ?? 5000;
  const interval = opts.interval ?? 25;
  const deadline = Date.now() + timeout;
  let last: T | undefined;
  let lastErr: unknown;
  for (;;) {
    try {
      last = await probe();
      if (last) return last;
      lastErr = undefined;
    } catch (err) {
      // A throwing probe is "state not arrived yet" — the page may
      // still be mounting or the API mid-swap during a remount.
      lastErr = err;
    }
    if (Date.now() >= deadline) {
      throw new Error(
        `waitFor timeout (${timeout}ms)${opts.label ? `: ${opts.label}` : ""}` +
          ` — last: ${JSON.stringify(last)}` +
          (lastErr !== undefined
            ? ` — last error: ${lastErr instanceof Error ? lastErr.message : String(lastErr)}`
            : ""),
      );
    }
    await sleep(interval);
  }
}
