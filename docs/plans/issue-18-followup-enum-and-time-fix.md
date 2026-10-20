---
id: "PLAN-034J6OTwge4rS96WZWkZgd"
type: "plan"
status: "completed"
title: "Issue #18 フォローアップの実装（公開 enum 方針適用・時間演算修正・改善トリアージ）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/41"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-18-followup-enum-and-time-fix.md"
  implemented-by:
    - "../tasks/apply-non-exhaustive-to-public-enums.md"
    - "../tasks/fix-time-arithmetic-saturation.md"
    - "../tasks/small-fixes-from-followup-triage.md"
  depends-on:
    - "implement-issue-18-data-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-18-followup-enum-and-time-fix.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/followup-enum-compat-and-time-arithmetic.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #18 フォローアップの実装（公開 enum 方針適用・時間演算修正・改善トリアージ） Implementation Plan

## Goal

spec `SPEC-034J6OCXHiP1izKRzbMmgz`（Issue #41）の受け入れ基準 AC1–AC4 を充足する: 全公開 enum への enum レベル `#[non_exhaustive]` 適用と lint 強制化、時間演算の `i128` 中間精度化と `MediaRange::checked_end` 新設、小改善トリアージ（`try_new`・`Custom` 構造的不変条件・丸め doc 注記）、延期項目の Issue 記録。

## Scope

単一の実装ストリームとして扱う。3 系統の変更はいずれも「Issue #18 で確定したデータモデル API のフォローアップ修正」という同一クリティカルパス・同一リリース意図上にあり、spec で単一 Issue として管理されている。範囲外（spec Out of Scope と一致）: 借用 planar ビューの新規抽象（#29/#23/#25）、RT テスト選択・alloc 検証（#22）、`VideoFrame` 所有権・pool（#26）、`FrameRate`/`TimeBase` 分離・`Timecode` 拡張・丸め誤差累積ポリシー（新規 Issue 起票）、`convert` の契約変更、variant レベル `#[non_exhaustive]`、クレート内部 enum への強制。

## Upstream Documents

- [../specs/issue-18-followup-enum-and-time-fix.md](../specs/issue-18-followup-enum-and-time-fix.md) — `SPEC-034J6OCXHiP1izKRzbMmgz`（approved）。本 plan が実装する spec。Issue #41 範囲・受け入れ基準の正本
- [GitHub Issue #41](https://github.com/scarecrow173/ruviform/issues/41) — 一次ソース（`relations.source`）。3 スコープと延期項目の既存 Issue 対応表を規定

## Design Inputs

- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。時間モデル（正確な有理数演算・f64 を中核表現にしない）と依存方向ルールの根拠
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT ADR。「保証範囲は API 単位で定義・明示」→ 変更対象 API の `**RT contract:**` ラベル更新規則の根拠
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — 公開 enum `#[non_exhaustive]` 統一方針 ADR（accepted）。T1 の適用規則（enum レベル・閉集合包含・兄弟クレート=外部・内部 enum 免除）の正本

## File Map

enum を含むファイル（T1 対象。11 enum・9 件新規付与 + 2 件維持）:

- `crates/ruviform-media-core/src/error.rs` — `Error`
- `crates/ruviform-media-core/src/time.rs` — `Rounding`（T1）。**T2 主対象**: `MediaRange::end()` 修正・`checked_end` 新設・`MediaTime::checked_add`/`checked_sub` 修正・テスト・doc
- `crates/ruviform-media-core/src/stream.rs` — `StreamKind`
- `crates/ruviform-media-core/src/transport.rs` — `TransportStatus`
- `crates/ruviform-audio-core/src/layout.rs` — `ChannelLayout`（T1）。**T3 対象**: `Custom` payload の非空 newtype 化・`custom()`/`channels()`/`speaker_mask()`/テスト・doctest 更新
- `crates/ruviform-audio-core/src/format.rs` — `SampleFormat`（付与済み・維持確認）
- `crates/ruviform-video-core/src/color.rs` — `ColorPrimaries` / `TransferCharacteristics` / `MatrixCoefficients` / `ColorRange`
- `crates/ruviform-video-core/src/pixel.rs` — `PixelFormat`（付与済み・維持確認）

その他の変更対象:

- `crates/ruviform-media-core/src/lib.rs`、`crates/ruviform-audio-core/src/lib.rs`、`crates/ruviform-video-core/src/lib.rs` — `#![allow(clippy::exhaustive_enums)]` 撤去 + 隣接ポリシーコメントを enum ADR を指す内容に更新（audio-core lib.rs は T3 の `Custom` payload 変更に伴う再エクスポート調整も共有ファイル）
- `crates/ruviform-audio-core/src/buffer.rs` — `AudioBuffer::try_new` 新設（`from_interleaved` と検証共有）・`new()` doc 相互参照
- `AGENTS.md` — 暫定規約に enum ポリシー 1 行追記
- `docs/adr/public-enum-non-exhaustive-policy.md` — 本変更が適用するポリシーの正本（作成済み・変更不要）

## Implementation Sequence

### T1: 公開 enum への `#[non_exhaustive]` 適用（task: apply-non-exhaustive-to-public-enums）

- [x] 9 件の enum（`Error`・`Rounding`・`StreamKind`・`TransportStatus`・`ChannelLayout`・`ColorPrimaries`・`TransferCharacteristics`・`MatrixCoefficients`・`ColorRange`）の `pub enum` 直上に `#[non_exhaustive]` を付与（`PixelFormat`・`SampleFormat` は既存を維持）
- [x] 3 `lib.rs` の `#![allow(clippy::exhaustive_enums)]` を撤去し、隣接ポリシーコメントを「exhaustive 前提」から「enum ADR の統一ポリシー適用」へ書き換え
- [x] `AGENTS.md` 暫定規約にポリシー 1 行を追記
- [x] クレート間 exhaustive `match` の非存在を再 grep で確認（現時点で存在しないことを確認済み。`match self` は同一クレート内のため合法）

### T2: 時間演算の早期飽和・Err 修正（task: fix-time-arithmetic-saturation）

- [x] 失敗する回帰テストを先に追加（`MediaRange::end()` / `checked_add` が「duration の中間変換が `i64` 超過・最終和は表現可能」で誤値/`Err` になることを確認）
- [x] delta を `i128` 正確有理数（`value * num * other_den / (den * other_num)`）として計算し、丸めは最後に 1 回・`i64` 範囲検査は最終値のみに統一。`MediaRange::end()`（トータル・真の最終オーバーフローのみ飽和）、`MediaRange::checked_end(rounding) -> Result<MediaTime>`（新設）、`MediaTime::checked_add`/`checked_sub` に適用
- [x] 負 duration・全 `Rounding` モードの境界テスト、`# Panics`/`# Errors` ドキュメント・RT ラベル更新。`convert` の契約は変更しない

### T3: 小改善トリアージ（task: small-fixes-from-followup-triage）

- [x] `AudioBuffer::try_new(spec) -> Result<Self>` — `from_interleaved` と形状検証を共有。`new()` は panic 契約を維持し doc 相互参照
- [x] `ChannelLayout::Custom` の構造的不変条件 — 非空 mask newtype（例: `SpeakerMaskSet`、`new(mask) -> Result<Self>` + `mask()` アクセサ）を `Custom` payload にし、コンストラクタ・`channels()`・`speaker_mask()`・テスト・doctest を更新。`SpeakerMask` 自体は不変
- [x] `MediaTime::convert`/`checked_add` の「呼び出しごとに 1 回の丸め」「境界で変換して誤差累積を避ける」doc 注記（doc-only）

### T4: 検証・Issue 起票・ドキュメント同期（本 plan の Verification 節で実行。task doc なし）

- [x] 検証スイート（`fmt-check` / `check` / `lint` / `test` / `test-rt` / `cargo metadata` / `prek`）を実行
- [x] 延期項目を Issue #41 の対応表どおりに記録（借用 planar ビュー → #29/#23/#25、RT テスト選択・alloc 検証 → #22、`VideoFrame` 所有権・pool → #26、`FrameRate`/`TimeBase` 分離・`Timecode` 拡張・丸め方針 → 新規 Issue 起票）
- [x] spec/plan/task のステータス・`changes` 記録を実施結果に同期

## Dependencies and Risks

### Dependency Graph

| 依存元 | 依存先 | 内容 |
| --- | --- | --- |
| T2 | T1 | 直列実行順 T1→T2。ファイル競合はない（T2 は media-core time.rs のみ）が、T1 の lib.rs 変更確定後に行う順序とする |
| T3 | T1・T2 | T3 は T1 と audio-core の `layout.rs`/`lib.rs` を共有するため T1 完了後が必須。順序 T1→T2→T3 |
| T4 | T1・T2・T3 | 総合検証と Issue 記録は全実装完了後に実施 |
| 本 plan | `implement-issue-18-data-model.md` | Issue #18 のデータモデル API が確定済みであることを前提（その API 表面を修正する変更のため） |

- `ruviform-math` / `ruviform-rt` / `ruviform-exec` への影響なし（3 データモデルクレートのみ）
- audio-core ↔ video-core の相互依存は作らない（現行維持）

### Risk Register

| リスク | 影響 | 確率 | 対応 |
| --- | --- | --- | --- |
| 将来の variant 追加による破壊 | `#[non_exhaustive]` により設計上 non-breaking。外部 `match` は `_` arm 必須 | 低（本対応で解消） | ポリシーを ADR + lint で強制 |
| `#[non_exhaustive]` 属性追加自体の破壊 | 属性は付加的（additive）。外部の exhaustive `match` だけがコンパイルエラーになる | 低 | クレート間 exhaustive `match` は現時点で存在しないことを grep 済み。兄弟クレートのコンパイル成功が実証 |
| `end()` の飛飽和修正による意味変更 | 「中間 `i64` 超過・最終表現可能」という隅のみ挙動が変わる（誤値→真値、`Err`→`Ok`）。既存の正常系入力には非破壊 | 中 | 回帰テストを先行追加し、変更前後で正しい値に変わることを固定 |
| `Custom` payload 変更による API 差分 | `ChannelLayout::Custom(SpeakerMask)` → `Custom(SpeakerMaskSet)` で直接構築する呼び出し側がコンパイルエラー | 低（`publish = false`・利用は workspace 内） | 内部利用箇所を同時修正。`custom()` 経由の構築は `Err` 契約を維持 |
| `checked_sub` の `i64::MIN` duration 否定 | `i64::MIN` を `i64` で否定するとオーバーフロー | 中 | 否定は `i128` 上で行い、最終値のみ `i64` 検査（spec Requirements どおり） |
| データ損失・外部依存 | なし（属性・整数演算・doc の変更のみ。依存追加なし） | なし | — |

## Task Breakdown

| Task | 内容 | 対応 Phase |
| --- | --- | --- |
| [apply-non-exhaustive-to-public-enums.md](../tasks/apply-non-exhaustive-to-public-enums.md) | 全公開 enum への enum レベル `#[non_exhaustive]` 適用・allow 撤去・AGENTS.md 追記 | T1 |
| [fix-time-arithmetic-saturation.md](../tasks/fix-time-arithmetic-saturation.md) | 時間演算の `i128` 中間精度化（回帰テスト先行）・`checked_end` 新設 | T2 |
| [small-fixes-from-followup-triage.md](../tasks/small-fixes-from-followup-triage.md) | `try_new`・`Custom` 構造的不変条件・丸め doc 注記 | T3 |
| T4（doc なし・本 plan の Verification 節で実行） | 検証スイート・延期項目の Issue 記録・ドキュメント同期 | T4 |

## Verification

| ID | 対象 | 検証コマンド / 確認 |
| --- | --- | --- |
| V-1 | T1（3 クレート enum） | `mise exec -- cargo test -p ruviform-media-core`・`mise exec -- cargo test -p ruviform-audio-core`・`mise exec -- cargo test -p ruviform-video-core`・`mise exec -- cargo clippy --workspace --all-targets -- -D warnings`（allow 撤去後の `exhaustive_enums` 検出を含む）・`grep -rn "#\[non_exhaustive\]" crates/*/src` で 11 enum を監査 |
| V-2 | T2（media-core 時間演算） | `mise exec -- cargo test -p ruviform-media-core`（回帰テスト・負 duration・全丸めモード境界を含む）・clippy 0 warnings・doc `# Errors`/`# Panics` 記載確認 |
| V-3 | T3（audio-core 小修正） | `mise exec -- cargo test -p ruviform-audio-core`（`try_new` 失敗経路・`Custom` 非空不変条件を含む）・clippy 0 warnings・doc 相互参照確認 |
| V-4 | T4（ワークスペース全体） | `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt`・`mise exec -- cargo metadata --format-version 1 --no-deps`（`publish`=`[]` 維持・依存辺不変）・`mise exec -- prek run --all-files` |
| V-5 | クレート間互換性の実証 | audio-core / video-core が media-core の `#[non_exhaustive]` enum に依存したまま `mise run check` / `mise run test` が通る（スクラッチ downstream クレートの新設は不要 — sibling crate のコンパイル成功が exhaustive `match` 非存在の実証） |

振る舞いレベルの保証（enum wildcard-match 互換・`i128` 中間精度・`Custom` 不変条件・`try_new` 失敗経路）は [test spec](../test-specs/followup-enum-compat-and-time-arithmetic.md) に委譲し、ここでは step ごとの検証入口のみを管理する。

## Rollback Strategy

| 対象 | 戻し方 | 影響 | 備考 |
| --- | --- | --- | --- |
| 本変更全体 | `git revert` で対象 PR の commit を revert（squash マージなら merge commit `-m 1`） | enum 属性・時間演算・小修正・AGENTS.md 記述が一括で巻き戻る | 単一 PR 集約を推奨 |
| 部分的戻し | 系統ごとのファイル単位 revert | enum 系（T1）と時間演算系（T2）は別ファイルのため独立に戻せる。T3 は T1 と audio-core `layout.rs`/`lib.rs` を共有するため T1 revert 時は T3 側のコンフリクトに注意 | T4 の Issue 起票は revert 対象外（起票済み Issue は手動で整理） |

## Open Questions / Gaps

- `checked_end` の `rounding` 引数は呼び出し側指定（`end()` は `NearestEven` 固定の現行を維持するか、`checked_end` へ丸め指定を移すかは実装時に doc 整合として確認）
- `SpeakerMaskSet` の命名・配置は例示。`layout` モジュール内の非空 mask newtypeとして実装し、`pub use` の要否は `Custom` payload の公開 API 露出に応じて実装時に確定する
- `checked_sub` の `i64::MIN` duration は `i128` 否定で扱う規則（Risk Register と spec Requirements で確定済み。実装詳細は `scaled_fraction` / `convert_value` 等の既存 helper との整合を実装時に選択）

## Self-Review

- [x] spec の Requirements・受け入れ基準 AC1–AC4 が T1–T3 と T4、V-1〜V-5 に対応している
- [x] enum ADR の規則（enum レベル・閉集合包含・兄弟=外部・内部免除）と RT ADR の API 単位ラベル規則が実装・検証に反映されている
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は spec Deferred Design Concerns と Open Questions / Gaps に記録）
- [x] ファイルパス・relation・status・用語が spec/ADR/タスクと整合している

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（T1–T3 は別クレート別ファイル中心で並行余地があるが、T1/T3 の audio-core 共有ファイルのため直列順序 T1→T2→T3 を維持するのが安全）
- Independent work streams: T2（media-core time.rs のみ）は T1 とファイル競合しないが、順序保証のため T1 完了後に実行。T4 は全実装完了後の直列フェーズ
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提（`relations.depends-on`）: [implement-issue-18-data-model.md](implement-issue-18-data-model.md)（`PLAN-034J5K603qXDNmT82wI9v7`・completed）で確定したデータモデル API が本変更の対象表面。
- 明示的な仮定: クレート間 exhaustive `match` は現時点で存在しない（`match self` は同一クレート内で合法・grep 確認済み）。`checked_end` は新規 API であり呼び出し側の既存コードは存在しない。
- 未解決の依存: T4 の新規 Issue 起票（`FrameRate`/`TimeBase` 分離・`Timecode` 拡張・丸め誤差累積ポリシー）は本 plan の実施時に起票番号を確定する。
- 上流参照: spec `SPEC-034J6OCXHiP1izKRzbMmgz`、design `DESIGN-034IPynWe2V30FLubzQenb`、RT ADR、enum ADR `ADR-034J6OnZouPI8HQy6cIg9D`、Issue #41・元参照 Issue #18 / PR #39。
