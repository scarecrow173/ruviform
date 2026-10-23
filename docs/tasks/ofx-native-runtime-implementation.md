---
id: "TASK-034L6yeVFSaiVUoV4RiRtm"
type: "task"
status: "todo"
title: "Implement native OFX runtime"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-98-ofx-host-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-98-ofx-host-adapter.md"
    - "docs/tasks/ofx-core-contract-implementation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/video-plugin-plain-contract-tests.md"
    - "docs/test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Native runtime implementation

## Work

approved plan の **actions 5/6/7/8 を 1 task の ordered substeps として実行**。

### A5 — 選択可能 package + scaffold + tests

- `crates/ruviform-plugin-ofx` 最小 compilable package/lib/test targets + workspace explicit membership
- `mise.toml` に確定 TOML どおり `tasks.test-ofx`/`tasks.lint-ofx`/`tasks.test-ofx-msrv` を追加
- pinned header 閉包: `ofxInteract.h`（`ofxImageEffect.h` が 2 回 include）+ 公式 LICENSE を同 pin で取得・新 source hash 記録・BSD-3 通知保持（captured headers/manifest byte-for-byte reuse）
- real C fixture build harness + structured `Unsupported` を返す honest compilable API scaffold（**全 OK な SDK callback stub ではない**）
- `tests/native_host.rs` + fixture 契約テスト: admission/lifecycle/property/param/memory/callback 境界（TSpec 正本どおり）

### A6 — semantic red

- `mise run test-ofx` — compiled package で **host-behavior assertion に到達**（例: real `OfxHost::load_library` が structured `Unsupported` を返す vs 要求 real DLL admission）。package/import/command/compiler 欠落は red proof ではなく stop→blocker。native body は実 red 記録の前に書かない

### A7 — real native runtime + C bridge

- C が SDK layout/suite table/variadic/`OfxHost`/typed accessor を所有・Rust は fixed-arity primitive/pointer
- admission 順序（optional DLL `OfxSetHost`→GetNumber/GetPlugin→per-plugin setHost/main→Load→Describe/DescribeInContext→Create）・per-entry `PluginRejection`・admitted index≠native index・empty admitted lib 明示・canonical cache・shared `Rc` session 1・`CreateInstance` 前 SingleInstance 検査・予約・destroy 高々 1 回 terminal・quarantine・成功 Create なしで Destroy なし
- Send-safe owned registry（foreign thread callback は `Rc`/`RefCell`/metadata に触れない・Mutex を foreign call 跨がせない）・16B aligned owned pointer memory・`ErrMemory`・panic→SDK status・ParamV1 全 slot（truthful Unsupported 含む）・未知 suite/version→NULL・unsafe は FFI 局在 + SAFETY
- **render は明示的 `Unsupported` のまま**（次 task まで fake compat なし）

### A8 — green

- `mise run test-ofx`・`mise run lint-ofx`・`mise run test-ofx-msrv` 全 exit 0（実 log）

## Completion Gate

green 実 log + lead review の後にのみ **buildable な native scope 全体を一つの単位として commit**。

## Verification

- [ ] `mise run test-ofx` — exit 0
- [ ] `mise run lint-ofx` — exit 0
- [ ] `mise run test-ofx-msrv` — exit 0（MSRV 1.89 で lib+tests+C bridge コンパイル・実 log）
