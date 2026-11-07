---
id: "SPEC-7DGwosr0bGXXRsfeRfhe1v"
type: "spec"
status: "approved"
title: "Issue #294 GUI-CONTRACT lane 接続 — ruviform 候補 tree 上の boundary guard / Fake driver semantic を consumer-verify へ"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-390-driver-contract-lab.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-294-gui-contract-lane-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
    - "https://github.com/scarecrow173/synvelis/issues/175"
  defers: []
  deferred-by: []
---

# Issue #294 GUI-CONTRACT lane 接続（ruviform 候補 tree 評価）

## 背景

Issue #294 の 2026-11-07 横断レビュー確定仕様 v2 は `GUI-CONTRACT`（GPUI なしで
Ruviform common + 両 Consumer 通常 ViewSpec/headless が build/test でき、
public API に GPUI/Toolkit 型が露出せず禁止依存と allowlisted composition root
を検査）と `GUI-SEMANTIC`（Fake Driver + GPUI Driver の共有 fixture）を
**既存 Daily Full + manual 経路**へ接続することを要求する。RF #390 統合により
`mise run check-ui-driver-boundary`（boundary guard）と
`mise run test-ui-driver-contract`（UI-DVR-01/05/07/08 Fake layer fixture）が
ruviform 側の正規入口として実行可能になった。

現行 `consumer-verify.yml` は両 Consumer job の Record step で
`GUI-CONTRACT checks` / `GUI semantic/fake-oracle checks` を**固定 `not_run` 行**
として記録している。本 spec は、この固定行のうち実行可能な部分（ruviform 候補
tree 上の boundary guard と Fake driver semantic）を実 lane 化する変更を定める。

## 要求

### R1 候補 tree 評価の分離

- guard / semantic は `inputs.ruviform_rev || github.sha` で決まる**候補
  commit tree**上で実行する。evidence.mjs・workflow 定義を供給する dispatch ref
  checkout と混同しない（workflow ヘッダの「検証対象と検証器の revision を混ぜ
  ない」方針の拡張）。
- 候補 tree は Consumer clone と同一の
  `git init` + `fetch --depth 1 origin $RUVIFORM_REV` + `checkout --detach FETCH_HEAD`
  統一経路で専用 scratch dir へ取得し、`rev-parse HEAD` が候補 SHA と一致する
  ことを確認してから評価する。dispatch ref checkout（workspace）の HEAD が
  候補 SHA と一致しても同一 SHA の別 checkout として扱う
  （`working-directory:` が step outputs を参照できない制約から、
  実装は一律 clone で統一する）。

### R2 task 存在性の正直な判定

- 候補 tree の `mise.toml` に `[tasks.check-ui-driver-boundary]` /
  `[tasks.test-ui-driver-contract]` が存在するか probe し、存在する task のみを
  実行する。
- 候補に task がない（RF #390 merge より古い候補）場合は owner（RF #390）と
  再実行条件付き `not_run` を記録する。**dispatch ref の task で代用して
  `pass` にしない。**

### R3 別 command / 別結果

- boundary guard と Fake driver semantic は別 step・別 lane 行として記録する
  （guard 成功 ≠ semantic 成功 ≠ 実 Native/OS 成功）。
- lane 行の outcome は step outcome の `o()` 写像（success→pass、
  failure/cancelled→fail、skipped→skipped、その他→not_run）に従う。

### R4 未接続 lane の維持

- Consumer 側通常 View/Domain headless の GUI-CONTRACT 相当（AL #166 /
  SV #175 Owner）は未実装のため owner + 再実行条件付き `not_run` 行を維持する。
- `GUI visual/OS + 実 window lanes`、`macOS/実 OS lanes`、
  `PLUGIN-NATIVE-DOCK (実 plugin binary)` の `unsupported` 固定行は変更しない。

### R5 既存契約の不変

- `workflow_dispatch` 専用 trigger、concurrency tuple key、
  `cancel-in-progress: false`、expected SHA fail-closed 照合、extra_tasks の
  pattern/最大 8 件検証、同一 tuple 抑止、全終端経路の証拠 artifact、
  not-run-is-not-pass 集約は変更しない。
- 新 workflow・PR required gate・PR-push trigger・impact 選択・証跡 DB・
  新 runner framework は追加しない。
- Consumer repo 側の source には触れない。

## Acceptance

- [ ] 両 verify job が候補 tree 上で `check-ui-driver-boundary` /
      `test-ui-driver-contract` を実行し、step outcome 由来の lane 行が
      evidence JSON / Job Summary に別行で記録される。
- [ ] 候補に task がない場合 owner 付き `not_run` が記録され、dispatch ref
      代用の `pass` が構造的に起きない。
- [ ] `mise run test-ci-consumer-verify`（workflow 構造契約 test 含む）と
      `mise exec -- actionlint .github/workflows/consumer-verify.yml` が pass。
- [ ] `driver-lab` の実行 evidence（`RUVIFORM_DRIVER_LAB_EVIDENCE`）が
      step summary に記録される。
- [ ] visual/OS/macOS/plugin の `unsupported` 行と Consumer 側
      `not_run` 行が保持される。
