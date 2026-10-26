# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/adr`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| ADR-034JROe7Y5kwFgAkp9Q0bu | 音声エンジンのグラフ再構成方式（不変スナップショット交換） | accepted | [audio-engine-snapshot-swap.md](./audio-engine-snapshot-swap.md) |
| ADR-lg9vK3EAHePSiJf7YHThrm | 広範色管理の段階的範囲 | accepted | [broad-color-management.md](./broad-color-management.md) |
| ADR-034JOpkx5tdDPVGzdlb0bb | chunk 分割時の OutputCallbackInfo timestamp 合成 | accepted | [chunk-outputcallbackinfo-timestamp.md](./chunk-outputcallbackinfo-timestamp.md) |
| ADR-034JZDNRML000000000002 | denormal 崖対策の実装方針（エンジン層ソフトウェア flush ノード） | accepted | [denormal-mitigation-policy.md](./denormal-mitigation-policy.md) |
| ADR-034JEe7SAbksaycyrum16R | DSP/SIMD 二層構造とバックエンドの採否 | accepted | [dsp-simd-backend-adoption.md](./dsp-simd-backend-adoption.md) |
| ADR-034JcihlNe8WCZVpp9RBCq | DSP/SIMD PoC 評価アプローチ | accepted | [dsp-simd-poc.md](./dsp-simd-poc.md) |
| ADR-034JceOKDwTeWus9xBVXND | FrameRate/TimeBase 意味分離と Timecode 拡張 | accepted | [framerate-timebase-separation-and-timecode-extension.md](./framerate-timebase-separation-and-timecode-extension.md) |
| ADR-034IO02s3n3KHsxnHckpRn | ライセンス・公開方針 | accepted | [licensing-and-distribution-policy.md](./licensing-and-distribution-policy.md) |
| ADR-034INZsD434t5cQ7gBp7XN | メディアI/Oバックエンドの抽象境界分離 | accepted | [media-io-backend-abstraction.md](./media-io-backend-abstraction.md) |
| ADR-2iNuZm3ymoyzRUmpgnKr0x | 音楽時間モデル（PPQN 960 tick 中核・piecewise-constant テンポマップ） | accepted | [musical-time-model.md](./musical-time-model.md) |
| ADR-034JRK9VFJbrUPAvy3FJ94 | オフライン実行の決定論保証範囲（逐次参照等価） | accepted | [offline-determinism-scope.md](./offline-determinism-scope.md) |
| ADR-034L2CiSoCLplTQvNluQfY | OFX ホストの video 契約境界と C ABI バインディング（Issue #98） | accepted | [ofx-host-video-contract-and-c-abi.md](./ofx-host-video-contract-and-c-abi.md) |
| ADR-Lhe1Psi2bhfA35B3FABi6q | OFX 等映像プラグイン規格の対応範囲 | accepted | [ofx-video-plugin-scope.md](./ofx-video-plugin-scope.md) |
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
| ADR-034JQvAkxQaMfRbMHDRLoM | 本番 SIMD カーネルの精度契約形（ulp 固定 + 有限入力前提） | accepted | [simd-ulp.md](./simd-ulp.md) |
| ADR-034IO01BpGWCl1DcEbcTQI | UIアーキテクチャ方針（WebViewファースト・ハイブリッド） | accepted | [ui-architecture-webview-hybrid.md](./ui-architecture-webview-hybrid.md) |
| ADR-034Jc9Cguud7nZ2Tt88Ptu | UI 技術選定（wry・描画バックエンド・フロントエンド FW・配布）— PoC 採否 | accepted | [ui-tech-selection-poc.md](./ui-tech-selection-poc.md) |
| ADR-034JZVID00000000000002 | A/V 同期判定ポリシー（audio master + PTS 追従） | accepted | [video-av-sync-policy.md](./video-av-sync-policy.md) |
| ADR-034JcidaiOOip5EhNPOnDI | video-proc カーネル API 形状と色変換・HDR 対応範囲 | accepted | [video-proc-kernel-api-and-color-hdr-scope.md](./video-proc-kernel-api-and-color-hdr-scope.md) |
| ADR-034JNYt58J8NlUkn6RfJMs | workspace MSRV 引上げ（1.85 → 1.89）と fearless_simd 本番昇格の解禁 | accepted | [workspace-msrv-1-85-1-89-fearless-simd.md](./workspace-msrv-1-85-1-89-fearless-simd.md) |
