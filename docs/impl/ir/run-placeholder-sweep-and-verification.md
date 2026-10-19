---
id: "IMPL-034Ixn011MsolyFpmBB87l"
type: "impl"
status: "completed"
title: "Run placeholder sweep and verification"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/run-placeholder-sweep-and-verification.md"
    modified:
      - type: "file"
        path: "docs/plans/replace-template-derived-docs-with-ruviform.md"
      - type: "file"
        path: "docs/adr/replace-template-derived-docs-metadata-with-ruviform.md"
      - type: "file"
        path: "docs/tasks/run-placeholder-sweep-and-verification.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/run-placeholder-sweep-and-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/replace-template-derived-docs-with-ruviform.md"
    - "../../designs/replace-template-derived-docs-with-ruviform.md"
    - "../../specs/replace-template-derived-docs-with-ruviform.md"
    - "../../adr/replace-template-derived-docs-metadata-with-ruviform.md"
    - "baseline-placeholder-sweep.md"
    - "verify-reproducible-setup-and-work-memo.md"
    - "rewrite-readme-for-ruviform.md"
    - "update-apm-identity-and-lockfile.md"
    - "rewrite-agents-for-ruviform.md"
    - "review-config-files-minimal-edits.md"
    - "../../../AGENTS.md"
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

# Run placeholder sweep and verification

## Summary

task `TASK-034IwCjptfvbSFBQaSKigx`（plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` の終端スライス T-VERIFY）として、Phase F-1〜F-8 を実行し、plan の検証一覧 V-1〜V-11 の判定と ADR/plan の双方向メタデータ編集を行った。

結果: **V-1〜V-11 すべての判定基準を満たした**（`mise install --locked` の global-config caveat は Risk 8 の判定基準どおり「repo 管理の全 7 ツールが解決」を以て合否とし、pass + caveat 記録）。F-2 の最終スイープは S-1 ベースライン `IMPL-034IwOgrHkNf8v0cnrOpIl` と比較して、書き換え対象リテラル残存 11 行が全件 0 になり、非 `docs/` 残存は除外対象（`apm.lock.yaml` の APM 生成物パス 198 hit、レビュー済み無変更の `.pre-commit-config.yaml:1` のみ）となった。F-1 では ADR `derived-by` に plan バックリンクを追加し、plan `derives-from` に ADR を追加・`related` から除去して双方向整合を取った。

追記（完了後の追検証）: 本 IR 作成時点で未実行としていた GitHub Actions 実ディスパッチは同日 2026-10-19 に `main` で 2 回実施された。初回は rust toolchain components 欠落で失敗し、修正（PR #7）後の 2 回目で全ステップ green となった。詳細は Validation 末尾の「完了後の追検証」節。

## Context

- 実施タスク: `docs/tasks/run-placeholder-sweep-and-verification.md`（`TASK-034IwCjptfvbSFBQaSKigx`）
- 上流: `docs/plans/replace-template-derived-docs-with-ruviform.md` の F-1〜F-8（実施手順）と V-1〜V-11（判定基準）が正本
- ベースライン消費元: `docs/impl/ir/baseline-placeholder-sweep.md`（`IMPL-034IwOgrHkNf8v0cnrOpIl`、S-1・`TASK-034IwBWH50nD9Unn8HghJD`）
- 検証対象の兄弟スライス IR: `rewrite-readme-for-ruviform.md`（T-README）・`update-apm-identity-and-lockfile.md`（T-IDENT）・`rewrite-agents-for-ruviform.md`（T-AGENTS）・`review-config-files-minimal-edits.md`（T-CONFIG）
- 除外ルール（design/plan の判定基準、S-1 IR から継承）: `docs/` 配下の履歴記録と `apm.lock.yaml` 等の生成物内 APM パッケージ由来パス（`assets/templates/`・`template-variants.*`・`frame-template.html`）は書き換え対象外

## Implementation

### Changes

- `docs/plans/replace-template-derived-docs-with-ruviform.md` — F-1。`relations.derives-from` に `"../adr/replace-template-derived-docs-metadata-with-ruviform.md"` を追加（design の先例どおり spec の後に配置）し、`relations.related` から同 ADR を除去。front matter のみの in-place 編集
- `docs/adr/replace-template-derived-docs-metadata-with-ruviform.md` — F-1。`relations.derived-by` に `"../plans/replace-template-derived-docs-with-ruviform.md"` を追加（既存の design バックリンクは維持）。本文・`status: "accepted"` は無変更
- `docs/impl/ir/run-placeholder-sweep-and-verification.md` — 新規（本 IR）
- `docs/tasks/run-placeholder-sweep-and-verification.md` — status `todo` → `done`、checkbox 完了、`implemented-by` に本 IR を追記
- `docs/impl/ir/README.md`・`docs/tasks/README.md` — index 追記（タスクの changes 宣言に含まれるが、委譲境界によりオーケストレーターの最終化で更新される）
- 上記以外の追跡ファイルは検証のみで無変更（prek auto-fix の取り込み対象も発生しなかった）

### Decisions During Implementation

- **F-1 の plan `derives-from` 配置順は design の先例に倣った**。design `DESIGN-034IvBNBZvoikAgd6DJxdD` は `derives-from` を `spec` → `ADR` の順で記述しているため、plan でも既存の `design` → `spec` の末尾に ADR を追加した。
- **`apm.lock.yaml` の 198 hit は S-1 と同じ 3 パターンのみ**。`assets/templates/` 配下パス・`adr-doc/references/template-variants.*`・`brainstorming/scripts/frame-template.html` の 3 種であり、パターン外の hit は 0 件（`grep -vE` で実測確認）。生成物内 APM パッケージ由来パスとして除外対象のまま。
- **`apm.lock.yaml` は `main` 比で無差分**。lockfile は依存パッケージの pin のみを記録し、manifest の root `name`/`description` を保持しないため、T-IDENT の `apm.yml` 2 行変更は lockfile に波及しなかった。manifest↔lockfile の整合は `apm install --frozen` exit 0 が担保する。
- **`mise install --locked` exit 1 は Risk 8 の global-config caveat と判定**。失敗した 7 ツール（`ripgrep`・`ffmpeg`・`python@3.13`・`node@26`・`pnpm`・`@openai/codex`・`ccusage`）はすべて `C:\Users\akiyoshi\.config\mise\config.toml` 由来で repo lockfile 外。repo `mise.toml` の全 7 ツール（`uv`・`prek`・`git-lfs`・`pipx:apm-cli`・`pipx:oraios/serena`・`rust`・`actionlint`）は already installed で解決済みのため、判定基準どおり pass とし caveat を記録する。
- **`node` は repo 管理ツールではなく環境前提**。`node v26.10.0` が PATH 上に存在（global mise config 由来）することを確認してから F-7 を実行した。

## Final Sweep（F-2 / V-1 対 S-1 ベースライン比較）

### Environment

- OS/シェル: Windows 11 x64 / Git Bash
- 実施日: 2026-10-19。スイープ時点の HEAD: `8281921`（F-1 の front matter 編集のみ未コミット、worktree 上で `docs/` 2 ファイル modified）
- 母集団: `git ls-files` → 60 ファイル（S-1 の 55 から +5。増分は兄弟スライス追加の docs 系のみで、書き換え対象集合の定義には影響しない）

### Results Overview（`-- . ':(exclude)docs/**'` 適用の非 `docs/` 集合）

| grep | S-1 ベースライン非 `docs/` hit | 最終スイープ非 `docs/` hit | 差分 |
| --- | --- | --- | --- |
| G1 `git grep -in "project.template"` | 2（`README.md:1`・`apm.yml:1`） | 0 | −2（書き換え完了） |
| G2 `git grep -in "Use this template"` | 1（`README.md:8`） | 0 | −1（書き換え完了） |
| G3 `git grep -n "派生リポジトリ\|派生プロジェクト\|言語・フレームワーク非依存\|言語・フレームワークに依存しない"` | 6（`README.md:3`・`AGENTS.md:3,5,75,80,91`） | 0 | −6（書き換え完了） |
| G4 `git grep -in "template\|テンプレート" -- . ':(exclude)docs/**'` | 204（`AGENTS.md` 3・`README.md` 2・`apm.yml` 1・`apm.lock.yaml` 198） | 198（`apm.lock.yaml` のみ） | −6（残存は全件除外対象の生成物パス） |
| G6（補助）`git grep -in "language.agnostic"` | 2（`.pre-commit-config.yaml:1`・`apm.yml:3`） | 1（`.pre-commit-config.yaml:1`） | −1（`apm.yml:3` の description は T-IDENT で改稿済み） |

- 書き換え対象リテラル残存（S-1 の 11 行）: **全件 0**。`mise.toml` L8 の意味的残存コメントも T-CONFIG C-1 で書き換え済み（`git diff main -- mise.toml` は同コメント行 1 行のみの変更を示す）
- `.pre-commit-config.yaml:1` は C-3 の「定位は設計どおり・無変更既定」判定どおり残存する既知のレビュー済み残存であり、書き換え対象ではない
- `docs/` 側 hit は履歴記録の増分として増加（除外対象。回帰ではない。S-1 IR「Risks」記載どおり）

### V-1 判定: pass

除外対象を除く追跡ファイルにテンプレート由来プレースホルダーの残存なし。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64 / Git Bash / git 2.54.0.windows.1 / node v26.10.0（global mise config 由来の環境前提・導入確認済み）/ mise 2026.10.1 / apm-cli 0.33.0（pipx 経由・repo lockfile 管理）。

### V-1〜V-11 結果表

| V | 検証内容 | 結果 | 証跡 |
| --- | --- | --- | --- |
| V-1 | プレースホルダー最終スイープ（除外対象以外に残存なし） | **pass** | 上記「Final Sweep」表。G1/G2/G3 の非 `docs/` hit は 0、G4 残存 198 は `apm.lock.yaml` の APM 生成物パスのみ（パターン外 hit 0 を実測）、`.pre-commit-config.yaml:1` はレビュー済み無変更残存 |
| V-2 | README が実態適合・#2 手順反映・未検証明示 | **pass** | `README.md` 目視監査。「Use this template」手順なし（G2=0）。design の 7 節構成どおり。検証済みコマンドと「Windows 11 x64（Git Bash）でのみ実機検証済み・macOS / Linux では未検証」の限定記載あり（L35） |
| V-3 | ラベル明示・禁止記載なし | **pass** | `## Status` の「実在・検証済み」/「構想（未実装）」/「検討中（採否は #1 で決定）」/「未確定（#1 で決定）」ラベル・「仮置き」注記を確認。禁止記載（LICENSE 存在・未作成クレート実在・RT-safe 断定・CI 実ディスパッチ成功・macOS/Linux 検証済み・テンプレート再利用手順・存在しない文書リンク）なし |
| V-4 | `apm.yml` 識別子・lockfile 整合 | **pass** | `git diff main -- apm.yml` は `name: project-template → ruviform` と `description` の 2 行のみ。`targets`・依存・MCP 維持。`mise exec -- apm install --frozen` exit 0（9 APM dependencies・lockfile presence verified） |
| V-5 | 構文有効・prek 実行可 | **pass** | `mise exec -- prek run --all-files` exit 0。8 hooks 全 pass（check-yaml が `apm.yml`/`.pre-commit-config.yaml`、check-toml が `mise.toml`/`Cargo.toml` を検証）。auto-fix による差分なし |
| V-6 | 無関係な再構成なし・挙動不変 | **pass（caveat 記録）** | `mise run fmt-check`・`check`・`lint`・`test`・`verify-ci` すべて exit 0（test は 1 unit + 1 doctest pass）。`git diff main -- mise.toml` はコメント 1 行のみ、`mise.lock`・`.mise/locks/` 無差分。`mise install --locked` は global-config caveat で exit 1 だが repo 管理 7 ツール全解決（Risk 8 判定基準で pass） |
| V-7 | LICENSE 不在・ライセンス未決定の整合 | **pass** | `test ! -f LICENSE` exit 0。`git diff main -- Cargo.toml` 空（exit 0）。`README.md` `## License` と `AGENTS.md` `### ライセンス注記` が未決定・`license = "MIT"` 仮置き注記を保持 |
| V-8 | LFS 方針整合 | **pass** | `git diff main -- .gitattributes` 空（exit 0）。`AGENTS.md` `## Gitと安全性` L169 の非メディアフィクスチャ方針と `README.md` `## Repository layout` L90 の LFS/フィクスチャ注記を確認 |
| V-9 | 記載コマンドの実在性 | **pass** | README/AGENTS の `mise run` 記載名（`setup`・`fmt`・`fmt-check`・`check`・`lint`・`test`・`build`・`check-ci`・`lint-ci`・`test-ci`・`build-ci`・`verify-ci`）はすべて `mise.toml` `[tasks.*]` に存在し `mise tasks` 出力と一致。`mise exec` 記載（`apm install --frozen`・`prek run --all-files`・`git lfs`・`git push`）も実在ツール |
| V-10 | docs グラフ監査 | **pass** | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` → `{"files": 24, "findings": []}`（F-1 後・exit 0）。node v26.10.0 導入済み確認 |
| V-11 | メタデータ編集・検証報告 | **pass** | ADR `relations.derived-by` に design・plan の両バックリンクあり（本文・`status: "accepted"` 無変更）。plan `derives-from` に ADR 追加済み・`related` から除去済み。本 IR が実行済み/未検証を区分して記載 |

### diff vs `main` の宣言整合（Risk 4 対応）

`git diff main --name-only`（26 ファイル）: `AGENTS.md`・`README.md`・`apm.yml`・`mise.toml`（4 つの宣言済み書き換え/編集対象）+ `docs/` 配下 22 ファイル（issue #3 の spec/ADR/design/plan/task/IR/index 追加・更新）。宣言外の逸脱なし。`Cargo.toml`・`.gitattributes`・`.gitignore`・`.pre-commit-config.yaml`・`.github/workflows/ci.yml`・`apm.lock.yaml`・`mise.lock`・`.mise/locks/` は `main` 比で無差分（`git diff --exit-code` で個別確認済み）。

### 実行した検証

- F-1 front matter 編集（plan `derives-from`/`related`、ADR `derived-by`）— `git diff` で front matter のみの変更を確認
- F-2 最終スイープ G1・G2・G3・G4・G6 と対象ファイル個別 grep — 上記表どおり
- F-3 `mise exec -- prek run --all-files` — exit 0
- F-4 `mise install --locked` — exit 1（global-config caveat。repo 管理 7 ツール全解決）、`mise exec -- apm install --frozen` — exit 0
- F-5 `mise run fmt-check`/`check`/`lint`/`test`/`verify-ci` — すべて exit 0
- F-6 README/AGENTS 目視監査・`test ! -f LICENSE`・no-change ファイル群の `git diff --exit-code`・`git diff main --name-only`
- F-7 `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — findings 0

### 完了後の追検証: GitHub Actions 実ディスパッチ（2026-10-19）

Issue #2 の workflow 設計（IR `IMPL-034IemQE6RlhuBXtirqKVX` `add-manual-ci-workflow.md`）で「本 PR マージ後に確認する」と引き継がれていた実ディスパッチを、dispatch-only workflow `ci`（`.github/workflows/ci.yml`）に対して `main` 上で 2 回実施した。

| Run | 結果 | 内容 |
| --- | --- | --- |
| `39249234364` | **failed** | `mise run fmt-check` で `cargo-fmt is not installed for the toolchain '1.99.0-x86_64-unknown-linux-gnu'` により失敗。fresh Ubuntu runner への rustup 初期導入が minimal profile（rustfmt/clippy を含まない）であることが原因。Windows 側は既存インストールに同梱済みだったためローカルでは顕在化しなかった。`mise install --locked` 自体は成功 |
| `39249585284` | **all green（約 30 秒）** | 修正 `aa9505e`（PR #7・merge commit `6aeadf6`。`mise.toml` の rust エントリを `components = ["rustfmt", "clippy"]` として pin し `mise.lock` を再生成）適用後、全ステップ pass |

- 実測値（run `39249585284`）: `df -h` はビルド前 `/dev/root 72G・59G used・14G avail (82%)`、ビルド後 `60G used・13G avail (84%)`。`du -sh target` = `7.1M`（内訳: `target/ci/deps` 7.0M・`target/ci/build` 4.0K）。ディスク余裕は十分
- pass した CI ステップ: `mise exec -- prek run --all-files` → `mise run fmt-check` → `check-ci` → `lint-ci` → `test-ci`（CI profile・job env `CARGO_INCREMENTAL=0`）
- 初回ディスパッチで検出され PR #7 で修正された不具合（Linux fresh 環境での rust components 欠落 → `fmt-check` 失敗）は、実ディスパッチで捕捉を意図していたクロスプラットフォーム差異そのものであり、手動ディスパッチ設計の妥当性を実証する結果となった
- これにより Issue #2 の workflow 設計で残っていた最後の「未実行」検証が消化された

### 未実行・未検証条件（引き継ぎ）

- **macOS / Linux での実機検証**: 未実施（README Quickstart・AGENTS mise 節で「未検証」と明示済みのとおり）
- **mise 未導入の素の Windows での手順**: 未実施（mise 導入はリポジトリ外前提として記載済み）
- ~~GitHub Actions での実ディスパッチ~~: **解消済み**（上記「完了後の追検証」節。`main` で 2 回ディスパッチし、初回失敗→PR #7 修正後に全 green）
- **`mise install --locked` の caveat**: global mise config が repo lockfile 外ツールを要求する環境ではその分のみ exit 1（今回実測）。repo 管理ツールの解決は全件成功しており、README/AGENTS のフォールバック記載と整合

## Risks

- **`docs/` 側 hit 数は本 IR・task status 更新でも増加する**。除外ルールにより評価対象外であり、非 `docs/` 集合（書き換え対象集合）が比較基準である点は S-1 IR と同一。
- **`apm.lock.yaml` の hit 数・行番号は APM 依存解決結果に依存して変動し得る**。今回は再生成後も `main` 比で無差分だったが、将来の依存更新で hit 数が変わっても種別（APM 由来パスのみ）が維持される限り除外対象のまま。
- **本 IR 自身を含む `docs/impl/ir/`・index README の更新はオーケストレーター側の最終化に委譲されている**。`relations.changes` は task 宣言どおり index 2 ファイルを含めて記録しているが、当該ファイルの実編集は本委譲の範囲外。

## Follow-ups

- README `## Status`（L18）の「GitHub Actions 上での実ディスパッチは未実施です」記述は「完了後の追検証」により陳腐化している。実ディスパッチ実績（失敗→PR #7 修正→全 green）を反映する README 更新は別タスク/別コミットで実施すること（本 IR の編集範囲外）。
- オーケストレーター: `docs/impl/ir/README.md`・`docs/tasks/README.md` の index 更新と、本タスク分のコミット（本 IR・task status 更新・F-1 の plan/ADR front matter 編集を含む）。
- 将来の変更者: `apm.yml` の manifest フィールド（`name`/`description`）を変更しても `apm.lock.yaml` には波及しない（lockfile は依存 pin のみを記録）。整合は `apm install --frozen` で担保する。
