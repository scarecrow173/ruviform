---
id: "TSPEC-034QdTJK0E2ZE5tkSjRLwr"
type: "test-spec"
status: "draft"
title: "consumer-verify workflow の gate 意味論（not run≠pass・preflight gating・単一SHA guard・秘密非露出）"
created: "2026-11-01"
updated: "2026-11-03"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# consumer-verify workflow の gate 意味論（not run≠pass・preflight gating・単一SHA guard・秘密非露出）

## Purpose

`consumer-verify.yml` が「未検証を緑に見せない」情報 lane として機能することを保証する。初回実走・credential 未設定時・入力誤り時の 3 ケースで観測可能な振る舞いを固定する。

## Feature

Cross-Repo 候補 revision の Consumer 互換検証 lane。

## Rules

- credential 未設定時、各 Consumer job は `not run` を job summary に出力して exit 0 する
- 全 lane が `not run` の run は summary job が workflow を失敗にする（証拠ゼロを緑にしない）
- 不正 `ruviform_rev`（非 40hex）は preflight fail → Consumer job は `fail (preflight failed)` を報告し credential 未設定とは区別する
- `Cargo.lock` の `scarecrow173/ruviform.git` source 行は (a) 1 行以上 (b) 全行 `?rev=<候補>` + `#<候補>` でなければ fail（両 lane 同一 guard）
- 秘密（Deploy Key）がログ・artifact・`git config` に出力・残留しない
- test lane 実行前に `git lfs pull` で LFS fixture が materialize される
- manifest 正規化は `rev` pin 置換・`branch` 追従除去・`rev` 未指定への注入を同一ルールで `rev = "$RUVIFORM_REV"` へ揃える（依存形態差異を guard 前に吸収し、書換 no-op は 0 行 guard で fail）
- `verify-synvelis` は workspace `Cargo.toml` 不在 ref のみ `not run (no buildable workspace)` を報告し、存在時は fetch→check→test の実 lane を実行する（presence は後方互換ガードであり lane の代替ではない）
- `verify-synvelis` は consumer 側に `.cargo/config.toml` が無いため、`CARGO_NET_GIT_FETCH_WITH_CLI`/`GIT_LFS_SKIP_SMUDGE` を job env で供給する（値は `true`/`false` 文字列）

## Examples

- Given credential 未設定、When dispatch、Then verify-audiolens/synvelis は `not run (credential not configured)`、summary は workflow 失敗
- Given `ruviform_rev=abc`、When dispatch、Then preflight fail、両 verify job は `fail (preflight failed)`
- Given 候補 SHA `41772a7…`、When audio-lens lane、Then Cargo.lock の ruviform.git source は全て同 SHA
- Given synvelis が `rev` 未指定の既定ブランチ追従 dep、When 候補 SHA `a014b3f…`、Then manifest に `rev` が注入され lock の全 15 source が `?rev=<候補>#<候補>` に解決し `mise run check`/`test` が実行される（ローカル scratch 実測済み）
- Given workspace `Cargo.toml` を持たない `synvelis_ref`、When dispatch、Then `not run (no buildable workspace)` で他 lane は走らない

## Guarantees

- `unknown`/`not run` が `pass` として表示・集計されない
- 失敗 lane でも証拠表が `GITHUB_STEP_SUMMARY` に出力される（`if: always()`）
- verify job 間に依存がなく、片側失敗が他側の実行を止めない

## Non-goals

- 実際の pass/fail 結果そのもの（credential・Consumer コードの健全性は検証対象外）
- 3 OS matrix・Golden/baseline・自動 trigger

## Risk

- これらを外すと「全 not run の緑 run」や「入力誤りが credential 問題に偽装」が起き、unknown≠pass の Issue 要件が成立しない
