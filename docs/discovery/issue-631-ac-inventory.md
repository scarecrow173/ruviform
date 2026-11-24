---
id: "DISC-rf631-ac-inventory-01"
type: "discovery"
status: "final"
title: "Issue #631 AC disposition census — GUI Test Driver AC 行の証拠 inventory"
created: "2026-11-24"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-631-ac-inventory.md"
      - type: "doc"
        path: "docs/specs/issue-631-ac-inventory.md"
      - type: "doc"
        path: "docs/test-specs/issue-631-ac-inventory.md"
      - type: "doc"
        path: "evidence/rf631/ac-inventory.md"
    modified:
      - type: "file"
        path: "evidence/rf631/plugin-dock-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console-list.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console-list-include-real.jsonl"
  depends-on:
    - "POLICIES.md"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-631-scenario-catalog.md"
    - "docs/specs/issue-631-real-row.md"
    - "docs/specs/issue-631-test-lab.md"
    - "docs/specs/issue-631-editor-input.md"
    - "docs/specs/issue-631-command-receipt.md"
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/specs/issue-389-window-ime.md"
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/specs/issue-631-real-window-dock.md"
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/specs/issue-631-evidence-fields.md"
    - "docs/specs/issue-631-gpu-field.md"
    - "docs/specs/issue-630-clap-test-editor.md"
    - "docs/specs/issue-388-virtual-first-ac.md"
  related:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-ac-inventory.md"
  verified-by:
    - "docs/test-specs/issue-631-ac-inventory.md"
metadata:
  issue:
    number: 631
    url: "https://github.com/scarecrow173/ruviform/issues/631"
---

## 概要

Issue #631（GUI Test Driver — 実 Window の Scenario Runner）の受入条件 7 行を、
canonical ledger と既存証拠に突合し、行ごとの現行 verdict を棚卸しした close-out
判定材料である。本 slice は証拠の突合と分類のみを行い、実装・verdict 意味の
変更・Issue 本文の更新は行わない。最終的な checkbox 適用は Coordinator の
判定に委ねる。

## 証拠 inventory

### canonical ledger

| ledger | sha / 条件 | case 内訳 |
| --- | --- | --- |
| `evidence/rf631/plugin-dock-real-scenario.jsonl`（read-only 正本） | `a0e34564`（run `rf631-plugin-dock-real`・Windows interactive・DComp on） | 32 case 全 pass — GPUI window 起動・OS 入力・Receipt・画素・multiwindow・DPI・IME・NativeSlot・editor attach/dock・DComp 既知 fail 再現・session 系（crash/hang は #886 out-of-process L1 oracle） |
| `evidence/rf631/plugin-dock-scenario.jsonl` | `06a5a9e0`（本 slice で HEAD 再生成・canonical 入口 `mise run test-ui-plugin-dock`） | 9 case = 7 pass + `editor-attach` unsupported + `dock-pixels` not_run |
| `evidence/rf631/plugin-dock-console.jsonl` | `06a5a9e0`（`mise run test-ui-plugin-dock-console`・exit 0 = Fail なし） | 同上 9 case |
| `evidence/rf631/plugin-dock-console-list(-include-real).jsonl` | 本 slice で再生成・補助証拠（canonical = mise、直接 cargo 実行） | catalog 投影 9 row / real 含む 41 row |
| `evidence/rf631/plugin-dock-real-headless.jsonl`（read-only） | `a0e34564`・`--blocked-probe` 自己検査 | 32 row 全て `not_run` + `environment_blocked` — fail-closed 実証 |
| `evidence/rf631/plugin-dock-lab.jsonl`・`real-window-surface.jsonl`（read-only） | `eafe0a08` 等 | Test Lab lane・real-window lifecycle（close/dpi）記録 |

### 検証記録と spec 系

`evidence/rf631/verification.md`（slice 別 canonical=mise 記録の集約）に加え、
AC 各行の根拠となる spec 群は `depends-on` に列挙した。CI 構造の根拠は
`.github/workflows/ci.yml`（`ui-verify`/`ui-verify-macos`/`ui-gpui-verify`/
`ui-gpui-verify-macos` — read-only 参照）。

## 対象 AC と case → AC 対応の論理

- **AC1（実 Window 起動・入力・Receipt・画素・実アプリ State・2Window Focus/Close/Undo・DPI/IME）**:
  `real-window-mount`→起動、`real-os-input`→入力、`real-command-receipt`→Receipt、
  `real-capture`/`real-dcomp-pixel`→画素 Capture、`command-receipt`/`real-command-receipt`
  の `semantic_tree`+`receipts`→実アプリ State 照合、`real-multiwindow-focus`→2Window
  Focus、`real-window-close`→armed Veto→Approve の Close 语义、`real-window-dpi`/
  `real-window-ime`→DPI/IME。Undo は GPUI window 側语义ではなく consumer の
  app-state 语义として `docs/specs/issue-631-real-row.md` が catalog 非収載を
  記録済み（NotApplicable 相当）。
- **AC2（Lab GUI・consumer GUI への Driver-neutral 投影）**: `plugin-dock-lab`
  が `ruviform-ui` ViewSpec による実 GPUI window を建てて Test Lab を実在化。
  Synvelis/AudioLens への実投影は製品固有 Oracle を各 repo に置く設計どおり
  consumer 責務 — ruviform 側の判定は「runner が driver-neutral な投影経路を
  持つこと（ScenarioCapability + 公開投影 seam）」で済み、consumer 実走 legs は
  external-owner の残件。
- **AC3（実 Child HWND→NativeSurfaceSlot・DComp 既知 Fail 再現・Pass 判定）**:
  `real-native-slot`→ParentSlotLease、`real-editor-attach`→test_editor の実
  `WS_CHILD` `IPlugView`、`real-dcomp-*` family→既知 Fail 再現・二分 probe、
  `real-editor-{input,focus,resize,redock}`→Pass 判定。class C の確定機序は
  `#917` re-scope で `expected_outcome` note 化 —「未解決問題を Runner 整備だけで
  解消済みにしない」条項を、拒否 `applied`/`win32_error`/`after` の実測記録として
  保持している。
- **AC4（証拠 field schema・verdict 異軸記録）**: ledger の `environment`
  （os/arch/scale_factor/dcomp/display_adapters）・`case_seed`・
  `window_identities`・`plugin_artifact` sha256・`repo.sha`・`semantic_tree`/
  `receipts`・capture sha256 が全行に存在し、`verdict` は `required`/
  `capabilities` から独立した top-level field。
- **AC5（fail-closed NotRun・capability NotApplicable・macOS/Linux Deferred）**:
  `--blocked-probe` ledger が real 32 row・CLAP 14 row の全 `not_run`+
  `environment_blocked` を bin 自己検査で強制。capability 未実証は gated
  verdict、timeout は定義済 event。macOS/Linux GUI は POLICIES §2 の Deferred。
- **AC6（共通 UI/API・GPUI 型非漏出）**: `check-ui-driver-boundary` が CI
  `ui-gpui-verify` に収載済みで cargo-metadata+`syn` の機械検査を毎走査。
- **AC7（3OS OS-L1 seam 既存 CI・Windows real lane opt-in）**: headless GUI
  test seam 14 task が 3OS verify job に収載、real-window lane は
  workflow_dispatch 専用の opt-in で PR required 条件に非該当。

## 制約・限界

- real-window ledger は read-only 正本であり本 slice では再生成しない（GUI 必須
  lane は対象外）。記録 sha は `a0e34564` で HEAD `06a5a9e0` より 3 commit 前 —
  介在 commit は docs/evidence/CLAP lane のみで、runner 意味の変更はない。
- 画素 Capture は `real-capture` が window/slot-region の capture を記録し、
  editor 画素は `real-dcomp-pixel` が担う — 両者は別 case で、editor 画素の
  証拠を `real-capture` に読み替えない。
- AC7 の「統合」は CI 構造の記述的充足であり、対象 SHA の CI 実走証拠収集は
  merge 後の Coordinator gate — 本 slice は lane を増設しない。
- Issue checkbox は全行 `[ ]` のまま。`[x]` 適用は本 slice の権限外。
