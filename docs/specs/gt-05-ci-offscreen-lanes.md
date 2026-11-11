---
id: "SPEC-034VtOJn6cykr1CPohlDAX"
type: "spec"
status: "approved"
title: "RF-GT-05 CI offscreen lanes — GPUI lane CI 収載 + Linux offscreen golden"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/specs/gt-05-ci-offscreen-lanes.md"
      - type: "file"
        path: "docs/designs/gt-05-ci-offscreen-lanes-design.md"
      - type: "file"
        path: "docs/plans/gt-05-ci-offscreen-lanes-plan.md"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-314-real-scenario-lane.md"
    - "docs/impl/ir/issue-314-real-scenario-lane.md"
    - "docs/specs/issue-314-real-window-adapter.md"
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

# RF-GT-05 CI offscreen lanes — GPUI lane CI 収載 + Linux offscreen golden

## 背景

Issue #314（GPUI semantic/visual golden 基盤）の残件 RF-GT-05 は
2 つの独立した gap を持つ。

1. `ruviform-ui-gpui` の driver lane 群（boundary guard・driver
   contract・parity・native slot・component/view・golden core・
   scenario contract・`gpui`/`gpui-test` feature の check/lint/test・
   consumer observation）が `mise.toml` に定義済みだが
   `.github/workflows/ci.yml` のどの job からも呼ばれておらず、
   CI で一度も検証されていない。
2. real-window pixel lane（`tests/real_window.rs`）は Windows
   interactive desktop 前提の manual lane（`test-ui-real-scenario`）
   のみで、GPU/物理 display を持たない CI 環境での pixel 証拠経路が
   存在しない。gpui 0.2.2 の公開 API は render-target の
   offscreen readback を提供しないため、「offscreen」は render
   target ではなく headless 環境（Xvfb）を指す。

実装中の実測で Linux 経路の成立条件が判明した（詳細は design
参照）: gpui 0.2.2 の X11 backend は `HasWindowHandle` を
`unimplemented!()` で panic させるため raw-window-handle seam は
使えず、window 識別は gpui 自身が open 時に設定する
`_NET_WM_PID` 経由で行う。また gpui は X server が depth-32
visual を提供する限り常に ARGB（depth-32）window を作るが、
Mesa < 24.2 の software X11 WSI は depth-24 の `PutImage` を
hardcode しており全 present が `BadMatch` で失敗する
（= 恒久的な黒画面）。さらに bare Xvfb では WM の
map/visibility semantics が無く present が drawable に届かない
ため、openbox 等の window manager が lane の必須要件となる。

## 要件

1. `ci.yml` に GPUI driver lane job を追加し、既存
   `ui-verify`/`ui-verify-macos` と同じ fail-closed
   `ci-summary` 収載方式（`needs` + checkout-sha leg + summary
   宣言）で ubuntu/windows matrix を検証する。
2. `ci.yml` に Linux offscreen pixel lane job を追加し、
   `xvfb-run` + openbox + lavapipe（software Vulkan）上で
   `tests/real_window.rs` を実行する。apt 依存は gpui の build
   要件（X11/Wayland/fontconfig/Vulkan/EGL dev）と runtime 要件
   （xvfb・openbox・mesa vulkan drivers・fonts）を満たす。
3. `gpui_driver::capture` に Linux/X11 backend を追加し、
   window drawable の `GetImage` readback で実 pixel を取得する。
   window 識別は `_NET_WM_PID` + EWMH `_NET_CLIENT_LIST`（fallback:
   window tree walk）で行い、WM reparenting と WM 無し環境の双方を
   正直に扱う。pixel/decode 不能・window 未解決は `UiError::Host`/
   `Unsupported` であり、semantic state や draw plan の代替は
   絶対に行わない。
4. `supported_capabilities` の `VisualCapture` を Linux にも
   開放し、evidence の `backend_detail`/`mechanism` が OS 毎の
   capture 経路を正直に記録する。
5. `mise.toml` に `test-ui-offscreen-xvfb` task を追加し、CI と
   ローカル検証で同一入口とする。`--test-threads=1` を必須化する
   （同一 PID の複数 toplevel が並走すると `_NET_WM_PID` 解決が
   曖昧になり、下位 window の readback が occlusion される）。
6. 対応 OS で capture backend が存在しない場合・環境が不通な場合は
   従来どおり `Unsupported`/`environment_blocked` を維持し、
   pass への偽装をしない。

## 対象外

- macOS capture backend（引き続き `Unsupported` placeholder）。
- OS-level input（Issue #313 scope — `RealOsPointerOrKeyboard`
  は claim しない）。
- gpui 0.2.2 への render-to-image / 真の render-target offscreen
  readback API の追加・patch（upstream 依存の変更は本 slice の
  範囲外）。
- bare Xvfb（WM 無し）で present が成立しない根本原因の追求 —
  coordinator 判断により openbox 必須を lane の正式要件として
  文書化し、acceptance 対象外とする。
- consumer 製品 fixture の実接続、Plugin Dock snapshot、
  Issue #314 の Close。

## 受入条件

- `ui-gpui-verify`（ubuntu+windows）と `ui-gpui-offscreen-verify`
  が `ci.yml` に存在し、`ci-summary` の `needs` と summary.mjs
  の `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` に収載される。
- Linux lane が Xvfb+openbox+lavapipe で `real_window` test を
  実行し、実 pixel（非一様な画素値）が `CapturePath::RealWindow`
  の golden 証拠として記録される。semantic state・draw plan の
  代替は無い。
- Windows の既存 real-scenario lane と headless `gpui-test` lane
  が退行しない。
- 既存検証 lane（`fmt-check`/`check`/`lint`/`test`/`test-ui-*` 系）
  が退行しない。

## Deferred Design Concerns

- macOS pixel capture backend — capture API 未確定のため別 slice。
- 真の render-target offscreen readback — gpui upstream に
  `render_to_image` 相当が入った時点で再評価。
- bare Xvfb での present 不成立の根本原因 — lane は openbox 前提で
  成立済み。再調査は upstream gpui の visibility/refresh 改修時。

## Sources and Relations

- Issue #314（RF-GT-05）— `https://github.com/scarecrow173/ruviform/issues/314`
- `docs/specs/issue-314-real-scenario-lane.md` — Windows real-window
  manual lane の先行 slice。
- `docs/specs/issue-314-real-window-adapter.md` — `RealOsGuiTest`
  adapter の確立 spec。
- 実測知見: gpui 0.2.2 `platform/linux/x11/window.rs` の
  `window_handle()` が `unimplemented!()`、`client.rs` の
  `find_visuals` が transparent visual を常時優先、
  `_NET_WM_PID` を `process::id()` で設定。Mesa sw X11 WSI の
  `PutImage` depth が 24.2 で `chain->depth` 化。
