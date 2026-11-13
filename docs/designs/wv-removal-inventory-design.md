---
id: "DESIGN-ZwNUJSnzzVrmRrqQ7S6lJ9"
type: "design"
status: "approved"
title: "Issue #465 WebView 移行台帳現行化 — 4 分類スキームと証拠設計"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/465"
  changes:
    added:
      - type: "file"
        path: "evidence/rf465/inventory.json"
      - type: "file"
        path: "evidence/rf465/README.md"
      - type: "doc"
        path: "docs/specs/wv-removal-inventory.md"
      - type: "doc"
        path: "docs/plans/wv-removal-inventory-plan.md"
      - type: "doc"
        path: "docs/tasks/wv-removal-inventory-task.md"
      - type: "doc"
        path: "docs/test-specs/wv-removal-inventory.md"
      - type: "doc"
        path: "docs/impl/ir/wv-removal-inventory.md"
      - type: "doc"
        path: "docs/impl/exp/wv-removal-inventory.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
  implements:
    - "../specs/wv-removal-inventory.md"
  implemented-by: []
  depends-on:
    - "../specs/wv-removal-inventory.md"
    - "../discovery/issue-465-legacy-webview-inventory.md"
    - "../designs/issue-465-legacy-webview-p0p1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/wv-removal-inventory.md"
    - "../discovery/issue-465-legacy-webview-inventory.md"
    - "../adr/issue-465-legacy-webview-retirement-status.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/wv-removal-inventory.md"
    - "../discovery/issue-465-legacy-webview-inventory.md"
  derived-by:
    - "../plans/wv-removal-inventory-plan.md"
    - "../tasks/wv-removal-inventory-task.md"
  verifies: []
  verified-by: []
  references:
    - "evidence/rf465/inventory.json"
  defers: []
  deferred-by: []
---

# Issue #465 WebView 移行台帳現行化 — 設計

[spec](../specs/wv-removal-inventory.md) が確定した範囲の設計。
P0/P1 の
[棚卸し](../discovery/issue-465-legacy-webview-inventory.md)と
[設計](../designs/issue-465-legacy-webview-p0p1.md)を前提とし、
Issue #465 の現行受入条件（現行 main 基準の台帳化・4 分類・
Owner/Trigger 記録）を満たす成果物構成を定める。

## 設計方針

### D1. 分類は 4 値 — `remove_candidate` / `reuse` / `provisional` / `exception`

P0/P1 の 3 値（現行/legacy/共有）を、撤去判断に直結する 4 値へ拡張する。

| 分類 | 定義 | 典型例 |
| --- | --- | --- |
| `remove_candidate` | WebView 固有で共有意味論を持たない資産。P2 gate 成立後の撤去候補 | `ruviform-ui-webview` crate、`ui/showcase`、`ui/browser-test`、`poc/*` コード、webview mise/CI lane、consumer webview bin/assets |
| `reuse` | driver-neutral 契約・共有 schema・独立資産。撤去後も残る正本 | `rvf.ipc.v1`、`driver`/`view`/`behavior`/`dock`/`native_slot`、`ruviform-ui-gpui`、`ruviform-ui-test` の driver-neutral lane、Scenario(#464)/Golden(#314)、`ruviform-viz`、`ui/guitest/fixtures` |
| `provisional` | legacy だが現役 consumer・test harness・共有 fixture が依存。Owner+退役 Trigger を記録して維持 | `ruviform-ui` 旧 API 面、`ui/{core,components,viz,binding,guitest}` TS 実装、FakeHost、consumer webview bin/dep/lane（AL#189/SV#232 完了まで） |
| `exception` | legacy を意図的に保持する資産（歴史証拠・明示的保持方針） | `poc/*/RESULTS.md`・選定 ADR、superseded 済み WebView ADR/旧 GUI テスト文書 |

分割の原則: **package/dir 単位ではなく「削除した場合に壊れる契約」
単位で分類する**。`ui/guitest` は fixtures（`reuse`）と TS 実装
（`provisional`）に分離し、`poc/` はコード（`remove_candidate`）と
証拠文書（`exception`）に分離する。`crates/ruviform-ui/tests/
ipc_conformance.rs` のように schema 部（`reuse` 相当）と
`rvf://data` 部（legacy）が同居する対象は entry の `notes` に
検証対象単位の内訳を記録する。

### D2. 台帳の正本は `evidence/rf465/inventory.json`（機械可読）

- `entries[]`: 資産単位（crate/package/bin/lane/api/fixture/docs）。
  各 entry は `id`・`repo`・`area`・`path`・`kind`・`summary`・
  `classification`・`webview_specific`・`consumers`・`evidence`・
  `owner`・`retirement_trigger`・`diff_vs_p0p1`・`notes` を持つ。
- `gates`: P2 物理撤去の gate（#419 系 + AL#189/SV#232）を機械可読に
  記録し、`NotRun` を明示する。
- `summary`: repo 別の分類集計。
- 根拠の説明・差分サマリ・保護対象の検証・収集方法は
  `evidence/rf465/README.md` に記述し、JSON は索引可能な事実の正本
  とする（Markdown 表を JSON に複製しない）。

### D3. consumer 列挙は read-only・checkout 正本

`gh api search/code` は本タスク時点で `incomplete_results: true` /
0 件を返したため、AL/SV は origin/main HEAD に fetch 済みの clean
checkout で file-level 列挙した（両 repo `0e2bfee5`/`8119d3e8`、
ruviform pin `fd1d4d5f`）。Issue 状態は `gh issue view` で確認
（AL#189/SV#232/RF#419/#262 すべて OPEN）。consumer repo への変更・
Issue 起票は行わない。

### D4. `ruviform-ui-webview` の変更は doc 注記のみ

crate root doc（`src/lib.rs`）に legacy/preview 分類・移行先
（`ruviform-ui-gpui`）・撤去 gate・台帳参照を追記し、README に同旨の
注記を追加する。`#[deprecated]` は付与しない — P0/P1 と同じ判断で、
consumer・examples・`-D warnings` lane を破壊するため。API・feature・
example・挙動は一切変更しない。

### D5. 差分は entry 単位と README サマリの二層で記録

各 entry の `diff_vs_p0p1` に個別差分を書き、README に「追加・
消失・分類変更・consumer 差分」の集約を記述する。これにより
「台帳が現行 main を反映している」ことを証拠付きで示す。

### D6. doc chain は spec→design→plan→task→test-spec→ir/exp

`docs/specs|designs|plans|tasks|test-specs|impl/{ir,exp}/
wv-removal-inventory*` の 7 文書を新設する。test-spec は監査・
分類検証の保証内容を記録し、ir/exp は実施記録・試行記録とする。
生成 index（各 dir の README.md）は doc-maintenance 手順で再生成する。

## File Map

| ファイル | 変更 |
| --- | --- |
| `evidence/rf465/inventory.json` | 新設 — 台帳正本（52 entries） |
| `evidence/rf465/README.md` | 新設 — schema/収集方法/差分/検証 |
| `docs/specs/wv-removal-inventory.md` | 新設 |
| `docs/designs/wv-removal-inventory-design.md` | 新設（本書） |
| `docs/plans/wv-removal-inventory-plan.md` | 新設 |
| `docs/tasks/wv-removal-inventory-task.md` | 新設 |
| `docs/test-specs/wv-removal-inventory.md` | 新設 |
| `docs/impl/ir/wv-removal-inventory.md` | 新設 |
| `docs/impl/exp/wv-removal-inventory.md` | 新設 |
| `crates/ruviform-ui-webview/src/lib.rs` | crate doc に Legacy status 節を追加 |
| `crates/ruviform-ui-webview/README.md` | 冒頭に legacy/preview 注記を追加 |
| `docs/{specs,designs,plans,tasks,test-specs}/README.md`, `docs/impl/{ir,exp}/README.md` | 生成 index の再生成 |

## リスクと対策

- **分類の恣意性**: 分類基準（「削除で壊れる契約」単位）と各
  entry の `evidence`/`consumers`/`notes` で判定根拠を残し、
  review-protocol で再検証する。
- **consumer 列挙の陳腐化**: 対象 SHA を JSON/README に固定し、
  再検証時は同 SHA または更新 SHA を明記して差分を取る。
- **doc コメントが lint を壊す**: コメントのみの変更。intra-doc
  link は既存参照（`session`/`host`/`scheme`/`embed`/`detect`）を
  変更しない。`cargo check`/`clippy`/`test`（feature 有無両方）で確認。
- **「維持/再利用」の過少分類**: `rvf.ipc.v1`・Scenario・
  `ruviform-viz`・`ui/guitest/fixtures` を `reuse` に明示し、
  README の保護対象検証表で再確認する。

## 検証

- `mise run fmt-check`
- `mise run check ruviform-ui-webview` / `lint ruviform-ui-webview` /
  `test ruviform-ui-webview`（package 単位）
- `mise run check-ui-webview` / `lint-ui-webview` / `test-ui-webview`
  （feature lane — doc 注記が feature 有効側の doc にも載るため）
- `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js`
  の audit で broken relation がないこと
- `inventory.json` の JSON 構文・集計の機械検証
