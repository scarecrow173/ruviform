# RF313 — OS Capability Matrix 証拠レポート（Windows 実 OS キーボード smoke・部分スコープ）

Issue #313「RF-GT-04: OS 別ケースマトリクス + 実 OS smoke」の
**Windows 部分スコープ**の実測証拠。実 GPUI window に value control
（Fader + Seek）を mount し、Win32 `SendInput` で OS レベルの
キーストロークを注入して、`WindowLifecycle::Action` intent が
実 OS 入力経路で到達することを確認した。

## 判定

| 対象 | 判定 | 根拠 |
| --- | --- | --- |
| Windows 実 OS キーボード入力（SendInput）→ value control の commit intent 到達 | **Pass** | `verdict-windows.json`: verdict=pass、checks 全 4 項目 ok、exit code 0 |
| Tab フォーカス移動（#451 `observe_keystrokes` → `focus_next`）が OS 入力で機能 | **Pass** | Tab → gain、Tab → seek と異なる control に commit が分散（value_commits 2 件） |
| #479 キーボード規約（Right=+step / Left=−step、atomic begin+commit） | **Pass** | gain normalized 0.55（>0.5）、seek normalized 0.45（<0.5）。両 intent が `atomic: true` の begin/commit pair |
| ポインタ（マウス）実入力 | **NotRun** | カーソル実占有を伴うため本 dispatch 対象外。owner: 後続 pointer smoke タスク |
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

## 対象・環境

| key | value |
| --- | --- |
| fixture | `mise run example-ui-gpui-real-os-smoke`（本 Issue で追加した manual lane。canonical = mise） |
| 実体 | `cargo run -p ruviform-ui-gpui --features gpui --example real_os_smoke -- --out evidence/rf313-os-capability-matrix/verdict-windows.json` |
| view spec | `Panel[Fader(gain), Seek(seek)]` — tab stop が 2 つだけの最小構成。`ValueDescriptor::ranged(0.0,1.0)` + `default 0.5` + `step 0.05`（plain domain） |
| keystroke | `Tab, Right, Tab, Left`（`os_input::OsKey`）。Ready + `Focused(true)` 観測後に injector thread から投入 |
| verdict | `verdict-windows.json`（verdict=pass、reasons=[]、checks 4/4 ok、elapsed 2011ms、ticks 18） |
| OS | Windows 11 Pro（`verdict-windows.json` environment.os_version、CIM `Win32_OperatingSystem` 実測） |
| arch | x86_64 |
| GPU | NVIDIA GeForce RTX 4070 SUPER（CIM `Win32_VideoController` 実測） |
| display | 1920×1080 × 2 monitor、scale_factor 1.0（`WindowDriver::monitors()` 実測） |
| revision | `9d7e930ab914b7bba831f06162ee03f506e13674`（`git rev-parse HEAD` 実測） |
| toolchain | rustc 1.99.0（mise pin）、GPUI 0.2.2 |
| 実行日時 | 2026-11-09T01:17:27Z（verdict JSON timestamp_utc） |

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

## 実行した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run example-ui-gpui-real-os-smoke` | 0 | verdict=pass、`verdict-windows.json` 採取（Windows 11 Pro 実機・対話 session） |
| `mise run check-ui-gpui` | 0 | `cargo check -p ruviform-ui-gpui --features gpui --all-targets` Pass |
| `mise run lint-ui-gpui` | 0 | `cargo clippy -p ruviform-ui-gpui --all-targets --features gpui -- -D warnings` Pass |
| `mise run test-ui-gpui` | 0 | 65 unit + 20 consumer_observation + 4 lab_story + 7 scenario_contract Pass（3 golden_capture real-window tests は desktop session 前提で ignored — 従来通り） |
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

# 出力先・revision の上書き
RF313_SMOKE_VERDICT=/tmp/v.json RF313_SMOKE_REVISION=<rev> \
  cargo run -p ruviform-ui-gpui --features gpui --example real_os_smoke
```

## NotRun / Unsupported の理由と owner

| 項目 | status | 理由 | owner / 後続 |
| --- | --- | --- | --- |
| ポインタ実入力（マウス移動・クリック） | NotRun | カーソル実占有・座標・clip/復元の設計が必要。本 dispatch はキーボードのみ | pointer smoke タスク（未起票） |
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
