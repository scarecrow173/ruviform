# Issue #628 — 受入条件（AC）棚卸し表

- Issue: https://github.com/scarecrow173/ruviform/issues/628
- 作成: close-out slice（`feat/628-close-out`）、2026-11-15
- 判定語彙: POLICIES.md の `Required/Deferred/NotApplicable` ×
  `Pass/Fail/NotRun/Unsupported`。`environment_kind` は
  `deterministic-backend`（OS-L1）のみ — 実 OS/GUI/plugin ABI の
  成功は一切主張しない。
- 根拠列の test 名は `crates/ruviform-audio-test/tests/` 以下、
  PR 番号は merge 済み slice のもの。

## Slice 対応関係

| Slice | PR | merge commit | 範囲 |
| --- | --- | --- | --- |
| 1st deterministic core | `scarecrow173/ruviform#638` | — | VirtualClock・PCM fixture・MIDI trace・oracle・RunMetadata（typed） |
| 2nd scripted backend | `scarecrow173/ruviform#659` | `98fd430` | port-trait backend・fault injector・manual pump |
| 3rd pipeline wiring | `scarecrow173/ruviform#664` | `3566519` | `Session::open`/typed runtime/`InputStage` への配線 |
| close-out（本 slice） | 本 PR | — | RunMetadata JSON・`Cancellation` fault・docs 修正・本棚卸し |

## AC 棚卸し

| # | 受入条件（Issue 転記） | verdict | 理由 | 根拠 |
| --- | --- | --- | --- | --- |
| 1 | 同一 Scenario seed+version+clock 設定から 3OS で安定した PCM/MIDI 期待 Trace を生成でき、再現に使う Run metadata を保存する | **Pass** | deterministic 生成と metadata 保存は OS-L1 で実装・検証済み（close-out slice で JSON save/load 追加）。「3OS で安定」は merge 後 SHA `554de77f` の ci run `43807854662`（workflow_dispatch・rerun 後全 job green）で verify×3OS・l1-real-verify×3OS が実走 Pass — 3OS matrix 証拠充足（2026-11-16 簿記更新） | `tests/clock.rs`（決定論的 clock）・`tests/pcm.rs`・`tests/midi.rs`（`playthrough_trace_matches_independent_oracle` 等）・`tests/metadata.rs`（`json_string_round_trip_preserves_all_fields`・`json_file_save_load_round_trip`・`json_wire_uses_case_schema_v1_kebab_case`）。CI run `43807854662` @ `554de77f`（初回 attempt は l1-real-verify windows で note_off release tail の boundary fail → rerun で全 platform green・flake 記録） |
| 2 | Audio Input→本番 Recording、Output→本番 Playback、MIDI→本番 Timestamp/Host adapter を既存契約のまま接続し、Fake-only 分岐を製品処理へ侵入させない | **Pass**（OS-L1 契約範囲） | output leg は `Session::open`+`GraphPlan`（production consumer 契約）、input leg は typed runtime（`RtRuntime::process`）、MIDI leg は `MidiInputEnumerator`→`InputStage::collect_block` に加えて **MIDI→Host adapter `attach()` 配線も PR `scarecrow173/ruviform#676`（merge `de3ed4e8`・#630 runtime-integration slice 内）で main に回収済み** — 両 fixture e2e suite の attach_integration mod が `ruviform_audio_plugin_bridge::attach`→`PluginNode`→typed `GraphRuntime` を audio-test scripted MIDI backend で駆動し、ci run `43807854662` の l1-real-verify lane で 3OS×2format 実走 Pass。test-only crate が production 契約を消費する一方向依存で fake 分岐は製品側に存在しない。実 endpoint（仮想 MIDI pair）は `scarecrow173/ruviform#629`/`#630` の EXT-L3 領域で引き続き NotRun（2026-11-16 簿記更新） | `tests/pipeline_output.rs`・`tests/pipeline_input.rs`・`tests/pipeline_midi.rs`（PR `scarecrow173/ruviform#664`）。attach_integration e2e（PR `scarecrow173/ruviform#676`・merge `de3ed4e8`）。`attach()` 実 endpoint は `scarecrow173/ruviform#629`/`#630` 領域 |
| 3 | Format/Channel/Buffer/Clock、loss/xrun/disconnect/reconnect/permission/queue-full/RT 境界を正負 Test で検証し、Fault と Expected/Observed が一致する | **Pass**（deterministic-backend OS-L1 範囲） | 全 fault class を正負 test で網羅: format/buffer negotiation（`UnsupportedConfig`）、frame-position jump、capture suppression、gap→Xrun（loss 可視化）、vanish/invalidate→reconnect、open/start/stop fault、permission denial（`Backend("permission_denied")`/`DeviceBusy`）、queue-full（`Backend("queue_full")`/`Xrun`）、RT 境界（`rt_` alloc-free）。本 slice で `Cancellation` を追加し audio 両方向・mid-stream inject・負例を検証。実デバイス上の xrun/disconnect 実 timing は EXT-L3 で NotRun | `tests/backend.rs`・`tests/backend_faults.rs`（`cancellation_aborts_{output,input}_delivery_and_terminates`・`inject_cancellation_mid_stream_*`・`backend_cancelled_error_alone_does_not_terminate_stream`・`cancellation_with_invalidate_reports_ordered_errors`）・`tests/pipeline_faults` 相当（`tests/pipeline_*.rs` 内 fault 伝播）。実 HW/実 OS の fault 実 timing は未検証 |
| 4 | RT 対象 Callback で allocation/lock/wait/IO の禁止を検証し、通常 OS-L1 の 3OS Native Build/Link/Headless・必要 Feature のテストを既存 CI に統合 | **Pass** | `rt_` prefix + `assert_no_alloc` で clock-read/PCM fill/MIDI trace/scripted backend pump/pipeline 3 leg の alloc-free を検証済み、`mise run test-rt` が実際に収集・実行（rt_contract.rs 9 tests）。新規テストは既存 `test`/`test-rt` lane に自動収集されるため新規 lane 不要 — 設計判断として明記。「3OS Native Build/Link」は merge 後 SHA `554de77f` の ci run `43807854662` で verify（windows/ubuntu/macos）・rt-verify 全 green — 3OS matrix 証拠充足（2026-11-16 簿記更新） | `tests/rt_contract.rs`（`rt_virtual_clock_reads_do_not_alloc`・`rt_scripted_backend_{output,input}_pump_does_not_alloc`・`rt_pipeline_*`）。ci run `43807854662` @ `554de77f`・`evidence/rf628/verification.md` ほか各 slice verification 記録 |
| 5 | 独立 PCM 数値/Byte Oracle・MIDI Trace Oracle を独立 Fixture で検証し、Ruviform 本番処理と自己参照の正解値共用を避ける | **Pass** | `oracle` module は `pcm_reference_block`/`midi_expected_playthrough` を独立実装で生成し、被試験コード（fixture generator/production scheduler）の出力を期待値に使わない。PCM は silence/impulse/ramp/square/noise/constant=Exact、sine/sweep=libm+tolerance。MIDI oracle は play-through 期待列と receipt/fault ledger を独立 fixture で照合 | `tests/pcm.rs`（oracle import・Exact/tolerance 比較）・`tests/midi.rs`（`playthrough_trace_matches_independent_oracle`・seek/loop/pause/unrepresentable 系）・`src/oracle.rs` |
| 6 | AudioLens f64/長尺/解析非回帰と Synvelis PCM/Project/RT/Recording/MIDI の独立 Oracle（双方の resolved Ruviform SHA）を、同じ Fixture version で実行し Pass/Fail/NotRun を別集計 | **Partial** | consumer oracle 自体は AudioLens/Synvelis 両 repo で merge 済み（task 前提）。ただし「双方の resolved Ruviform SHA で同一 fixture version 実行・Pass/Fail/NotRun 別集計」の証拠は consumer repo 側の責務であり、本 repo の evidence からは直接検証不能 = Deferred 記録 | consumer repo の merge 済み oracle（本 repo 範囲外）。本 repo 側の供給物は fixture version・RunMetadata JSON・deterministic trace 契約 |
| 7 | 本 Harness の公開必要 API・内部 Test-only API、製品配布除外・軽量ローカルテスト・Daily Full CI レーンの範囲を明文化 | **Pass** | crate README・`src/lib.rs` module doc・`Cargo.toml` description/依存理由コメント・docs chain で明文化。`publish = false`（workspace 継承）・production crate からの依存なし（一方向: audio-test→production contracts）・`assert_no_alloc` は dev-dep のみ・daily/full lane は既存 CI で変更不要 | `crates/ruviform-audio-test/README.md`（Status: test-only・Non-responsibilities・Verification levels）・`src/lib.rs`・`Cargo.toml`・docs chain（`docs/{specs,designs,plans,tasks,test-specs}/issue-628-close-out.md`） |

## 本 slice で正直に Deferred / NotRun のまま残る項目

| 項目 | 状態 | 理由・所有 |
| --- | --- | --- |
| MIDI→Host adapter `attach()` 実 endpoint 接続（仮想 MIDI pair 実走） | NotRun | attach() 配線自体は PR `scarecrow173/ruviform#676`（merge `de3ed4e8`）で回収済み。残るのは仮想 endpoint 実走で `scarecrow173/ruviform#629` 領域 |
| MIDI 側 fault への cancellation 拡張 | Deferred | 本 slice は audio stream のみ。`backend/midi.rs` の `MidiFeed`/`MockPort` fault 体系は別設計 |
| 実 OS/実デバイス・実 thread timing・xrun・duplex・latency 実走 | NotRun | EXT-L3。Windows 仮想 endpoint は `scarecrow173/ruviform#629` 領域 |
| real VST3/CLAP ABI plugin 経路 | NotRun | EXT-L3。`ruviform-plugin-test-{vst3,clap}` は別 Issue（#630 系）の管轄 |
| 3OS 横断 Native Build/Link/Headless（対象 SHA 追従） | Pass @ `554de77f` | merge 後 SHA `554de77f` の ci run `43807854662`（rerun 後全 job green）で充足済み。次の merge 後 SHA では daily/手動 Full CI が新規証拠を供給する継続 gate |
| `*-msrv`・`*-full` 系検証 | NotRun | daily/手動 Full CI 管轄（POLICIES.md §5・task 指定） |
| AC6 consumer 別集計証拠 | Deferred | consumer repo（AudioLens/Synvelis）側の検証成果物。本 repo からは Pass を主張しない |

## 検証実行記録

close-out slice の canonical 検証結果は
`evidence/rf628/close-out-verification.md` を参照
（`mise run` 経由・実 exit code 記録）。
