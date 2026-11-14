# Issue #650 検証証拠 — bench-smoke lane coverage 拡張

- 対象: `scarecrow173/rf650-bench-smoke` @ `d5aa1b4b`（= `origin/main`）
- 実施日: 2026-11-15
- 環境: Windows 11 x64・rustc/cargo 1.99.1 (58cf8f9c3)・mise
  2026.11.2 windows-x64・node v26.12.1

## 変更内容

- `mise.toml` `[tasks.bench-smoke]`: `run` を
  `cargo bench -p ruviform-math -p ruviform-rt -p ruviform-viz
  -p ruviform-numeric -p ruviform-io-e2e -- --test` へ拡張し、
  `description` を同じ 5 package 列挙へ更新。
- `docs/development-environment.md`:
  - L21 `bench-smoke` 行のコマンド列・説明を新列挙へ追随
    （説明は「`bench` と同じ対象」参照形）。
  - L89 `test-ci-consumer-verify` 詳細行へ Case schema v2 の
    最小追記（consumer-verify audit の drift 修正）。

## bench target 実在の再確認（Worker 側）

- `crates/ruviform-numeric/Cargo.toml:25` — `[[bench]]` +
  `benches/corr.rs`（Issue #265）
- `crates/ruviform-io-e2e/Cargo.toml:64` — `[[bench]]` +
  `benches/io_composition.rs`（Issue #306 G05）

両 crate とも bench target が実在するため `-p` 追加が正しい結論。

## 検証結果（全て実測）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run bench-smoke` | 0 | Pass — 5 bench target 全て実行 |
| `mise exec -- cargo bench --workspace --no-run` | 0 | Pass（Issue AC 由来の canonical 指定・補助証拠） |
| `mise run fmt-check` | 0 | Pass |
| `mise exec -- prek run --all-files` | 0 | Pass |
| `mise tasks` | 0 | Pass — `bench-smoke` description が 5 package 列挙を含む |

### `mise run bench-smoke` 実行行抜粋

`evidence/rf650/bench-smoke-excerpt.txt` に抜粋を保存。主要行:

```
153:     Running benches\io_composition.rs (target\release\deps\io_composition-bdd7996e1b636551.exe)
210:     Running benches\simd_poc.rs (target\release\deps\simd_poc-55b9cff17777beff.exe)
2724:     Running benches\corr.rs (target\release\deps\corr-f0bdfd20cecdc86b.exe)
2835:     Running benches\rt_observability_poc.rs (target\release\deps\rt_observability_poc-d837b25c5d91c7bd.exe)
2947:     Running benches\viz.rs (target\release\deps\viz-b37eb7fbe13d2209.exe)
```

- `io_composition`（ruviform-io-e2e）: `Testing
  io_composition/staged_write|staged_seal|staged_publish|
  staged_readback|tar_gzip_staged|tar_zstd_staged` 全て `Success`
- `corr`（ruviform-numeric）: `Testing fft_len/...` 各ケース
  `Success`
- `simd_poc`（ruviform-math）・`rt_observability_poc`
  （ruviform-rt）・`viz`（ruviform-viz）も従来どおり実行

### `mise tasks` 出力 snippet（description 更新の証拠）

```
bench-smoke  Smoke-test the ruviform-math, ruviform-rt, ruviform-viz, ruviform-numeric and ruviform-io-e2e benchmarks with `cargo bench -p ruviform-math -p ruviform-rt -p ruviform-viz -p ruviform-numeric -p ruviform-io-e2e -- --test` (bench profile; runs each bench once without measurement; the CI bench job calls this; writes only target/)
```

### NotRun

- `*-msrv` 系・`*-full` 系・3 OS 横断 — 本 slice の必須検証対象外
  （Daily Full CI 委譲。ユーザー指定どおり未実行）。
- bench 実計測 — smoke lane は `-- --test`（計測なし）であり、
  計測値は受入証拠にしない（`mise run bench` は全量計測入口・
  手動実行向け）。

## consumer-verify audit（Refs #294 residual）

照合先: `ci/consumer-verify/evidence.mjs`
（`SCHEMA_VERSION = 2`・`LEVELS = ["OS-L1","GUI-L2","EXT-L3"]`・
`ENVIRONMENT_KINDS` 5 値 closed set・`REQUIREMENTS =
["Required","Deferred","NotApplicable"]`・`makeLaneRecord`/
`assertLaneRecord` の lane record field・`deriveStatus` 行列）と
`ci/consumer-verify/tests/evidence.test.mjs` の fixture test、
および doc chain `docs/specs/issue-294-virtual-first-evidence-schema.md`。

| 箇所 | 判定 | 根拠 |
| --- | --- | --- |
| L28 `test-ci-consumer-verify` task 表行 | no drift | 「lane 記録」「tuple 証拠 JSON/summary 形状」は汎用表現で、Case schema v2 を含む lane record を否定・不正確化しない。表の粒度では追記不要 |
| L89 `test-ci-consumer-verify` 詳細行 | **drift → 最小修正** | contract test 内訳の詳細列挙が Case schema v2 field（`case_id`/`level`/`environment_kind`/`requirement` + optional oracle/fixture/`resolved_ruviform_sha`、requirement/outcome 直交、v1 row 拒否）に未言及。`evidence.test.mjs` がこれらを pin しているため同じ粒度で追記。`schemaVersion: 2` の値も明記 |
| L116/L123 `decide-consumer-verify` | no drift | 証拠 JSON から復元する tuple key（`tuple.ruviformRev`/`tuple.consumerResolvedSha`/`tuple.contract`/`consumer.{name,repo}`/`run.runId`/`status`）は schemaVersion 2 でも不変で実在（`buildEvidence` で確認） |
| L124 `consumer-verify.yml` | no drift | lane 結果語彙 pass/fail/not_run/unsupported/skipped = `OUTCOMES`、Consumer status pass/fail/not_run/unsupported = `deriveStatus` と一致。「証拠 JSON（`contract: consumer-verify-tuple/v1`）」は正確（contract は v1 のまま、schemaVersion とは別軸）。同節は lane record field を列挙する粒度ではない |
| L126-127 `ci-issue-tracker` scope | no drift | Issue 集約の scope 記述であり schema v2 の影響を受けない |

修正は L89 の 1 箇所のみ。監査対象節の全面書き換え・再構成は
行っていない（scope 外）。

## review-protocol

config + docs の小 slice のため proportionate な 1 cycle の
self-review を実施: diff 全体（`mise.toml` 2 行・dev-env doc 2 箇所・
新規 doc chain・evidence）を通読し、(a) `run` の `-p` 列挙が
`description`・dev-env doc・`mise tasks` 出力と一致、(b) `-- --test`
と smoke 性質の維持、(c) L89 追記が `evidence.mjs`/`evidence.test.mjs`
の語彙と一致、(d) 範囲外ファイルの変更なし、を確認。blocker 0。
