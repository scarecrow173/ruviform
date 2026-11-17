# Upstream issue draft — free-audio/clap-validator

> Worker 権限外のため未起票。Coordinator/ユーザーが内容を確認のうえ
> https://github.com/free-audio/clap-validator/issues/new へ投稿する。
> 投稿後は本リポジトリ側 docs（design/impl）へ Issue URL を追記する（残 gate）。

---

**Title:** Validator panics and loses all results when a plugin exits the sandboxed child with status 0

**Body:**

## Summary

When a plugin terminates the sandboxed validation child process with exit
status 0 without writing its result file, `clap-validator` panics inside
`SandboxOperation::run_sandboxed` (`src/cli/sandbox.rs`) instead of
reporting the test as crashed/failed. The panic propagates through the
scoped worker threads in `parallel_fork_join` (`src/cli.rs`) and kills the
whole validator: exit code 101, **no JSON report is emitted and every
other test result is discarded**.

A plugin that crashes the child with a *non-zero* status is handled
correctly (`SandboxError::Crashed` → per-test `crashed` result, JSON
emitted, exit 1). The bug is specific to the success-exit-without-output
path.

## Root cause

`run_sandboxed` only treats `!status.success()` as a crash:

```rust
// src/cli/sandbox.rs (v0.4.1 / master @ 5bc2ef3)
if !status.success() {
    return Err(SandboxError::Crashed(status));
}

let output =
    std::fs::read_to_string(&output_file).expect("Failed to read the output file from the sandboxed operation");
let result: Self::Result =
    serde_json::from_str(&output).expect("Failed to deserialize the output from the sandboxed operation");
```

If the child calls `std::process::exit(0)` (or otherwise exits successfully)
before `SandboxPayload::dispatch` writes the result JSON, `status.success()`
is true but the output file is missing/empty, so the `.expect()` panics.

That panic happens inside a `std::thread::scope` worker spawned by
`IteratorExt::parallel_fork_join` (`src/cli.rs`), so it surfaces as:

```
ERROR thread '<unnamed>' panicked at 'Failed to deserialize the output from the
sandboxed operation: Error("EOF while parsing a value", line: 1, column: 0).
This is a bug in the validator': src\cli\sandbox.rs:98
ERROR thread 'main' panicked at 'a scoped thread panicked. This is a bug in the
validator': src\cli.rs:55
```

and the process exits 101 without producing `--json` output.

## Reproduction

Observed with clap-validator 0.4.1 on Windows against a CLAP plugin fixture
whose `process()` calls `std::process::exit(0)` when it receives a parameter
event (the fixture is public: `scarecrow173/ruviform`,
`crates/ruviform-plugin-test-clap`, plugin id
`org.ruviform-test.clap-test-fault`):

```sh
clap-validator validate --json \
  --plugin-id org.ruviform-test.clap-test-fault \
  --include param-fuzz-basic \
  target/debug/ruviform_plugin_test_clap.dll
```

Result: exit 101, empty stdout, the two panic messages above (~15 ms).
Adding `-j1` moves the panic to the main thread directly (no scoped thread).

Any plugin able to terminate the host process with status 0 during a test
triggers the same path — the fixture is just a convenient reproducer.

## Expected behavior

A sandboxed child that exits without producing a result should be reported
as a crashed/failed test (like `SandboxError::Crashed` already does for
non-zero exits), the panic should not escape the per-test boundary, and the
validator should still emit its JSON report for the remaining tests.

## Notes / related observations

- `param-fuzz-sample-accurate` against the same fixture does **not**
  trigger this: it reports `crashed: exit code: 0xc0000409` cleanly with
  JSON output and exit 1 (the child dies via `abort()`, i.e. non-zero
  status → `SandboxError::Crashed` path works).
- Separately observed on 0.4.1: `preset-discovery-crawl`/`preset-discovery-load`
  time out after 45 s for a plugin that does not implement preset
  discovery; the deadlock fix from issue #41 appears to be merged on
  master but is not in the 0.4.1 binary.
- `param-set-events`/`param-set-no-cookies` failures on 0.4.1 match the
  symptom fixed by issue #42 (already on master, unreleased).
- The 45 s `wait_timeout` path returns `SandboxError::Timeout` but does
  not kill the timed-out child process; orphaned children were observed
  still running (and holding the plugin DLL locked) after the run.

## Environment

- clap-validator 0.4.1 (Windows x86_64), sha256
  `825439ad3ff98770a24656013eeb11591407687b97058e1db9549c4f7d53e38a`
- Plugin: `ruviform_plugin_test_clap.dll` (debug build, Windows)
- Verified still present on master @ `5bc2ef3`.
