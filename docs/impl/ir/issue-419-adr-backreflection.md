---
id: "IMPL-034XPQFMlLz1ivXaIBwQjs"
type: "impl"
status: "completed"
title: "Issue #419 後継ADR verdict逆反映 docs-only slice 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/419"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/impl/exp/issue-419-adr-backreflection.md"
    modified:
      - type: "doc"
        path: "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
      - type: "doc"
        path: "docs/specs/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/designs/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/test-specs/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/plans/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/tasks/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/tasks/issue-419-adr-backreflection.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-419-adr-backreflection.md"
    - "docs/designs/issue-419-adr-backreflection.md"
    - "docs/test-specs/issue-419-adr-backreflection.md"
    - "docs/plans/issue-419-adr-backreflection.md"
    - "docs/tasks/issue-419-adr-backreflection.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/impl/exp/issue-419-adr-backreflection.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-419-adr-backreflection.md"
    rejected: []
---

# Issue #419 後継ADR verdict逆反映 docs-only slice 実装記録

## Summary

Issue #419 の「Coordinator 決裁済み verdict（outcome = CONDITIONAL）を後継 ADR へ逆反映する」docs-only slice として、対象 ADR（`docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`）の More Information 逆反映欄・Confirmation・frontmatter を更新し、slug `issue-419-adr-backreflection` の文書チェーン 7 ファイルを作成し、触れた全 dir の管理 index を再生成した。実装コード・crate・CI workflow・依存変更・ADR status 変更・Issue 投稿・`evidence/` 編集は含まない。

## Context

- Task: `docs/tasks/issue-419-adr-backreflection.md`
- Plan: `docs/plans/issue-419-adr-backreflection.md`
- Spec: `docs/specs/issue-419-adr-backreflection.md`
- Design: `docs/designs/issue-419-adr-backreflection.md`（approved）
- Test-spec: `docs/test-specs/issue-419-adr-backreflection.md`
- Issue: https://github.com/scarecrow173/ruviform/issues/419

## Implementation

### Changes

- `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md` - `## More Information` 末尾の「GPUI 最終採用判定の逆反映欄（未記入）」を 2026-11-15 dated 記録へ置換（outcome = CONDITIONAL・Verdict 規則該当・「L1 のみ評価済み、L2/L3 非保証」・標準 Driver 方針維持・Native Dock 無条件採用不可・OS 別 Capability 宣言記録・既知制約/将来 Owner・`../../evidence/rf262-gpui-verdict/` 相対リンク）。`### Confirmation` に逆反映実施の dated 記録を追加（既存の無条件採用を宣言しない制約は維持）。frontmatter `updated` を `2026-11-15` へ更新、`references` に `#419`/`#532` を追加。`status: "proposed"`・`id`・`created`・既存 `relations`・Decision Outcome・判定フロー・縮退順序・責務マップ・Capability Matrix・Verification は不変。
- `docs/specs/issue-419-adr-backreflection.md` - 問題・Audience・Scope/Non-goals・受入条件・Evidence・Deferred を定義（SPEC-034XPQgOgKyGN5Ydw7c93D）。
- `docs/designs/issue-419-adr-backreflection.md` - 逆反映の記録構造（D1）・判定語彙/誠実性規則（D2）・文書チェーン（D3）を定義（DESIGN-034XPQhsUWcCgjNEs31H4d）。
- `docs/test-specs/issue-419-adr-backreflection.md` - 逆反映記述の不変条件を Rule（R-RECORD/R-VOCAB/R-SCOPE/R-HONESTY）と Example（EX-1〜4）へ固定（TSPEC-034XPQypgsINSAsr2QivQO）。
- `docs/plans/issue-419-adr-backreflection.md` - docs-only slice の実装計画（PLAN-034XPQz5nuJOYW06YM1ygQ）。
- `docs/tasks/issue-419-adr-backreflection.md` - 文書チェーン完成・検証タスク（TASK-034XPQF7BrLnw2ekZ4Q3r0）。
- `docs/impl/exp/issue-419-adr-backreflection.md` - verdict 転記・語彙適用・文書規約適用の試行記録（`.md` 自由形式、issue-262 先例）。
- `docs/adr/README.md`・`docs/{specs,designs,test-specs,plans,tasks,impl/ir,impl/exp}/README.md` - `doc-maintenance index apply`/`sync_impl_index.js`/`docs/adr` index 手順で管理 index を再生成。`docs/impl/exp/README.md` の hand-curated「Reports and raw data」節に `.md` exp 記録の 1 行を追加（生成領域外の手書き領域。issue-262 先例どおり）。

### Decisions During Implementation

- **verdict の決裁分離**: 本 slice は「Coordinator 決裁済みの outcome = CONDITIONAL を ADR に記録した」ことのみを記述し、採否の再判定・正式決裁宣言・ADR status 変更・Issue 投稿は行わない（Coordinator / design authority 権限）。spec/design/test-spec/IR で一貫して明記。
- **「推奨」と「決裁」の分離**: `handoff-419.md`/`verdict-matrix.md` は verdict を「推奨」と記すが、outcome（CONDITIONAL）は Coordinator 決裁済みのものとして転記。「推奨」を「決裁」に読み替える記述は置かない。
- **判定語彙の分離**: OS-L1 Pass・Windows L2 部分実証・Windows L3/Gate D 未達・macOS/Linux L2/L3 Deferred・Gate C 性能/Gate E NotRun を verdict-matrix の語彙どおり記録し、`NotRun`/`Deferred`/`Unsupported`/`Fail` を `Pass` にしない。`native_surface_slot`/`ime`/focus release は「宣言」と明記し実測 Pass と混同しない。
- **Capability Matrix 不変**: ADR の宣言テンプレート表（Capability Matrix）は宣言→実測の混同を避けるため原則触れず、宣言記録は逆反映欄に集約した。
- **exp `.md` の index 扱い**: `docs/impl/exp` の管理 index（生成領域）は `.jsonl` のみを対象とし、`.md` 自由形式記録は hand-curated 節に列挙する慣行（issue-262 先例）。生成領域は `sync_impl_index.js` で再生成し、curated 節への 1 行追加は慣行どおり実施した。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-419-adr-backreflection.md` - verdict 転記方法・判定語彙適用・文書規約適用の試行記録。ADR 逆反映記述の根拠。

### Rejected

- なし（docs-only slice のため実装アプローチの比較実験は不要。記録手法の試行は exp に記録）。

## Validation

- `evidence/rf262-gpui-verdict/` は読み取り専用の判定正本として変更していない（`git status` で差分なしを確認）。
- 文書監査（`audit_docs`/`audit_impl_record`/`audit_experiment_log`/`doc_status` 系 `index plan`）で front matter・relation・index 整合を確認。
- `mise run fmt-check` + `prek run --files` で Markdown/リンクの軽量チェックを実行。
- 管理 index は `doc-maintenance index apply`・`sync_impl_index.js` の生成手順のみで再生成し、`index plan` で `in-sync` を確認。
- 未実行項目（Rust check/lint/test/build・`*-full`・`*-msrv`・Feature Matrix・3OS・実機 GUI/Native Dock/Consumer Oracle gate）は docs-only のため NotRun（POLICIES §5。統合検証は merge 後対象 SHA の CI 証拠）。

## Risks

- 「推奨」を「決裁」と読み替える記述 → design D2 の決裁/推奨分離で対応。
- `NotRun`/`Fail`/`Unsupported`/`Deferred` の `Pass` 化 → test-spec R-VOCAB・design D2 で固定。
- ADR status の誤変更 → `status: "proposed"` 不変を spec/Acceptance・task Done When・review で確認。
- 既存本文の不用意な書き換え → design D1.4 の触れない範囲。Decision Outcome・判定フロー・縮退順序・責務マップ・Capability Matrix・Verification は不変。
- index の手編集 → 生成手順のみを mutator とし `index plan` で `in-sync` を確認。

## Follow-ups

- #419: ADR `status` の正式決裁（`proposed`→`accepted`/`conditional`/`rejected`）— design authority 権限。残 DoD: 仮想優先 Gate（`#630`/`#631`）証拠深度記載・`#465` Legacy 撤去の独立判断。
- #532: Windows Native Dock 画素の根本 Blocker（DComp `topmost=true`、open）。対策決裁・実装が前提。
- #388/#389: Native Dock 残件（Dock layout/clip/tab・keyboard/IME routing・plugin 主導 resize 等）。
- #243/#313: 実 GUI 残件（非 Blocking の既知制約として ADR に記録済み）。
- Gate C 性能・Gate E Consumer Oracle: NotRun のまま後続（性能予算 Issue・consumer-verify 対象 SHA run・AL #166/SV #176）。
