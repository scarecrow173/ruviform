---
id: "IMPL-034XXbYaPK0skVnCFabuid"
type: "impl"
status: "in-progress"
title: "Headless scenario console plugin-dock-console (issue-631 console slice)"
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
  implements:
    - "docs/tasks/issue-631-scenario-console.md"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-631-scenario-console.md"
    rejected: []
---

# Headless scenario console plugin-dock-console (issue-631 console slice)

## Summary

merge 済み scenario catalog（`HEADLESS_MATRIX`）の headless consumer
として `plugin-dock-console` bin を新設した。`--list` は
`ScenarioMatrix::project()` の capability projection を 1 行 1 JSON
で出力し（verdict 語彙を含まない・実行を主張しない）、`--list
--include-real` は `real_catalog` の metadata 4 行を
`session_required:true`/`lane:"real-window-dock"` 付きで追記する
（`real-dock` feature 必須、非搭載 build では exit 2 で拒否）。
`--run` は `--case`/`--scenario`/`--driver` selector（flag 内 OR・
flag 間 AND）で `HEADLESS_MATRIX.cells()` を filter し、
`scenario::run_selected` 経由で catalog 順に `run_entry` 実行、
`jsonl::render_jsonl` で lane `plugin-dock-console` の JSONL を出力
する。selector は全経路 fail-closed（unknown・空 intersection・
CLI misuse・real-lane 実行試行は exit 2・無出力）。

## Context

`docs/tasks/issue-631-scenario-console.md`（plan
`docs/plans/issue-631-scenario-console.md`、spec
`docs/specs/issue-631-scenario-console.md`、design
`docs/designs/issue-631-scenario-console.md`、test-spec
`docs/test-specs/issue-631-scenario-console.md`）の console slice。
Issue `scarecrow173/ruviform#631` tracker の後続 slice であり、
GPUI 製 Scenario Console / Test Lab GUI 本体は後続へ残す。
`docs/designs/issue-464-scenario-runner.md` の「runner は 1 関数で
executor ではない」制約を維持 — console は `ScenarioEntry`/
`run_entry`/`render_jsonl` の再利用のみで、独自 executor・JSON DSL
・verdict 権威の変更は行わない。

## Implementation

### Changes

- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs`
  （新規）: 手書き `env::args` parse（既存 bin と同型）、
  `--list`/`--include-real`/`--run`/`--case`/`--scenario`/
  `--driver`/`--out`/`--help`。`cell_selected`（OR/AND 述語）、
  `validate_run_selectors`（real-lane 拒否 → unknown → 空
  intersection の順で fail-closed）、`driver_name`（serde の
  kebab-case 語彙を再利用）。
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`（新規）:
  `RUN_ID`/`SCENARIO`/`LANE`/`DRIVER`/`RealCatalogRow`/
  `REAL_CATALOG` — real lane の GPUI 非依存 metadata。全 build で
  compile され、console の real selector 認識と `--include-real`
  listing を feature gate なしで支える。
- `crates/ruviform-plugin-dock-test/src/scenario.rs`:
  `run_selected(env, keep)` 追加 — `HEADLESS_CONTEXT` を 1 つ構築し
  `HEADLESS_MATRIX.cells()` を filter して `run_entry` を catalog
  順に実行。`run_all` は `run_selected(env, |_, _| true)` へ委譲
  （挙動不変）。`HeadlessContext` fields は private のまま。
- `crates/ruviform-plugin-dock-test/src/lib.rs`: `pub mod
  real_catalog` + `run_selected` re-export。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`:
  `RUN_ID`/`SCENARIO` を `real_catalog` へ集約（`RUN_ID` は
  re-export で `real_dock::RUN_ID` 互換維持）、lane 文字列
  `"real-window-dock"` リテラル 2 箇所を `real_catalog::LANE` へ、
  `real_catalog_matches_entries` test 追加（feature `real-dock`
  下で `real_entries()` と `REAL_CATALOG` の case/scenario/
  required を pairwise 検証）。
- `crates/ruviform-plugin-dock-test/tests/console.rs`（新規）:
  `env!("CARGO_BIN_EXE_plugin-dock-console")` で 14 test —
  list 投影・verdict 語彙非含有、`--include-real`（feature で
  分岐: real-dock 下は 12 行 listing、非搭載は exit 2 拒否）、
  full/単一/OR/AND selector、`editor-attach` 単独 `not_run`、
  `artifact`+`editor-attach` の正直 verdict、unknown/空
  intersection/real-lane/CLI misuse の exit 2、`--out`、`--help`。
- `crates/ruviform-plugin-dock-test/README.md`: console 節を追記、
  catalog 節を `run_selected`/`real_catalog` 構成に更新。
- `evidence/rf631/`: `plugin-dock-console.jsonl`（`--run --case
  view-mount`）、`plugin-dock-console-list.jsonl`（`--list`）、
  `plugin-dock-console-list-include-real.jsonl`（real-dock build の
  `--list --include-real`）を追加。`plugin-dock-scenario.jsonl` は
  lane 再実行で更新（verdict 非回帰）。`verification.md` に console
  slice の検証記録を追記。

### Decisions During Implementation

- `run_selected` を `pub`（`pub(crate)` 不可 — bin target は別
  crate で crate-private item を見られない）とし、selector 述語を
  `impl Fn(&ScenarioEntry<HeadlessContext>, ScenarioDriver) -> bool`
  に限定。`HeadlessContext` 構築は scenario module 内に閉じ、
  caller は catalog cell の選択のみ行う — lane state を外部から
  組み立てる経路を作らない。
- console の JSONL `lane` は `plugin-dock-console` — run id は
  `rf631-plugin-dock` を共有するが、絞込み実行が full lane の
  `test-ui-plugin-dock` 証拠と混同されないよう provenance を分離。
- real lane metadata は `real_dock`（feature-gated・ctx 保持）から
  分離した `real_catalog`（非 gate・metadata のみ）に置いた。
  `real_dock.rs` の lane 文字列は `real_catalog::LANE` を共有し、
  feature 付き build では consistency test が両表の drift を検出。
- `--include-real` は非 `real-dock` build で「行を静かに落とす」
  代わりに exit 2 で拒否 — 要求された内容を満たせないことを正直に
  報告する fail-closed を採用（design doc に記録済み）。
- selector 検証の順序: real-lane 認識 → 各値の unknown 判定 →
  組合せの空判定。real case 名は headless catalog に存在しない
  ため、先に real を判定しないと「unknown case」として誤案内
  される。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-631-scenario-console.md`（+ 同 stem の
  `.jsonl` event log）— selector 設計・`--include-real` の
  fail-closed 判断・`run_selected` 公開範囲の試行を記録。

### Rejected

- なし。

## Validation

canonical = mise（Windows、rust toolchain は mise 管理）:

- `mise run fmt-check` — pass
- `mise run check|lint|test ruviform-plugin-dock-test`（default）
  — pass（console integration 14 tests + 既存 4 tests）
- `mise run check|lint|test ruviform-plugin-dock-test --features
  real-dock` — pass（`real_catalog_matches_entries`・
  `list_include_real_appends_real_lane_rows` 含む）
- `mise run test-ui-plugin-dock` — pass。JSONL verdict は
  pre-console と同一（差分は repo_sha・worktree path・fixture
  DLL sha256 の provenance のみ）
- `mise run test-ui-scenario-contract`（fake + gpui）— pass
- `mise run check-ui-driver-boundary` — pass（dep-graph/
  signature/allowlist 全て — console 経路は GPUI 型を漏らさない）
- `mise exec -- prek run --all-files` — pass

補助証拠（canonical ではない）: `cargo run --locked` による
`--list`/`--list --include-real`/`--run --case view-mount` の実出力を
`evidence/rf631/` に保存。

## Risks

- `--driver` の受理語彙は serde の kebab-case 文字列に依存 —
  `ScenarioDriver` に variant が増えた場合、console は自動的に新名を
  受理する（`driver_name` が serialize 結果を使うため hard-code なし）。
  逆に driver 名の rename は selector 語彙を変える破壊的変更 —
  catalog 側の serde 語彙は既に evidence に固定済み。
- `run_selected` は `run_all` と同じ context 構築を共有するが、
  selector predicate は bin 側の自由記述 — 誤った述語で lane state
  を「部分的に」進める可能性は残る（例: `artifact` を skip して
  後続 case を走らせると依存 case は `NotRun` に落ちる — これは
  意図した fail-closed であり、test で固定済み）。
- `--list` の `runnable`/`missing` は宣言 capability の投影であり
  live 実行可能性の保証ではない — catalog 層の既存契約をそのまま
  継承（test-spec の Non-goals に明記）。

## Follow-ups

- GPUI 製 Scenario Console / Test Lab GUI 本体 — 後続 slice
  （本 slice は headless interaction model のみ）。
- `mise.toml` への console lane task 追加 — 別 Worker 占有中のため
  本 slice では未対応。bin+tests のみ納品。
- `test_editor` 実物・実 Native Dock Pass・dock-pixels・child HWND
  input/resize/focus/re-dock — Issue #630 残存 scope / #532 DComp
  判定まで NotRun/Unsupported 維持。
- Synvelis/AudioLens consumer への catalog 投影 — 各 repo 側 scope。
