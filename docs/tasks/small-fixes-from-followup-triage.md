---
id: "TASK-034J6Pq9WM0XMkGqryZrvQ"
type: "task"
status: "done"
title: "フォローアップトリアージ由来の小修正（try_new・Custom 不変条件・丸め注記）"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/41"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/layout.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/time.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
  implemented-by:
    - "../impl/ir/small-fixes-from-followup-triage.md"
  depends-on:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
    - "apply-non-exhaustive-to-public-enums.md"
    - "fix-time-arithmetic-saturation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-18-followup-enum-and-time-fix.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/followup-enum-compat-and-time-arithmetic.md"
  references: []
  defers: []
  deferred-by: []
---

# フォローアップトリアージ由来の小修正（try_new・Custom 不変条件・丸め注記）

## Work

plan `PLAN-034J6OTwge4rS96WZWkZgd` の Phase T3。Issue #41 スコープ 3「小改善」の 3 項目を適用する。各項目は独立した `## Work` チェックボックス群として扱う。

### (a) `AudioBuffer::try_new`

- [x] `crates/ruviform-audio-core/src/buffer.rs` に `AudioBuffer::try_new(spec) -> Result<Self>` を追加する。形状検証（`channels * frames` の `usize` オーバーフロー・`isize::MAX` バイト上限）は `from_interleaved` と共有し、失敗は `Err(Error::InvalidValue(..))` で返して panic しない
- [x] `AudioBuffer::new()` は従来どおり異常入力で panic する `-> Self` の契約を維持し、`new` ↔ `try_new` の doc を相互参照させる（panic を避けたい呼び出しは `try_new` へ、という案内）

### (b) `ChannelLayout::Custom` の構造的不変条件

- [x] `crates/ruviform-audio-core/src/layout.rs` に非空 mask の newtype（例: `SpeakerMaskSet`。`SpeakerMask` を wrap し `new(mask) -> Result<Self>` + `mask()` アクセサ）を導入し、`ChannelLayout::Custom` の payload を `SpeakerMask` から newtype へ変更する
- [x] `ChannelLayout::custom`・`channels()`・`speaker_mask()`・関連テスト・doctest を新 payload に合わせて更新する（`ChannelLayout::custom` の `Err(Error::InvalidValue("mask"))` 契約は維持）
- [x] `SpeakerMask` 自体は変更しない（空 mask は値として引き続き合法。不変条件を課すのは `Custom` payload 側）
- [x] `lib.rs` の `pub use` 再エクスポートを必要に応じて調整する（newtype が公開 API に露出する場合は再エクスポート対象に含める）

### (c) 丸め誤差累積の doc 注記（doc-only・コード変更なし）

- [x] `crates/ruviform-media-core/src/time.rs` の `MediaTime::convert` / `checked_add`（必要に応じて `MediaDuration::convert`・モジュール doc）に、「変換・演算は呼び出しごとに 1 回の丸めを適用する（single-rounding-per-call）」こと、および「変換を連鎖させると丸め誤差が累積するため、境界で変換する」旨の案内を注記する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-18-followup-enum-and-time-fix.md](../plans/issue-18-followup-enum-and-time-fix.md) — `PLAN-034J6OTwge4rS96WZWkZgd`（approved）。Phase T3 の正本
- [../specs/issue-18-followup-enum-and-time-fix.md](../specs/issue-18-followup-enum-and-time-fix.md) — `SPEC-034J6OCXHiP1izKRzbMmgz`（approved）。AC3 の正本

## Dependencies

- 前提: plan・`apply-non-exhaustive-to-public-enums.md`・`fix-time-arithmetic-saturation.md`（順序 T1→T2→T3。T1 と audio-core `layout.rs`/`lib.rs` を共有するため T1 完了後が必須。T2 の time.rs doc 注記と (c) のモジュール doc が隣接するため T2 後に実施）
- 後続: T4（検証・Issue 起票。plan Verification 節で実施）
- 外部クレート依存・`unsafe` なし

## Affected Files

- `crates/ruviform-audio-core/src/buffer.rs` — `try_new` 追加・`new` doc 相互参照
- `crates/ruviform-audio-core/src/layout.rs` — 非空 mask newtype・`Custom` payload・`custom()`/`channels()`/`speaker_mask()`・テスト・doctest
- `crates/ruviform-audio-core/src/lib.rs` — newtype の `pub use` 調整（必要時）
- `crates/ruviform-media-core/src/time.rs` — (c) の doc 注記のみ（doc-only）

## Done When

- [x] `AudioBuffer::try_new()` が `Result` を返し、`from_interleaved` と同じ検証失敗を `Err` で表出する（panic しない）
- [x] `ChannelLayout::Custom` が空 mask で構築不能になり（構造的。doc 注記ではなく型で閉じる）、`custom()` の `Err(Error::InvalidValue("mask"))` 契約が維持される
- [x] 丸め誤差累積の doc 注記が `MediaTime::convert`/`checked_add` に存在する
- [x] `mise exec -- cargo test -p ruviform-audio-core`・`mise exec -- cargo test -p ruviform-media-core`（doctest 含む）と clippy がグリーン

## Verification

- [x] `mise exec -- cargo test -p ruviform-audio-core` — `try_new` 失敗経路・`Custom` 非空不変条件・既存テストの更新を含め合格
- [x] `mise exec -- cargo test -p ruviform-media-core` — doc 注記追加後の doctest を含め合格
- [x] `mise exec -- cargo clippy -p ruviform-audio-core --all-targets -- -D warnings`・`mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` — 0 warnings
- [x] `mise run fmt-check` — 差分なし
