---
id: "SPEC-HvasimHE9mJVFN1ay6D68D"
type: "spec"
status: "proposed"
title: "Issue #465 Legacy WebView 基盤の段階的廃止 P0/P1 — 文書歴史化と棚卸し"
created: "2026-11-10"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/455"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-465-legacy-webview-inventory.md"
      - type: "doc"
        path: "docs/designs/issue-465-legacy-webview-p0p1.md"
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
    generated: []
  implements: []
  implemented-by:
    - "../designs/issue-465-legacy-webview-p0p1.md"
    - "../plans/implement-issue-465-legacy-webview-p0p1.md"
  depends-on:
    - "../discovery/issue-465-legacy-webview-inventory.md"
    - "../specs/issue-384-driver-neutral-gui-architecture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "../designs/issue-384-driver-neutral-gui-contract.md"
    - "../adr/issue-465-legacy-webview-retirement-status.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-465-legacy-webview-inventory.md"
  derived-by:
    - "../designs/issue-465-legacy-webview-p0p1.md"
    - "../adr/issue-465-legacy-webview-retirement-status.md"
    - "../plans/implement-issue-465-legacy-webview-p0p1.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #465 Legacy WebView 基盤の段階的廃止 P0/P1 — 文書歴史化と棚卸し

## 背景

GUI 標準方針は Issue #384（2026-11-07 確定仕様 v2）で
**Driver-neutral 契約 + GPUI 標準 Driver** へ移行した。しかし旧
WebView-first 前提の記述・資産が docs・`ruviform-ui` API・`ui/*`・
PoC・mise/CI lane・両 consumer（synvelis・audio-lens）に残り、
「両 driver を parity 維持する」旧前提と現行契約が混在している。

Issue #465 はこの旧基盤の**段階的廃止・撤去**を 3 repo で整合させる
上位 Issue であり、P2（物理撤去）は consumer 製品側撤去（AL #189 /
SV #232）と Native 受入を gate とする別判断である。#419 GPUI 採用
判定は 2026-11-16 に CONDITIONAL 採用（#384 ADR status=accepted・
PR #718）として決裁済み。本 spec はその **P0（文書歴史化・legacy
明記）と P1（API 棚卸し・consumer 追跡）のみ**を対象とし、P2 は
残 gate 未達のため実施しない。

## 目的

- 旧「両 driver parity 必須」前提の記述を歴史/旧方式として分類し、
  現行の driver-neutral + GPUI 標準 Driver 方針を冒頭で明確化する。
- `legacy`/`preview` 扱いの資産を一覧化し、撤去対象・移行先・
  consumer 影響・撤去 owner を記録する。
- `ruviform-ui` の旧 API 面を棚卸しし、driver-neutral への移行先を
  文書として決定する（実装変更は非破壊的な doc コメント整理のみ）。
- `webview`/`ui/`/Browser(Playwright)/`mise`/Cargo/CI/README の
  利用者を追跡し、consumer 両 repo の現状を読み取り専用で記録する。

## 受入条件

| # | 条件 | 結果の記録先 |
| --- | --- | --- |
| A1 | 旧「両 driver parity」前提を含む docs が歴史/旧方式として識別可能であり、現行契約が driver-neutral + GPUI 標準 Driver であることが明記されている | 本チェーンの docs + 各対象 doc への注記 |
| A2 | legacy 資産の一覧（crate・API・`ui/*`・PoC・mise・CI・lockfile・README）が作成され、各項目に分類・移行先・撤去 owner が記録されている | `docs/discovery/issue-465-legacy-webview-inventory.md` |
| A3 | `ruviform-ui` 旧 API（`eval`/`url`/`devtools`/`Ready{url}`/`Message(String)`/`RenderTier` DOM 系/`rvf.ipc.v1`/bridge/detect）が棚卸しされ、共有 vs 撤去候補が分類され、移行先が決定記録されている | 同上 §1.2/§3 + `crates/ruviform-ui` doc コメント |
| A4 | consumer 両 repo（synvelis・audio-lens）の WebView 資産・利用 API・lock pin・影響が読み取り専用で記録されている | 同上 §2 |
| A5 | P2 が残 gate（AL #189/SV #232 製品撤去 + consumer Native 受入）未達で NotRun/対象外であること、Issue #465 が Open 維持であることが明記されている | 本 spec・ADR・PR 本文 |

## 非目標

- `crates/ruviform-ui-webview`・`ui/`・`poc/`・旧 API 型・Cargo
  member・CI lane の**物理削除・除外**（P2、別 PR）。
- consumer repo（synvelis・audio-lens）の**編集**（読み取りのみ）。
- 旧 API の移動・改名・`#[deprecated]` による warning 発生
  （consumer・adapter が現役利用中のため P1 では付与しない —
  判断根拠は inventory §1.2）。
- 新規機能・dependency・feature・unsafe の追加。
- WebView を新規 `ViewSpec` の parity 対象にすること、または
  WebView 描画不足を GPUI 受入 blocker にすること（#455
  not-planned と整合）。

## 制約

- 編集範囲: `docs/**`、`crates/ruviform-ui/**`（doc コメント等の
  非破壊整理のみ）、`crates/ruviform-ui-test/fixtures/driver-lab/
  boundary.json`（必要時のみ — 今回は変更不要と判断）。
- 検証は `mise.toml` タスクが正本。doc/comment のみの変更のため
  `fmt-check`/`check`/`lint`/`test` と driver 境界 lane を実行する。
- 旧文書は削除せず、履歴証拠として保持したまま分類注記を付ける。

## 現行化記録（2026-11-24）

P0/P1 納品（PR #475・#535）以降の状態変化を反映する台帳現行化 slice
の記録。分類・削除判定の仕様面を現行値へ更新したもので、本 spec の
P0/P1 受入条件（A1〜A5）自体は変更しない。

- **再実測値**: consumer lock pin は AL `e4c9f6f0`・SV `cd4213a1`
  （各 repo `Cargo.lock` 実測 — 記録値 `54439fdc`/`79d1db0a` および
  evidence/rf465 の共通値 `fd1d4d5f` から遷移し、tuple は 2 値に
  分岐）。ruviform `origin/main` = `fc355d5c`。
- **gate 変化**: #419 CLOSED（CONDITIONAL 採用・PR #718）・#532
  CLOSED（class A 根治・class C は honest Fail 維持）・#262/#294/
  #464/#627 すべて CLOSED。残 gate = AL #189/SV #232（OPEN）+
  #630/#631（virtual-first L3/L2 証拠経路・OPEN）+ consumer Native
  受入。SV PR #618/#619 着地済み。
- **行別削除判定監査**: 判定ルール「Legacy consumer 利用なし・独立
  Oracle PASS・同一 SHA tuple PASS・rollback 可能」+ 順序「製品
  撤去 → 共通撤去」に照らし、(a) consumer/vendor 参照 grep、
  (b) fixture 共有監査、(c) 代替 Oracle 実在照合の 3 手順で行別
  監査を実施。判定結果（削除可/保留/例外+理由）の正本は
  [inventory §5](../discovery/issue-465-legacy-webview-inventory.md)
  — `ui/browser-test`+`ui/showcase` は条件付き削除可、`ui/guitest/
  fixtures` は移転完了まで削除禁止、その他共通部は順序ルール上
  すべて保留（物理削除は対象外・次 slice 以降）。
