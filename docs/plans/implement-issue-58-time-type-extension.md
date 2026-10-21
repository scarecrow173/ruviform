---
id: "PLAN-034JNstCXHhzzwOvo9iiCv"
type: "plan"
status: "approved"
title: "Issue #58 時間型拡張の実装（FrameRate 導入・Timecode シグネチャ移行・丸め誤差テスト）"
created: "2026-10-21"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/58"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/time.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/timecode.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/time-type-extension-framerate-timecode-rounding.md"
  implemented-by:
    - "../tasks/implement-framerate-type.md"
    - "../tasks/migrate-timecode-to-framerate.md"
    - "../tasks/add-time-precision-tests.md"
    - "../tasks/sync-docs-and-verify-issue-58.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/time-type-extension-framerate-timecode-rounding.md"
    - "../designs/time-model-extension.md"
    - "../adr/framerate-timebase-separation-and-timecode-extension.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  derived-by:
    - "../tasks/implement-framerate-type.md"
    - "../tasks/migrate-timecode-to-framerate.md"
    - "../tasks/add-time-precision-tests.md"
    - "../tasks/sync-docs-and-verify-issue-58.md"
  verifies: []
  verified-by:
    - "../test-specs/time-type-extension-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/42"
    - "https://github.com/scarecrow173/ruviform/issues/27"
  defers: []
  deferred-by: []
---

# Issue #58 時間型拡張の実装（FrameRate 導入・Timecode シグネチャ移行・丸め誤差テスト） Implementation Plan

## Goal

Issue #42 の設計チェーン（spec `SPEC-034JceNZQvAawWwMH4ct91`・ADR `ADR-034JceOKDwTeWus9xBVXND`・design `DESIGN-034JcePtzKFix4IuxyE5mD`、すべて確定済み）で決定された時間型拡張を `ruviform-media-core` に実装する: `FrameRate`（`TimeBase` newtype・fps 向き有理数）の導入、`Timecode::from_media_time`/`from_media_time_drop` の `frame_rate` 引数を `TimeBase` から `FrameRate` へ破壊的置換、設計文書で定義された丸め誤差累積ポリシーの検証テスト（時間精度行列・連鎖変換誤差上界）の追加。Issue #27（動画エンジン）より先に完了させる前提のブロッカー Issue。

## Scope

単一の実装ストリームとして扱う。`FrameRate` 導入・`Timecode` 移行・丸め誤差テストは同一クレート（`ruviform-media-core`）内の同一関心事（時間モデルの型分離と精度契約）であり、Issue #58 の受け入れ条件として単一 Issue で管理されている。

範囲外（Issue #58 スコープ外・#42 spec Out of Scope と一致）:

- `Timecode` → `MediaTime` 逆変換 API（設計 Deferred。編集/NLE 需要発生時の後続 Issue）
- ドロップフレームの非 NTSC 一般化・フィールドレート（保留。再着手トリガはインターレース/非 NTSC drop 実需要）
- frame index 型の新設・所有決定（Issue #26/#27 境界）
- 音楽時間（小節/拍/PPQN・テンポマップ。Issue #33）
- `MediaTime::convert` / `MediaDuration::convert` の変換契約自体の変更（丸めモード・`Err` 条件は現行維持）
- `MasterClock` 実装・A/V 同期の実配線（ruviform-rt/#27 系。本 plan はアンカー規約の検証のみ）
- `vtc` 等外部クレート採用（ADR Option D 却下済み・依存ゼロ方針）
- `proptest` 等の依存追加（下記 Dependencies で deterministic 列挙を選択）

## Upstream Documents

- [../specs/time-type-extension-framerate-timecode-rounding.md](../specs/time-type-extension-framerate-timecode-rounding.md) — `SPEC-034JceNZQvAawWwMH4ct91`（approved）。Issue #42 の設計 spec で、コード実装を本 Issue #58 に明示的に委譲している正本（「`FrameRate` 実装・`Timecode` シグネチャ変更・プロパティテストのコード → 後続の実装 Issue #58」）
- [GitHub Issue #58](https://github.com/scarecrow173/ruviform/issues/58) — 一次ソース（`relations.source`）。実装スコープと受け入れ条件（設計 Verification Notes 全項目・`nominal_fps` u64 中間演算・精度行列・連鎖プロパティテスト・`nominal_fps()==0` 拒否回帰・drop 集合維持・作業契約遵守）を規定

## Design Inputs

- [../designs/time-model-extension.md](../designs/time-model-extension.md) — `DESIGN-034JcePtzKFix4IuxyE5mD`（approved）。`FrameRate` API 形状（`new`/`from_fps`/`numerator`/`denominator`/`frame_duration`/`nominal_fps`）、`Timecode` 新シグネチャ、変換規約（`frame_duration()` が唯一の `TimeBase` 返却 API・`From` impl 非提供）、丸め誤差累積ポリシー（3 原則・ユースケース別精度基準・0.5 tick 閾値・検証手段）、分類表の正本
- [../adr/framerate-timebase-separation-and-timecode-extension.md](../adr/framerate-timebase-separation-and-timecode-extension.md) — `ADR-034JceOKDwTeWus9xBVXND`（accepted）。Option A（`TimeBase` newtype）採用・破壊的置換・drop 対象集合現行維持・`FrameRate` struct 形状の決定正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — 時間モデル確定契約（有理数中核・`f64` 非中核・音声サンプル時刻優先・ドロップフレームは表示規約）と分類表記録の整合対象
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT ADR。新公開 API への `**RT contract:**` ラベル付与規則の正本
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — 公開 enum `#[non_exhaustive]` 統一方針。`Error` は `#[non_exhaustive]` のまま既存 variant を再利用する方針の根拠

## File Map

- `crates/ruviform-media-core/src/time.rs` — **T1 主対象**: `FrameRate` 型を追加する（`TimeBase` newtype・private フィールド・検証付き `new()`/`from_fps()`・`numerator()`/`denominator()`・`frame_duration()`・`nominal_fps()`・compact `Debug`・単体テスト）。配置は `time.rs` 内 — `SampleRate` が audio-core の `time.rs` に置かれる先例に倣い、rate 系 newtype は時間プリミティブと同居させる判断（設計文書はクレート配置のみ規定し、モジュール選択は実装 Issue に委任）。**T3 でも使用**: 連鎖変換プロパティテストを `mod tests` に追加
- `crates/ruviform-media-core/src/timecode.rs` — **T2 主対象**: `from_media_time`/`from_media_time_drop` の `frame_rate` 引数を `FrameRate` に置換、nominal grid 計算を `FrameRate::nominal_fps()` へ移譲、`elapsed_frames` の引数型を `FrameRate` に変更、doc・doctest・単体テストを移行
- `crates/ruviform-media-core/src/lib.rs` — `pub use time::{…, FrameRate}` 追加、crate-level doc の型列挙更新、`rt_drop_frame_roundtrip` テストの `FrameRate` 移行
- `crates/ruviform-media-core/src/error.rs` — 変更不要（`InvalidValue("frame_rate")`・`UnsupportedDropFrameRate` を再利用。新 variant なし）
- `AGENTS.md` — T4 で `ruviform-media-core` の記述に `FrameRate` 導入済みを反映するか確認（ワークスペース節のクレート説明が型列挙レベルの場合のみ追記）

依存追加なし（media-core 依存ゼロ維持・dev-dependency も追加しない — 後述）。

## Implementation Sequence

### T1: `FrameRate` 型の実装（task: implement-framerate-type）

- [ ] `time.rs` に `pub struct FrameRate(TimeBase)` を追加（`#[derive(Copy, Clone, PartialEq, Eq, Hash)]`・`Ord`/`PartialOrd` なし・`#[non_exhaustive]` なし — struct は ctor で不変条件を強制）
- [ ] API: `pub const fn new(numerator: u32, denominator: u32) -> Result<Self>`（`TimeBase::new` に委譲・失敗は `Err(Error::InvalidValue("frame_rate"))` に正規化）、`pub const fn from_fps(fps: u32) -> Result<Self>`（`new(fps, 1)` 相当・`fps == 0` で同エラー）、`pub const fn numerator(self) -> u32`、`pub const fn denominator(self) -> u32`、`pub const fn frame_duration(self) -> TimeBase`（逆数 `TimeBase::new(denominator, numerator)`・保持値非ゼロで失敗し得ない）、`pub const fn nominal_fps(self) -> u32`（round-half-up `(2*num + den) / (2*den)` を `u64` 中間演算で — `u32` 中間だと `num > ~2^31` で `2*num` が溢れる）
- [ ] `fmt::Debug` を `FrameRate(30000/1001)` 型の簡潔表示に（`SampleRate(48000)` 先例）
- [ ] 単体テスト: gcd 正規化（`new(60_000, 2_002)` → `30000/1001`）、ゼロ拒否（`new(0,1)`/`new(1,0)`/`from_fps(0)` → `Err(InvalidValue("frame_rate"))`）、`frame_duration()` の逆数（`30000/1001` fps → `TimeBase(1001/30000)`）、`nominal_fps()` の round-half-up（`24`/`25`/`30000/1001`→30/`60000/1001`→60/`24000/1001`→24）と `u32::MAX` 級境界（`4294967295/1` → `u32::MAX` に飽和しないオーバーフローなし計算、`1/4294967295` → 0）
- [ ] 全公開 API に `**RT contract:**` ラベル（RT-suitable — `const`・純粋整数演算・alloc なし）。`to_time_base()`・`From<FrameRate> for TimeBase`・`From<TimeBase> for FrameRate` を設けないことを doc で明示

### T2: `Timecode` API の `FrameRate` 移行（task: migrate-timecode-to-framerate）

- [ ] `from_media_time(t: MediaTime, frame_rate: FrameRate)`・`from_media_time_drop(t: MediaTime, frame_rate: FrameRate)` に破壊的置換（旧 `TimeBase` シグネチャ併設なし）。`elapsed_frames(t, rate)` ヘルパの引数も `FrameRate` に変更し `rate.numerator()`/`rate.denominator()` を使用
- [ ] `from_media_time` 内の nominal grid 計算（`(2*num + den) / (2*den)`）を削除して `frame_rate.nominal_fps()` に移譲（規則の二重実装を避ける）。`nominal_fps() == 0` 拒否は `Err(Error::InvalidValue("frame_rate"))` で現行維持
- [ ] 呼び出し側移行: `timecode.rs` 単体テスト・doctest 両関数・`lib.rs` の `rt_drop_frame_roundtrip` を `FrameRate` 構築に書き換え（例: `FrameRate::new(30_000, 1_001).unwrap()`・`FrameRate::from_fps(25).unwrap()`）。ワークスペース内の他呼び出し箇所がないことを grep で確認（現状 `Timecode` 呼び出しは media-core 内テスト・doctest のみ）
- [ ] 回帰テスト維持: `sub_fps_rate_is_rejected` 相当を `FrameRate` 版で残す（`FrameRate::new(1, 1_000)` → `nominal_fps()==0` → `Err(InvalidValue("frame_rate"))`）。drop 対応集合 `30000/1001`・`60000/1001` のみ・他は `Err(UnsupportedDropFrameRate)` の契約を維持するテスト（`drop_requires_ntsc_rate` 相当）を `FrameRate` 版で残す
- [ ] エラー doc（`# Errors`）と `**RT contract:**` ラベルを新シグネチャに合わせて更新

### T3: 時間精度・連鎖変換プロパティテストの追加（task: add-time-precision-tests）

- [ ] **時間精度行列テスト**: 音声 base（`1/44100`・`1/48000`・`1/90000`）× 映像 rate（`FrameRate` の `frame_duration()` から導出する tick base: `24`・`25`・`30000/1001`・`60000/1001` fps 相当）の `MediaTime::convert` 変換を網羅。負値拒否（負 instants の丸め対称性を既存 `Rounding` 契約どおり検証）・境界値（`0`・`i64::MAX` 近傍・`i64::MIN` 近傍）を含む。アンカー直接変換の誤差が宛先 base の 0.5 tick 以下に収まることを全組み合わせで検証
- [ ] **往復変換誤差上界テスト**: `t.convert(B, NearestEven).convert(元base, NearestEven)` の往復誤差が「中間 base B の 0.5 tick を元 base 単位に換算した値 + 元 base の 0.5 tick」以下であることを検証（粗い中間 base — 例: `1/48000` → `1/24` → `1/48000` — では大きな往復誤差が出得ることも境界ケースとして確認。設計例: 1000 tick @ `1/48000` → `1/24` で 0 に丸め → 復帰も 0 → 往復誤差 1000 tick が上界内に収まる）
- [ ] **N 回連鎖の累積上界テスト**: 代表 base 集合 `{1/48000, 1/44100, 1/90000, 1001/30000, 1001/60000, 1/25, 1/24}` の巡回連鎖で、累積誤差が「Σ（各ホップ宛先 base の 0.5 tick）を共通単位（秒相当の有理数比較）に換算した値」を超えないことを検証。対照実験として「アンカーからの直接変換は常に宛先 base の 0.5 tick 以下」を同じ base 集合で確認し、ポリシー遵守形と base-hopping 形の誤差差を観測する
- [ ] **実装方式**: `proptest` は採用せず、固定 base 集合 × 固定値集合の自前 deterministic 列挙で実装する（media-core は依存ゼロ方針 — dev-dependency すら現状持たず、列挙で要件を完全にカバーできるため依存追加規約上の理由記録・審査コストを回避。設計文書の「要PoC」分類項目を deterministic 列挙で解決した旨を IR に記録）
- [ ] 誤差測定は `f64` を介さず整数有理数比較で行う（`|往復誤差|` の tick 差を `i128` の `value * base` 換算で比較。中核表現に `f64` を持ち込まない契約の維持）

### T4: ドキュメント同期と総合検証（task: sync-docs-and-verify-issue-58）

- [ ] spec#42 `implemented-by` に本 plan、status を `implemented` へ。design#42/adr#42/baseline/RT ADR/enum ADR の `derived-by` に本 plan を追加（双方向 relation 整合）
- [ ] `AGENTS.md` ワークスペース節の media-core 記述への `FrameRate` 反映要否を確認・必要なら追記
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench`・`mise exec -- prek run --all-files` を実行・全パス
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` の findings が 0
- [ ] Issue #58 受け入れ条件全項目と設計文書 Verification Notes 11 項目の充足を確認し IR を記録

## Dependencies and Risks

### Dependency Graph

| 依存元 | 依存先 | 内容 |
| --- | --- | --- |
| T2 | T1 | `Timecode` 移行は `FrameRate` 型の存在が前提（シグネチャ・`nominal_fps()` 移譲・テストが `FrameRate` を構築） |
| T3 | T1 | 精度行列の映像 rate tick base は `FrameRate::frame_duration()` から導出 |
| T4 | T1・T2・T3 | 総合検証と文書同期は全実装完了後 |
| 本 plan | Issue #42 チェーン（spec/ADR/design 確定済み） | API 形状・互換性方針・丸めポリシーは上流で確定済み。本 plan は再設計しない |
| 本 plan | Issue #27（ブロック関係） | #58 は #27 の必須先行。本 plan の完了が #27 の前提 |

- 他クレートへの影響なし（`FrameRate` は media-core 公開 API として新設されるが、現行の呼び出し側は media-core 内テスト・doctest のみ。sibling crate は `Timecode` を呼んでいない — grep 確認済み）
- media-core ↔ audio-core/video-core の依存辺は不変（`FrameRate` は audio-core の `SampleRate` を参照しない・依存追加なし）

### Risk Register

| リスク | 影響 | 確率 | 対応 |
| --- | --- | --- | --- |
| `nominal_fps()` の `u32` 中間演算オーバーフロー | `2*num` が `num > ~21億` で `u32` を溢れ誤 nominal grid / パニックを生む | 中（`u32::MAX/1` 等の極端入力で顕在） | `u64` 中間演算で実装し、`4294967295/1`・`1/4294967295` の境界テストを T1 に必須化（Issue 受入条件） |
| 破壊的シグネチャ置換の呼び出し側漏れ | 旧 `TimeBase` 渡しの呼び出しがコンパイルエラーで残る | 低（ワークスペース内限定・grep で全域把握可能） | `from_media_time` 呼び出しを全件 `FrameRate` 構築へ移行し `mise run check` で漏れを検出 |
| 往復誤差上界の定義誤り | 上界式が実装より緩すぎ/厳しすぎで無意味なテストになる | 中 | 設計文書の定義どおり「中間 base 0.5 tick の元 base 換算 + 元 base 0.5 tick」を `i128` 有理数で計算し、対照実験（アンカー直接 ≤0.5 tick）で式の意味を固定 |
| 丸めポリシーの文書契約が形骸化 | ポリシーは型強制されないため後続実装で無視される | 低 | プロパティテスト（T3）が客観検証手段として残り、`time.rs` モジュール doc の既存注記と設計文書が規約を固定 |
| `TimeBase` 値としての fps 有理数の流出 | `to_time_base()` 等があれば `MediaTime::new(v, fr.into())` の混同がコンパイルを通る | 低（提供しないことを ADR/設計で確定） | 提供しない API を T1 の doc に明記し、レビュー・audit で監視 |

## Task Breakdown

| Task | 内容 | 対応 Phase |
| --- | --- | --- |
| [implement-framerate-type.md](../tasks/implement-framerate-type.md) | `FrameRate` 型の実装（`time.rs`）と単体テスト | T1 |
| [migrate-timecode-to-framerate.md](../tasks/migrate-timecode-to-framerate.md) | `Timecode` シグネチャの `FrameRate` 破壊的置換・呼び出し側/doctest 移行 | T2 |
| [add-time-precision-tests.md](../tasks/add-time-precision-tests.md) | 精度行列・往復誤差上界・N 回連鎖累積上界テスト（deterministic 列挙） | T3 |
| [sync-docs-and-verify-issue-58.md](../tasks/sync-docs-and-verify-issue-58.md) | front matter 同期・AGENTS 反映確認・総合検証・audit findings 0・IR 記録 | T4 |

## Verification

| ID | 対象 | 検証コマンド / 確認 |
| --- | --- | --- |
| V-1 | T1（`FrameRate`） | `mise exec -- cargo test -p ruviform-media-core`（`FrameRate` 単体テスト・`nominal_fps` 境界を含む）・`mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` |
| V-2 | T2（`Timecode` 移行） | `mise exec -- cargo test -p ruviform-media-core`・doctest 通過（`cargo test --doc`）・`grep -rn "from_media_time" crates/` で旧 `TimeBase` シグネチャ呼び出しが残存しないこと・`sub_fps`・drop 集合の回帰テスト合格 |
| V-3 | T3（精度・プロパティテスト） | `mise exec -- cargo test -p ruviform-media-core` で精度行列・往復上界・N 回連鎖上界・対照実験が合格 |
| V-4 | T4（ワークスペース全体） | `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt`・`mise run bench`・`mise exec -- prek run --all-files` |
| V-5 | ドキュメント整合 | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` findings 0・`route_graph.js` で本チェーンの implementation ゲート到達 |
| V-6 | 受入条件トレース | Issue #58 の全 AC（Verification Notes 11 項目・`nominal_fps` u64 演算と境界・精度行列・連鎖プロパティ・`nominal_fps()==0` 回帰・drop 集合維持・作業契約遵守）が T1–T4 と V-1〜V-4 で充足されること |

振る舞いレベルの保証（`FrameRate` 不変条件・`Timecode` 契約・丸め誤差上界）は [test spec](../test-specs/time-type-extension-contracts.md) に委譲し、ここでは step ごとの検証入口のみを管理する。

## Rollback Strategy

| 対象 | 戻し方 | 影響 | 備考 |
| --- | --- | --- | --- |
| 本変更全体 | `git revert` で対象 PR の commit を revert | `FrameRate`・`Timecode` 新シグネチャ・テスト群が一括で巻き戻る。`frame_rate: TimeBase` の旧 API に戻る | 単一 PR 集約 |
| 部分的戻し | T3（テスト追加）は他と独立に revert 可能。T2（シグネチャ移行）を revert するには T1 も必要（`FrameRate` 型依存） | T1 のみ残すと未使用型が公開 API に残る | 部分 revert は T3 のみが無害 |

## Open Questions / Gaps

- `FrameRate` の配置を `time.rs` とした判断（設計文書はクレート配置のみ規定）。`timecode.rs`・`framerate.rs` 新設も選択肢だったが、`SampleRate` 先例（rate 系 newtype は time モジュール）に倣う
- `nominal_fps()` の `u32` 返却は `u64` 計算結果の縮小を伴う — `4294967295/1` の `nominal_fps` は `u32::MAX` となる（`num` が `u32::MAX` のとき `(2*num + den)/(2*den)` = `4294967295`。`u64` 結果が `u32` に収まることは `num ≤ u32::MAX` と `den ≥ 1` から保証される）
- 音声 base × 映像 rate 行列の「負値拒否」は `Timecode` の負 `MediaTime` 拒否（T2 回帰）と `convert` の負値丸め対称性（T3）の両側を指す

## Self-Review

- [x] Issue #58 の受け入れ条件（Verification Notes 11 項目・精度行列・連鎖プロパティ・回帰・drop 集合・作業契約）が T1–T4・V-1〜V-6 に対応している
- [x] ADR Option A・破壊的置換・副次決定（struct 形状・`frame_duration()` 一本化）と設計文書の提案シグネチャに忠実
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は Scope 範囲外と Open Questions に記録）
- [x] ファイルパス・relation・status・用語が spec/ADR/design/タスクと整合している

## Implementation Handoff

- Recommended mode: inline 実行（本セッションが本体として逐次進行。T1→T2→T3→T4 の直列順序。T2/T3 は T1 後なら並行可能だが同一ファイル群のため直列が安全）
- Independent work streams: なし（T2・T3 はともに T1 依存。T3 の `time.rs` テスト追加は T2 とファイルを分けられるが task 依存として T1 のみに限定）
- User approval required before dispatch: no（セッション本体が実装）
- If declined or unavailable: 同上

## Review Handoff

- 前提（`relations`）: 上流は #42 チェーンで全件確定済み（spec approved・ADR accepted・design approved）。本 plan は設計の再評価を行わず実装に専念する
- 明示的な仮定: `Timecode` の呼び出し側は media-core 内テスト・doctest のみ（`publish = false`・外部利用者不在の ADR 前提と一致）
- 未解決の依存: なし（#58 は #27 のブロッカーであり、自身のブロッカーはない）
- 上流参照: spec `SPEC-034JceNZQvAawWwMH4ct91`、design `DESIGN-034JcePtzKFix4IuxyE5mD`、ADR `ADR-034JceOKDwTeWus9xBVXND`、Issue #58 / #42 / #27
