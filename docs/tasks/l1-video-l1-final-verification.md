---
id: "TASK-034NXuAACPRKeBy1TSW1EO"
type: "task"
status: "todo"
title: "L1 video/NONRT final verification（Full Gate・AC/R-V 証拠・IR・status 同期・closeout）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-118-video-l1-implementation.md"
    modified:
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-118-video-l1.md"
      - type: "file"
        path: "docs/tasks/l1-wire-v3-video-codec.md"
      - type: "file"
        path: "docs/tasks/l1-video-runner-and-erased-surface.md"
      - type: "file"
        path: "docs/tasks/l1-video-host-surface.md"
      - type: "file"
        path: "docs/tasks/l1-video-fixture-and-e2e.md"
      - type: "file"
        path: "docs/tasks/l1-video-docs-sweep.md"
      - type: "file"
        path: "docs/specs/issue-118-video-l1.md"
      - type: "file"
        path: "docs/designs/issue-118-video-l1.md"
      - type: "file"
        path: "docs/adr/issue-118-video-l1-host-surface.md"
      - type: "file"
        path: "docs/adr/issue-118-video-l1-create-path-segment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-118-video-l1.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/tasks/l1-video-fixture-and-e2e.md"
    - "docs/tasks/l1-video-docs-sweep.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-video-runner-and-erased-surface.md"
    - "docs/tasks/l1-video-host-surface.md"
    - "docs/tasks/l1-wire-v3-video-codec.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-118-wire-v3-codec-tests.md"
    - "docs/test-specs/issue-118-video-failed-parity-tests.md"
    - "docs/test-specs/issue-118-video-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/117"
  defers: []
  deferred-by: []
---

# L1 video/NONRT final verification（Full Gate・AC/R-V 証拠・IR・status 同期・closeout）

## Work

全 task 着地後の final gate — Full `mise` 検証列 + 3 test-spec + IR 作成 + AC/R-V 証拠マトリクスの棚卸し + status 同期 + closeout。コード変更はなく、doc/status/IR/証拠のみ。`ir-001` の「実行結果 → evidence 表 → AC/R-V 照合 → status 同期 → closeout」の構造を踏襲する。

1. `docs/impl/ir/issue-118-video-l1-implementation.md`（新規 — `ir-001` と同じ IR スキーマ・file 名は `issue-118-video-l1-implementation.md`）: 実施要約（`video()` 借用 façade・`Route::OutOfProcessVideo`/`Slot::OutOfProcessVideo`・`descriptors` kind-tag・`InstKind` 分離・`Creating` ゲート拡張・`request_timed`・`vshm`/`l1/vshm`/`export/vshm`・`run_video`/`VideoEntry`/stash replay・`erase_video`/`ErasedVideoPlugin`/`VideoPluginFactory`・`l1-test-video-worker`/`l1_video_e2e`/`l1_video_runner_e2e`/`ruviform-plugin-l1-video-runner`）・**変更ファイル map（T1〜T4 全 file の行数差分・新規/変更/削除分類 — `ir-001` の changed-file 表と同じ粒度）**・**検証証拠（`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・`cargo test -p ruviform-plugin-core`/`export`/`host` の suite 別結果・`cargo doc` warning 0・`test-l1-real-plugin`/`lint-l1-real-plugin` は audio CLAP/VST3 lane のみで video lane は v1 対象外と明記）**・**既知差分**（audio `Err` reply で mirror 不変 — `ir-001` の `Err` reply 収束差分と同じ書式・`VideoOutOfProcessSpec` の `isolation`/`lookahead_blocks` 非存在 pin・`descriptor(handle)` の v1 提供有無・real video adapter lane 欠如・`BlockIn` inert・`Err` reply mirror 不変の L0-faithful 契約）・AC/R-V 証拠マトリクス（design `## Requirements` 表の AC1-AC9 + R-B1-B8 + R-F1-F7 + R-V1-V7 全行を検証 task/test/証拠に 1:1 対応 — #117 IR のマトリクス形式）
2. `docs/impl/ir/README.md` — IR index 行を追加（`ID | Title | Status | File` — `ir-001` の行と同じ schema）
3. `docs/plans/implement-issue-118-video-l1.md` — status を `completed` へ（全 task done + IR 着地後）・`relations.implemented-by` に `docs/impl/ir/issue-118-video-l1-implementation.md` を追加・`relations.changes.added`/`modified` に T1〜T4 の全 file を確定値で記入（`Cargo.toml` 変更・新規 bin/test target・`video_plugin.rs`/`vshm.rs`/`l1/video.rs`/`l1/vshm.rs`/`video.rs` 等 — plan 作成時は予定リストだったものを着地値で確定）
4. 6 task doc — status を `done` へ同期・`updated` を着地日に揃える・`relations.implemented-by` に `docs/impl/ir/issue-118-video-l1-implementation.md` を追加（IR が task の実施記録を担う — `ir-001` の task→IR 参照パターン）。plan の `blocks`/`depends-on` チェーンが DAG と一致していることの最終確認
5. `docs/specs/issue-118-video-l1.md`・`docs/designs/issue-118-video-l1.md`・両 ADR — `relations.implemented-by` に `docs/impl/ir/issue-118-video-l1-implementation.md` と 6 task doc を追加（spec/design/ADR → IR/task の双方向 link — `doc_status.js` が `derived-by`/`implemented-by` の reciprocal を検査するため、spec/design/ADR 側も更新が必要）。status は既に approved/in-progress のまま（IR が landed を示すので spec status は変更しない — #117 の spec→IR パターン）
6. `docs/test-specs/issue-118-*.md` 3 件 — `relations.verified-by` に IR を追加（test-spec → IR の reciprocal — `doc_status.js` が `verified-by`/`verifies` の対称性を検査するため）。status は `approved` のまま（test-spec は oracle であり実施状態を持たない）
7. AC/R-V 証拠マトリクスの最終棚卸し — IR の表に対し、`cargo test` 出力・`mise run` 結果・`cargo doc` warning 0・`test-l1-real-plugin` lane 欠如明記の全てを「実行した検証」と「未実行/対象外の検証」に分けて記述（未実行を「実行した」と書かない — AGENTS の「実際に実行して成功していない検証を成功したと報告しない」規則に従う）。AC9 の real video adapter lane は「v1 対象外 — `mise run test-l1-real-plugin`/`lint-l1-real-plugin` は CLAP/VST3 audio のみ」と明記
8. closeout — `git status`/`git diff --check` で doc-only 変更（Rust ソース無変更）を確認・コミットは `Refs #118` + Devin trailer で行う（plan の commit 手順どおり — `git push` は行わない）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T6 — 全 task done + doc sweep 後の final gate）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md) `## Requirements` 表（AC1-AC9 + R-B1-B8 + R-F1-F7 + R-V1-V7 — IR マトリクスの全行）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md)（AC/R-V 行の正本）
- [issue-117-bundle-child-implementation.md](../impl/ir/issue-117-bundle-child-implementation.md)（IR スキーマ・差分注記・AC/R-V マトリクス形式の source）

## Dependencies

- 先行タスク: `l1-video-fixture-and-e2e`（T4 — fixture/E2E 着地）+ `l1-video-docs-sweep`（T5 — README/親文書/test-spec reconcile 着地）の両方
- T1-T4 の全 code surface が確定し T5 の doc 整合が通った後でないと IR の証拠表が書けない

## Affected Files

- `docs/impl/ir/issue-118-video-l1-implementation.md`（新規）— IR 本体（実施要約・変更 file map・検証証拠・既知差分・AC/R-V マトリクス）
- `docs/impl/ir/README.md` — IR index 行追加
- `docs/plans/implement-issue-118-video-l1.md` — status `completed`・`implemented-by`/`changes` 確定
- `docs/tasks/l1-wire-v3-video-codec.md`・`l1-video-runner-and-erased-surface.md`・`l1-video-host-surface.md`・`l1-video-fixture-and-e2e.md`・`l1-video-docs-sweep.md` — status `done`・`implemented-by` に IR 追加
- `docs/specs/issue-118-video-l1.md`・`docs/designs/issue-118-video-l1.md`・`docs/adr/issue-118-video-l1-host-surface.md`・`docs/adr/issue-118-video-l1-create-path-segment.md` — `implemented-by` に IR + task doc 追加
- `docs/test-specs/issue-118-wire-v3-codec-tests.md`・`issue-118-video-failed-parity-tests.md`・`issue-118-video-e2e-tests.md` — `verified-by` に IR 追加

## Done When

- [ ] `docs/impl/ir/issue-118-video-l1-implementation.md` が `ir-001` スキーマで存在し、AC1-AC9 + R-B1-B8 + R-F1-F7 + R-V1-V7 の全行が検証 task/test/証拠に対応している
- [ ] `docs/impl/ir/README.md` に IR 行が追加され index と一致している
- [ ] plan status が `completed`、6 task status が `done`、全 `implemented-by`/`verified-by` reciprocal が通っている
- [ ] IR の「実行した検証」と「未実行/対象外」の区別が明示され、AC9 real video adapter lane が v1 対象外と明記されている
- [ ] `doc_status.js audit --json` が `blocking: 0`・`findings: []`（IR + status 同期後の最終 audit）
- [ ] `git status`/`git diff --check` が clean で、Rust ソース変更がない

## Verification

- [ ] `mise run fmt-check` — pass
- [ ] `mise run check` — pass（`--workspace --all-targets`）
- [ ] `mise run lint` — pass（`-D warnings`）
- [ ] `mise run test` — pass（`--workspace` 全 suite — `l1_video_e2e`/`l1_video_runner_e2e`/`l1_proxy_e2e`/`l1_runner_e2e` 含む）
- [ ] `mise run test-rt` — pass（`rt_` フィルタ — video は NONRT のため audio RT suite が無退行であることの確認）
- [ ] `mise run bench-smoke` — pass（`--test` 1 回実行 — `ruviform-math`/`ruviform-rt` ベンチの smoke lane が video 変更で壊れていないことの確認）
- [ ] `cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps` — warning 0
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` — `blocking: 0`・`findings: []`（最終 audit — IR 着地後）
- [ ] `prek run --all-files`（または `mise exec -- prek run --all-files`）— pass（README/docs の hook を含む）
- [ ] `mise run test-l1-real-plugin` / `lint-l1-real-plugin` — video lane は v1 対象外（audio CLAP/VST3 lane のみ — opt-in のため実行は任意。IR に「対象外」と明記）
