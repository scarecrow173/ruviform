---
id: "TASK-034QdV9z3ctYdM4HMD0MsZ"
type: "task"
status: "done"
title: "T2 consumer-verify.yml Cross-Repo CI lane（preflight/verify×2/summary）"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md"
  references: []
  defers: []
  deferred-by: []
---

# T2 consumer-verify.yml Cross-Repo CI lane（preflight/verify×2/summary）

## Work

`.github/workflows/consumer-verify.yml` を新設する。design §Components の 4 job 構造をそのまま実装する。
- `workflow_dispatch` inputs: `ruviform_rev`（空・job env で `inputs.ruviform_rev || github.sha` 解決）・`audiolens_ref`/`synvelis_ref`（main・branch/tag 名のみ検証）
- `preflight`: secrets は step env 経由で `[ -n "$KEY" ]` 判定 → output `credential_configured`。`RUVIFORM_REV` 40hex・ref 形式を検証し不正は job fail
- `verify-audiolens`（`needs: [preflight]`・`if: always()`）: gating step が `needs.preflight.result` を検査（fail→`fail (preflight failed)` exit 1）。credential 未設定→`not run` exit 0。設定時: RUNNER_TEMP 鍵・ハードコード fingerprint pin の known_hosts・`GIT_SSH_COMMAND` → SSH clone（depth1・`GIT_LFS_SKIP_SMUDGE=1`）→ `url.insteadOf`（`.git` suffix 付き）→ manifest rev 一括書換 → `cargo fetch`（非 locked）→ repo URL キーの単一 SHA guard → `mise install` → `check-ci`/`check-cli-headless`/`test-ci`（test 前に `git lfs pull`）→ `if: always()` の証拠表 summary + job `outputs.status`
- `verify-synvelis`: 同構造。workspace `Cargo.toml` 非存在 → `not run (no buildable workspace)`
- `summary`（`if: always()`・全 job needs）: outputs.status + result 集約。fail/cancelled→workflow 失敗、全 lane not run→workflow 失敗（証拠ゼロ）、1 lane 以上 pass かつ fail 無し→成功
- cleanup（`if: always()`・job 末尾）: `url.insteadOf` 除去→鍵削除。cleanup 後 git/ssh 操作禁止
- permissions: `contents: read`

## Classification

- Type: implementation
- Size: L

## Upstream Documents

- `docs/plans/issue-219-consumer-cross-repo-ci-gate.md`
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`

## Dependencies

なし（T1 と独立）

## Affected Files

- `.github/workflows/consumer-verify.yml`

## Done When

- design の gating/guard/真偽表/credential パターンを逸脱なく実装
- review 確定修正（`.git` suffix・`secrets` の if 不使用・`inputs.default` 非使用・`if: always()` summary・outputs.status・LFS pull・単一 SHA URL 照合）が全て反映
- `actionlint` pass

## Verification

- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml`
- [ ] design §Components との 1 対 1 照合
