# Issue #262 GPUI Native First 採用 Gate — 評価 report

- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/262
- 対象 Repo: `scarecrow173/ruviform`（worktree `G:\orca\workspaces\ruviform\rf262-verdict`、branch `scarecrow173/rf262-verdict`）
- 証拠 HEAD SHA: `7aab630207e96863c31203629774364df87aa543`（origin/main HEAD、修正 PR #626 マージ済み）
- 判定正本: `evidence/rf262-gpui-verdict/verdict-matrix.md`
- 本 report の verdict は**推奨**であり決裁ではない（決裁・Issue 投稿・ADR status は Coordinator）。

## 1. 証拠収集方法

### 1.1 CI run 証拠（ancestor SHA）

| Run | URL | SHA | 結果 |
| --- | --- | --- | --- |
| 43428172820 | https://github.com/scarecrow173/ruviform/actions/runs/43428172820 | `e93c438adc4af5b498e10926fe859ac1338ac390` | 全 lane success（windows/macos/linux の ui-gpui-verify・ui-verify・rt-verify・verify 等 19 leg）。checkout SHA artifact で実測 SHA 一致を確認済み（rf388 report の手法） |
| 43439434737 | https://github.com/scarecrow173/ruviform/actions/runs/43439434737 | `365c74ac4b8e696044fb9bae386dfd03f86ecf32` | 全 20 job success |
| 43540652567 | https://github.com/scarecrow173/ruviform/actions/runs/43540652567 | `2613b2388e0413af15913032421c57857e393682` | `verify (ubuntu-latest)` FAILED（`e2e_disable_env_kills_active_transport`）+ `ci-summary` failed。`daily-ci` run 43540630514（同 SHA）は success。修正 PR #626 = HEAD `7aab6302` |
| HEAD `7aab6302` | — | `7aab630207e96863c31203629774364df87aa543` | CI run 未実施（merge 後対象 SHA CI 証拠待ち、POLICIES §5） |

`consumer-verify.yml` は workflow_dispatch 専用。最新 run 43347745016 は別 SHA `3692c6fa` であり、対象 SHA の run は存在しない（Gate E = NotRun）。

### 1.2 先行 evidence/（ancestor SHA の実機 PoC）

| 証拠 | 内容 | 判定への寄与 |
| --- | --- | --- |
| `evidence/rf388-os-l1-ci/report.md` + `handoff-262-419.md` | run 43428172820 の 3 OS L1 CI 照合。OS-L1 Pass は実 Native Dock を証明しない旨明記 | Gate A の主要証拠 |
| `evidence/rf388-389-native-dock/report.md` | Windows 11・GPUI 0.2.2・RTX 4070 SUPER。in-process probe attach HWND 成功。Dock PASS は NotRun | Gate D 機構層 |
| `evidence/rf388-native-dock-windowing/report.md` | 33 checks pass（clip/focus/key/detach/reattach/lifecycle/event ordering）。native surface 画素 `Unsupported（実測）` — GPUI 0.2.2 `directx_renderer.rs:793` `CreateTargetForHwnd(topmost=true)` が child HWND を覆う。`WS_EX_NOREDIRECTIONBITMAP` で GDI fallback なし | Gate D 画素層 Fail + #532 根拠 |
| `evidence/rf388-native-dock-layout/report.md` | tab visibility/resize/lifecycle の先行証拠。DComp 診断で解釈が supersede。Dock PASS 不付与 | Gate D（superseded 解釈） |
| `evidence/rf389-vst3-editor/report.md` | 外部 `C:/Program Files/Common Files/VST3/iZotope/Vinyl.vst3` 1.12.0（sha256 `4bfe8ba1453fe6875c518327436c67e4d629a27f1f7384c01714664f8a5b1e92`）。実 `IPlugView` attach/detach 成功。Dock layout/clip/tab・keyboard/IME routing NotRun。CLAP/AU・macOS/Linux・L1 cross-process Unsupported/NotRun | Gate D VST3 L0 |
| `evidence/rf389-session-hardening/report.md` | session/lease/slot event ordering | Gate D 機構層補強 |
| `evidence/rf313-os-capability-matrix/report.md` | Windows SendInput keyboard/pointer smoke Pass。macOS/Linux・IME・a11y・DPI 変化・monitor disconnect・2 window NotRun。環境診断で SendInput が現行ホストで無効化（legacy injection は動作）— 環境制限であり製品 Pass ではない | Gate B |
| `evidence/rf243/window-job-headless-verification.md` + `evidence/issue-243-multiwindow-verify/` | window/job headless verify・multiwindow verdict=pass | Gate B window/2window |
| `evidence/rf465/README.md` + `inventory.json` | Legacy WebView inventory。両 consumer の ruviform pin `fd1d4d5f`、AL gpui/webview lane 並存、SV webview は mise task のみ | Gate E L1 傍証・#465 境界 |
| `docs/impl/exp/issue-532-dcomp-audit.md` | #532 DComp topmost 未解消の監査 | Gate D Blocker |

先行 PoC 証拠の実行 SHA は HEAD の ancestor であることを `git merge-base` で確認済み。差分は契約層に影響しない範囲であり、「ancestor SHA の証拠」として扱う（最新 SHA の Pass に読み替えない）。

### 1.3 ローカル lane 再実走（HEAD `7aab6302`、Windows 11）

証拠再実走として正規 lane を HEAD 上で実行。結果は `evidence/rf262-gpui-verdict/verification.md` に記録。

- `mise run test-ui-scenario-contract`: fake 16 passed + gpui 7 passed
- `mise run check-ui-driver-boundary`: `gui-boundary-guard --offline` verdict: pass
- `mise run test-ui-native-slot`: 13 passed
- `mise run test-ui-parity`: 14 passed
- `mise run test-ui-gpui`: lib 77 + windowed_test_app bin（compile/link）+ consumer_observation 22 + golden_capture 1 passed/3 ignored + lab_story 4 + scenario_contract 7 + windowed_uitest 2 — 全 pass

### 1.4 宣言的 Capability

`crates/ruviform-ui-gpui/src/lib.rs` `WindowCapabilities`（349-364 付近）:
- `resize`/`focus`/`scale_factor`/`multi_window`: Supported
- `ime`: **Unsupported**（Issue #389 owns contract、未配線）
- `native_surface_slot`: `surface::BACKEND_AVAILABLE` 時のみ Supported。`BACKEND_AVAILABLE = cfg!(target_os = "windows")`（`gpui_driver/surface.rs:38`）により **Windows のみ**、macOS/Linux は Unsupported 宣言

## 2. Gate 別判定まとめ

### Gate A — L1 契約基盤: **Pass（L1 契約層）**

Driver-neutral 境界・semantic fixture・compile/link・RT 非干渉は 3 OS の CI（ancestor SHA）と HEAD ローカル再実走で裏付け。headless/契約層の Pass であり実 GUI・実 plugin の Pass ではない。

### Gate B — Windows L2: **能力別に部分実証**

- Pass: window lifecycle・OS input（SendInput keyboard/pointer→intent）・focus/key 配送・2 window（過去実測）・DPI scale_factor（単一環境）
- Unsupported（宣言）: IME・focus release
- NotRun: a11y・DPI 動的変化・物理入力・macOS/Linux 全項目（Deferred）
- 利用機能が確定するまで全能力を必須化しない（POLICIES §2）。

### Gate C — 正しさ・性能: **semantic Pass / 性能 NotRun**

semantic 正しさは headless 契約層で Pass。visual/pixel は headless RGBA8 + Xvfb+lavapipe 限定環境で Pass。実 Native surface 画素は Fail（#532）。性能次元（FPS/frame-time/CPU/GPU/memory/latency vs baseline）は定量証拠なしで NotRun。

### Gate D — 実 Native Dock: **未達・無条件本番採用不可**

- 機構層（HWND parentage/geometry/clip/focus/key/redock/detach/reattach）: Pass（33 checks、実 Vinyl VST3）
- 画素層: **Fail** — GPUI 0.2.2 DComp `topmost=true` が child HWND を覆い plugin 画素非合成。`WS_EX_NOREDIRECTIONBITMAP` で GDI fallback なし。対照実験で plugin 固有でなく GPUI window 配下の全 child HWND が非合成
- Dock PASS（完了条件全体）: 各 report が「不成立/Partial」「Dock PASS 不付与」と明記
- VST3 L0 attach/detach: Pass。CLAP/AU・L1 cross-process: Unsupported。macOS/Linux backend: Unsupported+NotRun。他社 plugin 広範・副作用系: NotRun
- **#532 が open Blocker**。当該能力は採用済みとせず条件付き/未受入を明記。

### Gate E — Consumer Oracle: **NotRun**

consumer-verify 対象 SHA run なし。AL #166/SV #176 が製品 Owner。rf465 inventory の L1 互換記録は傍証であり製品 Oracle の Pass ではない。

## 3. 推奨 verdict

| 領域 | 推奨判定 |
| --- | --- |
| GPUI Native First を標準 Driver とする設計方針 | 維持 |
| OS-L1 | 条件付き採用可（ancestor SHA CI 証拠。HEAD SHA run 未実施） |
| Windows L2 | 能力別に部分実証（IME Unsupported・a11y/DPI/物理入力 NotRun） |
| Windows L3 / Gate D | 未達・無条件本番採用不可（#532） |
| macOS/Linux L2/L3 | Deferred（native_surface_slot Unsupported 宣言） |
| Gate C 性能 | NotRun |
| Gate E | NotRun |

Issue #262 Verdict 規則「全必須 OS/format で D・E が未達なら GPUI 標準 Driver 方針維持、Native Dock の無条件本番採用不可と明記し #419 を accepted にしない」に該当。

## 4. 既知ギャップ・未解決事項

1. **#532 DComp topmost 未解消** — Windows Native Dock 画素の根本 Blocker。対策決裁・実装が前提。
2. **HEAD `7aab6302` の CI run 未実施** — 統合受入は merge 後対象 SHA の CI 証拠待ち（POLICIES §5）。
3. **run 43540652567 の `verify (ubuntu-latest)` Fail** — `e2e_disable_env_kills_active_transport`。修正 PR #626 が HEAD にマージ済みだが同 SHA CI run 未実施。本 slice 範囲外（Issue #625 tracker、#626 で対応済み）。
4. **Gate C 性能未測定** — 性能予算 Issue 未確定。
5. **Gate E Consumer Oracle 未実施** — consumer-verify 対象 SHA run なし。AL #166/SV #176 が Owner。
6. **Windows L2 残能力** — IME（Unsupported 宣言）・a11y・DPI 動的・物理入力・plugin 主導 resize・DPI/マルチモニタ・2 window・crash/hang が NotRun。製品利用機能確定待ち。
7. **macOS/Linux `native_surface_slot` backend 未実装** — `Unsupported` 宣言（Capability 欠落として追跡）。L2/L3 は利用要件発生まで Deferred。
8. **SendInput 環境制限** — 現行ホストで SendInput が無効化された記録あり（`environment-blocked-sendinput.md`）。環境要因であり製品 Pass ではない。影響する再実行は環境依存。

## 5. Review status / remaining blockers

- 文書チェーン・証拠の review-protocol 結果は IR/PR に記録。
- 残 Blocker: #532（Native Dock 画素）、HEAD SHA CI 未実施、Gate C/E 未実施 — いずれも本 slice 範囲外の後続事項。
