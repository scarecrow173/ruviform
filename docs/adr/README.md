# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/adr`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| ADR-034JROe7Y5kwFgAkp9Q0bu | 音声エンジンのグラフ再構成方式（不変スナップショット交換） | accepted | [audio-engine-snapshot-swap.md](./audio-engine-snapshot-swap.md) |
| ADR-lg9vK3EAHePSiJf7YHThrm | 広範色管理の段階的範囲 | accepted | [broad-color-management.md](./broad-color-management.md) |
| ADR-034JOpkx5tdDPVGzdlb0bb | chunk 分割時の OutputCallbackInfo timestamp 合成 | accepted | [chunk-outputcallbackinfo-timestamp.md](./chunk-outputcallbackinfo-timestamp.md) |
| ADR-20261101-variable-ppqn-rational-beat | 可変PPQN・分解能非依存の音楽時間・Native MIDI時刻保持 | accepted | [configurable-ppqn-resolution-independent-musical-time.md](./configurable-ppqn-resolution-independent-musical-time.md) |
| ADR-034QdDEaffrclLniNBLhQe | Cross-Repo CI アクセス認証と候補revision検証方式 | proposed | [cross-repo-ci-revision.md](./cross-repo-ci-revision.md) |
| ADR-034JZDNRML000000000002 | denormal 崖対策の実装方針（エンジン層ソフトウェア flush ノード） | accepted | [denormal-mitigation-policy.md](./denormal-mitigation-policy.md) |
| ADR-034JEe7SAbksaycyrum16R | DSP/SIMD 二層構造とバックエンドの採否 | accepted | [dsp-simd-backend-adoption.md](./dsp-simd-backend-adoption.md) |
| ADR-034JcihlNe8WCZVpp9RBCq | DSP/SIMD PoC 評価アプローチ | accepted | [dsp-simd-poc.md](./dsp-simd-poc.md) |
| ADR-034JceOKDwTeWus9xBVXND | FrameRate/TimeBase 意味分離と Timecode 拡張 | accepted | [framerate-timebase-separation-and-timecode-extension.md](./framerate-timebase-separation-and-timecode-extension.md) |
| ADR-034NNxNMDjKssvme4VQnrv | L1 supervisor 配置 — `ruviform-plugin-host` opt-in util 層と core `PluginHost` 無変更境界（Issue #116） | accepted | [issue-116-supervisor-placement.md](./issue-116-supervisor-placement.md) |
| ADR-034Qb8Lg3MqZx1SuVcEj6k | L1 bundle 子の `Failed` 粒度・`InstanceFailed` fan-out・隔離選択面（Issue #117） | accepted | [issue-117-bundle-child-failed-granularity.md](./issue-117-bundle-child-failed-granularity.md) |
| ADR-034NVCYbbmwSxZnvHYNkAx | video/NONRT プラグイン L1 の create-path・segment sizing（CreateVideoInstance に VideoProcessConfig を載せる形）（Issue #118） | accepted | [issue-118-video-l1-create-path-segment.md](./issue-118-video-l1-create-path-segment.md) |
| ADR-034NVCXBHjsHlfixAwUSHE | video/NONRT プラグイン L1 の別立て host 表面・plugin_id 名前空間・Dedicated 固定（Issue #118） | accepted | [issue-118-video-l1-host-surface.md](./issue-118-video-l1-host-surface.md) |
| ADR-034O9Rrn3iBoq4HxT07eVQ | issue-121-l1-diagnostics-policy | accepted | [issue-121-l1-diagnostics-policy.md](./issue-121-l1-diagnostics-policy.md) |
| ADR-s16uWrHwN9qD8cKo517PU7 | Issue #181 Multi-Window — contract placement, close-decision separation, and IPC routing | accepted | [issue-181-multi-window-contract-placement-close-decision-ipc-routing.md](./issue-181-multi-window-contract-placement-close-decision-ipc-routing.md) |
| ADR-QYZTHdxPLL6w7vQT2qyPuW | Issue #182 Dock Tree — contract placement, two-phase view move, persistence schema, and close policy | accepted | [issue-182-dock-tree-placement-move-contract-and-persistence.md](./issue-182-dock-tree-placement-move-contract-and-persistence.md) |
| ADR-034QhNYvqRm2YZ7nXzOiMb | issue-185 generic probe contract placement and RT handoff model | accepted | [issue-185-generic-probe-contract-placement-and-rt-handoff-model.md](./issue-185-generic-probe-contract-placement-and-rt-handoff-model.md) |
| ADR-NWm4RRQv0BypmTnFRlChlN | Timed MIDI contract placement, clock authority, and deferral of OS backend selection | proposed | [issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md](./issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md) |
| ADR-034QhrQYpNhSfuTeq7hyfp | Issue #191 PCM WAV output backend: ruviform-io-wav crate, MediaSink port, staged publish, conversion policy | accepted | [issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md](./issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md) |
| ADR-Tm4TW9CWQgvmIkMzROw0ei | M4 plugin adapter: bridge crate placement, node-internal scheduler ownership, bus role enumeration, plan retire model | accepted | [issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md](./issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md) |
| ADR-I20b5KtQfHowI9YL4J5I0u | Issue #197 Media Reader / Prefetch — crate placement, boundary additions, prefetch ownership/generation contract | accepted | [issue-197-media-reader-boundary-additions.md](./issue-197-media-reader-boundary-additions.md) |
| ADR-YPgjjmV3ZLSeoH45bD5llz | Issue #198 sequential offline render placement (ruviform-audio::render) + capability/staging contracts | accepted | [issue-198-sequential-offline-render-placement-and-contracts.md](./issue-198-sequential-offline-render-placement-and-contracts.md) |
| ADR-034Qoirxpj2Nt7TtjWeDgk | Issue #233 prefetch contract improvements — doc-only clarifications, consumer drain assist + seek discard flag, fused reopen-at-position | accepted | [issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md](./issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md) |
| ADR-rgzm45MqKPlpzqO7Uhbqeq | Issue #255 CorrelationPlan 変換長の非2冪高速サイズ化 — 既定選択の変更と混合基数 FFT の採用 | accepted | [issue-255-nonpow2-fft-transform-length.md](./issue-255-nonpow2-fft-transform-length.md) |
| ADR-034IO02s3n3KHsxnHckpRn | ライセンス・公開方針 | accepted | [licensing-and-distribution-policy.md](./licensing-and-distribution-policy.md) |
| ADR-034QfjWFfUXvgvaObuCUwS | M2 typed graph audio runtime placement, stateful RT ownership, and activation/retirement for Issue #195 | proposed | [m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md](./m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md) |
| ADR-034INZsD434t5cQ7gBp7XN | メディアI/Oバックエンドの抽象境界分離 | accepted | [media-io-backend-abstraction.md](./media-io-backend-abstraction.md) |
| ADR-034TRn2qP9wXkLmVcYbJdE | メディア観測事実の分類モデルと非破壊 PCM 取得経路（Issue #151） | accepted | [media-observation-non-destructive-pcm.md](./media-observation-non-destructive-pcm.md) |
| ADR-034JRK9VFJbrUPAvy3FJ94 | オフライン実行の決定論保証範囲（逐次参照等価） | accepted | [offline-determinism-scope.md](./offline-determinism-scope.md) |
| ADR-034L2CiSoCLplTQvNluQfY | OFX ホストの video 契約境界と C ABI バインディング（Issue #98） | accepted | [ofx-host-video-contract-and-c-abi.md](./ofx-host-video-contract-and-c-abi.md) |
| ADR-Lhe1Psi2bhfA35B3FABi6q | OFX 等映像プラグイン規格の対応範囲 | accepted | [ofx-video-plugin-scope.md](./ofx-video-plugin-scope.md) |
| ADR-034Qkwzty3hORnIr3aDMO7 | Parameter control events contract placement and deferral of generic Control Graph crate | proposed | [parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md](./parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md) |
| ADR-034IO01ZSFQcu2jEt2p0Tb | 対象プラットフォーム・CPUアーキテクチャ・ツールチェーン方針 | accepted | [platform-and-toolchain-targets.md](./platform-and-toolchain-targets.md) |
| ADR-034IO01bG8AyjIJWxHeL77 | プラグインアーキテクチャ方針（ホスト・制作双方とCLAP/VST3同格） | accepted | [plugin-architecture-clap-vst3.md](./plugin-architecture-clap-vst3.md) |
| ADR-034JDzseXORnckswW1oYKh | Plugin binding method: clack for CLAP, coupler-rs vst3 for VST3 | accepted | [plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md](./plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md) |
| ADR-034JZPGUI0000000000007 | プラグイン GUI 埋め込み方式（外部親ウィンドウへの wry attach） | accepted | [plugin-gui-embedding.md](./plugin-gui-embedding.md) |
| ADR-034JZLISO0000000000002 | プラグイン分離戦略（インプロセス既定 + opt-in アウトプロセス） | accepted | [plugin-isolation-strategy.md](./plugin-isolation-strategy.md) |
| ADR-034MbMOuLa1T5TWfzINoHl | プラグイン処理の non-RT 制御/RT 処理所有分離（transferable RT endpoint） | accepted | [plugin-rt-ownership-split.md](./plugin-rt-ownership-split.md) |
| ADR-034J6OnZouPI8HQy6cIg9D | 公開 enum への `#[non_exhaustive]` 統一方針 | accepted | [public-enum-non-exhaustive-policy.md](./public-enum-non-exhaustive-policy.md) |
| ADR-034IO00JjhxeTXUbCZtcDX | リアルタイム契約と検証方針 | accepted | [realtime-contract-and-verification-policy.md](./realtime-contract-and-verification-policy.md) |
| ADR-034Ivm746Q38Wa21HeqUiI | テンプレート由来ドキュメント・識別子のRuviform固有化方針 | accepted | [replace-template-derived-docs-metadata-with-ruviform.md](./replace-template-derived-docs-metadata-with-ruviform.md) |
| ADR-034JOU73yGYKDc21f7C2SY | RT 検証機構の選定（assert_no_alloc 標準化・thread-check 意味論） | accepted | [rt-assert-no-alloc-thread-check.md](./rt-assert-no-alloc-thread-check.md) |
| ADR-034Jcra57UlCiVE1Poq2DO | RT 基盤プリミティブの実装方式（SPSC・退避ドロップ・スナップショット交換・スレッド優先度・契約型） | accepted | [rt-foundation-primitives-implementation.md](./rt-foundation-primitives-implementation.md) |
| ADR-034IcJdtqm6TP6MylyP5J7 | 開発基盤の toolchain・Cargoプロファイル・CI方針 | accepted | [rust-dev-foundation-toolchain-and-ci-profile.md](./rust-dev-foundation-toolchain-and-ci-profile.md) |
| ADR-R2GFzIbod68oHGdQZTuyJW | 第 2 ターゲット NLE 編集モデル | accepted | [second-target-editing-model.md](./second-target-editing-model.md) |
| ADR-034Jc9RslnH7QSQ1nZEEHL | オーディオデバイスバックエンドの選定（cpal 条件付き採用） | accepted | [select-audio-device-backend.md](./select-audio-device-backend.md) |
| ADR-154-media-backends | AVI/MP4/AAC 入力バックエンドの配置: AVI は専用 pure-Rust クレート、MP4/AAC は symphonia opt-in feature | accepted | [select-media-backends-avi-and-mp4.md](./select-media-backends-avi-and-mp4.md) |
| ADR-034JQvAkxQaMfRbMHDRLoM | 本番 SIMD カーネルの精度契約形（ulp 固定 + 有限入力前提） | accepted | [simd-ulp.md](./simd-ulp.md) |
| ADR-20261101-synvelis-driven-generic-execution-boundaries | Ruviformの汎用Graph/Audio Execution/Stateful Processorとアプリ意味論の責務境界 | accepted | [synvelis-driven-generic-execution-boundaries.md](./synvelis-driven-generic-execution-boundaries.md) |
| ADR-20261102-transport-span-scheduler-applied-receipt | Generic Transport Span Scheduler・Applied Receipt・Clock/Plan Generation 契約と Plugin min/max Block 方針 | accepted | [transport-span-scheduler-applied-receipt.md](./transport-span-scheduler-applied-receipt.md) |
| ADR-034QdPgLCqwTPQ4FHtjLsj | Typed multi-bus Graph IR placement and Port/Connection validation model (Issue #184 M1) | accepted | [typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md](./typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md) |
| ADR-034IO01BpGWCl1DcEbcTQI | UIアーキテクチャ方針（WebViewファースト・ハイブリッド） | accepted | [ui-architecture-webview-hybrid.md](./ui-architecture-webview-hybrid.md) |
| ADR-034Jc9Cguud7nZ2Tt88Ptu | UI 技術選定（wry・描画バックエンド・フロントエンド FW・配布）— PoC 採否 | accepted | [ui-tech-selection-poc.md](./ui-tech-selection-poc.md) |
| ADR-034JZVID00000000000002 | A/V 同期判定ポリシー（audio master + PTS 追従） | accepted | [video-av-sync-policy.md](./video-av-sync-policy.md) |
| ADR-034JcidaiOOip5EhNPOnDI | video-proc カーネル API 形状と色変換・HDR 対応範囲 | accepted | [video-proc-kernel-api-and-color-hdr-scope.md](./video-proc-kernel-api-and-color-hdr-scope.md) |
| ADR-034JNYt58J8NlUkn6RfJMs | workspace MSRV 引上げ（1.85 → 1.89）と fearless_simd 本番昇格の解禁 | accepted | [workspace-msrv-1-85-1-89-fearless-simd.md](./workspace-msrv-1-85-1-89-fearless-simd.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| ADR-2iNuZm3ymoyzRUmpgnKr0x | 音楽時間モデル（PPQN 960 tick 中核・piecewise-constant テンポマップ） | superseded | [musical-time-model.md](./musical-time-model.md) |
