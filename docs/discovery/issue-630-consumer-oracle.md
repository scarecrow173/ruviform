---
id: "DSC-630consumerOracle01"
type: "discovery"
status: "final"
title: "Issue #630 AC7 consumer-oracle slice — 調査記録"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-consumer-oracle.md"
    - "docs/designs/issue-630-consumer-oracle.md"
    - "evidence/rf630/consumer-oracle/manifest.json"
  related:
    - "../specs/issue-630-consumer-oracle.md"
    - "evidence/rf630/consumer-oracle-verification.md"
---

# Issue #630 AC7 consumer-oracle slice — 調査記録

consumer oracle anchor・fixture contract 正本・dep 非汚染の
read-only 実測記録。測定日: 2026-11-21。ruviform worktree HEAD は
`812555d8`（PR #833 merge = origin/main 最新）。

## consumer repo の対象 SHA

| repo | SHA | 備考 |
| --- | --- | --- |
| synvelis | `d68f0e54c09763b93eb556353098c7a92532dfcf` | merge PR #614（ruviform resolved rev `cd4213a1d665600f6adfefa0c3a90f2be9b4e4ce`） |
| audio-lens | `fef857f684997449725e9f1bf38eca354a748d2c` | PR #265（ruviform resolved rev `e4c9f6f0715da3ac479f8c437d21a021327e51da`） |

## oracle anchor の実測（全て探索記録どおり存在）

- `crates/synvelis-player/src/binding.rs` —
  `PluginBindingSession`（line 351）と `check_frame_base`
  （line 120・dispatch 内部でも使用）を確認。dispatch は
  (binding, event) 全件を staging 前に検証し、foreign
  instance/node/generation/clock record を型付き拒否する
  binding oracle。
- `crates/synvelis-player/src/process_boundary.rs` —
  `ProcessBoundarySession`（line 768 付近の session 契約）と
  `PrecisionProvenance`（line 768 — instance/node/capability/
  graph generation の 4 pin で attach 時の delivery context を
  固定し、不整合を typed refusal にする）を確認。
- `crates/synvelis-player/src/plugin_state.rs` +
  `crates/synvelis-project/src/plugin_state.rs` —
  `StateCaptureQuality`（`BestEffort`/`Coordinated`/`Quiescent`
  variant、player 側 line 147-151 で分岐）と
  `PluginStatePort`/`PluginStateHost` adapter を確認。
- `evidence/sv176/pd-01-07-case-definitions.md` +
  `evidence/sv176/acceptance-ledger.md` — PD-01..07 schema dims
  （Case/OS/plugin format/artifact identity 6 dims/host+repo SHA/
  Expected/Observed/Result/Artifact/Owner）と lane 分離
  （Headless/Real OS/Real Plugin ABI）。freeze 宣言どおり Real
  OS/Real Plugin 行は全て `NotRun`、唯一の Unsupported は
  `editor-attach`（upstream `createView` null — #630 側 slice で
  fixture は実装済み）と記録されている。
- `scarecrow173/audio-lens#127` 本文（`gh issue view` で取得）—
  「#629 OS Audio/MIDI仮想Endpointと#630実Plugin Test Suiteは
  AudioLensが現在その機能を製品提供しない限りNotApplicableで
  あり、導入した場合だけOwner/Caseを追加する」の宣言を確認。
  al#127 は plugin host 非提供により NotApplicable 記録のみ。

## consumer Issue 本文の fixture 名指し

- `scarecrow173/synvelis#101`: 「#630の実Plugin ABI Test Gain・
  Stateful・Fault Pluginと#628 VirtualClock」— binding oracle の
  主 fixture は `test_gain`。
- `scarecrow173/synvelis#127`: 「#630のTest Gain/Instrument/
  Stateful/FaultにLoop/Tempo/Automation/Process境界とRTを渡す」
  — process boundary + crash isolation。
- `scarecrow173/synvelis#128`: 「#630の実バイナリ Test
  Stateful/Fault Plugin」— `test_fault` は state capability
  の対照（state 拡張なし → Unsupported 半分）として機能。

## fixture contract 正本の現状（README 補完が必要だった面）

- CLAP README は `plugin_count() == 5` のままで
  `test_spawn_fail`（`org.ruviform-test.clap-test-spawn-fail` —
  実装は `lib.rs` で 6 descriptor、`create_plugin` が `None`
  を返す listed-but-uncreatable）を列挙していなかった。
  `clap.factory.preset-discovery` という第 2 の entry factory
  （provider `org.ruviform-test.clap-presets`、4 preset、
  `clap-test-stateful` に tag）も未記載だった。
- 両 README に `test_instrument`・`test_spawn_fail` の
  semantics 節がなかった。instrument の contract は source
  module doc（voice law・mix law・transport click law）と
  descriptor（VST3 `Instrument|Fx` subCategories・CLAP
  `instrument`/`synthesizer`/`stereo` features・note port
  `CLAP|MIDI` preferred MIDI）から read-only 実測して記述した。
- fault mode の 8 帯域表は spec doc と module doc にのみ存在
  — README `test_fault` 節に帯域表 + normalized arm 値を追加。
- VST3 editor の event token 形式（`f:{state}`/
  `kd:{key:04x}:{key_code:04x}:{modifiers:04x}`/`ku:{…}`/
  `po:{seq}`/`pc:{seq}`、window title =
  `rvf-test-editor-attach-{seq}` + `|` + comma-joined events）は
  `editor_view.rs` の観測面として実在するが README 未記載
  — token 表として追加。CLAP 側に対応 token 面は存在しない
  （popup = floating window verb 契約）。
- preset 面: VST3 は controller `IUnitInfo`+`IUnitHandler(2)`・
  component `IProgramListData`/`IUnitData`・`"Factory"` program
  list（`Default`/`Half`/`Boost`/`Mute` — RFST codec を data
  payload に再利用）、CLAP は `clap.ext.preset-load` +
  preset-discovery factory — 両方 README 未記載だったため
  `test_stateful` 節に追加。

## descriptor identity の実測

- VST3 factory: `countClasses == 12` — 5 component+controller
  対（gain/fault/editor/stateful/instrument）+ `test_spawn_fail`
  （enumerable・`createInstance` → `kInvalidArgument`）+
  `test_stateful_single`。vendor `"Ruviform Test"`・version
  `"0.0.0"`・sdkVersion `"VST 3.x"`・`kManyInstances`。
  subCategories: instrument のみ `"Instrument|Fx"`、他の audio
  class は `"Fx"`、controller は `""`。
- CLAP factory: `plugin_count() == 6` + entry-level
  `clap.factory.preset-discovery` factory。instrument は
  `instrument`/`synthesizer`/`stereo`、他は `audio-effect`/
  `stereo`。note port は `"Notes"` id 0・dialects
  `CLAP|MIDI`・preferred `MIDI`。

## dep 非汚染の実測

`grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test"
../synvelis/Cargo.lock ../audio-lens/Cargo.lock` → exit 1
（no match = 両 consumer 非含有）。audio-lens `Cargo.lock:67`
の `ruviform-audio-test` は #628 dev-dep 経路であり test
fixture crate ではない。実行記録は
`evidence/rf630/consumer-oracle-verification.md`。

## rf#768 との境界

`.rs`・`tests/` を一切触れない lease で占有競合なし。
`l1_fault_test_plugin_e2e.rs`/`l1_stateful_test_plugin_e2e.rs`
の oracle 語彙・law 表は read-only 参照に留めた。
