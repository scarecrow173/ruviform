# SPEC Documents

<!-- doc-suite:generated-index -->

Directory: `docs/specs`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| SPEC-034INYy2aDLxse9dy8MgGZ | Ruviform 設計ベースライン文書群の作成（アーキテクチャ・クレート構成・RT/オフライン実行モデル） | approved | [architecture-design-baseline-deliverables.md](./architecture-design-baseline-deliverables.md) |
| SPEC-034Kq7ipiNwhFkBrHtwlII | ruviform-audio-analysis 実装 spec | approved | [audio-analysis.md](./audio-analysis.md) |
| SPEC-034Jc9QEqYLVfg1CXXTiPV | オーディオデバイス境界 trait と ruviform-audio-device PoC（バックエンド評価・採否） | implemented | [audio-device-backend-evaluation-poc.md](./audio-device-backend-evaluation-poc.md) |
| SPEC-034JROd7c0jBSZ1hnPEAg5 | 音声エンジン（pull グラフ・デバイスアダプタ統合・RT パス） | approved | [audio-engine.md](./audio-engine.md) |
| SPEC-034JOoOaExdxXgKrQ80l7x | chunk 分割時の OutputCallbackInfo timestamp 合成 | approved | [chunk-timestamp-synthesis.md](./chunk-timestamp-synthesis.md) |
| SPEC-034JZDNRML000000000001 | denormal 対策（エンジン層: ソフトウェア flush 方針） | approved | [denormal-mitigation.md](./denormal-mitigation.md) |
| SPEC-034JPubrypO0IylcWkuqlz | fearless_dispatched 軽量カーネル劣後の原因分離（低負荷再計測） | approved | [dispatched-overhead-analysis.md](./dispatched-overhead-analysis.md) |
| SPEC-034Jci19siJZexLs53Xh5d | DSP/SIMD 比較ベンチマーク PoC と採否 ADR | implemented | [dsp-simd-poc-adr.md](./dsp-simd-poc-adr.md) |
| SPEC-034IcwhwIFSNUNKtX7KK0Q | Rust 開発基盤の確立（mise/APM lockfile・最小ワークスペース・手動CI） | approved | [establish-rust-dev-foundation.md](./establish-rust-dev-foundation.md) |
| SPEC-034J5FYaYfeL5zkzb4sIV4 | メディアデータモデル API の実装（media-core / audio-core / video-core） | implemented | [implement-media-data-model-api.md](./implement-media-data-model-api.md) |
| SPEC-034JcaMXubILS7j1FX5qob | メディア I/O 抽象境界と第一バックエンド PoC の実装 | implemented | [implement-media-io-abstraction-and-first-backend-poc.md](./implement-media-io-abstraction-and-first-backend-poc.md) |
| SPEC-034JcgdAKPNeh1YCFyprzh | プラグイン基盤の実装（ruviform-plugin-core + CLAP/VST3 アダプタ・binding 方式 PoC） | implemented | [implement-plugin-foundation.md](./implement-plugin-foundation.md) |
| SPEC-034Jcr3dOT5Uxp4znj42II | RT 基盤の実装（ruviform-rt プリミティブ・スレッド優先度機構・スナップショット交換） | implemented | [implement-rt-foundation.md](./implement-rt-foundation.md) |
| SPEC-034JcicHlRidBgcK1wPtj2 | 動画基盤 API の確定（video-core / video-proc・色変換・HDR 対応範囲） | approved | [implement-video-foundation-api.md](./implement-video-foundation-api.md) |
| SPEC-034J6OCXHiP1izKRzbMmgz | Issue #18 フォローアップ（公開 enum 方針適用・時間演算修正・改善トリアージ） | implemented | [issue-18-followup-enum-and-time-fix.md](./issue-18-followup-enum-and-time-fix.md) |
| SPEC-034Jc9BJlI7ULEBWoIlMP2 | Issue #28 UI 技術検証 PoC（wry・描画バックエンド・フロントエンド FW・配布） | implemented | [issue-28-ui-poc.md](./issue-28-ui-poc.md) |
| SPEC-034JNYkUPfvyc1GQ5hXtHN | workspace MSRV 引上げ（fearless_simd 本番昇格前提） | approved | [msrv-raise-for-fearless-simd.md](./msrv-raise-for-fearless-simd.md) |
| SPEC-ey1gakUyUmWw9lBalMOE79 | 音楽時間設計（小節/拍/PPQN・テンポマップ） | approved | [music-time-design.md](./music-time-design.md) |
| SPEC-034JRarfc0pLFZoZeGMVhn | オフライン実行ドライバ（チャンク駆動・進捗/キャンセル・決定論モード） | approved | [offline-execution-driver.md](./offline-execution-driver.md) |
| SPEC-034JZPGUI0000000000001 | プラグイン GUI PoC（規格準拠ネイティブビューへの WebView 埋め込み検証） | approved | [plugin-gui-poc.md](./plugin-gui-poc.md) |
| SPEC-aNDyfHu1ZkBojTHEltDl94 | ruviform-plugin-host 新設（L0 インプロセス・ハンドル/コマンド境界） | approved | [plugin-host-l0.md](./plugin-host-l0.md) |
| SPEC-034JZLISO0000000000001 | プラグイン分離方式設計（サンドボックス・プロセス分離） | approved | [plugin-isolation-design.md](./plugin-isolation-design.md) |
| SPEC-034Ivc5nj1AE6P2PcZU6Ki | テンプレート由来のドキュメント・識別子をRuviform向けに置き換える | approved | [replace-template-derived-docs-with-ruviform.md](./replace-template-derived-docs-with-ruviform.md) |
| SPEC-034JOU6EjinMh0rhxE6yM9 | RT 検証基盤（allocation 検出・thread-check・検証文書） | approved | [rt-verification-lane.md](./rt-verification-lane.md) |
| SPEC-X9YOosDAuB8PX2Qv31thuB | ruviform-dsp 新設（上位 DSP カーネル層 — prepare/process 分離・RT 適合） | approved | [ruviform-dsp.md](./ruviform-dsp.md) |
| SPEC-5eEaW8lLmoyVPgocJnooYD | 第 2 ターゲット設計（NLE 範囲・OFX 等映像プラグイン規格・広範色管理） | approved | [second-target-design.md](./second-target-design.md) |
| SPEC-034JQubfZRC7nRmTsYvLiq | SIMD dispatch 層（prepare 固定 + カーネル/サイズ別ルーティング + 本番精度契約） | approved | [simd-dispatch-layer.md](./simd-dispatch-layer.md) |
| SPEC-034JceNZQvAawWwMH4ct91 | 時間型拡張（FrameRate/TimeBase 意味分離・Timecode 拡張・丸め誤差累積方針） | implemented | [time-type-extension-framerate-timecode-rounding.md](./time-type-extension-framerate-timecode-rounding.md) |
| SPEC-034JZVID00000000000010 | 動画エンジン（フレームパイプライン・A/V 同期・シーク） | approved | [video-engine.md](./video-engine.md) |
