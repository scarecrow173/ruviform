# evidence/rf768 — verification record

`ruviform#768` — `l1_fault` / `l1_stateful` E2E staleness oracle 監査
（`ruviform#763` 同 class）の修正後検証記録。
ローカル検証＝変更範囲の検証。統合検証は merge 後の対象 SHA の
必須 CI 結果（Coordinator が確認）。

## Canonical verification（mise）

| lane | command | result |
| ---- | ------- | ------ |
| canonical E2E | `mise run test-l1-real-plugin` | Pass（exit 0）— `l1_real` 10/10×2、`l1_fault` 19/19×2、`l1_stateful` 3/3・5/5（両 ABI・`-- --ignored` 実走） |
| typecheck | `mise run check-l1-real-plugin` | Pass（exit 0） |
| lint | `mise run lint-l1-real-plugin` | Pass（exit 0・`-D warnings`） |
| fmt | `mise run fmt-check` | Pass（exit 0） |
| fixture pkg | `mise run check ruviform-plugin-test-vst3` / `-clap` | Pass |
| fixture pkg | `mise run lint ruviform-plugin-test-vst3` / `-clap` | Pass |
| fixture pkg | `mise run test ruviform-plugin-test-vst3` / `-clap` | Pass（全 suite `0 failed` — fixture src 本 slice zero-diff・default 経路不変の確認） |
| hooks | `mise exec -- prek run --all-files` | Pass（全 hook Passed/Skipped） |

## 補助証拠（canonical ≠ mise）— dual-binary loop

`evidence/rf768/loop.sh`（`evidence/rf763/loop3.sh` 型 — vst3+clap
test binary 並走 + sibling main checkout での cargo build 負荷 lane）。
binary は `cargo test --features l1-runner --no-run` で取得し
`bin-pre/`（修正前）・`bin-post/`（修正後）に退避。

### pre-fix（logs-bin-pre/）

| lane | result |
| ---- | ------ |
| vst3-stateful | 30/30 pass |
| clap-stateful | 30/30 pass |
| vst3-fault | 1/5 pass |
| clap-fault | 3/5 pass |

fail 6 件は全て `hang_converges_on_timeout` の
`instantiate` で `SpawnFailed { cause: Timeout }` — spawn deadline
環境 flake（`evidence/rf763` 記録済み同 class）。staleness-oracle
由来の失敗は 0 件（詳細は `reproduction.md`）。

### post-fix（logs-bin-post/）

| lane | result |
| ---- | ------ |
| vst3-stateful | 30/30 pass |
| clap-stateful | 30/30 pass |
| vst3-fault | 1/5 pass |
| clap-fault | 3/5 pass |

fail 6 件は pre-fix と同一 signature（`hang_converges_on_timeout` の
`SpawnFailed { cause: Timeout }` at instantiate）— 本修正による
regression なし。matched-only 分類・bounded resend を通す全
oracle path（rt_alloc/bad_time/invalid/error/disarm/stateful
round-trip）は全 run で pass — substituted call を計数対象から
外しても最低到達数 gate が本環境では充足した。

## 判定

- staleness-oracle 由来の fail は pre/post とも 0 件 — 修正の正しさは
  canonical lane PASS と監査の構造論証（`docs/discovery/
  issue-768-staleness-oracle.md`）で示す。
- `hang` spawn-deadline flake は本 slice 対象外の residual として
  記録（test-file 変更では解消できない runner spawn 経路の問題 —
  `ruviform#767` 系の別 class）。

## 第 2 slice（`l1_real_adapter` / `l1_real_test_plugin` 非 instrument 面）

### Canonical verification（mise）

| lane | command | result |
| ---- | ------- | ------ |
| canonical E2E | `mise run test-l1-real-plugin` | Pass（exit 0）— `l1_real_adapter_e2e` 3/3×2・`l1_real_test_plugin_e2e` 10/10×2・`l1_fault` 19/19×2・`l1_stateful` 3/3+5/5（両 ABI・`-- --ignored` 実走）。対象 `*_l1_full_series` は matched-only 計数化後に pass |
| typecheck | `mise run check-l1-real-plugin` | Pass（exit 0）— `LegDeltas` 返却化の初回 run で `dropped` 未読 warning を検出 → failure line に `{dropped_calls} dropped` を含めて解消（warning 抑制ではなく診断情報として使用） |
| lint | `mise run lint-l1-real-plugin` | Pass（exit 0・`-D warnings`） |
| fmt | `mise run fmt-check` | Pass（exit 0） |

### 補助証拠（canonical ≠ mise）— `loop2.sh` concurrent loop

| phase | lane | result |
| ----- | ---- | ------ |
| pre-fix（`BIN_DIR=bin-pre N=15`） | clap/vst3-adapter・clap/vst3-testplugin | 60/60 pass（fail 0 — provenance gap は fail しない soundness hole のため、pre-fix pass は gap 未存在を意味しない。`reproduction.md` 第 2 slice 節参照） |
| post-fix（`BIN_DIR=bin-post N=15 LOAD=1`） | 同 4 lane + sibling main checkout の cargo build 負荷 | 60/60 pass — matched-only 計数化で最低到達数 gate が負荷下・実 drop 条件下でも充足。substituted/dropped 計数が gate を阻害する事象は観測されず |

### 判定（第 2 slice）

- `*_l1_full_series` の delivered provenance gap を matched-only
  分類で解消 — `passthrough == 0` が matched の必要十分条件は
  現行 counter semantics 下で確定（`ruviform#767` が semantics を
  変えた場合は分類前提の再確認が必要 — PR 本文で相互リンク）。
- 修正の正しさは canonical lane PASS（matched-only で全 test
  green）+ 構造論証 + 副次強化（matched all-zero は underrun
  免責でなく membership fail へ — oracle は弱体化でなく強化）。
- `ParamValue` event は全 call 構造再送 + plain overwrite 冪等 —
  bounded resend 不要の監査結論どおり・観測上も fail なし。
- ローカル検証＝変更範囲の検証。統合受入は merge 後の対象 SHA の
  必須 CI 結果（Coordinator が確認）。
