# rf294 — Issue #294 first slice（Case schema v2）検証 ledger

`ci/consumer-verify/` の証拠契約を virtual-first Case schema へ additive 拡張した slice の検証記録。実行していない検証は Pass と記載しない。doc chain: `docs/{discovery,specs,designs,plans,tasks,test-specs}/issue-294-virtual-first-evidence-schema.md` + `docs/impl/ir/issue-294-virtual-first-evidence-schema.md`。

## 1. 実行環境

- worktree: `G:/orca/workspaces/ruviform/rf294-evidence-schema`（branch `scarecrow173/rf294-evidence-schema`）
- OS: Windows 11（Git Bash）/ node 26.12.1（mise 管理）
- canonical 検証: `mise run` / `mise exec --` 経由。`node --test ci/` は補助証拠（canonical = mise）。

## 2. 実行した検証

| # | コマンド | 結果 | exit |
| --- | --- | --- | --- |
| 1 | `mise run test-ci-consumer-verify` | **82 pass / 0 fail / 0 skip**（`node --test ci/consumer-verify/` — evidence.test.mjs + consumer-verify-yml-contract.test.mjs。schema v2 pin・必須 4 項目・enum 閉集合・requirement×outcome 直交・optional 型/SHA 検証・v1 行拒否・yml Case spec 形状・非既定分類 pin を含む） | 0 |
| 2 | `mise exec -- actionlint .github/workflows/consumer-verify.yml` | エラー 0 | 0 |
| 3 | `mise exec -- prek run --all-files` | 全 hook pass（trim/eof/check-yaml/check-toml/line-ending/bom/private-key/actionlint） | 0 |
| 4 | `node --test ci/`（補助・canonical=mise） | **333 pass / 0 fail** — `ci/tracker` の `deriveStatus`/`OUTCOMES` import contract・`ci/daily` decide 系を含む downstream consumer 非破壊 | 0 |
| 5 | docs index 再生成 | `doc_maintenance.js index apply --dir docs/{discovery,specs,designs,plans,tasks,test-specs}` 各 1 written・`sync_impl_index.js --kind ir --apply` 1 change（`--kind exp` は in-sync・変更不要） | 0 |

## 3. NotRun / Unsupported

- **実 runner dispatch（consumer-verify workflow_dispatch）**: NotRun — post-merge・Coordinator 管轄。本 slice は schema/記録層のみで lane 実行内容・回数は不変。v2 証拠 artifact の実走確認は merge 後の dispatch で行う。
- **`*-msrv`・`*-full` 系 lane**: 対象外（タスク指定どおり — Rust 変更なし・JS/workflow/docs のみ）。
- **GUI-L2/EXT-L3/physical-device の実測**: Unsupported（既存どおり — ubuntu headless CI では capability 不足。`unsupported` 固定行の outcome・理由は不変で、Case 分類のみ正直に宣言）。

## 4. 実装中に検出・修正した不具合

- `OPTIONAL_LANE_FIELDS` を snake_case で定義した初版は、`makeLaneRecord` の rest 引数（camelCase option 名）と不一致となり `lane` CLI が全 optional flag を `unknown option` で拒否 → `mise run test-ci-consumer-verify` が即 fail で検出。camelCase 定義へ修正し全 optional flag の snake_case serialize を test で pin。

## 5. 判断メモ

- `contract` は `consumer-verify-tuple/v1` 固定: `ci/daily/decide.mjs` が contract 文字列一致で証拠 artifact を回復する downstream consumer であり、`schemaVersion`/`lanes` は未検査（コード確認済み）。contract bump は `ci/daily`+`daily-ci.yml`+`docs/development-environment.md` へ波及し本 slice scope 外。
- `outcome` は rename せず `requirement` を別軸フィールドとして追加: `ci/tracker` contract test が `outcome` 名と 5 値語彙を pin（`node --test ci/` 333 pass で非破壊を実測）。
- `requirement`×`outcome` 直交: `NotApplicable`+`fail` 等の組合せを test で許容 pin — 要求度と結果を 1 項目へ畳み込まない（#627 contract §2）。
- workflow の全記録経路を同時更新: `rec()` spec 化・extra-task/tuple 直接 lane・fallback `printf`（`schemaVersion: 2`）。`set -uo pipefail`（`-e` なし）下で `lane` 失敗は当該行欠落に留まり pass 水増しにならない — spec 形状は yml contract test が pin。
- `docs/development-environment.md` の `consumer-verify-tuple/v1` 証拠契約記述は別 Worker リースのため未編集 — `schemaVersion` 2・lane Case 項目の後続同期が必要（PR 本文に記録）。
