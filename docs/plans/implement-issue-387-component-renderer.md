---
id: "PLAN-034T6RWSw6Tv9Mz2qJ5eL8"
type: "plan"
status: "proposed"
title: "Issue #387 Component/View Specs + Headless Behavior + GPUI Renderer 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/387"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-387-component-view-specs.md"
  implemented-by:
    - "../tasks/t1-component-view-spec-issue-387.md"
    - "../tasks/t2-headless-behavior-issue-387.md"
    - "../tasks/t3-semantic-fixtures-issue-387.md"
    - "../tasks/t4-gpui-renderer-issue-387.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-387-component-view-specs.md"
  derived-by:
    - "../tasks/t1-component-view-spec-issue-387.md"
    - "../tasks/t2-headless-behavior-issue-387.md"
    - "../tasks/t3-semantic-fixtures-issue-387.md"
    - "../tasks/t4-gpui-renderer-issue-387.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-387-component-renderer.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #387 実装計画

## Scope split

| Task | 内容 | 依存 |
| --- | --- | --- |
| T2 | `ruviform-ui::behavior` — value mapper + interaction machine + command/binding 語彙 | — |
| T1 | `ruviform-ui::view` — spec schema + `ViewHost` projection + reconcile/lifecycle/identity | T2(view は behavior 型を参照) |
| T3 | `ruviform-ui-test` — semantic fixtures(UI-DVR-02/03, 2 story fake layer, 2-theme 同値)+ `cases.json`/`stories.json` binding 更新 | T1+T2 |
| T4 | `ruviform-ui-gpui` — `ThemeMap` + spec→element renderer + 入力 normalize + example + gpui headless 試験 | T1+T2 |

## File map

| 変更 | ファイル |
| --- | --- |
| T1 新設 | `crates/ruviform-ui/src/view.rs` |
| T1 変更 | `crates/ruviform-ui/src/lib.rs`(`pub mod view`)。`ruviform-viz` 依存は追加しない(`WaveformBin` 中立 payload 型、D5) |
| T2 新設 | `crates/ruviform-ui/src/behavior.rs` |
| T2 変更 | `crates/ruviform-ui/src/lib.rs`(`pub mod behavior`) |
| T3 新設 | `crates/ruviform-ui-test/tests/component_view.rs`、`fixtures/themes/{synvelis-dark,audiolens-dark}.dtcg.json` |
| T3 変更 | `crates/ruviform-ui-test/Cargo.toml`(dev-dep: ruviform-ui-theme)、`fixtures/driver-lab/{cases,stories}.json` |
| T4 新設 | `crates/ruviform-ui-gpui/src/render.rs`、`tests/component_render.rs`、`examples/component_showcase.rs` |
| T4 変更 | `crates/ruviform-ui-gpui/src/lib.rs`(`pub mod render`)、`mise.toml`(`example-ui-gpui-components` lane 必要なら) |

## リスクと対応

| リスク | 対応 |
| --- | --- |
| gpui TestAppContext が headless Windows で window entity を建てられない | 試行→失敗なら gpui binding `not_run`+rerun 条件、renderer 単体の mapping/入力 normalize は unit test で担保 |
| spec payload 型と `ruviform-viz::ViewBin` の field 意味 drift | 片方向変換を consumer 側に集約、field 名を `ViewBin`/`LodBucket` に一致させて drift を検出しやすくする |
| spec tree 再帰の unwrap/nesting | enum tree、depth は製品 spec で浅い(≦5)。recursive type は Box |
| drag 中 pointer が element 外へ | window 全 mouse move listener(`cx.listener`)で drag 継続、view bounds clamp。mouse up 外は `on_mouse_up_out` で Commit |
| `CommandBinding` ack timeout | 時計を持たず `on_tick(now: Instant)`/deadline 指定で caller 駆動 — headless deterministic を維持 |

## 検証マトリクス

| 検証 | 入口 | 期待 |
| --- | --- | --- |
| headless workspace | `mise run check`/`lint`/`test` | GPUI なしで全 pass(新規 module 含む) |
| boundary guard | `mise run check-ui-driver-boundary` | 中立 module に gpui/wry/OS 参照無し |
| driver contract lab | `mise run test-ui-driver-contract` | manifest validation pass(UI-DVR-02/03 + stories fake binding が cargo-test) |
| GPUI typecheck | `mise run check-ui-gpui` | feature `gpui` で check pass |
| GPUI lint | `mise run lint-ui-gpui` | `-D warnings` pass |
| GPUI test | `mise run test-ui-gpui` | render unit test + headless semantic parity(可能なら) |
| showcase | `mise run example-ui-gpui-components` | 実 window manual(記録のみ、自動判定なし) |
| 回帰 | `mise run fmt-check` | format pass |

## ロールバック

- `ruviform-ui` 追加は新 module のみ — `view.rs`/`behavior.rs` 削除 +
  `lib.rs`/`Cargo.toml` 追加分の除去で復旧。
- `ruviform-ui-test`/`ruviform-ui-gpui` の追加も新ファイル中心。
  manifest binding は git diff で戻せる。
