---
id: "DSC-rf868hostDirtyNotice"
type: "discovery"
status: "final"
title: "Issue #868 production host dirty/mark_dirty 通知経路の棚卸しと実装要否の判断"
created: "2026-11-22"
updated: "2026-11-22"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/868"
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-adapter-state-bridge.md"
    - "docs/discovery/issue-186-param-control-events-inventory.md"
  related:
    - "https://github.com/scarecrow173/ruviform/issues/891"
    - "https://github.com/scarecrow173/ruviform/issues/873"
    - "evidence/rf868/dirty-notification-inventory.md"
---

# Issue #868 production host dirty/mark_dirty 通知経路の棚卸し

`docs/specs/issue-630-adapter-state-bridge.md`（118-171 行）が「host 側
`HostState`/`mark_dirty` は登録しない（KISS — dirty 通知は後続 slice の
判断材料）」と記録し残した論点の棚卸し記録。対象は plugin→host 方向の
dirty 通知経路 2 系統 — CLAP `clap_host_state`（`HostState`/`mark_dirty`）
と VST3 `IComponentHandler`/`IComponentHandler2`（`performEdit`/
`restartComponent`/`setDirty`）— と、adapter から consumer までの
中立層・wire 経路。コード照合は本 worktree 上の実ファイルに対して実施し、
一次情報の対応表は `evidence/rf868/dirty-notification-inventory.md` に置く。

## 棚卸しの対象と一次情報

- CLAP host bridge: `crates/ruviform-plugin-clap/src/host.rs`
  （`host` feature・clack-host 0.2.0）。plugin 側は
  `crates/ruviform-plugin-test-clap/src/stateful.rs`（`test_stateful`）。
- VST3 host bridge: `crates/ruviform-plugin-vst3/src/host.rs`
  （`host` feature・vst3 crate 0.3.0 bindings）。plugin 側は
  `crates/ruviform-plugin-test-vst3/src/` 各ファイル。
- 中立層: `crates/ruviform-plugin-core/src/plugin.rs`（`Plugin` 動詞）、
  `processor.rs`（`drain_output_reports`）、`wire.rs`（`Command`/`Reply`）。
- host 面: `crates/ruviform-plugin-host/src/{command,composite,host,
  supervisor}.rs`、子側 dispatch は `crates/ruviform-plugin-export/src/lib.rs`。

## CLAP: `clap_host_state` の経路は宣言面から存在しない

CLAP の dirty 通知は host extension `clap_host_state` の `mark_dirty()`
コールバックである。plugin は `clap_host` 経由で `HostState` を照会し、
存在すれば `mark_dirty` を呼んで「状態が変わったので再 save を」という
合図を送る。clack-extensions 0.2.0 では `HostState`（host 側）と
`HostState::mark_dirty(&HostMainThreadHandle)`（plugin 側ヘルパ）が対応する。

production bridge はこの extension を宣言していない。
`BridgeHandlers::declare_extensions`（host.rs:337-342）は host extension を
1 つも登録しない空実装で、`MainThread<'a> = ()` が置かれている。
clack の `HostState` 登録条件は `HostHandlers::MainThread<'a>: HostStateImpl`
を満たすことなので、登録には `()` を置き換える実 handler 型が別途必要になる。
`BridgeShared` の `request_restart`/`request_process`/`request_callback`
（host.rs:315-319）は `clap_host` 本体の要求動詞であり、extension 経路とは
別物 — dirty 通知とは無関係である。

plugin 側の発火実体も存在しない。`test_stateful` は `PluginStateImpl`
（`clap.ext.state` の save/load、stateful.rs:535-580）を実装するが
`mark_dirty` は呼ばない。同じ host callback seam の
`HostParams::rescan(VALUES)`（stateful.rs:596-601）と
`HostPresetLoad::loaded`/`on_error`（stateful.rs:631-644）は発行コードが
あるが、bridge 側に extension がないため `get_extension` が `None` を返し
到達しない。未配線の構造はこの点で CLAP 側全体に共通する。

経路の評価: `mark_dirty` は **extension 未登録による不可達**。plugin 側は
`Some` 条件でコールするため、通知の喪失は plugin の分岐で明示的に起きる
（bridge へ届いて破棄される形ではない）。docs/designs の却下記録
（issue-630-adapter-state-bridge.md:152-156・193-196）と整合する正直な
境界である。

## VST3: `performEdit` は到達するが ack 後に捨てられる

VST3 で plugin が host へ通知する面は 3 段に分かれる。

1. `IComponentHandler::{beginEdit,performEdit,endEdit}` — パラメータ編集の
   通知。DAW 系 host は `performEdit` を document-dirty の引き金と
   automation 記録の入力として扱う。
2. `IComponentHandler::restartComponent(flags)` — `kReloadComponent`・
   `kParamValuesChanged`・`kLatencyChanged` 等の flag 別再送要求。
3. `IComponentHandler2` — handler への QI で得る拡張面で、`setDirty(bool)`
   （非パラメータ変化の明示 dirty フラグ）、`requestOpenEditor`、
   `startGroupEdit`/`finishGroupEdit` を持つ。

production bridge の `BridgeComponentHandler`（host.rs:756-786）は init 時に
`controller.setComponentHandler` で確実に注入される（host.rs:2885-2899）。
したがって `beginEdit`/`performEdit`/`endEdit` への呼出は到達するが、
実装は `kResultOk` を返すだけで値も通知も転送しない
acknowledge-only である（host.rs:762-776）。plugin から見れば
「host が編集を受理した」に見えるが、値は捨てられる。`restartComponent`
は `kNotImplemented` を返す正直な拒否（host.rs:778-785）— flag 要求の
再送を実行できないのに `kResultOk` を返す方が不誠実になるという判断が
コメントに記録されている。`Interfaces = (IComponentHandler,)` しか
宣言しないため `IComponentHandler2` は QI で不可達であり、`setDirty` の
受入口自体がない。`ProcessData.outputParameterChanges`/`outputEvents` も
常に null で、process 時の param 出力に別の戻り口はない（module doc、
host.rs:158-162）。

fixture 側の発火実体も存在しない。全 `setComponentHandler` 実装
（`controller.rs` の `ParamController`:365 と `TestEditorController`:507、
`preset.rs`:664 経由の委譲、`single.rs`:480）は handler を保持せず、
`single.rs` は once-contract 監査のため呼出回数を数えるだけである。
`test_editor` の `IPlugView` も handler を呼ばない。

経路の評価: `performEdit` 系は **到達するが ack-and-drop**、
`setDirty`/`restartComponent` は **面不在または正直な拒否**。CLAP より
一段不誠実さの度合いが強いのは、到達した `performEdit` が `kResultOk`
を返しながら捨てられる点である — plugin GUI 編集が無言で失われる
現行境界である。

## 中立層・wire・host 面には dirty の語彙がない

adapter から先の経路は pull 型しか存在しない。

- `Plugin::save_state`/`restore_state`（plugin.rs:175-189）は host 起点の
  pull verb で、plugin 発の通知 surface は trait に存在しない。
- `PluginProcessor::drain_output_reports`（processor.rs:526-528）は
  plugin→host の唯一の逆方向 surface で、canonical `Event`（`ParamValue`・
  `Midi`）を pull drain する。CLAP bridge は plugin の output events を
  `BoundedOutputBuffer`→`pending_reports` へ集約しここへ繋ぐ
  （host.rs:66-81・Issue #842）。VST3 側は emit するものがなく到達点も
  空である。dirty フラグ相当の語彙は report item にもない。
- wire `Command`/`Reply`（wire.rs）は request/reply のみで child→host の
  非同期通知 direction を持たない。plugin→host のデータは wire v4 の
  `DrainOutputReports`（wire.rs:869-881、子側 dispatch は
  export/lib.rs:741-759 の `report_backlog`）が運ぶ pull verb だけである。
- `Supervisor::set_checkpoint`（supervisor.rs:628-659）は caller が
  `SaveState` blob を push 登録する方式で、`poll` は `SaveState` を
  実行しない。crash 復帰時に replay する checkpoint の鮮度を
  「dirty を受けたら再 save」で保つ起動点がない。
- `HostCommand`/`HostReply`（command.rs）にも dirty 照会・通知の
  vocabulary はない。

既存の pull 型 report 経路は plugin→host の bounded backlog を
L0（in-process）と L1（wire 越し）の双方で確立済みであり、dirty のような
「後で読んでほしい合図」はこの seam に載る形が既成である。

## 同じ seam に残る兄弟の未配線経路

dirty 通知と同じ「plugin→host の host-side callback」を共有する未配線は
複数ある。棚卸しの完全性のために併記する。

- CLAP `clap_host_params`（`HostParams::rescan`/`clear`）— `test_stateful`
  が state/preset load 後に `rescan(VALUES)` を発行済みで不達。
- CLAP `clap_host_preset_load`（`HostPresetLoad::loaded`/`on_error`）—
  同上、発行コードはあるが不達。
- VST3 `IComponentHandler::restartComponent` — `kParamValuesChanged` 等の
  再送要求で、CLAP `rescan` と同種の「値が変わった」通知族。現行は
  `kNotImplemented` の正直な拒否。
- CLAP `clap_host_gui`（`request_resize`/`request_show`/`request_hide`/
  `closed`）と VST3 `IPlugFrame::resizeView`（host.rs:734-741 の
  `kResultFalse` defined refusal）— Issue #873 のスコープ。
- CLAP `clap_host_latency`/`clap_host_tail`/`clap_host_note_name`/
  `clap_host_track_info`/`clap_host_audio_ports`（rescan 系）等 —
  すべて `declare_extensions` 未登録で同じ不可達。

## dirty 通知が host 側で必要になるユースケース

1. **plugin GUI 経由の編集 → document dirty**。native editor attach
   （L0 dock、#389 系）と wire v6 floating GUI（#872/#882/#883）により
   実 plugin の editor が開ける。そこでの編集は host の verb を経由しない
   plugin 発の状態変化であり、VST3 では `performEdit`/`setDirty`、CLAP
   では param output event または `mark_dirty` に出る。consumer の
   「プロジェクト未保存」表示・preset dirty 表示がこの通知なしには
   plugin GUI 起点で立たない。L1 では編集が子プロセス内に閉じるため、
   wire を越えた通知なしには host 側で不可視である。
2. **autosave / crash 復帰 checkpoint の鮮度**。`set_checkpoint` は
   push-only で、「状態が変わったら再 save して checkpoint を更新」
   という運用の起動点がない。現状では consumer が周期 `SaveState`
   するか、自身が起こした変化だけを追跡するほかない。
3. **param 値の再読（re-fetch）要求**。VST3 `restartComponent
   (kParamValuesChanged)`・CLAP `rescan(VALUES)` は「値が変わったので
   読み直してほしい」という dirty と同族の通知で、plugin 内部での
   program/preset 切替を host 側が拾う経路になる。
4. **gesture 境界（begin/endEdit）**。automation 記録の単位を与える
   通知族で、handler 面を共有するが dirty 自体とは別の語彙である。

1 と 2 は foundation が提供すべき経路としての実需であり、3 は同じ seam の
追加語彙、4 は隣接するが独立した論点である。

## 実装要否の判断

**判断: 実装は必要 — pull 型の dirty 通知を最小限の語彙で実装する。**

根拠は 3 点ある。

1. **plugin 発の状態変化が観測不能なまま実害を持つ段階に入った**。
   却下決定時（adapter-state-bridge design）は GUI attach がなく
   plugin 発の変化は実質存在しなかった。現在は実 plugin の editor が
   L0/L1 双方で開け、GUI 編集は host の verb を一切経由しない。
   VST3 の `performEdit` は `kResultOk` を返しつつ捨てる ack-and-drop
   であり、plugin は「host に届いた」と認識する — 喪失が plugin 側で
   明示的に分岐される CLAP 未登録より一段不誠実である。
2. **transport の新規性がない**。wire は request/reply 限定を維持し、
   既存の pull 型 report 経路（`drain_output_reports`/
   `DrainOutputReports` family）に dirty 通知を載せれば、L0/L1 双方の
   既成経路に合流する。非同期通知 direction の新設という最大の
   コスト項目が不要になる。
3. **foundation としての責務**。document-dirty/autosave と crash 復帰
   checkpoint の鮮度管理は DAW consumer（Synvelis）の前提契約であり、
   consumer 側が自身の verb だけで追える範囲（host 起点の変化）は
   既に自足している。欠けているのは plugin 起点の変化の通知だけで、
   それは foundation の経路責務である。

同時に、scope を絞る根拠も記録する。「param 値報告の全面配線」
（`outputParameterChanges`・gesture event・params rescan の中立配線）は
#186 の Deferred Design Concerns に残る別の論点であり、dirty の最小
slice に抱き合わせない。dirty は「再 save が必要」という合図であり、
値の内容を運ぶ param 報告とは語彙が違う。

後続 Issue: **#891** `[実装/plugin] plugin→host state dirty 通知経路`
を起票済み（本棚卸しの判断結果の実装 slice）。

## 設計方針の記録（#891 の出発点）

- **transport**: pull 型 report 経路に載せる。child→host の非同期
  direction を新設しない — `drain_output_reports`/`DrainOutputReports`
  family と同じ「producer が bounded backlog に積み、host が drain する」
  形を維持する。
- **所有層**: ABI callback は adapter bridge が所有する（CLAP
  `HostStateImpl::mark_dirty`、VST3 `IComponentHandler2::setDirty`・
  `performEdit`）。neutral 語彙は plugin-core、consumer 照会面は
  plugin-host/composite、wire 語彙は plugin-core/wire + export runner
  dispatch — `pending_reports` と同じ責務分割である。
- **design で確定する残論点**: dirty の中立語彙（専用 flag/verb か
  report item の拡張か）、`performEdit` の id+value を param report へ
  配線するか dirty 扱いに留めるか、`restartComponent` の flag 別応答、
  gesture（`beginEdit`/`endEdit`）の扱い、`performEdit` の ack-and-drop
  境界の解消方法。
- **fixture**: dirty 発火手段が両形式とも存在しないため、test plugin
  suite への発行手段追加が acceptance の一部になる。

## 未確定・Deferred/NotRun の正直な境界

- 実 plugin（市販 VST3/CLAP）の `mark_dirty`/`performEdit` 実走挙動は
  未観測 — 仕様上の経路と fixture 照合までが本棚卸しの範囲。実走確認は
  #891 実装時の real lane で行う。
- `IComponentHandler2::setDirty` に加え `requestOpenEditor`・group edit
  系が handler2 に同居するため、登録時は兄弟メソッドの応答方針も
  併せて確定する必要がある。
- CLAP `mark_dirty` は `[main-thread]` 契約で、発火が plugin の main
  thread からのみ。host 側受取・反映のスレッド整合は #891 の
  design で扱う。
