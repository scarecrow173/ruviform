---
id: "SPEC-034Uay7qlGIC70AEeDcn7u"
type: "spec"
status: "proposed"
title: "Issue #468 RootedDir containment の canonical 前提の設計判断"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/468"
    - "https://github.com/scarecrow173/synvelis/issues/184"
    - "https://github.com/scarecrow173/synvelis/pull/186"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted.rs"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/issue-468-rooteddir-canonical-premise.md"
    - "../plans/implement-issue-468-rooteddir-canonical-premise.md"
  depends-on:
    - "../specs/rooted-read-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/rooted-read-capability-placement.md"
    - "../designs/rooted-read-capability.md"
    - "../test-specs/rooted-read-capability.md"
  refines:
    - "../specs/rooted-read-capability.md"
  refined-by: []
  derives-from:
    - "../specs/rooted-read-capability.md"
  derived-by:
    - "../designs/issue-468-rooteddir-canonical-premise.md"
    - "../plans/implement-issue-468-rooteddir-canonical-premise.md"
    - "../adr/issue-468-rooteddir-canonical-premise.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #468 RootedDir containment の canonical 前提の設計判断

## 背景

`ruviform-io-file` の `RootedDir` は、`WithinRoot` で絶対 link target を
canonical root の成分列と TEXTUAL に照合する。consumer が非 canonical な
綴り（macOS `/var`→`/private/var`、Windows junction/8.3 名など）で絶対
symlink target を作ると、kernel 上は root 内を指すのに `EscapesRoot` と
誤判定される。

synvelis#184（CI `verify-macos` / `verify (windows-latest)` 失敗）でこの
挙動が観測され、synvelis 側は fixture を `fs::canonicalize` する修正
（PR scarecrow173/synvelis#186、production コード無変更）で解消済み。
本 Issue は ruviform 側の**設計判断**として、現行契約が意図どおりかを
記録し、「canonical 前提」を明示することを求める。

## 目的

- 現行の containment 契約が意図どおりかどうかの設計判断を記録する。
- 「canonical 前提」（絶対 link target は `canonical_root()` の成分列で
  綴られていること、または相対 target であること）を契約文書・API doc
  のいずれかで明示し、consumer の落とし穴を減らす。

## 受入条件

| # | 条件 | 結果の記録先 |
| --- | --- | --- |
| A1 | 現状の containment 契約が意図どおりかどうかの設計判断を記録する。挙動変更を選ぶ場合は TOCTOU・escape 防止を弱めないセキュリティ上の根拠を design 文書で示す | `docs/adr/issue-468-rooteddir-canonical-premise.md`・`docs/designs/issue-468-rooteddir-canonical-premise.md` |
| A2 | 「canonical 前提」を契約文書・API doc・debug check のいずれかで明示する | `crates/ruviform-io-file` の API doc・`docs/` の契約文書 |

## 非目標

- resolver・vfs backend の挙動変更（本 Issue は判断記録と契約明示であり、
  誤判定の解消は consumer 側の綴り規則または別 Issue の設計変更で扱う）。
- consumer（synvelis・audio-lens 等）側の修正。
- 新規依存・feature の追加。

## 制約

- 既存の containment 保証（handle 相対 traversal・no-follow・
  `EscapesRoot`/`LoopDetected` 等の typed 失敗）を弱めない。
- `unsafe` 新規追加なし、公開 enum `#[non_exhaustive]` 規約・MSRV 1.89・
  `publish = false` 継承など既存の暫定規約に従う。
- 検証は `mise run` タスク経由（`check-io-file` / `test-io-file` /
  `check` / `lint`）。
