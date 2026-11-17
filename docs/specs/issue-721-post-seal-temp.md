---
id: "SPEC-034YXxVwj3nAIfWDF2rZcG"
type: "spec"
status: "approved"
title: "Issue #721 post-seal temp 改竄境界 — bench-smoke 配線照合と現行契約維持の設計判断"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/721"
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/designs/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/plans/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/tasks/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/test-specs/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/impl/ir/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/impl/exp/issue-721-post-seal-temp.md"
      - type: "file"
        path: "docs/impl/exp/issue-721-post-seal-temp-exp.jsonl"
      - type: "file"
        path: "evidence/rf721/"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
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
    - "docs/specs/issue-306-io-e2e-cases.md"
    - "docs/designs/issue-306-io-e2e-cases.md"
    - "docs/specs/issue-650-bench-smoke-coverage.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/721"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/650"
    - "https://github.com/scarecrow173/ruviform/issues/646"
  defers: []
  deferred-by: []
metadata:
  issue: 721
---

# Issue #721 post-seal temp 改竄境界 — bench-smoke 配線照合と現行契約維持の設計判断

## What Should Be Built

Issue #721 が引き受ける `scarecrow173/ruviform#306` close 時の 2 残件を確定する。

1. **残件 (a): bench-smoke lane 配線の現状照合（実測確認のみ）** —
   `mise run bench-smoke` が `ruviform-io-e2e` の `io_composition`
   bench（`staged_write`・`staged_seal`・`staged_publish`・
   `staged_readback`・`tar_gzip_staged`・`tar_zstd_staged`）を
   計測なし 1 回実行することを実測で確認し、
   `docs/development-environment.md` の bench 系記述が現行
   `mise.toml` と一致することを照合する。配線は merged PR
   `scarecrow173/ruviform#651`（commit `e69d92d0`）で main 着地済み
   の見込みであり、ドリフトが残っていた場合のみ相談経路に乗せる。
2. **残件 (b): post-seal temp 改竄の設計判断を設計文書で確定（主成果）** —
   `StagedFile::seal()` 後・`SealedFile::publish()` 前に staging
   temp が外部から改竄されても publish が内容を再検証しない現行契約
   について、「現行契約維持」または「検出/強化の実装」のいずれかを
   `docs/designs/issue-721-post-seal-temp.md` の決定記録として確定する。
   consumer（audio-lens `al-job`/`al-media`、synvelis
   `synvelis-library`/`synvelis-player`）の staged publish 利用実績を
   根拠として記録する。

## Why Now

`scarecrow173/ruviform#306` の verdict は「現行契約として正直に
pin 済み（回帰試験で固定）。再検討する場合は io-file 側の設計
Issue」と残件を記録して close したが、その判断本体を確定した
ADR/設計文書は存在しない。`scarecrow173/ruviform#721` がその
owner として判断を確定し、契約境界を将来の agent・consumer が
誤読しない形で文書化する必要がある。

## Users and Value

- ruviform の開発者・将来の agent: post-seal temp 改竄が seal 保証の
  外・呼び出し側信頼境界内であることが設計文書で一意に参照でき、
  「publish 時再検証」の要否を再議論する際の判断材料が揃う。
- consumer（audio-lens・synvelis）: 自らの staged publish 利用が
  要求する保証レベルと ruviform 契約の境界が一致していることを
  設計判断の根拠として確認できる。

## Scope

### In Scope

- `mise run bench-smoke` の実測実行と `ruviform-io-e2e`
  `io_composition` 各 case の実行確認、`docs/development-environment.md`
  bench 系記述との read-only 照合。
- post-seal temp 改竄境界の設計判断（維持/強化）を
  `docs/designs/issue-721-post-seal-temp.md` に確定し、根拠・
  検討した強化案・却下理由を記録。
- 採用契約を pin する既存 `post_seal_temp_*` 試験
  （`crates/ruviform-io-e2e/tests/staged_lifecycle.rs`）が現行契約を
  表す旨の明記。契約変更時のみ期待更新・新挙動 pin 試験を追加。
- `crates/ruviform-io-file/README.md` §信頼境界・保証範囲との整合
  確認と必要最小の doc 補強（実装変更なしの場合）。
- `evidence/rf721/` の検証記録と spec→design→plan→task→test-spec→
  impl の内部文書チェーン。

### Out of Scope

- 「強化」不採用の場合の `crates/` 実装変更全般。
- `docs/development-environment.md` の編集（`scarecrow173/ruviform#720`
  の並行 Dispatch が占有中 — ドリフト判明時は `orca orchestration ask`
  で相談）。
- `mise.toml` の bench-smoke 配線変更（main 着地済みの見込み。未配線が
  残っていた場合は `ask` で占有拡張を依頼）。
- `docs/**/README.md` index の編集・再生成（Coordinator が統合時に
  実施）。
- consumer リポジトリ（audio-lens・synvelis）の変更 — `StagedFile`
  利用実績は read-only 参照のみ。
- `Cargo.toml`/`Cargo.lock`・`.github/**`・`AGENTS.md`/`POLICIES.md`。
- `*-full`・`*-msrv`・3OS 横断 lane（日常の必須検証に含めない）。

## Requirements

- `mise run bench-smoke` が `ruviform-io-e2e` `io_composition` を
  計測なし 1 回実行することを実測で確認し、`docs/development-environment.md`
  の記述と一致すること（未配線が残っていた場合は配線してから確認）。
- post-seal temp 改竄について「現行契約維持」または「検出/強化の
  実装」のいずれかの設計判断が設計文書で確定し、採用した契約を pin
  する回帰試験が存在する（維持なら既存 `post_seal_temp_*` テストが
  現行契約を表す旨を明記、変更なら新挙動を固定するテストを追加）。
- 変更が io-file/io-e2e の既存 suite に回帰を起こさない
  （`mise run test ruviform-io-file` / `mise run test-io-e2e` pass）。
- headless/fixture lane を本 Issue の Pass 判定とし、実 FS fault の
  OS 別挙動差が残る場合は環境別に NotRun/Unsupported を分離記録する
  （headless Pass を実 OS 受入に昇格しない）。

## Acceptance Criteria

- [x] `mise run bench-smoke` が `ruviform-io-e2e`（`io_composition`）を
      含む bench を計測なし 1 回実行することを実測で確認し、
      `docs/development-environment.md` の記述と一致している
      （`evidence/rf721/`）。
- [x] post-seal temp 改竄の設計判断が設計文書で確定し、採用契約を
      pin する回帰試験が存在する（`docs/designs/issue-721-post-seal-temp.md`
      + `post_seal_temp_*` 既存試験が pin）。
- [x] `mise run test ruviform-io-file` / `mise run test-io-e2e` が
      pass し既存 suite に回帰がない。
- [x] headless/fixture lane の Pass と実 FS fault の OS 別挙動差の
      NotRun/Unsupported を分離記録している
      （`evidence/rf721/verification.md`）。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- publish 時の内容再検証（seal 時ハッシュを publish 直前に再計算）|
  consumer が要求する保証レベルを超える防御であり、同一信頼境界内の
  攻撃者には TOCTOU が残るため実効境界を生まない | consumer が
  「seal 後〜publish 前の temp 改竄」を脅威モデルに含める要件を
  出した場合、または publish コストに対し完全性保証が見合う
  ユースケースが現れた場合 | 境界が曖昧なまま「seal = publish までの
  完全性保証」と誤読されるリスク — 本設計文書と README 補強で
  明示して緩和。

## Sources and Relations

- Issue: `scarecrow173/ruviform#721`（本 slice）、先行 `#306`
  （COVERED・pin 済み verdict）、`#650`/PR #651（bench-smoke 配線
  landed）、`#646`（coverage gap の発覚元）。
- 設計根拠: `docs/designs/issue-306-io-e2e-cases.md` Case 1/Case 2
  （現行動作 pin・「将来の再検証実装時に契約見直しを強制」の明記）、
  `crates/ruviform-io-file/README.md` §信頼境界・保証範囲、
  `crates/ruviform-io-file/src/staged.rs`（`seal`/`publish` doc）、
  `crates/ruviform-io-e2e/tests/staged_lifecycle.rs`
  （`post_seal_temp_rewrite_is_not_reverified_before_publish`・
  `post_seal_temp_delete_fails_publish_not_published`）。
- consumer 根拠: audio-lens `crates/al-job/src/fsutil.rs`
  （`stage_file`/`publish_sealed`）・`crates/al-media/src/wav.rs`、
  synvelis `crates/synvelis-library/src/import.rs`・`cache.rs`・
  `preview.rs`・`crates/synvelis-player` — いずれも同一信頼境界内の
  staged publish 利用。
- 関連 Issue: `scarecrow173/audio-lens#127`・`scarecrow173/synvelis#28`・
  `scarecrow173/synvelis#84`（CLOSED）・`scarecrow173/synvelis#128`・
  `scarecrow173/synvelis#17`・`scarecrow173/synvelis#91`（強化選択時の
  要件根拠・受入側）、`scarecrow173/ruviform#720`（並行 Dispatch・
  `docs/development-environment.md` 占有）。
- 実測証拠: `evidence/rf721/`。
