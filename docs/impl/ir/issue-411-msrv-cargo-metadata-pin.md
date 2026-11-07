---
id: "IMPL-au25MvES11UeGCrqOXqw94"
type: "impl"
status: "completed"
title: "Issue #411 — cargo_metadata 0.19 系への依存選定修復"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/411"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_boundary.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-411-msrv-cargo-metadata-pin.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-411-msrv-cargo-metadata-pin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/designs/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/plans/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/test-specs/issue-411-msrv-cargo-metadata-pin.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #411 — cargo_metadata 0.19 系への依存選定修復 実装記録

## 実施内容

- `crates/ruviform-ui-test/Cargo.toml`: `cargo_metadata = "0.23"` →
  `"0.19"`（解決先 0.19.2・cargo-platform ^0.1.2 に閉じる。
  選定根拠を Cargo.toml コメントと design に記録）。
- `Cargo.lock`: `mise run check`（cargo check --workspace
  --all-targets）による正規再生成 — `cargo_metadata` 0.23.1 →
  0.19.2、`cargo-platform` 0.3.3 → 0.1.9 のみ差分。
- `crates/ruviform-ui-test/src/driver_boundary.rs`: 0.19.2 で
  name 系が `String` 化したことへの呼出互換調整 —
  `*.name.to_string()` 12 箇所 → `*.name.clone()`
  （clippy `implicit_clone` 解消。生成値は同一 String）。
- `mise.toml`: `[tasks.test-ui-test-msrv]` 新設（`test-ofx-msrv`
  同型・`mise exec rust@1.89.0 -- cargo test -p ruviform-ui-test`、
  `CARGO_TARGET_DIR=target/msrv-ui-test`）。
- doc: `docs/development-environment.md` opt-in タスク節へ追記、
  crate README の stale 依存記述（os_pipe 言及）を実依存へ修正。

## RED → GREEN 実測

- RED（修復前）: `mise run test-ui-test-msrv` →
  `error: rustc 1.89.0 is not supported by the following package:
  cargo-platform@0.3.3 requires rustc 1.91`・exit 101。
  cargo-platform 以外の MSRV 超過指摘なし。
- GREEN（修復後）: 同 task・Rust 1.89.0 で 76 test 全 pass
  （driver_boundary 10・driver_contract 5・e2e_process 15・
  fake_driver 14・headless_contract 32・lib/bin unittest 0）。

## 検証実績（Windows 11 x64・Git Bash、mise rust 1.99.0 既定）

| 検証 | 結果 |
| --- | --- |
| `mise run fmt-check` | pass |
| `mise run check` | pass（lock 再生成も兼用） |
| `mise run lint` | pass（workspace clippy -D warnings） |
| `mise run test` | pass（workspace 全 test・doctest 含む） |
| `mise run check-ui-driver-boundary` | pass（verdict pass・3 check 全 pass） |
| `mise run test-ui-driver-contract` | pass（5/5） |
| `mise run test-ofx-msrv` | pass（既存 MSRV lane 退行なし） |
| `mise run test-ui-test-msrv` | pass（Rust 1.89.0・76 test） |

## NotRun / 引き渡し

- Synvelis 側 lock 更新と `mise run test-msrv` 再検証: 別 repo・
  別作業（root 管轄）。本 PR の candidate SHA では RF 側 manifest/
  lock は収束済み — Consumer lock 再生成で cargo_metadata 0.19.2 /
  cargo-platform 0.1.9 に収束する想定（manifest 制約上 MSRV 超過へ
  再発する経路なし）。
- macOS / Linux 実機: NotRun（Windows のみ実測）。
- `cargo_metadata` 0.20/0.21 系列: 評価済み採用せず（design 記録
  — platform 0.2 系で MSRV 適合だが cargo-util-schemas 系の追加
  transitive tree を引くため 0.19.2 を選定）。
