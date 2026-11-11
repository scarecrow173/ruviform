---
id: "TASK-034VuaZaztYPIHufg5gKhm"
type: "task"
status: "in-progress"
title: "Add GT-05 CI offscreen lanes"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/add-gt-05-ci-offscreen-lanes.md"
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
    - "docs/plans/gt-05-ci-offscreen-lanes-plan.md"
  implemented-by:
    - "docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md"
  depends-on:
    - "docs/plans/gt-05-ci-offscreen-lanes-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/gt-05-ci-offscreen-lanes-plan.md"
    - "docs/specs/gt-05-ci-offscreen-lanes.md"
    - "docs/designs/gt-05-ci-offscreen-lanes-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Add GT-05 CI offscreen lanes

## Work

Issue #314 RF-GT-05: (a) 既存 `test-ui-*` GPUI driver lane 群を
`ci.yml` に収載し、(b) Linux Xvfb+openbox+lavapipe 上で X11
`GetImage` による実 pixel readback を行う offscreen golden lane を
CI/mise に確立する。`gpui_driver::capture` に Linux backend を
追加し、`_NET_WM_PID` + EWMH window 解決・visual decode・
`client_geometry` を実装。`VisualCapture` capability を Linux に
開放し、OS 別の settle/evidence 定数で lane を配線する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/gt-05-ci-offscreen-lanes-plan.md`
- `docs/specs/gt-05-ci-offscreen-lanes.md`
- `docs/designs/gt-05-ci-offscreen-lanes-design.md`

## Dependencies

- gpui 0.2.2 の Linux X11 backend が `_NET_WM_PID` を設定する
  こと（upstream source 確認済み）
- Mesa ≥ 24.2（sw X11 WSI の `chain->depth` 修正）
- openbox（bare Xvfb では present 不成立 — lane 必須要件）
- `docs/specs/issue-314-real-scenario-lane.md` の real-window
  story matrix（`tests/real_window.rs`）が存在すること

## Affected Files

- `.github/workflows/ci.yml` — `ui-gpui-verify` +
  `ui-gpui-offscreen-verify` 追加、`ci-summary.needs` 収載
- `ci/summary/summary.mjs` + `ci/summary/tests/summary.test.mjs` —
  REQUIRED_JOBS/CHECKOUT_LEGS 更新
- `mise.toml` — `[tasks.test-ui-offscreen-xvfb]` 追加
- `crates/ruviform-ui-gpui/Cargo.toml` — `dep:x11rb`（linux,
  `gpui` feature）+ `env_logger` dev-dep
- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs` — X11
  backend + `X11_RESOLVE_BY_PID`
- `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs` —
  `raw_parent_of` OS 別分割 + Linux clippy 修正
- `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` —
  `VisualCapture` Linux 開放
- `crates/ruviform-ui-gpui/tests/real_window.rs` — Linux gate・
  env_logger・OS 別定数
- `Cargo.lock` — x11rb/env_logger/libc pin
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  gt-05-ci-offscreen-lanes*` — 文書チェーン + IR

## Done When

- [x] `ui-gpui-verify`（ubuntu+windows）と
  `ui-gpui-offscreen-verify`（ubuntu）が `ci.yml` に存在し、
  `ci-summary`/`summary.mjs` に収載される
- [x] `mise run test-ui-offscreen-xvfb` が Xvfb+openbox 上で
  `real_window` test を `--test-threads=1` で実行する
- [x] Linux X11 `GetImage` backend が実 pixel を返し、semantic/
  draw-plan の代替が無いことをコードレビューで確認できる
- [x] Docker（mesa 25.0.7 + Xvfb + openbox + lavapipe）で
  `real_window` 2 test pass・実 pixel 非一様を実測
- [x] `VisualCapture` が Linux で claim され、evidence の
  mechanism/backend_detail が OS 別 capture 経路を記録する
- [ ] Windows 側既存 lane（`test-ui-real-scenario`・
  `test-ui-gpui` 等）と全 `test-ui-*`/fmt/check/lint/test が
  退行しない（検証実行中）
- [ ] PR CI で ubuntu-24.04 stock mesa による lane 初回実測

## Verification

- [x] Docker Debian + mesa 25.0.7 + Xvfb + openbox + lavapipe:
  `cargo test -p ruviform-ui-gpui --features real-scenario --test
  real_window -- --test-threads=1` → 2 passed（~96 s）
- [x] bare Xvfb 対照実験で黒画面（openbox 必須の実証）
- [x] xtrace で `PutImage` `BadMatch`（depth-24 vs window depth-32）
  を観測、mesa 24.2 `chain->depth` 修正を source 確認
- [ ] `mise run fmt-check`/`check`/`lint`/`test`（Windows）
- [ ] `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/
  `test-ui-consumer-observation`（Windows + Docker Linux）
- [ ] `mise run test-ui-offscreen-xvfb` 相当の新 task 形状を
  Docker で再現
- [ ] `node --test ci/summary/tests/summary.test.mjs`、
  actionlint
- [ ] PR CI 実測（ubuntu-24.04 stock mesa）
