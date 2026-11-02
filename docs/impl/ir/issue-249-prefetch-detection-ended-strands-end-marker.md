---
id: "IMPL-034QAt6Gd3zLwrqmsnq9CK"
type: "impl"
status: "completed"
title: "Issue 249 prefetch detection-Ended strands End marker"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/249"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-249-prefetch-ended-outcome-must-emit-end-marker.md"
      - type: "file"
        path: "docs/impl/ir/issue-249-prefetch-detection-ended-strands-end-marker.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/README.md"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-249-prefetch-ended-outcome-must-emit-end-marker.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-244-media-reader-prefetch-contract-residual-warts.md"
    - "docs/impl/ir/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md"
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

# Issue 249 prefetch detection-Ended strands End marker

## Summary

`fill_block` の EOF arm が `End` marker emit の 1 pump 手前で `PumpOutcome::Ended` を返す wart（#244 review で記録された残件 P2）を修正した。`Ended` の outcome semantics を「`End` marker がその call で ring に載った」に整合させ、EOF 検出と emit を同一 step に畳み込んだ。Emission ロジックは共有 `emit_end` に集約し、fault / dead-generation 経路の lazy emission（`Boundary` → `Fault` → `End` sequencing が必須なため）は `pump` step 2 から同じ helper を呼ぶ形で維持。公開 signature / type / enum variant の変更はなく、source-compatible。#197/#233/#244 の boundary-first emission・dead-generation sequence・`End` commit-once は全て維持される。

## Context

- 実施タスク: `docs/tasks/issue-249-prefetch-ended-outcome-must-emit-end-marker.md`
- 上流: `docs/impl/ir/issue-244-media-reader-prefetch-contract-residual-warts.md` の Follow-ups が本 wart を P2・別 issue 候補として記録 — 「documented `Idle`/`Ended` 到達まで pump を反復する manual driver が検出時点 `Ended` で停止すると `End{gen}` が ring に出ず、consumer は `eof` を観測できない」。
- 修正方針の選択肢は issue のとおり 3 つ: (a) outcome を marker emission に整合させる、(b) drain を保証する、(c) contract を文書修正のみで明示する。`PumpOutcome::Ended` の rustdoc は当初から "the stream's `End` marker was emitted" を約束しており、(a) はその contract を実態に合わせる修正として選択。contract owner は prefetch producer であり consumer 側の変更は一切ない。

## Implementation

### Changes

- `crates/ruviform-media-reader/src/prefetch.rs`
  - `emit_end(&mut self) -> PumpOutcome` を新設 — `End{generation}` を synthesize し `end_emitted` を synthesis 時点で latch（commit-once・#244 を維持）したうえで `try_push`。成功で `Ended`、ring full で `pending` front へ requeue し `Backpressured`。
  - `pump` step 2: inline だった emission block を `self.emit_end()` 呼び出しに置換（fault / dead-generation 経路は `Fault` 等の marker を `pending` 経由で `End` より先に sequence する必要があるため lazy のまま — 経路の非対称性は contract 上必須であり変更なし）。
  - `fill_block` EOF arm: block を re-park し `stream_ended = true` を立てたあと `self.emit_end()` を返す。EOF 検出 pump が `Ended` を返すのは marker が実際に ring に載った場合のみとなり、ring full 時は `Backpressured`（marker は `pending` commit 済み）が返る。`pending` は `fill_block` 到達時点で必ず空（step 1 が先に drain）のため ordering 影響なし。
  - Docs: module `## Contract` Drive bullet・`pump()` rustdoc・`PumpOutcome::Ended` rustdoc を新 semantics に更新し、無 comment だった `stream_ended` field に doc を追加。
- `crates/ruviform-media-reader/README.md` — `pump()` contract bullet に `Ended` semantics（marker が ring に載った call でのみ返る・EOF 検出=emit 同一 step・ring full は `pending` commit 済み `Backpressured`）を追記。
- `crates/ruviform-media-reader/tests/prefetch.rs` — 新規 2 テスト:
  - `rd02_detection_ended_leaves_end_marker_on_ring` — 2 block の source で最初の `Ended` まで `pump()` して即停止（documented driver pattern）。以降の producer pump なしで consumer が `eof` に到達し `telemetry.eof_seen()` となること、その後の `pump` が `Idle` を返す（duplicate End なし）ことを assert。
  - `rd02_end_committed_at_eof_survives_a_later_seek` — `Ended` 後の seek が commit 済み `End{g1}` を飲み込まず、new `Boundary` より先に sequence されることを `discard_pending` の `eof_seen` で assert。

### Contract amendment — observable behavior changes (migration note)

#197 semantics からの**意図的な観測挙動変更**（無条件修正）。API signature・type・enum variant は不変で source-compatible。

| Path | Before | After |
| --- | --- | --- |
| EOF detection pump | `Ended` を marker 未 emit で返す → 検出時点で止める driver は `End{gen}` を strand | `End` を同一 step で emit — `Ended` は marker が ring に載った call でのみ返る（full なら `pending` commit 済み `Backpressured`） |
| `Ended` outcome の回数 | detection + emission の 2 回返り得た | generation あたり最大 1 回（emission pump のみ） |
| `Ended` 後の seek | pending の EOF state が reset され旧 gen の `End` は合成されず沉默 | `End{old}` は検出時点で commit 済みのため seek では消えず、new `Boundary` より先に観測される（`seek_discard_pending` の opt-in purge は従来どおり例外） |
| 検出 pump が ring full | `Ended`（marker 未存在） | `Backpressured`（marker `pending` commit 済み、次 pump の step 1 で確実に push） |

fault arm（fatal decode error）の `End` は従来どおり step 2 lazy emission — `Fault` → `End` の ring ordering 維持に必須であり、同 arm が返す `Pushed` の mislabel は #244 記録済みの別 wart（本 issue 対象外・変更なし）。

#### Consumer impact

- **ruviform 内利用**: `run()` spawn loop は pump を継続するため実害なし・挙動不変。`tests/rt_fill.rs` の `pump until Ended` pattern は修正後も `Ended` を受け取る（marker が実際に載る）。observable 差は「`Ended` が最大 1 回」「`End{old}` が seek に吞まれない」の 2 点で、いずれも contract 上の honesty 改善。
- **synvelis**: **impacted（positive・対応不要）** — local worktree `sv38-editing-poc/crates/synvelis-player/src/driver.rs:248-257` が唯一の manual driver（`Prefetch::manual` は resolver.rs:118）。`while pump()==Pushed` の bounded loop は非 `Pushed` で一律 break するため新 semantics で変更不要。旧 semantics では detection `Ended` で break した場合 `End` は次の `Every(n)` cycle の pump でやっと emit され、`run()` 終端直前に検出されると strand し得た。修正後は detection pump で marker が ring に載るため、終端 cycle での stranding hazard が消える。`Ended` 後の seek で `End{old}` が `Boundary{new}` より先に観測されるが、consumer は generation adopt で正しく処理する（boundary-first invariant と整合）。
- **audio-lens**: **no impact** — local worktree を確認: `ruviform-ui` / `ruviform-ui-webview` / `ruviform-viz` のみ利用で `ruviform-media-reader` への依存・`PumpOutcome`/`pump()`/`Prefetch` の参照は一切なし。#244 で `unknown` だった分類を解消。

### Decisions During Implementation

- **eager emission（選択）vs 文書修正のみ**: contract を「検出時 `Ended` は非 terminal」と文書化する選択肢は manual driver に trap を残す。既存 rustdoc が既に "End marker was emitted" を約束していたため、outcome を marker emission に整合させる方が contract の自己矛盾を最小にする修正と判断。
- **emission を `emit_end` に集約**: EOF arm と step 2 が同一の commit/push/requeue ロジックを必要とし、commit-once invariant の authoritative 実装を 1 箇所に置く（DRY・同じ不変条件を所有する境界）。
- **fault arm は eager 化しない**: fatal-decode arm は `Fault` を `pending` back に積んで `stream_ended` を立てる。そこで `End` を eager push すると queued `Fault` を ring 上で追い越して `Fault` → `End` ordering が壊れる。step 2 lazy が ordering を保つ唯一正しい経路。
- **`Pushed` outcome mislabel（fault arm）は非修正**: #244 で記録済みの別 wart で、本 issue の原因ではないため scope 外として記録のみ。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64、Git Bash、mise 管理 rust 1.99.0。

| 検証 | 結果 |
| --- | --- |
| `cargo test -p ruviform-media-reader --test prefetch -- rd02_detection_ended rd02_end_committed`（TDD red） | fail（実装修正前に 2 件とも `End` strand / seek 吞み込みで失敗することを確認） |
| `cargo test -p ruviform-media-reader` | pass（prefetch 27 tests・reader 11 tests・rt_fill 3 tests — 新規 2 を含む全量 green） |
| `mise run fmt-check` | pass（`mise run fmt` で新規 test の整形差分を修正後） |
| `mise run check` | pass |
| `mise run lint` | pass（`-D warnings` clean） |
| `mise run test` | pass（workspace 全量 green・115 suite すべて ok・doctest 含む） |

## Risks

- **検出時 `Backpressured` への到達経路**: ring full と EOF 検出の同時成立は、marker が ring を埋めて pending が空・pool block が read に使える稀な構成でのみ発生（block item だけでは pool 制約上到達不能）。この経路は `emit_end` の shared ロジックと step-2 経路で実質 covered だが、専用の deterministic test は構成が複雑なため未追加 — 新 semantics の要所（detection `Ended` ⇒ marker out）は 2 テストで pin 済み。
- **`End{old}` の seek 前倒し観測**: `Ended` 後に seek すると旧 gen の `End` が new `Boundary` より先に consumer へ届く（旧 semantics では沉默 drop）。generation 付き marker で honest に処理されるが、旧「`End` は最終 item」と読んでいた仮想的 consumer は新順序を観測し得る。contract 的には正しい終端であり migration note に記録。
- **残る outcome-vocabulary wart**: fault arm の `Pushed` mislabel・`pump() -> Result` が error を返さない vestigial signature は #244 記録の pre-existing wart で、本 issue の原因でないため変更せず別途対応。

## Follow-ups

- audio-lens consumer impact は `no impact` に確定（`ruviform-media-reader` 非依存を local worktree で検証）— #244 の `unknown` を解消。
- synvelis 側で boundary-drain タイミング検証が必要になった場合は `driver.rs` の bounded pump loop と quiesce-drain→seek sequence を対象にする（本修正で hazard は縮小、追加対応は不要と評価）。
- outcome vocabulary の残 wart（fault arm `Pushed`・`pump() -> Result` vestigial）は別 issue 候補として継続記録。

## Commit

- `fix(issue-249): emit End at EOF detection so Ended cannot strand the marker`（branch `scarecrow173/rf249-ended-wart`、未 push）
