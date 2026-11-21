---
id: "SPEC-rf388-virtual-first-ac-01"
type: "spec"
status: "approved"
title: "Issue #388 Virtual-first AC 照合 — AC 各行の disposition 決定・検証 Scope・採否決裁形態の提案（docs-only disposition）"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-388-virtual-first-ac.md"
      - type: "file"
        path: "docs/discovery/issue-388-virtual-first-ac.md"
      - type: "file"
        path: "docs/test-specs/issue-388-virtual-first-ac.md"
      - type: "dir"
        path: "evidence/rf388-virtual-first-ac/"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/specs/issue-857-pixel-occlusion.md"
    - "docs/specs/issue-388-native-surface-slot.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-388-virtual-first-ac.md"
    - "docs/test-specs/issue-388-virtual-first-ac.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/specs/issue-857-pixel-occlusion.md"
    - "docs/specs/issue-389-ime-verbs.md"
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "evidence/rf631/plugin-dock-real-scenario.jsonl"
    - "evidence/rf857/run-record.md"
    - "evidence/rf388-virtual-first-ac/"
    - "evidence/rf388-os-l1-ci/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-residual.md"
  derived-by:
    - "docs/discovery/issue-388-virtual-first-ac.md"
    - "docs/test-specs/issue-388-virtual-first-ac.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-388-virtual-first-ac.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/839"
    - "https://github.com/scarecrow173/ruviform/issues/857"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 388
---

# Issue #388 Virtual-first AC 照合 — AC 各行の disposition 決定・検証 Scope・採否決裁形態の提案

canonical real-dock lane（`evidence/rf631/plugin-dock-real-scenario.jsonl`
`b27fb6a1`、29 case = 24 pass + 2 fail + 3 unsupported）の verdict
集合と Issue #388 の現行 AC 各行を 1 行ずつ突合し、残存
fail/unsupported の処遇を証拠つきで記録する AC 照合 disposition
slice。本 slice は **docs/evidence のみ** — verdict 意味論を変える
`.rs` 変更・lane 再実走・Issue 本文編集は行わない。証拠 inventory・
case→AC 対応根拠・RT rationale は
`docs/discovery/issue-388-virtual-first-ac.md`、AC 行 × 証拠の突合表は
`docs/test-specs/issue-388-virtual-first-ac.md` を参照。**Issue #388
の Close 判定・AC checkbox の最終適用は本 slice では行わず、
Coordinator が AC 照合で最終判断する。**

## AC 行の disposition 決定

照合判定の語彙: `done` = 証拠対応が成立、`deferred` = 別 gate に保留、
`not_applicable` = 対象外。要求状態（`Required`/`Deferred`/
`NotApplicable`）と case verdict（`pass`/`fail`/`not_run`/
`unsupported`）は別軸であり、Fail/Unsupported を done や Pass に
読み替えない。

| AC 行 | 照合判定 | 根拠（突合表は test-spec 側） |
| --- | --- | --- |
| AC1 Virtual-first 移行受入（Windows の実 Plugin ABI/OS child HWND Test Native Editor の画素・Clip・Focus・Resize・Dock/ReDock・RT を実 Window で検証。DComp 既知 Fail は修正・再実走まで Fail、Fake/Floating だけで Pass にしない） | **done**（証拠対応成立 — 最終 [x] 適用は Coordinator） | 画素 `real-dcomp-pixel` pass（screen green `0.999994`、DComp on、gpui `v0.2.2-rf2`、canonical `b27fb6a1` 再確認 + `evidence/rf857/run-record.md`）、Clip `real-dcomp-uncovered`/`real-dcomp-present`/`real-dcomp-rasterfix` pass + PR #501 ancestor clip 実績、Focus `real-editor-focus`/`real-multiwindow-focus` pass、Resize `real-editor-resize` pass、Dock/ReDock `real-editor-attach`/`real-editor-redock` pass、実 child HWND/実 Plugin ABI = `test_editor` 実 `WS_CHILD` `IPlugView` + `environment_kind="real-window"`。RT = rationale 記録（discovery）。既知 Fail 2 件は class C で fail 維持（AC 文が要求する「Fail のまま維持」どおり）。driver `real-os` のみで Fake/Floating 代替なし |
| AC2 Driver-neutral NativeSurfaceSlot/Fake Driver/2-phase rollback 契約と Headless fixture 統合（PR #399/#424/#527） | **done**（既に `[x]`。再検証しない） | `evidence/rf388-native-surface-slot/report.md`（NSC-01〜13・capability matrix 20 行）+ `evidence/rf388-os-l1-ci/report.md` の `test-ui-native-slot` 13 pass（対象 SHA `e93c438a`・3OS） |
| AC3 Windows 代表 VST3 操作試験と DComp 画素制約の記録（PR #438/#501 — 実 Native Dock Pass ではない） | **done**（既に `[x]`。再検証しない） | `evidence/rf388-native-dock-windowing/report.md`（33 check 全 pass・DComp `topmost=true` 遮蔽の実測特定）。記録行の充足であって Dock Pass 主張ではない |
| AC4 3OS 既存 CI で公開契約・GPUI/OS private handle 境界・Fallback/Detach/RT 非干渉の Compile/Link/Headless を対象 SHA で照合 | **done**（既に `[x]`。再検証しない） | `evidence/rf388-os-l1-ci/report.md`（SHA `e93c438a`・run 43428172820・全 19 checkout leg の実測 SHA 一致・`ci-summary` PASS） |
| AC5 #262/#419 への引き渡し（L1 完了可否・実 Native Dock 非保証・Floating/Generic/Mock ≠ Dock Pass） | **done**（既に `[x]`） | `evidence/rf388-os-l1-ci/handoff-262-419.md`（投稿済みとして Issue で `[x]` 済み）。引き渡し先 #262/#419 は CLOSED — 採否決裁形態は後述 |
| AC6 Windows GUI-L2/EXT-L3 Required（#630 実 Child HWND + #631 GPUI 実 Window で実画素・Clip/Resize/Focus/ReDock/RT 非干渉。Fake/Generic/Floating は Pass にできない。#532 DComp 既知 Fail は修正・実走まで未受入） | **done**（証拠対応成立 — 最終 [x] 適用は Coordinator。残存 class C fail の扱いは後述の判断点を参照） | AC1 と同一証拠集合。実 child HWND = `test_editor`（`ruviform_plugin_test_vst3.dll` sha256 `0bb0de2017ce…`）の `RuviformTestEditor` `WS_CHILD` HWND を `child_enumerated`/`occlusion_hit_test` で実測。実 Window = `environment_kind="real-window"`。DComp 既知 Fail のうち画素系（class A/B）は修正・実走済み pass、残存は class C 診断 2 件（後述） |
| AC7 macOS/Linux L1 のみ必須（両 OS の既存 CI で Native Build/Link・必要 Feature/Headless/RT を対象 SHA で検証。実 GUI/物理 Device/実 Plugin は使用開始時まで Deferred） | **deferred**（Coordinator gate — close 時点対象 SHA の 3OS CI 実走証拠が必要） | 前回照合 `e93c438a`・run 43428172820 は旧 SHA の実績（後続 merge 多数）で現 HEAD ではない。close 時点 SHA での実走・run URL/verdict 収集が必要 — `done` にしない（NotRun pending） |
| AC8 検証 Scope 明記（採用/利用機能・OS/Device/Format・Run URL/Oracle・新規 CI Workflow なし） | **done**（記録形態の充足 — 最終 [x] 適用は Coordinator） | 下記「検証 Scope」節 + test-spec の Scope 表に記録。新規 CI Workflow は作っていない |

## 残存 fail/unsupported の処遇（確定事項の記録）

| case | verdict | 処遇 | 根拠 |
| --- | --- | --- | --- |
| `real-dcomp-bandescape` | fail | **既知 Fail 継続**（class C — `WS_EX_LAYERED` 適用不能の確定診断。別 Issue 切り出し不要） | `docs/specs/issue-532-dcomp-residual.md` の決裁を引用（silent no-op・error 87・`latched_as_toplevel`・`0x80070006`）。診断目的達成済み。昇格・re-baseline・Pass 化はしない |
| `real-dcomp-childinv` | fail | **既知 Fail 継続**（同上・child-level で同一機構） | 同上。canonical JSONL `detail` = "editor lifecycle verified but the child-inventory probe did not complete: `ci_apply_layered_exstyle` refused" |
| `real-window-ime` | unsupported | **capability 限界として記録**（`Ime` unclaimed — IME adapter API なし。#389 scope として追跡中） | JSONL `detail` = "unsupported capability: Ime" + `real_catalog.rs` doc + `docs/specs/issue-389-ime-verbs.md`（approved） |
| `real-session-crash` | unsupported | **#389 lifecycle 領域として記録**（in-process lane では fixture の process-killing fault を lane が生き残れない。out-of-process host は #389 L1 領域） | JSONL `detail` + `real_catalog.rs` doc（"Issue #389 lifecycle remainder"）+ evidence tag `session:crash:in-process-lane-nonviable` |
| `real-session-hang` | unsupported | **同上**（wedging hang fault — plugin call が返らず host process を共有） | 同上 + evidence tag `session:hang:in-process-lane-nonviable` |

### 処遇の補足

- **verdict の昇格・re-baseline・Pass 化は行わない。** 2 fail は実測に
  基づく honest 記録、3 unsupported は POLICIES.md §3 の「根拠を伴う
  技術的非対応」の定義どおり。disposition は証拠対応付けの記録で
  あり、JSONL の verdict 値・case 意味論の変更ではない。
- **class C が AC6「#532 DComp 既知 Fail は修正・実走まで未受入」と
  の関係**: AC 文の「DComp 既知 Fail」が指す主たる対象は、plugin
  画素が DComp topmost plane に覆われる画素合成問題（class A —
  `real-dcomp-pixel`/`real-dcomp-rasterfix`）であり、#839 根治 fix +
  #857 class B 処遇で**修正・実走済み**。残存 2 case は棄却済み fix
  経路（`WS_EX_LAYERED`）の可否を問う診断 probe であり、#532
  residual chain で「既知 Fail として継続・別 Issue 不要」と決裁
  済みのものをそのまま維持する。最終的に AC6 の `[x]` をどう判断
  するか（残存 fail を受入阻害と読むか、AC1 と同じく「Fail 維持を
  正直に記録する」条項の充足と読むか）は **Coordinator の判定** —
  本 spec は両読みを併記し証拠だけを確定する。
- **unsupported 3 行の追跡先**: `real-window-ime` は
  `docs/specs/issue-389-ime-verbs.md`（approved）、crash/hang は
  `docs/specs/issue-389-real-session-lifecycle.md`（approved）および
  catalog doc の「Issue #389 lifecycle remainder」記述により、
  **すべて Issue #389 scope として既に追跡先が存在する**。本 slice
  では新規 Issue 起票提案を要する残件は確認されなかった（後述）。

## RT 非干渉の扱い

AC1/AC6 の「RT（非干渉）」項に対応する明示 case は lane に存在しない
ため、`docs/discovery/issue-388-virtual-first-ac.md` の rationale 記録
で対応付ける（#532 residual chain 同型）:

- 作用面は UI thread / scenario caller thread の GUI 経路のみで、
  audio RT path と共有 state・lock・thread なし。
- lane は `test_editor` の GUI verb のみを行使し audio engine・
  `process()` を駆動しない — RT 境界を touch しない。
- 既存の headless RT 契約（`test-rt` 166 test、`e93c438a` で Pass）は
  旧 SHA 実績として記録し、close 時点 SHA での非回帰再確認は AC7
  gate に含める。
- 実 Plugin GUI hang/crash 時の RT 影響は #389 共同検証領域であり、
  crash/hang fault 注入は `Unsupported`（in-process lane 構造限界）。

## 検証 Scope（AC8 対応 — 採用/利用機能の明記）

| 項目 | 値 |
| --- | --- |
| 採用/利用する機能 | Windows Native Plugin Dock（NativeSurfaceSlot → 実 child HWND docking、GPUI Dock 実 GUI 埋め込み）、DComp on 経路 |
| OS / arch / session | Windows x86_64、interactive desktop（`environment_kind="real-window"`）。headless 契約層は 3OS CI（`e93c438a` 実績） |
| Device / GPU | NVIDIA GeForce RTX 4070 SUPER（DISPLAY1 primary、multi-monitor 環境だが被測定は DISPLAY1） |
| Plugin / Format | `test_editor`（#630 正規 ABI Test Plugin、VST3 実バイナリ `ruviform_plugin_test_vst3.dll` sha256 `0bb0de2017ce…`、実 `WS_CHILD` `IPlugView`）。CLAP `test_editor`（`clap.ext.gui`）は存在するが本 lane 未実施 — **CLAP format は NotRun**（Pass に繰り上げない）。AU/macOS/Linux 実 Plugin は Deferred |
| GPUI / DComp | fork `v0.2.2-rf2`（`b3144f2`）、`gpui_disable_direct_composition=null`（DComp on — `noredirectionbitmap: false` で実証） |
| Oracle | 画素: `win32-bitblt-screen` screen green `>= 0.5`（独立 oracle。`PrintWindow`/window-DC は診断 note のみ）。editor 操作: session events・child HWND 契約/状態・OS 効果 oracle（非画素）。fail-closed: 外部遮蔽残存時は `not_run` 退避（#857 実証） |
| Run | ローカル interactive lane `mise run test-ui-plugin-dock-real`（CI 非収載 — Run URL なし。証拠 = canonical JSONL `b27fb6a1` + `evidence/rf857/`）。headless 契約層 = 既存 `ci.yml`（`e93c438a`・run 43428172820） |
| 必須としない範囲 | 未利用 Device・Plugin 形式（CLAP 実走・AU・市販 Plugin）・追加環境（DPI!=1.0・multi-monitor mixed-DPI・fullscreen 遷移・複数 host）は必須としない（POLICIES.md §2） |
| 新規 CI Workflow | なし |

## 採否決裁形態の提案

Issue #388 本文は「#262 は採用可否を本 PoC から判定」「#262/#419
と Synvelis #176 に送る」という handoff 形態を定めるが、**rf#262
と rf#419 はともに CLOSED**（2026-11-21 時点 `gh issue view` で
確認 — #262「GPUI Native First 採用Gate」、#419「後継 ADR の最終
採用」）。#532 residual chain で同じ構造に遭遇し、そこでの提案を
踏襲する:

- **採用判定の入力正本は本 doc chain** — canonical lane の verdict
  集合・class C 残存の確定診断・capability 限界（IME/crash/hang）・
  検証 Scope を、#262 が求めた「採用判定の必須入力」に相当する
  情報として `issue-388-virtual-first-ac` chain に集約する。
- **最終採用決裁は #388 本体で行う形態を提案** — #262/#419
  CLOSED により上流 Gate は消滅したと解し、Issue #388 の AC 照合
  時に Coordinator が「Windows VST3 L0・GPUI Dock 実 GUI 埋め込み
  （DComp on 経路）の採用可否」を確定する。新規 ADR / 新 Issue へ
  の分離は、採用範囲の拡大（CLAP 実走・他 OS・製品適用）要件が
  発生した時点で切り出す。
- **採否の候補評価軸（決裁は Coordinator）**: (a) 採用 —
  Windows/VST3/L0 の実 Dock が Required 項目を実測 pass で満たし
  残存は class C 診断と capability 限界のみ、(b) 条件付き採用 —
  class C fail 継続・IME/crash/hang unsupported・CLAP NotRun を
  受入条件として明記した上での採用、(c) 非採用/保留 — 残存
  fail/unsupported を採用阻害と判断する場合。**本 slice は判定を
  下さない。**
- **SV #176 への伝播** — cross-repo のため記録のみ（製品側受入は
  Coordinator/owner 側で判断）。

## Issue checkbox 更新案（Worker は Issue 本文を編集しない）

Coordinator が Issue #388 本文へ適用を判断するための案:

| AC 行 | 現行 | 更新案 | 文面修正の提案 |
| --- | --- | --- | --- |
| AC1 Virtual-first 移行 | `[ ]` | **`[x]` 提案** | 末尾に照合済み証拠への言及を追記する案: 「（virtual-first-ac 照合: canonical `b27fb6a1`・`evidence/rf388-virtual-first-ac/`・`docs/test-specs/issue-388-virtual-first-ac.md`。class C 既知 Fail 2 件は fail 維持）」 |
| AC2 NativeSurfaceSlot 契約 | `[x]` | 変更なし | — |
| AC3 Windows VST3 操作試験 + DComp 制約記録 | `[x]` | 変更なし | — |
| AC4 3OS CI 照合（`e93c438a`） | `[x]` | 変更なし | — |
| AC5 #262/#419 引き渡し | `[x]` | 変更なし | — |
| AC6 Windows GUI-L2/EXT-L3 | `[ ]` | **`[x]` 提案**（判断点あり） | 「#532 の DComp 既知 Fail」の読み: 画素合成問題（class A/B）は修正・実走済み。残存 `real-dcomp-bandescape`/`real-dcomp-childinv` は #532 residual 決裁済みの class C 診断 probe で、AC6 を `[x]` とする場合は「class C 既知 Fail は継続維持（#532 residual 決裁どおり）」の但し書き追加を提案。厳格に「lane 内に fail が残る間は未受入」と読む場合は `[ ]` 維持 — Coordinator 判定 |
| AC7 macOS/Linux L1 | `[ ]` | **`[ ]` 維持提案** | close 時点対象 SHA の 3OS CI 実走証拠（run URL・verdict）が揃ってから Coordinator が適用。前回 `e93c438a` は旧 SHA 実績 |
| AC8 検証 Scope | `[ ]` | **`[x]` 提案** | 「（scope 明記: `docs/specs/issue-388-virtual-first-ac.md` 検証 Scope 節・`docs/test-specs/issue-388-virtual-first-ac.md` Scope 表）」の追記案 |

## 起票提案（残件の切り出し要否）

照合で確認した残件はすべて既存 Issue の scope に内包されており、
**新規 Issue の起票提案はない**:

- `real-window-ime`（IME capability）→ #389 scope（catalog doc +
  `docs/specs/issue-389-ime-verbs.md` で追跡済み）
- `real-session-crash`/`real-session-hang`（fault fixture）→ #389
  lifecycle scope（catalog doc「lifecycle remainder」+
  `docs/specs/issue-389-real-session-lifecycle.md`）
- class C 2 case の恒久扱い（re-scope / `Unsupported` 化の設計判断）
  → #532 residual chain で「既知 Fail 継続」決裁済み。再検討が必要に
  なった場合は lane 設計判断として別起票（現時点では提案しない）
- DPI!=1.0・multi-monitor・CLAP 実走・市販 Plugin → 利用要件発生時に
  case 化する Deferred 項目（POLICIES.md §2.1。必須ゲートから外す
  方針どおり記録のみ）

## スコープ外（本 slice で扱わない）

- `real_dock.rs`/`real_catalog.rs` 等の verdict 意味論変更、`detail`
  文言更新、case re-scope（docs-only disposition のため全て後続）。
- real lane の再実走（interactive session が必要。canonical JSONL
  `b27fb6a1` を正本とする — close 時点 SHA での再実走要否は
  coordinator gate）。
- macOS/Linux L1 の対象 SHA 実走・run URL/verdict 証拠収集
  （AC7 = Coordinator gate）。
- 実 Native Dock 採否の最終決裁（採用/条件付き/非採用 + ADR 要否）
  — Coordinator/ユーザー gate。
- Issue #388 本文の編集・AC checkbox の適用・Issue Close、新規
  Issue 実起票・GitHub 依存登録、docs index（`docs/**/README.md`）
  整備、merge-SHA CI 証拠収集 — すべて Coordinator。
- audio-lens / synvelis 側の文書・関係確認 — 他リポジトリ、範囲外。
