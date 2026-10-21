---
id: "SPEC-034JceNZQvAawWwMH4ct91"
type: "spec"
status: "approved"
title: "時間型拡張（FrameRate/TimeBase 意味分離・Timecode 拡張・丸め誤差累積方針）"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/42"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/implement-issue-58-time-type-extension.md"
  depends-on:
    - "implement-media-data-model-api.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "issue-18-followup-enum-and-time-fix.md"
  derived-by:
    - "../adr/framerate-timebase-separation-and-timecode-extension.md"
    - "../designs/time-model-extension.md"
    - "../plans/implement-issue-58-time-type-extension.md"
  verifies: []
  verified-by:
    - "../test-specs/time-type-extension-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/41"
    - "https://github.com/scarecrow173/ruviform/issues/27"
  defers: []
  deferred-by: []
---

# 時間型拡張（FrameRate/TimeBase 意味分離・Timecode 拡張・丸め誤差累積方針）

## What Should Be Built

GitHub Issue #42 で定義された時間モデル拡張の設計成果物として、2 系統の決定・記録を行う。本 spec が要求するのは設計判断の確定と文書化であり、コード実装自体は後続の実装 Issue #58 に委ねる（Issue #42 は design → spec フローの設計 Issue。#58 は #27 より先に完了させる前提）。

- **`FrameRate` / `TimeBase` の意味的分離と `Timecode` 拡張の方式決定**: フレームレート（nominal fps: 24, 25, `30000/1001`, `60000/1001` 等）を表す型と `TimeBase`（任意の有理数 tick 長 — 1 秒の分割）の責務分離を決定し、`Timecode` 関連 API が意味的に正しい型を受け取る形を定義する。現行 `Timecode::from_media_time` / `from_media_time_drop` の `frame_rate: TimeBase` 引数について、分離導入時の互換性方針（破壊的変更 or 新 API 併設）を決定・記録する。
- **ドロップフレーム規則の拡張性の分類記録**: 30/60 nominal 以外のドロップフレーム一般化、フィールドレート等の拡張性を検討し、確定/保留/要PoC の 3 区分で設計ベースラインの分類表に記録する。
- **丸め誤差累積・sample/frame 精度の方針確定**: `time.rs` の doc 注記済み原則（「呼び出しごとに最大 1 回の丸め」「安定アンカーからの変換」、#18 で導入）を、A/V 同期（audio master + video PTS 追従）・編集タイムライン等のユースケースに落とした設計指針として定義し、累積誤差が許容閾値内に収まることを検証できる形で記述する。実装側の検証手段（連鎖変換のプロパティテスト等）を特定する。

成果物は本 spec と、判断正本となる ADR（[framerate-timebase-separation-and-timecode-extension.md](../adr/framerate-timebase-separation-and-timecode-extension.md)）、API 形状と丸めポリシーの詳細設計（[time-model-extension.md](../designs/time-model-extension.md)）、および設計ベースラインの分類表更新である。

## Why Now

- Issue #41 spec の Deferred Design Concerns で「`FrameRate`/`TimeBase` 意味分離・`Timecode` 拡張・丸め誤差累積ポリシー」が「新規 Issue（起票対象）」として延期記録され、Issue #42 として起票済み。#41 で確定した `convert` 契約（丸めモード・`Err` 条件は現行維持）を前提に、延期項目を設計確定へ進める段階にある。
- 現行 `Timecode::from_media_time(t, frame_rate: TimeBase)` は `TimeBase`（1 tick の有理数長）型の引数に fps 向きの有理数（`30000/1001` = 29.97 fps）を渡す設計であり、tick 長と fps が逆数関係にある 2 つの意味を同一型が担っている。`SampleRate(TimeBase)`（audio-core、Hz 意味を与える newtype）の先例があるにもかかわらず動画側に対応する型がなく、video-core にも frame rate 型は未存在である。
- Issue #27（動画エンジン: フレームパイプライン・A/V 同期・シーク）が本論点の消費者であり、エンジン側が `MediaTime` と timecode を組み合わせる API を使い始める前に型と丸め方針を確定させる方が変更コストが最小である。ワークスペースは `publish = false`・外部利用者不在のため、破壊的変更が許容できるのは現時点に限られる。
- 丸め誤差累積は現状 doc 注記（「repeated base-hopping is discouraged」）に留まり、ユースケース別の精度基準・検証可能な許容閾値・検証手段が未規定である。A/V 同期・編集タイムラインの実装が始まる前にポリシーとして明文化する必要がある。

## Users and Value

- **engines / media-io の実装者（Issue #27 等）**: `Timecode` 変換が意味的に正しい引数型を取ることで、fps と timebase の混同ミスが型で防げる。A/V 同期・編集計算での丸め精度要件が基準化され、どの精度が要求される文脈かを迷わずに済む。
- **API 利用者・将来の外部利用者**: `FrameRate` と `TimeBase` の責務分離により、秒の有理数分割（tick 長）とフレーム/秒（fps）の逆数関係が型で明示され、誤用の芽を設計段階で摘める。
- **レビュアー・後続 Issue の作業者**: ドロップフレーム拡張性の分類（確定/保留/要PoC）がベースライン分類表に記録され、再着手トリガが明確になる。丸め累積の許容閾値と検証手段が定義され、実装の正しさを客観的に検証できる。

## Scope

### In Scope

- `FrameRate` / `TimeBase` 意味分離の方式決定（newtype / 独立型 / 型を増やさない現状維持 / 外部型採用の選択肢評価を含む）と ADR への記録
- `Timecode::from_media_time` / `from_media_time_drop` の引数型を `FrameRate` 系へ移行する際の互換性方針（破壊的変更 or 新 API 併設）の決定と記録
- 新しい `Timecode` API シグネチャの設計（`frame_rate` 引数の型、`FrameRate` → `TimeBase` 等の変換規約、`nominal_fps` 等の派生情報の提供形）を設計文書に記述
- ドロップフレーム規則の拡張性（30/60 nominal 以外への一般化、フィールドレート）の検討と、確定/保留/要PoC 分類の設計ベースライン分類表への記録
- `#[non_exhaustive]` ポリシー（[enum ADR](../adr/public-enum-non-exhaustive-policy.md)）に整合する拡張設計であることの確認（`Error` の `#[non_exhaustive]` による将来 variant 追加余地の確保等）
- 丸め誤差累積ポリシーの定義:「呼び出しごとに最大 1 回の丸め」「安定アンカーからの変換」「base-hopping 禁止」を文書契約として明文化し、ユースケース別（A/V 同期・編集タイムライン・Timecode 表示）の sample 精度 / frame 精度基準表、許容閾値の定義方法、実装側検証手段（連鎖変換プロパティテスト等）を設計文書に記述
- 設計ベースライン分類表・`derived-by` relations、`docs/designs/overview.md` の Detailed Design Documents リスト、各ディレクトリ索引の更新

### Out of Scope

- コード実装そのもの（`FrameRate` 型・`Timecode` シグネチャ変更・テストの実装は後続の実装 Issue #58）
- 音楽時間（小節/拍/PPQN・テンポマップ）— Issue #33 スコープ（Issue #42 非目標どおり）
- `MediaTime::convert` / `MediaDuration::convert` の変換契約自体の変更（丸めモード・`Err` 条件は #41 spec どおり現行維持）
- `MasterClock` の実装（ruviform-rt・Issue #27 系のスコープ。本 spec はアンカーとしての参照位置づけのみ）
- `Timecode` → `MediaTime` 方向の逆変換 API の確定（設計文書で Deferred に記録）
- frame index 型の新設（所有は media-core vs video-core の境界問題として設計文書で Deferred に記録。Issue #26/#27 境界）

## Requirements

- Rust edition 2024・MSRV 1.85・stable のみを前提とした設計であること。`unsafe` 不使用・media-core 外部依存ゼロの現行方針と整合すること
- `FrameRate` を導入する場合は `SampleRate(TimeBase)` の先例（`crates/ruviform-audio-core/src/time.rs`）に整合する newtype 構成を基本案とし、private フィールド + 検証付きコンストラクタ（不変条件を ctor で強制）とすること。外部クレート型（`vtc` 等）を公開 API に露出しない
- `Timecode` API の移行方針は「破壊的変更 or 新 API 併設」の二者をトレードオフつきで評価し、決定と根拠を ADR に記録すること
- ドロップフレーム拡張性の分類は設計ベースラインの 3 区分（確定/保留/要PoC、「将来検討への無分類の先送りはしない」原則）で記録し、保留項目には再着手トリガを明記すること
- 拡張設計は [enum ADR](../adr/public-enum-non-exhaustive-policy.md) の統一ポリシーに整合すること（公開 enum の新規追加・`Error` variant 拡張余地を `#[non_exhaustive]` で担保）
- 丸め誤差累積ポリシーは型強制ではなく文書契約として定義し、型強制を採らない理由を記録すること。ユースケース別精度基準と累積誤差の許容閾値は検証可能な記述（例: 各ホップの丸め誤差 ≤ そのホップ宛先 base の 0.5 tick、アンカーからの直接変換では累積しないため宛先 base の 0.5 tick 以下）とすること
- 新 API の RT 契約は [RT ADR](../adr/realtime-contract-and-verification-policy.md) どおり API 単位の `**RT contract:**` ラベルで定義することを設計文書に明記すること
- 作成・更新する文書は doc-suite 規約（front matter 全 relations キー・索引更新・相対パスリンク）に従い、`node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` の findings が 0 であること

## Acceptance Criteria

- [ ] **AC1**: `FrameRate`/`TimeBase` 意味分離の方式（選択肢評価つき）と `FrameRate` の型形状（private フィールド + 検証付き `new()`、fps 向き有理数）が ADR に決定記録され、`Timecode` 関連 API が意味的に正しい型を受け取る新シグネチャが設計文書に定義されている
- [ ] **AC2**: `Timecode::from_media_time(_drop)` の `frame_rate` 引数の互換性方針（破壊的変更 or 新 API 併設）が決定され、根拠（`publish = false`・外部利用者不在等）とともに ADR に記録されている
- [ ] **AC3**: ドロップフレーム規則の拡張性（30/60 nominal 以外への一般化・フィールドレート）が検討され、確定/保留/要PoC の分類が設計ベースラインの分類表と設計文書の分類表に記録されている（保留は再着手トリガ明記）
- [ ] **AC4**: 丸め誤差累積ポリシーが設計文書に明文化され、A/V 同期（audio master + video PTS 追従）・編集タイムライン・Timecode 表示等のユースケース別に sample 精度 / frame 精度の要求基準表、累積誤差の許容閾値の定義方法、実装側検証手段（連鎖変換プロパティテスト等）が記述されている
- [ ] **AC5**: 設計が `#[non_exhaustive]` ポリシーと整合している（新規公開 enum を増やさない、または増やす場合の方針、`Error` の `#[non_exhaustive]` による拡張余地を明記）
- [ ] **AC6**: 設計ベースラインの分類表・`derived-by`、`docs/designs/overview.md` の Detailed Design Documents、specs/adr/designs 各 README 索引が更新され、`audit_docs.js --type all --json` の findings が 0 である

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `Timecode` → `MediaTime` 逆変換 API | Issue #42 受入条件外。設計文書で Deferred として位置づけのみ記録 | 編集/NLE 系機能の需要発生時（後続 Issue） | ラベル→時刻の逆解釈を各呼び出し側が独自実装し不整合が生じる |
| フィールドレート・非 NTSC nominal のドロップ一般化 | 拡張性の検討と分類記録のみ（設計しない） | 分類表の保留項目（再着手トリガは需要発生・インターレース対応時） | 現行 2 レート限定が暗黙の最終決定として固定化される |
| `vtc` 等外部クレート採用の再評価 | ADR で却下候補として記録（依存ゼロ・外部型非露出方針） | 依存追加規約どおり各実装 Issue での再評価余地 | 依存ゼロ方針との不整合を暗黙に持ち込む |
| frame index 型の所有（media-core vs video-core） | 本 spec の要求外。設計文書で Deferred に記録 | Issue #26 / #27 の境界設計 | フレーム番号の型所在が曖昧なまま実装が進む |
| 音楽時間（小節/拍/PPQN・テンポマップ） | Issue #42 非目標どおり対象外 | Issue #33（ベースライン保留項目） | メディア時間と音楽時間の混在設計 |
| `FrameRate` 実装・`Timecode` シグネチャ変更・プロパティテストのコード | 設計のみ。コードは本 spec の成果物に含めない | 後続の実装 Issue #58 | 設計未確定のまま実装が先行する |

## Sources and Relations

- 一次ソース: GitHub Issue [#42 [design] 時間型拡張（FrameRate/TimeBase 意味分離・Timecode 拡張・丸め誤差累積方針）](https://github.com/scarecrow173/ruviform/issues/42)（`relations.source`）。Issue #41 spec の Deferred 表が指す「新規 Issue」として起票された設計 Issue。
- 上流文書（`relations.derives-from`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（時間モデルの確定契約: 有理数中核・音声サンプル時刻優先・ドロップフレームは表示規約・責務所在）と [issue-18-followup spec](issue-18-followup-enum-and-time-fix.md)（`SPEC-034J6OCXHiP1izKRzbMmgz`。本論点の延期元トリアージと `convert` 契約現行維持の前提）。
- 派生（`relations.derived-by`）: [FrameRate/TimeBase 分離 ADR](../adr/framerate-timebase-separation-and-timecode-extension.md)（方式・互換性・拡張性分類の決定正本）と [時間モデル拡張の設計](../designs/time-model-extension.md)（API 形状・丸め誤差累積ポリシーの詳細設計）。
- 前提（`relations.depends-on`）: [implement-media-data-model-api.md](implement-media-data-model-api.md)（`SPEC-034J5FYaYfeL5zkzb4sIV4`。本 spec が拡張対象とする `TimeBase`/`MediaTime`/`Timecode` API 表面を確定した先行 spec）。
- 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（暫定規約 — unsafe 原則禁止・依存追加規約・`#[non_exhaustive]` ポリシー）。
- 参考（`relations.references`）: [Issue #41](https://github.com/scarecrow173/ruviform/issues/41)（本論点の延期元フォローアップ Issue）、[Issue #27](https://github.com/scarecrow173/ruviform/issues/27)（動画エンジン — 本設計の消費者）。
