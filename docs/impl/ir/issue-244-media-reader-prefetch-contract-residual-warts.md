---
id: "IMPL-034QxLzTwkgQnhtvv7jXvB"
type: "impl"
status: "completed"
title: "Issue 244 media-reader prefetch contract residual warts"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/244"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-244-media-reader-prefetch-contract-warts.md"
      - type: "file"
        path: "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
      - type: "file"
        path: "docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md"
      - type: "file"
        path: "docs/designs/issue-244-media-reader-prefetch-warts.md"
      - type: "file"
        path: "docs/plans/issue-244-media-reader-prefetch-warts.md"
      - type: "file"
        path: "docs/test-specs/issue-244-media-reader-prefetch-warts.md"
      - type: "file"
        path: "docs/tasks/issue-244-t1-boundary-first.md"
      - type: "file"
        path: "docs/tasks/issue-244-t2-end-commit-once.md"
      - type: "file"
        path: "docs/tasks/issue-244-t3-seeded-coverage.md"
      - type: "file"
        path: "docs/tasks/issue-244-t4-verification-record.md"
      - type: "file"
        path: "docs/impl/ir/issue-244-media-reader-prefetch-contract-residual-warts.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/README.md"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-244-t1-boundary-first.md"
    - "docs/tasks/issue-244-t2-end-commit-once.md"
    - "docs/tasks/issue-244-t3-seeded-coverage.md"
    - "docs/tasks/issue-244-t4-verification-record.md"
    - "docs/plans/issue-244-media-reader-prefetch-warts.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
    - "docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md"
    - "docs/designs/issue-244-media-reader-prefetch-warts.md"
    - "docs/discovery/issue-244-media-reader-prefetch-contract-warts.md"
    - "docs/impl/ir/issue-233-media-reader-prefetch-contract-improvements.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-244-media-reader-prefetch-warts.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-244-media-reader-prefetch-warts.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 244 media-reader prefetch contract residual warts

## Summary

Issue #233 review で残存した 4 wart を全て解消した。2 件の P2 は ADR (Option A) で承認された**無条件の contract semantics 修正**として実装: failed seek / failed reopen で `Boundary` が先に emit されるよう producer emission order を矯正し、reopen 失敗は `Boundary{gen,0}` → `Fault{gen,of_kind}` → `End{gen}` の dead-generation sequence で observably に終端する。2 件の P3 は `end_emitted` の commit-once 化と seeded-fault fixture による `discard_pending` coverage 追加。公開 signature / type / enum variant の変更・consumer 側ロジック変更は一切なし。

## Context

- 実施タスク: `docs/tasks/issue-244-t1-boundary-first.md`〜`issue-244-t4-verification-record.md`
- 上流: `docs/plans/issue-244-media-reader-prefetch-warts.md`、`docs/designs/issue-244-media-reader-prefetch-warts.md`、`docs/specs/issue-244-media-reader-prefetch-contract-warts.md`、`docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md`
- 動機: #233 review で検出された #197 pre-existing contract wart の残件（boundary adoption 前の Fault stale-drop・reopen 失敗時の generation 未 adopt starve・duplicate End・coverage 欠落）。

## Implementation

### Changes

- `crates/ruviform-media-reader/src/prefetch.rs`
  - `exec_cmd` `Seek` 拒否 arm: `Fault` → `Boundary` の push 順を `Boundary{gen, position}` → `Fault{gen, Unseekable}` に反転。consumer が generation を adopt してから fault を観測するため、`fault_events`/`faults.unseekable` に計上され `stale_drops` に落ちない。
  - `exec_cmd` `Reopen` `Some(first)` post-reopen seek 失敗 arm: 同様に `Boundary` → `Fault` 順へ反転。
  - `exec_cmd` `Reopen` `Err` arm: `Boundary{gen, 0}` を `Fault` 前に追加 — dead generation は `Boundary` → `Fault` → `End` で終端し、consumer は generation を adopt・fault を計上・`eof` を報告する（starve 解消）。
  - `pump` step 2: `end_emitted = true` を `try_push` 前に移動 — ring full で `pending` へ requeue された `End` は合成時点で commit 済みとなり、再合成されない（generation あたり `End` は正確に 1 回）。
  - Module `## Contract`: generation bullet に boundary-first emission invariant、faults bullet に `End` commit-once を追記。`seek`/`seek_discard_pending`/`reopen`/`reopen_at` rustdoc の failure-path 記述を修正済み semantics に更新（`reopen_at` の旧 wart 記述 2 paragraph を置換）。
- `crates/ruviform-media-reader/README.md` — prefetch contract bullet に boundary-first emission・dead-generation `Boundary→Fault→End`・`End` commit-once を追記（日本語、既存スタイル）。
- `crates/ruviform-media-reader/tests/prefetch.rs`
  - `SeededSource` fixture 追加 — `Arc<AtomicBool>` gate で `Read`/`Seek` を失敗化（construction 時の symphonia probe は unarmed で通過し seekable `Fact::known` を確保、arm 後の最初の operation が原子的に失敗）。
  - `rd02_reopen_at_seek_failure_faults_then_reanchors` を更新 — 旧 `stale_drops >= 1` pin を `fault_events >= 1` + `stale_drops == 0` の修正済み observable に置換（suite 内で唯一 semantics を pin していたテスト）。
  - 新規 4 テスト:
    - `rd02_seek_failure_reports_fault_after_boundary` — armed seek 失敗で `faults.unseekable == 1`・`fault_events`・`stale_drops == 0`・frame-0 re-anchor。
    - `rd02_reopen_failure_adopts_dead_generation` — failing `open` closure で boundary adopt（`generation == g0+1`・`position == 0`）・`faults.io == 1`・`eof`・`stale_drops == 0`。
    - `rd02_end_committed_once_under_backpressure` — `block_count: 1`（data-ring 9 slot）を block 1 + failed reopen 4 回の 8 marker で飽和、requeue された `End` が push された後の次 `pump` が `Idle` を返すことを assert（duplicate End 抑止）。
    - `rd02_discard_pending_applies_matching_fault_and_end_then_boundary` — seeded read 失敗で `Fault{gen}` + `End{gen}` を ring に積み、`discard_pending` が matching-gen fault を計上・`eof_seen` を立て、boundary adopt で `ended` を clear（post-drain content 復帰で証明）。

### Contract amendment — observable behavior changes (migration note)

#197 semantics からの**意図的な観測挙動変更**（ADR Option A、opt-in ではなく無条件修正）。API signature・type・enum variant は不変で source-compatible。

| Path | Before | After |
| --- | --- | --- |
| Rejected `seek` / `seek_discard_pending` | `Fault` が `Boundary` 先行で emit → consumer は old generation のまま stale-drop（`stale_drops` 増） | `Boundary` → `Fault` — `fault_events` +1・`FillReport.faults.unseekable` に計上 |
| `reopen_at` post-reopen seek 失敗 | 同上（`stale_drops` 増） | 同上（fault 計上 + frame-0 re-anchor） |
| `reopen` / `reopen_at` で `open`/`reopen` 失敗 | `Fault` + `End` のみ・`Boundary` なし → generation 未 adopt のまま stale-drop・starve | `Boundary{gen,0}` → `Fault` → `End` — generation adopt・position→0・fault 計上・`eof` |
| Ring-full `End` requeue | `end_emitted` が立たず duplicate `End` を合成し得た | commit-once — `End` は generation あたり正確に 1 回 |

#### Consumer impact

- **ruviform 内利用**: `stale_drops` が failure path で減少し `fault_events`/per-kind `faults` が増加 — counter の semantic 改善。これら counter で alerting/assertion している箇所は新 semantics で解釈する（in-repo では上記テスト更新 1 件のみ）。
- **synvelis**: **no impact** — Synvelis #24 の quiesce-drain→seek workaround は変更後も有効。failure path での fault observability が改善されるのみで、source-compatible・追加対応不要。
- **audio-lens**: **unknown** — repository 不在のため #219 convention に従い影響不明と記録。下流検証は上記 table の counter/state 変更点を対象に行う。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64、Git Bash、mise 管理 rust 1.99.0。

| 検証 | 結果 |
| --- | --- |
| `mise run fmt-check` | pass |
| `mise run check` | pass |
| `mise run lint` | pass（`-D warnings` 下で `needless_continue` 1 件を修正後 clean） |
| `mise run test` | pass（workspace 全量 green + doctest） |
| `mise run test-rt` | pass（`rt_fill.rs` 3 テスト含む `rt_` lane） |
| `cargo test -p ruviform-media-reader` | pass（prefetch.rs 25 tests・rt_fill.rs 3 tests — 新規 4 + 更新 1 を含む） |
| review-protocol | 2-reviewer 独立レビュー実施 → verdict **ACCEPT WITH NON-BLOCKING NOTES**（詳細は Follow-ups 節） |

## Risks

- **観測 counter の semantic 変化**: failure path で `stale_drops` が減少し `fault_events`/per-kind `faults` が増加する — ADR の migration note で記録済みの意図的変更。counter に依存する下流 assertion は新 semantics で解釈すること（in-repo は `rd02_reopen_at_seek_failure_faults_then_reanchors` 1 件を更新済み）。
- **Dead-generation anchor = frame 0**: reopen 失敗時の `Boundary` は要求された `first` ではなく `0` で anchor — stream が generation N で一度も開始していない honest anchor として ADR で承認済み。要求 target は `Fault.first_frame` に残る。
- **`SeededSource` の atomicity 仮定**: fixture は arm 後の最初の post-open operation を原子的に失敗させる。symphonia adapter が seek probe を複数回呼ぶ場合でも初回失敗で `Err` arm に到達するため deterministic。

## Follow-ups

Review-protocol verdict: **ACCEPT WITH NON-BLOCKING NOTES**。2 reviewer（cooperative/adversarial）計 10 candidate、P0/P1 なし。doc-precision nit（bootstrap generation・superseded dead generation・`seek_discard_pending` purge 例外・`pending` bound・`end_emitted` コメント）と `rd02_reopen_at_seek_failure_faults_then_reanchors` の kind-agnostic assert 緩み（`faults.unseekable` 化）は本 change 内で修正済み。残る CONFIRMED（いずれも pre-existing、本 change の regression ではないため記録・非修正）:

- **`PumpOutcome::Ended` が `End` marker emit の 1 pump 手前でも返る**（P2、新規検出 wart・#244 残件には非該当）: `fill_block` の EOF arm は `stream_ended` を立てて `Ended` を返すが `End` marker は次 pump の step 2 で合成される。documented `Idle`/`Ended` 到達まで pump を反復する manual driver が検出時点 `Ended` で停止すると `End{gen}` が ring に出ず、consumer は `eof` を観測できない。lazy-End model 由来の pre-existing semantics で `PumpOutcome` 契約の変更判断（検出時 Ended を非 terminal 化するか marker を eager に emit するか）が必要 — 別 issue 候補として記録。spawn `run()` は pump 継続するため実害は manual driver のみ。
- **`fill_block` fatal-decode arm が `Pushed` を返すが item は `pending` 行き**（P3、pre-existing outcome mislabel）: ring 到達を示す `Pushed` semantics と不一致 — 上記 `Ended` 問題と同じ outcome-vocabulary wart 群。
- **部分 seek 移動後の失敗で `Boundary` は実 position を報告するが "old position" と読めるコメントがある**（P3、spec/design の deferred concern 記録済み）: `SeededSource`/`NoSeek` の atomic 失敗では不発。reader-level の partial-seek fidelity は本 issue の対象外。
- **mis-sequenced `frames > 0` Fault の triple-attribution**（P3、pre-existing consumer quirk）: 同 marker が `fault_events` + `stale()` + `Step::Faulted` に計上され得る。boundary-first 化で honest sequence からは到達不能 — defensive path として温存。
- **`pump() -> Result` が error を返さない**（P3、vestigial）: `run()` の `Err(_) => break` arm は到達不能。
- consumer-side gen-mismatch `Fault` arm は boundary-first 化により honest sequence では到達不能 — defensive sequencing として温存。
- AudioLens consumer impact は `unknown` のまま — 下流 repo が得られた時点で migration note table に基づき検証する。
- `frames > 0` fault span は引き続き producer が emit しない（test-spec non-goal）。

## Commit

- `fix(issue-244): boundary-first prefetch faults, dead-generation End, commit-once End`（branch `scarecrow173/rf244-prefetch-wart` HEAD、未 push）
