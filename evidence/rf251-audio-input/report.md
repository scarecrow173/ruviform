# RF251 Audio Input OS-L1 既存CI証拠照合 report — Issue #251

- Issue: https://github.com/scarecrow173/ruviform/issues/251
- 対象 SHA: `b9eaf4880681734b2d65dbf200ca8c49c6e4c81e`（main HEAD・PR #602 merge）。作業開始時点の main HEAD は `49043603`（PR #601 merge）だったが、作業中に PR #602 が merge され main が進行したため、タスク仕様の指示どおり最新 main に合わせて記録した。作業 branch `docs/251-audio-input-os-l1-evidence` は `origin/main` = `b9eaf488` 起点
- 収集日: 2026-11-13
- 作業 branch: `docs/251-audio-input-os-l1-evidence`
- 本 slice の範囲: Issue #251 の OS-L1 受入条件（既存 CI での Audio Device 関連コード Build/Link・Mock/Device capability/Error/RT Headless 回帰の対象 SHA 照合、非 Windows 実 Backend の実装有無と Unsupported 契約の正確な記録、実測済み/NotRun/Unsupported の OS×Backend×Device×Case 一覧、物理 fault・macOS/Linux 実機の Deferred 明記）に限定。コード変更なし・証拠記録のみ。macOS/Linux 実機測定・物理 USB 抜去・実 OS 権限拒否・xrun 実誘発・SV #115/#117 製品側受入・#315 異 Clock Drift は対象外で Issue は open 継続。

## 対象 SHA の CI 状態（最重要の正直な記録）

**対象 SHA `b9eaf488`（および直前の `49043603`）に対する completed CI run は収集時点で存在しない。**

- `.github/workflows/ci.yml` は `workflow_dispatch` 専用（Issue #365/#326 Stage 4 — PR 作成・main push では起動しない）。`gh api repos/scarecrow173/ruviform/actions/runs?head_sha=b9eaf488…` および `?head_sha=49043603…` はいずれも 0 件、`status=in_progress`/`queued` にも該当 run なし。
- 次回の検証機会は daily-ci scheduler（cron `17 21 * * *` UTC、main HEAD を `expected_sha` 付きで `ci.yml` へ dispatch する契約 — `.github/workflows/daily-ci.yml`）。
- したがって **対象 SHA の OS-L1 統合検証は `NotRun`（Pending）** であり、本 report の CI 証拠は「同一コードに対する直近 completed run」の補助証拠として扱う（`POLICIES.md` §3・§5: 対象 main SHA の必須 CI 結果が揃うまで統合検証は NotRun）。

## 証拠 run の選定と SHA 差分

| 項目 | 値 |
| --- | --- |
| 採用 run | Full CI `ci.yml` workflow_dispatch run [43428172820](https://github.com/scarecrow173/ruviform/actions/runs/43428172820) — 2026-11-13T12:47:36Z 開始、conclusion=`success`、`ci-summary` verdict=`PASS`（19 lane leg + ci-summary の計 20 job 全て success） |
| run の対象 SHA | `e93c438adc4af5b498e10926fe859ac1338ac390`（`ci-summary` の 19 checkout leg 全て実測 `e93c438a…` で一致・verdict=Pass を job log で確認） |
| 対象 SHA との差分 | `git diff e93c438a..b9eaf488` = `.github/workflows/ci.yml`（verify 系 job への `actionlint` tool 追加のみ・8 行）+ `.pre-commit-config.yaml`（actionlint local hook）+ `crates/ruviform-ui-test` の test コード（`dummy_app.rs`/`e2e_process.rs`、Issue #313）+ docs/evidence。**`crates/ruviform-audio-device`・`ruviform-audio-core`・`ruviform-audio` の差分はゼロ** — audio device 関連コードは 3 SHA（`e93c438a`/`49043603`/`b9eaf488`）で同一 |
| 判定 | run 43428172820 の cargo build/test/lint 証拠は対象 SHA とコード同一だが **SHA が異なるため対象 SHA の Pass にはしない**。対象 SHA の CI 確定は daily-ci dispatch 後の同 SHA run をもって Coordinator が判定する |

## 検証スコープと Oracle

| 項目 | 内容 |
| --- | --- |
| OS | `windows-latest` / `ubuntu-latest` / `macos-latest`（GitHub hosted runner） |
| 採用 Feature | workspace default のみ（`check-full`/`lint-full`/`test-full` は `--all-features` なし）。opt-in feature `real-device` / `realtime` / `pipewire-backend` は CI のどの lane でも有効化されない（後述「Feature opt-in の実行範囲」） |
| Device/Format | CI 上は実 Device なし — mock backend・cpal host の headless 経路・型付きエラー面のみ。実機は過去証拠（PR #568/#581、Audient iD4/WASAPI）を別枠で引用 |
| Run URL / Oracle | run 43428172820。Oracle = 各 job/step success、cargo test の `test result` 集計行、`checkout-sha-*` artifact の実測 SHA、`ci-summary` の fail-closed verdict、および本 report が個別確認した job log 内のテスト名・件数 |
| 新規 CI Workflow | 作成していない（既存 `ci.yml` の dispatch run のみを証拠とする） |

## audio-device 関連 job/log 証拠（run 43428172820、SHA `e93c438a`）

`ruviform-audio-device` は workspace leaf adapter（workspace 内に依存元 crate なし — `crates/ruviform-audio` は doc 言及のみ、実依存は外部 Consumer の Synvelis）。port 契約は `ruviform-audio-core::device`。CI での直接カバーは `verify`（ubuntu/windows）・`verify-macos`・`rt-verify`（ubuntu）のみ。`bench` job は `bench-smoke`（`cargo bench -p ruviform-math -p ruviform-rt -p ruviform-viz -- --test`）で audio-device を compile すらしない。`icc-verify`/`compression-verify`/`io-structured-verify`/`ui-*`/`l1-real-verify` は他 crate・feature 向けの package/feature scope lane、`msrv-verify` は audio-device 非対象（後述）。

| job | OS | audio-device 関連の観測（job log 実測） | job URL |
| --- | --- | --- | --- |
| verify | ubuntu-latest | `lint-full` で `Checking ruviform-audio-device`（clippy `-D warnings` 通過）。`test-full`: lib unittests **59 passed**（`convert`/`device`/`error`/`host_impl`/`stream` 各 tests、`rt_input_callback_path_allocates_nothing`・`rt_output_callback_path_allocates_nothing`・`clock_profile_*`・`map_input_timestamp_*`・error 写像 tests 含む）、`tests/mock_backend.rs` **4 passed**、`tests/mock_input_backend.rs` **12 passed**（`rt_input_callback_handoff_allocates_nothing`・`rt_input_callback_queue_pressure_stays_bounded`・`same_clock_correlation_via_clock_domain`・`scripted_errors_reach_the_error_callback`・`open_failures_use_the_defined_error_surface`・`cpal_adapter_input_open_reports_typed_surface` 等）、`tests/real_input_e2e.rs` **0 tests**（`real-device` feature 未設定で空 compile — 正直な非実行）、Doc-tests **6 passed**。ALSA backend は `libasound2-dev` install step 経由で link | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691153) |
| verify | windows-latest | 同上構成で lib unittests **75 passed**（上記 59 + `wasapi_input::tests` 16 件 — `handle_packet_*`・`position_tracker_*`・`map_win_error_maps_audclnt_codes_per_context`（`E_ACCESSDENIED` 行含む）・`rt_wasapi_packet_path_allocates_nothing` 等）、`mock_backend` **4**、`mock_input_backend` **12**、`real_input_e2e` **0**、Doc-tests **6** | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691209) |
| verify-macos | macos-latest | `coreaudio-rs v0.14.2`（+ `objc2-core-audio` 系）を compile/link（CoreAudio backend の実在証拠）。lib unittests **59 passed**（`wasapi_input` は `cfg(windows)` で非含有）、`mock_backend` **4**、`mock_input_backend` **12**、`real_input_e2e` **0**、Doc-tests **6** | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691243) |
| rt-verify | ubuntu-latest | `mise run test-rt`（`cargo test --workspace rt_`）: audio-device lib から `rt_input_callback_path_allocates_nothing`・`rt_output_callback_path_allocates_nothing`（2 passed / 57 filtered）、`mock_input_backend` から `rt_input_callback_handoff_allocates_nothing`・`rt_input_callback_queue_pressure_stays_bounded`（2 passed / 10 filtered）を確認 | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691112) |
| msrv-verify | ubuntu-latest (rust@1.89.0) | 対象は `test-ofx-msrv`/`test-ui-test-msrv`/`test-observability-msrv`/`test-io-msrv` の 4 crate set のみ — **audio-device の MSRV 検証 lane は存在しない**（NotRun・既存 lane 構成の記録。CI pin は rust 1.99.0、workspace `rust-version = "1.89"`） | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691257) |
| l1-real-verify(-macos) | 3 OS | 実 Plugin L1 lane（`test-l1-real-plugin`）。job 内 comment に「the adapter dependency tree contains no cpal」と明記 — **audio-device のカバーなし**（plugin host adapter の fixture cdylib E2E） | [job](https://github.com/scarecrow173/ruviform/actions/runs/43428172820/job/131894691699) |

補足: `verify` 系 job は `prek run --all-files`・`fmt-check`・`lint-full`（`cargo clippy --workspace --all-targets -- -D warnings`、example `list_devices`/`play_sine` 含む全 target）・`test-full`（`cargo test --workspace`、default feature）を実行。`test-full` が `real_input_e2e.rs` を compile するが `#![cfg(feature = "real-device")]` で中身は空 — CI で実機テストが走らないことの log 上の証拠。

## 非 Windows 実 Backend の実装有無と Unsupported 契約（コード確認 @ `b9eaf488`）

`crates/ruviform-audio-device` の backend 構造（`src/lib.rs`・`src/device.rs`・`src/host.rs`・`src/stream.rs`・`src/wasapi_input.rs`・`Cargo.toml` を精読）:

| OS | cpal host（default feature） | adapter 側の実装 | port 契約上の正直な応答 |
| --- | --- | --- | --- |
| Windows | WASAPI（cpal `cfg(windows)` 常時 host） | input は `native_input_route`（`host_name=="WASAPI"`）で `wasapi_input.rs` の **WASAPI-native path** へ分岐（Issue #565 — `frame_position` を運べる唯一経路）。output は cpal portable | input: `frame_position=Some`（per-packet `GetBuffer` device position）・`input_latency()=Ok(Some(StreamLatency{frames, coverage:Unknown, provenance:Carried}))`（`GetStreamLatency`）・`clock_domain=MONOTONIC`・capture provenance=`Carried`。output: `output_latency()=Ok(None)`（port default — cpal は portable latency query を持たない） |
| macOS | CoreAudio（cpal `target_vendor="apple"` 常時 host、`coreaudio-rs`/`objc2-core-audio` で link — CI log で compile 確認） | **native path なし** — `native_input_route` は `#[cfg(not(windows))]` で常に `false`、portable cpal `build_input_stream` のみ | `frame_position=None`（cpal が frame position を公開しない — 捏造しない設計）・`input_latency()=Ok(None)`・`output_latency()=Ok(None)`・`clock_profile("CoreAudio")=MONOTONIC`/capture provenance=`Derived`（`mach_absolute_time` 系の audited claim） |
| Linux | ALSA（cpal linux target dep 常時。`pipewire-backend` feature で PipeWire 追加可・opt-in で CI 未踏） | 同上 — portable cpal path のみ | `frame_position=None`・`input_latency()=Ok(None)`・`output_latency()=Ok(None)`・`clock_profile("ALSA")=UNKNOWN`（per-substream device clock で same-clock を証明不能 → `same_clock_domain` が正しく false を返し duplex は fallback） |

- **OS レベルの「未対応プラットフォーム」gate は存在しない**: 3 OS 全てで cpal が実 backend を提供し、adapter は同一 port 契約で build される。cpal の `Null` host は win/mac/linux 以外の fallback であり対象外。
- **Unsupported 契約**（`device.rs`/`error.rs`/port `device.rs` で確認）: `!supports_input()` → `DeviceError::Unsupported`（input 非対応 device）、非 F32/範囲外 config → `UnsupportedConfig`、`ErrorKind::PermissionDenied`/`E_ACCESSDENIED` → `Backend("permission_denied")`（両経路で parity — PR #581）、host 不在 → `Backend("host_unavailable")`、`DeviceNotAvailable` は Control→`DeviceNotFound`/ErrorCallback→`StreamInvalidated` の context 別写像、`Xrun` は ErrorCallback のみ `Xrun`。
- **Timeout**: port v1 は意図的に `open_timeout` を持たない（`device.rs` L170-174「opening delegates to the backend default, which may block」— 契約上の決定）。実機 lane は 30 s watchdog（`MAX_RUN`）で hang を abort するが、timeout 契約自体は NotApplicable/未実装ではなく設計上の非存在。
- **存在しない Device IO を実動作と主張しない**: macOS/Linux で実 InputStream の capture・latency・frame_position・duplex・device loss の実測証拠は本 report に存在しない（後述 matrix で NotRun/Deferred）。cpal path の `frame_position=None`/`input_latency=Ok(None)`/`output_latency=Ok(None)` は「unsupported を型で正直に返す」契約応答であり、実測 Pass ではない。

## Feature opt-in の実行範囲（受入条件「Feature opt-in の明示」対応）

| feature | 内容 | CI 実行範囲 | 実機/手動 lane |
| --- | --- | --- | --- |
| `default`（空） | cpal backend・portable path・wasapi_input（Windows compile） | `verify` 3 OS + `rt-verify` で compile/test 済み（上表）。`bench` job（bench-smoke）は対象外 | — |
| `real-device` | `tests/real_input_e2e.rs` の `#[ignore]` 5 test を有効化する gate | **CI では一切実行されない**（`test-full` は feature なしで compile のみ・0 tests）。`lint-audio-real-input`（clippy `-D warnings` + feature）も CI lane なし | `mise run test-audio-real-input` / `mise run lint-audio-real-input` — opt-in manual lane（mise.toml 明記「never CI」）。Windows+Audient iD4 で PR #568/#581 が実施 |
| `realtime` | `cpal/realtime` → `audio_thread_priority`（RT 優先度昇格。Windows/Android のみ作用） | **未踏** — どの CI lane も本 feature を有効化しない | 手動 `cargo` 経路のみ（専用 mise task なし） |
| `pipewire-backend` | `cpal/pipewire`（Linux PipeWire host 追加・`libpipewire-0.3-dev` 必須） | **未踏** — Linux lane は ALSA default のみ | 手動 opt-in（専用 mise task なし） |

## OS×Backend×Device×Case 一覧（実測 Pass / Unsupported / NotRun / Deferred）

判定語彙は `POLICIES.md` §3。証拠 URL は run 43428172820（SHA `e93c438a`、対象 SHA と audio 関連 crates 同一）または PR/実機記録。**対象 SHA `b9eaf488` 自体の CI は全行で NotRun(Pending)** — 以下の CI 由来 Pass は「同一コードの直近 completed run」の補助証拠。

### Windows / WASAPI / Audient iD4（実機・opt-in lane 証拠）

| Case | verdict | 証拠 |
| --- | --- | --- |
| Native Build/Link（adapter + cpal WASAPI + windows crate + wasapi_input） | Pass（補助） | verify (windows) job 131894691209（`e93c438a`）+ 実機 lane が同一コードで build |
| Host/Device 列挙・default input device 解決 | Pass（実機） | PR #568 `test-audio-real-input` 4/4（`Analogue 1/2` endpoint 解決） |
| Format negotiate・非 F32/範囲外の `UnsupportedConfig` 拒否 | Pass（実機） | PR #568 — format 拒否 7 行を実機 supported 空間で検証（`real_input_open_rejects_unsupported_configs`） |
| 実 Capture（open→start→planar callback→stop） | Pass（実機） | PR #568 — 192 callbacks / 84,672 frames、order violations=0 |
| Timestamp（callback/capture instant・provenance） | Pass（実機） | PR #568 — fresh QPC delivery instant 修正後 `capture > callback` 違反 0 |
| `frame_position`（per-packet device position） | Pass（実機） | Issue #565 で WASAPI-native path に実装。`real_input_e2e` が「WASAPI input must report frame_position on every callback」を assert し、PR #568（4/4）で `Some` 192/192・gaps=0、PR #581（5/5）でも継続。packet-position discontinuity は `ClockDiscontinuity` に写像 |
| `clock_domain` / same-clock duplex 判定 | Pass（実機・domain verdict まで） | PR #568 — input/output domain 一致・`same_clock_domain` verdict・UNKNOWN rejection。**frame 相関 math 自体は NotRun**（synvelis#115/#117・Issue #315 管轄） |
| `input_latency()` | Pass（実機） | PR #568 — `Ok(Some{frames:0, coverage:Unknown, provenance:Carried})` + 実効 latency 10.04–10.12 ms を callback `callback-capture` で観測 |
| `output_latency()` / 実効 output latency | Pass（実機・記録値） | PR #581 — `output_latency()=Ok(None)`（cpal portable の正直応答）・実効 `playback-callback` min=0 / max≈32.0 ms / mean≈30.9 ms（102 samples・assert なし記録） |
| Error 写像（typed `DeviceError`、`E_ACCESSDENIED`→`permission_denied` parity 含む） | Pass（headless + 実機 error 経路） | headless: `map_win_error_maps_audclnt_codes_per_context`・`error::tests::*`（verify windows 75 tests 内）。実機: PR #581 で parity 追加、PR #568 で `StreamInvalidated`/`ClockDiscontinuity` 実観測 |
| Device loss / invalidation | Pass（実機・代替経路） | PR #568 — 既定 endpoint 切替で `StreamInvalidated`×2 + `ClockDiscontinuity`×1 を error callback に観測。**物理 USB 抜去（`AUDCLNT_E_DEVICE_INVALIDATED`）は Deferred/NotRun** — 同一 typed error の別 trigger であり物理経路の Pass ではない |
| OS permission denied 実再現 | **NotRun** | headless 写像 parity のみ（PR #581）。privacy deny を制御する環境がない |
| xrun / underrun 実誘発 | **NotRun** | 観測 0 件（PR #568/#581）。誘発負荷は lane 責務外・別途 Owner/Trigger 付き Gate |
| RT（callback path alloc-free） | Pass（headless） | `rt_input_callback_path_allocates_nothing`・`rt_output_callback_path_allocates_nothing`・`rt_wasapi_packet_path_allocates_nothing`（verify windows + rt-verify） |
| Timeout（`open_timeout`） | NotApplicable（契約上非存在）+ lane watchdog | port v1 は `open_timeout` を持たない設計（`device.rs`）。実機 lane は 30 s watchdog |

### Windows / WASAPI / CI runner（実 Device なし headless）

| Case | verdict | 証拠 |
| --- | --- | --- |
| Native Build/Link・clippy `-D warnings`・workspace test | Pass（補助・対象 SHA は NotRun） | verify (windows) job 131894691209 |
| Mock Input/Output・Device capability/Error/RT headless 回帰 | Pass（補助） | lib 75 + mock_backend 4 + mock_input_backend 12 + doctest 6（同上）・rt-verify の rt_ filter |
| 実 Device IO（capture/latency/device loss） | NotRun | `test-audio-real-input` は opt-in manual lane で CI 非実走。runner の audio device 有無・権限は未検証環境 |

### macOS / CoreAudio / CI runner

| Case | verdict | 証拠 |
| --- | --- | --- |
| Native Build/Link（cpal CoreAudio backend — `coreaudio-rs` compile/link） | Pass（補助・対象 SHA は NotRun） | verify-macos job 131894691243 |
| Mock/headless・型付きエラー面・clock_profile（CoreAudio→MONOTONIC/Derived）・RT | Pass（補助） | lib 59 + mock 4+12 + doctest 6・`rt_*` 含む |
| 実 InputStream capture・timestamp 実測・`frame_position` | **NotRun** | 実機 lane が macOS に存在しない。cpal path は `frame_position=None` が契約上の正直応答（実測ではない） |
| `input_latency()`/`output_latency()` 実値 | Unsupported（契約上 `Ok(None)`）+ 実測 NotRun | cpal portable path は latency query を populate しない設計。実機での確認自体が NotRun |
| same-clock duplex 実測 | NotRun | `clock_domain=MONOTONIC` は audited claim（unit test 済み）だが実 stream での duplex 観測なし |
| device loss・permission denied・xrun 実機 | NotRun | macOS 実機 session 未実施 |

### Linux / ALSA（+ opt-in PipeWire）/ CI runner

| Case | verdict | 証拠 |
| --- | --- | --- |
| Native Build/Link（cpal ALSA backend — `alsa`/`alsa-sys` link、`libasound2-dev` step） | Pass（補助・対象 SHA は NotRun） | verify (ubuntu) job 131894691153 |
| Mock/headless・型付きエラー面・clock_profile（ALSA→UNKNOWN → duplex fallback）・RT | Pass（補助） | lib 59 + mock 4+12 + doctest 6・`rt_*`・`same_clock_correlation_via_clock_domain` |
| PipeWire backend（`pipewire-backend` feature） | NotRun | opt-in feature・CI 未踏・`libpipewire-0.3-dev` 必要 |
| PulseAudio/JACK host | Unsupported（adapter 未公開） | cpal 0.18.2 に `pulseaudio`/`jack` feature は存在するが `ruviform-audio-device` は公開していない（`pipewire-backend` のみ opt-in）— adapter の surface に経路が存在しないため実行不能 |
| 実 InputStream capture・latency・duplex・device loss・permission・xrun | **NotRun** | macOS と同様 — 実機 lane なし。ALSA runner には音声 device がなく `cpal_adapter_input_open_reports_typed_surface` は device 不在を skip/typed error で処理 |

### 横断

| Case | verdict | 証拠 |
| --- | --- | --- |
| 対象 SHA `b9eaf488` の Full CI（3 OS Build/Link + headless 回帰） | **NotRun（Pending）** | completed run が存在しない（workflow_dispatch 専用）。daily-ci（cron `17 21 * * *` UTC）は「対象 SHA に有効な Full CI 結果がない場合のみ `expected_sha` 付きで `ci.yml` を dispatch（sha 毎 2 回上限）」の決定契約 — `b9eaf488` は履歴なしのため次回 scheduler 実行で dispatch される見込み（main がさらに進行した場合は最新 SHA が対象になる）。同一 audio コードの直近 run 43428172820 @ `e93c438a` を補助証拠として上表に記録 |
| audio-device の MSRV（rust 1.89）検証 | NotRun | `msrv-verify` の対象は ofx/ui-test/observability/io の 4 set のみ |
| `realtime` feature（cpal/realtime → RT 優先度） | NotRun | opt-in・CI lane なし・専用 mise task なし |
| SV 固有の録音/Frame 相関受入 | 対象外（委譲） | synvelis#115/#117・Issue #315 — 本 repo の OS-L1 証拠の範囲外 |

## 実機証拠の引用（CI headless 証拠との区別）

以下は **opt-in manual lane（`mise run test-audio-real-input`、feature `real-device`）の過去実機証拠**であり、現行 CI run では実走しない（lane 定義に「never CI」と明記）。CI headless 証拠（上記 run 43428172820）とは別系統として記録する。

| PR | merge commit | 実機環境 | 結果 |
| --- | --- | --- | --- |
| [#568](https://github.com/scarecrow173/ruviform/pull/568)（Stage W closeout） | `68accdc7`（2026-11-11 merge） | Windows 11 x64・Audient iD4・WASAPI・44.1 kHz F32 Stereo・`Analogue 1/2` endpoint・rust 1.99.0 | `test-audio-real-input` **4/4 Pass**（capture 192cb/84672f・order violations=0・実効 input latency 10.04–10.12 ms・`input_latency()=Ok(Some)`・domain 一致+UNKNOWN rejection・format 拒否 7 行）。device loss は既定 endpoint 切替で `StreamInvalidated`×2+`ClockDiscontinuity`×1 実観測。物理 USB 抜去・permission denied・xrun は NotRun/観測なし |
| [#581](https://github.com/scarecrow173/ruviform/pull/581)（Stage W residual） | `2db36a54`（2026-11-12 merge） | 同上 iD4/WASAPI | `test-audio-real-input` **5/5 Pass**。`map_win_error` に `E_ACCESSDENIED`→`Backend("permission_denied")` parity 追加（headless 検証）。`real_output_stream_observes_latency` で実効 output latency min=0/max≈32.0 ms/mean≈30.9 ms・`output_latency()=Ok(None)`・errors=0・xrun=0 を記録（assert なし・値は環境依存） |

両 PR の IR: `docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md`・`docs/impl/ir/issue-251-stage-w-output-latency.md`（headless slice は `issue-251-audio-device-real-backend-input-plumbing-headless-slice.md` = PR #538）。

## 判定まとめ

- **対象 SHA `b9eaf488` の OS-L1 CI 証拠: NotRun（Pending）。** `ci.yml` は workflow_dispatch 専用で同 SHA の completed run がなく、daily-ci（cron `17 21 * * *` UTC）による dispatch が次の検証機会。統合検証の Pass 判定は Coordinator が対象 SHA の必須 CI 結果で行う（`POLICIES.md` §5）。
- **補助証拠（同一コード・SHA 差あり）: run 43428172820 @ `e93c438a` は 20/20 job success・ci-summary verdict=PASS。** `audio-device`/`audio-core`/`audio` の差分ゼロのため audio-device の Build/Link・Mock Input/Output・Device capability/Error/RT headless 回帰は 3 OS で確認できるが、SHA が異なるため対象 SHA の Pass には繰り上げない。
- **非 Windows backend**: macOS=CoreAudio・Linux=ALSA(+opt-in PipeWire) の実装は cpal 層に実在し 3 OS で compile/link されるが、adapter の native gap-fill は Windows/WASAPI input のみ。macOS/Linux の実 Device IO（capture/latency/frame_position/duplex/device loss/permission/xrun）は **NotRun** であり、cpal path の `None`/`Ok(None)` 応答は Unsupported の正直な契約表現であって実測 Pass ではない。
- **実機証拠**: Windows/WASAPI/Audient iD4 の input/output・latency・clock・format・error・device loss（代替 trigger）は PR #568/#581 で Pass 済み（opt-in lane・CI 非含有）。物理 USB 抜去・実 OS 権限拒否・xrun 誘発・macOS/Linux 実機は **Deferred — NotRun を Pass にしない**。
- **Issue #251 は open 継続。** 残る阻害要因は (a) 対象 SHA の必須 CI 完了、(b) macOS/Linux 実機測定（Stage M/L closeout）、(c) 物理 fault（USB 抜去・permission denied・xrun）の Owner/Trigger 付き後続 OS Gate、(d) SV #115/#117 の製品側受入と #315 の異 Clock Drift — いずれも本 slice の証拠記録の範囲外。
