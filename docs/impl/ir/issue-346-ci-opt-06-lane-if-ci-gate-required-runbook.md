---
id: "IMPL-034RP8X0bpYtK5In9i3F00"
type: "impl"
status: "completed"
title: "Issue #346 CI-OPT-06 選択実行有効化（lane if 配線）・ci-gate 単一 required 移行 runbook"
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
    - "docs/designs/issue-346-ruleset-migration-runbook.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-333-impact-based-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-346-ruleset-migration-runbook.md"
    - "docs/impl/ir/issue-333-ci-opt-06-impact-gate.md"
    - "docs/impl/exp/issue-346-selective-enforcement.md"
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

# Issue #346 CI-OPT-06 選択実行有効化（lane if 配線）・ci-gate 単一 required 移行 runbook

## Summary

issue-333 設計の段階移行 runbook Stage C/D を実行し、変更影響ベース選択実行を
**有効化**した。6 つの検証 lane job（`verify`・`rt-verify`・`icc-verify`・
`ui-verify`・`l1-real-verify`・`bench`）に `needs: [impact]` と
`if: always() && needs.impact.outputs.lane_* != 'false'` を配線し、
classifier が「影響なしを証明した」lane のみ skip する。さらに
Issue #333/#326 の 2026-11-05 改訂合意を受け、macOS legs（`verify`・
`ui-verify`・`l1-real-verify`）を PR Required から**物理分離** — matrix を
ubuntu+windows に縮小し、macOS leg を `*-macos` 別 job として `ci-gate` の
`needs` から存在ごと除外した。macOS legs は継続実行され、結果は新設の
`macos-report` job（`if: always()`・non-blocking）が OS 別
`Pass`/`Fail`/`NotRun`/`Pending`/`NotApplicable`/`Unsupported` で記録する。
skip される lane の required context 報告が止まる deadlock を不可分対処として、
`ci-gate` を唯一の安定 required context とする ruleset 移行 runbook
（`docs/designs/issue-346-ruleset-migration-runbook.md`）を新設。
`push`/`workflow_dispatch` は classifier の `FULL_EVENTS` で常に
expected=ALL（Full CI 維持）。

## Context

Issue #346（CI-OPT-06 follow-up・#333 の残課題）。macOS arm64 runner 逼迫で
required lane が queue→cancel される実害を受け、owner 判断で Shadow 観測期を
前倒しして有効化へ進む。Shadow 実績: PR #347/#348 で selective 判定・偽陰性 0
を 2 件観測（`docs/impl/exp/data/issue-334-*`・`issue-335-*` jsonl）。

**実装中の設計改訂（2026-11-05・coordinator 通知）**: Issue #333/#326 の
合意改訂により、macOS legs を PR Required から物理分離する構成へ変更した
（当初実装は全 12 context を `ci-gate` needs に含める案だった）。ruleset
required contexts の正本は `main-checkstatus`（id `26547674`）で、
coordinator により既に 3 macOS context 除去済みの 9 件へ改訂済み
（`main-protect` id `26529830` は pull_request ルールのみで required
contexts を持たない）。移行操作自体は引き続き coordinator 担当、worker は
コード配線と runbook 化を担当する。

## Implementation

### Changes

- `.github/workflows/ci.yml`:
  - 6 lane job に `needs: [impact]` +
    `if: always() && needs.impact.outputs.lane_<id> != 'false'` を追加
    （`rt_verify`/`icc_verify`/`ui_verify`/`l1_real_verify`/`bench`/`verify`
    — output 名の hyphen→underscore 変換は classify.mjs の GITHUB_OUTPUT
    emit 規約に一致）。
  - `verify`/`ui-verify`/`l1-real-verify` の matrix を
    `[ubuntu-latest, windows-latest]` に縮小（required legs）。
  - `verify-macos`/`ui-verify-macos`/`l1-real-verify-macos` 新設:
    `needs: [impact]` + 親 lane の `lane_*` 出力 `if:` + `runs-on:
    macos-latest`。parent lane と同一ステップ（Linux-only step は除外）を
    実行する非ブロッキング observe leg。
  - `macos-report` job 新設: `needs: [impact, verify-macos,
    ui-verify-macos, l1-real-verify-macos]` + `if: always()` で
    `ci/impact/observe.mjs` を実行し `macos-observe-report` artifact を
    出力。check 自体は常に緑（verdict はデータとして記録）。
  - `ci-gate` の `needs`/`if: always()` は無変更 — `*-macos` job は
    needs に存在ごと含まない（物理分離）。comment を改訂記述へ更新。
  - `impact` job の Shadow Mode 記述 comment を enforcement 記述へ更新。
- `ci/impact/map.mjs`: `LANES[*].requiredContexts`（PR 必須 subset）と
  `observeJob` を追加し `contexts` を実際の発行 context 名へ更新、
  `OBSERVE_LANES`/`OBSERVE_LANE_IDS`（`*-macos` job → parent lane 対応表）
  を新設。
- `ci/impact/observe.mjs` 新設: `evaluateObserve` — expected 集合 ×
  `*-macos` needs 結果の status 表（Pass/Fail/NotRun/Pending/
  NotApplicable/Unsupported）。cancelled は常に `NotRun`（runner 逼迫を
  `NotApplicable` にしない）、`NotApplicable` は classifier 証明済み
  skip のみ、分類不能+未実行は `Pending`。常に exit 0（non-blocking）。
- `ci/impact/lib.mjs`: decision JSON の lane entry に
  `requiredContexts`/`observeJob` を追加（audit 用・shape 拡張のみ）。
- `ci/impact/gate.mjs`: row の `contexts` 報告を `requiredContexts`
  優先に変更（gate が実際に照合する context を報告）。
- `ci/impact/tests/workflow.test.mjs` 新設: lane→impact 配線・required
  matrix の macOS 除外・`*-macos` job の存在/if・`ci-gate` needs の
  macOS 非含有・`macos-report` needs 分離・`impact` 無条件実行を pin。
- `ci/impact/tests/observe.test.mjs` 新設: 合意シナリオ
  (a)(b)(d)(e) + status 語彙の全分岐を pin。
- `ci/impact/tests/gate.test.mjs`: `*-macos` needs エントリが gate 判定に
  無関係である pin + required leg 失敗 pin を追加（シナリオ (a)(b)(c)）。
- `docs/designs/issue-346-ruleset-migration-runbook.md` 新設: ruleset
  `main-checkstatus` required context を `9 lane context → ci-gate 単一` へ
  「追加→報告確認→除去」で無保護期間なく移行する操作 runbook + macOS
  分離設計 + Release/Consumer 受入の macOS 実測要件 + 移行中 deadlock
  扱い + 監視規則 + rollback。
- `docs/impl/exp/issue-346-selective-enforcement.md` + `data/issue-346-enforcement-before-runs.jsonl`:
  Before（Shadow 全実行）計測の記録と After 未実測の明示。
- `AGENTS.md`: `ci.yml` 説明文を Shadow Mode から enforcement + macOS
  分離記述へ同期、`docs/designs/README.md`（generated index）・
  `docs/impl/ir/README.md`・`docs/impl/exp/README.md` 更新。

### Decisions During Implementation

- `if:` は `always() && outputs.lane_* != 'false'` — `impact` job 失敗・出力
  欠落時は出力が空になり lane が**実行される側**に倒す（fail-open coverage）。
  fail-closed 性は gate の classification-error Fail 側に保持する設計どおり。
- **macOS legs は job 物理分離**（rev 2026-11-05 合意）: `needs` は job
  単位でしか指定できず matrix leg を部分的に外せないため、matrix 縮小 +
  `*-macos` 別 job が唯一の正しい境界。`continue-on-error` や ruleset 側の
  除外のみでは「job が red でも required context を報告しない deadlock」と
  「job を緑に偽装する形骸化」のどちらかに落ちる。
- required context の移行対象は `ci-gate` 単一 — `impact-classify`・
  `macos-report` を別途 required にしない（前者の失敗は gate の
  classification-error で Fail になる、後者は non-blocking 記録）。
- `macos-report` の check は常に緑（exit 0）: verdict は JSON artifact +
  step summary に記録する。leg `Fail` を check 赤で表す案は却下 — runner
  逼迫期に毎 PR が赤くなり alarm fatigue で信号が死ぬため、データ記録 +
  Release 受入での実測要求に分離した。
- observe 評価は gate.mjs と別モジュール（`observe.mjs`）: verdict 語彙と
  fail 条件が異なる責務であり、単一 authoritative でも共通 helper でも
  なく、純粋関数として独立 pin できる境界を採った。
- `docs/designs/issue-333-impact-based-ci.md` 自体は契約変更せず、冒頭に
  日付付き改訂注記のみ追加（契約本文は改訂後構造を前提として読める範囲を
  明示）。操作 runbook は引き続き新規 design doc として分離。
- ruleset 移行自体は本 PR には含めない（workflow 外の admin 操作）。
  移行中 deadlock 窓（Phase 2 merge〜Phase 3 完了）の扱いを runbook に明記。

## Related Experiments

### Adopted

- Shadow 実績（PR #347 run 41840536142・PR #348 run 41848239157）:
  `decision=selective`・`falseNegatives=0`・not-expected lane が全 pass で
  over-executed 監査として記録済み（`docs/impl/exp/data/issue-334-*`・
  `issue-335-*` jsonl）。
- Before 計測（2026-11-05 収集・`ci/metrics/collect-ci-metrics.mjs
  --runs 15`）: wall p50 307s / p95 497s・代表 run critical path 235s・
  runner_total 1474s・macOS queue 滞留（push run 41851785028 queued
  2279s+）を実測記録。raw: `docs/impl/exp/data/issue-346-enforcement-before-runs.jsonl`。

### Rejected

- lane 内 step 条件付き skip（job は常に success）: 「何も検証していない
  成功」を required context が緑として報告する形骸化（issue-333 design が
  既に不採用とした案 — 同じ理由で踏襲）。
- `if:` なしの lane + artifact 参照による分岐: job `if:` + `needs` output の
  方が宣言的で gate の `needs.result` 照合と一致するため採用せず。
- ruleset の同時置換（9 除去 + ci-gate 追加を 1 操作）: `ci-gate` が全 PR で
  報告される実績確認前に単一化すると、報告経路の不具合時に無保護になる
  リスクがあるため「追加→確認→除去」の 2 段階を採用。
- macOS leg を required matrix に残して `continue-on-error` 等で非致命化:
  rev 2026-11-05 合意で明示的に不採用 — job 単位の `needs` は matrix leg を
  分離できず、required context の報告欠落 deadlock が残るか結果を緑に
  偽装するかの二択に落ちる。物理分離が唯一の正しい境界。
- `macos-report` で leg `Fail` 時に check を赤くする案: runner 逼迫期に
  全 PR が赤くなり non-blocking の意味が失われる。verdict は artifact
  データに記録し Release 受入で評価する。

## Validation

実測環境（ローカル）: Windows 11 x64 / Git Bash、mise 管理 toolchain
（node 26.12.1）。

| 項目 | 実行 | 結果 |
| --- | --- | --- |
| `mise run test-ci-impact` | `node --test ci/impact/` | Pass — 51/51（classify 19 + gate 13 + observe 13 + workflow 6） |
| `mise exec -- actionlint .github/workflows/ci.yml` | — | 実施結果は PR 本文の検証証拠欄を参照 |
| `prek run`（変更ファイル全 hook） | commit 時フック | 同上 |
| classifier E2E（本 PR 差分） | `.github/**`・`ci/**` → `decision=full`・6 lane expected | 同上 |
| 本 PR の CI run（rev 前・run 41863215849） | macOS legs 3 連続 queue→cancel（`ui-verify (macos-latest)`・attempt 1–3）・他 10 job 全緑 | macOS 逼迫の実害を自己再現 — 物理分離の動機を実証 |
| 本 PR の CI run（rev 後） | required contexts 全緑 + `macos-observe-report` 記録 | CI 上で確認（PR checks 参照） |

## Risks

- 移行中 deadlock 窓（Phase 2 merge 〜 Phase 3 完了）: selective PR が旧 9
  context を報告せず merge 不能になる。runbook に最小化・復旧手順を明記済み。
- `impact` job が critical path に直列化するため全 run の wall が +~20-25s
  増える（Shadow 期は並列で critical path 外）。見返りは selective PR の
  runner_total 削減と macOS queue 滞留の回避。
- 偽陰性の残存リスク: Shadow 実績は 2 PR（selective）のみで蓄積が薄い。
  gate-report の監視規則で継続検出し、検出時は `CI_IMPACT_DISABLE=1` で
  即時 Full CI に退避する。
- macOS 実測欠落リスク: `*-macos` legs が非必須になったため、長期の
  runner 逼迫では `macos-observe-report` の `NotRun` が常態化しうる。
  Release 受入での Pass 要求が safety net（runbook 監視表に記録）。
- `*-macos` job と required lane の step 重複（GHA は YAML anchor 非対応）:
  両者の step が乖離する保守リスク。verify-macos は prek/fmt/lint/test の
  同一 mise task で抽象化されているため乖離余地は小さい。

## Follow-ups

- ruleset `main-checkstatus`（id `26547674`）の Phase 1〜3 を coordinator が
  `gh api` で実施（runbook どおり「ci-gate 追加 → 報告確認 → 9 除去」）。
- selective 実動証（docs-only PR で NotApplicable 判定 + `macos-report` の
  `NotApplicable`/`NotRun` 記録）を merge 後の後続 PR で確認し
  `docs/impl/exp/issue-346-selective-enforcement.md` に記録。
- After メトリクス（wall/runner_total/lane skip 率）を同一収集手順で実測し
  Before と比較。
- Release/Consumer 受入側が `macos-observe-report` artifact を候補 SHA
  紐づけ実測として消費する運用を別途確認（本 PR では artifact 出力まで）。
