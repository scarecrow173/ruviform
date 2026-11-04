---
id: "SPEC-x9aakFSwqMZzdqGoq2uyc3"
type: "spec"
status: "approved"
title: "Issue #332 CI-OPT-05 Playwright Chromium 準備・ブラウザキャッシュと実行時間の最適化"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/332"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/330"
    - "https://github.com/scarecrow173/ruviform/issues/310"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-332-playwright-browser-cache.md"
    - "docs/plans/issue-332-playwright-browser-cache.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/designs/issue-310-browser-gui-test-harness.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-332-playwright-browser-prep-inventory.md"
  derived-by:
    - "docs/designs/issue-332-playwright-browser-cache.md"
    - "docs/plans/issue-332-playwright-browser-cache.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-332-playwright-browser-cache.md"
  references:
    - "mise.toml"
    - ".github/workflows/ci.yml"
    - "ui/browser-test/package-lock.json"
    - "ci/metrics/collect-ci-metrics.mjs"
  defers: []
  deferred-by: []
---

# Issue #332 CI-OPT-05 Playwright Chromium 準備・ブラウザキャッシュと実行時間の最適化

## What Should Be Built

`ui-verify` CI ジョブ（3 OS matrix）で毎 run 行われる Playwright Chromium バイナリのダウンロードを、Playwright バージョン・Chromium revision・OS/arch に紐づく `actions/cache` で再利用化する。併せて browser install を `test-ui-browser` から独立した mise task / CI step に分離し、browser setup 時間・download 有無・実テスト時間を個別計測可能にする。Linux の `playwright install-deps`（OS system library）はブラウザキャッシュと混同せず現行維持。cache miss・破損・バージョン変更時は必ず正しいブラウザを再取得する fail-closed fallback を持つ。

## Why Now

CI-OPT-00（#327）で計測基盤と lane 分類が定義され、CI-OPT-03（#330）で npm ci / showcase dist の重複が解消済み。残る反復コストは Chromium バイナリの毎 run ダウンロード（3 OS × 1 回、ubuntu 実測 ~9s の download + install 処理）と、それが `test-ui-browser` 内に同居して「browser 準備」と「実テスト」の時間が分離できないこと。download は外部 CDN 依存であり、分離・cache 化は実行効率と外因失敗耐性の両方を検証能力を削らずに改善できる。

## Users and Value

- PR 作成者・レビュアー: `ui-verify` の browser lane が外部ネットワーク状態に依存しにくくなり、browser setup と test の時間内訳が CI step から直接読める。
- Ruviform CI メンテナ: Playwright バージョン更新時の cache 挙動が key 設計として明示され、破損時の回復経路が文書化される。
- Consumer（Synvelis / AudioLens）: Ruviform 側 CI 契約（required checks・Evidence）を変えないまま基盤が効率化される。

## Scope

### In Scope

- `mise.toml` に browser install 専用タスクを分離し、`test-ui-browser` がそれを `depends` で再利用する構造。
- `.github/workflows/ci.yml` の `ui-verify` ジョブへ browser バイナリ cache（`actions/cache`）・準備 step・launch 検証 step の追加。OS/arch・Playwright バージョン・Chromium revision に紐づく key。
- Linux `playwright install-deps chromium`（apt system deps）の現行維持と、browser バイナリ cache との分離の明文化。
- Cold/Warm の browser setup 時間・download 有無・実テスト時間の分離記録（#327 テンプレート整合）。
- `AGENTS.md` の検証入口テーブル・ui-verify 記述の同期。

### Out of Scope

- Playwright テストの並列化（worker 数変更）。flake 率・runner 負荷の計測が前提であり本 Issue では変更しない（Deferred Design Concerns へ記録）。
- Linux apt system deps の cache 化（third-party action 依存・system lib 部分復元リスクを伴い別途設計が必要）。
- テスト内容・GUI fixture 範囲・Evidence（evidence.json/trace/screenshot）・artifact 保持期間の変更。
- `verify` ジョブ等 Rust 側 lane・cargo cache の変更（CI-OPT-02/04 の範囲）。
- mise tool cache の cold 対策（#327 で観測済み・別 Issue 候補）。

## Requirements

- ブラウザバイナリ cache は Playwright バージョン・Chromium revision・OS/arch を key に含め、version/revision 変更時に必ず新規取得へ落ちること。
- OS system library（Linux apt）は cache と分離し、毎 run 必要に応じて install されること（browser cache 有無に依存しない）。
- cache miss・部分復元・破損・バージョン変更のいずれでも、テスト実行前に正しいブラウザが取得されること。破損復元済みバイナリでテストに進まないこと（launch 検証で fail-closed）。
- browser download と実テストが CI step として分離計測可能であること（`collect-ci-metrics.mjs` の既存 lane 分類で `cache-restore` / `playwright` / `ui` に落ちる step 名）。
- 既存の `test-results/` Evidence（evidence.json・trace・screenshot）・30 日 artifact 保持・対象 GUI fixture 範囲・`retries: 0` 等の検証契約は不変。
- Browser PASS は引き続き browser-lane の証拠であり、OS WebView / native GPU / 製品 E2E の成功として扱わない（#313/#314 への誤認防止）。
- 第 2 の独自キャッシュ機構・独自スクリプトを導入しない（`actions/cache` と mise task の標準機能を用いる）。

## Acceptance Criteria

- [ ] AC-1: Cold/Warm の browser setup 時間・download 有無・実テスト時間が分離記録される。
- [ ] AC-2: 3 OS で Playwright 既存テストと Evidence が成功する。
- [ ] AC-3: revision 変更 / 破損 / cache miss で安全に通常 install へ fallback する。
- [ ] AC-4: UI test 回帰・flake 増加なし、必要な失敗 Artifact を維持する。
- [ ] AC-5: Browser PASS を実 WebView / native GPU / 製品 E2E 成功として扱わない。
- [ ] AC-6: 改善の Before/After と Rollback を記録する。

## Deferred Design Concerns

- Playwright test workers 並列化 | 30 spec の suite が runner 負荷支配になった場合に効果 | flake 率・競合（hermetic serve）・runner CPU 実測の記録が前提 | 現行デフォルト workers（CPU/2）を変更しない。
- Linux `playwright install-deps` の cache / パッケージ前置 | install-deps 4–23s が支配的になった場合 | third-party apt cache action の信用境界評価・system lib 整合が別途必要 | 現行 run 維持。
- mise tool cache の cold 対策 | #327 で全 job 系統的 miss を観測（13–23s/job） | CI-OPT 系列の別 Issue | 本 Issue の範囲外。

## Sources and Relations

- 上流: Issue #332（本 spec の要件源）、#327（Before/After 計測テンプレート・Fail-Closed 規約の正本）。
- 前提: #330（npm ci / dist 重複の解消済み）、#310（browser-test lane・evidence 契約）。
- 実測源: `docs/discovery/issue-332-playwright-browser-prep-inventory.md`。
