---
id: "EVI-6ZwTJLLh6cmDuzD2Tii0Z5"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "オーディオデバイス・MIDI の検証記録"
created: "2026-11-20"
updated: "2026-11-24"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/188", "https://github.com/scarecrow173/ruviform/issues/251", "https://github.com/scarecrow173/ruviform/issues/253", "https://github.com/scarecrow173/ruviform/issues/565", "https://github.com/scarecrow173/ruviform/issues/629", "https://github.com/scarecrow173/ruviform/issues/707"]
  informs: []
  explains: []
  supersedes: []
---

# オーディオデバイス・MIDI の検証記録

この文書は、オーディオデバイスと MIDI の backend に関する検証・実行結果を記録したものである。内容は後に削除された過去の実装記録から抽出した証拠に基づき、この文書はそれらなしで読めるよう自己完結させている。各事実には由来する Issue 番号を付記する。実測値や観測された動作は事実として記し、記録上の評価や制限はその旨を明示して区別する。

## 実機エンドポイント probe の結果

Windows 実機でエンドポイントを列挙する probe を実行した結果、仮想 endpoint が未 provisioning の環境では検証が fail-closed に NotRun を返すことが確認された (Issue #629)。

1st slice では、Windows 11 10.0.26300.9974 の実機で endpoint_probe が exit 0 で終了し、61 endpoint を列挙した。virtual candidate は 0 件で、判定は NOT_PROVISIONED だった。併せて実行した midi_endpoint_probe も exit 0 で、input 0・output 1 port、loopback candidate 0、NOT_PROVISIONED を返した。probe の出力はテキストファイルとして保存された (Issue #629)。

4th slice では、Windows 11 26300（Audient iD4 と NVIDIA HDA、仮想 endpoint 未 provisioning）の実機で probe を再実行し、65 endpoint の列挙、port view 5 output・2 input、pair candidates none—NOT_PROVISIONED を観測した。NVIDIA Virtual Audio Device は 'virtual audio' の hint に一致したが catalog:absent であり、記録上これは情報記録であって証明ではない。権限は ConsentStore の microphone=Allow・NonPackaged=Allow・DeviceAccess\Global\{guid}=absent を確認した (Issue #629)。

verifier の直接実行は verdict: NotRun—NOT_PROVISIONED で exit 0 だった。oracle 閾値は provisional で、Pass 経路（open→stream→oracle→lifecycle）は未実走のまま provisioned runner 待ちと記録された。canonical 検証は全て Pass だった (Issue #629)。

## frame_position とクロックドメインの実機検証

Windows 11 + Audient iD4 の実機で、入力ストリームの frame_position 供給と duplex のクロックドメイン一致を検証した (Issue #565)。

real_input_e2e（--features real-device, --ignored）を実行すると 2 tests が Pass した。real_input_stream_captures_planar_audio では callbacks=194、frame_position some=194・gaps=0・first=0・last=85113、capture timestamp some=194、clock discontinuities 0、clock domain 一致を観測した。real_duplex_streams_agree_on_clock_domain では input/output の clock domain が一致した（ClockDomainId(2)）。lib test 75 件も Pass し、clippy warning は 0 だった (Issue #565)。

別途、一時的な example による直接 probe（検証後に削除）でも、pinned/default endpoint の双方で全 callback に Some が付き、441 frames/callback で単調に進行し、DATA_DISCONTINUITY flag から ClockDiscontinuity が発火することを確認した (Issue #565)。

## オーディオ経路の E2E と canonical 検証

codec・runner・proxy・graph を横断する E2E と canonical 検証が全て Pass した (Issue #253)。

codec unit 51 tests、runner E2E（実 runner child の全 semantics）、proxy E2E 3 件（pull/chunk/failed）、graph E2E（endpoint attach 中の L1 drain）が全て Pass した。canonical 検証は fmt-check・check・lint・test・test-rt・test-plugin-core・lint-plugin-core・lint-l1-real-plugin・test-l1-real-plugin・verify-ci が全て exit 0 だった (Issue #253)。

初回 test では無関係な flake が 2 件（spawn/pipeline timeout）出たが、単独・全量の再実行で PASS し、変更起因でないことを確認した。reply tag 8 は旧 unknown-tag probe だったため、probe を 9 へ移動した (Issue #253)。

## MIDI 出力 backend の検証

MIDI 出力 backend の canonical 検証は全 lane で Pass し、実機 smoke は環境 gate により NotRun と記録された (Issue #629)。

MIDI output slice では、fmt-check・check・lint・test（audio-core 64 unit + 37 doc + 2 compile-fail、midi-device 24 unit + integration）・test-rt・prek が全て exit 0 だった。test-midi-real-device lane と MSRV lane は指定どおり NotRun と記録された。mock は sent_events/sent_sysex の順序記録と、fail_next_open/fail_next_send の one-shot 故障注入を提供する (Issue #629)。

2nd slice でも Windows host（mise 管理 toolchain）で canonical lane が全て Pass した。ruviform-audio-core は 64 unit + 37 doc + 2 compile-fail、ruviform-midi-device は unit 24 + output_ports 2 + ports 2 + doctest 1 を計測し、test-rt・prek も Pass した。#[ignore] の実デバイス send smoke は test-midi-real-device lane が自動収集するため mise 設定の変更は不要で、本 slice では NotRun と記録された。この slice の environment_kind 証拠は OS-L1 のみで、virtual-os-endpoint や実 OS L3 の Pass は主張していない (Issue #629)。

実機での出力確認として、test-midi-real-device lane の再実行で real_device_silent_output_smoke が Pass した。GS Wavetable Synth の実 WinMM port へ非可聴の PC/CC/NoteOff が実際に WinMM を通過した（verdict: Pass — 仮想証明ではない）。send_sysex は fallback 経路で Unsupported degrade となることを実機で確認した (Issue #629)。

port discovery では、production backend の input_ports() が WinMM の input-only を反映し、MidiPortOrigin が Unknown であることが判明した。このため loopback の send→receive 検証には midiOut* 送信経路の adapter 追加が必要と記録された（discovery gap）(Issue #629)。

## MIDI ノード配線の実装記録

MIDI ノード配線は review-protocol を経て全指摘が修正され、最終検証が green になった (Issue #188)。

14 件の指摘に対し、Reviewer A/B → Validator（13/14 confirmed）→ Judge BLOCKED（P1: V1/V2/V4/V6/V7 と blocking-P2 の V5）を経て全件修正された。最終的に fmt・check・clippy -D warnings・test・test-rt が全て green で、rt_midi_wiring_no_alloc・rt_device_restart_no_alloc を含んだ。新規 test は midi_wiring の 14 本と chase 単体 7 本のほかで構成された (Issue #188)。

PoC 完了時点の未達も記録された。VST3 経路で MIDI channel voice が届かない（4 cap 全て Unsupported）、実 OS MIDI backend が存在せず port 契約のみ、InputStage は PluginNode call path 未結合、audio-lens への影響は未検証の暫定評価、という制限である。これらの未達項目は後続の検証対象として記録された (Issue #188)。

## loopback 検証で発見した latent defect

loopback 検証では、実機 lane の初回実行で latent defect を発見し、修正後に PASS へ至った (Issue #629)。

test-midi-real-device lane の初回実行は FAILED（exit 101）となった。原因は midiOutOpen(CALLBACK_FUNCTION|MIDI_IO_STATUS) が GS Wavetable Synth で MMSYSERR_INVALFLAG（rc=10）を返したことである。PowerShell の P/Invoke による切り分けで、CALLBACK_FUNCTION 単独は rc=0 で成功し、MIDI_IO_STATUS flag を含めると rc=10 になることを実測した。この defect は、2nd slice がこの lane を実行せず NotRun と記録していたため潜在化していた (Issue #629)。

修正は coordinator 承認（Option A）の scope 拡張のもとで windows_output.rs に対して行われ、再実行は PASS（exit 0）だった。lib smoke・loopback_round_trip（NotRun—NOT_PROVISIONED）・real_device_silent_output_smoke（GS Wavetable へ非可聴 send が実 WinMM を通過、verdict: Pass）・output_ports/ports smoke が全て ok だった。send_sysex は fallback 経路で Unsupported degrade を実機確認した (Issue #629)。

## 仮想 endpoint lane の fail-closed NotRun

仮想 endpoint を対象とする各 lane は、未 provisioning の環境で fail-closed に NotRun を返す挙動が複数 slice で記録された (Issue #629)。

test-midi-virtual-endpoint lane の self-run は exit 0・2 passed で、loopback_round_trip が verdict: NotRun—NOT_PROVISIONED（0 input ports・1 output port、fail-closed）、real_device_silent_output_smoke が verdict: Pass（GS Wavetable Synth の実 WinMM port — 仮想証明ではない）を確認した。退行確認として test-midi-real-device は exit 0（全 #[ignore] ok・NotRun verdict のまま）、test-audio-virtual-endpoint は exit 0（1 passed）だった。記録上、Pass・virtual-os-endpoint・実 OS L3 実走 Pass は主張しないと明記された (Issue #629)。

provisioning runbook slice でも同じ 3 lane の self-run を fail-closed で記録した。test-midi-virtual-endpoint は exit 0・2 passed で、loopback_round_trip は verdict: NotRun NOT_PROVISIONED（0in/1out）、real_device_silent_output_smoke は verdict: Pass on GS Wavetable（仮想証明ではない）。test-midi-real-device の退行確認は exit 0 で全 ignored ok、test-audio-virtual-endpoint は exit 0・1 passed だった。canonical 検証は fmt-check・check・lint・test・prek が全て Pass で、MSRV は NotRun だった (Issue #629)。

test-os-endpoint-wiring lane の self-run では、mise run が exit 0 で #[ignore] leg 2 件とも ok（2 passed・3 filtered out。deterministic legs は lane 対象外であることを確認）だった。audio leg は observed:"NOT_PROVISIONED — no render↔capture virtual name-stem pair"、MIDI leg は observed:"NOT_PROVISIONED — no bidirectional name match" を出力した。cargo test --test os_endpoint -- --ignored --nocapture および cargo test -p ruviform-audio-test --test os_endpoint -- --ignored --nocapture の直接実行では deterministic legs 3 件が passed し、audio leg は observed:"no input devices on this host"、MIDI leg は observed:"NOT_PROVISIONED — no bidirectional name match" だった。いずれも run_metadata JSON（result:"not-run"、level:"ext-l3"、environment_kind:"virtual-os-endpoint"）と verdict: NotRun を出力して exit 0 した。audio leg の理由文字列は host の input device 列挙状態に依存し、NOT_PROVISIONED と no input devices の両方が観測済みで、いずれも同じ fail-closed NotRun 系である。既存 lane の退行確認も全て exit 0 で挙動不変だった (Issue #629)。

test-audio-virtual-endpoint の self-run は mise run で exit 0・1 passed だった。canonical run に --nocapture は含まれないため verdict 行は libtest capture 内にあり、--nocapture 補助実行で verdict: NotRun—no input devices on this host を確認した。4th slice 時点の NOT_PROVISIONED とは理由文字列が異なるが、同じ fail-closed NotRun 系である。endpoint_probe の再実行も exit 0 で provisioning: NOT_PROVISIONED を維持し、metadata tests 11 件全 pass（新規 schema 語彙 pin テストを含む）、lint-audio-real-input も Pass だった (Issue #629)。

環境 gate の設計として、verdict は常に仮想 endpoint case を NotRun とし、virtual-os-endpoint evidence を主張しない文言を埋め込む。2nd slice では environment_kind 証拠が OS-L1 のみであることを確認し、virtual-os-endpoint や実 OS L3 の Pass は主張していない (Issue #629)。

## MIDI 故障注入

MIDI backend の故障注入は、mock による決定的な再現と、実 device での timing 検証の未実施に分かれる。

MIDI cancellation の fault slice として、正負 6 test と mock の unit test 4 件を追加した。cancelled read が Backend("cancelled") と subscription 終了を再現し、非終端の Backend("cancelled")・inject_dropped・inject_unsupported が終了しない負例も pass した。canonical 検証は fmt-check・check/lint/test × ruviform-audio-core・ruviform-audio-test・test-rt・prek が全て exit 0 で、git merge origin/main は Already up to date だった (Issue #707)。

実 MIDI device/endpoint の fault 実 timing は EXT-L3 で NotRun と記録された（#629/#630 領域）。Issue close 判定で「Err vs stats 分解」の妥当性が問われる可能性が Risks に記録済みである (Issue #707)。

## Issue #251 OS-L1 CI 証拠の再照合（2026-11-24）

Issue #251 の OS-L1 受入（3 OS での Build/Link・Mock/headless/RT 回帰の対象 SHA 照合）の証拠を、候補 SHA `fc355d5c`（収集時点の main HEAD）で再照合した。対象 SHA 自体の completed CI run は存在せず（`ci.yml` は workflow_dispatch 専用）、対象 SHA の統合検証は NotRun（Pending）のまま正直に記録した (Issue #251)。

補助証拠として、audio 関連 crate（`ruviform-audio-device`/`ruviform-audio-core`/`ruviform-audio`）の差分がゼロである直近の completed run `45248600736` @ `a0e34564`（workflow_dispatch・`expected_sha` 一致・ci-summary verdict: PASS・全 23 job success・2026-11-23）を採用した。job log 実測では ruviform-audio-device の lib unittests が ubuntu/macos で 59・windows で 75（`wasapi_input` 16 件含む）passed、`mock_backend` 4・`mock_input_backend` 12・Doc-tests 6・rt_ contract 2+2 (filtered) で、いずれも前回照合（run 43428172820 @ `e93c438a`・PR #604）と同じ回帰内容が同じコードに対して確認された (Issue #251)。

#629 由来の仮想 endpoint lane はこの run でも実行されなかった。`ruviform-audio-device/tests/virtual_endpoint.rs` は `real-device` feature gate で 0 tests compile、`ruviform-audio-test/tests/os_endpoint.rs` の `audio_virtual_endpoint_round_trip` は全 3 OS で `#[ignore]` の ignored 表示だった。EXT-L3 実走・独立 Oracle 判定は引き続き provisioned runner 待ちの gate（Issue #629・blockedBy #762）で、NotRun—NOT_PROVISIONED 系の fail-closed 記録を維持する (Issue #629, Issue #251)。

対象 SHA・採用 run・job 証拠・OS×Backend×Device×Case の最新 matrix と判定語彙の正本は `evidence/rf251-audio-input/report.md` を参照 (Issue #251)。

## 記録の範囲と限界

この文書が記録する検証には、実行されなかった項目と外部 gate 待ちの項目がある。

test-midi-real-device lane と MSRV lane が指定どおり NotRun と記録された slice がある。WASAPI slice の Pass 経路（open→stream→oracle→lifecycle）は oracle 閾値が provisional のため未実走で、provisioned runner 待ちである。loopback_round_trip は 0 input ports・1 output port の環境で NOT_PROVISIONED のまま NotRun だった。#[ignore] の実デバイス send smoke は test-midi-real-device lane が自動収集するため、2nd slice では NotRun と記録された (Issue #629)。

残る受入（provisioned runner での実 provisioning・driver install・license 判断・実走 Pass・CI 統合・consumer 実受入）は外部 gate/後続 slice として残っており、完了状況は記録からは判断できない。実 MIDI device/endpoint の fault 実 timing は EXT-L3 で NotRun である (Issue #629, Issue #707)。

Issue #188 の PoC では、VST3 経路の MIDI channel voice 未達・実 OS MIDI backend 不在・InputStage 未結合・audio-lens 影響未検証が未達として記録された。Issue #565 の範囲外として、WASAPI output stream の frame_position 対応は後続 Issue 送りとされた (Issue #188, Issue #565)。

本記録全体では、environment_kind 証拠は OS-L1 に限られ、virtual-os-endpoint や実 OS L3 での実走 Pass はどの slice でも主張されていない (Issue #629)。
