---
id: "TSPEC-zbT71fmv3NCZBzB3xof25n"
type: "test-spec"
status: "approved"
title: "Issue #333 CI-OPT-06 test specification — classifier 分類契約・gate 判定規則"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "docs/specs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-333-impact-based-ci.md"
  changes:
    added:
      - type: "directory"
        path: "ci/impact"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-333-impact-based-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/plans/issue-333-impact-based-ci.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-333-impact-based-ci.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #333 test specification — classifier・gate・Shadow wiring

## Feature: Impact Classifier の分類契約

### Rule: crate 変更は verify + lane-owner lane + 逆依存 lane を返す

- Example: `crates/ruviform-math/src/lib.rs` のみ変更 → `verify`・`rt-verify`
  （rt_ 保持）・`bench`（bench-smoke 対象）が expected、`icc-verify`・
  `l1-real-verify` が not expected。`ui-verify` は math の逆依存先に
  ui-webview が無ければ not expected。
- Example: `crates/ruviform-plugin-core/src/lib.rs` 変更 → `verify` +
  `rt-verify`（plugin-core は rt_ 保持）+ `l1-real-verify`（逆依存に
  clap/vst3）。
- Example: `crates/ruviform-ui/src/lib.rs` 変更 → `verify` + `ui-verify`
  （逆依存に ui-webview/ui-test）+ `rt-verify`（ui は rt_ 非保持だが
  逆依存先の rt_ 保持 crate があれば追加 — fixture で実 workspace 依存に
  基づき検証）。

### Rule: ui パッケージ変更は ui-verify（+verify なし）

- Example: `ui/core/src/index.ts` 変更 → `ui-verify` のみ expected（ui/core
  は cargo workspace 外）。逆依存（components/guitest/showcase/browser-test）
  も ui-verify に収斂。
- Example: `ui/components/package.json` 変更 → `ui-verify`。

### Rule: docs/poc 変更は verify のみ

- Example: `docs/specs/x.md` 変更 → `verify` のみ。
- Example: `AGENTS.md` 変更 → `verify` のみ。
- Example: `poc/ui/package.json` 変更 → `verify` のみ。

### Rule: 共有・CI・repo 設定と未分類パスは Full CI

- Example: `Cargo.lock` → 全 6 lane expected（decision=full・理由記録）。
- Example: `mise.toml` → full。`.github/workflows/ci.yml` → full。
- `ci/impact/map.mjs` → full（分類器自身の変更）。
- `.gitignore`・`apm.yml` → full。
- 存在しない top-level `vendor/x` → full（unknown）。
- Example: `docs/x.md` + `Cargo.lock` の混在 → full（1 ファイルでも
  full クラスがあれば全体 full）。

### Rule: イベント種別

- `push`/`workflow_dispatch`/`--all` → decision=full。
- `pull_request` で diff 空 → decision=full（理由 `empty-diff`）。
- `CI_IMPACT_DISABLE=1` → expected=ALL（decision=full・理由
  `impact-disabled`）。

### Rule: 出力契約

- decision JSON は schemaVersion/event/base/head/decision/decisionReasons/
  files/cratesTouched/cratesImpacted/lanes{expected,oses,features,reasons}
  を持つ。`expected:false` lane の reasons は空でなくてよいが expected
  決定には寄与しない。

## Feature: `ci-gate` 判定規則

### Rule: expected×actual の判定マトリクス（design 表どおり）

- expected=true + success → Pass。
- expected=true + failure/cancelled → Fail。
- expected=true + skipped → Fail（期待 lane の skipped は成功扱いしない）。
- expected=false + skipped → NotApplicable。
- expected=false + success → Pass（over-executed・記録）。
- expected=false + failure/cancelled → Fail（実行結果は必ず信用 — Shadow
  期の偽陰性検出列）。
- impact job 失敗/expected 出力欠落 → 全 lane expected として照合した上で
  classification-error Fail。
- needs 欠落（lane job が needs に無い/result 不明）→ Fail（fail-closed）。

### Rule: 出力

- summary 表に lane/expected/actual/判定/理由が含まれる。
- `gate-report.json` が生成され、base/head SHA・モード・判定を記録。
- verdict=fail で exit 1、pass で exit 0。

## Feature: ci.yml Shadow wiring

### Rule: trigger・job 構成

- `pull_request.types` は opened/synchronize/reopened/ready_for_review/
  edited を含む。`push`・`workflow_dispatch` は変更なし。
- `impact` job は全 trigger で実行・rust+node を install・
  `node --test ci/impact/` → `classify.mjs` の順で自己検証+分類。
- `ci-gate` は全 7 needs + `if: always()` で必ず実行。
- 既存 6 job の steps・`if:`・matrix は無変更（required contexts 12 件維持）。

### 検証方法

- `mise exec -- actionlint .github/workflows/ci.yml` pass。
- `mise exec -- prek run --all-files` pass（check-yaml 等）。
- `node --test ci/impact/` が全 fixture pass（`impact` job 内でも同一実行）。
- 本 PR の CI run で `impact`/`ci-gate` が緑・summary 表と artifact
  （impact-decision・gate-report）が出力されることを観測し IR/exp に記録。

## 関連文書

- 要求: [docs/specs/issue-333-impact-based-ci.md](../specs/issue-333-impact-based-ci.md)
- 設計: [docs/designs/issue-333-impact-based-ci.md](../designs/issue-333-impact-based-ci.md)
- 実装計画: [docs/plans/issue-333-impact-based-ci.md](../plans/issue-333-impact-based-ci.md)
