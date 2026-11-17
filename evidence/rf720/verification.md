# Issue #720 consumer-verify-tuple contract v1→v2 bump — 検証記録

- 対象: `scarecrow173/ruviform` Issue #720（consumer-verify-tuple
  contract v1→v2 bump）
- ブランチ: `chore/720-consumer-tuple-v2`（base: `origin/main`
  `7124f0b2`）
- 実施環境: Windows（worktree `G:\orca\workspaces\ruviform\rf720-tuple-v2`）、
  mise 管理 toolchain（node は `mise.toml` pin）
- 実施日: 2026-11-17

## 設計論点の決定（Issue 本文「設計論点」）

| 論点 | 決定 | 根拠 |
| --- | --- | --- |
| (a) 過去 run の v1 artifact の扱い | **hard cutover** — v2 のみ受理。v1 証拠は proven different tuple として `non-target`（寄与なし・retry 計上なし・verified にならない）。両受理の移行窓は設けない | `classifyRunEvidence` の contract 完全一致規則がそのまま実装。tuple の SHA 要素は日々動き両受理の実効 window は極小。bump の目的（証拠語彙拡張の tuple 識別への反映）に反して旧証拠を新 tuple の verified にしない。`--tuple-contract`/`DAILY_TUPLE_CONTRACT` が escape hatch |
| (b) `schemaVersion`/`lanes` 未検査の現状 | **現状維持** — consumer は `schemaVersion`/`lanes` を読まない。contract 文字列が producer/consumer 間の version 軸。証拠内部語彙は producer の `assertLaneRecord` が finalize 時に fail-closed 検査 | `evidence/rf294/verification.md` §5 の固定判断を踏襲。consumer が必要とするのは tuple 同一性と `consumer.status` のみ。非互換変更は contract bump で通知する規律を本 Issue が確立 |

記録箇所: `ci/daily/decide.mjs` ヘッダコメント、`evidence.mjs`
`CONTRACT` コメント、`docs/development-environment.md`
decide-consumer-verify 節、`decide.test.mjs` の hard cutover pin。

## 実行した検証（canonical = mise）

| コマンド | OS | 結果 | exit |
| --- | --- | --- | --- |
| `mise run test-ci-consumer-verify` | Windows | **Pass** — 84 pass / 0 fail | 0 |
| `mise run test-ci-daily` | Windows | **Pass** — 113 pass / 0 fail | 0 |
| `mise run test-ci-tracker` | Windows | **Pass** — 108 pass / 0 fail（downstream 非破壊） | 0 |
| `mise exec -- actionlint .github/workflows/consumer-verify.yml .github/workflows/daily-ci.yml` | Windows | **Pass** | 0 |
| `mise exec -- node --test ci/`（補助証拠・canonical=mise） | Windows | **Pass** — 354 pass / 0 fail（ci/ 横断・tracker/summary/daily 含む） | 0 |
| `node ci/consumer-verify/evidence.mjs finalize` smoke（tmp lanes JSONL） | Windows | 発行証拠 JSON の `contract`/`tuple.contract` = `consumer-verify-tuple/v2`、`status=pass` を実測 | 0 |

## 受入条件の対応（headless 範囲）

- AC1（全経路 v2 統一）: **Pass** — producer `CONTRACT`・consumer
  既定 `tupleContract`・`DAILY_TUPLE_CONTRACT`・両 workflow の
  fallback/summary/ヘッダ・4 contract test・`development-environment.md`
  が全て v2。lease 内の残存 v1 言及は下記「legacy 明示分離」のみ。
- AC2（v1 artifact 方針の明記とテスト固定）: **Pass** — hard
  cutover をコードコメント・docs・`decide.test.mjs`（legacy v1 →
  non-target fixture + `resolveConfig` 既定 v2 pin）で固定。
- AC3（canonical test pass + downstream 非破壊）: **Pass** — 上表。
- AC4（`development-environment.md` 同期）: **Pass** — v2・
  schemaVersion 2・Case schema v2 lane record の記述と hard cutover
  方針を同期。
- AC5（post-merge 実 run gate）: **NotRun** — merge 後の
  `consumer-verify` workflow_dispatch 実 run で v2 証拠 artifact 発行
  と daily `decide-consumer-verify` の contract 一致回復を確認する
  外部 gate。Coordinator 所有の post-merge dispatch で追跡（本 slice
  の headless Pass とは分離）。

## 発見して対処した main 既存破損

`mise run test-ci-consumer-verify` が本 slice の変更前から fail
していた（base worktree・HEAD `7124f0b2` で再現確認 — 2 fail）。
原因: `70b48304`（rf294 close-out の stale owner 修正）が
`.github/workflows/consumer-verify.yml` の consumer-side owner 注記を
`AL #166/#127`→`AL #127`、`SV #175-177`→`SV #176/#177` へ更新したが、
`consumer-verify-yml-contract.test.mjs:231` の regex pin `#1(66|75)` が
未追従。契約の実体は「owner 帰属を持つ not_run 行」であり issue 番号
自体ではないため、regex を `#[0-9]+` へ修復（workflow の lane 行・
owner 注記は不変 — #629/#630/#631 の実走更新に委ねる）。

## legacy `consumer-verify-tuple/v1` 言及の明示分離

AC1 の「grep で 0 件または legacy 記述箇所の明示分離」に基づき、
repo 全体の残存 v1 言及を分類する:

- **意図的な policy/legacy fixture（lease 内・変更不要）**:
  `ci/daily/decide.mjs:52`（hard cutover 説明コメント）、
  `ci/daily/tests/decide.test.mjs:872-873`（env override escape
  hatch の pin）・`:1036-1040`（legacy v1 → non-target fixture）。
- **lease 外の stale/履歴記述（変更しない）**:
  `crates/ruviform-ui-test/src/driver_lab.rs:16`（doc comment —
  verdict 語彙の説明文脈。`crates/` は編集範囲外）、
  `docs/{designs,discovery,impl,plans,specs,tasks,test-specs}/
  issue-{219,294,650}-*.md`（履歴文書の当時記述）、
  `evidence/rf288/`・`evidence/rf294/`・`evidence/rf628/`・
  `evidence/rf650/`（過去 run の証拠・検証記録）。
  これらは発行当時の事実記録であり、現行 contract の pin ではない。
