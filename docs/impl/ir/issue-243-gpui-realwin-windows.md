---
id: "IMPL-034Ue3QZ2los1ccAjflw0X"
type: "impl"
status: "completed"
title: "Issue #243 GPUI 実機 GUI 検証 Windows 実窓ライフサイクル証拠"
created: "2026-11-09"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "docs/specs/issue-243-gpui-realwin-windows.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/window_lifecycle_probe.rs"
      - type: "file"
        path: "docs/specs/issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/designs/issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/plans/implement-issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/tasks/implement-issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/test-specs/issue-243-gpui-realwin-windows.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-243-gpui-realwin-windows.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/implement-issue-243-gpui-realwin-windows.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../reports/issue-431-real-verify.md"
    - "../ir/issue-243-multiwindow-closeout-slice.md"
    - "docs/designs/issue-243-gpui-realwin-windows.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/issue-243-gpui-realwin-windows.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-243-gpui-realwin-windows.md"
  verified-by: []
  references:
    - "docs/specs/issue-243-gpui-realwin-windows.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #243 GPUI 実機 GUI 検証 Windows 実窓ライフサイクル証拠

## Summary

Issue #243 の Windows 部分スコープとして、GPUI Driver（`ruviform-ui-gpui` の
`GpuiDriver`）の Window lifecycle を Windows 11 実機で実測した。観測用 example
`window_lifecycle_probe` を 1 本追加し、probe 既定モード・degenerate モード・
`real_window_lifecycle`・`minimal_window` の 4 実行で契約項目を実測した。
#431 で NotRun だった OS 経路 `Veto`・実窓 focus 遷移・id 非再利用・
`destroy_window` bypass を全て実測で確認した。Issue #243 全体は Open 継続
（macOS/Linux・Plugin Dock・DPI/モニター・consumer 回帰は本スコープ外）。

## Context

- spec: `docs/specs/issue-243-gpui-realwin-windows.md`
- design: `docs/designs/issue-243-gpui-realwin-windows.md`（approved）
- plan: `docs/plans/implement-issue-243-gpui-realwin-windows.md`
- test-spec: `docs/test-specs/issue-243-gpui-realwin-windows.md`
- task: `docs/tasks/implement-issue-243-gpui-realwin-windows.md`
- 先行証拠: `docs/reports/issue-431-real-verify.md`（2026-11-08 Windows 実機）
- headless 契約の正本: `crates/ruviform-ui-test/src/lifecycle.rs`（`run_lifecycle`）

## Environment（全実行共通）

- OS: Windows 11 Pro 10.0.26300（x64）・対話 session
- toolchain: mise 2026.10.1・rustc 1.99.0（`rust = "1.99.0"`）・GPUI 0.2.2
- revision: `57e4e8ce`（`origin/main` と同一。実行時点）
- 実施日: 2026-11-09
- モニタ環境: 1920x1080 × 2 面（scale=1.0 = 100% DPI、同一 DPI のみ）

## Implementation

### Changes

- `crates/ruviform-ui-gpui/examples/window_lifecycle_probe.rs`（新規・manual
  verification 専用、`required-features = ["gpui"]`、CI 対象外）
  - 既定モード: `main`/`veto` 2 窓生成 → Tick 駆動 step 機械で
    `focus`/`resize`/`monitors` を順に実行 → 外部 `WM_CLOSE` で
    veto へ Veto/Defer/Approve・main へ無応答既定 → `reborn` 再生成
    （id 非再利用）→ `request_close`+`Veto` → `destroy_window` bypass
    → trace 照合で PASS/FAIL・exit code 反映。
  - `--degenerate-create` モード: 0x0 `WindowSpec` の生成応答を分類出力
    （別プロセス・別実行。観測専用で assert しない）。
- `crates/ruviform-ui-gpui/Cargo.toml`: `[[example]]` エントリ追加のみ
  （workspace `check --all-targets` が default feature で example を
  build しないために必須。メンバー定義・依存は不変）。

### Decisions During Implementation

- OS close の駆動は `evidence/issue-431-real-verify/wm-close.ps1` を再利用
  （`PostMessageW(WM_CLOSE)` = close ボタンと同じメッセージだが物理入力では
  ない。証拠上この区別を維持する）。probe ops（~5s）と close 開始（15s）を
  `-StartWaitMs 15000` で時間的に分離した。
- `destroy_window` の `CloseRequested` bypass は契約上の期待として assert
  した。実測では bypass を確認（reborn trace は `CloseRequested` 1 回のみ）。
- 0x0 spec は失敗を起こさず platform が最小サイズへ clamp したため、
  「生成失敗」は invalid spec では誘発できないことが実測で判明。
  分類結果をそのまま記録した（推測で失敗を主張しない）。

## 実行と実測結果

### 1. probe 既定モード（OS close 経路を含む総合検証）

コマンド（補助証拠。canonical な example 実行入口は mise task 未登録のため
cargo を直接使用 — `mise.toml` は編集禁止範囲）:

```sh
powershell -NoProfile -ExecutionPolicy Bypass -File \
  evidence/issue-431-real-verify/wm-close.ps1 \
  -Exe target/debug/examples/window_lifecycle_probe.exe \
  -Steps "veto,veto,veto,main" -StartWaitMs 15000
```

exit code: **0**（`[wm-close] exited code=0`・`[probe] PASS`）

`wm-close.ps1` 側の観測（window 生存の外部確認）:

```text
[wm-close] WM_CLOSE -> 'rf243 probe veto'   → after: alive=True  open=True
[wm-close] WM_CLOSE -> 'rf243 probe veto'   → after: alive=True  open=True
[wm-close] WM_CLOSE -> 'rf243 probe veto'   → after: alive=True  open=False
[wm-close] WM_CLOSE -> 'rf243 probe main'   → after: alive=True  open=False
[wm-close] exited code=0
```

probe stderr（要点。全量は実行時ログ。時系列順）:

```text
[probe] capabilities: WindowCapabilities { resize: Supported, focus: Supported,
  scale_factor: Supported, multi_window: Supported, ime: Unsupported,
  native_surface_slot: Supported }
[probe] created w1 'rf243 probe main'
[probe] created w2 'rf243 probe veto'
[probe] main: Created / Ready / Focused(true) / Resized 1280x800 / Moved 320,152
[probe] veto: Created / Ready / Focused(true) / Resized 1280x800 / Moved 320,152
[probe] main: Focused(false) / veto: Focused(true)          ← 起動時の活性化
[probe] op: focus(veto, true)     → ok
[probe] op: focus(main, true)     → ok
[probe] veto: Focused(false) / main: Focused(true)          ← window 別 focus 遷移
[probe] op: focus(veto, false)    → Unsupported (contract)  ← gpui 0.2.2 の契約上の正解
[probe] op: focus(w1844...51615, true) → Host (contract)    ← 未知 window
[probe] op: resize(main, 960x600) → ok
[probe] main: Resized 960x600                               ← 実 window の resize 反映
[probe] op: monitors()
[probe] monitor: 1920x1080 @0,0 scale=1 / 1920x1080 @1920,0 scale=1  ← 2 面実値
[probe] checkpoint: last_focus={w1: true, w2: false}; failures=0
[probe] veto: CloseRequested #1 -> Veto    → Destroyed なし（開いたまま）
[probe] veto: CloseRequested #2 -> Defer   → Destroyed なし（close-pending 維持）
[probe] veto: CloseRequested #3 -> Approve → veto: Destroyed（Destroyed は 1 回のみ）
[probe] reborn created w3 (veto was w2)    ← id 非再利用
[probe] reborn: Created / Ready / Focused(true) / Resized / Moved
[probe] main: CloseRequested — no response (default approve)
[probe] main: Destroyed                                       ← 無応答既定 Approve
[probe] op: request_close(w3) → reborn: CloseRequested #1 -> Veto（programmatic 経路）
[probe] op: destroy_window(w3) → reborn: Destroyed           ← CloseRequested 追加なし = bypass 確認
[probe] trace main:   [Created, Ready, CloseRequested, Destroyed]
[probe] trace veto:   [Created, Ready, CloseRequested×3, Destroyed]
[probe] trace reborn: [Created, Ready, CloseRequested, Destroyed]
[probe] PASS
```

### 2. probe `--degenerate-create`（生成失敗縮退の観測）

```sh
./target/debug/examples/window_lifecycle_probe.exe --degenerate-create
```

exit code: **0**。0x0 `WindowSpec` は **拒否されず受理**された:

```text
[probe-degenerate] create_window ok w1
[probe-degenerate] Created / Ready / Focused(true)
[probe-degenerate] Resized { width: 120, height: 1 }   ← OS が最小サイズへ clamp
[probe-degenerate] Moved { x: 960, y: 552 }
[probe-degenerate] classification: 0x0 spec accepted: Created+Ready observed
```

分類: **受理（失敗ではない）**。driver は spec を検証せず、Windows が
最小サイズ（120x1 物理 px）へ clamp して window を開いた。15s 観測後に
自死 exit 0。invalid spec では「window 生成失敗」経路（`Destroyed` 縮退・
id 退役）は誘発できない — 真の OS レベル生成失敗（リソース枯渇等）は
本 dispatch では誘発不能として NotRun に分類。

補足: 初回は `wm-close.ps1` 経由で実行したが、スクリプトの固定
`WaitForExit(10000)` が分類出力（15s）前にプロセスを force-stop した。
直接実行で再測し分類を取得（挙動自体は同一）。

### 3. `real_window_lifecycle`（Dock・OS close の現行 main 再確認）

```sh
powershell -NoProfile -ExecutionPolicy Bypass -File \
  evidence/issue-431-real-verify/wm-close.ps1 \
  -Exe target/debug/examples/real_window_lifecycle.exe \
  -Steps "inspector,inspector,main"
```

exit code: **0**（`[real] PASS`）。両窓 `Ready` 後に DockWorkspace で
v2 の 2-phase move commit（owner=inspector）・v1 の begin/abort
（owner=main 維持）・v1 detach/reattach（owner=inspector）を各段で検査し、
`inspector` の Destroyed 後に view 2 件を `main` へ `RelocateTo`。
OS `WM_CLOSE` は Defer（開いたまま）→ Approve（Destroyed）→ main Approve。
#431 報告の PASS を現行 main（`57e4e8ce`）で再確認した。

### 4. `minimal_window`（単一窓 smoke）

```sh
powershell -NoProfile -ExecutionPolicy Bypass -File \
  evidence/issue-431-real-verify/wm-close.ps1 \
  -Exe target/debug/examples/minimal_window.exe -Steps "minimal"
```

exit code: **0**。`Created`/`Ready`/`Focused(true)`/`Resized 1280x800`/
`Moved` → `WM_CLOSE` → `CloseRequested` → approve → `Destroyed` → 終了。

## 項目別 判定表

| 項目 | 経路 | 結果 | 根拠 |
| --- | --- | --- | --- |
| capabilities 実値 | gpui 内部 API | Pass | probe §1 冒頭（ime=Unsupported, native_surface_slot=Supported on Windows） |
| 複数 window 生成（2→3 窓） | gpui 内部 API | Pass | w1/w2/w3 の Created/Ready/Destroyed が window 別に独立発行 |
| window 別 focus（独立性） | gpui 内部 API（`focus(w,true)`→`activate_window`） | Pass | `Focused(false)`/`Focused(true)` が window 別に遷移。last_focus={main:true, veto:false} |
| `focus(w,false)` | gpui 内部 API | Pass（契約どおり Unsupported） | `UiError::Unsupported` を返す（no-op でない） |
| 未知 window への `focus` | gpui 内部 API | Pass | `UiError::Host` |
| `resize` → `Resized` | gpui 内部 API | Pass | `Resized 960x600` が resize 呼出後に到達 |
| `monitors()` 実値 | gpui 内部 API | Pass（近似の限界あり） | 2 面 1920x1080 scale=1。scale factor は driver の既知の推定値（先頭 window の観測値） |
| OS close: Veto | OS メッセージ（`PostMessageW WM_CLOSE`） | Pass | #431 NotRun を解消。window 生存を trace と外部確認の両方で確認 |
| OS close: Defer | OS メッセージ | Pass | close-pending 維持・window 生存 |
| OS close: Approve | OS メッセージ | Pass | `Destroyed` 1 回のみ |
| OS close: 無応答既定 | OS メッセージ | Pass | `respond_close` なしで `Destroyed`（window 単位の既定 Approve） |
| `request_close`（programmatic） | gpui 内部 API | Pass | `CloseRequested` 経路を通り Veto で生存 |
| `destroy_window` bypass | gpui 内部 API | Pass | `CloseRequested` 追加発行なしに `Destroyed` |
| window id 非再利用 | gpui 内部 API | Pass | reborn=w3 ≠ veto=w2/main=w1 |
| `Destroyed` 二重発行なし | 全経路 | Pass | retire は idempotent（trace 上 Destroyed は各窓 1 回） |
| Dock 2-phase/abort/detach/reattach/relocate | gpui 内部 API + DockWorkspace | Pass | `real_window_lifecycle` 再実行 PASS（owner 検査込み） |
| 単一窓 lifecycle smoke | OS メッセージ | Pass | `minimal_window` exit 0 |
| 生成失敗縮退（invalid spec） | gpui 内部 API | 実測: 失敗せず受理 | 0x0 → 120x1 clamp で Ready。真の生成失敗は誘発不能 → NotRun |
| view state 維持 | DockWorkspace モデル | Pass（モデル層） | detach が ViewInstanceId を保持・close 時 RelocateTo。描画 view 内部状態の個別計装はなし（制限として記録） |

## NotRun / Unsupported（理由と owner）

| 項目 | 分類 | 理由 | owner / 再開条件 |
| --- | --- | --- | --- |
| macOS / Linux 実機 | NotRun | 本 dispatch の環境は Windows のみ | 後続 #243 dispatch（各 OS 実機） |
| DPI 変更・別 DPI モニタ間移動・モニタ切断・`ScaleFactorChanged` 実遷移 | NotRun | 本環境は 2 面とも 100% DPI で DPI 差・切断を起こせない | 後続 #243 dispatch（複数 DPI/切断可能な環境） |
| IME 入力 | Unsupported | `capabilities().ime = Unsupported`（#389 の契約範囲） | Issue #389 |
| 物理ユーザー入力（実クリック・ドラッグ・キー入力） | NotRun | 自動化経路なし。`WM_CLOSE` は OS メッセージ注入であり物理入力と称さない | #313 共通 Driver lane または手動検証計画 |
| 真の window 生成失敗（OS リソース枯渇等） | NotRun | invalid spec では誘発不能（実測: 受理+clamp）。強制的な失敗注入経路が driver にない | 必要なら driver 側の fault-injection 拡張を別 Issue で |
| Native Plugin Dock 受入 | 対象外 | #388/#389・Synvelis #176 の OS 別 Gate | scarecrow173/ruviform#388/#389・synvelis#176 |
| NativeSurface attach 失敗経路 | NotRun | `native_surface_slot=Supported` だが attach 失敗の注入経路を本 dispatch で未実施 | #388/#389 系 |
| consumer 非回帰（AudioLens/Synvelis） | 対象外 | consumer repo の作業 | audio-lens#166・synvelis#113/#114 |
| `post_event` の実窓配送 | 部分 | headless fixture で検証済み。実窓では `RvfRoot` の描画件数に依存し自動判定経路なし（本 probe 未実施） | 必要なら描画観測経路の拡張を別 Issue で |

## Related Experiments

### Adopted

<!-- 実験ログなし -->

### Rejected

<!-- なし -->

## Validation

`git fetch origin main && git merge origin/main` → Already up to date
（`57e4e8ce`）。lint 修正後の最終コードで全ゲートを再実行:

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | clean |
| `mise run check` | 0 | `cargo check --workspace --all-targets` clean |
| `mise run lint-ui-gpui` | 0 | clippy `--features gpui -- -D warnings` clean（依存 `proc-macro-error2` の future-incompat note は既存・本変更と無関係） |
| `mise run test-ui-gpui` | 0 | 61 + 17 + 4 tests 全 pass |
| probe 既定モード（`wm-close.ps1` 駆動） | 0 | `[probe] PASS`（lint 修正後の最終バイナリで再実行・同一 trace） |
| probe `--degenerate-create` | 0 | 受理+clamp 分類（最終バイナリで再確認） |
| `real_window_lifecycle` / `minimal_window` | 0 | `[real] PASS` / clean exit（lint 修正は probe のみで driver 非依存のため再実行不要と判断 — probe のみ最終コードで再検証済み） |

補助証拠（canonical = mise ではない直接 cargo 実行）:
`cargo build -p ruviform-ui-gpui --features gpui --example window_lifecycle_probe`
→ exit 0。example 実行自体は mise task 未登録のため `wm-close.ps1` 経由の
直接実行（上表に記載）。

## Risks

- focus/幾何イベントはタイミング依存。probe は lifecycle kind と window id の
  対応を正本とし、checkpoint で ~5s の settle を置いた。それでも OS の
  focus 奪取（ユーザー操作・他アプリ）があれば `last_focus` 判定は変わり得る
  — 本実行では安定して PASS。
- `monitors()` の scale factor は driver 既知の近似（生存 window の観測値）。
  本環境は全モニタ 100% のため実害なし。異 DPI 環境での精度は未検証。
- degenerate の 0x0 clamp 値（120x1）は OS/GPUI 版依存の実測値。

## Follow-ups

- macOS/Linux での同等 probe 実行（同じ example がそのまま使える）。
- 複数 DPI 環境での `ScaleFactorChanged`・モニタ切断の実測。
- `post_event` 実窓配送の観測経路（描画 or ログ）の検討。
- ~~docs 生成 index（`docs/*/README.md`）と `docs/impl/ir/README.md` の
  再生成・追記は dispatch の編集禁止範囲のため未実施 — Coordinator/後続へ~~
  **実施済み（2026-11-11）**: closeout slice
  （`docs/impl/ir/issue-243-multiwindow-closeout-slice.md`）で #243 関連
  doc status 更新と管理 index 再生成を実施。
