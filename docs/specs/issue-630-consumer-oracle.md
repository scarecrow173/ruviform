---
id: "SPEC-630consumerOracle01"
type: "spec"
status: "approved"
title: "Issue #630 AC7 consumer-oracle slice — fixture contract manifest + consumer oracle 対応表 + dep 非汚染 evidence spec"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-fault-modes.md"
  changes:
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/README.md"
    added:
      - type: "file"
        path: "docs/specs/issue-630-consumer-oracle.md"
      - type: "file"
        path: "docs/designs/issue-630-consumer-oracle.md"
      - type: "file"
        path: "docs/test-specs/issue-630-consumer-oracle.md"
      - type: "file"
        path: "docs/discovery/issue-630-consumer-oracle.md"
      - type: "file"
        path: "evidence/rf630/consumer-oracle-verification.md"
      - type: "file"
        path: "evidence/rf630/consumer-oracle/manifest.json"
  implements:
    - "docs/specs/issue-630-test-plugin-suite.md"
  implemented-by:
    - "docs/designs/issue-630-consumer-oracle.md"
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-fault-modes.md"
    - "POLICIES.md"
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "evidence/rf630/consumer-oracle-verification.md"
    - "evidence/rf630/consumer-oracle/manifest.json"
---

# Issue #630 AC7 consumer-oracle slice — spec

## 目的

Issue #630 AC7「Synvelis #101/#127/#128/#176 および AL #127 の独立
Consumer Oracle を照合し、Ruviform Test Plugin 自体が AudioLens
への新依存を強制しない」のうち、ruviform 側で完結する照合・記録
面を消化する。deliverable は fixture contract manifest・consumer
oracle 対応表・verdict 表・dep 非汚染 evidence で、コード変更を
伴わない docs+evidence 完結の headless slice である。

## 解釈（採用した意味論）

AC7 の「照合」が「consumer 実走まで要求」と解釈される場合に備え、
責務分割を明記する: 各 consumer Issue
（`scarecrow173/synvelis#101`/`#127`/`#128`/`#176`・
`scarecrow173/audio-lens#127`）は #630 fixture の製品 Host 実走を
自 Issue の AC/lane として Own しており、ruviform 側は fixture
contract の正本表と対応表・verdict 記録を提供する。実走 legs は
consumer 側の別 slice の責務であり、本 slice の着手条件でも納品
条件でもない（探索記録
`rf630-ac7-consumer-oracle.explore.json` の採用解釈と同一）。

## スコープ

- `evidence/rf630/consumer-oracle/manifest.json` — schema 名つき
  の fixture contract 正本表。`evidence/rf630/abi-validator/run.json`
  （schema `rf630-abi-validator-run/2`）の先例に倣い schema 名
  `rf630-fixture-contract/1` を付ける。
- `docs/specs`・`docs/designs`・`docs/test-specs`・`docs/discovery`
  の `issue-630-consumer-oracle.md` 4 文書。
- `crates/ruviform-plugin-test-vst3/README.md`・
  `crates/ruviform-plugin-test-clap/README.md` の contract 面補完
  （欠落していた `test_instrument`/`test_spawn_fail` 節・fault mode
  表・editor event token 表・preset 面・stale な
  `plugin_count()` 修正）。
- `evidence/rf630/consumer-oracle-verification.md` — 本 slice の
  検証記録。

## 要件

### 1. fixture contract manifest

`test_gain`/`test_instrument`/`test_stateful`/`test_stateful_single`/
`test_fault`/`test_editor`/`test_spawn_fail` の全 7 fixture class について、
以下の contract 面を正本表として freeze する:

- descriptor identity（class 名 / CLAP plugin id / factory 構成）
- param 面（id/name/domain/normalized mapping/capability flag）
- DSP law
- RFST v2 state blob layout
- fault mode 表
- editor event token（`f:`/`kd:`/`ku:`/`po:`/`pc:` — VST3 のみ）

値の正本は各 crate の README。manifest は README の版（sha256）と
節への pin として記述し、contract 値を複写しない（二重正本化の
禁止 — 詳細は design doc）。

### 2. consumer oracle 対応表

各 consumer Issue の独立 oracle と fixture の写像を記録する:

| consumer Issue | 独立 oracle anchor | fixture |
| --- | --- | --- |
| `scarecrow173/synvelis#101` | `PluginBindingSession`/`check_frame_base`（`crates/synvelis-player/src/binding.rs`） | `test_gain`（param binding・frame offset） |
| `scarecrow173/synvelis#127` | `ProcessBoundarySession`/`PrecisionProvenance`（`crates/synvelis-player/src/process_boundary.rs`） | `test_gain`+`test_instrument`+`test_fault`（process boundary・MIDI/transport・crash isolation） |
| `scarecrow173/synvelis#128` | `PluginStatePort`/`StateCaptureQuality`（`crates/synvelis-player/src/plugin_state.rs` + `crates/synvelis-project/src/plugin_state.rs`） | `test_stateful`(+`_single`)・対照として `test_fault`/`test_spawn_fail`（state capability 差） |
| `scarecrow173/synvelis#176` | PD-01..07 schema dims（`evidence/sv176/pd-01-07-case-definitions.md` + `acceptance-ledger.md`、freeze 済み） | `test_editor`（実 HWND+popup・event token・CLAP floating） |
| `scarecrow173/audio-lens#127` | — | `NotApplicable`（AL は plugin host 機能を製品提供しない — 本文の宣言を記録するのみ） |

### 3. verdict 表

contract-shape が headless で照合できる行は照合結果として記録し、
consumer 側実走が要る行は `external-owner`/`NotRun` のまま残す。
照合不能な行・観測不能な行を Pass と装わない。語彙は POLICIES §3
（Required/Deferred/NotApplicable × Pass/Fail/NotRun/Unsupported）
に統一する。

### 4. dep 非汚染 evidence

両 consumer の `Cargo.lock` に `ruviform-plugin-test-{vst3,clap}`・
`ruviform-plugin-dock-test` が非含有であることの grep 実測を記録
する（コマンド・両 repo の対象 SHA・exit code・観測結果）。

## 受入条件（slice 確定版）

1. `evidence/rf630/consumer-oracle/manifest.json` が schema 名つきで
   存在し、全 7 fixture class の descriptor identity・param 面・
   DSP/state/fault/editor token law が crate README への pin で宣言
   されている（値の二重正本化なし）。
2. consumer oracle 対応表が 5 consumer を網羅する: sv#101→param
   binding、sv#127→process boundary+MIDI/transport+crash
   isolation、sv#128→RFST v2 state quality、sv#176→test_editor
   PD-01..07、al#127→NotApplicable。
3. verdict 表で contract-shape 一致行は headless 照合結果、
   consumer 実走 legs は external-owner/NotRun のまま残っている
   （照合不能行・観測不能行を Pass と装っていない）。
4. 両 consumer `Cargo.lock` の grep 実測（コマンド・両 repo の
   対象 SHA・exit code・結果）が evidence に記録されている。
5. doc chain 4 文書 + verification evidence + 両 crate README が
   揃っている。docs index（`docs/**/README.md`）は未同期のまま
   （Coordinator が統合時に整備）。
6. lease 境界: `.rs`/`tests/`/`mise.toml`/`docs/**/README.md`/
   `Cargo.toml`/`Cargo.lock`/`.github/**`/他リポジトリに差分が
   ない（`git diff` で証明）。

## 非目標

- CLAP output-event の host 側観測経路（`OutputEvents::void` →
  drain 配線）— Issue #630 残件。後続 slice 対象
  （`scarecrow173/ruviform#253` の VST3 report wire 再利用見込み）。
- editor popup の real-HWND host harness / L1 editor 面 —
  Issue #630 残件。`scarecrow173/ruviform#631` real-dock lane 責務。
- AC4 の実 Window/DComp gate（`scarecrow173/ruviform#532` 着地待ちの
  外部 gate・既知 soft）— 本 slice で Pass/Fail を判定しない。
- consumer 実走 legs — 各 consumer Issue の責務。consumer repo
  への書き込み・consumer lane の起動・実走の代行はしない。
- `.rs`/`tests/`/`Cargo.toml`/`Cargo.lock`/`mise.toml`/`.github/**`
  変更、docs index 更新、新規 mise task/lane 追加 — 一切なし。
- al#127 の実走 — AL は plugin host 非提供のため NotApplicable
  記録のみ（無理に対応付けない）。
- 無関係なリファクタリング・既存文書の書き換え — 既存 doc
  chain・evidence は read-only 参照。
- PR は `Refs #630` — AC7 の確定判定は consumer 実走 evidence
  待ち + AC4 gate・残件が残るため Issue は開いたまま。
