---
id: "TSPEC-qrIvFGA7o1j1g1gLAH1gW2"
type: "test-spec"
status: "approved"
title: "consumer-verify GUI-CONTRACT lane — 候補 tree 評価・task probe・別結果記録の構造契約"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-294-consumer-verify-tuple-evidence.md"
    - "docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
    - "docs/plans/issue-294-gui-contract-lane-wiring.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# GUI-CONTRACT lane 接続の構造契約（Feature / Rule / Example）

## Feature: 候補 tree での guard / semantic 実行

- **Rule**: lane step は `RUVIFORM_REV` の tree で実行される。
  **Example**: 専用 scratch dir への `git fetch --depth 1 origin "$RUVIFORM_REV"`
  + `checkout --detach FETCH_HEAD` + `rev-parse HEAD` 照合。
- **Rule**: lane step の working-directory は候補 scratch dir
  （`${{ runner.temp }}/ruviform-candidate` — 一律 clone で固定）。
- **Rule**: `actions/checkout` の `ref:` へ候補 SHA を渡さない
  （evidence.mjs 供給は dispatch ref — 既存不変条件の維持）。

## Feature: task 存在 probe と正直な not_run

- **Rule**: 候補 mise.toml の `[tasks.<name>]` 存在を grep し、存在 task のみ
  実行。**Example**: `[tasks.check-ui-driver-boundary]` /
  `[tasks.test-ui-driver-contract]`。
- **Rule**: task 不在時は owner（RF #390）+ 再実行条件付き `not_run`、
  dispatch ref の task で代用しない。

## Feature: 別 command / 別結果

- **Rule**: boundary guard と fake semantic は別 step・別 lane 行。
  **Example**: `steps.gui_boundary.outcome` と `steps.gui_semantic.outcome`
  が別々の rec 行に写像される。
- **Rule**: `unsupported` 固定行（visual/OS、macOS、PLUGIN-NATIVE-DOCK）と
  Consumer 側 `not_run` 行（AL #166 / SV #175）は維持。
