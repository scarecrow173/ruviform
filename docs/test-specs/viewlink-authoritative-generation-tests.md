---
id: "TSPEC-42XLbuUif4jmek9sQGVA3S"
type: "test-spec"
status: "approved"
title: "ViewLink authoritative generation テスト仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/designs/viewlink-authoritative-generation.md"
    - "docs/plans/viewlink-authoritative-generation.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/binding/test/link.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/viewlink-authoritative-generation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/viewlink-authoritative-generation-t1-sdk-epoch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/viewlink-authoritative-generation.md"
  derived-by: []
  verifies:
    - "docs/designs/viewlink-authoritative-generation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ViewLink authoritative generation テスト仕様

`ui/binding/test/link.test.ts` に追加する恒久テスト（node --test + MockDawGateway/MockAnalysisBackend）。

## Feature: attach応答generationの採用

- fresh linkへ `attach("window-A", 7)` → `link.generation === 7`、binding attach epoch、subscription `ctx.generation`、dispatch `ctx.generation` が7
- attach後の late-created binding も同じauthoritative epochを採用し、そのwire ctx.generationが一致
- `detach` → `attach("window-A", 11)` → 全要素が11へ再同期
- QueryBindingの発行query `generation` がauthoritative値に一致

## Feature: epoch遷移

- 同window・同値（指定）または未指定 → 冪等no-op（再購読・attach回数不変）
- 同window・異epoch指定 → in-flight gestureのcancel（旧epochでmint）、旧subscription解放、新epochで再購読・binding再attach
- 異window + 指定epoch → `dock-transfer` detach後に指定epoch採用
- epoch遷移後に旧epoch listenerへのprojectionが新epochへ漏出しない（staleとして落ちる）

## Feature: 無効入力の原子性

- `-1`、`1.5`、`NaN`、`Number.MAX_SAFE_INTEGER + 1` を指定 → `RangeError` で拒否、attached/windowId/generation/subscriptions/bindingsは不変

## Feature: local fallback

- `attach(windowId)`（generation省略）→ 既存の `generation_ += 1` を維持（既存テスト群の契約不変）
