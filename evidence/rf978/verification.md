# evidence/rf978 — MIDI miniport（DMusUART 派生 virtual MIDI loopback）設計と検証記録

Issue #978（[Issue #952](https://github.com/scarecrow173/ruviform/issues/952) Slice C）の実装記録。Slice B の `drivers/rvf-virtual-audio/` package に DMus MIDI miniport を同梱し、WinMM midiIn/midiOut に同名 `Ruviform MIDI Loopback` pair を列挙させて `tests/loopback.rs`（`mise run test-midi-virtual-endpoint`）の bidirectional name match + round-trip を `windows-2025` x64 test-signed install 上で成立させる。本ファイルは設計意図・ローカル検証結果・runner 実走の状態を記録する。**runner 実走は PR merge 前に branch ref への `workflow_dispatch` で実施**（rf977 attempt 14-21 で確立した pre-merge 実証パターン — §2）。

## 1. workflow / driver 設計

### 1.1 採用方式 — DMusUART vendoring + software loopback

MIDI miniport には WDK 8.1 "DirectMusic UART Driver Sample"（DMusUART — `uri247/wdk81` @ `5a28cfcafb45fcc7fcbb5bb1975cddfd7202f98f`、`DirectMusic UART Driver Sample/C++/`）を vendoring し、MPU-401 UART hardware 経路を software render→capture loopback に置き換えた。

- DMusUART は `microsoft/Windows-driver-samples`（全履歴）・`microsoft/Windows-classic-samples` のいずれにも存在しないため、verbatim MSDN samples mirror からの vendoring が唯一の経路。
- license は MS-PL ではなく **MS-LPL v1.1** — §3(F) platform limitation（Windows OS/runtime/application platform 上で動く software に限る）は kernel driver に固有のため実質非制約、§3(D) の全文同梱義務に `LICENSE-MS-LPL.txt` で対応。upstream copyright 表記は派生ファイル先頭に保持（§3(C)）。
- in-box `PcNewMiniport(CLSID_MiniportDriverDMusUART)` は実 I/O resource を要求するため使用不可 — vendoring + virtual-device 適応が必須経路。
- 出所・変更差分の正本は `drivers/rvf-virtual-audio/PROVENANCE.md`。

### 1.2 filter / loopback 設計

- **登録**: `adapter.cpp` の `InstallMidiFilter`（`StartDevice` の `InstallAllCaptureFilters` 直後）が `InstallSubdevice(L"MidiLoopback", CLSID_PortDMus, CreateMiniportRvfMidi, IID_IPortDMus)` で DMus subdevice を登録。`g_MaxMiniports` に `g_MaxMidiFilters=1` を加算（`minipairs.h`）。subdevice cache には載せない（topology pairing・power interface・後続 lookup 無し — PnP remove での teardown は portcls 管理）。
- **filter**: 5 pin（legacy render sink / DirectMusic render sink / bridge out / bridge in / capture source）+ synth/input 2 node の upstream DMusUART 構成を維持。
- **同名 port**: render/capture 両 stream pin の `KSPIN_DESCRIPTOR.Name` に同一独自 GUID `RVF_MIDI_LOOPBACK_NAME = {714b96c3-8003-41c2-b673-1b086cbdb3db}` を割当て、INF `HKR\MediaCategories\{guid}\Name = "Ruviform MIDI Loopback"` で文字列に結び付ける（audio endpoint の `RVF_CABLE_*` と同じ pin-name GUID→MediaCategories 機構）。wdmaud が DMus filter を WinMM midiIn/midiOut に展開する際 pin-name がポート表示名になるため同名 pair が成立する。
- **loopback 経路**: render stream の `ConsumeEvents → Write` で流れる MIDI byte を 128-byte 内部 ring FIFO（`m_MidiInputBuffer`）へ積み（`FeedCaptureFifo`）、`IPortDMus::Notify(m_pServiceGroup)` で capture stream の `SourceEvtsToPort` を駆動して `DMUS_KERNEL_EVENT` 化 → `m_sinkMXF->PutMessage`。FIFO は `m_InputBufferLock`（`KSPIN_LOCK`）で serialize（`Write` は timer DPC 経由の DISPATCH_LEVEL）。capture 未 RUN 時は cable 未接続として送信 byte を破棄。FIFO full 時は partial write → timer retry。
- **resource/IRQ 非依存化**: root-enumerated virtual device には IRQ/I/O port resource が割り当たらないため、`ProcessResources`/`InitializeHardware`/`PcNewInterruptSync`/ISR を全削除し `m_fInitialized=TRUE` で `SetState(KSSTATE_RUN)` を成立させた。
- **upstream 潜在 bug 回避**: `ConsumeEvents` の package 分岐 — upstream の `PutMessageLocked` は `!m_TimerQueued` 時に `ConsumeEvents()` を再帰呼出しするが同関数は `m_DpcSpinLock` を取得し直すため deadlock。enqueue のみに留め drain は呼出元 loop に委譲した。

### 1.3 workflow — `midi-loopback` stage

`virtual-audio-driver-probe.yml` に `midi-loopback` stage を追加（`stream-roundtrip` と `cleanup` の間、同一の fail-closed stage 語彙）:

- install 済み devnode 上で `mise run test-midi-virtual-endpoint`（`cargo test -p ruviform-midi-device --test loopback -- --ignored --test-threads=1`）を実走し、`verdict:` 行を `stage-midi-loopback-<os>.txt` ログと `vaudio-probe-<os>.json` の stage record に記録。
- `NotRun`（pair 未検出）は正直な観測値で失敗扱いしない。`Fail` のみ `error` 記録。
- artifact は既存 `stage-*-<os>.txt` glob が網羅。
- `endpoint_probe` 型の独立 probe script は設けない — Rust test 自身が WinMM enumeration（`enumerated:`/`candidate[i]:` 行）と round-trip を出力するため、それがそのまま probe 証拠になる（`stream-roundtrip` と同構造）。

## 2. 実走結果の記録経路

1. `gh workflow run virtual-audio-driver-probe.yml --ref scarecrow173/rf978-midi-miniport` で **branch ref に pre-merge dispatch** する（rf977 attempt 14-21 のパターン — PR 段階の branch head でも `workflow_dispatch` は当該 ref に workflow が存在すれば実行可能）。
2. artifact（`virtual-audio-driver-probe-windows-2025-attempt-<n>`）を `gh run download` で取得し、`stage-midi-loopback-*` を含む stage 記録を `evidence/rf978/` へ `attempt<n>-*` として記録・コミット。
3. 判定は stage 成立範囲の解釈として本ファイル §4 へ追記。

## 3. ローカル検証記録（PR 段階・この worktree で実施）

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| workflow 構文検査 | `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml` | **Pass**（exit 0・指摘なし） |
| workflow contract 全走査 | `mise run test-ci-daily` | **Pass**（113 tests） |
| tracker workflow contract | `mise run test-ci-tracker` | **Pass**（108 tests） |
| 衛生チェック | `mise exec -- prek run --all-files` | **Pass**（全 hook） |
| フォーマット | `mise run fmt-check` | **Pass**（exit 0 — crates 非変更で無差分） |
| workflow run-block PS parse | `pwsh ParseInput` | **Pass**（15/16 — fail は pre-existing `env-build-surface` の `"${{ runner.* }}"` 文字列内式、GitHub が実行前に置換するため実害なし。midi-loopback stage は parse OK — `local-syntax-check.txt`） |
| vcxproj XML | `pwsh [xml]` load | **Pass**（well-formed・ClCompile=19・`RvfMidiMiniport.cpp`/`RvfMidiMpu.cpp` 含む） |
| INF 構造 + 相互参照 | pwsh regex | **Pass**（9 項目 OK・`%Strings%` 未解決 0・KSNAME/GUID 3 側一致 — `local-inf-check.txt`） |
| header API 照合 | dmusicks.h/dmusprop.h vs upstream+ReactOS mirror | **Pass**（`PACKAGE_EVT`/`DMUS_KEF_*`/`IMP_IMXF`/`CLSID_*` — Write は IMXF 外の追加メソッドで正しい） |
| InstallSubdevice 照合 | common.h:580 signature + common.cpp:1764 impl | **Pass**（引数順一致・MiniportPair NULL は callback forward のみで安全） |
| Cargo `check`/`lint`/`test`/`build`・`*-full` | — | **NotApplicable**（`crates/` 変更なし — 編集境界外） |
| driver kernel build / 署名 / install / MIDI 列挙 / round-trip | — | **NotRun**（worker ホストに WDK/MSBuild 未搭載 — `Windows Kits\10\Include` に `km/` 不在を確認。runner 上で判定） |
| runner 実走（branch `workflow_dispatch`） | — | **Done**（attempt 1-4 — §4。driver-build/install/endpoint-probe/stream-roundtrip **Pass**、midi-loopback は §4.1 の crate latent bug で test crash） |

補助証拠: 同ディレクトリの `local-syntax-check.txt`・`local-inf-check.txt`。ローカル実行ログは `actions.jsonl`。

## 4. runner 実走記録

| attempt | run | stage 結果 | 観測 |
| --- | --- | --- | --- |
| 1 | [45530647095](https://github.com/scarecrow173/ruviform/actions/runs/45530647095) | driver-build / install / endpoint-probe / stream-roundtrip / midi-loopback **全 step green** | `midi-loopback` は `test_exit=5; verdict=no-verdict-line` — test process が `STATUS_ACCESS_VIOLATION (0xc0000005)` で crash し verdict 未到達。当時の mapping は `^Fail` のみ error 化するため crash が `ok` に丸められた（後続 attempt で fail-closed 化） |
| 2 | [45531712361](https://github.com/scarecrow173/ruviform/actions/runs/45531712361) | midi-loopback `error`（正直化） | 子プロセス WinMM probe 追加 — 全 call succeed、`Ruviform MIDI Loopback` が in/out 両方向で列挙・open/send/close・loopback send 経路まで完動を確認。**ただし probe の DRV_QUERYDEVICEINTERFACE 定数が誤値（1006/1007、正値 2060/2061）で interface query だけは未実測だった** |
| 3 | [45533730791](https://github.com/scarecrow173/ruviform/actions/runs/45533730791) | midi-loopback `error` | CALLBACK_FUNCTION+MIDI_IO_STATUS+MIDIHDR probe 追加 — **`callback count=3 lastMsg=0x3c3 (MM_MIM_DATA) lastP1=0x3c80`（送付した note-off byte が render→capture FIFO 経路を通り user-mode callback に届いた）＝ driver loopback が end-to-end で機能する決定的証拠**。test は依然 `enumerated:` 未到達で crash。LocalDumps/WER では dump 取得できず |
| 4 | [45534521149](https://github.com/scarecrow173/ruviform/actions/runs/45534521149) | midi-loopback `error` | test exe を cdb 直下で実行し **crash stack を確定**（下記 §4.1）。crash は `interface_id()` → `midiInMessage(index, DRV_QUERYDEVICEINTERFACE=2060)` 内部の `winmmbase!StringCchCopyW` — write to `0x90` |
| 5 | [45535388527](https://github.com/scarecrow173/ruviform/actions/runs/45535388527) | midi-loopback `error`（test は依然 crash — crate bug 未修正のため） | probe に正しい定数（2060/2061）と正しい引数順の DRV query を実装 — **device-index-as-handle で `rc=0`・`\\?\root#media#0000#{6994ad04-93ef-11d0-a3cc-00a0c9223196}\midiloopback` を取得**（§4.1 の引数取り違えを対照証明）。open-handle 版は `rc=8` NOTSUPPORTED（workaround として不適 — index 経路のみ応答）。callback round-trip も再現（`count=3 lastMsg=0x3c3 lastP1=0x3c80`） |

### 4.1 crash root cause（attempt-4 cdb + attempt-5 probe で確定）

```
winmmbase!StringCchCopyW+0x2e        <- AV: Attempt to write to address 0x90
winmmbase!mregHandleInternalMessages+0x1e5
winmmbase!midiIDMessage+0x10c
winmmbase!midiInMessage+0x57
loopback!interface_id                <- midiInMessage(index, DRV_QUERYDEVICEINTERFACE, ...)
loopback!enumerate_at -> input_ports -> select_loopback_pair
```

- `crates/ruviform-midi-device` `os::windows::interface_id()`（`windows.rs`）は `DRV_QUERYDEVICEINTERFACE` を **`dw1=size, dw2=buf` の取り違え順**で呼ぶ。WinMM 契約（および midir 実装）は `dw1=buf pointer, dw2=size`。
- winmmbase は `dw1` を宛先 pointer として `StringCchCopyW` するため、**AV address `0x90` = interface string byte size 144 が pointer 解釈されたもの**。attempt-5 で正しい引数順の同 query が `rc=0` + interface path（72 UTF-16 units = 144 bytes = `0x90` で整合）を返すことを対照証明済み。
- latent 条件: `SIZE` query が 0 を返す port（legacy / interface 未応答）は early-return で crash に到達しない。interface を応答する real WDM device（本 port を含む）が input に存在した時点で確定 crash — OS/driver 非依存の純粋な crate bug。`resolve_index` も同じ `interface_id` を通るため同じ crash 経路を持つ。
- **修正は `crates/**`（本 task の編集境界外）**: `interface_id` の `DRV_QUERYDEVICEINTERFACE` 呼出しで `dw1`/`dw2` を正順に入れ替える（`buf.as_mut_ptr() as usize, size as usize`）— 1 行修正。
- 付随観測: open-handle 版の同 query は `rc=8` NOTSUPPORTED（「open してから query」の workaround は不適）。device-id-as-handle 経路のみ応答する — midir と同じ呼出し形態が正しい。

### 4.2 driver 機能判定

`midiIn`/`midiOut` 両方向の `Ruviform MIDI Loopback` 同名列挙・open/close・`midiOutShortMsg` 送信・**render→capture FIFO 経由の callback 受信（note byte 往復、attempt-3/5 で再現）**・**DRV_QUERYDEVICEINTERFACE（正しい引数順）での interface path 応答** まで probe で完動を確認 — WinMM 契約上の deliverable（同名 pair 列挙 + round-trip 成立 + interface-id 応答）は driver 側で達成済み。`tests/loopback.rs` が verdict に到達できないのは §4.1 の crate 引数取り違え bug による（`crates/**` は編集境界外 — coordinator 判断へ報告）。
