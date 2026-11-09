---
id: "DESIGN-034UF2Q9OH59VxiQZUfUgN"
type: "design"
status: "draft"
title: "Issue #219 Cross-Repo 影響判定・CI Gate 運用設計 — Consumer 棚卸し・影響分類テンプレート・候補 Gate・切り分け/保留復旧・M0-M7 証拠可視化"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-219-cross-repo-impact-gate-operations.md"
    modified:
      - type: "doc"
        path: "docs/discovery/issue-219-consumer-cross-repo-ci.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/adr/cross-repo-ci-revision.md"
    - "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-294-daily-tuple-trigger.md"
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/designs/issue-275-observability-integration-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
    - "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/81"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
  defers: []
  deferred-by: []
---

# Issue #219 Cross-Repo 影響判定・CI Gate 運用設計 — Consumer 棚卸し・影響分類テンプレート・候補 Gate・切り分け/保留復旧・M0-M7 証拠可視化

## Context

Issue #219「Synvelis・AudioLens 並行開発の互換契約と Cross-Repo CI Gate」のうち、headless で完結する設計・文書 slice の運用設計正本。Ruviform 変更ごとに両 Consumer（Synvelis = DAW、AudioLens = 高精度オフライン音声解析）への影響分類・受入証拠を運用可能にする基盤を、既存 canonical 文書と整合する形で一箇所に集約する。

**権威関係（本書は再決定しない）:**

| 関心事 | 正本 |
| --- | --- |
| 方式決定（Deploy Key・rev 正規化・dispatch lane） | [ADR cross-repo-ci-revision](../adr/cross-repo-ci-revision.md) |
| 要件（R1–R10・AC1–AC12） | [spec issue-219-consumer-compat-contract](../specs/issue-219-consumer-compat-contract.md)（proposed） |
| gate 実装設計・credential runbook | [design issue-219-cross-repo-ci-gate-credential-runbook](issue-219-cross-repo-ci-gate-credential-runbook.md)（approved） |
| 候補 tuple 証拠契約・UI lane・task 接続点 | [design issue-294-cross-consumer-candidate-gate](issue-294-cross-consumer-candidate-gate.md)・[design issue-294-daily-tuple-trigger](issue-294-daily-tuple-trigger.md)・[design issue-294-gui-contract-lane-wiring](issue-294-gui-contract-lane-wiring.md) |
| M0〜M7 実行順・G0〜G7 受入・Cross-Consumer 受入条件 | [design issue-193-migration-execution-plan](issue-193-migration-execution-plan.md)（approved）+ Issue #193 本文（実行権威） |
| 棚卸し・実測証拠 | [discovery issue-219-consumer-cross-repo-ci](../discovery/issue-219-consumer-cross-repo-ci.md)（append-only Evidence） |
| CI 計測 Baseline | [design issue-327-cross-repo-ci-baseline](issue-327-cross-repo-ci-baseline.md)（approved） |
| **本書** | 上記を横断する**運用設計の統合正本**（draft — review 待ち）。個別の方式決定・要件・実装詳細は上記正本に委ね、重複記述は参照で代替する |

**現状の要約（2026-11-10 時点・詳細は Components §1）:** consumer-verify.yml・PR/Issue template は実装済みで手動 dispatch 運用中。両 consumer repo は #365 型の dispatch-only CI + daily scheduler 構造を導入済み。audio-lens は Cargo.lock 正本契約（#140）へ移行完了、synvelis は 11 member workspace・20 git dep の branch 追従形。M1 着手 Gate の #199/#201 は両方 CLOSED。

## Scope

### In Scope

- 両 Consumer の依存形態・required features・runner/toolchain・Git アクセス形態の棚卸し（2026-11-10 read-only snapshot を正本化）
- Ruviform 変更 PR での両 Consumer 影響分類・リンク先記録の運用設計（影響なし根拠を含む）
- Cross-Repo CI Gate の運用設計（候補 revision・lane 構成・access・秘密管理・trigger/cadence・保留/復旧）— 実 workflow の新規導入・変更は対象外
- 不適合時の切り分け責務（Ruviform 修正 vs Consumer Adapter 更新 vs Consumer 側潜在欠陥）と保留/復旧手順
- M0〜M7 Consumer 別受入証拠リンク・未実測事項の見える化設計（M1 Gate #199/#201 の現状を飛ばさない記述）

### Out of Scope

- 実 CI workflow の新規導入・変更・実実行、private credential の設定、実 Consumer repo での build/test/smoke 実測、3 OS matrix、Golden/baseline lane の実行（いずれも NotRun — Verification Notes 参照）
- 各 Capability 実装（#184〜#198 等）・M0〜M7 Gate 判定・Consumer repo 自体への変更
- consumer-verify の required check 昇格・自動 trigger 有効化（spec R9/Deferred — 実測後判断）
- #294 が Owner の tuple/UI/Golden lane 拡張、#275/#281 が Owner の Observability Gate 実装

## Components and Boundaries

### 1. Consumer 棚卸し（正本: discovery `## Evidence` 2026-11-10 snapshot）

運用設計上の要点のみを転記する。測定値の正本は discovery の dated snapshot であり、本表は設計判断に必要な差分だけを保持する。

| 観点 | Synvelis | AudioLens |
| --- | --- | --- |
| 依存形態 | git URL ×20・`rev`/`branch` 未指定の**既定ブランチ追従**。解決 SHA の記録は Cargo.lock が担う（consumer AGENTS.md「SHA 恒久固定禁止」規約） | git URL ×17・**manifest pin 非保持**。Issue #140 完了形で **Cargo.lock `source` fragment が正本**。`tools/ruviform-lock-rev.mjs`（`check-ruviform-lock`）が単一 SHA を fail-closed 検証、`tools/update-ruviform-rev.mjs` が追従更新入口 |
| lockfile 解決 | 26 ruviform crate @ `0f5b5737`（main 比 30 commit 遅れ・2026-11-10） | 20 ruviform crate @ `84d21641`（main 比 44 commit 遅れ・同） |
| required features | 観測上 non-default feature なし | `ruviform-io-symphonia` `["mp4","aac"]`・`ruviform-observability` `["logging"]` |
| toolchain/runner | mise `rust 1.99.0` + `node 26.12.1`、workspace `rust-version 1.89`、edition 2024 | mise `rust 1.99.0` + `node 26.12.1` + `uv 0.13.5` + `biome 2.6.2`、CI profile（`--profile ci`）task 群 |
| 検証 lane（consumer 側） | `check`/`test --locked`/`test-rt`/`check-headless-deps`/`gw10`/`sync-ui-deps`/`check-ruviform-rev`/`check-ui`/`test-ui`/`build-ui` | `check-ci`/`test-ci`/`check-cli-headless`/`check-ruviform-lock`/`check-ruviform-rev`/`sync-ui-deps`/`check-ui`/`test-ui`/`check-ui-webview`/`baseline` |
| CI 構造 | `ci.yml` dispatch-only + `expected_sha`、`daily-ci.yml`（cron + decide→dispatch）、`ci-issue-tracker*` — ruviform #365/#367/#368 と同型 | 同左（cron 時刻のみ相違） |
| repo アクセス | public・匿名 HTTPS read 可。`deploy_key` mode が private 復帰時の canonical 経路 | 同左（`.cargo/config.toml` の `git-fetch-with-cli`/`GIT_LFS_SKIP_SMUDGE` 存続） |

**運用上の含意:**

- 依存形態が consumer ごとに異なる（branch 追従 vs lockfile 正本）ため、consumer-verify の manifest 正規化は 3 形態を同一ルールで `rev` 注入する設計（runbook 2026-11-03 追記）。audio-lens の lockfile 正本契約との整合（scratch 注入 `rev` が consumer 側 `check-ruviform-rev` 系検査に与える影響）は**未実測の観測事項**であり、次回実 dispatch で確認する（Verification Notes の NotRun 項目）。
- 両 consumer が daily-ci + expected_sha の dispatch-only CI を持つため、「candidate tuple の日次検証」は consumer 側 daily と ruviform 側 consumer-verify の**二層**で構成される。重複機構は作らず、consumer-verify は ruviform 候補 revision 起点の横断証拠、consumer daily-ci は consumer main 起点の独立 CI として役割を分ける（#294 deferred「Consumer ci.yml 側の expected_sha 契約」と同じ境界）。
- 棚卸しは dated snapshot の追記で更新する。consumer repo の状態が変わった場合（dep 形態・member・task・visibility）は本表を直接書き換えず、discovery に新 snapshot を追記して本表との差分を確認する。

### 2. 影響判定テンプレート運用設計

記録形式の正本は `.github/PULL_REQUEST_TEMPLATE.md`（実装済み）と spec R1/R2/R10。本節は運用面の契約を固定する。

**分類語彙と記録欄:**

- Consumer 別分類: `impacted` / `no impact` / `unknown`（Synvelis・AudioLens を独立に評価 — 片側の結果を他方へ外挿しない）
- 証拠語彙: `pass` / `fail` / `not run`（evidence JSON では `unsupported`/`skipped` も使用。`not run` と `fail` を区別し、`unknown` を `pass` に見せない）
- PR 本文の必須欄: `## Consumer 影響分類` 表（Consumer × 分類 × 根拠・対応 Issue/PR リンク）+ `## 検証証拠` 表（対象・lane/test・OS・feature・ruviform revision・結果・許容例外・Rollback）
- `impacted` は対応 Issue/PR リンク必須、`no impact` は根拠必須（根拠なしの no impact は認めない）、`unknown` は未検証理由と解消先（起票 Issue または consumer-verify dispatch 予定）必須

**分類の実施責務:**

- PR 著者（Ruviform 変更者）が分類を記入する。機械検査 lane（`consumer-impact-check.yml`）は Issue #366 Stage 1 で撤去済みのため、**現状の強制力はレビュアー human gate のみ** — merge 前に分類欄の記入・語彙・`unknown` 解消先リンクをレビュアーが確認する運用を維持する（runbook 設計どおり required check 化は scope 外）。
- 分類対象の変更種別: 公開 API/feature/behavior/dependency/CI 基盤の変更。`ui/*` 共通基盤を含む差分は spec R10 の Consumer 固有評価軸（Synvelis = 制御/Transport/RT 非阻害、AudioLens = f64・NaN 欠測・Validity/Provenance・bounded Dataset 可視化）を適用する。
- Issue 運用（spec R2）: 共通契約の変更・不具合は Ruviform Capability Issue、製品固有の Adapter/移行/回帰対応は Consumer Issue（継続追従: Synvelis #28・AudioLens #127、Observability 系: Synvelis #81・AudioLens #138）へ起票し相互リンク。影響なし時は Issue を乱発せず根拠のみ記録する。

### 3. Cross-Repo CI Gate 運用設計

実装・方式の正本は runbook 設計 + #294 設計群 + `consumer-verify.yml`。本節は運用契約のみを固定する。

**候補 revision の選定:**

- `ruviform_rev`（既定 = dispatch 対象 ref の `github.sha`）が候補 Ruviform SHA。各 verify job は `git rev-parse HEAD` で consumer 解決 SHA を捕捉し、`*_expected_sha` 非空時は fail-closed で tuple 照合（不一致は `fail (tuple_mismatch)` — 別 tuple の緑を期待 tuple の証拠に流用しない）。
- `*_ref`/`*_expected_sha` は branch/tag/40-hex SHA を受理（#294 で拡張済み）。証拠契約は `consumer-verify-tuple/v1`（evidence JSON + artifact `consumer-evidence-<name>-attempt-<n>`）。

**lane 構成（2026-11-10 時点の workflow 実装）:**

- 両 consumer 共通: manifest 正規化（3 形態 sed）→ `cargo fetch` + 単一 SHA guard（repo URL キー・fail-closed）→ consumer 側 mise lane → 証拠 JSON/summary/artifact。
- audio-lens: `check-ruviform-lock` → `check-ci` → smoke（`al-cli` ci build + `check-cli-headless`）→ `test-ci`（LFS fixture 後）→ `sync-ui-deps` → `check-ruviform-rev --require-vendor` → `setup-ui`/`check-ui`/`test-ui` → GUI-CONTRACT boundary guard・GUI-SEMANTIC fake driver contract（**候補 tree**）→ `extra_tasks`。
- synvelis: `check-ruviform-rev` → `check` → `test`（LFS）→ `test-rt` → `check-headless-deps` → `sync-ui-deps` → `setup-ui`/`check-ui`/`test-ui`/`build-ui` → `check-ruviform-rev --require-vendor` → 同 GUI 2 lane（候補 tree）→ `extra_tasks`。
- check/test/smoke は実装・実走済み。**Golden/baseline 計測 lane は未接続**（#294 が Owner・Deferred — 実行時間と fixture 要件が未測のため）。GUI の semantic Pass・pixel Golden・実 OS・Native Dock・RT/PCM・f64/CLI 精度は別 Gate であり、本 lane の pass をもって両者 Pass としない（Issue 本文の証拠分離原則）。

**private repo アクセスと秘密管理:**

- access mode は lane ごとに `deploy_key`（canonical・private 前提）/ `anonymous`（public 期間の補完）/ `not_run` を解決し、証拠表 `access:` に記録。Deploy Key は read-only・3 repo 登録・`CROSS_REPO_DEPLOY_KEY` secret のみ経由。
- 秘密のログ露出防止: 鍵は `RUNNER_TEMP` のみ・echo/artifact/`git config` 残留禁止・known_hosts は GitHub 公表 fingerprint の workflow 内定数 pin・cleanup（`url.insteadOf` 除去→鍵削除）は `if: always()` で git/ssh 消費 step の後に固定。evidence JSON/summary は env allowlist のみ転写し token 系を含まない（#294 contract test で固定済み）。
- `permissions: contents: read`、third-party action への秘密委譲なし、`GITHUB_TOKEN` も cross-repo に不使用。

**trigger / cadence / 実測トリガ方針:**

- trigger は `workflow_dispatch` のみ（自動 trigger・required 昇格は spec R9 Deferred — 初回実測と credential 稼働確認後に判断）。
- 運用上の dispatch 契機（runbook 2026-11-03 追記 2 件目）: (a) `impacted` 分類 merge 後の確認、(b) M-gate 受入判定前の証拠収集、(c) `unknown` 解消、(d) consumer 追従 PR 前の事前互換確認、(e) 月次健全性確認（自動 trigger 無しの未検証滞留防止）。
- 実測トリガ: 本 slice では workflow 変更を行わない。次回 dispatch の観測事項は Verification Notes の NotRun 項目に列挙。

### 4. 不適合時の切り分け責務と保留/復旧手順

**切り分けフロー（fail 検出 → 責務判定 → 起票 → 復旧）:**

```text
consumer-verify fail / consumer daily-ci fail / PR 分類 impacted
  → 証拠確認: run URL + consumer-evidence-*.json（lanes/status/tuple/access）
  → lane 単位で切り分け（fail した lane のみが対象 — 他 lane の pass は有効）
  → 責務判定（下表）→ 起票 + 相互リンク → 復旧 → 再 dispatch で検証
```

**責務マトリクス:**

| 症状 | 責務 Owner | 対応 |
| --- | --- | --- |
| 共有契約の破壊・公開 API/behavior の非互換 | Ruviform（該当 Capability Issue） | 修正 Issue 起票。consumer 側は旧契約での互換経路を維持（spec R2・#193 RB0: Additive 導入・旧 Adapter/Compat 維持） |
| Consumer Adapter/追従の未対応 | 該当 Consumer Issue（Synvelis #28・AudioLens #127、Observability は Synvelis #81・AudioLens #138） | consumer 側で追従 PR を起票。Ruviform は仕様を一方の製品に合わせて破壊しない |
| Consumer 側の潜在欠陥（Ruviform 起因でない） | 該当 Consumer Issue | 先例: audio-lens export metadata 絶対パス混入（run 41604498852 で検出 → consumer 側対応・ruviform 非起因と切り分け済み） |
| tuple 不一致・clone/fetch 失敗 | 運用（dispatch 入力または candidate 消失） | `fail (tuple_mismatch)`/`fail` 証拠として記録。期待 SHA の再指定か candidate の存在確認 |
| workflow 自体の不具合 | Ruviform（本 lane の Owner = #219/#294） | 情報 lane・non-required のため merge は止まらない。修正 PR で対応 |

**保留（hold）:**

- 情報 lane のため consumer-verify の fail は merge を機械的に止めない。ただし `impacted` 分類の経路では、共有契約の破壊/未検証が該当経路の Gate のみを `not run (blocked: <理由>)` で Blocked とし、復旧/互換経路/契約確定後に解除する（spec ルール 6 — Consumer 固有の非依存タスクは一律停止しない）。
- candidate の consumer 側導入保留は consumer Owner の判断: 追従 PR を merge しない・lockfile pin を旧 SHA に留めることで candidate の波及を止められる。Ruviform 側は revert/修正 Issue で対応し、保留状態は Issue 相互リンクで追跡する。
- `unknown` 残置のまま「互換性確認済み」とみなさない。merge 時点の human gate で `unknown` の解消先リンクを確認する。

**復旧（recovery）:**

- Consumer 側: manifest pin / Cargo.lock を旧 SHA へ戻す（追従 PR の revert と同一経路。audio-lens は `update-ruviform-rev.mjs` 相当の lockfile 更新 PR revert、synvelis は lockfile 更新 PR revert）。
- Ruviform 側: 該当 change の revert または修正 Issue 起票。revert は Additive 契約（RB0）に従い旧経路を壊さない範囲で行う。
- credential: Deploy Key 失効は runbook Appendix（3 repo の key 削除 → secret 削除/差替）。repo 再 private 化で anonymous → `not_run` へ自動後退。
- 証拠の保存: run URL・evidence artifact・job summary を Issue/PR にリンクして残す（append-only の Evidence snapshot とは別に、run 単位の証拠は workflow artifact が正本）。

### 5. M0〜M7 Consumer 別受入証拠リンク・未実測事項の見える化

**証拠の正本配置（3 層）:**

| 層 | 正本 | 役割 |
| --- | --- | --- |
| 棚卸し・時系列証拠 | discovery `issue-219-consumer-cross-repo-ci.md` `## Evidence`（append-only dated snapshot） | consumer 依存形態・実測値・分類記録の時系列蓄積。既存表は改変せず dated snapshot を追記 |
| Gate 別受入条件 | [design issue-193-migration-execution-plan](issue-193-migration-execution-plan.md)「Gate 別 Consumer 証拠」表 + Issue #193 本文 | G0〜G7 各 Gate に Synvelis/AudioLens 別の証拠要件を対応付ける受入権威 |
| tuple 検証証拠 | consumer-verify evidence JSON（`consumer-verify-tuple/v1`）+ artifact | 実 SHA tuple・lane 別 outcome・access mode の機械可読証拠（#294） |

**運用規則:**

- M0〜M7（対応 Gate は G0〜G7）の Consumer 別受入証拠は、上記 3 層を相互リンクして追跡する。片側のみの pass を両者 Pass と読み替えない（Issue 本文・#193 Cross-Consumer 受入条件）。
- **M1 Gate #199/#201 の現状を飛ばさない**: #199（clippy 緑化）・#201（PR/Push 自動検証と required checks・main 保護）は**両方 CLOSED**（2026-11-10 `gh` 確認）。ただし #201 後の CI 構造は #365/#366 で dispatch-only + ruleset enforcement disabled へ再編されており、「required checks 実効」は現状ゼロ — M1 受入の文脈では「Gate 自体は通過済みだが、required 機構は再編済みの現行構造（daily-ci dispatch + ci-summary 集約 gate）で評価する」ことを正直に記録する。consumer 別の M1 相当証拠（consumer 側 CI の required/保護状態）は各 consumer repo の責務であり、本 slice では read-only 観測のみ記録する — 両 consumer repo は ruviform と同型の ruleset 構成（`main-protect` active・`main-checkstatus` disabled・2026-11-10 `gh api` 実測）で、required status checks の実効はない。
- 未実測事項の見える化: `not_run`/`unsupported`/`skipped` は pass に計上されず、pass ゼロの run は workflow fail。Deferred lane（Golden/baseline・3 OS・実 GUI/Native Dock・RT/PCM・f64/CLI 精度）は Owner と再実行条件を明記した固定行として証拠表に残す（#294 の not_run/unsupported 分離規則を継承）。
- Observability Gate（#275 設計正本・#281 OBS-06）は本 slice の CI 構造を**利用**し、別の Cross-Repo CI 基盤を作らない（#281 本文明記）。OBS-06 の PASS 証跡は同じ tuple/証拠語彙（実行 ref・feature・コマンド・OS・PASS/FAIL/NOT RUN）に従う。

## Data and Control Flow

```text
Ruviform PR
  → PR 本文「Consumer 影響分類」記入（著者）→ human gate（レビュアー）
  → merge → 必要に応じ consumer-verify dispatch（候補 rev 指定）
       → preflight（入力検証 + access mode 解決）
       → verify-audiolens / verify-synvelis（独立・並列）
            → clone → tuple 照合 → manifest 正規化 → fetch+guard
            → consumer mise lanes + GUI-CONTRACT/SEMANTIC（候補 tree）
            → evidence JSON + summary + artifact
       → summary job（tuple 表 + status 集約・fail-closed）
  → fail 時: Components §4 の責務マトリクスで切り分け → 起票/相互リンク → 復旧 → 再 dispatch
  → 証拠: run URL + artifact + discovery dated snapshot（棚卸し更新時）
```

## Non-Functional Constraints

- fail-closed 原則: `unknown`/`not run`/`unsupported`/`skipped`/tuple 不一致を pass に昇格しない。pass を主張できるのは実行 lane が存在し全実行 lane が pass のときのみ。
- Consumer 独立性: 両 consumer の job・日次 CI・受入証拠は互いに独立。片側の失敗/未検証が他方や merge を一律停止しない。
- 最小権限・秘密非露出: read-only deploy key・secrets.* のみ・ログ/artifact 非出力・pinned known_hosts・cleanup 固定。
- 正本の一元化: 方式・要件・証拠契約の正本は既存 canonical 文書。本書は運用統合のみを担い、重複する方式決定を再記述しない。

## Risks and Trade-offs

| リスク | 評価・対応 |
| --- | --- |
| audio-lens lockfile 正本契約と manifest rev 注入の整合未確認 | consumer-verify の sed は「rev 未指定」形として注入するが、consumer 側 `check-ruviform-rev`/`check-ruviform-lock` が manifest pin 非保持を契約検査する場合に lane 結果へ影響し得る。**次回実 dispatch で観測**（Verification Notes）。consumer repo 変更はしない |
| 棚卸し snapshot の陳腐化 | consumer は活発に更新される（7 日で dep 数・member が変化）。discovery の dated snapshot 追記運用で時系列を保持し、本表は「最新 snapshot との差分確認が必要」な運用文書として扱う |
| 影響分類の機械検査欠如 | `consumer-impact-check.yml` 撤去後は human gate のみ。`unknown` の merge 時点確認をレビュアー責務として明記済み。再機械化は spec R9 の実測後判断に従う |
| required checks 実効ゼロ | #365/#366 再編で ruleset enforcement disabled。consumer-verify 昇格判断は「両 consumer lane の稼働実績」待ちのまま変わらず |

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 |
| --- | --- | --- |
| Golden/baseline・3 OS matrix・実 GUI/Native Dock/RT/PCM・f64/CLI lane | 実行時間・fixture・実機要件が未測。#294 が UI/Golden/RT 拡張の Owner | Issue #294（+ #313/#314・#388/#389） |
| consumer-verify の自動 trigger・required 昇格 | spec R9 — 初回実測と credential 稼働確認後に判断 | Issue #219 spec Deferred |
| Deploy Key 実設定・private 復帰時の SSH 経路実走 | owner/admin 作業・public 期間は anonymous で代替中 | runbook Appendix・Issue #219 |
| audio-lens `RUVIFORM_DEPLOY_KEY` 前提の consumer 側 ci.yml 記述 | consumer repo の責務（public 化後の stale 記述） | audio-lens #127 系 |
| OBS-06 の Gate 実装・PASS 証跡収集 | #281 Owner。基盤は本 slice の CI 構造を利用 | Issue #281・#275 |

## Verification Notes

- [x] 棚卸し値は `gh api` read-only 実測（2026-11-10）— repo visibility・HEAD・manifest/lockfile dep 形態・mise task・workflow 構成・lockfile 解決 SHA の main 祖先性と commit 距離を確認。証拠は discovery 2026-11-10 snapshot。
- [x] 既存正本（spec/ADR/runbook 設計/#294 3 設計/#193 設計/#327 設計）との権威関係・用語（impacted/no impact/unknown、pass/fail/not run、unsupported/skipped、tuple、access mode）を照合。
- [ ] consumer-verify の新 lane 構成を現行 consumer main で通した実走 — **NotRun**（実 Consumer repo での実測は本 slice 対象外・次回 dispatch 観測事項）。
- [ ] audio-lens lockfile 正本契約と manifest rev 注入の整合 — **NotRun**（同上・観測事項として記録）。
- [ ] 実 CI workflow の変更・credential 設定・3 OS・Golden lane — **NotRun**。
