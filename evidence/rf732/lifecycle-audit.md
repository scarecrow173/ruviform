# Issue #732 — production CLAP adapter lifecycle 監査報告

対象: `crates/ruviform-plugin-clap`（clack 0.2.0 / clap-sys 0.5.0・
cdylib `ruviform_plugin_clap.dll`・plugin id
`org.ruviform.clap-gain-poc`）。
先例: `evidence/rf722/lifecycle-audit.md`（VST3 同型監査・PR #731）。
環境: Windows（worktree `rf732-clap-audit`）、rustc は mise pin、
Node v26.x、clap-validator 0.4.1（pinned release binary）。

## 結論サマリ

**2 件の契約欠陥を発見・修正した**（いずれも「受理 phase 過狞」
パターンの CLAP 版 — fixture が clap-validator 実走で確立した契約
への production divergence）。terminalization・bus/port info・
latency/tail・process status の各経路に欠陥はなかった。

| # | 欠陥 | 検出 | 修正 |
| --- | --- | --- | --- |
| D1 | params `get_info` が `module: b"/"` を報告 — clap-validator が leading `/` の module 名を拒否（param 系 9 leg 失敗） | clap-validator 実走 | `module: b""`（fixture と同じ root module）+ wire metadata を `gain_param_info()` に集約して unit test で pin |
| D2 | `to_process_config` が fractional sample rate を `rate.fract() != 0.0` で拒否 → 1234.57 Hz で `activate` false（`process-varying-sample-rates` 失敗） | clap-validator 実走 | fixture と同じく `rate.round()` で整数 Hz に丸めて受理 |

core（`ruviform-plugin-core`）は無変更 — 条件付き lease 不使用。

## 監査方法

1. CLAP 仕様一次情報: `include/clap/plugin.h`（lifecycle 契約:
   init/activate/start_processing/process/stop_processing/reset/
   deactivate/destroy の `[thread & state]` 条件、audio-ports・
   params・latency/tail extension の意味論）。
2. clack 0.2.0 binding: `PluginWrapper`/`PluginInstance` の状態機械と
   raw C callback の対応を crate ソースで確認。
3. adapter 実コード照合: `plugin.rs`・`adapt.rs`・`gain.rs`・
   `host.rs`（read-only）と共有中立機械
   `crates/ruviform-plugin-core/src/lifecycle.rs`。
4. **fixture との divergence 照合**: `crates/ruviform-plugin-test-clap`
   （rf#630・clap-validator 全 leg Passed の実績）の対応実装と
   production を対比 — D1/D2 はこの対比で根本原因を確定した。
5. 外部検証: clap-validator 0.4.1 による production leg 実走
   （`evidence/rf732/abi-validator/`・red は `abi-validator-red/`）。

## パターン別判定

### 1. terminalization — 欠陥なし

| 観点 | 判定 | 根拠 |
| --- | --- | --- |
| CLAP `destroy` の終端性 | ✅ 契約どおり終端 | CLAP は plugin instance ごとに `destroy` が終端操作（再 init 経路なし）。中立機械 `Destroy → Destroyed`（終端・全 event 拒否、`lifecycle.rs`:21-24,55-63）と一致 |
| destroy の前提 | ✅ | clack `PluginInstance::drop` が active processor を残したまま destroy する場合に備え自動 deactivate。adapter の `Shared::drop`（`plugin.rs`:139）は `LifecycleEvent::Destroy` を遷移し、`Activated`/`Processing` からの Destroy は中立機械が拒否 → drop は記録のみで panic しない（fail-closed） |
| destroy 後のイベント・二重 destroy | ✅ 拒否 | `Destroyed` は全 event を `IllegalTransition` で拒否。`Shared::drop` は `Drop` で 1 回のみ、二重 Destroy も中立機械が拒否 |
| init の 1 回制限 | ✅ | clack `PluginWrapper` は init 成功後にのみ instance を構築し、`UNINITIALIZED`/`INITIALIZING` 状態の callback を `PluginCalledDuringInitialization` で拒否。adapter `initialize`（`plugin.rs`:103）は `Created → Initialized` |
| **VST3 との非対称（重要）** | ✅ 正しい差異 | VST3 `terminate` は COM object を再 initialize 可能にする SDK 契約のため `Created` リセットが必要だった（rf#722 修正 1）。CLAP `destroy` は instance 終端であり、中立機械の `Destroyed` 終端維持は契約と整合 — **終端化欠陥は CLAP には存在しない** |

### 2. 受理 phase — 2 欠陥（D1/D2）+ lifecycle 遷移は欠陥なし

lifecycle 遷移自体は契約と整合（下表）。欠陥は遷移「前」の
config/metadata 受理層にあった:

| callback | CLAP 契約 | adapter 遷移 | 判定 |
| --- | --- | --- | --- |
| `activate` | `[main-thread & !active]` | `Activate`（`Initialized` のみ、`:365-401`） | ✅ 遷移は正しい。**ただし config 受理で D2**（下記） |
| `start_processing` | `[audio-thread & active & !processing]` | `StartProcessing`（`Activated` のみ、`:493`） | ✅ |
| `stop_processing` | `[audio-thread & active & processing]` | `StopProcessing`（`Processing` のみ、`:501`） | ✅ error channel なし・記録のみ |
| `process` | `[audio-thread & active & processing]` | `Process` self-loop ゲート（`:410`）+ frames/ports 検証 | ✅ `frames_count > max_frames` 拒否（test `:782`）、pointer 検証 fail-closed |
| `reset` | `[audio-thread & active]` | **遷移なし** — mirror から gain 再同期（`:485`） | ✅ param 値不変の契約と一致 |
| `deactivate` | `[main-thread & active]`（post-stop） | `Deactivate`（`Activated` のみ、`:477`） | ✅ `Processing` からは host 順序違反として拒否 |

#### D1: params `module: b"/"` — 欠陥・修正済み

- **症状**: clap-validator の param 系 9 case（`param-set-events`・
  `param-set-no-cookies`・`param-set-wrong-namespace`・
  `param-fuzz-basic`・`param-fuzz-bounds`・`param-fuzz-sample-accurate`・
  `param-fuzz-modulation`・`param-conversions`・`param-default-values`）
  が "The module name for parameter 'Gain' (stable ID 0) starts with
  a leading slash: '/'" で failed。
- **根本原因**: `plugin.rs` `get_info` が `module: b"/"` を報告。
  CLAP params extension の `module` は "/" を separator とする
  module path であり、先頭 `/` は validator が拒否する。fixture は
  全 plugin で `module: b""`（空 = root）。
- **修正**: wire metadata を `gain_param_info()`（`plugin.rs`:262）に
  集約し `module: b""` に修正。`get_info` は同 fn を `set` する。
  `ParamInfoWriter` が unsafe 構築のため直接 test できない制約を、
  metadata 生成の分離で解消 — unit test
  `param_info_reports_validator_clean_metadata` が module 空・
  name/flags/min/max/default を pin する。
- **契約根拠**: CLAP `clap_param_info.module`（"The module path of
  the parameter"）+ clap-validator の module 名検証 + fixture の
  `module: b""` 実績。

#### D2: fractional sample rate 拒否 — 欠陥・修正済み

- **症状**: `process-varying-sample-rates` が "Error while processing
  with 1234.57hz sample rate — 'clap_plugin::activate()' returned
  false" で failed。
- **根本原因**: `adapt.rs::to_process_config` が
  `rate.fract() != 0.0` を defined reject に含め、1234.57 Hz の
  activate を `ActivateFailed` にしていた。CLAP `activate` の
  `sample_rate` は f64 で整数制約は仕様にない。中立 `SampleRate`
  が整数 Hz なのは core の型制約であり、f64→u32 の写像方法
  （reject vs round）は adapter 層の契約判断 — fixture は
  `rate.round()` で受理する実績ある選択をしていた。
- **修正**: fract 条件を除去し `rate.round() as u32` に変更
  （fixture `block.rs::to_process_config` と同一写像）。
  `rate > u32::MAX` 等の残り reject 条件は不変（round 後も u32
  範囲内であることは fixture と同じ順序で保証）。doc comment と
  test を新契約に更新。
- **影響評価**: gain の DSP は sample rate 非依存。`ProcessConfig.
  sample_rate` は transport/timebase 換算（`adapt.rs`:853）に使うが
  ±0.5 Hz の丸め誤差は無視できる。fixture と同一の写像であり
  新規の意味論を導入しない。
- **契約根拠**: CLAP `activate` の sample_rate は f64・範囲制約の
  規定なし + fixture の round 受理実績 + clap-validator
  `process-varying-sample-rates` の受理期待。

### 3. bus/port info 自己整合 — 欠陥なし

| 観点 | 判定 | 根拠 |
| --- | --- | --- |
| audio-ports 宣言 | ✅ | `declare_extensions`（`plugin.rs`:189）は `PluginAudioPorts` + `PluginParams` のみ。stereo 1 ペア（in/out 各 `channel_count=2`・`IS_MAIN` index 0 のみ・`in_place_pair` 対応 id）— `process` の `MAX_CHANNELS=2`・port pair view と一致 |
| f32/f64 整合 | ✅ | `SUPPORTS_64BITS` 非宣言 ↔ f64 pair 拒否で宣言と受理が一致 |
| in-place | ✅ | `in_place_pair` 宣言どおり mutable output-only view（alias した参照を作らない） |
| latency 報告 | ✅ 非宣言で正しい | zero-latency gain・latency extension 非宣言は妥当 |
| tail 報告 | ✅ | tail extension 非宣言・gain は tail 非生成。`map_process_result`（`adapt.rs`:939-945）は `Continue+tail>0`→`Tail`・`Sleep`→`Sleep`・`Error`→`Err`（test `:1907-1920`）。CLAP `Tail` は長さを持たないが tail 未宣言 plugin が Tail を返す経路は gain processor（`CONTINUE`/`SLEEP` のみ、`gain.rs`:278-282）に存在しない |
| params 宣言 | ✅（D1 修正後） | `IS_AUTOMATABLE \| REQUIRES_PROCESS`・process 内 sample-accurate event 適用（test `:802`）と flush 経路（`:829`）の両方実装済み |

### host.rs 対称性（read-only 確認）

`host` feature の host 側 bridge（`host.rs`:1104-1180）は neutral
`ProcessStatus` ⇄ CLAP `ProcessStatus`・transport・latency の写像を
担い、plugin 側 `map_process_result` と対称。非対称な欠陥は確認
されなかった（GUI bridge は rf#389 領域・変更しない）。
`host.rs`:1048 の rate 変換は neutral→CLAP 方向（整数→f64）で
lossless のため D2 の影響なし。

## TDD red-green 記録

- **red（外部）**: `evidence/rf732/abi-validator-red/run.json` —
  production leg `Failed`（10 failed: param 系 9 + varying-sample-rates
  1）、verdict Failed (exit 1)。
- **red（unit）**: `to_process_config_rounds_fractional_sample_rate`
  ・`param_info_reports_validator_clean_metadata` を追加 —
  修正前コードでは失敗する契約（fractional reject・module `/`）。
  ※ plugin.rs の `module` 修正は test 追加と同時に適用したため
  unit red は validator 出力が一次証拠。
- **green（unit）**: `cargo test -p ruviform-plugin-clap --lib` —
  50 tests pass（新規 2 test 含む）。
- **green（外部）**: `evidence/rf732/abi-validator/run.json` —
  production leg `Passed`（26 passed, 17 skipped, 1 warning,
  0 failed）、verdict **Passed (exit 0)**（fixture 5 legs +
  production + informational + VST3 235/235）。

## 外部 validator 実走

clap-validator 0.4.1（provisioned: `%LOCALAPPDATA%/ruviform/
plugin-abi-validator/clap-validator.exe`）+ VST3 SDK validator
（`vst3sdk-build/bin/Release/validator.exe`）を使用。

実行コマンド（green）:

```
node ci/plugin-abi-validator/run.mjs \
  --clap-validator <clap-validator.exe> \
  --vst3-validator <validator.exe> \
  --clap-production \
  --evidence-dir evidence/rf732/abi-validator
```

結果: verdict **Passed (exit 0)** — `run.json` 参照。
leg 別: test_gain 26p/17s/1w、test_fault 20p/17s、test_stateful
31p/12s/1w、test_instrument 30p/14s、test_editor 17p/26s/1w、
**production_gain 26p/17s/1w/0f**、VST3 fixture 235p/0f。
informational fault-unfiltered leg は Unsupported（意図的 crash
surface・verdict 非影響）。

既定 lane: `mise run test-plugin-abi-validator`（引数なし）は
validator 未指定のため fail-closed で NotRun（opt-in 設計どおり・
実走は上記明示パス指定で実施）。

## 残課題・範囲外メモ

- clap-validator 0.4.1 の param-fuzz panic（exit 0xc0000409）は
  rf#726 が upstream 判断を担当 — 本 slice では遭遇せず
  （production leg は param-fuzz 系を含め全 pass）。
- denormal 処理の perf warning（"3.99x longer to process
  denormals"）は fixture と同種の観察事項 — gain=0.0 で denormal が
  発生し得るが fail ではなく、DSP tuning は本 slice 範囲外。
- `min_frames` 未満の frames 寛容受理は観察事項（host 違反入力・
  安全性影響なし）。
- docs index（`docs/**/README.md`）は未同期 — Coordinator が統合時に
  再生成。
