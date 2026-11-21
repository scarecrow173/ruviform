# RF868 dirty/mark_dirty 通知経路 — コード照合記録

- Issue: https://github.com/scarecrow173/ruviform/issues/868（調査）
- Branch: `scarecrow173/rf868-host-dirty-inventory`（worktree
  `G:\orca\workspaces\ruviform\rf868-host-dirty-inventory`）
- Base: main `c58fc946` 起点の worktree
- 本文書は `docs/discovery/issue-868-host-dirty-notification.md` の
  一次情報対応表。照合は worktree 上の実ファイルに対して実施
  （行番号は照合時点）。

## CLAP — `clap_host_state`/`mark_dirty`

| 観点 | 実コード | 判定 |
| --- | --- | --- |
| host extension 登録面 | `crates/ruviform-plugin-clap/src/host.rs:337-342` — `BridgeHandlers::declare_extensions` は空実装（host extension 0 件登録） | `HostState` 未登録 |
| MainThread handler 型 | `crates/ruviform-plugin-clap/src/host.rs:333-335` — `type MainThread<'a> = ()` | `HostStateImpl` 実装なし（clack `ExtensionImplementation<HostState>` の境界条件 `MainThread<'a>: HostStateImpl` を満たせない） |
| 非登録の明示記録 | `crates/ruviform-plugin-clap/src/host.rs:112-113` — 「The host-side `HostState`/`mark_dirty` notification path is not registered」 | 設計上の既知境界 |
| clap_host 本体の要求動詞 | `crates/ruviform-plugin-clap/src/host.rs:315-319` — `BridgeShared::{request_restart,request_process,request_callback}` no-op | extension 経路とは別物（dirty と無関係） |
| state 動詞の実装位置 | `crates/ruviform-plugin-clap/src/host.rs:1483,1509` — `handle.get_extension::<PluginState>()` 経由の save/load | plugin→host 方向とは逆向き（host→plugin）の実装済み経路 |
| plugin 側 `mark_dirty` 発火 | `crates/ruviform-plugin-test-clap/src/` 内に `mark_dirty`/`HostState` の呼出なし（grep 0 件） | 発火実体なし |
| plugin 側 state 実装 | `crates/ruviform-plugin-test-clap/src/stateful.rs:535-580` — `PluginStateImpl::{save,load}`（`clap.ext.state`） | save/load は実装済み |
| 同 seam の未達 callback | `crates/ruviform-plugin-test-clap/src/stateful.rs:596-601` — `HostParams::rescan(VALUES)` 発行、`:631-644` — `HostPresetLoad::loaded/on_error` 発行 | host extension 未登録のため `get_extension` が `None` で不達 |
| binding の host 側面 | `clack-extensions 0.2.0` `src/state/host.rs:67-92` — `HostStateImpl::mark_dirty`、`clap_host_state{mark_dirty}` vtable、`src/state/plugin.rs:11-15` — plugin 側 `mark_dirty(&HostMainThreadHandle)` 呼出は extension 存在条件 | plugin は未登録時に呼ばない（沈黙は plugin 側分岐） |

## VST3 — `IComponentHandler`/`IComponentHandler2`

| 観点 | 実コード | 判定 |
| --- | --- | --- |
| handler 注入 | `crates/ruviform-plugin-vst3/src/host.rs:2885-2899` — `ComWrapper::new(BridgeComponentHandler)` を `controller.setComponentHandler` で注入（distributed/single 共通） | 呼出は到達する |
| `beginEdit`/`performEdit`/`endEdit` | `crates/ruviform-plugin-vst3/src/host.rs:762-776` — 各 `kResultOk` を返す no-op | ack-and-drop（値・通知は転送されない） |
| `restartComponent` | `crates/ruviform-plugin-vst3/src/host.rs:778-785` — `kNotImplemented`（`kResultOk` は flag 要求の再送を偽装するため拒否を選択） | 正直な拒否 |
| `IComponentHandler2` | `crates/ruviform-plugin-vst3/src/host.rs:758-760` — `type Interfaces = (IComponentHandler,)` | `setDirty`/`requestOpenEditor`/group edit は QI 不可達 |
| process 時の param 出力 | `crates/ruviform-plugin-vst3/src/host.rs:158-162`（module doc）— `outputParameterChanges`/`outputEvents` 常に null | process 経由の逆方向口なし |
| component↔controller 接続 | `crates/ruviform-plugin-vst3/src/host.rs:2905-2921` — distributed plugin の `IConnectionPoint` 双方向 connect（drop で disconnect、:2224-2230） | host 通知とは別の plugin 内部経路（接続済み） |
| fixture の handler 保持 | `crates/ruviform-plugin-test-vst3/src/controller.rs:365,507`、`preset.rs:664` — handler 非保持の `kResultOk`。`single.rs:480-486` — once-contract の呼出 count のみ | plugin 側 `performEdit`/`setDirty` 発火実体なし |
| fixture の handler 呼出 | `crates/ruviform-plugin-test-vst3/src/editor_view.rs` — handler 呼出なし | editor からの編集通知なし |
| binding の拡張面 | `vst3 0.3.0` bindings `IComponentHandler2Vtbl` — `setDirty`/`requestOpenEditor`/`startGroupEdit`/`finishGroupEdit`、`RestartFlags_`（`kReloadComponent=1`/`kParamValuesChanged=4`/`kLatencyChanged=8` 等） | 実装対象の語彙は bindings に存在 |

## 中立層・wire・host 面

| 観点 | 実コード | 判定 |
| --- | --- | --- |
| state 動詞 | `crates/ruviform-plugin-core/src/plugin.rs:175-189` — `Plugin::{save_state,restore_state}` default `Err(Unsupported)` | host 起点 pull のみ、通知 surface なし |
| plugin→host report | `crates/ruviform-plugin-core/src/processor.rs:526-528` — `PluginProcessor::drain_output_reports`（canonical `Event` の pull drain） | 唯一の逆方向 surface。dirty 語彙なし |
| CLAP report 実装 | `crates/ruviform-plugin-clap/src/host.rs:66-81`（module doc）・`:744-755` — output events → `pending_reports`、`:1389` — drain | Issue #842 で配線済み（pull） |
| wire 方向 | `crates/ruviform-plugin-core/src/wire.rs` — `Command`/`Reply` は request/reply 限定。`DrainOutputReports`（:869-881）が唯一の plugin→host pull verb | child→host 非同期 direction なし |
| 子側 backlog | `crates/ruviform-plugin-export/src/lib.rs:427-430` — `report_backlog`、`:741-759` — `DrainOutputReports` dispatch（`WIRE_MAX_EVENTS` 単位で chunk） | pull 型 backlog の実装先例 |
| host 照会面 | `crates/ruviform-plugin-host/src/composite.rs:985-1000` — `PluginHost::drain_output_reports`（`ReportDrain::{Drained,Unavailable}`）、`host.rs:961-967,1054-1067` — `save_state`/`restore_state` 実行、`command.rs:81-91` — `SaveState`/`RestoreState` 動詞 | dirty 照会・通知 vocabulary なし |
| supervisor checkpoint | `crates/ruviform-plugin-host/src/supervisor.rs:628-659` — `set_checkpoint` push-only（`poll` は `SaveState` を実行しない）、`:1004-1038` — 復帰時 `RestoreState` replay | dirty 起動の再 checkpoint 経路なし |

## 先行記録との整合

| 先行記録 | 内容 | 本棚卸しとの関係 |
| --- | --- | --- |
| `docs/specs/issue-630-adapter-state-bridge.md:118-119` | host `HostState`/`mark_dirty` 非登録（KISS） | 現行実装と一致 |
| `docs/designs/issue-630-adapter-state-bridge.md:152-156,193-196` | `HostState` 登録却下（`MainThread=()` への `HostStateImpl` 実装増が理由） | 現行 `MainThread<'a> = ()` と一致、再評価の根拠を本棚卸しが補強 |
| `docs/discovery/issue-186-param-control-events-inventory.md:183` | plugin→host 報告「経路なし」 | 当時点の記録。Issue #842 で CLAP output events は `drain_output_reports` に配線済み（pull 型の前例が成立）。gesture/rescan・VST3 側は依然未配線 |
| `docs/specs/issue-186-param-control-events.md:132` | plugin→host param 報告は「Reported 観測の配線が必要になった時」に別 Issue 分割 | dirty 通知（#891）と param 値報告の全面配線は別語彙として切り分け |

## 結論サマリ

| 経路 | 現状 | plugin 発発火 | 到達可否 |
| --- | --- | --- | --- |
| CLAP `mark_dirty` | host extension 未登録 | fixture 発火なし | **不可達**（`get_extension` が `None`、plugin 側で分岐沈黙） |
| VST3 `performEdit`/`beginEdit`/`endEdit` | handler 注入済み・ack のみ | fixture 発火なし | **到達・ack-and-drop** |
| VST3 `restartComponent` | `kNotImplemented` | fixture 発火なし | 到達・正直な拒否 |
| VST3 `IComponentHandler2::setDirty` | `Interfaces` に面なし | fixture 発火なし | **不可達**（QI 失敗） |
| 中立/wire/host 照会面 | pull のみ（`save_state`/`restore_state`/`drain_output_reports`/`DrainOutputReports`） | — | dirty 語彙なし |
| 実装要否 | **要** — pull 型 report seam に載せる最小語彙（#891） | — | — |
