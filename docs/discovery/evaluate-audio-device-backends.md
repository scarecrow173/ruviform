---
id: "DISC-034Jc8wchRH37hqfhNLRPJ"
type: "discovery"
status: "resolved"
title: "オーディオデバイスバックエンド評価（cpal 等）の探索"
created: "2026-10-20"
updated: "2026-10-20"
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
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../ideas/audio-device-backend-evaluation.md"
    - "https://github.com/scarecrow173/ruviform/issues/23"
    - "https://github.com/scarecrow173/ruviform/issues/1"
  derived-by:
    - "../specs/audio-device-backend-evaluation-poc.md"
    - "../adr/select-audio-device-backend.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/RustAudio/cpal"
    - "https://github.com/RustAudio/cpal/issues/981"
    - "https://github.com/RustAudio/cpal/releases"
    - "https://github.com/mozilla/cubeb-rs"
    - "https://github.com/MeadowlarkDAW/rtaudio-rs"
    - "https://github.com/BillyDM/Firewheel"
    - "https://github.com/HEnquist/audioadapter-rs"
    - "https://crates.io/crates/audio_thread_priority"
  defers: []
  deferred-by: []
---

# オーディオデバイスバックエンド評価（cpal 等）の探索

## Exploration Goal

本書は GitHub Issue [#23 [poc] オーディオデバイスバックエンド評価（cpal 等）と採否](https://github.com/scarecrow173/ruviform/issues/23) の briefing／探索フェーズの成果を記録する canonical ドキュメントである。Issue #1 設計ベースライン「後続実装 Issue の提案一覧」から起票された Issue であり、上流の着想は idea 文書 [audio-device-backend-evaluation.md](../ideas/audio-device-backend-evaluation.md) に記録済み。

本書が回答する問い:

- `ruviform-audio-core` が定義するデバイス境界 trait（ポート）の契約に照らして、評価対象候補と評価軸をどう確定するか
- cpal と代替候補（cubeb-rs / rtaudio-rs / プラットフォーム直結等）の比較から、第一候補と採用条件は何か（仮説として）
- PoC／実機検証で確認すべき未確定点は何か

> **位置づけ**: 本書は探索の推論経路と一次情報を記録するものであり、最終決定を含めない。採否の決定記録は ADR（[select-audio-device-backend.md](../adr/select-audio-device-backend.md)）、作業範囲と受け入れ基準は spec（[audio-device-backend-evaluation-poc.md](../specs/audio-device-backend-evaluation-poc.md)）に委ねる。外部調査結果は 2026-10 時点の公開情報に基づく。

## Key Issues and Assumptions

### 内部契約要求（設計ベースラインより。L 番号は ruviform-architecture-design-baseline.md）

方向性を決める前に前提となる内部契約:

- **責務配分**: デバイス境界 trait（ポート）の定義所有は `ruviform-audio-core`（L193）。`ruviform-audio-device` はそれを実装する adapter で、cpal 依存は同クレート内に閉じる（L201, L213–214）。`ruviform-audio`（engine・未作成）が RT コールバック配線とセッション調停を担う（L199）。adapter → foundations/Primitives + ポート定義元のみ依存可、adapter→adapter 禁止（L163, L177）
- **RT 契約**: デバイスコールバックが RT パスの起点であり、コールバック内は alloc/dealloc 禁止・ブロッキング/I/O/無制限待機/非有界処理を制限（L201, L299, L430）。列挙・構成・開始・停止は非 RT 区間（制御スレッド）の責務（L285, L300）。バッファ受け渡しは事前確保済みへの `&mut` 借用（L335–336）。デバイス入力は有界キュー（L293）
- **`RtContext` 前提**: RT 契約区間を開始した駆動者のみが発行可能で、発行主体は `ruviform-audio`（L284）。現時点で `ruviform-rt` の mint API は未実装のため、trait 設計は将来の `RtContext` 配線を阻害しない形にする
- **データモデル**: planar f32 が正準、interleaved は境界のみ（L246）。`AudioBuffer::from_interleaved` / `copy_to_interleaved` / `copy_from_interleaved` / `AudioBufferViewMut` が audio-core に実装済み。`MasterClock`（ruviform-rt 所有）がデバイス時刻を保持する A/V 同期基準であるため、**バックエンドのストリーム時刻・タイムスタンプ取得 API が trait 要件**（L294, L240, L175）
- **backend 抽象規則**: 外部型・エラー・feature 名をポート定義元（audio-core）の API 面に露出しない。feature 名は「バックエンド有効化の意図」命名で固有名詞を出さない（L215）。バックエンド選択はクレート/feature 依存で表現し実行時文字列指定しない（media-io ADR）。バックエンドエラーは抽象エラー型に正規化（L270）。非対応機能は「非対応」を型/エラーで表現し全機能一律汎用化しない（L269, L448）。全公開 enum に `#[non_exhaustive]`（public-enum ADR）。`publish = false`、`unsafe` 原則禁止、stable-only・MSRV 1.85
- **分類表の位置づけ**: baseline L475「音声デバイス I/O バックエンド（cpal 等）｜要PoC｜抽象境界必須・メンテリスク」に本 Issue が該当。Phase 2 完了目安（L518）「デバイス経由の最小 RT パスが成立」に向け、PoC は「列挙＋ストリーム開始＋コールバック到達」までの実証が望ましい。CI はデバイス非依存（実機ループバックは手動検証、CI 代替はオフラインアンダーラン＋alloc フック、L436）

### 評価軸（baseline から導出）

コールバック駆動モデル／コールバック内 RT 適合性／サンプル形式（f32 直扱い・整数の変換点）／バッファ形状（interleaved vs planar）／デバイス列挙・既定デバイス・構成交渉／デバイス時刻・タイムスタンプ API／スレッド優先度（MMCSS・time-constraint・SCHED_FIFO）／対象プラットフォーム（Win/macOS/Linux、x86_64+aarch64）／メンテ状況・ライセンス・依存木・MSRV 適合／**trait 適合性（差替可能性の検証）**／エラー正規化のしやすさ／検証可能性（CI はデバイス非依存、実機は手動）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **cpal**（RustAudio/cpal。0.18.2・2026-08） | Apache-2.0・**MSRV 1.85 で ruviform と一致**。dependents 650+・累計 DL 2,500 万の事実上の標準。pure Rust＋薄い OS バインディング層で外部 C ライブラリ同梱なし。Windows=WASAPI、macOS/iOS=CoreAudio、Linux=ALSA（0.18 で PipeWire/PulseAudio ネイティブ追加）、Android=AAudio 直結、WASM=WebAudio。0.18 で `StreamTrait::now()`/`buffer_size()`・ハードウェアレイテンシ込み timestamp・`ErrorKind` 統一（Xrun/DeviceBusy/RealtimeDenied/StreamInvalidated）。`realtime` feature（=`audio_thread_priority`）でコールバックスレッド RT 昇格 | コールバックの `Data` は **interleaved `&mut [T]`**（planar 変換は境界で必要。audio-core の `copy_*_interleaved` と整合）。**duplex（同期入出力）API 未実装**（issue #349 closed・PR #1229 open）。WASAPI 排他モード未サポート。停滞実績あり（0.15.3 以降 1 年超リリースなし・issue #981 で 2025-06 に公式認め）— その後 roderickvd により 0.16→0.17→0.18.2 と回復済みだが、実質単独メンテナ依存のリスクは残存。ASIO は build.rs が SDK 自動ダウンロード（再現性注意） | **yes（条件付き）** |
| **cubeb-rs**（mozilla/cubeb-rs。cubeb 0.36 / cubeb-core 0.38） | Firefox 現役本番コードで Mozilla 公式メンテ。ISC。**1 コールバックで入出力同時の真の full-duplex（リクロック済み）**。レイテンシ補償済みオーディオクロック。Tier-1: PulseAudio（pure Rust）/CoreAudio/WASAPI/AAudio | `cubeb-sys` が cmake/C++ ビルド依存で依存木が重い・FFI=unsafe。PipeWire ネイティブ・WASM なし。コールバックに `Send+Sync` 要求（やや厳しい） | maybe（duplex 要件化時の代替） |
| **rtaudio-rs**（Meadowlark/rtaudio-rs。0.8.1） | MIT。RtAudio 6 C API の安全ラップ。**`Buffers` enum で真 duplex**。ALSA/Pulse/JACK/OSS/CoreAudio/WASAPI/DS/ASIO と広い。firewheel-rtaudio が採用 | C++ をソースから静的ビルド（ビルドツールチェイン依存）。Android 非対応。GitHub リポジトリは Codeberg へ移行済み | maybe（duplex 要件化時の代替） |
| **プラットフォーム直結自前 adapter**（wasapi / coreaudio-rs / alsa / pipewire / jack） | RT 制御自由度は最大。CamillaDSP が同構成の先例。coreaudio-rs の AudioUnit render コールバックは真のデバイスコールバック | 3 OS 分の保守コストが大きい。wasapi/alsa はコールバックではなく自前スレッド＋event/poll。pipewire は `!Send`/`!Sync` 注意。`audio_thread_priority` 併用が必須級 | maybe（中長期オプション） |
| **不採用群**（oboe-rs / libsoundio 系 / rust-portaudio / miniaudio ラッパ / sdl3 / wavy / nannou_audio） | — | oboe-rs は停滞かつ Android 限定（cpal 自身が脱却済み）。libsoundio 系は上流低活動・rsoundio は作者が abandonware 明記・soundio-rs は archived（cpal 推奨）。rust-portaudio は maintenance mode（cpal 推奨）。miniaudio は Rust ラッパ停滞・unsafe 境界が広い。sdl3 はミキシング層経由で粒度粗い。wavy は未完成停滞。nannou_audio は cpal ラッパでしかない | no |

補助クレート（バックエンドではないが評価上の関連資産）: `audio_thread_priority`（Mozilla。cpal `realtime` feature の実体。直結案では必須級）、`audioadapter-*`（HEnquist。interleaved↔planar 変換の実績あるユーティリティ。firewheel-cpal・CamillaDSP 採用）。参考先例: **firewheel**（Bevy 公式採用予定のオーディオエンジン）が engine↔デバイスを trait 分離し firewheel-cpal / firewheel-rtaudio を交換可能にしている — ruviform の port/adapter 構造と同型。

## Tentative Conclusions and Hypotheses

- **仮説（推奨）**: **cpal = 条件付き採用が第一候補**。条件: (a) 全型を `ruviform-audio-device` 内部に閉じ込め audio-core の API 面に漏らさない、(b) interleaved→planar f32 変換とコールバック内非 alloc を adapter 責務として明示、(c) duplex が要件化した時点で第 2 backend を追加可能な trait 設計、(d) ASIO は opt-in のまま隔離。根拠: MSRV 一致・採用実績・pure Rust 依存木・0.18 での timestamp/ErrorKind 改善が MasterClock 要件とエラー正規化要件に整合
- **仮説**: cubeb-rs / rtaudio-rs は duplex が要件化した場合の代替候補として保持する。両者とも真 duplex を持つが C++ ビルド依存が重く、現時点で第一候補とする理由はない（cpal の duplex 欠落が要件に衝突しない限り）
- **仮説**: プラットフォーム直結自前 adapter は中長期オプション。RT 制御自由度は最大だが 3 OS 分の保守コストが現フェーズの費用対効果に見合わない。trait 設計が差替可能性を保てば将来の移行経路は残る
- **確認済み（内部契約側）**: trait 定義所有は `ruviform-audio-core`、adapter は `ruviform-audio-device`、コールバック内は RT 契約適用、planar f32 正準・interleaved は境界のみ、外部型・feature 名の非露出、`#[non_exhaustive]` 公開 enum、MSRV 1.85 — これらは baseline・各 ADR で確定済みであり本探索の入力制約
- **確認済み（調査側）**: cpal のコールバック `Data` が interleaved `&mut [T]` であること、duplex API 未実装であること、0.18 系のリリース・機能改善は公開情報で確認済み。ただしコールバック内 alloc/ブロッキング挙動など実機依存の性質は未検証

## Open Questions

PoC／実機検証で確認すべき未確定点（本探索では未解決。回答者: PoC 実施者、期限: 採否 ADR の `accepted` 昇格までに要確認）:

- cpal コールバック内の alloc/ブロッキング挙動の実測（assert_no_alloc 手法の転用が候補。fundsp に先例）
- duplex（同期入出力）が ruviform の要件に入るか。入るなら cpal 単独では不十分 → trait の差替可能性設計が鍵となる（cpal PR #1229 を追跡）
- `BufferSize::Fixed` の尊重度・可変バッファ周期の実測
- `Send`/`Sync` の backend 間ばらつき（0.17〜 主要 backend で `Stream: Send` と改善済みとされるが要実測）
- Windows の WASAPI exclusive/ASIO 必要性、Linux の ALSA vs PipeWire 挙動差
- 各 API のライフタイム/所有権制約と `Processor`/`RtContext` 契約との適合（`RtContext` mint API 未実装のため阻害しない設計か）

## Promotion Candidates

- [x] spec-doc needed: Issue #23 の作業範囲（trait 設計・`ruviform-audio-device` 新設・cpal adapter PoC・評価レポート・採否 ADR 確定）と受け入れ基準を正式化 → [audio-device-backend-evaluation-poc.md](../specs/audio-device-backend-evaluation-poc.md)
- [x] adr-doc needed: バックエンド採否（cpal 条件付き採用・代替候補の棄却/保留理由）を決定記録として残す → [select-audio-device-backend.md](../adr/select-audio-device-backend.md)

## Sources and Relations

- 一次ソース（`relations.source`）: GitHub Issue [#23 [poc] オーディオデバイスバックエンド評価（cpal 等）と採否](https://github.com/scarecrow173/ruviform/issues/23)
- 上流（`relations.derives-from`）: idea [audio-device-backend-evaluation.md](../ideas/audio-device-backend-evaluation.md)、Issue [#23](https://github.com/scarecrow173/ruviform/issues/23)、親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)
- 関連（`relations.related`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（責務表・RT 契約・分類表）、[media-io-backend-abstraction.md](../adr/media-io-backend-abstraction.md)（backend 抽象規則の先行決定）、[realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md)（RT 契約・検証方針）、[platform-and-toolchain-targets.md](../adr/platform-and-toolchain-targets.md)（対象プラットフォーム・MSRV）、[AGENTS.md](../../AGENTS.md)（暫定規約）
- 外部出典（`relations.references`）: [cpal](https://github.com/RustAudio/cpal) ・ [releases](https://github.com/RustAudio/cpal/releases) ・ [issue #981（メンテナンス停滞の公式認め）](https://github.com/RustAudio/cpal/issues/981)、[cubeb-rs](https://github.com/mozilla/cubeb-rs)、[rtaudio-rs](https://github.com/MeadowlarkDAW/rtaudio-rs)、[Firewheel](https://github.com/BillyDM/Firewheel)（trait 分離によるバックエンド交換の先例）、[audioadapter-rs](https://github.com/HEnquist/audioadapter-rs)、[audio_thread_priority](https://crates.io/crates/audio_thread_priority)
