---
id: "DESIGN-034SmuehTqDxp5K68M5dSG"
type: "design"
status: "approved"
title: "Issue #366+#364 Full CI Stage 1 設計 — 選択実行撤去・3OS 常時実行・ci-summary fail-closed・expected_sha 契約"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/366"
    - "https://github.com/scarecrow173/ruviform/issues/364"
    - "docs/specs/issue-366-full-ci-stage1.md"
  changes:
    added:
      - type: "directory"
        path: "ci/summary"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted:
      - type: "directory"
        path: "ci/impact"
      - type: "file"
        path: ".github/workflows/consumer-impact-check.yml"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-366-full-ci-stage1.md"
  implemented-by:
    - "docs/plans/issue-366-full-ci-stage1.md"
  depends-on:
    - "docs/specs/issue-366-full-ci-stage1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-346-ruleset-migration-runbook.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-366-full-ci-stage1.md"
  derived-by:
    - "docs/plans/issue-366-full-ci-stage1.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "https://github.com/scarecrow173/ruviform/issues/326"
  defers: []
  deferred-by: []
---

# Issue #366+#364 Full CI Stage 1 設計 — 選択実行撤去・3OS 常時実行・ci-summary fail-closed・expected_sha 契約

## Context

現行 `ci.yml`（base `93801df`）は 13 job: `impact` 分類器（`impact-classify`）、7 lane job（`verify`・`rt-verify`・`icc-verify`・`compression-verify`・`ui-verify`・`bench`・`l1-real-verify`）、3 個の非ブロッキング `*-macos` job、`macos-report`（常に緑の observe）、`ci-gate`（分類期待集合 × 実績の集約 gate）。ruleset `main-checkstatus` の required context は lane 名 9 件（`verify`/`l1-real-verify`/`ui-verify` の ubuntu・windows leg + `rt-verify`/`icc-verify`/`bench`）であり、`impact-classify`/`ci-gate`/`macos-report`/`*-macos` は required ではない（#365 実測）。

Stage 1 の制約: PR/push trigger と 9 required context を維持しつつ、Full CI（全 lane 常時実行）と `ci-summary` fail-closed 集約へ移行する。required context を産む job 名・matrix 名・`name:` は一切変更しない。

## Scope

### In Scope

- 選択実行機構の撤去: `impact` job、`needs: [impact]`、各 lane の `if: always() && needs.impact.outputs.lane_* != 'false'`、`ci/impact/` 全ファイル、`CI_IMPACT_DISABLE` env 参照、mise task `test-ci-impact`。
- 全 lane 無条件実行（`pull_request`・`push`・`workflow_dispatch` 同一 lane 集合）。
- `io-structured-verify` job 新設（Issue #364）。
- `ci-gate`+`macos-report` 廃止 → `ci-summary` job 新設（`ci/summary/summary.mjs` + `ci/summary/tests/` contract tests + mise task `test-ci-summary`）。
- `workflow_dispatch` に `expected_sha` 入力を追加。
- `consumer-impact-check.yml` 削除（PR 自動起動の廃止。`consumer-verify.yml` は維持）。

### Out of Scope

- `pull_request`/`push` trigger 除去・ruleset 変更（Owner 作業・最終切替）。
- `*-macos` job の matrix 物理統合（据置。Deferred Design Concerns 参照）。
- `daily-ci.yml`/`ci-issue-tracker.yml`（#367/#368）。

## Components and Boundaries

### Lane 一覧（Stage 1 完了後の job 構成・計 12 job）

| job id | check name | OS | 実行内容（mise run） | required |
|---|---|---|---|---|
| `verify` | `verify (ubuntu-latest)` / `verify (windows-latest)` | u+w | prek → fmt-check → lint-ci → test-ci | 2 leg |
| `verify-macos` | `verify-macos` | macos | 同上（Linux 専用 step なし） | - |
| `rt-verify` | `rt-verify` | ubuntu | RT 選択非空 guard → test-rt | 1 |
| `icc-verify` | `icc-verify` | ubuntu | test-video-proc-icc → lint-video-proc-icc | 1 |
| `compression-verify` | `compression-verify` | ubuntu | check/test/lint-io-compression | - |
| `io-structured-verify` | `io-structured-verify` | ubuntu | check/test/lint-io-structured（新設・#364） | - |
| `ui-verify` | `ui-verify (ubuntu-latest)` / `ui-verify (windows-latest)` | u+w | setup-ui + 20 ui task + playwright | 2 leg |
| `ui-verify-macos` | `ui-verify-macos` | macos | 同上（Linux 専用 step なし） | - |
| `bench` | `bench` | ubuntu | bench-smoke | 1 |
| `l1-real-verify` | `l1-real-verify (ubuntu-latest)` / `l1-real-verify (windows-latest)` | u+w | test/lint-l1-real-plugin | 2 leg |
| `l1-real-verify-macos` | `l1-real-verify-macos` | macos | 同上 | - |
| `ci-summary` | `ci-summary` | ubuntu | test-ci-summary → summary.mjs → artifact | - |

未自動化の real GUI / real hardware 検証は本 workflow の対象外（実施済み Pass として表示しない）。

### `ci-summary` 判定器（`ci/summary/summary.mjs`）

- 入力: `NEEDS_JSON`（`toJson(needs)`）、`EXPECTED_SHA`（`inputs.expected_sha`・空可）、`HEAD_SHA`（**summary job 自身の実 checkout を `git rev-parse HEAD` で測定** — `github.sha` ではない）、`EVENT_SHA`（`github.sha`）、`CHECKOUT_DIR`（全 lane leg の `checkout-sha-*` artifact を `download-artifact` `pattern`+`merge-multiple` で集約した dir）、`WORKFLOW_REF`/`WORKFLOW_SHA`（`github.workflow_ref`/`workflow_sha` — 実行された workflow revision の証跡）、`EVENT_NAME`、`RUN_URL`、`GITHUB_STEP_SUMMARY`。
- verdict は `needs` context + 実測 checkout 証跡の純関数: 全 needs job の `result === 'success'` なら `pass`。`failure`/`cancelled`/`skipped`/未知値/`needs` に存在しない必要 job は `fail`。SHA 証跡も fail-closed: 宣言 leg の `checkout-sha` 未取得・不正値・命名規約外の証跡ファイル（`checkout-sha-malformed`）・同一 leg の attempt 間乖離（`checkout-sha-attempt-divergence`）・leg 間不一致（`lane-checkout-mismatch`）、summary 自身 checkout（`summary-checkout-mismatch`）・event sha（`event-checkout-mismatch`）の lane 共通値との不一致、および expected_sha 非空時の実測共通値との不一致（`expected-sha-mismatch`）/ 実測不能（`expected-sha-unverifiable`）は全て `fail`。
- **rerun 対応**: upload-artifact v4 は artifact 名が run 単位で一意（同名 upload は 409 Conflict・immutable・attempt をまたいで残存）。証跡 artifact は `checkout-sha-<leg>-attempt-${{ github.run_attempt }}` 命名で attempt ごとに新規 upload し、rerun-failed-jobs / rerun-all 両方で衝突せず全 attempt の証跡を保全する。評価器は leg ごとに最大 attempt の実測値を採用する — `needs.<job>.result` も最新 attempt の結果を返すため、verdict は常に「各 leg の最新 attempt が実際に checkout した revision」を記述する。同一 leg が複数 attempt で異なる sha を記録した場合は ref pin により理論上あり得ず、`checkout-sha-attempt-divergence` として fail。同一設計を `ui-browser-evidence-*`・`ci-summary-report` にも適用し、これらの upload も rerun で衝突しない（report は全 attempt 分を保全）。
- **3 概念の分離**: workflow revision（`github.workflow_sha`/`workflow_ref` — どの ci.yml が実行されたか）は証跡記録のみ・event sha（`github.sha` — run が dispatch/push された commit）は裏付け anchor・**実 checkout sha（各 lane で `git rev-parse HEAD` を実測）こそが「検証した revision」**。expected_sha は実測共通値（laneCommon）と照合し、`github.sha` を期待値の代理にしない。全 checkout step は `ref: ${{ github.sha }}` で trigger snapshot に pin（pull_request/push では既定と同義、workflow_dispatch で lane ごとの ref 浮動を排除）。
- **API 非依存**: 旧 `ci-gate` と同じ job 粒度判定（issue-333 IR「権限追加（checks API）を避ける」方針を継承）。`*-macos` を別 job のまま `needs` に含めるため、macOS leg の per-OS 結果は needs から直接得られる。per-leg（ubuntu/windows matrix 内訳）は各 leg 自身の check run が担う。checkout 実測値の集約も API ではなく artifact（`checkout-sha-<leg>-attempt-<n>` upload → `pattern`+`merge-multiple` download）で完結する。
- 出力: Job Summary（event・実測共通 sha・expected sha・event sha・summary checkout・workflow ref/sha・run URL・job 別結果表・leg 別 checkout 表・verdict）+ `ci-summary-report.json` artifact（`kind: "ci-summary"`・jobs 配列・checkoutShas 配列・reasons・verdict）。
- 入口検証: `ci-summary` job は `mise run test-ci-summary`（`node --test ci/summary/`）を判定前に実行。評価器の契約が壊れているなら job 自体が失敗し、誤 verdict を発行しない（旧 `impact` job の `test-ci-impact` 先行パターン踏襲）。

### `expected_sha` 契約

- `workflow_dispatch.inputs.expected_sha`（string・optional）。Daily scheduler（#367）が dispatch 時の main HEAD を渡す契約。
- `ci-summary` は `EXPECTED_SHA` 非空かつ**全 lane leg の実測 checkout 共通値**と不一致なら fail（実測不能・証跡欠落も fail）。`pull_request`/`push` では inputs context が空のため照合なし。
- SHA 不一致でも lane は実 SHA で完走する — run は「実 SHA の検証」として証拠を残し、期待 revision の検証済みとは記録しない。
- lane 証跡: 各 lane job は checkout（`ref: github.sha` pin）直後に `git rev-parse HEAD > checkout-sha-<leg>-attempt-<n>.txt` を記録し同名 artifact として upload（matrix leg は `CHECKOUT_LEG` job env で `<job>-<os>` を、artifact 名は `github.run_attempt` で attempt を一意化）。`ci-summary` が宣言 14 leg 分を必須証跡として収集・照合する — lane が緑でも実測値を出せない run は証跡欠落として fail。

## Data and Control Flow

- PR/push/dispatch → 全 11 lane job が分類なしで並列起動（checkout は `ref: github.sha` に pin・直後に実測 sha を `checkout-sha-<leg>-attempt-<n>` artifact として出力） → `ci-summary`（`if: always()`・11 needs）が全終了を待機 → lane 実測 sha を download → NEEDS_JSON + 実測 sha を評価 → verdict + 証拠出力。
- 失敗伝播: lane 失敗 → `needs.<lane>.result = failure` → `ci-summary` fail。needs 自体の job 削除/rename で key 欠落 → `missing` → fail（fail-closed）。checkout 実測 artifact 欠落・値不正・leg 間不一致・expected_sha 不一致も fail（証跡の無い run を「期待 revision 検証済み」にしない）。
- `consumer-impact-check.yml` 削除後、PR 本文の Consumer 影響分類要件は `PULL_REQUEST_TEMPLATE.md` の表とレビュアー確認（docs/development-environment.md 記載の human gate）で維持。

## Non-Functional Constraints

- required context 名の不変性: `verify`/`l1-real-verify`/`ui-verify` job id・matrix os 配列（`[ubuntu-latest, windows-latest]`）・`name:` テンプレートを不変とする。新規 check context（`io-structured-verify`・`ci-summary`）は required でないため追加してよい。
- 判定器は stdlib-only Node.js（ci/impact と同一方針・外部依存なし）。
- lane step は `mise run` 入口のみ呼ぶ（cargo 直接呼び禁止・AGENTS.md 検証入口規約）。
- 権限最小化: workflow 全体 `contents: read` のまま（`ci-summary` は API を呼ばないため追加権限なし）。

## Risks and Trade-offs

- **PR ごとの実行コスト増**: 選択実行の撤去で docs-only PR も全 lane 実行になる。#366/#365 が明示的に選んだ trade-off（Full CI は選択実行しない）。
- **`ci-summary` 未 required 期間**: Owner が ruleset を更新するまで `ci-summary` 失敗は merge を止めない。移行中の挙動は「9 lane context が required・`ci-summary` は正確な run 証拠」で、旧 `ci-gate` と同等以上の情報量。
- **macOS 失敗の可視性**: `*-macos` を `ci-summary` needs に含めるため macOS leg 失敗で run が赤くなる。required でないため merge ブロックにはならない（旧 `verify-macos` 失敗と同等の PR 表示）。
- **consumer-impact-check 削除**: 機械検査が消え、記入漏れ検出はレビュアー依存になる。#366 step 6 が明示する運用判断。required でないため deadlock リスクなし。
- **matrix 統合の据置**: `*-macos` 別 job の重複 ~250 行が残る。needs 直接参照・API 不要・per-OS needs 粒度の利点を取り、物理統合は `ci-summary` required 化後の単純化 stage に据え置く（判断を本書に記録）。

## Deferred Design Concerns

- `*-macos` 3 job の `verify`/`ui-verify`/`l1-real-verify` matrix 統合 | needs 直接参照（API 不要）を優先・required context 無変更のまま小さく切れる stage 境界 | `ci-summary` required 化後 | 重複が残る
- `verify` 等の macOS leg を matrix に入れた場合の per-leg 報告（Actions Jobs API + `actions: read` 権限） | 同上 | 統合実施時 | 権限追加の審査が必要
- lane 単位の追加 required（`compression-verify`・`io-structured-verify`）の required 化 | ruleset 変更は Owner 権限 | `ci-summary` 単一 required への移行判断時 | 新 lane は暫定で非 required

## Verification Notes

- [ ] `mise exec -- actionlint .github/workflows/ci.yml` pass。
- [ ] `mise run test-ci-summary`（node --test）で verdict 行列・expected_sha・report 形状を検証。
- [ ] `mise exec -- prek run --all-files`（check-yaml 等）pass。
- [ ] `mise run check-io-structured` / `lint-io-structured` / `test-io-structured` が lane 呼出しと同一入口で実行可能。
- [ ] git grep で `ci/impact`・`CI_IMPACT_DISABLE`・`needs.impact`・`ci-gate`・`macos-report` の残留参照ゼロ（履歴 docs を除く）。
