---
id: "TASK-wkAr7DIKDCofXJyOOnTLmo"
type: "task"
status: "done"
title: "T3 実 FS 回帰・crate/index/lockfile 更新・全体検証"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-file/tests/rooted_read.rs"
      - type: "file"
        path: "crates/ruviform-io-file/tests/common/mod.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/designs/issue-304-io-archive-extract.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/plans/rooted-read-capability.md"
  implemented-by: []
  depends-on:
    - "docs/plans/rooted-read-capability.md"
    - "docs/tasks/rooted-read-t2-public-api.md"
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
    - "docs/test-specs/rooted-read-capability.md"
  references: []
  defers: []
  deferred-by: []
---

# T3 実 FS 回帰・crate/index/lockfile 更新・全体検証

## Work

- `crates/ruviform-io-file/tests/common/mod.rs`: tempdir・
  symlink/junction helper（権限不足は明示 skip）・fifo helper
  （unix `mkfifo` 可能時）・canonical root 構築 helper。
- `crates/ruviform-io-file/tests/rooted_read.rs`: test-spec
  （`docs/test-specs/rooted-read-capability.md`）の実 FS 行列 —
  leaf/intermediate swap・`WithinRoot` 正常系（leaf/中間/絶対
  link・browse 降下）・escape（絶対/`..`/外 dir link）・loop/
  limit・nonregular（dir・fifo・NoFollow link）・windows reparse・
  `FileErrorKind` 分類。
- crate README（io-file の `rooted`・io-archive の vfs 依存）・
  `crates/README.md` 台帳・`Cargo.lock`・
  `docs/designs/issue-304-io-archive-extract.md` の Deferred
  Concern 節へ「dir-relative primitive 共通化は本設計で実施済」
  の追記。
- `mise run fmt` → `mise run fmt-check` / `check` / `lint` /
  `test`（workspace）実行と結果記録。
- 未実行 OS（unix 分岐は cfg コンパイルのみ）・実電源断・
  network FS・`openat2` 経路は未検証として結果に明記する。

## Verification

- `cargo test -p ruviform-io-file`（実 FS suite + doctest）・
  `cargo test -p ruviform-io-archive`（非回帰）・
  `cargo test -p ruviform-io-e2e`（所有 lane 結合）。
- `mise run fmt-check` / `check` / `lint` / `test` exit 0。
- 結果を PR 本文（実 head SHA・command・exit・未 OS/FS 制限）と
  検証 report に記録。
