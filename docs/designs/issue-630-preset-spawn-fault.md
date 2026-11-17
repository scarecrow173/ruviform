---
id: "DESIGN-630presetSpawnFault03"
type: "design"
status: "approved"
title: "Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/discovery/issue-630-preset-spawn-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-preset-spawn-fault.md"
  implements:
    - "docs/specs/issue-630-preset-spawn-fault.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-preset-spawn-fault.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-l1-runner.md"
  blocks:
    - "docs/plans/issue-630-preset-spawn-fault.md"
  supersedes: []
  superseded-by: []
---

# Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 設計

## 構成

### preset surface（両 ABI・`test_stateful` のみ）

- **VST3**: 既存 component/controller ペアに preset interface を
  分割実装。controller 側に `IUnitInfo`/`IUnitHandler`/
  `IUnitHandler2`（unit/program list 列挙 + preset 選択通知）、
  component 側に `IProgramListData`/`IUnitData`（preset 名解決 +
  state blob get/set）。preset 選択は live gain cell を更新し、
  `RFST` codec と同一 byte 表現で state と preset を一致させる。
  catalog は Factory list 1 本・root unit 1・preset 4 件。
- **CLAP**: `clack-extensions` `preset-discovery` feature を有効化
  し、factory に `PresetDiscoveryFactoryWrapper`、plugin に
  `clap.ext.preset-load`（`PluginPresetLoadImpl`）を実装。
  `state-context` は preset load が既存 `clap.ext.state` blob を
  再利用するため不要と判断し未導入（Coordinator 承認済みの
  条件付き選択肢）。catalog は VST3 と同一 4 preset。
- preset state は既存 state blob を再利用し、新規 codec を
  増やさない。

### spawn/pre-Hello fault

両 `l1-runner` bin に `SpawnFault` を追加。`main` で
`OutOfProcessSpec::args` 由来の argv を走査し、
`--ruviform-test-fault=MODE` を認識した場合のみ fixture load 後・
`run()` 前に `act()` を実行:

- `exit:N` — code N で終了
- `abort` — `std::process::abort()`
- `hang` — 無期限 sleep（親の Hello deadline が判定）
- `stderr:N` — N byte の marker を stderr に書き flush 後 exit 3
- `bad-fixture` — fixture 解決/load を強制失敗させ通常経路で終了

未認識 argv（未知の fault 値を含む）は `run()` にそのまま渡り、
production grammar が usage rejection する（fail-closed）。

### `test_spawn_fail`

- VST3: `CLASS_ROWS` に class を追加（`countClasses` 11）し
  `createInstance` dispatch には含めない → `kInvalidArgument`。
- CLAP: `plugin_count`/`plugin_descriptor` に追加（6 件）し
  `create_plugin` は `None`。
- 両 bridge の `enumerate_params` は create 失敗を握りつぶすため
  scan/descriptor 列挙は正常に完了する。

### L1 oracle

`tests/l1_fault_test_plugin_e2e.rs` に spawn-fault section を追加
（`l1-runner` feature + `#[ignore]`、実 cdylib + 実 runner binary）。
pre-Hello 死亡系は `Transport`/`ExitObserved`/`Internal` の
`PRE_HELLO_DEATH` 集合で受理し、`Handshake`/`Timeout` は厳密 pin。

## 逸脱の記録: 専用 fault bin ではなく runner argv を採用

当初案は spawn-fault 用の専用 bin を新設することだったが、
既存 l1-runner bin の `--ruviform-test-fault=MODE` argv 方式を
採用した（Coordinator 受理済み）。根拠:

1. **no-argv 契約の不変**: `run()` は wire v2 で argv を取らない。
   専用 bin は `run` 呼出前の argv parse を複製する必要があり、
   2 系統の runner 初期化が乖離するリスクがある。argv 方式は
   `run()` 本体に一切触れず、flag は bin 層で消費される。
2. **fail-closed**: 未認識の fault 値・未知 argv は `run()` の
   usage rejection に落ちる。typo した mode が黙って無視される
   ことはない。
3. **pre-Hello 到達性は専用 bin と同等**: fault は fixture load
   後・`run()` 前に発火するため、Hello 前死亡・hang・stderr
   capture の観測面は専用 bin 案と同一。`OutOfProcessSpec::args`
   で per-spawn に渡すため env mutation の並列 race もない。
4. **fixture 解決経路の単一化**: `RUVIFORM_L1_REAL_PLUGIN_DIR`
   による実 cdylib 解決・load 失敗経路（`bad-fixture`）を
   既存 runner と共有できる。

## 検証区分の正直な記録

- **canonical mise vs 補助 cargo**: `mise run test-l1-real-plugin`
  / `test-rt` / `prek` が canonical 検証入口。`cargo fmt` /
  `cargo clippy` / `cargo test` は crate 単位の補助確認として
  実行したもので、CI lane の正本ではない。
- **local 検証 vs 統合検証**: 本 slice の検証は全て local
  worktree 上の実行。`origin/main` merge 後の統合検証は未実施
  （merge は Coordinator 判断）。
- **validator opt-in 再実走**: NotRun。preset surface 追加により
  fixture の ABI surface は変化したが、外部 validator
  （clap-validator / VST3 SDK validator）の再実走は別 slice の
  opt-in runner 管轄であり本 slice では未実施。

## リスクと対応

- pre-Hello 高速死亡の `FailureCause` 分類 race → `PRE_HELLO_DEATH`
  集合 oracle + 厳密 pin の併用で対応。
- `test_spawn_fail` 追加による既存 scan 期待値の破壊 →
  `host_path.rs` の descriptor 期待を更新し、param enumeration
  の graceful degrade を両 bridge で確認済み。
- CLAP `plugin_count` 5→6 の assertion 影響 → host-path test を
  更新。
