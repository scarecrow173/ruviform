# Issue #630 外部 ABI validator slice — 検証記録

Issue #630 AC3（外部権威による ABI 検証）+ AC5 残部（artifact digest /
SDK・validator version・license 記録）の実走証拠。
POLICIES.md の「独立 Validator/Reference Host — 自己検証だけに依存
しない」要求を、Ruviform production adapter とは独立した 2 つの外部
validator の実走で充足する。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64) |
| Node | v26.12.1（mise 管理） |
| git HEAD | `70a5fa4d9d440a2556cd57d325643eaa499c9f8f`（branch `feat/630-abi-validator`、検証時点） |
| Rust | mise 管理 toolchain（`mise run check|lint|test` 経由） |

## 使用 validator（version / license / digest）

### clap-validator

- version: **0.4.1**（`clap-validator.exe --version` で確認）
- license: **MIT OR Apache-2.0**（free-audio/clap-validator リポジトリ）
- 配布形態: GitHub Releases pinned Windows x86_64 binary（sha256 は
  release page 記載値と照合済み）
- binary SHA-256: `825439ad3ff98770a24656013eeb11591407687b97058e1db9549c4f7d53e38a`
- path: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/clap-validator.exe`

### Steinberg VST3 SDK validator

- SDK version: **VST 3.8.1**（`validator.exe -version` 出力:
  `VST 3.8.1 Plug-in Validator`）
- SDK source commit: `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`
  （github.com/steinbergmedia/vst3sdk master HEAD・clone 日時点）
- license: **Steinberg VST3 License**（`vst3sdk/LICENSE.txt` —
  validator 実行のみに使用、SDK 成果物は配布物へ同梱しない）
- build: CMake + MSVC（VS2022 BuildTools 同梱 cmake）で `validator`
  target を Release build
- binary SHA-256: `5889a89685cd0362941659011ec82d8584e0ebb39d77030d1a2b2cbd2d68a4fd`
- path: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/vst3sdk-build/bin/Release/validator.exe`

## 対象 artifact（SHA-256）

| artifact | SHA-256 |
| --- | --- |
| `target/debug/ruviform_plugin_test_clap.dll` | `b770d7dafed4f45010b5fece9e6cc0111d0cd383a3703e0012ba3ee9931af470` |
| `target/debug/ruviform_plugin_test_vst3.dll` | `0f5e9e51e3bf2427502404d8ab6206442035e9f39f2b2920e0d39a8b2fb14688` |

VST3 は DLL を spec layout
`ruviform-plugin-test-vst3.vst3/Contents/x86_64-win/ruviform-plugin-test-vst3.vst3`
（bundle 名 = inner binary 名）に staging して validator に渡した
（runner が一時 dir に生成・実行後削除）。

## 実行コマンドと結果

### canonical run

```sh
node ci/plugin-abi-validator/run.mjs \
  --clap-validator "$LOCALAPPDATA/ruviform/plugin-abi-validator/clap-validator.exe" \
  --vst3-validator "$LOCALAPPDATA/ruviform/plugin-abi-validator/vst3sdk-build/bin/Release/validator.exe" \
  --evidence-dir evidence/rf630/abi-validator
```

- exit code: **0** / verdict: **Passed**
- 構造化結果: `evidence/rf630/abi-validator/run.json`
  （schema `rf630-abi-validator-run/1`）
- 生出力: `clap-test_*.txt`（6 本）・`vst3-validator.txt`

### leg 別結果

| leg | status | counts | 備考 |
| --- | --- | --- | --- |
| clap test_gain | Passed | success 26 / skipped 17 / warning 1 / failed 0 | warning = denormal 性能情報 |
| clap test_fault | Passed | success 19 / skipped 17 / warning 1 / failed 0 | `--exclude "param-(set|fuzz)"`（下記） |
| clap test_stateful | Passed | success 31 / skipped 12 / warning 1 / failed 0 | state-reproducibility 修正後 |
| clap test_instrument | Passed | success 30 / skipped 14 / warning 0 / failed 0 | |
| clap test_editor | Passed | success 17 / skipped 26 / warning 1 / failed 0 | warning = denormal 性能情報 |
| clap test_fault（unfiltered・informational） | Unsupported | — | validator exit 101（tool 側 panic、下記） |
| vst3 suite bundle | Passed | 235 passed / 0 failed | `Result:` summary より |

### test_fault の param 除外について

`test_fault` は stepped Fault parameter で abort/exit/hang を意図的に
武装する fixture。param 値を変更する validator case（`param-set-*` /
`param-fuzz-*`）は設計上 plugin crash を誘発するため、canonical run
では `--exclude "param-(set|fuzz)"` を宣言的に適用した。除外理由は
run.json の `leg.exclusion.reason` に記録。unfiltered run は
informational leg として別途実行し、validator 自体の exit 101
（`param-fuzz-sample-accurate` で sandbox thread panic —
clap-validator 側の tool bug）を Unsupported として証拠化した。
fatal fault surface は L1 out-of-process fault lane
（`mise run test-l1-real-plugin`）で別途検証済み。

### contract test

```sh
node --test ci/plugin-abi-validator/tests/
```

- exit code: **0** / 18 tests pass
- fail-closed 契約（missing tool/artifact → NotRun、spawn 失敗 →
  Failed、validator crash → 非 Pass）を固定。

## validator 指摘への対応（fixture 側修正・全て test crate 内）

| 指摘 | 修正 |
| --- | --- |
| CLAP param module path invalid | `b"/"` → `b""` |
| fractional sample rate 拒否 | validator 提供値を受理 |
| event 数 overrun | `MAX_EVENTS` 1024（bounded 維持） |
| instrument note 処理 fail | note-on/off event 処理を修正 |
| stateful state-reproducibility 3 件 fail | `state.load` 時に `clap_host_params.rescan(CLAP_PARAM_RESCAN_VALUES)` を要求（CLAP 契約違反 — 値変化を host へ通知必須） |
| VST3 setup/teardown 大量失敗 | `terminate` 非終端化・`setupProcessing` 受理 state 拡大・bus info 修正 |

production adapter crate（`ruviform-plugin-vst3`/`ruviform-plugin-clap`/
`ruviform-plugin-host`/`ruviform-plugin-export`/`ruviform-plugin-core`）
は **zero-diff** を維持。

## verdict

- CLAP 5 plugin + VST3 bundle: **全 canonical leg Passed、0 failed**。
- 本 slice の AC3（外部権威による ABI 検証）+ AC5 digest/provenance
  記録を **Passed** として記録する。
- 残る正直な記録: clap-validator の tool 側 panic（informational
  Unsupported leg）と、denormal 性能 warning（情報通知・非失敗）。

## 未検証・後続

- mise task `test-plugin-abi-validator` の登録は Coordinator が統合時に
  実施（本 slice では `mise.toml` 非編集。意図する TOML は
  `docs/designs/issue-630-abi-validator.md` に明記）。
- clap-validator `param-fuzz-sample-accurate` の tool 側 panic は
  upstream 依存 — 別 Issue 化の要否は Coordinator 判断。
- production `ruviform-plugin-vst3` adapter に VST3 fixture と同型の
  terminate/setupProcessing 契約疑義がある可能性 — 本 slice 範囲外、
  Coordinator へ報告済み。
