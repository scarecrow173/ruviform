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
        path: "ci/impact/observe.mjs"
      - type: "file"
        path: "ci/impact/tests/observe.test.mjs"
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
        path: "ci/impact/map.mjs"
      - type: "file"
        path: "ci/impact/lib.mjs"
      - type: "file"
        path: "ci/impact/gate.mjs"
      - type: "file"
        path: "ci/impact/tests/gate.test.mjs"
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
**正本は引き続き同書**であり、本文書は ruleset `main-checkstatus`（id
`26547674`・`required_status_checks`）の required context 集合を
**9 lane context → `ci-gate` 単一**へ移行するための運用 runbook・
rollback・移行中の deadlock 回避規則を与える。ruleset 操作自体は
workflow 外の管理者操作（`gh api`）で、コード側の lane `if:` 配線と
macOS leg の物理分離は本 PR で行う。

### 改訂記録

- **2026-11-05 改訂（Issue #333/#326 合意・本件が旧 3OS 必須記述に優先）**:
  macOS leg（`verify`/`ui-verify`/`l1-real-verify`）を PR Required から
  物理分離し、非ブロッキング observe job + `macos-report` 記録に移行。
  ruleset の required は既に 3 macOS context を除去済みの 9 件へ改訂済み
  （coordinator 実施済み）。移行計画は「9 個別 → `ci-gate` 単一」に改訂。
  旧版の「12 context」記述は本節の改訂に置き換わる。

### 背景: 前倒し判断

当初 Stage B（Shadow 観測）で複数 PR 分の偽陰性 0 蓄積を移行条件としていたが、
macOS arm64 runner 逼迫で required lane が queue → cancel される実害が発生
（例: `push` run 41851785028 が attempt 3 で queued 2279s 超。本 PR の CI
run 41863215849 でも macOS leg が queue 約 17–25 分で 3 連続 cancel された
実績あり — 実害は移行検討中も進行中）。owner 判断で有効化を前倒し。
Shadow 実績:

| PR | run | decision | 偽陰性 | 証跡 |
| --- | --- | --- | --- | --- |
| #345 | 41835836190 | full | 0（not-expected lane なし） | `docs/impl/exp/data/issue-333-*-run-41835836190.json` |
| #347 | 41840536142 | selective | 0（not-expected 3 lane 全 pass） | `docs/impl/exp/data/issue-334-shadow-classification-run-41840536142.jsonl` |
| #348 | 41848239157 | selective | 0（同上） | `docs/impl/exp/data/issue-335-shadow-classification-run-41848239157.jsonl` |

蓄積は薄いが監視は有効化後も継続する（§監視）。偽陰性の実害は「必要 lane が
expected:false と判定され skip される」ことのみ — gate-report の
`not-expected-lane-*` 行と `over-executed` 監査で継続検出する。

## 現行構成（改訂後）

### PR Required（ruleset 必須・`ci-gate` 照合対象）

| lane | required contexts（9） |
| --- | --- |
| `verify` | `verify (ubuntu-latest)` / `verify (windows-latest)` |
| `l1-real-verify` | `l1-real-verify (ubuntu-latest)` / `l1-real-verify (windows-latest)` |
| `ui-verify` | `ui-verify (ubuntu-latest)` / `ui-verify (windows-latest)` |
| `rt-verify` / `icc-verify` / `bench` | `rt-verify` / `icc-verify` / `bench` |

### 非ブロッキング observe（macOS・ruleset 非必須）

| observe job | parent lane | 記録先 |
| --- | --- | --- |
| `verify-macos` | `verify` | `macos-report`（summary + `macos-observe-report` artifact） |
| `ui-verify-macos` | `ui-verify` | 同上 |
| `l1-real-verify-macos` | `l1-real-verify` | 同上 |

設計規約（Issue #333/#326 rev 2026-11-05）:

- **物理分離必須**: macOS leg は `*-macos` の別 job。`ci-gate` の `needs`
  には存在ごと含めない。`continue-on-error`・結果無視・ruleset 側での
  緩和は代替にならない（job 単位の `needs` は matrix 内の leg を分離
  できないため）。
- macOS leg は PR/main で継続実行され、結果は `macos-report`
  （`if: always()`・non-blocking）が OS 別に `Pass` / `Fail` /
  `NotRun` / `Pending` / `NotApplicable` / `Unsupported` で記録する。
  **runner 逼迫（cancelled）は `NotRun` であり `NotApplicable` にしない**
  — `NotApplicable` は classifier が「影響なし」を証明した skip のみ。
  判定器が未定（分類失敗）で leg が未実行の場合は `Pending`。
- 判定器の期待集合は lane 単位（6 lane）のまま。PR Required legs
  （Linux/Windows）と非ブロッキング macOS legs・Full/Release Gate は
  消費側（`ci-gate` vs `macos-report`）で明確分離される。分類不能時は
  required legs が Full fail-closed（gate 全 expected → 全 required lane
  実行要求）に倒れ、macOS legs は `Pending` 記録。
- 選択実行の `if:` は macOS legs にも同じ `lane_*` 出力を適用する
  （expected:false の lane は macOS leg も skip → `NotApplicable` 記録）。

### `ci-gate` 判定（Required 側・fail-closed・issue-333 契約どおり）

expected+skipped・実行 lane の failure/cancelled・分類失敗は全て Fail。
`*-macos` job は `needs` に無いため評価対象外 — runner 逼迫による
cancelled や長時間 queued が gate を待たせることはない。
`impact-classify` 自体の失敗も gate の classification-error で Fail に
なるため required は単一で足りる。

### シナリオ契約（テスト pin 済み）

| scenario | 入力 | Required gate | macos-report |
| --- | --- | --- | --- |
| (a) macOS 長時間 queued → cancelled | `*-macos` leg `cancelled` | **pass**（needs 外） | `NotRun`・verdict `attention`（expected 時） |
| (b) macOS 実失敗 | `*-macos` leg `failure` | **pass**（needs 外） | `Fail`・verdict `attention` |
| (c) Linux/Windows leg 失敗 | required lane `failure`/`cancelled`/`skipped` | **fail** | 影響なし |
| (d) 全成功 | 全 legs `success` | pass | 全 `Pass`・verdict `pass` |
| (e) 分類不能 | `impact` 失敗 / expected 欠落 | **fail**（classification-error・required Full fail-closed） | 未実行 leg `Pending`・verdict `attention` |

pin 先: `ci/impact/tests/gate.test.mjs`・`ci/impact/tests/observe.test.mjs`・
`ci/impact/tests/workflow.test.mjs`（構造不変条件: required matrix は
ubuntu+windows のみ・`ci-gate` needs に macOS job を含まない・
`macos-report` needs = impact + 3 observe job）。

## Release・Consumer 受入での macOS 実測要件

PR の `ci-gate` は **Release 合格判定に再利用しない**。macOS legs は PR
必須ではなく observe 記録のため、リリース・正式 Consumer 受入では
**候補 SHA に紐づいた macOS 実測（`macos-observe-report` artifact の
`legs[].status == "Pass"`）を要求**する運用とする:

- `macos-observe-report` artifact は `base`/`head`/`event`/`verdict`/
  `legs[]` を含む JSON（`schemaVersion: 1`・`kind: "macos-observe"`）で
  永続化される（retention は artifact 既定）。
- `attention` verdict または leg `Fail`/`Pending`/`NotRun`（expected 時）
  は macOS 実測未達を意味し、候補 SHA の受入判定では不合格扱い。
- 取得経路: `gh api repos/.../actions/runs/<run>/artifacts` →
  `macos-observe-report`。`NotRun` 多発時は対象 run の `*-macos` job を
  re-run して実測を得る（非必須のため PR merge は阻害しない）。

## 移行対象の required context

| 現行 required（9・ruleset `main-checkstatus`） | 移行後 required（1） |
| --- | --- |
| `verify (ubuntu-latest)` / `verify (windows-latest)` | **`ci-gate`** |
| `l1-real-verify (ubuntu-latest)` / `(windows-latest)` | |
| `ui-verify (ubuntu-latest)` / `(windows-latest)` | |
| `rt-verify` / `icc-verify` / `bench` | |

`macos-report`・`*-macos` 各 job・`impact-classify` は ruleset 必須に
しない（`impact-classify` 失敗は `ci-gate` classification-error Fail に
内包される）。

## Scope

### In Scope

- lane job `if:` 配線 + macOS leg の `*-macos` job 物理分離 + `macos-report`
  新設（本 PR・コード側）。
- ruleset `main-checkstatus` required context 移行の無保護期間なし手順（運用側）。
- 移行中 deadlock（selective PR が旧 required context を欠落）の回避・復旧規則。
- rollback 手順（即時 Full CI 復帰・恒久 revert・ruleset 逆操作）。
- 有効化後の監視規則（偽陰性・誤判定・分類エラー・macOS 実測欠落の検出点）。
- Before/After メトリクスの記録先（`docs/impl/exp/issue-346-selective-enforcement.md`）。

### Out of Scope

- 分類規則・gate 判定表の変更（issue-333 契約に従う。変更は別 Issue で
  分類契約を改訂し Shadow 観測をやり直す）。
- `push`/`workflow_dispatch` の Full CI 維持の確認を超える動作変更
  （両 event は classifier が常に expected=ALL を出すため lane `if:` は常に真）。
- Consumer（Synvelis/AudioLens）candidate SHA 検証 workflow の変更 — 本移行は
  独立して成立する（macOS 実測の受入要件は上記運用規約の範囲）。
- 他 workflow（`consumer-verify.yml`・`consumer-impact-check.yml`）の変更。

## ruleset 移行 runbook（無保護期間なし手順）

前提: `gh` CLI・repo admin 権限・required context の正本は ruleset
`main-checkstatus`（id `26547674`）。ruleset PUT は**全量置換**のため、
必ず GET した現行 payload を改変して送る。

### Phase 1 — `ci-gate` を required に追加（9 → 10）

```sh
gh api repos/scarecrow173/ruviform/rulesets/26547674 > /tmp/ruleset.json
# rules[type=required_status_checks].parameters.required_status_checks に
# { "context": "ci-gate" } を末尾追加（既存 9 件は維持）
gh api -X PUT repos/scarecrow173/ruviform/rulesets/26547674 --input /tmp/ruleset.json
```

確認:

- `gh api repos/scarecrow173/ruviform/rulesets/26547674 --jq '.rules[] | select(.type=="required_status_checks") | .parameters.required_status_checks | length'` → `10`
- 既存 open PR は `ci-gate` check を持たない限り merge 不能になる。
  `strict_required_status_checks_policy` が有効なら merge には base との
  同期が必須であり、同期後の run は `ci-gate` job を含む workflow で再実行
  される — 報告待ちで merge が止まるのは意図する fail-closed であり、
  open PR の対応（sync・re-run）を PR 単位で確認する。

### Phase 2 — lane `if:` + macOS 分離 PR を merge

本 PR は `.github/**`・`ci/impact/**` 変更のため classifier が
`decision=full` を出し、全 lane が実行され 10 context 全て報告される
（自己完結検証）。`*-macos` legs は非必須のため runner 逼迫で cancelled
になっても merge を阻害しない。merge 時点で 10 required が全て緑である
ことを確認する。

### Phase 3 — 全 run で `ci-gate` 報告を確認後、旧 9 を除去

```sh
# 直近 run で ci-gate check が success で報告されていることを確認
gh api 'repos/scarecrow173/ruviform/commits/<merged-sha>/check-runs?per_page=100' \
  --jq '.check_runs[] | select(.name=="ci-gate") | {name, status, conclusion}'

# ruleset.json の required_status_checks を [{ "context": "ci-gate" }] 単一に編集
gh api -X PUT repos/scarecrow173/ruviform/rulesets/26547674 --input /tmp/ruleset.json
```

確認: 上記 length が `1` になること。**順序を逆にしないこと** — 先に 9 を
除去すると `ci-gate` 追加前の期間に required が空または片欠の無保護期間になる。

### 移行中 deadlock の扱い

Phase 2 merge 〜 Phase 3 完了の間、ruleset は `9 lane + ci-gate` のまま。
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
  `NotApplicable` 判定・`verdict: pass`・9 lane context 欠落でも
  `ci-gate` 単一で merge 可能であることを確認する。
- 同 PR で `macos-observe-report` artifact が skip された macOS leg を
  `NotApplicable`・runner 逼迫時を `NotRun` として記録することを確認する。
- `push: main` / `workflow_dispatch` run で decision=full・全 lane
  （required + observe）実行を確認する。

## 監視（有効化後の検出点）

| 検出点 | 意味 | 対応 |
| --- | --- | --- |
| gate-report `not-expected-lane-failure/cancelled` | **偽陰性** — skip された lane が本来 fail だった（本 PR 以降は lane 自体が skip されるため、同条件の再 run・別 PR でのみ顕在化） | 分類契約を修正し `CI_IMPACT_DISABLE=1` で即時全実行に退避 |
| gate-report `expected-lane-skipped` | lane `if:` 配線の破損（必要 lane が skip された） | 配線修正 or `CI_IMPACT_DISABLE=1` |
| gate-report `over-executed`（expected:false だが実行） | 誤判定（偽陽性） — コスト増のみ・安全性不変 | 分類規則の絞り込み修正 |
| `impact-classify` job 失敗 / `impact-decision` artifact 欠落 | 分類不能 → gate classification-error Fail・全 lane fail-open 実行 | classify.mjs 修正 |
| macos-observe-report `Fail`（expected or not） | macOS 実失敗 or 偽陰性顕在化 | 該当 leg 修正 or `CI_IMPACT_DISABLE=1` |
| macos-observe-report `NotRun` 常態化（expected legs） | macOS 実測欠落 — Release 受入判定不能 | runner 逼迫の解消 or 対象 run re-run |
| macos-observe-report `Pending` | 分類不能と leg 未実行の同時発生 | `impact` 失敗原因を修正 |
| selective 率・runner_total 推移 | 効果測定 | §metrics（exp doc）で Before/After 比較 |

## Rollback

| 状況 | 操作 | 効果 |
| --- | --- | --- |
| 選択実行を即時無効化（分類ミス疑い・runner 逼迫解消等） | repo variable `CI_IMPACT_DISABLE=1` を設定 → 対象 run を re-run | classifier が expected=ALL を出力し全 lane `if:` が真 → 全実行。code revert 不要。戻すときは variable 削除で即時再開 |
| 恒久ロールバック（配線自体を外す） | 本 PR の revert | lane `if:` が消え Shadow 同等（`impact`/`ci-gate`/`macos-report` 残留・macOS leg は別 job のまま） |
| ruleset 逆操作 | 9 context を再追加（追加→報告確認）してから `ci-gate` を除去 | 移行前と同じ required 構成。同じ「追加→確認→除去」順序を逆方向に適用 |
| macOS 必須化への復帰（将来・改訂の場合） | `*-macos` contexts を ruleset に追加 + observe job を required matrix へ再編する別 PR | rev 2026-11-05 以前の 3OS 必須構成へ。単独 ruleset 追加は `*-macos` が job 名で安定しているため可能だが、release 実測要件との整合を再評価 |
| `ci-gate` 自体の撤廃 | 上記で 9 復活後、required から `ci-gate` を除去し workflow から job 削除（別 PR） | 移行前状態へ完全復帰 |

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
- **runner 逼迫軽減効果**: required path が Linux/Windows 限定になったため、
  macOS queue→cancel による required 欠落・merge 遅延は構造的に解消される
  （本 PR 自身が証拠 — run 41863215849 で macOS leg 3 連続 cancel 後も
  required 9 context は全て報告済み）。

## Edge cases

- **本 PR 自身**: `.github/workflows/ci.yml`・`ci/impact/**` を含むため
  `ci-config` クラス → `decision=full`。全 lane（required + observe）が
  実行され 10 context が揃い、Phase 1 適用済みでも merge 可能。
- **`impact` job 失敗時**: lane は `always() && != 'false'` で全実行
  （fail-open coverage）、gate は classification-error で Fail（fail-closed
  判定）、macos-report は leg を `Pending` 記録。merge は止まるが
  coverage は落ちない設計意図。
- **cancelled required lane**: gate は expected/not-expected に関わらず
  `failure|cancelled` を Fail とする（実行済み結果は常に真の信号）。
- **cancelled observe leg**: `NotRun` 記録のみ — gate・ruleset ともに
  影響しない（構造的に merge 非阻害）。
- **PR base retarget**: `edited` type で再分類されるため stale 期待集合は
  残らない（issue-333 design の規約どおり）。
- **fork PR**: `impact`/`ci-gate`/`macos-report` は `contents: read` のみで動作。

## Verification Notes

- [x] `ci/impact/tests/workflow.test.mjs` が lane→impact 配線・required
      matrix の macOS 除外・`ci-gate`/`macos-report` の needs 分離・
      impact 無条件実行の構造不変条件を pin（`mise run test-ci-impact`）。
- [x] `ci/impact/tests/observe.test.mjs` がシナリオ (a)(b)(d)(e) と
      cancelled→NotRun・skip 分岐を pin。同 (c) は gate.test.mjs が pin。
- [x] `mise exec -- actionlint .github/workflows/ci.yml` で構文検証。
- [x] 本 PR の classifier E2E で `decision=full`・6 lane expected を確認。
- [ ] 本 PR の CI run で required contexts 全緑 + `macos-report` artifact
      記録を確認（Phase 1 適用後は `ci-gate` が required として合流）。
- [ ] Phase 4 の selective 実動確認は merge 後の後続 PR で実施
      （docs-only PR で `NotApplicable` 判定を観測）。
