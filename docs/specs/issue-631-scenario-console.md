---
id: "SPEC-034XWNER8Uft0DiZcXZEtb"
type: "spec"
status: "approved"
title: "Issue #631 Headless Scenario Console（plugin-dock-console bin — merge 済 catalog の headless consumer）"
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
  depends-on:
    - "docs/specs/issue-631-scenario-catalog.md"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-631-real-window-dock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-464-scenario-runner.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Headless Scenario Console（plugin-dock-console bin — merge 済 catalog の headless consumer）

## What Should Be Built

Issue #631（tracker）の後続 slice: merge 済み scenario catalog
（`HEADLESS_MATRIX` / `ScenarioMatrix::project` / `cells` /
`run_entry` / `run_entries` / `render_jsonl`）の **headless consumer**
として、`crates/ruviform-plugin-dock-test` に新規 bin
`plugin-dock-console` を追加する。GPUI 製 Scenario Console / Test Lab
GUI 本体ではなく、catalog の列挙（`--list`）と selector 付き実行
（`--run`）を提供する headless な interaction model である。

本 slice が提供するもの:

- `src/bin/plugin-dock-console.rs` — 手書き `env::args` parse の薄い
  CLI（新規依存なし、feature-gate なしで default build に含まれる）。
- `--list`: `HEADLESS_MATRIX.project()` の 1 行 1 JSON 出力
  （`{type:"catalog", case, scenario, driver, required, runnable,
  missing}`）。capability projection であり evidence ではない —
  verdict 語彙を一切出さない。
- `--list --include-real`: real lane の 4 行を `session_required:true`・
  `lane:"real-window-dock"` 付きで追記。real lane の catalog metadata
  を driver-neutral な非 feature-gated module
  （`src/real_catalog.rs`）へ分離し、console が全 build 構成で
  real selector を正直に識別できるようにする。`real-dock` feature
  なしの build では exit 2 で拒否（fail-closed）。
- `--run [--case <id>] [--scenario <id>] [--driver <name>]`:
  `HEADLESS_MATRIX.cells()` を selector で filter し、既存
  `run_entry`/`run_entries` と同一の catalog 実行経路で走らせ、
  `jsonl::render_jsonl` で現行 lane と同形の JSONL を出力。
  `scenario::run_selected`（新規 `pub fn`）が fresh `HeadlessContext`
  構築と catalog 順 sequencing を担い、`run_all` はその select-all
  投影になる。
- fail-closed selector 意味論: unknown selector・一致 0 件・
  real-lane 実行試行は exit 2。`Fail` verdict は exit 1、
  `NotRun`/`Unsupported`/`Blocked` は記録結果で exit 0。
- `tests/console.rs` 統合テスト、crate README 更新、
  `evidence/rf631/plugin-dock-console.jsonl` と verification 記録、
  本 slice の doc chain。

本 slice が**提供しない**もの（後続 slice / Issue 残 scope）:

- GPUI 製 Scenario Console / Test Lab GUI 本体。
- `test_editor` plugin fixture、実 Native Dock Pass、dock-pixels・
  child HWND input/resize/focus/re-dock の Pass 判定、DComp #532 判定。
- Synvelis/AudioLens consumer への投影、2Window Focus/DPI/IME 実検証、
  3OS 対応、`*-msrv`/`*-full` lane、`mise.toml` への console lane
  task 追加（別 Worker 占有 — PR 本文「未対応の作業」に記録）、
  CI workflow 変更、Issue auto-close。

## Why Now

catalog + driver projection（第三 slice）で case 列挙の単一ソースが
`HEADLESS_MATRIX` に集約されたが、それを消費する操作面はまだ lane
bin（全件実行のみ）しかない。selector 付きの部分実行と capability
一覧は、GUI console slice の前段として headless で先に固定できる —
「catalog 一覧は実行証拠に見せない」「unknown selector は fail-closed」
という verdict 正直性の契約を、GUI 実装から独立に検証可能にする。

## Users and Value

- 開発者/reviewer: `--list` で 8 headless cell + 4 real row の
  capability projection を JSON で取得し、`--run --case <id>` で
  個別 case を lane と同一 verdict で実行できる。
- 後続 GUI console slice: 同じ catalog/selector 契約を GPUI UI の
  backing に再利用できる（本 bin がその headless prototype）。
- evidence 消費者: `--run` の JSONL は `lane:"plugin-dock-console"`
  で full lane と区別され、部分実行が full lane 実績に見做されない。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs` 新規
  （auto-discover、`Cargo.toml` 変更不要・feature なし）。
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs` 新規 —
  `rf631.plugin-dock-real` の driver-neutral catalog metadata
  （`RUN_ID`/`SCENARIO`/`LANE`/`DRIVER`/`REAL_CATALOG` rows）。
  `real_dock.rs` は `LANE` const を共有し、feature-gated test で
  `real_entries()` との一致を検証。
- `crates/ruviform-plugin-dock-test/src/scenario.rs` —
  `run_selected`（selector predicate 付き部分実行）追加、
  `run_all` をその select-all 投影に。
- `crates/ruviform-plugin-dock-test/src/lib.rs` — module/export 追加。
- `crates/ruviform-plugin-dock-test/tests/console.rs` 新規。
- `crates/ruviform-plugin-dock-test/README.md` 更新。
- `evidence/rf631/plugin-dock-console.jsonl` + `verification.md` 追記。
- 本 slice の spec/design/plan/task/test-spec/impl 文書チェーンと
  docs index 再生成。

### Out of Scope

- GPUI console GUI、`test_editor`、実 Native Dock Pass・dock-pixels・
  child HWND 判定、DComp #532、製品 oracle、Synvelis/AudioLens 投影、
  2Window/DPI/IME、3OS、`*-msrv`/`*-full`。
- `mise.toml` task 追加（別 Worker 占有）、`.github/**`、
  `Cargo.lock` 依存差分（新規外部依存なし）、`boundary.json` 変更
  （composition-root grant 既存）、`ruviform-ui-test` の変更
  （bin 側 `cells()` filter で閉じる — driver-neutral selection
  helper は不要と判断）。
- Issue #631 の auto-close（tracker — `Refs #631` のみ）。

## Requirements

- `--list` は `HEADLESS_MATRIX.project()` の capability projection
  のみを出力し、verdict 語彙（`pass`/`fail`/`not_run`/`unsupported`/
  `blocked`）を含まない。catalog 収録 ≠ Pass。
- `--list --include-real` は real lane 4 行を `session_required:true`・
  `lane:"real-window-dock"` 付きで追記するが実行を主張しない。
  `real-dock` feature なし build では exit 2 で拒否。
- `--run` は selector（`--case`/`--scenario`/`--driver`、各 flag は
  繰返し可で同一 flag 内は OR・flag 間は AND）に一致する headless
  cell のみを catalog 順で実行し、既存 lane と同一の JSONL 形状・
  verdict を記録する。JSON DSL・独自 executor・二重 runner は作らない
  （`docs/designs/issue-464-scenario-runner.md` の制約）。
- fail-closed: unknown selector 値・selector 組合せの一致 0 件・
  real-lane case/scenario/driver の `--run` 指定・CLI misuse は
  exit 2（silent no-op にしない）。real-lane 実行は既存
  `plugin-dock-real-scenario` lane に委ね、console に 2 つ目の real
  executor を作らない。
- exit code: いずれかの verdict が `Fail` → 1。`NotRun`/
  `Unsupported`/`Blocked` は記録結果で 0。CLI misuse/unknown
  selector/real-lane 実行試行は 2。
- `RunId` は選択に関わらず `RUN_ID`（`rf631-plugin-dock`）、JSONL
  header の `lane` は `plugin-dock-console`（部分実行が full lane
  の証拠と見做されない provenance）。
- GPUI 非依存: console 経路に GPUI 型を漏らさず
  `check-ui-driver-boundary` green を維持。`boundary.json` 不変。
- verdict の正直性: `--run` 部分実行は実際に走った case の verdict
  をそのまま記録し、`test-ui-plugin-dock` lane の verdict を変えない。
- 公開 enum は enum レベル `#[non_exhaustive]` 方針に従う
  （本 slice の新規公開型は struct/const のみ）。`unsafe` 不使用。
- 新規外部依存・feature/optional 依存の追加なし。

## Acceptance Criteria

- [ ] `src/bin/plugin-dock-console.rs` が存在し、default build で
  `--list`/`--run` が動作する（`Cargo.toml` 変更不要）。
- [ ] `--list` が 8 headless row を `runnable`/`missing` 付きで出力し
  verdict 語彙を含まない。`--include-real`（feature build）が 4 real
  row を `session_required:true`/`lane:"real-window-dock"` 付きで
  追記し、非 feature build では exit 2。
- [ ] `--run` の selector 絞込みが lane と同一 JSONL verdict を出し、
  unknown selector・一致 0 件・real-lane 指定が exit 2 で fail-closed。
- [ ] `tests/console.rs` が上記を網羅し pass する（default +
  `--features real-dock`）。
- [ ] `mise run test-ui-plugin-dock` の JSONL verdict が非回帰、
  `test-ui-scenario-contract`・`check-ui-driver-boundary`・`fmt-check`・
  `prek` が green。

## Deferred Design Concerns

- GPUI 製 Scenario Console / Test Lab GUI 本体 — 本 slice は
  headless interaction model のみ | GUI slice 着手時 | catalog
  projection API が GUI 要件とずれた場合の手戻り
- `test_editor` 到着後の実 Native Dock Pass・dock-pixels 判定 —
  実物 fixture 未到着（`scarecrow173/ruviform#630` 残存 scope）|
  fixture merge 時 | NotRun/Unsupported の長期滞留
- `mise.toml` への console lane task — 別 Worker 占有中 | 占有解除後
  の後続 slice | console の canonical lane 証跡が cargo 直実行のみ
- Synvelis/AudioLens 製品 oracle・3OS 対応 — Deferred
  （POLICIES.md）| 各 product/OS slice 着手時 | driver-neutral
  contract の実証不足

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — 本 spec はその後続
  slice のみを対象）。
- 前 slice: `docs/specs/issue-631-scenario-catalog.md`（catalog +
  driver projection）、`docs/specs/issue-631-plugin-dock-scenario.md`
  （headless lane）、`docs/specs/issue-631-real-window-dock.md`
  （real-window lane）。
- `docs/designs/issue-464-scenario-runner.md` — 「runner は 1 関数で
  executor ではない」制約（console は catalog helper の再利用のみ）。
- `POLICIES.md` — 検証レベル・Pass/Fail/NotRun/Unsupported 規範・
  仮想優先・fail-closed。
- `docs/adr/public-enum-non-exhaustive-policy.md` — 公開 enum の
  `#[non_exhaustive]` 方針。
