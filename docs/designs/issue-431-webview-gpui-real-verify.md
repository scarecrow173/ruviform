---
id: "DESIGN-Wq8Ht3NcZ6RmLv1KbFg7Sd"
type: "design"
status: "proposed"
title: "Issue #431 実機 e2e・実窓 lifecycle 検証・OS close 駆動の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/431"
  changes:
    added:
      - "crates/ruviform-ui-webview/examples/e2e_smoke.rs"
      - "crates/ruviform-ui-webview/examples/e2e_smoke.html"
      - "crates/ruviform-ui-gpui/examples/real_window_lifecycle.rs"
      - "evidence/issue-431-real-verify/wm-close.ps1"
    modified:
      - "crates/ruviform-ui-webview/Cargo.toml"
      - "crates/ruviform-ui-gpui/Cargo.toml"
      - "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-431-webview-gpui-real-verify.md"
  implemented-by: []
  depends-on:
    - "../designs/issue-423-webview-dock-verify.md"
    - "../designs/issue-386-driver-neutral-gui-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/issue-431-legacy-host-trait-phase2-retain.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-431-webview-gpui-real-verify.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../reports/issue-423-gpui-real-window.md"
  defers: []
---

# Issue #431 実機 e2e・実窓 lifecycle 検証・OS close 駆動の設計

## 1. 方針

- headless で再現できない確認は、**実 OS の window を開く example** と、その example を
  **実 OS の close 操作と同じ `WM_CLOSE` で駆動する PowerShell 補助**の組で検証する。
- example は CI に入れず、`mise.toml` の manual タスクとして登録する（既存の
  `example-ui-gpui-minimal` / `poc-native-dock` と同じ扱い）。
- 判定は example の終了コードと、example が出力する trace の照合で行う。目視判定は使わない。

## 2. WebView 実機 e2e（`e2e_smoke`）

`crates/ruviform-ui-webview/examples/e2e_smoke.rs` は `run_multi` を 1 window で起動し、
次の 4 マイルストーンを順に観測する。

| マイルストーン | 発生源 | 意味 |
| --- | --- | --- |
| `Created` | `session::create_entry` | tao window と WebView2 の生成完了 |
| `ipc:ping` | page の `__rvf.call('demo.echo', {probe:'ping'})` | page → Rust IPC |
| `Ready` | `PageLoadEvent::Finished` | page load 完了（`Ready` で backlog を flush） |
| `ipc:received` | page が `ping` の result を受けて再送した `probe:'received'` | Rust → page の event 配送が page に届いた |

`ipc:received` は page 側が result promise を解決した後にしか送られないため、Rust→page 配送の
証拠になる。`ping` の result は `Ready` より前に届く場合は backlog 経由で `Ready` 後に配送される。
観測済みの順序（`Created` → `ipc:ping` → `Ready` → `ipc:received`）は、late receipt 経路を通ったことを示す。

ウィンドウ close 経路（`CloseRequested` → `Approve` → `Destroyed` → exit）は `demo_app` /
`demo_multi` を `wm-close.ps1` で駆動して確認する（§4）。

## 3. GPUI 実窓 lifecycle（`real_window_lifecycle`）

`crates/ruviform-ui-gpui/examples/real_window_lifecycle.rs` は次を実 window 上で行う。

1. `main` と `inspector` を生成し、両方が `Ready` になった時点で Dock 操作を行う。
   - v2 を `inspector` へ committed 2-phase move
   - v1 を `inspector` へ begin → abort（v1 は `main` に残る）
   - v1 を detach → reattach（`inspector` 所有）
2. OS close 要求に応答する。`inspector` の 1 回目は `Defer`（window は開いたまま）、
   2 回目は `Approve`、`main` は `Approve`。
3. `inspector` の `Destroyed` 後、Dock views は `main` へ `RelocateTo` する（consumer の close policy）。
4. 正規化した trace を、共有 fixture `ruviform_ui_test::lifecycle::expected_trace()` の
   `main`/`inspector` の項目と照合する。不一致または timeout（120 秒）は exit 1。

Dock 操作は `DockWorkspace` のモデル操作であり、描画やドラッグは含まない。実 window 上で確認するのは
「Dock の所有遷移が実 window の lifecycle と同じプロセスで成立すること」と
「OS close が defer/approve の契約どおりに効くこと」である。

### 3.1 Defer が OS close を止める仕組み

GPUI の `on_window_should_close` は `on_should_close` を呼び、`CloseRequested` を emit する。
handler が `Defer` を返すと `awaiting_response` が false のままになり、`should_close` は `false` を返す。
ここで window は閉じない。2 回目の close は新たな `CloseRequested` を emit し、`Approve` で閉じる。

`Veto` も同じ経路で window を閉じないが、本 example は `Defer` のみを OS 経路で検証する。
`Veto` は共有 fixture（headless）で検証済みであり、実 OS での `Veto` は未確認である。

## 4. OS close 駆動（`wm-close.ps1`）

`evidence/issue-431-real-verify/wm-close.ps1` は次を行う。

- 対象 exe を `cmd /c` 経由で起動し、stdout/stderr をファイルへ保存する（pipe は WebView2 の子プロセスに
  継承されて EOF が来なくなるため使わない）。
- app の PID（`cmd` の子）を求め、`EnumWindows` で同 PID の可視タイトル付き top-level window を列挙する。
- `-Steps "a,b,c"` の各要素について、タイトルに部分一致する window へ `WM_CLOSE`（close ボタンと同じ
  メッセージ）を送る。各 step 後に「プロセス生存」と「対象 window が開いているか」を出力する。
- 最終的な exit code と子プロセスの出力を表示する。

`WM_CLOSE` は winit/GPUI の close 要求と同一経路に入るが、物理的なマウスクリックそのものではない。
この差は報告書に明記する。

## 5. legacy trait Phase 2 判断

判断は `docs/adr/issue-431-legacy-host-trait-phase2-retain.md` に記録する。本設計では、判断に使う
観点（実装者・呼び出し元・consumer・Phase 2 gate）と、本 PR で削除しない理由のみを扱う。

## 6. 検証の入口

- 自動 lane: `mise run check-ui-webview` / `lint-ui-webview` / `test-ui-webview`、
  `check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`、`check-ui-driver-boundary` など既存タスク。
- manual lane: `mise run example-ui-webview-e2e`、`mise run example-ui-gpui-lifecycle`、
  `example-ui-gpui-minimal`。いずれも実 OS session が必要で CI に入れない。
