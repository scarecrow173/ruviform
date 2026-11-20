# evidence/rf768 — reproduction record（補助証拠）

`ruviform#768` — `l1_fault` / `l1_stateful` E2E staleness oracle 監査（`ruviform#763` 同 class）。
`evidence/rf763` 先例の dual-binary concurrent loop で修正前 flake 再現を試行した記録。
canonical lane は `mise run test-l1-real-plugin`（本ファイルは補助証拠）。

## 方法

- 修正前 test binary（`git worktree` HEAD `812555d8`、test file 未改変で
  `cargo test --features l1-runner --no-run` したもの）を `bin-pre/` に退避。
- `loop.sh`（本ディレクトリ）を `BIN_DIR=bin-pre N=30 FAULT_N=5 LOAD=1` で実行:
  `l1_stateful_test_plugin_e2e` 両 ABI binary を 30 iter 並走、続けて
  `l1_fault_test_plugin_e2e` 両 ABI を 5 iter 並走。LOAD lane は sibling
  main checkout（`/g/repositry/audio-products/ruviform`）での
  `cargo build --features l1-runner --tests` 反復 — `evidence/rf763/loop3.sh`
  先例どおり別 worktree を使い、実行中 binary が掴む fixture DLL を
  rebuild 対象から外した（`os error 5` 回避）。

## 結果（logs-bin-pre/）

| lane | 結果 |
| ---- | ---- |
| vst3-stateful | 30/30 pass |
| clap-stateful | 30/30 pass |
| vst3-fault | 1/5 pass（4 fail） |
| clap-fault | 3/5 pass（2 fail） |

**staleness-oracle 由来の失敗は 0 件**。観測された 6 件の fail はすべて
`real_test_fault_l1_hang_converges_on_timeout` が `instantiate` で
`SpawnFailed { cause: Timeout }` に panic する経路（logs-bin-pre
`vst3-fault-{2,3,4,5}.log`・`clap-fault-{1,3}.log`）— spawn/hello handshake
deadline が負荷下で切れる spawn 経路の flake であり、`evidence/rf763` で
同 test・同 `SpawnFailed{Timeout}`・同「負荷下のみ・再実行で回復」が
環境 flake として記録済みの **別 class**（本 slice の oracle 分類の
対象外 — residual として記録）。

修正前 stateful の単発 mutation 経路（`submit_block` 1 回 →
`wait_for_gain_output` が unclassified のまま 24 call poll）が
`dropped_submits` 発生時に**構造上必ず fail する**ことは audit で
確定したが、60 binary run（両 ABI 計）では mutation block の drop を
一度も踏めず flake は実発生しなかった。`ruviform#767` の実測
（無負荷でも ~7/44 call の周期的 drop）に対し、本環境での観測 drop
頻度は mutation 単発 block を落とすほど高くなかった — **flake
未再現・監査 finding は構造論証に基づく**、と正直に記録する。

fault 側の各 finding（substituted `Ok(CONTINUE)`/stale replay の誤計数）
は fail しない soundness gap であり loop では発生しない性質のもの —
修正後 loop の pass だけでは検証にならないため、修正の正しさは
canonical lane + 監査の構造論証で示す。

## 環境前提の補足

- attempt-1（logs-bin-pre-envfail/）は `cargo build --tests` のみで
  fixture cdylib が `target/debug/` に uplift されておらず全 iter が
  `fixture cdylib missing` assert で fail — 環境 set-up 不備として
  分類し、canonical lane の plain `cargo build -p ... --features
  l1-runner` 実行後に本記録の run を取り直した（test binary の
  `CARGO_BIN_EXE_*` 焼き付け経路自体は正しく、runner sibling に DLL
  が必要という lane 契約）。
- loop script・binary 退避・log 収集は `evidence/rf763` の形状に
  揃えた。
