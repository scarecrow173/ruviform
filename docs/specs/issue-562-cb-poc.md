---
id: "SPEC-034Xei1fO0DkKhCsfrRdzI"
type: "spec"
status: "approved"
title: "Issue #562 — Generic Alternate Evaluation Context headless 2候補比較 PoC（RF-CB-01）"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/562"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-562-cb-poc-design.md"
      - type: "file"
        path: "docs/test-specs/issue-562-cb-poc.md"
      - type: "file"
        path: "docs/impl/ir/issue-562-cb-poc.md"
      - type: "file"
        path: "docs/impl/exp/issue-562-cb-poc.md"
      - type: "file"
        path: "evidence/rf562/gap-report.md"
      - type: "file"
        path: "crates/ruviform-audio/tests/alternate_evaluation.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "../../POLICIES.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/219"
  defers: []
  deferred-by: []
---

# Issue #562 — Generic Alternate Evaluation Context headless 2候補比較 PoC（RF-CB-01）

## What Should Be Built

固定入力下で 2 つの代替 Graph/Plugin 状態（candidate A/B）を評価し、
State Isolation・Bus/Sidechain・Event・Tail・Latency/PDC・
Preview A/B 切替（Main/Recording 副作用なし）・RT NoAlloc/NoBlocking・
Seek/Loop・Failure の各契約を、Ruviform 既存の Typed Graph /
Offline Render / Probe API のみで headless に検証する PoC。
各能力を Pass/Partial/Gap で採点した gap report を
`evidence/rf562/` に台帳化し、Gap が実証された場合に限り
consumer 非依存の最小 Port 契約と後続 Issue 案を定義する。

## Why Now

Synvelis Creative Branch の比較・試聴に必要な「固定入力下での
代替 Graph/Plugin 状態評価」を、Ruviform が既に提供する
Typed Graph / Offline Render / Probe 能力で実現できるかを
PoC で検証する。不足が実証された場合に限り、DAW 非依存の
Generic Capability Contract を追加する — 重複 Generic API を
避けるための Gate が本 Issue の目的である。

## Users and Value

- Synvelis（Consumer）: Creative Branch の 2 候補比較・Preview・
  Adopt 判定に必要な generic な評価基盤の有無を確定できる。
- Ruviform（Provider）: 既存 API で充足する範囲と真の Gap を
  証拠付きで切り分け、不要な API 増設を回避できる。

## Scope

### In Scope

- headless Fake/Mock fixture による 2 候補 alternate evaluation
  （typed graph 独立 compile/runtime、`PreparedRender` per-instance
  独立性、probe 観測・整列）。
- 決定的 Headless Case による State Isolation / Bus・Sidechain /
  Event / Tail / Latency・PDC / Preview A/B（Main・Recording 副作用
  禁止）/ RT NoAlloc / Seek / Loop / Failure の検証。
- Pass/Partial/Gap 採点の gap report 台帳（`evidence/rf562/`）と
  文書チェーン（spec/design/test-spec/impl-ir/impl-exp）への反映。
- Gap なしの場合の「追加 Generic API 不要」決裁記録。

### Out of Scope

- 実 Plugin・実 GUI・実 Device・Windows L2/L3 検証（他 Owner/後続
  スライス。本 headless 結果を実 Plugin Pass に読み替えない）。
- macOS/Linux の GUI-L2/EXT-L3（Deferred）。
- `*-msrv` レーン、`*-full`、3 OS 横断、新規 CI workflow。
- Synvelis の Project/Track/Clip/Take/Variant/Branch/Adopt/
  Semantic Merge/Product UI 意味論の Ruviform への移入。
- AudioLens f64/長尺/CLI の実走 Gate（#294 側の責務。本スライスは
  対応表への記録のみ）。
- `crates/ruviform-plugin-*` / `crates/ruviform-graph` の src/tests
  編集（Coordinator 承認がない限り read-only 参照）。

## Requirements

- 2 候補は同一の検証済み GraphSpec と固定入力を共有し、mutable
  state（processor instance・slot・probe pair）を一切共有しない。
- 候補の評価は Main/Recording 経路の観測可能な出力・状態を変えない。
- 失敗は bounded（preflight reject / last-good 維持 / stale 除去）
  であり、他候補や Main を汚染しない。
- RT process 経路は計測区間で allocation しない
  （`assert_no_alloc`）。
- Gap 判定は Case ID・Expected/Observed・使用 API を伴う証拠で
  行い、推測で Pass/Gap を付けない。

## Acceptance Criteria

- [x] Ruviform 既存 Graph/Plugin/Offline/Probe API で 2 候補の
  State Isolation/Bus/Sidechain/Event/Tail/Latency を headless
  Fake/Mock fixture により検証し、Generic Gap を記録した
  （`evidence/rf562/gap-report.md`、tests/alternate_evaluation.rs）。
- [x] Preview A/B 切替時の Main/Recording 副作用禁止、RT
  NoAlloc/NoBlocking・Seek/Loop/PDC/Failure を決定的 Headless
  Case で検証した。
- [x] 現行 API で足りる範囲を証拠で確認し、Gap の有無を
  gap report で決裁した（結果: 機能 Gap なし。Consumer 側の
  評価入力同一性宣言と候補識別 bookkeeping が必要 — 後述）。
- [ ] （記録のみ）AL f64/長尺/CLI と SV f32/RT/Creative Branch の
  L1 独立 Oracle・対象 SHA を #294 へ接続 — 本スライスでは
  gap report の対応表記録に留め、実走 Gate は #294 の責務。
- [x] （確認）Windows L2/L3 必須範囲は本スライス対象外として
  NotRun/Deferred に記録し、headless 結果を実 Plugin の
  一律 Gate/Pass に読み替えていない。
- [x] （確認）macOS/Linux L1 は既存 CI レーンに委譲し、CI
  Workflow を増設していない。
- [x] NotRun/Fail を Pass 扱いせず、未使用 OS/Format/機器を
  NotRequired/Deferred と区別して記録した。

## Deferred Design Concerns

- 実 Plugin（VST3/CLAP）・実 GUI・実 Device を用いた Preview A/B
  試聴経路の検証 | 本スライスは headless のみ | Synvelis 側 Owner
  との Windows L2/L3 検証開始時 | headless Pass を実機 Pass と
  誤読するリスク。
- `PreparedEvaluationContext`/`EvaluationReceipt` 等の first-class
  評価コンテキスト型 | 現行 API（独立 runtime + probe anchor +
  既存 receipt）で充足を確認 | 実 Consumer 統合で bookkeeping が
  繰り返し必要と判明した場合 | consumer ごとの重複実装。

## Sources and Relations

- Issue: <https://github.com/scarecrow173/ruviform/issues/562>
- 横断 Consumer Gate: Issue #294（candidate-tuple）/ Issue #219
  （cross-repo impact gate）。
- 前提実装: typed graph（#184/#195 `src/typed/{compile,runtime}.rs`）、
  plugin lifecycle（#196 `ruviform-plugin-{core,host,clap,vst3}`）、
  offline render（#198 `src/render.rs` `PreparedRender`）、
  probe（#185/#308/#309 `src/typed/probe*.rs`）。
- 設計: `docs/designs/issue-562-cb-poc-design.md`、
  テスト仕様: `docs/test-specs/issue-562-cb-poc.md`、
  実装記録: `docs/impl/ir/issue-562-cb-poc.md`、
  実験記録: `docs/impl/exp/issue-562-cb-poc.md`、
  Gap 台帳: `evidence/rf562/gap-report.md`。
