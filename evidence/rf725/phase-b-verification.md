# Issue #725 — serde 語彙 Phase B 検証記録（emit flip）

- 日付: 2026-11-17
- ブランチ: `feature/725-serde-vocab-phase-b`
- worktree: `G:/orca/workspaces/ruviform/rf725-serde-vocab-phase-b`
- base: `origin/main` `1d18e2970d887fe925d3777ddb814bc46644af3d`
- 対象変更: `EnvironmentKind::RealAbiTestPlugin` の serde emit 側を
  schema 語彙 `test-plugin` に統一（Phase A design の第2 slice —
  atomic flip: serde `rename` 切替 + `as_str()` flip +
  plugin-dock-test literal assert 更新）
- 環境: Windows（Git Bash 経由）、rustc 1.99.1、canonical =
  `mise run` 経由

## flip 前後の emit 綴り

| 経路 | flip 前（Phase A merge 後） | flip 後（本 slice） |
| --- | --- | --- |
| serde serialize | `"real-abi-test-plugin"` | `"test-plugin"` |
| `EnvironmentKind::as_str()` | `"real-abi-test-plugin"` | `"test-plugin"` |
| deserialize `"test-plugin"` | 受容（Phase A alias） | 受容（canonical） |
| deserialize `"real-abi-test-plugin"` | 受容（canonical） | 受容（alias — 恒久的） |
| `scenario.rs`:280 `json!(kind.as_str())` | `real-abi-test-plugin` | `test-plugin`（自動追従・編集不要） |
| `plugin_dock.rs` :168,:211 assert | `"real-abi-test-plugin"` | `"test-plugin"` |

## 実施した検証（canonical = mise）

| コマンド | OS | toolchain | exit code | 結果 |
| --- | --- | --- | --- | --- |
| `mise run check ruviform-audio-test` | Windows | rustc 1.99.1 | 0 | Passed |
| `mise run lint ruviform-audio-test` | Windows | rustc 1.99.1 | 0 | Passed（clippy `-D warnings`） |
| `mise run test ruviform-audio-test` | Windows | rustc 1.99.1 | 0 | Passed — metadata 13 tests 全 pass（`environment_kind_wire_spellings_match_case_schema_v1`・`environment_kind_deserialize_accepts_both_vocab_spellings`・`run_metadata_from_json_accepts_test_plugin_spelling`・`environment_kind_serde_matches_as_str`・`json_rejects_unknown_vocabulary_and_malformed_input`・`json_string_round_trip_preserves_all_fields` を含む） |
| `mise run check ruviform-plugin-dock-test` | Windows | rustc 1.99.1 | 0 | Passed |
| `mise run lint ruviform-plugin-dock-test` | Windows | rustc 1.99.1 | 0 | Passed |
| `mise run test ruviform-plugin-dock-test` | Windows | rustc 1.99.1 | 0 | Passed — `fixture_run_reports_honest_verdicts` 含む 4/4 |
| `mise run test-ci-consumer-verify` | Windows | node (mise-managed) | 0 | Passed — 84/84 tests（schema 側非回帰 — `ci/consumer-verify/**` は変更なし） |
| `mise run fmt-check` | Windows | rustc 1.99.1 | 0 | Passed（初回 2 箇所の rustfmt line-join 指摘を修正後に pass） |
| `mise exec -- prek run --all-files` | Windows | prek (mise-managed) | 0 | Passed |

## 実効カバレッジの確認（degrade 経路の有無）

`crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`:
137-149 の fail-closed degrade 設計（artifact 未解決時
`ScenarioVerdict::NotRun` で early-return）に対し、本 worktree
では `ruviform_plugin_test_vst3.dll` が
`target/debug/deps/` に build されており、
`artifact::resolve_artifact`（`src/artifact.rs`:65-80）の
candidate 2（`current_exe()` の `deps/` 直下）で解決される。
したがって `fixture_run_reports_honest_verdicts` は degrade
経路ではなく **full assert 経路**を実行し、:168・:211 の
`"test-plugin"` literal assert が実効的に検証された
（NotRun/degrade のみではない）。

## 未検証（NotRun）

- `mise run check-full`/`lint-full`/`test-full`/`build-full`/
  `verify-full`・Feature Matrix・`*-msrv`・3OS 横断 —
  daily CI 委譲（POLICIES §5・共通契約の除外規定）。
- opt-in lane（`mise run test-ui-plugin-dock` /
  `test-ui-plugin-dock-console` / `test-ui-plugin-dock-real*` /
  `test-ui-plugin-dock-lab` 系）の実走と新規 evidence JSONL
  再採取 — CI 必須 gate でなく運用判断。必要なら Coordinator
  確認。
- GUI-L2/EXT-L3 の実機 lane — 本 slice は serde 語彙の
  serialization/test 挙動のみで実機 capability を使用しない
  ため対象外（OS-L1 相当のローカル検証で実施）。

## 補助証拠（canonical ではない直接確認）

- `grep -rn "real-abi-test-plugin" crates/` — 残存参照は
  `metadata.rs` の doc コメント・`tests/metadata.rs` の
  alias/旧綴り受容 pin のみ（意図的な互換参照）。
  `plugin_dock.rs`・`Cargo.toml`・`scenario.rs` に旧綴り
  literal なし。
- `git status` — 変更は lease 範囲内のみ（4 ソース/テスト
  ファイル + issue-629 docs 5 ファイル追記 + docs chain +
  `evidence/rf725/`）。`docs/**/README.md`・`Cargo.lock`・
  `ci/consumer-verify/**`・`.github/**`・consumer repo・
  historical evidence は無変更。
