# DESIGN Documents

<!-- doc-suite:generated-index -->

Directory: `docs/designs`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-OVERVIEW | System Design Overview | draft | [overview.md](./overview.md) |
| DESIGN-034Kq7iQj2mt7VuR8YP2vq | ruviform-audio-analysis 設計（レベル/スペクトル/波形ピーク） | approved | [audio-analysis.md](./audio-analysis.md) |
| DESIGN-034JcC9QK924kDbu4WhibC | オーディオデバイス境界 trait と ruviform-audio-device PoC の詳細設計 | approved | [audio-device-boundary-and-poc.md](./audio-device-boundary-and-poc.md) |
| DESIGN-034JROdBjynllTNAVDZwaI | 音声エンジン設計 | approved | [audio-engine.md](./audio-engine.md) |
| DESIGN-034JOoWdEHXnb8G1QyMzCm | chunk 分割時の per-chunk timestamp 合成設計 | approved | [chunk-timestamp-design.md](./chunk-timestamp-design.md) |
| DESIGN-034KScKaiUBXz4qKJZqM3h | ruviform-video-proc コンポジット/遷移カーネル設計（blend/affine/基本遷移初期セット） | approved | [composite-transition-kernels.md](./composite-transition-kernels.md) |
| DESIGN-034JZDNRML000000000003 | DenormalFlush ノード設計 | approved | [denormal-flush-node.md](./denormal-flush-node.md) |
| DESIGN-034IddXFCRKDIW01WXkSla | Rust 開発基盤の設計（最小ワークスペース・CIプロファイル・手動CI） | approved | [dev-foundation.md](./dev-foundation.md) |
| DESIGN-034JPvz9ieX8KoPbwDcvaX | dispatch 固定コスト分離の計測設計 | approved | [dispatched-overhead-method.md](./dispatched-overhead-method.md) |
| DESIGN-034JcqP2KiWsdes88fgvtt | DSP/SIMD 比較ベンチマーク PoC 詳細設計 | approved | [dsp-simd-poc.md](./dsp-simd-poc.md) |
| DESIGN-034KNBPbww1kvzVUF4Ab1H | ruviform-edit 設計（NLE 編集データモデル + 評価射影） | approved | [edit-data-model.md](./edit-data-model.md) |
| DESIGN-034KQXr3w4dYgKO79dklG9 | Foundation/Observability 候補モジュールの最小 API・依存 DAG・新設時期の設計 | approved | [foundation-observability-minimal-api.md](./foundation-observability-minimal-api.md) |
| DESIGN-034JcpvK9cZEfkeEbBLBoz | Issue #28 UI 技術検証 PoC の設計（使い捨て検証ハーネスと計測・評価方式） | approved | [issue-28-ui-poc.md](./issue-28-ui-poc.md) |
| DESIGN-034JO1BQYrzMQsaFmL3YkF | workspace MSRV 引上げの適用範囲と検証設計 | approved | [msrv-raise-design.md](./msrv-raise-design.md) |
| DESIGN-iwcBlHQ0lcdHJGucocxHOo | 音楽時間モデル設計（小節/拍/PPQN・テンポマップ・プラグイン ABI 整合） | approved | [music-time-model.md](./music-time-model.md) |
| DESIGN-034JRarHKPb6eKlZqW9WeO | オフライン実行ドライバ設計 | approved | [offline-execution-driver.md](./offline-execution-driver.md) |
| DESIGN-034JZPGUI0000000000002 | プラグイン GUI PoC 設計（外部親 HWND + 外部メッセージループへの wry attach） | approved | [plugin-gui-poc.md](./plugin-gui-poc.md) |
| DESIGN-mEXH6f0uKRaa2mwCWE8mxC | ruviform-plugin-host 設計（L0 インプロセス・ハンドル/コマンド境界） | approved | [plugin-host-l0.md](./plugin-host-l0.md) |
| DESIGN-034JZLISO0000000000003 | プラグイン分離設計（境界・プロセス監視・RT 面制約） | approved | [plugin-isolation.md](./plugin-isolation.md) |
| DESIGN-034IvBNBZvoikAgd6DJxdD | テンプレート由来ドキュメント・識別子のRuviform固有化の設計 | approved | [replace-template-derived-docs-with-ruviform.md](./replace-template-derived-docs-with-ruviform.md) |
| DESIGN-034JgsK9khgDsPqG04Sp6i | RT 基盤 API 設計（ruviform-rt プリミティブ・ruviform-exec スレッド優先度機構） | approved | [rt-foundation-api-design.md](./rt-foundation-api-design.md) |
| DESIGN-034JOU7C43HBPJBsnz8JQP | RT 検証レーン設計 | approved | [rt-verification-lane-design.md](./rt-verification-lane-design.md) |
| DESIGN-034IPynWe2V30FLubzQenb | Ruviform architecture design baseline | approved | [ruviform-architecture-design-baseline.md](./ruviform-architecture-design-baseline.md) |
| DESIGN-lzqhLAShvqF3efUlehuaUS | ruviform-dsp 設計（上位 DSP カーネル・math::dsp 層分担・prepare/process 分離） | approved | [ruviform-dsp.md](./ruviform-dsp.md) |
| DESIGN-Tl9CIdfNfmIADLgFIeaTOq | 第 2 ターゲット設計（NLE 編集モデル・OFX 範囲・色管理） | approved | [second-target-design.md](./second-target-design.md) |
| DESIGN-034Kx6CVMPDNgzKhJwfgSp | 共通基盤・Observability・Diagnostics・Debug とクレート分割の設計方針 | approved | [shared-foundations-and-modularization-policy.md](./shared-foundations-and-modularization-policy.md) |
| DESIGN-034JQxPUJjVzr5aekaU1B8 | SIMD dispatch 層設計（トークン繰り上げ + ルーティング + 公開 API 形） | approved | [simd-dispatch-layer.md](./simd-dispatch-layer.md) |
| DESIGN-034JcePtzKFix4IuxyE5mD | 時間モデル拡張の設計（FrameRate/TimeBase 分離・Timecode 拡張・丸め誤差累積方針） | approved | [time-model-extension.md](./time-model-extension.md) |
| DESIGN-034JZVID00000000000030 | 動画エンジン設計（パイプライン・スケジューラ・セッション） | approved | [video-engine.md](./video-engine.md) |
