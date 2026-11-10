---
id: "IMPL-034VjwX2YHM8koidKb6AhG"
type: "impl"
status: "completed"
title: "Issue #253 host-graph report consumer wiring"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-253-host-graph-report-consumer-wiring.md"
      - type: "file"
        path: "docs/specs/issue-253-hostgraph-reports.md"
      - type: "file"
        path: "docs/designs/issue-253-hostgraph-reports.md"
      - type: "file"
        path: "docs/plans/implement-issue-253-hostgraph-reports.md"
      - type: "file"
        path: "docs/tasks/issue-253-hostgraph-reports.md"
      - type: "file"
        path: "docs/test-specs/issue-253-hostgraph-reports.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-253-hostgraph-reports.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-253-hostgraph-reports.md"
    - "docs/plans/implement-issue-253-hostgraph-reports.md"
    - "docs/test-specs/issue-253-hostgraph-reports.md"
    - "docs/impl/ir/issue-253-vst3-midi-canonical-path.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-hostgraph-reports.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #253 host-graph report consumer wiring

## Summary

Issue #253 の残件「plugin→host report の host-graph consumer 配線」を
実装した。`ruviform-plugin-host` に opt-in・pull 型・非 RT の consumer
面 `drain_output_reports` を追加し、`ReportDrain`（`Drained` /
`Unavailable`）で drain 到達と層の非対応を正直に区別する。L0
`Plain`/`Shared` slot は plugin-core の中立契約
`PluginProcessor::drain_output_reports` へそのまま到達し（`Shared` は
pinned three-tier lock protocol 経由）、L1 audio/video slot は wire
protocol に report verb がないため `Unavailable` を返す。`ruviform-graph`
には変更を加えない — 依存方向（graph は plugin-host に依存しない純粋
IR/graph 層）により、graph node が endpoint 経由で駆動する plugin の
report は `PluginHost` + `PluginHandle` を保持する非 RT consumer が
host 経由で drain する形が正しい境界と判断した。headless fixture
（`ReportingGain`）で endpoint→graph node→mock session 駆動中の drain
を E2E 証明した。VST3 `param_reporting` capability は `Declared` のまま
— consumer 配線は本 slice で証明したが、実 plugin/実 DAW 往復と opt-in
実デバイス lane が残るため `Verified` 昇格はしない。

## Context

- 実施タスク: [task](../../tasks/issue-253-hostgraph-reports.md)
- 上流: [spec](../../specs/issue-253-hostgraph-reports.md)、
  [design](../../designs/issue-253-hostgraph-reports.md)、
  [plan](../../plans/implement-issue-253-hostgraph-reports.md)
- 契約保証の正本: [test-spec](../../test-specs/issue-253-hostgraph-reports.md)
- 前提: PR #555（`641f49dd`）の VST3 slice — plugin-core
  `drain_output_reports` 中立契約、VST3 `outputParameterChanges`/
  `outputEvents` emit、host mirror→canonical 復元、cap
  `param_reporting=Declared`。[IR](issue-253-vst3-midi-canonical-path.md)
  が「host-graph consumer への report 配線」を未実施残件として記録。

## Implementation

### Changes

- `crates/ruviform-plugin-host/src/host.rs`:
  - `ReportDrain` enum を追加（`#[non_exhaustive]`、`Debug` +
    `PartialEq`）。`Drained` = pending reports が caller の `out` に
    append された（0 件を含む — consumer は長さ差分で計る）、
    `Unavailable` = その層では report を公開できない（L1 audio/video
    — wire verb 未定義）。`BusReport` と同じ正直な
    reported/unavailable 形。
  - `InProcessHost::drain_output_reports(handle, out) ->
    Result<ReportDrain, HostError>` を `buses` の隣に追加。
    `Plain` slot は plugin を直接呼び（`buses` と同じ lifecycle
    非 gate の形）、`Shared` slot は pinned three-tier lock
    protocol（`shared_verb`）経由で `core.plugin` に到達する。
    stale/unknown handle は `InvalidHandle`、converged `Failed`/
    poisoned `Shared` は `Err(Failed)`（他の全 `Shared` verb と同一の
    収束）。doc は非 RT 契約（`out` append が allocate し得る、
    `Shared` arm は `core` mutex を取る、`process` 内から呼ばない）を
    明記。
  - test module に `ReportGain` fixture（`Send`、bounded pending
    `Vec<Event>`、block 毎に `ParamValue`+`Midi` の report pair を
    queue、`drain_output_reports` で `out.append`）と
    `ReportGainFactory`（`sendable` flag で `Shared`/`Plain` 両 arm を
    同一 producer code で駆動）を追加。
  - unit test 3 件: `drain_output_reports_plain_slot`（pre-process
    の空 `Drained`、sentinel 保存、producer order/offset/payload
    保持、repeat drain の no-append、post-drain の新規分のみ）、
    `drain_output_reports_shared_slot_while_endpoint_attached`
    （endpoint `try_lock` leg と host-side `process` の両経路の
    report が同一 pending buffer に call order で到達）、
    `drain_output_reports_error_surface`（stale/destroyed →
    `InvalidHandle`、endpoint panic 収束 `Failed` → `Err(Failed)`）。
- `crates/ruviform-plugin-host/src/composite.rs`:
  - `PluginHost::drain_output_reports(handle, out)` を `buses` の隣に
    追加。`Slot::InProcess` は `InProcessHost` へ delegate、
    `Slot::OutOfProcess`/`Slot::OutOfProcessVideo` は
    `Ok(ReportDrain::Unavailable)`（wire protocol に report verb が
    なく、report silence を推論させない）。
  - test module に同形の `ReportGain`/`report_pair` fixture
    （per-module duplication は既存 convention）と
    `drain_output_reports_routes_by_slot_kind` test（L0 で report が
    caller に到達 + repeat drain、L1 audio/video で `Unavailable`
    かつ `out` 不変、stale handle → `InvalidHandle`）。
- `crates/ruviform-plugin-host/src/lib.rs`:
  - `ReportDrain` を re-export。crate-level doc の RT 契約節に
    drain surface が非 RT only（caller `Vec` append の allocate、
    `Shared` arm の mutex）で RT path に暗黙 drain がないことを追記。
- `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`:
  - `ReportingGain` fixture（`Send` gain + bounded pending report
    queue、`GAIN_DESCRIPTOR` を共有して `l0_processing` で解決）と
    `report_pair` を追加。
  - `l0_reports_drain_through_host_while_endpoint_attached`: endpoint
    → `transform_node` → `Graph` → mock `Session` 駆動中に
    `PluginHost::drain_output_reports` が `Drained` + producer order
    の canonical `Event`（`ParamValue`/`Midi`、offset/payload 保持）
    を返し、repeat drain・post-drain block の新規分のみを検証。
  - `l1_reports_drain_answers_unavailable`: 実 `l1-test-worker`
    child に対し `Unavailable` + `out` 不変を検証（routing answer
    の証明であり report data ではない）。
- `docs/`: spec → design → plan → test-spec → task の文書チェーンを
  作成（graph router は pre-existing の global artifact-graph
  blocked のため local artifacts で手動構成）。

### Decisions During Implementation

- **consumer 面は host、graph ではない**: `ruviform-graph` は純粋
  IR/graph 層で `ruviform-plugin-host` に依存しない（依存方向契約）。
  `PluginEndpoint` は RT leg であり report を drain しない設計のため、
  graph node が endpoint 経由で plugin を駆動しつつ、`PluginHost` +
  `PluginHandle` を保持する非 RT consumer が host 経由で drain する
  分担が唯一の整合的境界。graph crate の変更はゼロ。
- **`ReportDrain` enum（`BusReport` と同形）**: drain が「到達した
  （0 件含む）」か「層が非対応」かを `Result` の `Ok` 内で区別する
  必要がある — L1 の `Unavailable` を `Err` にすると transport 障害と
  区別できず、report silence の推論を誘発する。`#[non_exhaustive]`
  で将来の drain 状態追加に耐える。
- **pull 型・非 RT・opt-in 維持**: `process`/endpoint/lifecycle の
  いずれにも暗黙 drain を追加しない。boundedness は producer
  （adapter）側 buffer が所有し、host は queue を持たない —
  `drain_output_reports` は「since last drain」の pull であり、
  未消費 report は producer の defined drop まで滞留する。
- **`Plain` は lifecycle 非 gate**: `drain_output_reports` は
  `buses` と同じく `instance_mut` の handle 解決のみで plugin に到達
  し、lifecycle state を検査しない（`Created` でも空 `Drained` を
  返す）。`Shared` のみ `shared_verb` の `Failed`/poisoned 収束を
  通る。
- **L1 は `Unavailable`（誤魔化さない）**: L1 wire protocol に
  report verb が存在しないため、L1 audio/video slot は
  `ReportDrain::Unavailable` を返す。`Ok` + 空 `out` にすると
  「report が無い」のか「読めない」のか区別できないため拒否。
- **capability は `Declared` 維持**: consumer 配線（host-graph 面）は
  本 slice で fixture E2E 証明したが、`param_reporting` cap が指すのは
  adapter→host の report 経路全体であり、実 plugin/実 DAW 往復と
  opt-in 実デバイス lane が未検証。`Verified` 昇格の根拠は実機検証を
  待つ（design/IR に明記）。
- **`PluginEndpoint` 無変更**: endpoint は `Send` + `try_lock` +
  allocation-free の RT leg であり、report drain は非 RT の host verb
  に留める — endpoint に report surface を追加すると RT contract を
  曖昧にする。

## Related Experiments

### Adopted

<!-- なし — 実験的分岐なし（既知解の機械的適用 + 設計判断は design に記録）。 -->

### Rejected

<!-- なし。 -->

## Validation

実施済み（Windows 11 x64, rust 1.99.0 via mise — canonical）:

- `git fetch origin main && git merge origin/main` — Already up to
  date（HEAD `ab2ebfe1` が origin/main を包含、merge commit なし）。
- `mise run fmt` → `mise run fmt-check` — PASS
- `mise run check` — PASS（`cargo check --workspace --all-targets`）
- `mise run lint` — PASS（`cargo clippy --workspace --all-targets
  -- -D warnings`。初回 `assert_is_empty` 指摘を `assert_eq!(out,
  [])` に修正して再検証）
- `mise run test` — PASS（`cargo test --workspace`、200
  `test result: ok`・失敗 0。新規 unit 4 件 + e2e 2 件を含む。
  `cargo test -p` 補助実行で見えた `l1_proxy_e2e` の runner-bin
  未 build 失敗は workspace build で解消）
- `mise run test-rt` — PASS（`cargo test --workspace rt_`、155
  `test result: ok`・失敗 0 — `rt_endpoint_graph_pull_no_alloc` 等の
  RT lane 非退行を確認）
- `mise run lint-l1-real-plugin` — PASS
- `mise run test-l1-real-plugin` — PASS（fixture cdylib + host
  runner build、`l1_real_adapter_e2e -- --ignored` の
  `real_adapter_l1_full_series` が CLAP/VST3 両 adapter で PASS）

補助証拠（canonical = mise）:

- `mise exec -- cargo test -p ruviform-plugin-host --lib
  drain_output_reports` — 4 tests PASS
- `mise exec -- cargo test -p ruviform-plugin-host --test
  plugin_graph_e2e` — 15 tests PASS

review-protocol: reviewer A（cooperative diff read）/ reviewer B
（adversarial probe）/ validator / judge を separated passes で実施
— test-spec DR-03 の verb-interleave 例に対する coverage gap を
CONFIRMED P2 として検出し、`buses` verb を processing と drain の
間に挟む assert を plain-slot test に追加して解消。他に CONFIRMED
P0/P1 なし。

NotRun / Unsupported:

- 実 OS MIDI backend・opt-in 実デバイス lane・実 DAW 往復 — Issue
  #253 の残 gate（本 slice 対象外、実機必須）。
- `param_reporting` の `Verified` 昇格根拠となる実 plugin/実機検証 —
  未実施のため `Declared` 維持。
- Synvelis/AudioLens 側の consumer 利用実装 — 別 Issue。

## Risks

- `Shared` slot の drain は `core` mutex の blocking `lock()` を取る
  — attached endpoint の `try_lock` RT leg と競合した場合、RT leg は
  defined busy bypass（`EndpointUnavailable`）を返す。非 RT verb が
  RT leg を一時的に bypass させる既存 protocol の範囲内であり、
  drain が RT path を block することはない（逆方向: RT leg が
  drain を待たせることもない — `try_lock` は待たない）。
- `Destroy` は pending reports を破棄し得る（defined behavior —
  drain は instance 生存中の pull であり、teardown 時の flush 契約は
  存在しない）。
- block を跨いだ report stream には block-boundary mark がない —
  per-block 帰属は consumer の drain cadence が担う（host は
  producer order と within-block offset をそのまま運ぶのみ）。

## Follow-ups

- Issue #253 は open のまま: 実 OS MIDI backend（macOS/Linux 含む）、
  opt-in 実デバイス lane、実 DAW 往復、L1 wire への report verb 追加
  （`Unavailable` を `Drained` にするには wire protocol 拡張が必要）
  が残る。
- Synvelis/AudioLens の consumer 側消費実装は別 Issue。
