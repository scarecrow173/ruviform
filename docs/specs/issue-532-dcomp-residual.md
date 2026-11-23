---
id: "SPEC-034ZccDcompResidual01"
type: "spec"
status: "approved"
title: "Issue #532 DComp residual — 残存 Fail の class 切り分けと処遇決定（docs-only disposition）"
created: "2026-11-21"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/839"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-dcomp-residual.md"
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-residual.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-residual.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-band-escape.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-child-inventory.md"
      - type: "file"
        path: "evidence/rf532/residual-disposition.md"
    modified:
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-pixel.md"
      - type: "file"
        path: "evidence/rf532/pinned-dep-results.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-839-gpui-dcomp.md"
    - "docs/specs/issue-532-rasterfix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/test-specs/issue-532-dcomp-residual.md"
    - "docs/specs/issue-839-gpui-dcomp.md"
    - "docs/discovery/issue-839-gpui-dcomp.md"
    - "evidence/rf532/pinned-dep-results.md"
    - "evidence/rf532/residual-disposition.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-839-gpui-dcomp.md"
  derived-by:
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "docs/test-specs/issue-532-dcomp-residual.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-residual.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 DComp residual — 残存 Fail の class 切り分けと処遇決定

Issue #839 の根治 fix（gpui fork `v0.2.2-rf2`）統合後、pinned-dep
real lane に残る 3 つの既知 Fail について、class の確定と処遇を
決裁する residual disposition slice。本 slice は **docs/evidence
のみ** — verdict 意味論を変える `.rs` 変更は行わない。class
切り分けの根拠は `docs/discovery/issue-532-dcomp-residual.md`、
AC 行との突合表は `docs/test-specs/issue-532-dcomp-residual.md`
を参照。Issue #532 の Close 判定は本 slice では行わず、
Coordinator が AC 照合で最終判断する。

## 処遇決定（確定）

| case | class | 決定 | 根拠 |
| --- | --- | --- | --- |
| `real-dcomp-pixel` | **B — HWND レベル外部遮蔽**（lane が spawn する別 top-level `Zed::Window` が docked child の screen rect を被覆。DComp plane class とは別） | **別 Issue 切り出し**（起票案は PR 本文）。#532 内では class B の既知 Fail として記録 — lane window 配置の問題であり、#532 が責務を持つ DComp 合成経路の不具合ではない | pinned JSONL `occlusion_hit_test`（全 probe point が hwnd `160105518` `Zed::Window` `184,89,1552x903` role `other`）+ `windowdc`/`printwindow` green `1.0` + w1 内 z-order 正常 |
| `real-dcomp-bandescape` | **C — `WS_EX_LAYERED` 適用不能の確定診断**（棄却済み fix 経路の可否判定 probe） | ~~既知 Fail として継続~~ → **Issue #917 で verdict 契約を意図的改訂（re-scope）** — as-child layered 拒否を「確定機序どおりの期待値」として `verdict: note` の診断行に転換し、required op 完遂で `Pass` に収束。診断目的達成済みで製品修正の対象は残っていないが、lane exit 1 常態化の解消を目的に #917 が本決定を再審査・更新した（下記「Issue #917 による class C 収束」） | `be_apply_layered_exstyle`/`be_apply_layered_lwa` refused、silent no-op + error 87（`evidence/rf532/band-escape-results.md`） |
| `real-dcomp-childinv` | **C — 同上**（child-level で同一機構を確定） | ~~既知 Fail として継続~~ → **Issue #917 で verdict 契約を意図的改訂（同上）** | `ci_apply_layered_*` refused、`latched_as_child=false`/`latched_as_toplevel=true`（`evidence/rf532/child-inventory-results.md`）、`WS_CHILD\|WS_EX_LAYERED` 作成 `0x80070006`（issue-839 discovery） |

### 決定の補足

- **verdict の昇格は行わない（本 slice の範囲 — Issue #917 での
  意図的改訂は下記参照）。** 3 case の `fail` はいずれも実測に
  基づく honest 記録で、disposition は「別 class 確定」の記録で
  あって verdict の書き換えではない。Fail→Pass への改竄、Fail→
  `Unsupported` への再基準化はいずれも本 slice では実施しない。
- **`Unsupported` re-baseline を採らない理由（class C）**:
  POLICIES.md §3 の `Unsupported` は「根拠を伴う技術的非対応」
  であり、`WS_EX_LAYERED` の child への非適用性を理由に
  re-baseline することは論理的には可能。ただし (a) 両 case は
  probe として正常完走しており refused op の事実記録は各
  test-spec の verdict 契約（「必須 mutation 拒否 = fail」）の
  設計どおりで、case 自体は unsupported ではない、(b)
  re-baseline は `real_dock.rs`/`real_catalog.rs` の verdict
  意味論変更を要し、fail-closed mapping・console 行数・headless
  非回帰の再検証を伴う — docs-only slice の範囲を超える。
  Issue #917 の再審査でもこの分析は維持され、entry-gate
  `Unsupported` 化は採られなかった（下記）。
- **class C の将来選択肢 → Issue #917 で実施済み**:
  本 slice が記録した選択肢のうち、「拒否を期待結果として
  検証する re-scope」を Issue #917 が採用・実装した。
  `be_apply_layered_{exstyle,lwa}`/`ci_apply_layered_{exstyle,lwa}`
  は `record_expected_refusal_row`（`verdict: note` +
  `expected_outcome` + `contradicts_confirmed_mechanism` flag）
  の期待拒否診断行へ転換され、refusal の `applied`/
  `win32_error`/`after`/`latched` 記録は維持される — Fail→Pass
  の改竄ではなく期待値検証への re-scope。frame/repaint/
  reparent/retry/restore の required op は strict のまま、
  reparent 後 top-level での `*_retry_*` op も strict を維持する
  （top-level layered は文書化された supported 経路であり、
  拒否は確定機序と矛盾しない異常として `Fail` 扱い）。as-child
  latch 成功は確定機序と矛盾する OS 依存の新規 data として
  flag 記録し Pass を妨げない — verdict は OS 実装詳細ではなく
  probe 契約の完遂を計る。entry-gate `Unsupported` 化は採らな
  かった: case 自体は unsupported ではなく（上記分析どおり）、
  entry-gate 化は per-run の refusal evidence を消す。「既知
  Fail として継続」も採らなかった: lane exit 1 の常態化が
  Issue #917 の発端そのものであり、診断目的達成後の恒久的な
  lane fail に継続の理由がない。
- **Issue #917 の収束確認**: verdict 収束後の canonical 帳簿は
  `evidence/rf631/plugin-dock-real-scenario.jsonl`（interactive
  lane 再実走で再生成）。2 case が収束 verdict を返し lane
  exit code が 0 になることを AC3 として確認する。
- **`real-dcomp-pixel` の `detail` 文字列の扱い**: JSONL の
  `detail` は旧文面 "consistent with the known GPUI DComp
  topmost occlusion" のまま残る。確定 class（外部 window 遮蔽）
  への文言追従は verdict/detail 生成コードの変更を伴うため、
  別 Issue 側の対応に含める。

## #532 AC 行への対応付け（要約 — 突合表は test-spec 側）

| AC 行 | 対応付け | 状態 |
| --- | --- | --- |
| Virtual-first移行受入（Regression 固定・採用方式で画素/Focus/Resize/ReDock/RT を修正後に実 Windows GUI Pass・既知 Fail 維持） | regression fixture = `real-dcomp-pixel`（Fail として固定済み）。採用方式（fork clip-partition）で `real-dcomp-rasterfix` baseline `0.999994` Pass、Focus=`real-editor-focus`/`real-multiwindow-focus`、Resize=`real-editor-resize`、ReDock=`real-editor-redock` 各 Pass。RT は rationale 記録（discovery 側）。既知 Fail 3 件は class B/C として維持 | **実質充足**（残存は class B の別 Issue 化 — 最終判定は Coordinator） |
| 制約・未成立の記録（済） | #388/PR #501 記録済み + #839 doc chain で根治の証拠を追加 | 充足（既に `[x]`） |
| 現行 L1 契約・Compile/Headless・公開 API/RT 非回帰の CI 対応付け | pinned run の headless lanes（console/headless/boundary/fmt-check）Pass。merge SHA `5f22bee6` の 3OS CI green は Coordinator gate | 部分充足（CI 証拠待ち = NotRun pending） |
| L2/L3 再開条件の記録 | 下記「L2/L3 再開条件」節に記録 | 充足（記録形態） |
| Windows GUI-L2/EXT-L3 実走 | #630 test_editor 実 ABI child HWND + #631 real-window/Pixel oracle で DComp 再現→修正を実走。再現 = baseline Fail 系証拠（`evidence/rf532/` 各 bisect/uncovered 記録）、修正 Pass = `real-dcomp-rasterfix` baseline。既知 DComp 画素 Fail は維持（class B/C 記録付き） | **実質充足**（同上） |
| macOS/Linux L1 のみ必須 | 対象 SHA の 3OS CI で検証 — Coordinator gate。実 GUI/実 Plugin は Deferred のまま | NotRun（Coordinator gate） |
| 検証 Scope の明記 | test-spec の Scope 表に記録。新規 CI Workflow なし | 充足 |

## L2/L3 再開条件の記録（AC 行対応）

Issue #532 本文の「L2/L3 利用要件が発生した場合の再開条件」と
方式 A/B/C の調査結果を、現時点の確定値として記録する:

- **対象 OS**: Windows のみ（macOS/Linux の合成問題は本 Issue
  対象外確定）。再開が必要になるのは Windows 上で新たな
  Native Dock 機能・Plugin 形式・host mode を提供開始するとき。
- **実 Plugin**: #630 `test_editor`（正規 ABI Test Plugin、
  実 child HWND）を標準 fixture とする。市販 Plugin は通常必須
  でない（POLICIES.md §2.1）。市販 Plugin 固有の問題が報告され
  た場合のみ個別 case を定義する。
- **Window/DPI**: GPUI 0.2.2 fork（`v0.2.2-rf2` 以降）、DComp
  on、scale_factor 1.0 で実測済み。DPI != 1.0、multi-monitor
  mixed-DPI、windowed/fullscreen 遷移は未検証 — 利用要件発生時に
  case 化する。
- **対応目的（再開 Trigger）**: (a) Native Dock を利用する新
  Windows 機能の提供開始、(b) `real-dcomp-pixel` の class B
  遮蔽解消後の pixel 経路受入、(c) class C probe の re-scope
  判断、(d) macOS/Linux で Native Dock 利用要件が発生した場合の
  L2/L3 昇格。
- **測定環境**: interactive Windows desktop session
  （`test-ui-plugin-dock-real` lane）、oracle =
  `win32-bitblt-screen` screen green `>= 0.5`。
- **方式 A/B/C の調査結果（確定）**: A = GPUI 最小修正 →
  **採用・実証済み**（fork clip-partition + noredirectionbitmap
  除去、baseline `0.999994`）。B = Windows private
  compositor/native attachment → A が成立したため未使用（調査の
  必要性消滅）。C = 代替合成経路 → 同上。棄却済み候補
  （`CreateSurfaceFromHwnd` bind・`WS_EX_LAYERED` 化・blit-model
  swapchain）の根拠は issue-839 discovery に確定済み。
- **担当面の状態**: #388/#389 は OPEN のまま統合受入 owner、
  SV #176 は cross-repo の製品側受入。#262/#419 は CLOSED —
  下記「採否 handoff 形態」参照。

## 採否 handoff 形態の決裁提案

Issue #532 本文は「採否は#262に渡す」「#262 の採用判定の必須
入力」「失敗処理: #388→#262→#419 へ報告」と定めるが、**rf#262
と rf#419 はともに CLOSED**（2026-11-21 時点、`gh issue view`
で確認）であり、closed issue への「必須入力」引き渡しは形態とし
て成立しない。

提案する形態（採否は Coordinator が決裁）:

- **方式 A（GPUI fork 最小修正）の採用実績は本 residual doc
  chain を正本とする** — `docs/discovery/issue-839-gpui-dcomp.md`
  の機構確定・`evidence/rf532/clipfix-results.md`/
  `evidence/rf532/pinned-dep-results.md` の実測・本 chain の
  class 切り分けが、#262 が求めた「採用判定の必須入力」に相当
  する情報をすべて含む。
- **最終採用決裁は #532 本体で行う** — #262 CLOSED により
  上流 Gate は消滅したと解し、#532 の AC 照合時に Coordinator
  が方式 A 採用を確定する形態を提案。新規 ADR / 新 Issue への
  分離は、#532 Close 後に「GPUI 方式の製品適用範囲を再評価する
  要件」が発生した時点で切り出す。
- **#388/#389 への引き渡し** — NativeSurfaceSlot 契約・Plugin
  Editor Session 側は OPEN のままであり、Native Dock の製品利用
  受入（残 Case: resize/clip/focus/re-dock/hang 等）は引き続き
  従来の担当線で扱う。本 slice はその入力として class A 解消の
  証拠と class B/C の残存状態を提供する。
- **SV #176 への伝播** — cross-repo のため本 slice では記録のみ
  （Coordinator/owner 側で伝播要否を判断）。

## スコープ外（本 slice で扱わない）

- `real_dock.rs`/`real_catalog.rs` 等の verdict 意味論変更、
  `detail` 文言更新、case re-scope（docs-only disposition のため
  全て後続）。
- real lane の再実走（interactive session が必要で、docs-only
  disposition では不要 — 実測は pinned JSONL を正本とする）。
- popup test 失敗（`crates/ruviform-plugin-test-vst3/src/
  editor_view.rs:798`、non-Windows `po:2` 未記録）— rf#630 領域。
- `lint-ui-gpui` の `cast_sign_loss`（`surface.rs:1644`）— main
  既存問題。
- merge SHA の 3OS CI green 待ち、docs index 整備、新規 Issue
  実起票・GitHub 依存登録・Issue #532 本文編集・Close —
  すべて Coordinator。
