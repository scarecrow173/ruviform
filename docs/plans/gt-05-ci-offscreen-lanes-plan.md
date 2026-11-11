---
id: "PLAN-034Vu9Of9q1USFJKWTGNY6"
type: "plan"
status: "approved"
title: "RF-GT-05 CI offscreen lanes plan"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/plans/gt-05-ci-offscreen-lanes-plan.md"
      - type: "file"
        path: "docs/specs/gt-05-ci-offscreen-lanes.md"
      - type: "file"
        path: "docs/designs/gt-05-ci-offscreen-lanes-design.md"
      - type: "file"
        path: "docs/tasks/add-gt-05-ci-offscreen-lanes.md"
      - type: "file"
        path: "docs/test-specs/gt-05-ci-offscreen-lanes.md"
      - type: "file"
        path: "docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "ci/summary/summary.mjs"
      - type: "file"
        path: "ci/summary/tests/summary.test.mjs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/real_window.rs"
      - type: "file"
        path: "Cargo.lock"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/gt-05-ci-offscreen-lanes.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/gt-05-ci-offscreen-lanes.md"
    - "docs/designs/gt-05-ci-offscreen-lanes-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/gt-05-ci-offscreen-lanes.md"
  references: []
  defers: []
  deferred-by: []
---

# RF-GT-05 CI offscreen lanes plan Implementation Plan

## Goal

GPUI driver lane 群を `ci.yml` に収載し、Linux Xvfb+openbox 環境で
X11 `GetImage` による実 pixel readback を行う offscreen golden
lane を CI と mise の双方に確立する。

## Scope

単一実装ストリーム — CI job・mise task・capture backend・test
配線・文書は相互依存が強く分割しない。`ui-gpui-verify`（headless
契約 lane）と `ui-gpui-offscreen-verify`（pixel lane）は同じ
crate の別 feature 面を検証するが、依存導入（x11rb・apt 群）と
evidence 規約を共有するため一つの plan とする。

## Upstream Documents

- `docs/specs/gt-05-ci-offscreen-lanes.md`

## Design Inputs

- `docs/designs/gt-05-ci-offscreen-lanes-design.md`

## File Map

- `.github/workflows/ci.yml` - `ui-gpui-verify`（ubuntu+windows
  matrix）と `ui-gpui-offscreen-verify`（ubuntu）を追加、
  `ci-summary.needs` に収載
- `ci/summary/summary.mjs` + `ci/summary/tests/summary.test.mjs` -
  `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` に新 lane を追加
- `mise.toml` - `[tasks.test-ui-offscreen-xvfb]` 追加
  （xvfb-run + openbox + readiness probe + `--test-threads=1`）
- `crates/ruviform-ui-gpui/Cargo.toml` - `dep:x11rb`（linux・
  `gpui` feature 配下、`image` feature）と `env_logger`
  dev-dep 追加
- `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs` -
  `raw_parent_of` の OS 別分割（Linux は `X11_RESOLVE_BY_PID`
  sentinel）、container drop・PhysRect・stub backend の
  Linux clippy 修正
- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs` -
  Linux X11 backend（`_NET_WM_PID` 解決・`_NET_CLIENT_LIST`/
  tree walk・`GetImage` readback・`client_geometry`）追加、
  `X11_RESOLVE_BY_PID` sentinel 定義
- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` -
  `VisualCapture` capability を Linux に開放、capability test 更新
- `crates/ruviform-ui-gpui/tests/real_window.rs` - Linux target
  gate 開放、`env_logger` init、OS 別
  `EVIDENCE_MECHANISM`/`STARTUP_SETTLE_TICKS`/`capture_backend_note`
- `Cargo.lock` - x11rb feature 解決・env_logger 追加・libc
  0.2.189 pin（xattr 0.2.3 `ENOATTR` 非互換の upstream fix）
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  gt-05-ci-offscreen-lanes*` - 文書チェーン + IR

## Implementation Sequence

- [x] T1 文書チェーン（spec → design → plan → task/test-spec）作成
- [x] T2 Linux build 環境の確立 — Docker Debian で gpui 依存解決、
  libc/xattr lockfile 修正、apt -dev 群の確定
- [x] T3 `capture.rs` に X11 backend 実装（window 解決・
  GetImage・geometry）+ `surface.rs` の sentinel 配線
- [x] T4 Xvfb 実機検証 — bare Xvfb で黒画面を観測、mesa A/B
  （22.3.6 vs 25.0.7）で depth-24 `PutImage` `BadMatch` を
  xtrace 特定、openbox で present 成立を確認
- [x] T5 `real_window.rs` capability/test 配線 +
  `tests/real_window.rs` Linux gate・OS 別定数
- [x] T6 `mise.toml` に `test-ui-offscreen-xvfb` 追加
  （openbox readiness probe・`--test-threads=1`）
- [x] T7 `ci.yml` に 2 job 追加 + `ci-summary.needs`・
  `summary.mjs`/test 更新
- [ ] T8 全 lane 検証（fmt-check/check/lint/test/test-ui-*/
  test-rt/verify-ci + Docker Linux 実機）
- [ ] T9 review-protocol self-review・origin/main merge・
  commit/push/PR

## Dependencies and Risks

- **mesa ≥ 24.2 必須** — Ubuntu 24.04 stock で成立見込みだが
  実機は 25.0.7 で検証済み。CI 初回で黒画面なら mesa version を
  第一容疑とする。
- **openbox 必須** — bare Xvfb では present 不成立（実測済み、
  根本原因は未確定・acceptance 対象外）。
- **gpui-test の headless lane と real-window lane の分離** —
  `gpui-test` は `real-scenario` を enable しないため、headless
  CI（`ui-gpui-verify`）は pixel を一切触らず、pixel は
  `ui-gpui-offscreen-verify` のみが担う。証拠層の混同は無い。
- **`_NET_WM_PID` 一意性** — `--test-threads=1` と単一 window
  story で担保。multi-window real-window test を将来この lane に
  載せる場合は識別方式の再設計が必要。

## Task Breakdown

- `docs/tasks/add-gt-05-ci-offscreen-lanes.md` — 本 plan 全体の
  実装 slice（単一 task、内部分解は Implementation Sequence）。

## Verification

- [x] Docker Debian + mesa 25.0.7 + Xvfb + openbox + lavapipe で
  `real_window` test 2 件 pass（~96 s）、実 pixel 非一様確認。
- [x] bare Xvfb で黒画面を再現（openbox 必須の対照実験）。
- [x] `xtrace` で `PutImage` `BadMatch` を観測し mesa source の
  depth hardcode → 24.2 `chain->depth` 修正を確認。
- [ ] `mise run fmt-check`/`check`/`lint`/`test`（Windows）。
- [ ] `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/
  `test-ui-consumer-observation`（Windows + Docker Linux）。
- [ ] `mise run test-ui-offscreen-xvfb` 相当を Docker で再現
  （新 task 形状の `sh -c` + openbox probe 込み）。
- [ ] `mise run test-rt`/`verify-ci`（存在する場合）。
- [ ] `node --test ci/summary/tests/summary.test.mjs`。
- [ ] actionlint（ci.yml 構文）。
- [ ] PR の CI で ubuntu-24.04 stock mesa による初回実測。

## Open Questions / Gaps

- bare Xvfb で present が成立しない正確な機序（gpui の
  visibility/refresh か mesa WSI present mode か）— lane は
  openbox 前提で成立済みのため未解決のまま。
- ubuntu-24.04 stock mesa（24.2.x）での実測 — CI 初回で確認。
- `docs/*/README.md` 管理 index は coordinator 運用により
  commit 対象外（merge 後に main で一括再生成）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（済み — T1〜T7 完了、T8 検証中）
- Independent work streams: none
- User approval required before dispatch: yes（coordinator 経由で
  本 thread に dispatch 済み）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer は design の「発見 3〜5」（depth-32 visual・mesa
  `BadMatch`・openbox 必須）と test の `--test-threads=1`・
  `STARTUP_SETTLE_TICKS` の OS 別値を重点確認されたい。
  `client_geometry` の border 処理は X11 semantics（GetGeometry
  は inside size）に基づく修正であり、border 付き WM 環境でも
  client rect が狂わないことを確認されたい。
