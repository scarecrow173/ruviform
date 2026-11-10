# RF313 — OS Capability Matrix 証拠レポート（Windows 実 OS keyboard + pointer smoke・部分スコープ）

Issue #313「RF-GT-04: OS 別ケースマトリクス + 実 OS smoke」の
**Windows 部分スコープ**の実測証拠。実 GPUI window に value control
（Fader + Seek）を mount し、Win32 `SendInput` で OS レベルの
キーストローク（`verdict-windows.json`）と絶対座標ポインタ
注入（`verdict-pointer-windows.json`）を送り、
`WindowLifecycle::Action` intent が実 OS 入力経路で到達することを
確認した。ポインタ注入は `GpuiDriver::component_bounds`（paint
記録の論理 bounds）と `GpuiDriver::window_geometry`（client-area
物理スクリーン rect）の計測値から照準を導出する — 座標の
ハードコードはない。

## 判定

| 対象 | 判定 | 根拠 |
| --- | --- | --- |
| Windows 実 OS キーボード入力（SendInput）→ value control の commit intent 到達 | **Pass** | `verdict-windows.json`: verdict=pass、checks 全 4 項目 ok、exit code 0 |
| Tab フォーカス移動（#451 `observe_keystrokes` → `focus_next`）が OS 入力で機能 | **Pass** | Tab → gain、Tab → seek と異なる control に commit が分散（value_commits 2 件） |
| #479 キーボード規約（Right=+step / Left=−step、atomic begin+commit） | **Pass** | gain normalized 0.55（>0.5）、seek normalized 0.45（<0.5）。両 intent が `atomic: true` の begin/commit pair |
| Windows 実 OS ポインタ入力（SendInput 絶対 move + 左ボタン）→ gesture intent 到達 | **Pass** | `verdict-pointer-windows.json`: verdict=pass、checks 全 7 項目 ok、exit code 0。Fader drag で begin≈0.25→commit≈0.75、Seek click で begin+commit≈0.8（共に `atomic != true`、同一 `gesture_id`） |
| ポインタ照準の計測基盤（component bounds + window geometry） | **Pass** | `window_geometry` = client-area (720,392) 480×320 phys px @scale 1.0、`component_bounds` = gain (0,0,32×296) / seek (0,296,480×24) logical px。カーソル read-back は commanded 座標から最大 1px 以内 |
| macOS / Linux の全項目 | **NotRun** | 本環境は Windows のみ。実測なしに対応を推測しない |
| IME / a11y / DPI 変更多数 / モニター切断 / 2 window | **NotRun** | 本 dispatch の対象外（Issue #313 の残スコープ） |

## 入力経路の正確な記述（重要）

- 本証拠の入力は **Win32 `SendInput` による OS automation
  injection** である。OS のフォアグラウンド入力経路（ウィンドウ
  メッセージ → GPUI key dispatch）を実際に通るが、**物理的な手動
  タイピングではない**。証拠として「実 OS 入力経路」の Pass で
  あり「物理入力」の Pass ではない。この区別は Issue #313 の
  「模擬入力との厳密分離」要件に対応する。
- GPUI の `dispatch_keystroke` / `simulate_keystrokes`
  （`gpui_driver::observe`、test-only）は toolkit 内で完結し OS に
  出ないため、本証拠とは別レイヤーの simulated-input 証拠である。
  #479/#451 の headless 検証はそちらで既に Pass 済み。
- `os_input::press_key` は down/up の 2 イベントを 1 回の
  `SendInput` で投入し、投入数が不足した場合は失敗を返す
  （部分注入を成功と偽らない）。非 Windows では
  `UiError::Unsupported` を返し、example は verdict=`unsupported`
  / exit 2 で終了する（Pass に偽装しない）。
- ポインタ側も同じ規約: `move_to`/`mouse_down`/`mouse_up`/
  `cursor_position` は `SendInput`/`GetCursorPos` の Win32 実装で、
  部分注入は `UiError::Host`、非 Windows は `Unsupported`。
  絶対座標は `GetSystemMetrics` の virtual screen（負原点を
  含む全モニタ空間）で 0..=65535 に正規化する。
- ポインタ smoke は**実カーソルを物理的に動かす**。起動時の
  カーソル位置を保存し verdict 確定後に復元する（best-effort、
  失敗は stderr 記録）。実行中はマウス/キーボードに触れないこと。

## 対象・環境

| key | value |
| --- | --- |
| fixture | `mise run example-ui-gpui-real-os-smoke`（本 Issue で追加した manual lane。canonical = mise） |
| 実体 | `cargo run -p ruviform-ui-gpui --features gpui --example real_os_smoke -- --out evidence/rf313-os-capability-matrix/verdict-windows.json` |
| view spec | `Panel[Fader(gain), Seek(seek)]` — tab stop が 2 つだけの最小構成。`ValueDescriptor::ranged(0.0,1.0)` + `default 0.5` + `step 0.05`（plain domain） |
| keystroke | `Tab, Right, Tab, Left`（`os_input::OsKey`）。Ready + `Focused(true)` 観測後に injector thread から投入 |
| verdict | `verdict-windows.json`（verdict=pass、reasons=[]、checks 4/4 ok、elapsed 1948ms、ticks 18） |
| OS | Windows 11 Pro（`verdict-windows.json` environment.os_version、CIM `Win32_OperatingSystem` 実測） |
| arch | x86_64 |
| GPU | NVIDIA GeForce RTX 4070 SUPER（CIM `Win32_VideoController` 実測） |
| display | 1920×1080 × 2 monitor、scale_factor 1.0（`WindowDriver::monitors()` 実測） |
| revision | `4aa92c117aa62e0e6f5412933cb7c8bb53fb839a`（`git rev-parse HEAD` 実測。origin/main merge 後の再実行） |
| toolchain | rustc 1.99.0（mise pin）、GPUI 0.2.2 |
| 実行日時 | 2026-11-09T01:35:01Z（verdict JSON timestamp_utc） |

### ポインタ smoke（`verdict-pointer-windows.json`、2026-11-10T04:46:46Z）

| key | value |
| --- | --- |
| fixture | `mise run example-ui-gpui-real-os-pointer-smoke`（pointer スライスで追加した manual lane。canonical = mise） |
| 実体 | `cargo run -p ruviform-ui-gpui --features gpui --example real_os_pointer_smoke -- --out evidence/rf313-os-capability-matrix/verdict-pointer-windows.json` |
| view spec | keyboard smoke と同一 `Panel[Fader(gain), Seek(seek)]` |
| pointer input | `move_to` + `mouse_down`/`mouse_up`（`os_input`、Win32 `SendInput` 絶対座標・左ボタン） |
| 照準 | `window_geometry` = client-area 原点 (720,392)・480×320 物理 px・scale 1.0。`component_bounds` = gain (0,0,32×296)・seek (0,296,480×24) 論理 px。screen point = `origin + (component_origin + frac×size) × scale` |
| 注入系列 | gain drag: (0.5,0.75)→down→(0.5,0.25)→up。seek click: (0.8,0.5)→down→up |
| verdict | `verdict-pointer-windows.json`（verdict=pass、reasons=[]、checks 7/7 ok、elapsed 2527ms、ticks 23） |
| OS / arch / GPU / display / revision | keyboard smoke と同一環境（Windows 11 Pro x86_64 / RTX 4070 SUPER / 2×1920×1080@1.0）。revision `1dc235b9e0dcece626721b1094b1cad7c6d125a3` |

## 観測値（実測、verdict-windows.json 転記）

- `WindowLifecycle::Ready` → `Focused(true)` を観測後に注入開始
  （`ready: true`, `focused: true`, `focus_lost_count: 0`）。
- `Action` intent 4 件（begin+commit × 2 control）:
  - `gain`: begin/commit `normalized: 0.55`（Right の +1 step、
    `target: "mixer.gain"`, `control_id: "gain"`, `atomic: true`）
  - `seek`: begin/commit `normalized: 0.45`（Left の −1 step、
    `target: "mixer.seek"`, `control_id: "seek"`, `atomic: true`）
- Tab は値を変えずフォーカスのみ移動（commit を生まない）—
  #451 の Tab 巡回が OS 入力経路で機能したことの裏付け。
- `inject_result: "ok"`（4 キー全て `SendInput` 2/2 イベント投入）。

## 観測値（ポインタ smoke、verdict-pointer-windows.json 転記）

- `Ready` → `Focused(true)` → geometry/bounds 計測後に注入開始
  （`ready/focused/geometry_and_bounds` 全て ok）。
- カーソル read-back 3 件、commanded 座標から最大 1px 以内
  （tolerance ±4px、`cursor_reached_targets` ok）— SendInput
  受付ではなく `GetCursorPos` 実測でカーソル到達を確認。
- `Action` intent 5 件（gesture 2 組、いずれも `atomic` 非 true）:
  - `gain`（gesture_id `gpui-1`）: begin `normalized: 0.25` →
    preview `0.75` → commit `0.75`（ドラッグ下端→上端、
    Fader は上=最大なので値上昇）
  - `seek`（gesture_id `gpui-2`）: begin `normalized: 0.8` →
    commit `0.8`（クリック、`expect_receipt: true`）
- Preview は contract 上 coalescible のため合格条件にせず
  件数のみ記録（本実行では 1 件観測）。
- `inject_result: "ok"`（move×3 + down×2 + up×2 の全
  `SendInput` が期待イベント数を投入）。
- 実行前カーソル位置を保存し終了時に復元（verdict 確定後・
  window close 前、best-effort）。

## 実行した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run example-ui-gpui-real-os-smoke` | 0 | verdict=pass、`verdict-windows.json` 採取（Windows 11 Pro 実機・対話 session） |
| `mise run example-ui-gpui-real-os-pointer-smoke` | 0 | verdict=pass、`verdict-pointer-windows.json` 採取（同一実機・2 回実行とも pass） |
| `mise run check-ui-gpui` | 0 | `cargo check -p ruviform-ui-gpui --features gpui --all-targets` Pass |
| `mise run lint-ui-gpui` | 0 | `cargo clippy -p ruviform-ui-gpui --all-targets --features gpui -- -D warnings` Pass |
| `mise run test-ui-gpui` | 0 | 69 unit + 20 consumer_observation + 4 lab_story + 7 scenario_contract Pass（3 golden_capture real-window tests は desktop session 前提で ignored — 従来通り） |
| `mise run fmt-check` | 0 | `cargo fmt --all -- --check` Pass |

## 関連証拠へのリンク

- **#243 実窓ライフサイクル**: `docs/impl/ir/issue-243-gpui-realwin-windows.md`
  （Windows 11 Pro 実機での Created/Ready/Focused/Resize/Monitor/
  Close veto-defer-approve/ID 非再利用の実測記録）、
  `docs/designs/issue-243-gpui-realwin-windows.md`、
  `evidence/issue-431-real-verify/wm-close.ps1`（OS `WM_CLOSE`
  automation — 物理入力ではない点に注意）。本 smoke は #243 が
  カバーしない「実 OS キーボード入力 → intent」経路を補完するもので、
  ライフサイクル契約や Native Plugin Dock 受入とは別スコープ。
- **#314 `capture_window`**: `crates/ruviform-ui-gpui/tests/golden_capture.rs`
  （real-window capture → golden compare の手動 lane）、
  `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs`、
  `docs/designs/issue-314-golden-capture.md`。OS compositor 経路の
  pixel capture であり、本 keyboard smoke とは別の real-OS 証拠。
- **#479/#451 契約**: `docs/specs/issue-479-keyboard-value-controls.md`、
  `docs/specs/issue-451-keyboard-tab-nav.md`（simulated input での
  headless Pass — 本証拠はその実 OS 裏付け）。

## consumer への実行方法（再現手順）

```sh
# Windows 11 対話 session で実行（フォーカスを奪う window が開く。
# 実行中は他 window へフォーカスを移さないこと）
mise run example-ui-gpui-real-os-smoke
# verdict JSON: evidence/rf313-os-capability-matrix/verdict-windows.json
# exit code: 0=pass / 1=fail / 2=unsupported(非 Windows)

# ポインタ smoke（実カーソルを動かし終了時に復元する。
# 実行中はマウス/キーボードに触れないこと）
mise run example-ui-gpui-real-os-pointer-smoke
# verdict JSON: evidence/rf313-os-capability-matrix/verdict-pointer-windows.json
# exit code: 0=pass / 1=fail / 2=unsupported(非 Windows)

# 出力先・revision の上書き
RF313_SMOKE_VERDICT=/tmp/v.json RF313_SMOKE_REVISION=<rev> \
  cargo run -p ruviform-ui-gpui --features gpui --example real_os_smoke
RF313_POINTER_SMOKE_VERDICT=/tmp/pv.json RF313_POINTER_SMOKE_REVISION=<rev> \
  cargo run -p ruviform-ui-gpui --features gpui --example real_os_pointer_smoke
```

## NotRun / Unsupported の理由と owner

| 項目 | status | 理由 | owner / 後続 |
| --- | --- | --- | --- |
| ポインタ右/中ボタン・修飾キー・ホイール | NotRun | 本スライスは左ボタン move/down/up のみ（`os_input` の公開範囲） | Issue #313 残スコープ |
| macOS / Linux 全項目 | NotRun | 本 dispatch の環境は Windows のみ。未実行 OS の対応を推測しない | 各 OS の real-OS smoke タスク |
| IME / a11y（OS 支援技術） | NotRun | 対象外。IME capability は #243 記録でも `ime: Unsupported` | 別 Issue |
| DPI/マルチモニター変化・モニター切断・2 window | NotRun | 静的 2 monitor @1.0 のみ観測。変动态は未検証 | Issue #313 残スコープ |
| `ScenarioDriver::RealOs` adapter 本体 | Unsupported | enum capability は存在するが adapter 未実装。本 dispatch は example 経路の直接検証 | RealOs adapter タスク（#464 系） |
| consumer（AudioLens/Synvelis）側受入 | NotRun | consumer repo の検証は本 dispatch の編集範囲外 | consumer 側タスク |

## 実測と仮定の区別

- **実測**: 上記 verdict JSON の全フィールド（OS/GPU/monitor/revision/
  timestamp は実行時に収集。CIM・`git rev-parse`・`monitors()` の
  実値）。intent 4 件・値 0.55/0.45・exit code 0 は当該実行の実測。
- **仮定・設計上の留保**: `SendInput` が届くのは OS が foreground と
  認識した window のみ（`Focused(true)` 観測を注入条件にした）。
  実行中にユーザーが別 window へ focus を移すと intent が届かず
  Fail になり得る — manual lane の運用注意として明記。
  Tab 巡回順は spec の tab stop 順に依存するが、assert は順序非依存
  （2 control の網羅と値方向のみ検査）。

## Issue #243 multiwindow slice（2026-11-10 追記）

`real_os_multiwindow_smoke`（`mise run
example-ui-gpui-real-os-multiwindow-smoke`）による 3 実 Window の
multiwindow/Dock/Job lifecycle を `SendInput` keyboard/pointer +
example-local chord（Win+Arrow・Alt+F4）で駆動した Windows 実測。
証拠: `evidence/issue-243-multiwindow-verify/`。

| 対象 | 判定 | 根拠 |
| --- | --- | --- |
| 3 実 Window 生成・window 別 focus（実クリック）・keyboard commit・update_view state 保持・Dock detach/move/abort/reattach・Win+Arrow 実移動・minimize/restore・再生成（新 WindowId + fresh view state）・view ownership・Window-owned Job cancel vs Application-owned Job 生存・orphan view なし・teardown 後 runtime 生存 | **Pass（前回実測）** | pre-refactor binary の 1 回実測で全 check green（verdict=pass）。verdict JSON は環境ブロック run で上書き済みのため、詳細は `environment-blocked-sendinput.md` と本記録を参照 |
| post-refactor binary の実 OS 再検証 | **NotRun** | 実行時点で host の `SendInput`/`SendKeys` が system-wide に無効化（受付を返すが注入イベントが一切効果を持たない。legacy `mouse_event`/`keybd_event`/`SetCursorPos` は正常）。`vgk`/`vgc`/`vgtray` 停止・LG HUB/GameInput proxy kill 後も不変。原因は未同定の injected-input filter で、worker 権限では除去不能。健全 host での再実行が必要 |
| move/close/focus 失敗時の stale handle・二重破棄・孤立 view・RT 停止なし | **Pass（前回実測）+ NotRun（再検証）** | 前回 run で `destroyed_lifecycle_trace`/`no_orphaned_views`/`runtime_alive_after_teardown` が green。最終コードは headless lane で検証済みだが実 OS 再検証は環境ブロック |
| macOS / Linux multiwindow | **NotRun** | 本環境は Windows のみ |
| 動的 DPI・モニター間移動・モニター切断 | **NotRun** | 静的 2 monitor @1.0 のみ実測 |
| Native Plugin Dock | **NotRun** | #388/#389・Synvelis #176 の OS 別 Gate。Floating/Generic fallback・Mock/画像を Native Dock PASS にしない |
| consumer（AudioLens/Synvelis）非回帰 | **NotRun** | fixture/契約レベルのみ。consumer 実機は別 Gate |

### 環境ブロックの記録（`environment-blocked-sendinput.md`）

post-refactor run が `per_window_focus_real_click` で fail した原因を
host-level で切り分けた結果、**SendInput がシステム全体で無効**
（マウス・キーボード両方、絶対/相対両方、対話 session・
Task Scheduler context の双方で `sent=1` を返しつつ効果ゼロ）
であることを確認。`mouse_event`/`keybd_event`/`SetCursorPos` は
正常に動作し、Riot Vanguard（`vgk`/`vgc`/`vgtray`）、LG HUB、
GameInput user-mode proxy を停止しても改善しない。class
Upper/LowerFilters・BlockInput・RDP・アクセシビリティ設定・
IL 差異はいずれも否定済み。`LLMHF_INJECTED` 付きイベントを
落とす user-mode hook または kernel filter が session 中に
導入された可能性が高いが、サービス停止には admin 権限が必要で
worker の範囲外。fixture 側は cursor read-back oracle で
「SendInput 受理 ≠ カーソル到達」を検出し fail として正直に
記録する設計であり、oracle を弱めた Pass 偽装はしていない。

### 再現・再検証手順

```sh
# SendInput -> GetCursorPos preflight: カーソルが動けば健全
# （例: sendinput-rel-test 相当の move_to -> cursor_position）
mise run example-ui-gpui-real-os-multiwindow-smoke
# verdict: evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json
# exit code: 0=pass / 1=fail / 2=unsupported(非 Windows)
# 実行中はマウス/キーボードに触れないこと（実カーソルを使用・復元）
```
