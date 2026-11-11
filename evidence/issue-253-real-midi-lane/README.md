# Issue #253 — opt-in real-device MIDI lane evidence

本ディレクトリは Issue #253 residual slice
（`feature/253-unsupported-transparency-real-lane`）で整備した
opt-in 実機検証 lane の実行記録。canonical lane は follow-up slice
（`chore/253-midi-real-device-lane`）で `test-midi-real-device`
として `mise.toml` に登録済み — 暫定入口
`cargo test -- --ignored` は正規 lane に昇格した。

## 環境

| 項目 | 値 |
| --- | --- |
| OS | Windows 11 x64（Orca workspace、Git Bash） |
| toolchain | rustc 1.99.0 (b940084d7 2026-09-28) / cargo 1.99.0 |
| MIDI input デバイス | なし（`list_midi_ports` が `no MIDI input ports` を報告） |

## 実行結果（補助証拠 — canonical = `mise run test-midi-real-device`、以下は lane 登録前の暫定入口コマンド実行記録）

### `cargo test -p ruviform-midi-device -- --ignored`

exit code: 0

```text
running 1 test
test os::windows::tests::real_device_open_read_close_smoke ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; ... finished in 0.03s

running 1 test
no MIDI input ports — nothing to verify
test real_device_open_read_observe_close ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; ... finished in 0.03s
```

### `cargo test -p ruviform-audio-plugin-bridge -- --ignored`

exit code: 0

```text
running 1 test
no MIDI input ports — nothing to verify
test real_midi_device_drives_input_tx_to_stage ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; ... finished in 0.03s
```

### `cargo run -p ruviform-midi-device --example list_midi_ports`

exit code: 0、出力: `no MIDI input ports`

## canonical lane 実行記録（`test-midi-real-device`）

follow-up slice `chore/253-midi-real-device-lane` で `mise.toml` に
登録した canonical lane の実走記録。環境は上記「環境」表と同一
（Windows 11 x64・rustc 1.99.0・MIDI input ポートなし）。

### `mise run test-midi-real-device`

exit code: 0

```text
[test-midi-real-device] $ cargo test -p ruviform-midi-device
  -p ruviform-audio-plugin-bridge -- --ignored --test-threads=1

running 1 test
test real_midi_device_drives_input_tx_to_stage ... ok

running 1 test
test os::windows::tests::real_device_open_read_close_smoke ... ok

running 1 test
test real_device_open_read_observe_close ... ok
```

pass した 3 test の内訳は `--nocapture` 実走（補助証拠）で確認 —
`real_device_open_read_observe_close` /
`real_midi_device_drives_input_tx_to_stage` はともに
`no MIDI input ports — nothing to verify` を記録して honest
return（lib smoke は device 非在で即 return）。実測値の観測は
0 件 — device 不在を Pass に偽装していない。

### `mise exec -- cargo run -p ruviform-midi-device --example list_midi_ports`

exit code: 0、出力: `no MIDI input ports`（環境再確認）

## 判定

- **lane 整備**: Pass — enumerate → (ports なし → honest skip) →
  (ポートがあれば) open → repeated read/pump → 観測 → close →
  `StreamInvalidated` の全経路が `#[ignore]` E2E として実装済み。
- **実デバイス観測**: NotRun — 本環境に MIDI input ポートが存在
  しないため、open/read/pump/close の実機経路と
  `dropped`/`unsupported` の実測値は未観測。両 lane は honest に
  "no MIDI input ports — nothing to verify" を報告して終了した。
- **非対応 OS 経路**: NotRun — Windows のみで実施。macOS/Linux で
  `DeviceError::Unsupported` を返す経路はコード上実装済み
  （`backend().input_ports()` が `Err(Unsupported)` → early
  return）だが未実測。
- **canonical lane 登録**: Done — `mise.toml` に
  `test-midi-real-device` として登録済み
  （`chore/253-midi-real-device-lane` slice。当時の仮称
  `test-midi-real-input` 相当）。本環境での lane 実走は上記のとおり
  全 `#[ignore]` test が honest skip・exit 0 — 実デバイス観測自体は
  引き続き NotRun。

## 実機保有環境での再現手順

```sh
# 1. ポート列挙の確認
cargo run -p ruviform-midi-device --example list_midi_ports

# 2. canonical lane（adapter + bridge 両経路）
mise run test-midi-real-device
# eprintln の観測値を見る場合（補助入口）:
# cargo test -p ruviform-midi-device -p ruviform-audio-plugin-bridge \
#   -- --ignored --nocapture
```

ポートが存在すれば、両 E2E は port capabilities・各 read/pump の
`received`/`pushed`/`dropped`/`unsupported` 観測値・close 後の
`StreamInvalidated` を `eprintln` で記録し、stamp 契約
（domain・`Carried` provenance・non-decreasing ticks）を assert
する。トラフィックの有無は assert しない。
