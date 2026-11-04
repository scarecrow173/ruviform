---
id: "PLAN-zwAUuudwYKnZuLsOviWhpU"
type: "plan"
status: "approved"
title: "Issue #333 CI-OPT-06 実装計画（Shadow Mode + 集約 gate）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/333"
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
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-333-impact-based-ci.md"
  implemented-by:
    - "docs/tasks/issue-333-t1-impact-classifier.md"
    - "docs/tasks/issue-333-t2-ci-gate-shadow-wiring.md"
  depends-on:
    - "docs/designs/issue-333-impact-based-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-333-impact-based-ci.md"
  derived-by:
    - "docs/tasks/issue-333-t1-impact-classifier.md"
    - "docs/tasks/issue-333-t2-ci-gate-shadow-wiring.md"
    - "docs/test-specs/issue-333-impact-based-ci.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-333-impact-based-ci.md"
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #333 実装計画 — Shadow Mode 導入

## Scope split

| Task | 内容 | 依存 |
| --- | --- | --- |
| T1 | `ci/impact/` — classifier + map + gate の実装と `node --test` fixture 検証 | なし |
| T2 | `ci.yml` へ `impact`/`ci-gate` job 追加（Shadow Mode）+ `pull_request` types 拡張 + mise task + AGENTS.md 同期 | T1（gate スクリプトの CLI 契約確定） |

## File map

- `ci/impact/map.mjs`: 分類規則データ（class glob・lane 定義・full trigger・
  lane-owner 導出ヒント）。コメント付きの reviewable data module。
- `ci/impact/classify.mjs`: CLI。`--event/--base/--head/--files/--all/--out`
  を取り decision JSON を出力。GITHUB_OUTPUT があれば decision/expected/report
  を emit。
- `ci/impact/gate.mjs`: CLI。`--needs-json <file|->`（`toJson(needs)` の
  JSON）と `--expected`（impact の expected 出力 JSON または decision JSON
  パス）を受け、判定表を stdout + summary ファイルに出力し exit code で
  Pass/Fail を返す。
- `ci/impact/tests/classify.test.mjs` / `ci/impact/tests/gate.test.mjs`:
  fixture ベースの契約テスト（後述 matrix）。
- `.github/workflows/ci.yml`: `on.pull_request.types` 追加・`impact` job・
  `ci-gate` job 追加。既存 6 job は無変更。
- `mise.toml`: `[tasks.test-ci-impact]` 追加。
- `AGENTS.md`: ci.yml の job 構成説明（`impact`/`ci-gate`・trigger types）と
  検証入口表の `test-ci-impact` 行を追記。

## 実装ノート

- classifier の crate 走査: `cargo metadata --no-deps --format-version 1`
  の `packages[].{name,manifest_path,dependencies[].name}` で member 間 DAG
  を構築（`path` 依存のみ member エッジとして扱う）。workspace member 名と
  `crates/<name>` の対応は `manifest_path` 由来（`crates/ruviform-x` ⇔
  `ruviform-x` — name と dir は一致しているが、不一致でも manifest_path で
  正しく写像する）。
- rt_ スキャン: `crates/<name>/tests/**`・`src/**` 内の `fn rt_` 出現
  （`#[test]` 注釈有無を問わない保守的検出 — 誤検出は偽陽性側にのみ働く）。
- lane-owner 導出: mise.toml の `[tasks.<name>]` `run` 文字列から
  `-p <crate>` / `--features <f>` を regex 抽出。タスク構造が変われば
  抽出失敗 → その lane は常時 expected（fail-open owner 規則どおり）。
- ui pkg DAG: `ui/*/tsconfig*.json` の `include`/`paths` に含まれる
  `../<pkg>`、`ui/*/src|test|page/**/*.ts` の `from "../<pkg>/` import、
  mise task `sources`/`depends` の `ui/<pkg>` を union。
- gate の expected 欠落時: `decision.json` を artifact から得られない場合は
  `needs.impact.outputs.expected`（compact lane→bool JSON）。両方無ければ
  classification-error → 全 lane expected として照合 + Fail。

## Risk register

- `needs` context の `toJson` サイズ・出力上限: lane 定義は固定 6+1 で
  上限に達しない。report 全文は artifact 併用。
- PR の merge commit が checkout される形で base fetch が必要: `git fetch
  --depth 1 origin $BASE_SHA` 後 two-dot diff（merge-base 不要の tree diff）。
- `node --test ci/impact/` の test discovery: `*.test.mjs` を自動検出
  （Node 26 既定）。Windows/macOS でも同一（`impact` job は ubuntu）。
- `edited` type 追加で title 編集時にも全 lane が走る — 代償は concurrency
  cancel で吸収（設計どおり受容）。

## Verification matrix

| 検証 | 手段 |
| --- | --- |
| 分類規則の正しさ | `classify.test.mjs` fixture: crate 変更/逆依存/docs-only/ui-pkg/shared-config/unknown/空 diff |
| Full fallback | fixture: Cargo.lock・mise.toml・.github・未分類パス |
| gate 判定表の全網羅 | `gate.test.mjs`: expected×actual 全組合せ + classification error |
| workflow 構文 | `mise exec -- actionlint` + prek check-yaml |
| Shadow の実運転 | 本 PR の CI run で `impact`/`ci-gate` が緑・summary 出力を観測（IR/exp に記録） |

## Rollback

- 本 PR は lane 実行条件を変えないため revert しても required check 構成・
  実行内容は従前どおり。将来 Stage D 以降の rollback は design の runbook 節。

## 関連文書

- 要求: [docs/specs/issue-333-impact-based-ci.md](../specs/issue-333-impact-based-ci.md)
- 設計: [docs/designs/issue-333-impact-based-ci.md](../designs/issue-333-impact-based-ci.md)
- 検証仕様: [docs/test-specs/issue-333-impact-based-ci.md](../test-specs/issue-333-impact-based-ci.md)
- タスク: [docs/tasks/issue-333-t1-impact-classifier.md](../tasks/issue-333-t1-impact-classifier.md) / [docs/tasks/issue-333-t2-ci-gate-shadow-wiring.md](../tasks/issue-333-t2-ci-gate-shadow-wiring.md)
