---
id: "IMPL-034Uh5vBGP1ASyRB9MdhY8"
type: "impl"
status: "completed"
title: "Issue #478 Fader/Knob 縦寸法契約 — grow+min_h slot 実装"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/478"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-478-fader-knob-grow-min-h-slot.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "docs/specs/issue-478-fader-knob-height-contract.md"
      - type: "file"
        path: "docs/plans/implement-issue-478-fader-knob-height-contract.md"
      - type: "file"
        path: "docs/tasks/implement-issue-478-fader-knob-height-contract.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-478-fader-knob-height-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-478-fader-knob-height-contract.md"
    - "docs/adr/issue-478-fader-knob-vertical-size-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/issue-478-fader-knob-height-contract.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-478-fader-knob-height-contract.md"
  verified-by: []
  references:
    - "docs/specs/issue-478-fader-knob-height-contract.md"
    - "docs/designs/issue-478-fader-knob-height-contract.md"
    - "docs/adr/issue-478-fader-knob-vertical-size-contract.md"
    - "docs/plans/implement-issue-478-fader-knob-height-contract.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #478 Fader/Knob 縦寸法契約 — grow+min_h slot 実装

## Summary

ADR `docs/adr/issue-478-fader-knob-vertical-size-contract.md` の Option C
（viz_slot 型 `flex_grow` + `min_h(px(120))` slot）を GPUI renderer に
適用した。`element_for` の `Fader | Knob` 分岐が持っていた `h_full`
ラップを廃し、viz canvas と同一の grow+min_h slot 契約に統一した。
`control_element` 非 Seek 経路の `h_full` も除去し、control/canvas は
`size_full` で slot を満たす形に改めた。これにより Fader/Knob が sibling
の固定高さ・最小高さを押し出すことがなくなり、canvas bounds は slot
bounds と一致し続ける（#450 契約の維持）。driver-neutral の
`ViewSpec`/`ControlSpec` と Seek 経路・#479 キーボード規約は不変。

## Context

- spec: `docs/specs/issue-478-fader-knob-height-contract.md`
- design: `docs/designs/issue-478-fader-knob-height-contract.md`（approved）
- ADR: `docs/adr/issue-478-fader-knob-vertical-size-contract.md`（proposed —
  accepted 化は plan の Implementation Handoff どおり coordinator /
  reviewer 判断に委ねる）
- test-spec: `docs/test-specs/issue-478-fader-knob-height-contract.md`
- plan: `docs/plans/implement-issue-478-fader-knob-height-contract.md`
  （I1–I3 が本 dispatch の要件）
- task: `docs/tasks/implement-issue-478-fader-knob-height-contract.md`
- 設計確定: PR #485（`cc7d5296`）。本実装の起点は PR #486（Issue #479
  キーボード規約、`be466ebe`）merge 後の `origin/main`。

## Environment（全実行共通）

- OS: Windows 11 Pro 10.0.26300（x64）・対話 session・CPU i9-9900K
- toolchain: mise 管理 rustc 1.99.0・GPUI 0.2.2
- revision: `be466ebe`（`origin/main` HEAD。実装は同一 worktree の
  `feat/478-fader-knob-height-contract` 上の未コミット差分として実施）
- 実施日: 2026-11-09
- gpui-test の `TestDisplay` は 1920×1080 logical px で、
  `TestAppContext::add_window_view` は maximize 状態の window を開く
  （test 内の `TEST_WINDOW_HEIGHT = 1080.0` はこの実測に基づく）。

## Implementation

### Changes

- `crates/ruviform-ui-gpui/src/render.rs`
  - `grow_slot(child)` helper を新設 — `flex_col` + `flex_grow` +
    `min_h(px(120.0))` の slot で child を包む。`viz_slot` は
    `grow_slot(paint.size_full())` に委譲し、slot 規則の authoritative
    な実装を一箇所に集約した。
  - `element_for` の `ComponentSpec::Fader(c) | ComponentSpec::Knob(c)`
    分岐を `div().flex().h_full()` から `grow_slot(control_element(...))`
    に変更。
  - `control_element` の非 Seek 経路から `.h_full()` を除去
    （`.size_full()` と `w(px(32.0))` は維持）。Seek 経路の
    `h(px(24.0))`・pointer/keyboard の既存挙動は不変。
  - gpui-test 追加・更新（test-spec の Rule/Example に対応）:
    - `TEST_WINDOW_HEIGHT` 定数と `canvas_height_px` helper を追加
      （`RvfView.bounds` に記録された canvas bounds の高さを読む）。
    - `rvf_view_fader_slot_shares_height_with_siblings`（新規）:
      Panel `[Fader, Seek, Waveform]` で Seek=24px、Fader/Waveform が
      `min_h` 床 120px を保ちつつ残り高を均等分配し、合計が Panel 高を
      超えないことを検証。
    - `rvf_view_fader_in_tabs_fills_body_remainder`（新規）: Tabs body
      の単独 Fader が tab header を除く残り高を満たすことを検証
      （`min_h` に張り付かない）。
    - `rvf_view_canvas_bounds_nonzero_for_parity_kinds`（既存へ assert
      追加）: `[Knob, Meter, TimeAxis, Spectrogram]` + Toggle で全
      canvas が非ゼロ、Knob/Meter/Spectrogram が `min_h` 床を保ち
      grow slot が残り高を分配し、TimeAxis=24px を維持することを検証。
- `docs/specs|plans|tasks/issue-478-*.md`・`implement-issue-478-*.md`:
  実装完了に伴う status 更新と checkbox / Verification の記録
  （task → `done`、plan → `completed`、spec → `implemented`）。
  design / test-spec は `approved` のまま、ADR は `proposed` のまま
  （accepted 化は reviewer / coordinator 判断）。
- `docs/impl/ir/README.md`: 本 record の status を `completed` に更新。

### Decisions During Implementation

- `min_h` は viz_slot と同じ `px(120.0)` に一本化した（plan の推奨
  どおり）。control 専用の別値は置かない — 「grow + min_h 床」は
  viz canvas と値制御で同一の slot 責務であり、値を分けると同一
  ポリシーの重複になるため。
- slot 規則を `grow_slot` に集約し `viz_slot` から再利用した。
  「正しい境界で修正する」原則に従い、Fader/Knob だけに局所例外を
  置くのではなく、同一契約を持つ既存 slot と一本化した。
- 非 Seek control は `.h_full()` を除去したが `.size_full()` は残した。
  高さの所有権は slot 側に移り、control/canvas は slot を満たす
  （canvas bounds = slot bounds、#450 の描画寸法契約を維持）。
- Seek 経路（`is_seek` → `h(px(24.0))`）、#479 のキーボード規約、
  `ViewSpec`/`ControlSpec` など driver-neutral 契約、依存・
  `Cargo.lock` は一切変更していない。

## Related Experiments

### Adopted

<!-- なし — 探索的実験は行わず、確定済み ADR の適用のみ。 -->

### Rejected

<!-- なし。 -->

## Validation

### RED → GREEN（test-spec Rule/Example 対応）

新規テストを旧実装（`h_full`）に対して先に実行し、契約違反を確認
してから renderer を修正した。

| 対象 | 修正前（RED） | 修正後（GREEN） |
| --- | --- | --- |
| `rvf_view_fader_slot_shares_height_with_siblings` | fail — Fader=936px / Waveform=120px（Fader が残り高を独占） | pass — Fader≈Waveform で残り高を分配、Seek=24px |
| `rvf_view_canvas_bounds_nonzero_for_parity_kinds`（追加 assert） | fail — grow slot 合計 1038.5px（期待 1032.0px ±0.5） | pass — Knob/Meter/Spectrogram が均等分配 |
| `rvf_view_fader_in_tabs_fills_body_remainder` | （単独子では旧来も全高を満たすため差分なし。回帰防止として追加） | pass — Fader = 1080 − tab header − 罫線 1px（body 残りを満たし `min_h` に張り付かない） |

### mise lane（canonical）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | clean |
| `mise run check` | 0 | `cargo check --workspace --all-targets` clean |
| `mise run lint` | 0 | clippy `-D warnings` clean |
| `mise run test` | 0 | workspace 全 test pass（再実行。初回は `ruviform-plugin-host` `l1_proxy_e2e::render_round_trips_valid_output` が 1 件 flaky 失敗 — "waiter registered" panic。単独再実行 pass・workspace 再実行 pass を確認し、本変更と非依存であることを確認） |
| `mise run test-ui-gpui` | 0 | unit 65 + consumer_observation 20 + scenario_contract 7 全 pass |
| `mise run test-ui-parity` | 0 | 12 tests pass |
| `mise run test-ui-consumer-observation` | 0 | 20 tests pass |
| `mise run test-ui-driver-contract` | 0 | 7 tests pass |
| `mise run test-ui-component-view` | 0 | 17 tests pass |
| `mise run check-ui-driver-boundary` | 0 | dep-graph / signature / allowlist 全 pass |

補助証拠（canonical = mise ではない直接 cargo 実行）:
`cargo build -p ruviform-ui-gpui --features gpui --example component_showcase`
→ exit 0（依存 `proc-macro-error2` の future-incompat note は既存・本変更と
無関係）。

### component_showcase 実窓確認（Passed）

`component_showcase`（Panel `[Tabs(Fader×2), Waveform, Seek]`、
640×480 logical）を実機で起動し、`orca computer` で観測した。

- window は正常に open・描画された（surface_bg が全面、fader の
  track/fill/thumb・waveform bins・seek strip を pixel 色解析で確認）。
- Fader は tab body の単独 grow 子のため、契約どおり残り高を満たす
  — pixel 実測で canvas 高さ約 299px（tab header 約 33px + 下罫線を
  除く body 残り）で、旧 `h_full` 時と同じ見た目を維持。
  sibling 構成での実高さ変化は showcase の spec に該当レイアウトが
  ないため、gpui-test の bounds 検証が権威ある証拠（test-spec の
  方針どおり）。
- tab ヘッダ click で `tab-select` intent（index=1）を観測。
- fader ドラッグで `Begin → Preview×13 → Commit` の gesture intent
  列（normalized 0.2→0.75）を観測 — pointer mapping が新 slot
  bounds で正しく機能。
- window を閉じて process は exit 0 で終了。

## Risks

- sibling 構成では Fader/Knob の実高さが変わる（grow slot の均等
  分配）。これは意図した契約変更であり、`component_showcase` の
  ような単独子配置では見た目不変を実測済み。
- 極端に低い Panel では `min_h` 合計が利用可能高を超え、依然
  overflow し得る。Panel の overflow/scroll は design の Deferred
  Design Concerns であり本 Issue の範囲外。
- `mise run test` の workspace lane で `ruviform-plugin-host` の
  `l1_proxy_e2e` に flaky な失敗を 1 度観測した（本変更と無関係・
  再実行で pass）。恒常的な flake なら別 Issue での追跡が望ましい。

## Follow-ups

- ADR `docs/adr/issue-478-fader-knob-vertical-size-contract.md` の
  `proposed → accepted` 化は plan の Implementation Handoff どおり
  coordinator / reviewer の判断に委ねる（本 record では変更しない）。
- Panel overflow/scroll、Tabs body の `size_full` 方針、orientation
  ベースの control sizing は design の Deferred Design Concerns
  （必要なら別 Issue）。
- Issue #478 の close と PR merge は Coordinator が行う。
