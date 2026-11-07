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

## merge 後再検証（origin/main 68bb8710 取込・HEAD e3a42630）

- merge: ort 自動 merge・手動 conflict 解消なし。Cargo.lock は
  union（削除行 0・cargo_metadata 0.19.2 / cargo-platform 0.1.9
  維持・main 側追加依存のみ additive）。managed index は
  doc_maintenance 正規再生成（issue-411 findings 0）。
- `mise run fmt-check` / `check` / `lint` / `check-ui-driver-boundary`
  / `test-ui-driver-contract`（5/5）/ `test-ui-component-view`
  （12/12・main 新設 lane）/ `test-ofx-msrv` / `test-ui-test-msrv`
  （Rust 1.89.0・88 test 全 pass）: 全て pass。
- `mise run test`（workspace）: `ruviform-plugin-host`
  l1_proxy_e2e `wedged_child_write_converges_and_teardown_is_bounded`
  が並行負荷時に 1 度だけ spawn timeout（flaky）。単独再実行で
  当該 test 含む suite 184/184 pass・本変更と無関係（PR407 でも
  既知 flaky 記録）。それ以外の workspace 全 test pass。
- 独立 review（内部 subagent）: PASS-WITH-NOTES — 全 readable
  artifact で修正正確性・guard 意味保持・制約遵守を確認。残 note
  （Cargo.lock・mise.toml の byte 検証）は本 worker が実測で
  全て閉じた（platform 0.1.9 単一・他 dep regression 0・task 定義
  実在確認）。
- branch `fix/gui-guard-msrv`（head e3a42630）を push、PR #413
  作成済み（merge/close は root 管轄）。

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
