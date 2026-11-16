# rf294 — Issue #294 consumer oracle lanes slice 検証 ledger

`consumer-verify.yml` 両 verify job への consumer oracle lane（rf628_oracle 系 test の probe→run→record）配線 slice の検証記録。実行していない検証は Pass と記載しない。doc chain: `docs/{discovery,specs,designs,plans,tasks,test-specs}/issue-294-consumer-oracle-lanes.md` + `docs/impl/ir/issue-294-consumer-oracle-lanes.md`。

## 1. 実行環境

- worktree: `G:/orca/workspaces/ruviform/rf294-consumer-oracle-lanes`（branch `feat/294-consumer-oracle-lanes`、base `303db66b`）
- OS: Windows 11（Git Bash）/ node v26.12.1（mise 管理）
- canonical 検証: `mise run` / `mise exec --` 経由。`node --test ci/` は補助証拠（canonical = mise）。

## 2. 実行した検証

| # | コマンド | 結果 | exit |
| --- | --- | --- | --- |
| 1 | `mise run test-ci-consumer-verify` | **84 pass / 0 fail / 0 skip**（`node --test ci/consumer-verify/` — evidence.test.mjs + consumer-verify-yml-contract.test.mjs。更新した CASE_SPEC_RE・rshaRows 件数（AL 9/SV 12）・oracle pin test を含む） | 0 |
| 2 | `mise exec -- actionlint .github/workflows/consumer-verify.yml` | エラー 0 | 0 |
| 3 | `mise exec -- node --test ci/`（補助・canonical=mise） | **335 pass / 0 fail** — `ci/tracker`/`ci/daily`/`ci/summary` 等 `evidence.mjs` export consumer の横断退行確認 | 0 |
| 4 | `mise exec -- prek run --all-files` | 全 hook pass（trim/eof/check-yaml/check-toml/line-ending/bom/private-key/actionlint） | 0 |

## 3. NotRun / Unsupported

- **実 runner dispatch（consumer-verify workflow_dispatch）**: NotRun — post-merge・Coordinator 管轄。本 slice は lane 配線 + 構造契約 pin のみで、oracle lane の実走証拠（pass/fail/not_run の実値・artifact JSON）は merge 後の dispatch で取得する。
- **`*-msrv`・`*-full` 系 lane**: 対象外（タスク指定どおり — Rust 変更なし・yml/JS/docs のみ）。
- **per-case (per-test fn) verdict**: Unsupported（本 slice）— consumer test が機械可読 verdict を出力しないため file/domain 粒度 lane 行に留まる。consumer repo 側 emitter は AL #127 / SV #115 後続 slice。
- **実 OS lane（Windows L2/L3・macOS/Linux L1・OS 仮想 Endpoint・実 ABI Test Plugin・GUI Runner）**: Unsupported/Deferred — RF #629/#630/#631・#313/#314 の別 gate。oracle lane は deterministic-backend OS-L1 のみで実 OS/実機・GUI の証拠ではない。
- **consumer-side GUI contract fixture**: Deferred — AL #166/#127・SV #175-177 が Owner。
- **`contract` v1→v2 bump**: Deferred — `ci/daily`・`daily-ci.yml`・docs 連動の後続 slice。本 slice は `contract: consumer-verify-tuple/v1`・schemaVersion 2 を維持。

## 4. 実装中に検出・修正した不具合

- なし（初回 `mise run test-ci-consumer-verify` で 84 pass — contract test 更新と workflow 配線が整合）。

## 5. 判断メモ

- **probe→run→record 3 分岐**: gui_candidate の RF #390 先例を踏襲。probe='1'→run step outcome の `o()` 写像、'0'→owner 付き `not_run`（AL `scarecrow173/audio-lens#127` / SV `scarecrow173/synvelis#115` — oracle test を含む consumer ref で再実行・dispatch ref/別 crate 代用の偽 pass 禁止）、else→probe outcome をそのまま写す（skipped/fail を正直記録）。
- **file 粒度 verdict**: SV の `rf628_oracle` と `rf628_oracle_rt_alloc` は別 step・別 lane 行。`--test` 併記の単一実行は coverage 同等だが file 別 verdict を埋没させるため却下（IR `metadata.experiments.rejected`）。
- **`fixture_version` 記録値**: consumer `Cargo.lock` の `ruviform-audio-test` package version を `grep -A3`+`grep -m1` で抽出し `ruviform-audio-test-<version>@<RUVIFORM_REV>` を構成。single-SHA guard が全 ruviform source を候補 SHA へ pin 済みのため SHA 部は宣言値=解決値。抽出失敗時は `FIXTURE_VER=""` → `--fixture-version` flag 省略（null）で値を捏造しない — outcome とは切り離される。
- **`rec()` spec additive 拡張**: case spec 第 1 引数を `...|<resolved_sha>[|<oracle>[|<fixture_version>]]` へ拡張（`IFS='|' read` の field 6/7、非空時のみ flag）。既存 5 field 呼出しは不変。oracle+fixture は `$RUVIFORM_REV` 同伴のペア必須として CASE_SPEC_RE で pin。oracle 行のみ `node "$EV" lane` 直接呼出しの代替案は spec/flag 対応が分散するため却下。
- **run step の依存は `fetch` 成功のみ**: `check`/`test` lane の成否に oracle verdict を連動させない（独立 oracle 判定）。`git lfs pull` は clone の LFS smudge skip を materialize するため各 run step で実行（test-full lane と同型・2 回目以降 no-op）。
- **`--expected`/`--observed` は記録しない**: consumer test が機械可読値を出力しない — 捏造禁止で null のまま。
- **docs index 未同期**: `docs/**/README.md` は例外なく未編集・未再生成 — Coordinator が統合時に再生成（PR 本文「未対応の作業」に明記）。
