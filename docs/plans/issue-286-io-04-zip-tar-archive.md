---
id: "PLAN-034UAvKoJIuTCFJwOSaF5Z"
type: "plan"
status: "approved"
title: "Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/286"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/specs/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/plans/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/tasks/issue-286-io-04-zip-tar-archive.md"
      - type: "doc"
        path: "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-286-io-04-zip-tar-archive.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計 Implementation Plan

## Goal

Issue #286（IO-04 ZIP/TAR Archive — Entry API・安全な展開・
終了/公開契約）の合意済み設計（v0.1・v0.2・2026-11-04・
最終整合性レビュー追記）を、形式別 Reader/Writer/Entry
契約・Backend 隠蔽・所有権/finish・検証済み終了 Receipt・
Secure Extraction の脅威モデル・終了/公開契約・Format 範囲・
Consumer 利用面・証拠項目対応を含む正本設計書として
`docs/` チェーンに文書化する（docs のみ。コード変更なし）。

## Scope

単一実装ストリーム・1 plan・1 task。変更は `docs/` の
`issue-286-*` 新規文書 5 件と生成 index のみ。
`crates/`・`.github/`・`mise.toml`・`Cargo.*`・既存他
Issue 文書には触れない。per-implementation の詳細設計・
実走テスト・Consumer 実測・matrix 実走は対象外（実装/
受入段階の別 Issue/dispatch）。

## Upstream Documents

- [docs/specs/issue-286-io-04-zip-tar-archive.md](../specs/issue-286-io-04-zip-tar-archive.md)（approved）
- Issue #286 本文（合意済み設計 v0.1・v0.2・2026-11-04・
  最終整合性レビュー追記 — 受入正本）
- Issue #282（親・汎用 I/O 基盤 横断設計正本）

## Design Inputs

- [docs/designs/issue-286-io-04-zip-tar-archive.md](../designs/issue-286-io-04-zip-tar-archive.md)（approved）
- 参照正本: `docs/designs/issue-302-io-archive-zip.md`・
  `issue-303-io-archive-tar.md`・`issue-304-io-archive-
  extract.md`（per-implementation 詳細正本）、`docs/designs/
  issue-295-…`・`issue-296-…`（IO-01 File 契約）、
  `docs/designs/issue-300-…`・`issue-301-…`（IO-03
  Compression）、`docs/designs/issue-288-…`（cross-consumer
  gate）、`docs/designs/issue-306-…`（合成 E2E）、
  `docs/designs/issue-409-…`・`rooted-read-capability.md`
  （rooted capability）、`docs/designs/
  ruviform-architecture-design-baseline.md`（DAG/RT 正本）、
  `crates/ruviform-io-archive/`（実装済み crate — 設計の
  確定値の根拠）。

## File Map

- `docs/specs/issue-286-io-04-zip-tar-archive.md` — spec
  （何を・なぜ・範囲・受入条件）。
- `docs/designs/issue-286-io-04-zip-tar-archive.md` — IO-04
  横断設計正本（形式別 Reader/Writer/Entry 契約・Backend
  隠蔽・所有権/finish・Receipt 語彙・Secure Extraction 脅威
  モデル・終了/公開分離・Format 範囲・Feature/Backend 確定
  値・DAG/RT・Consumer 利用面・証拠項目対応・設計/実装
  分離）。
- `docs/test-specs/issue-286-io-04-zip-tar-archive.md` —
  設計文書の正しさを検証する test-spec（R1–R12）。
- `docs/plans/issue-286-io-04-zip-tar-archive.md` — 本計画。
- `docs/tasks/issue-286-io-04-zip-tar-archive.md` — 単一
  task（docs チェーン作成の実施記録）。
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`
  — 生成 index。

## Implementation Sequence

- [x] Issue #286/#282 本文と関連 Issue 状態（#283〜#306・
  #409・#219・#294・AL #139/#143・SV #17/#25/#82/#84）を
  `gh`/`git log` で棚卸し（2026-11-09 snapshot）。
- [x] 既存 IO 文書チェーン（#295/#296・#300/#301・
  #302/#303/#304・#288・#305/#306・#409）と
  `crates/ruviform-io-archive` の実装（src/{zip,tar,
  extract,entry,limits,error,receipt}.rs・Cargo.toml
  feature matrix・crate README）を走査し、参照先の実在を
  確認。
- [x] spec を作成（Issue #286 AC の dispatch スコープ対応
  付け）。
- [x] design（IO-04 横断設計正本）を作成 — 形式別契約・
  Backend 隠蔽・所有権/finish・Receipt・Secure Extraction・
  終了/公開契約・Format 範囲・Consumer 面・証拠項目対応。
- [x] test-spec / plan / task を作成。
- [x] docs index を再生成（`doc_maintenance.js apply`）。
- [x] review-protocol によるレビュー（参照実在性・契約
  語彙の整合・証拠項目対応の網羅を確認 — blocker なし）。
- [x] `git fetch origin main && git merge origin/main` で
  最新化し index を再生成・再検証（Already up to date —
  `95b0323f` 以降の差分なし）。
- [x] `mise run fmt-check` + doc-status 系 lint を実行。
- [x] commit → push → PR 作成（`Refs #286`）。

## Dependencies and Risks

- 参照先の Issue 状態は変動し得る（AL #139/#143 CLOSED、
  SV #82 CLOSED・#84 OPEN・#25 CLOSED — 2026-11-09
  snapshot）。本書は snapshot として記録し、接続表の
  「役割」記述が将来古くなる可能性を test-spec Risk に
  明記済み。
- `docs/specs/issue-219-consumer-compat-contract.md` は
  status `proposed` — 影響分類語彙の参照正本として spec の
  `depends-on` で接続するが、本 slice の承認対象ではない。
- 既存の broken relation（`docs/impl/ir/issue-243-…`・
  `issue-313-…`・`issue-410-…` 等、`route_graph.js` が
  報告）は本タスク範囲外の既存問題であり、doc-status 全体
  lint がこれを報告する可能性がある — 本 PR では修正せず
  証拠として報告する。
- per-implementation の詳細（parser 内部・buffer 戦略・
  fixture 行列・OS backend 実装）は #302/#303/#304 が正本。
  本書は横断契約と索引のみを所有し、記述の drift は参照
  構造で緩和する。per-implementation test-spec の rule 番号
  が改訂された場合は本書の対応表も追従が必要。

## Task Breakdown

- T1: spec 作成（本 task に包含 — docs チェーンは 1 単位の
  論理変更）。
- T2: design（IO-04 横断設計正本）作成。
- T3: test-spec / plan / task 作成 + index 再生成。
- T4: review-protocol レビューと blocker 解消。
- T5: main 取り込み・最終検証・commit/push/PR。

## Verification

- [x] `mise run fmt-check` pass。
- [x] doc-status 系 lint/audit で本 PR 追加分に broken
  relation・未解決参照がないこと（既存の範囲外 finding は
  分離して記録 — audit_docs で issue-286 関連 finding 0）。
- [x] 設計文書の参照先（Issue/PR/test-spec/mise task/
  crate/doc パス）が実在することを作成時に確認済みで
  あること。
- [x] PR diff が `docs/` の新規 `issue-286-*` 文書と生成
  index のみであることを `git status`/`git diff` で確認。

## Open Questions / Gaps

- Consumer 実測・consumer-verify tuple run・macOS/Linux
  実機・性能比較は実装/受入段階の要件であり本 slice では
  `not run`/`NotRun`/`Unsupported` として記録する（解消
  しない gap ではなく意図的な分離）。
- ZIP64・sparse/special/link・Directory 原子的 Publish・
  永続 Recovery Journal は Deferred Design Concerns として
  設計文書に記録（実需・要件確定待ち）。
- `openat2`/`RESOLVE_BENEATH`・`cap-std` 採用は Deferred
  （現行の dirfd/handle 相対 + no-follow + CreateNew +
  identity 照合で主要脅威をカバー）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology
  are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no（review-protocol
  で代替）
- If approved: discover available implementation and
  delegation capabilities in the current environment, then
  execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- Reader/Writer/Entry モデル節が ZIP `Read + Seek` indexed・
  TAR `Read`/`Write` streaming の形式別 API 族・共通 format
  中立型・「形式が実在する情報のみ記録」を漏れなく固定し、
  #302/#303 設計への参照を持つか。
- Backend 隠蔽節が外部 `zip`/`tar` 非採用・内部 Parser +
  非公開 codec・Serde 非依存・`std::io` ベース・
  `source()` のみ診断を #302/#303/#304 設計と矛盾なく
  記述しているか。
- 所有権/状態機械節が Entry 排他借用・明示 `finish` のみ
  が完了証拠・TAR `Ready`/`EntryOpen`/`Pending`/`Failed`・
  `discard_pending`/`discard_remaining`・Writer poison を
  含むか。
- Receipt 節が `ArchiveReadReceipt`/`EntryReadReceipt` の
  typed summary・`EntryChecksum`/`EntryDisposition`・
  `verified: bool` 非丸め・真正性 ≠ 整合性を含むか。
- Secure Extraction 節が信頼モデル・Path Policy・canonical
  衝突・kind/budget・OS/FS 別保護レベル表・TOCTOU 境界・
  Cleanup/Recovery を含むか。
- Format 範囲表の受理/`Unsupported` が `Cargo.toml` 実装・
  crate README・#302/#303/#304 設計に一致するか。
- Compression 組合せ節が tar.gz/tar.zst の 2 層 finish
  独立・ZIP 内部 codec の capability 閉じ込め・IO-03 への
  必須依存なしを含むか。
- 終了/公開契約節が Entry→Archive→Compression→StagedFile
  publish の finish 連鎖分離・`ExtractedTree` ≠ Directory
  Publish・無条件 Atomic Directory Publish 非保証を含むか。
- 証拠項目対応表の API/ZIP/TAR/SEC/EXT/INT/COMP/DEP/OWN/
  WRT/REC/OS 各行が実在する test-spec rule/節・#288 matrix
  行（A-N1〜A-X5）・Issue/PR 番号へ解決できるか。
- Consumer 利用面節が AL #139/#143・SV #82/#84/#25・SV
  Project #17 の Issue 番号と責務分界（schema は Consumer・
  `ExtractedTree` ≠ Project Transaction）が #288 と同一
  原則か。
