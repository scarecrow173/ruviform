---
id: "DESIGN-oq5cpmBZrL3fUcNnuUzPxg"
type: "design"
status: "approved"
title: "Issue #333 CI-OPT-06 Impact Classifier・集約 Required Gate・段階移行設計"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "docs/specs/issue-333-impact-based-ci.md"
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
  implemented-by:
    - "docs/plans/issue-333-impact-based-ci.md"
  depends-on:
    - "docs/discovery/issue-333-impact-classifier-inventory.md"
    - "docs/specs/issue-333-impact-based-ci.md"
    - "docs/adr/cross-repo-ci-revision.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/designs/issue-331-check-clippy-test-dedup.md"
    - "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-333-impact-based-ci.md"
    - "docs/discovery/issue-333-impact-classifier-inventory.md"
  derived-by:
    - "docs/plans/issue-333-impact-based-ci.md"
    - "docs/test-specs/issue-333-impact-based-ci.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-333-impact-based-ci.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #333 設計 — 変更影響ベース検証 DAG・Fail-Closed Required Gate

## Overview

2 つの新 job を `ci.yml` に追加する。

- **`impact`**: 変更ファイルを分類し「期待 lane 集合」（どの lane が走る
  べきか）を `decision` JSON として出力する分類器 job。全 trigger で実行。
- **`ci-gate`**: 全検証 lane の `needs` 結果と `impact` の期待集合を照合する
  集約ゲート job。安定名で、将来の required context 単位。

本 PR は **Shadow Mode**: lane job の `if:` は一切変更せず全 lane が実行され、
`impact` の分類結果は証拠出力のみ。`ci-gate` は end-state と同一の判定
ロジックを既に持ち、non-required job として実運転で観測される。選択実行の
有効化・Ruleset 移行は偽陰性なし確認後の後続手順（「段階移行 runbook」節）。

### 設計上の核心判断

1. **gate は mode を持たない純粋関数** — `expected`(impact 出力) と
   `actual`(`needs` results) の照合のみ。Shadow 期は「全 lane 実行される」
   事実と照合され、enforce 期は「skip された lane = NotApplicable」と照合
   される。同じコードが両期で正しい。
2. **分類規則はリポジトリから機械導出** — crate 逆依存（`cargo metadata`）、
   RT テスト保持 crate（`fn rt_` スキャン）、feature lane 対象 crate
   （mise task の `-p` 引数）、ui パッケージ DAG（tsconfig/相対 import/
   mise sources）。静的リスト化すると進化で陳腐化し偽陰性を生むため、
   導出不能時は Full CI に退化させる（fail-closed）。
3. **緊急停止は repo variable のみ** — `CI_IMPACT_DISABLE=1` で classifier が
   expected=ALL に退化。`CI_SCCACHE_DISABLE=1` と同型で、workflow 編集・
   revert なしに選択判定を無効化できる。

## Lane ↔ 検証責務対応表（現状固定値）

| lane (job id) | context 名 | OS | 検証内容（mise task） |
| --- | --- | --- | --- |
| `verify` | `verify (<os>)` | ubuntu/windows/macos | `prek run --all-files` + `fmt-check` + `lint-ci` + `test-ci`。workspace 全 member・default feature・全 target 種の compile/lint/test と言語非依存 hook（docs の whitespace/EOF を含む） |
| `rt-verify` | `rt-verify` | ubuntu | `test-rt` = `cargo test --workspace rt_`（rt_ プレフィックス RT 契約テスト + 空選択 reject） |
| `icc-verify` | `icc-verify` | ubuntu | `test-video-proc-icc` + `lint-video-proc-icc`（`ruviform-video-proc --features icc`） |
| `ui-verify` | `ui-verify (<os>)` | ubuntu/windows/macos | `check/lint/test-ui-webview`（Rust feature lane）+ `check/test/build-ui-{components,viz,binding,showcase}` + `check/test-ui-{headless,guitest}` + `check-ui-browser-test` + `test-ui-browser`（実 Chromium） |
| `l1-real-verify` | `l1-real-verify (<os>)` | ubuntu/windows/macos | `test-l1-real-plugin` + `lint-l1-real-plugin`（plugin-clap/vst3 `--features host` + `#[ignore]` E2E） |
| `bench` | `bench` | ubuntu | `bench-smoke` = `cargo bench -p ruviform-math -p ruviform-rt -p ruviform-viz -- --test` |

## 変更クラス → 期待 lane 契約

分類器の**正本契約**。classifier は下表を `ci/impact/map.mjs` のデータ +
機械導出で実装し、本節がレビュー可能な意味論の owner とする。

### クラス定義（path → class）

| class | 判定（glob・先勝ちではなく全マッチの union） | 期待 lane |
| --- | --- | --- |
| `shared-config` | `Cargo.toml`・`Cargo.lock`・`mise.toml`・`mise.lock`・`.mise/**`・`rust-toolchain*`・`.cargo/**`・`deny.toml` | **Full CI**（共有 toolchain/依存解決の変更は全 lane に影響し得る） |
| `ci-config` | `.github/**`・`.pre-commit-config.yaml`・`ci/**` | **Full CI**（CI 自身・分類器自身の変更は自己診断できないため全実行） |
| `repo-config` | `.gitattributes`・`.gitignore`・`.editorconfig`・`apm.yml`・`apm.lock.yaml`・`.agents/**`・`.claude/**`・その他ルート直下の隠しファイル/設定 | **Full CI**（共通設定・曖昧さ回避） |
| `crate:<name>` | `crates/<name>/**`（`<name>` は workspace member 名と照合） | `verify` + `<name>` が lane-owner である lane + 逆依存 crate の lane（後述） |
| `ui-pkg:<name>` | `ui/<name>/**`（`<name>` は ui/ 直下の npm パッケージ） | `ui-verify` + 逆依存パッケージ経由（全て `ui-verify` に収斂） |
| `docs` | `docs/**`・ルート `*.md`（`README.md`/`AGENTS.md` 等）・`LICENSE*` | `verify` のみ（prek の言語非依存 hook が唯一効く lane） |
| `poc` | `poc/**` | `verify` のみ（workspace 外・cargo lane は到達しない・prek のみ意味を持つ） |
| `unknown` | 上記いずれにも一致しないパス | **Full CI** |

**補足**:
- `crates/<name>/Cargo.toml` 単体の変更（deps/feature 追加）も `crate:<name>`
  として扱い、lane 拡大は逆依存規則に委ねる。feature 集合を変え得る変更は
  対象 feature lane が lane-owner 経由で既に expected になる。
- `ui/` 直下でパッケージ外のファイル（例: 将来の共有設定）は `ui-pkg`
  に一致しなければ `unknown` → Full CI。
- `ci/impact/**` は `ci-config`（Full CI）— 分類器自身の変更は必ず
  全 lane で検証される。
- リネーム/削除で残った旧パスも --name-only のパスとして同規則で分類。

### 依存逆方向伝播規則

1. **Cargo crate**: `cargo metadata --no-deps --format-version 1` で
   workspace member と member 間依存を取得し、変更 crate の**全推移的
   逆依存先**（その crate に依存する member を再帰的に）を impacted set
   に加える。cargo metadata 実行不能・解析失敗 → 伝播を断念せず Full CI。
2. **ui パッケージ**: ui/<pkg> 間依存を以下 3 源の union で機械導出:
   (a) tsconfig `include`/`compilerOptions.paths` の `../<pkg>` 参照、
   (b) `ui/<pkg>/{src,test,page}/**` の `../<other-pkg>/` 相対 import、
   (c) mise task `sources`/`depends` の `ui/<pkg>` 参照。変更 pkg の
   推移的逆依存先を impacted に加える（いずれも lane は `ui-verify`）。
3. **crate→lane-owner 写像**（crate が変わった/impacted 時に追加される lane）:
   - `rt-verify`: `fn rt_` を `tests/`・`src/` に持つ crate（実リポジトリ
     スキャンで導出 — rt 契約テスト追加時に静かに追従）。
   - `icc-verify`: `test-video-proc-icc`/`lint-video-proc-icc` task の
     `-p`/`--features` 引数から導出（現行 `ruviform-video-proc`）。
   - `l1-real-verify`: `test-l1-real-plugin`/`lint-l1-real-plugin` task の
     `-p` 引数から導出（現行 clap/vst3）。
   - `ui-verify`（Rust 側）: `check/lint/test-ui-webview` task の `-p`
     引数から導出（現行 `ruviform-ui-webview`）。
   - `bench`: `bench-smoke` task の `-p` 引数から導出（現行 math/rt/viz）。
   - 上記いずれの導出も失敗した場合、その lane は「常に expected」に退化
     （fail-open lane-owner）。全導出が壊れても Full CI にはならない —
     壊れた写像だけ安全側に倒す。
4. **verify の全包含**: workspace member の変更は `verify` を必ず含む
   （全 member を compile/test する lane なので、lane-owner 写像に無い
   crate でも verify は必要）。
5. **impacted crate への lane-owner 適用**: 逆依存先 crate が lane-owner
   ならその lane も expected（例: `ruviform-plugin-core` 変更 → 逆依存に
   clap/vst3 → `l1-real-verify` expected。`ruviform-ui` 変更 → 逆依存に
   ui-webview/ui-test → `ui-verify` expected）。
6. **Full CI fallback**: (a) shared-config/ci-config/repo-config/unknown の
   1 ファイルでも存在、(b) 分類不能・空 diff・diff 取得失敗、
   (c) cargo metadata 失敗、のいずれかで `decision: "full"` — 全 lane
   expected。`decision` と理由は出力に必ず記録する。

### 期待集合の表現

lane 選択単位は **job 粒度**（matrix leg 分割は行わない）。各 lane は
`{expected: bool, oses: [...], features: [...], reasons: [...]}` を持つ。
`features` は lane が検証する feature（`icc`/`webview`/`host`）を監査用に
保持する情報フィールドで、選択性には使わない。

## `decision` JSON 出力契約

```jsonc
{
  "schemaVersion": 1,
  "event": "pull_request",        // or "push" | "workflow_dispatch" | "local"
  "base": "<sha>",                // diff 基準（非 PR では before sha or null）
  "head": "<sha>",
  "decision": "selective",        // or "full"
  "decisionReasons": ["..."],     // full 時の理由（例: "shared-config: Cargo.lock"）
  "impactDisabled": false,        // CI_IMPACT_DISABLE=1 反映
  "files": [{"path": "crates/ruviform-math/src/lib.rs", "classes": ["crate:ruviform-math"]}],
  "cratesTouched": ["ruviform-math"],
  "cratesImpacted": ["ruviform-math", "ruviform-dsp"],
  "uiPackagesTouched": [],
  "lanes": {
    "verify":         {"expected": true,  "oses": ["ubuntu-latest","windows-latest","macos-latest"], "features": [], "reasons": ["crate:ruviform-math"]},
    "rt-verify":      {"expected": true,  "oses": ["ubuntu-latest"], "features": [], "reasons": ["rt-tests:ruviform-math"]},
    "icc-verify":     {"expected": false, "oses": ["ubuntu-latest"], "features": ["icc"], "reasons": []},
    "ui-verify":      {"expected": false, "oses": ["ubuntu-latest","windows-latest","macos-latest"], "features": ["webview"], "reasons": []},
    "l1-real-verify": {"expected": false, "oses": ["ubuntu-latest","windows-latest","macos-latest"], "features": ["host"], "reasons": []},
    "bench":          {"expected": true,  "oses": ["ubuntu-latest"], "features": [], "reasons": ["bench:ruviform-math"]}
  }
}
```

`expected: false` の lane は分類上「影響なしが証明できた」場合のみ —
すなわち lane-owner でも impacted 逆依存先にも無い。これが `NotApplicable`
判定の唯一の根拠になる。

## 集約ゲート `ci-gate` 判定規則

`ci-gate` は `needs: [impact, verify, rt-verify, icc-verify, ui-verify,
bench, l1-real-verify]` + `if: always()` で常に実行し、以下の純粋判定を
行う（`expected` は `needs.impact.outputs.expected`、無ければ
classification-error 扱い）:

| expected | actual result | 判定 | 説明 |
| --- | --- | --- | --- |
| true | success | **Pass** | 期待どおり検証され成功 |
| true | failure / cancelled | **Fail** | 期待 lane が失敗/中断 |
| true | skipped | **Fail** | 期待 lane が実行されず required 検証欠落 |
| false | skipped | **NotApplicable** | 分類上影響なしが証明され実行されず |
| false | success | **Pass（over-executed）** | 不要 lane が実行 — 失敗ではないが記録（分類と実行のずれを監査） |
| false | failure / cancelled | **Fail（false-negative 検出）** | 実行された lane の失敗は常に真の信号。Shadow 期はこれが偽陰性メーターになる |
| — | impact job 失敗/出力欠落 | **Fail（classification error）** | 期待集合が証明不能 → 全 lane expected に退化させて照合した上で fail |

- `NotRun`/`Unsupported`/`NotApplicable` の語彙: `skipped` した lane は
  「分類証明済みなら NotApplicable、証明なしなら NotRun（Fail）」。
  `Unsupported`（環境が lane を実行不能）は現行全 lane ubuntu/windows/macos
  標準 runner で発生しないが、将来 lane が非対応 runner を持つ場合のために
  `reasons`/`notes` に明示記録する語彙として契約に残す。
- 判定表は job summary（`GITHUB_STEP_SUMMARY`）に lane×OS×expected×actual×
  判定×理由の表で出力し、`gate-report.json` を artifact 化する。
  これが「期待集合/実結果/対象外理由が PR 上で監査可能」の実装。
- **Shadow 期の実効**: 全 lane が必ず実行されるため actual が skipped に
  なることはなく、判定は「全 lane Pass or 失敗伝播」+「false/failure =
  偽陰性シグナルの表面化」となる。enforce 有効化後は同一コードが
  skipped↔NotApplicable を処理する。

## ci.yml 差分設計

```yaml
# pull_request の activity type に edited・ready_for_review を追加。
# edited は base retarget（再分類必須）を捕捉するため — title/body 編集でも
# 発火する代償は concurrency group による前回 run の cancel で吸収。
on:
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened, ready_for_review, edited]

jobs:
  impact:
    name: impact-classify
    runs-on: ubuntu-latest
    outputs:
      decision: <full|selective>
      expected: <compact JSON lane->bool>
      report: <decision JSON 全文>
    steps:
      - checkout（PR: fetch-depth で head、base sha を追加 fetch）
      - mise + rust + node install（classifier は cargo metadata を使う）
      - node --test ci/impact/（分類器の自己検証 — 壊れた分類器で判定しない）
      - node ci/impact/classify.mjs --event <event> --base <sha> --head <sha>
        --out impact-decision.json → GITHUB_OUTPUT へ decision/expected/report
      - upload artifact impact-decision + step summary 表

  ci-gate:
    name: ci-gate
    needs: [impact, verify, rt-verify, icc-verify, ui-verify, bench, l1-real-verify]
    if: always()
    runs-on: ubuntu-latest
    steps:
      - checkout（gate.mjs 実行のため）
      - mise + node
      - node ci/impact/gate.mjs --needs "$NEEDS_JSON" --expected ...
        → summary 表 + gate-report.json artifact + 判定で exit code
```

- `impact` は既存 lane と `needs` 関係を持たず並列に走る（critical path
  に乗らない）。`ci-gate` は全 lane 終了後にのみ走る非対称点で、
  critical path への追加コストは数秒（checkout + node 実行のみ・rust 不要）。
- `impact` の diff: `pull_request` では `pull_request.base.sha` vs
  `github.sha`（merge commit = base+branch の合成ツリー）の two-dot
  `git diff --name-only` で PR 変更を得る。`push` は `event.before` vs
  `github.sha`（decision は常に full だがファイル記録は証拠に残す）。
  `workflow_dispatch` は diff なしで full。
- 既存 6 job・12 context は一切無変更。`impact`/`ci-gate` は required
  contexts に含めない（この時点では観測用の非 required job）。

## mise.toml 差分設計

```toml
[tasks.test-ci-impact]
description = "Run ci/impact classifier + gate contract tests via node --test"
run = "node --test ci/impact/"
```

`impact` job が起動時にこの task を実行して分類器を自己検証する。

## 段階移行 runbook（後続作業の正本）

1. **Stage A（本 PR）— Shadow Mode**: `impact`+`ci-gate` 追加・全 lane 無変更・
   12 required contexts 維持。`ci-gate` は非 required。
2. **Stage B — Shadow 観測期**: N run 分の `gate-report.json` を収集し、
   「`expected:false` の lane が `failure/cancelled`」= 偽陰性 0 件を確認。
   偽陰性が出たら分類契約を修正して観測をやり直す（この間 skip は絶対に
   有効化しない）。
3. **Stage C — Ruleset 移行（無保護期間なし手順）**:
   a. `main-protect` ruleset に `ci-gate` を required context として**追加**
      （12 件 + 1 件 = 13 件の状態。既存 PR は ci-gate check が無いため
      merge 時に ci-gate が pending — 対象 PR は base との merge/rebase で
      ci-gate を持つ run を得るまで merge 不能。これは意図する fail-closed
      であり、追加直後に影響 PR 一覧を runbook で確認する運用とする）
   b. 移行 PR（lane `if:` 有効化）を作成 — この PR は新 required `ci-gate`
      を含む workflow を持つため自己完結的に検証される。
   c. 移行 PR merge 後、`ci-gate` が全新規 run で報告されることを確認して
      から 12 件を required から**除去**（除去は `ci-gate` 追加後に限定 —
      順序を逆にすると required context が空または片欠の無保護期間になる）。
4. **Stage D — lane `if:` 有効化 PR**:
   ```yaml
   verify:
     needs: [impact]
     if: always() && needs.impact.outputs.lane-verify != 'false'
   ```
   - `if:` は「明示的に不要と判定された時だけ skip」（`!= 'false'`）にし、
     impact 失敗・出力欠落では lane が**実行される側**に倒す（fail-open
     coverage）。その上で `ci-gate` が classification error を fail とする
     ため fail-closed 性は gate 側に保持される。
   - `push`/`workflow_dispatch` では impact が expected=ALL を出すため
     `if:` は常に真 — 全実行維持がコードで保証される。
5. **Rollback**:
   - 即時: repo variable `CI_IMPACT_DISABLE=1` → classifier は expected=ALL を
     出力（`if:` は全 lane 実行）。code revert 不要。
   - 恒久: Stage D PR の revert。`ci-gate`+12 required の併用期に戻すかは
     ruleset 操作で調整（12 件再追加は `ci-gate` を残したまま可能 — 同じ
     「追加→確認→除去」順序を逆方向に適用）。
   - `ci-gate` 自体の除去: 12 件 required を復活させてから `ci-gate` を
     required から外す（同じく追加→除去の順序で無保護期間を作らない）。

## 失敗モードと復旧

- **classifier の誤判定（偽陽性）**: 影響のない lane まで expected → 実行
  コスト増のみ、merge 安全性は不変。分類規則を絞って修正。
- **classifier の偽陰性**: 必要 lane を expected:false → Shadow 期に
  `ci-gate` の `false/failure` 行で検出。検出されない偽陰性（その lane が
  緑だったため見えない）は理論上残るが、実害は lane が本当に fail する
  場合のみ — Stage B の観測で実質 0 確認を acceptance にする。
- **`impact` job 自体の失敗**: lane は fail-open で全実行、`ci-gate` は
  classification error で Fail — 赤い gate が修正を促す。
- **`ci-gate` の誤判定**: 非 required 期に実運転観測で潰す（本 PR 自身の
  run が最初のデータ点）。
- **artifact/output の欠落**（impact 成功だが expected 出力空）:
  classification error 同様 Fail。

## Edge cases

- **PR base 変更（retarget）**: `edited` type で run が発火し、新 base.sha
  で再分類される。`ci-gate` の summary には base/head SHA を記録し、
  stale check との照合を可能にする。
- **synchronize（push 追従）**: 新 merge commit で再分類。concurrency
  group が旧 run を cancel — cancel された lane は `cancelled` result を
  残すが、その run の `ci-gate` は `if: always()` で走り Cancelled を
  Fail と記録する（観測として正しい; merge をブロックする required
  check ではない）。
- **空 diff / merge のみの PR**: diff が空 → decision=full（証拠として
  「empty diff」を記録）。
- **新規 crate/ui パッケージ追加を含む PR**: cargo metadata / ui スキャンに
  自動的に現れる → lane-owner 写像が自然に追従する。
- **crates/ 直下の新ディレクトリで workspace member でない**:
  `crate:<name>` 照合に失敗 → `unknown` → Full CI。
- **巨大 diff / バイナリファイル**: --name-only はパスのみ返すため影響なし。
- **LFS 対象ファイル変更**: パス class 規則どおり分類（CI は LFS を取得
  しないため通常どおり lane が走る）。
- **fork からの PR**: `impact`/`ci-gate` は `contents: read` のみで外部
  書き込み権限を必要としない。

## Alternatives（検討済み・不採用）

- **lane 内 step 条件付き skip**（job は常に success で内部 step を skip）:
  required context は緑を返すが「何も検証していない成功」を量産し、
  監査時に skip 理由と結果の対応が取れない。job 粒度 + gate 集約の方が
  正直で ruleset 移行も単純。
- **dorny/paths-filter**: glob→boolean 部分のみで、逆依存伝播・lane 契約・
  監査 JSON は結局自前。third-party action pin のコストに見合わない。
- **静的 lane-owner 表**: 33 crate の静的メンテは crate 追加・task 変更で
  陳腐化し静かな偽陰性を生む。機械導出を採用。
- **gate が checks API で per-leg 詳細を取得**: `checks: read` 権限追加と
  API 依存を伴う。matrix `needs` result の job 粒度で契約は十分に成立し、
  per-OS 内訳は各 leg 自身の check run が持つ。不採用。
- **`pull_request` types に `edited` を入れない**: base retarget で再分類
  されず stale 期待集合が残る。コスト（title 編集での再実行）より正確性を
  取る。

## 関連文書

- 要求: [docs/specs/issue-333-impact-based-ci.md](../specs/issue-333-impact-based-ci.md)
- 前提調査: [docs/discovery/issue-333-impact-classifier-inventory.md](../discovery/issue-333-impact-classifier-inventory.md)
