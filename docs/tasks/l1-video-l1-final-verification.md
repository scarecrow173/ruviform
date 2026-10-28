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
        path: "docs/specs/issue-118-video-l1.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-fmt-check-*.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-check-*.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-lint-*.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-test-*.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-test-rt-*.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-bench-smoke-*.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-cargo-doc-*.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-prek-*.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-118-final-gate-doc-audit-*.log"
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

全 task 着地後の final gate — Full `mise` 検証列 + IR 作成 + AC/R-V 証拠マトリクスの棚卸し + status 同期 + closeout。コード変更はなく、doc/status/IR/証拠のみ。`docs/impl/ir/issue-117-bundle-child-implementation.md`（直近の final-verification IR — 本 task の exemplar）の「実行結果 → evidence 表 → AC/R-V 照合 → status 同期 → closeout」の構造を踏襲する。relation/status の同期規約も #117 closeout の先例に従う（詳細は item 3-6 — `doc_status.js` の reciprocal 規則と整合する edge のみを張る）。

1. `docs/impl/ir/issue-118-video-l1-implementation.md`（新規 — #117 IR と同じ IR スキーマ）: 実施要約（`video()` 借用 façade・`Route::OutOfProcessVideo`/`Slot::OutOfProcessVideo`・`descriptors` kind-tag・`InstKind` 分離・`Creating` ゲート拡張・`request_timed`・`vshm`/`l1/vshm`/`export/vshm`・`run_video`/`VideoEntry`/stash replay・`erase_video`/`ErasedVideoPlugin`/`VideoPluginFactory`・`l1-test-video-worker`/`l1_video_e2e`/`l1_video_runner_e2e`/`ruviform-plugin-l1-video-runner`）・**変更ファイル map（T1〜T4 全 file の行数差分・新規/変更/削除分類 — #117 IR の changed-file 表と同じ粒度）**・**検証証拠（`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・`cargo test -p ruviform-plugin-core`/`export`/`host` の suite 別結果・`cargo doc` warning 0・`test-l1-real-plugin`/`lint-l1-real-plugin` は audio CLAP/VST3 lane のみで video lane は v1 対象外と明記）**・**既知差分**（audio `Err` reply で mirror 不変 — #117 IR の差分記述と同じ書式・`VideoOutOfProcessSpec` の `isolation`/`lookahead_blocks` 非存在 pin・`descriptor(handle)` は **v1 未提供**（pinned — handle→descriptor lookup API は公開面に生えない・`video()` façade doc に明記）・real video adapter lane 欠如・`BlockIn` inert・`Err` reply mirror 不変の L0-faithful 契約）・AC/R-V 証拠マトリクス（design `## Requirements` 表の AC1-AC9 + R-B1-B8 + R-F1-F6 + R-V1-V6 全行を検証 task/test/証拠に 1:1 対応 — #117 IR のマトリクス形式）。**front-matter edge は #117 IR convention**: `implements` → 本 task（`l1-video-l1-final-verification`）のみ・`depends-on` → plan+spec・`verifies: []`（impl の `verifies` は型 convention で空欄）・`verified-by` → `issue-118-*` test-spec 3 件 + T5 で reconcile した baseline test-spec 群
2. `docs/impl/ir/README.md` — IR index 行を追加（`ID | Title | Status | File` — #117 IR の行と同じ schema）
3. `docs/plans/implement-issue-118-video-l1.md` — status を `completed` へ（全 task done + IR 着地後 — front-matter `status` と body の `> status:` 行の両方を完了形へ更新）・`relations.implemented-by` は 6 task のまま（**IR は追加しない** — #117 plan convention: plan の `implemented-by` は task 集合のまま・IR→plan の edge は IR 側の `depends-on`/`related` が担う）・`relations.changes.added`/`modified` を着地値で確定（T1〜T4 の全 file — `Cargo.toml` 変更・新規 bin/test target・`video_plugin.rs`/`vshm.rs`/`l1/video.rs`/`l1/vshm.rs`/`video.rs` 等の予定リストを実績へ）
4. task status 同期 — **per-task flip convention（#117 先例）**: T1〜T5 の task doc は各 task closeout で status `done`・`updated` 同期済みのはずで、T6 では全 6 task の status と `docs/tasks/README.md` index 行の一致を**検査**する。未同期が残っていた場合は index 行・task doc を修正して IR に incident として記録する（#117 で T4 の index 行のみ未同期だった先例 — 想定される差分はない）。**本 task（T6）の doc のみ** status `done`・`updated` 着地日・`implemented-by` に `docs/impl/ir/issue-118-video-l1-implementation.md` を追加（`implemented-by` → IR は T6 task のみ — T1〜T5 task doc は `implemented-by: []` 維持・#117 IR の「plan `implemented-by` → 6 task・task `implements` → plan・T6 task `implemented-by` → 本 IR — 相互 link 完備」のまま）。plan の `blocks`/`depends-on` チェーンが DAG と一致していることの最終確認
5. `docs/specs/issue-118-video-l1.md` — status を `approved → implemented` へ（#117 closeout convention — spec は実装完了時に `implemented` へ flip・AC checkbox は先例どおり未チェックのまま証跡を IR coverage 表へ集約）。`implemented-by` は `[design, plan]` のまま・`verified-by` は test-spec のまま（**IR・task への edge は追加しない** — spec/design/ADR → IR/task の `implemented-by` edge は `inconsistent-reciprocal-relation` と衝突する: IR の `implements` は T6 task のみ・task の `implements` は plan のみで、upstream doc を IR/task が「実装した」という逆向き edge は reciprocal を満たせない）。`docs/specs/README.md` の status 列を `implemented` へ同期。**`docs/designs/issue-118-video-l1.md` と両 ADR は無変更**（status `approved`/`accepted` 維持・`implemented-by: []` 維持 — #117 で design/ADR が closeout で無変更だった先例。upstream traceability は既存の `derived-by`/`implemented-by` edge で完備済み）
6. `docs/test-specs/issue-118-*.md` 3 件 — **file 自体は変更しない**。`verified-by` に IR は追加しない（#117 IR 明記の convention — 「IR `verifies` は空・`verified-by` は test-spec 群 — spec `verified-by` に IR は追加しない。追加すると `inconsistent-reciprocal-relation` が IR `verifies` 空欄と衝突する」）。status は `approved` のまま（test-spec は oracle であり実施状態を持たない — #117 では `proposed→approved` だったが #118 3 件は plan 作成時点で approved 済み）。検証側の表現は IR 側の `verified-by` 列挙で済む（一方向 edge — #117 IR が新規+reconcile 対象の test-spec を列挙した先例）
7. AC/R-V 証拠マトリクスの最終棚卸し — IR の表に対し、`cargo test` 出力・`mise run` 結果・`cargo doc` warning 0・`test-l1-real-plugin` lane 欠如明記の全てを「実行した検証」と「未実行/対象外の検証」に分けて記述（未実行を「実行した」と書かない — AGENTS の「実際に実行して成功していない検証を成功したと報告しない」規則に従う）。AC9 の real video adapter lane は「v1 対象外 — `mise run test-l1-real-plugin`/`lint-l1-real-plugin` は CLAP/VST3 audio のみ」と明記。各 gate の出力は `docs/impl/exp/data/issue-118-final-gate-*.log` として保存し IR の証拠節から link する（#117 の `generated` 証跡 convention）
8. closeout — `git status`/`git diff --check` で doc-only 変更（Rust ソース無変更）を確認・コミットは `Refs #118` + Devin trailer で行う（plan の commit 手順どおり — `git push` は行わない）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T6 — 全 task done + doc sweep 後の final gate）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md) `## Requirements` 表（AC1-AC9 + R-B1-B8 + R-F1-F6 + R-V1-V6 — IR マトリクスの全行）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md)（AC/R-V 行の正本）
- [issue-117-bundle-child-implementation.md](../impl/ir/issue-117-bundle-child-implementation.md)（IR スキーマ・差分注記・AC/R-V マトリクス形式の source）

## Dependencies

- 先行タスク: `l1-video-fixture-and-e2e`（T4 — fixture/E2E 着地）+ `l1-video-docs-sweep`（T5 — README/親文書/test-spec reconcile 着地）の両方
- T1-T4 の全 code surface が確定し T5 の doc 整合が通った後でないと IR の証拠表が書けない

## Affected Files

- `docs/impl/ir/issue-118-video-l1-implementation.md`（新規）— IR 本体（実施要約・変更 file map・検証証拠・既知差分・AC/R-V マトリクス）
- `docs/impl/ir/README.md` — IR index 行追加
- `docs/plans/implement-issue-118-video-l1.md` — status `completed`（front-matter + body `> status:` 行）・`changes` 着地値確定（`implemented-by` は 6 task のまま）
- `docs/specs/issue-118-video-l1.md` — status `approved → implemented`（relation edge は既存のまま維持）
- `docs/tasks/README.md` — index 行同期（T6 自身の `done` + 未同期 residual の修正）
- `docs/plans/README.md` — plan status 列 `completed` へ
- `docs/specs/README.md` — spec status 列 `implemented` へ
- `docs/impl/exp/data/issue-118-final-gate-*.log`（generated）— gate 出力証跡（#117 convention）
- 無変更（#117 closeout convention — 本 task の file map に含めない）: `docs/designs/issue-118-video-l1.md`・両 ADR・`docs/test-specs/issue-118-*.md` 3 件・T1〜T5 task doc（各 closeout で status 同期済み・`implemented-by: []` 維持）

## Done When

- [ ] `docs/impl/ir/issue-118-video-l1-implementation.md` が #117 IR スキーマで存在し、AC1-AC9 + R-B1-B8 + R-F1-F6 + R-V1-V6 の全行が検証 task/test/証拠に対応している
- [ ] `docs/impl/ir/README.md` に IR 行が追加され index と一致している
- [ ] plan status が `completed`、spec status が `implemented`、6 task status が `done` で index 行と一致し、edge convention が #117 closeout の先例どおり（IR の `implements` → T6 task・T6 task の `implemented-by` → IR・spec/design/ADR/test-spec の relation は無変更）で `doc_status.js` reciprocal が通っている
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
