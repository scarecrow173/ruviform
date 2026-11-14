# Issue #631 plugin-dock scenario skeleton — 実装実験記録（2026-11-15）

`docs/tasks/issue-631-plugin-dock-scenario.md` の first slice
（plugin-dock scenario skeleton）実装で試行・観測した事項の記録。
実装結果の正本は
[`docs/impl/ir/issue-631-plugin-dock-scenario.md`](../ir/issue-631-plugin-dock-scenario.md)、
イベントログは
[`plugin-dock-scenario-skeleton-probes.jsonl`](plugin-dock-scenario-skeleton-probes.jsonl)
を参照。

## 試行 1: `run_editor_attach` の `too_many_lines`（clippy）

- **目的**: session begin → fake slot declare/lease → `ParentSlotLease`
  → redeem → `session.open(Docked)` → teardown を 1 関数で直列に書く。
- **観測**: 初回 lint で `too_many_lines`（148/100）が発火。setup 失敗
  ごとの部分 teardown が match arm に散在し、関数が読みにくかった。
- **結論**: `fake_dock_slot_spec()`（SlotSpec 組立）と
  `attempt_editor_attach()`（attempt + teardown を `(AttachAttempt,
  Vec<String>)` で返す）に分割。`run_editor_attach` は verdict 写像のみに
  なり、部分失敗時の cleanup も helper 内で完結する。挙動は不変 —
  `editor-attach: unsupported` verdict を lane で再確認。

## 試行 2: `SampleRate`/`ProcessConfig` の fallible 変換 lint

- **観測**: `SampleRate::new(48_000)` と `ProcessConfig::new(...)` の
  `match` が `manual_let_else` / `single_match_else` に抵触。
- **結論**: `test_gain_config() -> Option<ProcessConfig>` helper に集約。
  host lifecycle case では `let Some(config) = ... else` で fail outcome
  に落とし、activate 経路では `ok_or(HostError::InvalidValue)` に統一。

## 試行 3: JSONL 出力の `serde_json::Map` API

- **観測**: `Map::shrink_to_fit` が存在せず初回 compile error。
  `ScenarioReport` は意図的に非 Serialize（driver 型を adapter が所有）
  のため、case 行は public field から `Map::from_iter` で組立。
- **結論**: shrink 呼出を削除。adapter 内部 ID（`targets`）は JSONL に
  出さない判断をコードコメントに明記。

## 試行 4: verdict の実測確認

- **観測**: `mise run test-ui-plugin-dock` の実走で
  `view-mount=pass` / `artifact=pass` / `host-lifecycle=pass` /
  `editor-attach=unsupported` / `dock-pixels=not_run`。
  editor-attach detail は `createView returned null →
  HostError::Unsupported → EditorOpError::Unsupported`。
- **結論**: 設計どおり `test_gain` に editor surface が無い現状で
  Dock Pass を主張しないことを JSONL evidence で確認。digest・version・
  plugin id・`environment_kind=real-abi-test-plugin` も記録された。
