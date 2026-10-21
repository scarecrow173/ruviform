---
id: "ADR-034Jc9RslnH7QSQ1nZEEHL"
type: "adr"
status: "accepted"
title: "オーディオデバイスバックエンドの選定（cpal 条件付き採用）"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "media-io-backend-abstraction.md"
    - "realtime-contract-and-verification-policy.md"
    - "platform-and-toolchain-targets.md"
    - "public-enum-non-exhaustive-policy.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../specs/audio-device-backend-evaluation-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/evaluate-audio-device-backends.md"
    - "https://github.com/scarecrow173/ruviform/issues/23"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/RustAudio/cpal"
    - "https://github.com/RustAudio/cpal/issues/981"
    - "https://github.com/RustAudio/cpal/pull/1229"
    - "https://github.com/mozilla/cubeb-rs"
    - "https://github.com/MeadowlarkDAW/rtaudio-rs"
    - "https://github.com/BillyDM/Firewheel"
    - "https://github.com/HEnquist/audioadapter-rs"
    - "https://crates.io/crates/audio_thread_priority"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# オーディオデバイスバックエンドの選定（cpal 条件付き採用）

## Context and Problem Statement

GitHub Issue #23（オーディオデバイスバックエンド評価と採否）の確定決定として、`ruviform-audio-device`（adapter）が実装するデバイス境界 trait（ポート定義所有は `ruviform-audio-core`、設計ベースライン責務表）の背後に置くオーディオデバイスバックエンドを選定する。ベースライン分類表はこの領域を「要PoC・抽象境界必須・メンテリスク」と位置づけており、cpal は事実上の標準である一方、停滞実績（issue #981 で 2025-06 に公式認め）を含むメンテナンスリスクがある。抽象境界の維持を前提に、cpal・代替候補（cubeb-rs / rtaudio-rs / プラットフォーム直結）を評価軸（discovery 文書に記録）で比較し、採否と適用条件を記録する。外部調査結果は 2026-10 時点の公開情報に基づく。

## Decision Drivers

* コールバック駆動モデルとコールバック内 RT 契約（alloc/dealloc 禁止・ブロッキング/I/O/無制限待機の制限）への適合性
* デバイス境界 trait への適合性と差替可能性（adapter 交換で第 2 backend を追加できること）
* データモデル整合: planar f32 正準・interleaved は境界のみ、ストリーム時刻/タイムスタンプ API（`MasterClock` 要件）、バックエンドエラーの抽象エラー正規化
* MSRV 1.85・stable のみ・`unsafe` 原則禁止・依存木サイズとライセンス（`publish = false` 方針と整合）
* 対象プラットフォーム（Windows/macOS/Linux、x86_64+aarch64）とスレッド優先度機構（MMCSS・time-constraint・rtkit）
* メンテナンス状況（リリース頻度・メンテナ構成・停滞実績）と、停滞時に交換可能な構造になっていること

## Considered Options

* **cpal**（RustAudio/cpal）— pure Rust のデファクト標準。WASAPI/CoreAudio/ALSA（0.18〜 PipeWire・PulseAudio ネイティブ）/AAudio/WebAudio をカバー
* **cubeb-rs**（mozilla/cubeb-rs）— Firefox 本番コード。1 コールバック同期入出力の真の full-duplex
* **rtaudio-rs**（Meadowlark/rtaudio-rs）— RtAudio 6 の安全ラップ。`Buffers` enum で真 duplex
* **プラットフォーム直結自前 adapter**（wasapi / coreaudio-rs / alsa / pipewire / jack を個別 adapter として実装）
* **その他不採用群**（oboe-rs / libsoundio 系 / rust-portaudio / miniaudio ラッパ / sdl3 / wavy / nannou_audio）

## Decision Outcome

Chosen option: "**cpal**（条件付き採用）"。Rust エコシステムのデファクト標準であり、MSRV 1.85 が ruviform と一致、pure Rust の薄い OS バインディング層で外部 C ライブラリを同梱せず、対象プラットフォームを単一依存でカバーする。0.18 系で `StreamTrait::now()`/`buffer_size()`・ハードウェアレイテンシ込み timestamp・`ErrorKind` 統一（Xrun/DeviceBusy/RealtimeDenied/StreamInvalidated）が入り、`MasterClock` 要件とエラー正規化要件に整合する。`realtime` feature（`audio_thread_priority` 実体）でコールバックスレッドの RT 昇格が可能。

採用は以下の条件付きとする（いずれも設計ベースライン・media-io 抽象境界 ADR の既存契約の適用であり新規制約ではない）:

1. **非露出**: cpal の全型・エラー・列挙値を `ruviform-audio-device` 内部に閉じ込め、`ruviform-audio-core` の API 面に露出しない。feature 名に固有名詞を出さない
2. **adapter 責務の明示**: interleaved `Data` → planar f32 への境界変換と、コールバック内非 alloc の規律を adapter の責務として trait 契約に明示する
3. **差替可能性**: duplex（同期入出力）が要件化した時点で第 2 backend を追加可能な trait 設計とする（cpal は duplex API 未実装のため、trait がバックエンド固定の形状にならないこと）
4. **ASIO は opt-in 隔離**: 既定で無効。build.rs の SDK 自動ダウンロードは再現性の注意点として記録し（`CPAL_ASIO_DIR` で回避可）、対応判断は後続のプラットフォーム要件評価に委ねる

他候補の扱い:

* **cubeb-rs / rtaudio-rs**: 真 duplex を持つ最有力の代替候補として**保留**。duplex が要件化した時点、または cpal のメンテナンスが再停滞した場合の再評価対象とする。現時点で採用しないのは、C++/cmake ビルド依存と FFI（cubeb-sys）・C++ 静的ビルド（rtaudio）による依存木の重さ・`unsafe` 境界が、現行要件（出力ストリーム中心）に見合わないため
* **プラットフォーム直結自前 adapter**: **中長期オプションとして保留**。RT 制御自由度は最大で CamillaDSP の先例があるが、3 OS 分の adapter を自前保守するコストが現フェーズに見合わない。trait の差替可能性が維持されれば将来の移行経路は残る
* **不採用群**: 停滞・archived・機能不足のため**棄却**（oboe-rs は停滞・Android 限定で cpal 自身が脱却済み、libsoundio 系は abandonware/archived、rust-portaudio は maintenance mode で cpal 推奨、miniaudio は Rust ラッパ停滞、sdl3 は粒度が粗い、wavy 未完成、nannou_audio は cpal ラッパ）

### Consequences

* Good, because: 単一の軽量依存で主要プラットフォームをカバーでき、公開 API に外部型を露出しない構造でメンテナンスリスクを隔離できる。trait 設計が条件 (3) を満たせば cubeb/rtaudio/直結への交換経路が残り、cpal 依存の判断を後から覆しやすい。`audio_thread_priority` 実体による RT 昇格と、fundsp に先例のある assert_no_alloc 検証手法がそのまま使える
* Bad, because: cpal の実質単独メンテナ依存と停滞実績のリスクは残存する（条件付き採用の「条件」の維持・再評価トリガの監視が必要）。duplex API 未実装のため duplex 要件化時は cpal 単独では不足し、第 2 backend の追加コストが発生する。interleaved バッファ形状との変換コスト・WASAPI 排他モード未対応・プラットフォーム個別設定 API 未確定といった機能ギャップを adapter が吸収する必要がある

### Confirmation

下記 Verification 全項目を PoC 証跡で確認し、2026-10-21 に `proposed` → `accepted` へ昇格した。証跡: 実験ログ `docs/impl/exp/device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl`（実機 Windows 11/WASAPI/cpal 0.18.2 実行記録）、IR `docs/impl/ir/implement-audio-core-device-port.md`・`docs/impl/ir/implement-audio-device-cpal-adapter.md`・`docs/impl/ir/run-device-poc-examples-and-record-experiment-log.md`、テスト `rt_output_callback_path_allocates_nothing`（assert_no_alloc）・`tests/mock_backend.rs`（非 cpal 差替可能性）。再評価トリガ（duplex の要件化・cpal メンテナンスの再停滞・対象プラットフォームでの致命的挙動不良）はいずれも観測されなかった。未観測として後続へ引き継ぐ項目: ALSA vs PipeWire 比較（Linux レーン）、Send/Sync の backend 間ばらつき、WASAPI exclusive/ASIO 要否。なお初報で未観測項目に挙げた「初回 callback timestamp=0」は、レビューで example の計測器バグ（`fetch_min` が sentinel 上で更新不能）由来と確定し、修正後の再実行では backend 時計と整合する値が観測された（実験ログ seq11）。トリガ発生時は本 ADR の status 見直しと代替候補の再評価を行う。

## Implementation Plan

* Affected paths:
  * `crates/ruviform-audio-core`: デバイス境界 trait（ポート）の定義を追加
  * `crates/ruviform-audio-device`: 新設。cpal adapter の PoC 実装（列挙・既定構成・出力ストリーム開始・コールバック到達まで）
  * `docs/impl/exp/`: PoC 評価レポート（実験ログ）の記録先
  * ルート `Cargo.toml` / `AGENTS.md` / README 類: クレート追加に伴う記述同期
* Patterns to follow: [media-io-backend-abstraction.md](media-io-backend-abstraction.md) の抽象境界規則（外部型・エラー・feature 名の非露出、分離クレート＋opt-in、実行時文字列選択の禁止）をデバイス境界に適用する。interleaved↔planar 変換には audio-core 実装済みの `AudioBuffer::from_interleaved` / `copy_to_interleaved` / `copy_from_interleaved` / `AudioBufferViewMut` を使う。コールバックは事前確保済みバッファへの `&mut` 借用でデータを受け渡す
* Patterns to avoid: cpal の型・エラー・`StreamTrait` 等を `pub` で再エクポートしない。feature 名に `cpal` 等の固有名詞を使わない（バックエンド有効化の意図を表す命名に限定）。コアクレートから `ruviform-audio-device` への強制依存を作らない。adapter→adapter 依存を作らない。コールバック内で alloc/dealloc・ブロッキング・I/O・無制限待機を行わない。全機能提供を目指して trait を肥大化しない（非対応機能は型/エラーで「非対応」を表現）
* Dependencies or configuration: `cpal` は `ruviform-audio-device` の内部依存として opt-in 追加（理由記録・`Cargo.lock` 追跡）。`realtime` feature 経由の `audio_thread_priority` 利用を検討する。ASIO 関連 feature は既定で無効のままとする

## Verification

* [x] `ruviform-audio-core` の公開 API（`cargo doc` 出力・公開 trait/型）に cpal 由来の型・エラー・feature 名が存在しない。全公開 enum に `#[non_exhaustive]` が付いている — `cargo doc --no-deps` 生成物の backend 名 grep 0 件（IR `implement-audio-core-device-port.md` Validation）。`exhaustive_enums` lint ゲートで強制
* [x] `ruviform-audio-device` を有効化しない構成でコアクレートが `cargo check`/`cargo build` に成功し、cpal 依存が `ruviform-audio-device` の `[dependencies]` にのみ記述されている（`cargo metadata` で確認） — IR `implement-audio-device-cpal-adapter.md` Validation 記録どおり実測
* [x] PoC が「列挙 → 既定出力構成 → 出力ストリーム開始 → コールバック到達（planar f32 への境界変換）」を実証し、手順が `docs/impl/exp/` に記録されている（実機依存部分は手動検証。CI はデバイス非依存テストのみ） — 実験ログ `device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl`: `list_devices`（WASAPI host・出力 5 台・既定 Audient iD4）と `play_sine`（`Fixed(480)` 拒否→`Default` 開通・496 コールバック・`RESULT: OK`）
* [x] コールバック内非 alloc が trait 契約として明示され、assert_no_alloc 相当またはコードレビュー証跡で検証されている — `OutputDataCallback`/`OutputErrorCallback` doc の RT 契約 + `rt_output_callback_path_allocates_nothing`（assert_no_alloc）が `mise run test-rt` で合格 + 実機 counting `#[global_allocator]` で streaming 窓 2 alloc イベント（プロセス全体上限値）
* [x] duplex 要件化時に第 2 backend を追加できる trait 形状になっている（バックエンド固有情報が trait シグネチャに混入していない） — `tests/mock_backend.rs` が非 cpal フェイク backend でポート全フローを `Box<dyn>` 経由で検証。trait 分解（Enumerator/Device/Stream）は設計 `DESIGN-034JcC9QK924kDbu4WhibC` どおり
* [x] `mise run fmt-check` / `check` / `lint` / `test` がパスし、`unsafe` は SAFETY コメント付き局所許可のみ — workspace 検証全パス。`unsafe` は `play_sine` example の計数 `#[global_allocator]`（局所 `#[expect(unsafe_code)]` + SAFETY コメント）のみで src/ tests/ には存在しない

## Pros and Cons of the Options

### cpal（採用）

* Good, because: pure Rust・依存木が薄く外部 C ライブラリ同梱なし。MSRV 1.85 で一致。主要プラットフォーム＋Android/WASM を単一依存でカバー。0.18 で timestamp（ハードウェアレイテンシ込み）・`ErrorKind` 統一・`StreamTrait::now()` が入り、`MasterClock`・エラー正規化要件に整合。`realtime` feature でスレッド RT 昇格。rodio/kira/firewheel-cpal/oddio/fundsp 等の採用実績が豊富
* Bad, because: コールバック `Data` は interleaved のみ（planar 変換は境界で必須）。duplex API 未実装（PR #1229 open）。WASAPI 排他モード・プラットフォーム個別設定 API が未確定。停滞実績があり実質単独メンテナ依存のリスクが残る

### cubeb-rs（保留）

* Good, because: Firefox 現役本番コードで Mozilla 公式メンテ。真の full-duplex（リクロック済み・レイテンシ補償済みオーディオクロック）。Tier-1 backend が充実
* Bad, because: `cubeb-sys` の cmake/C++ ビルド依存で依存木が重い・FFI=unsafe。PipeWire ネイティブ・WASM なし。コールバック `Send+Sync` 要求がやや厳しい

### rtaudio-rs（保留）

* Good, because: `Buffers` enum で真 duplex。対応 host API が広い（ASIO 含む）。firewheel-rtaudio が採用し trait 分離での交換実績がある
* Bad, because: RtAudio C++ の静的ビルドでツールチェイン依存。Android 非対応。GitHub リポジトリは Codeberg 移行済みでエコシステムの重心が散る

### プラットフォーム直結自前 adapter（保留・中長期）

* Good, because: RT 制御自由度が最大（自前スレッド＋`audio_thread_priority`）。CamillaDSP の先例。coreaudio-rs の AudioUnit render コールバック等は真のデバイスコールバック
* Bad, because: 3 OS 分の adapter を自前実装・保守するコストが大きい。wasapi/alsa はコールバック駆動ではなく自前スレッド＋event/poll が必要。pipewire は `!Send`/`!Sync` 制約。現フェーズの費用対効果に見合わない

### 不採用群（棄却）

* Bad, because: oboe-rs（停滞・Android 限定・cpal 脱却済み）、libsoundio 系（rsoundio=作者 abandonware 明記・soundio-rs=archived で cpal 推奨）、rust-portaudio（maintenance mode・cpal 推奨）、miniaudio（Rust ラッパ停滞・unsafe 境界が広い）、sdl3（ミキシング層経由で粒度粗い＋C 依存）、wavy（未完成停滞）、nannou_audio（cpal ラッパでしかない）

## More Information

* 一次ソース: GitHub Issue [#23 [poc] オーディオデバイスバックエンド評価（cpal 等）と採否](https://github.com/scarecrow173/ruviform/issues/23)
* 上流ドキュメント: discovery [evaluate-audio-device-backends.md](../discovery/evaluate-audio-device-backends.md)（評価軸・候補比較・要 PoC 項目）、並行 spec [audio-device-backend-evaluation-poc.md](../specs/audio-device-backend-evaluation-poc.md)（実施範囲・受け入れ基準）
* 関連決定: [media-io-backend-abstraction.md](media-io-backend-abstraction.md)（抽象境界規則の先行決定）、[realtime-contract-and-verification-policy.md](realtime-contract-and-verification-policy.md)（RT 契約・検証方針）、[platform-and-toolchain-targets.md](platform-and-toolchain-targets.md)（対象プラットフォーム・MSRV）、[public-enum-non-exhaustive-policy.md](public-enum-non-exhaustive-policy.md)（公開 enum 方針）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（責務表・分類表）
* 参考資料: [cpal](https://github.com/RustAudio/cpal)（[issue #981: メンテナンス停滞の経緯](https://github.com/RustAudio/cpal/issues/981)、[PR #1229: duplex API](https://github.com/RustAudio/cpal/pull/1229)）、[cubeb-rs](https://github.com/mozilla/cubeb-rs)、[rtaudio-rs](https://github.com/MeadowlarkDAW/rtaudio-rs)、[Firewheel](https://github.com/BillyDM/Firewheel)（engine↔デバイス trait 分離による firewheel-cpal/firewheel-rtaudio 交換の先例）、[audioadapter-rs](https://github.com/HEnquist/audioadapter-rs)（interleaved↔planar 変換ユーティリティ）、[audio_thread_priority](https://crates.io/crates/audio_thread_priority)（RT スレッド昇格）
