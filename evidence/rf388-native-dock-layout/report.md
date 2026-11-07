# RF388 — GPUI Panel/Tab レイアウト内 VST3 Editor 埋め込み（Windows / L0）証拠レポート

本レポートは Issue #388 の Native Dock Gate のうち、**Windows / VST3 / L0** の
「GPUI Panel/Tab レイアウト内への実 plugin editor 埋め込み」を実機で確認した結果を記録する。
**Dock PASS は付与しない。** 本 slice で確認できたのは次の範囲に限られる（後述の NotRun / Unsupported を参照）。

## 判定

| 対象 | 判定 | 根拠 |
| --- | --- | --- |
| Windows / VST3 / L0 の GPUI Panel/Tab 内 attach（同一 window の layout 矩形への追従） | **Pass** | `check=attach_parented`・`check=attach_visible`・`projected_rect` |
| tab 切替による hide / 再 show（slot 状態と editor の OS 可視性） | **Pass** | `check=tab_hide`・`check=tab_show`、ピクセル採取（後述） |
| window resize に対する slot / container 追従 | **Pass（grow / shrink とも）** | `check=resize_grow_follows`・`check=resize_shrink_follows` |
| focus（container への `SetFocus`） | **Pass（container 単位のみ）** | `check=focus_container`。editor 内部の key/IME 配送は未検証 |
| close → editor 破棄 → lease / slot 解放 | **Pass** | `check=close_session`・`check=release_lease`・`check=release_slot` |
| **Native Dock PASS（完了条件全体）** | **不成立（Partial）** | 下記 NotRun/Unsupported の項目が完了条件に含まれるため |

Dock PASS を付与しない理由: Issue #388 の Dock PASS 定義は、GPUI Main Window 内の Dock 矩形と同じ
clip / 重なり / タブ可視性 / resize / 入力 / focus の管理下で plugin の実 editor が動作することを要求する。
本 slice は上記のうち tab 可視性・resize 追従・focus（container 単位）・lifecycle を確認したが、
clip（ancestor clip の overhang）・キーボード/IME/popup 入力・re-dock/move/float・plugin 側 resize 受理・
DPI/複数モニタ・複数 window・crash/hang 隔離は未実施のため、Dock PASS とは言えない。

## 対象 plugin / 環境

| key | value |
| --- | --- |
| plugin | `C:/Program Files/Common Files/VST3/iZotope/Vinyl.vst3` |
| size / plugin | 2,045,952 bytes / Vinyl 1.12.0（1 class） |
| host | `ruviform-poc-native-dock` debug build（`--features gpui,vst3`）/ Windows 11 x64 / GPUI 0.2.2 |
| 実行コマンド | `RVF_POC_SCENARIO=dock RVF_POC_EVIDENCE_DIR=<dir> RVF_POC_VST3_PATH='C:/Program Files/Common Files/VST3/iZotope/Vinyl.vst3' ./target/debug/ruviform-poc-native-dock.exe`（mise: `mise run poc-native-dock-layout`） |
| exit code | 0（`summary failures=0`） |
| 生ログ | `evidence/rf388-native-dock-layout/dock-vinyl-win11.log` |
| 画面キャプチャ | `dock-attached.png` / `dock-tab-hidden.png` / `dock-tab-shown.png` / `dock-resize-grow.png` / `dock-resize-shrink.png`（`RVF_POC_EVIDENCE_DIR` 指定時のみ採取） |
| ピクセル採取 | `pixel-sample.ps1` → `pixel-sample.txt` |

## 観測値（実測）

| check id | 結果 | 観測値（抜粋） |
| --- | --- | --- |
| bind_native_slot | pass | `slot_index=0`、layout key `dock.editor` を slot へ bind |
| projection（renderer による geometry/visibility 投影） | pass | `projected_rect=(1.0, 38.0, 998.0, 681.0) visibility=Visible` |
| attach_parented | pass | editor HWND は container HWND の子（`parent_of == container`） |
| attach_visible | pass | `editor_rect=(461, 230, 553, 400)`、`IsWindowVisible=true` |
| update_view（tab → Other） | pass | `Ok(())` |
| tab_hide | pass | `slot_visibility=Hidden editor_visible=false` |
| update_view（tab → Editor） | pass | `Ok(())` |
| tab_show | pass | `slot_visibility=Visible editor_visible=true` |
| focus_container | pass | `focus_hwnd == container`（`0x…` 同値） |
| resize_grow_follows | pass | window 1280x900 → slot `(1,38,1278,861)`、container `(461,230,1278,861)`、editor 不変 `553x400` |
| resize_shrink_follows | pass | window 520x560 → slot `(1,38,518,521)`、container `(461,230,518,521)`、editor 不変 `553x400` |
| close_session | pass | `state=Some(Closed) editor_alive=false` |
| unbind_native_slot / release_lease / release_slot | pass | `container_alive=false` |
| slot_event_order | pass | `lease_revoked=Some(8) destroyed=Some(9)`（`LeaseRevoked` が `Destroyed` より前） |

### ピクセル確認（`pixel-sample.txt`）

editor 領域（PNG 座標 `(20..60, 100..140)`、editor 原点は window 原点 `(452,161)` からの相対 `(9,69)`）の平均 RGB:

| 画面 | 平均 RGB | 解釈 |
| --- | --- | --- |
| attached | `38,45,54` | Vinyl editor 表示 |
| tab-hidden | `20,24,29` | GPUI 背景（editor 非表示） |
| tab-shown | `38,45,54` | Vinyl editor 再表示 |
| resize-grow | `38,45,54` | 拡大後も editor 表示 |
| resize-shrink | `38,45,54` | 縮小後も editor 表示 |

各キャプチャは `foreground_is_window=true`（GPUI window が前面）で採取した。
起動元シェルがフォーカスを持つ実行では全キャプチャが `foreground_is_window=false`（他 window による遮蔽）となったため、
Ready 時に `WindowDriver::focus(window, true)` で window を activate する処理を追加し（`activate_window=true`）、
遮蔽のない採取のみを採用した。遮蔽された採取（`foreground_is_window=false`）はピクセル証拠として使わない。

## 独立レビュー（review-protocol）の対応

- 独立した 2 名のレビュー（協調的・敵対的、読み取り専用）と検証を実施。
- 採用・修正した指摘: スロット仕様が取れない場合に無判定で成功終了していた経路（`projection` を fail に）、
  focus 判定が container 0 と一致しうる経路（container≠0 を必須化）、`switch_tab` のコメントと実挙動の不一致
  （次段への遷移を戻り値で明示）、resize が container サイズを検証していなかった点（物理サイズ一致を必須化）、
  close が `Closed` を検証していなかった点（`state == Closed` を必須化）、slot event 順序が未検証だった点
  （`slot_event_order` を追加）、`update_spec` が host event を drain していなかった点（`flush_events` に統一）。
- 採用しなかった指摘と理由: 「ログファイルが存在しない」は読み取り時点の古い worktree に基づく誤指摘で、
  `dock-vinyl-win11.log` は実在する（本レポートの値はこのファイルから転記）。
- 既知の未修正（非ブロッキング・記録のみ）: `update_view` が拒否理由を呼び出し元へ返さない（result channel 未実装）、
  view を持たない window への `update_view` は Ok を返し無視する（doc に明記）、open 失敗時に lease/container の
  teardown を行わない（harness の失敗経路。exit 1 で終了）、`Done` 段が tick ごとに summary を再出力する、
  pixel 採取の座標は実行 1 回分に固定（補助証拠のみ）。

## 発見事項（実装上の注意）

1. **既存の native-dock PoC（floating 領域 probe）は `GpuiSurfaceAdapter::new` で独立 ledger を作っていた。**
   renderer の geometry/visibility 投影は driver の canonical surface（`GpuiDriver::surface()`）にしか届かないため、
   Dock layout scenario は canonical surface を使う。floating probe 側は従来どおり（変更なし）。
2. **tab 切替は production に入口がなかった。** `GpuiDriver::update_view` を追加し、既存の `ViewHost::update`
   経路（renderer の inactive-tab 抑止）へ接続した。拒否された spec は前の木を残す（`ViewHost::update` 契約）が、
   その拒否は driver 呼び出し元へ返らない（現状の制限。結果は slot 状態で観測する）。
3. **editor 幅 553px は container（slot）より狭い縮小でも clip を作れなかった。** slot は window の端まで伸び、
   editor の overhang は window の外（frame 領域）に出るため、画面上で「container による clip」は観測できない。
   ピクセル採取でも overhang 帯は frame 色（209,209,209）であり editor 色ではないため、clip の Pass 根拠にはしない。
   → clip は NotRun（後述）。
4. **plugin 側 resize は受理されていない。** container は host 側で追従したが、editor の矩形は全 resize で
   不変（553x400）。VST3 host bridge に host→plugin の `IPlugView::onSize` / `checkSizeConstraint` 経路はなく、
   plugin 主導の `resizeView` は既存どおり `kResultFalse`（defined refusal）。

## 未達 / 正直な分類

| 項目 | 状態 | 理由 |
| --- | --- | --- |
| ancestor clip による editor の overhang 切り取り | **NotRun** | slot が window 端まで伸びるため container 外の overhang が画面に出ない。clip は pixel 証拠がない |
| plugin 側 resize 受理（host `onSize` / plugin `resizeView`） | **Unsupported** | VST3 bridge に host→plugin resize 経路なし。`resizeView` は defined refusal |
| keyboard / IME / popup 入力の editor 配送 | **NotRun** | container への focus のみ確認。editor 内部への key/IME 配送は未実装・未検証 |
| pointer 入力（tab クリック）による実操作 | **NotRun** | tab 切替は `update_view` によるホスト駆動。pointer 操作の自動化は未実施 |
| re-dock / move to other dock / float / detach-reattach | **NotRun** | 単一 window 内の tab 切替のみ。`DockWorkspace` 2-phase move との接続は未実施（#182 経路） |
| DPI 変更 / 複数モニタ | **NotRun** | 実行環境は単一 scale。切替未実施 |
| 2 window（同 editor の複数 window 所有） | **NotRun** | 単一 window のみ |
| crash / hang 隔離（GUI 失敗が DSP/RT/Project に波及しないこと） | **NotRun** | 障害注入なし。本 slice は in-process L0 のため隔離の証拠にならない |
| GPUI swapchain 遮蔽（z-order の重なり） | **NotRun（pixel 証拠のみ）** | visible 状態の合成は採取したが、遮蔽の系統的検証はしていない |
| macOS / Linux（X11 / XWayland / Wayland）・CLAP・AU | **Unsupported / NotRun** | 本 slice は Windows / VST3 / L0 のみ。CLAP/AU は未実装、Wayland は規格上 Unsupported |
| L1（別プロセス）editor 埋め込み | **Unsupported** | 現行 L1 経路は GUI を `Unsupported` で拒否（既存 e2e で pin）。L0 への黙示的退化はしていない |
| 他社 VST3 の広範互換 | **NotRun** | Vinyl 1 バイナリのみ |

## 受入 Gate 対応（PR 本文用）

- 「Dock PASS の可否」: **不成立（Partial）**。上表の Pass は Windows / VST3 / L0 の部分的な行に限る。
- 「Floating window・Generic editor・top-level attach・fake surface・screenshot」は Dock PASS の根拠にしていない。
  本 slice の Pass 行は GPUI Panel/Tab layout の矩形へ slot が投影された container に、実 VST3 editor の
  子 HWND が実際に parent されていること（`GetParent`）と OS 可視性で確認した。
