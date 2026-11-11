# Issue #253 — opt-in real-device MIDI lane evidence

本ディレクトリは Issue #253 residual slice
（`feature/253-unsupported-transparency-real-lane`）で整備した
opt-in 実機検証 lane の実行記録。canonical lane 登録（mise task）は
`mise.toml` が本 slice の編集範囲外のため follow-up として残り、
暫定入口は `cargo test -- --ignored`。

## 環境

| 項目 | 値 |
| --- | --- |
| OS | Windows 11 x64（Orca workspace、Git Bash） |
| toolchain | rustc 1.99.0 (b940084d7 2026-09-28) / cargo 1.99.0 |
| MIDI input デバイス | なし（`list_midi_ports` が `no MIDI input ports` を報告） |

## 実行結果（補助証拠 — canonical = mise、本 lane は暫定入口）

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
- **canonical lane 登録**: follow-up — `mise.toml` は別 Worker 占有
  のため本 slice 対象外。`test-midi-real-input` 相当の lane 登録が
  残作業。

## 実機保有環境での再現手順

```sh
# 1. ポート列挙の確認
cargo run -p ruviform-midi-device --example list_midi_ports

# 2. adapter 経路（enumerate → open → read → 観測 → close）
cargo test -p ruviform-midi-device -- --ignored --nocapture

# 3. bridge 経路（SystemBackend → InputTx::pump → InputStage）
cargo test -p ruviform-audio-plugin-bridge -- --ignored --nocapture
```

ポートが存在すれば、両 E2E は port capabilities・各 read/pump の
`received`/`pushed`/`dropped`/`unsupported` 観測値・close 後の
`StreamInvalidated` を `eprintln` で記録し、stamp 契約
（domain・`Carried` provenance・non-decreasing ticks）を assert
する。トラフィックの有無は assert しない。
