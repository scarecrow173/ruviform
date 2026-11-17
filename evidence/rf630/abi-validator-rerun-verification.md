# Issue #630 外部 ABI validator 再実走 slice — 検証記録

Issue #630 AC3 残件（PR#735 で deferral された post-#735 ABI
surface — CLAP preset-discovery/preset-load・VST3 IUnitInfo 系・
`test_spawn_fail`）の外部権威検証と、stale だった run.json
（schema /1、pre-#735 fixture）の post-#735 更新。
POLICIES.md「独立 Validator/Reference Host — 自己検証だけに依存
しない」を、Ruviform production adapter から独立した pin 済み
外部 validator の再実走で充足する。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| Node | v26.12.1（mise 管理） |
| git HEAD | `1d18e2970d887fe925d3777ddb814bc46644af3d`（branch `feature/630-abi-validator-rerun`、origin/main と同一 — merge 時点で差分なし） |
| Rust | mise 管理 toolchain（`mise run check|lint|test` 経由） |

## 使用 validator（version / license / digest — 前 slice と同一 pin）

### clap-validator

- version: **0.4.1**（`clap-validator.exe --version` で確認）
- license: **MIT OR Apache-2.0**（free-audio/clap-validator）
- binary SHA-256: `825439ad3ff98770a24656013eeb11591407687b97058e1db9549c4f7d53e38a`
- path: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/clap-validator.exe`

### Steinberg VST3 SDK validator

- SDK version: **VST 3.8.1**（`validator.exe -version`）
- SDK source commit: `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`
- license: **Steinberg VST3 License**（`vst3sdk/LICENSE.txt` —
  validator 実行のみ、成果物へ同梱しない）
- binary SHA-256: `5889a89685cd0362941659011ec82d8584e0ebb39d77030d1a2b2cbd2d68a4fd`
- path: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/vst3sdk-build/bin/Release/validator.exe`

## 対象 artifact（SHA-256 — post-#735 fixture）

| artifact | SHA-256 |
| --- | --- |
| `target/debug/ruviform_plugin_test_clap.dll` | `09bc4cdd13fd977c3a9a16d3cd8d72ac610f3dd2118febef9eb02e000abc86a7` |
| `target/debug/ruviform_plugin_test_vst3.dll` | `8496b044436996011326b559aee052e51d00dc17d163328ddf0bd794bec62efb` |

前 slice 記録（`12b9d82a…` / `05e7eaed…`）から更新 — #735 の
preset surface・`test_spawn_fail` 追加を含む build が validator
に実際に検証された artifact。debug cdylib の bit 再現は保証
されないため、digest は run.json の記録が正本。

## canonical 実走

```sh
mise run test-plugin-abi-validator
# = node ci/plugin-abi-validator/run.mjs
#   （tool は --validator-dir 既定候補
#    %LOCALAPPDATA%/ruviform/plugin-abi-validator/ から解決）
```

- 実行時刻: 2026-11-17T11:06:03Z–11:06:12Z（run.json `startedAt`/
  `finishedAt`）
- exit code: **0** / verdict: **Passed**
- 構造化結果: `evidence/rf630/abi-validator/run.json`
  （schema `rf630-abi-validator-run/2`）
- 生出力: `clap-test_*.txt`（7 本）・`vst3-validator.txt`
- build: `cargo build -p ruviform-plugin-test-clap
  -p ruviform-plugin-test-vst3` exit 0（runner 内で実行）

### leg 別結果（canonical 7 leg — 全 Passed）

| leg | status | counts | 備考 |
| --- | --- | --- | --- |
| clap test_gain | Passed | 27 passed / 14 skipped / 1 warning / 0 failed | preset-discovery crawl/load 除外を記録 |
| clap test_fault | Passed | 20 / 14 / 1 / 0 | preset-discovery + `param-(set|fuzz)` の 2 除外 |
| clap test_stateful | Passed | 32 / 9 / 1 / 0 | preset surface 追加後も 0 failed |
| clap test_instrument | Passed | 31 / 11 / 0 / 0 | 同上 |
| clap test_editor | Passed | 18 / 23 / 1 / 0 | 同上 |
| clap test_spawn_fail | Passed | 6 passed / 1 skipped / 0 warning / validator exit 1 | **declared expectation met**: 35 plugin-instance case が create_plugin-null signature で declined、unexpected 0・crashed 0 |
| vst3-sdk-validator | Passed | 235 passed / 0 failed（exit 0） | `test_spawn_fail` の decline 証拠 47 occurrences を `declaredUncreatable` に記録 |

### informational leg（canonical verdict 対象外）

| leg | status | 内容 |
| --- | --- | --- |
| clap test_fault unfiltered | Unsupported | validator exit 101・JSON report 無し（`param-fuzz` 系で tool 側 panic — 前 slice と同じ既知の tool 側 bug）。fault param case を含む実行であり、fatal fault mode が意図的に process を終了させる設計と整合。canonical verdict には含めない |

## post-#735 surface の扱い（declared-expectation）

### CLAP `test_spawn_fail`

fixture 契約は「descriptor に列挙されるが `create_plugin` が
意図的に `None` を返す」（spawn-phase CreateInstance 拒否
oracle）。clap-validator はこれを plugin-instance case の failed
として報告するため、runner は `expectCreateDecline` 宣言で
「全 failed case が create-decline signature に一致・crash 0・
decline 1 件以上」のみを Passed とする。予期しない failed・
crash・decline 未観測は Failed — fail-closed を維持。

### preset-discovery crawl/load の宣言的除外

clap-validator 0.4.1 の library-level `preset-discovery-crawl` /
`preset-discovery-load` case は、fixture の preset-discovery
factory 登録を検出して実行されると validator 内部で 45 秒
timeout + JSON 無し終了する。upstream 既知問題
（free-audio/clap-validator issue #41 — MetadataReceiver の
self-deadlock、0.4.1 pin 以降の master で修正済み）と判断し、
`preset-discovery-(crawl|load)` を target-level の宣言的除外と
した。全 fixture CLAP leg の `leg.exclusions` に pattern・理由・
upstream 参照を記録。

**これは外部 preset 検証の省略であり Pass ではない。**
`preset-discovery-descriptor-consistency` case は全 leg で実行
され catalog を検証しており、preset load/save 往復は fixture の
in-process ABI test（`mise run test ruviform-plugin-test-clap` 等）
が別証拠として存在する。validator pin を upstream 修正を含む
release に更新した際に除外を再評価する。

### VST3 `test_spawn_fail`

suite validator は bundle 内全 class を scan→`createInstance`
するため、`test_spawn_fail` の意図的拒否（`kInvalidArgument`）
に対し `Failed to create component instance of test_spawn_fail!` /
`Failed to setup test!` を反復出力する。これらは validator の
`Result:` 集計に含まれないため、runner は raw output から marker
出現回数を `leg.declaredUncreatable` に記録する。declared class
の decline 証拠が 0 件なら leg を Failed に倒す（拒否を観測
しなければ Pass にしない）。

## contract test

```sh
node --test ci/plugin-abi-validator/tests/
```

- 結果: **32 pass / 0 fail**、exit 0（補助証拠 — canonical mise
  task 未定義）
- 新規固定: declared create-decline の met/unexpected/crash/
  no-JSON fallback、`clapLegExclusions` 合成、VST3
  declared-uncreatable の記録・反転・no-op、schema /2 フィールド。

## 非回帰検証（実 exit code）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-plugin-test-vst3` | 0 | Pass |
| `mise run check ruviform-plugin-test-clap` | 0 | Pass |
| `mise run lint ruviform-plugin-test-vst3` | 0 | Pass（`-D warnings`） |
| `mise run lint ruviform-plugin-test-clap` | 0 | Pass（同上） |
| `mise run test ruviform-plugin-test-vst3` | 0 | Pass（91 passed, 1 ignored 他） |
| `mise run test ruviform-plugin-test-clap` | 0 | Pass（66 passed, 1 ignored 他） |
| `mise run check-l1-real-plugin` | 0 | Pass |
| `mise run lint-l1-real-plugin` | 0 | Pass（`-D warnings`） |
| `mise run test-l1-real-plugin` | 0 | Pass（最終実行で全 suite 緑 — 下記 flaky 注記） |
| `mise run test-rt` | 0 | Pass |
| `mise exec -- prek run --all-files` | 0 | Pass（初回は `vst3-validator.txt` の trailing whitespace/EOF を hook が自動修正 → 再実行で全 Pass） |
| `node --test ci/plugin-abi-validator/tests/` | 0 | 32 pass |
| `mise run test-plugin-abi-validator` | 0 | verdict Passed |

### flaky 注記（正直な記録）

`mise run test-l1-real-plugin` の途中実行で
`l1_real_test_plugin_e2e` の instrument 系 test
（`real_test_plugin_l1_instrument_note_off_releases_to_silence`、
`real_test_plugin_l1_instrument_transport_clicks`）が frame-exact
PCM assertion で断続的に fail した。失敗 test は実行ごとに異なり
（clap/vst3 それぞれで観測）、最終実行では両 crate の e2e suite
が 10/10 で全緑。`crates/` 配下は本 slice で zero-diff
（`git diff origin/main -- crates/` が空）であり、本 slice の
変更（`ci/` の JS runner + evidence + docs のみ）が原因では
ありえない。高負荷時の既存 lane flaky として記録し、恒久対応は
別 Issue の判断に委ねる。

## verdict

- CLAP 6 plugin + VST3 bundle: **全 canonical leg Passed、0 failed**。
- AC3 の headless 残件（post-#735 preset/spawn ABI surface の
  独立権威検証）を **Passed** として記録。ただし
  preset-discovery crawl/load は upstream deadlock により
  **宣言的除外（未実施 — Pass ではない）**であり、descriptor-
  consistency case と fixture in-process ABI test が代替証拠。
- AC5 の artifact digest 再記録を兼ねる（上表の post-#735
  SHA-256）。

## 未検証・後続（正直な記録）

- **AC4**（実 Window/DComp 解消判定）: `scarecrow173/ruviform#532`/
  `#389`/`#631` 側 gate — 本 slice 対象外、NotRun。
- **AC7**（consumer verdict — synvelis#101/#127/#128/#176・
  audio-lens#127）: 各 Issue open の consumer gate — 本 slice
  対象外、NotRun。「AudioLens への新依存を強制しない」は
  本 slice でも依存追加なしで構造的に維持。
- **preset-discovery crawl/load の外部検証**: clap-validator の
  pin 更新（upstream issue #41 修正を含む release）後に再評価。
- **clap-validator `param-fuzz` tool 側 panic**: informational
  Unsupported leg として記録済み。upstream 依存 — Issue 化要否は
  Coordinator 判断（前 slice から継続）。
- **docs index**: `docs/**/README.md` は Coordinator が統合時に
  再生成（本 slice では未編集・未同期を許容）。
- **統合検証**: ローカル検証は変更範囲の検証。merge 後 main SHA
  の必須 CI 証拠が統合受入の正本（Coordinator が確認）。
