---
id: "TSPEC-034OpmKTAHuW8dcRI9Rv2Y"
type: "test-spec"
status: "approved"
title: "L1 audio diagnostics counters（Issue #121 — dropped/passthrough/failed/dropped-results 計数・寿命・RT 制約）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/104"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-observability/README.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/endpoint.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
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
  related:
    - "docs/impl/ir/rt-observability-poc.md"
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/adr/issue-121-l1-diagnostics-policy.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-121-diagnostics.md"
    - "docs/designs/issue-121-l1-diagnostics.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 audio diagnostics counters（Issue #121 — dropped/passthrough/failed/dropped-results 計数・寿命・RT 制約）

## Purpose

pin `ruviform-observability` counter pair 在 L1 audio 経路の計数语义 — spec R-B1/R-B2・AC1/AC2 と設計「audio counter の定義」節の 4 counter・計装点・寿命規則。これらは #76 closeout の underrun/dropped-submit/`RESULT_FAILED` 可観測化要求であり、将来の refactoring で counter pair 形状や計装点が変わっても保持すべき不変条件を記録する。実装が完全に別の観測機構に置き換わった場合のみ退役。

## Feature

`InstShared` 所有の `Arc<AtomicU64>`×4（`dropped_submits`・`passthrough_submits`・`results_failed_observed`・`results_dropped`）が in-place `RtLeg`・detached endpoint leg・host query 面で共有され、block submit drop・pass-through 出力・`RESULT_FAILED` 初回観測・result enqueue 失敗を RT-safe に計数する。読み出しは非 RT の `Relaxed` 逐次読みで、counter 間整合 transaction ではない。

## Rules

- **計装点（spec R-B1・設計承認案）**: `dropped_submits` は strict-alternation slot busy または `try_push` 失敗で block ごと最大 +1（busy slot は `try_push` を呼ばないため両原因は同一 block に併発しない）；`passthrough_submits` は `RtLeg::pass_through` 内部で代替出力が実際に成立した block ごとに +1 — **warmup pass-through も計数に含む**（設計レビュー確定・起動時増分は仕様どおりの値）・かつ `endpoint.rs` の arm-anomaly 直接 `leg.pass_through` 経路も捕捉する；`results_failed_observed` は `drain_to` が `RESULT_FAILED` 要素を初回 `try_pop` で観測したとき +1（stale/held 行きも初回 pop で計数・held 再取り出しは二重計数しない）；`results_dropped` は reader 側 `push_result` の `try_push` 失敗で破棄された result 要素のみ +1 — **enqueue 成功後に generation 切替・queue 破棄・RT 未観測で失われる result は含まない**。
- **`results_dropped` の意味限定（設計レビュー確定）**: `results_dropped` と `results_failed_observed` の差を「観測前の消失量」と解釈してはいけない — 両者は異なる事象を数える。`results_dropped` は「enqueue 失敗で破棄された result 数」のみで、全消失量は計れない。
- **寿命（設計・spec R-B1）**: counter は `InstShared` の `Arc` で leg swap・endpoint checkout・`arm`/`arm_apply` で値を保持し、re-mint で新 counter（0 開始）、destroy 後は query が not-found。per-instance 計数の死後保持はしない（死後診断は stderr retention の責務）。
- **RT 制約（spec R-B2・AC2）**: writer は `fetch_add(Relaxed)` のみ — `RtLeg::process`/`drain_to`/`push_result` 経路で alloc・lock・blocking・log write を混入させない。既存 `rt_l1_*` assert_no_alloc 窓は無退行。
- **読み出し语义**: 複数 counter の `Relaxed` 逐次読みは同一時点の整合 snapshot を保証しない — API doc に明記。`u64` `fetch_add` は wrap 仕様（構成上到達不能・~10³ 年）。
- **video 非混入**: audio counter は video route に載らない（spec R-B6 — video 側は stderr/cause parity のみ）。

## Examples

- warmup pass-through 3 回 → `passthrough_submits += 3`
- slot busy で 1 block 破棄 → `dropped_submits += 1`；同一 block で複数失敗判定 → 最大 +1
- `RESULT_FAILED` 初回 pop → `results_failed_observed += 1`；同じ held result 再利用 → 追加増分なし
- reader `push_result` enqueue 失敗 → `results_dropped += 1`；enqueue 成功・RT 未観測 → `results_dropped` は増えない
- leg swap 後 → 累積値維持；re-mint 後 → 0 開始；destroy 後 → query not-found
- endpoint checkout leg の arm-anomaly `pass_through` → `passthrough_submits` も +1（`RtLeg::pass_through` 内部計装で捕捉）

## Guarantees

- 全計装は `Relaxed` atomic のみ — RT 経路に alloc/lock/blocking を導入しない（AC2）
- in-place と detached endpoint 双方の audio 経路で同じ counter が観測可能（AC1）
- counter の意味は設計表どおり — 後続の同値性変更は本 spec の Rules を維持する
- query 面は destroy 後 not-found、re-mint は新値 — 旧 lifecycle の値と混ぜない

## Non-goals

- counter の sink/export・集計ログ・rate 変換（#101/#104 後続の foundation 作業）
- result 消失の全量計測（`results_dropped` の意味限定どおり不可能）
- video route の audio counter 適用
- counter 間の整合 transaction 読み（非要求 — `Relaxed` 逐次読みが契約）

## Risk

- 計装点を `pass_through` 進入点に置くと warmup/未成立 block が混入し spec「実際に出力された回数」に違反
- `RtLeg::process` の 2 call site にだけ計装すると `endpoint.rs:549` の anomaly 経路が漏れ AC1 の「双方観測」が欠損
- counter を `RtLeg` 側に持たせると leg swap で値が失われ lifecycle 契約が破壊される
