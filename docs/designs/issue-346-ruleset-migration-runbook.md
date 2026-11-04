---
id: "DESIGN-034RP4B7OViSh6O4YCu1rz"
type: "design"
status: "approved"
title: "Issue #346 CI-OPT-06 選択実行有効化・ruleset required context 移行 runbook"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/346"
  changes:
    added:
      - type: "file"
        path: "ci/impact/tests/workflow.test.mjs"
      - type: "file"
        path: "docs/designs/issue-346-ruleset-migration-runbook.md"
      - type: "file"
        path: "docs/impl/ir/issue-346-ci-opt-06-lane-if-ci-gate-required-runbook.md"
      - type: "file"
        path: "docs/impl/exp/issue-346-selective-enforcement.md"
      - type: "file"
        path: "docs/impl/exp/data/issue-346-enforcement-before-runs.jsonl"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/designs/issue-333-impact-based-ci.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-333-impact-based-ci.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-333-impact-based-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/impl/exp/issue-327-ci-baseline.md"
    - "docs/impl/exp/issue-333-impact-based-ci.md"
    - "docs/impl/exp/issue-346-selective-enforcement.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-333-impact-based-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "https://github.com/scarecrow173/ruviform/issues/346"
    - "https://github.com/scarecrow173/ruviform/issues/326"
  defers: []
  deferred-by: []
---

# Issue #346 CI-OPT-06 選択実行有効化・ruleset required context 移行 runbook

## Context

[issue-333-impact-based-ci.md](./issue-333-impact-based-ci.md)「段階移行
runbook」の Stage C/D を実行する操作手順書。分類契約・gate 判定表の
**正本は引き続き同書**であり、本文書は ruleset `main-protect`（id
`26529830`・`required_status_checks`・`strict_required_status_checks_policy: true`）
の required context 集合を **12 lane context → `ci-gate` 単一**へ移行するための
運用 runbook・rollback・移行中の deadlock 回避規則を与える。ruleset 操作自体は
workflow 外の管理者操作（`gh api`）で、コード側の lane `if:` 配線は本 PR で行う。

### 背景: 前倒し判断

当初 Stage B（Shadow 観測）で複数 PR 分の偽陰性 0 蓄積を移行条件としていたが、
macOS arm64 runner 逼迫で required lane が queue → cancel される実害が発生
（例: `push` run 41851785028 が attempt 3 で queued 2279s 超）。owner 判断で
有効化を前倒し。Shadow 実績:

| PR | run | decision | 偽陰性 | 証跡 |
| --- | --- | --- | --- | --- |
| #345 | 41835836190 | full | 0（not-expected lane なし） | `docs/impl/exp/data/issue-333-*-run-41835836190.json` |
| #347 | 41840536142 | selective | 0（not-expected 3 lane 全 pass） | `docs/impl/exp/data/issue-334-shadow-classification-run-41840536142.jsonl` |
| #348 | 41848239157 | selective | 0（同上） | `docs/impl/exp/data/issue-335-shadow-classification-run-41848239157.jsonl` |

蓄積は薄いが監視は有効化後も継続する（§監視）。偽陰性の実害は「必要 lane が
expected:false と判定され skip される」ことのみ — gate-report の
`not-expected-lane-*` 行と `over-executed` 監査で継続検出する。

## 移行対象の required context

| 現行 required（12） | 移行後 required（1） |
| --- | --- |
| `verify (ubuntu-latest)` / `verify (windows-latest)` / `verify (macos-latest)` | **`ci-gate`** |
| `l1-real-verify (ubuntu-latest)` / `(windows-latest)` / `(macos-latest)` | |
| `ui-verify (ubuntu-latest)` / `(windows-latest)` / `(macos-latest)` | |
| `rt-verify` / `icc-verify` / `bench` | |

`ci-gate` は全 lane の `needs` 結果 × `impact` 期待集合を照合する fail-closed
集約判定（判定表は issue-333 設計が正本）。expected+skipped・実行 lane の
failure/cancelled・分類失敗は全て Fail。`impact-classify` 自体の失敗も
gate の classification-error で Fail になるため required は単一で足りる。

## Scope

### In Scope

- lane job `if:` 配線（本 PR・コード側）。
- ruleset `main-protect` required context 移行の無保護期間なし手順（運用側）。
- 移行中 deadlock（selective PR が旧 required context を欠落）の回避・復旧規則。
- rollback 手順（即時 Full CI 復帰・恒久 revert・ruleset 逆操作）。
- 有効化後の監視規則（偽陰性・誤判定・分類エラーの検出点）。
- Before/After メトリクスの記録先（`docs/impl/exp/issue-346-selective-enforcement.md`）。

### Out of Scope

- 分類規則・gate 判定表の変更（issue-333 契約に従う。変更は別 Issue で
  分類契約を改訂し Shadow 観測をやり直す）。
- `push`/`workflow_dispatch` の Full CI 維持の確認を超える動作変更
  （両 event は classifier が常に expected=ALL を出すため lane `if:` は常に真）。
- Consumer（Synvelis/AudioLens）candidate SHA 検証との連携 — 本移行は
  独立して成立する。
- 他 workflow（`consumer-verify.yml`・`consumer-impact-check.yml`）の変更。

## ruleset 移行 runbook（無保護期間なし手順）

前提: `gh` CLI・repo admin 権限・ruleset id `26529830`（`main-protect`）。
ruleset PUT は**全量置換**のため、必ず GET した現行 payload を改変して送る。

### Phase 1 — `ci-gate` を required に追加（12 → 13）

```sh
gh api repos/scarecrow173/ruviform/rulesets/26529830 > /tmp/ruleset.json
# rules[type=required_status_checks].parameters.required_status_checks に
# { "context": "ci-gate" } を末尾追加（既存 12 件は維持）
gh api -X PUT repos/scarecrow173/ruviform/rulesets/26529830 --input /tmp/ruleset.json
```

確認:

- `gh api repos/scarecrow173/ruviform/rulesets/26529830 --jq '.rules[] | select(.type=="required_status_checks") | .parameters.required_status_checks | length'` → `13`
- 既存 open PR は `ci-gate` check を持たない限り merge 不能になる。
  `strict_required_status_checks_policy: true` により merge には base との
  同期が必須であり、同期後の run は `ci-gate` job を含む workflow で再実行
  される — 報告待ちで merge が止まるのは意図する fail-closed であり、
  open PR の対応（sync・re-run）を PR 単位で確認する。

### Phase 2 — lane `if:` 有効化 PR を merge

本 PR は `.github/**` 変更のため classifier が `decision=full` を出し、全 lane
が実行され 13 context 全て報告される（自己完結検証）。merge 時点で
13 required が全て緑であることを確認する。

### Phase 3 — 全 run で `ci-gate` 報告を確認後、旧 12 を除去

```sh
# 直近 run で ci-gate check が success で報告されていることを確認
gh api 'repos/scarecrow173/ruviform/commits/<merged-sha>/check-runs?per_page=100' \
  --jq '.check_runs[] | select(.name=="ci-gate") | {name, status, conclusion}'

# ruleset.json の required_status_checks を [{ "context": "ci-gate" }] 単一に編集
gh api -X PUT repos/scarecrow173/ruviform/rulesets/26529830 --input /tmp/ruleset.json
```

確認: 上記 length が `1` になること。**順序を逆にしないこと** — 先に 12 を
除去すると `ci-gate` 追加前の期間に required が空または片欠の無保護期間になる。

### 移行中 deadlock の扱い

Phase 2 merge 〜 Phase 3 完了の間、ruleset は `12 lane + ci-gate` のまま。
この窓内で selective 分類される PR は skip された lane の required context が
永遠に報告されず **deadlock**（`Expected — Waiting for status to be reported`）
になる。扱い:

- Phase 3 は Phase 2 merge 後**直ちに**実施し、窓を最小化する。
- 窓内で deadlock した PR は Phase 3 完了後に再 run（空 commit・sync・
  re-run）すれば lane context 待ちが解消される（ruleset 要件が変わるため
  check 要件が再評価される）。
- 復旧が急ぐ場合は `CI_IMPACT_DISABLE=1` を一時設定し PR を re-run して
  全 lane 実行させる（完了後に戻す）。

### Phase 4 — 有効化の実動確認

- docs-only 等の selective PR で `impact-decision` artifact の
  `expected:false` lane が `skipped` 結果になり、`gate-report` が
  `NotApplicable` 判定・`verdict: pass`・12 lane context 欠落でも
  `ci-gate` 単一で merge 可能であることを確認する。
- `push: main` / `workflow_dispatch` run で decision=full・全 lane 実行を
  確認する。

## 監視（有効化後の偽陰性・誤判定検出点）

| 検出点 | 意味 | 対応 |
| --- | --- | --- |
| gate-report `not-expected-lane-failure/cancelled` | **偽陰性** — skip された lane が本来 fail だった（本 PR 以降は lane 自体が skip されるため、同条件の再 run・別 PR でのみ顕在化） | 分類契約を修正し `CI_IMPACT_DISABLE=1` で即時全実行に退避 |
| gate-report `expected-lane-skipped` | lane `if:` 配線の破損（必要 lane が skip された） | 配線修正 or `CI_IMPACT_DISABLE=1` |
| gate-report `over-executed`（expected:false だが実行） | 誤判定（偽陽性） — コスト増のみ・安全性不変 | 分類規則の絞り込み修正 |
| `impact-classify` job 失敗 / `impact-decision` artifact 欠落 | 分類不能 → gate classification-error Fail・全 lane fail-open 実行 | classify.mjs 修正 |
| selective 率・runner_total 推移 | 効果測定 | §metrics（exp doc）で Before/After 比較 |

## Rollback

| 状況 | 操作 | 効果 |
| --- | --- | --- |
| 選択実行を即時無効化（分類ミス疑い・runner 逼迫解消等） | repo variable `CI_IMPACT_DISABLE=1` を設定 → 対象 run を re-run | classifier が expected=ALL を出力し全 lane `if:` が真 → 全実行。code revert 不要。戻すときは variable 削除で即時再開 |
| 恒久ロールバック（配線自体を外す） | 本 PR の revert | lane `if:` が消え Shadow 同等（`impact`/`ci-gate` 残留・gate は全 lane expected+実行を照合） |
| ruleset 逆操作 | 12 context を再追加（追加→報告確認）してから `ci-gate` を除去 | 移行前と同じ required 構成。同じ「追加→確認→除去」順序を逆方向に適用 |
| `ci-gate` 自体の撤廃 | 上記で 12 復活後、required から `ci-gate` を除去し workflow から job 削除（別 PR） | 移行前状態へ完全復帰 |

## Before/After メトリクス

計測記録は `docs/impl/exp/issue-346-selective-enforcement.md` が正本
（`ci/metrics/collect-ci-metrics.mjs` 同一手順で再現可能）。

- **Before（Shadow・全 lane 実行）**: 直近 15 run 収集 — wall p50 307s /
  p95 497s・代表 run の critical path 235s・runner_total 1474s。
- **After（選択実行有効化）**: **未実測** — merge 後の selective PR 群で
  同一手順により計測する。削減期待は「`expected:false` lane の exec 時間」で
  概算可能（例: docs-only PR なら ui-verify+l1-real+rt+icc+bench を skip →
  代表 run 換算で runner_total ~937s 削減・critical path は verify
  (windows-latest) ~220s 級へ縮小見込み。いずれも見積りであり実測ではない）。

## Edge cases

- **本 PR 自身**: `.github/workflows/ci.yml`・`ci/impact/tests/**` を含むため
  `ci-config` クラス → `decision=full`。全 lane 実行され 13 context が揃い、
  Phase 1 適用済みでも merge 可能。
- **`impact` job 失敗時**: lane は `always() && != 'false'` で全実行
  （fail-open coverage）、gate は classification-error で Fail（fail-closed
  判定）。merge は止まるが coverage は落ちない設計意図。
- **cancelled lane**: gate は expected/not-expected に関わらず
  `failure|cancelled` を Fail とする（実行済み結果は常に真の信号）。
- **PR base retarget**: `edited` type で再分類されるため stale 期待集合は
  残らない（issue-333 design の規約どおり）。
- **fork PR**: `impact`/`ci-gate` は `contents: read` のみで動作。

## Verification Notes

- [x] `ci/impact/tests/workflow.test.mjs` が lane→impact 配線・gate 集約・
      impact 無条件実行の構造不変条件を pin（`mise run test-ci-impact`）。
- [x] `mise exec -- actionlint .github/workflows/ci.yml` で構文検証。
- [x] 本 PR の classifier E2E で `decision=full`・6 lane expected を確認。
- [ ] 本 PR の CI run で 13 context（12 lane + `ci-gate`）全緑を確認
      （Phase 1 適用後は `ci-gate` が required として合流）。
- [ ] Phase 4 の selective 実動確認は merge 後の後続 PR で実施
      （docs-only PR で `NotApplicable` 判定を観測）。
