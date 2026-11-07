---
id: "IMPL-034TwpApE4er3OshirZyW5"
type: "impl"
status: "completed"
title: "Issue #414 projection→controlled display 正規経路"
created: "2026-11-07"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/414"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/designs/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/plans/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/test-specs/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/impl/ir/issue-414-projection-controlled-display.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-414-projection-controlled-display.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-414-projection-controlled-display.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-414-projection-controlled-display.md"
    - "docs/designs/issue-414-projection-controlled-display.md"
    - "docs/plans/issue-414-projection-controlled-display.md"
    - "docs/test-specs/issue-414-projection-controlled-display.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/410"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #414 projection→controlled display 正規経路 実装記録

## 実施内容

- `crates/ruviform-ui/src/behavior.rs`
  - `projection_value_input(&serde_json::Value) -> Option<ValueInput>`：
    projection payload の正規 decode。有限 JSON number →
    `ValueInput::Plain`、`{"plain": n}` → Plain、
    `{"normalized": n}` → Normalized、それ以外
    （文字列・null・bool・配列・複数 key・非有限値）→ None。
    業務 JSON を無条件に f64 へ解釈しない。
  - `InteractionControl::pending_external: Option<ControlValue>` 追加。
  - `InteractionControl::apply_external(ValueInput) -> Option<ControlValue>`：
    Plain/Normalized のみ受理し `resolve_value_input`（既存
    ValueMapper：range policy・snapping・domain 変換・非有限拒否）で
    絶対値へ解決。gesture 中は `pending_external` へ stage、idle では
    `committed_value` を即時更新。intent/gesture event は発行しない。
  - `end_gesture` で `pending_external` を flush し、gesture 終了後の
    committed/display へ authoritative 値を着地させる。
- `crates/ruviform-ui/src/view.rs`
  - `ViewHost::apply_projection`：binding `on_projection` の
    revision gate を先に通し、accepted 時のみ decode →
    `dispatcher.control_mut(control_id).apply_external(input)`。
    stale/equal revision・undecodable payload は display を触らず
    `observed` のみに記録される既存契約を維持。
  - `register_control`：生存 binding の `last_observed` を新規
    control の mapper 経由で復元適用（detach→reattach・再登録で
    display が default に留まらない）。
  - `apply_projection` doc に Controlled Display / Applied /
    Observed の三状態区別と正規経路を明記。
- `crates/ruviform-ui-gpui/src/render.rs`
  - `#[gpui::test]` 2 件：実 inbox → `apply_projection` →
    `observe().display.normalized`（`control_element` が読む値その
    もの）を検証。idle 即時更新と gesture 中 preview 優先の両経路。
- `crates/ruviform-ui-test`
  - `tests/component_view.rs`：`ui_dvr_09_*` 3 件（idle/stale/
    undecodable、gesture commit+cancel flush、detach-reattach 復元）。
  - `fixtures/driver-lab/cases.json`：`UI-DVR-09` required case 追加
    （fake=component_view cargo-test、gpui=test-ui-gpui、
    real-os=not_run）。
  - `tests/driver_contract.rs`：required_cases 数を 8→9 に更新
    （manifest 整合 assertion の追従）。

## RED → GREEN 実測

- RED（修復前）: 外部非ゼロ projection を idle で受けても
  `snapshot.observed` のみ更新され `snapshot.display.normalized`
  （renderer 消費値）は初期値のまま残る — Issue #414 の反例を
  gpui test・component_view test で再現確認。
- GREEN（修復後）: 同一経路で display/committed が authoritative 値へ
  更新されることを全 lane で確認。

## 検証実績（Windows 11 x64・Git Bash、mise rust 1.99.0 既定）

| 検証 | 結果 |
| --- | --- |
| `mise run fmt-check` | pass |
| `mise run check` | pass |
| `mise run lint` | pass（workspace clippy -D warnings） |
| `mise run lint-ui-gpui` | pass（gpui feature clippy -D warnings） |
| `mise run test` | pass（workspace 全 test・doctest 含む） |
| `mise run check-ui-driver-boundary` | pass（dependency/signature/allowlist 全 pass） |
| `mise run test-ui-driver-contract` | pass（5/5・required_cases 9 整合） |
| `mise run test-ui-component-view` | pass（15/15・ui_dvr_09 3 件含む） |
| `mise run check-ui-headless` / `test-ui-headless` | pass（67 test） |
| `mise run check-ui-components` / `test-ui-components` | pass（94 test） |
| `mise run check-ui-binding` / `test-ui-binding` | pass（87 test） |
| `mise run check-ui-guitest` / `test-ui-guitest` | pass（135 test） |
| `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` | pass（31 test） |
| `mise run test-ui-test-msrv` | pass（Rust 1.89.0 MSRV lane） |

## merge 後再検証

- `git fetch origin main && git merge origin/main`：HEAD =
  origin/main = `79d1db0a` で既に最新（merge 対象差分なし・
  Already up to date）。上記検証はすべてこの HEAD 上で実施。

## NotRun / 引き渡し

- macOS / Linux 実機・実 OS window: NotRun（Windows のみ実測。
  driver-lab manifest の real-os case は not_run のまま）。
- Issue #410（view command ledgers・epoch correlation・retention）
  領域: 本変更は ledger 機構に非接触。retention/correlation の
  契約は別 Issue で扱う。
- SV 側（synvelis）恒久 workaround / 第二 ledger: 追加していない
  （Issue 受入条件どおり RF 所有境界で修復）。SV183 の Ready 化は
  RF414 解消確認後の root 管轄。

## 残リスク

- `pending_external` flush は `end_gesture` 集約のため、gesture
  終了の全経路で一貫適用されるが、gesture 中に複数 projection が
  届いた場合は最新の 1 件のみが着地する（staging は上書き保持。
  最終 authoritative 値が残る設計であり意図どおり）。
