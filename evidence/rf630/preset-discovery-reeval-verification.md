# Issue #630 preset-discovery 外部検証再評価 slice — 検証記録

前 slice で clap-validator 0.4.1 の `preset-discovery-(crawl|load)`
deadlock（upstream free-audio/clap-validator issue #41）を理由に
宣言的除外していた surface を、upstream PR #41 merge commit
`47e242404df89f3c8454aaf1c02bb0caca6db2ce` を含む build で再実走した。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| Node | v26.12.1 |
| git HEAD | `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`（run.json 記録値） |
| Rust | mise 管理 toolchain（cargo 1.99.1 / rustc 1.99.1） |

## 使用 validator / artifact（SHA-256）

| 対象 | SHA-256 |
| --- | --- |
| clap-validator `pin-47e2424/clap-validator.exe` | `b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26` |
| VST3 SDK `vst3sdk-build/bin/Release/validator.exe` | `5889a89685cd0362941659011ec82d8584e0ebb39d77030d1a2b2cbd2d68a4fd` |
| `target/debug/ruviform_plugin_test_clap.dll` | `9fd29888e005631131a79fa0d939b9e17c6a2a3222d81b987412dfe003277271` |
| `target/debug/ruviform_plugin_test_vst3.dll` | `bf35cb36381a6de3da08163ea513bea3f0c5032917db969f7a5467cb6d58c6db` |

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

## canonical 実走

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
| clap test_gain | Passed | 29 / 14 skipped / 1 warning / 0 failed | preset-discovery 除外なし |
| clap test_fault | Passed | 23 / 14 / 0 / 0 | `param-(set|fuzz)` のみ除外 |
| clap test_stateful | Passed | 34 / 9 / 1 / 0 | preset-discovery 除外なし |
| clap test_instrument | Passed | 33 / 11 / 0 / 0 | 同上 |
| clap test_editor | Passed | 20 / 23 / 1 / 0 | 同上 |
| clap test_spawn_fail | Passed | 8 / 1 skipped / 0 warning / validator exit 1 | declared expectation met: 35 declined / 0 unexpected / 0 crashed |
| vst3-sdk-validator | Passed | 235 passed / 0 failed | `test_spawn_fail` decline 47 occurrences 記録 |

informational `test_fault` unfiltered leg は Unsupported
（validator exit 101・JSON 無し、tool 側 panic）で verdict 対象外。

## preset-discovery の実測

- run.json の全 fixture CLAP leg command に
  `--exclude preset-discovery-(crawl|load)` は無い。
- raw `evidence/rf630/abi-validator/clap-*.txt` に
  `preset-discovery-crawl` / `preset-discovery-load` /
  `preset-discovery-descriptor-consistency` が出力され、crawl/load は
  `success` として観測される（例: `clap-test_spawn_fail.txt`、
  `clap-test_fault-unfiltered.txt`）。
- 前 slice の「除外=未実施」は解消。descriptor-consistency に加え
  crawl/load も外部 validator で実行・合格。

## contract test / 非回帰（実 exit code）

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

## 未検証・後続（正直な記録）

- AC4（実 Window/DComp）・AC7（consumer verdict）は本 slice 対象外、
  NotRun。
- clap-validator `param-fuzz` tool 側 panic（informational Unsupported）
  は upstream 依存として継続記録。
- docs index（`docs/**/README.md`）は Coordinator が統合時に再生成。
- 統合検証は merge 後 main SHA の必須 CI 証拠が正本。
