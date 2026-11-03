/**
 * Source-port tests — `MemoryWaveformSource` and `ManualSnapshotSource`
 * reference implementations of the bounded/revision/NaN data contract.
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  ManualSnapshotSource,
  MemoryWaveformSource,
  binAxisHz,
  type MeterFrame,
} from "../src/source.ts";

describe("MemoryWaveformSource", () => {
  it("zoomed-in query returns bounded raw samples (f64, NaN kept)", async () => {
    const src = new MemoryWaveformSource(
      Float64Array.from([0.5, -0.5, Number.NaN, 0.25]),
      1,
      { axis: "samples" },
    );
    const view = await src.query({ start: 0, end: 4, buckets: 16 });
    assert.equal(view.kind, "samples");
    if (view.kind === "samples") {
      const ch = view.channels[0];
      assert.equal(ch?.length, 4);
      assert.equal(ch?.[2] !== undefined && Number.isNaN(ch[2]), true);
      assert.equal(ch?.[3], 0.25);
    }
  });

  it("zoomed-out query reduces to min/max/rms/count buckets", async () => {
    const data = Float64Array.from({ length: 100 }, (_, i) =>
      i === 50 ? Number.NaN : (i % 10) / 10 - 0.45,
    );
    const src = new MemoryWaveformSource(data, 1, { axis: "samples" });
    const view = await src.query({ start: 0, end: 100, buckets: 10 });
    assert.equal(view.kind, "bins");
    if (view.kind === "bins") {
      const ch = view.channels[0];
      assert.equal(ch?.length, 10);
      const b5 = ch?.[5];
      assert.ok(b5 !== undefined);
      assert.ok(b5.count === 9); // one NaN skipped in bucket 5
      assert.ok(b5.min <= b5.max);
      assert.ok(Number.isFinite(b5.rms));
    }
  });

  it("all-NaN bucket → count 0 + NaN stats (never zero-filled)", async () => {
    const src = new MemoryWaveformSource(
      Float64Array.from({ length: 20 }, () => Number.NaN),
      1,
      { axis: "samples" },
    );
    const view = await src.query({ start: 0, end: 20, buckets: 4 });
    assert.equal(view.kind, "bins");
    if (view.kind === "bins") {
      for (const b of view.channels[0] ?? []) {
        assert.equal(b.count, 0);
        assert.ok(Number.isNaN(b.min));
      }
    }
  });

  it("meta.revision advances on replace; stale responses detectable", async () => {
    const src = new MemoryWaveformSource(
      Float64Array.from([1, 2, 3, 4]),
      1,
      { axis: "samples" },
    );
    const meta0 = src.meta();
    const viewP = src.query({ start: 0, end: 4, buckets: 4 });
    src.replace(Float64Array.from([9, 9, 9, 9]));
    const view = await viewP;
    // The view carries the revision it was produced from — the element
    // compares it to meta() at resolve time.
    assert.equal(view.revision, meta0.revision);
    assert.notEqual(src.meta().revision, meta0.revision);
  });

  it("subscribe notifies on replace; unsubscribe stops it", async () => {
    const src = new MemoryWaveformSource(
      Float64Array.from([1]),
      1,
      { axis: "samples" },
    );
    let notified = 0;
    const off = src.subscribe?.(() => notified++);
    src.replace(Float64Array.from([2]));
    off?.();
    src.replace(Float64Array.from([3]));
    assert.equal(notified, 1);
  });

  it("aborted signal rejects the query", async () => {
    const src = new MemoryWaveformSource(
      Float64Array.from([1]),
      1,
      { axis: "samples" },
    );
    const ctl = new AbortController();
    ctl.abort();
    await assert.rejects(src.query({ start: 0, end: 1, buckets: 4 }, ctl.signal));
  });

  it("extent honors the axis unit + sample rate", () => {
    const src = new MemoryWaveformSource(
      Float64Array.from({ length: 48000 }),
      1,
      { axis: "seconds", sampleRate: 48000 },
    );
    assert.equal(src.meta().extent, 1);
  });

  it("out-of-range query clamps to extent", async () => {
    const src = new MemoryWaveformSource(
      Float64Array.from([1, 2, 3, 4]),
      1,
      { axis: "samples" },
    );
    const view = await src.query({ start: -10, end: 50, buckets: 8 });
    assert.equal(view.kind, "samples");
    if (view.kind === "samples") {
      assert.equal(view.channels[0]?.length, 4);
    }
  });
});

describe("ManualSnapshotSource", () => {
  it("push stores latest; subscribe notifies; sequence gaps kept", () => {
    const src = new ManualSnapshotSource<MeterFrame>({ id: "m", revision: 0 });
    assert.equal(src.snapshot(), null);
    let n = 0;
    const off = src.subscribe(() => n++);
    src.push({ sequence: 5, channels: [{ peak: 0.5, rms: 0.3 }] });
    assert.equal(n, 1);
    assert.equal(src.snapshot()?.sequence, 5);
    off();
    src.push({ sequence: 8, channels: [] });
    assert.equal(n, 1);
  });
});

describe("binAxisHz", () => {
  it("uniform and centers axes map bins to Hz", () => {
    assert.equal(
      binAxisHz({ kind: "uniform", firstHz: 0, stepHz: 10 }, 3),
      30,
    );
    assert.equal(binAxisHz({ kind: "centers", hz: [100, 200] }, 1), 200);
    assert.ok(
      Number.isNaN(binAxisHz({ kind: "centers", hz: [] }, 0)),
    );
  });
});
