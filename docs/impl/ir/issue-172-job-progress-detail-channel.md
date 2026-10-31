---
id: "IMPL-034PrGKUBVdoCZyyNagkd2"
type: "impl"
status: "completed"
title: "Issue #172 job progress detail channel"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/172"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/service.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/ipc.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/runtime.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/demo.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/runtime.rs"
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "docs/designs/issue-156-gui-host-foundation.md"
      - type: "file"
        path: "docs/impl/ir/issue-172-job-progress-detail-channel.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-156-gui-host-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-171-data-registry-eviction-contract.md"
    - "docs/impl/ir/issue-156-gui-foundation-verification.md"
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

# Issue #172 job progress detail channel

## Summary

audio-lens #105 GUI 実装の起票案。`JobContext::progress(done,total)`
は件数のみで「どの stage/処理中か」を運べず、stage 名表示に別 op
が必要だった。進捗 report に optional の `detail` 文字列を添える
経路を `ruviform-ui` に追加した:

- `JobContext::progress_detail(done,total,detail)` を新設
  (既存 `progress(done,total)` は detail なし report として
  従来どおり動作)
- wire 上は `job`/`state:"progress"` Event に optional の
  `detail` フィールドを additive 追加(detail を持たない
  report ではキー自体を emit しない。`rvf.ipc.v1` の既存
  consumer は不変)
- latest-wins 合体は counters と同じ slot を detail も辿り、
  最新 report の detail が残る

## Context

`JobContext::progress` → `JobSignal::Progress` → `UiRuntime`
mailbox drain → `Event::job_progress` → `{"kind":"job",
"state":"progress","done":D,"total":T}` の伝搬経路は Issue #156
の GUI host foundation で確立済み。Issue #172 はこの経路に
「label 相当の detail 文字列」を載せる API を求める
audio-lens #105 起票由来の実装依頼。target は
`crates/ruviform-ui/` の job/progress 層。

## Implementation

### Changes

- `crates/ruviform-ui/src/service.rs`:
  - `JobContext::progress_detail(done,total,detail:
    impl Into<String>)` を追加。detail は report 単位の添付で
    後続 report へ持ち越されない契約を doc に明記(stage 名の
    永続表示は「最新の detail 付き event を保持する」consumer
    側の責務)。
  - 既存 `progress(done,total)` は内部共通 `report()` に
    `detail: None` で委譲 — 署名・挙動とも不変。
  - `JobSignal::Progress` に `detail: Option<String>` を追加
    (crate 内部 enum)。
- `crates/ruviform-ui/src/ipc.rs`:
  - `Event::Job` に `detail: Option<String>` を追加
    (`skip_serializing_if` — `None` ではキーを emit しない
    additive 拡張)。
  - `Event::job_progress_detail(job,done,total,detail:
    Option<impl Into<String>>)` を新設。既存
    `job_progress(job,done,total)` は `detail: None` 委譲で
    署名不変。`job_finished`/`job_failed`/`job_cancelled` は
    `detail: None`。
- `crates/ruviform-ui/src/runtime.rs`: drain 内の
  latest-progress 合体テーブルが `(done,total)` から
  `(done,total,detail)` を保持するよう拡張し、emit は
  `job_progress_detail` 経由に変更。合体規則は不変(最新
  report が detail ごと勝つ)。
- `crates/ruviform-ui/src/demo.rs`: `demo.sleep` が
  `"sleeping"`、`demo.synthesize` が `"peaks"` の detail を
  report へ添付するよう更新 — GUI/CLI 共通デモで detail 経路を
  実演する。
- `crates/ruviform-ui/tests/runtime.rs`: `StageService`
  (detail 付き `test.stages` / detail なし `test.plain`)を追加し
  `job_progress_detail_reaches_the_wire` で伝播・wire 形状・
  互換を検証。
- `crates/ruviform-ui/README.md`: IPC v1 bullet に detail 契約
  の子項目を追加。
- `docs/designs/issue-156-gui-host-foundation.md`: wire 規約
  リストに `detail` optional フィールドの規定を追加。

### Decisions During Implementation

- **detail は per-report(runtime/producer いずれの sticky
  状態も持たない)** — 「report 毎に任意で添える」が issue の
  "label 相当の detail 文字列を進捗に添える" の最も単純な
  対応(KISS)。sticky にすると「detail を持たない report が
  続く場合の意味(維持か消去か)」と消去 API が新たに必要に
  なり、内部状態(JobContext 内の mutex または JobRecord 側の
  保持)が増える。stage 名は report の度に同じ文字列を渡せば
  済み、合体で最新が残るため実用上の差はない。永続表示は
  consumer が最後の detail 付き event を保持する契約として
  doc に明記した。
- **wire フィールド名は `detail`** — `JobSpec.label`(job 全体の
  静的ラベル)と区別するため。issue の "detail/label" のうち
  detail 側を採用。
- **`Event::Job` へ additive フィールド追加・スキーマは v1 の
  まま** — `detail` は serialize 時のみ存在する optional
  outbound フィールドで、既存 consumer(未知キー無視)には
  無影響。Intent 方向の parse 契約は不変のため
  `SCHEMA_VERSION` は据え置き。
- **`ruviform-exec` 側の `Progress`/`JobEvent` には触らない** —
  issue の対象は `JobContext`(ruviform-ui)であり、exec 側
  `Progress` は counters 専用の共有 atomic プリミティブ
  (`JobEvent` は `Copy`)。detail を入れるとカウンタ更新毎の
  共有文字列状態が要り層の境界を侵す。stage 名は report を
  発行する job body が既に知っている情報であり、UI 層で添付
  するのが正しい境界(Follow-ups に CLI 側同等需要を記録)。
- **合体 slot に detail を同梱** — `latest` map が counters と
  detail を一緒に保持するため「最新 report の detail が残る」が
  自然に成立し、drain バッチ跨ぎで別管理する必要がない。

## Related Experiments

### Adopted

なし — 設計・実装とも初版で確定。

### Rejected

- sticky detail(producer 側 `set_detail` または runtime 側
  JobRecord 保持) — 上記 Decisions。消去意味論と内部状態を
  要し、issue の要求に対し過剰。

## Validation

ゲート(全て実実行・exit 0):

| コマンド | 結果 |
| --- | --- |
| `mise run fmt` | pass |
| `mise run check` | pass(`cargo check --workspace --all-targets`) |
| `mise run lint` | pass(clippy `-D warnings` clean) |
| `mise run test` | pass(`cargo test --workspace` 全緑) |
| `mise run build` | pass |
| `mise run check-ui-webview` | pass |
| `mise run lint-ui-webview` | pass(`-D warnings`) |
| `mise run test-ui-webview` | pass(scheme 7 tests) |

detail 契約テスト(`crates/ruviform-ui/tests/runtime.rs`):

- `job_progress_detail_reaches_the_wire` —
  `test.stages`(最終 report に detail 添付)の drain 結果に
  `Event::Job { state: Progress, detail: Some("peaks") }` が
  含まれ、terminal event より前に到達すること。
  `test.plain`(detail なし report のみ)では progress event の
  `detail` が `None` — 既存 `progress(done,total)` 呼出の
  動作維持。wire 形状は `job_progress_detail` で `"detail"`
  キーが emit され、`job_progress` ではキー自体が出ない
  ことを検証。

## Risks

- `Event::Job` variant へ `detail` フィールドを追加。wire 上は
  additive だが、`Event::Job { .. }` を struct literal で構築する
  外部コードは field 追加でコンパイルエラーになる
  (`#[non_exhaustive]` enum の variant はパターンの `..` で
  網羅できる一方、構築には全 field 指定が要る)。本 workspace
  内の構築は `Event::job_*` constructor 経由のみで影響なし。
  AL 側は event を deserialize/参照するだけなので無影響。
- detail は per-report のため「前の detail を維持したい」
  consumer は最後の detail 付き event を保持する必要がある —
  README/rustdoc/design doc に契約として明記済み。

## Follow-ups

- audio-lens #105 側で view op が `detail` を表示する際は、
  「最新の detail 付き progress event を terminal event まで
  保持」する consumer 側規約を AL 側文書へ転記する。
- `ruviform-exec::OfflineJob`/`JobEvent`(CLI 側 job 経路)にも
  同等の detail 経路の需要が出た場合は別 issue で検討 —
  `JobEvent` は `Copy` enum のため設計が別途要る。
