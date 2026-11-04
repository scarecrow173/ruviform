---
id: "TASK-034RoYvGpykxcXUHa5X8VR"
type: "task"
status: "done"
title: "Issue 276 T3: contract tests + host example + docs + verification"
created: "2026-11-03"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/276"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-276-generic-logging-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-276-generic-logging-adapter.md"
    - "docs/tasks/task-276-t2-writer-sinks-build.md"
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
    - "docs/test-specs/issue-276-generic-logging-adapter.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 276 T3: contract tests + host example + docs + verification

## Work

`tests/logging.rs` に test-spec E1–E6・E11–E12 の契約試験(file sink 経由で
dual filter routing・Text/JsonLines 内容・retention・失敗分類・scoped dispatch・
health 非再帰)を実装。`examples/logging_host.rs` で Host が dispatch を構築・
scoped/global の使い方・shutdown の流れを示す。`README.md` に logging feature 節
(契約・RT 禁止・stdout 非出力・失敗分類・retention 委譲)を追加。
`origin/main` 取り込み・lockfile 再生成後、全検証と `docs/impl/ir/` IR を仕上げる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-276-generic-logging-adapter.md`
- `docs/test-specs/issue-276-generic-logging-adapter.md`
- `docs/tasks/task-276-t2-writer-sinks-build.md`

## Dependencies

- T2(writer + build が前提)。

## Affected Files

- `crates/ruviform-observability/tests/logging.rs`(新規)
- `crates/ruviform-observability/examples/logging_host.rs`(新規)
- `crates/ruviform-observability/README.md`
- `Cargo.lock`(生成物・再生成)
- `docs/impl/ir/issue-276-*.md`(新規)

## Done When

- [x] test-spec の E1–E6・E11–E12 が `tests/logging.rs` で緑。
- [x] example が `cargo build --examples --features logging` でコンパイル。
- [x] README に feature・契約・制約が記載。
- [x] `origin/main` merge 後の全検証が緑で IR が記録済み
      (`HEAD..origin/main` = 0 commit で merge 不要と確認)。

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test`
- [x] `cargo check -p ruviform-observability --no-default-features`
- [x] `cargo check -p ruviform-observability --features logging`
- [x] `cargo test -p ruviform-observability --features logging`
- [x] `cargo +1.89 clippy -p ruviform-observability --all-targets --features logging -- -D warnings -A clippy::float_cmp -A clippy::similar_names`(MSRV informational。`mise exec rust@1.89.0` 経由)
- [x] `cargo tree -p ruviform-observability --edges normal`(依存 DAG)
- [x] `cargo clippy -p ruviform-observability --features logging --all-targets -- -D warnings`(feature lane)
- [x] `cargo run -p ruviform-observability --features logging --example logging_host`(動作・stdout 0 byte 確認)
