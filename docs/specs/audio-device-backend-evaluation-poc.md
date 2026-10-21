---
id: "SPEC-034Jc9QEqYLVfg1CXXTiPV"
type: "spec"
status: "implemented"
title: "オーディオデバイス境界 trait と ruviform-audio-device PoC（バックエンド評価・採否）"
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
  depends-on:
    - "establish-rust-dev-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/select-audio-device-backend.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/evaluate-audio-device-backends.md"
    - "https://github.com/scarecrow173/ruviform/issues/23"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# オーディオデバイス境界 trait と ruviform-audio-device PoC（バックエンド評価・採否）

## What Should Be Built

GitHub Issue #23 の成果物として、以下を確定・実装する。

- **デバイス境界 trait（ポート）の設計・定義**: `ruviform-audio-core` に、オーディオデバイス境界を表す trait 群を定義する。責務表どおりポート定義の所有は audio-core であり、`ruviform-audio-device`（adapter）がこれを実装する。少なくともデバイス列挙・既定デバイス/既定構成の取得・構成の記述（`AudioFormat` / `BufferSpec` / `ChannelLayout` で表現）・出力ストリームの開始/停止・ストリーム時刻/タイムスタンプの取得（`MasterClock` 要件対応）をカバーする。コールバックは RT 契約区間の起点であり、事前確保済みバッファへの `&mut` 借用でデータを受け渡す形にする。`RtContext` の mint API は `ruviform-rt` 未実装のため、trait は将来の `RtContext` 配線（発行主体は `ruviform-audio`）を阻害しない設計とする
- **`ruviform-audio-device` クレート新設 + cpal adapter PoC**: adapter ルール（foundations/Primitives + ポート定義元へのみ依存）に従う分離クレートとして `ruviform-audio-device` を新設し、cpal を内部依存として閉じ込めた adapter を実装する。PoC の実証範囲は「デバイス列挙 → 既定出力構成の取得 → 出力ストリーム開始 → コールバック到達（interleaved → planar f32 への境界変換を含む）」まで
- **評価レポート**: discovery の評価軸に沿った候補比較と PoC 結果を `docs/impl/exp/` の実験ログ（または discovery 文書への追補）として記録する
- **採否 ADR の確定**: [select-audio-device-backend.md](../adr/select-audio-device-backend.md)（cpal 条件付き採用・`accepted`）を PoC 結果で確認し、`accepted` へ昇格または差し戻す

## Why Now

- 設計ベースライン分類表（L475）で「音声デバイス I/O バックエンド（cpal 等）」は「要PoC・抽象境界必須・メンテリスク」と位置づけられており、本 Issue が該当する。Phase 2 完了目安（L518）「デバイス経由の最小 RT パスが成立」の前提となる
- `ruviform-audio-device` が未作成・audio-core にデバイス境界 trait が未定義（「defined in a later issue」）の現状では、trait 契約が現実のバックエンド制約（コールバック駆動・interleaved バッファ・レイテンシ・タイムスタンプ）に適合するか検証できない
- cpal は事実上の標準だが停滞実績（issue #981）を含むメンテナンスリスクが指摘されており、回復後も実質単独メンテナ依存が残る。採否には比較と条件の根拠が必要であり、backend 抽象の維持方針を先に確定する必要がある

## Users and Value

- **engine 層（`ruviform-audio`）の実装者**: RT コールバック配線・セッション調停の接続先となるポート契約が確定し、バックエンド種別を意識せず設計できる
- **`ruviform-audio-device` 実装者**: adapter の責務範囲（interleaved→planar 境界変換・コールバック内非 alloc・エラー正規化）と、cpal 依存の閉じ込め先が明示される
- **レビュアー・後続 Issue の作業者**: 採否の根拠（評価軸・比較・条件）と PoC の実証範囲が文書化され、duplex 要件化時の再評価トリガが明確になる

## Scope

### In Scope

- `ruviform-audio-core` へのデバイス境界 trait（ポート）の設計・定義（列挙・既定構成・構成記述・出力ストリーム開始/停止・ストリーム時刻/タイムスタンプ・エラー正規化・コールバック RT 契約の明示）
- `ruviform-audio-device` クレートの新設と cpal adapter の PoC 実装（列挙 → 既定出力構成 → 出力ストリーム開始 → コールバック到達＋interleaved→planar f32 境界変換まで）
- 評価軸に沿った候補比較と PoC 結果の記録（`docs/impl/exp/` 実験ログまたは discovery 追補）
- 採否 ADR（`select-audio-device-backend.md`）の確定（`proposed` → `accepted` 昇格または条件見直し）
- コールバック内非 alloc 規律の契約明示と、可能な範囲での検証（assert_no_alloc 相当またはコードレビュー証跡）

### Out of Scope

- duplex（同期入出力）の本格対応（要件化時は第 2 backend 追加として再評価。cpal PR #1229 を追跡）
- エンジン層 `ruviform-audio` のセッション調停・RT コールバック配線本体
- `RtContext` mint API（RT 基盤 Issue の範囲。本 spec は配線を阻害しない設計のみ要求）
- 実機ループバック試験の CI 化（実機依存部分は手動検証手順の文書化に留め、CI にはデバイス非依存テストのみ置く）
- ASIO 対応（opt-in 隔離のまま。build.rs の SDK 自動ダウンロードは再現性の注意点として記録のみ）
- `audio_thread_priority` の単独採用可否・cubeb/rtaudio/直結 adapter の実装（採否 ADR で保留・棄却理由を記録）

## Requirements

- Rust edition 2024・MSRV 1.85・stable のみ。`publish = false`（workspace 継承）、`[lints] workspace = true`。`unsafe` は使用しない（必要な場合のみ SAFETY コメント付き局所許可）
- ポート定義は `ruviform-audio-core` に置き、外部クレートの型・エラー・feature 名を公開 API に露出しない。全公開 enum は enum レベル `#[non_exhaustive]`
- `ruviform-audio-device` は分離クレート＋opt-in とし、cpal 依存は同クレート内部に閉じる。feature 名は「バックエンド有効化の意図」命名で固有名詞を出さない。コアクレートからの強制依存なし。adapter→adapter 依存を作らない
- バックエンド選択はクレート/feature 依存で表現し、実行時の文字列指定にしない
- コールバック内は alloc/dealloc 禁止・ブロッキング/I/O/無制限待機/非有界処理を制限（RT 契約）。列挙・構成・開始・停止は非 RT 区間の API とする。バッファ受け渡しは事前確保済み領域への `&mut` 借用、デバイス入力は有界キュー
- バックエンドのストリーム時刻・タイムスタンプ取得を trait 要件に含める（`MasterClock` の A/V 同期基準要件）
- バックエンドエラーは抽象エラー型に正規化し外部エラー型を露出しない。バックエンド非対応機能は「非対応」を型/エラーで表現し、全機能を一律汎用化しない
- planar f32 が正準。interleaved は境界のみで、変換には audio-core の `from_interleaved` / `copy_to_interleaved` / `copy_from_interleaved` / `AudioBufferViewMut` を利用する
- 依存追加（cpal 等）は理由を記録し `Cargo.lock` を追跡する

## Acceptance Criteria

- [x] 評価軸に沿った候補比較が文書化され、cpal 採否＋代替候補（cubeb-rs / rtaudio-rs / 直結）の扱いが採否 ADR で確定している
- [x] `ruviform-audio-core` にデバイス境界 trait（ポート）が定義され、外部型・エラー・feature 名が公開 API に露出していない。全公開 enum は `#[non_exhaustive]`
- [x] `ruviform-audio-device` が分離クレートとして存在し cpal 依存がそこに閉じている（feature 名に cpal を露出しない・コアクレートからの強制依存なし）
- [x] PoC が「デバイス列挙 → 既定出力構成の取得 → 出力ストリーム開始 → コールバック到達（planar f32 へ境界変換）」を実証する。実機依存部分は手動検証手順として文書化し、CI にはデバイス非依存テストのみ置く
- [x] コールバック内の非 alloc 規律が trait 契約として明示され、可能な範囲で検証されている（assert_no_alloc 相当またはコードレビュー証跡）
- [x] `mise run fmt-check` / `check` / `lint` / `test` および `mise exec -- prek run --all-files` がパスし、`unsafe` なし（例外は SAFETY コメント付き局所許可）

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| duplex（同期入出力）対応 | 要件化しない。trait は第 2 backend を追加可能な設計のみ要求 | duplex が要件化した時点で採否 ADR 再評価（cpal PR #1229 追跡。cubeb/rtaudio が代替候補） | duplex 前提の trait 形状欠落で第 2 backend 追加が破壊的変更になる |
| `RtContext` mint API | 未実装のまま。trait は将来の配線を阻害しない設計のみ | RT 基盤 Issue（`ruviform-rt`） | コールバック契約と capability モデルが不整合になる |
| デバイス入力パイプライン | 有界キューの契約方向のみ記録し実装しない | engine 層（`ruviform-audio`）設計 Issue | 入力経路の責務境界が曖昧になる |
| WASAPI exclusive / ASIO | 非対応のまま。ASIO は opt-in 隔離・build.rs SDK 自動ダウンロードは再現性注意として記録 | プラットフォーム要件の後続評価 | 要件化時に trait/API の穴が露呈する |
| 実機ループバック CI 化 | 手動検証手順の文書化に留める | CI 方針の後続 Issue（CI 代替はオフラインアンダーラン＋alloc フック） | デバイス依存テストが CI を不安定化する |

## Sources and Relations

- 一次ソース（`relations.source`）: GitHub Issue [#23 [poc] オーディオデバイスバックエンド評価（cpal 等）と採否](https://github.com/scarecrow173/ruviform/issues/23)
- 上流（`relations.derives-from`）: discovery [evaluate-audio-device-backends.md](../discovery/evaluate-audio-device-backends.md)（評価軸・候補比較・推奨仮説・未確定点）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（責務表・RT 契約・分類表・Phase 2 完了目安）、Issue #23
- 前提（`relations.depends-on`）: spec [establish-rust-dev-foundation.md](establish-rust-dev-foundation.md)（`SPEC-034IcwhwIFSNUNKtX7KK0Q`）が確立した workspace・lint・mise 検証タスクを検証基盤として利用する
- 関連（`relations.related`）: [realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md)（コールバック内 RT 制約・API 単位保証）、[media-io-backend-abstraction.md](../adr/media-io-backend-abstraction.md)（外部型非露出・分離クレート＋opt-in・feature 命名）、[public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md)（公開 enum 方針）、[platform-and-toolchain-targets.md](../adr/platform-and-toolchain-targets.md)（対象プラットフォーム・MSRV 1.85）、採否 ADR [select-audio-device-backend.md](../adr/select-audio-device-backend.md)（並行の決定記録。本 spec の成果物として確定する）、[AGENTS.md](../../AGENTS.md)（暫定規約）
- 参考（`relations.references`）: 親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（設計ベースラインの一次ソース）
