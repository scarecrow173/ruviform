---
id: "REPORT-Bf5Kp2Xs7Yn4Dz8Lg3Tc6Hv"
type: "report"
status: "partial"
title: "Issue #431 WebView 実機 e2e・GPUI 実窓 lifecycle 検証・legacy trait 判断の記録（Windows 11 x64）"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/431"
  changes:
    added:
      - "evidence/issue-431-real-verify/wm-close.ps1"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/issue-431-webview-gpui-real-verify.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-431-webview-gpui-real-verify.md"
    - "../adr/issue-431-legacy-host-trait-phase2-retain.md"
    - "../reports/issue-423-gpui-real-window.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-431-webview-gpui-real-verify.md"
  derived-by: []
  verifies:
    - "../specs/issue-431-webview-gpui-real-verify.md"
  verified-by: []
  references: []
  defers: []
---

# Issue #431 検証報告（Windows 11 x64）

実施日: 2026-11-08。環境: Windows 11 x64・Git Bash・mise 2026.10.1・Rust 1.99.0・
WebView2 Runtime 157.0.4341.42（レジストリ `HKLM\SOFTWARE\WOW6432Node\...\EdgeUpdate\Clients` の `pv`）。
macOS / Linux は未検証。

## 0. 受入条件との対応

| 条件 | 結果 | 根拠 |
| --- | --- | --- |
| A1 WebView 実機で往復・`Created`/`Ready` 実発行 | **Pass** | §1.1（e2e_smoke）・§1.2（demo の OS close） |
| A2 GPUI 実窓の close veto/defer/approve・Dock 2-phase・detach/reattach | **Pass（部分）**: defer/approve・Dock 遷移は Pass。OS 経路の veto（`Veto`）は NotRun | §2.2・§2.3 |
| A3 legacy trait の判断と根拠 | **Pass**（判断: 保留。status は proposed） | `docs/adr/issue-431-legacy-host-trait-phase2-retain.md` |
| A4 `check-ui-driver-boundary` pass | **Pass** | §4 |
| A5 manual タスク登録・CI 非対象 | **Pass** | `mise.toml`（`example-ui-webview-e2e`・`example-ui-gpui-lifecycle`） |

## 1. WebView 実機 e2e

### 1.1 e2e_smoke（自動化した往復）

コマンド: `mise run example-ui-webview-e2e`（実体は `cargo run -p ruviform-ui-webview --features webview --example e2e_smoke`）

```text
[e2e] backend: webview2 state=Available version=Some("157.0.4341.42")
[e2e] Created
[e2e] ipc:ping
[e2e] Ready
[e2e] ipc:received
[e2e] PASS milestones=["Created", "ipc:ping", "Ready", "ipc:received"]
exit=0
```

- `Created`: window と WebView2 の生成完了。`ipc:ping`: page の `__rvf.call` が Rust まで届いた。
- `Ready` は `ipc:ping` の後に来ている。page の script は load 完了前に実行され、result は backlog 経由で
  `Ready` 後に配送された。`ipc:received` は page が result を解決して再送した intent であり、
  Rust → page の event 配送が page 側に届いたことを示す。

### 1.2 demo_app / demo_multi（起動と OS close）

`cargo build --examples` 後、`evidence/issue-431-real-verify/wm-close.ps1` で起動し `WM_CLOSE` を送った。

| 対象 | 操作 | 観測 | exit |
| --- | --- | --- | --- |
| demo_app | `demo_app` window へ `WM_CLOSE` | 対象 window が閉じ、プロセスが終了 | 0 |
| demo_multi | `panel` へ `WM_CLOSE` | プロセスは生存（loop 継続） | — |
| demo_multi | `main` へ `WM_CLOSE` | プロセス終了 | 0 |

両者は起動直後に `backend: webview2 state=Available` を出力し、タイムアウトまで panic せず動作した
（WebView2 の `Chrome_WidgetWin_0 ... Error = 1411` は終了時の既知のノイズで、結果に影響しない）。

### 1.3 WebView の未検証項目（NotRun）

| 項目 | 状態 | 手順 |
| --- | --- | --- |
| `ScaleFactorChanged`（DPI の異なるモニタ間移動） | NotRun（実 OS lane。複数 DPI モニタが必要） | `demo_app` を DPI の異なるモニタへドラッグ。ログは `demo_app` の出力に現れないため、必要なら `DriverEvent` を出力する手順を追加する |
| 手動の resize / focus 切替 | NotRun（操作が必要） | `demo_app` で window をドラッグしてリサイズし、別アプリを前面にして戻す |
| devtools | NotRun（debug ビルドのみ・目視） | debug 実行で devtools を開く |
| macOS WKWebView / Linux WebKitGTK | Unsupported（本環境に無い） | — |

## 2. GPUI 実窓

### 2.1 minimal_window（報告書 #423 の M1）

`mise run example-ui-gpui-minimal` を起動し、`WM_CLOSE` を送った。

```text
[fixture] window id: w1
[event] w1: Created
[event] w1: Ready
[event] w1: Focused(true)
[event] w1: Resized { width: 1280, height: 800 }
[event] w1: Moved { x: 320, y: 152 }
[event] w1: CloseRequested — approve
[event] w1: Destroyed — exiting loop
[fixture] exited cleanly
exit=0
```

M1 は **Pass**（`CloseRequested` → approve → `Destroyed` → 終了コード 0）。ただし物理的な close ボタンの
クリックではなく、同じ `WM_CLOSE` を送った結果である。#423 報告書の M1 は、この結果で補完する。

### 2.2 real_window_lifecycle（Dock・close の実窓検証）

コマンド: `powershell ... wm-close.ps1 -Exe target/debug/examples/real_window_lifecycle.exe -Steps "inspector,inspector,main"`

| step | 観測 |
| --- | --- |
| 1 回目 `inspector` へ `WM_CLOSE` | `CloseRequested` → `respond Defer`。window は **開いたまま** |
| 2 回目 `inspector` へ `WM_CLOSE` | `CloseRequested` → `respond Approve` → `Destroyed`。window が閉じる |
| `main` へ `WM_CLOSE` | `CloseRequested` → `respond Approve` → `Destroyed` → loop 終了 |

```text
[real] main: Created / Ready
[real] inspector: Created / Ready
[real] inspector: CloseRequested / respond Defer
[real] inspector: CloseRequested / respond Approve
[real] inspector: Destroyed
[real] main: CloseRequested / respond Approve
[real] main: Destroyed
[real] PASS
exit=0
```

Dock の 2-phase 遷移は両 window の `Ready` 後に実行し、各段で所有者を検査した（いずれも成立）:

- v2 を `inspector` へ begin → commit（所有者が `inspector` になる）
- v1 を `inspector` へ begin → abort（v1 は `main` に残る）
- v1 を detach → reattach（所有者が `inspector`）
- `inspector` の Destroyed 後、views を `main` へ `RelocateTo`（2 件）

**陰性対照**: `-Steps "inspector,main"`（`inspector` を 1 回しか閉じない）では、
`[real] FAIL: trace {"inspector": ["Created", "Ready", "CloseRequested"], ...} != expected {...}` を出し、
exit 1 で終了した。trace 照合が失敗を検出できることを確認した。

### 2.3 GPUI の未検証項目（NotRun）

| 項目 | 状態 | 理由・手順 |
| --- | --- | --- |
| OS 経路の `Veto` | NotRun（headless では検証済み） | `real_window_lifecycle` は `Defer`/`Approve` のみ使う。Veto の実機確認は、`Veto` を返す variant を追加した example が必要 |
| M2 移動・リサイズ | NotRun（手動操作） | `minimal_window` を起動し、window をドラッグ・リサイズして `Moved`/`Resized` を確認 |
| M3 focus 切替 | NotRun（手動操作） | 別アプリを前面にして `Focused(false)`、戻して `Focused(true)` を確認 |
| M4 ScaleFactorChanged | NotRun（実 OS lane） | DPI の異なるモニタへ移動 |
| Dock の UI（ドラッグ・描画） | 範囲外 | Dock は `DockWorkspace` のモデル操作を検証した。描画とドラッグ UI は consumer 側（#262/#243/#390 系） |
| IME・GPU device lost | NotRun（実 OS lane） | #262/#243/#390 系 |

## 3. legacy trait の判断

判断は保留（Phase 2 の削除はしない）。根拠・使用箇所の棚卸し・再評価条件は
[ADR](../adr/issue-431-legacy-host-trait-phase2-retain.md) に記録した。
要点: Phase 2 gate（Synvelis の native 最小 Slice・audio-lens の `WindowLifecycle` 直接消費・両 consumer の実 OS 合格）は未達。
`HostEvent`・`WindowHandle` は runtime・テスト・consumer の経路上にあり、`HostHandle` は実装者 0 件で
公開 API の再 export のみである。

## 4. 検証ゲート（merge 候補の HEAD で実行。exit 0 が全件）

| task | exit | 主な結果 |
| --- | --- | --- |
| `fmt-check` | 0 | — |
| `check` | 0 | — |
| `lint` | 0 | — |
| `check-ui-driver-boundary` | 0 | `[gui-boundary-guard] verdict: pass`（dep-graph / signature / allowlist すべて pass） |
| `test-ui-driver-contract` | 0 | 7 passed |
| `check-ui-headless` | 0 | — |
| `test-ui-headless` | 0 | — |
| `test-ui-component-view` | 0 | — |
| `test-ui-test-msrv` | 0 | — |
| `check-ui-gpui` | 0 | — |
| `lint-ui-gpui` | 0 | — |
| `test-ui-gpui` | 0 | テストバイナリ 3 件の結果: 4 passed / 46 passed / 0 passed（失敗 0） |
| `check-ui-webview` | 0 | — |
| `lint-ui-webview` | 0 | — |
| `test-ui-webview` | 0 | 10 passed（doctest 1 件は既存の `ignore`：`embed.rs`） |
| `test` | 0 | workspace 全体（171 + 他 crate の合計すべて pass） |

実機の manual 項目（§1.1・§1.2・§2.1・§2.2）は CI の対象外のため、上の表には含めていない。

## 5. review-protocol 結果

独立した 2 名のレビュー（A: コードと example、B: 文書と主張の根拠）を read-only で実施し、
Validator 相当の確認を作業者が行った。判定は ACCEPT WITH NON-BLOCKING NOTES（blocker なし）。

| ID | 重大度 | 指摘 | 対応 |
| --- | --- | --- | --- |
| A-1 | P2 | e2e の判定が集合の包含で、文書の「順序どおり」と不一致 | `in_order` で `Created < ipc:ping < ipc:received` と `Created < Ready` を判定に追加し、再実行して PASS |
| A-2 | P3 | e2e の exit code 2（backend 不在）が doc comment に無い | doc comment に追記 |
| A-3 | P3 | `wm-close.ps1` が `PostMessageW` の戻り値を捨て、失敗時も成功として表示する | 戻り値を判定し、失敗時は明示的に出力 |
| A-4 | P3 | `wm-close.ps1` の `-ArgList` が cmd 行へ無加工で入る | 信頼できる手元の引数のみを想定する手動補助。変更なし |
| A-5 | P3 | `wm-close.ps1` が一時ログ（`$env:TEMP`）を削除しない | 証跡として残す方針。変更なし |
| B-1 | P2 | mise task・T3・spec A2 の「veto」が OS 経路の Veto 実行と読める | 「defer/approve」へ統一。Veto の OS 経路は NotRun と明記（§2.3）。example のコメントも修正 |
| B-2 | P2 | task 表の T8〜T10 が pending のまま | README・検証・報告書の完了に合わせて done へ更新 |
| B-3 | P1（条件付き） | レビュー B は `/tmp` の検証ログを読めず、§4 の exit code を独立に確認できなかった | 作業者が各タスクの exit code と主要行（テスト件数・verdict）をログから直接確認した。レビュー後も §4 の一部を再実行（後述） |
| B-4 | P3 | `4 / 46 passed` が曖昧 | テストバイナリごとの結果として明記 |
| B-5 | P3 | §5 と §6 が空の参照だった | 本節に統合 |

レビュー後の再実行（判定・文書の修正を反映）: `fmt-check`・`lint-ui-webview`・`test-ui-webview`・
`example-ui-webview-e2e`（PASS, exit 0）・GPUI の陽性シナリオ（`wm-close.ps1` 経由、PASS, exit 0）。
§4 の他のゲートは修正前に実行したもので、修正は文書・判定条件・PowerShell 補助に限られるため、
Rust の公開面には影響しない。`check-ui-driver-boundary` と `lint-ui-gpui` は最終確認として再実行する。

## 6. 再現手順（manual）

```sh
# WebView 往復（WebView2 必須）
mise run example-ui-webview-e2e

# GPUI lifecycle（実 OS session 必須）
mise run example-ui-gpui-lifecycle        # 手動で inspector を 2 回・main を 1 回 close
# または自動（別シェル不要）
powershell -NoProfile -ExecutionPolicy Bypass -File evidence/issue-431-real-verify/wm-close.ps1 \
  -Exe target/debug/examples/real_window_lifecycle.exe -Steps "inspector,inspector,main"

# 旧 demo の close 経路
powershell -NoProfile -ExecutionPolicy Bypass -File evidence/issue-431-real-verify/wm-close.ps1 \
  -Exe target/debug/examples/demo_multi.exe -Steps "panel,main"
```

`wm-close.ps1` は `WM_CLOSE` を送る。物理的なマウス操作の代替ではなく、close ボタンと同じ
メッセージを送る点が実機操作との差である。
