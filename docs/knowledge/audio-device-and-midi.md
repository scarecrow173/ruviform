---
id: "KNOW-CmfIfzajrECw9xSJUSHMmP"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "オーディオデバイス・MIDI のバックエンド設計"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/188", "https://github.com/scarecrow173/ruviform/issues/253", "https://github.com/scarecrow173/ruviform/issues/565", "https://github.com/scarecrow173/ruviform/issues/629", "https://github.com/scarecrow173/ruviform/issues/707"]
  informs: []
  explains: []
  supersedes: []
---

# オーディオデバイス・MIDI のバックエンド設計

この文書は、オーディオデバイスと MIDI バックエンドの設計・実装から得られた再利用可能な知見をまとめたものである。内容は歴史的な実装記録から抽出したものであり、元の記録は削除済みのため、この文書は単独で読める形にしてある。各知見には出典の Issue 番号を併記し、観測された事実と解釈を区別する。

## MIDI イベント処理のクレート境界

Timed MIDI input の語彙・stamp/anchor/mapper・NoteLedger は media-core、EventKind::Midi・wire codec・merge は plugin-core、input port は audio-core、consumer 機構は bridge、規格固有差分は adapter に配置するという境界で実装された。OS backend・graph 配線・VST3 IEventList・CLAP NoteExpression/SysEx・L1 verb・録音 tap は当時分割 Issue に送られた。raw stamp は consumer のために保存し、chain 内に PPQN quantization を置かない決定がされた (Issue #188)。新規クレートを作らず既存 crate に追加する ADR Option A が採用された。

`merge_call_events` の merge 規約は plan/live/release を bounded stable k-way merge し、同一 offset は source index 順に並べる。ただし同一 `{group,channel,key}` の NoteOff を NoteOn より先に置くのは cross-source collision のみに適用され、同一 source の authored `[On, Off]` 順序（zero-length note）は保存される。scratch 不足は `MergeOverflow` で fail-closed に処理する (Issue #188)。

wire codec は未知 tag/sub-tag を `Malformed` で拒否し、silent drop の経路を作らない。`EventKind::Midi` は wire kind tag 1、routing byte は `group<<4|channel`、sub-tag 0–6 が canonical payload、reserved u64 はゼロ必須とする。未知 tag・範囲外 field は `WireError::Malformed` であり、黙って encode/drop する経路は存在しない (Issue #188)。

capability 宣言は正直さを保つ方針で行う。`midi_channel_voice`・`midi_ump`・`note_expression`・`midi_sysex` の 4 capability field を追加し、行列値は L0/L1=Verified、CLAP=Declared、VST3=全項目 Unsupported + defined-skip note と宣言した。adapter の非 ParamValue `let-else` 暗黙 skip は match + counted defined-drop に置き換えられた (Issue #188)。

`chase_span_events` は有界 backward scan と generation-tag 一致で実装する。録音済みイベントの loop/seek chase は plan prefix（`position ≤ ctx.position_ticks`）を `CHASE_SCAN_BUDGET=2048` で backward scan し、`plan.generation == ctx.plan_generation` の snapshot のみを対象とする。「P 時点で発音中の note」の last NoteOn を offset 0 で再発火する。CC/pitch/param の last-value chase は未実装で deferred contract とされた (Issue #188)。

NoteLedger eviction は有界 evicted-key queue で synthetic NoteOff を供給する。capacity eviction された旧 key は有界 queue に記録され、`CallSink::drain_evicted` が毎 call offset 0 で synthetic NoteOff を merge 先頭へ届ける。merge source order は `[evict, release, chase, plan, live]` である。transport 不連続検出は span ctx key 比較に加え position の前 span 期待後続一致も見るため、同 epoch SetClock ジャンプも捕捉される (Issue #188)。

RT 経路での dealloc/capacity 喪失は review で修正された。`TransportScheduler::device_restart` の `mem::take(&mut pending)` が RT 上の dealloc と reserved capacity 喪失を起こすことが判明し、in-place reject + clear に修正された（`drain_receipts`/`drain_faults` も同種バグを修正）。foreign-epoch SetClock は `StaleEpochControl` で拒否し、node cursor を表現不能な epoch へ遷移させない (Issue #188)。

InputStage の fault は全て counter+report で可観測とする。queue full/unmapped domain/before anchor/epoch mismatch/ledger overflow/merge overflow の各 fault が `InputStageCounters`/`BlockInputStats`/`NodeReport` 経由で可観測である。transport 不連続（tg 変化・Stop・device_restart）では `NoteLedger::release_all` を offset 0 で供給し stuck-note を防ぐ。`INPUT_STAGE_CAPACITY=1024`・`INPUT_DRAIN_MAX` budget で bounded である (Issue #188)。

## MIDI output port と WinMM backend

`midi_output` port は `midi_input` の鏡像として設計された。`ruviform-audio-core` に `MidiOutputEnumerator`/`MidiOutputPort`/`MidiOutput`/`MidiOutputInfo` と deterministic mock を追加し、`MidiPortOrigin`/`MidiPortCapabilities` は `midi_input` から re-export して共有した（複製ではない）。`send_sysex` は default で `Err(DeviceError::Unsupported)` を返し、long-message 非対応 backend の honest refusal とした (Issue #629)。

`encode_message` は `decode_message` の逆写像で `try_from` を使う。group!=ZERO と未知 MidiKind（`#[non_exhaustive]` wildcard）は None で拒否し、値は `MidiValue::as_u7`/`bend_as_u14` の canonical narrowing で取り出す。pitch bend の 7-bit byte 取り出しで `as u8` cast は `clippy::cast_possible_truncation`（workspace は `-D warnings`）に抵触するため `u8::try_from(...).ok()?` を採用した (Issue #629)。

表現不能 event と framing 違反は `DeviceError::Unsupported` に統一した。`encode_message` が None の event と `0xF0…0xF7` でない SysEx bytes はどちらも「この port の wire 表現で送れない」ため同じ error に写像する。`DeviceError` に `InvalidArgument` 相当 variant がなく、追加は shared enum で影響が大きいため見送った。写像の限界は記録に残した (Issue #629)。

SysEx 送信は同期 `MM_MOM_DONE` wait + stack frame 生存の buffer で実装した。`midiOutLongMsg` は非同期で MIDIHDR/lpData は `MM_MOM_DONE` まで生存必須だが、callback は allocation/OS call 不可のため完了通知は `Mutex<bool>`+`Condvar` の flag のみとした。buffer（`Box<MIDIHDR>`+`Box<[u8]>`）を `send_sysex` の stack frame に置き prepare→longmsg→done→unprepare を同期実行し、pool/世代カウンタを持たない（`&mut self` 直列化で競合は構造上起きない）。device stall 時に control path が無期限 block し得る点は doc 明記の既知リスクとした (Issue #629)。

`midiOutShortMsg` は queue acceptance のみを返す fire-and-forget である。WinMM の `midiOutShortMsg` は queue 受理のみを返し、device 側消費・順序保証は driver 依存であり、port contract が fire-and-forget として正直に宣言済みである。port identity は index+name のみで、WinMM に安定 ID がない制約の正直な帰結として、index 再編や同名 device で誤 device を開く可能性が残る (Issue #629)。

`midiOutOpen` の `MMSYSERR_INVALFLAG` 対応として、`CALLBACK_FUNCTION` で 1 回 retry する fallback を実装した。fallback 経路では `MM_MOM_DONE` が届かないため、`send_sysex` は早期 `DeviceError::Unsupported` に degrade する。driver 参照中 buffer の unprepare は unsound なため fire-and-forget 降格ではなく拒否が唯一の正直な degrade である。`send`（`midiOutShortMsg`）は completion 不要で無影響である (Issue #629)。

`midiOutMessage` + `DRV_QUERYDEVICEINTERFACE` は crash し得るため呼ばない。Microsoft GS Wavetable Synth（output index 0）で `DRV_QUERYDEVICEINTERFACESIZE` が `rc=0,size=2` を返した後の fetch 呼出しが WinMM 内で segfault した実測がある。input 側 `midiInMessage` は安全に動作する。output 側 stable ID 取得は断念し、port identity は index+name fallback のみに限定された（SetupAPI 等の別手段は後続課題）(Issue #629)。

## WASAPI 入力パスと frame_position

cpal 0.18.2 の WASAPI host は `GetBuffer` の `device_position` を Xrun 判定にのみ使い捨て、公開 `InputCallbackInfo` に frame position がない。timestamp→frame 合成は port 契約で禁止されている（不連続を隠す + QPC≠audio 発振器でドリフトするため）。adapter→adapter 依存禁止のため別 crate では解決不能であり、「第 2 backend は別 crate」原則の例外として `wasapi_input.rs` を crate 内に実装した（Coordinator 承認済み）(Issue #565)。

`frame_position` は `device_position` のみから供給する。packet の `device_position` のみを `frame_position` に使い timestamp 合成はしない。`PositionTracker` が `prev_position+prev_frames` との連続性を検査して jump/reset を `ClockDiscontinuity` に写像し、first packet の position 0 は正常扱いとする。scratch 超過 packet は複数 callback に分割し `pos_next = pos_current + frames_delivered` で進める (Issue #565)。

rollback は `#[cfg(windows)]` 分岐の revert のみで cpal 経路に即復帰できる。`frame_position` が None に戻るだけで他契約は不変である。残存リスクとして `DATA_DISCONTINUITY` flag を立てるが position が連続する device では `ClockDiscontinuity` が過剰報告され得る。ただし cpal も同一条件で Xrun を報告しており parity は保たれている (Issue #565)。

`windows::core::implement` macro 展開には module-level `#[expect]` と `windows-core` 直接依存が必要である。`implement` macro の展開 span には item-level `#[expect]` が効かないため、`unsafe_code`/`clippy::multiple_unsafe_ops_per_block`/`clippy::missing_safety_doc` を module level の `#[expect]` に集約した（implement は notification/async handler の 2 箇所のみ）。また `implement` が `windows_core::...` path に展開されるため `windows-core = "0.62"` を直接依存に追加した。windows crate 自体は cpal の dep として lockfile に既存で新規 package 追加はなかった (Issue #565)。

非 Windows・非 WASAPI・output stream は cpal 経路のまま `frame_position=None` で不変である。Windows 限定 compile の `rt_` test は ubuntu の test-rt lane で収集されないため local 実行を証拠とする (Issue #565)。

## WASAPI probe の実測 quirk と query-only 境界

`IAudioClient::IsFormatSupported` は有効な out-param が必須である。`ppclosestmatch=NULL` を渡すと本ホスト build で `E_POINTER` を返す実測 quirk がある。常に有効な out-pointer を渡し、返却された closest format は `CoTaskMemFree` で解放する。WASAPI のこの API は「サポート外でも closest を返す」設計であり NULL out-param を拒否する実装がある (Issue #629)。

not-present/unplugged endpoint も列挙されるが Activate は `E_NOTFOUND` 系で失敗する。probe ホストで 61 endpoint が列挙されたがほぼ stale であり、列挙に現れるだけでは不十分で activate 成功が実用性の下限である（ただし streaming 証明でもない）。`GetStreamLatency` は `Initialize` 前に `AUDCLNT_E_NOT_INITIALIZED` を返すため推測せず `NotRun` として報告した (Issue #629)。

audio probe は cpal ではなく windows crate の COM/WASAPI を直接使用した。endpoint state（not-present 含む全 state）と生 endpoint id は cpal port surface に出ないため、`IMMDeviceEnumerator`/`IAudioClient` を直接 query した。cpal view は friendly-name join 補助のみ。`IAudioClient::Initialize`/`Start` は呼ばず副作用最小の query-only probe とした。依存・feature 追加なし (Issue #629)。

probe verdict は常に仮想 endpoint case を `NotRun` とする。「列挙できた≠使える」を固定し、両 probe は未 provisioning 環境を `NotRun`/`NOT_PROVISIONED` で fail-closed に報告し、virtual-os-endpoint evidence を主張しない文言を出力に埋め込んだ。virtual/loopback 判定は name heuristic / bidirectional match に限定し「candidate (unconfirmed)」表記とした。API が仮想性を証明しないため provisioning 決定の根拠にしない (Issue #629)。

## 仮想エンドポイント verifier の oracle 設計

WASAPI verifier は SRC 耐性のある独立 oracle 3 系統を採用した。信号は 440Hz sine(0.20)+8frame 0.70 impulse/100ms。oracle は (a) marker 検出 `|x|>0.5`+20ms refractory で ≥3 個・平均間隔 100ms±10%、(b) 観測/期待 RMS ratio [0.2,3.0]、(c) Goertzel 440Hz tone share ≥0.25 である。仮想 cable 内部 SRC（48kHz）を通っても成立し bit-exact は要求しない。閾値は provisional で provisioned runner 実測で締める (Issue #629)。

MIDI loopback verifier の pair 特定は name 双方向一致のみで行う。port id 体系が方向で異なるため `info().name` の双方向一致で pair 化し、name match は candidate であり virtual の証明ではない（fail-closed を test-spec R6 で固定）。期待列は独立 literal oracle（`MidiValue::from_u7`/`bend_from_u14`）。timestamp 検証は宣言 domain・Carried・非減少 ticks のみで、driver-domain tick 差は観測記録に留め send wall-clock との絶対 latency は主張しない (Issue #629)。

name-stem pair 規則は probe と verifier で複写同期する。example は test から import できないため `VIRTUAL_NAME_HINTS`/`is_virtual_name`/`DIRECTION_TOKENS`/`virtual_stem` を両 file に複写し相互参照 sync コメントを付けた。stem は小文字化→英数字以外分割→方向語（input/output/render/capture/in/out）除去→空白 join で "CABLE Input"↔"CABLE Output" を同一 stem に正規化する。name match は candidate であり virtual-os-endpoint の証明にも Pass 根拠にもしない (Issue #629)。

WASAPI verifier の観測 discipline は capture を channel 0 のみ pre-allocated buffer に try_lock 付き bounded copy とし、溢れ・lock 失敗は counter 可視化、RT alloc/block/panic なしとする。frame_position gap が通知済み discontinuity 数を超えた場合のみ Fail（未通知不連続は契約違反）。xrun/error/discontinuity は AC5 領域として記録のみで gate しない。60s watchdog 超過で exit 3。powerShell/registry 観測（driver 署名・microphone 権限）は失敗時も `NotRun`/absent note で probe exit 0 を維持する (Issue #629)。

## Test Runtime 結線と lane 運用

Test Runtime wiring は逆方向 dev-dep（audio-test→device crates）を採用した。`ruviform-audio-device`→`ruviform-audio-test` dev-dep は audio-test が引く `ruviform-audio`/plugin-bridge/graph 系が device crate の default cargo test compile graph に載るため不承認のままである。逆方向（audio-test dev-deps→両 device crate）は crate cycle を生じず（`ruviform-audio` は両 device crate に依存しない）、plugin-bridge→midi-device・plugin-test-vst3→両 device crate の先例と整合する。dev-dep に留め lib/consumer の解決を汚染しない。verifier 側で RunMetadata 準拠 JSON を emit する schema 重複案は不採用（schema 単一正本を保てない）(Issue #629)。

`RunMetadata::for_environment` は `NotRun` 初期化で fail-closed を型保証する。`for_environment(case_id, requirement, level, environment_kind, oracle, parameters, repo, sha)` を追加し result を `CaseResult::NotRun` で初期化 — 構築だけでは Pass にならない。`deterministic()` は `OsL1`+`DeterministicBackend` の shorthand として維持。expected 側は Test Runtime が供給する Fixture/metadata、observed 側は独立 literal/tolerance oracle で自己参照 Oracle 禁止を維持する (Issue #629)。

opt-in lane 登録の確立パターンは、provisioned pair 前提の manual lane を mise.toml に description へ provisioning 必須・manual・never CI・fail-closed verdict: `NotRun`・evidence 格納先を明記して登録する。専用 lint lane は既定 `lint <crate>`（`--all-targets`）が coverage するため追加しない。`--test loopback` は name filter でなく target 全体収集とし description で両テストの依存差を明記（name filter の fail-open 回避）。silent-send smoke は実 device 依存で virtual-os-endpoint の証拠にならないことを明記する (Issue #629)。

`EnvironmentKind` は additive のみ・serde 綴り gap は既知 gap として記録する。Case Schema v1 closed set に欠落していた `VirtualOsEndpoint`/`PhysicalDevice` を additive 追加し literal pin テストを新設した。既存 `RealAbiTestPlugin` の `real-abi-test-plugin` ↔ schema `test-plugin` の綴り差は serde 語彙の破壊的変更（plugin-dock-test が evidence JSONL に出力中・AL/SV が dev-dep 消費）のため本 slice では解消せず既知 gap として docs 記録し後続 slice の serde 移行判断へ hand-off した (Issue #629)。

append-only Evidence 節と generated index の運用は、「never edit/reorder/delete existing」規約で dated entry のみ追記する。既存 entry の stale 記述は編集せず新規 entry で解消を明記する。追記 anchor 直後に未読の末尾行があり一時 orphaned になったため、追記前に節末尾まで読むべきことを確認した。docs/**/README.md index は生成物で slice 編集範囲外、Coordinator 統合時に再生成する (Issue #629)。

libtest capture のため verdict 採取には `--nocapture` 補助証拠が必要である。canonical lane の run に `--nocapture` を含めないため verdict 行は libtest capture 内に隠れ、canonical ログでは `test ... ok` のみ見える。evidence は canonical lane 実行ログ + `--nocapture` 付き直接実行を「補助証拠」と明記した 2 部構成とする運用が確立した (Issue #629)。

provisioning runbook の内容は、専用 Windows runner は repo 外工程（開発 PC への無断 driver 導入禁止）。license 事前確認を必須ゲート化: VB-CABLE donationware・loopMIDI 私的非商用限定・VAC 商用 perpetual・Windows MIDI Services loopback は OS 付属。候補別 install/cleanup/probe 手順・capability probe 判定主導フロー・両 lane 検証手順・`NotRun` 解釈・driver 署名/権限/cleanup の記録責任分担を収録した。capability matrix は ❓/⚠️ 項目と Open Questions #1-#7 を実測待ちとして明記し、runbook は候補を決め打ちしない (Issue #629)。

os-endpoint-lane の `--ignored` の収集範囲と merged IR 非接触は、lane self-run で `2 passed・3 filtered out` により `--ignored` が `#[ignore]` leg のみを対象とし deterministic legs は lane 対象外であることを確認した。リース外の merged IR は無変更とし、Follow-up 消化は本 slice 側の IR・design・survey 追記で記録する方針。append-only 文書は dated 追記のみ (Issue #629)。

MIDI output mock の scripted fixture の契約は、`midi_output::mock` の `MockBackend`/`MockPort` は `Arc<Mutex<PortState>>` で送信 MidiEvent・SysEx bytes を順序記録（`sent_events`/`sent_sysex`/`sent_count`/`clear_sent`）。`fail_next_open`/`fail_next_send` は one-shot 消費型で失敗 send は未記録、close 後 send は `StreamInvalidated`。`midi_input` との鏡像で共有型 `MidiPortOrigin`/`MidiPortCapabilities` を再利用（複製ではない）。非 Windows `enumerate_output_ports` は `DeviceError::Unsupported` (Issue #629)。

name match は常に candidate であり virtual 証明ではない（fail-closed の中核規律）。全 slice を通じた一貫した境界として、pair/name match は candidate ヒューリスティックであり provenance 証明ではない。verdict 文言・test-spec・docs で virtual-os-endpoint Pass を主張せず、未 provisioning の self-run は verdict: `NotRun` を正直に記録し Pass を偽装しない。`environment_kind=virtual-os-endpoint` の RunMetadata emit も実際に endpoint を stream した candidate run のみに限定される (Issue #629)。

MSRV/daily CI lane は `NotRun` として記録する運用。全 slice で MSRV lane・*-full lane は daily CI 管轄として実行せず `NotRun` と明記。self-run evidence は canonical mise lane + 補助証拠の構成で、Pass 主張を明示的に否定する記述を evidence・docs・PR に残す。これにより未 provisioning 環境の headless 検証と provisioned runner での実走を混同しない (Issue #629)。

Issue #629 の slice 分割の構造は、1st survey+capability probe、2nd midi_output port+WinMM backend（PR #658）、3rd MIDI loopback verifier（PR #663）、4th WASAPI streaming verifier（PR #673）、5th audio virtual lane 登録+EnvironmentKind（PR #680）、6th provisioning runbook+MIDI virtual lane 分離、AC3 Test Runtime wiring、AC6 残部 os-endpoint-wiring lane の 7 slice に分割された。各 slice が spec→design→plan→task→test-spec→impl ir/exp の docs chain を持つ (Issue #629)。

virtual-endpoint-survey task の制約は、probe は手動 example・control-path のみ・`#[cfg(windows)]` 隔離・fail-closed verdict。編集許可範囲は doc chain・2 example・evidence/rf629/・管理 index のみで src/・Cargo.toml・mise.toml・workflow は触らない。driver install 禁止（本機に driver を導入しない）。OS API 面は example 内 win module に閉じ enabled feature のみ使用 (Issue #629)。

survey の候補比較の結論は、discovery doc で VB-CABLE/Hi-Fi・VAC・loopMIDI/teVirtualMIDI・Windows MIDI Services・SysVAD を比較し capability matrix・コードギャップ（当時 midiOut* 不在・`MidiPortOrigin::Unknown`）・provisioning runbook 方針を記録した。採用 endpoint の確定は provisioning 実測後の ADR/decision として後続 slice に送られた (Issue #629)。

`--wasapi-endpoint` の device→audio-test dev-dep は coordinator 不承認で oracle 自足。Coordinator 回答 (A) により `ruviform-audio-device`→`ruviform-audio-test` dev-dep は不採用（2026-11-15 承認済み依存変更リスト外）。oracle は test 内独立 deterministic generator（`signal_sample`: 440Hz sine+100ms impulse marker burst）で自足させ、#628 PcmFixture/RunMetadata との IF は docs に記録して後続 slice へ hand-off。新規 feature flag・dependency なし (Issue #629)。

MIDI loopback の期待列は `MidiScenario` を再利用せず独立 literal。`MidiScenario`（audio-test）は `TransportScheduler` 駆動用で port 直接送信に非適合のため再利用せず、形式言語のみ共有。4 event の固定リテラル列（NoteOn/CC/PitchBend/NoteOff、ch3、velocity 100/64、CC10=90、bend 0x2A00 center 以外）を `expected_channel_voice_sequence()` に置き、観測列への順序保存部分列一致で判定。timestamp 検証は ports.rs 先例どおり宣言 domain・Carried・非減少 ticks (Issue #629)。

MIDI output plan の SysEx の `MHDR_DONE` 待ちを blocking wait にしない判断は、`send_sysex` の `MHDR_DONE` は `CALLBACK_FUNCTION`+`MM_MOM_DONE` で受け、unprepare は done 確認後のみ（input の lazy requeue と対称）。blocking wait にしない判断は design に記録され `DeviceBusy` 直列化で対処。異常系で done が来ない場合の close→`midiOutReset` 依存は MSDN 契約に基づく。index+name fallback の誤解決リスクは input と同じ制約として記録 (Issue #629)。

pitch bend の 7-bit byte 取り出しは `u8::try_from` で lint 回避。`clippy::cast_possible_truncation` は u16→u8 の `as` を機械的に警告し workspace は `-D warnings` で実質 deny。`(raw14 & 0x7F) as u8` の代わりに `u8::try_from(...).ok()?` を採用 — Option 返しの関数と整合し `#[expect]` suppress を増やさない。doc_markdown lint 対応として doc comment 内の SysEx/WinMM は全て backtick 化した（`#[allow]` による lint 隠蔽は禁止）(Issue #629)。

lane 分離は、`test-midi-virtual-endpoint` は loopback target 全体収集、`real_device_silent_output_smoke` は仮想証拠にならない。loopback test target には `loopback_round_trip`（仮想 pair verifier）と `real_device_silent_output_smoke`（GS Wavetable 依存の実 device smoke）が同居。name filter で絞る案もあったが Coordinator 第一案の target 全体収集 + description 明記を採用。silent output smoke は実 WinMM output port への silent 送信 smoke であり virtual-os-endpoint の証拠にならないことを description・design・記録で明記。`test-midi-real-device` は run 不変の包括 lane（物理 port 系 + 全 `#[ignore]` 収集）として位置づけ、仮想 pair 専用は `test-midi-virtual-endpoint` と棲み分けた (Issue #629)。

probe 拡張の OS 観測は全て fail-safe best-effort（exit 0 維持）。endpoint_probe に pair-candidate 表（port view の (name,direction) から verifier と同一規則）、`Win32_PnPSignedDriver` を PowerShell 経由で name-hint regex 照合し staged catalog（`C:\Windows\INF\<inf>.cat`）に `Get-AuthenticodeSignature` を best-effort 適用、HKCU privacy keys（`DeviceAccess\Global\{2EEF81BE-…}`、`ConsentStore\microphone`、`NonPackaged`）を `Get-ItemProperty` で照会を追加。spawn 失敗・非ゼロ終了・キー absent は全て `NotRun`/absent の note で probe の exit code 0 を維持（midi_endpoint_probe の WMS 検出先例に準拠）。driver signature 観測は情報記録であって仮想 endpoint の証明ではない (Issue #629)。

verifier は production API のみ + test 内独立 oracle、name-stem pair 規則を probe と複写同期。`tests/virtual_endpoint.rs`（約1250行、`#![cfg(feature="real-device")]` + `#[ignore]`）は production `DeviceEnumerator`/`Device`/stream trait のみを使い、oracle は test 内 deterministic generator（`signal_sample`: 440Hz sine 0.20 + 8frame 0.70 impulse marker/100ms）で自足。`ruviform-audio-test` への dev-dep は Coordinator 決定(A)で不承認のため Test Runtime 結線は docs 記録のみ。example は test から import できないため `VIRTUAL_NAME_HINTS`/`is_virtual_name`/`DIRECTION_TOKENS`/`virtual_stem` を両 file に複写し相互参照の sync コメントを付けた (Issue #629)。

SRC 耐性 oracle は 3 系統（marker 間隔・RMS ratio・Goertzel tone share）で bit-exact を回避。仮想 cable 内部 SRC（VB-CABLE 系 48kHz）を通っても成立するよう bit-exact・位相厳密比較は行わない。oracle は (a) marker 数≥3 かつ平均間隔 100ms±10%（`|x|>0.5` + 20ms refractory で impulse のみ検出 — sine 0.20 は閾値未満）、(b) 観測/期待 RMS ratio [0.2,3.0]（SRC は unity-gain 仮定）、(c) Goertzel 440Hz 成分が全 power の 25% 以上 — の 3 系統。閾値は provisional で provisioned runner の実測で締める余地を残した。capture は channel 0 のみ pre-allocated buffer に try_lock 付き bounded copy（溢れ・lock 失敗は counter 可視化、RT 経路で alloc/block/panic なし）(Issue #629)。

frame_position gap は通知済み discontinuity 超過時のみ Fail、xrun 等は記録のみ。frame_position gap 数が通知済み `ClockDiscontinuity` 数を超えた場合のみ Fail（未通知の不連続は契約違反）。xrun/error/discontinuity の発生自体は AC5 領域として記録のみで gate しない。`StreamInvalidated` は対話的 device 除去が必要で本 slice 範囲外のため `NotRun` 固定。lifecycle は start→3s 観測→stop→post-stop query→restart→re-stop→drop（API に close はなく Drop が配信を止める）、60s watchdog（超過で exit 3）(Issue #629)。

WMS availability は cmdlet と service を別行で報告（service 稼働≠loopback 使用可）。probe は `powershell.exe -NoProfile -NonInteractive -Command` で `Get-Command New-MidiLoopback` と `Get-Service midisrv` を照会。実測で `New-MidiLoopback: absent` だが `midisrv: Running` という「service は常駐するが PowerShell module/cmdlet 未導入」の状態を区別して記録した。spawn 失敗・非ゼロ終了は `NotRun` note で probe を落とさない（`cmd /c ver` 先例）。module 内 `#[cfg(windows)]` の `Command` 呼出のみで新規 unsafe・windows feature なし (Issue #629)。

## fault 注入と cancelled semantics

fault 注入は MockPort read-level に置き leg-level script を却下した。cancelled observable は `MidiInput` subscription 契約の性質であり fixture 層が backend 観測可能性を所有するため、`MockPort`/`MockInput` の read-level injector（`fail_next_read`/`inject_cancelled`）を採用した。`MidiLeg`/`MidiFeed` の leg-level fault script は subscription 状態を複製し port() observable と乖離し得るため不採用（exp jsonl seq 1 decision）。production trait・`DeviceError`・pipeline 配線は不変である (Issue #707)。

cancelled semantics は error 値と分離し、dead subscription は fault を消費しない。`inject_cancelled` は `terminated=true` + `Err(Backend("cancelled"))` で subscription を終了（後続 `StreamInvalidated`）。`fail_next_read(Backend("cancelled"))` は非終端の負例として区別を固定する。read は `closed||terminated` を fault pop より先に検査するため dead subscription は armed fault を消費しない。cancelled read は close と区別（`is_closed=false`・`is_terminated=true`）し backend abort を observable に分離する (Issue #707)。

read 契約は `Err` と stats を同時に返せない制約がある。`MidiInput::read -> DeviceResult<MidiReadStats>` は `Err` と stats を同時に返せないため、audio 側の「error callback + data callback」併存観測は `fail_next_read`（失敗 read）と `inject_dropped`/`inject_unsupported`（stats 付き成功 read）に分解される。この分解の妥当性は Issue close 判定で問われ得るリスクとして記録された (Issue #707)。

MockPort に read-level fault injector（`fail_next_read`/`inject_cancelled`）を FIFO で追加した。`PortState` に `terminated: bool` と `read_faults: VecDeque<ReadFault>`（private enum `Fail(DeviceError)`/`Cancel`）を追加し、`MockPort::fail_next_read`・`inject_cancelled`・`is_terminated` を fixture handle 面に追加した。`MockInput::read` は `closed||terminated` を先に検査→`read_faults.pop_front()`→`Fail` は `Err(error)`、`Cancel` は `terminated=true` + `Err(Backend("cancelled"))`。いずれも drain 前に return するため pending/counters/out を消費せず、dead subscription は armed fault を消費しない。audio 側 `StreamFault::Cancellation`（#628 merge 0b510cf8）の MIDI 写像を deterministic OS-L1 で再現した (Issue #707)。

cancelled read は close と区別（`is_terminated` vs `is_closed`）、`Backend("cancelled")` 単体は非終端。cancelled read は close と区別: `is_closed()==false`・`is_terminated()==true` — consumer が close していなくても backend 側 abort で subscription が死ぬ observable を分離する。error 値 `Backend("cancelled")` と cancel semantics を分離し、`fail_next_read(Backend("cancelled"))` は非終端（負例 test で固定）。却下案: `MidiLeg`/`MidiFeed` leg-level fault script — subscription 状態の複製と observable 乖離リスクにより不採用。制約として `MidiInput::read` が `Err` と stats を同時に返せない契約上、audio 側「error callback + data callback」併存観測は失敗 read と stats 付き成功 read に分解される (Issue #707)。

## report drain の制御経路

report drain は control-plane verb で data plane ではない。report drain は plugin の非 RT 状態を読む操作であり shm slot+doorbell の block 単位 RT 契約に乗らないため、`SaveState`/`GetParam` と同じ instance-scoped command→bounded reply の形を採用した。`WIRE_VERSION` 3→4、`Command::DrainOutputReports`（tag 23）・`Reply::Reports`（tag 8）。report event は既存 canonical `Event`/`EventKind` codec を再利用し decode は `WIRE_MAX_EVENTS=1024` で allocation 前に bound する (Issue #253)。

runner backlog は chunking のためだけに存在する。plugin の `drain_output_reports` は since-last-drain を caller vec に append する契約で、runner は `report_backlog` に転記して先頭 `WIRE_MAX_EVENTS` のみ oldest-first に reply、残りは次回 drain。host `L1Instance::drain_output_reports` は chunk が `WIRE_MAX_EVENTS` ちょうどなら再要求・short reply で終了し、他 reply payload は `Err(Ipc)` で fail-closed。video slot は report verb を持たず `Unavailable` 維持（YAGNI）、capability は `Declared` 維持（実 plugin/実 DAW 未検証）(Issue #253)。

単一実装ストリームの根拠と chunk 意味論。wire 語彙 1 verb/reply 対とその 4 層配線（wire codec→runner→host proxy→composite→fixture E2E）は密結合のため分割しない。`WIRE_MAX_EVENTS`（=1024）を chunk サイズと decode bound の双方に使い、codec test の境界 assert で固定。host loop 終了条件は「chunk < `WIRE_MAX_EVENTS`」で、異常 reply は `Err(Ipc)` で fail-closed (Issue #253)。

## 未確定・保留事項

以下は素材が記録する gap・open question・未解決事項である。

- Issue #188 の残件（OS MIDI backend adapter・VST3 IEventList 翻訳・controller/param chase・note_expression/midi_sysex 語彙）は follow-up として記録されているが、後続 Issue での消化状況は対象文書からは不明である。
- Issue #629 の残受入（provisioned runner での実 provisioning・driver install・license 判断・wiring leg 実走 Pass・CI/runner 統合・AC7 consumer 実受入）は全て外部 gate であり、コードからは検証不能である。文書上「外部 gate/後続 slice」として残っており、現在も未完了かどうかは文書からは判断できない。
- `real-abi-test-plugin`↔`test-plugin` の serde 綴り差は既知 gap として記録されているが、移行判断は後続 slice 送りで未解決である。
- WASAPI output stream の `frame_position` 対応は Issue #565 の範囲外として明示的に後続 Issue 送りである。
- `midiOutMessage`+`DRV_QUERYDEVICEINTERFACE` の crash により output 側 stable ID 取得は断念されており、SetupAPI 等の別手段か provisioning 環境での再検証が必要である (Issue #629)。
- `DeviceError` に `InvalidArgument` 相当 variant がなく、`Backend`/`UnsupportedConfig` も厳密には caller-contract 違反を表さないため、shared `DeviceError`（lease 外・影響大）への variant 追加は行わず写像の限界を記録に残した (Issue #629)。
- `MidiInput::read` が `Err` と stats を同時に返せない契約上、audio 側「error callback + data callback」併存観測は失敗 read と stats 付き成功 read に分解される。この分解の妥当性は Issue close 判定で問われ得るリスクとして記録された (Issue #707)。
- capability matrix は ❓/⚠️ 項目と Open Questions #1-#7 を実測待ちとして明記し、採用 endpoint の確定は provisioning 実測後の ADR/decision として後続 slice に送られた (Issue #629)。
- `docs/plans`・`docs/tasks` 配下の文書は今回の対象に含まれず未読であり、タスク文書側にのみ記載された決定・却下理由があれば本書に未反映の可能性がある。exp/*.jsonl イベントログの中身も未読（対応する exp .md の要約のみを根拠にした）。
