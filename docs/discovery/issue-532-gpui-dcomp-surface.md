---
id: "DISC-034XAXQbwZKvh8lVqSB9dy"
type: "discovery"
status: "resolved"
title: "Issue #532 GPUI 0.2.2 DComp public-surface 監査 — 方式A/B/C 技術入力"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/designs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/specs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/plans/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/tasks/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/test-specs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "evidence/rf532/gpui-public-surface-audit.md"
    modified:
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-native-dock.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
    - "docs/tasks/issue-532-gpui-dcomp-surface.md"
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
    - "evidence/rf388-native-dock-windowing/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
  derived-by:
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
---

# Issue #532 GPUI 0.2.2 DComp public-surface 監査 — 方式A/B/C 技術入力

## Summary

vendored gpui 0.2.2（crates.io pin、`gpui = "0.2.2"`）の Windows platform 層を
読み取り専用で監査し、方式 A/B/C の技術入力を固定した docs-only slice。
**確定事実**: (1) `CreateTargetForHwnd` の `topmost` literal は crate 全体で
`directx_renderer.rs:793` の `true` 1 箇所のみで、Option A の `[patch]` 面積は
最小、(2) gpui の DComp/DXGI/renderer 全てが `pub(crate)` 以下で downstream
からは HWND（`HasWindowHandle`）と env var `GPUI_DISABLE_DIRECT_COMPOSITION`
以外に触れない、(3) env var は `"true"`/`"1"` のみ受理し非 DComp 経路は
`CreateSwapChainForHwnd` + `WS_EX_NOREDIRECTIONBITMAP` なしの通常 DWM 合成へ
切り替わる zero-code 代替経路、(4) gpui 0.2.2 が最新 publishable 版で
zed repo 現行版は publish 不能（`` `zed-industries/zed#55271` ``）。
**未解消のまま保持**: exit-127 の原因（仮説のみ・PENDING）、`topmost=false`
の実画素効果、実 plugin 画素の表示可否 — いずれも実 OS 依存で本 slice の
範囲外。

## Exploration Goal

先行監査 `docs/discovery/issue-532-dcomp-native-dock.md`（status: resolved）の
Open Questions が残した upstream public-surface 調査を行う: (a) DComp
オブジェクトグラフの列挙と `topmost` 引数の全出現箇所、(b)
`GPUI_DISABLE_DIRECT_COMPOSITION` の意味論と非 DComp 経路の実動作、
(c) public vs `pub(crate)` の到達可能性、(d) Option A のパッチ面積、
(e) Option B に必要な private-internal 要件、(f) gpui バージョン/
アップグレード情勢、(g) exit-127 のコードパス仮説。実画素実験・採否
verdict・patch/vendor 適用は対象外（interactive Windows 11 desktop session
依存の外部 gate）。

## Key Issues and Assumptions

- **監査対象の固定**: `gpui-0.2.2` vendored source
  （`C:/Users/akiyoshi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/`）。
  ruviform の pin（`crates/ruviform-ui-gpui/Cargo.toml:54`）と一致。
  行番号は本 slice で独立に再検証済み（先行監査の `directx_renderer.rs:793`
  等と drift なし — `evidence/rf532/gpui-public-surface-audit.md`）。
- **読み取り専用の境界**: registry source は編集しない。patch/vendor/
  `[patch.crates-io]` 追加・gpui fork 取り込みは本 slice の仕事ではない。
- **非 DComp 経路の意味**: env var は process-wide・platform 初期化時に
  1 回だけ読まれる（`platform.rs:138-139`）。window 単位の切替や
  runtime 変更は不可。
- **`topmost=false` の意味論は未検証**: MSDN 上 FALSE は visual tree を
  window 内容の一部として合成し child window が上に来うる、が、
  `WS_EX_NOREDIRECTIONBITMAP` window での実挙動は fixture 実測が必要。
  断定しない。

## Findings

### F1: DComp オブジェクトグラフ（生成・保持・所有）

モジュール経路は全て private: `gpui.rs:31` `mod platform;` →
`platform.rs:24` `mod windows;`（cfg windows）→ `platform/windows.rs` の
private submodule 群 → `pub(crate) use` のみ（platform.rs:83）。

生成連鎖: `WindowsPlatform::new`（platform.rs:90 付近）が message-only
HWND・`DirectWriteTextSystem`・env var を初期化 → window ごとに
`WindowCreationInfo`（pub(crate)、platform.rs:818-829）経由で
`WindowsWindow::new`（window.rs:361-）→ `WindowsWindowInner::new`
（208-236）→ `WindowsWindowState::new`（80-147）→
`DirectXRenderer::new(hwnd, devices, disable_direct_composition)`
（window.rs:108）。

`DirectXRenderer::new`（directx_renderer.rs:127-168）内で:
- `DirectXRendererDevices::new`（99-124）: `dxgi_device` は
  `!disable` 時のみ `device.cast::<IDXGIDevice>()`（110-114）。
- `DirectXResources::new`（655-697）: swapchain を分岐（663-672）。
- `DirectComposition::new`（790-801、`!disable` 時のみ 147-156）:
  `DCompositionCreateDevice`（get_comp_device、1017-1019）→
  **`CreateTargetForHwnd(hwnd, true)`（793）** → `CreateVisual()`（794）
  → `set_swap_chain`: `SetContent(swap_chain)`（805）→
  `SetRoot(&comp_visual)`（806）→ `Commit()`（807）。
- 保持: `DirectXRenderer.direct_composition: Option<DirectComposition>`
  （46）。`DirectComposition` は private struct（93-97）。
- 所有: `WindowsWindowState.renderer`（window.rs:51）→ window 寿命。
- device-lost: `handle_device_lost_impl`（226-284）が devices/resources/
  pipelines/DirectComposition を全再生成。復旧時の disable 判定は env
  ではなく `self.direct_composition.is_none()`（227）で mode を維持。
- Drop: `DirectXRenderer` の Drop（997-1006）+ `DirectComposition` の
  COM Release（通常 field 自動 drop）。

`CreateTargetForHwnd`/`CreateVisual`/`SetContent`/`SetRoot`/`Commit`/
`DCompositionCreateDevice`/`CreateSwapChainForComposition`/
`CreateSwapChainForHwnd`/`MakeWindowAssociation` の出現は全て
`directx_renderer.rs` 内の各 1 箇所。`IDComposition*` 参照は同ファイル
のみ（`vsync.rs` はコメント言及のみ）。`src/platform/blade/` は
linux/freebsd/macos 限定で Windows 非対象（platform.rs:16-18）。

### F2: `GPUI_DISABLE_DIRECT_COMPOSITION` の意味論

- const: `pub(crate) const DISABLE_DIRECT_COMPOSITION` =
  `"GPUI_DISABLE_DIRECT_COMPOSITION"`（directx_renderer.rs:29）。
- 読み取り: `platform.rs:138-139`、Windows platform 層唯一の
  `std::env::var`。受理値は `"true"` と `"1"` のみ
  （`is_ok_and(|v| v == "true" || v == "1")`）。
- タイミング: `WindowsPlatform::new` で 1 回。process-wide・window 生成前
  設定必須・runtime 切替不可。
- 非 DComp 経路の実動作（`disable_direct_composition` 分岐の全容）:
  1. `WS_EX_NOREDIRECTIONBITMAP` を exstyle に付けない（window.rs:409-411
     skip）→ DWM redirection surface が割り当てられる。
  2. `dxgi_device` を作らない（directx_renderer.rs:110-114）。
  3. `DirectComposition` object を作らない（147-156）。
  4. swapchain = `CreateSwapChainForHwnd`（1073）+
     `MakeWindowAssociation(DXGI_MWA_NO_ALT_ENTER)`（1074）。
     desc: `DXGI_SCALING_NONE` + `DXGI_ALPHA_MODE_IGNORE`（1056-1071）。
     composition 経路との差分は `DXGI_SCALING_STRETCH` +
     `DXGI_ALPHA_MODE_PREMULTIPLIED`（1027-1043）。
     `FLIP_SEQUENTIAL`・`BufferCount=3`・`B8G8R8A8_UNORM` は共通。
  5. `Present`/vsync/描画は両経路で同一（205、vsync.rs `DwmFlush`）。
- **含意**: env var は `topmost` を patch せずに DComp 経路全体を
  迂回する **zero-code 代替経路**。child HWND は通常の DWM z-order
  合成に復帰し plugin 画素が見える可能性がある（仮説・実機検証待ち）。
  唯一の既知 blocker は exit-127（F5）。

### F3: public vs `pub(crate)` の到達可能性

- **到達不能**: `gpui::platform`（private mod、gpui.rs:31）配下の全て。
  `Platform`/`PlatformWindow` trait（platform.rs:163/460 `pub(crate)`）、
  `WindowsPlatform`/`WindowsWindow`/`WindowsWindowInner`/
  `WindowsWindowState`/`WindowCreationInfo`（platform.rs:818）、
  `DirectXRenderer`/`DirectXRendererDevices`/`DirectXDevices`/
  `DirectXAtlas`/`DirectWriteTextSystem`、`DirectComposition`
  （directx_renderer.rs:93、private struct）、
  `DISABLE_DIRECT_COMPOSITION` const（29）。DComp device/target/visual/
  swapchain/renderer/window exstyle のいずれも公開 API なし。
- **到達可能**: `gpui::Window` の `HasWindowHandle` impl
  （src/window.rs:4845-4849）→ `Win32WindowHandle` → **実 HWND**。
  ruviform は既に `surface.rs:132-138` で使用中。
- **注意**: `Window` の `HasDisplayHandle` は `WindowsWindow::
  display_handle` = `unimplemented!()`（platform/windows/window.rs:
  496-497）へ転送され Windows で呼ぶと panic。ruviform 未使用。
- 唯一の downstream 到達可能な合成経路スイッチは env var
  `GPUI_DISABLE_DIRECT_COMPOSITION`（process-wide）。

### F4: Option A のパッチ面積と upstream 情勢

- `topmost` literal は **`directx_renderer.rs:793` の `true` 1 語のみ**。
  `false` への変更は `DirectComposition::new` 内部の 1 token diff で、
  呼び出し側 2 箇所（150/265）に自動適用。シグネチャ変更不要。
- ただし `[patch.crates-io]`/fork/vendor が前提であり、upstream に
  新しい publishable 版がない（F6）ため patch base は 0.2.2 のみ。
- `topmost=false` は GPUI visual tree 全体を child HWND 背面へ移す。
  plugin HWND が GPUI UI の上に描かれる方向だが、全 child window に
  グローバルに効く点と、NOREDIRECTIONBITMAP window での non-topmost
  表示可否は fixture 実測が必要（断定禁止 — 先行 discovery OQ と同じ
  未検証状態を維持）。

### F5: exit-127 のコードパス仮説（PENDING）

- gpui Windows 経路に `process::exit`/`abort` の明示呼び出しなし
  （crate 内の `process::exit` は mac/metal_renderer.rs:139 `exit(1)`
  のみ、platform.rs:1054 はコメントアウト済み）。ruviform/PoC も
  exit(127) を呼ばない（PoC は 1/2/3、plugin host test worker は
  既定 3）。
- 非 DComp 経路の失敗候補（`CreateSwapChainForHwnd` 1073・
  `MakeWindowAssociation` 1074・resources 分岐 663-672・window 作成
  window.rs:409-411）は全て `anyhow::Result` で Err 伝播し、exit 127
  を直接生まない。伝播先は `open_window` Result → caller unwrap →
  panic（通常 exit 101）。
- 仮説: **H1** 観測は wrapper/launcher の exit code 混同の可能性
  （127 = POSIX command-not-found、artifact なし）、**H2** WndProc 内
  panic/abort（exit code は runtime 依存で 101 とは限らない）、
  **H3** DXGI 失敗の別終了経路、**H4** 子 process の exit code 混同、
  **H5** device-lost は回復機構に吸収され exit しない。
- 詳細は `evidence/rf532/gpui-public-surface-audit.md` 監査項目 7。
  **実 OS での stderr/exit code 捕捉再現が必要で本 slice では close
  しない**。

### F6: gpui バージョン/アップグレード情勢

- crates.io `gpui` 最新 publishable = **0.2.2**（2025-10-22 publish）。
  ruviform pin と一致。
- zed repo 現行 gpui は publish 不能: `` `zed-industries/zed#55271` ``
  （2026-08-19 LeadcodeDev コメント）— `gpui_macros` が自身の manifest
  で `publish = false`、`gpui_shared_string`/`gpui_util` が workspace
  `publish = false` を継承し `cargo publish -p gpui` が失敗。0.2.2 は
  platform split（`zed-industries/zed#49277`）以前の「構造的に別の
  crate」。crates.io 版への upstream 修正降下の見込みなし。
- 帰結: 0.2.2 脱却の手段は zed git dep / fork+`[patch]` / 現行 0.2.2
  上の回避（env var・private interposition）のみ。

### F7: Option B（private 合成）の到達可能な設計空間

- gpui 内部再利用は不可能（F3）。公開面は HWND + windows-sys 0.61
  （DirectComposition/Dxgi/D3D11 feature 未導入）。
- 候補（全て未実装・実機検証待ち）:
  - **B-env**: `GPUI_DISABLE_DIRECT_COMPOSITION=1`（zero code、
    exit-127 が blocker）。
  - **B-region**: GPUI hwnd への `SetWindowRgn` で dock 領域を除外し
    DComp 画素を clip — window region が DComp content に適用されるか
    要検証。layout 追従コストが論点。
  - **B-dcomp2**: 自前 `IDCompositionDevice` +
    `CreateTargetForHwnd(hwnd, false)` の第二 target — 同一 HWND への
    複数 target 共存と non-topmost 表示可否が未確認。windows-sys への
    feature 追加（`Win32_Graphics_DirectComposition`/`Dxgi`/
    `Direct3D11`）が必要。system DLL のみで新規 binary 依存なし。
  - **B-swapchain**: 独自 present — gpui が window/present を所有し
    非現実的。
- 結論: Option B の実質的な private 面積は「OS 公開 API による
  interposition」であり、DComp 内部再利用ではない。

## Alternatives and Comparison

方式 A/B/C の比較枠組みは先行 discovery・`docs/designs/
issue-532-dcomp-native-dock.md` を継承。本監査が各方式へ追加する
技術入力:

| 方式 | 本監査が追加する事実 | 残る不確実性 |
| --- | --- | --- |
| A: GPUI 内最小修正 | `topmost` literal は 1 箇所（793）のみ・patch 面積最小。upstream publishable 新版なし → fork/vendor が前提 | `topmost=false` の実画素・z-order 効果（fixture 実測待ち） |
| A': env var 迂回（新規区分） | `GPUI_DISABLE_DIRECT_COMPOSITION` は patch 不要の zero-code 経路。非 DComp 経路の全差分をコードで確認済み | exit-127 の原因（PENDING）・実画素での child 合成（未検証） |
| B: Windows private 合成 | gpui DComp 内部は全て private で再利用不能。実質的選択肢は env var / `SetWindowRgn` / 第二 DComp target の OS interposition | 各候補の実機 semantics（region の DComp clip・第二 target 共存） |
| C: 代替 Native 経路 | 変更なし（HWND は公開取得可能だが top-level window 方式は別途設計が必要） | 実 plugin 画素の普遍性（先行 OQ のまま） |

## Tentative Conclusions and Hypotheses

- **confirmed**: `CreateTargetForHwnd(hwnd, true)` は crate 全体で
  `directx_renderer.rs:793` の 1 箇所のみ。Option A の patch 面積は
  literal 1 語 + fork/vendor 運用。
- **confirmed**: gpui の DComp/DXGI/renderer/window internals は全て
  `pub(crate)` 以下で downstream から到達不能。公開面は HWND
  （`HasWindowHandle`）と env var のみ。
- **confirmed**: `GPUI_DISABLE_DIRECT_COMPOSITION` は `"true"`/`"1"`
  のみ受理・process-wide・platform 初期化時 1 回読み。非 DComp 経路は
  `CreateSwapChainForHwnd` + `WS_EX_NOREDIRECTIONBITMAP` なしの通常
  DWM 合成。
- **confirmed**: gpui 0.2.2 が最新 publishable 版。zed repo 現行版は
  publish 不能（`` `zed-industries/zed#55271` ``）。
- **hypothesis**: env var 経路（A'）が child HWND の DWM 通常合成を
  復帰させ plugin 画素を表示する可能性 — exit-127 解消と実機 fixture
  で確かめるまで未検証。
- **hypothesis**: `topmost=false`（A）が child HWND を GPUI visual の
  背面に描画し dock 用途では plugin が上に来る可能性 — 同上。
- **hypothesis**: `SetWindowRgn` punch-out（B-region）が DComp content
  を clip し child を透かす可能性 — window region と DComp の相互作用は
  実機検証が必要。

## L2/L3 再開条件への影響

先行 discovery の再開条件表（対象 OS・GPU・GPUI version・feature
tuple・実 Plugin・Window/DPI・測定環境・証拠・Owner）は変更なし。
本監査は「再開時に最初に試すべき zero-code 経路（env var）と、その
blocker（exit-127）の切り分け手順」を追加するに留まる。実機調査の
手順設計は `docs/designs/issue-532-gpui-dcomp-surface.md` を参照。

## Open Questions

先行 discovery（`issue-532-dcomp-native-dock.md`）の Open Questions
への回答状況:

- `topmost=false` の z-order/表示効果 → **依然 open**（実機 fixture
  待ち）。本監査で patch 面積（literal 1 語）と呼び出し経路は確定。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` の exit-127 → **依然 open**
  （PENDING）。コードパス仮説 H1〜H5 を記録、実 OS 再現が必要。
- Option B が必要とする upstream private API → **解消**: 必要な
  internal は全て `pub(crate)` で到達不能。Option B の実質的設計空間は
  OS 公開 API interposition（env var / `SetWindowRgn` / 第二 DComp
  target）に限定されることを確認。
- 実 plugin の offscreen surface 普遍性 → **依然 open**（実 plugin
  binary 検証が必要、本 slice 範囲外）。

## Promotion Candidates

- [x] spec-doc needed: 本 slice の受入条件を
  `docs/specs/issue-532-gpui-dcomp-surface.md` として固定（済）。
- [x] design-doc needed: 監査結果の方式 A/B/C への写像を
  `docs/designs/issue-532-gpui-dcomp-surface.md` として設計記録（済）。
- [ ] adr-doc needed: 方式 A/B/C（または A' env var）の採否が決まった
  時点で #262 の ADR 入力として別途起票（将来・本 slice 対象外）。

## Sources and Relations

- `docs/discovery/issue-532-dcomp-native-dock.md` — 先行監査（正本・
  resolved）。本監査はその Open Questions の upstream 調査分を処理。
- `docs/designs/issue-532-dcomp-native-dock.md` — L2/L3 再開条件・
  方式 A/B/C 調査手順の設計（変更なし・参照のみ）。
- `evidence/rf532/gpui-public-surface-audit.md` — 本監査の証拠ノート
  （コードパス・行番号・判定根拠の全文）。
- `evidence/rf388-native-dock-windowing/report.md` — DComp 制約の実機
  証拠（exit-127 観測の出所）。
- vendored `gpui-0.2.2` source（cargo registry、読み取り専用）—
  監査対象。行番号は `evidence/rf532/gpui-public-surface-audit.md` に
  集約。
- crates.io `gpui` version history — 0.2.2 が最新 publishable。
- `` `zed-industries/zed#55271` `` — gpui publish 不能の技術的根拠
  （inline code 参照・auto-reference 抑制の repo 規約に従う）。
- `crates/ruviform-ui-gpui/Cargo.toml`・`crates/ruviform-ui-gpui/src/
  gpui_driver/surface.rs` — ruviform 側の gpui pin・HWND 取得経路。

## Evidence

### 2026-11-15 — 監査実施記録（本 slice）

- **監査対象**: vendored `gpui-0.2.2`
  （`index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/`）。repo HEAD は
  `docs/532-gpui-dcomp-surface-audit` branch（base = origin/main
  `3b8b73224f0f792d70f260e764a7b5b8f4c0aa4e`）。
- **実施した調査**: `src/platform/windows/{directx_renderer,platform,
  window,directx_devices,vsync,events,windows}.rs`・`src/gpui.rs`・
  `src/platform.rs`・`Cargo.toml` の精読。`CreateTargetForHwnd`/
  `IDComposition*`/`DComposition*`/`GPUI_DISABLE_DIRECT_COMPOSITION`/
  `process::exit`/`env::var` の crate 全体 grep。crates.io version
  history と `` `zed-industries/zed#55271` `` の web 確認。ruviform 側の
  `surface.rs` HWND 取得経路・`Cargo.toml` windows-sys feature の確認。
- **実施していない検証**: 実画面・実 plugin・実 GUI の L2/L3 試験、
  exit-127 の実 OS 再現、`topmost=false`/env var/`SetWindowRgn` の
  実画素効果、patch/vendor の適用（全て対象外・PENDING）。
- **判定**: 監査 7 項目中 6 項目を確定事実として固定（F1-F4・F6・F7）。
  exit-127 は仮説記録のみで open 維持（F5）。先行 discovery の
  Open Questions 3 件を解消・1 件を部分更新（patch 面積確定・効果は
  open）・2 件を open 維持。
