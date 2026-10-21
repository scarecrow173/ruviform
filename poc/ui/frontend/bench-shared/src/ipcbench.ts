// メッセージ境界計測（AC4、FW 非依存）。
// Rust 側 exp/ipc.rs が evaluate_script で __poc.start(cfg) を呼ぶと、
// JSON RTT → 単方向 ingest → バイナリ pull → push 観測 → 3 経路併用（coexist）
// の順で測定し、完了時に onDone(stats) → done メッセージを返す。
// Rust → JS 方向の受け口は __poc.pong(seq) / __poc.push(seq) / __poc.pushDone()。
//
// 計測の分担（design/test-spec 正本）:
// - ipc_rtt_ms{avg,p50,p99}: rtt_ping の seq 往復時間（ペイロード 64B〜64KiB）
// - ipc_ingest_msg_s       : ingest フラッドの Rust 側到達レート
// - protocol_pull_MBps     : fetch(poc://data/blob) → arrayBuffer 完走
// - push_msg_s + 観察       : evaluate_script push の受信間隔・ドロップ・coalesce
// percentile 集計は Rust 側 metrics.rs の責務 — ここでは生サンプル列を返す。

/** Rust が start cfg で注入するパラメータ（正本は exp/ipc.rs）。 */
export interface IpcBenchConfig {
  rtt_sizes: number[];
  rtt_iters: number;
  ingest_n: number;
  pull_sizes: number[];
  pull_iters: number[];
  push_burst: number;
  push_paced: number;
  push_coexist: number;
  push_tick_ms: number;
  coexist_seq_base: number;
  coexist_rtt_iters: number;
  /** [bytes, iters] の組。 */
  coexist_pull: [number, number][];
  stall_ms: number;
}

/** start cfg の既定値（Rust 側注入値が常に優先。欠落時の防御）。 */
const DEFAULTS: IpcBenchConfig = {
  rtt_sizes: [64, 1024, 16384, 65536],
  rtt_iters: 32,
  ingest_n: 500,
  pull_sizes: [65536, 1048576, 16777216],
  pull_iters: [16, 8, 3],
  push_burst: 200,
  push_paced: 160,
  push_coexist: 96,
  push_tick_ms: 4,
  coexist_seq_base: 1000,
  coexist_rtt_iters: 8,
  coexist_pull: [
    [65536, 4],
    [1048576, 2],
  ],
  stall_ms: 2500,
};

export interface IpcBenchHooks {
  /** 進捗表示先（任意）。 */
  hud?: HTMLElement | null;
  /** 完了時の done.stats コールバック。 */
  onDone: (stats: Record<string, unknown>) => void;
  /** 致命的失敗（bench_error）コールバック。 */
  onError: (message: string) => void;
  /** IPC postMessage 送信口（テスト差し替え用に注入）。 */
  post: (obj: unknown) => void;
}

export type IpcBenchState = "idle" | "running" | "done" | "error";

/** push 1 件の受信記録。`tag` は macrotask 境界スタンプ（coalesce 検出用）。 */
interface PushRec {
  seq: number;
  t: number;
  tag: number;
}

/** `poc://data` 名前空間の origin。
 * Windows では wry が `poc://app/...` を `http://poc.app/...` に書き換えるため、
 * ページ origin が http: 系なら `http://poc.data`、素の `poc:` なら `poc://data`。 */
function dataBase(): string {
  return location.protocol === "poc:" ? "poc://data" : "http://poc.data";
}

/** Promise にタイムアウトを付ける（到達しなければ null）。 */
function withTimeout<T>(p: Promise<T>, ms: number): Promise<T | null> {
  return Promise.race([
    p,
    new Promise<null>((resolve) => setTimeout(() => resolve(null), ms)),
  ]);
}

/** push 受信記録の集約（受信間隔・ドロップ・同一タグ着弾 = coalesce 指標）。 */
function summarizePushes(
  recs: PushRec[],
  burstTotal: number
): Record<string, unknown> {
  if (recs.length === 0) {
    return {
      received: 0,
      dropped: 0,
      elapsed_ms: 0,
      msg_s: 0,
      interval_ms: [],
      coalesced: 0,
      max_batch: 0,
      burst_received: 0,
      paced_received: 0,
    };
  }
  const intervals: number[] = [];
  for (let i = 1; i < recs.length; i++) intervals.push(recs[i].t - recs[i - 1].t);
  const seqs = recs.map((r) => r.seq);
  const min = Math.min(...seqs);
  const max = Math.max(...seqs);
  // seq 連続を前提に欠番をドロップとして数える。
  const dropped = Math.max(0, max - min + 1 - recs.length);
  // 同一 macrotask タグへ複数着弾 = 受信側バッチ化（coalesce）の指標。
  const byTag = new Map<number, number>();
  for (const r of recs) byTag.set(r.tag, (byTag.get(r.tag) ?? 0) + 1);
  let coalesced = 0;
  let maxBatch = 0;
  for (const n of byTag.values()) {
    if (n > 1) coalesced += n;
    if (n > maxBatch) maxBatch = n;
  }
  const elapsed = recs[recs.length - 1].t - recs[0].t;
  return {
    received: recs.length,
    dropped,
    min_seq: min,
    max_seq: max,
    elapsed_ms: elapsed,
    msg_s: elapsed > 0 ? ((recs.length - 1) * 1000) / elapsed : 0,
    interval_ms: intervals,
    coalesced,
    max_batch: maxBatch,
    burst_received: recs.filter((r) => r.seq <= burstTotal).length,
    paced_received: recs.filter((r) => r.seq > burstTotal).length,
  };
}

/** AC4 メッセージ境界ベンチ。状態機械は Rust 側が所有し、こちらは測定のみ。 */
export class IpcBench {
  private state: IpcBenchState = "idle";
  private phaseName = "idle";
  /** RTT ping の通し番号（全フェーズで単調増加 — waiter の衝突を防ぐ）。 */
  private nextSeq = 1;
  private readonly pongWaiters = new Map<number, (t: number) => void>();
  private pushRecs: PushRec[] = [];
  private coexistPushRecs: PushRec[] = [];
  private lastPushAt = 0;
  private pushDoneResolve: (() => void) | null = null;
  /** macrotask 境界スタンプ。同じ tag に複数 push が着弾 = 受信側バッチ化。 */
  private tag = 0;
  private cfg: IpcBenchConfig = DEFAULTS;

  constructor(private readonly hooks: IpcBenchHooks) {
    // setInterval(0) はクランプ後も macrotask ごとに回り、
    // 「同じイベントループターンで届いた push」の検出スタンプになる。
    setInterval(() => {
      this.tag++;
    }, 0);
  }

  get currentState(): IpcBenchState {
    return this.state;
  }

  get currentPhase(): string {
    return this.phaseName;
  }

  /** `__poc.pong(seq)`: rtt_ping の返送着弾（往復時間を確定する）。 */
  onPong(seq: number): void {
    const waiter = this.pongWaiters.get(seq);
    if (waiter) {
      this.pongWaiters.delete(seq);
      waiter(performance.now());
    }
  }

  /** `__poc.push(seq)`: Rust 定速/バースト push の受信記録。 */
  onPush(seq: number): void {
    const rec: PushRec = { seq, t: performance.now(), tag: this.tag };
    // seq 帯でメイン push（1〜360）と coexist push（1001〜）を分離する。
    if (seq > this.cfg.coexist_seq_base) {
      this.coexistPushRecs.push(rec);
    } else {
      this.pushRecs.push(rec);
    }
    this.lastPushAt = rec.t;
  }

  /** `__poc.pushDone()`: Rust 側送信完了（フェーズ待ちの解放）。 */
  onPushDone(): void {
    this.pushDoneResolve?.();
  }

  /** `__poc.start(cfg)` から呼ばれる測定本体。二重起動は冪等に無視する。 */
  async start(cfg: unknown): Promise<void> {
    if (this.state !== "idle") return;
    this.state = "running";
    const c: IpcBenchConfig = {
      ...DEFAULTS,
      ...(typeof cfg === "object" && cfg !== null ? cfg : {}),
    };
    this.cfg = c;
    const errors: string[] = [];
    const phasesMs: Record<string, number> = {};
    const stats: Record<string, unknown> = {
      cfg_echo: c,
      phases_ms: phasesMs,
      errors,
    };
    const setPhase = (name: string): (() => void) => {
      this.phaseName = name;
      if (this.hooks.hud) this.hooks.hud.textContent = `phase=${name}`;
      const t0 = performance.now();
      return () => {
        phasesMs[name] = performance.now() - t0;
      };
    };
    try {
      // Phase 1: JSON RTT — ペイロードサイズ別に逐次 ping-pong。
      const rtt: Record<string, number[]> = {};
      const rttMissed: Record<string, number> = {};
      for (const size of c.rtt_sizes) {
        const end = setPhase(`rtt:${size}`);
        const r = await this.measureRtt(size, c.rtt_iters);
        rtt[String(size)] = r.samples;
        rttMissed[String(size)] = r.missed;
        end();
      }
      stats.rtt = rtt;
      stats.rtt_missed = rttMissed;

      // Phase 2: 単方向 ingest — 往復しない到達レート計測（Rust が窓計測）。
      {
        const end = setPhase("ingest");
        for (let i = 1; i <= c.ingest_n; i++) {
          this.hooks.post({ type: "ingest", seq: i });
        }
        this.hooks.post({ type: "ingest_end", n: c.ingest_n });
        end();
      }
      stats.ingest = { sent: c.ingest_n };

      const base = dataBase();

      // Phase 3: バイナリ pull — fetch → arrayBuffer 完走時間。
      const pull: Record<string, unknown> = {};
      for (let i = 0; i < c.pull_sizes.length; i++) {
        const size = c.pull_sizes[i];
        const end = setPhase(`pull:${size}`);
        pull[String(size)] = await this.measurePull(
          base,
          size,
          c.pull_iters[i] ?? 1
        );
        end();
      }
      stats.pull = pull;

      // Phase 4: push — バースト + 定速の受信間隔・ドロップ・coalesce 観察。
      {
        const end = setPhase("push");
        this.pushRecs = [];
        this.lastPushAt = 0;
        this.hooks.post({ type: "push_begin" });
        await this.waitPushDone(6000, c.stall_ms);
        stats.push = summarizePushes(this.pushRecs, c.push_burst);
        end();
      }

      // Phase 5: coexist — 定速 push 受信中に rtt ping + pull fetch を並行し、
      // 3 経路併用の実現性（遅延劣化・ドロップの有無）を採る。
      {
        const end = setPhase("coexist");
        this.coexistPushRecs = [];
        this.hooks.post({ type: "coexist_begin" });
        const rttMs = await this.measureRtt(64, c.coexist_rtt_iters);
        const pullMs: Record<string, number[]> = {};
        for (const [size, iters] of c.coexist_pull) {
          const r = await this.measurePull(base, size, iters);
          pullMs[String(size)] = r.latencies;
        }
        await this.waitPushDone(6000, c.stall_ms);
        const cp = summarizePushes(this.coexistPushRecs, c.push_burst);
        stats.coexist = {
          rtt_ms: rttMs.samples,
          rtt_missed: rttMs.missed,
          pull_ms: pullMs,
          push_received: cp.received,
          push_dropped: cp.dropped,
          push: cp,
        };
        end();
      }

      this.state = "done";
      this.phaseName = "done";
      if (this.hooks.hud) this.hooks.hud.textContent = "done";
      this.hooks.onDone(stats);
    } catch (e) {
      this.state = "error";
      this.phaseName = "error";
      this.hooks.onError(e instanceof Error ? e.message : String(e));
    }
  }

  /** JSON RTT 1 サイズ分。逐次 ping-pong で往復時間を収集する。
   * タイムアウト（返送喪失）は missed として数え、サンプルからは除外する。 */
  private async measureRtt(
    sizeBytes: number,
    iters: number
  ): Promise<{ samples: number[]; missed: number }> {
    // JSON メッセージ全体が sizeBytes 目安になるよう pad で水増しする。
    const baseLen = JSON.stringify({
      type: "rtt_ping",
      seq: 0,
      t0: 0,
      pad: "",
    }).length;
    const pad = "x".repeat(Math.max(0, sizeBytes - baseLen));
    const samples: number[] = [];
    let missed = 0;
    for (let i = 0; i < iters; i++) {
      const seq = this.nextSeq++;
      const t0 = performance.now();
      const pong = new Promise<number>((resolve) =>
        this.pongWaiters.set(seq, resolve)
      );
      this.hooks.post({ type: "rtt_ping", seq, t0, pad });
      const t1 = await withTimeout(pong, 2000);
      if (t1 === null) {
        missed++;
        this.pongWaiters.delete(seq);
      } else {
        samples.push(t1 - t0);
      }
    }
    return { samples, missed };
  }

  /** バイナリ pull 1 サイズ分。fetch → arrayBuffer 完走の要求毎時間を収集。 */
  private async measurePull(
    base: string,
    bytes: number,
    iters: number
  ): Promise<{ bytes: number; iters: number; latencies: number[] }> {
    const latencies: number[] = [];
    for (let i = 0; i < iters; i++) {
      const t0 = performance.now();
      const res = await fetch(`${base}/blob?bytes=${bytes}&r=${i}`, {
        cache: "no-store",
      });
      if (!res.ok) {
        throw new Error(`pull ${bytes} -> HTTP ${res.status}`);
      }
      const buf = await res.arrayBuffer();
      const dt = performance.now() - t0;
      if (buf.byteLength !== bytes) {
        throw new Error(`pull ${bytes} -> received ${buf.byteLength} bytes`);
      }
      latencies.push(dt);
    }
    return { bytes, iters, latencies };
  }

  /** push フェーズの受信待ち。pushDone 受信・ストール・上限時間のいずれかで戻る。 */
  private waitPushDone(capMs: number, stallMs: number): Promise<void> {
    return new Promise((resolve) => {
      const t0 = performance.now();
      const finish = () => {
        clearInterval(timer);
        this.pushDoneResolve = null;
        resolve();
      };
      this.pushDoneResolve = finish;
      const timer = setInterval(() => {
        const now = performance.now();
        const idle = this.lastPushAt > 0 ? now - this.lastPushAt : 0;
        // pushDone が欠落した場合でも、受信が stallMs 止まれば打ち切る。
        if (now - t0 > capMs || (this.lastPushAt > 0 && idle > stallMs)) {
          finish();
        }
      }, 40);
    });
  }
}
