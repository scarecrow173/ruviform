---
id: "TSPEC-5ibPChaXqhbfYjL713aqKw"
type: "test-spec"
status: "approved"
title: "plugin-host L0 テスト仕様"
created: "2026-11-03"
updated: "2026-11-03"
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
    - "docs/designs/plugin-host-l0.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies:
    - "docs/designs/plugin-host-l0.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# plugin-host L0 テスト仕様

## ケース

- `scan_lists_registered_factories`: 登録 factory の descriptor 列を `Scan` が返す
- `instantiate_unknown_id_rejected`: 未登録 `plugin_id` → `UnknownPluginId`
- `lifecycle_command_chain`: Instantiate→Initialize→Activate→StartProcessing→（process）→StopProcessing→Deactivate→Destroy の正規駆動と状態遷移確認
- `illegal_transitions_rejected`: `Processing` 中 `Deactivate`・`Activated` 中 `Destroy`・二重 `Initialize` 等が `Lifecycle` エラーで reject
- `activate_failure_reports_activate_failed`: `prepare` 失敗プラグイン → `ActivateFailed` 遷移で `Initialized` に留まる・再試行可能
- `state_roundtrip`: `SaveState`→`RestoreState` でプラグイン状態が復元
- `gui_unsupported_by_default`: 既定 `Plugin` 実装で `OpenGui` → `Unsupported`
- `stale_handle_rejected`: Destroy 後のハンドル・generation 不一致スロット → `InvalidHandle`
- `process_requires_processing_state`: `Processing` 以外での `process` が reject される
- `rt_process_no_alloc`: assert_no_alloc 窓内で `process` を連打し 0 alloc を実測
