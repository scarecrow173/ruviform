---
id: "DESIGN-RLPMudBgwjqSh9kxCAvNCq"
type: "design"
status: "approved"
title: "Issue #532 presentation-probe 設計（cover inventory・capture path variant・fingerprint・verdict 意味論）"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-presentation-probe.md"
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
    - "docs/specs/issue-532-presentation-probe.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-presentation-probe.md"
    - "docs/designs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-presentation-probe.md"
    - "docs/designs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-native-dock.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-532-foreground-probe.md"
    - "docs/specs/issue-532-foreground-probe.md"
  derived-by:
    - "docs/plans/issue-532-presentation-probe.md"
    - "docs/tasks/issue-532-presentation-probe.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #532 presentation-probe 設計（cover inventory・capture path variant・fingerprint・verdict 意味論）

## Context

foreground-probe の実走証拠（`evidence/rf532/foreground-probe-bisect.md`）
で残った 3 候補を判別する。本設計の判断軸は「各候補を分離する
最小の観測集合」と「観測だけで判別不能な部分を正直に
`unresolved` として残す境界」。

判別ロジック（設計上の対応表 — 実走結果の解釈は evidence 文書
側の責務）:

- **(a) hit-test-transparent cover**: `EnumWindows` inventory が
  child rect と交差する全 top-level window を列挙する。
  `WindowFromPoint` は transparent/layered/disabled/hidden window
  を skip するため、hit-test が child を返しても visual cover が
  残り得る。fingerprint で screen capture と suspect raster を
  対比すれば「screen に出ているものがどの HWND の surface か」
  が分かる。
- **(b) child presentation 経路**: child の window DC が green
  を持つのに container/root の `PrintWindow` raster に green が
  無ければ、child surface → 親 raster の presentation 段で
  落ちている。root の `PW_RENDERFULLCONTENT` variant は DComp/
  flip-model content を捕捉し得る経路で、child green が root
  raster にだけ現れるかが分岐点。
- **(c) capture 盲点**: screen `BitBlt` が 0 でも root の
  `PrintWindow(PW_RENDERFULLCONTENT)` が green を示せば、
  「合成結果にはあるが BitBlt screen DC が捉えない」= capture
  経路の盲点が支持される。fingerprint がどの suspect にも
  一致しない場合は HWND を持たない plane（DComp visual 等）を
  疑う方向のデータになる。

## Scope

### In Scope

- 新規 `src/ospres.rs`（`#[cfg(feature = "real-dock")]`、
  query-only）— top-level inventory + presentation attributes。
- `src/ospix.rs` — `capture_print_window_flags(hwnd, flags)` の
  追加（既存 `capture_print_window` はその wrapper として維持）。
- `src/real_dock.rs` — `real-dcomp-present` case、probe 本体、
  fingerprint 集計 helper、catalog 配線。
- `src/real_catalog.rs`・`tests/console.rs`・`lib.rs`・
  `Cargo.toml`（feature comment のみ）・`README.md` の整合。

### Out of Scope

- `osmut` の変更・再利用（変異なし — 下記評価参照）。
- `osoccl`/`oschild` の契約変更（既存 `probe`/`observe` をそのまま
  再利用する）。
- consumer crate・adapter・GPUI vendor・production compositor・
  `mise.toml`・docs index。
- Issue `#532` の解決・方式採否・close・merge。

## Components and Boundaries

### `src/ospres.rs`（新規、query-only）

境界規約は `osoccl`/`ospix` と同一: HWND は `u64`、返り値は
plain record のみ、Win32 型は module 内に閉じる、unsafe は FFI
wrapper に限定（`#[expect(unsafe_code)]` + SAFETY）、非 Windows
は `supported:false` stub。

公開面（`pub(crate)`）:

- `TopLevelWindow` — 1 top-level window の識別 facts:
  `hwnd: u64`、`z_index: u32`（`EnumWindows` 列挙順 = z-order、
  0 が最前面）、`class_name: Option<String>`、
  `rect: Option<ScreenRect>`、`visible: bool`、`alive: bool`、
  `disabled: Option<bool>`、`exstyle: Option<osoccl::ExstyleFacts>`、
  `owner: Option<u64>`（`GetWindow(GW_OWNER)`）、
  `pid: Option<u32>`・`owner_pid: Option<u32>`
  （`GetWindowThreadProcessId`）。
- `PresentationAttributes` — 1 HWND の presentation facts:
  `supported: bool`、`cloaked: Option<u32>`（`DWMWA_CLOAKED`
  生値 — shell/app/inherited cloak bit）、`region: Option<
  RegionFacts>`、`update: Option<UpdateFacts>`、
  `layered: Option<LayeredFacts>`。各 query は独立に縮退する。
- `RegionFacts { region_type: i32, bounds: Option<ScreenRect> }`
  — `GetWindowRgn` の戻り値（NULLREGION/SIMPLEREGION/
  COMPLEXREGION/ERROR の生値）と `GetRgnBox` の window 座標
  bounding box。`NULLREGION` = window 全体が clip されている
  重要なシグナル。
- `UpdateFacts { has_update: bool, rect: Option<ScreenRect> }`
  — `GetUpdateRect(erase=FALSE)`。`has_update=false` は
  pending paint なし（正常）。client 座標系であることを
  フィールド名/記録で明示する。
- `LayeredFacts { alpha: Option<u8>, color_key: Option<u32>,
  flags: Option<u32> }` — `GetLayeredWindowAttributes`。
  `WS_EX_LAYERED` でない window では API が FALSE を返す
  → `None` に縮退（「layered でない」は exstyle 側が既に
  記録している）。
- `inventory() -> Inventory` — `EnumWindows` 全 top-level を
  z-order で列挙。`Inventory { supported, own_pid: u32,
  windows: Vec<TopLevelWindow>, total: usize, truncated: bool }`。
  cap は `INVENTORY_CAP = 256`（実デスクトップは ~30、
  `truncated` で正直に記録）。`own_pid` は
  `GetCurrentProcessId` — caller が own-process/foreign を
  `pid == own_pid` で判定するための値。
- `attributes(hwnd: u64) -> PresentationAttributes` — 対象
  HWND の presentation attributes。chain（child/container/
  root）と cover 候補に対して呼ぶ。

Windows 実装の注意:

- `EnumWindows` の callback は `oschild::collect` と同じ
  `unsafe extern "system" fn` + `LPARAM` パターンで HWND を
  `Vec<u64>` に集め、その後各 window の facts を個別 query で
  構築する（callback 内で重い query をしない）。
- `DwmGetWindowAttribute` は windows-sys に無いため
  `#[link(name = "dwmapi")] unsafe extern "system"` の local
  宣言（`PrintWindow` の先例と同じ）。`DWMWA_CLOAKED = 14`、
  `cbAttribute = 4`（DWORD）。HRESULT `S_OK = 0` のみ値を採用、
  それ以外は `None`。
- `GetWindowRgn` は `CreateRectRgn(0,0,0,0)` で作った HRGN を
  渡し `GetRgnBox` で bounding box を読み `DeleteObject` で
  破棄する。戻り値の region type 生値を記録し、`GetLastError`
  で失敗を区別する。
- `GetUpdateRect`/`GetLayeredWindowAttributes`/`GetWindowRgn` は
  いずれも message を送らない query API（`GetWindowTextW` の
  ような cross-process `WM_GETTEXT` リスクは無い）。
- `GetWindowThreadProcessId` は pid を out-param で返す
  （thread id が戻り値）。owner pid は `GetWindow(GW_OWNER)`
  → その hwnd の pid。

### `src/ospix.rs`（拡張）

- `capture_print_window_flags(hwnd: u64, flags: u32)
  -> Result<PixelCapture, String>` を追加。`flags` は
  `PrintWindow` の `nflags` にそのまま渡す。DIB サイズは
  `flags & PW_CLIENTONLY != 0` なら `GetClientRect`、それ以外は
  `GetWindowRect`（window 座標系全体）— flag と raster の
  対応を一意にするため、raster がどの rect を覆うかを
  `PixelCapture` ではなく呼び出し側が flag から導けるようにする
  （`PixelCapture` に rect 情報は足さない — variant の識別は
  呼び出し側が step row の `flags`/`path` フィールドに記録する。
  backend 文字列は `win32-printwindow` のまま — `&'static str`
  であり flag 値ごとの suffix を型で表現できないため、flags を
  別フィールドで記録する方式を実装時に確定した）。
- 既存 `capture_print_window` は `capture_print_window_flags(
  hwnd, PRINT_WINDOW_CLIENT_FULL)` の wrapper として維持 —
  `real-dcomp-pixel` の挙動・backend 文字列
  （`win32-printwindow`）を変えない。flags variant でも backend
  は同一文字列を維持し、variant の区別は step row の `flags`
  フィールドが担う（実装時確定 — 初版は backend suffix を
  想定していたが `&'static str` 制約で flags 別フィールド方式に
  変更）。
- 非 Windows stub にも同じ signature を追加（`Err`）。

### `src/real_dock.rs`

新 case `real-dcomp-present` — `real-dcomp-foreground` の直後
（catalog index 6）。probe 本体は foreground probe と同じ骨格:

```
observe_editor_presentation_probe(container, sessions, session_id,
                                   host, handle, artifacts, steps)
  → attach contract checks（check_attached_children, 共有）
  → presentation_probe_children(&observation, container, artifacts, steps)
      pres_inventory step（ospres::inventory を 1 度だけ —
        child rect 交差 window を z-order で抽出・own_process/pid
        注記。per-child ではなく probe 全体で共有する snapshot）
      per child:
        osoccl::probe(child) → root/parent/hits/child_rect/chain
        pres_attributes step（child/container/root + cover 候補
          cap 16 に ospres::attributes）
        pres_cover step（child rect と交差する top-level window
          の z-order inventory + covers_child/pid/exstyle 注記）
        pres_capture step（{child, container, root} ×
          flags {0, 0x1, 0x2, 0x3} の PrintWindow + 各 target の
          window DC → green stats、opt-in PNG artifact）
        pres_screen step（child rect の screen BitBlt —
          real-dcomp-pixel と同じ oracle、fingerprint の入力）
        pres_fingerprint step（screen capture vs suspect raster /
          root・container raster の overlap 量子化一致率）
  → conclude_editor_session（共有 tail）
```

`PresentationProbeOutcome { evaluated, completed, families,
failure }`:

- `evaluated` — 前提成立（container token・presented・children
  非空・root 解決可）。`false` → `NotRun`。
- `completed` — 計画した全 observation family が top-level call
  を完了したか。個別 API 失敗は step row の `error` として
  記録され family は完了扱い。family-level の早期 return
  （例: inventory が `supported:false` を返した — 非 Windows
  stub でのみ起こり得る防御経路）のみ `false` → `Fail`。
- verdict: `lifecycle_ok && evaluated && completed` → `Pass`。
  green・hit-test・fingerprint の値は verdict 入力にしない。

fingerprint の比較規約:

- screen 側: `ospix::window_screen_rect(child)` +
  `capture_screen_rect`（child rect 全体、`real-dcomp-pixel` と
  同一経路）。
- suspect 側: `WindowFromPoint` hit が解決した HWND（dedup）+
  inventory の交差 window 上位 `FINGERPRINT_CAP = 8`。
  各 suspect に `window_screen_rect` +
  `capture_print_window_flags(hwnd, 0)`（window 座標系全体）。
  raster dims が suspect rect と一致しない場合は `size_mismatch`
  として比較を skip（捏造しない）。
- root/container 側: `capture_print_window_flags(target, 0)` の
  raster と child rect の overlap で同じ比較を行う
  （`match_root`/`match_container`）。
- 色比較: RGB を `>>3` で 5bit/channel 量子化（32³=32768 bin）
  し、overlap 画素毎に bin 一致率 `match_fraction` を記録。
  suspect raster が overlap を部分的にしか覆わない場合は
  overlap 領域のみで正規化する。
- screen 領域の上位 3 色 bin（pixel 数付き）も記録 — 人間が
  「screen に何が出ているか」を即座に読めるようにする
  note 級データ。

capture variant 行列:

- 対象: child・container・root の 3 HWND（container/root は
  `osoccl::probe` の `parent`/`root` から取得、欠落時はその
  target の行を `skipped` で記録）。
- flags: `0x0`（window 全体）/ `0x1`（`PW_CLIENTONLY`）/
  `0x2`（`PW_RENDERFULLCONTENT`）/ `0x3`（両方 — 既存経路と同値）。
- 各 target に `capture_window_dc` も 1 本ずつ（child は既存
  `real-dcomp-pixel` と同じ診断、container/root は surface 保持の
  確認）。
- 全 capture は `note` verdict の step row — green stats +
  opt-in artifact。verdict 入力にしない。

### 変異候補の評価（不採用の記録）

| 候補 | 評価 | 理由 |
| --- | --- | --- |
| `RedrawWindow`/`UpdateWindow` で child の再描画を強制 | 不採用 | plugin の paint 経路は `IPlugView`/timer/resize event 依存で、強制 invalidate が child surface の再 paint を保証しない。効果が観測不能なら変異の意味が無い。 |
| `DwmRegisterThumbnail` で child の DWM-level raster を取得 | 不採用 | destination HWND が必要で、それ自体が新規 window 変異になる。child が DWM に登録されているかの判定も別途要る — 観測 slice の範囲を超える。 |
| Magnification API / Windows.Graphics.Capture | 不採用 | session/window 作成を伴う重い基盤で、既存依存に無い。capture 盲点候補 (c) の判定は `PrintWindow(PW_RENDERFULLCONTENT)` variant で代替可能。 |
| `osmut` 既存 mutation の再利用 | 不採用 | foreground slice で z-order/foreground を動かしても screen green は戻らなかった — 同じ変異を繰り返しても新情報が無い。 |
| DComp visual tree 走査 | deferred | `IDCompositionDevice`/`IDCompositionTarget` の visual 列挙は COM 経路が必要で公開面に無い。cover inventory + fingerprint + `PW_RENDERFULLCONTENT` で HWND/plane の切り分けを先に行う。 |

## Data and Control Flow

probe は child 毎に直列実行（`observation.children` の順）。
各 family は独立に step row を push し、失敗は `error`/`skipped`
フィールドに縮退する。`PresentationProbeOutcome` は
`families` に実行した family 名を記録し、`completed` は
計画 family 数との一致で判定する。

`observed` extras は `child_extras` + `presentation_probe`
summary（`evaluated`/`completed`/`families`/`failure`）。

## Non-Functional Constraints

- `unsafe` は FFI wrapper のみ、SAFETY コメント必須。
- 新規 crate/feature/dependency なし。`dwmapi` は local extern
  link（`PrintWindow` 先例）。
- `windows-sys` features は既存の `Win32_Foundation` /
  `Win32_Graphics_Gdi` / `Win32_System_Threading` /
  `Win32_UI_WindowsAndMessaging` の範囲内 — `EnumWindows`/
  `GetWindowRgn`/`GetRgnBox`/`GetUpdateRect`/
  `GetLayeredWindowAttributes`/`CreateRectRgn`/`GetCurrentProcessId`
  は全て既存 feature で到達可能（実装時に確認）。
- probe は read-only — window の作成・破棄・移動・style 変更・
  message 送信を一切行わない。`PrintWindow` は `WM_PRINT` を
  内部送信し得るが `ospix` の確立済み read-path 契約の範囲内
  （対象は自 window + cover 候補の診断 raster 取得）。
- RT contract: non-RT only。

## Risks and Trade-offs

- `PrintWindow` が foreign window（Chrome/Zed 等）で refuse/空
  raster を返す可能性 — `error`/`size_mismatch` として記録し、
  fingerprint はその suspect を skip するだけで probe は継続。
- `EnumWindows` は同一 desktop の top-level のみ — 別 desktop/
  secure desktop の cover は原理的に見えない。`inventory` の
  `total`/`truncated` で範囲を正直に記録し、解釈側の制限として
  文書化する。
- `PrintWindow` raster と screen の座標対応は window rect 基準の
  近似 — DPI/非 client 領域の差で `size_mismatch` が出る場合が
  ある。その場合は比較を skip して記録のみ残す（誤った一致率を
  出さない）。
- fingerprint の量子化一致率は「同じ色」しか見ない — cover が
  半透明/変化する内容（動画・アニメーション）だと一致率が
  不安定になる。上位色 bin 記録と併せて解釈側の注意点とする。
- probe 実行時間: child 毎に ~20-30 capture + ~30 window の
  attribute query — sub-second 級、bounded。

## Deferred Design Concerns

- DComp visual tree / `IDXGISwapChain` の直接観測 — 次 slice
  の最有力候補として evidence に記録。
- `PW_RENDERFULLCONTENT` が DComp content を実際に捕捉するかは
  実装依存 — flag variant の結果がそのまま証拠になる。
- cover inventory の child window 層（foreign top-level の子孫
  HWND、例 `Chrome_RenderWidgetHostHWND`）は top-level
  inventory に現れない — hit-test が返す HWND は fingerprint
  suspect に直接含める設計でカバーする。foreign window の
  子孫を網羅的に列挙するかは evidence を見て判断。

## Verification Notes

- `cargo fmt --all -- --check`、`mise run check/lint/test
  ruviform-plugin-dock-test`（default + `--features real-dock`）、
  `mise run test-ui-plugin-dock`、`test-ui-plugin-dock-console`、
  `test-ui-scenario-contract`、`check-ui-driver-boundary`、
  `prek run --all-files`。
- `plugin-dock-real-scenario --blocked-probe` で 15 case 全て
  `not_run` + `environment_blocked`。
- interactive lane baseline + `GPUI_DISABLE_DIRECT_COMPOSITION=1`
  env-on の 2 本を実走し JSONL evidence を記録。
