# RF-VTEST-03 single-component slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（test plugin suite の single-component slice）
- Slice: VST3 `test_stateful_single` fixture class — production bridge の
  `single_component == true` arm の初実バイナリ実走
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-20
- Case Schema v1 語彙・`environment_kind=test-plugin`。deterministic evidence と real-run evidence を分離。

## 実装概要

- `crates/ruviform-plugin-test-vst3/src/single.rs`（新規）:
  `TestStatefulSingle` — 1 COM object が `IComponent` +
  `IAudioProcessor` + `IProcessContextRequirements` +
  `IEditController` を実装。component/audio surface は
  `TestComponent<TestStatefulProcessor>`（`controller_cid = Self::CID`
  で構築 → `getControllerClassId` が own-CID）へ委譲。gain の真実は
  1 `SharedGain` cell — controller value/state verb は mirror ではなく
  同一 cell に束縛。strict `terminate`（`Initialized` 以外で
  defined rejection — double-terminate 観測 pin）、
  `setComponentHandler` once-contract、`IConnectionPoint` 非実装。
- `src/component.rs`: `lifecycle_state()` probe 追加（strict
  terminate 判定用 — shared shell 契約は不変）。
- `src/stateful.rs`: `TestStream`/`ib_stream` を
  `#[cfg(test)] pub(crate) mod test_stream` へ切り上げ。
- `src/factory.rs`: `countClasses` 11→12、index 11 =
  `test_stateful_single`（`"Audio Module Class"`・controller row なし、
  fresh `uid` で非衝突）。
- `src/lib.rs` + `README.md`: 12 class / 7 descriptor の記述更新。
- `tests/host_path.rs`: descriptor 数・index pin 更新 + single
  component param/state round-trip/`ctrl_len=0` framing/malformed
  rejection tests。
- `tests/l1_stateful_test_plugin_e2e.rs`: single component の
  round-trip + malformed e2e 追加。

## 受入条件の照合

| 受入条件 | 結果 | 証拠 |
| --- | --- | --- |
| 1 COM object / own CID / class 12 (index 11, controller row なし) | Passed | fixture unit tests + factory count + `vst3-validator.txt` の `name = test_stateful_single` |
| production load→init→prepare→process→teardown (L0) + L1 save/restore wire round-trip | Passed | host_path 19 tests / L1 `real_stateful_single_*` 2 tests |
| `single_component==true` arm 実走（single init/terminate、handler 1 回、pairing なし、`ctrl_len=0`） | Passed | strict-terminate + once-contract + `ctrl_len=0` 24-byte framing pin（L0+L1）。観測境界は下記参照 |
| 既存 lane 緑のまま | Passed | package test / L1 real-plugin / `rt_` / lint / fmt / prek 全緑 |

## deterministic evidence

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | Passed |
| `mise exec -- cargo check -p ruviform-plugin-test-vst3 --all-targets` | Passed |
| `mise exec -- cargo test -p ruviform-plugin-test-vst3 --lib` | Passed（`single::` 7 tests 含む） |
| `mise run check-plugin-vst3-clap` | Passed |
| `mise run lint-plugin-vst3-clap` | Passed（`-D warnings`） |
| `mise run test-plugin-vst3` | Passed（host_path 19 tests 含む） |
| `mise exec -- cargo test --workspace rt_` | Passed |
| `mise exec -- prek run --all-files` | Passed |

## real-run evidence

| コマンド | 結果 |
| --- | --- |
| `mise run test-l1-real-plugin` | Passed — `real_stateful_single_test_plugin_l1_save_mutate_restore_save` + `..._rejects_malformed_state` 含む全 suite 緑 |
| `node ci/plugin-abi-validator/run.mjs`（explicit validator paths） | Passed — VST3 SDK validator 282 passed / 0 failed、`TestSuite : test_stateful_single` 実走 |

注: L1 lane の初回実行では `real_test_fault_l1_hang_converges_on_timeout` が
spawn timeout で flake（単独再実行で pass）。本 slice 起因ではなく
タイミング flake として記録 — 全 lane 再実行で緑を確認済み。

## 観測境界（残存 — honest 記録）

- `setComponentHandler` が shared object に「呼ばれた」ことは bridge
  経由では構造的に観測不能（`BridgeComponentHandler` は
  acknowledge-only no-op）。once-contract（2 回目 reject → `init`
  失敗）で「せいぜい 1 回」のみ pin した。
- `IConnectionPoint` 非実装は「pairing が試みられれば `init` 失敗」
  の pin — 成功経路が pairing 非依存であることの間接証拠。
- `controller.setState`/`getState`/`setComponentState` は production
  single 経路では呼ばれない — honest 実装 + unit test カバーのみ。
  `kNotImplemented` ではないことを Pass と書かない。

## 残件（非目標として切り分け）

- CLAP 側対応（single-component 相当の対象なし）、`test_fault`
  alloc/bad-timestamp、`mark_dirty`/`performEdit`、GUI-L2 real-dock
  lane、consumer oracle、MSRV/`*-full` lane。
- 外部 validator の provisioned 整備（conventional layout での検出 —
  本 run は explicit paths で実走済み）。
