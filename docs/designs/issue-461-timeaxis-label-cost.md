---
id: "DESIGN-034U1MKPf89ADLuIJkjzgP"
type: "design"
status: "approved"
title: "Issue #461 残課題 — TimeAxis label の canvas 内共有 layer 描画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/461"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-461-timeaxis-label-cost.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "docs/designs/issue-461-canvas-paint-layer.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-461-timeaxis-label-cost.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-461-canvas-paint-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-434-spectrogram-kind.md"
  refines:
    - "docs/designs/issue-461-canvas-paint-layer.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-461-timeaxis-label-cost.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #461 残課題 — TimeAxis label の canvas 内共有 layer 描画

## Context

`time_axis_element`（`crates/ruviform-ui-gpui/src/render.rs`）は、viewport 内の各 tick について tick mark 用の canvas quad に加え、label 用に `div().absolute().left(relative(m.x)).top(px(space_sm)).text_color(text_dim).child(label)` を 1 tick につき 1 つ生成していた。上限 `MAX_TIME_AXIS_TICKS` = 1,024 で最大 1,024 の label `div` が element tree に並ぶ。

gpui 0.2.2 の text 経路を確認した結果、各 label の text 要素は `WrappedLine::paint` → `paint_line`（`gpui-0.2.2/src/text_system/line.rs`）を通り、`paint_line` は行ごとに `window.paint_layer(line_bounds, ...)` を開いて glyph を描く。label bounds は互いに交差するため、各 layer の `BoundsTree::insert` が広い subtree を走査し、insert 全体で O(n²) となる。これは PR #462 が quad に対して解消したのと同じ構造問題が text 経路に残っていたものである。

計測（debug build、headless `TestAppContext`、修正前コードの一時 probe）で原因を切り分けた:

| 条件 | repaint（1,024 ticks） |
| --- | --- |
| label あり（実際の表示文字列） | 約 797 ms / 1.72 s |
| label 空文字 | 約 124〜128 ms |
| 1 文字 label | 約 784〜895 ms |
| 全 tick 同一 1 文字 label | 約 746〜805 ms |
| label あり 256 ticks | 約 55〜57 ms |
| 全 tick viewport 外（cull 済み） | 約 262 µs |

label の文字列内容・長さはほぼ無関係で、非空 label の個数に対してほぼ 2 乗で増加する。`axis_mark_plan` の viewport cull は既に効いており（全 cull で 262 µs）、element tree の div 数ではなく text line ごとの `paint_layer` → `BoundsTree::insert` が支配的である。gpui の `LineLayoutCache` は text・font size・font runs・wrap で keyed されフレーム間で layout を再利用するため、shaping 自体は cache hit が効く。

## Scope

### In Scope

- `crates/ruviform-ui-gpui/src/render.rs` の `time_axis_element` とその label 描画経路。
- `TimeAxisSpec` / `AxisTick` の公開契約を変えない範囲での paint 層の実装見直し。
- 契約上限 payload の repaint budget 回帰テスト（既存 `rvf_view_max_payload_canvas_repaint_budget` への TimeAxis ケース追加）。
- 本チェーンの文書（spec / design / test-spec / plan / task）と `docs/designs/issue-461-canvas-paint-layer.md` の残課題更新。

### Out of Scope

- `ruviform-ui` の公開契約変更（`TimeAxisSpec` / `AxisTick` の形・値域・意味）。
- consumer 側（audio-lens / synvelis）のコードと pin 更新。
- Table の行×列 `div` など TimeAxis 以外の element tree 規模コスト。
- 実機 GPUI 窓での計測（headless `TestAppContext` で `Scene` 構築の全経路を通す）。

## Components and Boundaries

- `time_axis_element`: tick mark quad（従来どおり 1 つの `paint_layer`）と label glyph（新設の 1 つの共有 `paint_layer`）を canvas paint closure から描く。label 用の `div` は生成しない。
- `shape_axis_labels`（新規、canvas prepaint）: `axis_mark_plan` の結果から各 mark の label を `WindowTextSystem::shape_text` で shaping し、`WrappedLine` の列・origin・label ink の union bounds を paint payload（`AxisLabelPaint`）として返す。text style は `window.text_style()` に `palette.text_dim` の色を被せたもので、旧 label `div` の `.text_color(palette.text_dim)` が解決したのと同じ style になる。
- `paint_axis_label_line`（新規、canvas paint）: shaping 済み `LineLayout` の glyph を `window.paint_glyph` / `window.paint_emoji` で描く。glyph 配置は gpui `paint_line` の単一 style・左寄せ・wrap 無しの場合と同じ計算（`padding_top = (line_height - ascent - descent) / 2`、`baseline_offset`、`glyph.position` の差分累積、`content_mask` による glyph 単位の clip）を踏襲し、layer は呼び出し側の共有 layer を使う。
- 破られていた不変条件の所有層は `ruviform-ui-gpui` の renderer（`render.rs`）であり、修正も同じ層で行う。gpui の `paint_line` が行ごとに layer を開くのは text element 一般の仕様であり、アプリ側で変更できないため、label を text element 経路から外すのが境界として正しい。

## Data and Control Flow

1. prepaint: `axis_mark_plan(spec)` で viewport 内 tick を求め（従来どおり）、canvas bounds を `bounds_store` に記録しつつ `shape_axis_labels` で全 label を shaping する。各 label の origin は旧 `.absolute().left(relative(m.x)).top(px(space_sm))` と同じ `(bounds.origin.x + width * m.x, bounds.origin.y + space_sm)`。
2. paint: tick mark quad を 1 つの `paint_layer(bounds)` に描く（#462 と同じ）。
3. paint: label ink の union bounds が存在すれば `paint_layer(ink)` を 1 つ開き、その中で全 label の全 `WrappedLine` を `paint_axis_label_line` で描く。layer bounds は旧 per-line layer が覆っていた領域（全 label の union）と一致するため、sibling 要素との描画順は保存される。
4. `\n` を含む label は `shape_text` が複数 `WrappedLine` を返し、`line_height` 刻みで積む（旧 text element の行積みと同じ）。空 label は shaping 結果が空になり ink に寄与しない。

## Non-Functional Constraints

- repaint は debug build・headless `TestAppContext` で計測する。`Scene` 構築（`BoundsTree::insert`）は headless でも実経路を通る。
- 公開契約（`TimeAxisSpec` / `AxisTick` / `MAX_TIME_AXIS_TICKS`）と描画結果（tick 位置・label 位置・色・major/minor の描き分け）は不変。
- `unsafe` なし。`#[allow]` による lint 隠蔽なし。

## Risks and Trade-offs

| リスク | 影響 | 対策 |
| --- | --- | --- |
| glyph 配置の再実装が gpui `paint_line` とずれる | label の位置・太さの微細な差 | `paint_line` の単一 style・wrap 無し経路と同じ式を踏襲。underline/strikethrough/decoration run は label が単一 `to_run` のため発生しない。emoji は `paint_emoji` に分岐（gpui と同じ） |
| label ink union が canvas 全域に近づき、layer bounds が広がる | layer 自体の insert コスト・clip 範囲 | label は軸上部の細い帯に限定され、union はその帯を覆う。旧 per-line layer の union と同じ領域であり、悪化しない |
| shaping を prepaint に移すことで prepaint コストが増える | repaint 全体 | `LineLayoutCache` がフレーム間で layout を再利用するため 2 回目以降は cache hit。初回 shaping は 1,024 label でも budget（500 ms）を大きく下回る実測（約 11 ms/repaint） |
| `window.text_style()` の継承が旧 div 経路と異なる | font/size の差 | 旧 label `div` も同じ `window.text_style()` を祖先経由で解決しており、canvas prepaint 時点の stack は同じ祖先を反映する。アプリ内に `.text_style()` 指定はなく、色のみ `.text_color` で上書きしていた点も `style.color` の上書きで再現 |

## 採らなかった案

- 表示範囲外 tick の追加 cull。`axis_mark_plan` は既に viewport で cull しており、全 cull で 262 µs と実質無料。残コストは cull ではなく text paint にあった。
- label 文字列のアプリ側 shaping キャッシュ。gpui `LineLayoutCache` が既に text/run/size keyed でフレーム間再利用しており、cache hit 時の計測（同一 label 約 0.8 s）から shaping cache miss ではないことが分かった。主因は per-line `paint_layer` であり、cache 追加は原因を直さない。
- label のみの間引き（major のみ label 等）。parity fixture が minor tick の label（`"250"`）を期待しており、視覚・契約の退行になる。
- tick 数の契約上限の引き下げ。`MAX_TIME_AXIS_TICKS` は公開契約であり、consumer の正当な利用を制限する根拠がない。

## Deferred Design Concerns

- Table の行×列 `div` 生成（`MAX_TABLE_ROWS` × `MAX_TABLE_COLUMNS`）も element tree 規模のコストであり、必要なら別途監査する（#461 design の残課題を継続）。
- 実 GPUI 窓での実機 paint 計測。headless で `Scene` 構築の全経路を通すため O(n²) 回帰は検知可能。実機の体感確認は consumer 側の pin 更新後に委ねる。

## Verification Notes

- [x] 診断計測: 修正前コードの一時 probe で label 条件別・tick 数別の repaint を計測し、text paint の per-line layer が支配的であることを確認した。
- [x] 修正後計測: `rvf_view_max_payload_canvas_repaint_budget` に TimeAxis（1,024 labelled ticks）を追加し、実測約 11 ms/repaint（budget 500 ms）で通過することを確認した。
- [x] drawn plan / parity / consumer observation の既存契約が維持されること（`axis_mark_plan` と spec 由来の `DrawnMark` は変更していない）。
