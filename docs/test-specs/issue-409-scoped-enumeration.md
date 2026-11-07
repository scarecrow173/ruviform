---
id: "TSPEC-034TnnBwjsmwufsaEZZ0RG"
type: "test-spec"
status: "approved"
title: "Issue #409 scoped re-root + bounded enumeration テスト仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-409-scoped-enumeration.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-409-scoped-enumeration.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #409 scoped re-root + bounded enumeration テスト仕様

## Purpose

Issue #409 の二契約（`open_scoped_root` 再 root 化、`read_dir_each`
bounded 列挙）の保証を固定する。Consumer workaround 解除に依存する
ため誤った floor 縮小・全量 materialize への回帰を防ぐ。

## Feature

Rooted capability の scoped re-root と停止可能な directory 列挙

## Rules

- re-root は解決した handle のみを新 floor とする（祖先 chain を
  引き上げない）。新 floor 内 `..`・sibling・absolute link の新
  scope 外解決は `EscapesRoot`。
- re-root の lexical anchor は `canonicalize_handle()` 由来のみ。
  取得不能時は `Unsupported` で失敗し、path 文字列再 bind /
  anchor なし縮小を行わない。
- re-root は `NodeId` 照合・Strong identity を要求しない
  （Weak FS でも動作）。
- 保持 handle 経由のため、root rename/replacement 後も
  `open_scoped_root` は継続動作し、旧 pathname 置換物へ別
  authority を取得しない。
- `read_dir_each` は全量 Vec 化せず entry ごとに visitor を呼び、
  `Stop` 後は後続 OS 列挙 call / per-entry probe / callback を
  発行しない（OS 固定 batch 内の未訪問 record 破棄は許容）。
- `EnumerationOutcome::{Complete, Stopped}` で完走 / 早期停止を
  区別し、OS 失敗は `Err`。列挙 state は呼出しごとに独立。
- `read_dir` は `read_dir_each` 上の collect で既存契約を維持
  （返却型・整列非保証・error 写像不変）。

## Examples

- Given root 取得後に root を rename → `open_scoped_root("")`
  が成功し、新 scope で `read_dir` 可能。
- Given root 取得後に root 名を別 dir で置換 → 保持 handle の
  `open_scoped_root` は旧（rename 元）dir を floor とし、置換
  物とは無関係。
- Given scope=`a/` 内 link が `..` で親に向く → `open("..",
  WithinRoot)` / absolute `/root_parent/b` は `EscapesRoot`。
- Given N 件の dir に `remaining+1` 個目で `Stop` → 列挙
  counter が prefix 件数で止まり、probe/OS call が続かない。
- Given 巨大論理 dir → 停止後の `enum_pulled` は prefix 件数に
  止まり、全量 pull にならない。
- Given visitor 即時 `Stop`（max_entries=0 相当）→ `Stopped`。
- Given 列挙途中の OS 失敗注入（`list_fail_at`）→ `Err` で
  `Stopped` ではない。
- Given visitor panic → OS 列挙資源が解放され、次回列挙は独立
  state で完走する。

## Guarantees

- `open_scoped_root` が path re-open / identity 照合を行わない
  こと（handle 権威）。
- absolute link target が新 canonical anchor prefix 内なら
  `WithinRoot` で解決可能（既存正常系の黙示縮小なし）。
- `Unsupported` が「保証不能」として分類され、保証可能な
  backend では発生しない。

## Non-goals

- Consumer 側接続（synvelis `local.rs`）の検証 — 別担当。
- 列挙順序の保証（既存どおり非整列）。
- `pin`/`open_pinned` identity 契約の変更検証（既存 spec で担保）。

## Risk

- anchor 必須化で稀な backend で `Unsupported` — 実測 OS では
  発生しないことを確認し未実測は NotRun。
- mock tree-model 化で既存 test が壊れないこと（`put`/`remove`
  順序依存を保持）。
