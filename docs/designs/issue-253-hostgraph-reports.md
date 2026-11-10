---
id: "DESIGN-034Vj33PXgAsLoeNPU9c8f"
type: "design"
status: "approved"
title: "Issue #253 plugin→host report の host-graph consumer 配線"
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
  implements:
    - "docs/specs/issue-253-hostgraph-reports.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/engine-plugin-host-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
---

# 設計: plugin→host report の host-graph consumer 配線

上流: `docs/specs/issue-253-hostgraph-reports.md`。前提実装:
PR #555（`641f49dd`）の `PluginProcessor::drain_output_reports`
中立契約と VST3 adapter の emit/収集経路
（`docs/designs/issue-253-vst3-midi-canonical-path.md` R2）。

## Context

plugin-core の契約は完成している: plugin は `process` 中に canonical
`Event`（`ParamValue`/`Midi`）を自身の bounded pending buffer に積み、
`drain_output_reports(&mut Vec<Event>)` が since-last-drain 分を
caller の vec に append する pull 型・非 RT の drain である
（`crates/ruviform-plugin-core/src/processor.rs`、
`ErasedPlugin` forwarding は `plugin.rs`）。VST3 host mirror
（`ruviform-plugin-vst3/src/host.rs`）は adapter 出力 queue をこの
契約へ写像済み。

欠けているのは host 側の consumer 面だけである。`PluginHost` /
`InProcessHost` は instance の lifecycle・slot ルーティング・
`Shared` セルの lock protocol を所有しており、plugin の
`&mut`（`Plain`）または `SendCore` 内の `&mut`（`Shared`）へは
host の非 RT verb 経路でしか到達できない。endpoint は `Send` の
RT leg であり report drain の置き場ではない（後述）。

依存方向の制約: `ruviform-graph` は plugin/RT 非依存の純粋な IR
層（`docs/designs/ruviform-architecture-design-baseline.md`）で、
`ruviform-plugin-host` への依存を持てない。逆に host は graph を
知らない。したがって consumer 面は **host 側の pull API** とし、
graph crate は変更しない — graph node が所有する endpoint が RT で
report を生み、host/consumer が非 RT で drain する分業が既存の
ownership split（Issue #113 →
`docs/designs/engine-plugin-host-integration.md`）と一致する。

## Scope

### In Scope

- `InProcessHost::drain_output_reports` と
  `PluginHost::drain_output_reports` — handle + caller 所有
  `&mut Vec<Event>` を取り、since-last-drain の report を append
  する非 RT API。応答型 `ReportDrain`（`Drained`/`Unavailable`）を
  `host.rs` に `BusReport` と同型で追加し `lib.rs` から export。
- L0 `Plain`/`Shared` 両スロットの drain（`Shared` は endpoint
  attach 中を含む）。
- L1 audio / L1 video スロットの `Unavailable` defined 応答。
- fixture E2E（`tests/plugin_graph_e2e.rs`）と unit テスト。

### Out of Scope

- L1 wire への report verb・runner emit（`l1/`、`wire.rs` — 別
  slice。`Unavailable` が正直な現状回答）。
- adapter emit 経路の変更、`ruviform-audio-plugin-bridge` の
  `cap.rs` 変更（編集範囲外・後述の capability 判断）。
- `ruviform-graph` / `ruviform-plugin-core` の変更（契約は既存の
  まま十分 — plugin-core は無変更、graph は依存方向上変更不要）。
- Synvelis/AudioLens の consumer 実装、実 plugin/実 DAW/実
  デバイス検証。

## Components and Boundaries

- **`InProcessHost::drain_output_reports`**（`host.rs`）— L0 の
  唯一の plugin 到達面。`instance_mut(handle)?` で
  `InvalidHandle`、その後 slot で分岐:
  - `Plain` → `plugin.drain_output_reports(out)` を直接呼び
    `Ok(ReportDrain::Drained)`。`buses` と同じく lifecycle gate は
    置かない（report buffer は plugin 内部状態であり、状態に関わらず
    滞留分を引き出せるのが pull 契約の意味）。
  - `Shared` → `Self::shared_verb(slot, &mut instance.lifecycle,
    |_, _, core| { core.plugin.drain_output_reports(out); Ok(..) })`。
    三層 lock protocol（mirror pre-check → `core.lock()` → in-CS
    mirror re-read）をそのまま使い、`Failed` 観測 union・poisoned
    lock の収束は他の `Shared` verb と同一に `Err(HostError::Failed)`。
    endpoint の `try_lock` RT leg と `core.lock()` は同一 mutex で
    直列化されるため、attach 中でも drain は定義された動作をする
    （in-flight の 1 block 分だけ待ち得る — 非 RT 呼出しなので許容、
    `buses`/`SaveState` と同じ性質）。
- **`PluginHost::drain_output_reports`**（`composite.rs`）—
  `slot_mut(handle)?` で generation 検査後:
  - `Slot::InProcess(inner)` → `self.l0.drain_output_reports(inner,
    out)` にそのまま委譲。
  - `Slot::OutOfProcess(_)` / `Slot::OutOfProcessVideo(_)` →
    `Ok(ReportDrain::Unavailable)`。L1 wire に report verb が無い
    事実を error ではなく値で返す — `buses` の `BusReport::
    Unavailable` と同じ honest split（handle は有効、層が供給
    できない）。
- **`ReportDrain`**（`host.rs`、`lib.rs` export）—
  `#[non_exhaustive]` enum、`Debug + PartialEq`（`BusReport` と同じ
  derive セット）:
  - `Drained` — drain が plugin の pending buffer に到達した
    （append 数 0 を含む。caller は `out.len()` 差分で件数を知る）。
  - `Unavailable` — この層は report を供給できない（L1 audio/
    video）。将来 L1 wire が report を運ぶようになれば variant を
    増やすか `Drained` に集約するかはその slice が決める。
- **`PluginEndpoint`** — 変更なし。endpoint は `Send` の RT leg
  であり、report drain は非 RT の host verb である。consumer
  （Synvelis/AudioLens 側の graph owner）は `PluginHost` + handle
  を保持しており、endpoint の所有権を持たなくても drain できる。

## Data and Control Flow

```
plugin.process (RT leg: host.process / endpoint.process_audio)
  └─ plugin pending buffer に canonical Event を積む（bounded、
     溢れは producer 側 defined drop + 計数）        … 既存契約

consumer（非 RT loop: Synvelis/AudioLens/テスト）
  └─ host.drain_output_reports(handle, &mut out)
       ├─ L0 Plain  → plugin.drain_output_reports(out) 直接
       ├─ L0 Shared → shared_verb → core.plugin.drain_output_reports
       │             （endpoint try_lock と mutex で直列化）
       └─ L1/video → Ok(Unavailable)（plugin に到達しない）
```

- report は plugin が生成した順（block 内は producer が frame
  offset で sort 済み — VST3 host mirror の既存挙動）に append
  される。host は順序・offset を再解釈しない。
- drain 間に複数 block が処理された場合、`frame_offset` は各
  block 内相対のまま連結される — block 境界の帰属は consumer の
  drain 周期（例: block 期間ごとに 1 回 drain）で解釈する。
  これは plugin-core 契約の意味論であり host は新しい情報を足さ
  ない。
- `Destroy` で instance が消えると pending report も一緒に捨て
  られる — drain しなかった report の欠損は defined（consumer は
  次の handle で `InvalidHandle` を観測する）。

## Non-Functional Constraints

- **RT contract:** drain は **non-RT only** — `out` への append は
  caller 側 allocation を許し、`Shared` arm は `core.lock()`
  （blocking）を取る。`process`/`PluginEndpoint::process`/
  `process_audio` 経路には一切の drain・blocking・allocation を
  追加しない（spec R1 のユーザー指定 ①）。
- **opt-in:** drain を呼ばなければ report は plugin の bounded
  pending buffer に滞留し、capacity 超過分は producer 側で defined
  drop + 計数される — consumer が opt-in しない限り何も流れない。
- **bounded:** host 側に新しい queue/buffer は作らない。bound は
  producer（plugin/adapter）の declared capacity がそのまま効き、
  host は back-pressure を plugin の pending 満杯で自然に実現する
  （pull 型の利点 — push 型の host-side queue + overflow 経路を
  新設しない）。
- **lifecycle:** `Failed` 観測 union・poisoned lock 収束は既存
  `shared_verb` protocol に完全に乗る。新しい失敗モードを増やさ
  ない。
- **wire 非対象:** `ReportDrain`/`drain_output_reports` は host
  ローカル面であり wire encoding を持たない（`buses`/
  `latency_frames`/`audio_counters` と同じく `HostCommand` でも
  wire verb でもない）。

## Risks and Trade-offs

| risk | mitigation |
| --- | --- |
| consumer が drain を忘れ、report が producer bound で drop される | 既存契約どおり producer が計数 — consumer は drain 周期を自分で管理する責務。host は drop を隠さない（`Unavailable`/`InvalidHandle`/`Failed` は全て defined） |
| `Shared` drain が endpoint の in-flight block と競合 | `core.lock()` が直列化 — 最大 1 block の process 時間だけ待つ。非 RT 呼出しなので許容（`buses`/`SaveState` と同一性質） |
| L1 が将来 report を運んだとき `Unavailable` が嘘になる | `#[non_exhaustive]` enum — L1 wire slice が variant/戻り値を設計し直す。現時点で `Unavailable` は正直 |
| block 境界が report stream から読めない | 契約どおり — consumer の drain 周期で解釈。host が block 番号を付けると producer 契約を超える情報になるため付けない（YAGNI） |

## 検討した代替案

- **`PluginEndpoint` に drain を生やす** — 不採用。endpoint は
  `Send` で RT owner に move される leg であり、report drain は
  非 RT の host verb。endpoint に `&mut self` の非 RT メソッドを
  足しても consumer（host/graph owner）が endpoint を持つとは
  限らず、host+handle を持つ consumer からは依然到達できない。
  host 側 drain が ownership split と一致する唯一の面。
- **host 側 bounded queue へ push** — 不採用。RT path に書き込み
  責務を足すか、process 後に host が plugin から pull して queue
  へ移す二重 buffer が要る。plugin-core の pull 契約が既に
  bounded + back-pressure を持つため、host は中継せず直接委譲する
  のが最小の整合変更。
- **`HostCommand` verb 化** — 不採用。command は serialize 可能な
  plain data で L1 wire と共有する語彙; report drain は caller の
  vec へ append する host-local 面で、`buses`/`latency_frames`/
  `audio_counters` と同じ「method、command ではない」分類に属す。
- **`ruviform-graph` に consumer 型を置く** — 不採用。graph は
  plugin/RT 非依存の IR 層で `ruviform-plugin-host` への依存を
  持てない（依存方向）。consumer が必要とするのは canonical
  `Event`（plugin-core の型）であり、graph IR の関心外。

## Capability 判断（`param_reporting`）

`ruviform-audio-plugin-bridge/src/cap.rs` の VST3
`param_reporting` は **`Declared` のまま維持** とする。根拠:

- 本 slice で証明できるのは「neutral 契約 → L0 host drain →
  consumer」までの fixture E2E であり、cap 行列が問う
  「VST3 adapter の report が実 plugin・実 DAW 往復で正しく
  consumer に届く」end-to-end ではない。
- L1 wire に report verb が無く `Unavailable` — cap の
  `Verified` が意味する到達性の一部（out-of-process 経路）を
  本 slice は供給しない。
- `cap.rs` は本タスクの編集範囲外（`ruviform-audio-plugin-bridge`
  は別 scope）— 変更しないことが scope 規約とも一致する。
- `Declared` の note が指す「no graph consumer wired yet」は
  本 slice で解消されるが、note 文の更新自体も `cap.rs` 編集を
  要するため本 slice では行わず、IR に現状を記録する。
  （consumer 配線が host 側に存在する事実は本 design/IR と
  E2E が証拠。）

## Verification Notes

- [ ] `InProcessHost` unit: `Plain`/`Shared` drain、空 drain、
  連続 drain、`InvalidHandle`、`Failed` 収束（endpoint panic →
  mirror `Failed` → drain `Err(Failed)`）。
- [ ] `PluginHost` unit: L0 委譲、L1 `test_fixture` スロット →
  `Unavailable`、video スロット → `Unavailable`、stale handle →
  `InvalidHandle`。
- [ ] E2E（`plugin_graph_e2e.rs`）: report を出す `Send` fixture
  → `prepare_endpoint` → transform/source node → mock device
  session callback 駆動 → `host.drain_output_reports` で canonical
  `Event` 取得、2 回目 drain は空、L1 instance は `Unavailable`。
- [ ] `rt_` lane 非退行: drain は RT path に存在しないため
  `rt_endpoint_graph_pull_no_alloc` がそのまま証拠になる。
- [ ] canonical 検証: `mise run fmt-check` / `check` / `lint` /
  `test` / `test-rt` / `lint-l1-real-plugin` /
  `test-l1-real-plugin`。
