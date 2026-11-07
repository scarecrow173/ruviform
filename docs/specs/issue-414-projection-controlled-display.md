---
id: "SPEC-kR4mNxQwY8vLpTzH2jFd6A"
type: "spec"
status: "approved"
title: "Issue #414 — authoritative projection を controlled display へ到達させる"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/414"
    - "https://github.com/scarecrow173/ruviform/issues/387"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/designs/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/plans/issue-414-projection-controlled-display.md"
      - type: "file"
        path: "docs/tasks/issue-414-projection-controlled-display.md"
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
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "https://github.com/scarecrow173/ruviform/issues/387"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-387-component-view-specs.md"
    - "docs/specs/viewlink-authoritative-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/414"
  derived-by:
    - "docs/designs/issue-414-projection-controlled-display.md"
    - "docs/plans/issue-414-projection-controlled-display.md"
    - "docs/test-specs/issue-414-projection-controlled-display.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #414 — authoritative projection を controlled display へ到達させる

## 背景と問題

Issue #387 で確立した Component/View contract では、consumer が送る
`StateProjection`（authoritative observed state）は `CommandBinding`
の `last_observed` にのみ記録され、`InteractionControl` が保持する
controlled display 値（`committed_value` / gesture preview）には
一切到達しない。

- `ViewHost::apply_projection` は `CommandBinding::on_projection`
  へ渡すだけで、`InteractionControl` を更新しない。
- `ViewHost::observe` の `display` は `InteractionControl::value()`
  から、`observed` は binding から別経路で取る。
- GPUI renderer（`ruviform-ui-gpui`）は `snapshot.display.normalized`
  のみを描画するため、外部の非ゼロ projection を受けても表示は
  初期値または最後の local commit のまま残る。

結果として「Observed（authoritative truth）」と「Controlled
Display（描画値）」が分離したまま接続されず、Synvelis 側の Seek
初期値 0 問題（外部 `ProjectController` の非ゼロ更新が表示に
反映されない）の直接原因となっている。

## 要求（Issue 受入条件）

1. Controlled Display / Applied / Observed の区別が正規文書化され、
   authoritative projection から表示更新へ到達する正規経路が
   Ruviform 所有境界で提供される。
   - 業務 JSON を無条件に `f64` へ解釈する実装は禁止。
   - gesture Preview 中の表示を無条件で上書きする実装は禁止。
2. 非ゼロ外部変更 / idle→表示更新 / drag, commit, cancel /
   stale, revision, reattach の契約が意味ある回帰で確認される。
3. Fake の `observed` だけでなく ViewHost の `display` と実
   Renderer が参照する値の対応が検証される。実行していない OS を
   Pass にしない。
4. （文脈条件）SV175 成果は別経路で保持され、この Issue だけで
   親の RF387/175 受入は完了にしない。

## 非目標

- Issue #410 の view command ledgers（retention / correlation /
  epoch 機構）には触れない。
- consumer（Synvelis）側の恒久 workaround・第二 ledger で本問題を
  隠さない。
- `apply_receipt` の ledger 意味論は変更しない — receipt の
  `Applied` は domain 変更の報告であり observed truth ではない。

## 参照

- TypeScript 参照実装 `ui/components/src/control-base.ts`
  （`pendingExternal_` → gesture 終了時 flush →
  programmatic atomic `set`）、`ui/binding/src/dom.ts`
  （controlled `value` write が intent を emit しない loop-free
  契約）。規則のみを参照し、コードはコピーしない。
- `docs/specs/issue-387-component-view-specs.md`、
  `docs/specs/viewlink-authoritative-generation.md`。
