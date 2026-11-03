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
- **audio-lens の `mise install` を lane 必要 tool に scope した理由（実 dispatch で判明）**: run 41604118237 で `mise install --locked`（全 tool）が `actionlint`/`pipx:serena-agent` の platform entry 不在により失敗（consumer `mise.lock` 側の gap・`--locked` は全 OS で復元不可）。lane が使うのは rust/git-lfs/node のみのため synvelis lane と同型に `mise install --locked rust git-lfs node` へ scope — install 高速化の副次効果もあり（5367ebf）。

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
- 実 dispatch 証拠（branch ref・候補 `e9e6abe`・anonymous mode・preflight `deploy_key=absent`）:
  - run [41604118237](https://github.com/scarecrow173/ruviform/actions/runs/41604118237): preflight 4s・verify-synvelis 全緑 1m42s・verify-audiolens が fetch step の全量 `mise install --locked` で失敗（consumer `mise.lock` gap）→ summary は正しく fail。
  - run [41604498852](https://github.com/scarecrow173/ruviform/actions/runs/41604498852)（scoped install 後）: preflight 5s・verify-synvelis 全緑 1m14s・verify-audiolens は check-ci・smoke 2 lane 通過のうえ `test-ci` で `al-cli --test cli` export 3 test fail を検出 → summary fail（gate の意図どおり・consumer 側潜在欠陥を検出）。
- consumer-impact-check: PR #274 で初回実トリガー・classification check pass（3s）。
- 未実施: `deploy_key` mode の SSH 経路（credential 未設定のため実走不可・既存構造は維持）、全 lane not run → 失敗ケースの実観測（片 lane fail→fail は観測済み）

## AudioLens 検出事項（consumer 側・起票案）

run 41604498852 の test-ci で `al-cli --test cli` の export 3 test（`full_pipeline_reaches_export`・`export_command_packages_a_job_without_export_in_its_plan`・`compressed_only_mp4_runs_the_default_pipeline_end_to_end`）が fail: `export file analysis/inputs/IN-*.metadata.json still embeds absolute path /tmp/…`。機序は `al-job` が input path を absolutize して manifest `source_path`/`source_path_abs` に書き込み（job.rs:60・manifest.rs:90-91）、export portability gate（al-dataset `export/package.rs`）は forbidden 側のみ `\`→`/` 正規化のため Windows では JSON 内 `\\` と不一致で blind — Linux で初めて検出可能になった潜在 portability 欠陥。ruviform 起因ではない（audio-lens の pin は既に候補 SHA と一致）。consumer 側対応（package 内 metadata の source path 相対化/除去・gate の両 separator 照合）を audio-lens 側 Issue として起票する提案を issue #219 進捗コメントに記載。

## Risks

- repo visibility が再度 private に変わると anonymous probe が false となり `not_run` に戻る — `deploy_key` mode が canonical 経路として残るため private 前提の設計は維持されるが、credential 一次設定の open blocker は引き続き未解消。
- anonymous HTTPS の GitHub 側 rate/abuse 制限は未計測（clone 3 件 + cargo fetch 程度では実害の見込みは小さい）。

## Follow-ups

- `CROSS_REPO_DEPLOY_KEY` 一次設定（owner/admin 作業・runbook Appendix）は open blocker のまま — private 復帰時の canonical 経路として必要。
- audio-lens export 3 test の consumer 側修正（上記「AudioLens 検出事項」・audio-lens #127 系の追従 Issue 起票案）と、修正後の verify-audiolens 再 dispatch による lane green 化。
- audio-lens `mise.lock` の `actionlint`/`pipx:serena-agent` platform entry gap（`--locked` 全 OS 復元不可）も consumer 側起票対象。
- 自動 trigger・required 化の判断は実走証拠の蓄積後（spec R9）。初回実走証拠は取得済みだが audio-lens lane が consumer 側欠陥で赤のため、required 昇格の前例はまだ作らない。
