---
id: "TASK-034IwCbgksOGvhF5E4CQbp"
type: "task"
status: "done"
title: "README.md の Ruviform 向け全面改稿"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
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
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/replace-template-derived-docs-with-ruviform.md"
  implemented-by:
    - "../impl/ir/rewrite-readme-for-ruviform.md"
  depends-on:
    - "../plans/replace-template-derived-docs-with-ruviform.md"
    - "baseline-placeholder-sweep.md"
  blocks:
    - "run-placeholder-sweep-and-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# README.md の Ruviform 向け全面改稿

## Work

plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-README（Phase R-1〜R-6）。`README.md` を design の新構成（`# Ruviform`＋概要、`## Status`、`## Quickstart`、`## Repository layout`、`## Documentation`、`## Contributing`、`## License` の 7 節）どおり全面改稿し、「Use this template」等のテンプレート再利用手順を除去する。記載できるのは #2 の IR `docs/impl/ir/verify-reproducible-setup-and-work-memo.md` で実機検証済みの事実のみで、構想・検討中・未確定・未検証・仮置きの区分をラベル明示する。

- [x] R-1. `README.md` を design の新構成どおり全面改稿する: `# Ruviform`＋概要段落（Rust を中心とした再利用可能なマルチメディア基盤という現時点の方向性、音声・動画とその下位の計算基盤（DSP・解析・codec・container・SIMD）を同じ基盤上で再利用可能にする意図、対象読者を一文で、詳細設計は Issue #1 の後続タスクである旨の案内）、`## Status`、`## Quickstart`、`## Repository layout`、`## Documentation`、`## Contributing`、`## License` → 全 7 節を design どおりの順序で全面改稿済み。旧テンプレート記述（`# Project Template`・「新規プロジェクトでの使い方」・「管理対象」）は除去
- [x] R-2. `## Status` でラベル分けを行う: 実在・検証済み（Rust 1.99.0 ツールチェーンの mise pin＋`mise.lock`、ルート `Cargo.toml`＋`crates/ruviform-core` の最小ワークスペースと smoke test、mise 検証タスク群、`workflow_dispatch` 専用 CI＝静的検証のみ済み・実ディスパッチ未実施）/ 構想（`ruviform-audio`/`ruviform-video` と下位の DSP・解析・codec・container・SIMD）/ 検討中（`wgpu`・WebView UI・ネイティブ UI・プラグイン連携）/ 未確定（対応プラットフォーム・公開 API・サポート形式・ライセンス・edition/MSRV・最終クレート分割） → `### 実在・検証済み`/`### 構想（未実装）`/`### 検討中（採否は #1 で決定）`/`### 未確定（#1 で決定）` の 4 区分で全項目を記載。`Cargo.toml` の `edition`/`rust-version`/`license` 現行値は仮置きの旨も併記
- [x] R-3. `## Quickstart` に IR 一次記録の検証済みコマンドのみ記載する: `mise install --locked`（正本・固定復元。global mise config が repo lockfile 外ツールを要求する環境ではその分のみ exit 1 となり得る caveat とフォールバック `mise install` を併記）、`mise run setup`（= `apm install`＋`prek install`＋`git lfs install --local`）、`mise run fmt-check`/`check`/`lint`/`test`/`build`、`mise run verify-ci`（CI 同等・`target/ci/` 並存の注意）、厳格検証 `mise exec -- apm install --frozen`。前提条件（mise 導入済み・`mise trust`・ネットワーク到達先・git 認証・書き込み権限）と、実機検証は Windows 11 x64（Git Bash）のみ・macOS/Linux は未検証である旨を明記する → 全コマンドと caveat・フォールバック・前提条件・検証環境限定を記載。`target/ci/` 並存の注意も節末尾に記載
- [x] R-4. `## Repository layout` に実在構成のみのツリー/表を書く: `Cargo.toml`/`Cargo.lock`/`crates/ruviform-core/`/`mise.toml`/`mise.lock`/`.mise/locks/`/`apm.yml`/`apm.lock.yaml`/`.pre-commit-config.yaml`/`.gitattributes`/`.gitignore`/`.github/workflows/ci.yml`/`docs/`/`AGENTS.md`。存在しないディレクトリ・文書を案内しない。節末尾に LFS フィクスチャ方針の注記枠を設ける（`.gitattributes` の音声/動画拡張子ルール維持・小さなテストフィクスチャは非メディア形式で Git に置く・CI は必要になるまで LFS オブジェクトを取得しない） → 指定全エントリを実在確認済みの表で記載し `README.md` 自身も追加。節末尾に LFS フィクスチャ方針の注記を設置
- [x] R-5. `## Documentation`・`## Contributing`・`## License` を書く: 存在する文書へのリンクのみ（`docs/designs/overview.md`・`docs/designs/dev-foundation.md`・`docs/specs/`・`docs/adr/`・Issue #1/#2/#3 への導線）、`AGENTS.md` の作業契約・Conventional Commits・Issue 駆動・prek フックへの導線、**ライセンス未決定の明示**（`LICENSE` ファイルは存在せず、`Cargo.toml` の `license = "MIT"` は仮置きで配布方針の確定ではない旨の注記） → 3 節とも記載済み。リンク先は全て実在確認済み（`docs/designs/overview.md`・`docs/designs/dev-foundation.md`・`docs/specs/`・`docs/adr/`・`AGENTS.md`・Issue #1/#2/#3）
- [x] R-6. 禁止記載の自己監査を行う: `LICENSE` が存在する/ライセンス決定済み、未作成クレート・API が実在する、RT-safe・リアルタイム保証の断定、GitHub Actions 実ディスパッチ成功済み、macOS/Linux 検証済み、「Use this template」等のテンプレート再利用手順、存在しない文書・テスト・ベンチマークへのリンク・言及——のいずれもないことを確認する → 目視監査＋grep で全項目ゼロを確認（IR `Validation` に記録）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/replace-template-derived-docs-with-ruviform.md](../plans/replace-template-derived-docs-with-ruviform.md) — `PLAN-034IvX2cFeBjPV4UMfDJ2x`（approved）。Phase R-1〜R-6 / V-2・V-3 の正本
- [../designs/replace-template-derived-docs-with-ruviform.md](../designs/replace-template-derived-docs-with-ruviform.md) — `DESIGN-034IvBNBZvoikAgd6DJxdD`（approved）。`README.md` の新構成・表現ルール・禁止記載の直接入力
- [../adr/replace-template-derived-docs-metadata-with-ruviform.md](../adr/replace-template-derived-docs-metadata-with-ruviform.md) — `ADR-034Ivm746Q38Wa21HeqUiI`（accepted）。ドキュメント定位・暫定/確定境界・ライセンス表記・LFS ポリシーの決定
- [../specs/replace-template-derived-docs-with-ruviform.md](../specs/replace-template-derived-docs-with-ruviform.md) — `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（approved）。Acceptance Criteria の根拠
- [../impl/ir/verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md) — #2 の一次記録。検証済みコマンド・環境限定・caveat の唯一の事実ソース

## Dependencies

- 前提タスク: `baseline-placeholder-sweep.md`（`TASK-034IwBWH50nD9Unn8HghJD`）— ベースライン記録済みであること
- 後続をブロック: `run-placeholder-sweep-and-verification.md` — 最終スイープ・内容監査の対象を確定する
- 並行可能: `update-apm-identity-and-lockfile.md`、`review-config-files-minimal-edits.md` とは S-1 後に並行着手可

## Affected Files

- `README.md` — 全面改稿（既存のテンプレート内容は置き換え）
- `docs/impl/ir/rewrite-readme-for-ruviform.md` — 新規。改稿内容・自己監査結果を記録する IR
- `docs/impl/ir/README.md` — IR index への追記

## Done When

- [x] R-1〜R-6 がすべて完了し、design の 7 節構成（`# Ruviform`＋概要 / Status / Quickstart / Repository layout / Documentation / Contributing / License）になっている → `grep -n "^## " README.md` が 6 節を design 順で返すことを確認
- [x] 検証済み・構想・検討中・未確定・未検証・仮置きの各ラベルが全該当箇所に適用され、design の禁止記載 7 項目がない → `grep -n "構想\|検討中\|未確定\|未検証\|仮置き" README.md` で全ラベル一致、禁止記載は目視監査でゼロ確認
- [x] README 記載の `mise run <name>` / `mise exec -- <cmd>` がすべて `mise.toml` / 実在ツールに存在する（V-9 の対象） → `setup`/`fmt-check`/`check`/`lint`/`test`/`build`/`verify-ci` が `mise.toml` L18-L86 に実在、`apm`/`prek`/`git`（git-lfs）は `mise.toml` `[tools]` 管理下
- [x] 改稿結果と自己監査結果が IR `docs/impl/ir/rewrite-readme-for-ruviform.md` に記録されている → IR `IMPL-034IwXvs801LZXZlip1tZ6` に記録

## Verification

- [x] `grep -in "Use this template" README.md` exits 1（テンプレート再利用手順なし） → exit 1（0 hit）確認済み
- [x] `grep -in "project.template" README.md` exits 1 → exit 1（0 hit）確認済み
- [x] `grep -n "^## " README.md` が `## Status`・`## Quickstart`・`## Repository layout`・`## Documentation`・`## Contributing`・`## License` の 6 節を返す → 6 節をこの順序で確認（L7/33/73/92/102/108）
- [x] `grep -n "mise install --locked\|mise run setup\|mise run fmt-check\|mise run check\|mise run lint\|mise run test\|mise run build\|mise run verify-ci\|apm install --frozen" README.md` が各検証済みコマンドに一致する → 全コマンド一致（L50/57/60/63-68/106）
- [x] `grep -n "構想\|検討中\|未確定\|未検証\|仮置き" README.md` が各ラベルに一致する → 全ラベル一致（L5/16/20/25/29/31/35/78/110）
- [x] `test ! -f LICENSE` exits 0 かつ `grep -n "未決定" README.md` が `## License` 節の注記に一致する → `LICENSE` 不在（exit 0）・L110 の `## License` 節注記に一致
- [x] `mise exec -- prek run --all-files` exits 0（end-of-file-fixer・mixed-line-ending 等の軽量チェック） → exit 0（全 hook Passed、check illegal windows names のみ Skipped）
- [x] Manual: design の表現ルール 6 区分と禁止記載 7 項目への適合を目視監査（V-2・V-3。補助） → 6 区分ラベル全適用・禁止記載 7 項目ゼロを目視確認（IR `Validation` に記録）
