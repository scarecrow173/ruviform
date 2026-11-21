# Issue #388 Virtual-first AC 照合表 — AC 行 × 証拠 disposition

canonical real-dock lane（`evidence/rf631/plugin-dock-real-scenario.jsonl`
@ `b27fb6a1`、29 case = 24 pass + 2 fail + 3 unsupported）と Issue
#388 現行 AC 各行の 1 行ずつの突合記録。判定語彙は 3 軸分離:
照合判定 `done`/`deferred`/`not_applicable`、要求状態
`Required`/`Deferred`/`NotApplicable`、case verdict `pass`/`fail`/
`not_run`/`unsupported`（JSONL 正本）。Fail/Unsupported は done や
Pass に読み替えない。詳細は `docs/test-specs/issue-388-virtual-first-ac.md`
の突合表、disposition 決定と採否・checkbox 更新案は
`docs/specs/issue-388-virtual-first-ac.md`。

## AC 照合表

| AC 行 | 要求状態 | 対応証拠 | case verdict | 照合判定 |
| --- | --- | --- | --- | --- |
| AC1 Virtual-first 移行受入（画素・Clip・Focus・Resize・Dock/ReDock・RT を実 Window で検証。DComp 既知 Fail は修正・再実走まで Fail、Fake/Floating だけで Pass にしない） | Required | `real-dcomp-pixel` green `0.999994`（DComp on、`v0.2.2-rf2`）・`real-dcomp-uncovered`/`present`/`rasterfix`・`real-editor-focus`/`real-multiwindow-focus`・`real-editor-resize`・`real-editor-attach`/`real-editor-redock`・`environment_kind="real-window"`・`test_editor` 実 `WS_CHILD`・RT rationale（discovery） | 該当 case 全 pass + class C 2 件 fail 維持 | **done**（最終 `[x]` は Coordinator） |
| AC2 NativeSurfaceSlot/Fake/2-phase rollback + Headless fixture（PR #399/#424/#527） | Required | `evidence/rf388-native-surface-slot/report.md`（NSC-01〜13・matrix 20 行）+ `test-ui-native-slot` 13 pass（`e93c438a`・3OS） | — | **done**（既に `[x]`） |
| AC3 Windows VST3 操作試験 + DComp 画素制約記録（PR #438/#501、実 Dock Pass ではない） | Required | `evidence/rf388-native-dock-windowing/report.md`（33 check 全 pass・DComp `topmost=true` 実測特定） | — | **done**（既に `[x]`） |
| AC4 3OS 既存 CI で公開契約・境界・Fallback/Detach/RT 非干渉 Compile/Link/Headless を対象 SHA 照合 | Required | `evidence/rf388-os-l1-ci/report.md`（SHA `e93c438a`・run 43428172820・`ci-summary` PASS・checkout leg SHA 一致） | — | **done**（既に `[x]`） |
| AC5 #262/#419 引き渡し（L1 可否・実 Native Dock 非保証・Floating/Generic/Mock ≠ Pass） | Required | `evidence/rf388-os-l1-ci/handoff-262-419.md`（投稿済みとして `[x]`） | — | **done**（既に `[x]`） |
| AC6 Windows GUI-L2/EXT-L3 Required（#630 実 Child HWND + #631 実 Window で画素・Clip/Resize/Focus/ReDock/RT 非干渉。#532 既知 Fail は修正・実走まで未受入） | Required | AC1 と同一証拠集合 + `test_editor`（`ruviform_plugin_test_vst3.dll` sha256 `0bb0de2017ce…`）`RuviformTestEditor` `WS_CHILD` 実測 | 該当 case 全 pass + class C 2 件 fail 維持 | **done**（残存 fail の AC 文解釈は Coordinator 判断点 — spec「処遇の補足」） |
| AC7 macOS/Linux L1 のみ必須（両 OS 既存 CI で Native Build/Link・Feature/Headless/RT を対象 SHA で検証。実 GUI/物理 Device/実 Plugin は Deferred） | Required | 前回照合 `e93c438a`・run 43428172820 は旧 SHA 実績。close 時点対象 SHA の 3OS CI 実走・run URL/verdict 収集が未実施 | — | **deferred**（Coordinator gate・NotRun pending） |
| AC8 検証 Scope 明記（機能・OS/Device/Format・Run URL/Oracle・新規 CI Workflow なし） | Required | `docs/specs/issue-388-virtual-first-ac.md` 検証 Scope 節 + `docs/test-specs/issue-388-virtual-first-ac.md` Scope 表。Workflow 追加なし | — | **done**（記録形態の充足） |

## 残存 fail/unsupported の処遇

| case | verdict | 処遇 | 根拠 |
| --- | --- | --- | --- |
| `real-dcomp-bandescape` | fail | 既知 Fail 継続（class C — `WS_EX_LAYERED` 適用不能の確定診断） | `docs/specs/issue-532-dcomp-residual.md` 決裁引用（昇格・re-baseline なし） |
| `real-dcomp-childinv` | fail | 同上 | 同上（`detail`: `ci_apply_layered_exstyle` refused） |
| `real-window-ime` | unsupported | capability 限界として記録（`Ime` unclaimed — #389 scope） | JSONL `detail` + `real_catalog.rs` + `docs/specs/issue-389-ime-verbs.md` |
| `real-session-crash` | unsupported | #389 lifecycle 領域（in-process lane 構造限界） | JSONL `detail` + evidence tag `session:crash:in-process-lane-nonviable` |
| `real-session-hang` | unsupported | 同上 | 同上 + `session:hang:in-process-lane-nonviable` |

## checkbox 更新案（Coordinator 適用判断用）

- AC1: `[ ]` → `[x]` 提案（class C 既知 Fail 2 件は fail 維持の但し書き付き）
- AC6: `[ ]` → `[x]` 提案（判断点: 残存 class C fail の AC 文解釈 — spec 参照）
- AC8: `[ ]` → `[x]` 提案（Scope 明記の充足）
- AC7: `[ ]` 維持提案（close 時点 SHA の 3OS CI 証拠待ち）
- AC2–AC5: `[x]` のまま変更なし

Issue 本文の編集・適用・Close は Coordinator が行う。Worker は
更新案の納品のみ。
