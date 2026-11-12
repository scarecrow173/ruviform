---
id: "DESIGN-MnDOJoPVmxmAz1hdgVs3T3"
type: "design"
status: "approved"
title: "Issue #586 設計 — ir front matter 是正分類・index marker 配置・検証手順"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/586"
    - "docs/specs/issue-586-impl-index-generation.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-586-impl-index-generation-design.md"
    modified:
      - type: "directory"
        path: "docs/impl/ir/"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/specs/issue-586-impl-index-generation.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-586-impl-index-generation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-586-impl-index-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-586-impl-index-generation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #586 設計 — ir front matter 是正分類・index marker 配置・検証手順

## Overview

`docs/impl/ir` の 30 件の front matter を `implementationRecordSchema`(`impl-doc` skill の audit / `sync_impl_index` が使う schema)に適合させ、`docs/impl/ir/README.md`・`docs/impl/exp/README.md` に generated-index marker を追加して owner の sync engine で index を再生成する。本文・ファイル名・既存 id は変更しない。

schema の要点(sync_impl_index.js / audit_impl_record.js 内の `implementationRecordSchema` より):

- `id`: 空でない文字列。`type`: `"impl"` 固定。`status`: `draft|in-progress|completed|blocked|abandoned|superseded`。
- `created` / `updated`: `YYYY-MM-DD`。`owners`: 文字列配列。`title`: 空でない文字列。
- `relations`: 既知 relation field は文字列配列、`changes` は 6 field(added/modified/deleted/renamed/moved/generated)それぞれ `{type: string, ...} の配列`。未知 key は strip され error にならない。
- `metadata`: オブジェクト必須で `experiments: {adopted: string[], rejected: string[]}` を持つ。`metadata` 内の未知 key(`issues` 等)は strip されるため保持してよい。

## Front matter 是正の分類と判断

| 分類 | 対象 | 是正 |
| --- | --- | --- |
| A. `metadata` 欠落 | gt-05, issue-181, issue-184, issue-188-timed-midi, issue-196, issue-198, issue-298, issue-314, issue-386, issue-388-slice3, issue-411, issue-423, issue-426, issue-238, issue-242, issue-328 | 末尾に `metadata: {experiments: {adopted: [], rejected: []}}` を追加。gt-05 は `relations` 内に誤配置された `experiments:` ブロックを `metadata` へ移す |
| B. `metadata.experiments` 欠落 | issue-388-layout-poc(`issues: [388]`), issue-389-vst3(`issues: [389]`) | 既存 `metadata` に `experiments` を追加し `issues` は保持 |
| C. `changes` が文字列 | issue-157, issue-167, issue-187, issue-278-records, issue-278-export, issue-290, issue-291, issue-292, issue-293 | 各 `- "path"` を `- type: "file"\n  path: "path"` に変換。ディレクトリを指すもの(`crates/ruviform-viz`、末尾 `/` の `src/<module>/`)は `type: "directory"`。repo 内の既存慣行(`type: "file"` が docs/*.md にも使われる)に合わせ file/directory の 2 値に限定 |
| D. `type`/`status` 不正 | issue-238, issue-242, issue-328, issue-329 | `type: "impl-record"` → `"impl"`、`status: "done"` → `"completed"`(本文・タイトルが実装完了を示し、enum の完了相当値は completed のみ) |
| E. front matter 自体なし | issue-259, issue-260 | 完全な front matter を新規追加。`id` は `IMPL-<22文字 Base62>` を新規採番(既存 id の変更ではない)。`status` は本文の "delivered (headless-verified)" → `"completed"`。`created`/`updated` は git 初回コミット日 2026-11-03。`owners` はコミット author に合わせ `["scarecrow173"]`。`relations.source` は各 Issue URL、`related`/`derives-from` は親設計 `docs/designs/issue-257-driver-neutral-ui-contract.md` |

判断に迷った値と保守的な選択:

- `status: "done"` → `"completed"`: impl enum に done はなく、完了系の唯一の値は completed。作業は完了済みで本文にも検証記録がある。
- `status: "delivered (headless-verified)"` → `"completed"`: 同上。
- `type` 値: 既存の正しい文書が file/directory/doc/crate/workflow/template/task を混在使用するが、schema は非空文字列のみ要求。意味を変えない最小の選択として file/directory に統一(ディレクトリ判定は trailing `/` または repo 内実在ディレクトリ)。
- `relations` 内の未知 key(`satisfies` 等)と `metadata.issues`: schema が strip するためそのまま保持(情報を消さない)。

## Index marker 配置

- `docs/impl/exp/README.md`: 手書きの 3 列キュレーション表・Reports 節を全て generated region の前(preamble)に残し、ファイル末尾に managed region(marker + `Directory:` + 1 列表)を追加する。managed region より後に `|` 行を置くと `stray-table-row` で reject されるため、marker は必ず最後の表以降に置く。
- `docs/impl/ir/README.md`: 既存内容はほぼ generated 形式(4 列表)だが marker がない。H1 と `Directory:` 行の間に marker を挿入するだけで parse 可能になり、`index apply` が active/settled 2 層・ファイル名順・セル同期を再生成する(現行 161 行に対し source は 240 件で、大幅な drift があるのは想定内)。
- 再生成は owner の safe sync API(`sync_impl_index.js --kind <ir|exp> --apply`、または `doc_maintenance.js index apply`)のみを使い、README を手編集しない。

## Verification

- `index plan --dir docs/impl/ir` / `--dir docs/impl/exp` → `in-sync`(apply 後)。
- `doc_maintenance.js index apply`(引数なし)→ exit 0。
- `audit_impl_record.js` / `audit_experiment_log.js` を変更前後で実行し error 数を比較。baseline: ir 187 / exp 7。front matter 修正で invalid-front-matter が減少し、schema 適合化により missing-section が可視化される分を分けて報告する。
- `git diff` で既存文書の本文・ファイル名・id が不変であることを確認(front matter のみの差分)。
