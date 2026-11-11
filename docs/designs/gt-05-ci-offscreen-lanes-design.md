---
id: "DESIGN-034Vu9v1geiqmFittsRAEQ"
type: "design"
status: "approved"
title: "RF-GT-05 CI offscreen lanes design"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/designs/gt-05-ci-offscreen-lanes-design.md"
      - type: "file"
        path: "docs/specs/gt-05-ci-offscreen-lanes.md"
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
  implements:
    - "docs/specs/gt-05-ci-offscreen-lanes.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-314-real-window-adapter.md"
    - "docs/designs/issue-314-real-scenario-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/gt-05-ci-offscreen-lanes.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# RF-GT-05 CI offscreen lanes design

## Context

RF-GT-05 は (a) 既存 `test-ui-*` GPUI lane 群の CI 収載と
(b) GPU/物理 display を持たない環境での real-window pixel 証拠の
2 本立て。(b) は「offscreen」という語が render-target readback を
連想させるが、gpui 0.2.2 の公開 API には swapchain texture への
readback 経路が存在しない（`copy_texture_to_buffer` は trait に
あるが対象 texture は private）。よって本設計の「offscreen」は
**headless な X 環境（Xvfb）上の実 X11 window** を意味し、pixel
は X server の `GetImage` で window drawable から直接読む —
semantic state・draw plan・observer 出力の代替は一切行わない。

設計上の核心は Linux 経路の成立条件であり、これはコードではなく
**gpui 0.2.2 × Mesa × X server の実測**で確定した。以下の発見は
全て container（Debian + mesa 25.0.7 / mesa 22.3.6 A/B）と
`xtrace`/`xwininfo`/`xprop` による protocol レベル観測に基づく。

### 発見 1: raw-window-handle seam は Linux で使えない

gpui 0.2.2 `platform/linux/x11/window.rs` の
`X11Window::window_handle()` は `unimplemented!()` — `HasWindowHandle`
経由の XID 取得は panic する（error ではなく panic）。
`capture_raw_parent` は `gpui-test` feature でのみ skip されていた
既存経路であり、Linux real-window では同じ理由で seam を使えない。

### 発見 2: window 識別は `_NET_WM_PID` が唯一の正直な経路

gpui は window open 時に `_NET_WM_PID` = `std::process::id()` を
自ら設定する（`client.rs`）。capture 側はこれを読み、process の
own toplevel を解決する。候補集合は:

- EWMH `_NET_CLIENT_LIST`（root property、WM が管理）— openbox
  等の WM は client window を frame window に reparent するため
  root-child 列挙だけでは client を拾えない。WM 存在時はこちらが
  authoritative。
- `QueryTree` の再帰 walk（fallback）— bare Xvfb 等 WM 無しでは
  `_NET_CLIENT_LIST` が存在しない。

0 件または複数件の pid match は `UiError::Host` — どの window を
読むか推測することは証拠の捏造になるため、曖昧さは失敗として
扱う。`--test-threads=1` はこの一意性の前提でもある。

### 発見 3: gpui は常に depth-32 ARGB visual を選ぶ

`client.rs` の `find_visuals` は `screen.allowed_depths` を走査し
transparent（32-bit ARGB）visual を**常に優先**する。Xvfb を含む
全ての一般的 X server は depth-24 root screen 上に depth-32 visual
を `allowed_depths` に列挙するため、**全ての gpui window は
depth-32 になる**（透明性の実需要とは無関係）。

### 発見 4: Mesa < 24.2 の sw X11 WSI は depth-24 `PutImage` を hardcode

xtrace で観測した実 protocol 列: lavapipe（llvmpipe）の WSI present
は window drawable への `PutImage`（ZPixmap）。gpui の window は
depth-32 だが、Mesa 22.3.6 の `src/gallium/wsi/wsi_common_x11.c`
sw present path は image depth に `24` を hardcode しており、
depth 不一致で **全 `PutImage` が `BadMatch`** → 恒久的黒画面。
`xtrace` 上は request が出ているのに pixel が一切変わらない、
という症状の正体はこれ。

Mesa 24.2 系（commit ~2024-06、damage-region rework）で sw path
は `chain->depth`（window の実 depth）を使うよう修正済み。
Ubuntu 24.04 の mesa 24.2.x にも含まれる。**CI lane の mesa 要件は
≥ 24.2** であり、ubuntu-24.04 の stock `mesa-vulkan-drivers` で
成立する。

### 発見 5: bare Xvfb では WM 無しに present が成立しない

mesa 25.0.7（depth bug 修正済み）でも bare Xvfb では黒画面のまま。
window は map され `MapNotify`/`VisibilityNotify`/`Expose` も届く
が present が drawable に反映されない。openbox を同じ display に
立てると即座に実 pixel が得られる（実測: 401 colors の実描画内容）。
gpui の X11 refresh loop は `is_mapped && !FULLY_OBSCURED` でのみ
回るが、bare Xvfb での不成立の正確な機序（visibility state の
遷移か present mode か）は未確定 — coordinator 判断により根本原因
追求は acceptance 対象外とし、**WM（openbox）は lane の正式要件**
として扱う。

## 決定

### 決定 1: CI job 構成（`ci.yml`）

既存 lane と同一の fail-closed 収載方式を踏襲する。

- `ui-gpui-verify`（matrix: ubuntu-latest + windows-latest）—
  driver-neutral 契約 family 7 task + `check-ui-gpui`/`lint-ui-gpui`/
  `test-ui-gpui`/`test-ui-consumer-observation`。headless
  `gpui-test` 前提で OS pixel は触らない。Linux leg は gpui build
  に必要な -dev 群（xcb/xkbcommon/x11/xi/xtst/wayland/fontconfig/
  freetype/vulkan/egl/gl）を apt install する。
- `ui-gpui-offscreen-verify`（ubuntu-latest のみ）— 上記に加え
  `xvfb xauth openbox x11-utils fonts-dejavu-core fontconfig` を
  install し、`mise run test-ui-offscreen-xvfb` を実行。evidence
  artifact は `if: always()` で lane 失敗時も回収する
  （`environment_blocked` 診断に JSONL が必要なため）。
- 両 job とも Issue #366 の checkout-sha leg 規約
  （`CHECKOUT_LEG` env + attempt 付き artifact + sccache GHA backend）
  を既存 job と同一 step 構成で実装。`ci-summary.needs` と
  `summary.mjs` の `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` に追加。

### 決定 2: `mise.toml` の `test-ui-offscreen-xvfb`

```toml
run = "xvfb-run -a -s '-screen 0 1920x1080x24' sh -c 'openbox >/dev/null 2>&1 & i=0; while [ $i -lt 50 ]; do xprop -root _NET_SUPPORTING_WM_CHECK >/dev/null 2>&1 && break; sleep 0.1; i=$((i+1)); done; exec cargo test -p ruviform-ui-gpui --features real-scenario --test real_window -- --test-threads=1'"
```

- openbox は xvfb-run が起こした display 内で background 起動し、
  `_NET_SUPPORTING_WM_CHECK`（EWMH の WM 生存確認 property）を
  xprop で poll してから test を開始する。固定 sleep ではなく
  readiness probe にしたのは CI runner の起動遅延で flake しない
  ため。
- `--test-threads=1`: 同一 PID の toplevel が複数並走すると
  `_NET_WM_PID` 解決が ambiguous になり、occlusion で `GetImage`
  が下位 window を読めない。直列化はこの lane の正当な要件。
- `RUVIFORM_DRIVER_LAB_EVIDENCE`/`RUVIFORM_GOLDEN_EVIDENCE`/
  `RUVIFORM_REAL_WINDOW_PNG_DIR` を `evidence/issue-314-offscreen-xvfb/`
  に固定（既存 real-scenario lane と同じ規則）。

### 決定 3: X11 capture backend（`gpui_driver::capture`）

`x11rb`（既に gpui の依存木に存在、`image` feature のみ追加）で
新規 connection を張り、X resource は server-global なので gpui が
自 connection で作った window を別 connection から読める。

- `raw_parent` = `X11_RESOLVE_BY_PID`（`usize::MAX` sentinel —
  XID は 29-bit resource id で衝突しない）を `surface.rs` の
  `raw_parent_of` が Linux で記録し、capture 時に lazy 解決する。
- `own_window`: `_NET_WM_PID` を intern → 候補集合
  （`_NET_CLIENT_LIST` 優先、無ければ `QueryTree` 再帰 walk、
  depth≤16・件数≤4096 で bound）→ 各候補の property format-32
  value を `process::id().to_ne_bytes()` と比較（x11rb は host
  byte order で negotiate）。1 件のみ受理、0/複数は `UiError::Host`。
- `capture_client_area`: `GetImage(ZPixmap)` →
  `x11rb::image::Image` + `PixelLayout::from_visual_type` で
  visual mask 経由の RGB decode → RGBA8 行優先。palette/gray
  visual は decode 不能として `UiError::Unsupported`。
  `backend = "x11-getimage"`。
- `client_geometry`: `GetGeometry`（inside size — border は
  drawable 外なので `border_width` は引かない）+
  `TranslateCoordinates` で root 座標の client rect を返す
  （input targeting 用。WM reparent 下でも正しい）。
- macOS/その他は従来どおり `Unsupported` placeholder。

### 決定 4: `VisualCapture` capability と settle

`real_window.rs` の `supported_capabilities` で `VisualCapture` を
`cfg!(any(windows, target_os = "linux"))` に拡張。test 側は
`STARTUP_SETTLE_TICKS` を OS 別に分ける — Windows は driver 既定
30 tick（~0.5 s）だが、Linux は lavapipe の初回 frame shader
compile が秒単位のため 600 tick（~9.6 s）。`EVIDENCE_MECHANISM`・
`capture_backend_note` も OS 別に分け、evidence が実際に走った
capture 経路（`mise-task:test-ui-real-scenario` vs
`mise-task:test-ui-offscreen-xvfb`、Win32 vs X11 GetImage）を
記録する。

### 決定 5: `env_logger` dev-dependency

`env_logger = "0.11"` を dev-dep に追加し、test 側で
`env_logger::builder().is_test(false).try_init()` を呼ぶ。
この lane は host の GPU/X11 stack に依存し、capture verdict だけ
では `BadMatch`/`lavapipe` 初期化失敗/present mode を診断できない
— `RUST_LOG` で gpui/blade の log を surface するのは恒常的な
lane 診断基盤であり、今回の黒画面調査（xtrace → mesa source
照合）を可能にした経路でもある。

### 決定 6: Linux 固有 lint 修正（同時に実施）

この crate は Linux で初めて clippy 対象になったため、
`-D warnings` 下の既存指摘を併せて修正:

- `surface.rs`: `raw_parent_of` を OS 別に分割（Linux は sentinel
  返却のみで rwh を呼ばない = panic 回避）。
- `binding.container = None` → `self.bindings.remove(&index)`
  （drop で container 破棄の semantics は同一、dead store 解消）。
- `PhysRect` field に `cfg_attr(not(windows), expect(dead_code))`
  — Win32 backend のみが読む field であることを明示。
- non-Windows stub `backend` module に `expect(unused_self,
  unnecessary_wraps)` — Win32 interface parity の stub であることを
  明示。

### 決定 7: `Cargo.lock` の libc/xattr 修正

gpui の `zed-async-tar` → `xattr 0.2.3` が `libc::ENOATTR` を参照
するが、libc 0.2.190 で Linux の `ENOATTR` が削除され Linux build
が壊れていた（upstream 既知問題）。`cargo update -p libc
--precise 0.2.189` で lockfile のみ修正 — 依存追加ではなく既知の
upstream 互換 fix。

## リスクと trade-off

- **ubuntu-24.04 の mesa 24.2.x での sw WSI depth 修正は source
  確認済みだが実機未検証** — Docker 検証は mesa 25.0.7。
  `chain->depth` 使用は 24.2.8 source で確認済みだが、CI 初回で
  黒画面が出た場合は mesa version を疑う（evidence JSONL の
  `backend_detail` と `RUST_LOG` 出力で切り分け可能）。
- **`--test-threads=1` で lane が直列化** — 2 test × 4 cell ×
  ~12 s ≈ 100 s 弱。並列化は window 識別の曖昧性を生むため採らない。
- **openbox 依存** — 軽量 WM で apt 標準。bare Xvfb が将来
  動くようになれば外せるが、現時点では必須。
- **`_NET_WM_PID` は process 単位** — test process 内で複数
  window を同時に開く経路（multi-window test）は本 lane の
  対象外（`real_window.rs` は単一 window story のみ）。

## Verification Notes

- [x] Docker（Debian + mesa 25.0.7 + Xvfb + openbox + lavapipe）で
  `cargo test -p ruviform-ui-gpui --features real-scenario --test
  real_window -- --test-threads=1` → 2 passed（~96 s）、実 pixel
  非一様を確認。
- [x] 同一条件の bare Xvfb では黒画面（openbox 必須の実証）。
- [x] Mesa 22.3.6 A/B で `BadMatch` 根本原因を xtrace で特定、
  24.2 の `chain->depth` 修正を source 確認。
- [ ] ubuntu-24.04 stock mesa での CI 初回実測（PR の CI 結果で
  検証）。
