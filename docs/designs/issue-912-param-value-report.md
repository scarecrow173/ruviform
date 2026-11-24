# Issue #912 — plugin→host param 値報告の全面配線 設計記録

status: implemented
issue: ruviform#912 (param 値報告経路の配線)
precedent: Issue #891 `docs/designs/issue-891-dirty-notification.md`（本記録は D1/D2/D4 の写像を継承する）

## 0. スコープ確定（現状訂正を前提に）

Issue 本文の「VST3 `outputParameterChanges`/`outputEvents` は常時 null」は stale。
process 経路（output param queue→`EventKind::ParamValue`、outputEvents→`EventKind::Midi`）は
#253/#842 で `drain_output_reports` に配線済み。**本 Issue の未配線は main-thread/lifecycle 面**:

| 発生源 | ABI | 現状 | 本 Issue の扱い |
| --- | --- | --- | --- |
| `performEdit(id, value)` | VST3 | `DirtyNotice::ParamEdited` のみ | 値を param report 面に載せる（dirty 面は維持＝二面 fan-out） |
| `beginEdit`/`endEdit` | VST3 | `kResultOk` 無通知 | gesture edge として通知 |
| `restartComponent(flags)` | VST3 | `kParamValuesChanged` exact のみ受理→`StateDirty`、他 flag は `kNotImplemented` | 全呼び出しを通知として届ける。応答の honest 境界は §6 |
| `clap_host_params.rescan`/`clear`/`request_flush` | CLAP | 拡張未登録（到達しない） | `HostParams` 登録＋通知写像 |
| `clap_host_preset_load.loaded`/`on_error` | CLAP | 拡張未登録 | `HostPresetLoad` 登録＋通知写像（`preset-discovery` feature 追加） |
| `PARAM_GESTURE_BEGIN/END` output event | CLAP | `translate_output_event` の defined skip | gesture edge として写像 |
| `PARAM_VALUE`/`PARAM_MOD` output event | CLAP | `ParamValue` は #842 で `Event` 化済み、`ParamMod` は skip | **変更しない**（§5 二面配送の境界） |
| `DirtyNoticeQueue::dropped_edits` | both | accessor のみ、wire/照会面に露出なし（#891 deferred） | `DroppedEdits` variant で drain 先頭に露出（§4） |

## 1. 配送 seam の設計判断（受入条件 1）

### 選択: 専用中立 enum `ParamNotice` + 専用 pull verb（案 C）

#891 D1 が dirty 通知で実証した三要素をそのまま適用する:
`boundary.rs` の `#[non_exhaustive]` 中立 enum + `ParamNoticeQueue`、
`plugin.rs` の `Plugin::drain_param_notices`（default 空実装）+ `ErasedPlugin` 中継、
wire の `Command::DrainParamNotices`（tag 31）/ `Reply::ParamNotices`（tag 10）+ `WIRE_VERSION` bump。

### 却下案

- **案 A: `Event`/`EventKind`/`Reply::Reports` 拡張 — 却下。**
  `Event` は `frame_offset` を持つ sample-accurate 契約。`performEdit`/`beginEdit`/`rescan`/
  `restartComponent`/`loaded` はすべて main-thread 面の発火で frame offset を持たない。
  offset 0 で流せば「ブロック先頭の sample-accurate」と「offset なし」を consumer が区別できず、
  #891 D1 が `Reply::Reports` payload 拡張を棄却した根拠（frame_offset の意味を嘘にする）と同型。
  さらに gesture/rescan/preset-load は `EventKind` に載らず、新 kind tag = payload 変更で
  結局 bump 必須。process 由来と main-thread 由来が同一面に混在し読み手の前提も壊れる。
- **案 B: `DirtyNotice` 拡張 — 却下。**
  `DirtyNoticeQueue` の格納契約（flag latch coalesce + edits ≤64 FIFO）は dirty bookkeeping 用で、
  automation 記録が要求する「発生順を保存する値・gesture ストリーム」の意味論と衝突する。
  gesture edge は #891 D4 で「dirty consumer には noise」と意図的に落とされた経緯があり、
  同じ面に載せると dirty consumer の意味論を変えてしまう。variant 追加も結局 bump 必須。

### wire bump 確定

本 scope（値 + gesture + rescan + clear + flush + preset-load + dropped 露出）を実現する全案で
新 verb/reply/payload が必須 → `WIRE_VERSION` **8 → 9**。
`Command::DrainParamNotices` = tag **31**、`Reply::ParamNotices` = tag **10**、
`WIRE_MAX_PARAM_NOTICES = 1024`（`WIRE_MAX_DIRTY_NOTICES` と同じ chunked-pull 契約）。
`DirtyNotice::DroppedEdits`（variant tag 3）は同一 bump に同乗する additive 変更。

## 2. 中立語彙 `ParamNotice`（`boundary.rs`）

```rust
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]   // Copy 不可 — preset 系が String を保持
pub enum ParamNotice {
    /// plugin 起因の値報告 — VST3 `performEdit(id, normalized)`。
    /// 非有限値は adapter 境界で skip（中立面に有限域を持ち込まない既存法則）。
    ValueChanged { param_id: ParamId, value: ParamValue },
    /// gesture 開始 — VST3 `beginEdit`、CLAP `PARAM_GESTURE_BEGIN` output event。
    /// `frame_offset`: process 経路到達（CLAP）のみ `Some`、main-thread 到達
    /// （VST3）は `None`。「offset なし」を `0` に潰さない正直な区別。
    GestureBegin { param_id: ParamId, frame_offset: Option<u32> },
    /// gesture 終了 — 同上。
    GestureEnd { param_id: ParamId, frame_offset: Option<u32> },
    /// 値・メタ・構成の再読/再起動要求 — CLAP `clap_host_params.rescan`、
    /// VST3 `restartComponent(flags)`（全呼び出し。届ける≠果たす、§6）。
    RescanRequested { aspects: RescanAspects },
    /// host 側の param 参照（automation/modulation）クリア要求 — CLAP `clear`。
    ClearReferences { param_id: ParamId, aspects: ClearAspects },
    /// flush 要求 — CLAP `request_flush`。bridge に flush verb は存在しないため
    /// 通知として届けるのみ（consumer は report drain で応じうる）。
    FlushRequested,
    /// preset load 成功 — CLAP `HostPresetLoad::loaded`。
    PresetLoaded { location: PresetLocation, load_key: Option<String> },
    /// preset load 失敗 — CLAP `HostPresetLoad::on_error`。
    PresetLoadFailed {
        location: PresetLocation,
        load_key: Option<String>,
        os_error: i32,
        message: Option<String>,
    },
    /// queue が容量超過で落とした notice 数（前回 drain 以降）。
    /// `ParamNoticeQueue::drain` が先頭で合成する。adapter/plugin は直接
    /// enqueue しない（queue 合成の専用 variant）。
    Dropped { count: u32 },
}
```

### 補助型

```rust
/// `RescanRequested` の aspect bitset — CLAP `clap_param_rescan_flags` と
/// VST3 `RestartFlags` の和を中立 bit に写像。u32 newtype + `bits()`/
/// `from_bits_retain`/`contains`/`is_empty`。
pub struct RescanAspects(u32);
// VALUES / INFO(CLAP INFO ↔ VST3 kParamTitlesChanged) / TEXT / ALL /
// RELOAD(kReloadComponent) / IO(kIoChanged) / LATENCY(kLatencyChanged) /
// MIDI_CC(kMidiCCAssignmentChanged) / KEYSWITCH(kKeyswitchChanged) /
// OTHER（写像先のない source bit の存在のみ保持する lossy 端子 —
// kIoTitlesChanged/kNoteExpressionChanged/kPrefetchableSupportChanged/
// kRoutingInfoChanged/kParamIDMappingChanged 等はここに畳む）

/// `ClearReferences` の aspect bitset — CLAP `clap_param_clear_flags` 写像。
pub struct ClearAspects(u32);
// AUTOMATION / MODULATION / OTHER（同上の lossy 端子）

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresetLocation { Plugin, File { path: String } }
```

`OTHER` の意味: 未写像の source bit は「何か要求された」存在のみ保持し、どの bit かは落とす
（honest lossy 端子 — consumer が解釈不能な情報を運ばない）。応答面の honest 境界とは別層
（§6: VST3 は引き続き `kNotImplemented` で拒否する）。

### `DirtyNotice` への追加（同一 bump）

```rust
DroppedEdits { count: u32 }   // wire tag 3 — 前回 drain 以降に落とした edit 数。
                              // `DirtyNoticeQueue::drain` が先頭で合成。
```

## 3. 格納契約

### `ParamNoticeQueue`（`boundary.rs`、`DirtyNoticeQueue` 隣接）

- **coalesce しない** — 各 notice は発生（occurrence）であり、automation 記録が要求する
  「発生順を保存するストリーム」の意味論を守る。`VecDeque<ParamNotice>` FIFO。
- 上限 `MAX_PARAM_NOTICES = 256`、超過は drop-oldest（新しい値ほど新鮮）。
- drop 観測面を初めから付ける（`dropped_edits` の教訓）:
  `pending_dropped`（drain で `Dropped{count}` を先頭合成して reset）+
  `dropped_notices()` accessor（cumulative `usize`）。
- `is_empty()` — backlog も pending_dropped も空。

### `DirtyNoticeQueue` への追加

- `pending_dropped_edits` カウンタを追加し、`drain` が `DroppedEdits{count}` を
  **edit backlog より先頭**に合成して reset（drop は生き残った backlog 要素より先に
  起きた — drop-oldest の性質上、先頭が時間的に正直）。
- 既存 `dropped_edits()`（cumulative accessor）はそのまま。`is_empty()` は
  pending_dropped_edits も条件に含める。
- 既存 drain 順（edits → flag edge）は不変 — drop が無ければ出力列は従来と完全一致し、
  既存 oracle（`[ParamEdited, StateDirty]`）は非破壊。

### RT 境界（重要）

- `ParamNoticeQueue` の enqueue は main-thread/非 RT 面のみ（VST3 handler、CLAP
  `HostParamsImplMainThread`/`HostPresetLoadImpl`/`HostParamsImplShared`）。
- **CLAP gesture output event は `process` 内 RT 面に到達** — `Arc<Mutex<ParamNoticeQueue>>`
  を RT で lock しない。`pending_reports` と同じ「prepare で reserve した bounded
  `Vec<ParamNotice>` への capacity 内 push」を使い、drain が両 source を合成する
  （non-RT 面のみ `ParamNotice` は非 `Copy` だが、push は move で capacity 内なら
  allocate しない — RT 安全は Copy-shape variant のみ enqueue することで担保。
  preset 系は main-thread 経路のみ到達するので RT Vec には乗らない）。

## 4. drain seam（既存土台の写像）

| 層 | dirty 面の既存物 | 本 Issue の新規物 |
| --- | --- | --- |
| trait | `Plugin::drain_dirty_notices`（default 空） | `Plugin::drain_param_notices`（default 空） |
| erase | `ErasedPlugin::drain_dirty_notices` + `Erased<P>` passthrough | 同名の sibling |
| in-process | `InProcessHost::drain_dirty_notices`（Plain 直接/Shared pinned lock） | `drain_param_notices`（同一構造） |
| composite | `PluginHost::drain_dirty_notices` → `ReportDrain` | `drain_param_notices` → `ReportDrain`（video slot は `Unavailable` を維持） |
| L1 | `L1Instance::drain_dirty_notices` chunked pull | `drain_param_notices` chunked pull |
| export | `InstanceEntry.dirty_backlog` + dispatch arm | `param_backlog` + `DrainParamNotices` arm |
| wire | tag 28 / tag 9 | tag 31 / tag 10 |

## 5. 二面配送と dedup の確定

- **VST3 `performEdit` は二面 enqueue**: `DirtyNotice::ParamEdited`（dirty 面 —
  document-dirty consumer を壊さない）+ `ParamNotice::ValueChanged`（report 面）。
  dedup（`ParamEdited` の report 面への移管）は却下: 二面は別ドメイン
  （re-save 判断 vs param 観測）であり、dirty 面から外すと #891 の consumer 契約を壊す。
- **CLAP `PARAM_VALUE` output event は `Event` 面に留める**（#842 の既存経路）。
  `ParamNotice` 面へ複写しない（二重配送は consumer の dedup 負担と「どちらが正本か」
  問題を生む）。棲み分け: **process 経路の sample-accurate 値は `Event`、
  main-thread/lifecycle 通知は `ParamNotice`**。`PARAM_MOD` は従来どおり defined skip
  （modulation は consumer 要求外）。
- VST3 `restartComponent(kParamValuesChanged)` は `RescanRequested{VALUES}` +
  従来どおり `StateDirty` latch（両ドメインに正直に届ける）。

## 6. honest 境界（#891 D4 の継承）

「通知として consumer へ届ける」ことと「bridge が実行を果たす」ことは別物:

| 発生 | 応答 | 通知 |
| --- | --- | --- |
| VST3 `restartComponent` == `kParamValuesChanged` exact | `kResultOk`（現行維持） | `RescanRequested{VALUES}` + `StateDirty` latch |
| VST3 `restartComponent` その他/複合 flag | `kNotImplemented`（現行維持 — `kResultOk` は「host が reload/bus 再構成を果たす」約束になり嘘） | `RescanRequested{mapped aspects}`（要求の到達は正直に届ける） |
| VST3 `performEdit` 非有限値 | `kResultOk`（現行維持） | `ValueChanged` skip + `StateDirty` degrade（現行 dirty 側の法則を報告面にも適用） |
| VST3 `beginEdit`/`endEdit` | `kResultOk`（現行維持） | `GestureBegin`/`GestureEnd` |
| CLAP `rescan`/`clear`/`request_flush` | void（CLAP 側に応答なし） | `RescanRequested`/`ClearReferences`/`FlushRequested` |
| CLAP `loaded`/`on_error` | void | `PresetLoaded`/`PresetLoadFailed` |

## 7. 文字列境界

`PresetLocation::File.path`・`load_key`・`message` は `String`。
wire encode は `push_str`（`u32 len + bytes`）、decode は `read_string` で
`WIRE_MAX_STRING_BYTES` 上限を強制。**adapter は enqueue 時に
`WIRE_MAX_STRING_BYTES` で切り詰める** — encode 側は bound を検査しない
（codec の「encode は trusted」法則）ため、切り詰めないと peer 側が frame 全体を
reject する。切り詰めは正直な損失（prefix は届く）として doc に明記。

## 8. wire codec 割当

`encode_param_notice`/`decode_param_notice`（`encode_dirty_notice` 準拠の `u8` tag 形式）:

| tag | variant | payload |
| --- | --- | --- |
| 0 | `ValueChanged` | `u32 param_id` + `f64 value` |
| 1 | `GestureBegin` | `u32 param_id` + `u8 has_offset`（strict {0,1}）+ `has_offset=1` のとき `u32` |
| 2 | `GestureEnd` | 同上 |
| 3 | `RescanRequested` | `u32 aspects` |
| 4 | `ClearReferences` | `u32 param_id` + `u32 aspects` |
| 5 | `FlushRequested` | なし |
| 6 | `PresetLoaded` | `u8 location_kind`（0=Plugin,1=File+`push_str`）+ `u8 has_key`+`push_str` |
| 7 | `PresetLoadFailed` | location + key + `i32 os_error` + `u8 has_msg`+`push_str` |
| 8 | `Dropped` | `u32 count` |

`DirtyNotice` codec 追加: tag 3 = `DroppedEdits` + `u32 count`。
`Reply::ParamNotices` decode は `read_count(WIRE_MAX_PARAM_NOTICES)` で bounded。

## 9. fixture と oracle

- **VST3**: `ParamController::adopt_and_announce` が `performEdit(0, 0.5)`+`setDirty(1)`
  を無条件発火済み → `ValueChanged{0,0.5}` が e2e で届く。`beginEdit`/`endEdit` は
  fixture に追加しない（restore 経路に gesture edge を捏造しない — gesture の
  到達は bridge 単体 test で検証）。
- **CLAP**: `apply_restored_gain` が値変更 edge で `rescan(VALUES)`+`mark_dirty` を
  発火済み → `HostParams` 登録で `RescanRequested{VALUES}` が e2e で届く。
  `loaded`/`on_error` は host→plugin の wire verb が存在しないため e2e 不可 —
  bridge 単体 test（in-process `PluginInstance` + `RescanProbe` 先例）で到達を検証。
- e2e: `l1_stateful_test_plugin_e2e.rs` 両 ABI に `#[ignore]` test を追加。
  `RestoreState` → `PluginHost::drain_param_notices` → `ReportDrain::Drained` で
  VST3 は `[ValueChanged{0,0.5}]`（2回 restore で2回）、CLAP は
  `[RescanRequested{VALUES}]`（change edge のみ）。

## 10. consumer 契約（synvelis 需要との対応）

- automation 記録: `GestureBegin`/`GestureEnd` で区間を、`ValueChanged`（+ process 面
  `EventKind::ParamValue`）で値を記録。`ClearReferences` は記録済み automation の
  クリア要求として解釈可能。
- host UI ライブ値反映: `ValueChanged` + `RescanRequested{VALUES}`（「値を再読め」=
  report 面を参照し直せ）+ `PresetLoaded`（preset 適用による一括変化）。
- drop 観測: `Dropped`/`DroppedEdits` で back-pressure が consumer 照会面に出る。
