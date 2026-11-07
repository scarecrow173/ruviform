---
id: "IMPL-034ThzZO69OSZMwwEi0OLO"
type: "impl"
status: "completed"
title: "Issue #294 GUI-CONTRACT lane 接続 — 候補 tree 上の boundary guard / Fake driver semantic 評価"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/390"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-294-gui-contract-lane-wiring.md"
      - type: "file"
        path: "docs/designs/issue-294-gui-contract-lane-wiring.md"
      - type: "file"
        path: "docs/plans/issue-294-gui-contract-lane-wiring.md"
      - type: "file"
        path: "docs/tasks/issue-294-t3-gui-contract-lane-wiring.md"
      - type: "file"
        path: "docs/test-specs/issue-294-gui-contract-lane-wiring.md"
    modified:
      - type: "workflow"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "../../tasks/issue-294-t3-gui-contract-lane-wiring.md"
  implemented-by: []
  depends-on:
    - "../../designs/issue-294-gui-contract-lane-wiring.md"
    - "../../plans/issue-294-gui-contract-lane-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-294-gui-contract-lane-wiring.md"
    - "../../test-specs/issue-294-gui-contract-lane-wiring.md"
    - "../../impl/ir/issue-294-consumer-verify-tuple-evidence.md"
    - "../../specs/issue-294-cross-consumer-candidate-gate.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/issue-294-gui-contract-lane-wiring.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #294 GUI-CONTRACT lane 接続 — 候補 tree 上の boundary guard / Fake driver semantic 評価

## Summary

`consumer-verify.yml` の両 verify job（`verify-audiolens` / `verify-synvelis`）に、
RF #390 の正規 task `check-ui-driver-boundary`（GUI-CONTRACT boundary guard）と
`test-ui-driver-contract`（GUI-SEMANTIC fake layer）を**候補 commit tree**
（`RUVIFORM_REV`）上で実行する 3 step（`gui_candidate` / `gui_boundary` /
`gui_semantic`）を追加し、Record step の固定 `not_run` 行 2 本を step outcome
由来の実 lane 行へ置き換えた。`evidence.mjs`・status 導出・tuple/lock/concurrency
契約は無変更。

## Context

Task `docs/tasks/issue-294-t3-gui-contract-lane-wiring.md`（plan/design/spec:
`docs/*/issue-294-gui-contract-lane-wiring.md`）の実装。Issue #294 v2 確定仕様が
要求する「既存 Daily Full + manual 経路への GUI 契約検証接続」のうち、
実行可能になった guard と Fake semantic を接続する部分。

## Implementation

### Changes

- `.github/workflows/consumer-verify.yml`
  - ヘッダ comment に GUI contract lane の候補 tree 評価方針を追記。
  - 両 job に `gui_candidate` step を追加（`dep` step より後、
    `extra tasks` step より前）: `$RUNNER_TEMP/ruviform-candidate` へ
    `git init` + `fetch --depth 1 origin $RUVIFORM_REV` +
    `checkout --detach FETCH_HEAD` で候補 tree を取得し、
    `rev-parse HEAD` を候補 SHA と照合。候補 `mise.toml` の
    `[tasks.<name>]` 存在を grep probe して `boundary`/`contract` output を
    出力。task が 1 つでもあれば `mise trust` + `mise install --locked rust`。
  - `gui_boundary` / `gui_semantic` step を追加: `if` = 既存 lane 条件
    （synvelis は `presence == 'present'` も）+ 対応 probe output が `'1'`、
    `working-directory` は候補 dir。`gui_semantic` は
    `RUVIFORM_DRIVER_LAB_EVIDENCE` を `$RUNNER_TEMP` 配下に設定し、生成された
    JSONL を step summary へ追記してから元の exit code を返す。
  - Record step の固定 `not_run` 行 2 本（`GUI-CONTRACT checks` /
    `GUI semantic/fake-oracle checks`）を 3 分岐の lane 行へ置換:
    probe `"1"` → 実行 step outcome の `o()` 写像、`"0"` → RF #390 Owner 付き
    `not_run`（task を含む候補で再実行・dispatch ref 代用禁止）、
    その他 → `gui_candidate.outcome` + 未完了理由。Consumer 側通常
    View/Domain headless は別 `not_run` 行として保持（AL #166 / SV #175）、
    `unsupported` 固定行は無変更。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`
  - 旧「GUI-CONTRACT/GUI semantic は `not_run` 固定」assertion を新構造へ更新:
    3 step の存在と record 写像、候補 SHA fetch + rev-parse 照合、task probe、
    候補 dir `working-directory`、task 不在 `not_run` 行、
    `dep` より後の step 順序、旧行名の除去を pin。
- `docs/development-environment.md`: consumer-verify 記述の GUI lane 分を
  候補 tree 評価接続へ整合。
- chain docs（spec/design/plan/task/test-spec）新規作成 + 各 README 索引を
  `doc_maintenance.js apply` で再生成。

### Decisions During Implementation

- **一律 clone で候補 tree を取得**（dispatch ref == candidate でも workspace
  再利用なし）: `working-directory:` は step outputs を参照できないため、
  条件分岐する候補 dir では lane step の working-directory を静的に書けない。
  一律 `${{ runner.temp }}/ruviform-candidate` にすることで全経路で
  rev-parse 照合・working-directory が統一される（depth-1 fetch の追加
  コストのみ）。
- **probe 判定は候補 `mise.toml` の `[tasks.<name>]` 行 grep**: 旧候補
  （RF #390 以前）では task 未定義 → owner 付き `not_run`。dispatch ref の
  task で代用する偽 pass を構造的に排除（mise task 未存在時の実行失敗で
  `fail` 化しないため、probe で実行可否を先に分岐）。
- **`gui_semantic` は rc 捕捉 + summary 追記 + exit**: driver-lab の
  case 別 verdict JSONL を「候補 tree で評価した証拠」として Job Summary に
  残しつつ、失敗時は元の exit code で step fail にする。

## Related Experiments

### Adopted

なし（既存 workflow 構造への直接接続で、実験ログなし）。

### Rejected

なし。

## Validation

- `mise exec -- actionlint .github/workflows/consumer-verify.yml` → pass
- `mise run test-ci-consumer-verify` → 61 tests pass（新 GUI contract 構造
  test 両 job 分を含む）
- `mise run check-ui-driver-boundary` → verdict: pass（dep-graph/signature/
  allowlist 全 pass）
- `mise run test-ui-driver-contract` → 5 tests pass
- `bash -n` による全 run script 構文確認（35 blocks）
- 候補 checkout 機構のローカル実演: 現 tree SHA は probe `boundary=1
  contract=1`、pre-RF390 SHA `355b7a1` は `boundary=0 contract=0`
  （`not_run` 行へ正しく落ちる）

## Risks

- `mise install --locked rust` が候補の `mise.lock` 状態で失敗した場合は
  正直な `fail` として記録される（偽 pass 側には倒れない）。
- GUI-SEMANTIC が失敗した旧候補修正は ruviform 側の仕事であり、本 workflow
  側では step outcome をそのまま写すのみ。

## Follow-ups

- Consumer 側通常 View/Domain headless fixture（AL #166 / SV #175 Owner）が
  実装されたら `GUI-CONTRACT consumer-side` 行を実 lane 化。
- GPUI driver（#386）・実 OS（#313/#314）・実 plugin（#388/#389）lane は
  引き続き `not_run`/`unsupported` 固定行 — 各 Owner 側の実装で再分類。
