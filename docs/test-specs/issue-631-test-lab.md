---
id: "TSPEC-034YaYvGNtksNB54jY0Umq"
type: "test-spec"
status: "approved"
title: "Issue #631 GPUI Test Lab / Scenario Console GUI v1 テスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/designs/issue-631-test-lab.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 GPUI Test Lab / Scenario Console GUI v1 テスト仕様

## Purpose

`plugin-dock-lab` GUI v1 と mise lane 簿記が、verdict の正直性・
catalog 消費の一本化・fail-closed を仕様どおり満たすことを保証する。
interactive 実走は manual lane のため、機械検証は build/CLI/
headless fail-closed/契約非回帰に限定し、実 Window 証拠は
`evidence/rf631/` の実測記録に委ねる。

## Feature

GPUI Test Lab / Scenario Console GUI v1 — catalog projection・
selector 実行・verdict/evidence 表示を持つ windowed opt-in lane。

## Rules

- **Rule: catalog は単一情報源** — GUI の一覧は
  `HEADLESS_MATRIX.project()` + `real_catalog::REAL_CATALOG` のみから
  組み立て、case/scenario/driver/required の値を bin 内に複製しない。
- **Rule: projection は証拠ではない** — catalog 表に verdict 語彙を
  出さず、`runnable`/`missing`/`session_required` は宣言投影として
  表示する。headless/fake の成功を real-window/native-dock/editor/
  pixel 証拠に昇格しない。
- **Rule: real row は実行しない** — real catalog case の Run 指定は
  実行せず、`test-ui-plugin-dock-real` への委譲を status に表示する。
- **Rule: fail-closed** — `WindowLifecycle::Ready` が timeout 内に
  来ない場合、catalog 全 row を `NotRun` +
  `observed.environment_blocked` で JSONL 出力し exit 0。偽 pass
  も exit 1 もしない。
- **Rule: catalog 実行経路の再利用** — headless 実行は
  `run_selected`（`run_entry`/`run_scenario`）のみを通り、独自
  executor・2 つ目 runner を作らない。JSONL は `render_jsonl` で
  `lane = "plugin-dock-lab"` を記録する。
- **Rule: receipt 契約** — 処理した command intent には
  `CommandReceipt`（`applied`/`rejected`）を `post_event` で返し、
  未知 action は `rejected` で fail-closed。
- **Rule: opt-in 分離** — `lab-gui` は非 default feature、bin は
  `required-features` 付き。default build/`cargo test`/CI に GPUI
  backend を引き込まない。mise lane は manual/opt-in 表記。

## Examples

- Example: catalog row 写像 — `HEADLESS_MATRIX.project()` の各
  `EntryProjection` が `CatalogRow { case, scenario, driver,
  required, runnable, missing }` に、`REAL_CATALOG` の各 row が
  `session_required` 付き `CatalogRow` に対応する。unit test で
  row 数（8+4）・case id・driver 名・required 集合を assert。
- Example: blocked path — Ready 未到達を模した report 構築関数が
  12 row 全て `NotRun`・`observed.environment_blocked`・
  `environment_kind != "real-window"` を生成することを unit test
  で assert（`real_dock::blocked_reports_fail_closed` と同形）。
- Example: intent dispatch — `CommandIntent::Action { action:
  "lab.run-all" }` で headless 全 cell が worker に渡る keep
  predicate、`"lab.quit"` で exit 要求、未知 action で rejected
  receipt、real row 選択で実行されないことを pure 関数 test で
  assert。

## Guarantees

- `mise run test-ui-plugin-dock` の verdict 表が本 slice で不変。
- `mise run test-ui-plugin-dock-console` が exit 0/1/2 意味論どおり
  動作（Fail verdict → exit 1、CLI misuse → exit 2）。
- `mise run check-ui-driver-boundary` が green — GPUI/Win32 型が
  公開面・evidence に現れない。
- `plugin-dock-console` bin の挙動は不変（headless console 非回帰）。
- lab lane は interactive session でのみ Pass を名乗り、headless
  host では `environment_blocked`/`NotRun` を記録する。

## Non-goals

- GUI の interactive 操作そのものの自動検証（実 Window・実 input・
  pixel は manual lane の性質上、実走証拠は `evidence/rf631/`）。
- real catalog case の GUI 実行・real lane の verdict 変更。
- catalog projection API（filter/sort/group）の拡張検証。
- Synvelis/AudioLens consumer 投影。

## Risk

- verdict 昇格・fake pass・real row 誤実行が混入すると本系列の核心
  （正直な verdict）を毀損する — 上記 Rule と unit test・lane
  実測で抑止する。
- `lab-gui` feature が default 経路に GPUI を引き込むと headless
  検証経路が壊れる — `required-features` + 非 default feature で
  構造的に防ぐ。
