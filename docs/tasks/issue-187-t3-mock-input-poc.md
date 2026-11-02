---
id: "TASK-034Qx4ThH8UjhQhXw4woIO"
type: "task"
status: "done"
title: "Issue #187-T3: mock input backend + RT handoff PoC"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-187-input-port-headless-poc.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-187-t1-input-port-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
    - "docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-187-input-port-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #187-T3: mock input backend + RT handoff PoC

## Work

`crates/ruviform-audio-device/tests/mock_input_backend.rs` に `mock_backend.rs` と同構造の scripted fake input backend を実装し、deterministic capture→bounded SPSC handoff の RT 契約を検証する。

- `FakeInputEnumerator`/`FakeInputDevice`/`FakeInputStream`: script 化した capture block（frames・2ch 以上・deterministic sample pattern・`callback`/`capture` instant・`frame_position`）を stack scratch から `AudioBufferView` 経由で callback へ配送。`error_script` で scripted `DeviceError`（`Xrun`/`StreamInvalidated`/`ClockDiscontinuity`）を error callback へ配送。`Shared` sink で firings/errors/start/stop/now を記録（test thread lock・非 RT）。
- `clock_domain`/`input_latency`/`now` は mock が script で供給（`InputStream` trait の method 実装を兼ねる）。
- handoff: callback 内で `AudioBufferView` → pre-allocated `CaptureBlock` pool（`free` SPSC から `try_pop`）へ bounded copy → `filled` SPSC へ `try_push`。満杯 → overflow 計数（block しない）。非 RT drain で内容・順序・`frame_position` 連続性を検査して `free` へ recycle（#185 の chunk-pool 型）。
- `Cargo.toml` に dev-dep `ruviform-rt`（handoff channel）。adapter normal 依存には混入させない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-187-input-port-headless-poc.md`
- `docs/designs/issue-187-audio-input-port-capture-clock.md`

## Dependencies

- `docs/tasks/issue-187-t1-input-port-contract.md`（input contract）

## Affected Files

- `crates/ruviform-audio-device/tests/mock_input_backend.rs`（新規）
- `crates/ruviform-audio-device/Cargo.toml`（dev-dep `ruviform-rt`）

## Done When

- [ ] fake が 2ch+ scripted block を `callback`/`capture`/`frame_position`/`clock_domain` 付きで届け deterministic ordering
- [ ] `rt_` テストで callback→view→copy→`try_push` の alloc 0（`assert_no_alloc` 窓）
- [ ] `filled` 満杯で overflow 計数・`free` 枯渇で非ブロック（drop しない・次 block 再利用）
- [ ] 同一非 `UNKNOWN` `clock_domain` 一致で同一 clock 判定、`UNKNOWN` 同士は非一致
- [ ] `Xrun`/`StreamInvalidated`/`ClockDiscontinuity`/`Unsupported`/`UnsupportedConfig` が error callback/open result で観測
- [ ] `capture_stamp()` → `InputStamp` → `map_stamp_to_frame` へ bridge する経路の単体検証（`ClockAnchor` calibrate なしで `capture_stamp()` の `Some`/`None` 形状のみでも可）

## Verification

- [ ] `mise run check`/`lint`/`test`/`test-rt` green
- [ ] `cargo tree -p ruviform-audio-device --edges normal` に `ruviform-rt` 不在（dev-dep のみ）
