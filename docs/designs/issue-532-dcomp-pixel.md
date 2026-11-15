---
id: "DESIGN-034Yf0GO1eY84kAmbq6BBv"
type: "design"
status: "approved"
title: "Issue #532 dcomp-pixel 設計（real-dcomp-pixel case 構成・画素 oracle・env 記録・A' 実機観測手順）"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-dcomp-pixel.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/specs/issue-532-dcomp-pixel.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-631-real-window-dock.md"
    - "docs/designs/issue-631-real-editor-attach.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
    - "evidence/rf388-native-dock-windowing/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
  derived-by:
    - "docs/plans/issue-532-dcomp-pixel.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-pixel.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 dcomp-pixel 設計（real-dcomp-pixel case 構成・画素 oracle・env 記録・A' 実機観測手順）

## Context

`rf631.plugin-dock-real` lane（`crates/ruviform-plugin-dock-test`、
feature `real-dock`）は `test_editor` 実 `IPlugView` の child HWND
ライフサイクルを OS 観測済み（`real-editor-attach`）だが、**画面に
実際に合成される画素**は誰も採取していない（`real-capture` は
editor を attach しない window/slot 領域画素のみ）。Issue #532 の
DComp topmost 制約は「child HWND は存在し visible flag も立つが、
GPUI の topmost DComp visual に覆われて画素が出ない」ため、HWND
観測だけでは検出できない — 画面合成画素の oracle が必要。

先行実測（`evidence/rf388-native-dock-windowing/report.md`）が
確立した診断軸を本 slice の case に写像する: screen 領域の実画素
（何がユーザーに見えるか）と `PrintWindow(child)`（child 自身の
raster）の区別。audit
（`evidence/rf532/gpui-public-surface-audit.md`）が方式 A'
（`GPUI_DISABLE_DIRECT_COMPOSITION`、`"true"`/`"1"` のみ受理、
`WindowsPlatform::new` で process 起動時 1 回読み、非 DComp 経路は
`CreateSwapChainForHwnd`+`WS_EX_NOREDIRECTIONBITMAP` なし通常 DWM
合成）を zero-code 迂回として確定し、exit-127 を PENDING 仮説
H1-H5 として残した。本 slice はこの 2 入力を実機で固定する。

## Scope

### In Scope

- `real-dcomp-pixel` case の追加（`real_dock.rs` +
  `real_catalog.rs` + crate 内一致 test + README/doc 整合）。
- `ospix` module（Win32 画素採取、query/capture のみ）と
  `windows-sys` 既存 optional edge への `Win32_Graphics_Gdi`
  feature 追加。
- `attempt_real_attach` の open 後観測部を注入可能にする最小
  refactor（attach/pixel で attach 経路を共有）。
- `GPUI_DISABLE_DIRECT_COMPOSITION` 生値の case `environment`
  記録（real lane 全 report）。
- `--pixel-artifacts <dir>` opt-in flag による PNG 保存経路。
- baseline/env-on の実機実走と evidence 記録。

### Out of Scope

- 方式採否・本 fix・`ruviform-ui-gpui`/他 crate の変更・`mise.toml`
  変更・docs index 再生成・Issue close（spec の Out of Scope を
  継承）。

## Components and Boundaries

- **`src/ospix.rs`（新規、`#[cfg(feature = "real-dock")]`）**:
  Win32 画素採取の private module。`oschild`（query-only）と同じ
  境界規約 — window の作成・破棄・移動・メッセージ送信は行わない
  （`BitBlt`/`PrintWindow`/`GetWindowRect`/`GetAncestor`/`IsIconic`
  の読み取り系のみ）。`#[cfg(windows)]` 実装 + `#[cfg(not(windows))]`
  stub。戻り値は `u64` HWND・画素統計・`&'static str` backend
  tag のみ — Win32 型は module 内に閉じる。
- **`src/real_dock.rs`**: `real-dcomp-pixel` case・pixel lifecycle
  観測・env 記録 helper・`attempt_real_attach` の注入 refactor。
  case 表・module doc を更新。
- **`src/real_catalog.rs`**: `REAL_CATALOG` に 5 行目追加
  （`required = [NativePluginDock, VisualCapture]`）。
- **`src/bin/plugin-dock-real-scenario.rs`**: `--pixel-artifacts`
  flag 追加と `run_all_with_pixel_artifacts` 呼出。
- **`Cargo.toml`**: `windows-sys` features に `Win32_Graphics_Gdi`
  追加（既存 optional edge の feature 拡張 — 新規 dep なし）。

## Data and Control Flow

### case 構成

`real-dcomp-pixel` は `real-editor-attach` と同一の
`attempt_real_attach` 経路を通る: `lease_real_slot`（declare→
bind→projection→`lease_parent`）→ `container_token` 観測 →
`begin_session` → `ParentSlotLease` → `open_session_docked`
（`InProcessEditorOps` redeem → `session.open(Docked)`）。
`attempt_real_attach` は open 成功時の観測を `observe_open`
closure として注入可能に一般化し、attach case は従来の
`observe_editor_child_lifecycle`、pixel case は新規
`observe_editor_pixel_lifecycle` を渡す。open 拒否
（`Unsupported`）・setup 失敗・teardown の verdict 写像は
attach case と同一規約。

### 画素観測（`observe_editor_pixel_lifecycle`）

1. `oschild::observe(container)` で child 列挙（attach case と
   同一）→ `check_attached_children` で parent/visible/alive
   contract を記録（step rows は attach case と同じ語彙）。
2. top-level window の評価可能性を確認（`GetAncestor(GA_ROOT)`
   → `IsIconic`/`IsWindowVisible`）。iconic/非表示なら pixel
   step を `not_run`（reason 記録）とし、case verdict は
   lifecycle が全て pass でも `NotRun` — 環境が観測を提供
   できないことを Fail と偽らない。
3. child ごとに:
   - `GetWindowRect(child)` → 画面矩標 rect（`screen_rect`
     step、失敗/空矩形は fail row）。
   - **主 oracle** `capture_screen_rect`: `GetDC(NULL)` screen
     DC から `BitBlt(SRCCOPY)` で rect を 32bpp top-down DIB に
     転写 → BGRA→RGBA8。`win32-bitblt-screen` backend tag。
     これは**表示中の合成結果**（DComp visual・他 HWND・cursor
     を含む実画面）であり、DComp topmost が child を覆う現行
     では GPUI 画素が返る = green 不在 = 正直な `Fail`。
   - **補助診断** `capture_print_window`: `PrintWindow(child,
     PW_CLIENTONLY|PW_RENDERFULLCONTENT)` で child 自身の
     raster を 32bpp DIB に転写（`win32-printwindow`）。
     画面合成結果ではない — 「child が緑を paint したか」と
     「画面に出たか」を分離する診断軸。verdict は `note`
     （診断データであり gate ではない）。`PrintWindow` は
     windows-sys では `Win32_Storage_Xps` 限定のため
     `gpui_driver/capture.rs` と同じ local `extern "system"`
     宣言を使う。
4. `EDITOR_COLOR` 判定: RGBA `(r,g,b)` が `g >= 200 && r <= 80
   && b <= 80` の画素を green として計数（`test_editor` の
   `hbrBackground` solid brush = 単色塗り潰し、DWM 合成で
   色は変わらない前提 — 許容差は evidence に記録）。
   `green_fraction = green_pixels / total_pixels`。
   `GREEN_VISIBLE_FRACTION = 0.5` を可視閾値とし、child ごとの
   `screen_pixels` step verdict を pass/fail で記録。
   case の pixel 判定は **いずれか 1 child でも閾値以上**なら
   green 観測（`test_editor` は child を 1 つだけ作るが、
   列挙が複数を返しても「editor 面が画面に出た」事実として
   正直に扱う）。
5. `session.close` → `IsWindow(child)==0` → session 収束
   （`[Attached{Docked}, Closed]`）は attach case と同一の
   `close_editor_session`/`check_detached_children`/
   `check_session_converged` を共有 tail として再利用する
   （`observe_editor_child_lifecycle` から phase 2-4 を
   `conclude_editor_session` として抽出）。

### verdict 規約

- setup/plugin 欠落・open 拒否・lifecycle いずれかの fail →
  attach case と同じ写像（`NotRun`/`Unsupported`/`Fail`）。
- lifecycle 全 pass かつ pixel 評価済みで green なし → **`Fail`**
  （既知 DComp 未解消の正直な regression。detail に
  `green_fraction`・backend・観測法を記す）。
- lifecycle 全 pass かつ green あり → `Pass`（方式採用後の
  昇格経路）。
- lifecycle 全 pass だが pixel が評価不能（iconic/非表示/画面
  DC 取得失敗等）→ `NotRun`（Fail にも Pass にもしない）。
- child ゼロ・`EnumChildWindows` 非対応（非 Windows）→
  attach case と同じく contract fail / `Unsupported`。

### env 記録

`GPUI_DISABLE_DIRECT_COMPOSITION` の生値（未設定は `null`）を
key `"gpui_disable_direct_composition"` として real lane 全
report の `environment` に記録する（`finalize` の `extra` に
`dcomp_env_extra()` を merge — `LaneEnvironment`・headless
lane・bin 引数は不変）。env-on run の JSONL は canonical
`plugin-dock-real-scenario.jsonl` を上書きせず
`plugin-dock-real-scenario-dcomp-disabled.jsonl` 等の別名で
保存する（実走手順は plan/task 側）。

### pixel artifact

`plugin-dock-real-scenario --pixel-artifacts <dir>` 指定時のみ、
採取した各 frame を `RgbaImage::to_png()` で
`<dir>/real-dcomp-pixel-{screen,printwindow}-<index>.png` に
保存する（opt-in — canonical lane は stats のみで artifact を
書かない）。書込失敗は `note` row に記録し verdict に影響
させない。baseline/env-on は別 dir を指定して上書きを防ぐ。

## Non-Functional Constraints

- `unsafe` は `#[expect(unsafe_code)]` + SAFETY コメントの局所
  許可のみ（`oschild`/`gpui_driver/capture.rs` 先例）。
- HWND は `u64` のまま — Win32 型・GPUI 型を公開 API/evidence
  schema に出さない（`check-ui-driver-boundary` green）。
- 画素採取は scenario thread から同期的に行う（`BitBlt`/`Print-
  Window` は任意 thread から呼べる読取系; child HWND の生成
  thread = scenario caller thread で `PrintWindow` の
  `WM_PRINT` 配送も安全）。
- RT contract: non-RT only（既存 lane と同じ）。
- 依存追加なし — `windows-sys` 既存 optional edge の feature
  拡張（`Win32_Graphics_Gdi`）のみ。`Cargo.lock` 差分が出た
  場合は再生成のみ・統合時処理として PR 本文に明記。

## Risks and Trade-offs

- **screen BitBlt の環境依存**: cursor・他 window の重なり・
  window 最小化/非表示で green が消えうる。iconic/非表示は
  `not_run` で誠実に退避し、重なりは lane が管理する desktop
  での実測として limitation を文書化する（rf388 先例と同じ
  受容）。
- **閾値 0.5 の恣意性**: child は container client 全面を塗る
  ため正常時 ~1.0、DComp 覆い時 ~0.0 が期待値で、0.5 は両者を
  明確に分離する。実測の `green_fraction` を evidence に残す
  ため閾値議論は後続レビューで可能。
- **`PrintWindow(child)` が green を返す可能性**: child は
  `hbrBackground` の solid brush を持つ通常 GDI window で、
  `WM_PRINT`/`WM_ERASEBKGND` で自前 raster を返しうる —
  それが「child は paint したが画面に出ない」= DComp occlusion
  の決定的診断になる（rf388 の `printwindow_pixel` 手法と同じ
  狙い）。返らない場合も note 記録で誠実。
- **exit-127**: env-on run が落ちた場合 lane 自体が exit code
  を返す。`set -euo pipefail` 下で lane が落ちるのは観測結果
  そのもの — stderr/exit code を捕捉して H1-H5 に分類し、
  取り繕わない。
- **`attempt_real_attach` 一般化の影響**: attach case の
  verdict 経路を変えないこと — 注入は open 成功後の観測部の
  みで、open/teardown/失敗写像は不変（既存 test + 実機 lane
  で非回帰確認）。

## Deferred Design Concerns

- 方式 A/A'/B/C 採否・本 fix | 本 slice の観測結果が入力 |
  後続 slice | 観測なしの採否は exit-127 未解明のまま実装へ。
- `PrintWindow` の `PW_RENDERFULLCONTENT` が DComp window で
  意味を持つかの精査 | child は非 DComp の通常 window で本
  slice の診断目的には十分 | 将来 container/window 自体の
  raster 比較が必要になった時 | 診断軸の解釈違い。
- macOS/Linux での同等 pixel case | `NativePluginDock` が
  Windows-only で case 自体が Unsupported | GUI-L2 利用要件
  発生時 | 3OS 対応は Deferred。

## Verification Notes

- [x] 画素 oracle が screen 合成結果（主）と child raster
  （補助）に分離され、limitation が明記される設計。
- [x] verdict 規約が Pass 昇格なし・Fail 正直記録・評価不能時
  NotRun を保証。
- [x] env var 記録が JSONL `environment` に自己記述される。
- [x] 依存は既存 edge の feature 拡張のみ・新規 dep なし。
- [ ] `mise run check/lint/test ruviform-plugin-dock-test`
  （default + `real-dock`）green — 実装後に確認。
- [ ] 実機 lane baseline/env-on の実走記録 — plan/task の
  検証手順どおり実施。
