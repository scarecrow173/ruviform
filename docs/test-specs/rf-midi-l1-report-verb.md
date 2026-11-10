---
id: "TSPEC-034VnJrvHHjxgNUk56P3pD"
type: "test-spec"
status: "approved"
title: "Issue #253 L1 wire report verb テスト仕様"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
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
    - "docs/test-specs/issue-253-hostgraph-reports.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/rf-midi-l1-report-verb-design.md"
  derived-by: []
  verifies:
    - "docs/specs/rf-midi-l1-report-verb.md"
    - "docs/designs/rf-midi-l1-report-verb-design.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/pull/563"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 L1 wire report verb テスト仕様

## Purpose

plugin-core の `drain_output_reports` 契約と L0 host drain
（PR #563）は検証済みだが、L1（out-of-process）経路は wire に
report verb が無く `ReportDrain::Unavailable` を返していた。
本 test-spec は「wire-v4 `DrainOutputReports`/`Reply::Reports`
が canonical `Event` を bounded・順序保存・since-last-drain の
まま runner→host→consumer へ届ける」という保証を固定する。
video report verb・consumer 実装・実機検証が将来変わっても、本
保証の境界（control-plane verb・chunk 意味論・honest
`Unavailable`/error surface）は維持されるべき性質として残す。

## Feature: wire codec（`ruviform-plugin-core/src/wire.rs`）

### Rule WR-01: `Command::DrainOutputReports` は instance-scoped verb として roundtrip する

- **Example**: `Command::DrainOutputReports`（tag 23）を encode →
  decode すると同じ command が復元され、`Message::Command` と
  して instance id を伴って運ばれる。`WIRE_INSTANCE_NONE` を
  instance に持つ frame は scope 検査で reject される（registry-
  level verb ではない）。

### Rule WR-02: `Reply::Reports` は canonical `Event` 列を運ぶ

- **Example**: `EventKind::ParamValue` と `EventKind::Midi` の
  混在列を `Reply::Reports { events }`（tag 8）で encode →
  decode すると offset・kind・payload が完全一致で復元される。
  空 `events` は「pending report 無し」を意味し、decode も
  受理する。

### Rule WR-03: `Reply::Reports` decode は `WIRE_MAX_EVENTS` で allocation 前に bound される

- **Example**: count `WIRE_MAX_EVENTS + 1` の payload は
  `WireError::LimitExceeded`（何も allocate しない）、count が
  残 byte に収まらない場合は `Truncated`、宣言 count より途中で
  切れた event 列は `Truncated`/`Malformed`、event 内の
  `reserved` 非零 word は `Malformed` — いずれも partial
  `Reports` を返さない。

### Rule WR-04: 未知 reply/command tag は従来どおり strict reject

- **Example**: 語彙外の reply tag（9 以降）・command tag は
  `WireError::Malformed` — tag 8 の `Reports` 昇格で既知語彙が
  増えても unknown-tag rejection は緩まない。

### Rule WR-05: `WIRE_VERSION` は 4 — version gate・shm layout doc が整合する

- **Example**: `Hello` handshake・shm header の `wire_version`
  検査・`EVENT_WIRE_SIZE`（33 B）記述が全て version 4 を前提に
  一致し、assert テストが `WIRE_VERSION == 4` を固定する。

### Rule WR-06: 既存の audio/video 語彙・codec 挙動は非退行

- **Example**: 既存 command/reply/event codec の roundtrip・
  bounds・strict rejection test が全て pass — tag 23/8 の追加が
  既存 tag（video `12..=22` 等）や event codec の意味を変えない。

## Feature: runner backlog（`ruviform-plugin-export`）

### Rule RN-01: `DrainOutputReports` は plugin の since-last-drain report を backlog 経由で oldest-first に返す

- **Example**: 実 `ruviform-plugin-l1-runner` child で block を
  処理させると、plugin が pending に積んだ report（入力 event
  echo + deterministic MIDI marker を含む）が
  `Reply::Reports` に producer order で届く。backlog が
  `WIRE_MAX_EVENTS` を超える場合、1 reply は先頭
  `WIRE_MAX_EVENTS` 件のみで残りは次回 drain が返す。

### Rule RN-02: repeat drain は空を返す（since-last-drain）

- **Example**: drain 直後に再度 `DrainOutputReports` すると
  `Reply::Reports { events: [] }` — plugin/runner のいずれの層
  でも「前回以降の新規分」だけが返る。

### Rule RN-03: plugin hook の panic は `guarded` 経由で `HostError::PluginFailed` に写像される

- **Example**: drain を含む全 plugin hook は同一の `guarded`
  機構を通る — `PanicPlugin::init` の panic が
  `Err(HostError::PluginFailed)` になり child が生存することは
  `e2e_plugin_panic_is_plugin_failed_and_child_survives` で
  証明済み。drain 固有の panic fixture は持たないが、drain の
  `guarded` 呼出しは他 verb と同一経路であり、panic 時の
  error surface は verb 間で一貫する。

### Rule RN-04: drain は lifecycle-ungated・audio/video cross-kind rejection を変えない

- **Example**: `Created`/`Initialized`/`Activated`/`Processing`
  のいずれの状態でも drain verb は定義された reply を返す
  （空でも `Reports`）。video entry への audio verb・audio
  entry への video verb の cross-kind rejection は既存どおり。

## Feature: host proxy + composite（`ruviform-plugin-host`）

### Rule HP-01: `L1Instance::drain_output_reports` は short reply まで chunk を集約する

- **Example**: child が `WIRE_MAX_EVENTS` 超の backlog を持つ
  とき、host の 1 回の `drain_output_reports` 呼出しが複数 verb
  を loop して全件を `out` に append する — consumer には 1
  回の pull として連続 stream が見える。chunk 境界で順序は
  崩れない。

### Rule HP-02: `PluginHost::drain_output_reports` の L1 audio slot は `Drained` を返す

- **Example**: `--reports-per-block` を付けた実 `l1-test-worker`
  child + endpoint attach 中の graph 駆動 block の後、
  `host.drain_output_reports(handle, &mut out)` が
  `Ok(ReportDrain::Drained)` + canonical `Event`（`ParamValue`/
  `Midi`、producer order、offset/payload 保持）を返す。pre-seeded
  `out` は保持され、repeat drain は append 無しの `Drained`、
  post-drain block は新規分のみを返す。

### Rule HP-03: video slot は `Unavailable`、error surface は他 verb と同一

- **Example**: `Slot::OutOfProcessVideo` への drain は
  `Ok(ReportDrain::Unavailable)` で `out` 不変（video wire に
  report verb が無い事実を隠さない）。`Failed` instance は
  `Err(HostError::Failed)`、destroy 済み instance への verb は
  `Err(HostError::Ipc)` — drain 固有の失敗モードを増やさない。

### Rule HP-04: drain は非 RT 契約 — RT path に一切触れない

- **Example**: drain は control-plane command/reply のみを使い、
  `process`/doorbell/shm slot/endpoint channel を経由しない。
  `rt_endpoint_graph_pull_no_alloc` 系の RT lane test が非退行
  であることで、RT block 経路に allocation・blocking が無いこと
  が証明される。

## Feature: fixture 駆動（`l1-test-worker` / reference runner）

### Rule FX-01: `--reports-per-block=N` は処理成功 `BlockIn` ごとに deterministic report を queue する

- **Example**: `--reports-per-block=2` の worker が 4 block を
  処理すると、各 block が even `ParamValue`（id `index%8`、
  value `(index%100)/100`）+ odd `Midi` `NoteOn`（key
  `60+index%12`、velocity `index`）の run を queue し、running
  index が block 間で連続する。失敗/reject された block は
  report を queue しない。

### Rule FX-02: reference runner の `GainPlugin` は入力 event echo + deterministic marker を drain 可能にする

- **Example**: `ruviform-plugin-l1-runner` child に `ParamValue`/
  `Midi` を含む block を送ると、drain が入力 event（到着順）+
  block 毎の MIDI marker を producer order で返す — 実 plugin
  実装の report 契約を fixture レベルで証明する。

## Guarantees

- L1（out-of-process）audio 経路の report は consumer が
  `PluginHost::drain_output_reports` を呼ぶだけで canonical
  `Event` として取得できる — adapter 固有構造の再解釈は
  host/runner/wire のいずれにも存在しない。
- report の到達可能性は層ごとに正直: L0 = `Drained`（既存）、
  L1 audio = `Drained`（本 slice）、L1 video = `Unavailable`、
  handle 無効 = `InvalidHandle`、`Failed` = `Err(Failed)`、
  transport/child 異常 = `Err(Ipc)`/`Err(PluginFailed)`。
- wire reply は `WIRE_MAX_EVENTS` で chunk され、decode も同
  bound で allocation 前検査 — 1 verb が無制限 allocation を
  起こさない。
- drain は pull 型・非 RT・opt-in — RT path は report の存在を
  知らず、consumer が呼ばなければ report は producer の bounded
  buffer に留まる。

## Non-goals

- video protocol への report verb — video slot の `Unavailable`
  は維持（別 slice）。
- `ruviform-audio-plugin-bridge` の `unsupported` 計数透過欠落・
  `param_reporting` の `Declared`→`Verified` 昇格 — 別 slice/
  実機検証待ち。
- 実 OS MIDI backend・実デバイス lane・実 DAW 往復・
  macOS/Linux backend・Synvelis/AudioLens の consumer 消費実装。
- block 単位の report 帰属（drain が block 境界を mark しない —
  L0 と同じく consumer の drain 周期で解釈する契約）。

## Risk

- host loop の chunk 集約が wire の `command_timeout` を超える
  巨大 backlog — chunk は `WIRE_MAX_EVENTS` 固定で 1 verb の待ちは
  bounded reply と同程度。timeout は `execute` 既存機構が `Err`
  に写像し hang しない。
- fixture の deterministic report 式と test 側 mirror の乖離 —
  `report_events`/`worker_reports`/`l1_worker_reports` は同一式を
  3 箇所に mirror しており、fixture 変更時に test mirror の同期が
  必要（各所に「mirror」comment 記載済み）。
- drain を呼ばない consumer が report を失う — producer の
  bounded drop + 計数が defined な防御であり、host/runner は
  それを隠さない（L0 と同一契約）。
