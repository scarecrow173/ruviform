---
id: "TASK-034XCTbpfF11KBmNQiiynY"
type: "task"
status: "done"
title: "Issue #306 G01/G05 実装タスク"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/large_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/benches/io_composition.rs"
      - type: "file"
        path: "evidence/rf306/large-bench-report.md"
      - type: "file"
        path: "docs/tasks/issue-306-io-lane-completion.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-306-io-lane-completion.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-306-io-lane-completion.md"
    - "docs/designs/issue-306-io-lane-completion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-lane-completion.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G01/G05 実装タスク

## Work

`crates/ruviform-io-e2e` に G01 opt-in 4GiB 試験
（`tests/large_staged.rs`）と G05 criterion bench
（`benches/io_composition.rs`）を追加し、Cargo.toml・
lib.rs/README・evidence・doc chain を整備する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-io-lane-completion.md`
- `docs/designs/issue-306-io-lane-completion.md`
- `docs/test-specs/issue-306-io-lane-completion.md`
- `docs/specs/issue-306-io-lane-completion.md`

## Dependencies

- criterion `0.8.2` は `Cargo.lock` 既収録（ruviform-rt
  dev-dep と同形 pin）。
- `tests/common/mod.rs`（`TestDir` 等）・`tests/wav_fault.rs`
  （DiskFull wrapper 先例）・`tests/corpus_replay.rs`（`#[ignore]`
  先例）・`benches/rt_observability_poc.rs`（criterion 構成
  先例）を再利用/参照。

## Affected Files

- `crates/ruviform-io-e2e/Cargo.toml`（criterion dev-dep +
  `[[bench]]`）
- `crates/ruviform-io-e2e/tests/large_staged.rs`（新規）
- `crates/ruviform-io-e2e/benches/io_composition.rs`（新規）
- `crates/ruviform-io-e2e/src/lib.rs` / `README.md`
- `evidence/rf306/large-bench-report.md`（新規）
- `Cargo.lock`（生成物差分）
- `docs/*/issue-306-io-lane-completion.md` + 各 dir index
  （generator 再生成）

## Done When

- [ ] G01 opt-in lane が `#[ignore]` で実装され、preflight +
  5 scenario × 3 サイズを逐次実行、容量不足は NotRun。
- [ ] G05 bench が criterion で実装され、6 cell + peak alloc
  probe + env metadata を持つ。
- [ ] `mise run fmt-check`/`check-io-e2e`/`lint-io-e2e`/
  `test-io-e2e`/`test-io-e2e-corpus` 全 pass。
- [ ] G01 opt-in 実走・`mise run bench` 実走の結果を
  `evidence/rf306/` に記録。
- [ ] docs index を generator で再生成し整合。

## Verification

- [ ] `mise run fmt-check` exit 0
- [ ] `mise run check-io-e2e` exit 0
- [ ] `mise run lint-io-e2e` exit 0
- [ ] `mise run test-io-e2e` exit 0
- [ ] `mise run test-io-e2e-corpus` exit 0
- [ ] `mise run test ruviform-io-e2e --test large_staged --
  --ignored --nocapture` 実走・記録
- [ ] `mise run bench` 実走・記録
