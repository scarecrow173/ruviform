---
id: "IMPL-034IwXvs801LZXZlip1tZ6"
type: "impl"
status: "completed"
title: "Rewrite README for Ruviform"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/rewrite-readme-for-ruviform.md"
    modified:
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "docs/tasks/rewrite-readme-for-ruviform.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/rewrite-readme-for-ruviform.md"
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
    - "verify-reproducible-setup-and-work-memo.md"
    - "baseline-placeholder-sweep.md"
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

# Rewrite README for Ruviform

## Summary

task `TASK-034IwCbgksOGvhF5E4CQbp`（plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-README）として、`README.md` を design の新構成どおり全面改稿した。旧テンプレート内容（`# Project Template`・「新規プロジェクトでの使い方」節・「管理対象」表）を除去し、`# Ruviform`＋概要段落・`## Status`・`## Quickstart`・`## Repository layout`・`## Documentation`・`## Contributing`・`## License` の 7 節構成に置き換えた。

記載内容は #2 の一次記録 IR `verify-reproducible-setup-and-work-memo.md`「作業メモ（#3 転記用）」で実機検証済みの事実のみとし、design の表現ルールどおり 実在・検証済み / 構想 / 検討中 / 未確定 / 未検証 / 仮置き の各区分をラベル明示した。design の禁止記載 7 項目（LICENSE 存在・ライセンス決定済み、未作成クレート実在、RT-safe 断定、GitHub Actions 実ディスパッチ成功済み、macOS/Linux 検証済み、テンプレート再利用手順、存在しない文書へのリンク）は grep＋目視監査でいずれもないことを確認した。

## Context

- 実施タスク: `docs/tasks/rewrite-readme-for-ruviform.md`（`TASK-034IwCbgksOGvhF5E4CQbp`、R-1〜R-6 が本タスクの全作業）
- 上流: `docs/plans/replace-template-derived-docs-with-ruviform.md`（Phase R-1〜R-6 の正本）、`docs/designs/replace-template-derived-docs-with-ruviform.md`（README 新構成表・表現ルール 6 区分・禁止記載 7 項目の直接入力）、`docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`（ライセンス表記・LFS ポリシーの決定）
- 事実の一次ソース: `docs/impl/ir/verify-reproducible-setup-and-work-memo.md` §作業メモ（検証済みコマンド・環境限定・caveat）
- 前提: `docs/impl/ir/baseline-placeholder-sweep.md`（`README.md:1,3,8` が書き換え対象であるベースライン記録）

## Implementation

### Changes

- `README.md` — 全面改稿（39 行 → 110 行）。構成:
  - `# Ruviform`＋概要段落 2 段落: Rust 中心の再利用可能マルチメディア基盤という現時点の方向性、音声・動画と下位計算基盤（DSP・解析・codec・container・SIMD）の再利用意図、対象読者（訪問者・将来の利用者・コントリビューター・AI エージェント・#1 の設計者）を一文で、詳細設計は Issue #1 の後続タスクである旨の案内
  - `## Status`: 開発状況 = 設計・初期セットアップ段階を明示し、`### 実在・検証済み`（Rust 1.99.0 pin＋`mise.lock`、最小ワークスペース＋smoke test、mise 検証タスク群、`workflow_dispatch` 専用 CI＝静的検証のみ済み・実ディスパッチ未実施）/ `### 構想（未実装）`（`ruviform-audio`/`ruviform-video`、DSP・解析・codec・container・SIMD）/ `### 検討中（採否は #1 で決定）`（`wgpu`・WebView UI・ネイティブ UI・プラグイン連携）/ `### 未確定（#1 で決定）`（対応プラットフォーム・公開 API・サポート形式・ライセンス・edition/MSRV・最終クレート分割、`Cargo.toml` 現行値は仮置き）の 4 区分
  - `## Quickstart`: 検証環境限定（Windows 11 x64 Git Bash のみ・macOS/Linux 未検証）を冒頭に明記。前提条件（mise 導入・`mise trust`・ネットワーク到達先・git 認証・書き込み権限）と、IR 検証済みコマンドのみ（`mise install --locked`＋exit 1 caveat＋フォールバック `mise install`、`mise run setup`、`mise exec -- apm install --frozen`、`mise run fmt-check`/`check`/`lint`/`test`/`build`/`verify-ci`）。`target/ci/` 並存の注意を節末尾に記載
  - `## Repository layout`: 実在確認済みのパスのみの表（`Cargo.toml`/`Cargo.lock`/`crates/ruviform-core/`/`mise.toml`/`mise.lock`/`.mise/locks/`/`apm.yml`/`apm.lock.yaml`/`.pre-commit-config.yaml`/`.gitattributes`/`.gitignore`/`.github/workflows/ci.yml`/`AGENTS.md`/`README.md`/`docs/`）＋節末尾に LFS フィクスチャ方針の注記（拡張子ルール維持・小さなフィクスチャは非メディア形式で Git・CI は LFS 未取得）
  - `## Documentation`: 実在文書のみのリンク（`docs/designs/overview.md`・`docs/designs/dev-foundation.md`・`docs/specs/`・`docs/adr/`）＋ Issue #1/#2/#3 への導線
  - `## Contributing`: `AGENTS.md` 作業契約への導線、Issue 駆動、Conventional Commits、prek フック（`mise exec -- prek run --all-files`）
  - `## License`: ライセンス未決定・`LICENSE` 不在・`Cargo.toml` の `license = "MIT"` は仮置きの明示
- `docs/tasks/rewrite-readme-for-ruviform.md` — status `todo` → `done`、R-1〜R-6・Done When・Verification の全 checkbox を `[x]`＋結果注記、`implemented-by` に本 IR を追記。`updated` は同日（2026-10-19）のため据え置き
- `docs/impl/ir/rewrite-readme-for-ruviform.md` — 新規（本 IR。`new_impl_record.js` で in-progress 作成後に completed へ更新）
- `docs/impl/ir/README.md`・`docs/tasks/README.md` の index 更新は並行作業の衝突回避のため orchestrator 側の責務として委譲（本 IR の変更対象外）

### Decisions During Implementation

- **Quickstart の `sh` ブロックにフォールバック `mise install` を inline で併記した**。design は `mise install --locked` を正本・`mise install` をフォールバックとする構成を要求しており、コメントで caveat（global mise config が repo lockfile 外ツールを要求する環境ではその分のみ exit 1）とフォールバックを隣接配置することで、読者がコピー＆ペーストで完結できる形にした。
- **Repository layout に `README.md` 自身を追加した**。design の列挙には含まれないが実在ファイルであり、自己参照は誤記にならないと判断した。design 列挙外の新規エントリはこれのみ。
- **構想記述と検証済み事実をセクションで分離した**。design の「概要段落＋構想/検討中は意図、Status/Quickstart は検証済み事実」の切り分けに従い、概要段落の方向性は「構想しています」と意図表現に留め、Status 内は 4 区分のラベル分けで混在を回避した。
- **`mise run fmt` は README に記載しなかった**。`mise.toml` には存在するが、タスク R-3 と IR 一次記録の「検証済みコマンド」列挙に含まれないため記載対象外とした（`fmt` はソースを書き換える破壊的タスクでもある）。
- **Issue #3 の説明文は「リポジトリ文書・識別子の Ruviform 向け整備」とした**。README 内で「テンプレート」語を一切使わない制約（R-6・catch-all grep）のため、Issue の本来語を避けて内容を言い換えた。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64 / Git Bash / node v26.10.0 / mise 管理ツール（`mise exec` 経由）。task の `## Verification` に対応する実測結果:

| Check | コマンド | 結果 |
| --- | --- | --- |
| テンプレート再利用手順なし | `grep -in "Use this template" README.md` | exit 1（0 hit） |
| project.template なし | `grep -in "project.template" README.md` | exit 1（0 hit） |
| テンプレート変種なし | `grep -in "テンプレート\|template" README.md` | exit 1（0 hit） |
| 7 節構成 | `grep -n "^## " README.md` | `## Status`(L7)・`## Quickstart`(L33)・`## Repository layout`(L73)・`## Documentation`(L92)・`## Contributing`(L102)・`## License`(L108) の 6 節を design 順で確認（`# Ruviform`＋概要と合わせ 7 節） |
| 検証済みコマンド記載 | `grep -n "mise install --locked\|mise run setup\|mise run fmt-check\|mise run check\|mise run lint\|mise run test\|mise run build\|mise run verify-ci\|apm install --frozen" README.md` | 全コマンド一致（L50/57/60/63-68/71/106） |
| ラベル区分 | `grep -n "構想\|検討中\|未確定\|未検証\|仮置き" README.md` | 全ラベル一致（L5/16/20/25/29/31/35/78/110） |
| ライセンス注記 | `test ! -f LICENSE && grep -n "未決定" README.md` | `LICENSE` 不在（exit 0）、L110 の `## License` 節注記に一致 |
| prek フック | `mise exec -- prek run --all-files` | exit 0（trim trailing whitespace / fix end of files / check yaml / check toml / mixed line ending / fix utf-8 bom / detect private key: Passed、check illegal windows names: Skipped） |
| task audit | `node .agents/skills/doc-status/scripts/audit_docs.js --type task --json` | blocking findings 0。warning 3 件はいずれも `index-metadata-mismatch`（`docs/tasks/README.md` index が本タスクと並行スライス `review-config-files-minimal-edits.md`・`update-apm-identity-and-lockfile.md` の status をまだ `todo` と記載。index 更新は orchestrator 責務のため本 IR の変更対象外） |
| IR audit | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | findings 0 |

補助確認（目視監査・design 表現ルール 6 区分と禁止記載 7 項目）:

- 6 区分ラベルの適用: 検証済み＝「Windows 11 x64（Git Bash）で検証済み」＋環境添え、構想＝「構想（未実装）」「構想しています」、検討中＝「検討中（採否は #1 で決定）」、未確定＝「未確定（#1 で決定）」、未検証＝「macOS / Linux では未検証」、仮置き＝「（仮置き）」を全該当箇所に適用済み
- 禁止記載 7 項目: いずれもなし。`LICENSE` 不在は実測、未作成クレートは「構想（未実装）」明示、RT-safe・リアルタイム保証の記述なし、CI は「実ディスパッチ未実施」明記、macOS/Linux は未検証明記、再利用手順なし、リンク先は全て `test -f`/`test -d` 相当の実在確認済み
- タスク名照合: README の `mise run <name>`（`setup`/`fmt-check`/`check`/`lint`/`test`/`build`/`verify-ci`）は全て `mise.toml` L18-L86 の `[tasks.*]` に実在、`mise exec --` 配下の `apm`/`prek`/`git`（git-lfs）は `mise.toml` `[tools]` L10-L16 管理下

未実行の検証: `mise run fmt-check`/`check`/`lint`/`test`/`build`/`verify-ci` の再実行（本タスクは文書変更のみでコード系検証への影響なし。最終スモークは T-VERIFY の検証範囲）

## Risks

- **構想記述が確定方針と誤読される残存リスク**。ラベル分けで抑止済みだが、文言の最終確認は PR レビュー観点（design の Risks と同じ留保）。
- **#1 の決定後に構想・未確定の記述が陳腐化する**。design の Deferred Design Concerns どおり、README 側に「詳細設計は #1」の導線を残してあるため追従先は明確。
- **`mise install --locked` の環境 caveat は本文どおり environment-dependent**。global mise config を持たない環境では発生しないが、README は両方のケースを併記済み。

## Follow-ups

- `docs/impl/ir/README.md`（IR index）と `docs/tasks/README.md`（task index）への本 IR・本タスクの反映は orchestrator が行う。
- `run-placeholder-sweep-and-verification.md`（T-VERIFY）が最終スイープで `README.md` のベースライン 3 hit（L1/L3/L8）消滅と内容監査を確認する。本 IR での grep 実測値が比較入力となる。
- #1 の詳細設計確定時に README の構想・検討中・未確定区分を追従更新する（design Deferred Design Concerns）。
