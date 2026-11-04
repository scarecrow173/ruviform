# Issue #333 CI-OPT-06 Shadow Mode 観測記録（2026-11-05）

- 正本の分類・判定契約: `docs/designs/issue-333-impact-based-ci.md`（lane 行列・full fallback 規則・gate 判定表・段階移行 runbook）
- Raw 証拠: `docs/impl/exp/data/issue-333-impact-decision-run-41835836190.json` + `docs/impl/exp/data/issue-333-gate-report-run-41835836190.json`（PR #345 run の artifact 原本）
- 実装記録: `docs/impl/ir/issue-333-ci-opt-06-impact-gate.md`
- PR: https://github.com/scarecrow173/ruviform/pull/345

## 1. 観測条件

| 項目 | 値 |
| --- | --- |
| run_id | 41835836190（PR #345・pull_request 初回） |
| base / head | `f880c79`（main）/ `8306080`（merge head） |
| 変更内容 | ci.yml・mise.toml・AGENTS.md・ci/impact/ 新設・issue-333 docs 7 件 + 生成 index 6 件 |
| Shadow 状態 | lane `if:` 無変更（全 lane 実行）。`impact`/`ci-gate` は非 required |
| kill switch | `CI_IMPACT_DISABLE` 未設定（通常経路） |

## 2. impact 分類結果

`impact-classify` job: 18s・pass。decision artifact（`impact-decision`）:

| lane | expected | OS | feature | contexts |
| --- | --- | --- | --- | --- |
| verify | true | ubuntu/windows/macos | — | 3 |
| rt-verify | true | ubuntu | — | 1 |
| icc-verify | true | ubuntu | icc | 1 |
| ui-verify | true | ubuntu/windows/macos | webview | 3 |
| l1-real-verify | true | ubuntu/windows/macos | host | 3 |
| bench | true | ubuntu | — | 1 |

- `decisionReasons`: `ci-config:`（ci.yml + ci/impact/ 6 ファイル）・`shared-config:`（mise.toml）→ **full fallback 規則どおり発火**。AGENTS.md・docs/* は `docs` class に正しく分類。
- base commit fetch（`git fetch --depth=1`）→ two-dot tree diff が CI 内で正常動作。

## 3. ci-gate 判定結果

`ci-gate` job: 10s・pass。`gate-report` artifact:

- `verdict: "pass"`・`classificationOk: true`・`classificationProblems: []`
- 6 lane 全て `expected=true` / `result=success` / `verdict=Pass` / `note=""`
- needs 結果（`toJson(needs)`）経路で expected×actual 照合が end-to-end 動作。

## 4. Shadow 比較（expected vs actual）

| 観点 | 結果 |
| --- | --- |
| expected 集合 == 実行集合 | full のため一致（6 lane 全 expected・全実行・全 pass） |
| 偽陰性検出器（not-expected lane の fail 実行） | 未発動 — 本 run は full で not-expected lane なし。設計どおり |
| 12 required contexts | 全 pass（verify/l1-real-verify/ui-verify ×3OS + rt/icc/bench）— lane 無変更で契約不変 |
| expected 部分集合の実測（docs-only・単一 crate PR） | 未観測 — 今後の Shadow 期間で蓄積する偽陰性証拠の対象 |
| NotApplicable 判定の実測 | 未発動（full のため）。fixture test で contract 検証済み |
| workflow_dispatch 経路（`--all`） | 未発動 — 手動 dispatch 時に確認予定（分類コードパスは `--all` で同一 decision=full を生成） |
| `edited` type（base retarget 再分類） | 未発動 — types 追加済み、retarget 発生時に再 run される設計 |

## 5. 所要時間（本 run 実測）

| job | 時間 |
| --- | --- |
| impact-classify | 18s |
| ci-gate | 10s |
| bench | 1m6s |
| icc-verify | 28s |
| rt-verify | 1m28s |
| l1-real-verify | 38s / 41s / 1m19s（ubuntu/macos/windows） |
| verify | 1m48s / 3m35s / 4m4s（ubuntu/macos/windows） |
| ui-verify | 3m27s / 1m40s / 3m8s（ubuntu/macos/windows） |

- Shadow Mode では lane 削減なし（正しい状態）。impact+ci-gate の追加分は ~28s で critical path 外（verify windows 4m4s が最長）。
- 選択実行有効化後の効果量は docs-only 系 PR（verify のみ expected）で最大 — Shadow 期の偽陰性観測と併せて後続で実測する。

## 6. 残課題

- 部分集合 expected の実 run 証拠（docs-only/単一 crate/Ui-only 変更 PR）が揃うまで ruleset 移行・選択実行有効化は行わない（issue の段階設計）。
- `workflow_dispatch`・base retarget（`edited`）・kill switch の実動作観測も Shadow 期の確認対象。
- ruleset `main-protect` の required context 置換（12 → `ci-gate`）は design runbook Stage C/D の手順（同時並存 → 旧 context 削除の 2 段階）で無保護期間を作らず実施。
