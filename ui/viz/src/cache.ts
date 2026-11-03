/**
 * Byte-budgeted LRU view cache (the JS-side analogue of
 * `ruviform-viz` `BoundedCache`).
 *
 * View responses are cached under keys the caller builds
 * (`waveformViewKey`). The budget bounds *estimated payload bytes*, not
 * entry count — the caller supplies the estimate so the same cache can
 * hold different view shapes. Style is never part of a key: a theme
 * swap must not evict or re-query data (§12.5 paint/source separation —
 * verified by the theme-switch regression tests).
 */

export class LruViewCache<V> {
  private readonly map = new Map<string, { v: V; bytes: number }>();
  private used = 0;
  private hits_ = 0;
  private misses_ = 0;

  readonly maxBytes: number;

  constructor(maxBytes: number) {
    if (!(maxBytes > 0)) throw new Error("maxBytes must be > 0");
    this.maxBytes = maxBytes;
  }

  get(key: string): V | undefined {
    const e = this.map.get(key);
    if (e === undefined) {
      this.misses_ += 1;
      return undefined;
    }
    // LRU touch: re-insert at the end of insertion order.
    this.map.delete(key);
    this.map.set(key, e);
    this.hits_ += 1;
    return e.v;
  }

  /** Insert (or refresh) an entry; evicts least-recently-used entries
   * until the estimate fits. An entry larger than the whole budget is
   * still stored (evicting everything else) — the caller's bound comes
   * from the query contract, not from this cache. */
  set(key: string, v: V, bytes: number): void {
    // A non-finite estimate would poison `used` (NaN <= maxBytes is
    // false forever → the eviction loop drains every entry on each
    // set). Treat it as a minimal entry instead of corrupting the cache.
    const b = Number.isFinite(bytes) && bytes > 0 ? bytes : 1;
    const prev = this.map.get(key);
    if (prev !== undefined) this.used -= prev.bytes;
    this.map.delete(key);
    this.map.set(key, { v, bytes: b });
    this.used += b;
    for (const [k, e] of this.map) {
      if (this.used <= this.maxBytes) break;
      if (k === key && this.map.size === 1) break;
      this.used -= e.bytes;
      this.map.delete(k);
    }
  }

  clear(): void {
    this.map.clear();
    this.used = 0;
  }

  get hits(): number {
    return this.hits_;
  }
  get misses(): number {
    return this.misses_;
  }
  get bytes(): number {
    return this.used;
  }
  get size(): number {
    return this.map.size;
  }
}

/** Deterministic cache key for a waveform view: identity + revision +
 * geometry only — never style. */
export function waveformViewKey(
  sourceId: string,
  revision: number | string,
  channel: number,
  start: number,
  end: number,
  buckets: number,
): string {
  return `${sourceId}@${String(revision)}:${channel}:${start}:${end}:${buckets}`;
}
