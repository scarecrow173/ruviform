# Issue #262 GPUI Native First 採用 Gate — verdict matrix（正本）

- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/262
- 対象 Repo: `scarecrow173/ruviform`
- 証拠 SHA:
  - CI 証拠（ancestor SHA）: `e93c438adc4af5b498e10926fe859ac1338ac390`（run 43428172820）、`365c74ac4b8e696044fb9bae386dfd03f86ecf32`（run 43439434737）、`2613b2388e0413af15913032421c57857e393682`（run 43540652567, verify ubuntu FAILED）
  - 本 slice ローカル再実走: `7aab630207e96863c31203629774364df87aa543`（worktree HEAD = origin/main HEAD、Windows 11）
  - 先行 PoC 証拠: 各 report 記載の実行 SHA（HEAD の ancestor であることを `git merge-base` で確認）
- 判定語彙（POLICIES.md §3）: 要求 `Required`/`Deferred`/`NotApplicable`、結果 `Pass`/`Fail`/`NotRun`/`Unsupported`。別軸で管理。
- verdict は**推奨**であり決裁ではない。決裁宣言・Issue 投稿・ADR status 変更は Coordinator 権限。

凡例: requirement/result を分離。`Blocked` は open 前提 Issue による実行不能の併記（#532）。

## Gate A — L1 依存非漏出・Fake/GPUI semantic fixture（3 OS）

| CaseID | OS/Arch | Driver/Feature | Expected | Observed | Req | Result | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| GA-A1 driver-boundary | win/mac/linux x64 | `check-ui-driver-boundary`（gui-boundary-guard --offline） | 依存非漏出 verdict=pass | run 43428172820/43439434737 全 3 OS success。HEAD 再実走 `verdict: pass` | Required | **Pass** | ci run 43428172820・43439434737; evidence/rf262-gpui-verdict/verification.md |
| GA-A2 driver-contract | win/mac/linux | `test-ui-driver-contract`（UI-DVR-01/04/05/06/07/08） | 全 fixture Pass | run 43428172820 全 3 OS success（run log で個別 test 確認済み） | Required | **Pass** | evidence/rf388-os-l1-ci/report.md; run 43428172820 |
| GA-A3 scenario-contract fake | win | `test-ui-scenario-contract` fake lane（16 tests） | semantic scenario Pass | HEAD 再実走 16 passed。CI run でも success | Required | **Pass** | verification.md; run 43428172820 |
| GA-A4 scenario-contract gpui | win/mac/linux | `test-ui-scenario-contract` gpui lane（7 tests） | 同上 | HEAD 再実走 7 passed。CI run 43428172820 全 3 OS success | Required | **Pass** | verification.md; run 43428172820 |
| GA-A5 parity | win/mac/linux | `test-ui-parity` + `test-ui-gpui` parity tests | Fake/GPUI semantic parity | HEAD 再実走 parity 14 passed + gpui lib 77 passed。CI 全 3 OS success | Required | **Pass** | verification.md; run 43428172820 |
| GA-A6 native-slot contract | win/mac/linux | `test-ui-native-slot`（NSC-01〜13） | slot 契約 Pass | HEAD 再実走 13 passed。CI 全 3 OS success | Required | **Pass** | verification.md; run 43428172820 |
| GA-A7 component-view | win/mac/linux | `test-ui-component-view` | component semantic Pass | CI run 43428172820/43439434737 全 3 OS success | Required | **Pass** | evidence/rf388-os-l1-ci/report.md |
| GA-A8 gpui compile/link | win/mac/linux | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`（opt-in `gpui`/`gpui-test`/`uitest`、`windowed_test_app` bin link） | compile/link/test Pass | CI `ui-gpui-verify` lane 全 3 OS success（ubuntu lib55+win lib77+macos lib55、bin link 含む 0 failed）。HEAD 再実走 `test-ui-gpui` 全 target pass | Required | **Pass** | run 43428172820; verification.md |
| GA-A9 RT 非干渉 | win/mac/linux | `test-rt`（rt_* 166 tests）+ verify lane | RT regression なし | CI run 43428172820 `rt-verify`/`verify` success | Required | **Pass** | evidence/rf388-os-l1-ci/report.md |
| GA-A10 HEAD SHA CI | win/mac/linux | HEAD `7aab6302` の Full CI run | merge 後対象 SHA CI 証拠 | HEAD に対する CI run 未実施（POLICIES §5 統合受入は merge 後対象 SHA の CI 証拠） | Required | **NotRun**（Pending） | —（merge 後 CI 待ち） |
| GA-A11 CI Fail 記録 | linux | run 43540652567 `verify (ubuntu-latest)` | — | `e2e_disable_env_kills_active_transport` FAILED。修正 PR #626 = HEAD `7aab6302`。HEAD SHA run 未実施 | Required | **Fail**（当該 run）/ HEAD 側 **NotRun** | run 43540652567; Issue #625 tracker |

## Gate B — 実 OS Window/Input/Focus/IME/DPI/a11y/2Window（Windows L2 / macOS・Linux Deferred）

| CaseID | OS/Arch | Capability | Expected | Observed | Req | Result | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| GB-B1 window lifecycle | Windows 11 x64 | 実 GPUI window open/render/close | 実 OS window 動作 | rf313 real_os_smoke・rf243 window-job headless verify・issue-243 multiwindow verdict=pass | Required（利用機能） | **Pass** | evidence/rf313-os-capability-matrix/; evidence/rf243/; evidence/issue-243-multiwindow-verify/ |
| GB-B2 OS input keyboard/pointer | Windows 11 x64 | SendInput 等実入力 → intent 到達 | 実入力で intent 到達 | rf313: Tab/arrow commit・Fader drag・Seek click が intent 到達（OS automation injection 経路、物理手動ではない） | Required（利用機能） | **Pass** | evidence/rf313-os-capability-matrix/report.md |
| GB-B3 focus/key 配送 | Windows 11 x64 | container/plugin 間 focus・key boundary | focus 取得・key 到達 | rf388-native-dock-windowing: `focus_container`/`key_reaches_plugin`/`key_boundary_return` Pass。window focus release（`request_focus(window, focused:false)`）は gpui が focus release 非対応のため `Unsupported` を正直に返す（`real_window.rs` doc comment） | Required（利用機能） | **Pass（部分）** — focus release は別 Case | evidence/rf388-native-dock-windowing/report.md |
| GB-B4 focus release | Windows 11 x64 | window focus release | release 可能 | `request_focus(window, focused:false)` が `Unsupported` を返す（gpui が focus release 非対応、`real_window.rs` doc comment。issue-243 multiwindow JSON の `focus_release_unsupported` check でも確認） | Deferred（利用時 Required 化） | **Unsupported** | crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs; evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json |
| GB-B5 IME | Windows/mac/linux | IME composition/routing | IME 動作 | `WindowCapabilities.ime = Unsupported`（Issue #389 owns contract、未配線） | Deferred | **Unsupported** | crates/ruviform-ui-gpui/src/lib.rs:359 |
| GB-B6 DPI scale_factor | Windows 11 x64 | scale_factor 取得・動的変化 | scale 取得 + 動的変化 | `scale_factor: Supported` 宣言。実測は scale 1.0 単一環境のみ。動的 DPI 変化・HiDPI 複数 scale 未実測 | Required（宣言+単一環境）/ 動的変化 Deferred | **Pass（単一環境）** / **NotRun（動的変化）** | evidence/rf313-os-capability-matrix/ |
| GB-B7 a11y | Windows/mac/linux | accessibility | a11y 動作 | 実測証拠なし | Deferred | **NotRun** | — |
| GB-B8 2 window | Windows 11 x64 | multiwindow smoke | 2 window lifecycle | issue-243 verdict-multiwindow-windows.json verdict=pass（Win+Arrow/Alt+F4 chord）。ただし `environment-blocked-sendinput.md` に SendInput 環境無効化の記録あり（当時ホスト固有） | Required（利用機能） | **Pass（過去実測）** — 現行ホスト再現は環境依存 | evidence/issue-243-multiwindow-verify/ |
| GB-B9 macOS/Linux 全 L2 | macOS/Linux | Window/Input/Focus/IME/DPI/a11y/2Window | — | POLICIES §2 で Deferred。実機検証未実施 | Deferred | **NotRun**（Deferred、Pass にしない） | — |

## Gate C — Component/Viz 正しさ・負荷・性能

| CaseID | OS/Arch | 次元 | Expected | Observed | Req | Result | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| GC-C1 semantic 正しさ | win/mac/linux | parity/component-view/scenario/golden core | semantic Pass | GA-A3〜A7 と同一 lane で Pass | Required | **Pass**（headless 契約層） | run 43428172820; verification.md |
| GC-C2 visual/pixel headless | win/linux | `test-ui-golden` RGBA8 + `ui-gpui-offscreen-verify`（Xvfb+lavapipe 実 X11 window） | pixel 期待一致 | CI run 43428172820 `ui-gpui-offscreen-verify` success（Linux Xvfb+lavapipe 3 tests）。headless/限定環境 | Required | **Pass（headless/限定環境限定）** | run 43428172820 |
| GC-C3 実 Native surface 画素 | Windows | plugin child HWND 画素が GPUI に合成 | plugin 画素表示 | GPUI 0.2.2 DComp `topmost=true` で child HWND 非合成。plugin 画素観測不能 | Required | **Fail**（実測で非表示）— #532 Blocker | evidence/rf388-native-dock-windowing/report.md; docs/impl/exp/issue-532-dcomp-audit.md |
| GC-C4 性能（FPS/frame-time/CPU/GPU/memory/latency vs baseline） | win/mac/linux | 定量性能比較 | baseline 比較結果 | UI frame-time 性能の定量比較証拠なし。bench lane は DSP benchmark で UI 性能ではない | Deferred | **NotRun** | — |

## Gate D — 実 Plugin Native Dock（format×OS×L0/L1、EXT-L3）

| CaseID | OS/Format/Isolation | Expected | Observed | Req | Result | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| GD-D1 HWND 機構（attach/geometry/visibility/clip/focus/key/redock） | Windows/VST3/L0 | 機構層 Pass | rf388-native-dock-windowing 33 checks 全 pass。実外部 VST3 `Vinyl.vst3` 1.12.0（sha256 `4bfe8ba1…1b1e92`）。in-process probe attach HWND レベル成功 | Required | **Pass（機構層）** | evidence/rf388-native-dock-windowing/report.md; evidence/rf388-389-native-dock/report.md |
| GD-D2 native surface 画素表示 | Windows/VST3/L0 | plugin editor 画素が GPUI window 内に表示 | DComp `topmost=true`（`directx_renderer.rs:793` `CreateTargetForHwnd`）が child HWND を覆い非合成。`WS_EX_NOREDIRECTIONBITMAP` で GDI redirection fallback もなし。対照実験で plugin 固有でなく GPUI window 配下の全 child HWND が非合成 | Required | **Fail**（実測で非表示）+ **#532 Blocker（open）** | evidence/rf388-native-dock-windowing/report.md; docs/impl/exp/issue-532-dcomp-audit.md |
| GD-D3 Dock PASS（完了条件全体） | Windows/VST3/L0 | Native Dock 完了条件全体 Pass | rf388-native-dock-layout/windowing 両 report が「不成立（Partial）」「Dock PASS は付与しない」と明記 | Required | **未達** | evidence/rf388-native-dock-layout/report.md; evidence/rf388-native-dock-windowing/report.md |
| GD-D4 VST3 editor attach/detach（外部 plugin 実測） | Windows/VST3/L0 | 実 `IPlugView` attach/detach | rf389-vst3-editor: Vinyl 1.12.0 実 `IPlugView` attach/detach 成功。Dock layout/clip/tab・keyboard/IME routing は NotRun | Required | **Pass（attach/detach のみ）** / layout・IME **NotRun** | evidence/rf389-vst3-editor/report.md |
| GD-D5 CLAP editor | Windows/CLAP | CLAP editor attach | `CLAP/AU editor = Unsupported`（スコープ外維持） | Deferred | **Unsupported** | evidence/rf389-vst3-editor/report.md |
| GD-D6 AU editor | macOS/AU | AU editor attach | macOS 未実装 + AU 未対応 | Deferred | **Unsupported** + **NotRun** | evidence/rf389-vst3-editor/report.md |
| GD-D7 L1 cross-process editor | Windows/VST3/L1 | cross-process editor GUI | `L1 (cross-process) editor = Unsupported`（現行 L1 経路は GUI を Unsupported で拒否、L0 退化なし） | Deferred | **Unsupported** | evidence/rf389-vst3-editor/report.md |
| GD-D8 macOS/Linux native surface backend | macOS/Linux | NSView/X11/Wayland backend | `BACKEND_AVAILABLE = cfg!(windows)`。backend 未実装 | Deferred | **Unsupported**（宣言）+ L2/L3 **NotRun** | crates/ruviform-ui-gpui/src/gpui_driver/surface.rs:38 |
| GD-D9 他社 VST3 広範互換 | Windows/VST3/L0 | 複数 plugin で検証 | Vinyl 1 バイナリのみ検証 | Deferred | **NotRun** | evidence/rf389-vst3-editor/report.md |
| GD-D10 plugin 主導 resize・IME/popup・DPI/マルチモニタ・2 window・crash/hang・swapchain z-order | Windows/VST3/L0 | 各副作用の実測 | 各 report の NotRun 節に列挙 | Deferred | **NotRun** | evidence/rf388-native-dock-windowing/report.md; evidence/rf389-vst3-editor/report.md |

## Gate E — 両 Consumer 独立 Oracle（#294）

| CaseID | Repo/Consumer | Expected | Observed | Req | Result | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| GE-E1 consumer-verify 対象 SHA run | ruviform | 対象 SHA で consumer-verify 実行 | `consumer-verify.yml` は workflow_dispatch 専用。最新 run 43347745016 は別 SHA `3692c6fa`。対象 SHA（`e93c438a`/`365c74ac`/`7aab6302`）の run なし | Required（#294） | **NotRun** | .github/workflows/consumer-verify.yml |
| GE-E2 AudioLens 製品 GUI 受入 | audio-lens #166 | AL 製品 Native/UI 受入 | AL #166 が Owner。rf465 inventory で AL `al-desktop`(gpui) bin と webview lane が CI 並存を確認したが製品受入証拠は別 Issue | Deferred | **NotRun / Deferred** | evidence/rf465/README.md; AL #166 |
| GE-E3 Synvelis 製品 Native Dock 受入 | synvelis #176 | SV 製品 Native Plugin Dock 受入 | SV #176 が Owner。実 Plugin Dock は SV 側責務 | Deferred | **NotRun / Deferred** | SV #176 |
| GE-E4 既存 Consumer L1 互換（傍証） | ruviform+consumers | 既存 Consumer の L1 非回帰 | rf465 `inventory.json`: 両 consumer の ruviform pin `fd1d4d5f`、AL gpui/webview lane 並存、SV webview feature lane は mise task のみ残存 | Required（L1 傍証） | **部分的に記録済み**（L1 互換の傍証。製品 Oracle ではない） | evidence/rf465/README.md |

## 推奨 verdict（決裁は Coordinator）

| 領域 | 推奨判定 | 要点 |
| --- | --- | --- |
| GPUI Native First を標準 Driver とする設計方針 | **維持** | #262/#384 で決裁済み。Driver-neutral 公開契約は Gate A で裏付け |
| OS-L1（3 OS Build/Link・Headless・Feature・RT） | **条件付き採用可** | ancestor SHA の CI で全 lane success。HEAD `7aab6302` の CI run 未実施（統合受入は merge 後対象 SHA CI 証拠待ち） |
| Windows L2 | **能力別に部分実証** | Window/Input/Focus（key 配送）実測 Pass。IME=`Unsupported`、a11y/DPI 動的/物理入力=`NotRun`。focus release=`Unsupported` |
| Windows L3 / Gate D（Native Dock） | **未達・無条件本番採用不可** | HWND 機構 Pass だが DComp `topmost=true` で plugin 画素非表示（#532 open）。条件付き/未受入を明記 |
| macOS/Linux L2/L3 | **Deferred** | `native_surface_slot` backend は `Unsupported` 宣言（Capability 欠落追跡）。NotRun≠Pass |
| Gate C 性能 | **NotRun** | 定量比較証拠なし |
| Gate E | **NotRun** | consumer-verify 対象 SHA run なし。AL #166/SV #176 が Owner |

**Issue #262 Verdict 規則の適用**: 「全必須 OS/format で D・E が未達なら GPUI 標準 Driver 方針維持、Native Dock の無条件本番採用不可と明記し #419 を accepted にしない」に該当。#419 には「L1 のみ評価済み、L2/L3 非保証」を明記する。
