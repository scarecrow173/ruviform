# 内部 Design — Issue #625 main tip deterministic compile error 2 件の修正

- 文書種別: 内部 design（`ci-compile-fix-spec.md` の設計相当）
- 日付: 2026-11-16
- 対象 Revision: `cbcc83437616fdd7418da37183c6be66b6cfe225`

## Context

Full CI run `43783872777`（headSha = main tip `cbcc8343`）の失敗
job 一次ログ（`gh run view --job --log` で確認）:

- `rt-verify`・`verify-macos`: `error[E0433]: cannot find type
  MidiGroup/MidiChannel/NoteKey/MidiValue/MidiKind in this scope`
  ×9 — `crates/ruviform-midi-device` test target `output_ports`
  の compile 失敗（"could not compile `ruviform-midi-device`
  (test "output_ports") due to 9 previous errors"）。
- `verify (ubuntu-latest)`・`l1-real-verify (ubuntu-latest)`:
  `error: casting to the same type is unnecessary (u32 -> u32)`
  ×5 — `crates/ruviform-plugin-test-vst3/src/events.rs`
  293/294/303/316/351 行目（clippy `-D warnings` で lint-full
  失敗、"could not compile `ruviform-plugin-test-vst3` (lib)
  due to 5 previous errors"）。
- `ci-summary` は必須 job 失敗に伴う派生失敗。

両エラーとも非 Windows でのみ発火し、Windows ローカルでは
再現不能。混入 commit は `7e7270f0`（#629 output slice —
output_ports.rs 追加）と `de3ed4e8`（#630 test_instrument
slice — events.rs 追加）。

## Root cause

### (a) output_ports.rs E0433

`tests/output_ports.rs` の import 構造:

- `ruviform_audio_core::midi_output::{MidiOutput,
  MidiOutputEnumerator}` — 非 gated（port trait は全 platform
  で定義。`system.rs` の `SystemBackend` が非 Windows で
  `Err(DeviceError::Unsupported)` を返す honest-exit 設計）。
- `ruviform_audio_core::midi_output::{MidiOutputPort,
  MidiPortOrigin}` — `#[cfg(windows)]` gated。
  `output_enumerator_produces_ports_with_honest_declarations`
  （`#[cfg(windows)]` test）内でのみ使用。
- `ruviform_media_core::{MidiChannel, MidiGroup, MidiKind,
  MidiValue, NoteKey}` — `#[cfg(windows)]` gated だったが、
  非 gated の `real_device_send_smoke`（`#[test] #[ignore]`、
  全 platform で compile 対象）が 110-128 行目でこれらを使用
  → 非 Windows で E0433。

`real_device_send_smoke` の設計意図は「platform backend が無い
環境では `DeviceError::Unsupported` で honest exit する
cross-platform opt-in smoke」（docstring 明記）。`backend()`・
`SystemBackend`・`MidiOutputEnumerator`/`MidiOutput` trait・
`ruviform_media_core` の MIDI 型は全て platform 非依存
（`ruviform-media-core` は target 非依存の通常依存、
`ruviform-audio-core` の port trait も cfg なし）なので、
**import の `#[cfg(windows)]` を外すだけで非 Windows でも
compile でき、honest-exit 経路がそのまま機能する**。

### (b) events.rs unnecessary_cast ×5

vst3-0.3.0 bindings: `ProcessContext_::StatesAndFlags =
DefaultEnumType` で、`DefaultEnumType` は
`#[cfg(target_os = "windows")] c_int` / `#[cfg(not(windows))]
c_uint`（`support.rs:209-212`、`bindings.rs:14265`）。
`ProcessContext.state` は `uint32`（u32、bindings.rs:14241）。

`musical_from_context`/`transport_from_context` 内の
`state & F::kX as u32` は、Windows（i32→u32）では必須の
widening cast だが、off-Windows（u32→u32）では no-op cast と
なり `clippy::unnecessary_cast` が `-D warnings` 下で error。

## 採用した修正

### (a) output_ports.rs — import の `#[cfg(windows)]` を除去

`use ruviform_media_core::{MidiChannel, MidiGroup, MidiKind,
MidiValue, NoteKey};` の `#[cfg(windows)]` を外す 1 行削除。

**代替案と却下理由:**

- `real_device_send_smoke` を `#[cfg(windows)]` に寄せる案 —
  非 Windows での honest-exit（`Unsupported` で早期 return）
  カバレッジを失う。テスト docstring が明示する cross-platform
  opt-in 設計と矛盾し、`unsupported_platforms_return_
  explicit_unsupported` が既に Unsupported 契約を別途
  カバーしているため重複保護にもならない。不採用。
- test 本体を platform 別に分割する案 — KISS/YAGNI に反し、
  型 import の cfg 不整合という局所問題に対して過剰。

### (b) events.rs — `flag_to_u32` helper 追加 + 5 箇所の cast 置換

同 file の既存 idiom `enum_to_u16`/`enum_to_u8`
（`impl Into<i64>` + `try_from`、events.rs:76-83）と同じ発想で
`StatesAndFlags` → `u32` 変換 helper を追加:

```rust
fn flag_to_u32(value: impl Into<i64>) -> u32 {
    u32::try_from(value.into()).unwrap_or(u32::MAX)
}
```

- `i64` は `i32`（Windows `c_int`）と `u32`（off-Windows
  `c_uint`）の両方から lossless に widen できるため、
  platform の signedness 差を `as` cast なしで吸収する。
- crate ルートの `sdk_enum_i32`（lib.rs:110、同じ
  `impl Into<i64>` idiom で `DefaultEnumType` → `i32`）と
  同一方針。`component.rs` の file-level
  `#![allow(clippy::unnecessary_cast)]` は本 slice の要件
  （`#[allow]` で lint を隠さない）に反するため不採用。
  `#[expect]` は Windows で unfulfilled lint になるため
  使用不可（要件どおり）。
- `unwrap_or(u32::MAX)` の fallback は shipped flag 定数
  （2/512/1024/2048/8192）では到達不能で、`enum_to_*` の
  clamp-to-max fallback と同じ defined 挙動。

5 箇所（`F::kProjectTimeMusicValid`/`kTempoValid`/
`kTimeSigValid`/`kBarPositionValid`/`kPlaying`）を
`flag_to_u32(F::kX)` に置換。`ctx.state`（u32）との `&`
mask は u32 同士となり全 platform で compile・lint が通る。

## 検証設計

- canonical: `mise run fmt-check` + `mise run check|lint|test
  <package>`（Windows ホスト、変更 2 クレート対象）。
- 補助証拠（canonical=mise）: `rustup target` 済みの
  `x86_64-unknown-linux-gnu` / `aarch64-apple-darwin` に対する
  `cargo check --all-targets` / `cargo clippy --lib --
  -D warnings` — 失敗が発火した cfg 環境（`cfg(windows)` 偽・
  `DefaultEnumType = c_uint`）を target 指定で再現し、
  compile/lint の実測 Pass を取得する。cargo 直接実行のため
  補助証拠として明記（build script の C toolchain 制約に
  掛からない dep tree であることを実測で確認済み）。
- NotRun: Linux/macOS での test 実行・rt-verify lane 固有の
  RT 実行・merge 後の `ci` workflow_dispatch（Coordinator
  管轄）。
