---
id: "PLAN-034Z9nTs5uHkY4wMrPdVfC"
type: "plan"
status: "approved"
title: "Issue #630 test_instrument slice — 実装計画"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-test-instrument.md"
  implements:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
  implemented-by:
    - "docs/tasks/issue-630-test-instrument.md"
  depends-on:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
    - "docs/plans/issue-630-test-stateful.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-test-instrument.md"
    - "docs/test-specs/issue-630-test-instrument.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-instrument.md"
    - "docs/designs/issue-630-test-instrument.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-630-test-instrument.md"
  verified-by:
    - "docs/test-specs/issue-630-test-instrument.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_instrument slice — 実装計画

## File map

| file | 変更 |
| ---- | ---- |
| `crates/ruviform-plugin-test-vst3/src/instrument.rs` | 新規: DSP law + `TestInstrumentComponent` + unit tests |
| `crates/ruviform-plugin-test-vst3/src/events.rs` | 新規: `IEventList`→`EventKind::Midi` + `ProcessContext`→`TransportInfo` 独立 translator |
| `crates/ruviform-plugin-test-vst3/src/component.rs` | `InputSurface` enum + `with_surface` + `collect_block_events`/`block_transport` helper |
| `crates/ruviform-plugin-test-vst3/src/controller.rs` | `INSTRUMENT_CONTROLLER_CID` |
| `crates/ruviform-plugin-test-vst3/src/factory.rs` | `countClasses` 8→10、index 8/9 = instrument pair、`CLASS_ROWS` table 集約 |
| `crates/ruviform-plugin-test-vst3/src/lib.rs` | `mod instrument`/`mod events` + docs |
| `crates/ruviform-plugin-test-vst3/Cargo.toml` | `ruviform-media-core` 依存追加 |
| `crates/ruviform-plugin-test-vst3/tests/host_path.rs` | 5 plugin count、instrument discovery、in-process note lifecycle test |
| `crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs` | instrument descriptor pin + MIDI/transport helpers + 4 L1 cases |
| `crates/ruviform-plugin-test-clap/src/instrument_core.rs` | 新規: DSP law の neutral-contract 実装 + unit tests |
| `crates/ruviform-plugin-test-clap/src/instrument.rs` | 新規: clack entry/descriptor/main-thread/processor + tests |
| `crates/ruviform-plugin-test-clap/src/block.rs` | `translate_instrument_event` + `transport_from_clap` + translator 注入 |
| `crates/ruviform-plugin-test-clap/src/{lib,plugin}.rs` | 4 つ目 plugin dispatch + descriptor |
| `crates/ruviform-plugin-test-clap/Cargo.toml` | `ruviform-media-core` 依存追加 |
| `crates/ruviform-plugin-test-clap/tests/host_path.rs` | 4 plugin count、instrument discovery、in-process note lifecycle test |
| `crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs` | instrument descriptor pin + MIDI/transport helpers + 4 L1 cases |
| `crates/ruviform-plugin-clap/src/host.rs` | neutral `EventKind::Midi`→CLAP note/raw-MIDI encode + `TransportInfo`→CLAP transport 投影 |
| `crates/ruviform-plugin-core/src/wire.rs` | `encode_event_record`/`decode_event_record`（33-byte fixed record codec） |
| `crates/ruviform-plugin-host/src/l1/data.rs` | staging を record codec に変更 |
| `crates/ruviform-plugin-export/src/lib.rs` | export decode を per-record codec に変更 |
| `Cargo.lock` | 依存追加の反映 |

## Dependency graph

fixture DSP core（両 format 独立実装）→ fixture ABI translator
（VST3 `events.rs` / CLAP `block.rs` — production adapter と独立）
→ host bridge forwarding（CLAP `host.rs` の MIDI/transport encode
— VST3 は既存経路）→ L1 wire record codec（`plugin-core` wire +
host staging + export decode）→ in-process host_path oracle →
L1 e2e oracle → docs/evidence。production adapter crate
（`ruviform-plugin-{vst3,clap}` の translation/DSP 層）への
fixture 依存なし — 変更は CLAP host bridge の forwarding と
format-neutral wire codec のみ。

## Risk register

- **L1 event wire desync**: 33-byte 固定 slot に可変長 encoding
  → record codec（zero-pad + padding 検証）で対処。decode は
  record 単位で独立 — packed stream 仮定を除去。
- **L1 lookahead latency**: delivered block は source call より
  1〜3 block 遅延 → phase window を latency-margin で設定、
  遷移 call は未 assert。
- **signed zero / underrun ambiguity**: `-0.0` は `== 0.0` で
  判定、underrun は defined silence（全 zero）— sounding block
  の `out[0] == -1.0` で区別。
- **NoteOff 再武装**: fixture law は NoteOff 毎に release 再開始
  → test は単一 NoteOff + empty blocks。
- **offset sustain**: retrigger voice は後続 block に sustain →
  pre-offset silence を仮定しない oracle 設計。
- **VST3 factory 10 class**: `getClassInfo*` 3 variant の重複が
  `too_many_lines` 超過 → `CLASS_ROWS` 単一テーブルに集約
  （情報源の一元化、振る舞い不変）。

## Verification matrix

`cargo fmt --all -- --check` / `cargo clippy -p <touched> --all-targets -- -D warnings` /
`cargo test`（両 fixture crate: unit + host_path + L1 `--ignored`）/
touched production crates（core/host/export/clap/vst3）の
`cargo test` / `mise run test-l1-real-plugin` / `mise run test-rt` /
`prek run --all-files` / `git diff` 禁止パス（adapter translation
層）確認。
