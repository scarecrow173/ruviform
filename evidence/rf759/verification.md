# rf759 検証 evidence — Issue #759 plugin-clap `f64 as u32` cast lint 修復

- 対象: `bugfix/759-clippy-cast`（`origin/main` `03af1df4` 起点）
- 実施環境: Windows（worktree `rf759-clippy-cast`）、mise 管理
  toolchain（`mise.toml` 正本）
- 実施日: 2026-11-18

## 実行した検証（すべて exit 0）

| コマンド | 結果 | ログ |
| --- | --- | --- |
| `mise run fmt-check` | exit 0 | `fmt-check.txt` |
| `mise run check ruviform-plugin-clap` | exit 0 | `check-plugin-clap.txt` |
| `mise run lint ruviform-plugin-clap` | exit 0 | `lint-plugin-clap.txt` |
| `mise run test ruviform-plugin-clap` | exit 0 | `test-plugin-clap.txt` |
| `mise run lint ruviform-plugin-test-clap` | exit 0（Issue の破損 lane — 受入条件） | `lint-plugin-test-clap.txt` |
| `mise run check ruviform-plugin-test-clap` | exit 0 | `check-plugin-test-clap.txt` |
| `mise run test ruviform-plugin-test-clap` | exit 0（非回帰） | `test-plugin-test-clap.txt` |
| `mise run test-l1-real-plugin` | exit 0（host feature `--lib --bins` + `#[ignore]` E2E 全段） | `test-l1-real-plugin.txt` |

## 補助証拠（canonical = mise）

- `cargo test -p ruviform-plugin-clap --features host gui_size_component`
  （`mise exec` 経由）: `host::tests::gui_size_component_saturates_at_pixel_bounds`
  pass — `test-l1-real-plugin` の `--lib` 段でも同 test が
  `ok`（`test-l1-real-plugin.txt` L61）。

## 修正前の再現（Issue 記載との一致確認）

- 修正前 `mise run lint ruviform-plugin-test-clap` は
  `host.rs` の `width: size.width as u32` / `height: size.height as u32`
  に `cast_possible_truncation` + `cast_sign_loss` を各 1 件ずつ報告
  （2 cast × 2 lint = 4 events・exit 101）— Issue の「4 error」
  と一致。
- 修正中に `u32::MAX as f64` が `clippy::cast_lossless` に抵触する
  ことを実測し、`f64::from(u32::MAX)` へ修正（`unit_f64` の
  既存スタイルと一致）。

## 境界契約（Issue 受入条件「境界挙動の明記」）

`gui_size_component`（`host.rs`）の doc comment と unit test
`gui_size_component_saturates_at_pixel_bounds` に明記:

- 小数部: 最近接ピクセルへ round（`f64::round`、0.5 は 0 から遠い側）
- 負値・NaN・非有限 → `0`
- `u32::MAX` 超過 → `u32::MAX`
- 契約の選定根拠: `docs/discovery/issue-759-clippy-cast.md`
  （VST3 adapter の `ViewRect` 飽和写像との整合、拒否案の棄却理由）
