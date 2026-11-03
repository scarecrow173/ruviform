---
id: "IMPL-034WjH8pQrT5vXyZkBcDeF"
type: "impl"
status: "completed"
title: "Issue #219 Cross-Repo Consumer-Verify CI Gate 本体実装（synvelis 実 lane・PR 分類機械検査・smoke 整合）"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
  changes:
    added:
      - type: "workflow"
        path: ".github/workflows/consumer-impact-check.yml"
      - type: "file"
        path: "docs/test-specs/consumer-impact-check-pr-classification.md"
      - type: "file"
        path: "docs/impl/ir/issue-219-consumer-verify-gate-body.md"
    modified:
      - type: "workflow"
        path: ".github/workflows/consumer-verify.yml"
      - type: "doc"
        path: "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
      - type: "doc"
        path: "docs/specs/issue-219-consumer-compat-contract.md"
      - type: "doc"
        path: "docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md"
      - type: "doc"
        path: "docs/discovery/issue-219-consumer-cross-repo-ci.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "../../designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
    - "../../specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../adr/cross-repo-ci-revision.md"
    - "issue-219-consumer-verify-gate.md"
    - "issue-219-ui-merged-consumer.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
    - "../../specs/issue-219-consumer-compat-contract.md"
    - "../../discovery/issue-219-consumer-cross-repo-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #219 Cross-Repo Consumer-Verify CI Gate 本体実装（synvelis 実 lane・PR 分類機械検査・smoke 整合）

## Summary

Issue #219 の残件「Cross-Repo CI gate 本体の実装」を実装した。`consumer-verify.yml` の `verify-synvelis` を presence check stub から実 lane へ拡張（manifest rev 正規化 → `cargo fetch` → 単一 SHA guard → `mise run check` → `git lfs pull` → `mise run test`）、manifest 書換を rev pin・branch 追従・rev 未指定の 3 依存形態対応に一般化して両 lane 統一、audio-lens smoke に `al-cli` ci profile build を追加して spec R5 に整合。PR 本文の Consumer 影響分類欄を機械検査する `consumer-impact-check.yml`（`pull_request`・draft 除外・非 required）を新設。credential 非依存区間は synvelis/audio-lens のローカル scratch 実測で検証済み（synvelis lane は fetch 15 crate 単一 SHA 解決・check/test 全通過）。

## Context

- 上流: `docs/specs/issue-219-consumer-compat-contract.md`（R1–R10/AC1–AC12・本 IR で AC12 を追加）・`docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`（approved・2026-11-03 追記）・`docs/adr/cross-repo-ci-revision.md`（Option A+1+i）
- 先行実装: `docs/impl/ir/issue-219-consumer-verify-gate.md`（workflow 骨格・テンプレート・completed）・`docs/impl/ir/issue-219-ui-merged-consumer.md`（UI 横断ゲート・completed）
- 前提変化: 先行 IR 時点で synvelis は「workspace 未存在のため lane 対象外」だったが、main `f7aadd0` が 5 member workspace（rev 未指定の既定ブランチ追従 dep ×10・lock `322a661` 単一解決）へ前進し実 lane 化が可能になった
- Issue #219 の残件は Issue コメント（2026-11-03）記載どおり「cross-repo CI gate 本体の実装・required 化判断」であり、required 化は spec R9 で実走後判断のため本 IR の対象外

## Implementation

### Changes

- `.github/workflows/consumer-verify.yml`
  - `verify-synvelis` を実 lane 化。workspace `Cargo.toml` 存在時: private dep probe（`url.insteadOf` + `ls-remote`）→ manifest 正規化 → `mise install --locked rust git-lfs` → `cargo fetch` → 単一 SHA guard → `mise run check` → `git lfs pull` → `mise run test`。`Cargo.toml` 不在 ref は従来どおり `not run (no buildable workspace)`（presence 判定は後方互換ガードとして残存）。synvelis に `.cargo/config.toml` が無いため `CARGO_NET_GIT_FETCH_WITH_CLI=true`・`GIT_LFS_SKIP_SMUDGE=1` を job env で供給（`true`/`false` 文字列が必須 — `1` は cargo が reject）。
  - manifest rev 書換を「`rev` pin 置換・`branch` 追従除去・`rev` 未指定への注入」を同一ルールで `rev = "$RUVIFORM_REV"` へ正規化する sed に両 lane で統一。synvelis の rev 未指定 dep も audio-lens の rev pin dep も同じ guard（`?rev=<候補>#<候補>`）で検証でき、`rev`→`branch` 形態変更にも追随する（spec Deferred の override 経路更新ギャップを解消）。
  - audio-lens smoke を spec R5 定義（`check-cli-headless` ＋ `al-cli` の `ci` profile build）に整合 — `mise exec -- cargo build -p al-cli --profile ci` を `check-cli-headless` に先行する step として追加（audio-lens 側 `ci.yml` と同一入口・証拠表は別 row）。
- `.github/workflows/consumer-impact-check.yml` — 新設。`pull_request`（`opened`/`edited`/`synchronize`/`reopened`/`ready_for_review`・main 向け・`github.event.pull_request.draft == false` で draft 除外）で PR 本文の `## Consumer 影響分類` 表を機械検査。本文は `env` 経由で渡し、HTML コメント（複数行）を perl で除去してから検査。分類セルの語彙（impacted/no impact/unknown・大小文字・太字許容）・`unknown` 使用時の `### unknown の解消先` 記入有無を検証し、未記入・語彙外・解消先なしを job fail とする。分類内容の正しさは検査しない（レビュアー human gate を代替しない）・required checks に含めない。
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md` — 「2026-11-03 追記: gate 本体実装」節を追加（lane 構成・形態吸収・`.cargo/config.toml` 補完・scoped mise install・`--locked` test・smoke 整合・PR check・ローカル実測）。v1 scope の stale 行 2 箇所に追記ポインタを補記。
- `docs/specs/issue-219-consumer-compat-contract.md` — In Scope に `consumer-impact-check.yml`、AC12（機械検査の存在・fail 条件・非 required）を追加。
- `docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md` — manifest 正規化・synvelis 実 lane・env 供給のルールと Example を追加。
- `docs/test-specs/consumer-impact-check-pr-classification.md` — 新規（PR check の Feature/Rules/Examples/Guarantees/Non-goals/Risk）。
- `AGENTS.md` — CI 節に両 workflow の実 lane 記述と credential 前提、PR check lane の補助位置づけを反映。
- `docs/discovery/issue-219-consumer-cross-repo-ci.md` — `## Evidence` に dated snapshot を append（実装内容・ローカル実測表・verified/informational/not verified の区分・本変更自身の分類）。
- `docs/impl/ir/README.md` — 手動 curated index に本 IR の行を追記。

### Decisions During Implementation

- **synvelis lane を presence stub の拡張とした**: `Cargo.toml` 不在を `not run` で返す presence 判定は、workspace 化前の ref を `synvelis_ref` に指定した場合の後方互換ガードとして残し、存在時のみ実 lane を実行する。design の「Synvelis 未 buildable を fail にしない」方針を維持したままゲート本体を実装できた。
- **manifest 正規化を両 lane で同一 sed に統一**: audio-lens 用の「`rev` 置換のみ」では synvelis の `rev` 未指定形態を処理できず、別ルールを持つと形態差が保守点になる。strip（既存 `rev`/`branch`）→ `}` 直前へ `rev` 注入の一括正規化に統一し、3 形態を単一ルールで処理した（書換 no-op は単一 SHA guard の 0 行検出で fail — silent pass にならない）。
- **`.cargo/config.toml` を consumer repo に追加せず job env で補完**: synvelis 側の恒久的変更は本 Issue scope 外（マルチ repo 運用規約）。`CARGO_NET_GIT_FETCH_WITH_CLI=true` は CI 標準の env 経路で同一設定を実現する。consumer 側 config 追加は synvelis#28 系の別対応として報告に留める。
- **`mise install` を `rust git-lfs` に scope**: synvelis lane が使うツールのみに絞り、lazy でない全 tool（apm/uv/prek 等）の install 時間と失敗面を削減（audio-lens lane は既存の full install を維持）。
- **PR 分類検査を fail とした**: 未記入でも緑にすると記入が human gate 頼みに戻る。fail することで機械的な記入強制として機能させ、required check 化は owner の branch protection 判断に委ねる（情報 lane としての誠実さは「分類内容を機械判定しない」ことで担保）。
- **`## 検証証拠` 表は検査対象外**: 自由記述の表であり、機械判定に耐える記入形式が定義されていない（test-spec の Non-goals 記載どおり）。

## Related Experiments

### Adopted

なし（実装タスク。ローカル実測は Validation 表に記録）

### Rejected

なし

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| workflow 構文 | `mise exec -- actionlint .github/workflows/consumer-verify.yml` / `consumer-impact-check.yml` | 両者 exit 0（clean） |
| synvelis lane 実測 | `f7aadd0` scratch clone で manifest 正規化 → `CARGO_NET_GIT_FETCH_WITH_CLI=true` で `cargo fetch` → 単一 SHA guard → `mise run check` → `git lfs pull` → `mise run test` | fetch: 15 crate が `?rev=a014b3f9#a014b3f9` に単一解決（total=15・bad=0）。check exit 0（10.66s）。test exit 0（23 suite・0 failed・`--locked` 整合） |
| audio-lens manifest 正規化 | `987f236` scratch で同一 sed → `cargo metadata` | 14 dep の `rev` が候補に置換（`features` 保持・TOML 有効）・lock 15 source 全て `?rev=<候補>#<候補>`（total=15・bad=0） |
| PR 分類検査スクリプト | 5 case をローカル実行（未記入テンプレート・記入済み・unknown+解消先なし・unknown+解消先あり・セクション欠落） | 全 case で pass/fail が意図どおり（複数行 HTML コメントは記入と誤判定しないことを確認） |
| yaml parse | 両 workflow・test-spec front matter | parse OK（actionlint + audit で確認） |
| doc audit | `audit_docs.js --type test-spec`（本変更の新規・修正文書） | blocking finding なし |
| 既存 ci.yml 無改変 | `git status .github/workflows/ci.yml` | 変更なし |

## 未検証（意図的・別運用）

- `consumer-verify` の dispatch 実走・SSH/credential 経路（known_hosts pin・`url.insteadOf`・runner 上 `GIT_SSH_COMMAND`）: `CROSS_REPO_DEPLOY_KEY` 一次設定が未実施のため未実走（design Appendix runbook の owner/admin 作業）。`mise-action` の runner 上挙動・`git lfs` の runner 提供も同時に初回実走で確認する。
- audio-lens lane の `check-ci`/`test-ci`/`check-cli-headless`/`al-cli` ci build/`git lfs pull` の実行: ローカル実測は manifest 正規化 + lockfile 再解決まで（先行 IR が `cargo check` まで実測済み）。task 名の実在は audio-lens `mise.toml` で確認済み。
- 全 lane not run → 失敗・preflight 失敗の真偽表: 実 dispatch でしか観測できない（test-spec で契約化済み）。
- `consumer-impact-check` の PR への初回起動: 次回 PR で観測（ローカル 5 case 検証済み）。
- synvelis `test-msrv`（rust 1.89 toolchain）・`lint`/`build` lane: v1 の lane 対象外。

## Risks

- manifest 正規化の sed は 1 行 inline-table 形態を前提とする。`git` 指定を multi-line table で書く consumer への変更が入ると書換 no-op となるが、単一 SHA guard の 0 行検出で必ず fail するため silent pass にはならない。
- `consumer-impact-check` は非 required のため、未記入 PR の fail は遮断しない。human gate と併用する運用（AGENTS.md・test-spec に明記）。
- audio-lens が `rev`→`branch` 追従形へ移行した場合、正規化は動くが証拠表の「pin を旧 SHA へ revert」表現が形態にそぐわなくなる（文言の微修正で対応可能）。
- synvelis 側で `.cargo/config.toml` が追加され `GIT_LFS_SKIP_SMUDGE`/`git-fetch-with-cli` が宣言された場合、job env との二重指定は冪等で害なし（env が config を上書きするだけ）。

## Follow-ups

- owner/admin: `CROSS_REPO_DEPLOY_KEY` 一次設定（design Appendix runbook）→ 初回 dispatch → 結果を Issue #219・discovery `## Evidence` に追記。未実走の真偽表（全 not run→失敗・preflight fail）を実観測する。
- consumer 側候補: synvelis#28 系で `.cargo/config.toml`（`net.git-fetch-with-cli` + `GIT_LFS_SKIP_SMUDGE`）追加を検討（consumer repo 側変更のため別 Issue 運用）。
- spec/ADR の status 昇格・consumer-verify の required check 化・自動 trigger は Issue Done 基準（初回両 Consumer CI 実走・追従更新・起票〜解消ループ）達成後に判断（spec R9・Deferred）。
