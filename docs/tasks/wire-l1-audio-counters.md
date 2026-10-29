---
id: "TASK-034OpshwEw34KxMBdhQ5mw"
type: "task"
status: "done"
title: "T4. audio counters 計装・query 面（InstShared・RtLeg::pass_through 内部）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/76"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/endpoint.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/tasks/create-ruviform-observability-crate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-121-l1-audio-counters-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T4. audio counters 計装・query 面（InstShared・RtLeg::pass_through 内部）

## Work

`plugin-host/Cargo.toml` に `ruviform-observability` への `path` 依存を追加（下向きのみ）。counter は **`ruviform_observability::{Counter, CounterReader}` pair 型を必須使用**（spec R-F1「`ruviform-observability` の最初の実需要者」— 裸の `Arc<AtomicU64>` 直置きは path dep を死蔵させるため不可）。4 pair（`dropped_submits`・`passthrough_submits`・`results_failed_observed`・`results_dropped`）の所有は **`InstKind::Audio` arm 内**に pin — `InstShared` 本体の field には置かない（`InstShared` は `InstKind::Video` エントリも保持するため、素の field 配置は video エントリに audio counter を持たせる = spec R-B6/parity の非混入 pin に型レベルで反する。`InstKind::Audio { …, counters }` arm 追加で video が counter を持てない構造にする）。writer `Counter` の clone を `RtLeg`/`push_result` 経路へ、reader `CounterReader` を query 面へ。設計表どおり計装:

- `dropped_submits`: `RtLeg::process` の slot busy / `try_push` 失敗 — block ごと最大 +1（busy slot は `try_push` を呼ばない構造上の二重計数防止）
- `passthrough_submits`: **`RtLeg::pass_through` 内部で計装** — `data.rs` の 2 call site と `endpoint.rs:549` の arm-anomaly 経路を両方捕捉・warmup 込み・「代替出力が実際に成立した地点」
- `results_failed_observed`: `drain_to` の `RESULT_FAILED` 要素初回 `try_pop`（stale/held 行きも計数・held 再取り出しは +0）
- `results_dropped`: reader 側 `push_result` `try_push` 失敗のみ（enqueue 成功後の消失は含まない）

`RtLeg::new` の counter 引数対応 — 実サイトは **10 箇所**: production `l1/mod.rs:1988`・in-crate test `mod.rs:3385,3784`・`data.rs:968`（armed_leg test）・`endpoint.rs:780,1083`・`rt_endpoint_arm_apply_no_alloc`（:1295）・`#[path]` harness `l1_proxy_e2e.rs:1185,1305,1497,1691`。harness/テストサイトは `InstShared` 無しで pair を fresh 生成できる形にする（`Counter::pair()` 相当のコンストラクタがあれば counter arg なしでも組み立て可能）。composite に非 RT query 面（handle→`CounterReader`・leg swap/checkout で値維持・re-mint で新 counter・destroy 後 not-found）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`（「audio counter の定義」節）

## Dependencies

- `docs/tasks/create-ruviform-observability-crate.md`（counter pair 型の前提）

## Affected Files

- `crates/ruviform-plugin-host/Cargo.toml`・`src/l1/mod.rs`・`src/l1/data.rs`・`src/endpoint.rs`・`src/composite.rs`

## Done When

- [ ] 4 counter pair（`ruviform_observability::{Counter, CounterReader}`）が `InstKind::Audio` arm で所有され leg swap/checkout/arm で維持される（`InstKind::Video` は counter を持たない — 型レベル pin）
- [ ] `passthrough_submits` が `RtLeg::pass_through` 内部計装で endpoint anomaly 経路も捕捉する
- [ ] query 面が destroy 後 not-found・re-mint で 0 開始を返す
- [ ] `rt_l1_*` assert_no_alloc 窓が無退行

## Verification

- [ ] `mise run check` exits 0
- [ ] `mise run test` exits 0（既存 rt_ を含む）
- [ ] `mise run test-rt` exits 0（assert_no_alloc 無退行）
