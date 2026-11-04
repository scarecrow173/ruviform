---
id: "DESIGN-034RBIy0yxPWYhXdXUlsLi"
type: "design"
status: "approved"
title: "Issue #330 CI-OPT-03 UI npm install 集約・mise task freshness 設計"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/330"
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
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
  implements:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
  implemented-by:
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
  depends-on:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-310-browser-gui-test-harness.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
    - "docs/discovery/issue-330-ui-npm-ci-inventory.md"
  derived-by:
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
    - "docs/test-specs/issue-330-ui-npm-ci-dedup.md"
  verifies: []
  verified-by: []
  references:
    - "mise.toml"
    - ".github/workflows/ci.yml"
    - "ui/showcase/test/deps.test.ts"
    - "ui/browser-test/tsconfig.json"
    - "ui/showcase/tsconfig.build.json"
  defers: []
  deferred-by: []
---

# Issue #330 CI-OPT-03 UI npm install 集約・mise task freshness 設計

## Context

`ui-verify` ジョブは同一 OS runner 上で mise タスクを逐次実行する。現行は各 `check-ui-*` / `test-ui-*` / `build-ui-*` タスクの `run` 先頭に `npm --prefix ui/<pkg> ci --no-audit --no-fund` が内包されており、1 ジョブあたりの `npm ci` 実行は以下のとおり（実行順序は ci.yml の step 順）。

| package | 回数 | 内訳 |
| --- | --- | --- |
| ui/core | 2 | check-ui-headless, test-ui-headless |
| ui/components | 3 | check, test, build |
| ui/viz | 3 | check, test, build |
| ui/binding | 3 | check, test, build |
| ui/guitest | 2 | check, test |
| ui/showcase | 5 | check, test, build + check-ui-browser-test / test-ui-browser の `depends = ["build-ui-showcase"]` 経由 2 回 |
| ui/browser-test | 2 | check-ui-browser-test, test-ui-browser |
| **計** | **20** | |

Showcase dist は同一ジョブ内で 3 回（build ステップ + 2 回の depends 経由）`tsc`  emit される。いずれも lockfile / source が同一 revision なので結果は等価だが、実行時間は失われる。

パッケージ間の実依存は `ui/showcase/test/deps.test.ts` が正本として監査している（src 間 import は相対パスのみ、bare specifier 禁止）:

```
core ─── components ─── viz
core ─── binding
guitest ── core, binding          （test-only 基盤。showcase 系とは独立）
showcase ── core, components, viz, binding   （src 相対 import）
browser-test ── showcase/dist     （@rvf/showcase → ../showcase/dist paths）
```

dist を必要とする下流は `ui/browser-test` のみであり、その参照先は `ui/showcase/dist`（`tsconfig.build.json` が rootDir `..` で core/components/viz/binding の src を含めて emit した複合 dist）。components / viz / binding の個別 dist は他 package から消費されず、「package が単体で emit 可能であること」の検証成果物である。

## Scope

### In Scope

- mise タスク構造の再編: package-lock 駆動の `install-ui-*` タスク新設、verify タスクから `npm ci` 除去、dist 生成タスクへの `depends`・freshness 付与。
- `.github/workflows/ci.yml` の `ui-verify` ジョブへ準備ステップ（`mise run setup-ui`）追加。
- `AGENTS.md` の検証入口テーブル・lane 記述の同期。

### Out of Scope

- npm workspaces 化、lockfile 統合、node_modules の actions/cache 化。
- Playwright ブラウザ取得・OS system deps の変更。
- 検証ステップの分割単位・順序・Evidence の変更。
- 独自スクリプトによる freshness 判定（mise 標準機能の外に実装しない）。

## Components and Boundaries

### `install-ui-<pkg>` タスク（7 本、新設）

`ui/{core,components,viz,binding,guitest,showcase,browser-test}` それぞれに対応。`run` は `npm --prefix ui/<pkg> ci --no-audit --no-fund` 1 行のみ。

freshness 宣言:

- `sources = ["ui/<pkg>/package-lock.json", "ui/<pkg>/package.json"]` — lockfile と manifest の双方を入力とする。manifest のみの変更でも install を再実行し、`npm ci` 自身が lock/manifest 不整合を fail 報告できるようにする。
- `outputs = ["ui/<pkg>/node_modules/.package-lock.json"]` — `npm ci` が必ず書き出す hidden lockfile を成果物マーカーとする。

mise の freshness 判定（安定版 freshness check。artifact cache 実験機能は使わない）では、全 output 存在・最新 output が最新 source より新しい・source 一覧と各 size/mtime が前回成功時の記録と一致、の 3 条件を満たす場合のみ skip する。`node_modules` 削除・lockfile 更新・manifest 更新のいずれでも stale となり install が再実行されるため、準備済み状態の正当性は lockfile revision に対して保守的に保たれる。

### `setup-ui` タスク（新設、CI 準備ステップ）

`depends` に 7 本の `install-ui-*` を列挙する集約タスク（`run` なし）。`ui-verify` ジョブは cargo cache 復元後に `mise run setup-ui` を 1 ステップ実行し、job 内の install をここに集約する。後続タスクの `depends` 解決では各 install が `sources up-to-date, skipping` となり、`npm ci` は実行されない。

### verify 系タスク（既存名維持、`run` から `npm ci` 除去 + `depends` 付与）

| task | depends | run（従来どおり検証のみ） |
| --- | --- | --- |
| check-ui-headless / test-ui-headless | install-ui-core | typecheck / test |
| check/test/build-ui-components | install-ui-components | typecheck / test / build |
| check/test/build-ui-viz | install-ui-viz | typecheck / test / build |
| check/test/build-ui-binding | install-ui-binding | typecheck / test / build |
| check/test-ui-guitest | install-ui-guitest | typecheck / test |
| check/test-ui-showcase | install-ui-showcase | typecheck / test |
| build-ui-showcase | install-ui-showcase | build（+ freshness、後述） |
| check-ui-browser-test | install-ui-browser-test, build-ui-showcase | typecheck |
| test-ui-browser | install-ui-browser-test, build-ui-showcase | build + playwright install + test |

check/test タスクに freshness は付けない — 検証は入力が同一でも常に実行する義務がある（freshness で test を skip すると検証能力を削る）。

### dist 生成タスクの freshness（build-ui-components / viz / binding / showcase）

`build-ui-*` は emit 副作用を持つため `sources` / `outputs` を付与する。`build-ui-showcase` は 5 package の src を横断 compile するため:

- `sources = ["ui/showcase/src/**/*.ts", "ui/showcase/test/**/*.ts", "ui/showcase/tsconfig*.json", "ui/core/src/**/*.ts", "ui/components/src/**/*.ts", "ui/viz/src/**/*.ts", "ui/binding/src/**/*.ts"]`
- `outputs = ["ui/showcase/dist/**/*"]`

これにより `check-ui-browser-test` / `test-ui-browser` の `depends` 解決で showcase build は lockfile・source 未変更なら skip され、同一ジョブ内で 1 回の emit となる。`build-ui-components` / `build-ui-viz` / `build-ui-binding` も同型（include する sibling src を sources に含める）。

### `ui-verify` ジョブ差分

`Restore cargo registry/git index cache` の後、webview 系 step の前に `- name: Install UI npm dependencies (mise run setup-ui)` を挿入する。既存 step の分割・順序・`Upload UI browser evidence` は不変。`Install Playwright browser system deps (Linux)` は `npm --prefix ui/browser-test exec -- playwright install-deps` で browser-test の node_modules を必要とするが、setup-ui 段階で導入済みとなるため従来どおり機能する。

## Data and Control Flow

CI（1 OS）:

```
setup-ui
  └─ install-ui-{core,components,viz,binding,guitest,showcase,browser-test}
       → npm ci ×7（lockfile ごとに 1 回）
check-ui-headless → depends: install-ui-core(skip) → typecheck
test-ui-headless  → depends: install-ui-core(skip) → test
...（各 lane 同型）...
build-ui-showcase → depends: install-ui-showcase(skip) → tsc emit ×1
check-ui-browser-test → depends: install-ui-browser-test(skip) + build-ui-showcase(skip) → typecheck
test-ui-browser → depends: 同上 → page build → playwright install → test
```

ローカル単独実行（Fresh checkout で `mise run test-ui-headless`）: `depends` 解決で install-ui-core が node_modules 非存在ゆえ stale → `npm ci` 実行 → test 実行。タスク名・実行入口は従来互換。

Warm 再実行（lockfile 未変更）: install は `sources up-to-date, skipping`、verify のみ走る。

## Non-Functional Constraints

- 信頼境界: `npm ci` は lockfile 厳格性（lock と manifest の一致、lock 非改変）を担う。freshness skip は「前回成功 install 以後 lockfile/manifest が不変」な場合に限り許され、判断を npm の整合検査に肩代わりさせない。
- 可観測性: mise は skip 時に `[task] sources up-to-date, skipping` を出力する。CI ログ上で「install しなかった」ことが明示的に観測できる。
- 可搬性: freshness 判定は mise 側実装であり OS 非依存。`npm ci`・`node_modules/.package-lock.json` 生成は npm 標準挙動で 3 OS 同一。
- Rollback: 変更は mise.toml + ci.yml + AGENTS.md のみ。revert で旧構造に即復帰可能（検証能力の欠落なし）。

## Risks and Trade-offs

- リスク: mise task-state（`~/.local/state/mise/task-sources`）が self-hosted runner 等で job 跨ぎ永続化されると「前回 install 成功」の記録が残り得る。対策: freshness は output 実在性も条件に含めるため、node_modules が job 開始時に存在しない環境では必ず install が走る。GitHub-hosted runner（現行）は job ごとに新規 VM で非永続。
- リスク: `node_modules` 部分破損（lockfile 不変・`.package-lock.json` 残存・中身欠損）は skip を通り得る。対策: `npm ci` は清浄 install であり部分破損を前提にしない運用（現行も同じ前提）＋ 破損兆候があれば `mise run --force` で強制再実行可能。
- リスク: `mise.toml` は task 定義ファイル自身が暗黙 source となるため、本変更の初回実行は全 install が 1 度走る。conservative であり問題なし。
- トレードオフ: verify タスクが `depends` 経由で install を暗黙実行するため、「install なしで typecheck だけ」には `install-ui-*` を stale にしない運用が前提となる。従来も常に npm ci していたので後退なし。
- トレードオフ: `build-ui-*` の freshness は `dist` 部分削除を検出しない（output glob が空でなければ fresh 扱い）。dist 全体が無ければ stale になるため実害は限定的で、完全性は従来の tsc incremental と同等。

## Deferred Design Concerns

- npm workspaces による lockfile 一本化 | install 回数を 7→1 に更に圧縮可能だが、package 単位の private lockfile・`npm ci` 厳格性・publish 境界の再設計を伴う | install が依然 critical path を支配する場合 | 7 lockfile 運用の更新コスト継続。
- node_modules actions/cache | Fresh job の install 自体を削減可能だが lockfile hash key・部分復元の整合設計が必要 | CI-OPT-00 計測で install 支配率が高いままの場合 | キャッシュ不良時の fail モード追加。

## Verification Notes

- [x] mise freshness 挙動の実測（scratch project で sources/outputs + depends の skip・stale・output 欠落時再実行を確認）
- [x] `node_modules/.package-lock.json` が `npm ci` で生成されることの実測
- [ ] Fresh checkout 相当（node_modules 全削除）で全 UI タスクが依存解決込みで完走
- [ ] Warm 再実行で install が全 skip されること
- [ ] `test-ui-showcase`（deps.test.ts 含む）が不変で緑であること
