---
id: "IMPL-034R4JLReInVta6Yn0RgRW"
type: "impl"
status: "in-progress"
title: "Issue #219 consumer-verify anonymous-HTTPS access mode + 棚卸し refresh"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
  changes:
    added:
      - type: "task"
        path: "docs/tasks/issue-219-t5-anonymous-access.md"
      - type: "file"
        path: "docs/impl/ir/issue-219-consumer-verify-anonymous-https-access-mode-refresh.md"
    modified:
      - type: "workflow"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
      - type: "file"
        path: "docs/specs/issue-219-consumer-compat-contract.md"
      - type: "file"
        path: "docs/discovery/issue-219-consumer-cross-repo-ci.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-219-t5-anonymous-access.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
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

# Issue #219 consumer-verify anonymous-HTTPS access mode + 棚卸し refresh

## Summary

`consumer-verify.yml` に access mode fallback を追加した。3 repo（ruviform/audio-lens/synvelis）の public 化（2026-11-03 実測）で設計前提「全 repo private」が変化し、`CROSS_REPO_DEPLOY_KEY` 未設定のまま全 lane `not run` で証拠が滞留する構造を解消するため、secret 未設定 + 匿名 HTTPS read 可能な場合に `anonymous` mode で lane を実行する。`deploy_key` mode（SSH・known_hosts pin・`url.insteadOf`・cleanup）は private 前提の canonical 経路として維持し、secret 設定時は従来どおり優先される。合わせて棚卸しを refresh し（audio-lens pin `e9e6abe`・16 dep、synvelis 8 member・13 dep・lock `258df5a`・mise rust pin）、trigger/cadence と rollback 経路を design runbook に明記した。

## Context

- task: `docs/tasks/issue-219-t5-anonymous-access.md`
- plan: `docs/plans/issue-219-consumer-cross-repo-ci-gate.md`（completed）
- design: `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`（本変更を追記 2 件目として記録）
- spec: `docs/specs/issue-219-consumer-compat-contract.md`（R6/R7/R9 — Deferred に前提変化を追記）
- Issue: https://github.com/scarecrow173/ruviform/issues/219

## Implementation

### Changes

- `.github/workflows/consumer-verify.yml`: `preflight` に per-consumer 匿名 probe（`git ls-remote` HTTPS、consumer repo + ruviform repo の両方が必要）を追加し `anon_audiolens`/`anon_synvelis` outputs を新設。各 verify job の gate が `mode`（`deploy_key`/`anonymous`/`not_run`）を解決。SSH 鍵 install は `mode == 'deploy_key'` のみ、clone/probe は mode 別に SSH/HTTPS を切替、cleanup も `deploy_key` のみ。証拠表 header に `access:` を記録。`not run`・全 lane not run → workflow 失敗の既定は不変。
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`: 追記 2 件目 — 前提変化、access mode 表、trigger/cadence（dispatch 契機 5 項目 + 月次目安）、rollback 経路の統合記録。
- `docs/specs/issue-219-consumer-compat-contract.md`: Deferred に R6 credential 前提変化の再評価項目を追記。
- `AGENTS.md`: consumer-verify の実行前提記述を access mode 分岐に更新（T3 と同種の drift 修正）。
- `docs/discovery/issue-219-consumer-cross-repo-ci.md`: Evidence に同日 snapshot 追記（visibility・棚卸し差分・ローカル lane 実測）。

### Decisions During Implementation

- **anonymous fallback を追加する判断**: 現行構造のままでは repo public 化後も credential 一次設定待ちで証拠ゼロが継続する。anonymous HTTPS は秘密を一切持たない（最小権限の下限）ため、private 復帰に備えて deploy_key 経路を残しつつ public 期間は実走可能にした。`GITHUB_TOKEN` や Actions 標準 credential も使わず、匿名 read のみに依存。
- **probe を consumer 単位にした理由**: spec R7 の per-consumer 独立性を維持するため（片 consumer の再 private 化が他 lane を止めない）。
- **gate に `gate`（run/not_run/blocked）と `mode`（deploy_key/anonymous/not_run）の 2 output を持たせた理由**: 既存の `gate == 'run'` 条件を変えずに mode 別条件を追加でき、diff が最小で意図が明確。

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `mise exec -- actionlint .github/workflows/consumer-verify.yml` → pass
- ローカル scratch 検証（anonymous mode と同じ HTTPS 経路・候補 `e9e6abe514b8f3f0bfff2d976b8ca6551955ca09` = ruviform main HEAD）:
  - synvelis（main・8 member）: manifest 正規化 → `cargo fetch` → 単一 SHA guard（total=19・bad=0）→ `mise run check` exit 0 → `git lfs pull` + `mise run test` exit 0（全 suite 0 failed）
  - audio-lens（main・pin 同 SHA のため manifest diff は rev 値変化なしの形）: 正規化 → fetch → guard → `check-ci` → `cargo build -p al-cli --profile ci` → `check-cli-headless` → `git lfs pull` + `test-ci`（結果は discovery snapshot に記録）
- 未実施: `deploy_key` mode の SSH 経路（credential 未設定のため実走不可・既存構造は維持）、runner 上での dispatch 実走（PR で確認予定）

## Risks

- repo visibility が再度 private に変わると anonymous probe が false となり `not_run` に戻る — `deploy_key` mode が canonical 経路として残るため private 前提の設計は維持されるが、credential 一次設定の open blocker は引き続き未解消。
- anonymous HTTPS の GitHub 側 rate/abuse 制限は未計測（clone 3 件 + cargo fetch 程度では実害の見込みは小さい）。

## Follow-ups

- `CROSS_REPO_DEPLOY_KEY` 一次設定（owner/admin 作業・runbook Appendix）は open blocker のまま — private 復帰時の canonical 経路として必要。
- 実 dispatch（workflow_dispatch・anonymous mode）の runner 証拠を取得後、discovery `## Evidence` に追記。
- 自動 trigger・required 化の判断は初回実走証拠の後（spec R9）。
