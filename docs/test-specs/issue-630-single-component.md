---
id: "TSPEC-630singleComponent01"
type: "test-spec"
status: "approved"
title: "Issue #630 single-component slice — test specification"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-single-component.md"
    - "docs/designs/issue-630-single-component.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-single-component.md"
  implements:
    - "docs/specs/issue-630-single-component.md"
    - "docs/designs/issue-630-single-component.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-stateful.md"
  related:
    - "evidence/rf630/single-component-verification.md"
  verifies:
    - "docs/specs/issue-630-single-component.md"
    - "docs/designs/issue-630-single-component.md"
---

# Issue #630 single-component slice — test specification

## Purpose

production VST3 bridge の `single_component == true` 経路を、
実バイナリ fixture で実走検証する。全 4 受入条件のうち観測可能な
範囲を deterministic（in-crate + host_path L0）と real-run（L1
out-of-process + 外部 validator）に分離し、観測不能な面は
honest に残る境界として記録する。

## Fixture contract（`single.rs` unit tests — deterministic）

| test | 検証内容 |
| --- | --- |
| `reports_own_controller_class_id` | `getControllerClassId` が `Self::CID` を書き `kResultOk` |
| `double_initialize_is_rejected` | 2 回目 `initialize` が非 ok（single arm の init-once pin） |
| `terminate_is_strict_to_initialized` | `Created` からの `terminate` 非 ok → init → terminate ok → 2 回目非 ok（double-terminate pin） |
| `component_handler_installs_once` | `setComponentHandler` 初回 ok・2 回目非 ok |
| `domain_verbs_match_gain_law` | normalized `0.75` → plain `1.5`・`0.375` → `0.75`、format/parse、`getParamNormalized` |
| `malformed_state_blobs_are_rejected` | blob 長・magic・version・非有限値 → `kResultFalse`（component + controller 両 domain） |
| `value_and_state_verbs_share_one_cell` | controller `setParamNormalized` → component `getState` に反映（1 cell） |

## L0 host_path（in-process production bridge — deterministic）

| test | 検証内容 |
| --- | --- |
| discovery pin 更新 | 7 descriptors（6 → 7）、`test_stateful_single` が名前順 index 6 |
| `single_component_reports_gain_param_in_plain_domain` | `params()` が scan 経由で Gain 面（id 0, `[0.0,2.0]`, default 1.0）を返す — `controller_cid == component_cid` 判定 + cast が production 経路で成功 |
| `single_component_save_mutate_restore_save_round_trips` | load → init → prepare → process → save → mutate(`setParamNormalized`) → restore → save byte-exact → teardown。保存 blob 長 24 + `[u32 16][RFST][u32 0]` framing pin |
| `single_component_restore_rejects_malformed_blob` | 壊した blob の `restore_component_state` → `PluginFailed` |

## L1 real-plugin（out-of-process runner — real-run）

| test | 検証内容 |
| --- | --- |
| `real_stateful_single_test_plugin_l1_save_mutate_restore_save` | descriptor index 6 の `Vst3` plugin を spawn → delivered-audio ordering gate → `SaveState` wire → mutate → `RestoreState` wire → `SaveState` → byte-exact + audio oracle + clean teardown。wire blob の `ctrl_len = 0` を assert |
| `real_stateful_single_test_plugin_l1_rejects_malformed_state` | malformed `RestoreState` → defined error + plugin生存・後続処理継続 |

## 外部 ABI validator（optional leg — real-run）

`mise run test-plugin-abi-validator` は provisioned validator があれば実行。
`vst3-validator.txt` に `TestSuite : test_stateful_single` セクションが
現れ全 pass することを確認。provisioned でなければ honest `NotRun`。

## 観測境界（test-spec 確定）

- `setComponentHandler` が「呼ばれた」こと自体は bridge 経由では
  観測不能（`BridgeComponentHandler` は no-op）。once-contract で
  「せいぜい 1 回」のみ pin する。
- `IConnectionPoint` pairing は観測 pin にならない: bridge は
  `!single_component` guard で pairing block を試行せず、仮に
  試行されても `if let (Some, Some)` の `None` cast 許容で
  silent skip になる（fixture 非実装は `init` 失敗を引き起こさ
  ない）。「pairing なし」の根拠はコード guard と single arm の
  正常終了のみ。
- RT invariance は `process` が `TestComponent<TestStatefulProcessor>`
  委譲のため既存 `rt_` suite でカバー（新規コードパスなし）。
