---
id: "DESIGN-rhqM5KfSVajqQ3vvtVDHol"
type: "design"
status: "approved"
title: "Issue #465 Legacy WebView P0/P1 — 分類ポリシーと文書歴史化の設計"
created: "2026-11-10"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-465-legacy-webview-inventory.md"
      - type: "doc"
        path: "docs/specs/issue-465-legacy-webview-p0p1.md"
      - type: "doc"
        path: "docs/adr/issue-465-legacy-webview-retirement-status.md"
      - type: "doc"
        path: "docs/plans/implement-issue-465-legacy-webview-p0p1.md"
      - type: "doc"
        path: "docs/tasks/implement-issue-465-legacy-webview-p0p1.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/event.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/window.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/ipc.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/bridge.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/detect.rs"
      - type: "doc"
        path: "docs/specs/issue-310-browser-gui-test-harness.md"
      - type: "doc"
        path: "docs/designs/issue-310-browser-gui-test-harness.md"
      - type: "doc"
        path: "docs/test-specs/issue-310-browser-gui-test-harness.md"
      - type: "doc"
        path: "docs/specs/issue-311-ipc-conformance-replay.md"
      - type: "doc"
        path: "docs/designs/issue-311-ipc-conformance-replay.md"
      - type: "doc"
        path: "docs/test-specs/issue-311-ipc-conformance-replay.md"
      - type: "doc"
        path: "docs/specs/issue-312-test-gui-host-lifecycle.md"
      - type: "doc"
        path: "docs/designs/issue-312-test-gui-host-lifecycle.md"
      - type: "doc"
        path: "docs/test-specs/issue-312-test-gui-host-lifecycle.md"
      - type: "doc"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/discovery/README.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
  implements:
    - "../specs/issue-465-legacy-webview-p0p1.md"
  implemented-by: []
  depends-on:
    - "../discovery/issue-465-legacy-webview-inventory.md"
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../designs/issue-384-driver-neutral-gui-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../adr/issue-465-legacy-webview-retirement-status.md"
    - "../adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-465-legacy-webview-p0p1.md"
    - "../discovery/issue-465-legacy-webview-inventory.md"
  derived-by:
    - "../plans/implement-issue-465-legacy-webview-p0p1.md"
    - "../adr/issue-465-legacy-webview-retirement-status.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #465 Legacy WebView P0/P1 設計

[spec](../specs/issue-465-legacy-webview-p0p1.md) と
[棚卸し](../discovery/issue-465-legacy-webview-inventory.md) が確定した
範囲の実施設計。決定の正本は
[ADR: Legacy WebView 撤去の段階的位置づけ](../adr/issue-465-legacy-webview-retirement-status.md)。

## 設計方針

### D1. 分類は 3 値（現行 / legacy / 共有）で記録する

棚卸しの各資産に「現行」「legacy」「共有」の分類を付ける。
「legacy」は**動作維持・新規 parity 目標外・P2 で撤去判断**を意味し、
即時削除を意味しない。`rvf.ipc.v1` のように WebView adapter と
driver-neutral 契約が共有する面は「共有」として区別し、WebView
撤去後も残る部分を明示する。

### D2. 文書歴史化は「banner 注記 + index 表示」で行い、本文は改訂しない

superseded 済み ADR（`ui-architecture-webview-hybrid`・
`ui-tech-selection-poc`）と同じ方式を、まだ `approved` のまま残る
WebView 前提 spec/design/test-spec（Issue #310/#311/#312 系）に適用
する。各文書の H1 直後に `> **歴史/旧方式（legacy）**` の注記を挿入
し、(a) 本文は当時の実装記録として保持、(b) 現行標準は driver-neutral
契約 + GPUI Driver（#384）と driver-lab の Fake/GPUI 検証、(c)
物理撤去は #465 P2 の別判断、を明記する。status は `approved` のまま
維持し、歴史性は banner と本チェーンの索引で識別する（status 値の
変更は graph 契約上の影響が大きいため採用しない）。

対象は「WebView/DOM/Playwright を現行の GUI 検証標準として記述する」
文書に限定する。`rvf.ipc.v1` schema や `ruviform-ui-test` の
driver-neutral lane のように現行でも使う部分は「共有/現行」として
注記内で区別する。

### D3. `ruviform-ui` の整理は doc コメントのみ。`#[deprecated]` は付与しない

旧 API（`HostEvent`/`HostHandle`/`WindowHandle`/`WindowConfig`/
`RenderTier` DOM 系/`bridge`/`detect`）は module doc と item doc に
「legacy — driver-neutral 移行先は `driver` モジュール（Issue #386）
および `native_slot`（Issue #388）」を明記する。`#[deprecated]` は
`ruviform-ui-webview`・`ruviform-ui-test`・両 consumer が現役で使用
しており、`-D warnings` lane を破壊するため付与しない（inventory
§1.2 に判断根拠を記録）。型の移動・削除・シグネチャ変更は行わない。

### D4. consumer は読み取り専用の記録に留める

synvelis・audio-lens の棚卸し結果は inventory §2 に記録するのみ。
両 repo の編集・Issue 起票・pin 更新は本スコープ外（consumer 側の
撤去は各 repo の Issue が owner）。

### D5. `boundary.json` は変更しない

`crates/ruviform-ui-test/fixtures/driver-lab/boundary.json` は
driver 境界 guard の機械可読 ruleset であり、棚卸し記録の置き場では
ない。今回の変更は文書・doc コメントのみで fixture 契約に影響しない
ため、許可されていた編集権は行使しない。

### D6. 行別削除判定は「削除可/保留/例外」の 3 値で inventory に記録する（2026-11-24 追加）

物理削除判定の設計。判定ルールは Issue 本文（2026-11-10 節）の
「Legacy consumer 利用なし・独立 Oracle PASS・同一 SHA tuple
PASS・rollback 可能な行だけ消す」+ 削除順「AL #189 / SV #232 の
製品撤去 → 共通撤去」を正本とし、P0/P1 の 3 値分類（現行/legacy/
共有）・`evidence/rf465` の 4 値（remove_candidate/reuse/
provisional/exception）と以下で対応付ける。

| 判定 | 意味 | 対応する分類 |
| --- | --- | --- |
| 削除可 | 4 条件をすべて満たし、撤去 slice で条件付き実施可能 | remove_candidate のうち consumer 非依存の leaf |
| 保留 | 順序ルールまたは consumer/保持契約依存で現時点では削除不可 | provisional・依存中の remove_candidate |
| 例外 | 削除禁止（共用正本の移転前）または AC の保管方針 | reuse（共用 fixture）・exception（歴史証拠） |

監査は行別に 3 手順で行う: (a) consumer/vendor 参照 grep（両 repo
の `sync-ruviform-ui.mjs` PACKAGES・全ソース参照・lock pin 実測）、
(b) fixture 共有監査（保持契約側からの直接参照 — 例:
`ipc_conformance.rs:24`→`ui/guitest/fixtures`）、(c) 代替 Oracle
実在照合（削除判定の根拠となる driver-neutral/headless lane が
`mise.toml` に実在すること — `test-ui-driver-contract`/
`test-ui-parity` 等。存在しない代替を根拠にしない）。「削除可」は
package 丸ごとの一律判定ではなく、依存・fixture・設定行まで条件を
記録する（例: `ui/browser-test`+`ui/showcase` は `@rvf/showcase` →
`../showcase/dist` 依存のため原子的撤去が条件）。判定結果の正本は
[inventory §5](../discovery/issue-465-legacy-webview-inventory.md)。

## File Map

| ファイル | 変更 |
| --- | --- |
| `docs/discovery/issue-465-legacy-webview-inventory.md` | 新設 — 棚卸し正本 |
| `docs/specs|designs|adr|plans|tasks/issue-465-*.md` | 新設 — 本チェーン |
| `docs/specs|designs|test-specs/issue-310-*.md` `issue-311-*.md` `issue-312-*.md` | legacy banner 注記（9 ファイル） |
| `docs/development-environment.md` | `ui-verify` 等 WebView/DOM lane が legacy である旨の注記 |
| `crates/ruviform-ui/README.md` | 旧 API 面の legacy 分類節を追加 |
| `crates/ruviform-ui/src/{lib,event,window,ipc,render,bridge,detect}.rs` | module/item doc に legacy 分類と移行先を明記 |
| `docs/{specs,designs,discovery,adr,plans,tasks}/README.md` | 生成 index の再生成（`doc_maintenance.js`） |

## リスクと対策

- **banner の文言が現行契約と矛盾する**: 「legacy = 削除予定」ではなく
  「動作維持・parity 目標外・P2 で撤去判断」と定義し、共有面
  （`rvf.ipc.v1`・`ruviform-ui-test` driver-neutral lane）を明示的に
  除外する。
- **doc コメント変更が lint/test を壊す**: コメントのみの変更であり
  `cargo doc` の intra-doc link 切れに注意（`cargo test` の doctest
  と `lint` で検出）。
- **index 再生成の差分が大きい**: `doc_maintenance.js apply` の生成物
  のみを stage し、手編集と混ぜない。

## 検証

- `mise run fmt-check` / `check` / `lint` / `test`
- `mise run check-ui-driver-boundary` / `test-ui-driver-contract`
- `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js plan --json` で doc audit が新規 broken relation を出さないこと

## 現行化記録（2026-11-24）

- gate 状態: #419 CONDITIONAL 採用・status=accepted（PR #718）、#532
  close（class C は honest Fail 維持）、#262/#294/#464/#627 全
  CLOSED。残 gate = AL #189/SV #232（製品撤去）+ #630/#631
  （virtual-first L3/L2 証拠経路、OPEN）+ consumer Native 受入。
- consumer pin 再実測: AL `e4c9f6f0`・SV `cd4213a1`（tuple 分岐）。
  SV PR #618/#619（headless PluginDock panel + doc chain）着地済み。
- 行別削除判定監査（D6）を実施し、判定結果を inventory §5 に記録
  — `ui/browser-test`+`ui/showcase` 条件付き削除可、`ui/guitest/
  fixtures` 削除禁止（移転まで）、`poc/{ui,plugin-gui}` 例外（保持）、
  共通部は順序ルール上すべて保留。本 slice は docs/監査記録のみで
  物理削除は対象外。
