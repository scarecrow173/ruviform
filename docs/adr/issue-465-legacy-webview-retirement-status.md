---
id: "ADR-mNxXAe50ct0jI4imZUBwuT"
type: "adr"
status: "proposed"
title: "Issue #465 旧 WebView 実装群を legacy/preview に分類し物理撤去を別 gate に委ねる"
created: "2026-11-10"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/455"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-465-legacy-webview-inventory.md"
      - type: "doc"
        path: "docs/specs/issue-465-legacy-webview-p0p1.md"
      - type: "doc"
        path: "docs/designs/issue-465-legacy-webview-p0p1.md"
      - type: "doc"
        path: "docs/plans/implement-issue-465-legacy-webview-p0p1.md"
      - type: "doc"
        path: "docs/tasks/implement-issue-465-legacy-webview-p0p1.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/issue-465-legacy-webview-p0p1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../designs/issue-465-legacy-webview-p0p1.md"
    - "../discovery/issue-465-legacy-webview-inventory.md"
    - "driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../designs/issue-465-legacy-webview-p0p1.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #465 旧 WebView 実装群を legacy/preview に分類し物理撤去を別 gate に委ねる

## Context

Issue #384 の確定仕様 v2 で GUI 方針は Driver-neutral 契約 + GPUI
標準 Driver（仮決定）へ移行したが、旧 WebView 実装（`wry`/`tao`
adapter・`ui/*` TS・Playwright harness・旧 `ruviform-ui` API 面・
関連 mise/CI lane）は consumer が現役利用するため残存している。
Issue #465 はこの残存物の段階的廃止を扱うが、P2 の物理撤去は
両 consumer の製品側撤去（AL #189 / SV #232）と Native 受入を
前提とする（#419 GPUI 採用判定は 2026-11-16 に CONDITIONAL 採用・
status=accepted として決裁済み — More Information 参照）。

現状の問題は「旧実装が現行機能として読める文書・API 記述が残り、
新規実装者が WebView parity を目標と誤認しうる」ことにある。

## Decision

1. 旧 WebView 関連資産を **legacy/preview** に分類する。分類は
   `docs/discovery/issue-465-legacy-webview-inventory.md` を正本とし、
   「現行」「legacy」「共有」の 3 値で記録する。
2. Driver-neutral 契約（`driver`/`view`/`behavior`/`dock`/
   `native_slot`）と GPUI 標準 Driver、`rvf.ipc.v1` の共有 schema、
   `ruviform-ui-test` の driver-neutral lane は**現行**として維持する。
3. 旧 `ruviform-ui` API 面（`HostEvent`/`HostHandle`/`WindowHandle`/
   `WindowConfig`/`RenderTier` DOM 系/`bridge`/`detect`）は doc
   コメントで legacy と移行先を明記する。`#[deprecated]` と物理削除は
   P2 に委ねる。
4. **物理撤去（P2）は本 ADR では決定しない**。別 PR の判断事項と
   する。判定ルールは Issue 本文（2026-11-10 節）の「Legacy
   consumer 利用なし・独立 Oracle PASS・同一 SHA tuple PASS・
   rollback 可能な行だけ消す」+ 順序「製品撤去（AL #189/SV #232）
   → 共通撤去」が正本であり、行別判定の監査結果は棚卸し §5 に
   記録する（2026-11-24 現行化）。
5. WebView を新規 `ViewSpec` の parity 対象にせず、WebView 描画不足を
   GPUI 受入 blocker にもしない（#455 not-planned と整合）。

## Consequences

- Good, because: 旧資産の存続理由と撤去条件が一箇所に集約され、
  新規作業が誤って WebView parity を目標にすることを防げる。
  consumer への破壊的影響なしに移行先が文書化される。
- Bad, because: legacy 表記の維持管理が必要。P2 gate が長期化すると
  旧 API 面の doc コメントが陳腐化するリスクがある（inventory を
  更新して追跡する）。

## Alternatives Considered

### Option A — 即時削除

* Bad, because: 両 consumer が WebView feature/bin・`ui/*` vendored
  asset・`run_multi`/`HostEvent` を現役利用しており、synvelis の
  webview bin は現行 pin で既に HEAD 非互換の API を参照している。
  削除は consumer 移行完了が前提。

### Option B — parity 維持（旧前提の継続）

* Bad, because: #384/#455 で否定済み。2 系統の共通契約第二正本化と
  検証コスト倍増を招く（#384 ADR Option C と同じ理由）。

### Option C — 本 ADR の選択（legacy 分類 + 段階的撤去）

* Good, because: 現行契約を明確化しつつ、consumer 互換を壊さずに
  撤去条件を明示できる。

## More Information

- 棚卸し正本: [inventory](../discovery/issue-465-legacy-webview-inventory.md)（§5 に 2026-11-24 実施の行別削除判定監査）
- 後継関係: 本 ADR は [driver-neutral-gui-gpui-driver-webview-first-issue-384.md](driver-neutral-gui-gpui-driver-webview-first-issue-384.md) の移行表・縮退順序を Issue #465 の P0/P1 として実行するものであり、GPUI 最終採用判断（#262/#419）を代替しない。
- **gate 状態の現行化（2026-11-24 記録）**: #419 は CLOSED — GPUI
  後継 ADR は PR #718（2026-11-16 merge）で status=accepted・
  **CONDITIONAL 採用**として決裁済み（GPUI Native First 方針維持、
  Native Dock の無条件本番採用は不可）。#532 は CLOSED — class A
  根治（方式 A clip-partition・gpui v0.2.2-rf2）、class C
  （bandescape/childinv）は honest Fail 維持。#262・#294・#464・
  #627 もすべて CLOSED。**P2 の残 gate は製品側撤去（AL #189・
  SV #232、ともに OPEN）と、virtual-first L2/L3 証拠経路である
  #630・#631（ともに OPEN）を含む consumer Native 受入**。SV 側は
  PR #618/#619（headless PluginDock panel + doc chain、2026-11-23
  merge）が着地済み。
- 関連 Issue: #257（共通 UI 正本）、#294（cross-repo CI lane）、#455（ViewSpec parity not-planned）、#464（consumer-neutral scenario）、#262/#419（GPUI 採否 gate — 決裁済み）、#532（DComp 制約 — class C residual 記録）、#630/#631（virtual-first L3/L2 証拠経路 — OPEN）。consumer 側の物理撤去 Owner Issue は audio-lens #189・synvelis #232（ともに OPEN、2026-11-09 起票 — RF#465 P1 の per-repo Issue 管理要求への対応）。従来の追跡系列は synvelis #175/#28 系・audio-lens #166/#127 系。
