---
id: "ADR-mNxXAe50ct0jI4imZUBwuT"
type: "adr"
status: "proposed"
title: "Issue #465 旧 WebView 実装群を legacy/preview に分類し物理撤去を別 gate に委ねる"
created: "2026-11-10"
updated: "2026-11-10"
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
#419（GPUI 採用判定）と両 consumer の Native 受入を前提とする。

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
4. **物理撤去（P2）は本 ADR では決定しない**。#419 採用判定 +
   consumer Native 受入の成立を gate とする別 PR の判断事項とする。
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

- 棚卸し正本: [inventory](../discovery/issue-465-legacy-webview-inventory.md)
- 後継関係: 本 ADR は [driver-neutral-gui-gpui-driver-webview-first-issue-384.md](driver-neutral-gui-gpui-driver-webview-first-issue-384.md) の移行表・縮退順序を Issue #465 の P0/P1 として実行するものであり、GPUI 最終採用判断（#262/#419）を代替しない。
- 関連 Issue: #257（共通 UI 正本）、#294（cross-repo CI lane）、#455（ViewSpec parity not-planned）、#464（consumer-neutral scenario）、#262/#419（GPUI 採否 gate）、consumer 側は synvelis #175/#28 系・audio-lens #166/#127 系。
