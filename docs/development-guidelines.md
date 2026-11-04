# 開発規約・設計方針 詳細リファレンス

この文書は [AGENTS.md](../AGENTS.md) の Progressive Disclosure 構成における詳細側です。AGENTS.md の `### 暫定規約` 詳細部分、`### アーキテクチャ確定契約（設計ベースライン）`、`### 共通基盤・Observability・Diagnostics・Debug の設計方針（必要時参照）` から分離した記述を収録します（移動のみ・内容量は落としていません）。各記述が指す正本（設計ベースライン・ADR・`mise.toml`・`workspace.lints` 等）との齟齬がある場合は正本を優先します。

## 暫定規約（詳細）

- MSRV clippy 検証（Issue #223）: lint gate の対象は pin toolchain のみで、MSRV 1.89 上の clippy は情報 lane として扱う。1.89 検出器のみが発火する `float_cmp` / `similar_names` は toolchain 版差の既知ノイズであり、MSRV 上で検査する場合は `-A` フラグで除外する: `cargo +1.89.0 clippy --workspace --all-targets -- -D warnings -A clippy::float_cmp -A clippy::similar_names`。これら 2 lint は pin toolchain でも pedantic に残るため `workspace.lints` で `allow` しない（pin gate を弱めない）。MSRV でこれら以外の失敗が残る場合は除外リストへ機械追加せず、個別に判定する。
- RT 検証レーン（Issue #22）: `rt_` プレフィックスの RT 契約テストは `assert_no_alloc` 測定窓内で処理経路を実行し、窓内の alloc/dealloc はテストバイナリを abort させて CI で検出する。fixture・warm-up の確保は窓外に置く。ruviform-rt 既存の `alloc_counter`（API セマンティクスの実装機能）は変更せず併存し、新規の検証用途には `assert_no_alloc` を使う。CLAP アダプタは CLAP `thread-check` 拡張を `plugin.init` で一度照会し、ホストが非オーディオスレッドと応答した `process` を defined rejection する（拡張未提供・応答不定は従来経路）。機械検出の対象は alloc/dealloc と当該スレッド契約のみ — ブロッキング操作・ファイル/ネットワーク I/O・無制限待機/リトライ・実行時間無界・ロック保持時間等の静的検証不能な制約はドキュメント契約 + コードレビュー観点であり、CI 成功は RT 安全性の完全証明ではない。

## アーキテクチャ確定契約（設計ベースライン）

以下は Issue [#1](https://github.com/scarecrow173/ruviform/issues/1) 由来の設計ベースライン（[docs/designs/ruviform-architecture-design-baseline.md](docs/designs/ruviform-architecture-design-baseline.md)、approved）で確定した契約の要約。詳細な定義・根拠は同書の各節が正本であり、本節と齟齬がある場合は同書を優先する。
- RT/オフライン分離: リアルタイム（RT）経路と非 RT を API・ライフサイクルで分離する。処理契約は `Processor`（`prepare`/`process` 分離）と `RtContext` capability が定義する。RT クリティカルパスでは alloc/dealloc を禁止し、ブロッキング操作・I/O・無制限の待機/リトライ・非有界処理を制限する。「RT-safe」等の保証範囲は API 単位で定義・明示し、未確定の API を RT-safe と断定しない。検証可能な違反は CI で検出するが、CI 成功は完全証明ではない。OS レベルのハードリアルタイム保証は提供しない。正本: 同書「RT／非 RT の分離方式」「RT 契約（API 単位の保証定義）」節。
- SIMD: スカラー参照実装を必須とし、カーネルごとに「ビット完全一致」または「ulp 上限内」の等価性契約を定義してスカラーを参照として検証する。stable Rust のみを使用し、`unsafe` は暫定規約どおり SAFETY コメント付きの局所許可のみで例外的に認める。SIMD 二層構造（ランタイム分岐＋フォールバック、dispatch 層の `ruviform-math` 集約を含む構造仮説）とバックエンド選定の採否は要 PoC。正本: 同書「DSP／SIMD 設計」採否。
- クレート依存方向: 依存は上位 → 下位のみとし、循環依存は禁止。基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない。詳細な層内順序・adapter ルール・契約型の所有は同書「レイヤー構造と依存方向ルール」節が正本。

## 共通基盤・Observability・Diagnostics・Debug の設計方針（必要時参照）

- 共通基盤の責務境界、段階的なモジュール→クレート分割、Memory / Collections / Sync / Identity / Telemetry、Logging / Profiling / Metrics / Diagnostics / Debug、OS / IPC との境界は [docs/designs/shared-foundations-and-modularization-policy.md](docs/designs/shared-foundations-and-modularization-policy.md)（Issue #101）を設計方針の参照先とする。
- これらの機能や RT 計測経路、クレート・依存 DAG・公開 API の所有者・feature 境界を新設・変更・レビューする際は、必要に応じて同書の該当節を参照する。無関係な変更に同書の全読は要求しない。
- `ruviform-foundation` / `ruviform-observability` / `ruviform-debug` / `ruviform-platform` 等は現時点では構成候補であり、クレート・API が実装されたことを意味しない。具体的な実装・既存 API の移動は別途設計・検証する。既存 accepted ADR（特に RT 契約・RT プリミティブ方式）と矛盾する変更は、必要な ADR 改訂を経て行う。
