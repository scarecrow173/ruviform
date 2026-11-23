# Issue #630 preset-discovery 外部検証再評価 slice — 検証記録

前 slice で clap-validator 0.4.1 の `preset-discovery-(crawl|load)`
deadlock（upstream free-audio/clap-validator issue #41）を理由に
宣言的除外していた surface を、upstream PR #41 merge commit
`47e242404df89f3c8454aaf1c02bb0caca6db2ce` を含む build で再実走した
（PR#752）。本記録はその後に fixture surface が変化した現行 HEAD
`5099cbe5`（post-`scarecrow173/ruviform#911`・`#909`・`#914`）での
canonical lane 再実走による証拠更新を含む。宣言的除外構成は PR#752
着地時のまま維持（library-level 除外なし、test_fault の plugin-level
`param-(set|fuzz)` のみ）。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| Node | v26.12.1 |
| git HEAD | `5099cbe5836b5d53735951463e9244e7a840991c`（run.json 記録値。前回記録は `b6d0cc6a`） |
| Rust | mise 管理 toolchain（cargo 1.99.1 / rustc 1.99.1） |

## 使用 validator / artifact（SHA-256）

| 対象 | SHA-256 |
| --- | --- |
| clap-validator `pin-47e2424/clap-validator.exe` | `b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26` |
| VST3 SDK `vst3sdk-build/bin/Release/validator.exe` | `5889a89685cd0362941659011ec82d8584e0ebb39d77030d1a2b2cbd2d68a4fd` |
| `target/debug/ruviform_plugin_test_clap.dll` | `f8dc6cdc2976284e39243f656e34d49e30ee7215fbbc6844f6fede094ac9f20f` |
| `target/debug/ruviform_plugin_test_vst3.dll` | `e3cbf96400b2207c1c76e68229b4602210cecf6f2f70ed07e202d70367c6594e` |

旧 0.4.1 release binary（SHA-256
`825439ad3ff98770a24656013eeb11591407687b97058e1db9549c4f7d53e38a`）
は `%LOCALAPPDATA%/ruviform/plugin-abi-validator/clap-validator.exe`
に残置。本 slice では `pin-47e2424/` を `CLAP_VALIDATOR` に指定した。

### validator provenance / provisioning

- clap-validator:
  - source: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/src/clap-validator-upstream`
    （upstream `https://github.com/free-audio/clap-validator`、HEAD
    `47e242404df89f3c8454aaf1c02bb0caca6db2ce`、
    `git describe --tags --always --dirty` = `0.4.1-5-g47e2424`）。
  - build/provision: upstream checkout で
    `cargo build --release --bin clap-validator` を実行し、生成された
    `target/release/clap-validator.exe` を
    `%LOCALAPPDATA%/ruviform/plugin-abi-validator/pin-47e2424/clap-validator.exe`
    へ配置。Cargo dependency や commit 済み binary ではなく、runner には
    `CLAP_VALIDATOR` / `--clap-validator`（または `--validator-dir` を
    pin directory に向ける規約）で供給する。
  - version/license: `clap-validator.exe --version` は
    `clap-validator 0.4.1` を出力する。license は upstream `Cargo.toml`
    の `license = "MIT"` と `LICENSE`（MIT、Copyright 2022 Robbert van der
    Helm）。`--version` は 0.4.1 release と同じ文字列のため、pin の識別は
    source commit / `git describe` / SHA-256 で行う。
- VST3 validator:
  - source: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/vst3sdk`
    （`https://github.com/steinbergmedia/vst3sdk.git`、HEAD
    `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`、
    `git describe --tags --always --dirty` = `v3.8.1_build_84`、commit
    subject `VST3 SDK 3.8.1`）。
  - build/provision: SDK source を CMake + Visual Studio 17 2022 / MSVC で
    Release build し、`vst3sdk-build/bin/Release/validator.exe` を
    `VST3_VALIDATOR` / `--vst3-validator` で供給する（CMakeCache:
    `CMAKE_BUILD_TYPE=Release`、generator `Visual Studio 17 2022`、
    `CMAKE_HOME_DIRECTORY=.../vst3sdk`）。
  - version/license: `validator.exe --version` は
    `VST 3.8.1 Plug-in Validator` と
    `Program by Steinberg (Built on Nov 17 2026)` を出力する。license は
    root `vst3sdk/LICENSE.txt` の MIT License（Copyright 2026 Steinberg
    Media Technologies GmbH）。

## canonical 実走（HEAD `5099cbe5`）

```sh
CLAP_VALIDATOR="$LOCALAPPDATA/ruviform/plugin-abi-validator/pin-47e2424/clap-validator.exe" \
VST3_VALIDATOR="$LOCALAPPDATA/ruviform/plugin-abi-validator/vst3sdk-build/bin/Release/validator.exe" \
mise run test-plugin-abi-validator
```

- verdict: **Passed** / exit **0**
- 構造化結果: `evidence/rf630/abi-validator/run.json`
  （schema `rf630-abi-validator-run/2`）
- build: `cargo build -p ruviform-plugin-test-clap
  -p ruviform-plugin-test-vst3` exit 0

### leg 別結果（canonical 7 leg — 全 Passed）

| leg | status | counts | 備考 |
| --- | --- | --- | --- |
| clap test_gain | Passed | 30 / 14 skipped / 0 warning / 0 failed | preset-discovery 除外なし |
| clap test_fault | Passed | 22 / 14 / 1 / 0 | `param-(set|fuzz)` のみ除外 |
| clap test_stateful | Passed | 34 / 9 / 1 / 0 | preset-discovery 除外なし |
| clap test_instrument | Passed | 33 / 11 / 0 / 0 | 同上 |
| clap test_editor | Passed | 20 / 23 / 1 / 0 | 同上 |
| clap test_spawn_fail | Passed | 8 / 1 skipped / 0 warning / validator exit 1 | declared expectation met: 35 declined / 0 unexpected / 0 crashed |
| vst3-sdk-validator | Passed | 282 passed / 0 failed | `test_spawn_fail` decline 47 occurrences 記録 |

informational `test_fault` unfiltered leg は Unsupported
（validator exit 101・JSON 無し、tool 側 panic）で verdict 対象外。
raw log では `param-set-*` の sandbox deserialize panic
（`sandbox.rs:98`）・`param-fuzz-modulation` の 45s timeout crash・
`cli.rs:55` の scoped-thread panic を記録 — upstream
`free-audio/clap-validator#44` の既知状態が現行 pin でも持続。

## preset-discovery の実測

- run.json の全 fixture CLAP leg command に
  `--exclude preset-discovery-(crawl|load)` は無い。
- raw `evidence/rf630/abi-validator/clap-*.txt` に
  `preset-discovery-crawl` / `preset-discovery-load` /
  `preset-discovery-descriptor-consistency` が出力され、全 6 canonical
  fixture leg（test_gain・test_fault・test_stateful・test_instrument・
  test_editor・test_spawn_fail）で 3 case とも `success` として観測
  される。informational unfiltered leg の raw log でも preset-discovery
  3 case の completed を確認（param 系で validator 自身が落ちる前に
  完了済み）。
- 前 slice の「除外=未実施」は解消済みのまま維持。fixture surface 変化
  （#911 dirty 通知・#909 dropped_submits 帰属・#914 full_series 期待値）
  後の現行 HEAD でも、crawl/load/descriptor-consistency が外部
  validator で実行・合格。

## contract test / 非回帰（実 exit code、HEAD `5099cbe5`）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `node --test ci/plugin-abi-validator/tests/` | 0 | 32 pass / 0 fail |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-plugin-test-vst3` | 0 | Pass |
| `mise run check ruviform-plugin-test-clap` | 0 | Pass |
| `mise run lint ruviform-plugin-test-vst3` | 0 | Pass |
| `mise run lint ruviform-plugin-test-clap` | 0 | Pass |
| `mise run test ruviform-plugin-test-vst3` | 0 | Pass |
| `mise run test ruviform-plugin-test-clap` | 0 | Pass |
| `mise run check-l1-real-plugin` | 0 | Pass |
| `mise run lint-l1-real-plugin` | 0 | Pass |
| `mise run test-l1-real-plugin` | 0 | Pass |
| `mise run test-rt` | 0 | Pass |
| `mise exec -- prek run --all-files` | 0 | Pass（初回は `vst3-validator.txt` の trailing whitespace/EOF を hook が自動修正 → 再実行で全 Pass） |
| `mise run test-plugin-abi-validator` | 0 | verdict Passed |

環境メモ: 初回の canonical 実行では mbx managed target の実体化が
leg 実行に間に合わず artifact missing の NotRun となった（run.json
の `build.ok` は true）。fixture build 再実行で DLL が実体化した後の
2 回目が本記録の Passed。informational leg の孤児 sandbox 子プロセス
（clap-validator.exe、fixture DLL を map したまま残留）が後続
`cargo test` の LNK1104 を起こしたため、検証の継続前に該当 PID を
強制終了した。upstream の孤児プロセス残存は既知の tool 側制約であり、
fixture/adapter 側の問題ではない。

## 未検証・後続（正直な記録）

- AC4（実 Window/DComp）・AC7（consumer verdict）は本 slice 対象外、
  NotRun。
- clap-validator `param-fuzz` tool 側 panic（informational Unsupported）
  は upstream `free-audio/clap-validator#44` 依存として継続記録。
  release pin による rerun は `scarecrow173/ruviform#867` で別追跡。
- docs index（`docs/**/README.md`）は Coordinator が統合時に再生成。
- 統合検証は merge 後 main SHA の必須 CI 証拠が正本。
