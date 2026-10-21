---
id: "IMPL-034JxNTyasfMjdgGXnvVPY"
type: "impl"
status: "completed"
title: "Implement audio-core device port"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-core/src/device.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-audio-core-device-port.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-audio-device-port-and-cpal-poc.md"
    - "../../designs/audio-device-boundary-and-poc.md"
    - "../../specs/audio-device-backend-evaluation-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/audio-device-port-and-cpal-adapter.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement audio-core device port

## Summary

task `TASK-034Jglv8LtJHUgWH3wlu0o`（plan `PLAN-034JgkWCimxdvr81k6b0ow` Phase A）として、`ruviform-audio-core` に設計正本 `DESIGN-034JcC9QK924kDbu4WhibC`「`ruviform-audio-core` の `device` モジュール（ポート定義）」節どおりのデバイス境界ポートを実装した。`pub mod device`（crate root への flat re-export は意図的に行わない）に 3 object-safe trait（`DeviceEnumerator`・`Device: Send`・`OutputStream: Send`）、値型 7 種、コールバック型 2 種、`#[non_exhaustive]` な `DeviceError` + `DeviceResult`、`DEVICE_CLOCK_TIMEBASE`（ナノ秒 `TimeBase`）を定義し、`buffer.rs` に `AudioBufferViewMut::try_new`（借用のみ・非確保・RT-suitable）を追加した。契約テスト 5 本（object safety・`Send`・HRTB 強制変換・`DeviceError` 契約・公開 enum variant・モック backend 結合）を `lib.rs` の `#[cfg(test)]` に配置。

## Context

- 実施タスク: [../../tasks/implement-audio-core-device-port.md](../../tasks/implement-audio-core-device-port.md)
- 上流: [plan](../../plans/implement-audio-device-port-and-cpal-poc.md) Phase A、[spec](../../specs/audio-device-backend-evaluation-poc.md)（`SPEC-034Jc9QEqYLVfg1CXXTiPV`）受け入れ基準「ポート定義・非露出・`#[non_exhaustive]`」、[test-spec](../../test-specs/audio-device-port-and-cpal-adapter.md)（`TSPEC-034JgliZJcJ6cl2OeZLkuZ`）Rules
- 設計正本: [audio-device-boundary-and-poc.md](../../designs/audio-device-boundary-and-poc.md)（`DESIGN-034JcC9QK924kDbu4WhibC`・approved）— trait 分解理由・`RtContext` 非搭載理由・`DeviceError` 選択理由・`try_new` 契約は同書に従う
- 前提: `ruviform-audio-core` → `ruviform-media-core` の既存 path 依存で `MediaTime`/`TimeBase`/`Error`/`Result` を利用。外部依存追加なし・`unsafe` 不使用

## Implementation

### Changes

- `src/device.rs`（新規・587 行）— ポート定義本体:
  - trait: `DeviceEnumerator`（`output_devices`/`default_output_device`・非 RT・`Send` 非要求）、`Device: Send`（`name`/`id`/`default_output_format`/`supported_output_formats`/`open_output_stream`・非 RT）、`OutputStream: Send`（`start`/`stop` 非 RT・`format()` RT-suitable・`buffer_size()`/`now()` は制御経路 API・Drop で送出停止を doc 契約化）。全メソッドに `**RT contract:**` ラベル
  - 値型: `OutputStreamConfig`（Copy+PartialEq）、`#[non_exhaustive] BufferSize { Default(default), Fixed(usize) }`、`#[non_exhaustive] SupportedBufferSize { Unknown, Range{min,max} }`（`min <= max` 不変条件は adapter 写像側の責務として doc 化）、`SupportedOutputFormat`（Clone・非 Copy — 将来の非 Copy フィールド余地を残す設計どおり）、`DeviceId` opaque newtype（`new`/`From<String>`）、`OutputTimestamp { callback, playback: Option }`（Copy）、`OutputCallbackInfo { timestamp }`（Copy・将来フィールド追加の入れ物として doc 化）
  - `DEVICE_CLOCK_TIMEBASE: TimeBase` — `TimeBase::new(1, 1_000_000_000)` は `const fn -> Result` のため `match` で const 評価（`unreachable!` は書式マクロ展開が非 const のため literal `panic!` を使用）
  - コールバック型: `OutputDataCallback = Box<dyn for<'a> FnMut(AudioBufferViewMut<'a, f32>, OutputCallbackInfo) + Send + 'static>`、`OutputErrorCallback = Box<dyn FnMut(DeviceError) + Send + 'static>`。RT 契約（alloc/dealloc 禁止・ブロッキング/I/O/無制限待機/非有界処理の制限・全フレーム書込義務・panic 禁止・制御 API 再入禁止・drop-offload 指針）を doc に明記
  - `#[non_exhaustive] DeviceError`（Copy+Clone+Debug+PartialEq+Eq+Hash・`&'static str` payload のみ）8 variant + `DeviceResult<T>` + `Display`/`core::error::Error` 実装（`Backend` は分類名をそのまま出力）
- `src/buffer.rs` — `AudioBufferViewMut::try_new(data: &'a mut [S], channels: usize, frames: usize) -> Result<Self>` 追加（impl ブロックを `impl<'a, S>` に変更して `'a` を命名）。`channels == 0` → `InvalidValue("channels")`、`channels * frames` overflow → `InvalidValue("frames")`（checked_mul。borrow のみで確保しないため `isize::MAX` バイトガードは不適用 — 借用スライスは言語上その範囲内にある）、`data.len()` 不一致 → `InvalidValue("data")`。TDD: テスト 3 本を先に書き E0599 で red 確認 → 実装 → green
- `src/lib.rs` — `pub mod device;` 追加、crate doc 更新（`device` モジュール言及・「defined in a later issue」記述の解消・non-flat の設計判断の記録・既存の broken intra-doc links `[`TimeBase`]`/`[`MediaTime`]`/`[`Rounding`]`/`[`Error`]` を qualified path で修復）、`mod tests` 内に `device_contract` サブモジュールとして契約テスト 5 本（`StubEnumerator`/`StubDevice`/`StubStream` モック含む）

### Decisions During Implementation

- **`DEVICE_CLOCK_TIMEBASE` は `pub const`** — 設計の「const-eval 可否を実装時に確認」に対し、`TimeBase::new` が `const fn` なので `match` による const unwrap で `const` 宣言を実現（`fn`/`OnceLock` は不要と確認）
- **`DeviceError::Backend` の Display は分類名をそのまま出力**（設計どおり `f.write_str(name)`。`"backend failure: {name}"` のような装飾は付けない）
- **`try_new` の overflow ガードを `InvalidValue("frames")` として追加** — `checked_mul` により overflow shape を panic ではなく `Err` に倒す（`data.len()` が表現不能な積に一致し得ないため設計の長さ不一致規約の延長）
- **契約テスト内でモック backend を実装** — test-spec の「非 cpal 実装でもポートを実装できる」契約を audio-core 側で先行的に機械検証（adapter 側の `tests/mock_backend.rs` は Task B の範囲）
- **`SupportedOutputFormat` は Clone・非 Copy**（設計シグネチャどおり。全フィールドが Copy だが将来余地として維持）
- **crate doc の pre-existing broken intra-doc links を併せて修復**（`cargo doc` が warning ゼロになり cpal 非露出チェックの grep 対象が clean になる副次的効果）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash・mise pin の Rust 1.99.0。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `mise run fmt` / `mise run fmt-check` | 差分なし |
| 型チェック | `mise run check`（workspace `--all-targets`） | clean |
| lint | `mise exec -- cargo clippy -p ruviform-audio-core --all-targets -- -D warnings` および `mise run lint`（workspace） | clean・warnings 0（`exhaustive_enums` ゲート通過＝全公開 enum が `#[non_exhaustive]`） |
| テスト | `mise exec -- cargo test -p ruviform-audio-core` | 40 unit（新規 8: try_new 3 + device_contract 5）+ 28 doctest（新規 7）全件合格 |
| RT レーン | `mise exec -- cargo test -p ruviform-audio-core rt_` | `rt_view_mut_try_new_feeds_interleaved_boundary_write` 等が選択・合格 |
| doc | `mise exec -- cargo doc -p ruviform-audio-core --no-deps` | 成功・warnings 0・生成物への cpal/backend 型・エラー・feature 名の grep（`cpal\|cubeb\|wasapi\|asio\|pipewire\|alsa\|coreaudio\|oboe\|aaudio`）一致 0 |

TDD 記録: `try_new` テストは実装前に書き `E0599`（associated function not found）で red を確認後に実装して green。

## Risks

- `Device::name` の写像先（cpal `description()` vs `Display`）は設計どおり「要実装時確認」のまま — Task B の adapter で確定する（ポート側は `DeviceResult<String>` のみ要求）
- `OutputStream::now()` の RT 適合は backend 依存のため制御経路 API として doc 明記（設計どおり。実機確認は PoC example の実験ログ側）
- `#[non_exhaustive]` の機械的強制は `clippy::exhaustive_enums` ゲート依存（crate 内 `match` では不可視）— 契約テストは variant 群をピン留めするが、non_exhaustive そのものは lint で保証
- `BufferSize::Fixed(0)`・`usize → u32` 縮小の事前検証は adapter 責務（ポート型は値を運ぶのみ）として doc 化済み — Task B で実装されるまで未検証

## Follow-ups

- [implement-audio-device-cpal-adapter.md](../../tasks/implement-audio-device-cpal-adapter.md)（`TASK-034JglAvWUFQoLhvYXOw2O`）— ポート trait の cpal 実装 + interleave pack 経路 + エラー写像（本 IR のポート契約が前提）
- `DeviceEnumerator` の `Send` 非要求は engine セッション調停設計時に再評価余地あり（設計 Deferred Concerns どおり）
- `RtContext` 配線は `ruviform-rt` mint 実装後に `OutputCallbackInfo` フィールド追加またはシグネチャ追加で対応（設計どおり据え置き）
