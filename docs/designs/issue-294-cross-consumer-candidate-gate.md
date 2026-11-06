---
id: "DESIGN-034S9rV6yB3dG7jL0nQ4sU"
type: "design"
status: "proposed"
title: "Issue #294 Cross-Consumer候補tuple Gate 設計 — 証拠契約・解決SHA捕捉・UI lane 段階接続・task 接続点"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  changes:
    added:
      - type: "directory"
        path: "ci/consumer-verify"
      - type: "file"
        path: "docs/discovery/issue-294-cross-consumer-candidate-gate.md"
      - type: "file"
        path: "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/discovery/issue-294-cross-consumer-candidate-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
  defers: []
  deferred-by: []
---

# Issue #294 Cross-Consumer候補tuple Gate 設計 — 証拠契約・解決SHA捕捉・UI lane 段階接続・task 接続点

## Context

`consumer-verify.yml` は候補 Ruviform SHA を scratch checkout へ注入し両 Consumer の mise lane を走らせる dispatch 専用 lane として健全に動作している（synvelis pass / audiolens test-ci fail の実績）。#367 scheduler が「期待 SHA を入力で渡す」dispatch 契約を `ci.yml` に対して確立済みで、本設計は同じ思想を Consumer 側に拡張する — **判定側が期待 tuple を入力し、検証側が実測 tuple を証拠として返す**。scheduler 自身・失敗 Issue 起票・実機 lane は別責務（Out of Scope）で、本設計はその入力・証拠契約のみを実装する。

## Scope

### In Scope

- `.github/workflows/consumer-verify.yml` の拡張（job 構造・既存 lane 意味論は維持）
- `ci/consumer-verify/evidence.mjs`（lane 記録・tuple 照合・status 導出・証拠 JSON/サマリ生成）と `ci/consumer-verify/tests/`（stdlib-only Node、`ci/daily`/`ci/summary` と同一構成）
- `mise.toml` `test-ci-consumer-verify`、`docs/development-environment.md` 対応表

### Out of Scope

- `ci/daily/*`・`daily-ci.yml`（#367）、`ci-issue-tracker*.yml`（#368）、`ci.yml`、Consumer repo 内ファイル
- macOS/実 OS/GPUI Driver-neutral 実 GUI/Native Plugin Dock lane の追加（GUI-CONTRACT #390・#313/#314・#388/#389・正式受入フェーズ。GUI v2: 新規独立 GUI workflow・PR 必須 GUI gate は設けない）

## Components and Boundaries

### 1. `ci/consumer-verify/evidence.mjs`

判定コアは副作用のない pure function 群、I/O は `main()` のみ（`ci/summary`・`ci/daily` と同型）。3 コマンド:

| コマンド | 振る舞い |
| --- | --- |
| `classify-ref <input>` | `sha`（40-hex）/`ref`（branch/tag 安全形）/`invalid` を stdout に出力。workflow の入力検証と checkout 分岐に使う |
| `lane --file <jsonl> --name <s> --command <s> --feature <s> --os <s> --outcome <o> [--reason <s>]` | lane 記録を JSONL に 1 行追記。outcome ∈ `pass`/`fail`/`not_run`/`unsupported`/`skipped` |
| `finalize --file <jsonl> --out <json> [--summary <md>] [--status-out <file>]` | tuple（env）＋lanes を読み、status を導出し、証拠 JSON・Markdown 表・`status=<s>` 行を出力 |

#### tuple 環境変数（finalize が読む）

| env | 由来 | 空時 |
| --- | --- | --- |
| `RUVIFORM_REV` | workflow env（40-hex 検証済み） | 必須・欠落は error |
| `CONSUMER_NAME` / `CONSUMER_REPO` | job 固定（`audiolens`/`synvelis`） | 必須 |
| `REQUESTED_REF` | `inputs.<consumer>_ref` | 必須 |
| `RESOLVED_SHA` | checkout 後 `git rev-parse HEAD` 実測 | 空可（`null` 記録。EXPECTED_SHA 非空なら fail-closed で `shaMatch=false`） |
| `EXPECTED_SHA` | `inputs.<consumer>_expected_sha` | `null`（照合なし=記録のみ） |
| `ACCESS_MODE` | gate step が解決（`deploy_key`/`anonymous`/`none`） | `none` 扱い |
| `GATE` | `run`/`not_run`/`blocked` | `not_run`→status=`not_run`、`blocked`（preflight 失敗）→status=`fail` |
| run メタ | `GITHUB_RUN_ID`/`GITHUB_RUN_ATTEMPT`/`GITHUB_SERVER_URL`/`GITHUB_REPOSITORY`/`GITHUB_EVENT_NAME` | 欠落は `null` 記録（test 環境許容） |

`shaMatch` は `EXPECTED_SHA` と `RESOLVED_SHA` から finalize 側で導出する（step output の bool を信頼しない — env 二重経路の drift を防ぐ）。EXPECTED 非空で RESOLVED が未捕捉でも `shaMatch=false`（一致を立証できないものは不一致として fail-closed）。

#### status 導出（単一権威・fixture 固定）

優先順:

1. `GATE=blocked` → `fail`（reason=`preflight_failed`）
2. `GATE=not_run` → `not_run`（reason=`no_access_path` 等、gate 由来）
3. `EXPECTED_SHA` 非空かつ解決 SHA が不一致（または未捕捉）→ `fail`（reason=`tuple_mismatch`、expected/resolved を記録。検証 lane は実行されない）
4. いずれかの lane `fail` → `fail`
5. ≥1 lane `pass` かつ `fail` なし → `pass`（`unsupported`/`not_run`/`skipped` lane は `coverage` 欄で残存記録。Pass の内訳が透明でなければ pass にしない）
6. lane が `unsupported` のみ（≥1）→ `unsupported`
7. 上記以外（全 `not_run`/`skipped`・lanes 空）→ `not_run`

**生成規則の固定**: `unsupported` は workflow が静的に宣言する capability-absent 行（実窓 host・macOS/実 OS）でのみ生成し、lane/task の実行失敗を `unsupported` へ再分類しない（実行して落ちたものは `fail`）。`skipped` は GitHub の step `outcome=skipped`（上流 lane 失敗・gate 不達）をそのまま写す派生記録に限る。`unknown`・未記録 lane が残っても pass に寄与しない。

#### 証拠 JSON（`consumer-evidence-<name>.json`）

```json
{
  "schemaVersion": 1,
  "kind": "consumer-verify-evidence",
  "contract": "consumer-verify-tuple/v1",
  "run": {"workflow","runId","runAttempt","runUrl","event","repository"},
  "candidate": {"ruviformRev": "<40hex>"},
  "consumer": {"name","repo","requestedRef","resolvedSha","expectedSha","shaMatch"},
  "accessMode": "...",
  "tuple": {"ruviformRev","consumerResolvedSha","schemaVersion":1},
  "lanes": [{"name","command","feature","os","outcome","reason"}],
  "status": "...",
  "statusReason": "...",
  "coverage": {"pass":n,"fail":n,"notRun":n,"unsupported":n,"skipped":n},
  "generatedAt": "<iso>"
}
```

`shaMatch` は `expectedSha` 未提供時 `null`（照合なし≠成功）。`contract` 文字列は tuple の「検証契約バージョン」要素として証拠に含める。秘密・token・key 内容は一切書き込まない（env 参照は allowlist のみ）。

#### Markdown サマリ（`--summary`）

`## <Consumer> — ruviform <rev> (access: <mode>)` ＋ tuple 表（requested ref / expected / resolved / match）＋ 従来形式の lane 表（lane/OS/feature/ruviform revision/結果/許容例外/Rollback）を出力。従来の summary 形式は人間可読正本として維持する。

### 2. `.github/workflows/consumer-verify.yml`

#### 入力追加（`workflow_dispatch` のみ。trigger 集合は変更しない）

| input | 既定 | 用途 |
| --- | --- | --- |
| `ruviform_rev` | `github.sha` | 不変 |
| `audiolens_ref` / `synvelis_ref` | `main` | branch/tag/**40-hex SHA** を受理（説明文更新） |
| `audiolens_expected_sha` / `synvelis_expected_sha` | 空 | 期待 Consumer SHA。非空なら 40-hex 必須、解決 SHA と fail-closed 照合 |
| `audiolens_extra_tasks` / `synvelis_extra_tasks` | 空 | 追加 mise task（カンマ区切り、`^[A-Za-z0-9][A-Za-z0-9_-]*$`・最大 8）。RF #306 等の Consumer 側 test command 接続点 |

#### concurrency

```yaml
concurrency:
  group: consumer-verify-${{ inputs.ruviform_rev || github.sha }}-${{ inputs.audiolens_ref }}-${{ inputs.synvelis_ref }}
  cancel-in-progress: false
```

同一組合せの重複 run を直列化（打ち切りで証拠喪失しない）。scheduler の抑制と二重化しない補助線。

#### preflight job（拡張）

1. `actions/checkout`（**ref 指定なし = dispatch 対象 ref、すなわち `github.sha`**）— evidence.mjs を workflow 自身の revision から供給する。候補 SHA（`ruviform_rev`）では**ない**: script 導入前の候補 SHA を検証する run でも script は存在しなければならず、検証対象と検証器の revision を混ぜない。
2. `mise install --locked node`（ruviform pin toolchain）→ 入力検証: `ruviform_rev` 40-hex、各 `_ref` は `classify-ref` が `sha`/`ref` を返すもの、各 `_expected_sha` は空または 40-hex、extra_tasks は `^[A-Za-z0-9][A-Za-z0-9_-]*$` カンマ区切り・最大 8。違反は preflight fail（lane は `fail (preflight failed)` を報告する既存経路）。
3. `mise run test-ci-consumer-verify`（**script 自己検証**: 壊れた証拠器が誤 status を出さない。`daily-ci`/`ci-issue-tracker` と同じ型）。
4. credential/anonymous probe（不変）。

#### verify 各 job（構造共通・`timeout-minutes: 120`）

```
checkout ruviform（actions/checkout 既定 = dispatch ref。evidence.mjs 供給）
→ gate（preflight gate・access mode 解決。blocked でも exit 1 せず証拠 finalize へ回す）
→ SSH credential / mise setup + ruviform 側 `mise install --locked node`（evidence.mjs 実行用）
→ clone consumer（変更: git init + remote add + fetch --depth 1 origin <ref-or-sha> + checkout --detach FETCH_HEAD）
→ resolve-sha: git rev-parse HEAD → outputs.resolved_sha。EXPECTED_SHA 非空 && 不一致
   → outputs.tuple_ok=false + lane 記録 fail(tuple_mismatch)。以降の検証 lane は全 skip
→ dep access probe（不変）→ manifest rev 注入 sed（不変）→ cargo fetch + grep guard（不変）
→ lanes（後述）→ extra_tasks（入力時のみ、各 task を lane 記録）
→ record-evidence（if: always()）: lane 行を evidence.mjs lane で JSONL 化
   → finalize で証拠 JSON+Job Summary+status output
→ upload-artifact（if: always()、attempt 固有名、if-no-files-found: error）
→ cleanup credential（if: always() && deploy_key。git/ssh 使用 step はこの後に置かない）
```

- **checkout は job 先頭 step**: gate=blocked/not_run の終端経路でも evidence.mjs が使えるよう、credential 判断より前に置く。
- `git fetch origin <sha> --depth 1`: GitHub は ref 非公開（unadvertised）commit の want も受理する（protocol v2）— 境界はサーバ側オブジェクト存在であり、GC 済み・一度も push されていない SHA のみ fetch 失敗（→ 正直な fail 証拠）。ref 名と同一経路で統一し、解決 SHA を必ず捕捉する。`--branch` clone では記録できない SHA を得る。
- `checkout --detach FETCH_HEAD` で tag/branch/SHA を同一 post-state に揃える。
- clone/resolve 失敗は `resolved_sha` 空で記録され、期待 SHA 提供時は fail-closed で `shaMatch=false` → `fail`。clone 不能を pass に偽装しない。
- **artifact の全終端経路保証**: Job Summary は REST API で機械再取得できないため、evidence JSON は gate=not_run・tuple_mismatch・workspace 不在・lane 途中失敗の全経路で emit する（record step と upload step の `if: always()` が要件）。artifact 名は `consumer-evidence-<name>-attempt-${{ github.run_attempt }}` で rerun 衝突を避ける。
- **credential ordering**: `sync-ui-deps` は内部で ruviform への `git fetch` を行うため、git/ssh を使う step は全て cleanup step より前（deploy_key 復帰時の順序不具合を防ぐ）。upload-artifact・cleanup 後段は git/ssh 不使用。

#### lane 集合（既定実行・いずれも既存 mise 入口）

**verify-audiolens**（fetch 後、従来 lane に追加）

| lane | 入口 | 補足 |
| --- | --- | --- |
| manifest rev pin + cargo fetch | 既存 | grep guard 不変 |
| `check-ruviform-lock` | `mise run` | Consumer 側 authoritative 単一 SHA 検証（#140 lockfile 正本） |
| `check-ci` | 既存 | |
| smoke（`al-cli` ci build + `check-cli-headless`） | 既存 | |
| `test-ci`（LFS fixtures） | 既存 | `git lfs pull` 後 |
| `sync-ui-deps` | `mise run` | ui/vendor を候補 SHA で再生成（cargo checkout 依存） |
| `check-ruviform-rev --require-vendor` | `mise exec -- node tools/check-ruviform-rev.mjs --require-vendor` | lock+REV 整合・vendor 必須（AL 側規約の lane 用法） |
| `setup-ui` → `check-ui` → `test-ui` | `mise run` | npm ci → tsc typecheck+vendor 照合 → node:test(happy-dom) |

**verify-synvelis**（fetch 後、従来 lane に追加）

| lane | 入口 | 補足 |
| --- | --- | --- |
| manifest rev pin + cargo fetch | 既存 | grep guard 不変 |
| `check-ruviform-rev` | `mise run` | lockfile 単一 SHA（vendor 未生成時点では lock のみ検証） |
| `check` | 既存 | |
| `test`（LFS fixtures） | 既存 | `git lfs pull` 後 |
| `test-rt` | `mise run` | RT no-alloc 系 lane（RT 証拠 AC の headless 側） |
| `check-headless-deps` | `mise run` | headless 依存 guard |
| `sync-ui-deps` → `setup-ui` | `mise run` | vendor 再生成 + npm ci |
| `check-ui` → `test-ui` → `build-ui` | `mise run` | tsc / vitest(happy-dom,実 SolidJS render) / vite build+assets manifest |
| `check-ruviform-rev --require-vendor` | `mise exec -- node tools/check-ruviform-rev.mjs --require-vendor` | lock+REV+assets manifest の完全整合（manifest は build-ui 産物のため **必ず build-ui 後**。事前実行は manifest 検証が vacuous） |

- UI lane は fetch 成功のみに依存（rust check/test lane と独立に証拠を採る — rust lane 失敗でも UI 証拠が残る）。
- GUI 系 lane の固定行は未接続と capability 不足を分離する（skip=pass にしない・unsupported 混同もしない正直記録）:
  - `not_run`（capability は足りるが検査入口が未 wiring — Owner と再実行条件を記録）: `GUI-CONTRACT checks (public API/禁止依存/allowlist — GPUI 不要)`（Owner: RF #390）と `GUI semantic/fake-oracle checks`（Owner: GUI-SEMANTIC + Consumer oracle、AL #166/#127・SV #175-177）。Consumer 側専用 mise task 接続時に `extra_tasks`/専用 lane で再実行。`extra_tasks` を指定してもこれら固定行の分類は固定のままで、extra lane 行として別記録される（再分類は Owner 判断）。
  - `unsupported`（capability 不足）: `GUI visual/OS + 実 window lanes`（RF #313/#314）、SV `PLUGIN-NATIVE-DOCK (実 plugin binary)`（RF #388/#389 + SV #176 — Floating/Generic fallback 成功を Pass にしない）、`macOS/実 OS lanes`。
- node ツール導入: AL lane は既に `mise install --locked rust git-lfs node`。SV lane も `rust git-lfs node` へ拡張（npm 系 UI task の前提）。

#### extra_tasks 接続点

- 1 step が入力 task を順に `mise run <task>` で実行し、各 task を個別 lane 行として記録（途中失敗でも後続 task を試行して全 lane の outcome を残す。任一失敗で step 自体 fail）。
- 未入力時は step 自体スキップされ lane 表にも出ない（skip≠pass）。preflight が pattern/件数を検証済み。

#### summary job（拡張）

- 従来の Consumer×status 表に加え、tuple 表（ruviform rev・両 Consumer resolved/expected sha・match）を出力（各 job の `outputs` 経由。未取得は `unknown` と明示）。
- 不変条件は維持: job failure/cancelled/preflight fail → workflow 失敗。全 Consumer `not_run`/`unsupported`（pass ゼロ）→ workflow 失敗。`unsupported` は pass でも fail でもない証拠語彙として集計に出す。

### 3. `mise.toml` / 文書

```toml
[tasks.test-ci-consumer-verify]
description = "Run ci/consumer-verify contract tests via `node --test ci/consumer-verify/` (classify-ref, lane/status derivation matrix, tuple match/mismatch, evidence JSON+summary shape, workflow yml contract; requires node from the pinned toolchain)"
run = "node --test ci/consumer-verify/"
```

`docs/development-environment.md` の対応表に追記。`ci/daily` と同様に `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` で workflow 構造契約（必須 inputs・lane step 存在・evidence/artifact 経路・`on:` が workflow_dispatch のみ）を textual に固定する。

## Data and Control Flow

```
workflow_dispatch(ruviform_rev, *_ref, *_expected_sha, *_extra_tasks)
  └─ preflight: 入力検証(classify-ref) → checkout@RUVIFORM_REV → test-ci-consumer-verify(自己検証) → probe access
       ├─ verify-audiolens ─┐
       └─ verify-synvelis ──┤
            gate(mode) → checkout ruviform@REV → clone consumer(fetch FETCH_HEAD)
            → resolve-sha → tuple_match(expected? 照合)
                 mismatch → evidence fail(tuple_mismatch) → artifact
            → sed 注入 → cargo fetch → grep guard
            → rust lanes / ui lanes / extra_tasks(各 lane JSONL 記録)
            → finalize → consumer-evidence-<name>.json + step summary + status
            → upload-artifact / cleanup
       └─ summary: 両 job の status+resolved_sha を tuple 表に集約
            · 任一方 fail/job failure/preflight fail → workflow fail
            · pass ゼロ（全 not_run/unsupported）→ workflow fail（証拠ゼロ）
            · ≥1 pass かつ fail なし → workflow success（not_run/unsupported は記録として残る）
```

**fail-closed 不変条件**:
- `pass` を主張できるのは「その Consumer job が実行 lane を持ち、全実行 lane が pass」のときのみ。gate 不達・tuple 不一致・clone 不可・lane fail・証拠生成失敗のいずれも pass にしない。
- `not_run`/`unsupported`/`skipped`/`unknown` は証拠語彙として記録されるが pass に計上されず、pass ゼロの run は workflow fail。
- 期待 tuple との不一致は lane 実行自体を止める fail（別 tuple の緑を期待 tuple の証拠に流用しない）。

## Non-Functional Constraints

- stdlib-only Node（`ci/summary`・`ci/daily` 踏襲）。YAML パーサ依存を作らない（contract test は textual）。
- 秘密の非露出: 既存の deploy_key/known_hosts pin/cleanup 構造は不変。evidence JSON・summary・ログに env から token 系を転写しない。extra task 名は pattern 検証でシェル注入を排除。
- runner 加重は段階的: 新 lane は node/npm 系が主で rust lane の増分なし。artifact は JSON 1 ファイル/consumer（小）。
- Consumer 独立 CI・独立 daily と重複した機構・証拠 DB を作らない。証拠の再利用は同一候補 revision・同一条件の完全証拠のみ。

## Risks and Trade-offs

- **`git fetch origin <sha>` の受理範囲**: GitHub は ref 非公開 commit の fetch も受理する（unadvertised-object wants over protocol v2）。境界はサーバ側オブジェクト存在 — GC 済み・未 push の SHA は fetch 失敗 → clone step fail → `fail` 証拠として正直に出る。depth 1 のため merge-base 等の深い履歴は取らない（不要）。
- **npm ci の runner 依存**: Consumer の `package-lock.json` は repo コミット済みで再現性あり。npm registry 障害時は lane fail として記録（ruviform 側責務ではないが evidence に残る）。
- **Consumer task の将来変更**: workflow は Consumer の mise task 名に依存する。task 改名/削除は lane `fail`/`not_run` として可視化され、静かな pass 化はしない。extra_tasks 接続点は新規 task を workflow 編集なしで接続できる。
- **checkout ruviform の追加コスト**: verify 各 job で dispatch ref の浅い checkout（既定 depth 1）のみ。候補 SHA 自体の存在は consumer 側 `cargo fetch` の rev pin 解決で検証される（候補が消えていれば fetch fail → 正直な fail 証拠）。
- **status 導出を JS に集約する代償**: workflow に checkout 依存が増えるが、bash 二重実装（AL/SV で drift）を排除し fixture test で契約化できる利点が上回る。checkout は dispatch ref 固定のため「script が存在しない候補」という破綻はない。
- **extra_tasks は operator 信頼入力**: pattern 検証は shell 注入の排除であり task の意味論は制約しない（例: `dev-ui` のような対話/重 task も投入可能）。job `timeout-minutes` が唯一の暴走境界 — これは任意 branch ref（=任意 mise.toml）を選べる既存入力と同一の露出であり、workflow_dispatch 前提では許容範囲と判断。`skipped`/`not_run` に倒さず失敗は `fail` で記録する。
- **concurrency の限界**: 同一 group の pending は最大 1 件で、3 件目以降の同時 dispatch は queued 側がキャンセルされる。重複抑止の正本は #367 scheduler であり、本 concurrency は直列化の補助線に留める（dedup を過大にうたわない）。
- **deploy key と Consumer 制御コードの共存（受容リスク）**: `sync-ui-deps` 等の git 消費 step が終わるまで key は runner temp に残り、同じ checkout 内の Consumer 制御スクリプト（npm postinstall・build script）は原理的に key path を読み得る。deploy key は該当 Consumer repo への read-only で、workflow_dispatch 前提＋ref は operator 指定のため露出は許容と判断。緩和として key は runner temp のみに置き git-credential へは残さず、git/SSH 消費 step 完了直後に cleanup step を固定する。
- **job cancel/timeout 中の証拠欠落（受容リスク）**: `timeout-minutes` 超過・runner 喪失・手動 cancel では `if: always()` step も走らず lane 途中で JSON が残らないことがある。この場合 tuple は artifact 不在＝未検証として扱われ、#367 scheduler の「履歴不明は再実行」ルールで回収される — 欠落は pass に偽装しないため fail-closed と整合。

## Verification Notes

- [x] `mise run test-ci-consumer-verify`（`node --test ci/consumer-verify/`）— 61 tests / 0 fail。classify-ref 行列、status 導出全枝（pass/fail/tuple_mismatch/not_run/unsupported/coverage/blocked）、証拠 JSON schema・`shaMatch` 三値、summary 形状、extra-task lane 記録、非 run gate での invalid SHA 受容、lanes 行 schema 検証（forged outcome 拒否）、`consumer-verify-yml-contract`（必須 inputs・trigger 限定・rev-parse 捕捉・resolution `always()`・全 step id↔rec 対応・`tuple_ok` ゲート・lanes ファイル truncate・extra backstop・result env pin・always() 証拠経路・finalize 劣化経路・attempt 固有 artifact・unsupported ≥2 行・GUI gate の not_run/unsupported 分離・concurrency group が tuple 全要素を key 化・`inputs.*` の run: 非混入）を含む。
- [x] `mise exec -- actionlint`（全 workflow、consumer-verify.yml を含む）— エラー 0。検出→修正済み: `runner` context が `jobs.<job>.env` で使用不可だったため runner.temp 派生 path を無条件先頭 step の `$GITHUB_ENV` 供給へ変更。
- [x] `mise exec -- prek run --all-files`（check-yaml・trailing-whitespace 等）— 全 pass。
- [x] fetch+FETCH_HEAD checkout の機構 smoke — temp repo で branch 名・40-hex SHA の双方を `git init + remote add + fetch --depth 1 + checkout --detach FETCH_HEAD` → `rev-parse HEAD` で解決できることを確認（SHA fetch は `uploadpack.allowAnySHA1InWant` を立てたローカル remote で再現 — GitHub は既定で有効）。
- [x] 実 runner 検証（review 基準 head `30205f2` で実測 — 修復後 head は別候補であり本結果を正式 Pass へ転用しない）: coordinator dispatch [run 42302538464](https://github.com/scarecrow173/ruviform/actions/runs/42302538464)（`gh workflow run --ref feature/consumer-verify-gui-candidate-gates`、候補 RF `30205f2` + AL `04f1861` + SV `d8bf8ca` の期待 SHA 照合付き）— preflight success・両 Consumer の clone/expected-SHA 照合 success・verify-synvelis 全 lane **pass**・verify-audiolens は `check-ci lane` で **fail**（`SinkConfig.span_events` 欠落の実 API 不整合を検出 — gate 設計通り fail-closed）・summary の not-run-is-not-pass 強制が正しく発火。
- [x] 秘密非露出: evidence JSON・summary に KEY/token 系の転写がないことを code review＋contract test（env allowlist）で確認。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 |
| --- | --- | --- |
| scheduler 側の tuple 判定・consumer-verify dispatch | #367 責務（本設計が供給する入力・証拠契約を利用） | Issue #365/#367 |
| tracker による consumer-verify 失敗 Issue 起票 | #368 責務（監視 workflow 拡大は別 Issue） | Issue #368 |
| macOS runner・実 GUI Gate lane | RF #313/#314 と正式受入フェーズ。現状は `unsupported` 記録で追跡 | Issue #294 本体 |
| Consumer ci.yml 側の `expected_sha` 契約 | AL/SV 独立 CI の責務（reuse 時の証拠利用は同一 tuple 条件のみ） | 各 Consumer repo |
