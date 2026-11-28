# rf#1064 consumer-verify single-SHA lockfile guard — verification evidence

## 対象と環境

- Issue: `scarecrow173/ruviform#1064`（partial slice。Issue は close しない）。
- base SHA: `939af2e12c9c80a96f8db22e00feeb9459041202`（最終検証前に `origin/main` を fetch し、同 SHA のままであることを確認）。
- code head / branch CI target SHA: `b1745816100b85e229b49a5e29ca8fbc963096e4`。
- final head: 本 evidence を含む evidence-only descendant commit。コード差分と branch CI target は上記 code head で固定し、evidence commit は検証対象の意味を変えない。
- OS/Arch: Windows 11 / x86_64（MINGW64 shell）。
- toolchain: `node v26.12.1`、`mise 2026.11.2 windows-x64`、`rustc 1.99.1 (58cf8f9c3 2026-10-13)`、`cargo 1.99.1 (5f94df478 2026-08-27)`。
- canonical verification: `mise run ...` / `mise exec -- prek ...`。consumer scratch checkout に対する直接 `cargo fetch` は補助証拠（canonical = mise）として記録する。

## 原因と修正方針

原因は、従来の Synvelis/AudioLens 両 lane の manifest pin が scratch checkout 直下の root `Cargo.toml` だけを sed で書き換えていたことにある。Synvelis では nested workspace member の tracked manifest に crate-local direct ruviform git dependency があり、少なくとも `crates/synvelis-player/Cargo.toml` の `ruviform-plugin-export` と `crates/synvelis-script-luau/Cargo.toml` の `ruviform-script` / `ruviform-script-luau` が root-only 正規化の対象外だった。そのため candidate rev source と default-branch source が混在し、cargo fetch 後の single-SHA guard が setup 段階で fail-closed していた。

修正では、両 lane 重複の sed を `ci/consumer-verify/pin-ruviform-rev.mjs` へ集約した。helper は consumer scratch checkout の root `Cargo.toml` から workspace `members` / `exclude` を読み、git tracked file に限定して workspace member manifest を列挙する。対応する single-line inline-table dependency について、既存意味論どおり rev pin 置換・branch 除去・rev 未指定への rev 注入を行う。table-shaped dependency、複数行 inline table、tag pin、入れ子 inline table など、この contract では安全に書き換えられない relevant shape は silently skip せず fail-closed にする。

## 受入条件ごとの結果

| AC | 結果 | 証拠 |
| --- | --- | --- |
| nested/tracked `Cargo.toml` の relevant な ruviform git dependency forms が candidate rev に正規化される | Pass | helper 実測で Synvelis scratch checkout は `scanned 17 workspace manifest(s); changed 3 manifest(s); normalized 32 dependency line(s)`。変更対象は root、`crates/synvelis-player/Cargo.toml`、`crates/synvelis-script-luau/Cargo.toml`。AudioLens 実測は `scanned 12 workspace manifest(s); changed 1 manifest(s); normalized 19 dependency line(s)` |
| nested crate-local direct git dependency fixture/contract test が追加され、root-only regression を検出できる | Pass | `ci/consumer-verify/tests/pin-ruviform-rev.test.mjs` に nested member fixture、untracked file boundary、CLI 契約を追加。`consumer-verify-yml-contract.test.mjs` が両 lane の shared helper 使用と root-only sed 排除を固定。`mise run test-ci-consumer-verify` は 95/95 pass |
| 正規化できない relevant dependency shape は fail-closed | Pass | table-shaped dependency、multiline inline table、tag pin、unmatched workspace member pattern、CLI failure path の test が pass。実装は unsupported shape で非 zero exit |
| 指定正規検証が exit 0 | Pass | 下表の `mise run fmt-check` / `test-ci-consumer-verify` / `test-ci-daily` / `test-ci-tracker` / `prek run --all-files` がすべて exit 0 |
| branch CI を最終 code headで dispatch して記録する | Pass | run `46132073358` は head SHA `b1745816100b85e229b49a5e29ca8fbc963096e4`、input `ruviform_rev` 同 SHAで dispatch。詳細は下記 branch CI 節 |
| Synvelis lane が setup ではなく実 check/test lane に到達する | Pass（branch CI target 上） | run `46132073358` で `Pin ruviform rev in manifest` と `cargo fetch + single-SHA lockfile guard` が successし、その後 `check-full lane` に到達した。最終的な consumer product lane の成否は本 slice の保証範囲と分けて記録する |

## 補助証拠: legacy root-only 再現と修正後 scratch checkout

いずれも consumer repo の永続ファイルは変更していない。clone した scratch checkout 内だけで実施した補助証拠である。

| 検証 | 結果 |
| --- | --- |
| Synvelis scratch checkout に従来の root-only sed を適用し `cargo fetch` | `fetch_rc=0`、`rootonly_total=122 rootonly_bad=88`。workflow と同じ grep 判定で 88 行が `?rev=<candidate>#<candidate>` 形に解決されず、Issue に記録された guard 失敗形状を再現 |
| Synvelis scratch checkout に新 helper を適用し `cargo fetch` | helper 出力 `scanned 17 workspace manifest(s); changed 3 manifest(s); normalized 32 dependency line(s)`。lockfile grep は `fullhelper_total=36 fullhelper_bad=0` で、全行が candidate rev の single-SHA 形 |
| AudioLens scratch checkout に新 helper を適用 | helper 出力 `scanned 12 workspace manifest(s); changed 1 manifest(s); normalized 19 dependency line(s)`。root-only だった既存 lane でも同じ helper が使えることを確認 |

## 実行した正規検証

| Command | Exit | Result |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run test-ci-consumer-verify` | 0 | Pass（95 tests, 95 pass, 0 fail） |
| `mise run test-ci-daily` | 0 | Pass（113 tests, 113 pass, 0 fail） |
| `mise run test-ci-tracker` | 0 | Pass（108 tests, 108 pass, 0 fail） |
| `mise exec -- prek run --all-files` | 0 | Pass（trim trailing whitespace / end of files / yaml / toml / mixed line ending / BOM / private key / actionlint） |
| `mise exec -- actionlint .github/workflows/consumer-verify.yml` | 0 | Pass（補助証拠。workflow 単体の静的検査） |

これらはローカル検証であり、merge 後の main SHAに対する統合検証ではない。統合受入の証拠は merge後の対象 SHAの必須 CI結果として Coordinatorが確認する。

## branch CI（manual workflow_dispatch）

- Run: https://github.com/scarecrow173/ruviform/actions/runs/46132073358
- workflow: `consumer-verify`
- event: `workflow_dispatch`
- branch: `fix/1064-consumer-lockfile`
- head SHA / input `ruviform_rev`: `b1745816100b85e229b49a5e29ca8fbc963096e4`
- dispatch 時刻: `2026-11-28T09:46:59Z`

| Job / step | 状態 | 解釈 |
| --- | --- | --- |
| preflight | success | workflow 自身の `mise run test-ci-consumer-verify` self-test を含めて通過 |
| verify-audiolens: Pin ruviform rev in manifest | success | shared helper が consumer scratch checkoutで実行できた |
| verify-audiolens: cargo fetch + single-SHA lockfile guard | success | setup guard を通過 |
| verify-synvelis: Pin ruviform rev in manifest | success | nested manifest 正規化が実 runner で実行できた |
| verify-synvelis: cargo fetch + single-SHA lockfile guard | success | 本 Issueの setup guard を通過 |
| verify-synvelis: check-full lane | success | setup失敗ではなく実 check/test lane側へ進み、check lane は通過 |
| verify-synvelis: test-full lane | success | setup後の実 test lane も通過 |
| verify-synvelis: test-rt lane | failure | `ld terminated with signal 7 [Bus error]` により `synvelis-player` の `verified_import` / `tim04_l1_abi_state` test binary link に失敗。lockfile guard や manifest pin の失敗ではない |

最終状態は completed / overall conclusion `failure` だった。job 結果は preflight `success`、verify-audiolens `success`、verify-synvelis `failure`、summary `failure`。summary の失敗は lane failureを集約する `Enforce not-run-is-not-pass` であり、新たな検証失敗ではない。

artifact `consumer-evidence-audiolens-attempt-1` は status `pass`、resolved consumer SHA `0f1a1aac86336bff98df8bab1162e23bf5c36672`、coverage `pass=16 fail=0 notRun=1 unsupported=2 skipped=1` だった。artifact `consumer-evidence-synvelis-attempt-1` は status `fail`、statusReason `lane_fail: mise run test-rt (RT no-alloc)`、resolved consumer SHA `88ec448e16b5cf868f73c8af4f50902c73b079da`、coverage `pass=6 fail=1 notRun=1 unsupported=3 skipped=12` だった。Synvelis の pass laneは `dep-access`、`manifest-pin`、`manifest-fetch`、`rev-check`、`check-full`、`test-full` であり、本 slice が対象とする setup guard通過と実 check/test lane到達は branch CI上で確認できた。`test-rt` 失敗後の headless/oracle/UI/GUI lane は workflow の依存関係により skipped であり、これらの lane結果を pass と主張しない。

`test-rt` の失敗ログは linker が bus errorで終了したもので、`synvelis-player` test binary `verified_import` と `tim04_l1_abi_state` の link に失敗した。ruviform側 source はすべて `?rev=b1745816100b85e229b49a5e29ca8fbc963096e4#b1745816` でcompileされており、default-branch source混在や single-SHA guard失敗の症状ではない。この製品/環境側 failureは本 sliceの非対象として修正していない。

## NotRun / non-claims

- merge後 main SHAの統合検証: NotRun。本 evidence は branch headのローカル検証と manual branch CIであり、post-merge integration CI evidenceではない。
- consumer product の全機能互換性: 本 sliceでは保証しない。setup通過後の consumer check/test/product lane failureがあれば、本修正の成功とは別問題として記録する。
- GUI-L2 / EXT-L3 / 実 OS / 実機検証: NotRun。workflow 変更 sliceのローカル必須検証ではなく、branch CIの記録範囲に限定する。
- `Cargo.lock` / root `Cargo.toml` / `mise.toml` / `mise.ci.toml`: 変更していない。依存関係変更がないため `Cargo.lock` 再生成は不要だった。
- docs index: 編集・再生成していない。Coordinatorが統合時に整備する。
- rf#1034 frozen領域、他 workflow、audio-lens/synvelis consumer repositoryの永続ファイル: 変更していない。
