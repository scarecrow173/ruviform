# RF388 slice 3 — Windows/VST3/L0 windowing 実機証拠レポート

Issue #388 / RF-GPUI-05 NativeSurfaceSlot slice 3。ancestor clip・keyboard/focus
routing・DockWorkspace 2-phase re-dock を実機で確認した。**Dock PASS は付与しない。**
さらに本 slice で **GPUI DirectComposition が child HWND を画面に合成しない**という
プラットフォーム制約を実測で特定した（後述「重大な発見」）。

## 判定

| 対象 | 判定 | 根拠 |
| --- | --- | --- |
| ancestor clip / overhang（layout 駆動: window を slot 最小未満へ resize） | **Pass** | `clip_overhang_region`（期待 region と `GetWindowRgn` 完全一致）・`clip_overhang_editor_alive` |
| direct clip（`SharedSurface::set_clip` → `SetWindowRgn`） | **Pass** | `clip_direct_region`（`(0,0,499,681)` 完全一致）・`clip_direct_hit_test`（`WindowFromPoint` が clip 内=plugin 子孫、clip 外=GPUI frame を返す）・`clip_direct_cleared` |
| editor の clip 下生存（parented・visible flag・HWND 生存） | **Pass** | `clip_overhang_editor_alive`・`redock_detach_surface_survives` |
| keyboard/focus routing（container ⇄ plugin の境界） | **Pass** | `focus_container`・`key_to_container_not_plugin`・`click_focuses_plugin`・`key_reaches_plugin`・`key_boundary_return`（probe は plugin 側 message counter + `WH_GETMESSAGE` hook、Vinyl は hook + plugin tree 判定）。`click_focuses_plugin` は click が focus を plugin tree に移さなかった場合に `SetFocus` fallback で pass しうる（経路は check の `path=` detail に記録。Vinyl run は `path=click` で fallback 不使用） |
| re-dock / float via 2-phase move（detach→reattach） | **Pass** | `redock_begin_move`・`redock_busy_refuses_lease`・`redock_commit_move`・`redock_move_events`・`redock_detach`・`redock_detach_surface_survives`・`redock_reattach_same_surface`（同一 editor HWND `0x4ef0150` で生存再 attach）。`begin_move`/`commit_move` は同一 window 内の degenerate move（busy/lease handshake の実証。window 間の実 re-dock 証拠は detach_view/attach_view 側） |
| close/unbind/lease/slot 解放・per-slot event 順序 | **Pass** | `close_session`・`unbind_native_slot`・`release_lease`・`release_slot`・`slot_event_order`（同一 slot で `LeaseRevoked`→`Destroyed`） |
| **native surface の画素が画面に表示されること** | **Unsupported（実測）** | GPUI `CreateTargetForHwnd(hwnd, topmost=true)` により child HWND が swapchain に覆われる（後述） |
| **Native Dock PASS（完了条件全体）** | **不成立（Partial）** | 画素表示が成立しないため実用上の Dock 完了とは言えない |

## 対象 plugin / 環境

| key | value |
| --- | --- |
| plugin | `C:/Program Files/Common Files/VST3/iZotope/Vinyl.vst3`（2,045,952 bytes / Vinyl 1.12.0 / 1 class） |
| host | `ruviform-poc-native-dock` debug build（`--features gpui,vst3`）/ Windows 11 x64 / GPUI 0.2.2 |
| 実行コマンド | `RVF_POC_SCENARIO=dock-windowing RVF_POC_VST3_PATH='C:/Program Files/Common Files/VST3/iZotope/Vinyl.vst3' RVF_POC_EVIDENCE_DIR=target/poc-evidence-vst3 ./target/debug/ruviform-poc-native-dock.exe`（mise: `mise run poc-native-dock-windowing`） |
| exit code | 0（`summary failures=0`、check 33 件全 pass） |
| 生ログ | `dockw-vinyl-win11.txt` / check 一覧 `checks-vinyl.jsonl` |
| probe ログ | `dockw-probe-win11.txt`（in-process probe、同一 33 check 全 pass） |
| 画面キャプチャ | `dockw-attached.png` / `dockw-clip-overhang.png` / `dockw-clip-direct.png` / `dockw-detached.png` / `dockw-reattached.png` |

## 観測値（実測・抜粋）

| check id | 結果 | 観測値 |
| --- | --- | --- |
| attach_parented | pass | editor HWND `0x4ef0150` の `GetParent` = container `0x4541072` |
| attach_visible | pass | `editor_rect=(461,230,553,400)`、`IsWindowVisible=true` |
| clip_overhang_region | pass | window 380x300 → slot `(1,38,398,298)`、期待 region `(0,0,379,262)` = `GetWindowRgn` 実測と一致 |
| clip_direct_region | pass | `set_clip` left-half → region `(0,0,499,681)` 完全一致 |
| clip_direct_hit_test | pass | clip 内 `WindowFromPoint`=plugin 子孫 `0x3440198`、clip 外=GPUI frame `0x18707d4` |
| key_to_container_not_plugin | pass | container_got=true plugin_got=false（`WH_GETMESSAGE` hook） |
| key_reaches_plugin | pass | container_got=false plugin_got=true（click で plugin focus 後の key 配送） |
| key_boundary_return | pass | focus を container へ戻すと key は container のみが受信 |
| redock_detach_surface_survives | pass | detach で `slot_visibility=Hidden`・`IsWindowVisible=false` だが editor HWND 生存・parent 維持 |
| redock_reattach_same_surface | pass | reattach 後 `editor_hwnd=0x4ef0150`（同一 HWND）・`parent_ok=true` |
| slot_event_order | pass | 同一 slot で `lease_revoked=Some(14)` < `destroyed=Some(15)` |

## 重大な発見: GPUI DirectComposition が child HWND を覆い隠す（実測）

slice 2（`evidence/rf388-native-dock-layout/`）で「editor 背景 38,45,54」を Vinyl の
画素と解釈したが、本 slice の compositor 診断で **その色は GPUI の `control_track`
（`0x262d36`）であり、plugin の画素ではなかった**ことが判明した。

実測した事実:

- plugin editor（probe・Vinyl 双方）・container（`Static`）ともに `GetClipBox` が
  `NULLREGION` を返し、`GetPixel`（自前 DC）は全点で `CLR_INVALID`。
- `screen_pixel`/`PrintWindow` は editor 領域で常に `0x00362d26`（= GPUI
  `control_track`）。child の surface が画面に出ていないことを示す。
- **対照実験**: container の子として新規 probe-class HWND（`WS_CHILD|WS_VISIBLE`）を
  作成しても同じく非表示 → plugin 固有ではなく **GPUI window 配下の全 child HWND が
  合成されない**。
- 祖先 chain: `RvfProbeEditor → Static(container) → Zed::Window`。GPUI window の
  exstyle `0x00240100` = `WS_EX_NOREDIRECTIONBITMAP | WS_EX_APPWINDOW | WS_EX_WINDOWEDGE`。
- 原因: GPUI 0.2.2 `platform/windows/directx_renderer.rs:793` が
  `IDCompositionDevice::CreateTargetForHwnd(hwnd, /* topmost= */ true)` を呼ぶ。
  MSDN 定義上 `topmost=TRUE` は visual tree を **window の children の上**に描画する
  （`FALSE` なら children の背面）。つまり GPUI の swapchain が全 child HWND を覆う。
- `WS_EX_NOREDIRECTIONBITMAP` 付き window には GDI redirection surface がなく、
  child window の clip/pixel も含め画面合成が DComp 経路に一本化される。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` で非 DComp 経路を試したが scenario 途中で
  プロセスが exit 127 で落ちた（開発中の観測。ログ artifact なし。GPUI 側の別経路の
  問題と推定し、本 slice の範囲外）。

影響: HWND レベルの機構（parentage・geometry・visibility flag・`SetWindowRgn` clip・
`WindowFromPoint` hit-test・focus・key 配送・lifetime・re-dock）は全て実測で正しく動くが、
**plugin editor の画素はユーザーには一切表示されない**。NativeSurfaceSlot の視覚的
目的は現行 GPUI 経路では未達。対処は GPUI 側の変更（`topmost=false`、あるいは
child を被覆しない surface 構成）か adapter 層の設計変更が必要であり、本 slice の
編集範囲（`surface.rs`/`render.rs` slot projection・PoC・証拠）では解決できない。

## capability matrix 更新（本 slice で実測した行のみ）

| 項目 | 判定 | 根拠 |
| --- | --- | --- |
| Windows/VST3/L0 ancestor clip/overhang | **Pass** | `clip_overhang_*`（region 完全一致・editor 生存） |
| Windows/VST3/L0 direct `set_clip` | **Pass** | `clip_direct_region`・`clip_direct_hit_test`・`clip_direct_cleared` |
| Windows/VST3/L0 keyboard/focus routing（key のみ。IME/popup は NotRun） | **Pass** | `focus_container`・`key_*`・`click_focuses_plugin` |
| Windows/VST3/L0 re-dock/detach/reattach 2-phase | **Pass** | `redock_*`（同一 HWND 生存・busy lease 拒否・event 順序） |
| Windows/VST3/L0 native surface 画素表示 | **Unsupported（実測）** | GPUI DComp `topmost=true` が child HWND を覆う。plugin 画素は観測不能 |
| macOS 全行・Linux X11/XWayland・Wayland・AU・CLAP・L1 isolation | **NotRun/Unsupported 維持** | 本 dispatch 対象外（前回 matrix と同じ） |
| plugin 側 resize（`onSize`）・DPI/マルチモニタ・2 window・crash/hang・swapchain z-order | **NotRun 維持** | 本 dispatch 必須外 |

## 検証（実行コマンド・exit・結果）

環境: Windows 11 x64 / Git Bash / Rust toolchain は mise pin（`rust = "1.99.0"`）/ gpui 0.2.2。

| command | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | 整形差分なし |
| `mise run check` | 0 | `cargo check --workspace --all-targets` |
| `mise run lint` | 0 | clippy `-D warnings` |
| `mise run test` | 0 | workspace 全 suite ok |
| `mise run test-ui-gpui` | 0 | gpui feature tests ok（`slot_clip` test 含む） |
| `mise run poc-native-dock-windowing`（Vinyl） | 0 | 33 check 全 pass / `failures=0` |
| `mise run poc-native-dock-windowing`（probe） | 0 | 同上（`dockw-probe-win11.txt`） |
| `mise run poc-native-dock-layout`（Vinyl） | 0 | 既存 lane 回帰なし / `failures=0` |
| `RVF_POC_SCENARIO=dock …`（probe・Vinyl） | 0 | 回帰なし / `failures=0` |

補助証拠（canonical=mise、直接 binary 実行は同等の再現確認）: 上記 PoC binary 直接実行も
probe/Vinyl とも `failures=0`。

## 既知の制限（review 由来・非ブロッキング）

- **外部 `SharedSurface::set_clip` と projection ledger の desync**:
  `AppliedSlot.clip` は `project_slot_frame` 経由の clip のみを記録する。
  consumer が `set_clip` を直接呼ぶと adapter 側 `binding.clip` は更新されるが
  ledger は `None` のまま。次 frame で投影 clip `None` が要求されると dedupe
  で port が再 assert されず、stale region が残りうる。PoC は明示 clear で
  回避済み。単一 writer 前提の契約明確化が今後の課題（`native_slot.rs` doc 未記載）。
- **clip 証拠の範囲**: `clip_overhang_*` の mask は window viewport
  （`content_mask` fallback）であり、中間 ancestor の `overflow:hidden` による
  clip は本 layout では未発生。spec の Out-of-scope（中間 ancestor clip）どおり
  将来課題。
- **`slot_clip` の端例**: `visible != bounds` の f32 比較は非 2 冪 layout 値で
  1 ulp 差を生じ `Some(≈bounds)` を返しうる（clip-capable driver では dedup 済み
  `SetWindowRgn` が 1 回走るのみで無害。clip-Unsupported driver では slot が
  Hidden 化しうる latent edge）。NaN bounds は上流 GPUI が生成しない前提。
- **`redock_scratch_declared` は常に pass の記録 check**（実 assert は直前の
  `Err` early-return）。check 数 33 に tautology 1 件を含む。

## 実施していない検証（NotRun）

- macOS / Linux / Wayland 全行、AU・CLAP、L1 isolation（#389 joint）。
- plugin 主導 resize（`onSize`）、DPI/マルチモニタ、2 window、crash/hang 注入、
  swapchain z-order の追加実測。
- IME・popup の入力 routing（key のみ実測）。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` 経路での child 合成確認（GPUI 側 crash で未達。
  別 Issue 候補）。

## Review protocol

独立した 2 名の reviewer（cooperative / adversarial、いずれも read-only の
別コンテキスト subagent）に `render.rs` の `slot_clip`/`project_slot_frame`、
`poc.rs`/`dock_windowing.rs`、証拠 artifact、doc chain を照合させた。
coordinator が candidate を検証・重複排除し、CONFIRMED P0/P1 のみ修正した。

- **修正済み（CONFIRMED）**:
  - 生ログ `dockw-*.log` が `.gitignore` の `*.log` に合致し commit 対象外
    → `.txt` へ rename（DComp 診断の唯一の根拠 artifact のため必須）。
  - report の未実証 sub-claim を修正（brush 変更実験の記述削除、
    `GPUI_DISABLE_DIRECT_COMPOSITION` exit 127 を「開発中の観測・artifact なし」
    に格下げ、Vinyl lane の plugin counter 記述を hook+tree 判定へ修正）。
  - test-spec R1-2 が不可能な pixel 確認を要求 → `WindowFromPoint` hit-test
    代替へ書き換え、DComp による画素不可の注記を追加。
  - task T1–T7 の checkbox を完了へ更新、`docs/impl/ir/` の実装記録を新規作成
    （test-spec/design の dangling reference を解消）。
- **非ブロッキングとして記録（P2/P3、修正せず報告）**:
  - `click_focuses_plugin` が `SetFocus` fallback で pass しうる構造
    （Vinyl run は `path=click` で実 click 成立。report に開示済み）。
  - `SharedSurface::set_clip` 外部呼出しが projection ledger を desync しうる
    latent 契約 gap（上記「既知の制限」）。
  - `slot_clip` の f32 ulp/NaN 端例（上記「既知の制限」）。
  - `WH_GETMESSAGE` hook は happy path のみ明示 unhook（全 failure path は
    `Stage::Done`→`drv.exit()` で process 終了するため実害なし）。
  - `redock_scratch_declared` の常時 pass 記録 check（上記「既知の制限」）。
- **検証済みで指摘なし**: `content_mask().bounds` の座標系一致（gpui 0.2.2
  `with_content_mask` の prepaint push と canvas prepaint hook の位相）、
  zero-size clip → 空 HRGN → `SetWindowRgn` の `None`/`Some(0)` 区別、
  `directx_renderer.rs:793` の `CreateTargetForHwnd(hwnd, true)` 行番号一致、
  report↔log の 33 check 一致、HWND 生存確認順序、hook の thread 帰属。
- 判定: **NO BLOCKERS**（P0/P1 なし）。
