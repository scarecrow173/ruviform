---
id: "DESIGN-034XWNVav0w3OewvNDtAcm"
type: "design"
status: "approved"
title: "Issue #631 Headless Scenario Console 設計"
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
  derives-from:
    - "docs/specs/issue-631-scenario-console.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Headless Scenario Console 設計

## Context

`ruviform-plugin-dock-test` は Issue #631 の consumer-layer
composition root であり、headless lane（`scenario.rs` の
`HEADLESS_MATRIX` + `run_all`）、real-window lane
（`real_dock.rs`、`real-dock` feature）、および 2 つの lane bin
（`plugin-dock-scenario` / `plugin-dock-real-scenario`）を持つ。
第三 slice で case 列挙は `ScenarioMatrix` catalog に集約されたが、
消費者側の操作面は「全件実行する lane bin」のみである。

本設計は merge 済み catalog の headless consumer として
`plugin-dock-console` bin を追加する: `--list` が
`ScenarioMatrix::project()` の capability projection を 1 行 1 JSON
で出し、`--run` が selector で filter した cell を既存の catalog
実行経路（`run_entry` 系 + `render_jsonl`）で走らせる。GPUI 製 GUI
本体ではない — headless な interaction model の先行 slice。

## Scope

### In Scope

- `src/bin/plugin-dock-console.rs` 新規（auto-discover、feature
  gate なし）。
- `src/real_catalog.rs` 新規 — real lane の driver-neutral catalog
  metadata（非 feature-gated）。
- `src/scenario.rs` の `run_selected` 追加と `run_all` の投影化。
- `src/real_dock.rs` の `LANE` const 共有化と catalog 一致 test。
- `src/lib.rs` の module/export 配線、`tests/console.rs`、README、
  evidence、doc chain。

### Out of Scope

- GPUI console GUI、real lane の console 経由実行（既存
  `plugin-dock-real-scenario` lane が担う）、`test_editor`、実
  Native Dock Pass、製品 oracle、3OS、`mise.toml`/CI/`Cargo.lock`/
  `boundary.json`/`ruviform-ui-test` の変更。

## Components and Boundaries

- `plugin-dock-console` bin — 薄い CLI shell。`env::args` の手書き
  parse（既存 bin と同じ先例・新規依存なし）、selector 検証
  （fail-closed）、`--list`/`--run` の dispatch、JSONL/JSON 行の
  出力と exit code 判定のみを持つ。scenario 実行ロジック・verdict
  権威は持たない。
- `scenario::run_selected(env, keep)` — `HEADLESS_MATRIX.cells()` を
  predicate で filter し、fresh `HeadlessContext` を 1 つ構築して
  catalog 順に `run_entry` を呼ぶ。`run_all` は
  `run_selected(env, |_, _| true)` の select-all 投影となり、
  lane context 構築と sequencing の単一ソースになる。
  `HeadlessContext` の field は private のまま — bin は context を
  構築できず、部分実行も lib 側の narrow な入口経由に限定する
  （`pub(crate)` では bin（別 crate target）から見えないため
  `pub` とするが、公開面は predicate 付きの 1 関数のみ）。
- `real_catalog` module — `rf631.plugin-dock-real` の catalog
  metadata を driver-neutral データとして保持:
  `RUN_ID`/`SCENARIO`/`LANE`/`DRIVER` const と `RealCatalogRow`
  （`case`/`scenario`/`required`）の `REAL_CATALOG` 4 行。
  GPUI・session 型を含まないため feature-gate しない — console が
  default build でも real selector を「未知」ではなく「real lane
  行き」として識別できることが目的。`real_dock.rs`（feature 配下）
  は `LANE` const を共有し、feature-gated test が
  `real_entries()` と `REAL_CATALOG` の case/scenario/required
  一致を検証する（drift は test で fail）。
- `real_dock::catalog_rows()` のような gated accessor は**採用
  しない** — non-feature build の `--run` が real selector を
  正直に拒否するには metadata が非 gated である必要があり、
  `--include-real` の行データも同じ表を使うため、metadata を
  `real_catalog` へ分離する方が単一ソースとして一貫する
  （spec 推奨形からの deviation — 理由をここに記録）。

## Data and Control Flow

### `--list`

1. `HEADLESS_MATRIX.project()` の各 `EntryProjection` を
   `{type:"catalog", case, scenario, driver, required, runnable,
   missing}` の 1 行 JSON で出力（`driver`/`required`/`missing` は
   既存 serde 語彙: kebab-case、`missing` は `null` または
   capability 名）。verdict 語彙は出さない — capability
   projection であり evidence ではない。
2. `--include-real` 指定時:
   - `real-dock` feature あり: `REAL_CATALOG` の 4 行を
     `{type:"catalog", case, scenario, driver:"real-os",
     required, runnable:null, missing:null, session_required:true,
     lane:"real-window-dock"}` で追記。`runnable`/`missing` を
     `null` にするのは、RealOs の capability 提供が session 開始時の
     環境検出（`supported_capabilities()`）であり静的宣言がない
     ため — runnable を名乗らないのが正直。
   - feature なし: exit 2 +「real lane が compile-in されていない」
     メッセージ（要求内容を静かに落とさない fail-closed）。

### `--run`

1. selector parse: `--case`/`--scenario`/`--driver` は繰返し可、
   同一 flag 内は OR・flag 間は AND。`--out <path>` は stdout 既定。
2. real-lane 判定を先に行う: selector 値が `REAL_CATALOG` の
   case/scenario、または `real_catalog::DRIVER`（`real-os`）に
   一致すれば exit 2 +「real 実行は `plugin-dock-real-scenario`
   lane（`mise run test-ui-plugin-dock-real`、opt-in）を使え」。
   console に 2 つ目の real executor は作らない。
3. 各 selector 値が headless catalog と個別に 1 件以上一致するか
   検証 — しなければ「unknown selector」として exit 2。
4. 組合せ filter 後の cell が 0 件なら exit 2（silent no-op 禁止）。
5. `run_selected(env, keep)` で catalog 順に実行 →
   `render_jsonl(RUN_ID, "plugin-dock-console", repo_sha, reports)`
   を `--out`/stdout へ。`lane` field が console 経路を示すため
   部分実行が `test-ui-plugin-dock` lane の証拠と混同されない。
6. exit code: いずれか `Fail` → 1、他は 0、CLI/selector 誤り → 2。

### selector の部分実行 semantics

filter は一致 cell のみを実行する。artifact 依存 case
（`artifact` が `ctx.library`/`ctx.artifact` を満たす）は、選択に
`artifact` が含まれなければ `NotRun`（「plugin artifact unavailable
— see the `artifact` case verdict」）を正直に記録する — 前提 case
がこの run で実行されなかった事実を偽装しない。catalog 順は
binding-major で固定のため、`--case artifact --case editor-attach`
のような選択では `artifact` が先に走り context が満たされる。

## Non-Functional Constraints

- GPUI 非依存: `real_catalog`・bin・`run_selected` は GPUI 型を
  含まない。`check-ui-driver-boundary` green・`boundary.json` 不変。
- verdict 正直性: `--list` は実行証拠に見せない。`--run` は
  `run_entry`/`render_jsonl` の既存 contract を変えず、
  `test-ui-plugin-dock` lane の verdict を非回帰に保つ。
- fail-closed: unknown selector・一致 0 件・real-lane 実行試行・
  CLI misuse は exit 2。`NotRun`/`Unsupported`/`Blocked` は
  記録結果であって失敗ではない（exit 0）。
- 互換性: `run_all`/既存 bin/JSONL schema の外部 contract 不変
  （`run_all` は `run_selected` への委譲で挙動同一）。
- 公開面: 新規公開は `real_catalog` module（struct/const のみ、
  enum 追加なし）と `scenario::run_selected` のみ。`unsafe` なし、
  新規依存なし。

## Risks and Trade-offs

- `REAL_CATALOG` と `real_entries()` の drift — feature-gated
  consistency test（case/scenario/required の zip 一致）で検出。
  `LANE` const は `real_dock` の spec_extras も共有し、listing
  label と report `environment.lane` の乖離を防ぐ。
- selector の OR/AND 規則は console 独自の UX 判断 — 同一 flag
  繰返しを OR にしたのは「複数 case を選んで実行」の自然な形で
  あり、実装は Vec 収集 + per-dimension `any` で済む範囲。
- `--driver` の名前解決は `ScenarioDriver` の serde
  （kebab-case）を唯一の語彙とし、bin に driver 名 literal を
  置かない（real-os 判定も `real_catalog::DRIVER` 経由）。
- 部分実行で `artifact` 未選択の依存 case が `NotRun` になるのは
  意図的 — 「選択に前提が無い実行」を Pass に見せない正直さを
  優先し、design/spec に明記。

## Deferred Design Concerns

- GUI console が必要とする richer projection（filter/sort/group・
  per-driver capability の live query）— GUI slice で要件確定後に
  拡張 | GUI slice 着手時 | 現行 `cells()`/`project()` + bin 側
  selector で不足する軸が出た場合の再設計
- `mise.toml` への console lane task（canonical lane 化）— 別
  Worker 占有中 | 占有解除後の後続 slice | console 検証が cargo
  直実行（補助証拠）のみになる期間
- real lane の console 経由実行 — 意図的に非対応（実行は
  `plugin-dock-real-scenario` が担う）| real executor 統合を
  検討する slice 着手時 | 二重 executor 化の誘惑

## Verification Notes

- [ ] `mise run fmt-check`
- [ ] `mise run check|lint|test ruviform-plugin-dock-test`
  （default + `--features real-dock`）
- [ ] `mise run test-ui-plugin-dock`（JSONL verdict 非回帰）
- [ ] `mise run test-ui-scenario-contract`（fake/gpui 両 lane）
- [ ] `mise run check-ui-driver-boundary`
- [ ] `mise exec -- prek run --all-files`
- [ ] `cargo run -p ruviform-plugin-dock-test --bin
  plugin-dock-console -- --list` / `--run --case <one>` の実出力を
  `evidence/rf631/` に保存（補助証拠 — canonical = mise）
