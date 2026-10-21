---
id: "TASK-034JNwTZ8WalgowFahLc8E"
type: "task"
status: "done"
title: "Timecode API の FrameRate 移行（破壊的シグネチャ置換と呼び出し側移行）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/58"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-58-time-type-extension.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-58-time-type-extension.md"
    - "implement-framerate-type.md"
  blocks:
    - "sync-docs-and-verify-issue-58.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-58-time-type-extension.md"
    - "../designs/time-model-extension.md"
    - "../adr/framerate-timebase-separation-and-timecode-extension.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/time-type-extension-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# Timecode API の FrameRate 移行（破壊的シグネチャ置換と呼び出し側移行）

## Work

plan `PLAN-034JNstCXHhzzwOvo9iiCv` の Phase T2。`crates/ruviform-media-core/src/timecode.rs` の `from_media_time`/`from_media_time_drop` の `frame_rate` 引数を `TimeBase` から `FrameRate` へ破壊的置換し（ADR 副次決定 1 — 旧シグネチャ併設なし）、呼び出し側（クレート内テスト・doctest・`lib.rs` RT テスト）を移行する。

- [ ] `pub fn from_media_time(t: MediaTime, frame_rate: FrameRate) -> Result<Self>` / `pub fn from_media_time_drop(t: MediaTime, frame_rate: FrameRate) -> Result<Self>` にシグネチャ置換。旧 `frame_rate: TimeBase` 版は残さない
- [ ] `elapsed_frames(t: MediaTime, rate: FrameRate)` ヘルパの引数型を変更し、内部で `rate.numerator()`/`rate.denominator()` を使う（`floor(t * rate)` の厳密 `i128`/`u128` 演算・負時刻 `Err(InvalidValue("negative time"))` 拒否は現行維持）
- [ ] `from_media_time` の nominal grid 計算 `(2*num + den) / (2*den)` を `frame_rate.nominal_fps()` 呼び出しに移譲（規則の二重実装を避ける）。`nominal_fps() == 0` → `Err(Error::InvalidValue("frame_rate"))` の拒否は維持
- [ ] `from_media_time_drop` の drop 判定 `(30_000, 1_001)`/`(60_000, 1_001)` マッチを `frame_rate.numerator()`/`denominator()` に書き換え（`FrameRate` は `TimeBase::new` 経由で正規化済みのため非正規化スペル `60_000/2_002` も従来どおり受理される）。`Err(UnsupportedDropFrameRate)` 契約維持
- [ ] 呼び出し側移行: `timecode.rs` の `mod tests`（`base()` ヘルパ → `FrameRate` 構築ヘルパへ）・両関数の doctest・`lib.rs` の `rt_drop_frame_roundtrip` を `FrameRate::new`/`from_fps` で書き換え。`grep -rn "from_media_time\|Timecode" crates/` で media-core 外の呼び出しがないことを確認
- [ ] `use crate::time::{FrameRate, MediaTime}` 等の import 更新。`# Errors` doc と `**RT contract:**` ラベルを新シグネチャに整合（`frame_rate` が `TimeBase` ではなく `FrameRate` を取る旨・エラー条件の記述更新）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-58-time-type-extension.md](../plans/implement-issue-58-time-type-extension.md) — Phase T2 の正本
- [../designs/time-model-extension.md](../designs/time-model-extension.md) — 新シグネチャ・`nominal_fps()` 移譲方針の正本
- [../adr/framerate-timebase-separation-and-timecode-extension.md](../adr/framerate-timebase-separation-and-timecode-extension.md) — 破壊的置換の決定（副次決定 1）

## Dependencies

- 前提: `implement-framerate-type.md`（`FrameRate` 型・`nominal_fps()` が前提）
- 後続をブロック: `sync-docs-and-verify-issue-58.md`
- 外部クレート依存・`unsafe` なし

## Affected Files

- `crates/ruviform-media-core/src/timecode.rs` — シグネチャ置換・内部計算・テスト・doctest
- `crates/ruviform-media-core/src/lib.rs` — `rt_drop_frame_roundtrip` テストの `FrameRate` 移行

## Done When

- [ ] `from_media_time`/`from_media_time_drop` が `frame_rate: FrameRate` を受け取り、旧 `TimeBase` シグネチャが残存しない（grep で確認）
- [ ] `nominal_fps() == 0` 拒否の回帰テスト（旧 `sub_fps_rate_is_rejected` 相当、`FrameRate::new(1, 1_000)` → `Err(InvalidValue("frame_rate"))`）が存在する
- [ ] drop 対応集合が `30000/1001`・`60000/1001` のみで他は `Err(UnsupportedDropFrameRate)` の契約を維持するテストが存在する（非正規化 `60_000/2_002` の受理を含む）
- [ ] 全 doctest・単体テスト・`lib.rs` RT テストが `FrameRate` で書き換わり合格する

## Verification

- [ ] `mise exec -- cargo test -p ruviform-media-core` — 全テスト・doctest 合格
- [ ] `mise exec -- cargo clippy -p ruviform-media-core --all-targets -- -D warnings` — 0 warnings
- [ ] `mise run fmt-check` — 差分なし
