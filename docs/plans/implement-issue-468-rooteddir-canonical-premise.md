---
id: "PLAN-034UaIKFsXeS7a1ILrmjKe"
type: "plan"
status: "approved"
test-spec-skip: "挙動変更なしの doc comment・契約文書のみの変更。canonical 前提の実行可能な pin は既存の実 FS 試験 absolute_link_alias_spelling_escapes_root（crates/ruviform-io-file/tests/rooted_read.rs）が既に担っており、新規 test-spec は作成しない"
title: "Issue #468 RootedDir canonical 前提の契約明示 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/468"
  changes:
    added:
      - type: "doc"
        path: "docs/adr/issue-468-rooteddir-canonical-premise.md"
      - type: "doc"
        path: "docs/specs/issue-468-rooteddir-canonical-premise.md"
      - type: "doc"
        path: "docs/designs/issue-468-rooteddir-canonical-premise.md"
      - type: "doc"
        path: "docs/tasks/implement-issue-468-rooteddir-canonical-premise.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted.rs"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
      - type: "doc"
        path: "docs/specs/rooted-read-capability.md"
      - type: "doc"
        path: "docs/designs/rooted-read-capability.md"
      - type: "doc"
        path: "docs/test-specs/rooted-read-capability.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
  implements:
    - "../specs/issue-468-rooteddir-canonical-premise.md"
  implemented-by:
    - "../tasks/implement-issue-468-rooteddir-canonical-premise.md"
  depends-on:
    - "../designs/issue-468-rooteddir-canonical-premise.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-468-rooteddir-canonical-premise.md"
    - "../adr/issue-468-rooteddir-canonical-premise.md"
    - "../specs/rooted-read-capability.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-468-rooteddir-canonical-premise.md"
    - "../designs/issue-468-rooteddir-canonical-premise.md"
  derived-by:
    - "../tasks/implement-issue-468-rooteddir-canonical-premise.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #468 RootedDir canonical 前提の契約明示 実装計画

## Goal

`RootedDir` containment の canonical 前提（絶対 link target は
`canonical_root()` の成分列で綴るか相対 target にする）を、設計判断
（ADR/design）として記録し、API doc・crate README・契約文書に明示する。
resolver の挙動・公開 API 形状は変更しない。

## Scope

単一の実装ストリーム。変更は doc comment と契約文書のみで、コードの
実行経路・公開 API・依存関係に影響しない。分割不要。

## Upstream Documents

- `docs/specs/issue-468-rooteddir-canonical-premise.md`（proposed）
- `docs/designs/issue-468-rooteddir-canonical-premise.md`（approved）
- `docs/adr/issue-468-rooteddir-canonical-premise.md`（proposed —
  判断の正本）
- 上流契約の正本: `docs/specs/rooted-read-capability.md`、
  `docs/designs/rooted-read-capability.md`、
  `docs/test-specs/rooted-read-capability.md`

## Design Inputs

- `docs/designs/issue-468-rooteddir-canonical-premise.md`（approved）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-io-file/src/rooted.rs` | module doc と `RootedDir::open`/`open_with`/`open_scoped_root`/`canonical_root`/`FollowMode::WithinRoot` の doc comment | doc のみ。コード行は変更しない |
| `crates/ruviform-io-file/README.md` | rooted 節に canonical 前提の bullet を追記 | 日本語 README の既存構成に合わせる |
| `docs/specs/issue-468-rooteddir-canonical-premise.md` | spec（作成済み） | front matter relations 確定済み |
| `docs/designs/issue-468-rooteddir-canonical-premise.md` | design 判断記録（作成済み・approved） | 同上 |
| `docs/adr/issue-468-rooteddir-canonical-premise.md` | ADR（作成済み・proposed） | 同上 |
| `docs/specs/rooted-read-capability.md` | 既存 spec に新規文書への `related`/`derived-by` back-link を最小追加 | approved 文書の relation 保守のみ。本文は変更しない |
| `docs/designs/rooted-read-capability.md` | 同上（design 側） | 同上 |
| `docs/test-specs/rooted-read-capability.md` | 同上（test-spec 側） | 同上 |
| `docs/plans/README.md`・`docs/tasks/README.md`・`docs/specs/README.md`・`docs/designs/README.md`・`docs/adr/README.md` | index 再生成 | script が更新済み。task を done にする際は `doc_maintenance.js` で再生成 |

## Implementation Sequence

- [ ] T1: `rooted.rs` module doc に canonical premise の段落を追加する
  （`open` に渡す path の綴りは任意で anchor は handle 由来 canonical、
  canonical 前提が効くのは絶対 link target の綴りであること）。
- [ ] T2: `RootedDir::open`/`open_with` の doc に、引数 path の綴りを
  canonical にする必要がないことと、canonical 前提の適用範囲
  （tree 内の絶対 link target）を明記する。
- [ ] T3: `open_scoped_root` の doc に、新 floor の anchor も
  handle 由来 canonical であり同じ canonical 前提が絶対 link target
  に適用されることを明記する。
- [ ] T4: `canonical_root()` の doc に、consumer が root 内に絶対
  symlink を作る場合の target 基底としてこの path を使う手順と、
  「その capability の anchor に対する綴り」である旨を追記する。
- [ ] T5: `FollowMode::WithinRoot` の doc は既に alias 縮小を明記済み。
  `canonical_root()` への相互参照が不足していれば補強する（現状で
  十分なら変更しない）。
- [ ] T6: `crates/ruviform-io-file/README.md` の rooted 節に、
  canonical 前提と consumer 側の回避策（canonical 綴りまたは相対
  target）を箇条書きで追記する。
- [ ] T7: 既存 `rooted-read-capability` 系 spec/design/test-spec の
  front matter `related`（必要なら `derived-by`）に本 Issue の文書を
  back-link として追加する。本文は変更しない。
- [ ] T8: `mise run fmt-check` / `check-io-file` / `test-io-file` /
  `check` / `lint` を実行し結果を記録する。
- [ ] T9: review-protocol で文書チェーンと diff をレビューし、
  blocker が残っていないことを確認する。
- [ ] T10: Conventional Commits で commit → push → PR 作成
  （日本語本文）→ worker_done。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | doc comment の記述が実装と食い違う | low | medium | 記述は `rooted.rs`/`resolve.rs` の現行実装を直接確認して書く。検証は型・lint・既存試験で担保 |
| 2 | main の既存 broken doc relation（`docs/impl/ir/issue-389-...`、`docs/plans/implement-issue-404-405-...`）が `artifact-graph` blocker を出し graph route を塞ぐ | 確定（main 既存） | low | 本タスク範囲外のため修正しない。新規文書が新たな broken relation を生まないことだけ確認し、PR に既存事象として報告する |
| 3 | API doc の英語トーンと既存記法（「RT contract」等）の不統一 | low | low | 既存 doc comment の文体に合わせる。lint ゲートは doc 内容を検査しないためレビューで担保 |
| 4 | 公開 API 互換への影響 | none | none | doc comment のみで API 形状・挙動は不変 |

## Task Breakdown

- T1〜T7 は `crates/ruviform-io-file` の doc/comment と既存文書
  relation のみを触る単一ストリーム。T8〜T10 は検証・レビュー・
  受入の流れ。
- test-spec は `test-spec-skip`（front matter 参照）により作成しない。

## Verification

| Step | Verification command or criteria | Pass condition |
| --- | --- | --- |
| T1〜T7 | `mise run check-io-file` | exit 0 |
| T1〜T7 | `mise run test-io-file`（`absolute_link_alias_spelling_escapes_root` を含む） | exit 0・契約 pin 試験が無変更で pass |
| T8 | `mise run check` / `mise run lint` / `mise run fmt-check` | 全て exit 0 |
| T8 | `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --current briefing --focus docs/specs/issue-468-rooteddir-canonical-premise.md --json` | 本タスク起因の broken-relation が出ない（main 既存の issue-389/issue-404 分は残存を許容） |
| T9 | review-protocol の最終レポート | verdict が ACCEPT または ACCEPT WITH NON-BLOCKING NOTES |

## Rollback

- 変更は doc comment・README・docs 文書のみ。問題があれば該当 commit を
  `git revert` すれば完全に元に戻る。公開 API・永続データ・生成物の
  互換影響はない（`Cargo.lock`・lockfile 系の変更なし）。

## Open Questions / Gaps

- なし。canonical 前提の適用範囲（絶対 link target の綴りのみで、
  `open` の引数 path の綴りは対象外）は design 文書で確定済み。

## Self-Review

- [x] Every spec requirement maps to a task（A1 → 文書チェーン T0
  相当の spec/ADR/design、A2 → T1〜T7）。
- [x] No placeholders remain。
- [x] File paths, relations, status values, and terminology are consistent。

## Implementation Handoff

- Recommended mode: inline execution（worker が直列に実施。doc/comment
  のみの小規模変更で分割不要）。
- Independent work streams: none。
- User approval required before dispatch: no（本タスクは worker 契約に
  より review-protocol を承認の代替として実施済み）。

## Review Handoff

- 前提: `RootedDir::open`/`open_scoped_root` の anchor は handle 由来
  canonical であり、canonical 前提が効くのは tree 内の絶対 link
  target の綴りのみ — `crates/ruviform-io-file/src/rooted.rs` と
  `src/rooted/resolve.rs` を根拠に確認済み。
- reviewer への着目点: doc 記述と実装の一致、ADR の却下理由
  （照合時正規化が TOCTOU 不変条件と衝突する点）の妥当性、
  main 既存の broken relation を本タスクで修正していないこと。
