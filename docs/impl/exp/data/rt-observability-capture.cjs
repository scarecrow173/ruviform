"use strict";

// Capture corrected cells immediately after each run, before Criterion overwrites them.
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const root = path.resolve(__dirname, "../../../..");
const groups = {
  counter_equal_work: ["disabled", "flag/off", "flag/on", "always"],
  overflow_policy_nonzero: ["drop", "count", "aggregate", "retire_leak"],
};
const args = process.argv.slice(2);
const revision = "matched-work-nonzero-v2";

function rowsForRun(run) {
  const hash = crypto.createHash("sha256")
    .update(fs.readFileSync(path.join(root, "crates/ruviform-rt/benches/rt_observability_poc.rs")))
    .digest("hex");
  return Object.entries(groups).flatMap(([group, cells]) => cells.map(cell => {
    const dir = path.join(root, "target/criterion", group, cell, "new");
    const estimates = JSON.parse(fs.readFileSync(path.join(dir, "estimates.json"), "utf8"));
    const benchmark = JSON.parse(fs.readFileSync(path.join(dir, "benchmark.json"), "utf8"));
    if (benchmark.full_id !== `${group}/${cell}` ||
        benchmark.throughput.Elements !== 512) {
      throw new Error(`Unexpected cell schema: ${group}/${cell}`);
    }
    const [column, variant = null] = cell.split("/");
    const median = estimates.median.point_estimate;
    if (!(median > 0 && Number.isFinite(median))) throw new Error("Invalid estimate");
    return {
      run, revision, harness_sha256: hash, group, column, variant, elems: 512,
      median_ns: median, mean_ns: estimates.mean.point_estimate,
      stddev_ns: estimates.std_dev.point_estimate,
      median_ci95_lo_ns: estimates.median.confidence_interval.lower_bound,
      median_ci95_hi_ns: estimates.median.confidence_interval.upper_bound,
      ns_per_op: median / 512,
    };
  }));
}

if (args[0] === "--capture" && args.length === 3) {
  const run = Number(args[1]);
  if (![1, 2, 3].includes(run)) throw new Error("Expected run 1, 2 or 3");
  const rows = rowsForRun(run);
  fs.writeFileSync(args[2], rows.map(row => JSON.stringify(row)).join("\n") + "\n",
    { flag: "wx" });
  console.log(`Captured ${rows.length} cells from run ${run}`);
} else if (args[0] === "--summary" && args.length === 4) {
  const rows = args.slice(1).flatMap((file, i) => {
    const batch = fs.readFileSync(file, "utf8").trim().split("\n").map(JSON.parse);
    if (batch.length !== 8 || batch.some(r => r.run !== i + 1 || r.revision !== revision ||
        r.ns_per_op !== r.median_ns / r.elems)) throw new Error(`Invalid capture: ${file}`);
    return batch;
  });
  if (new Set(rows.map(r => r.harness_sha256)).size !== 1) throw new Error("Mixed harnesses");
  const cells = Object.fromEntries(Object.entries(groups).flatMap(([group, names]) =>
    names.map(name => {
      const values = rows.filter(r =>
        [r.group, r.column, r.variant].filter(Boolean).join("/") === `${group}/${name}`)
        .map(r => r.ns_per_op);
      if (values.length !== 3) throw new Error(`Missing cell ${group}/${name}`);
      return [`${group}/${name}`, {
        runs_ns_per_op: values, mean_of_run_medians_ns_per_op: values.reduce((a, b) => a + b) / 3,
      }];
    })));
  const pairs = ["always", "flag/off", "flag/on"].map(name => [name,
    cells[`counter_equal_work/${name}`].runs_ns_per_op.map((v, i) =>
      v - cells["counter_equal_work/disabled"].runs_ns_per_op[i])]);
  console.log(JSON.stringify({
    revision, harness_sha256: rows[0].harness_sha256, cells,
    paired_counter_deltas_ns_per_op: Object.fromEntries(pairs),
    interpretation: "Single-thread uncontended estimates; not worst-case RT latency bounds.",
  }, null, 2));
} else {
  throw new Error("Use --capture <run> <new.jsonl> or --summary <run1.jsonl> <run2.jsonl> <run3.jsonl>");
}
