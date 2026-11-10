---
id: "TSPEC-034Vj5N4I9PzPBumQHnxgK"
type: "test-spec"
status: "approved"
title: "Issue #253 host-graph report consumer テスト仕様"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
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
  related:
    - "docs/tasks/issue-253-hostgraph-reports.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-hostgraph-reports.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-253-hostgraph-reports.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# テスト仕様: Issue #253 host-graph report consumer

## Purpose

plugin-core の `drain_output_reports` 契約と VST3 adapter の emit
経路（PR #555）は既に検証済みだが、host/graph 側に consumer が
report を引き出す面が無いまま残っていた。本 test-spec は
「host の drain API が canonical `Event` を opt-in・bounded・
非 RT のまま consumer に届ける」という保証を固定する。consumer
実装（Synvelis/AudioLens）や L1 wire report verb が将来変わっても、
本保証の境界（pull 型・honest `Unavailable`・RT path 無変更）は
維持されるべき性質として残す。

## Feature: host drain API（L0）

### Rule DR-01: `Plain`/`Shared` 両スロットで since-last-drain の report を返す

- **Example**: report を出す fixture を `InProcessHost` /
  `PluginHost` で `Processing` まで駆動し `process` を数 block
  回すと、`drain_output_reports(handle, &mut out)` が
  `Ok(ReportDrain::Drained)` を返し、`out` に各 block の
  canonical `Event`（`ParamValue`/`Midi`、frame offset 保存）が
  plugin の生成順で append される。

### Rule DR-02: drain は繰り返し呼べ、空 drain は `out` 不変

- **Example**: 1 回目の drain で全 report を回収した後、process
  を挟まず再度 drain すると `Ok(Drained)` で `out` は増えない。
  process をさらに回してから drain すると新規分だけが返る
  （since-last-drain）。

### Rule DR-03: opt-in — drain しなければ consumer に report は見えない

- **Example**: drain を一度も呼ばずに複数 block process しても
  RT 経路・host の他の verb は一切 report を消費・転送しない。
  report は plugin の bounded pending buffer に滞留し、capacity
  超過分は producer 側の defined drop に留まる（host は観測も
  横取りもしない）。

### Rule DR-04: endpoint attach 中の `Shared` スロットでも drain が成立する

- **Example**: `prepare_endpoint` で endpoint を attach し、
  graph node（`PluginEndpoint::process_audio`）経由で RT leg を
  駆動した後、host 側 `drain_output_reports` が RT leg が生んだ
  report を返す — `Shared` の `core` mutex が endpoint の
  `try_lock` と verb の `lock()` を直列化する既存 protocol の上で
  動く。

### Rule DR-05: caller 所有 vec への append — 既存内容を保持する

- **Example**: `out` に既存 `Event` が入った状態で drain すると、
  既存分はそのまま残り report が末尾に append される
  （`Vec::append` 意味論 — plugin-core 契約の caller-reuse 規約）。

## Feature: defined rejection / honest availability

### Rule AV-01: L1 audio/video スロットは `Unavailable`

- **Example**: `L1Instance::test_fixture` から mint した
  `OutOfProcess` スロット、および video スロットに対する
  `drain_output_reports` は `Ok(ReportDrain::Unavailable)` —
  wire に report verb が無い事実を error にも `Drained` にも
  偽装しない。handle は有効（`InvalidHandle` ではない）。

### Rule AV-02: stale/unknown handle は `InvalidHandle`

- **Example**: 未知 slot・generation 不一致・`Destroy` 済み
  handle への drain は `Err(HostError::InvalidHandle)`。

### Rule AV-03: `Failed` 観測 union は他の `Shared` verb と同一に収束する

- **Example**: endpoint panic で mirror が `Failed` に収束した
  `Shared` スロットへの drain は `Err(HostError::Failed)`。
  poisoned lock も同じ収束（`shared_verb` の既存 protocol —
  drain 固有の失敗モードを増やさない）。

### Rule AV-04: `Plain` スロットは lifecycle gate なし（`buses` と同型）

- **Example**: `Plain` スロットの drain は `buses` と同じく
  lifecycle 状態に関わらず plugin の drain を直接呼ぶ — report
  buffer は plugin 内部状態であり、滞留分を引き出せるのが pull
  契約の意味。

## Feature: 非退行 / RT 契約

### Rule REG-01: RT path に drain・blocking・allocation を追加しない

- **Example**: `process`/`PluginEndpoint::process`/
  `process_audio` の実装・呼出し経路は無変更。
  `rt_endpoint_graph_pull_no_alloc`（`assert_no_alloc` lane）が
  そのまま通ることで、RT block 経路に allocation が無いことが
  証明される。drain API 自体は non-RT only と doc 明記
  （`out` append の allocation 許容、`Shared` arm の blocking
  `core.lock()` 許容）。

### Rule REG-02: 既存 host/endpoint/graph 契約の非退行

- **Example**: `plugin_graph_e2e`・`l1_proxy_e2e`・host.rs /
  composite.rs の既存テストが全て pass — drain 追加が lifecycle・
  endpoint attach・L1 data plane・RT lane のいずれも変えない。

## Guarantees

- L0（in-process）の report は consumer が `PluginHost` /
  `InProcessHost` の `drain_output_reports` を呼ぶだけで
  canonical `Event` として取得できる — adapter 固有構造の
  再解釈は host に存在しない。
- report の到達可能性は層ごとに正直: L0 = `Drained`、
  L1 audio/video = `Unavailable`、handle 無効 = `InvalidHandle`、
  `Failed` = `Err(Failed)`。
- drain は pull 型・非 RT — RT path は report の存在を知らず、
  consumer が opt-in しない限り report は producer の bounded
  buffer に留まる。

## Non-goals

- L1 wire への report verb・runner emit（`Unavailable` が現状の
  正直な回答 — 別 slice）。
- adapter（VST3/CLAP/OFX）の emit 経路変更 — PR #555 の実装を
  そのまま使う。
- `cap.rs` の `param_reporting` 昇格 — fixture E2E は neutral
  契約→L0 host drain の証明であり、実 plugin/実 DAW 往復・L1
  経路を含む capability `Verified` の根拠にはならない
  （`Declared` 維持、design/IR に判断記録）。
- block 単位の report 帰属（drain が block 境界を mark しない —
  consumer の drain 周期で解釈する契約どおりの意味論）。
- 実 OS MIDI backend・実デバイス lane・実 DAW 往復・
  macOS/Linux backend。

## Risk

- drain を呼ばない consumer が report を失う — producer の bounded
  drop + 計数が defined な防御であり、host はそれを隠さない。
  consumer 側の drain 周期管理は利用側 Issue の責務。
- `Shared` drain が endpoint の in-flight block を待つ — 非 RT
  呼出しで最大 1 block 分。RT 遅延に影響しない（RT path は
  `try_lock` のまま）。
- L1 が将来 report を運ぶと `Unavailable` が陳腐化する —
  `#[non_exhaustive]` enum として拡張余地を残し、L1 wire slice
  が応答形を再設計する。
