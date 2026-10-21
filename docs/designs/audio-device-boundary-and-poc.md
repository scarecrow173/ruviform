---
id: "DESIGN-034JcC9QK924kDbu4WhibC"
type: "design"
status: "approved"
title: "オーディオデバイス境界 trait と ruviform-audio-device PoC の詳細設計"
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
  implements:
    - "../specs/audio-device-backend-evaluation-poc.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../discovery/evaluate-audio-device-backends.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/audio-device-backend-evaluation-poc.md"
    - "../adr/select-audio-device-backend.md"
  derived-by:
    - "../specs/chunk-timestamp-synthesis.md"
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
    - "chunk-timestamp-design.md"
    - "../plans/implement-issue-59-chunk-timestamp.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "https://github.com/RustAudio/cpal"
  defers: []
  deferred-by: []
---

# オーディオデバイス境界 trait と ruviform-audio-device PoC の詳細設計

## Context

GitHub Issue #23（`relations.source`）の design フェーズ成果物として、承認対象の spec [audio-device-backend-evaluation-poc.md](../specs/audio-device-backend-evaluation-poc.md)（`SPEC-034Jc9QEqYLVfg1CXXTiPV`、proposed）と採否 ADR [select-audio-device-backend.md](../adr/select-audio-device-backend.md)（`ADR-034Jc9RslnH7QSQ1nZEEHL`、proposed・cpal 条件付き採用）を詳細設計に落とす。評価軸・候補比較の正本は discovery [evaluate-audio-device-backends.md](../discovery/evaluate-audio-device-backends.md) にある。

設計ベースライン（[ruviform-architecture-design-baseline.md](ruviform-architecture-design-baseline.md)）が確定した責務配分: **ポート（デバイス境界 trait）の定義所有は `ruviform-audio-core`**、**`ruviform-audio-device`（adapter）がそれを実装し cpal 依存を内部に閉じ込める**。`ruviform-audio`（engine・未作成）が RT コールバック配線とセッション調停を担い、engine↔adapter の直接依存は構造的に排除される（adapter はポート定義元である audio-core にのみ依存する）。

本設計で確定させるもの:

1. `ruviform-audio-core` の `device` モジュールに置くポート API の具体的な形状（trait 名・メソッドシグネチャ・値型・エラー型）
2. `ruviform-audio-device` クレートの構成（マニフェスト・モジュール分割・cpal 依存の閉じ込め方・コールバック配管）
3. PoC の検証設計（CI に置けるデバイス非依存検証・実機手動検証手順・非 alloc 検証方針）
4. PoC 結果を採否 ADR に反映して `accepted` へ昇格（または条件見直し）する確定計画

設計入力として確認済みの実コード事実: `AudioBuffer`/`AudioBufferView`/`AudioBufferViewMut`/`BufferSpec`（planar チャンネルメジャー、`copy_to_interleaved`/`copy_from_interleaved`/`from_interleaved` が RT-suitable な境界変換）、`AudioFormat`/`SampleFormat`（`#[non_exhaustive]`、F32/F64）、`ChannelLayout`（variant が channel count を所有、`Custom(SpeakerMaskSet)` は非空不変）、`SampleRate`/`SampleIndex`（整数時間契約）、`ruviform_media_core::Error`（Copy + `&'static str` payload の RT-suitable エラー）、`MediaTime`/`TimeBase`/`MediaDuration`（有理数時刻）、`RtContext`（`ruviform-rt`、mint API 未実装のプレースホルダ）。

cpal API の記述（`Host`/`Device`/`Stream`、`build_output_stream`、0.18 の `StreamTrait::now()`・`ErrorKind` 統一等）は briefing 時点の公開情報ベースであり、**個々のシグネチャ確証は実装時に `cargo doc` / ソースで確認する（各所に「要実装時確認」と明記）**。

## Scope

### In Scope

- `ruviform-audio-core` への `device` モジュール追加: デバイス境界 trait（ポート）群と関連値型・エラー型の定義。デバイス列挙・既定出力デバイス取得・構成記述・出力ストリーム open/start/stop・ストリーム時刻/タイムスタンプ取得・エラー正規化・コールバック RT 契約の明示をカバーする
- `AudioBufferViewMut` への公開コンストラクタ追加（可変フレームのコールバックバッファを事前確保スクラブ上に構成するため。audio-core 側の小さな拡張）
- `ruviform-audio-device` クレート新設と cpal adapter の PoC 実装設計（列挙 → 既定出力構成 → 出力ストリーム開始 → コールバック到達＋interleaved→planar f32 境界変換まで）
- PoC 検証設計: CI 収容可能なデバイス非依存テスト（object safety・マッピング単体テスト・モックバックエンド）、実機手動検証手順（examples）、コールバック内非 alloc の検証方針、評価レポートの記録先（`docs/impl/exp/` 実験ログ）
- 採否 ADR の確定計画（Confirmation 項目 → PoC 証跡の対応付け、`accepted` 昇格 or 条件見直しのトリガ）
- ワークスペース帳簿の更新（クレート新設に伴う実施事項の列挙）: ルート `Cargo.toml` の `members` への `crates/ruviform-audio-device` 追加、`AGENTS.md` のワークスペース構成記述の同期、必要に応じ `docs/designs/overview.md` 索引の更新

### Out of Scope

- duplex（同期入出力）の実装・入力ストリーム API の実装（trait 形状として将来の追加を阻害しないことのみ要求）
- エンジン層 `ruviform-audio` のセッション調停・RT コールバック配線本体・`Processor` グラフ駆動
- `RtContext` mint API（RT 基盤 Issue の範囲）
- 実機ループバック試験の CI 化・`assert_no_alloc` 機構の CI 標準装備（RT 検証 Issue の範囲。本 PoC では試行と証跡記録のみ）
- ASIO 対応（opt-in 隔離のまま。build.rs の SDK 自動ダウンロードは再現性注意点として記録のみ）
- cubeb/rtaudio/プラットフォーム直結 adapter の実装（採否 ADR で保留・棄却済み）
- `ChannelLayout` に arbitrary channel count を表現する variant の追加（後述の Deferred Concern）

## Components and Boundaries

### 全体構造

```text
ruviform-audio (engine・未作成)              ruviform-audio-device (adapter・本 Issue で新設)
        │                                          │
        │  Box<dyn DeviceEnumerator/Device/        │  impl DeviceEnumerator/Device/OutputStream
        │      OutputStream>（audio-core のポート） │  内部: cpal::{Host,Device,Stream}
        ▼                                          ▼
ruviform-audio-core::device ──────────────►  cpal 0.18 ──► OS host API
  （ポート trait・値型・DeviceError の定義所有）        （WASAPI/CoreAudio/ALSA/PipeWire…）
```

依存方向: `ruviform-audio-device` → `ruviform-audio-core`（→ `ruviform-media-core`）。adapter→adapter 依存なし、engine↔adapter 直接依存なし（engine はポート経由でのみ接続）。cpal 型・エラー・feature 名は `ruviform-audio-device` の内部に留め、audio-core（ポート定義元）の公開 API 面にも adapter 自身の公開 API 面にも露出しない（media-io ADR の抽象境界規則をデバイス境界に適用。adapter クレートの内部 API は安定性契約の直接対象外だが、保守の一貫性のため公開面にも cpal 型を出さない方針を取る）。

### 設計方針（ポート API 共通）

- **object safety**: engine がバックエンドを差し替えて使うため、ポート trait はすべて object-safe とし、`Box<dyn ...>` で受け渡す。ジェネリックメソッド・`Self` 返却・関連型を持たない。コールバックは `Box<dyn FnMut ...>` で渡す（トレイトオブジェクト経由でも object safety が保たれる）
- **trait 分解**: 列挙・デバイス・ストリームの 3 trait に分ける。ライフサイクル（列挙は一過性、デバイスハンドルは軽量参照、ストリームは OS リソースとコールバックを所有）とスレッド所在（列挙・open/start/stop は制御スレッド＝非 RT、データコールバックはバックエンドの RT スレッド）が異なるため 1 つの god-trait にしない。cpal の `Host`/`Device`/`Stream`、cubeb の context/stream、WASAPI の device/client という各バックエンドの実モデルとも同型で、第 2 backend 追加時に無理がない
- **Send 境界**: `Device` と `OutputStream` は `Send` を要求する（制御スレッドで生成し別スレッドへ引き渡せること）。`Sync` は要求しない（バックエンド間でばらつきがあることが判明している・要実測）。`DeviceEnumerator` は制御スレッドオブジェクトとしてスレッド境界を課さない
- **「非対応」の表現**: 欠落は `Option`（値が存在しない）・enum variant（`BufferSize::Default` / `SupportedBufferSize::Unknown`）・`DeviceError::{Unsupported, UnsupportedConfig}`（機能・構成の非対応）で型として表現する。bool フラグや「未実装で panic」は使わない。全機能を一律汎用化しない（media-io ADR / baseline L269, L448）
- **RT 契約の所在**: メソッドごとに `**RT contract:**` ラベルで保証範囲を明示する（audio-core 既存の慣例と realtime ADR に一致）。データコールバック内は RT 契約区間、それ以外は非 RT API

### `ruviform-audio-core` の `device` モジュール（ポート定義）

新設 `src/device.rs` を `pub mod device` として公開する。ルートへの flat re-export は行わず `ruviform_audio_core::device::*` の名前空間でポート群を束ねる（項目数が多く・「境界」というまとまりを名前で表現するため。既存の flat re-export 方針からの逸脱は意図的）。

```rust
//! device.rs — audio device boundary ports (in ruviform-audio-core).

/// デバイス列挙・既定デバイス取得のエントリポイント（非 RT）。
/// adapter が実装する（cpal `Host` 相当）。制御スレッド上のオブジェクト。
pub trait DeviceEnumerator {
    /// 現在存在する出力可能デバイスのハンドル列。0 台なら空 Vec。
    /// 非 RT: OS API を呼び・確保を伴う。
    fn output_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>>;

    /// システム既定の出力デバイス。未設定/不存在は `Ok(None)`
    /// （「無し」を型で表現。エラーではない）。
    /// 非 RT。
    fn default_output_device(&self) -> DeviceResult<Option<Box<dyn Device>>>;
}

/// 1 台の出力デバイスへの生存ハンドル（非 RT）。
/// `Send`: 制御スレッド以外へ引き渡せる。`Sync` は要求しない。
pub trait Device: Send {
    /// 表示用デバイス名（確保を伴うため非 RT。識別は `id` も参照）。
    /// 非 RT。cpal 0.18 の対応 API は `description()`（構造化
    /// `DeviceDescription`）と `Display`（`device.to_string()`）であり、
    /// どちらを写像先にするかは要実装時確認。
    fn name(&self) -> DeviceResult<String>;

    /// バックエンド固有の不透明なデバイス識別子。バックエンドが
    /// 安定 ID を持たない場合は `Ok(None)`（非対応を型で表現）。
    /// cpal 0.18 の `id()` は `Result<DeviceId, Error>` を返すため、
    /// 照会自体の失敗（列挙後の消失等）は `Err` で返し `None`
    /// （非対応）と区別する。非 RT。
    fn id(&self) -> DeviceResult<Option<DeviceId>>;

    /// デバイス既定の出力フォーマット。
    /// adapter は「既定構成 + サポート空間」を考慮し、open 可能な
    /// `SampleFormat::F32` の形式を優先して返す（既定が F64/整数既定でも
    /// F32 レンジが存在すればそこから合成した `AudioFormat` を返す —
    /// 整数既定デバイスで利用側の open フローが即 `UnsupportedConfig`
    /// に落ちないよう、交渉の第一責務を adapter に置く設計判断）。
    /// F32 に写像可能な形式が皆無の場合、または既定構成のチャンネル数が
    /// `ChannelLayout` に写像不能な場合は
    /// `Err(DeviceError::UnsupportedConfig)`（後述の制限）。非 RT。
    fn default_output_format(&self) -> DeviceResult<AudioFormat>;

    /// サポートされる出力フォーマット空間（範囲記述の列）。
    /// 構成交渉の入力。PoC では F32 に写像可能なレンジのみを報告する
    /// （整数のみのレンジは adapter が除外し「f32-capable 空間」を返す。
    /// 空 Vec は「F32 対応レンジ無し」であり、「サポート情報を照会
    /// できない」失敗とは `Err` で区別される）。非 RT。
    fn supported_output_formats(&self) -> DeviceResult<Vec<SupportedOutputFormat>>;

    /// 出力ストリームを open する。`config` は交渉済みの具体構成。
    /// 非 RT（OS リソース確保・スレッド起動を伴う）。コールバックは
    /// `start()` 後にバックエンドの RT スレッドで発火する。
    /// `open_timeout` 相当の引数は v1 では持たず adapter が backend
    /// 既定に委ねる（必要になれば `OutputStreamConfig` のフィールド
    /// 追加で吸収する内部変更。publish = false のため許容）。
    fn open_output_stream(
        &self,
        config: &OutputStreamConfig,
        data_callback: OutputDataCallback,
        error_callback: OutputErrorCallback,
    ) -> DeviceResult<Box<dyn OutputStream>>;
}

/// open 済み出力ストリーム。Drop で送出停止（バックエンド定義。
/// cpal は drop で停止）。`Send`。
pub trait OutputStream: Send {
    /// 送出開始/再開。非 RT（OS 呼出・短時間ブロックしうる）。
    fn start(&self) -> DeviceResult<()>;
    /// 送出停止。非 RT。
    fn stop(&self) -> DeviceResult<()>;
    /// 実際に適用されたフォーマット（open 後の真値）。
    /// RT-suitable — 保存済み値の返却。
    fn format(&self) -> AudioFormat;
    /// 実際のコールバックバッファサイズをバックエンドへ照会する。
    /// cpal `StreamTrait::buffer_size() -> Result<FrameCount, Error>`
    /// の写像: `Ok(n)` → `Ok(BufferSize::Fixed(n))`（バックエンド報告の
    /// 推定値。`Default` 指定時も cpal は設定済み既定値を返す）、
    /// 照会不能（`UnsupportedOperation` 等）→ `Err`。
    /// 設計判断: 照会失敗と「バックエンド既定」を混同しないため
    /// `DeviceResult` で返す（保存済み要求値のエコーは情報を持たない）。
    /// 報告値は推定であり保証された上限ではない — 実フレーム数は
    /// コールバックの `out.frames()` / `OutputCallbackInfo` 側で観測する。
    /// 非 RT（バックエンド照会を伴いうる。要実装時確認）。
    fn buffer_size(&self) -> DeviceResult<BufferSize>;
    /// デバイス時計の現在値（`MasterClock` 配線用）。cpal `now()` の
    /// `StreamInstant` を `as_nanos()` → u128→i64 飽和チェック付きで
    /// `MediaTime` へ変換する（飽和時は `Err`）。
    /// RT-suitable と見なせるかはバックエンド依存のため、PoC では
    /// 制御経路 API として扱う（コールバック内では `OutputTimestamp`
    /// を使う方が確実）。要実機確認。
    fn now(&self) -> DeviceResult<MediaTime>;
}
```

値型（すべて `Copy`/`Clone` の値型か不透明 newtype。公開 enum は enum レベル `#[non_exhaustive]`）:

```rust
/// 出力ストリームの open 構成（交渉済みの具体値）。
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutputStreamConfig {
    /// ストリームフォーマット（rate + layout + sample encoding）。
    pub format: AudioFormat,
    /// 要求するコールバックあたりフレーム数。
    pub buffer_size: BufferSize,
}

/// コールバックあたりフレーム数の要求/報告値。
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum BufferSize {
    /// バックエンドが選ぶ（既定。コールバックごとの実フレーム数は
    /// 変動しうる → 可変フレーム対応は adapter の責務）。
    #[default]
    Default,
    /// 固定フレーム数の要求。尊重されるかはバックエンド依存
    /// （cpal `BufferSize::Fixed` の尊重度は要実機確認の未確定点）。
    /// adapter 責務: cpal 側が `u32` のため `usize`→`u32` の縮小
    /// キャストで溢れる値と `Fixed(0)` は open 前に
    /// `Err(DeviceError::UnsupportedConfig)` で事前検証する。
    Fixed(usize),
}

/// サポートされるバッファサイズ能力。
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SupportedBufferSize {
    /// バックエンドが範囲情報を報告しない。
    Unknown,
    /// 包含 min/max フレーム数。`min <= max` が不変条件 — adapter は
    /// バックエンド報告値の写像時に検証（assert）し、違反レンジは
    /// 除外するか `Unknown` に倒す。
    Range { min: usize, max: usize },
}

/// デバイスのサポート出力フォーマット空間の 1 エントリ（範囲記述）。
/// adapter が生成する非 RT 記述型。チャンネル数はカウントとして保持
/// し、`ChannelLayout` への写像は交渉時の責務（後述）。
#[derive(Clone, Debug, PartialEq)]
pub struct SupportedOutputFormat {
    /// チャンネル数。
    pub channels: u32,
    /// サポート下限サンプルレート（Hz・包含）。
    pub min_sample_rate: u32,
    /// サポート上限サンプルレート（Hz・包含）。
    pub max_sample_rate: u32,
    /// バッファサイズ能力。
    pub buffer_size: SupportedBufferSize,
    /// PCM サンプルエンコーディング。
    pub sample_format: SampleFormat,
}

/// バックエンド固有のデバイス識別子（不透明 newtype）。
/// 文字列形の内部表現は実装詳細。表示・永続化のみに使い、内容を
/// 解釈しない。非 RT 経路でのみ取得される（`Device::id`）。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeviceId(/* opaque */ String 等);

impl DeviceId {
    /// バックエンド報告の識別子文字列から構成する公開コンストラクタ
    /// （adapter が構築するための入口。`From<String>` 実装でもよい）。
    /// 非 RT — 確保を伴う。
    pub fn new(id: String) -> Self { /* ... */ }
}

/// デバイス時計インスタントの時刻表現に使う timebase 規約:
/// `TimeBase(1, 1_000_000_000)`（ナノ秒単調時計。起点は
/// バックエンド定義で、インスタント間差分のみに意味を持つ）。
/// `MediaTime` 再利用により `MasterClock` への受け渡しに新しい
/// 時刻型を持ち込まない（f64 秒を中核にしない契約とも整合）。
/// `TimeBase` は private フィールドのため `TimeBase::new` の const
/// 評価可否（`Result` の const `unwrap` を含む）が宣言形を決める —
/// const 不可なら `fn` または `OnceLock` で提供する。実装時確認。
pub const DEVICE_CLOCK_TIMEBASE: TimeBase = TimeBase::new(1, 1_000_000_000);
```

コールバック関連型:

```rust
/// 1 回の出力コールバックに付随するデバイス時計タイムスタンプ。
/// RT-suitable — `Copy` 値型。adapter がコールバック内でスタック上に
/// 構成する（確保しない）。
#[derive(Copy, Clone, Debug)]
pub struct OutputTimestamp {
    /// コールバックが発火した時点のデバイス時計インスタント。
    /// `DEVICE_CLOCK_TIMEBASE`（ナノ秒）の `MediaTime`。
    pub callback: MediaTime,
    /// このコールバックで書かれたバッファが DAC に到達する時点
    /// （バックエンド報告のハードウェアレイテンシ込み）。
    /// バックエンドが報告できない場合 `None`（非対応を型で表現）。
    /// `MasterClock` は `playback` 優先、無ければ `callback` に
    /// フォールバックする方針（配線は engine 側の責務）。
    pub playback: Option<MediaTime>,
}

/// 出力データコールバックへ渡す実行時情報コンテナ。
/// RT-suitable — `Copy` 値型。将来の per-callback 情報追加は
/// シグネチャ変更ではなくフィールド追加で吸収するための入れ物
/// （`RtContext` 配線については「コールバック形状と `RtContext` の
/// 扱い」節を参照）。注意: 非 `Copy` なフィールドの追加で `Copy`
/// 適合は失われうる — 追加するフィールドは RT 契約を保つ値型に
/// 限る前提とする。
#[derive(Copy, Clone, Debug)]
pub struct OutputCallbackInfo {
    /// このコールバックのデバイス時計タイムスタンプ。
    pub timestamp: OutputTimestamp,
}

/// 出力データコールバック。`open_output_stream` に渡され、
/// `start()` 後にバックエンドの RT スレッドで駆動される。
///
/// **RT contract:** コールバック内は RT 契約区間 — alloc/dealloc
/// 禁止・ブロッキング/ファイル・ネットワーク I/O/無制限待機/非有界
/// 処理の制限（realtime ADR）。`out` は adapter が事前確保した
/// planar スクラブ上に構成した有界 view（フレーム数はコールバック
/// ごとに `out.frames()` で得る。`BufferSize::Default` では変動する）。
/// `Send`: バックエンドスレッドへ移送される。
///
/// **ライフサイクル・利用契約（実装側の義務）:**
/// - コールバックは `out` の全フレームを毎回書き込む義務を持つ。
///   スクラブは使い回されるため、書き残しは前回コールバックの
///   ステイルデータとしてそのまま送出される。adapter は安全側
///   デフォルトとして呼出前に露出領域をゼロフィルしてよい
///   （有界 memset で RT-suitable）。
/// - コールバックオブジェクトの Drop 位置はバックエンド定義であり、
///   RT スレッド上で実行されうる。キャプチャする値の Drop は
///   確保・ブロッキングを伴わないものに留め、重い破棄が必要な構造は
///   engine 側で別スレッドへ移送して破棄する（drop-offload）指針。
/// - コールバック内で panic してはならない（バックエンド RT
///   スレッド上の panic は未定義挙動/プロセス異常終了につながりうる）。
/// - コールバック内から `OutputStream` の制御 API（`start`/`stop`/
///   drop）を再入呼出しすることは未定義 — 禁止。
pub type OutputDataCallback = Box<
    dyn for<'a> FnMut(AudioBufferViewMut<'a, f32>, OutputCallbackInfo) + Send + 'static,
>;

/// ストリームエラー通知コールバック（xrun 等）。
/// バックエンドのストリームスレッド上で呼ばれうるため RT-adjacent として
/// 扱う — `DeviceError` が `Copy` なのはこの経路で確保させないため。
/// 実装側は確保・ブロッキングを行わないこと。
/// 非致命アドバイザリ（`Backend("device_changed")` — 既定デバイス由来
/// ストリームの自動再経路）が届きうる: ストリームは継続しており
/// 再構築不要であり、engine はエラー停止ではなくガイダンスとして扱う。
pub type OutputErrorCallback = Box<dyn FnMut(DeviceError) + Send + 'static>;
```

エラー型（独自 enum を採用 — 根拠は下記「エラー型の選択」節）:

```rust
/// デバイス境界の正規化エラー型。バックエンド固有エラーは
/// adapter がこの型へ写像し、外部エラー型を露出しない。
/// `Copy` + `&'static str` payload で RT 経路でも確保せず作れる
/// （media-core `Error` と同じ手法）。
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeviceError {
    /// 該当デバイスが存在しない/列挙後に消失した。
    DeviceNotFound,
    /// デバイスまたはバックエンドがビジー（制御スレッドで再試行可能）。
    DeviceBusy,
    /// 要求した構成（フォーマット・レイアウト・バッファサイズ）が
    /// デバイスに非対応。
    UnsupportedConfig,
    /// バックエンドが当該機能自体を非対応とする
    /// （例: タイムスタンプ・duplex・ホスト選択）。
    Unsupported,
    /// バッファ under/overrun（xrun）。
    Xrun,
    /// RT スレッド優先度昇格が拒否された。
    RealtimeDenied,
    /// ストリームがバックエンド側で無効化された（デバイス抜去等）。
    StreamInvalidated,
    /// 上記に写像できないバックエンド失敗。確保しない
    /// `&'static str` の分類名のみ保持し、バックエンドの
    /// メッセージ文字列は取り込まない。
    Backend(&'static str),
}

/// `device` モジュールの Result エイリアス。
pub type DeviceResult<T> = core::result::Result<T, DeviceError>;

// media-core `Error` と同等に標準エラー契約を実装する
// （`Backend` は分類名をそのまま出力。確保しない固定文言のみ）。
impl core::fmt::Display for DeviceError { /* variant ごとの固定文言 */ }
impl core::error::Error for DeviceError {}
```

#### trait 分解の理由と却下案

- **採用**: `DeviceEnumerator`（列挙・既定取得）→ `Device`（情報・構成空間・open）→ `OutputStream`（start/stop・実測値・時計）の 3 分離。ライフサイクルとスレッド所在の差に対応し、cpal/cubeb/WASAPI の実モデルと同型
- **却下**: 単一 `AudioBackend` god-trait — 列挙用ハンドルとストリーム所有が混在し、`Send` 境界・Drop 意味論・ライフタイムの記述が曖昧になる。バックエンド差し替え時に全部を再実装させる
- **却下**: 関連型を持つ 1 trait（`type Device; type Stream`）— `dyn` 利用を複雑化し、engine が `Box<dyn>` で扱う前提に合わない
- **却下**: デバイスを ID/値型のみで表現し enumerator が内部レジストリを持つ方式 — cpal が `Device` オブジェクトに構成問い合わせと open をぶら下げるモデルと合わず、adapter 内に生存期間管理の層を余計に作る
- **却下**: spec 記述の構成表現どおり `BufferSpec` を open 構成に使う案 — `BufferSpec` の `frames` は固定値を持ち `BufferSize::Default`（バックエンド選択）を表現不能であり、レート/レイアウトは `AudioFormat` と情報重複するため、`OutputStreamConfig { format: AudioFormat, buffer_size: BufferSize }` の組に置き換えた
- **将来拡張の置き場**: 入力・duplex は本 Issue では定義しないが、メソッド名を `output_*` 系統に揃えてあるため、将来 `input_devices()` / `open_input_stream()` / `InputCallbackInfo` 等を追加しても既存シグネチャを変更しない（追加は実装側にとって破壊的だが workspace 内部のため吸収可能）。duplex 用の `open_duplex_stream`（第 2 backend で実装）も同様に追加型で吸収できる

#### コールバック形状と `RtContext` の扱い（重要な決定）

- コールバックは **`AudioBufferViewMut<'a, f32>` を値渡し**で受け取る。ベースラインの例（`out: &mut AudioBuffer`）からの意図的変更: コールバックに必要な capability は「事前確保済み領域への書込み＋形状の問い合わせ」であり、所有権を持つ `AudioBuffer` ではなく借用 view がこの概念に正確に対応する。view は RT-suitable な借用のみを持ち、確保系コンストラクタが型上到達できない
- **`RtContext` パラメータは置かない**。根拠: (a) ベースラインの発行規則では `RtContext` を発行するのは RT 区間を開始した駆動者＝`ruviform-audio`（adapter のコールバック内で発行）であり、ポート経由で adapter が `RtContext` を渡す形は発行主体モデルと矛盾する、(b) mint API は `ruviform-rt` 未実装で本 Issue のスコープ外、(c) `publish = false` のワークスペース内部 API のため、将来 mint が実装されてもシグネチャ追加または `OutputCallbackInfo` フィールド追加で対応可能であり「配線を阻害しない」要件は情報コンテナの用意で満たせる。engine が mint に必要な情報（デバイスタイムスタンプ）は `OutputCallbackInfo` が既に運ぶ
- `OutputCallbackInfo` を裸のタイムスタンプではなく struct にしたのはこの拡張点を明示するため（フィールド追加は内部変更として吸収可能）

#### エラー型の選択（`media_core::Error` 再利用 vs `DeviceError`）

- **採用: audio-core 独自の `DeviceError`（`#[non_exhaustive]` enum + `DeviceResult` エイリアス）**
- 根拠: (a) `ruviform_media_core::Error` の variant は有理数時刻・値域の話題（`InvalidTimeBase`/`TimeOverflow`/`InvalidValue`/`UnsupportedDropFrameRate`）で、デバイス列挙・ストリーム制御・xrun といった境界エラーのタクソノミを Primitives 層の media-core に混入させるのは責務違反、(b) エラー正規化はポート定義元（audio-core）の責務であり、正規化先もポートと同居させる方が変更の局所性が高い、(c) `ErrorKind`（Xrun/DeviceBusy/RealtimeDenied/StreamInvalidated 等）への写像はデバイス固有の語彙を要求する
- `Copy` + `&'static str` payload の手法（確保しないエラー）は media-core `Error` の実績あるパターンを踏襲し、RT-adjacent の `OutputErrorCallback` 経路でも確保せず生成できる。`Backend(&'static str)` は「その他」を分類名だけで表現し、バックエンドのメッセージ文字列（確保を伴う）を取り込まない

#### `AudioBufferViewMut` 公開コンストラクタの追加（audio-core 側の付随変更）

現状 `AudioBufferViewMut` は `AudioBuffer::view_mut()` 経由でのみ得られ、frames は構築時固定。cpal の `Data`（`&mut [f32]` interleaved）は `BufferSize::Default` でコールバックごとに可変長になりうるため、「最大容量で事前確保したスクラブ上に、そのコールバックの実フレーム数 n の view を毎回構成する」能力が adapter に必要。

```rust
impl<'a, S: Sample> AudioBufferViewMut<'a, S> {
    /// `data`（channel-major slab、`data.len() == channels * frames`）
    /// 上の可変 view を構成する。借用のみ・確保なし。
    /// RT contract: RT-suitable。
    /// Error: `channels == 0` または長さ不一致で `Error::InvalidValue`。
    /// 命名は buffer.rs の規約（`new` = panic 系 / `try_new` = `Result`
    /// 返却）に整合させ `try_new` とする。
    pub fn try_new(data: &'a mut [S], channels: usize, frames: usize) -> Result<Self>;
}
```

`channels` を `ChannelLayout` ではなく `usize` で受けるのは view が既にレイアウトを持たない設計（`channels`/`frames` のみ）と一致させるため。これは既存不変条件（view は連続スラブ上の有界借用）を変えない最小の拡張であり、`AudioBuffer` の「構築後に伸長しない」不変条件には触れない。

### `ruviform-audio-device` クレート構成

```text
crates/ruviform-audio-device/
├── Cargo.toml
├── src/
│   ├── lib.rs       # クレート doc（責務・許可依存層・RT 契約の転記）+ 公開面 re-export
│   ├── host.rs      # Host（cpal::Host のラッパ）→ DeviceEnumerator 実装、HostId
│   ├── device.rs    # cpal::Device → Device 実装（name/id/format 問い合わせ・open）
│   ├── stream.rs    # cpal::Stream → OutputStream 実装 + コールバック配管
│   ├── convert.rs   # 設定マッピング・交渉・レイアウト推論・interleave 書込の純粋関数群
│   └── error.rs     # cpal 各エラー型 → DeviceError の正規化関数群
└── examples/
    ├── list_devices.rs  # ホスト/デバイス/既定構成の列挙ダンプ（非 RT 診断）
    └── play_sine.rs     # 既定出力に sine を送るストリーム PoC（手動検証）
```

マニフェスト（方針）:

```toml
[package]
name = "ruviform-audio-device"
description = "Audio device I/O adapter for Ruviform (device boundary port implementation; cpal backend)"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
ruviform-audio-core = { path = "../ruviform-audio-core" }
# `MediaTime`/`TimeBase` を adapter が直接利用するため追加
# （baseline の adapter→media-core 依存方向は合法。audio-core の device
# モジュールから再エクスポートする案も検討したが、依存方向が同一で
# ある以上、利用箇所を明示する直接依存を採用）。
ruviform-media-core = { path = "../ruviform-media-core" }
cpal = "0.18"   # 0.18 系: 統一 Error/ErrorKind・StreamTrait::now()・HW レイテンシ込み
                # timestamp・PipeWire/PulseAudio ネイティブ。MSRV 1.85 適合は実装時確認
                # cpal 0.18.2 は default = []（OS 既定 host のみ。Linux=ALSA）

[features]
default = []
# コールバックスレッドの RT 優先度昇格（cpal `realtime` → audio_thread_priority）。
# opt-in の意図命名。固有名詞ではないため規約適合。
# cpal の `realtime` は Windows/Android にのみ作用する。Linux の RT 昇格は
# `realtime-dbus`（rtkit 経由・要 `libdbus-1-dev` システム依存）であり、
# 必要になれば別 feature（例: `realtime-dbus`）として追加する —
# 本 PoC では Linux RT 昇格は未検証として扱う。
realtime = ["cpal/realtime"]
# PipeWire host の任意実機比較用（discovery の「ALSA vs PipeWire」確認項目）。
# PoC 既定構成では無効。有効化には `libpipewire-0.3-dev` システム依存が必要で、
# PipeWire ストリームが `!Send` になりうるリスクは要実機確認。
pipewire-backend = ["cpal/pipewire"]
# 注: cpal `pulseaudio` backend は MSRV 1.88 > workspace MSRV 1.85 のため配線不可。

[lints]
workspace = true
```

- **cpal 依存の閉じ込め方式 — 直接依存を採用**。検討した optional 依存+feature 案（`features.cpal = ["dep:cpal"]`）は却下: media-io ADR の「分離クレート+opt-in」はクレート単位の規則であり、`ruviform-audio-device` 自身が「cpal バックエンド」という opt-in 単位。クレート内に空の cfg 分岐を作ると adapter が事実上「cpal なしでビルドできる無内容クレート」になり、存在意義と検証（`cargo check` が中身を型検査しない）を損なう。第 2 backend（cubeb 等）は別クレート（例: `ruviform-audio-device-cubeb`）を追加する形であり、同一クレート内の複数バックエンド feature 切替は行わない — バックエンド選択はクレート/feature 依存で表現し実行時文字列指定にしない規約と整合
- **`cpal` の型を公開 API に出さない**: `pub use cpal::*` 等の再エクスポート禁止、`HostId` は `pub struct HostId(cpal::HostId)` のように private フィールドで包む（cpal 型がシグネチャに現れない）。`Host`/`Stream` 等の公開型は cpal ハンドルを private 保持し、ポート trait を実装する
- **ホスト選択**: `default_host()`（OS 既定 host）に加え、`available_host_ids() -> Vec<HostId>` + `host(id: HostId) -> DeviceResult<Host>` + `Host::name() -> &'static str` / `HostId::name() -> &'static str` を最小限公開する。`available_host_ids` が返すのは有効化された backend feature に対応する host id のみであり、既定構成（`default = []`）では PipeWire host id は列挙されない。Linux での ALSA vs PipeWire 挙動差は discovery が挙げた PoC 確認項目であり、host 選択はその実測（`pipewire-backend` 有効時の任意検証）に必要（要実装時確認: cpal `available_host_ids`/`host_from_id`/`Host::id().name()` の正確な形）
- **opt-in host feature と環境前提**: cpal 0.18.2 は `default = []` で、`pipewire`/`pulseaudio`/`jack`/`asio`/`realtime`/`realtime-dbus` は全て opt-in（docs.rs で確認済み）。PoC 既定構成は default host のみ（Linux=ALSA / Windows=WASAPI / macOS=CoreAudio）に限定し、PipeWire 比較は `pipewire-backend` feature の定義＋実機任意検証とする設計判断（理由: discovery の比較項目を潰さずに既定ビルドのシステム依存と `!Send` リスクを増やさないため）。Linux ビルドは ALSA backend に `libasound2-dev` のシステム依存が必須。`pulseaudio` backend は MSRV 1.88 > workspace MSRV 1.85 のため配線不可。`jack`/`asio`/`realtime-dbus`（rtkit・`libdbus-1-dev` 必要）も本 PoC では配線しない
- **feature 命名**: ポート側（audio-core）は feature を持たない。adapter 側の `realtime` は「バックエンド有効化の意図」命名の範囲内（固有名詞を含まない）。`pipewire-backend` は opt-in backend 有効化を示す adapter 固有 feature（cpal feature 名をそのまま公開面にせず `-backend` 接尾辞で意図を表す）。ASIO 関連（cpal `asio` feature）は配線しない・既定無効のまま隔離し、build.rs の SDK 自動ダウンロード（`CPAL_ASIO_DIR` で回避可）は実験ログに再現性注意として記録する

`lib.rs` の公開面（概念）:

```rust
pub struct Host { /* private: cpal::Host */ }
impl DeviceEnumerator for Host { /* output_devices / default_output_device */ }
impl Host {
    /// バックエンド表示名（`"ALSA"`/`"WASAPI"` 等）。
    pub fn name(&self) -> &'static str { /* ... */ }
    /// このホストの `HostId`。
    pub fn id(&self) -> HostId { /* ... */ }
}

#[derive(Clone, Debug)]
pub struct HostId { /* private: cpal::HostId */ }
impl HostId {
    /// バックエンド表示名（`Host::name` と同じ値）。
    pub fn name(&self) -> &'static str { /* ... */ }
}

pub fn default_host() -> Host;
pub fn available_host_ids() -> Vec<HostId>;
pub fn host(id: HostId) -> DeviceResult<Host>;
```

`Host` は具象型として公開する（engine が `Box<dyn DeviceEnumerator>` でも `Host` 直でも使える）。adapter クレートの公開 API は安定性契約の直接対象外（baseline feature 方針）だが、具象入口＋trait 実装は「利用側はアダプタを依存解決の選択肢として取り込み、抽象境界を通じて使用する」規則と整合する。

### マッピング設計（adapter 内部責務）

**エラー正規化**（`error.rs`、cpal 0.18 統一 `cpal::Error` の `e.kind() -> ErrorKind` → `DeviceError`。cpal 0.18.2 では旧来の `DevicesError`/`BuildStreamError`/`StreamError`/`DefaultStreamConfigError` は廃止され、列挙・構成照会・open・play/pause・err_cb の全経路が `Result<_, Error>` / `Error` に統一されている — docs.rs で確認済み）:

| `ErrorKind` | 制御経路（列挙・構成照会・open・start/stop/`now`/`buffer_size`） | err_cb 経路（`OutputErrorCallback`） |
| --- | --- | --- |
| `DeviceBusy` | `DeviceBusy` | `DeviceBusy` |
| `DeviceNotAvailable` | `DeviceNotFound`（列挙後の消失・ID 不整合） | `StreamInvalidated`（ストリーム喪失の意味論。device 追従しないストリームで消失時に発火） |
| `DeviceChanged` | —（制御経路では発火しない） | **エラー化しない**（非致命アドバイザリ。cpal 規約で default device 由来ストリームは自動再経路され継続する。`Backend("device_changed")` で通知するなら engine は「ストリーム継続・再構築不要、必要なら既定デバイス再確認」というガイダンスとして扱う設計とし、致命エラーとして潰さない） |
| `UnsupportedConfig` | `UnsupportedConfig` | `UnsupportedConfig` |
| `UnsupportedOperation` | `Unsupported` | `Unsupported` |
| `RealtimeDenied` | `RealtimeDenied` | `RealtimeDenied` |
| `StreamInvalidated` | `StreamInvalidated` | `StreamInvalidated` |
| `Xrun` | —（通常発火しない） | `Xrun` |
| `HostUnavailable` | `Backend("host_unavailable")` | `Backend("host_unavailable")` |
| `PermissionDenied` | `Backend("permission_denied")` | `Backend("permission_denied")` |
| `InvalidInput` | `Backend("invalid_input")` | `Backend("invalid_input")` |
| `ResourceExhausted` | `Backend("resource_exhausted")` | `Backend("resource_exhausted")` |
| `BackendError` | `Backend("backend")` | `Backend("backend")` |
| `Other` | `Backend("other")` | `Backend("other")` |

正規化は `fn map_error(e: &cpal::Error, ctx: ErrorCtx) -> DeviceError`（`ErrorCtx` = 制御経路/err_cb の区別）のように `e.kind()` 分岐＋呼出コンテキストで 1 関数に集約し、呼出側が個別に写像ルールを持たない。`ErrorKind` は `#[non_exhaustive]` のため将来追加 variant 用 wildcard arm を `Backend("other")` に倒す。`StreamTrait::play`/`pause` 等の制御失敗も同じ `Error`/`ErrorKind` 経路で正規化される（個別の「play 失敗」分類は持たない）。

**フォーマット・レイアウトマッピング**（`convert.rs`、純粋関数として切り離して単体テスト可能にする）:

```rust
// adapter 内部（pub でもよいが安定性契約外）
fn map_sample_format(f: cpal::SampleFormat) -> Option<SampleFormat>; // F32→F32, F64→F64, 他は None
fn layout_for_channels(channels: u32) -> Option<ChannelLayout>;      // 1→Mono,2→Stereo,4→Quad,6→5.1,8→7.1, 他は None
fn negotiate(
    desired: &OutputStreamConfig,
    supported: &[SupportedOutputFormat],
) -> Option<cpal::StreamConfig /* + sample_format */>; // 要実装時確認の cpal 型
```

- **サンプルフォーマット**: PoC では `SampleFormat::F32` のみサポート。`default_output_format()` は「既定構成 + サポート空間」を考慮して open 可能な F32 形式を優先返却する設計判断（推奨案採用 — 理由: 既定が F64/整数の実機で利用側の実証フローが即 `UnsupportedConfig` に落ちるのを防ぎ、交渉の第一責務を adapter に集約する）。F32 に写像可能な形式が皆無なら `Err(UnsupportedConfig)`。`supported_output_formats()` は `map_sample_format` が `None` を返す整数のみレンジを除外し「f32-capable 空間のみ報告」とする（空 Vec は「F32 対応レンジ無し」を意味し「照会失敗」とは `Err` で区別される — 区別不能にならないよう明記）。`open_output_stream` は `config.format.sample_format != F32` を `Err(UnsupportedConfig)` で拒否する（F64 経路・整数変換は実装しない = 非対応をエラーで表現。`SampleFormat` の `#[non_exhaustive]` が将来の整数 variant 追加余地を保持）（ALSA 系での cpal 内部変換の有無は要実機確認）
- **チャンネル数→レイアウト**: cpal はチャンネル「数」のみ報告しスピーカー位置を持たない。adapter の写像は既知カウントのみ命名レイアウトに写像し、未知カウント（3ch/12ch 等）は `None`。`ChannelLayout::Custom` は非空 `SpeakerMaskSet` を要求し位置情報がないため adapter は正しい mask を構成できない — 嘘の位置を書かない。ただし「未知カウントのデバイスが一切使えない」わけではない点を限定しておく: 真の制限は `default_output_format()`/`supported_output_formats()` が `AudioFormat` として報告できるのが既知カウントに限られることであり、engine が `supported_output_formats()` の生カウントから `ChannelLayout::Custom` を推測構成（独自 mask 仮定）すれば open の余地は残る。任意カウント対応は `ChannelLayout` への variant 追加余地（`#[non_exhaustive]` で可能）として Deferred Concern に記録
- **interleaved→planar 境界変換**: 出力経路は「engine が planar view に書く → adapter が `view.copy_to_interleaved(dst)` で cpal `Data` に packing」。`copy_to_interleaved` は RT-suitable な有界コピー（audio-core 実装済み・テスト済み）。adapter が自前の deinterleave を書かない（audio-core の変換 API を使う = spec Requirement）。端数・失敗時の規定: `data.len() % channels != 0` の残尾は完全フレームとして view に写像しない（`n = data.len() / channels` のみ写像）上、dst 側の残部を adapter がゼロフィルする。`copy_to_interleaved` がエラーを返した場合も dst 残部は adapter がゼロフィルし、「未定義データを送出しない」ことを規約とする

**コールバック配管**（`stream.rs`）:

```rust
// 概念構造（実装詳細）
// `error_callback` は data クロージャと共有せず、cpal の err_cb
// （`build_output_stream_raw` の第 2 クロージャ）に別途 `move` で渡す。
// 共有に `Arc`+`Mutex` を使うと RT スレッド上のロックで RT 契約違反
// になるため不可 — クロージャごとに所有権を分ける。
struct DataCallbackState {
    data_callback: OutputDataCallback,
    scratch: Vec<f32>,   // len = frames_capacity * channels（open 時に確保、非 RT）
    channels: usize,
    frames_capacity: usize,
}
// err_cb 側は `error_callback: OutputErrorCallback` を単独で move 所有。
```

`build_output_stream` の data クロージャは `move` で `DataCallbackState` を所有（`FnMut` + `Send`、ロック不要 — 単一コールバックスレッド）。各発火:

1. `n = data.len() / channels`（実フレーム数。残尾 `data.len() % channels` 分は dst の未書込領域としてゼロフィルする）
2. `n <= frames_capacity`: `AudioBufferViewMut::try_new(&mut scratch[..channels*n], channels, n)` を構成（呼出前に露出領域をゼロフィルするのは安全側既定）→ user cb を呼ぶ → `view.copy_to_interleaved(data)`（エラー時は dst 残部をゼロフィル）
3. `n > frames_capacity`: 容量分の chunk に分割して、スクラブの再借用でその都度 `try_new` した 2 個目以降の view を構成 → cb → copy を有界回繰り返す（非確保・有界。move 済み view を使い回すのではなく chunk ごとに構成し直す）。タイムスタンプは同一 `OutputCallbackInfo` を共用する近似 — PoC ではこの近似を明記し、厳密な per-chunk timestamp 合成は将来検討とする
4. `frames_capacity` の決定: `frames_capacity = min(effective_max, FRAMES_CAPACITY_HARD_CAP)`。`effective_max` は `BufferSize::Fixed(f)` 要求時は `f`（尊重されず実測が超えた場合は 3. の chunk 経路が受ける）、`Default` 時は `SupportedBufferSize::Range` の `max`、それも無ければフォールバック定数（例: 8192 frames。要実機確認で調整）。`FRAMES_CAPACITY_HARD_CAP` は定数（8192〜16384 frames 程度）— `Range.max` に無制限採用すると巨大報告値で数百 MB〜GB を確保し abort しうるため上限を設ける。上限超過分は 3. の既存 chunk 分割経路が受ける。スクラブ確保は open 時（非 RT）なので、`Vec::try_reserve` 等の fallible 確保で失敗を `DeviceError` に変換する選択肢も実装時に検討する

この設計により **コールバック内で Vec を伸長しない**（確保は open 時のみ）ことを構造的に保証する。`OutputTimestamp` の構成は `StreamInstant::as_nanos() -> u128` を主経路とする純粋関数とし、u128→i64 の飽和チェック付きで `MediaTime`（`DEVICE_CLOCK_TIMEBASE` = ナノ秒）へ直接変換する（`as_nanos` は docs.rs 0.18.2 で確認済み。同一ストリーム内では全 `StreamInstant` が同一クロックを共有すると cpal docs が保証するため差分・比較は意味を持つ。ストリームを跨いだ起点共有は保証されない）。飽和・写像不能時の予備経路として「起動時インスタントを記録して `duration_since` 差分から構成」する epoch 差分方式を置き、採用経路は実験ログに記録する。

## Data and Control Flow

### PoC 実証フロー（列挙 → open → start → コールバック到達）

```text
[制御スレッド（非 RT）]                              [RT スレッド（バックエンド駆動）]
  let host = audio_device::default_host();               │
  let device = host.default_output_device()?.unwrap();   │
  // フォーマット交渉: adapter は F32 open 可能形式を      │
  // 優先返却する。Err 時は supported 空間から F32 レンジ  │
  // を選択し AudioFormat を合成（利用側フォールバック）。 │
  let fmt = match device.default_output_format() {       │
      Ok(f) if f.sample_format == SampleFormat::F32 => f,│
      _ => select_f32_format(                            │
              &device.supported_output_formats()?)?,     │
  };                                                     │
  let cfg = OutputStreamConfig { format: fmt,            │
            buffer_size: BufferSize::Fixed(480) };       │
  let stream = device.open_output_stream(&cfg, cb, ecb)?;│
  stream.start()?; ──────────────────────────────► コールバック発火
                                                     AudioBufferViewMut::try_new(scratch, ch, n)
                                                     data_cb(view, OutputCallbackInfo)
                                                         │  engine が planar f32 を書く
                                                     view.copy_to_interleaved(cpal Data)
                                                     err_cb(DeviceError) ← xrun 等
  stream.stop()?; / drop(stream) ◄─────────────────── 送出停止
```

- フォーマット交渉: `default_output_format()` は F32 open 可能形式を優先返却する adapter 契約のため、既定が非 F32 のデバイスでも多くはそのまま open できる。`Err(UnsupportedConfig)` または非 F32 報告時は `supported_output_formats()` の F32 レンジから利用側が `AudioFormat` を合成して open するフォールバック経路を持つ（図中 `select_f32_format` は PoC example 側の補助関数）
- RT 境界の所在: `open_output_stream` に渡した `data_callback` の呼び出し以降が RT 契約区間。列挙・構成・open/start/stop/`now()`/`buffer_size()` は非 RT（制御スレッド）
- データ変換点: planar f32（canonical）は `AudioBufferViewMut` 経由で engine が書き、境界変換（planar→interleaved packing）は adapter が `copy_to_interleaved` で行う。interleaved は cpal `Data` との接点のみに存在する
- エラー経路: err_cb は RT-adjacent。`DeviceError` は Copy で確保なし。err_cb 実装（利用側）も確保・ブロッキング禁止を doc で契約化する

## PoC 検証設計

### CI に置けるデバイス非依存検証（`cargo test`/`check`/`lint` で機械検証）

| 検証 | 場所 | 内容 |
| --- | --- | --- |
| object safety・`Send` | audio-core `#[cfg(test)]` | `fn _assert(_: &dyn DeviceEnumerator, _: &dyn Device, _: &dyn OutputStream)` 等の `Box<dyn …>` 構成可能性に加え、`Box<dyn Device>`/`Box<dyn OutputStream>` が trait object 経由で `Send` を満たす assert、および `FnMut(AudioBufferViewMut<'_, f32>, OutputCallbackInfo) + Send + 'static` なクロージャが `OutputDataCallback` へ強制変換（coerce）可能であること（HRTB `for<'a>` 推論の成立）をコンパイルで確認 |
| 型契約 | audio-core テスト | `DeviceError` が `Copy`、`OutputCallbackInfo`/`BufferSize`/`SupportedBufferSize` が意図した型レベル制約（`#[non_exhaustive]` 全公開 enum）を満たすこと。clippy `exhaustive_enums` ゲートでも強制済み |
| `AudioBufferViewMut::try_new` | audio-core テスト | 正常構成・`channels==0`・長さ不一致のエラー、`copy_to_interleaved` との結合 |
| マッピング単体テスト | adapter `convert.rs` テスト | `map_sample_format`・`layout_for_channels`・`negotiate` の純粋関数を合成入力で検証（デバイス不要） |
| エラー正規化 | adapter `error.rs` テスト | `ErrorKind` が値として構成可能なら分類関数を網羅テスト、不可能なら `DeviceError` 側の等価性・Display 等を検証（要実装時確認に応じてフォールバック） |
| **モックバックエンド結合** | adapter `tests/mock_backend.rs`（または audio-core 側テスト） | ポート trait を実装するフェイク `Device`/`OutputStream` を書き、`open_output_stream` → 登録コールバックをスクリプト通り発火 → `start`/`stop`/`now` を踏む流れを `Box<dyn …>` 経由で検証。trait 適合性（第 2 backend が実装可能な形状か）をハードウェアなしで証明する主要テスト |
| interleave 書込経路 | adapter テスト | コールバック本体の「view 構成→cb→`copy_to_interleaved`」を合成 `&mut [f32]` に対して実行し、チャンク分割経路（n>capacity）も含めて検証 |

- doctest 方針: `device` モジュール公開 API の例はハードウェア非依存とし、モック実装で組み立てるか `no_run`/`ignore` を指定する（CI で実デバイスに触れる例を doctest に置かない）

### コールバック内非 alloc の検証方針

- trait 契約として `OutputDataCallback`/`OutputErrorCallback` の doc に RT 契約を明記（alloc/dealloc 禁止・ブロッキング/I/O/無制限待機/非有界処理の制限）
- 機械検証の試行: `assert_no_alloc` を **adapter の dev-dependency** として追加し、コールバック本体経路（view 構成＋`copy_to_interleaved`＋テスト用 user cb）を `#[test]` で包んで alloc ゼロを検証する。テスト名は `rt_` プレフィックスとし `mise run test-rt` の対象に接続する。追加理由: PoC の受け入れ基準「コールバック内非 alloc の検証」をコードレビュー証跡だけでなく機械検証で補強するため。なお alloc フック系の CI 標準装備自体は RT 検証 Issue の所管であり、本 PoC では「使えるかの実証＋限界の記録」に留める
- 測定範囲の注記: `assert_no_alloc` が計測できるのは adapter 自身のコールバック経路（テストから直接駆動する自前コード）のみで、cpal が駆動する RT スレッド内部（cpal/OS 側の確保）は包めない。cpal 内部を含めた実測の直近手段は、example バイナリに計数型 `#[global_allocator]` を仕込んで `play_sine` 実行中の alloc を計数する方式 — 試行結果と限界を実験ログの方針として記録する
- フォールバック: `assert_no_alloc` の統合が環境都合で困難な場合（グローバルアロケータ差替・feature 要件等）、コードレビュー証跡＋コールバック経路に確保 API が存在しないことの静的確認（`grep` で Vec/String/Box 到達点の確認）を実験ログに記録する

### 実機手動検証手順（CI 非依存・実機のみ）

- `cargo run -p ruviform-audio-device --example list_devices [--features pipewire-backend]`: ホスト（`available_host_ids`）・デバイス列挙・既定デバイス・`default_output_format`・`supported_output_formats` のダンプ。実行 OS・ホスト API・結果を実験ログに記録。既定構成（default host のみ。Linux=ALSA）では PipeWire host id は列挙されず、ALSA vs PipeWire の比較は `pipewire-backend` 有効化（要 `libpipewire-0.3-dev`）での任意検証とする
- `cargo run -p ruviform-audio-device --example play_sine [--features realtime]`: 既定出力デバイスに sine を数秒送出。コールバック内で I/O・確保を行わないため観測値は `AtomicUsize`/`AtomicU64` にのみ記録（コールバック回数・最小/最大実フレーム数・xrun 回数・最終 `OutputTimestamp`）し、`stop()` 後に非 RT 側で要約出力する — これにより「コールバック到達」「可変フレーム実測」「xrun 頻度」「timestamp の整合」を 1 手順で観測する。期待値の差異として、WASAPI では出力ストリームの xrun が報告されない（capture のみ報告）ことが分かっている — OS ごとの xrun 観測値 0 は「xrun 未発生」と「未報告」を区別しない旨を実験ログに記録する。`realtime` feature は Windows/Android にのみ作用し、Linux の RT 昇格（`realtime-dbus`＝rtkit・`libdbus-1-dev` 必要）は本 PoC では未検証である点にも注意
- 確認すべき未確定点（discovery の Open Questions との対応）: コールバック内 alloc/ブロッキング挙動、`BufferSize::Fixed` 尊重度、`Send`/`Sync` の backend 間ばらつき（`pipewire-backend` 有効時の `!Send` リスクを含む）、WASAPI exclusive/ASIO 必要性、ALSA vs PipeWire 挙動差（`pipewire-backend` 有効時に `available_host_ids` で比較）、ライフタイム/所有権制約と `RtContext` 契約の適合
- 記録先: `docs/impl/exp/` に `new_experiment_log.js` で実験ログを作り、環境（OS/ホスト/cpal バージョン）・手順・観測値・判定を記録する

### ADR 確定計画

PoC 完了後、[select-audio-device-backend.md](../adr/select-audio-device-backend.md) の `Confirmation`/`Verification` 項目を実験ログの証跡と対応付けて確認し、`status: proposed → accepted` へ昇格する。確認項目と証跡の対応:

| ADR Verification 項目 | PoC 証跡 |
| --- | --- |
| audio-core 公開 API に cpal 由来の型・エラー・feature 名が無い | `cargo doc -p ruviform-audio-core` 出力レビュー＋`#[non_exhaustive]` 適合（lint ゲート） |
| audio-device 無効構成でコアがビルド可・cpal 依存が adapter のみ | `cargo metadata` で依存木確認＋`cargo check -p ruviform-audio-core` |
| 列挙→既定構成→出力開始→コールバック到達の実証 | `play_sine` 手動実行結果（実験ログ） |
| コールバック内非 alloc の検証 | assert_no_alloc 試行結果 or コードレビュー証跡（実験ログ） |
| duplex 要件化時に第 2 backend を追加できる trait 形状 | モックバックエンド結合テスト＋本設計の trait 分解・非露出確認 |
| `mise run fmt-check`/`check`/`lint`/`test` パス・unsafe 局所許可のみ | CI 同等検証 |

再評価トリガ（duplex 要件化・cpal 再停滞・致命的挙動不良）が観測された場合は `accepted` 昇格を保留し、条件見直し版を別途起票する。

## Non-Functional Constraints

- **依存方向**: `ruviform-audio-device` → `ruviform-audio-core`（→`ruviform-media-core`）のみ。adapter→adapter 禁止、audio-core は adapter に依存しない（依存解決は利用側の責務）
- **外部型の非露出**: cpal の型・エラー・feature 名を audio-core 公開 API 面に出さない。adapter 公開面にも cpal 型を出さない（`HostId` newtype 等の opaque 化）
- **公開 enum**: 全て enum レベル `#[non_exhaustive]`（`BufferSize`/`SupportedBufferSize`/`DeviceError`。`SampleFormat`/`ChannelLayout` は既適合）
- **`unsafe` 不使用**: cpal 内部の unsafe は依存先の実装詳細。adapter 自身は原則禁止（例外は SAFETY コメント付き局所許可のみ）
- **RT 契約**: コールバック内は alloc/dealloc 禁止・ブロッキング/I/O/無制限待機/非有界処理を制限。列挙・構成・open/start/stop は非 RT API。保証範囲は API 単位の `**RT contract:**` ラベルで明示し、CI 成功を RT 安全性の完全証明とみなさない（realtime ADR）
- **planar f32 canonical**: interleaved は cpal `Data` との接点のみ。変換は audio-core の `copy_to_interleaved`/`copy_from_interleaved`/`from_interleaved`/`AudioBufferViewMut` を利用し、adapter で独自 deinterleave を実装しない
- **時間表現**: デバイス時計は `MediaTime`（ナノ秒 timebase の規約）で表現し、f64 秒を中核にしない。フレーム位置の中核は `SampleIndex`+`SampleRate`（本ポートではコールバック時刻のみ扱う）
- **MSRV/edition**: edition 2024・MSRV 1.85・stable のみ。cpal 0.18.x の MSRV 適合（実装時に `cargo` で確証。cpal 0.18 は MSRV 1.85 と公開情報）
- **publish**: `publish = false` 継承（workspace.package）。クレート内 API の変更余地は内部のため許容
- **依存追加記録**: cpal・`assert_no_alloc`（dev-dep・試行）の追加理由を PR/実験ログに記録し、`Cargo.lock` を追跡

## Risks and Trade-offs

| リスク/トレードオフ | 内容と緩和 |
| --- | --- |
| **interleaved→planar 変換コスト** | コールバック毎に 1 回の有界コピーが必須（cpal `Data` が interleaved）。`copy_to_interleaved` は RT-suitable で確保なしのため契約上問題なし。実コストは実験ログで計測観察する |
| **可変フレームと事前確保の両立** | `BufferSize::Default` では実フレーム数が変動しうる → `min(effective_max, FRAMES_CAPACITY_HARD_CAP)` の上限付きスクラブ＋`AudioBufferViewMut::try_new` での per-callback view 構成で対応。容量超過は chunk 分割（有界・非確保）。`BufferSize::Fixed` 尊重度が低い backend では chunk 経路が常時動く — 実機で確認 |
| **チャンネル数→レイアウトの情報欠落** | cpal はスピーカー位置を報告しない。adapter が `AudioFormat` として報告できるのは既知カウントに限られ、未知カウント（3ch 等）は adapter 報告経路では `UnsupportedConfig` — ただし engine が `supported_output_formats()` の生カウントから `ChannelLayout::Custom` を推測構成すれば open の余地は残る（真の制限は報告側）。将来 `ChannelLayout` variant 追加余地あり（Deferred） |
| **`Send`/`Sync` の backend ばらつき** | trait は `Send` のみ要求。backend 実装上 `Stream` が `!Send` なら adapter 実装が成立しない → 実機/PoC で各 OS 確認（discovery の未確定点） |
| **err_cb のスレッド安全性** | バックエンドスレッドから呼ばれるため利用側実装の非確保・非ブロッキングを doc 契約に明記。`DeviceError` Copy でポート側は確保しない |
| **cpal 依存のメンテリスク** | 実質単独メンテナ依存の残存（ADR 記録済み）。ポート非露出により交換可能 — 第 2 backend 追加時に trait が通用するかはモックテストで先に検証 |
| **`now()` の RT 適合性不明** | バックエンド依存・要実機確認のため、PoC では制御経路 API として扱い RT 契約を主張しない。`MasterClock` 配線はコールバック内 `OutputTimestamp` を優先経路とする |
| **f32 以外のサンプルフォーマット非対応** | PoC では F32 のみ。デバイスが整数のみなら `UnsupportedConfig` — cpal の raw builder による内部変換等の代替は実装時確認・将来検討 |

## Deferred Design Concerns

| 据え置く項目 | 理由 | 再着手トリガ | 無視した場合のリスク |
| --- | --- | --- | --- |
| 入力ストリーム・duplex API | spec Out of Scope。`output_*` 命名で追加余地を確保済み | duplex/入力要件化（採否 ADR 再評価。cpal PR #1229 追跡。cubeb/rtaudio 代替候補） | duplex 前提の trait 形状欠落で第 2 backend が破壊的変更になる |
| `RtContext` のコールバック配線 | mint API 未実装（RT 基盤 Issue）。本設計は `OutputCallbackInfo` の情報コンテナで配線を阻害しない | `ruviform-rt` mint 実装＋`ruviform-audio` engine 設計 | capability モデルとコールバック契約の不整合 |
| 任意チャンネル数のレイアウト表現 | cpal が位置情報を報告せず `SpeakerMaskSet` を正しく構成不能 | 3ch/12ch 等を含むデバイス要件・multichannel 設計 | 未知カウントデバイスが `UnsupportedConfig` で使えない |
| F64・整数サンプルフォーマット経路 | PoC は F32 のみ。`SampleFormat` non_exhaustive で拡張余地あり | f64 本格対応・整数直接出力の要件化 | 非 f32 デバイス/フォーマットでの利用制限 |
| WASAPI exclusive / ASIO / プラットフォーム個別設定 | spec Out of Scope。ASIO は opt-in 隔離 | プラットフォーム要件評価 Issue | 要件化時の API 穴 |
| `open_timeout` 等の open パラメータ | v1 では backend 既定に委ねる最小設計 | 実機検証で必要性確認 | フレーク時の制御不能（`OutputStreamConfig` フィールド追加で対応可） |
| `DeviceEnumerator` のスレッド移動・再列挙ポリシー | 制御スレッドオブジェクトとして `Send` 非要求 | engine のセッション調停設計 | engine 側での取り回し制約（必要なら境界追加の内部変更） |
| `assert_no_alloc` の CI 標準装備 | 本 PoC は試行と証跡記録のみ。恒常化は RT 検証 Issue | RT 検証 Issue | alloc 違反の検出が属人レビュー依存 |
| `OutputStream::now()` の RT 適合保証 | backend 依存・実機確認待ち | `MasterClock` 配線の実装時 | RT 経路からの誤用（doc で現状制御経路と明示） |

## Verification Notes

- [ ] `cargo doc -p ruviform-audio-core` で公開面に cpal 由来の型・エラー・feature 名が存在しないこと、`device` モジュールの全公開 enum に `#[non_exhaustive]` があること
- [ ] `mise run fmt-check` / `check` / `lint` / `test` および `mise exec -- prek run --all-files` がパスする（`unsafe` 局所許可のみ）
- [ ] audio-core テストで `Box<dyn Device>`/`Box<dyn OutputStream>`/`Box<dyn DeviceEnumerator>` の object safety、trait object 経由の `Send` 適合、`OutputDataCallback` へのクロージャ強制変換（HRTB）がコンパイル確認される
- [ ] adapter のモックバックエンド結合テストが、trait を実装する非 cpal 実装で open→コールバック→start/stop の流れを検証する
- [ ] コールバック経路の非 alloc が assert_no_alloc で検証されるか、失敗時はコードレビュー証跡が実験ログに残る
- [ ] 実機で `list_devices`/`play_sine` が「列挙→既定構成→ストリーム開始→コールバック到達（planar f32 境界変換）」を観測し、結果が `docs/impl/exp/` の実験ログに記録される
- [ ] `cargo metadata` で cpal が `ruviform-audio-device` の `[dependencies]` にのみ存在し、コアクレートがそれなしで `cargo check` できる
- [ ] ワークスペース帳簿が同期される（ルート `Cargo.toml` の `members`・`AGENTS.md` のクレート構成記述・必要に応じ `docs/designs/overview.md` 索引）
- [ ] PoC 結果に基づき採否 ADR が `accepted` 昇格または条件見直しされる（Confirmation 項目との対応表どおり）
