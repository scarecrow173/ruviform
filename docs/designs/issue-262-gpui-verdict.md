---
id: "DESIGN-034XzvxSlT8Mm4uX42xwFb"
type: "design"
status: "approved"
title: "GPUI Native First採用Gate verdict評価設計（Issue #262）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-262-gpui-verdict.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-262-gpui-verdict.md"
  implemented-by:
    - "docs/plans/issue-262-gpui-verdict.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/test-specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-465-legacy-webview-p0p1.md"
    - "docs/designs/ci-os-l1-gpui-macos.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-262-gpui-verdict.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-262-gpui-verdict.md"
  references:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "POLICIES.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf388-os-l1-ci/report.md"
    - "evidence/rf388-os-l1-ci/handoff-262-419.md"
    - "evidence/rf388-389-native-dock/report.md"
    - "evidence/rf388-native-dock-layout/report.md"
    - "evidence/rf388-native-dock-windowing/report.md"
    - "evidence/rf389-vst3-editor/report.md"
    - "evidence/rf313-os-capability-matrix/report.md"
    - "evidence/rf243/window-job-headless-verification.md"
    - "evidence/rf465/README.md"
    - "docs/impl/exp/issue-532-dcomp-audit.md"
  defers: []
  deferred-by: []
---

# GPUI Native First採用Gate verdict評価設計（Issue #262）

本書は Issue #262 の採用 Gate 評価の**方法論と判定構造**を定義する。個別 Case の expected/observed/verdict/証拠 URL の正本は `evidence/rf262-gpui-verdict/verdict-matrix.md`、証拠収集の記録は `evidence/rf262-gpui-verdict/report.md` とする。コード変更・CI 変更・ADR status 変更は含まない。

## Context

- Issue #262 の技術方針決裁（2026-11-10 節）は確定済み: 公開 UI 契約は Driver-neutral、標準実装 Driver は GPUI Native First、外部 gpui 型/OS handle/Toolkit API は Consumer 通常コードに漏らさない。Fake/GPUI の同一 semantic scenario を使う。**これは PoC 成功/3 OS 合格を意味せず、採用最終承認（#419 ADR status）は実測 Gate 待ち**。
- Gate 構成（Issue 本文）: ① A: dependency leak・Fake/GPUI semantic fixture（headless 先行可）、② B: Windows→macOS→Linux の Window/Focus/IME/DPI/a11y/2Window、③ C: Fader/Meter/Waveform 正しさ・負荷・性能（同条件 baseline と予算）、④ D: #388/#389/#532 実 Plugin native dock（format×OS×L0/L1 別、floating/overlay/Generic 不可）、⑤ E: #294 両 Consumer の個別 non-regression。開始を待ち合わせない。
- 判定表の必須項目（Issue 本文）: CaseID、OS/版、GPUI/Driver SHA、GPU/Device、plugin name/version/binary hash、process isolation、feature/capability、expected/observed、Pass/Fail/Unsupported/NotRun/Blocked、証拠 URL、再実行/rollback。
- Verdict 規則（Issue 本文）: 全必須 OS/format で D・E が未達なら「GPUI 標準 Driver 方針維持、Native Dock の無条件本番採用不可」と明記し #419 を accepted にしない。Worker は測定と推奨を行い、判定資料が揃うまで採用済みを自称しない。
- POLICIES.md §2/§3/§5: Windows は L1 必須 + 実際に提供・利用する能力の L2/L3 必須、macOS/Linux は L1 のみ必須（L2/L3 は利用要件発生まで Deferred）。要求状態（Required/Deferred/NotApplicable）と実行結果（Pass/Fail/NotRun/Unsupported）を分離。ローカル検証は変更範囲に比例、統合受入は merge 後対象 SHA の CI 証拠。

## Decision overview

**推奨 verdict（記録。決裁は Coordinator）**:

| 領域 | 推奨判定 | 根拠の骨子 |
| --- | --- | --- |
| GPUI Native First を標準 Driver とする設計方針 | **維持** | Issue #262/#384 で決裁済みの方針。Driver-neutral 公開契約は Gate A の証拠で裏付け済み |
| OS-L1（3 OS Native Build/Link・Headless contract・必要 Feature・RT 非干渉） | **条件付き採用可** | 3 OS の Full CI で全 lane success（run 43428172820 @ `e93c438a`、run 43439434737 @ `365c74ac`）。ただし最新 main SHA `7aab6302` の同一 SHA CI run は未実施（POLICIES §5: 統合受入は merge 後対象 SHA の CI 証拠） |
| Windows L2（実 Window/Input/Focus 等の使用能力） | **部分的に実証済み・能力別に未受入あり** | 実 OS 入力（SendInput keyboard/pointer）・2 window lifecycle・focus の実測 Pass あり（rf313/rf243/issue-431）。IME は driver が `Unsupported` 宣言、DPI 動的変化・a11y・物理入力は NotRun。利用機能が確定するまで全能力を必須化しない |
| Windows L3 / Gate D（Native Plugin Dock） | **未達・無条件本番採用不可** | HWND 機構は実測 Pass だが、GPUI DComp `topmost=true` による plugin 画素の非表示が #532 で未解決。`Native Dock PASS` は各 report が「不成立/Partial」と明記 |
| macOS/Linux L2/L3 | **Deferred（NotRun≠Pass）** | POLICIES §2 どおり利用要件発生まで Deferred。`native_surface_slot` backend は Windows のみ実装で macOS/Linux は `Unsupported` 宣言（Capability 欠落として追跡） |
| Gate C 性能次元 | **NotRun** | FPS/frame-time/CPU/GPU/memory/latency の baseline 比較は未測定 |
| Gate E（両 Consumer Oracle） | **NotRun** | `consumer-verify.yml` は workflow_dispatch 専用で対象 SHA run なし。AL #166/SV #176 が製品 Owner |

## D1. 判定方法論

### D1.1 証拠の階層

| 階層 | 内容 | 本 slice での扱い |
| --- | --- | --- |
| CI run 証拠 | `ci.yml` workflow_dispatch / daily-ci schedule の対象 SHA 固定 run | run URL・job 一覧・checkout SHA artifact を照合。`e93c438a`（run 43428172820）と `365c74ac`（run 43439434737）は全 job success。`2613b238`（run 43540652567）は `verify (ubuntu-latest)` Fail を正直に記録 |
| 先行 evidence/ | `evidence/rf*` の実機 PoC report | 実測値・判定語彙・NotRun/Unsupported の明記をそのまま転記。Pass の昇格はしない |
| ローカル lane 再実走 | 本 slice で HEAD `7aab6302` 上で実行した `mise run test-ui-*` / `check-ui-driver-boundary` | 同一 SHA 補強証拠。`verification.md` に実コマンド・exit・結果を記録 |
| 宣言的 Capability | `WindowCapabilities`/`SlotCapabilities` の Supported/Unsupported 宣言 | `ime: Unsupported`（全 OS）、`native_surface_slot: Supported`（Windows のみ、`BACKEND_AVAILABLE = cfg!(target_os = "windows")`）を Capability 欠落として追跡 |

### D1.2 SHA 照合規則

- CI 証拠の対象 SHA は checkout artifact の実測 SHA と一致することを確認する（rf388 report が全 19 leg で実施済みの手法を踏襲）。
- 本 slice のローカル再実走は worktree HEAD `7aab630207e96863c31203629774364df87aa543`（= origin/main HEAD）上で実施し、証拠 SHA として記録する。
- 先行 PoC 証拠（rf388-*/rf389-*/rf313/rf243）はそれぞれの実行 SHA が HEAD の ancestor であることを `git merge-base` で確認し、差分が契約層に影響しないことを確認した上で「ancestor SHA の証拠」として扱う（rf532 監査と同じ手法）。
- HEAD に対する CI run が存在しない間、「最新 main SHA の統合受入」は `NotRun`（Pending 相当）として記録し、Pass にしない（POLICIES §5）。

### D1.3 verdict 語彙の適用規則

- `Pass`: 対象 Case が定義された Oracle（test 成功・実測一致・CI job 成功 + 個別 test 確認）で観測された。
- `Fail`: 実行したが期待と不一致（例: run 43540652567 の `e2e_disable_env_kills_active_transport`）。
- `NotRun`: 未実施（CI lane なし・実機なし・Feature 無効・Deferred）。
- `Unsupported`: 実装/環境が明示的に非対応を宣言（例: `ime: Unsupported`、macOS/Linux `native_surface_slot` backend 未実装、L1 editor ops Unsupported）。
- `Blocked`: 前提 Issue/環境が未解決で実行不能（本 slice では #532 による Dock 画素 Case を「Blocked 相当の未達」として扱うが、verdict 語彙上は `Fail`（実測で非表示を確認）+ 要件上の未受入として記録する。#532 自体は open の Blocker として併記）。

## D2. Gate A — 依存非漏出・Fake/GPUI semantic fixture（L1 契約基盤）

評価対象: 公開 UI 契約の Driver-neutral 性、Fake/GPUI 両 adapter の同一 semantic scenario、依存方向ガード。

| Case 群 | lane（mise task / CI job） | 証拠 |
| --- | --- | --- |
| 依存非漏出（dep-graph + public signature 検査） | `check-ui-driver-boundary`（`gui-boundary-guard --offline`）in `ui-gpui-verify` ×3 OS | run 43428172820（3 OS success）・run 43439434737（3 OS success）・本 slice ローカル再実走 |
| Driver contract fixture（UI-DVR-01/04/05/06/07/08） | `test-ui-driver-contract` | 同上 + run log で `7 passed` 確認済み（rf388 report） |
| Fake/GPUI parity（UI-DVR-02/03 + payload retention） | `test-ui-parity` + `test-ui-gpui` 内 GPUI 側 | 同上 |
| Consumer-neutral Scenario Contract（#464） | `test-ui-scenario-contract`（fake 16 + gpui 7） | 同上 |
| NativeSurfaceSlot 契約（NSC-01〜13） | `test-ui-native-slot` | 同上（`13 passed` を 3 OS で確認済み） |
| Component/View semantic（#387） | `test-ui-component-view` | 同上 |
| GPUI feature Compile/Link（`gpui`/`gpui-test`/`uitest`、`windowed_test_app` bin link 含む） | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` | 同上（ubuntu lib 55 + windows lib 77 + macos lib 55、`windowed_test_app` link 含む `0 failed`） |
| RT 非干渉 | `test-rt`（`cargo test --workspace rt_`、166 tests）+ `test-full` 内 `rt_*` | run 43428172820 `rt-verify`/`verify` success |

**Gate A 評価: Pass（L1 契約層）。** Driver-neutral 境界・semantic fixture・compile/link は 3 OS の CI とローカル再実走で裏付け。ただしこれは headless/契約層の Pass であり、実 GUI・実 plugin の Pass ではない（rf388 report と同じ限定を維持）。

## D3. Gate B — 実 OS Window/Input/Focus/IME/DPI/a11y/2Window

評価対象: Windows で実際に提供・利用する GUI 能力の L2。POLICIES §2 により Windows のみ Required（利用機能に限定）、macOS/Linux は Deferred。

| 能力 | Case | 証拠 | verdict |
| --- | --- | --- | --- |
| Window open/render/close（実 GPUI window） | real_os_smoke・real_window_lifecycle・multiwindow smoke | `evidence/rf313-os-capability-matrix/`（SendInput keyboard/pointer Pass）、`evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json`（verdict=pass（checks 記録あり））、`evidence/rf243/` | **Pass**（Windows、OS automation injection 経路。物理手動入力ではない点を明記済み） |
| Input（keyboard/pointer → intent 到達） | `verdict-windows.json`・`verdict-pointer-windows.json` | rf313 report: Tab/arrow commit・Fader drag・Seek click が実 OS 入力経路で intent 到達 | **Pass**（Windows） |
| Focus | container focus・window focus | rf388-native-dock-windowing: `focus_container`/`key_reaches_plugin`/`key_boundary_return` Pass。window focus release（`request_focus(window, focused:false)`）は gpui が focus release 非対応のため `Unsupported` を正直に返す（`real_window.rs` の doc comment に明記） | **Pass（部分）** — focus 取得・container/plugin 間 key 配送は実測。focus release は `Unsupported` 宣言 |
| IME | `ime` capability | `WindowCapabilities.ime = Unsupported`（lib.rs:359、Issue #389 owns contract） | **Unsupported**（宣言済み技術的非対応。NotRun ではない） |
| DPI（scale_factor/動的変化） | `scale_factor` capability + 実測 | `scale_factor: Supported` 宣言。実測は scale 1.0 環境のみ（rf313: 1920×1080×2 monitor scale 1.0）。動的 DPI 変化・HiDPI 複数 scale は NotRun | **Pass（宣言+単一環境実測）/ NotRun（動的変化）** |
| a11y | — | 実測証拠なし | **NotRun** |
| 2 Window | multiwindow smoke | `verdict-multiwindow-windows.json` verdict=pass（Win+Arrow/Alt+F4 chord 含む）。ただし `environment-blocked-sendinput.md` に SendInput が環境要因で無効化された再実行不能の記録あり（当時のホスト固有事象） | **Pass（過去実測）** — 現行ホストでの再現は環境依存 |
| macOS/Linux 全項目 | — | POLICIES §2 で Deferred | **Deferred / NotRun** |

**Gate B 評価: Windows は「利用機能」の範囲で部分的実証。** Window/Input/Focus（key 配送まで）は実測 Pass。IME は `Unsupported`、a11y・動的 DPI・物理入力は NotRun。製品がこれらを利用する場合は個別に Required 化が必要（Deferred 記録）。macOS/Linux は Deferred であり NotRun を Pass にしない。

## D4. Gate C — Component/Viz 正しさ・負荷・性能

評価対象: Fader/Meter/Waveform 等の semantic 正しさ・visual/semantic parity・性能（FPS/frame-time/CPU/GPU/memory/latency）。

| 次元 | 証拠 | verdict |
| --- | --- | --- |
| semantic 正しさ（parity/component-view/scenario/golden core） | Gate A と同一 lane 群（`test-ui-parity`/`test-ui-component-view`/`test-ui-scenario-contract`/`test-ui-golden`） | **Pass**（headless 契約層） |
| visual/pixel | `test-ui-golden`（headless RGBA8 normalization）+ `ui-gpui-offscreen-verify`（Linux Xvfb+lavapipe 実 X11 window pixel 3 tests、run 43428172820） | **Pass（headless 環境限定）** — 実 desktop GPU での画素比較ではない |
| 実 Native surface 画素 | rf388-native-dock-windowing: GPUI 自身の UI は描画されるが plugin child HWND は DComp で非表示（`Unsupported（実測）`） | **Fail 相当の未達**（#532） |
| 性能（FPS/frame-time/CPU/GPU/memory/latency vs baseline） | bench lane は DSP benchmark であり UI frame-time ではない。UI 性能の定量比較証拠なし | **NotRun** |

**Gate C 評価: semantic 層は Pass、性能次元は NotRun。** 「同条件 baseline と予算での比較」は未実施のため、性能面の採用根拠は存在しない。これは #419 で「性能非保証」として明記すべき範囲。

## D5. Gate D — 実 Plugin Native Dock（format×OS×L0/L1）

評価対象: 実プラグインのネイティブ GUI をメイン GPUI Dock 内に収容する実証。floating/overlay/Generic は代替不可。

| Case | 証拠 | verdict |
| --- | --- | --- |
| Win/VST3/L0 attach・geometry・visibility・clip・focus・key・re-dock（HWND 機構） | `evidence/rf388-native-dock-windowing/report.md`: 33 checks 全 pass（attach_parented/clip_*/key_*/redock_*）。実外部 VST3 `Vinyl.vst3` 1.12.0（sha256 記録済み） | **Pass（機構層）** |
| Win/VST3/L0 native surface 画素表示 | 同 report「重大な発見」: GPUI 0.2.2 `directx_renderer.rs:793` `CreateTargetForHwnd(topmost=true)` が child HWND を覆う。plugin 画素は観測不能、`Unsupported（実測）`。対照実験で plugin 固有でなく GPUI window 配下の全 child HWND が非合成 | **Fail（実測で非表示）→ #532 未解決** |
| Win/VST3/L0 Dock PASS（完了条件全体） | rf388-native-dock-layout/windowing 両 report が「不成立（Partial）/ Dock PASS は付与しない」と明記 | **未達** |
| Win/VST3/L1（cross-process editor） | rf389-vst3-editor: `L1 (cross-process) editor = Unsupported`（現行 L1 経路は GUI を Unsupported で拒否、L0 退化なし） | **Unsupported（宣言）** |
| Win/CLAP・AU editor | rf389-vst3-editor: `CLAP/AU editor = Unsupported`（スコープ外維持） | **Unsupported / NotRun** |
| macOS/Linux 全行（NSView/X11/Wayland backend） | `BACKEND_AVAILABLE = cfg!(windows)`。rf388-389 report: macOS/Linux = Unsupported(backend 未実装)/NotRun | **Unsupported（宣言）+ NotRun** |
| 他社 VST3 広範互換 | Vinyl 1 バイナリのみ検証 | **NotRun** |
| plugin 主導 resize・IME/popup・DPI/マルチモニタ・2 window・crash/hang・swapchain z-order | 各 report の NotRun 節 | **NotRun** |

**Gate D 評価: 未達。** Issue の Verdict 規則により「GPUI 標準 Driver 方針維持、Native Dock の無条件本番採用不可」に該当。#532 が open で DComp 画素問題が未解決のため、当該能力は採用済みとせず**条件付き/未受入**を明記する。HWND 機構の Pass を Dock Pass に昇格させない。

## D6. Gate E — 両 Consumer 独立 Oracle

評価対象: Synvelis（#176 製品 Native Dock）・AudioLens（#166）の個別 non-regression/製品受入。

| Case | 証拠 | verdict |
| --- | --- | --- |
| `consumer-verify.yml` 対象 SHA run | workflow_dispatch 専用。最新 run 43347745016 は別 SHA `3692c6fa`。対象 SHA（`e93c438a`/`365c74ac`/`7aab6302`）の run なし | **NotRun** |
| AudioLens 製品 GUI 受入 | AL #166 が Owner。rf465 inventory で AL の webview/gpui lane 並存を確認済みだが製品受入証拠は別 Issue | **NotRun / Deferred** |
| Synvelis 製品 Native Dock 受入 | SV #176 が Owner。実 Plugin Dock は SV 側責務 | **NotRun / Deferred** |
| 既存 Consumer 互換（L1 レベル） | rf465 `inventory.json`: 両 consumer の ruviform pin `fd1d4d5f`、AL `al-desktop`(gpui) bin と webview lane が CI で並存、SV webview feature lane は mise task のみ残存 | **部分的に記録済み**（L1 互換の傍証。製品 Oracle ではない） |

**Gate E 評価: NotRun。** 両 Consumer の独立 Oracle は未実施。#294 が統合責務を持つ。

## D7. 推奨 verdict と引き渡し境界

### D7.1 推奨 verdict（Coordinator 決裁用の記録）

1. **GPUI Native First を標準 Driver とする設計方針: 維持。** 公開契約の Driver-neutral 性は Gate A で裏付け済み。
2. **OS-L1: 条件付き採用可。** 3 OS の Native Build/Link・Headless contract・必要 Feature・RT 非干渉は CI 証拠で Pass。ただし対象 SHA の CI 証拠は ancestor SHA（`e93c438a`/`365c74ac`）に基づき、最新 main `7aab6302` の run は未実施 — POLICIES §5 により統合受入は merge 後対象 SHA の CI 証拠を要する。
3. **Windows L2: 能力別。** Window/Input/Focus（key 配送）は実測 Pass。IME=`Unsupported`、a11y/DPI 動的/物理入力=`NotRun`。製品が利用する能力が確定次第個別 Required 化。
4. **Windows L3 / Gate D（Native Dock）: 未達・無条件本番採用不可。** #532 DComp 未解消。条件付き/未受入を明記。
5. **macOS/Linux L2/L3: Deferred。** `native_surface_slot` は `Unsupported` 宣言（Capability 欠落として追跡）。
6. **Gate C 性能・Gate E Consumer Oracle: NotRun。** 「L1 のみ評価済み、性能・実 GUI・実 Plugin・Consumer 製品受入は非保証」として #419 に引き渡す。

### D7.2 #419 への引き渡し内容（`evidence/rf262-gpui-verdict/handoff-419.md`）

- 「L1 のみ評価済み、L2/L3 非保証」を ADR に明記する要求
- Gate D 未達による「Native Dock 無条件本番採用不可」の条件付き判定材料
- macOS/Linux の `native_surface_slot` `Unsupported` を Supported/Unsupported/NotRun の OS 別記録として反映する要求
- #465 への境界: Legacy 撤去は L1 機能/既存 Consumer 互換（rf465 inventory の `reuse`/`provisional` 分類）と未実 GUI 製品機能の破壊を独立判断。ADR の L1 採用だけで製品 GUI 機能や実 Plugin 表示を保証しない

### D7.3 本 slice が行わないこと

- verdict の決裁宣言（accepted/conditional/rejected の選択）— Coordinator 権限
- Issue コメント投稿・Close 判定・ADR status 変更
- #532/#388/#389 の実装修正、#465 の撤去実施

## Risks and Trade-offs

| リスク | 影響 | 対応 |
| --- | --- | --- |
| ancestor SHA の CI 証拠を最新 SHA の Pass と誤認 | 統合受入の偽装 | D1.2 の SHA 照合規則と「HEAD run 未実施 = NotRun」の明記。POLICIES §5 に整合 |
| HWND 機構 Pass を Dock Pass に昇格 | Gate D の過大評価 | 各 report の「Dock PASS 不成立」明記をそのまま転記。verdict-matrix で機構層と画素層を別 Case に分離 |
| `Unsupported`（宣言）と `NotRun`（未実施）の混同 | Capability 欠落の見落とし | D1.3 の語彙規則。`ime`/`native_surface_slot`/L1 ops は宣言値を根拠に `Unsupported`、未測定能力は `NotRun` |
| Consumer Oracle 不在を Pass 扱い | Gate E の虚偽 | consumer-verify に対象 SHA run がないことを明記し `NotRun` |
| 性能未測定を「問題なし」と解釈 | Gate C の過小評価 | 性能次元を独立 Case として `NotRun` 記録 |

## Deferred Design Concerns

- 実 Native Dock 製品受入 — #532 解消 + #388/#389 残件 + SV #176。Trigger: #532 対策決裁。
- Windows L2 残能力（IME/a11y/動的 DPI/物理入力/plugin resize/DPI・マルチモニタ/2 window/crash-hang） — 製品利用能力の確定待ち。Trigger: Consumer GUI 機能確定・#313/#243 Owner 割当。
- macOS/Linux `native_surface_slot` backend・L2/L3 — 利用要件発生まで Deferred。
- Gate C 性能測定 — 性能予算 Issue の確定待ち。
- Gate E — `consumer-verify.yml` 対象 SHA run または AL #166/SV #176 受入。

## Verification Notes

- 本設計の検証は文書照合（verdict-matrix の Case 網羅性、test-spec の Rule/Example 割当、`mise run fmt-check` + `prek` 軽量チェック、impl-doc audit、証拠再実走 lane）に限定する。
- `mise run check`/`test`/`test-rt` 等の package 指定タスクはコード変更がないため対象外。証拠再実走として `test-ui-scenario-contract`/`check-ui-driver-boundary`/`test-ui-native-slot`/`test-ui-parity`/`test-ui-gpui` を HEAD 上で実行し `evidence/rf262-gpui-verdict/verification.md` に記録する。
- 実 Native Dock・実 GUI・実 Plugin・Consumer 製品受入の Pass 主張は行わない。
