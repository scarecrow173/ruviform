---
id: "DESIGN-034VnJlduDjKcQQfErsMwS"
type: "design"
status: "approved"
title: "Issue #253 L1 wire report verb（drain_output_reports の L1 実装）"
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
    - "docs/specs/rf-midi-l1-report-verb.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-253-hostgraph-reports.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/engine-plugin-host-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rf-midi-l1-report-verb.md"
    - "docs/designs/issue-253-hostgraph-reports.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/pull/563"
  defers: []
  deferred-by: []
---

# 設計: L1 wire report verb（drain_output_reports の L1 実装）

上流: `docs/specs/rf-midi-l1-report-verb.md`。前提実装:
PR #563（Issue #253 host-graph consumer 配線 slice）の
`PluginHost::drain_output_reports` / `InProcessHost::
drain_output_reports` / `ReportDrain`
（`docs/designs/issue-253-hostgraph-reports.md`）、および PR #555
（`641f49dd`）の canonical `Event`/`EventKind` codec と
`PluginProcessor::drain_output_reports` 中立契約
（`docs/designs/issue-253-vst3-midi-canonical-path.md`）。

## Context

plugin→host report の中立契約は完成している: plugin は `process`
中に canonical `Event`（`ParamValue`/`Midi`）を自身の bounded
pending buffer に積み、`drain_output_reports(&mut Vec<Event>)` が
since-last-drain 分を caller の vec に append する pull 型・非 RT
の drain である。L0（in-process）は host drain API 経由で既に
consumer に届く。

欠けているのは L1（out-of-process）経路だけである。L1 は
control-plane の `wire::Command`/`wire::Reply` 語彙で runner child
と話し、data plane は shm slot + doorbell で動く。report drain は
plugin の非 RT 状態を読む操作なので、data plane ではなく
**control-plane の command/reply verb** として運ぶのが既存の
責務分割と一致する — `SaveState`/`GetParam` 等と同じ「instance
scoped verb → bounded reply」の形である。

wire 語彙は `WIRE_VERSION` で strict に版管理されており、runner
（`ruviform-plugin-export`）と host（`ruviform-plugin-host` の
`l1/`）は同一 workspace で同時に build される。よって version
bump は同時再 build で吸収され、後方互換ブロックは不要 — ただし
protocol 変更として spec/design/plan/task/IR/test-spec の文書
チェーンに記録する。

## Scope

### In Scope

- `crates/ruviform-plugin-core/src/wire.rs`: `WIRE_VERSION` 3→4、
  `Command::DrainOutputReports`（tag 23）・`Reply::Reports`
  （tag 8）の語彙追加、codec arm、境界/bounds unit test。
- `crates/ruviform-plugin-export/src/lib.rs`: `InstanceEntry` の
  report backlog と `DrainOutputReports` dispatch。
- `crates/ruviform-plugin-export/src/bin/
  ruviform-plugin-l1-runner.rs`: `GainPlugin` の report fixture
  （入力 event echo + deterministic MIDI marker）。
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`:
  `--reports-per-block=N` flag、per-instance pending report queue、
  `DrainOutputReports` dispatch arm。
- `crates/ruviform-plugin-host/src/l1/mod.rs`:
  `L1Instance::drain_output_reports`（chunked loop）。
- `crates/ruviform-plugin-host/src/composite.rs`: L1 audio arm の
  drain 配線（video は `Unavailable` 維持）。
- E2E: `l1_runner_e2e.rs`、`l1_proxy_e2e.rs`、
  `plugin_graph_e2e.rs`。
- 文書チェーン 6 点（spec/design/plan/task/IR/test-spec）。

### Out of Scope

- video protocol の report verb — video slot は `Unavailable` の
  まま（video wire 語彙に report が無い事実を隠さない）。
- `ruviform-audio-plugin-bridge` の `unsupported` 計数透過欠落 —
  別 slice（編集範囲外、PR 本文に記録）。
- `ruviform-midi-device`・`ruviform-audio-core`・consumer 側
  （Synvelis/AudioLens）の drain 消費・実 OS MIDI/実デバイス/
  実 DAW・macOS/Linux backend。
- `cap.rs` の `param_reporting` 昇格 — `Declared` 維持（後述）。

## Components and Boundaries

- **wire 語彙**（`wire.rs`）— `Command::DrainOutputReports` は
  audio domain の instance-scoped verb として tag 23 を割当
  （video command は `12..=22` を占有済みで、23 はその直上の空き）。
  `Reply::Reports { events: Vec<Event> }` は reply tag 8 —
  旧来「未知 tag」として test されていた 8 を正式語彙に昇格させ、
  未知 tag の strict reject test は 9 へ移した。encode は
  `u8 tag + u32 count + encode_event × count`、decode は
  `read_count(WIRE_MAX_EVENTS)` で allocation 前に bound し、
  truncated/malformed/reserved 非零は従来どおり reject。
  `shm::EVENT_WIRE_SIZE`（33 B）と slot sizing の doc は
  `WIRE_VERSION` 4 前提に更新済み。
- **runner**（`plugin-export/src/lib.rs`）— `InstanceEntry` が
  `report_backlog: Vec<Event>` を所有（`CreateInstance` で空初期化、
  `DestroyInstance` で entry ごと破棄）。dispatch arm は
  `self.drain_output_reports()`:
  1. `guarded(&mut *self.instance, |plugin| plugin.
     drain_output_reports(&mut self.report_backlog))` — plugin の
     since-last-drain report を backlog に append（panic は
     `guarded` 経由で `HostError::PluginFailed`）。
  2. backlog の先頭から `min(len, WIRE_MAX_EVENTS)` を
     `drain(..take)` で oldest-first に `Reply::Reports` へ。
     残りは backlog に残り次回 drain が拾う — runner は
     reorder/複製/無制限 buffer を持たない。
- **reference runner fixture**（`ruviform-plugin-l1-runner.rs`）—
  `GainPlugin` に `pending_reports: Vec<Event>` + `blocks: u32` を
  追加。`process` は入力 `EventList` の各 event を到着順に echo
  し、block 毎に deterministic な MIDI `NoteOn` marker
  （`gain_marker(blocks)`）を 1 件 append。`drain_output_reports`
  は pending を caller の vec へ `append`。既存の gain/
  lifecycle/save-restore/panic/underwrite fixture 挙動は不変。
- **test worker fixture**（`l1-test-worker.rs`）— `Flags` に
  `reports_per_block: u64`（`--reports-per-block=N`、default 0）。
  `WorkerEntry` が `pending_reports` + `next_report_index` を所有。
  `process_block` 成功時のみ `report_events(count, base)` を queue
  — even index は `ParamValue`（id `index%8`、value
  `(index%100)/100`）、odd は `Midi` `NoteOn`（key
  `60+index%12`、channel 0、velocity `index`）、`frame_offset` は
  run 内 index。`DrainOutputReports` dispatch は backlog 先頭から
  `WIRE_MAX_EVENTS` まで drain して `Reply::Reports` — 失敗/
  reject された block は report を queue しない。
- **host proxy**（`l1/mod.rs`）— `L1Instance::drain_output_reports
  (&mut self, out)` は `execute(DrainOutputReports)` を loop し、
  `Reply::Reports` chunk を `out` に append。chunk が
  `WIRE_MAX_EVENTS` ちょうどなら backlog 残存の可能性があるため
  再要求し、short reply（`< WIRE_MAX_EVENTS`、0 を含む）で終了。
  他の reply payload は protocol anomaly として `Err(Ipc)`。
  lifecycle-ungated（`post_command_state` wildcard が mirror を
  触らない）で、`execute` の `command_timeout`・`Failed` 収束・
  `InvalidHandle` 相当（instance 消滅 → `Err(Ipc)`）は他 verb と
  同一の error surface。
- **composite**（`composite.rs`）— `PluginHost::
  drain_output_reports` の `Slot::OutOfProcess`（audio）arm を
  `instance.drain_output_reports(out)?` → `Ok(Drained)` に変更。
  `Slot::OutOfProcessVideo` は report verb を持たないため
  `Unavailable` 維持、L0 委譲・stale handle `InvalidHandle`・
  `Failed` 収束は不変。

## Data and Control Flow

```
plugin.process (child 内・非 RT)
  └─ plugin pending buffer に canonical Event を積む（bounded、
     溢れは producer 側 defined drop + 計数）        … 既存契約

consumer（非 RT loop）
  └─ host.drain_output_reports(handle, &mut out)
       ├─ L0            → InProcessHost::drain_output_reports（既存）
       ├─ L1 audio     → L1Instance::drain_output_reports
       │                  └─ loop: Command{DrainOutputReports}
       │                       → child Runner::dispatch
       │                          └─ InstanceEntry::drain_output_reports
       │                               ├─ plugin.drain_output_reports
       │                               │   → report_backlog へ append
       │                               └─ backlog 先頭 ≤ WIRE_MAX_EVENTS
       │                                    → Reply::Reports{events}
       │                  ← chunk append、short reply で終了
       └─ L1 video     → Ok(Unavailable)（report verb 無し）
```

- report は producer が queue した順（block 内は producer が
  frame offset で sort 済み）に append される。wire/host/runner
  のいずれの層も順序・offset を再解釈しない。
- chunk 境界は wire 都合（`WIRE_MAX_EVENTS`）であり意味を持た
  ない — host loop が継ぎ合わせるため consumer には 1 回の pull
  として連続 stream が見える。
- drain を呼ばなければ report は child 側 backlog/plugin pending
  に滞留し、capacity 超過分は producer 側の defined drop — opt-in
  pull 契約は L1 でも不変。
- `Destroy`/child 終了で backlog は entry ごと破棄される —
  drain しなかった report の欠損は defined（consumer は次の
  handle で `InvalidHandle`/`Err(Ipc)`/`Err(Failed)` を観測）。

## Non-Functional Constraints

- **RT contract:** drain は control-plane command/reply のみ —
  `process`/doorbell/shm slot/endpoint channel には一切触れない。
  `out` append の allocation は non-RT caller 責務として許容
  （`buses`/`SaveState` と同じ面）。RT path に drain・blocking・
  allocation を追加しない。
- **bounded wire:** 1 reply は `WIRE_MAX_EVENTS` 件まで — decode
  も encode もこの bound で allocation 前検査する。無制限 backlog
  が 1 frame に載ることはない。
- **bounded child:** runner backlog は plugin の bounded pending
  buffer からの転記であり、plugin が drain するまで増え続けない
  — 実際は drain 毎に plugin pending が空になるため backlog は
  高々「plugin pending capacity + 未回収 chunk」程度に留まる。
  plugin pending の bound が authoritative。
- **lifecycle:** drain verb は lifecycle-ungated — report buffer
  は plugin 状態であり、L0 drain・`buses` と同じ「状態に関わら
  ず滞留分を引き出せる」意味論。`Failed` child/instance は
  `execute` の既存 error surface（`Err(Failed)`/`Err(Ipc)`）を
  そのまま返す。
- **strict codec:** 既存の malformed/truncated/reserved 非零/
  unknown-tag rejection を維持し、version-4 語彙追加で緩めない。

## Risks and Trade-offs

| risk | mitigation |
| --- | --- |
| host loop が `WIRE_MAX_EVENTS` ちょうどの chunk を無限に再要求する | child は毎回 backlog 先頭を drain するため chunk は必ず減少する — 同一 verb の反復で backlog が増える経路は無い（plugin drain は command 処理中にのみ走り、child は serial）。short reply で必ず終了 |
| 大きな backlog が command_timeout を超える | chunk は `WIRE_MAX_EVENTS`（decode bound と同値）で固定 — 1 verb の待ちは他の bounded reply と同程度。timeout は `execute` 既存機構が `Err` に写像 |
| `Reply::Reports` に他 kind が来る protocol anomaly | `L1Instance::drain_output_reports` は `Reports` 以外の reply を `Err(Ipc)` — 「verb を送っていない reply」という defined な anomaly 処理 |
| video slot が将来 report を必要とする | `ReportDrain` は `#[non_exhaustive]` — video wire 語彙拡張 slice が応答形を再設計する。現時点で `Unavailable` は正直 |
| block 境界が report stream から読めない | L0 と同じ契約 — consumer の drain 周期で解釈。host/runner は block 番号を付けない（producer 契約を超える情報を足さない） |

## 検討した代替案

- **data plane（shm slot）に report を載せる** — 不採用。
  report は plugin の非 RT 状態を読む操作で、data plane の
  block 単位 RT 契約（fixed-size slot・doorbell 相関）に乗らない。
  `EventList` codec は既に wire command/reply でも使われており、
  control-plane verb が責務分割と一致する。
- **`HostCommand` verb 化（`host.execute` 経由）** — 不採用。
  `HostCommand` は serialize 可能な plain data で L1 wire と
  共有する語彙; report drain は caller の vec へ append する
  host-local 面であり、`buses`/`latency_frames`/`audio_counters`
  と同じ「method、command ではない」分類に属す（前 slice design
  と同じ判断）。
- **host 側に中継 queue を新設（push 型）** — 不採用。pull 型の
  まま runner backlog + host loop で実現でき、RT path・host
  memory の新しい責務を増やさない。boundedness は producer
  （plugin pending）と wire chunk（`WIRE_MAX_EVENTS`）の二重で
  十分。
- **video にも同時に report verb を足す** — 不採用。video wire
  語彙は別 domain（`12..=22`）で report の consumer 需要が
  未定 — YAGNI。`Unavailable` が正直な現状回答。
- **`WIRE_VERSION` 据え置き + tag 後方互換** — 不採用。tag 空間は
  version と一体で strict 管理されており、旧 decoder は未知 tag
  を reject する。workspace 内同時 build のため bump が最も
  明確な protocol 変更記録になる。

## Capability 判断（`param_reporting`）

`ruviform-audio-plugin-bridge/src/cap.rs` の VST3
`param_reporting` は **`Declared` のまま維持** とする。根拠:

- 本 slice が証明するのは「neutral 契約 → L1 wire → L1 host
  drain → consumer」までの fixture E2E であり、cap 行列が問う
  「実 plugin・実 DAW 往復で report が正しく consumer に届く」
  end-to-end ではない。
- L1 経路が `Drained` を返すようになったが、実 OS MIDI backend・
  実デバイス lane・実 DAW VST3 往復・consumer 消費実装は全て
  未検証のまま — `Verified` の根拠は実機検証を待つ。
- `cap.rs` は本タスクの編集範囲外（`ruviform-audio-plugin-bridge`
  は別 scope）— 変更しないことが scope 規約とも一致する。
- bridge 側 `unsupported` 計数の透過欠落は別 slice — 本 slice の
  PR 本文「未対応の作業」に記録する。

## Verification Notes

- [x] wire codec unit: `DrainOutputReports`/`Reports` roundtrip、
  `WIRE_MAX_EVENTS` 境界（`LimitExceeded`/`Truncated`）、
  `WIRE_INSTANCE_NONE` reject、reserved 非零 `Malformed`、
  未知 reply tag（9 以降）の strict reject、version assert = 4。
- [x] runner E2E（`l1_runner_e2e.rs`）: 実 runner child で空
  drain・`InvalidHandle`・`ParamValue`+`Midi`+echo の producer
  order・repeat drain 空・`StopProcessing` 後 drain・
  deactivate/destroy/shutdown。
- [x] proxy E2E（`l1_proxy_e2e.rs`）: `--reports-per-block` で
  block 処理 → drain が両 kind・順序・repeat 空・destroy 後
  `Err(Ipc)`・`Failed` instance で `Err(Failed)`・1100 件
  backlog の chunked 回収を検証。
- [x] graph E2E（`plugin_graph_e2e.rs`）: 実 `l1-test-worker`
  child + endpoint attach 中に `Drained` + canonical `Event`、
  repeat drain・post-drain 新規分のみ。旧 `Unavailable` 期待を
  `l1_reports_drain_through_host_while_endpoint_attached` に
  置き換え。L0・video `Unavailable` の既存 coverage は維持。
- [x] composite unit: L1 `test_fixture` スロットの drain が
  `Drained` 経路に変わり、video `Unavailable`・stale
  `InvalidHandle` は不変。
- [x] canonical 検証: `mise run fmt-check` / `check` / `lint` /
  `test` / `test-rt` / `test-plugin-core` / `lint-plugin-core` /
  `verify-ci`（結果は IR の Validation 節に記録）。
