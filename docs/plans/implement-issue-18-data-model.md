---
id: "PLAN-034J5K603qXDNmT82wI9v7"
type: "plan"
status: "completed"
title: "Issue #18 データモデル API の実装（media-core / audio-core / video-core）"
created: "2026-10-20"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/18"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-core/src/error.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/time.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/timecode.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/transport.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/format.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/layout.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/sample.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/time.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/color.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/frame.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/pixel.rs"
      - type: "file"
        path: "crates/ruviform-video-core/src/plane.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-video-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-core/Cargo.toml"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/implement-media-data-model-api.md"
  implemented-by:
    - "../tasks/implement-media-core-data-model.md"
    - "../tasks/implement-audio-core-data-model.md"
    - "../tasks/implement-video-core-data-model.md"
    - "../tasks/sync-docs-and-verify-issue-18.md"
  depends-on:
    - "apply-issue-1-architecture-baseline-to-repo.md"
  blocks:
    - "implement-issue-26-video-foundation.md"
    - "implement-plugin-foundation.md"
    - "issue-18-followup-enum-and-time-fix.md"
    - "issue-20-i-o-poc.md"
    - "../tasks/implement-media-core-data-model.md"
    - "../tasks/implement-audio-core-data-model.md"
    - "../tasks/implement-video-core-data-model.md"
    - "../tasks/sync-docs-and-verify-issue-18.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../../mise.toml"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/implement-media-data-model-api.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/media-data-model-api.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #18 データモデル API の実装（media-core / audio-core / video-core） Implementation Plan

> **記録注記**: 本 plan は Issue #18 の実装（PR #39・merge commit `d041ba9`）完了後の遡及記録である。実施当時の実作業は `tmp/issue-18/api-brief.md` を正本バインド要件としてサブエージェント分割で行われた。status `completed`・各セクションは実績を記述する。

## Goal

設計ベースライン「データモデル」節の決定事項を、`ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` の公開 API として実装・検証し、Issue #18 を完了する（spec `SPEC-034J5FYaYfeL5zkzb4sIV4` の受け入れ基準 10 件を充足）。

## Scope

単一の実装ストリームとして扱う。3 クレートの API 確定は「データモデル実装 Issue」という同一クリティカルパス上にあり、media-core → audio-core / video-core の path 依存が変更の順序制約を与える。範囲外（spec Out of Scope と一致）: `Processor` / `RtContext` / `MasterClock`、`Packet` / demux / decode、音楽時間、f64 中核表現、色変換実装、フレームプール、デバイス境界 trait、外部依存の採用。

## Upstream Documents

- [../specs/implement-media-data-model-api.md](../specs/implement-media-data-model-api.md) — `SPEC-034J5FYaYfeL5zkzb4sIV4`（implemented）。本 plan が実装する spec。Issue #18 範囲・受け入れ基準の正本
- [GitHub Issue #18](https://github.com/scarecrow173/ruviform/issues/18) — 一次ソース（`relations.source`）

## Design Inputs

- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。「データモデル」節（L218 以降: 時間モデル・音声/動画データモデル・採否）と Phase 1 完了目安（L504）・依存方向ルールの根拠
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT ADR。「保証範囲は API 単位で定義・明示」→ 全公開 API の `**RT contract:**` ラベル付け規則の根拠
- 実装バインド要件: `tmp/issue-18/api-brief.md` — ベースラインと RT ADR を 3 クレートの型リスト・署名・グローバル制約へ蒸留した実装正本（git 未追跡の作業記録）

## File Map

- `crates/ruviform-media-core/src/time.rs` — `TimeBase` / `MediaTime` / `MediaDuration` / `MediaRange` / `Rounding`（有理数時間モデルの中核）
- `crates/ruviform-media-core/src/error.rs` — 共通 `Error` / `Result`
- `crates/ruviform-media-core/src/stream.rs` — `StreamId` / `StreamKind` / `StreamDescriptor`
- `crates/ruviform-media-core/src/timecode.rs` — `Timecode` 表示変換（SMPTE drop-frame 含む）
- `crates/ruviform-media-core/src/transport.rs` — `TransportStatus` / `TransportSnapshot`（opaque `reserved: [u64; 2]`）
- `crates/ruviform-media-core/src/lib.rs` — crate doc 更新 + 再エクスポート + `rt_` テスト
- `crates/ruviform-audio-core/src/sample.rs` — sealed `Sample`（f32 / f64）
- `crates/ruviform-audio-core/src/time.rs` — `SampleRate` / `SampleIndex`
- `crates/ruviform-audio-core/src/layout.rs` — `SpeakerMask` / `ChannelLayout`
- `crates/ruviform-audio-core/src/format.rs` — `SampleFormat` / `AudioFormat`
- `crates/ruviform-audio-core/src/buffer.rs` — `BufferSpec` / `AudioBuffer` / `AudioBufferView` / `AudioBufferViewMut`
- `crates/ruviform-audio-core/src/lib.rs` — crate doc 更新 + 再エクスポート + `rt_` テスト
- `crates/ruviform-audio-core/Cargo.toml` — `ruviform-media-core` path 依存追加
- `crates/ruviform-video-core/src/pixel.rs` — `PixelFormat`
- `crates/ruviform-video-core/src/color.rs` — 色・HDR メタデータ型群
- `crates/ruviform-video-core/src/plane.rs` — `Plane` / `PlaneMut` / `SharedPlane`
- `crates/ruviform-video-core/src/frame.rs` — `VideoFormat` / `VideoFrame`
- `crates/ruviform-video-core/src/lib.rs` — crate doc 更新 + 再エクスポート + `rt_` テスト
- `crates/ruviform-video-core/Cargo.toml` — `ruviform-media-core` path 依存追加
- `Cargo.lock` — path 依存の記録更新
- `Cargo.toml` / `README.md` / `AGENTS.md` — 「path 依存未設定」「公開 API 未確定」記述の同期更新

## Implementation Sequence

### Phase A: media-core データモデル

- [x] A-1. `error` / `time` / `stream` / `timecode` / `transport` モジュールと公開型を brief の署名どおり実装（整数演算のみ・`i128`/`u128` 中間表現・overflow は `Err`/`Option`）
- [x] A-2. `MediaTime` / `MediaDuration` の cross-base `Eq`/`Ord`/`Hash`（`u128` マグニチュード比較・最低項正規化ハッシュ）と全公開 fn の `**RT contract:**` ラベルを付与
- [x] A-3. `lib.rs` の crate doc を実責務へ更新し `pub use` フラット再エクスポート。`rt_` テスト 4 本を追加（cross-base ordering・exact/rounding 変換・checked 演算・drop-frame 往復）

### Phase B: audio-core データモデル（media-core 依存）

- [x] B-1. `Cargo.toml` に `ruviform-media-core` path 依存を追加
- [x] B-2. `sample` / `time` / `layout` / `format` / `buffer` を実装（sealed `Sample`・`SampleRate`/`SampleIndex`・`SpeakerMask`/`ChannelLayout`・planar `AudioBuffer` + 借用ビュー + 境界 interleaved 変換）
- [x] B-3. `lib.rs` crate doc 更新・再エクスポート・`rt_` テスト 3 本（view/channel アクセス・interleaved 往復・sample-index↔MediaTime）

### Phase C: video-core データモデル（media-core 依存）

- [x] C-1. `Cargo.toml` に `ruviform-media-core` path 依存を追加
- [x] C-2. `pixel` / `color` / `plane` / `frame` を実装（`#[non_exhaustive] PixelFormat`・色/HDR メタデータ・stride 独立 `Plane`/`PlaneMut`/`SharedPlane`・`Arc` backing `VideoFrame`）
- [x] C-3. `lib.rs` crate doc 更新・再エクスポート・`rt_` テスト 3 本（plane アクセス/行スライス・plane_mut 書込・タイムスタンプ）

### Phase D: ドキュメント同期・総合検証・レビュー

- [x] D-1. `Cargo.toml`（メンバーコメント）・`README.md`・`AGENTS.md` の記述を実装済み状態へ同期
- [x] D-2. 検証スイート（`fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench` / `cargo metadata` publish / `prek`）を実行
- [x] D-3. 独立レビュー（A・B）→ Validator → Judge → 修正 wave（A1 カバレッジ・A2 `rt_` 衝突・A3/C3 RT ラベル・C1/C2 `isize::MAX` 境界・C4 plane 不変条件・A4 `MediaDuration::to_seconds_f64` 追加）を実施し最終 ACCEPT を得る

## Dependencies and Risks

### Dependency Graph

| 依存元 | 依存先 | 内容 |
| --- | --- | --- |
| Phase B / C（実装タスク 2・3） | Phase A（タスク 1） | `ruviform-media-core` の公開型（`MediaTime` 等）を使用するため path 依存が先に必要 |
| Phase D（タスク 4） | Phase A・B・C（タスク 1〜3） | ドキュメント同期と総合検証は全クレート実装後に実施 |
| 本 plan | `apply-issue-1-architecture-baseline-to-repo.md` | Phase 1 基盤スケルトン・`publish = false` 方針等の先行適用済みを前提 |

- `ruviform-audio-core` と `ruviform-video-core` の相互依存は設けない（層内順序ルール）
- `ruviform-media-core` は依存ゼロを維持

### Risk Register

| リスク | 影響 | 確率 | 対応 | 結果 |
| --- | --- | --- | --- | --- |
| データ損失 | なし（新規コード追加のみ・既存 API の破壊なし） | なし | — | 発生せず |
| 破壊的変更 | 公開 API 未確定のスケルトンへの置換 | 低 | `publish = false` で外部公開を遮断済みのため影響範囲は workspace 内に限定 | 緩和済み |
| 外部依存の利用可否 | なし（依存ゼロ・std のみ方針） | なし | 外部クレート禁止を brief で明文化 | 発生せず |
| RT 契約ラベル漏れ | レビューで検出され得る契約違反 | 中 | 全公開 fn の `**RT contract:**` を完了条件化＋独立レビュー | A3/C3 として検出・修正済み |
| `rt_` フィルタ誤選択 | 非 RT テストが RT レーンに混入 | 中 | テスト命名時に `rt_` 部分文字列を回避 | A2 として検出・改名済み（`convert_exact` doctest は substring 制約の既知許容） |

## Task Breakdown

| Task | 内容 | 対応 Phase |
| --- | --- | --- |
| [implement-media-core-data-model.md](../tasks/implement-media-core-data-model.md) | media-core の時間・ストリーム・タイムコード・トランスポート・エラー型実装 | A |
| [implement-audio-core-data-model.md](../tasks/implement-audio-core-data-model.md) | audio-core のサンプル・時間・レイアウト・フォーマット・planar バッファ実装（path 依存含む） | B |
| [implement-video-core-data-model.md](../tasks/implement-video-core-data-model.md) | video-core のピクセル・色・プレーン・フレーム実装（path 依存含む） | C |
| [sync-docs-and-verify-issue-18.md](../tasks/sync-docs-and-verify-issue-18.md) | ドキュメント同期・検証スイート・独立レビューサイクル | D |

## Verification

| ID | 対象 | 検証コマンド / 確認 | 実績 |
| --- | --- | --- | --- |
| V-1 | タスク 1（media-core） | `mise exec -- cargo test -p ruviform-media-core`・`mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` | 23 単体 + 22 doctest 合格・clippy 0 warnings |
| V-2 | タスク 2（audio-core） | `mise exec -- cargo test -p ruviform-audio-core`・`mise exec -- cargo clippy -p ruviform-audio-core --all-targets -- -D warnings` | 29 単体 + 18 doctest 合格・clippy 0 warnings |
| V-3 | タスク 3（video-core） | `mise exec -- cargo test -p ruviform-video-core`・`mise exec -- cargo clippy -p ruviform-video-core --all-targets -- -D warnings` | 26 テスト合格（修正 wave 後）・clippy 0 warnings |
| V-4 | タスク 4（ワークスペース全体） | `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt`・`mise run bench`・`mise exec -- cargo metadata --format-version 1 --no-deps`（全 6 パッケージ `publish` が `[]`）・`mise exec -- prek run --all-files` | 全 PASS（80 単体 + 50 doctest、rt_ レーン意図した 10 テストのみ、依存辺は audio/video → media のみ） |

振る舞いレベルの保証（有理数変換の正確性・planar 正準・stride 契約・共有 backing 制約）は [test spec](../test-specs/media-data-model-api.md) に委譲し、ここでは step ごとの検証入口のみを管理する。

## Rollback Strategy

| 対象 | 戻し方 | 影響 | 備考 |
| --- | --- | --- | --- |
| 本変更全体 | `git revert -m 1 d041ba9`（merge commit）または `git revert bc81838`（単一 squash 相当の feature commit） | 3 クレートがスケルトン状態へ復帰、path 依存・ドキュメント記述も同時に巻き戻る | 変更が単一 feature commit に集約されているためクリーンに revert 可能 |
| 部分的戻し | 対象クレートのファイル単位 revert | 依存クレート（audio/video）が残ると build が壊れるため、依存配線込みで戻す | 推奨は全体 revert |

## Open Questions / Gaps

- `PixelFormat::Yuv444p.chroma_subsampling()` が `Some((1,1))` を返す解釈（brief は 420=`(2,2)`・packed=`None` のみ確定）。実装は統一グリッド解釈として文書化済み。意図が `None` であれば `plane_dimensions` に特殊ケースが必要
- `Bytes_per_sample` の範囲外 plane に対し `0` を返すトータル化は文書化済みの判断（panic/`Option` ではない）
- `mise run test-rt` の substring フィルタ精度: `convert_exact` 等 doctest 名が `rt_` を内包する既知の誤検出候補。フィルタ方式の改善は別 Issue 検討事項
- `AudioBuffer::new` は容量超過時に panic（`-> Self` の形状契約を維持し、絶対的容量上限を `isize::MAX` バイトで文書化）

## Self-Review

- [x] spec の Requirements・受け入れ基準 10 件が Phase A–D と V-1〜V-4、4 タスクに対応している
- [x] design の決定事項（有理数時間・planar f32 正準・plane/stride・色メタ・依存方向）と RT ADR の API 単位ラベル規則が実装・検証に反映されている
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は spec の Deferred Design Concerns と Open Questions / Gaps に記録）
- [x] ファイルパス・relation・status・用語が spec/design/タスクと整合している

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（実績: brief を正本とするサブエージェント分割実施 — media/audio/video の並行実装 + docs/verify/review フェーズ）
- Independent work streams: タスク 2・3 はタスク 1 完了後に並行可能。タスク 4 は全実装完了後の直列フェーズ
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 実施済みレビューサイクル: Reviewer A（spec 適合・4 件: A1 P2, A2/A3/A4 P3）+ Reviewer B（健全性・5 件: C1/C2 P2, C3/C4/C5 P3）→ Validator（8 CONFIRMED・A4 UNVERIFIED＝仕様曖昧）→ Judge（A1・A2・A3/C3 を blocking 判定）→ 修正 wave 7 件 → **ACCEPT WITH NON-BLOCKING NOTES**（ledger `tmp/issue-18/ledger.md`）
- コーディネータ rulings（判例として記録）: `MediaDuration::to_seconds_f64` 追加で A4 の仕様曖昧を解消 / C5 の exhaustive 列挙型は `publish = false` 内部クレートの文書化済み判断として維持（公開時に再評価）/ `AudioBuffer::new` は `-> Self` を維持し絶対容量上限の panic を文書化 / timecode は media-core の表示変換責務として範囲内 / デバイス境界 trait はポート契約として別 Issue / `MediaDuration` 命名（std 衝突回避・`Media*` 系統）
- 非 blocking の残課題: C5 `#[non_exhaustive]` ポリシー（公開時のみ再検討）、test-rt substring フィルタ精度
- 上流参照: spec `SPEC-034J5FYaYfeL5zkzb4sIV4`、design `DESIGN-034IPynWe2V30FLubzQenb`、RT ADR、Issue #18・親 Issue #1
