---
id: "TSPEC-034YYs26BK9ql96HKlJEJQ"
type: "test-spec"
status: "approved"
title: "Issue #721 post-seal temp 境界・bench-smoke 配線 テスト仕様"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/721"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-306-io-e2e-cases.md"
    - "docs/test-specs/issue-650-bench-smoke-coverage.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-721-post-seal-temp.md"
    - "docs/designs/issue-721-post-seal-temp.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/721"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/650"
  defers: []
  deferred-by: []
metadata:
  issue: 721
---

# Issue #721 post-seal temp 境界・bench-smoke 配線 テスト仕様

## Purpose

`StagedFile::seal()` 後〜`SealedFile::publish()` 前の staging
temp 外部改竄に対する契約境界が「現行契約維持」として確定した
ことを、既存試験と文書で pin し続けることを保証する。本 spec が
守る意図は「post-seal temp の内容完全性は呼び出し側信頼境界内の
事象であり、io-file は再検証しない」という境界の正直な記録で
あり、Issue #306 で pin された契約の判断本体を本 Issue で
確定したことを検証可能な形で残す。

あわせて残件 (a) として、`mise run bench-smoke` が
`ruviform-io-e2e` `io_composition` を計測なし 1 回実行する
配線が main 上で機能していることを実測で保証する。

## Feature

post-seal temp 改竄境界の契約 — `publish` は seal 後の temp
内容を再検証せず、外部 rewrite は改竄 bytes として publish
され、外部 delete は `NotPublished` になる。契約を pin するのは
`crates/ruviform-io-e2e/tests/staged_lifecycle.rs` の
`post_seal_temp_*` 2 試験であり、本 Issue はその契約が
設計文書・README・実測証拠と整合することを検証する。

## Rules

- **R1（契約 pin の維持）**: `post_seal_temp_rewrite_is_not_reverified_before_publish`
  は seal 後の外部 rewrite に対し
  publish 成功 + destination が改竄 bytes を持つことを assert
  し続ける。`post_seal_temp_delete_fails_publish_not_published`
  は外部 delete に対し `NotPublished` + destination 非生成を
  assert し続ける。いずれも実 FS で再現可能な振る舞いのみを
  assert する io-e2e の責務分離を維持する。
- **R2（文書整合）**: `docs/designs/issue-721-post-seal-temp.md`
  の決定（現行契約維持・強化案却下理由・consumer 照合）が
  spec の Requirements と矛盾しない。`crates/ruviform-io-file/
  README.md` §信頼境界の記述が採用契約と矛盾しない
  （post-seal 再検証なしの明示を含む）。
- **R3（回帰なし）**: io-file/io-e2e の既存 suite が doc のみの
  変更で回帰しない — `mise run test ruviform-io-file` /
  `mise run test-io-e2e` が pass する。
- **R4（bench-smoke 実測）**: `mise run bench-smoke` が
  `ruviform-io-e2e` `io_composition` の 6 case
  （`staged_write`・`staged_seal`・`staged_publish`・
  `staged_readback`・`tar_gzip_staged`・`tar_zstd_staged`）を
  計測なし 1 回実行して exit 0。`docs/development-environment.md`
  の bench 系記述が `mise.toml` `[tasks.bench-smoke]` と一致する
  ことを read-only で照合する。
- **R5（検証レベルの分離）**: headless/fixture lane の Pass を
  実 FS fault の OS 別挙動差（Windows ACL/readonly・POSIX
  share-lock 等）の受入に昇格しない。OS 別差が残る場合は
  環境別に NotRun/Unsupported を分離記録する（POLICIES.md §4）。

## Examples

- **E1（rewrite pin）**: Given `StagedFile` で bytes を write し
  `seal()`、When `SealedFile::temp_path()` の temp を
  `std::fs::write` で外部 rewrite（長さ変更あり）、Then
  `publish(CreateNew, FileAndDirectorySync)` が成功し
  destination に改竄 bytes が入り、temp は残らない。
- **E2（delete pin）**: Given seal 済み `SealedFile`、When temp
  を `std::fs::remove_file` で外部 delete、Then publish が
  `PublishEffect::NotPublished` で失敗し destination は生成
  されず、recovery handle の cleanup が `TempState::Absent`
  で成功する。
- **E3（suite 回帰）**: Given doc のみの変更、When
  `mise run test ruviform-io-file` と `mise run test-io-e2e`、
  Then 両者 exit 0 で既存試験全件 pass。
- **E4（bench-smoke 実測）**: Given main の `mise.toml`、When
  `mise run bench-smoke`、Then `io_composition` の 6 case が
  `Testing ... Success` で実行され exit 0（実測証拠は
  `evidence/rf721/`）。
- **E5（doc 照合）**: Given `docs/development-environment.md`
  の `bench`/`bench-smoke` 行、When `mise.toml` の
  `[tasks.bench]`/`[tasks.bench-smoke]` と照合、Then `-p`
  列挙・説明が一致する（drift 時は `orca orchestration ask`
  経路 — 本 slice は同ファイルを編集しない）。

## Guarantees

- 採用契約（publish は post-seal temp を再検証しない）が
  設計文書・README・`post_seal_temp_*` 試験・本 test-spec で
  一貫して記録される。
- 「強化」実装を将来行う場合、`post_seal_temp_rewrite_*` の
  期待更新が契約変更のトリガーとして機能する（#306 design の
  意図した固定）。
- bench-smoke lane の `ruviform-io-e2e` 配線が実測で確認され、
  #646 型の coverage gap が再発していないことが証拠化される。

## Non-goals

- post-seal 改竄への新規防御実装の検証（本 Issue は維持判断）。
- 実 FS fault の OS 別挙動差の網羅検証（Windows ACL/readonly・
  POSIX share-lock の環境別差は別 lane の責務）。
- bench の計測値・性能回帰判定（smoke は計測なし 1 回実行）。
- `docs/development-environment.md` の編集を伴う検証
  （`scarecrow173/ruviform#720` 占有中 — read-only 照合のみ）。

## Risk

- 契約が文書間で drift すると「seal = publish までの完全性保証」
  と誤読される — R2 の文書整合ルールと README 補強で緩和。
- `post_seal_temp_*` 試験が削除・改名されると境界の pin が
  失われる — io-e2e suite の pass（R3）と test 名の機械的
  存在が検出点。
- bench-smoke 配線の将来の drift（bench target 追加時の `-p`
  列挙漏れ）は #650 と同じ構造的リスク — 本 slice は実測確認
  のみで恒久対策は対象外。
- headless Pass を実 OS 受入に昇格させる誤判定 — R5 で分離を
  強制。

## Sources and Relations

- 検証対象: `docs/specs/issue-721-post-seal-temp.md`・
  `docs/designs/issue-721-post-seal-temp.md`（本 spec の verifies）。
- 契約 pin 正本: `crates/ruviform-io-e2e/tests/staged_lifecycle.rs`
  （`post_seal_temp_*`）、`docs/designs/issue-306-io-e2e-cases.md`
  Case 1/Case 2。
- 関連 test-spec: `docs/test-specs/issue-306-io-e2e-cases.md`・
  `docs/test-specs/issue-650-bench-smoke-coverage.md`。
- 実測証拠: `evidence/rf721/`。
- 関連 Issue: `scarecrow173/ruviform#721`・`#306`・`#650`・`#646`・
  `#720`（並行 Dispatch・dev-env doc 占有）。
