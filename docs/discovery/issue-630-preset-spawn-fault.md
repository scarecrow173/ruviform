---
id: "DSC-630presetSpawnFault01"
type: "discovery"
status: "final"
title: "Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 調査記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-l1-runner.md"
    - "evidence/rf630/test-preset-spawn-fault-verification.md"
  related:
    - "../specs/issue-630-preset-spawn-fault.md"
---

# Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 調査記録

## VST3 preset interface の帰属調査

vendored `vst3` 0.3.0 binding（VST 3.8 系）で interface の component /
controller 帰属を確定:

- controller 側: `IUnitInfo`（unit/program list 列挙）、
  `IUnitHandler` / `IUnitHandler2`（preset 選択の通知・復元）
- component 側: `IProgramListData`（preset 名・unit 解決）、
  `IUnitData`（preset state blob の get/set）

したがって preset surface は 1 クラスに同居せず、既存の
component/controller ペア構成に沿って両側へ分割実装する。
preset 選択時の gain 復元は `IUnitHandler::selectUnit` /
program change 経由で live gain cell に書き戻し、state codec
（`RFST`）と同一の byte 表現を使うことで preset と state の
往復を一致させる。

## CLAP preset surface の調査

`clack-extensions` の feature flag を調査:

- `preset-discovery`: factory-level extension
  （`PresetDiscoveryFactoryWrapper` + `clap.factory.preset-discovery`）
  で preset catalog を host に公開
- `state-context`: preset load 時に追加 context を渡す extension。
  preset load が既存 `clap.ext.state` blob を再利用する設計では
  不要と判断（Coordinator ask で `preset-discovery` 承認、
  `state-context` は「必要なら」の条件付き承認 → 不採用）
- plugin 側: `clap.ext.preset-load`（`PluginPresetLoadImpl`）を
  stateful plugin に実装し、preset 選択を gain 復元に写像

## spawn/pre-Hello fault の注入点調査

`ruviform_plugin_export::run` は wire v2 で argv を取らない
（plugin 選択と shared-memory bootstrap は `CreateInstance`
payload へ移動済み）。fault 注入点の候補を比較:

- 専用 fault bin: 新規 bin を増やし `main` で fixture load →
  fault → `run` を複製。pre-Hello 到達性は同等だが、
  fixture 解決・runner 初期化コードが 2 系統に分岐する
- 既存 l1-runner bin + argv flag: `OutOfProcessSpec::args` で
  per-spawn に `--ruviform-test-fault=MODE` を渡し、bin 側で
  `run()` 呼出前に parse・実行。`run()` 本体は無変更で
  no-argv 契約を維持。未認識値はそのまま `run` に落ちて
  usage rejection（fail-closed）

後者を採用（詳細は design doc / ADR）。

## create 拒否 fixture の調査

- VST3: `IPluginFactory::countClasses`/`getClassInfo` に
  `test_spawn_fail` class を列挙し、`createInstance` dispatch
  には含めない → `kInvalidArgument`。
  host bridge の `enumerate_params` は create 失敗を握りつぶして
  空 table を返すため scan 全体は壊れない
- CLAP: `plugin_count`/`plugin_descriptor` に descriptor を追加し
  `create_plugin` は `None` を返す。`enumerate_params` は
  `PluginInstance::new` 失敗を catch して空 table

重要な発見: 両 bridge は ABI create を `Initialize` まで遅延する
（wire `CreateInstance` verb は instance registry の mint のみで
plugin ABI create を呼ばない）。したがって plugin-level create
拒否は `Instantiate` ではなく `Initialize → PluginFailed` として
観測される。

## InstanceLimit 到達可能性の調査

child の instance registry 上限（64 mint）を host API 経由で
埋める probe を試行したが到達不能と判明:

- `Bundled` isolation: 子が満杯になると親が新しい子を spawn する
  （pool が複数子に分散）
- `Dedicated` isolation: instance 毎に専用の子を spawn するため
  単一子の registry は 1 件で頭打ち

`InstanceLimit` は host が発行しない内部 bound であり、probe
test は削除した（evidence に記録）。

## pre-Hello 死亡の FailureCause race

高速な pre-Hello 終了は pipe EOF / child exit 観測 /
channel teardown の順序で `Transport` / `ExitObserved` /
`Internal` のいずれにも分類されうる。`Handshake` / `Timeout`
は厳密 pin し、死亡系は `PRE_HELLO_DEATH` 集合で受理する
oracle に整理した。
