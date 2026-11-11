---
id: "SPEC-034UdM9XszyXZB0hZ8wvnc"
type: "spec"
status: "implemented"
title: "Issue #243 GPUI 実機 GUI 検証 — Windows 実窓ライフサイクル証拠（部分スコープ）"
created: "2026-11-09"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/431"
  changes:
    added:
      - "crates/ruviform-ui-gpui/examples/window_lifecycle_probe.rs"
      - "docs/designs/issue-243-gpui-realwin-windows.md"
      - "docs/plans/implement-issue-243-gpui-realwin-windows.md"
      - "docs/tasks/implement-issue-243-gpui-realwin-windows.md"
      - "docs/test-specs/issue-243-gpui-realwin-windows.md"
      - "docs/impl/ir/issue-243-gpui-realwin-windows.md"
    modified:
      - "crates/ruviform-ui-gpui/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/issue-243-gpui-realwin-windows.md"
  depends-on:
    - "../specs/issue-386-driver-neutral-gui-host.md"
    - "../specs/issue-431-webview-gpui-real-verify.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/issue-431-webview-gpui-real-verify.md"
    - "../reports/issue-431-real-verify.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-431-webview-gpui-real-verify.md"
  derived-by:
    - "../designs/issue-243-gpui-realwin-windows.md"
  verifies: []
  verified-by:
    - "../impl/ir/issue-243-gpui-realwin-windows.md"
  references:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  defers: []
---

# Issue #243 GPUI 実機 GUI 検証 — Windows 実窓ライフサイクル証拠（部分スコープ）

## What Should Be Built

Issue #243（GPUI Multiwindow/Dock・Native Surface 実機 GUI 検証、#181/#182 後続）の
**Windows 部分スコープ**として、GPUI 標準 Driver（`ruviform-ui-gpui` の `GpuiDriver`）が
実 OS window で発行する Window/View lifecycle を実測し、証拠として記録する。

具体的には、headless で検証済みの共有 Window/Dock lifecycle 契約
（`ruviform-ui-test` の `run_lifecycle` fixture、Issue #386/#431）のうち
実窓で観測可能な項目を Windows 実機で再確認し、#431 で NotRun だった項目
（OS 経路の `Veto`、実窓での focus 遷移、driver API 経由の resize、
id 非再利用・再生成、window 生成失敗の縮退経路）を可能な範囲で実測する。

本 spec は検証・証拠文書の作成と、観測用の manual example 1 本の追加を対象とし、
Issue #243 全体の受入（3 OS・Plugin Dock・DPI/モニター変動・consumer 非回帰）は
対象外とする。Issue は本 dispatch 完了後も Open 継続が前提。

## Why Now

Issue #243 の現行受入は「GPUI 標準 Driver に基づく実 OS Window/Dock/Focus/IME/DPI・
Window/View lifecycle の Capability 別 NotRun/Unsupported 記録」を要求する。
headless 契約（#181/#182 論理契約・#386 Driver 契約・#431 実機検証）は揃っているが、
#431 の Windows 実機証拠は 2026-11-08 時点のもので、OS 経路 `Veto`・実窓 focus 遷移・
生成失敗縮退・id 非再利用の実測が未取得のまま残っている。本 worktree（Windows 11）は
実 GPUI window を起動できるため、ここで欠落分の実測証拠を補完する。

## Users and Value

- Ruviform の GUI 基盤 consumer（Synvelis・AudioLens）が依存する
  Window/Dock lifecycle 契約の実 OS での成立性を、再現手順つきの証拠で確認できる。
- 後続の macOS/Linux dispatch・Plugin Dock 受入（#388/#389・Synvelis #176）が
  「何が Windows で実測済みで何が未検証か」を正確に参照できる。

## Scope

### In Scope

- Windows 11 上の実 GPUI window による以下の実測と記録:
  - 複数 window 生成（2 窓以上）と window 別 `Created`/`Ready`/`Destroyed`
  - window 別 focus: driver `focus(w, true)` による `Focused(true/false)` 遷移、
    `focus(w, false)` の `Unsupported` 応答
  - OS close 経路（`WM_CLOSE` = `PostMessageW`、close ボタンと同じメッセージ）の
    `CloseRequested` → `Veto`/`Defer`/`Approve`/無応答既定 Approve
  - `request_close`/`destroy_window`/`respond_close` の programmatic 経路
  - Dock 2-phase move・abort・detach/reattach・close 時 view relocate
    （`real_window_lifecycle` example の再実行による現行 main での再確認）
  - `resize` API → 実 `Resized` イベント、`monitors()` の実値
  - window id 非再利用（destroy 後の再生成で新 id）
  - window 生成失敗の縮退経路（degenerate `WindowSpec` での観測。結果は実測どおり記録）
- 実測結果の文書化: 実行コマンド・OS/toolchain・観測挙動・ログ証跡・
  各項目の Pass/Fail/NotRun/Unsupported 分類と再現手順
- 上記のための観測用 example 1 本（`window_lifecycle_probe`）の追加

### Out of Scope

- macOS / Linux の実機検証（本 dispatch の環境は Windows のみ）
- Native Plugin Dock の受入（#388/#389・Synvelis #176 の範囲。
  Floating/Fake Surface の結果を Plugin Dock PASS にしない）
- DPI 変更・マルチモニター間移動・モニター切断の実機検証（環境が整わなければ NotRun）
- 実ユーザー入力（物理マウス/キーボード/IME）に依存し自動化経路のない項目。
  `PostMessageW`/`SendInput` 等の OS メッセージ注入は「物理的ユーザー入力」と称さず、
  各項目がどの経路（gpui 内部 API / OS メッセージ / 手動）を経由したかを記録する
- AudioLens/Synvelis consumer の非回帰確認（consumer repo の作業）
- 製品コード・公開 API の変更（観測用コードを本番 crate に追加しない）

## Requirements

- 実測値と推測を区別して記録する。実行していない検証を Passed と書かない。
- OS メッセージ注入（`WM_CLOSE`）と gpui 内部 API（`request_close`/`focus`/`resize`）
  と手動操作を、証拠上で明確に区別する。
- headless 契約で定義済みだが実窓で観測できない項目は NotRun/Unsupported とし、
  理由と後続 owner を明記する。
- 検証は `mise.toml` タスクを正本とし、cargo 直接実行は補助証拠として明記する。

## Acceptance Criteria

- [ ] Windows 実 GPUI window で複数 window 生成・window 別 focus・window 閉鎖・
  detach/reattach の実挙動を可能な範囲で確認し、実測項目と確認方法を記録した
- [ ] window 生成失敗・close 要求（Veto/Defer/Approve/無応答既定）・view state 維持・
  id 非再利用など headless 契約で定義済みの挙動のうち実窓で観測できるものを実測した
- [ ] 実測結果を Windows 実機証拠として文書化した（実行コマンド、OS/ツールチェーン、
  観測した挙動、ログ等の証跡、Pass/Fail/NotRun/Unsupported 分類と再現手順）
- [ ] 対象外項目（macOS/Linux・Plugin Dock・DPI/モニター・物理入力依存・consumer 回帰）を
  NotRun/Unsupported として理由と owner つきで明記した
- [ ] `mise run fmt-check` / `mise run check` / `mise run test-ui-gpui` が pass する

## Deferred Design Concerns

- macOS/Linux での同等実測: 本 dispatch の環境外。再開条件は各 OS の実機環境。
  放置リスク: OS 別 Driver 差異（activation/close semantics）が未検出のまま残る。
- DPI/マルチモニター/モニター切断: `monitors()` は実値を記録するが、
  `ScaleFactorChanged` の実遷移は複数 DPI 環境が必要。owner: 後続の #243 dispatch。
- 物理ユーザー入力経由の検証（ドラッグ移動・実クリック・IME）: 自動化経路がない。
  owner: #313 共通 Driver lane または手動検証の計画。
- consumer 非回帰: audio-lens#166・synvelis#113/#114 が owner。

## Sources and Relations

- Issue #243 本文（2026-11-09 GPUI 移行後の受入判定節が現行正本）
- Issue #386 Driver-neutral GUI Host 設計（`docs/designs/issue-386-driver-neutral-gui-host.md`）
- Issue #431 実機検証報告（`docs/reports/issue-431-real-verify.md`）— 本 spec の直接の先行証拠
- 共有 lifecycle fixture（`crates/ruviform-ui-test/src/lifecycle.rs`）— headless 契約の正本
