---
id: "TASK-034UaK7UfGDq7HHHhdeetT"
type: "task"
status: "in-progress"
title: "Issue #468 RootedDir canonical 前提の契約明示"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/468"
  changes:
    added: []
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
    generated: []
  implements:
    - "../plans/implement-issue-468-rooteddir-canonical-premise.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-468-rooteddir-canonical-premise.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-468-rooteddir-canonical-premise.md"
    - "../designs/issue-468-rooteddir-canonical-premise.md"
    - "../adr/issue-468-rooteddir-canonical-premise.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-468-rooteddir-canonical-premise.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #468 RootedDir canonical 前提の契約明示

## Work

plan `docs/plans/implement-issue-468-rooteddir-canonical-premise.md` の
T1〜T10 を実施する。`crates/ruviform-io-file` の API doc（module doc・
`RootedDir::open`/`open_with`/`open_scoped_root`/`canonical_root`/
`FollowMode::WithinRoot`）と crate README に canonical 前提を明記し、
既存 `rooted-read-capability` 系文書へ back-link を追加する。
resolver の挙動・公開 API 形状は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-468-rooteddir-canonical-premise.md`
- `docs/specs/issue-468-rooteddir-canonical-premise.md`
- `docs/designs/issue-468-rooteddir-canonical-premise.md`
- `docs/adr/issue-468-rooteddir-canonical-premise.md`

## Dependencies

- なし（文書チェーンは本タスク内で完結）

## Affected Files

- `crates/ruviform-io-file/src/rooted.rs`（doc comment のみ）
- `crates/ruviform-io-file/README.md`
- `docs/specs/rooted-read-capability.md`・`docs/designs/rooted-read-capability.md`・`docs/test-specs/rooted-read-capability.md`（relation back-link のみ）

## Done When

- [ ] API doc・README に canonical 前提（絶対 link target は
  `canonical_root()` の成分列で綴るか相対 target にする。`open` の
  引数 path の綴りは対象外）が明記されている
- [ ] ADR/design/spec/plan/task の文書チェーンが揃い、relation が
  broken でない
- [ ] `mise run` 検証列が pass する

## Verification

- [ ] `mise run check-io-file` exits 0
- [ ] `mise run test-io-file` exits 0（`absolute_link_alias_spelling_escapes_root` が無変更で pass）
- [ ] `mise run check` exits 0
- [ ] `mise run lint` exits 0
- [ ] `mise run fmt-check` exits 0
- [ ] `route_graph.js --focus docs/specs/issue-468-rooteddir-canonical-premise.md` が本タスク起因の broken-relation を報告しない
