# OBS-06 Consumer 証跡棚卸し

2026-11-28 時点の [Ruviform #281][rf281] の部分記録。既存 CI のジョブ・ログ・artifact、各 Consumer の `Cargo.lock` と既存証跡を照合した。製品試験の新規実行、CI の手動起動、実端末・仮想デバイス・プロビジョニング runner の使用は行っていない。

## 照合対象

以下は棚卸し時点の各 `main` であり、同じ Ruviform を組み込んだ受入用 tuple ではない。Consumer の resolved SHA は、各コミットの `Cargo.lock` にある Ruviform Git source の `#` 以降を照合した値。

| ID | リポジトリと対象 SHA | Ruviform resolved SHA | 対象 SHA の実行証拠 |
| --- | --- | --- | --- |
| RF | `scarecrow173/ruviform` `ddd27b255862e6ec1659c620ac49526159796b0b` | 同左 | Actions の `head_sha` 完全一致検索で run なし |
| AL | `scarecrow173/audio-lens` `bd7bda931a9080b0e6233005016358f919922b11` | [Cargo.lock][al-lock] の 25 source が `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` | 同検索で run なし。RF と不一致 |
| SV | `scarecrow173/synvelis` `bf0442cfad73c5102d137f3f1e20ac74d84860b3` | [Cargo.lock][sv-lock] の 36 source が `511e1a7944c35797942ba065c092b9263067c48a` | 同検索で run なし。RF と不一致 |

以降の Case 表は上記 ID の repo/SHA/resolved SHA を参照する。`NotRun` はこの対象で該当 Case の実走証拠を確認できないことを示す。参照欄の旧 SHA の `Pass` / `Fail` は元の実行にだけ帰属する。数値の exit がログに明示されない成功 step は `0（step success）`、未実行は exit なしとする。

[POLICIES.md](../../POLICIES.md) §2–5 に従い、要求と結果は別軸とする。3OS の OS-L1 は `Required`。Windows の GUI-L2 / EXT-L3 は製品で利用する能力に限り `Required` で、`NotRun` / `Fail` の間はその機能を受け入れない。macOS/Linux の GUI-L2 / EXT-L3 は利用要件が生じるまで `Deferred / NotRun`。`NotApplicable` は対象機能がその境界を使わない場合、`Unsupported` は技術的非対応が確認された場合に限る。

## Ruviform 共通契約と OS-L1

OBS-01〜11 の契約・試験名は [OBS 受入台帳](../rf275/obs-acceptance-ledger.md) にある。[PR 521][pr521] は設計正本、[PR 547][pr547] は実装照合と Collector の drain 競合試験の参照先であり、いずれも Consumer 受入証拠ではない。

[Daily 45935355893][rf-daily] の対象 `7c26d01c6b228b11652aec12c44855208cab086a` では、[Full CI 45935377401][rf-ci] の以下の各 job が同じ SHA を checkout した。`mise run check-observability`、`mise run lint-observability`、`mise run test-observability` はそれぞれ 6 組合せを実行した（default/core、`logging`、`tracing-export`、`allocation-probe`、`integration`、all features）。Collector / Diagnostics は独立 feature ではなく core 側、Allocator の計測は `allocation-probe`。ここでの native link は試験バイナリの link と実行を指し、Consumer 製品の link を含めない。

| 既存 job・OS | Runner image / build | 結果（上記旧 SHA） |
| --- | --- | --- |
| [observability-verify / Windows][rf-win] | Server 2025 x64 `10.0.26100`、`20261116.315.1` | 3 task `Pass`、各 exit 0（step success） |
| [observability-verify / Linux][rf-linux] | Ubuntu 26.04 x64、`20261116.88.1` | 同上 |
| [observability-verify-macos][rf-mac] | macOS 26.7.2 arm64 `25G332`、`20261117.0516.1` | 同上 |
| [rt-verify / Linux][rf-rt] | Ubuntu 26.04 x64、`20261116.202.1` | `mise run test-rt` `Pass`、exit 0（step success）。専用 job は Linux のみ |

Matrix ログには `rt_try_record_and_counters_allocate_nothing`、`abandoned_leases_racing_shutdown_still_drain_partial`、`best_effort_sink_failure_is_degraded_and_rest_works`、`scope_counts_allocations_on_the_owning_thread` がある。`integration` の `debug_report_declares_secret_non_exposure` は共通 Debug Report の契約であり、各製品の出力全体に機密が出ないことまでは示さない。

| Case | 対象・OS | 正規入口 | 対象結果・担当 |
| --- | --- | --- | --- |
| RF-G0 Native Build/Link・Feature | RF / Win・macOS・Linux | `mise run check-observability` / `mise run lint-observability` / `mise run test-observability` | `Required / NotRun`。上記は別 SHA。RF #281、既存 OS-L1 CI は [RF #598][rf598] |
| RF-RT / RF-PRIV 共通 Headless | RF / Win・macOS・Linux | `mise run test-observability`、Linux では加えて `mise run test-rt` | `Required / NotRun`。上記は別 SHA。RF #281、共通契約の不適合は受入台帳の OBS コンポーネント担当 |

## AudioLens: CLI・Logging・f64

担当は [AL #127][al127]。参照 **AL-CI** は [Full CI 45935543875][al-ci]（`audio-lens` `e163676ae3270ea64d10d9a8a502c642d6fc20e9`、resolved RF は AL と同じ `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`）。[Windows][al-win]、[macOS][al-mac]、[Linux][al-linux] の `mise run test-full` / `mise run check-cli-headless` は exit 0（step success）。Windows Server 2025 x64 image `20261116.315.1`、macOS 26 arm64 image `20261117.0516.1`、Ubuntu 26.04 x64 image `20261116.202.1`。`mise run build-full` は同じ run の Windows/macOS で実行し、Linux はこの build step が skip（試験バイナリは `test-full` で link・実行）。[AL の pin 追従記録][al-pin] にも Windows の対象限定実走があるが、AL の main SHA に対する CI ではない。

| Case | 対象・OS | 正規入口と独立判定 | 既存実行証拠・対象結果 |
| --- | --- | --- | --- |
| AL-LINK | AL / 3OS L1 | `mise run check-full`、`mise run build-full`、`mise run test-full`、`mise run check-cli-headless`。CLI/解析を GUI 依存なしで確認 | AL-CI（Linux の単独 build は skip）。`Required / NotRun`（Consumer SHA 不一致） |
| AL-LOG-OFF / AL-CLI-STDOUT | AL / 3OS L1 | `mise run test al-cli --test observability`。既定 OFF でログファイルなし、明示 ON でも `inspect` stdout は JSON、ログは stderr/file | AL-CI の `logging_off_creates_no_log_files`、`inspect_logs_to_stderr_only_and_keeps_stdout_json` が `ok`。`Required / NotRun`（Consumer SHA 不一致） |
| AL-LOG-FAIL | AL / 3OS L1 | 同 CLI task。file sink 初期化失敗では警告と stderr に縮退して処理継続、無効オプションは exit 2 | AL-CI の `file_sink_init_failure_degrades_to_stderr` が `ok`（故障を検出した試験の成功）。`Required / NotRun`（Consumer SHA 不一致） |
| AL-PRIV | AL / 3OS L1 | 同 CLI task の `emitted_fields_stay_metadata_sized` は Stage 成功・失敗の field サイズを検査。機密文字列・パス・PCM の非露出は ON/OFF と故障時の sink/CLI/Export を別に照合 | AL-CI の field サイズ試験は `ok`。小さい field に機密が入らないことは未証明。機密漏出 Case の専用入口・対象証跡は未確認。`Required / NotRun` |
| AL-F64 / AL-LONG / AL-PREC-NEG | AL / 3OS L1 | `mise run test al-analysis --test rf628_oracle`。独立 PCM 期待値で f64/長尺を照合し、gap/NaN の有効性を別に検査。CLI/Dataset/Export の出力は `mise run test-full` の製品 Golden を別に照合 | AL-CI の `rf628_long_scenario_non_regression`、`rf628_gap_nan_validity` が `ok`。共通 Fixture tuple の実績は次節。`Required / NotRun`（Consumer SHA 不一致） |
| AL-OBS-NONINTERFERENCE | AL / 3OS L1 | CLI/解析の正規入口は上記。Logging/Tracing OFF・ON・sink 故障で同じ長尺 Fixture を処理し、f64/Export/CLI 結果と処理時間・メモリを比較する。DAW の RT callback 判定は適用しない | AL-CI では CLI OBS と長尺 f64 は別テスト。両者を組み合わせた正規入口と対象証跡は未確認。`Required / NotRun` |
| AL-GUI-OBS | AL / Windows L2 | AL `mise run test-ui-gpui-real` と RF #631。実 Window の入力・focus・画素と製品状態を確認。Logging/Tracing/Privacy、sink 故障後の解析継続と UI 非干渉の測定は別途必要 | [AL の実 GUI 証跡][al-real] は Windows 11 x64、AL `826dbf166c42eb70cb23b9038b9c544a44690495` / RF `cbcc83437616fdd7418da37183c6be66b6cfe225`、exit 0、7 Case `Pass`。対象 tuple で OBS を重ねた実測は `Required / NotRun` |
| AL-CLI-EXT | AL / Windows L3 | オフライン CLI/解析 Case には実 Audio/MIDI Endpoint や Plugin Host を要求しない | `NotApplicable / NotRun`（この Case に限定）。AL で別途提供する外部機能には個別に適用性を判断 |

## Synvelis: DAW・RT・Plugin・Project

担当は [SV #28][sv28]。参照 **SV-CI** は [Full CI 45935641612][sv-ci]（`synvelis` `2e32d871da081463e04571fa38c50d7170843dec`、resolved RF `0479fa62e77e669a7bf84320d3d5418ffb949fc6`）。`mise run test-full` は [Windows][sv-win] exit 1、[macOS][sv-mac] / [Linux][sv-linux] exit 101。3OS で `pc_c4_l1_{vst3,clap}_real_abi_binding` が失敗した。[Linux RT job][sv-rt] の `mise run test-rt` も `tim03_l1_{vst3,clap}_transport_application` で exit 101。ログは L1 runner バイナリ不在を報告しており、ABI や RT の実行結果そのものではない。Windows Server 2025 x64 image `20261116.315.1`、macOS 26 arm64 image `20261123.0526.1`、Ubuntu 26.04 x64 image `20261116.202.1`。

| Case | 対象・OS | 正規入口と独立判定 | 既存実行証拠・対象結果 |
| --- | --- | --- | --- |
| SV-LINK / SV-RT | SV / 3OS L1 | `mise run check-full`、`mise run build-app-native`、`mise run test-full`、`mise run test-rt`。callback の NoAlloc/NoBlock・予期しない drop なしは、OBS ON/OFF と故障下でも独立に観測 | SV-CI の 3OS workspace 試験と Linux RT は上記 `Fail`。別 SHA の runner 欠落を対象 SV の製品 `Fail` にしない。`Required / NotRun` |
| SV-PCM / SV-PROJECT | SV / 3OS L1 | `mise run test synvelis-recording --test rf628_oracle`、`mise run test synvelis-project`。録音 PCM と commit/save/reload を照合。Project/Undo 等の広い製品契約は別検査 | 共通 Fixture tuple の PCM と録音 Project の実績は次節。`Required / NotRun`（対象 tuple で未実走） |
| SV-FAULT / SV-RT-ALLOC | SV / 3OS L1 | 同録音 oracle の open/recovery、device vanish、位置/clock fault と verified prefix。`mise run test synvelis-recording --test rf628_oracle_rt_alloc` で scripted callback の計測窓を分離 | 次節のファイル単位 `Pass` は実 OS callback・NoBlock・OBS との負荷比較を含まない。`Required / NotRun`（対象 tuple で未実走） |
| SV-LOG / SV-SUB-FAIL | SV / 3OS L1 | `mise run test synvelis-app --test logging`。Host だけが global Subscriber を一度登録し、sink 故障時は残りの sink と処理が継続。Subscriber 障害の RT 非干渉は別 Case | SV-CI の `global_install_is_once_per_process`、`build_logging_reports_degraded_best_effort_sink` は 3OS で `ok`。二重登録/縮退の範囲を超えた障害は未確認。`Required / NotRun` |
| SV-GATEWAY / SV-PRIV | SV / 3OS L1 | `mise run gw10`、`mise run test synvelis-gateway`。Grant/Approval/Audit/Provenance が OBS に依存せず、拒否や Report 出力で PCM/パス/Prompt/Secret を露出しないことを別に照合 | SV-CI の 3OS `gw10-verify` 成功は Gateway の参照まで。OBS ON/OFF・故障時の機密非露出を照合する専用入口・対象証跡は未確認。`Required / NotRun` |
| SV-PLUGIN / SV-PLUGIN-FAIL | SV / 3OS L1 | `mise run test-pc-c4-l1`、`mise run test-tim03-l1`、`mise run test synvelis-app --test plugin_dock_probe`。Graph/Plugin の処理・状態・停止と、HostLost/lease/close 後の復旧を分ける | SV-CI の runner 欠落 `Fail` と [SV headless probe][sv-dock] の実バイナリ lifecycle `Pass`・editor 拒否 `Unsupported` は別実行。`Required / NotRun` |
| SV-GUI-OBS | SV / Windows L2 | SV `mise run run-app-native` は起動入口のみ。RF `mise run test-ui-real-os-scenario` / `mise run test-ui-plugin-dock-real` は共通 Runner。製品の実 Window/input/focus/画素・Logging/Privacy・障害後継続は [SV #114][sv114] / [SV #176][sv176] | [SV の実 GUI PR][sv-real] の `sv_os_real` は Windows x64、RF lock `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`、`mise exec -- cargo test -p synvelis-app --features real-os-scenario --test sv_os_real --locked -- --nocapture` exit 0。JSONL に SV 実行 SHA なし。5 Case の実窓/focus/geometry は実 OS pointer/pixel・OBS の証拠ではない。対象は `Required / NotRun` |
| SV-DEV-OBS | SV / Windows L3 | SV `mise run test-recording-virtual-endpoint`、RF `mise run test-os-endpoint-wiring`。本番 WASAPI/WinMM を通す利用能力で callback/timestamp/PCM/recovery と OBS 非干渉を測る | [SV #115 の既存 WASAPI run][sv-device] は別 tuple で `Fail`（`bff8114ed5056be82b6b3ce370c2141bf2e355d1` / RF `0479fa62e77e669a7bf84320d3d5418ffb949fc6`、Server 2025 x64 `10.0.26100`、harness exit 101）。対象の Audio/MIDI 利用能力は `Required / NotRun` |
| SV-ABI-OBS / SV-NATIVE-DOCK | SV / Windows L3（Editor の実画面は L2 とも照合） | RF `mise run test-plugin-abi-validator` / `mise run test-l1-real-plugin` と SV `mise run test synvelis-app --test plugin_dock_probe` を補助に、実 ABI の提供形式を本番 Host で検査。実 Native Dock は実 OS parent、画素・Resize・Focus・Crash/Recovery と RT/Privacy が必要 | [SV headless probe][sv-dock] は Windows x64、SV `90b2ab3439c36a5c9eb2170e394394fb77164e5e` / RF `20dc41f71c47674b3e51d1eb0921124b94f8bfaf`、exit 0。VST3/CLAP Load は実バイナリ、`dock-pixels` は `NotRun`。対象の提供形式・Dock は `Required / NotRun` |

## 共通 Fixture と過去の candidate tuple

[RF #294 の棚卸し](../rf294/ac-inventory.md) の完了は集約の実績であり、OBS-06 の製品受入ではない。既存 run の `Pass` / `Fail` の範囲を次のように区別する。

| 既存 run | RF candidate / 実行時の Consumer | 証跡の範囲 |
| --- | --- | --- |
| [42937524742][tuple-v1] | RF `9865cd57491f3f39061212a85f81f9ebce731bde`、AL `9c25df9958a54c1fed85280bcef6fa63a4ab7372`、SV `fb069c93426599a7b35c3affaafe13d1c1b55964` | [保存された artifact](../rf288/io06-gate-matrix.md)。`shaMatch=true`、AL と SV を別集計。GUI/OS/Native Dock に `Unsupported` / `NotRun` が残る |
| [43907984482][tuple-oracle] | RF `615cf159778aacc577e644166756e1d143c685fb`、AL `7afabbc4af62900774220f8de734354c0c58a0b8`、SV `5795b668e1d7296744a58a436b4f8cc547a23b39` | attempt 1 の両 artifact の `resolved_ruviform_sha` は candidate と一致。Consumer の `expectedSha` / `shaMatch` は `null`（指定 SHA の事前照合は主張しない）。以下の OS-L1 oracle が実走 |
| [44017688605][tuple-v2] | RF `be1acb235e40bf6220d24a98d83786ca0b3e134e`、AL `7afabbc4af62900774220f8de734354c0c58a0b8`、SV `acc0198c3532f7c5d4cbf3dc18211de3c5735be3` | tuple/v2 の両 artifact に同じ resolved RF と `ruviform-audio-test-0.1.0@be1acb235e40bf6220d24a98d83786ca0b3e134e`。同じ三つの Ubuntu oracle lane が `Pass`。一方 [44013452606][tuple-blocked] は別候補 `7124f0b2` で両 preflight が `Fail`（Consumer resolved SHA なし、製品 Case は未実行） |

run `43907984482` の三つの Case の共通 Fixture version は `ruviform-audio-test-0.1.0@615cf159778aacc577e644166756e1d143c685fb`、環境は `deterministic-backend`。SHA は Fixture ソースの revision であり、生成 PCM の artifact hash ではない。`consumer-evidence-{audiolens,synvelis}-attempt-1` の Case 行では `expected` / `observed` が `null`。テスト名と結果はログで照合できるが、個別の測定値や入力 digest は揃っていない。

| Case ID | 既存 job・OS | 正規入口 | 観測された結果 |
| --- | --- | --- | --- |
| `audiolens-rf628-oracle` | [verify-audiolens][tuple-al]、Ubuntu 26.04.1 x64 image `20261111.82.6` | `mise run test al-analysis --test rf628_oracle` | `Pass`、exit 0（step success）、7 tests。f64/長尺/gap/NaN。CLI の Logging/Privacy はこの Case に含まれない |
| `synvelis-rf628-oracle` | [verify-synvelis][tuple-sv]、Ubuntu 26.04.1 x64 image `20261111.196.1` | `mise run test synvelis-recording --test rf628_oracle` | `Pass`、exit 0（step success）、11 tests。録音 PCM/commit/reload と fault/recovery。Plugin/GUI/Gateway はこの Case に含まれない |
| `synvelis-rf628-oracle-rt-alloc` | 同 SV job / OS | `mise run test synvelis-recording --test rf628_oracle_rt_alloc` | `Pass`、exit 0（step success）、1 test。scripted callback の alloc/drop の検査であり、実 Device での性能測定ではない |

Windows で再照合する際は、四つの共通基盤の証拠を混ぜない。

| RF Owner・基盤 | 既存証拠 | OBS-06 での照合単位 |
| --- | --- | --- |
| [#628][rf628] VirtualClock/PCM/Fault | [Fixture 棚卸し](../rf628/ac-inventory.md) と上記 Ubuntu oracle | OS-L1。Case/version、seed、Clock 設定、入力 hash と期待/観測値を AL と SV で別記録 |
| [#629][rf629] 本番 OS 仮想 Endpoint | [RF wiring 実走](../rf1001/verification.md) は RF `50edac47118028c5ab4f45a0bbd9c86451c73998`、Windows Server 2025 x64、[run 45555696425][rf-wiring] の `os-endpoint-wiring` stage で Audio/MIDI `Pass`、test exit 0（正規入口 `mise run test-os-endpoint-wiring`、stage は `--nocapture` 付き補助コマンド）。[未 provision のログ](../rf629/os_endpoint_wiring_lane_selfrun-windows-local.txt) は両方 `NotRun`。SV の別 tuple の実測は SV-DEV-OBS 参照 | EXT-L3。OS が列挙・open した Endpoint、Driver 版、Format、Callback/Timestamp/Recovery と製品 oracle。Fake/後段注入は L1 のみ |
| [#630][rf630] 正規 ABI Test Plugin | [外部 validator 記録](../rf630/abi-validator-rerun-verification.md) は RF `1d18e2970d887fe925d3777ddb814bc46644af3d`、Windows x64、`mise run test-plugin-abi-validator` exit 0。ABI と artifact digest を記録 | EXT-L3。利用形式ごとに SDK/ABI、Plugin version/hash、Host の Load/Process/State/Unload・故障経路を結合。fixture 単体の成功だけでは足りない |
| [#631][rf631] 実 GUI Runner | [RF 検証記録](../rf631/verification.md)。旧 DComp の `Fail` と [RF #532 の close 記録][rf532] にある class A の修正後 `Pass`・class C の残存 `Fail` を各元 Case に保持 | GUI-L2。実 Window、入力/focus、画素、receipt と製品状態。Native Dock の実 OS 合成は EXT-L3 の実 ABI 証拠にも対応付ける。Fake/Offscreen/Floating の結果を転用しない |

## #219 / #294 への引き渡し

| Consumer | 今回の候補に対する結果 | 保留理由と次の担当 |
| --- | --- | --- |
| AL | `NotRun`（3OS L1、Windows 利用 GUI-L2） | AL #127 が同一 candidate を解決した Consumer SHA で CLI/Logging/Privacy/f64 を個別に採取。GUI の OBS ON/OFF・sink 故障も別記録 |
| SV | `NotRun`（3OS L1、Windows 利用 GUI-L2 / EXT-L3） | SV #28 が同一 candidate を解決した Consumer SHA で RT/Plugin/Gateway/Subscriber/Project を個別に採取。録音は SV #115、実 GUI は SV #114、実 Plugin/Dock は SV #176。runner 不在による旧 CI `Fail` は SV の CI/fixture 担当へ |

[RF #219][rf219] が互換運用、[RF #294][rf294] が candidate tuple と証跡形式の参照先、RF #281 が OBS 別判定を扱う。次回は既存 Daily を優先し、両 Consumer の checkout SHA と resolved SHA を別々に照合する。不一致なら候補の採用を保留し、正常/故障それぞれの Case ID、OS/build/arch、Feature、fixture version/hash、期待/観測値、Run URL/exit と Owner を記録する。共通 API/RT/Privacy の不適合は RF の該当 OBS Owner、製品 Adapter・出力や権限の不適合は Consumer Owner に割り当てる。採用に失敗した場合は各 Consumer Owner が直前の検証済み lock revision への復帰を判断する。この文書では pin、CI、Issue 状態を変更しない。

[rf281]: https://github.com/scarecrow173/ruviform/issues/281
[rf219]: https://github.com/scarecrow173/ruviform/issues/219
[rf294]: https://github.com/scarecrow173/ruviform/issues/294
[rf598]: https://github.com/scarecrow173/ruviform/issues/598
[rf628]: https://github.com/scarecrow173/ruviform/issues/628
[rf629]: https://github.com/scarecrow173/ruviform/issues/629
[rf630]: https://github.com/scarecrow173/ruviform/issues/630
[rf631]: https://github.com/scarecrow173/ruviform/issues/631
[rf532]: https://github.com/scarecrow173/ruviform/issues/532#issuecomment-7061993481
[pr521]: https://github.com/scarecrow173/ruviform/pull/521
[pr547]: https://github.com/scarecrow173/ruviform/pull/547
[al127]: https://github.com/scarecrow173/audio-lens/issues/127
[sv28]: https://github.com/scarecrow173/synvelis/issues/28
[sv114]: https://github.com/scarecrow173/synvelis/issues/114
[sv176]: https://github.com/scarecrow173/synvelis/issues/176
[al-lock]: https://github.com/scarecrow173/audio-lens/blob/bd7bda931a9080b0e6233005016358f919922b11/Cargo.lock
[sv-lock]: https://github.com/scarecrow173/synvelis/blob/bf0442cfad73c5102d137f3f1e20ac74d84860b3/Cargo.lock
[al-pin]: https://github.com/scarecrow173/audio-lens/blob/bd7bda931a9080b0e6233005016358f919922b11/evidence/al127/87a7ba8f-verification.md
[al-real]: https://github.com/scarecrow173/audio-lens/blob/6cb68413bd69aa440a88a6c1ee17bf3a5f8bad30/evidence/al219/gui-real-scenario.jsonl
[sv-dock]: https://github.com/scarecrow173/synvelis/blob/bf0442cfad73c5102d137f3f1e20ac74d84860b3/evidence/sv176/s1-headless-probe.md
[sv-device]: https://github.com/scarecrow173/synvelis/actions/runs/45834341661
[sv-real]: https://github.com/scarecrow173/synvelis/pull/673
[rf-wiring]: https://github.com/scarecrow173/ruviform/actions/runs/45555696425
[rf-daily]: https://github.com/scarecrow173/ruviform/actions/runs/45935355893
[rf-ci]: https://github.com/scarecrow173/ruviform/actions/runs/45935377401
[rf-win]: https://github.com/scarecrow173/ruviform/actions/runs/45935377401/job/140297257671
[rf-linux]: https://github.com/scarecrow173/ruviform/actions/runs/45935377401/job/140297257639
[rf-mac]: https://github.com/scarecrow173/ruviform/actions/runs/45935377401/job/140297257315
[rf-rt]: https://github.com/scarecrow173/ruviform/actions/runs/45935377401/job/140297257299
[al-ci]: https://github.com/scarecrow173/audio-lens/actions/runs/45935543875
[al-win]: https://github.com/scarecrow173/audio-lens/actions/runs/45935543875/job/140297848431
[al-mac]: https://github.com/scarecrow173/audio-lens/actions/runs/45935543875/job/140297848362
[al-linux]: https://github.com/scarecrow173/audio-lens/actions/runs/45935543875/job/140297848364
[sv-ci]: https://github.com/scarecrow173/synvelis/actions/runs/45935641612
[sv-win]: https://github.com/scarecrow173/synvelis/actions/runs/45935641612/job/140298111067
[sv-mac]: https://github.com/scarecrow173/synvelis/actions/runs/45935641612/job/140298110989
[sv-linux]: https://github.com/scarecrow173/synvelis/actions/runs/45935641612/job/140298111058
[sv-rt]: https://github.com/scarecrow173/synvelis/actions/runs/45935641612/job/140298111046
[tuple-v1]: https://github.com/scarecrow173/ruviform/actions/runs/42937524742
[tuple-oracle]: https://github.com/scarecrow173/ruviform/actions/runs/43907984482
[tuple-v2]: https://github.com/scarecrow173/ruviform/actions/runs/44017688605
[tuple-blocked]: https://github.com/scarecrow173/ruviform/actions/runs/44013452606
[tuple-al]: https://github.com/scarecrow173/ruviform/actions/runs/43907984482/job/133442222272
[tuple-sv]: https://github.com/scarecrow173/ruviform/actions/runs/43907984482/job/133442222414
