---
id: "TSPEC-034VERR0DyAdyknxAagH1U"
type: "test-spec"
status: "approved"
title: "Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane テスト仕様"
created: "2026-11-11"
updated: "2026-11-11"
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
    - "docs/test-specs/issue-253-os-midi-backend.md"
    - "docs/test-specs/rf-midi-l1-report-verb.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-253-unsupported-transparency-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-253-unsupported-transparency.md"
    - "docs/designs/issue-253-unsupported-transparency-design.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 bridge unsupported 計数透過 + opt-in 実機 MIDI lane テスト仕様

## Purpose

`MidiReadStats.unsupported` が adapter から bridge の全計数面
（`PumpStats`/`InputTx`/`InputStageCounters`/`NodeStatus` snapshot）
まで silent でなく透過することを固定する。backend の語彙外
traffic（SysEx・system common/realtime・malformed）は overload
loss（`dropped`）と別計数でなければならない — 本 test-spec は
「unsupported を読み捨てない」「`dropped` と混ぜない」「since-
last-read 消費意味論を保つ」という不変条件を守る。実機依存の
値（event 有無・非零計数）は Pass 条件にせず、契約上不変の
性質のみを assert する。

## Feature: mock 注入面（`ruviform-audio-core/src/midi_input/mock.rs`）

### Rule UT-01: `inject_unsupported` は since-last-read で一度だけ surface する

- **Example**: `port.inject_unsupported(3)` 後の最初の `read` は
  `stats.unsupported == 3` を返し、次の `read` は
  `stats.unsupported == 0` を返す。実 backend の counter
  drain（`queue.rs::Shared::drain`）と同じ消費意味論。

### Rule UT-02: `unsupported` は `dropped` と独立に計数される

- **Example**: `inject_dropped(2)` + `inject_unsupported(5)` 後の
  `read` は `stats.dropped == 2 && stats.unsupported == 5` を返す。
  片方の注入が他方を汚染しない。

## Feature: `InputStage::read` direct-read 経路（`tests/midi_delivery.rs`）

### Rule ST-01: `stats.unsupported` は `InputStageCounters::unsupported_backend` に累積される

- **Example**: `unsupported=4` を返す scripted input を
  `stage.read` すると `stage.counters().unsupported_backend == 4`。
  2 回目の read でさらに `unsupported=3` なら累積 `7`。
- **Example**: `unsupported` を返さない input（`0`）では
  `unsupported_backend == 0` — 既存挙動の回帰なし。

### Rule ST-02: `unsupported` は event 配送・reject・dropped 計数を変えない

- **Example**: `unsupported=4` + valid events + `dropped=3` の
  input で `counters.unsupported_backend == 4` かつ
  `counters.dropped_backend == 3` であり、event の
  mapped/delivered は unsupported 計数の影響を受けない。

## Feature: `InputTx::pump` channel 経路（`tests/midi_wiring.rs`）

### Rule TX-01: `PumpStats.backend_unsupported` は per-call の read 計数を返す

- **Example**: `MockBackend` の port に `inject_unsupported(4)`
  して `input_tx.pump(&mut *input)` すると `stats.
  backend_unsupported == 4`。

### Rule TX-02: `InputTx::unsupported_backend()` は pump 越しの累積を返す

- **Example**: `inject_unsupported(4)` → pump、`inject_unsupported(3)`
  → pump で `input_tx.unsupported_backend() == 7`。
  `dropped_backend()` は unsupported 注入では増えない。

### Rule TX-03: channel 経路の `InputStageCounters::unsupported_backend` は 0 を維持する

- **Example**: `InputTx::pump` 経由で unsupported が観測されても、
  `control.snapshot().input.unsupported_backend == 0`。
  `MidiInput::read` を呼ぶ側（InputTx）が計数 owner という責務
  境界を固定する（design §3 — `dropped` と同一の分割）。

## Feature: `NodeStatus` mirror（`status.rs`）

### Rule NS-01: `set_input` は `unsupported_backend` を absolute store で mirror する

- **Example**: `InputStage::read` で `unsupported_backend=4` を
  累積した stage の counters を `set_input` すると
  `status.snapshot().input.unsupported_backend == 4`。再 mirror で
  同値を上書き（加算ではない — stage が authority）。

## Feature: `#[ignore]` 実機 E2E（`ruviform-midi-device/tests/ports.rs` + bridge `tests/midi_wiring.rs`）

### Rule RL-01: 非対応 OS / port 無しは失敗ではなく honest skip である

- **Example**: `backend().input_ports()` が
  `Err(DeviceError::Unsupported)` なら記録して return。
  `Ok(vec![])` なら記録して return。どちらも test fail ではない。

### Rule RL-02: 実 port では open→read→close→StreamInvalidated が成立する

- **Example**: port が存在すれば `open()` → 複数回 `read` →
  `close()` → 後続 `read` が `Err(DeviceError::StreamInvalidated)`。

### Rule RL-03: 受信 event の stamp は port 宣言 domain と Carried provenance を持つ

- **Example**: `provides_timestamps` を宣言する port から届いた
  各 event は `stamp.domain == info.capabilities.clock_domain`、
  `stamp.provenance == Carried`、ticks は単調非減。
  unsupported/dropped 観測値は assert せず記録のみ。

### Rule RL-04: bridge 実機経路でも計数は一貫して透過される

- **Example**: `SystemBackend`→`InputTx::pump` で、全 pump の
  `backend_unsupported` 合計 == `input_tx.unsupported_backend()`、
  全 `backend_dropped` 合計 == `input_tx.dropped_backend()`。
  `close` 後の `pump` は `Err(StreamInvalidated)`。

## Coverage Notes

- unit/integration 層（UT/ST/TX/NS）で注入→透過の deterministic
  検証を完結させ、実機 E2E（RL）は scaffold + 実機有無の honest
  記録に留める。実 hardware traffic の非零 unsupported は
  assert しない（発生させる物理手段が lane 内に無い）。
- RT lane（`test-rt`）: 変更は計数追加のみで RT-suitable
  surface の allocation 形状を変えない — 既存 RT test が
  そのまま回帰検証になる。
