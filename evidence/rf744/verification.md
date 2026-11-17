# Issue #744 検証記録（2026-11-17）

`docs/tasks/issue-744-job-executor.md` の実装 slice で実施した
検証の実測記録。環境: Windows 11（MINGW64/Git Bash）、
rustc 1.99.1・cargo 1.99.1（mise pin）。canonical 検証は
`mise run` 経由、cargo 直接実行は補助証拠として区別する。

## cargo / mise 検証（canonical = mise）

| コマンド | OS | toolchain | exit | 結果 |
| --- | --- | --- | --- | --- |
| `mise run check ruviform-exec` | Windows 11 x86_64 | rustc 1.99.1 | 0 | Pass（`cargo check --locked --all-targets`） |
| `mise run lint ruviform-exec` | 同上 | 同上 | 0 | Pass（`cargo clippy --locked --all-targets -p ruviform-exec -- -D warnings`） |
| `mise run test ruviform-exec` | 同上 | 同上 | 0 | Pass（unit 39 + doctest 9、fail 0） |
| `mise run fmt-check` | 同上 | 同上 | 0 | Pass（`cargo fmt --all -- --check`。初回差分を `mise run fmt` で整形後に再実行して pass） |
| `mise run check ruviform-audio` | 同上 | 同上 | 0 | Pass（job.rs 変更の dependent 影響確認） |
| `mise run check ruviform-cli-sample` | 同上 | 同上 | 0 | Pass（同上） |
| `mise run check ruviform-ui` | 同上 | 同上 | 0 | Pass（同上） |
| `mise run check ruviform-viz` | 同上 | 同上 | 0 | Pass（同上） |
| `mise exec -- prek run --files <変更ファイル>` | 同上 | node/prek（mise 管理） | 0 | Pass |

補足: `JobState::Queued`・`JobError::Cancelled` は
`#[non_exhaustive]` enum への additive variant で、
dependent 4 crate の `check` が全て pass したことから
下流互換を実測確認した。

## doc_status lint（post のみ — baseline は main 既存 finding が大量のため差分評価）

- `evidence/rf744/doc-status-lint-{adr,designs,specs,plans,tasks,discovery,test-specs,impl}-post.txt`
- issue-744 由来の finding: `index-missing-entry`/
  `orphan-index`/`orphan-navigation`（docs index 未同期 —
  Coordinator が統合時に再生成で解消する許容済み warning）と
  `inconsistent-reciprocal-relation`（#727 design への
  derives-from — issue-721/716 と同型の許容 warning）、
  plan の `traceability-missing-upstream`/
  `plan-missing-test-spec-evidence`（relations を 727 先例
  どおりに修正済み — 修正後の finding は index 系のみ）。
- `docs/impl` は shared-suite 非対応で usage 出力
  （baseline と同じ scope mismatch、exit 1 は仕様どおり）。

## route_graph / impl-doc audits

- `route_graph.js --json` → `evidence/rf744/route-graph-post.json`
  （exit 0。blocked は main baseline 由来の既存 finding で、
  新規文書が新たな broken-relation を追加していない）。
- `audit_impl_record.js --dir docs/impl/ir` →
  `evidence/rf744/impl-ir-audit.txt`（exit 0。issue-744 の
  warning は `evidence/rf744/verification.md` 未作成時点の
  missing-target と README index 未同期のみ — 本ファイル作成で
  target は解消）。
- `audit_experiment_log.js --dir docs/impl/exp` →
  `evidence/rf744/impl-exp-audit.txt`（exit 0。同上）。

## NotRun / Unsupported

- `*-msrv`・`*-full`・3 OS 横断・実機/native/GUI gate —
  POLICIES.md §5 と Issue #744「実機/native gate: 不要」
  により対象外（daily CI 委譲）。Unsupported: なし。

## review-protocol

- self-review 実施済み（下記「レビュー結果」参照）。
  blocker なし。

## レビュー結果

- review-protocol: 1 回実施。blocker なし。
  - 確認観点: #727 API 契約との一致（公開 5 型・bounded 性・
    協調 cancel・join・shutdown・non-RT contract）、
    `OfflineJob` 非破壊、`#[non_exhaustive]` 網羅、
    deadlock contract の明記、テストの契約カバレッジ、
    docs chain の relations 整合。
  - 指摘対応: `try_wait` の `Disconnected` を `Panicked` に
    写像する内部 fault 経路をコメントで明記、`wait` の
    result 消費済み panic を doc に明記。blocker 相当の
    残件なし。
