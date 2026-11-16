# RF625 — CI compile 修正 slice 検証台帳・NotRun 表 — Issue #625

- 対象変更: `crates/ruviform-midi-device/tests/output_ports.rs`
  の `#[cfg(windows)]` 除去（E0433 ×9 解消）+
  `crates/ruviform-plugin-test-vst3/src/events.rs` の
  `flag_to_u32` helper 追加と `as u32` ×5 置換
  （`clippy::unnecessary_cast` 解消）
- 対象 Revision: `cbcc83437616fdd7418da37183c6be66b6cfe225`
  （main tip = 失敗 run `43783872777` の headSha）
- 実行環境: Windows 11 x64 ホスト（worktree `rf625-ci-fix`）、
  mise 管理 toolchain — `rustc/cargo 1.99.1
  (58cf8f9c3 2026-10-13)`。補助検証は同 toolchain の
  `--target` 指定。
- 判定語彙: `POLICIES.md` §3 — `Pass`=実測成功、
  `NotRun`=実行可能だが未実施、`Unsupported`=機構上の非対応。
  推測と実測を区別する。

## 0. CI 一次証拠（失敗の根拠）

run `43783872777` @ `cbcc8343`、`gh run view --job --log`:

| job | 観測エラー |
| --- | --- |
| `rt-verify` | `error[E0433]` ×9 — `output_ports` test で `MidiGroup`/`MidiChannel`/`NoteKey`/`MidiValue`/`MidiKind` 未解決（"could not compile `ruviform-midi-device` (test \"output_ports\") due to 9 previous errors"） |
| `verify-macos` | 同上 E0433 ×9 |
| `verify (ubuntu-latest)` | `clippy::unnecessary_cast`（u32→u32 no-op）×5 — events.rs:293/294/303/316/351（"could not compile `ruviform-plugin-test-vst3` (lib) due to 5 previous errors"） |
| `l1-real-verify (ubuntu-latest)` | 同 lint lane 失敗（同 file） |
| `ci-summary` | 必須 job 失敗の派生 |

## 1. 実行済み検証（canonical = mise）

| # | コマンド | OS | toolchain | exit code | 結果 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `mise run fmt-check` | Windows | rustfmt | 0 | **Pass** | `cargo fmt --all -- --check` |
| 2 | `mise run check ruviform-midi-device` | Windows | cargo 1.99.1 | 0 | **Pass** | `cargo check --locked --all-targets` |
| 3 | `mise run lint ruviform-midi-device` | Windows | cargo 1.99.1 + clippy `-D warnings` | 0 | **Pass** | |
| 4 | `mise run test ruviform-midi-device` | Windows | cargo 1.99.1 | 0 | **Pass** | lib 24 pass/2 ignored + output_ports 2 pass/1 ignored + ports 2 pass/1 ignored + loopback 0 pass/2 ignored + doctest 1 pass — `real_device_send_smoke` は `#[ignore]` manual opt-in で非実行（仕様どおり） |
| 5 | `mise run check ruviform-plugin-test-vst3` | Windows | cargo 1.99.1 | 0 | **Pass** | `--all-targets`（dev-dep 含む全 target compile） |
| 6 | `mise run lint ruviform-plugin-test-vst3` | Windows | cargo 1.99.1 + clippy `-D warnings` | 0 | **Pass** | |
| 7 | `mise run test ruviform-plugin-test-vst3` | Windows | cargo 1.99.1 | 0 | **Pass** | lib 84 pass/1 ignored + host_path 16 pass。`tests/l1_*_e2e.rs` は `l1-runner` feature なしで 0 test（既存仕様） |

## 2. 補助証拠（canonical=mise、cargo 直接実行 — `--target` 指定）

失敗が発火した cfg 環境（`cfg(windows)` 偽・
`DefaultEnumType = c_uint`）を target triple で再現した
compile/lint 検証。link を伴わない `cargo check`/`clippy`
のため Windows ホストから実行可能。dep tree に C toolchain
必須 crate（zstd-sys 等）が無いことを実測で確認。

| # | コマンド | target | exit code | 結果 | 対応 CI lane |
| --- | --- | --- | --- | --- | --- |
| 8 | `cargo check --target x86_64-unknown-linux-gnu -p ruviform-midi-device --all-targets` | linux-gnu | 0 | **Pass** | `rt-verify`・`verify (ubuntu-latest)` の E0433 経路 — `tests/output_ports.rs` を含む全 target が Linux cfg で compile |
| 9 | `cargo clippy --target x86_64-unknown-linux-gnu -p ruviform-midi-device --all-targets -- -D warnings` | linux-gnu | 0 | **Pass** | 同上 lint 面 |
| 10 | `cargo check --target x86_64-unknown-linux-gnu -p ruviform-plugin-test-vst3 --all-targets` | linux-gnu | 0 | **Pass** | `verify (ubuntu-latest)`・`l1-real-verify (ubuntu-latest)` の compile 面（lib + tests + bin） |
| 11 | `cargo clippy --target x86_64-unknown-linux-gnu -p ruviform-plugin-test-vst3 --lib -- -D warnings` | linux-gnu | 0 | **Pass** | **失敗の直接再現** — `c_uint` 環境で `unnecessary_cast` ×5 が解消されたことを `-D warnings` で実測 |
| 12 | `cargo check --target aarch64-apple-darwin -p ruviform-midi-device --all-targets` | apple-darwin (macos-latest = arm64) | 0 | **Pass** | `verify-macos`・`rt-verify` mac leg の E0433 経路 |
| 13 | `cargo check --target aarch64-apple-darwin -p ruviform-plugin-test-vst3 --all-targets` | apple-darwin | 0 | **Pass** | `verify-macos` の compile 面 |
| 14 | `cargo clippy --target aarch64-apple-darwin -p ruviform-plugin-test-vst3 --lib -- -D warnings` | apple-darwin | 0 | **Pass** | macOS での `c_uint` lint 面 |

## 3. NotRun / Unsupported

| lane / 検証 | 判定 | 理由 |
| --- | --- | --- |
| Linux/macOS での `cargo test` 実行 | **NotRun** | 当該 OS ホストなし。変更は compile/lint 面の cfg 修正で runtime 挙動の変更なし（`flag_to_u32` は shipped 定数で従来 cast と同値、output_ports は import のみ）。compile 面は §2 で実測済み。merge 後 CI が正本 |
| `verify (windows-latest)` 等 Windows CI lane | **NotRun(部分)** | ローカル Windows の check/lint/test が同等範囲をカバー。merge 後 CI で確認 |
| `rt-verify` の RT 実行部 | **NotRun** | RT device/実行環境なし。変更は test compile のみで RT code path 非接触 |
| `l1-real-verify` の L1 e2e（`l1-runner` feature） | **NotRun** | `mise run test` は default feature で l1 e2e を収集しない（0 test は既存仕様）。変更は events.rs の transport 変換内部のみで ABI/runner 非接触。merge 後 CI が正本 |
| `verify-macos` の macOS 実機 compile | **NotRun(補助あり)** | macOS ホストなしだが `aarch64-apple-darwin` target check/clippy が Pass（§2 #12-14）。link・test 実行は merge 後 CI |
| `*-msrv` 系タスク | **NotRun(Unsupported)** | タスク指示により daily-CI scope として実行禁止 |
| `*-full`・3OS 横断・Feature Matrix | **NotRun** | タスク指定により daily/手動 Full CI 委譲 |
| merge 後の `ci` workflow_dispatch | **NotRun(Pending)** | Coordinator が merge 後に実行。tracker auto-close の gate |

## 4. 補足・判断記録

- **output_ports.rs 採用方式**: `real_device_send_smoke` を
  `#[cfg(windows)]` に寄せる代替案は non-Windows honest-exit
  カバレッジを失うため不採用。media_core import の gate
  除去で、非 Windows では `output_ports()` が
  `Err(DeviceError::Unsupported)` を返し test が設計どおり
  early return する（`unsupported_platforms_return_
  explicit_unsupported` と併存）。
- **events.rs 採用方式**: `component.rs` 等には file-level
  `#![allow(clippy::unnecessary_cast)]` の先例があるが、
  本 slice 要件（`#[allow]` 禁止・`#[expect]` は Windows で
  unfulfilled）に従い `flag_to_u32` helper（既存
  `enum_to_u16`/`enum_to_u8`・crate `sdk_enum_i32` と同一
  `impl Into<i64>` idiom）を採用。全 platform で同一コードが
  compile・lint を通る。
- **Cargo.lock / 依存**: 変更なし（`--locked` で check/test
  通過を確認）。
- **統合受入**: ローカル検証＝変更範囲の検証。統合検証は
  merge 後の対象 SHA の必須 CI（`verify`/`rt-verify`/
  `verify-macos`/`l1-real-verify` 等）の結果で Coordinator
  が確認する（POLICIES.md §5）。
