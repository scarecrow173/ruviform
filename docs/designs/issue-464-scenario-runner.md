---
id: "DESIGN-GuVirdX8ynQSJId0tz7Dte"
type: "design"
status: "approved"
title: "Issue #464 slice — Scenario Runner・Report 組み立て契約の設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-464-scenario-runner.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/runner.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "docs/specs/issue-464-scenario-runner.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-464-scenario-runner.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-464-scenario-runner.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-464-scenario-runner.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 slice — Scenario Runner・Report 組み立て契約の設計

## Context

`ruviform-ui-test::scenario`（PR #477）には port（`SemanticGuiTest`）、
target 解決（`TargetQuery`/`ScenarioTarget` + epoch/generation pin）、
capability 語彙（`ScenarioCapability`/`ScenarioDriver`）、verdict 語彙
（`ScenarioVerdict`）、report 骨格（`ScenarioReport`）が揃っている。
一方で「scenario/case を宣言して実行し report を得る」経路は各
consumer の手作業であり、capability pre-flight・verdict 写像・
timeout/cleanup 記録の canonical な置き場がない。本 slice はその
組み立て層を `ruviform-ui-test` 内に additive に追加する。adapter
（`FakeGuiTest`/`GpuiGuiTest`/将来の Real OS driver）の実装には
触れない。

## Scope

### In Scope

- `crates/ruviform-ui-test/src/scenario/runner.rs`（新規）:
  `ScenarioSpec`・`ScenarioOutcome`・`ScenarioTimeout`・
  `run_scenario`・`settle_until`。
- `crates/ruviform-ui-test/src/scenario.rs`:
  `pub mod runner;` 追加、`ScenarioReport` に
  `timeout`/`cleanup` field と `new` 初期化を追加。
- `crates/ruviform-ui-test/src/lib.rs`: runner 型の re-export。
- `runner.rs` 内 `#[cfg(test)]` unit test。

### Out of Scope

- spec の Out of Scope と同一。`ruviform-ui`/`ruviform-ui-gpui`/
  `ruviform-observability`・`tests/`・`Cargo.toml`・既存 driver/
  driver_lab/proto には触れない。

## Components and Boundaries

- **`ScenarioSpec`**（宣言）: `run_id: RunId`・`scenario:
  ScenarioId`・`case: Option<CaseId>`・`required:
  BTreeSet<ScenarioCapability>`・`environment:
  Option<serde_json::Value>`・`timeout_ms: Option<u64>`。
  `TargetQuery` と同じ builder 流儀（`new` + `#[must_use]` な
  `case`/`require`/`environment`/`timeout_ms`）。
- **`ScenarioOutcome`**（本体の返り値）: `verdict:
  ScenarioVerdict`・`expected`/`observed:
  Option<serde_json::Value>`・`targets: Vec<ScenarioTarget>`・
  `receipts: Vec<CommandReceipt>`・`evidence: Vec<String>`・
  `detail: Option<String>`。pub field + `new(verdict)`。
- **`ScenarioTimeout`**（記録）: `budget_ms: u64`・`elapsed_ms:
  u64`・`timed_out: bool`。
- **`run_scenario`**（組み立て関数）:
  `pub fn run_scenario<G: SemanticGuiTest>(gui: &mut G,
  spec: &ScenarioSpec, body: impl FnOnce(&mut G) ->
  Result<ScenarioOutcome, ScenarioError>) -> ScenarioReport`。
  `lab_story_walk`/`parity_gesture_scenario` と同じ generic
  `&mut G` 形で、adapter 具象型を失わない。`dyn` 化も可能だが
  既存 helper の流儀に合わせる。
- **`settle_until`**（bounded 待機）: `pub fn settle_until<G:
  SemanticGuiTest>(gui: &mut G, ready: impl FnMut(&G) -> bool,
  max_settles: usize) -> Result<bool, ScenarioError>`。各反復で
  まず `ready` を評価し、偽なら `settle` する。`max_settles` 回
  の settle 後に最終評価して `bool` を返す。sleep を挟まない。

## Data and Control Flow

1. consumer の `#[test]` composition root が adapter を構築し
   （Fake: `FakeGuiTest::new`+mount、GPUI: `GpuiGuiTest`、Real OS:
   `TestDriver` 系 — いずれも Owner 既存実装）、`ScenarioSpec` を
   組み立てて `run_scenario` に scenario 関数を渡す。
2. `run_scenario` は `gui.driver()`/`capabilities()` を読み、
   report skeleton（run/scenario/case/driver/capabilities/
   required/environment）を作る。
3. pre-flight: `gui.require_capabilities(&spec.required)` が
   `Err` なら本体を実行せず `Unsupported`（error が
   `Unsupported` variant の場合）または `Fail` の verdict で
   report を返す。`detail` に capability 名を残す。
4. 本体 `body(gui)`:
   - `Ok(outcome)` → outcome の全 field を report に写し、
     verdict は outcome のものを採用。ただし `timeout_ms` 予算
     宣言時は `Instant` で実測し、`elapsed_ms > budget_ms` かつ
     verdict が `Pass` なら `Fail` に落とし `detail` に timeout
     を記録する（`timed_out = true` は budget 超過時のみ）。
   - `Err(ScenarioError::Unsupported{..})` → `Unsupported`。\n   - `Err(その他)` → `Fail`。`detail = error.to_string()`。
5. `timeout_ms` 宣言時は pre-flight 失敗・本体 error を含む全
   経路で `report.timeout = Some(ScenarioTimeout{..})` を記録する
   （実行しなかった pre-flight 拒否も elapsed≈0 の正直な記録）。
6. `cleanup` は runner では触らない — consumer/driver が run 後に
   `report.cleanup` へ記録する pub field（process exit・window
   close・fixture teardown 等の opaque JSON）。

## Decisions and Rationale

- **runner は 1 関数で executor ではない**: Issue #464 は
  「JSON DSL/独自 Test Executor/二重 Runner」を明示的に禁じる。
  `run_scenario` は port を組み合わせる普通の Rust 関数であり、
  scenario 定義は consumer の普通の関数のまま。
- **verdict の最終判断は consumer**: `ScenarioOutcome.verdict` を
  runner が上書きしないのは「Unsupported/NotRun/Blocked を pass
  にしない」要件と整合する唯一の正直な形。runner が強制するのは
  timeout 超過時の Pass→Fail のみ（予算を超えた実行が pass を
  名乗れないという契約違反の矯正）。
- **timeout は記録 + 事後矯正**: semantic port の同期呼び出しは
  preempt できないため runner は wall-clock を強制しない。
  `timeout_ms` は「この実行に許された予算」の宣言で、runner は
  elapsed を記録し超過時のみ verdict を矯正する。実 OS driver
  側の強制 timeout は #313 Owner の既存実装に委譲する。
- **`settle_until` は述語を `&G` で受ける**: `observe` は
  `&self` の read-only API なので、待機条件は観測のみで書ける。
  `FnMut(&G)` にすることで呼び出し側が `&mut` 再借用を意識せず
  `ready(&*gui)` で評価できる。`max_settles` は settle 回数の
  上限であり、述語評価回数の上限は `max_settles + 1`。
- **`ScenarioReport.timeout`/`cleanup` は additive field**:
  全構築が `ScenarioReport::new` 経由（repo 内 grep 確認済み）
  なので field 追加は `new` の初期化追加のみで済む。外部 consumer
  が struct literal で構築している場合は compile error になるが、
  test-support crate（`publish = false`）の additive 変更として
  許容し PR に明記する。
- **「source revision」の解釈**: Issue #464 の「generation /
  AttachEpoch / source revision を操作・観測時に照合」の
  source revision は consumer domain の `Revision`
  （`StateProjection.revision`/`CommandReceipt.revision`）を指す
  と解釈する。これは view identity ではなく domain 値の順序
  token であり、`observe` が返す `ComponentSnapshot.observed`
  （`StateProjection`）と `post_event` に渡す receipt/projection
  経由で既に照合可能 — view host が revision guard
  （遅延・重複 projection の drop）を内側で行うため、target pin
  への追加は不要。本 slice は `settle_until` の ready 述語で
  revision 条件を観測する経路を提供するに留める。
- **`ScenarioSpec`/`ScenarioOutcome` は struct + pub field**:
  `TargetQuery` の builder と `ScenarioReport` の pub field
  流儀を使い分ける — spec は構築時に組み立てる宣言なので
  builder、outcome/report は実行後に書き込む記録なので pub
  field。

## Non-Functional Constraints

- API は synchronous・non-RT。`Send`/`Sync` を新たに主張しない
  （GPUI `!Send` adapter をそのまま受けられる generic 形）。
- `unsafe` なし。公開 enum を追加しないため
  `#[non_exhaustive]` 追加は不要（struct のみ）。
- doctest: 公開 API の例は unit test で代替（既存 scenario
  モジュールと同じく contract test が正本）。

## Risks and Trade-offs

- **`run_scenario` が万能 runner 化する誘惑**: body は
  `FnOnce` 1 回で scenario は普通の関数のまま — step 列挙・
  DSL 的な実行モデルは持ち込まない。report 組み立てだけを
  引き受ける。
- **timeout の wall-clock 依存**: elapsed は環境で揺れるため
  verdict 矯正は「予算超過時のみ Pass→Fail」に限定し、
  elapsed 値自体を合否根拠にしない（記録のみ）。
- **`ScenarioOutcome.verdict` を信用する構造**: consumer が
  `Pass` を返せば report は `Pass` — これは契約どおり
  （verdict の最終判断は scenario 側）。runner の責務は
  evidence の正直な転記と timeout 矯正に限定する。

## Deferred Design Concerns

- Real OS driver の Ready/Visible/Input 完了条件・明示 timeout
  の adapter 側実装（#313）。
- consumer 側での `ScenarioSpec`/`ScenarioOutcome` 記述例と
  driver-lab manifest への接続（#426/#294）。

## Verification Notes

- [ ] `runner.rs` unit test: pre-flight `Unsupported`・verdict
  写像・timeout 矯正・`settle_until` bounded 動作。
- [ ] `mise run fmt-check` / `check` / `lint` / `test`。
- [ ] `mise run test-ui-scenario-contract`（既存 lane が壊れない
  こと）。
- [ ] `mise run check-ui-driver-boundary`（GPUI 型非露出）。
