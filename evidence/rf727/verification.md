# Issue #727 検証記録（2026-11-17）

`docs/tasks/issue-727-job-executor.md` の docs-only slice で実施した
検証の実測記録。環境: Windows 11 x86_64、Node.js（repo 標準）、
Rust toolchain は docs-only のため未使用。

## doc_status lint（baseline vs post）

- baseline: `evidence/rf727/doc-status-lint-*-baseline.txt`
  （変更前・main 状態で採取）
- post: `evidence/rf727/doc-status-lint-*-post.txt`
  （変更後で採取）
- 対象 dir: `docs/adr`・`docs/designs`・`docs/specs`・
  `docs/plans`・`docs/tasks`・`docs/discovery`・
  `docs/test-specs`・`docs/impl`
- 結果: 全 dir exit 0（`docs/impl` は shared-suite 非対応で
  usage 出力・baseline と同じ scope mismatch）。
- 新規文書由来の finding: index 未同期由来の
  `index-missing-entry`/`orphan-index`/`orphan-navigation`
  warning のみ（Coordinator が統合時に index 再生成で解消）。
  `traceability-missing-upstream`（spec）と
  `inconsistent-reciprocal-relation` は issue-721/issue-716
  の先例と同型の許容 warning。新規 blocking finding なし。

## route_graph

- baseline: `evidence/rf727/route-graph-baseline.json` —
  `status: blocked`（probe で `artifact-graph`・
  `focus-required`・既存 `broken-relation:*` が多数 — main の
  baseline 状態）。
- post: 変更後も同等の blocked（新規文書が新たな
  broken-relation を追加していないことを確認）。

## ADR scripts

- `node .agents/skills/adr-doc/scripts/review_adr.js --dir docs/adr`
- `node .agents/skills/adr-doc/scripts/audit_adr.js --dir docs/adr`
- `node .agents/skills/adr-doc/scripts/check_code_links.js --dir docs/adr`
- 結果は本ファイル末尾の「実行結果」節に記録。

## impl-doc audits

- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --dir docs/impl/ir`
- `node .agents/skills/impl-doc/scripts/audit_experiment_log.js --dir docs/impl/exp`
- 結果は本ファイル末尾の「実行結果」節に記録。

## fmt / prek

- `mise run fmt-check`（`cargo fmt --all -- --check`）— docs-only
  のため差分なし見込み。exit code を記録。
- `mise exec -- prek run --files <変更ファイル>` —
  trailing-whitespace/end-of-file/mixed-line-ending 等の軽量衛生。
  exit code を記録。

## NotRun / Unsupported

- NotRun: `*-msrv`・`*-full`・3 OS 横断・cargo 系 lane
  （`check`/`lint`/`test`/`build`）— docs-only slice のため
  POLICIES.md §5 により対象外。
- Unsupported: なし。

## 実行結果

| コマンド | exit | 結果 |
| --- | --- | --- |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/adr` | 0 | Pass（baseline と同等 + index 未同期 warning のみ） |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/designs` | 0 | Pass（同上） |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/specs` | 0 | Pass（同上 + precedented traceability warning） |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/plans` | 0 | Pass（同上） |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/tasks` | 0 | Pass（同上） |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/discovery` | 0 | Pass（新規文書なし・baseline と同一） |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/test-specs` | 0 | Pass（index 未同期 warning のみ） |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/impl` | 1 | Scope mismatch（shared-suite 非対応 — baseline と同じ。impl 専用 audit で別途確認） |
| `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --json` | 0 | `status: blocked` — baseline と同一の blocker 集合（43→43、added/removed 0）。新規 broken-relation なし |
| `node .agents/skills/adr-doc/scripts/review_adr.js --dir docs/adr` | 0 | Pass（warning のみ — ADR-centric な relation 解決のため非 ADR target は全件 warning。既存 ADR と同型） |
| `node .agents/skills/adr-doc/scripts/audit_adr.js --dir docs/adr` | 0 | Pass（同上 + index 未同期 warning） |
| `node .agents/skills/adr-doc/scripts/check_code_links.js --dir docs/adr` | 0 | Pass（warning のみ — Implementation Plan の repo-root 相対パスは docs/adr 起点解決のため missing 判定。既存 ADR と同型。本 ADR の line-break 由来 fragment は修正済み） |
| `node .agents/skills/impl-doc/scripts/audit_impl_record.js --dir docs/impl/ir` | 0 | Pass（index 未同期 warning のみ） |
| `node .agents/skills/impl-doc/scripts/audit_experiment_log.js --dir docs/impl/exp` | 0 | Pass（同上） |
| `mise run fmt-check`（`cargo fmt --all -- --check`） | 0 | Pass（docs-only・差分なし） |
| `mise exec -- prek run --files <変更ファイル>` | 0 | Pass（trim trailing whitespace / end of files / mixed line ending / utf-8 BOM / private key — 全て Passed） |

補足: `doc_status lint` の新規文書由来 finding は
`index-missing-entry`/`orphan-index`/`orphan-navigation`
warning と precedented `traceability-missing-upstream`
（spec）のみ。`docs/**/README.md` index は Coordinator が
統合時に再生成するため本 slice では未同期のまま。
