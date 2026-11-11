---
id: "IMPL-034VuaCh48ZagIaCgYEDjZ"
type: "impl"
status: "draft"
title: "RF-GT-05 CI offscreen lanes implementation record"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
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
    - "docs/tasks/add-gt-05-ci-offscreen-lanes.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/add-gt-05-ci-offscreen-lanes.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
  experiments:
    adopted: []
    rejected: []
---

# RF-GT-05 CI offscreen lanes implementation record

## Summary

Issue #314 RF-GT-05 を実装: (a) `test-ui-*` GPUI driver lane 群を
`ci.yml` に収載（`ui-gpui-verify` ubuntu+windows matrix）、
(b) Linux Xvfb+openbox+lavapipe 上で X11 `GetImage` による実
pixel readback を行う `ui-gpui-offscreen-verify` lane と
`mise run test-ui-offscreen-xvfb` を追加。`gpui_driver::capture`
に Linux backend（`_NET_WM_PID`/`_NET_CLIENT_LIST`/tree walk による
window 解決 + visual decode + `client_geometry`）を実装し、
`VisualCapture` capability を Linux に開放した。

## Context

`docs/tasks/add-gt-05-ci-offscreen-lanes.md`（plan
`docs/plans/gt-05-ci-offscreen-lanes-plan.md`、spec/design は
同 prefix）。先行 slice: `docs/specs/issue-314-real-scenario-lane.md`
（Windows real-window manual lane）と `docs/specs/
issue-314-real-window-adapter.md`（`RealOsGuiTest` adapter）。

## Implementation

### Changes

- `.github/workflows/ci.yml` — `ui-gpui-verify`（ubuntu+windows
  matrix、driver-neutral 契約 7 task + gpui feature 4 task +
  consumer observation、Issue #366 checkout-sha leg + sccache 準拠）
  と `ui-gpui-offscreen-verify`（ubuntu、apt に
  xvfb/xauth/openbox/x11-utils/fonts-dejavu-core/fontconfig 追加、
  `mise run test-ui-offscreen-xvfb`、evidence artifact は
  `if: always()`）を追加。`ci-summary.needs` に両 job を収載。
- `ci/summary/summary.mjs` + test — `REQUIRED_JOBS` に
  `ui-gpui-verify`/`ui-gpui-offscreen-verify`、
  `REQUIRED_CHECKOUT_LEGS` に `ui-gpui-verify-ubuntu-latest`/
  `-windows-latest`/`ui-gpui-offscreen-verify` を追加。
- `mise.toml` — `[tasks.test-ui-offscreen-xvfb]` 追加。
  `xvfb-run -a -s '-screen 0 1920x1080x24'` 内で openbox を
  background 起動し `_NET_SUPPORTING_WM_CHECK` を xprop で poll
  （最大 5 s）してから `cargo test ... -- --test-threads=1`。
  evidence env を `evidence/issue-314-offscreen-xvfb/` に固定。
- `crates/ruviform-ui-gpui/Cargo.toml` — `x11rb`（linux target・
  `gpui` feature・`image` feature、optional）と `env_logger`
  dev-dep を追加。
- `capture.rs` — `X11_RESOLVE_BY_PID`（`usize::MAX` sentinel）を
  定義し、Linux backend module を追加: `connect`/`window_xid`/
  `own_window`（`_NET_WM_PID` = pid、`_NET_CLIENT_LIST` 優先 +
  `QueryTree` fallback、0/複数 match は `UiError::Host`）/
  `client_list`/`tree_windows`/`geometry`/`find_visual`/
  `capture_client_area`（`GetImage` → `PixelLayout` decode →
  RGBA8、`backend="x11-getimage"`）/`client_geometry`
  （`GetGeometry` inside size + `TranslateCoordinates` → root
  座標）。macOS 等は従来の `Unsupported` placeholder を維持。
- `surface.rs` — `raw_parent_of` を OS 別に分割。Linux（non-test）
  は `X11_RESOLVE_BY_PID` を返し rwh seam を呼ばない
  （gpui 0.2.2 `X11Window::window_handle()` が `unimplemented!()`
  で panic するため）。non-Linux は従来の Win32/AppKit match を
  維持。container drop を `bindings.remove` に整理、PhysRect・
  stub backend の Linux clippy を `expect` で明示。
- `real_window.rs` — `supported_capabilities` の `VisualCapture`
  を `cfg!(any(windows, target_os = "linux"))` に拡張、
  capability unit test を追従。
- `tests/real_window.rs` — target gate を
  `any(windows, target_os = "linux")` に拡張。`env_logger` init
  （`RUST_LOG` で gpui/blade log を surface — host GPU/X11 stack
  依存 lane の恒常診断基盤）。`EVIDENCE_MECHANISM`/
  `capture_backend_note`/`STARTUP_SETTLE_TICKS` を OS 別に分割
  （Linux: `mise-task:test-ui-offscreen-xvfb`、600 tick ≈ 9.6 s —
  lavapipe 初回 frame shader compile 対策。Windows: 従来値 30）。
  実 pixel 非一様 assert（16 pixel 毎 RGB triplet の distinct > 1）
  を維持。
- `Cargo.lock` — `env_logger`/`jiff`/`env_filter` 等の追加分と、
  `libc` を 0.2.189 に pin（xattr 0.2.3 が参照する
  `libc::ENOATTR` が libc 0.2.190 で Linux から削除され gpui の
  `zed-async-tar` 依存が Linux build 不能になっていた upstream
  既知問題の lockfile 修正）。

### Decisions During Implementation

- **window 識別に `_NET_WM_PID` を採用**: gpui 0.2.2 の X11
  `PlatformWindow` は `HasWindowHandle`/`HasDisplayHandle` が
  `unimplemented!()` で rwh seam が panic する。gpui 自身が open
  時に `_NET_WM_PID` = `process::id()` を設定するため、capture
  側は lazy に own toplevel を解決する。`usize::MAX` sentinel は
  XID（29-bit）や HWND/NSView と衝突しない。
- **候補集合は `_NET_CLIENT_LIST` 優先 + `QueryTree` fallback**:
  openbox は client window を frame window に reparent するため
  root-child 列挙では拾えない。`_NET_CLIENT_LIST` が無い
  bare Xvfb では tree walk。0/複数 match は推測せず
  `UiError::Host`。
- **openbox を lane 必須要件化**: mesa 25.0.7（depth bug 修正済み）
  でも bare Xvfb は黒画面。openbox 起動で即座に実 pixel が得られ
  たため、coordinator 判断により WM を正式要件として文書化し、
  根本原因追求は acceptance 対象外とした。task は openbox を
  display 内で起動し `_NET_SUPPORTING_WM_CHECK` を poll する。
- **`--test-threads=1` を必須化**: 同一 PID の toplevel が並走
  すると `_NET_WM_PID` 解決が ambiguous、occlusion で `GetImage`
  が下位 window を読めない。
- **`client_geometry` の border 処理**: X11 `GetGeometry` の
  width/height は inside size（border は drawable 外）なので
  `border_width` を引かない — 当初 `2*border` を引いていたが
  review で修正（Xvfb では border=0 で顕在化しなかった）。
- **`env_logger` を恒常 dev-dep として保持**: 一時診断用ではなく、
  host GPU/X11 stack 依存 lane の `RUST_LOG` 診断経路として
  `tests/real_window.rs` で init。一時 probe
  （`examples/gpui_raw_probe.rs`）と `RUVIFORM_DIAG_PNG_DIR`
  dump・`TEMP-DIAG` marker は全て除去済み。
- **Linux 固有 clippy を同時修正**: この crate は Linux で初めて
  `-D warnings` clippy 対象になったため、`raw_parent_of` 分割・
  container drop・PhysRect/stub backend の `expect` を併せて実施。
- **libc pin**: `cargo update -p libc --precise 0.2.189` で
  lockfile のみ修正（依存追加ではなく upstream 互換 fix）。

## Related Experiments

### Adopted

- Xvfb + openbox + lavapipe（mesa 25.0.7）— Docker Debian で
  `real_window` 2 test pass・実 pixel 非一様を実測。lane の
  正式形として採用。

### Rejected

- bare Xvfb（WM 無し）— mesa 25.0.7 でも黒画面。present が
  drawable に届かない。根本原因は未確定だが lane 要件として
  openbox を採用したため追求しない。
- Mesa 22.3.6（Ubuntu 22.04 系）— depth-24 `PutImage` `BadMatch`
  で恒久的黒画面。mesa ≥ 24.2 を要件化。
- `MESA_VK_WSI_PRESENT_MODE=FIFO` — present mode 強制は
  depth `BadMatch` を解決しない（旧 mesa の問題）。
- `-pixdepths 24`/depth-32 screen 検証・Xvnc/Xephyr/Xorg dummy
  — いずれも depth-32 visual を隠せず gpui の visual 選択を
  変えられない。
- gpui headless platform — stub で render path ではない。
- 真の render-target offscreen readback — gpui 0.2.2 の公開 API
  に存在しない（`copy_texture_to_buffer` の対象 texture は
  private）。「offscreen」は headless 環境を指す。

## Validation

- [x] Docker Debian + mesa 25.0.7 + Xvfb + openbox + lavapipe:
  `cargo test -p ruviform-ui-gpui --features real-scenario --test
  real_window -- --test-threads=1` → **2 passed**（~96 s）。
  lane と同一形状（`xvfb-run` + `sh -c` + openbox +
  `_NET_SUPPORTING_WM_CHECK` probe + `--test-threads=1`）で再現。
- [x] bare Xvfb 対照実験 — 同一 build が黒画面（openbox/mesa
  の寄与を分離）。
- [x] `xtrace` — lavapipe present が window drawable への
  `PutImage`（depth-24 vs window depth-32）で `BadMatch` を
  観測。mesa 24.2.8 source で `chain->depth` 使用を確認。
- [x] Windows `test-ui-gpui`/`check-ui-gpui`/`lint-ui-gpui`/
  `test-ui-consumer-observation`/`test-ui-scenario-contract-gpui`
  等 — 全 pass（lane task 群）。
- [x] Linux `cargo check`/`clippy -D warnings`（gpui feature）
  — Docker で pass。
- [ ] `mise run fmt-check`/`check`/`lint`/`test`（Windows 全体）。
- [ ] `mise run test-rt`/`verify-ci`。
- [ ] `node --test ci/summary/tests/summary.test.mjs`、
  actionlint。
- [ ] PR CI で ubuntu-24.04 stock mesa（24.2.x）による初回実測 —
  source 上 `chain->depth` 修正済みだが実機未検証。

## Risks

- ubuntu-24.04 stock mesa 24.2.x での sw WSI depth 修正は source
  確認済みだが実機未検証 — CI 初回で黒画面なら mesa version を
  第一容疑（`RUST_LOG` + evidence JSONL で切り分け）。
- bare Xvfb での present 不成立の正確な機序は未確定
  （visibility/refresh か present mode か）— openbox を正式
  要件として lane は成立済み。
- `--test-threads=1` の直列化（~100 s）と `_NET_WM_PID` の
  process 単位一意性 — multi-window real-window test を本 lane
  に追加する場合は識別再設計が必要。
- gpui/blade の `RUST_LOG` 出力に `Found no xinput mouse pointers`
  の ERROR 行が出る（Xvfb に xinput device が無いため）—
  non-fatal で test は pass するが CI log のノイズになる。

## Follow-ups

- PR CI で `ui-gpui-offscreen-verify` の ubuntu-24.04 stock mesa
  実測を確認（黒画面時は mesa version を疑う）。
- `docs/*/README.md` 管理 index は coordinator 運用により commit
  対象外 — merge 後に main で一括再生成。
- macOS pixel capture backend・真の render-target offscreen
  readback は別 slice（gpui upstream の API 確定待ち）。
- bare Xvfb present 不成立の根本原因は upstream gpui の
  visibility/refresh 改修時に再評価。
