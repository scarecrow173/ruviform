---
id: "DESIGN-034YnzqSiMt4kOkhHo0CBy"
type: "design"
status: "approved"
title: "Issue #532 dcomp-occlusion 設計（query-only 遮蔽層診断の構成・実走手順・二分解釈）"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-dcomp-occlusion.md"
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
    - "docs/specs/issue-532-dcomp-occlusion.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-631-real-window-dock.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
    - "evidence/rf388-native-dock-windowing/report.md"
    - "evidence/rf631/verification.md"
  refines:
    - "docs/designs/issue-532-dcomp-pixel.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
  derived-by:
    - "docs/plans/issue-532-dcomp-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 dcomp-occlusion 設計（query-only 遮蔽層診断の構成・実走手順・二分解釈）

## Context

`rf631.plugin-dock-real` lane の `real-dcomp-pixel` case（前回
slice `issue-532-dcomp-pixel` で追加）は、docked `test_editor`
実 child HWND の screen-DC `BitBlt` 画素を主 oracle に
`EDITOR_COLOR` 緑の出現を判定し、baseline/env-on 両条件で正直な
`Fail`（screen green = 0.0、env-on でも `windowdc` = 1.0 なのに
screen = 0.0）を固定済み。残る問いは**遮蔽がどの層で起きるか**:

- GPUI root HWND が `WS_EX_NOREDIRECTIONBITMAP` を持つか
  （= DComp 経路が実際に有効か。`GPUI_DISABLE_DIRECT_COMPOSITION`
  の実効確認を兼ねる）
- WM レベルで child が topmost か（container/root の sibling
  z-order、GPUI 自身の子 window の有無）
- `WindowFromPoint` が child rect 内で何を返すか
  （WM 級遮蔽なら child 以外が返る。child が返るのに screen が
  0.0 なら遮蔽は合成器 — DComp visual または flip-model 提示
  plane — レベル）

window 階層の前提（実装確認済み）: GPUI window（top-level、
`WS_EX_NOREDIRECTIONBITMAP` は DComp 有効時のみ）→ container
HWND（`STATIC` class、`WS_CHILD|WS_VISIBLE|WS_CLIPCHILDREN|
WS_CLIPSIBLINGS`、`GpuiSurfaceAdapter` backend が作成）→
plugin child HWND（`test_editor` の `IPlugView`）。
`GetAncestor(child, GA_ROOT)` は GPUI window に到達する
（`ospix::presented` が既に同じ walk を使用）。

## Scope

### In Scope

- `src/osoccl.rs` 新規 module（`real-dock` feature、
  `#[cfg(windows)]` 実装 + `#[cfg(not(windows))]` stub）—
  `oschild`/`ospix` と同じ query-only・plain 値境界規約。
- `real_dock.rs` の `real-dcomp-pixel` 経路への note 級診断
  step row 3 系の配線（verdict 不変）。
- baseline/env-on の interactive 実走と証拠記録、遮蔽層二分の
  doc chain 記録。

### Out of Scope

- 方式採否・本 fix・変異系 probe・`mise.toml`・docs index・
  他 crate（spec の Out of Scope を継承）。

## Components and Boundaries

- **`src/osoccl.rs`（新規、`#[cfg(feature = "real-dock")]`）**:
  遮蔽層診断の private module。`ospix` と同じ境界規約 —
  window の作成・破棄・移動・属性変更・メッセージ送信は行わない
  （`GetAncestor`/`GetWindowLongPtrW`/`GetWindow`/`GetParent`/
  `GetClassNameW`/`GetWindowRect`/`IsWindow`/`IsWindowVisible`/
  `WindowFromPoint` の読み取り系のみ）。戻り値は `u64` HWND・
  `Option`/`Result`・整数・文字列の plain 値 — Win32 型は
  module 内に閉じる。公開入口は `probe(child: u64) ->
  OcclusionFacts` のみ。
- **`src/real_dock.rs`**: `observe_one_child_pixels` から
  `record_occlusion_notes` を呼び、facts を 3 系の note step
  row に整形する。role 分類（`editor_child`/`container`/
  `root`/`other`/`none`）と `above_*` 判定はここで行う
  （既知 HWND 集合を持つのは caller 側）。
- **`src/lib.rs`**: `#[cfg(feature = "real-dock")] mod osoccl;`
  を追加 — module 名は `osoccl`（`oschild`/`ospix` と同じ
  `os*` 命名）。
- **`Cargo.toml`**: 変更なし — 対象 API は全て既存 optional
  `windows-sys` edge の `Win32_UI_WindowsAndMessaging` +
  `Win32_Foundation`（`POINT`）feature 内。

## Data and Control Flow

### module API（`osoccl`）

```rust
/// 1 window の識別 facts — plain 値のみ。
pub(crate) struct WindowRow {
    pub hwnd: u64,
    /// sibling z-order 位置（0 = topmost）。z-order 走査由来のみ Some。
    pub z_index: Option<u32>,
    /// GetClassNameW — 失敗時 None。
    pub class_name: Option<String>,
    /// GetWindowRect（screen 座標）— 失敗/空時 None。
    pub rect: Option<ospix::ScreenRect>,
    pub visible: bool,   // IsWindowVisible
    pub alive: bool,     // IsWindow
}

/// GWL_EXSTYLE の分類済み facts。
pub(crate) struct ExstyleFacts {
    /// 生の extended style 値（u64 narrow）。
    pub value: u64,
    /// WS_EX_NOREDIRECTIONBITMAP — GPUI DComp 経路の marker。
    pub noredirectionbitmap: bool,
    /// その他の合成関連 flag: topmost/layered/transparent/
    /// noactivate/toolwindow/appwindow。
    pub topmost: bool,
    pub layered: bool,
    pub transparent: bool,
    pub noactivate: bool,
    pub toolwindow: bool,
    pub appwindow: bool,
}

/// z-order 走査の結果 — 先頭が topmost。
pub(crate) struct ZOrder {
    pub children: Vec<WindowRow>,
    /// 安全上限で走査を打ち切った場合 true。
    pub truncated: bool,
}

/// WindowFromPoint の 1 probe 結果。
pub(crate) struct PointHit {
    /// probe 点（virtual-screen 座標）。
    pub x: i32,
    pub y: i32,
    /// 返った window の facts — WindowFromPoint が NULL を返した
    /// 場合は None。
    pub window: Option<WindowRow>,
}

/// `probe` の戻り値 — 各部分は独立に degrade する。
pub(crate) struct OcclusionFacts {
    /// false は非 Windows stub（観測手段なし — caller は
    /// `observation.supported` で既に gate 済み）。
    pub supported: bool,
    /// `GetAncestor(child, GA_ROOT)` — GPUI top-level window。
    pub root: Option<u64>,
    /// root の GWL_EXSTYLE facts — root 取得または style 読取の
    /// 失敗時 None。
    pub root_exstyle: Option<ExstyleFacts>,
    /// root の direct children を z-order 順に — container の
    /// sibling（GPUI 自身の子 window を含む）を列挙。
    pub root_children: Result<ZOrder, String>,
    /// `GetParent(child)` — 通常は container token と一致。
    pub parent: Option<u64>,
    /// parent の direct children を z-order 順に — child の
    /// sibling を列挙。
    pub parent_children: Result<ZOrder, String>,
    /// child rect 内の代表点での WindowFromPoint 結果列。
    pub hits: Vec<PointHit>,
    /// child の screen rect — probe 点選定に使用。失敗時 None
    /// （詳細は先行する `screen_rect` step row が持つ）。
    pub child_rect: Option<ospix::ScreenRect>,
}

/// `real-dcomp-pixel` の docked child 1 つに対する遮蔽層診断。
/// query-only — window を作らない・動かさない・message を送らない。
pub(crate) fn probe(child: u64) -> OcclusionFacts;
```

### probe 点の選定

`child_rect`（`ospix::window_screen_rect` 再利用 — DRY）から
5 点: center + 四隅寄りの `(w/4,h/4)`・`(3w/4,h/4)`・
`(w/4,3h/4)`・`(3w/4,3h/4)`。複数点で「一部だけ遮蔽」の
見落としを防ぎ、安定性も観測できる。rect 取得失敗時は `hits`
空・`child_rect` None。

### z-order 走査

`GetWindow(parent, GW_CHILD)` → 先頭 sibling、以後
`GetWindow(current, GW_HWNDNEXT)` を NULL まで反復。
`GW_CHILD`/`GW_HWNDNEXT` は direct children の z-order（先頭が
topmost）を返す — `EnumChildWindows` は全子孫を返し順序保証が
ないため z-order には使わない。安全上限（1024）超過で
`truncated=true`。

### exstyle 診断 (a) の解釈

`WS_EX_NOREDIRECTIONBITMAP` は gpui `window.rs:409-411` で
`!disable_direct_composition` のときのみ付与される（audit
確定）。したがって:

- baseline（env 未設定）: flag **あり** が期待値 — DComp 経路。
- env-on（`"1"`/`"true"` 受理値）: flag **なし** が期待値 —
  env var が GPUI 初期化に実効した証拠。flag が残っていれば
  env var 未実効（受理値外・読み取り経路の問題）を意味する。

### 配線（`real_dock.rs`）

`observe_one_child_pixels` の末尾（`record_windowdc_pixels`
の後）に `record_occlusion_notes(index, child, container,
steps)` を追加。`observe_child_pixels` が既に持つ `container`
token（= `Option<u64>`、evaluability check 通過後は `Some`）を
`observe_one_child_pixels` へ渡す。child ごとに 3 row:

- `occlusion_exstyle`（note）: `{hwnd, root, exstyle,
  noredirectionbitmap, topmost, layered, transparent, noactivate,
  toolwindow, appwindow}` または `{hwnd, error}`。
- `occlusion_zorder`（note）: `{hwnd, parent, parent_children:
  [{hwnd, z_index, role, above_editor, class_name, rect,
  visible, alive}...], root_children: [{..., role,
  above_container, ...}...], truncated}`。`role` は
  `editor_child`/`container`/`root`/`other`、`above_*` は
  参照 window より z-order 上位（index が小さい）か。
- `occlusion_hit_test`（note）: `{hwnd, child_rect, points:
  [{x, y, hwnd, role, class_name, visible, alive}...]}`。
  `role` は hit hwnd と `{child, container, root}` の一致で
  `editor_child`/`container`/`root`/`other`/`none`。

いずれも `verdict: "note"` — oracle 判定・`PixelScan`・
`PixelObservation`・case verdict に一切影響しない。

### 二分解釈マトリクス（観測の正直な記録・断定しない）

| 観測 | 解釈 |
| --- | --- |
| env-on で root exstyle に `WS_EX_NOREDIRECTIONBITMAP` が残る | env var が GPUI 初期化に実効していない（受理値・経路の問題）— 確定的事実 |
| env-on で flag が外れ、`WindowFromPoint` が child を返し screen = 0.0 | WM 的には child が topmost → 遮蔽は合成器レベル（DComp visual / flip-model 提示 plane iFlip・DirectFlip・MPO）。**方式 A `topmost=false` では救えない可能性** |
| `WindowFromPoint` が root・container・他 window・GPUI 子を返す | WM 級 z-order/clip 遮蔽 — sibling z-order 行で犯人を特定可能 |
| baseline で flag あり + `WindowFromPoint` が child を返す | 従来の DComp topmost 機序（visual が children の上に合成）と整合 |

`WindowFromPoint` の limitation: GPU 合成経路（DComp visual /
flip-model plane）下で返す答えの決定性は未検証 — 観測値を
実測として記録し、解釈は推測と区別する。

### 実走手順

1. canonical: `mise run test-ui-plugin-dock-real`（env 未設定）
   → `evidence/rf631/plugin-dock-real-scenario.jsonl`。
2. 補助 env-on: `GPUI_DISABLE_DIRECT_COMPOSITION=1` 前置で直接
   `cargo run --locked -p ruviform-plugin-dock-test
   --features real-dock --bin plugin-dock-real-scenario -- --out
   evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl`
   （canonical JSONL を上書きしない — 前回と同じ命名・
   「補助証拠（canonical = mise）」明記）。
3. exit code・stderr を捕捉。exit 127 観測時は H1-H5 に分類。
4. 結果を `verification.md` 追記節・JSONL・doc chain に記録。

## Non-Functional Constraints

- `unsafe` は Win32 FFI 境界の `#[expect(unsafe_code)]` +
  SAFETY コメントの局所許可のみ（`oschild`/`ospix` 先例）。
- HWND は `u64` — Win32 型・GPUI 型を公開 API/evidence schema
  に出さない（`check-ui-driver-boundary` green）。
- query-only — 変異系 API（`SetWindowRgn`/`SetWindowPos`/
  `SetWindowLongPtrW`/`SetFocus` 等）は呼ばない。
- `GetWindowTextW` は window text を読むが cross-process で
  message を伴いうるため使わず、`GetClassNameW`（内部 class
  atom の直接読み）に留める — 厳密な query-only 境界。
- RT contract: non-RT only（既存 lane と同じ）。
- 依存追加なし — `windows-sys` 既存 optional edge の feature
  範囲内。`Cargo.lock` 差分が出た場合は再生成のみ。
- `WindowFromPoint` は scenario thread から同期的に呼ぶ
  （読み取り系・任意 thread から可）。

## Risks and Trade-offs

- **`WindowFromPoint` の答えの解釈**: hit-test は WM の
  z-order/visibility のみを見る — DComp visual や flip-model
  提示 plane は HWND ではないため hit-test に現れない。
  「child が返るのに screen = 0.0」は合成器レベル遮蔽の傍証
  になるが、`WindowFromPoint` が GPU 合成下で常に決定的な答えを
  返すかは未検証 — limitation として明記し断定しない。
- **exstyle の解釈限界**: flag 有無は env var の実効を示すが、
  flag が外れていても非 DComp swapchain（`CreateSwapChainForHwnd`
  + `FLIP_SEQUENTIAL`）の提示 plane が child を覆う可能性は
  排除できない — 二分結果は「次の一手」を絞るもので機序の
  最終確定ではない。
- **z-order 走査の情報量**: GPUI が内部で子 HWND を持つかは
  未知 — 観測して初めて分かる。列挙結果はありのまま記録し、
  解釈は証拠の範囲に留める。
- **headless 化リスク**: pool が headless 化した場合 real lane
  は全 case `NotRun`+`environment_blocked`（exit 0）— 実走部が
  実機 gate 化することを PR/verification に明記。

## Deferred Design Concerns

- 方式 A patch・B-region・B-dcomp2・C の採否と本 fix | 本
  slice の二分結果が入力 | 後続 slice | 観測なしの採否は
  提示 plane 系機序を見落とすリスク。
- 変異系 probe（`SetWindowRgn` punch-out・`SetWindowPos`
  z-order 実験） | 方式実験は adapter 側の後続 slice | 方式
  採否後 | query-only 境界の侵犯。
- `WindowFromPoint` 以外の hit-test 系（`RealChildWindowFromPoint`/
  `ChildWindowFromPoint`・`WindowFromPhysicalPoint`） | 本 slice
  の 3 系で二分が付かない場合の拡張候補 | 二分不能が判明した
  場合 | 診断網羅性の不足。
- macOS/Linux での同等診断 | `NativePluginDock` が
  Windows-only | GUI-L2 利用要件発生時 | 3OS 対応は Deferred。

## Verification Notes

- [x] 診断 3 系が note 級・verdict 非干渉・query-only である
  設計。
- [x] 対象 API が既存 `windows-sys` feature 内にあることを
  crates.io source で確認（`GetWindowLongPtrW`/`GetWindowLongW`/
  `WindowFromPoint`/`GetWindow`/`GetClassNameW`/
  `EnumChildWindows`/`GetAncestor`/`GWL_EXSTYLE`/
  `WS_EX_NOREDIRECTIONBITMAP`/`GW_CHILD`/`GW_HWNDNEXT`/`GA_ROOT`）。
- [x] env-on run が canonical JSONL を上書きしない手順。
- [ ] `mise run check/lint/test ruviform-plugin-dock-test`
  （default + `real-dock`）green — 実装後に確認。
- [ ] 実機 lane baseline/env-on の実走記録 — plan/task の
  検証手順どおり実施。
