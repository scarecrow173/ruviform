---
id: "TSPEC-034ZccDcompResidual01"
type: "test-spec"
status: "approved"
title: "Issue #532 DComp residual — pinned-dep verdict 集合と AC 各行の突合"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/839"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-residual.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/test-specs/issue-839-gpui-dcomp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
    - "docs/test-specs/issue-532-dcomp-band-escape.md"
    - "docs/test-specs/issue-532-dcomp-child-inventory.md"
    - "evidence/rf532/residual-disposition.md"
    - "evidence/rf532/pinned-dep-results.md"
    - "evidence/rf532/plugin-dock-real-pinned.jsonl"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-residual.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-532-dcomp-residual.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 DComp residual — pinned-dep verdict 集合と AC 各行の突合

`docs/specs/issue-532-dcomp-residual.md` の residual disposition を
検証する文書。pinned-dep real lane（`57c4619b` + gpui
`v0.2.2-rf2`、interactive Windows・DComp on）の verdict 集合を
Issue #532 の現行 AC 各行へ突合する。class 切り分けの根拠は
`docs/discovery/issue-532-dcomp-residual.md`、実測値の正本は
`evidence/rf532/plugin-dock-real-pinned.jsonl` と
`evidence/rf532/residual-disposition.md`。

## 検証対象の verdict 集合（pinned-dep run 実測）

JSONL の case rows 28 件: **22 pass + 3 unsupported + 3 fail**。

| case | verdict | class / 備考 |
| --- | --- | --- |
| `real-dcomp-rasterfix` | pass | class A 解消の中核証拠 — `fix_arm="baseline"`、screen green `0.999994`、docked-in-place |
| `real-dcomp-pixel` | fail（既知） | **class B** — 別 top-level `Zed::Window` の HWND 遮蔽（別 Issue 提案） |
| `real-dcomp-bandescape` | fail（既知） | **class C** — `WS_EX_LAYERED` 適用不能の確定診断（既知 Fail 継続） |
| `real-dcomp-childinv` | fail（既知） | **class C** — 同上 |
| `real-window-ime` / `real-session-crash` / `real-session-hang` | unsupported | 既知（adapter API なし / 未実装 fixture） |
| その他 21 case | pass | `real-window-mount`/`real-native-slot`/`real-editor-attach`/`real-capture`/`real-dcomp-foreground`/`real-dcomp-present`/`real-dcomp-uncovered`/`real-multiwindow-focus`/`real-os-input`/`real-command-receipt`/`real-window-dpi`/`real-window-close`/`real-editor-resize`/`real-editor-redock`/`real-session-terminate`/`real-session-degraded`/`real-session-recovery`/`real-session-timeout`/`real-editor-visibility`/`real-editor-input`/`real-editor-focus` |

注: `evidence/rf532/pinned-dep-results.md` 要約行の「24 pass」は
同 doc の表（22 pass 列挙）および JSONL 実測と不整合。本表は
JSONL 実測を正本とする（reconciliation は
`evidence/rf532/residual-disposition.md` に記録）。

## AC 行 × 証拠 突合表

| AC 行（Issue #532 現行受入条件） | 対応する証拠 | 判定 |
| --- | --- | --- |
| Virtual-first移行: DComp 実 Window 画素問題を Test Native Editor で再現する Regression を固定 | `real-dcomp-pixel` case が regression fixture として Fail を継続記録（baseline→clipfix→pinned の全 run で再現）。`evidence/rf532/` の uncovered/presentation/childinv/bandescape 各 bisect 記録 | 充足 |
| 〃: 採用方式で画素を修正後に実 Windows GUI で Pass | `real-dcomp-rasterfix` = Pass（`fix_arm="baseline"`、screen `0.999994`、治療 arm 不使用・child exstyle `0x0`）。`evidence/rf532/plugin-dock-real-pinned.jsonl` | 充足 |
| 〃: Focus / Resize / ReDock を修正後に実走 | `real-editor-focus` / `real-multiwindow-focus` / `real-editor-resize` / `real-editor-redock` = 全 pass（同一 pinned run） | 充足 |
| 〃: RT | lane に明示 RT case なし — `docs/discovery/issue-532-dcomp-residual.md` の RT 非干渉 rationale（fix は UI thread の presentation layer のみ・audio RT と共有 state/lock なし） | 充足（rationale 記録） |
| 〃: 既知 Fail は修正済み証拠が得られるまで残す | 3 既知 Fail を class B/C として維持（`docs/specs/issue-532-dcomp-residual.md` の処遇決定）。verdict 昇格・偽装なし | 充足（class B は別 Issue 提案） |
| 制約・未成立の記録を #388/PR #501 に記録 | 済（Issue 側 `[x]` 済み）。根治の証拠は #839 doc chain + pinned-dep-results | 充足 |
| 現行 L1: Fake/NativeSurfaceSlot 契約・Compile/Headless・公開 API/RT 非回帰を既存 CI の同候補 SHA に対応付け | pinned run の headless lanes: `test-ui-plugin-dock`/`test-ui-plugin-dock-console`/`test-ui-plugin-dock-real-headless`/`check-ui-driver-boundary`/`test-ui-gpui`/`test-ui-native-slot`/`fmt-check` 全 pass（`evidence/rf532/pinned-dep-results.md` lane 表）。merge SHA `5f22bee6` の 3OS CI green = **Coordinator gate** | 部分充足（統合 CI 証拠は NotRun pending） |
| L2/L3 再開条件と方式 A/B/C 調査手順・担当の記録 | `docs/specs/issue-532-dcomp-residual.md`「L2/L3 再開条件」節（対象 OS・実 Plugin・Window/DPI・目的・測定環境・A/B/C 確定結果・担当状態） | 充足（記録形態） |
| Windows GUI-L2/EXT-L3: #630 実 child HWND + #631 GPUI 実 Window/Pixel Oracle で DComp 問題の再現・修正/画素・Focus・Resize・ReDock を実走 | 再現 = `evidence/rf532/` 一連の baseline Fail 証拠（DComp on で screen green 0.000）。修正/画素 = `real-dcomp-rasterfix` Pass。Focus/Resize/ReDock = 上記 4 case Pass。市販 Plugin 不使用（正規 ABI test_editor） | 実質充足（class B の残存は別 Issue） |
| 〃: 既知の DComp 画素 Fail は修正後の実 Windows 証拠が得られるまで維持。Fake/Offscreen/Floating は代替にならない | 3 既知 Fail 維持 + class 確定記録。verdict は全て実 Window 実測由来（`environment_kind="real-window"`）で代替なし | 充足 |
| macOS/Linux L1 のみ必須 | 本 slice は docs-only で macOS/Linux 実走なし。対象 SHA の 3OS CI = **Coordinator gate**。実 GUI/物理 Device/実 Plugin は Deferred 継続 | NotRun（Coordinator gate） |
| 検証 Scope 明記・新規 CI Workflow なし | 下記 Scope 表。Workflow 追加なし | 充足 |

## 検証 Scope（AC 行対応の明記）

| 項目 | 値 |
| --- | --- |
| 採用/利用する機能 | Windows Native Plugin Dock（NativeSurfaceSlot → 実 child HWND docking）、DComp on 経路 |
| OS / arch / session | Windows x86_64、interactive desktop（`environment_kind="real-window"`） |
| Device / GPU | NVIDIA GeForce RTX 4070 SUPER（DISPLAY1 primary、multi-monitor） |
| Plugin / Format | `test_editor`（#630 正規 ABI Test Plugin、VST3 実バイナリ、実 child HWND）。`ruviform_plugin_test_vst3.dll` sha256 `8ca50ea7…` |
| GPUI / DComp | fork `v0.2.2-rf2`（`b3144f2`）、`gpui_disable_direct_composition=null`（DComp on） |
| Oracle | `win32-bitblt-screen` screen green `>= 0.5`（独立 oracle。`PrintWindow`/window-DC は診断 note のみ） |
| Run | ローカル interactive lane `mise run test-ui-plugin-dock-real`（CI 非収載 lane — Run URL は存在しない。証拠は `evidence/rf532/plugin-dock-real-pinned.jsonl`） |
| 新規 CI Workflow | なし |

## 本 slice の検証 lane 実施記録

docs-only disposition のため、必須は軽量チェックのみ
（POLICIES.md §5「文書・ポリシーのみの変更は軽量な衛生チェック」）。

| lane | 実行 | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 実施 | pass（exit 0） |
| `prek run --all-files` | 実施 | pass — trailing whitespace / end-of-file / YAML / TOML / mixed EOL / BOM / private-key / actionlint 全 pass（exit 0） |
| `mise run test-ui-plugin-dock-console` / `test-ui-plugin-dock` / `test-ui-plugin-dock-real-headless` / `check-ui-driver-boundary` | NotRun | `.rs` 変更なしのため非該当（verdict 意味論の変更を伴わない docs-only） |
| `mise run test-ui-plugin-dock-real` | NotRun | docs-only disposition のため real lane 再実走は範囲外（実測正本 = pinned JSONL `57c4619b`） |

## verdict の誠実性チェック

- 3 既知 Fail は verdict の昇格・偽装を行っていない — disposition
  は「別 class 確定」の記録であり、JSONL の verdict 値自体は
  変更しない。
- `real-dcomp-pixel` の class B 確定は `occlusion_hit_test` の
  measured fact（全 probe point が同一外部 top-level window を
  返す）に基づく。遮蔽除去後の挙動は未実測で、follow-up Issue
  側の検証対象として分離する。
- class C の Fail 継続は「実現不能が証明済み」の根拠
  （silent no-op・error 87・`0x80070006`・
  `latched_as_toplevel`）を引用した honest 記録であり、case の
  verdict 契約（必須 op 拒否 = fail）とも整合する。
- `real-dcomp-pixel` の `detail` 文字列（旧原因文面）と確定
  class の不整合は discovery/spec の両 doc に明記済み。文言追従は
  verdict/detail 生成コードの変更を伴うため後続作業とする。
