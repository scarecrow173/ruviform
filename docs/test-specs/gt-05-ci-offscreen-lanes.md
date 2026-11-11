---
id: "TSPEC-034VuaxHGNnRX9WpfdZpMq"
type: "test-spec"
status: "approved"
title: "RF-GT-05 CI offscreen lanes test spec"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/gt-05-ci-offscreen-lanes.md"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/gt-05-ci-offscreen-lanes.md"
  derived-by: []
  verifies:
    - "docs/specs/gt-05-ci-offscreen-lanes.md"
    - "docs/designs/gt-05-ci-offscreen-lanes-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# RF-GT-05 CI offscreen lanes test spec

## Purpose

GPUI driver lane の CI 収載と Linux offscreen pixel lane が、
「実 pixel 証拠」と「semantic/draw-plan/環境不通の区別」を
崩さないことを保証する。pixel lane は CI で走るため、lane 定義・
window 解決・capture verdict の契約をここに固定する。OS 固有の
実機依存（Windows interactive・X display）は Docker/CI でのみ
検証されるため、機構の正しさは unit test と実測の両方で担保する。

## Feature: CI 収載された GPUI driver lane 群

- **Rule**: `ci.yml` に `ui-gpui-verify`（ubuntu+windows matrix）
  と `ui-gpui-offscreen-verify`（ubuntu）が存在し、driver-neutral
  契約 family・`gpui`/`gpui-test` feature lane・consumer
  observation が CI で実行される。`ci-summary` の `needs` と
  `summary.mjs` の `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` に
  収載され、fail-closed（job 失敗 = summary 失敗）を維持する。
- **Rule**: headless `gpui-test` lane は OS pixel を一切触らない
  — `real-scenario` feature を enable しないため、capture は
  `Unsupported`/`NotRun` のまま。pixel 証拠は
  `ui-gpui-offscreen-verify` のみが担う。
- **Example**: `ui-gpui-verify` の Linux leg は gpui build に
  必要な X11/Wayland/fontconfig/Vulkan/EGL dev を apt install
  して `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` を実行する。

## Feature: Linux offscreen pixel lane（X11 GetImage）

- **Rule**: `test-ui-offscreen-xvfb` は `xvfb-run` + openbox +
  `_NET_SUPPORTING_WM_CHECK` readiness probe 上で
  `cargo test -p ruviform-ui-gpui --features real-scenario --test
  real_window -- --test-threads=1` を実行する。`--test-threads=1`
  は `_NET_WM_PID` 一意性の前提であり省略しない。
- **Rule**: `gpui_driver::capture` の Linux backend は window
  drawable の `GetImage` で実 pixel を返し、`backend` に
  `"x11-getimage"` を記録する。window 識別は `_NET_WM_PID` =
  `process::id()` で、候補は `_NET_CLIENT_LIST` 優先・無ければ
  `QueryTree` 再帰 walk。0 件/複数件は `UiError::Host` であり、
  推測で window を選ばない。
- **Rule**: pixel decode は window の visual mask 経由
  （`PixelLayout::from_visual_type`）。TrueColor/DirectColor 以外
  （palette/gray）は `UiError::Unsupported` — 捏造 decode は
  しない。
- **Rule**: `client_geometry` は `GetGeometry`（inside size）+
  `TranslateCoordinates` で root 座標の client rect を返す。
  `border_width` は drawable 外なので client size から引かない。
- **Rule**: openbox（または同等の EWMH WM）は lane の必須要件。
  bare Xvfb では present が成立しない（実測済み）ため、task が
  openbox を display 内で起動し readiness を待つ。mesa < 24.2
  では depth-24 `PutImage` `BadMatch` で恒久的黒画面になるため
  mesa ≥ 24.2 を要求する。
- **Rule**: `tests/real_window.rs` は実 pixel の非一様性を assert
  する（16 pixel 毎の RGB triplet が `BTreeSet` で >1 distinct）。
  uniform 画面（一様な色のみ）は capture が実際に読んでいない
  証拠として失敗する。
- **Rule**: env が不通（X display 無し・window open 失敗）なら
  `UiError::Host` → `environment_blocked` が driver-lab JSONL に
  記録され、GUI Fail と区別される。pass への繰り上げはしない。
- **Example**: Docker Debian + mesa 25.0.7 + Xvfb + openbox +
  lavapipe で `real_window` 2 test が pass（~96 s）、capture
  pixel が非一様（実描画内容）であることを `xwd`/`convert` で
  別途確認。bare Xvfb 対照実験では同一 build が黒画面のまま
  失敗する（openbox/mesa の寄与を分離）。

## Guarantees

- CI の全 lane が `ci-summary` に fail-closed で収載される。
- Linux lane の pixel 証拠は `CapturePath::RealWindow` +
  `os_class=linux` + `backend_detail` に `x11-getimage` 経路を
  記録し、semantic/draw-plan/observer の証拠層と混ざらない。
- capture backend 非対応 OS（macOS 等）と環境不通は
  `Unsupported`/`environment_blocked` のまま — 偽の pass は
  構造的に不可能。
- `--test-threads=1` により `_NET_WM_PID` 解決の一意性が保たれ、
  同一 PID の複数 toplevel が並走しない。

## Non-goals

- macOS pixel capture backend。
- OS-level input（Issue #313 scope）。
- gpui upstream への render-to-image API 追加・patch。
- bare Xvfb での present 不成立の根本原因解明（openbox を正式
  要件として lane は成立済み）。
- multi-window を同時に開く real-window test の本 lane への追加
  （`_NET_WM_PID` が process 単位のため識別再設計が必要）。

## Risk

- ubuntu-24.04 stock mesa（24.2.x）は source 上 `chain->depth`
  修正済みだが実機未検証 — CI 初回で黒画面が出た場合は mesa
  version を第一容疑とし、`RUST_LOG`（env_logger）と evidence
  JSONL で切り分ける。
- `--test-threads=1` の直列化で lane 時間が伸びる（~100 s）—
  許容範囲。並列化は証拠の曖昧性を生むため採らない。
- openbox/xprop の readiness probe が環境差で遅い場合、50 回
  × 0.1 s = 5 s の timeout 後も test は実行される（失敗時は
  capture verdict で診断可能）。
