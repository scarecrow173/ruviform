---
id: "PLAN-034KoNkBdCIchL4xE1DI7Z"
type: "plan"
status: "approved"
title: "Issue #91 実施計画 — 音楽時間 ABI 拡張（TransportInfo.musical + CLAP/VST3 写像）"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/music-time-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/tasks/musical-transport-abi-impl.md"
    - "docs/tasks/musical-transport-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/musical-transport-abi-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #91 実施計画

## 手順

1. `media-core::TransportSnapshot` に `musical: Option<MusicalTimeInfo>` 追加（`reserved` 温存・`with_musical`/`musical` アクセサ）
2. `plugin-core::TransportInfo` に同フィールド追加（doc の「musical time を持たない」記述を更新）
3. CLAP `adapt_transport`（plugin-clap）: `HAS_BEATS_TIMELINE`+`HAS_TEMPO` で `musical=Some`。beats→tick は `bits×960/65536` round-half-up（`to_bits()` 整数経路）、`tempo`→`bpm_milli` は `tempo*1000` round、sig は `HAS_TIME_SIGNATURE` で u8 写像（なければ 0/0）、`bar_start` は beats 同変換
4. VST3 `transport()`（plugin-vst3）: `kProjectTimeMusicValid`+`kTempoValid` で `musical=Some`。PPQ double→tick は `ppq*960` round、sig/bar は各 `kTimeSigValid`/`kBarPositionValid` で写像（なければ 0/0・0）
5. 単体テスト + `rt_` 検証・検証列

## 設計から実装へ具体化する判断

- 写像方向は inbound（規格→中立）のみ — outbound はホスト側アダプタ不在のため YAGNI
- `musical=Some` の成立条件: 位置（beats/PPQ）+テンポの双方が有効（`bpm_milli` に未設定表現がないため）。sig/bar は個別フラグで任意
- `bar_number` は中立側に対応フィールドなし（`MusicalTimeInfo` 非対応）— 写像しない
- **設計書誤記の修正**: `clap_beattime` の係数は実 ABI（`CLAP_BEATTIME_FACTOR` = 2³¹）に合わせて design の写像表を 65536 → 2³¹ に修正済み（実装は ABI 定数を `clack_common` から参照）
- `TransportSnapshot`（media-core）にも `musical` を追加 — plugin-core 側と対称のスナップショット境界であり、#33 設計の ABI スナップショット流通方針を保つため
- `bar_start_tick` は sig 有効時のみ写像（sig 0/0 = 未設定時は慣例通り 0。VST3 は加えて `kBarPositionValid` を要求）
- `tempo` は finite かつ正値のみ受理（NaN/0/負/inf → `musical=None`）。`bpm_milli` は ×1000 round 後 `1..=999_999_999` にクランプ

## Verification

- [x] `mise run` 検証列全パス・doc 監査チェーン起因 findings 0
- [x] test-spec 全ケース（flag ゲート・変換往復 ±1・unset 表現）
