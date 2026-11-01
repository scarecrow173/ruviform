---
id: "ADR-20261101-variable-ppqn-rational-beat"
type: "adr"
status: "accepted"
title: "可変PPQN・分解能非依存の音楽時間・Native MIDI時刻保持"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
metadata:
  adr:
    supersedes:
      - "docs/adr/musical-time-model.md"
    decision-makers: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/synvelis/issues/19"
    - "https://github.com/scarecrow173/ruviform/issues/189"
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
  supersedes:
    - "docs/adr/musical-time-model.md"
  superseded-by: []
  related:
    - "docs/designs/music-time-model.md"
    - "docs/designs/time-model-extension.md"
    - "docs/adr/plugin-rt-ownership-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/musical-time-model.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/90"
    - "https://github.com/scarecrow173/ruviform/issues/91"
  defers: []
  deferred-by: []
---

# 可変PPQNと分解能非依存の音楽時間

## Status

**accepted (architectural direction, 2026-11-01); implementation and API migration pending.**

本ADRは[既存の固定960 ADR](musical-time-model.md)の**PPQN固定および整数Tickが音楽位置の唯一の正本である方針を置き換える**。旧ADRの履歴と、#90/#91で実装された現在の960ベースのAPIは残る。実装完了を意味しない。

## Context

初期の音楽時間設計ではPPQN=960を固定し、`Ppqn`型・`Tick(i64)`・`TempoMap`・`MusicalTimeInfo`・CLAP/VST3写像がこの前提で構築された。

Synvelisは汎用DAWとしてProjectごとのPPQN設定、異なるSMF/MIDI Source-native分解能、より高精度な記譜位置、元Timestampの非破壊保存を要求する。960を唯一の音楽時間格子とすると、異なる分解能のSource位置や7連符等を整数Tickへ変換する時点で情報が失われ得る。

RuviformはSynvelis専用ではなく、マルチメディア処理で再利用できる時間契約を提供する必要がある。

## Decision

1. **Resolution-independent canonical musical position:** 音楽位置の正本は、特定PPQNの整数Tickだけに依存しない。正規化された有理数の拍位置（Beat Position）または同等の損失のない表現を採用する。整数幅・分母上限はPoCで決定し、溢れ/未対応値を定義済みエラーで拒否する。
2. **Configurable PPQN:** Ruviform汎用の`Ppqn`は960しか取り得ない制約を段階的に解除する。PPQNは正の整数であり、上限・精度・リソース予算は検証で定める。**新規Synvelis Projectの既定値960はSynvelisの製品Policy**であり、Ruviform Coreが他値を拒否する理由にはならない。
3. **Tickには分解能文脈が必要:** `Tick`の数値だけを異なるPPQN間で比較・合成しない。PPQNと組の型、または型安全な変換Contextを利用し、明示的なBeat位置変換を経由する。
4. **Source-native fidelity:** ImportされたMIDIの元Division/PPQNとTick、録音Deviceの時刻/Clock Domain、Source-relative Metadataを可能な限り非破壊に保持。Project PPQN変更だけでは元EventをQuantize/Retimingしない。時刻に基づく明示的なQuantizeとは分離する。
5. **Tempo/Time conversion:** Beat Position ↔ MediaTimeはTempoMapと明示丸めで変換し、特定960定数の依存を排除する。TempoMapの非RT Compile、不変Snapshot、RT有界クエリ、安定Native Anchorからの変換と境界の単回丸めを維持。**Piecewise-constant TempoMapは本変更では廃止しない。**
6. **Adapter ownership:** CLAP/VST3などのPlugin ABIには規格固有のBeat/PPQ表現へ変換する。内部PPQNを固定と推定せず、基準拍位置・精度・丸めを利用する。汎用時間型はmedia-core側、規格写像はAdapter側、Synvelis Project/Clip/MIDI/Undoはアプリ側で所有。
7. **Compatibility and migration:** 既存固定960型/処理の利用者に対し段階的な互換層、明示変換またはAPI移行方針を提供する。既存960 Projectの意味は変えず、Snap/Loop/Negative Pre-roll/Automation/Recording/Offlineで回帰試験する。

## Alternatives considered

| Option | Decision |
| --- | --- |
| 固定PPQN=960 | 初期実装として妥当だったが、Source Native Fidelity/一般化要件と衝突するため置換 |
| Projectごと可変PPQN + 整数Tickのみ正本 | 可変でも分解能に非整合な位置（1/7拍等）を丸めてしまうため不十分 |
| 常時非常に大きな固定PPQN | すべての有理数位置を完全には表現できず、値域とOverflowにも問題 |
| 浮動小数点の拍位置を唯一正本 | 丸め・可逆性・決定性の理由で不採用 |
| **有理Beat Position正本 + 可変PPQN** | **採用**：正確な音楽位置とUI/MIDIの明示分解能を分離 |

## Consequences and compatibility boundaries

- 既存の`music.rs`には960の定数依存が実装されている。ADR採用後も**現在のコードは固定960のまま**であり、新しいAPIが存在するとは主張しない。
- `MusicalTimeInfo.ppqn`やPlugin Transport変換、Bar/Beat分解、TempoMapセグメント計算に影響する。既存Call SiteとAudio/Video/CLI Consumerを監査する。
- High-precision Source EventとProject内部Beatを分けても、MIDI Export時の選択したPPQNが元位置を表現できない場合は丸め/損失があり得る。Loss報告・Export Config・原本保持を設計する。
- Ruviformは正確な有理数演算と無制限の分母を無条件に保証するのではなく、有限の計算能力を明示し、Overflow・容量・RT Budgetの条件を検証する。
- MIDI FileのSMF DivisionにはPPQN以外の形式もあるため、Import元のTimebase/Detailed MetadataがPPQNであると決めつけない。対応範囲は別PoCで決める。
- 旧[音楽時間設計](../designs/music-time-model.md)や閉じた#90/#91は実装履歴。後継のMigration Issue #189がコードと設計文書の更新を追跡する。

## Acceptance and migration plan

1. Ruviform #189で既存960依存の型/変換/Adapter/Documentation/Testsを棚卸しする。
2. Stable Beat Position + Validated PPQN + Safe Tick ConversionのAPI/上限/PrecisionをPoCし、新規実装と互換層を確定。
3. TempoMap / MusicalTimeInfo / CLAP/VST3 Transportの可変分解能対応、Roundtrip、Overflow、RT NoAlloc/Boundedのテストを導入。
4. Synvelis #19でProject PPQNの既定値960、Save/Reload、Raw MIDI Sourceの時刻保持、明示Quantizeと移行を検証。
5. 必要なRuviform既存Design/Spec/READMEを移行完了時に更新し、旧仕様を有効なAPIと誤認させない。

**実装完了ではなく、設計上の決定を記録したADRである。**
