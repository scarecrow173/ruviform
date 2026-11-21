---
id: "TSPEC-rf388-virtual-first-ac-01"
type: "test-spec"
status: "approved"
title: "Issue #388 Virtual-first AC 照合 — canonical real-dock verdict 集合と AC 各行の突合"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-388-virtual-first-ac.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-388-virtual-first-ac.md"
    - "docs/test-specs/issue-532-dcomp-residual.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-388-virtual-first-ac.md"
    - "docs/discovery/issue-388-virtual-first-ac.md"
    - "docs/test-specs/issue-532-dcomp-residual.md"
    - "docs/test-specs/issue-857-pixel-occlusion.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
    - "evidence/rf631/plugin-dock-real-scenario.jsonl"
    - "evidence/rf388-virtual-first-ac/ac-reconciliation.md"
    - "evidence/rf388-virtual-first-ac/run-record.md"
    - "evidence/rf857/run-record.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-388-virtual-first-ac.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-388-virtual-first-ac.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 388
---

# Issue #388 Virtual-first AC 照合 — canonical real-dock verdict 集合と AC 各行の突合

`docs/specs/issue-388-virtual-first-ac.md` の AC disposition を検証する
文書。canonical real-dock lane（`evidence/rf631/plugin-dock-real-scenario.jsonl`
@ `b27fb6a1`、interactive Windows・DComp on・gpui `v0.2.2-rf2`）の
verdict 集合を Issue #388 の現行 AC 各行へ 1 行ずつ突合する。証拠
inventory と case→AC 対応根拠は
`docs/discovery/issue-388-virtual-first-ac.md`、実測値の正本は
JSONL 自身と `evidence/rf388-virtual-first-ac/run-record.md`。

## 検証対象の verdict 集合（canonical run 実測）

JSONL の case rows 29 件: **24 pass + 2 fail + 3 unsupported**
（case row のトップレベル `verdict` を集計 — `observed`/`evidence`
内の副次 verdict（step 結果・`note`）とは混同しない）。

| case | verdict | 備考 |
| --- | --- | --- |
| `real-dcomp-pixel` | pass | 画素中核証拠 — screen green `0.9999941167472672`（`win32-bitblt-screen`、1278x798、1,019,838/1,019,844 px）、DComp on（root `noredirectionbitmap: false`）、#857 de-occlusion arm 経由 |
| `real-dcomp-present` / `real-dcomp-uncovered` / `real-dcomp-rasterfix` | pass | DComp presentation/clip 経路 |
| `real-dcomp-bandescape` | fail（既知） | **class C** — `WS_EX_LAYERED` 適用不能の確定診断（#532 residual「既知 Fail 継続」決裁） |
| `real-dcomp-childinv` | fail（既知） | **class C** — 同上（child-level。`detail`: `ci_apply_layered_exstyle` refused） |
| `real-window-ime` | unsupported | `required: [ime]`、`detail` = "unsupported capability: Ime" — #389 scope |
| `real-session-crash` | unsupported | in-process lane 構造限界（`session:crash:in-process-lane-nonviable`）— #389 lifecycle scope |
| `real-session-hang` | unsupported | 同上（`session:hang:in-process-lane-nonviable`）— #389 lifecycle scope |
| `real-editor-attach` / `real-editor-resize` / `real-editor-redock` / `real-editor-visibility` / `real-editor-input` / `real-editor-focus` / `real-editor-popup` | pass | editor ops — 実 child HWND 契約・OS 効果 oracle（非画素） |
| `real-session-terminate` / `real-session-degraded` / `real-session-recovery` / `real-session-timeout` | pass | #389 lifecycle の lane 共存行 — #388 では補助 context |
| `real-window-mount` / `real-native-slot` / `real-capture` / `real-dcomp-foreground` / `real-multiwindow-focus` / `real-os-input` / `real-command-receipt` / `real-window-dpi` / `real-window-close` | pass | real-window/semantic/input/dock 基盤 case |

## AC 行 × 証拠 突合表

判定語彙: 照合判定 `done`/`deferred`/`not_applicable`、要求状態
`Required`/`Deferred`/`NotApplicable`、case verdict `pass`/`fail`/
`not_run`/`unsupported`（JSONL 正本の値）— 3 軸を分離する。

| AC 行（Issue #388 現行受入条件） | 対応する証拠 | 照合判定 |
| --- | --- | --- |
| **AC1 Virtual-first 移行（Required）**: 実 Plugin ABI/OS child HWND Test Native Editor の画素を実 Window で検証 | `real-dcomp-pixel` pass — screen green `0.999994`・DComp on・gpui `v0.2.2-rf2`・capture_sha256 `89401018…`（canonical `b27fb6a1`）+ `evidence/rf857/run-record.md`（class B 処遇確定・fail-closed gate 実走実証） | **done** |
| 〃: Clip を実 Window で検証 | `real-dcomp-uncovered`/`real-dcomp-present` pass + `real-dcomp-rasterfix` pass（clip-partition）+ PR #501 ancestor clip（`evidence/rf388-native-dock-windowing/report.md` の `clip_overhang_region`/`clip_direct_region`/`clip_direct_hit_test`） | **done** |
| 〃: Focus を実 Window で検証 | `real-editor-focus`/`real-multiwindow-focus` pass（title event-log token oracle・multi-window focus） | **done** |
| 〃: Resize を実 Window で検証 | `real-editor-resize` pass（child `GetWindowRect` geometry oracle） | **done** |
| 〃: Dock/ReDock を実 Window で検証 | `real-editor-attach` pass（`child_enumerated` で実 child HWND・`Attached { mode: Docked }` event・`child_hwnd_detached` で破棄確認）+ `real-editor-redock` pass（attach-generation title advance oracle） | **done** |
| 〃: RT（非干渉）を検証 | lane に明示 RT case なし — `docs/discovery/issue-388-virtual-first-ac.md` RT rationale（UI thread GUI 経路のみ・audio RT と共有 state/lock/thread なし・lane は `process()` を駆動しない）+ 既存 headless RT 契約（`test-rt` 166 test・`e93c438a` 実績 — close SHA 再確認は AC7 gate） | **done（rationale 記録）** |
| 〃: 実 Plugin ABI / OS child HWND / 実 Window | `environment_kind="real-window"`（全 29 case）+ `test_editor`（`crates/ruviform-plugin-test-vst3` — `createView("editor")` → 実 `IPlugView` → `WS_CHILD` HWND）+ `RuviformTestEditor`/`RuviformSlotContainer`/`Zed::Window` HWND 階層記録 + plugin-artifact sha256 | **done（証拠属性）** |
| 〃: DComp 既知 Fail は修正・再実走まで Fail、Fake/Floating だけで Pass にしない | `real-dcomp-bandescape`/`real-dcomp-childinv` = fail 維持（class C 確定診断・昇格なし）。画素問題本体（class A/B）は修正・実走済み pass。driver `real-os` のみで Fake/Offscreen/Floating 由来 verdict なし | **done** |
| **AC2 NativeSurfaceSlot/Fake/2-phase rollback + Headless fixture（済・Required 充足済）** | PR #399/#424/#527 — `evidence/rf388-native-surface-slot/report.md`（NSC-01〜13・matrix 20 行）+ `test-ui-native-slot` 13 pass（`e93c438a`・3OS） | **done（既に `[x]`・再検証しない）** |
| **AC3 Windows VST3 操作試験 + DComp 画素制約記録（済）** | PR #438/#501 — `evidence/rf388-native-dock-windowing/report.md`（33 check 全 pass・DComp `topmost=true` 遮蔽の実測特定）。「実 Native Dock Pass ではない」記述どおりの記録行 | **done（既に `[x]`）** |
| **AC4 3OS 既存 CI で公開契約・境界・Fallback/Detach/RT 非干渉の Compile/Link/Headless を対象 SHA で照合（済）** | `evidence/rf388-os-l1-ci/report.md` — SHA `e93c438a`・run 43428172820・`ci-summary` PASS・全 checkout leg SHA 一致 | **done（既に `[x]`）** |
| **AC5 #262/#419 引き渡し（済）** | `evidence/rf388-os-l1-ci/handoff-262-419.md`（投稿済みとして `[x]`）。#262/#419 は CLOSED — 採否決裁形態の提案は spec 側 | **done（既に `[x]`）** |
| **AC6 Windows GUI-L2/EXT-L3（Required）**: #630 正規 ABI Test Native Editor から実 Child HWND 生成 | `test_editor`（`ruviform_plugin_test_vst3.dll` sha256 `0bb0de2017ce…`）— `child_enumerated` で `RuviformTestEditor` class・`WS_CHILD`・`parent_matches: true`・`alive` を実測（`real-editor-attach` 他全 dock case） | **done** |
| 〃: #631 GPUI 実 Window で実画素・Clip/Resize/Focus/ReDock/RT 非干渉を検証 | AC1 と同一証拠集合（上記各行）。`environment_kind="real-window"`・`Zed::Window` root（GPUI window） | **done** |
| 〃: 市販 Plugin は通常必須ではないが Fake/Generic/Floating は Pass にできない | 採用 fixture は正規 ABI `test_editor` のみ（POLICIES.md §2.1 どおり市販 Plugin 非必須）。Fake/Generic/Floating 由来の verdict は JSONL に存在しない | **done（証拠属性）** |
| 〃: #532 の DComp 既知 Fail は修正・実走まで未受入 | 画素合成問題（class A `real-dcomp-rasterfix` / class B `real-dcomp-pixel`）は #839 fix + #857 処遇で修正・実走済み pass。残存 `real-dcomp-bandescape`/`real-dcomp-childinv` は class C 確定診断として **fail 維持**（#532 residual「既知 Fail 継続」決裁を引用 — 昇格・re-baseline なし）。AC6 の `[x]` 判定は残存 fail の読みに依存 — **Coordinator 判断点**（spec「処遇の補足」） | **done（判断点付き）** |
| **AC7 macOS/Linux L1 のみ必須** | 前回照合（SHA `e93c438a`・run 43428172820）は旧 SHA 実績 — `ui-gpui-verify-macos`/`verify-macos`/`l1-real-verify-macos`・ubuntu 側 lane 全 success（`evidence/rf388-os-l1-ci/report.md`）。close 時点対象 SHA での 3OS CI 実走・run URL/verdict 収集は未実施 | **deferred（Coordinator gate・NotRun pending）** |
| **AC8 検証 Scope 明記・新規 CI Workflow なし** | spec「検証 Scope」節 + 下記 Scope 表。Workflow 追加なし（本 slice は docs+evidence のみ） | **done** |

## 検証 Scope（AC8 対応の明記）

| 項目 | 値 |
| --- | --- |
| 採用/利用する機能 | Windows Native Plugin Dock（NativeSurfaceSlot → 実 child HWND docking、GPUI Dock 実 GUI 埋め込み）、DComp on 経路 |
| OS / arch / session | Windows x86_64、interactive desktop（`environment_kind="real-window"`）。headless 契約層は 3OS CI |
| Device / GPU | NVIDIA GeForce RTX 4070 SUPER |
| Plugin / Format | `test_editor`（VST3 実バイナリ `ruviform_plugin_test_vst3.dll` sha256 `0bb0de2017ce…`、実 `WS_CHILD` `IPlugView`）。CLAP `test_editor` 実走は NotRun、AU/macOS/Linux 実 Plugin は Deferred |
| GPUI / DComp | fork `v0.2.2-rf2`（`b3144f2`）、DComp on（`noredirectionbitmap: false`） |
| Oracle | `win32-bitblt-screen` screen green `>= 0.5`（独立 oracle）+ 非画素 oracle（session events・child HWND 契約/状態・OS 効果）+ fail-closed gate（外部遮蔽残存時 `not_run` 退避） |
| Run | `mise run test-ui-plugin-dock-real`（CI 非収載 interactive lane — Run URL なし。canonical JSONL `b27fb6a1` + `evidence/rf857/`）。headless 層 = 既存 `ci.yml`（`e93c438a`・run 43428172820） |
| 新規 CI Workflow | なし |

## 本 slice の検証 lane 実施記録

docs-only disposition のため、必須は軽量チェックのみ（POLICIES.md
§5「文書・ポリシーのみの変更は軽量な衛生チェック」）。

| lane | 実行 | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 実施 | pass（exit 0 — 結果は PR 本文の Verification 節と整合） |
| `mise exec -- prek run --all-files` | 実施 | pass — trailing whitespace / end-of-file / YAML / TOML / mixed EOL / BOM / private-key / actionlint 全 pass（exit 0） |
| `mise run check`/`lint`/`test`（package 指定） | NotRun | `.rs` 変更なしのため非該当（該当 package なし） |
| `mise run test-ui-plugin-dock-console` | NotRun | verdict/catalog 非接触の docs-only のため非該当 |
| `mise run test-ui-plugin-dock-real-headless` | NotRun | fail-closed mapping 非接触のため非該当 |
| `mise run test-ui-plugin-dock-real` | NotRun | real lane 再実走は範囲外（canonical JSONL `b27fb6a1` が正本。close 時点 SHA での再実走要否は coordinator gate） |

## verdict の誠実性チェック

- 2 既知 fail（class C）は verdict の昇格・偽装を行っていない —
  disposition は #532 residual 決裁の引用であり、JSONL の verdict
  値は変更しない。`Unsupported` re-baseline・re-scope は verdict
  意味論変更を伴うため本 slice では不採用（spec「処遇の補足」）。
- 3 unsupported は POLICIES.md §3 の「根拠を伴う技術的非対応」の
  定義どおりで、Pass/Fail への読み替え・`done` への繰り上げは
  行わない（IME = capability 限界、crash/hang = in-process lane
  構造限界、いずれも #389 scope で追跡済み）。
- `real-session-terminate`/`degraded`/`recovery`/`timeout`・
  `real-editor-popup` の pass 行は #389/#630 の lane 共存行であり、
  #388 の照合では補助 context としてのみ扱う — #389/#630 の AC
  充足を代行主張しない。
- AC7 の `deferred` は前回 SHA `e93c438a` の実績を現 HEAD の成果と
  混同しない正直な記録 — close 時点対象 SHA の 3OS CI 実走が揃う
  まで `done` にしない。
- AC1/AC6 の `done` は証拠対応の成立を意味し、Issue checkbox の
  最終適用・Issue Close・実 Native Dock 採否決裁は Coordinator
  gate — 本 test-spec は判定を下さない。
