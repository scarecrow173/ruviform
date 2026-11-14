# Issue #262 — 検証実行記録（verification）

- 対象 Repo: `scarecrow173/ruviform`（worktree `G:\orca\workspaces\ruviform\rf262-verdict`）
- 証拠 SHA: `7aab630207e96863c31203629774364df87aa543`（worktree HEAD = origin/main HEAD）
- 実行環境: Windows 11 x64（ホスト）、mise 管理ツールチェーン
- 本記録は文書・証拠 slice の検証。Runtime/Consumer/OS 実機の新規検証は NotRun（既存証拠の照合と正規 lane 再実走のみ）。

## 実行した検証

| # | コマンド | OS | toolchain | exit | 結果 |
| --- | --- | --- | --- | --- | --- |
| 1 | `mise run test-ui-scenario-contract` | Windows 11 | mise rust（workspace pin） | 0 | fake lane 16 passed + gpui lane 7 passed。全 pass |
| 2 | `mise run check-ui-driver-boundary` | Windows 11 | 同上 | 0 | `gui-boundary-guard --offline` verdict: pass |
| 3 | `mise run test-ui-native-slot` | Windows 11 | 同上 | 0 | 13 passed |
| 4 | `mise run test-ui-parity` | Windows 11 | 同上 | 0 | 14 passed |
| 5 | `mise run test-ui-gpui` | Windows 11 | 同上 | 0 | lib 77 + windowed_test_app bin compile/link + consumer_observation 22 + golden_capture 1 passed/3 ignored + lab_story 4 + scenario_contract 7 + windowed_uitest 2 — 全 pass |

※ cargo 直接実行は補助証拠（canonical = mise）。上記は全て `mise run` 経由。

## 文書検証（実施済み）

| コマンド | OS | exit | 結果 |
| --- | --- | --- | --- |
| `mise run fmt-check`（= `cargo fmt --all -- --check`） | Windows 11 | 0 | Pass（差分なし） |
| `mise exec -- prek run --files <変更ファイル>` | Windows 11 | 0 | Pass（trim/eol/BOM/private key 全 Passed。yaml/toml/actionlint は対象外 skip） |
| `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index apply --dir docs/{specs,designs,test-specs,plans,tasks,impl/ir}` | Windows 11 | 0 | 各 dir `1 written`（index 再生成）。`docs/impl/exp` は generated index が `*.jsonl` のみ対象のため `0 written`（`.md` 記録は「Reports and raw data」節へ手動追記で対応） |
| `node .agents/skills/impl-doc/scripts/sync_impl_index.js --kind ir --apply` / `--kind exp --apply` | Windows 11 | 0 | 両方 `IN-SYNC`（index 既に source truth と一致） |
| `node .agents/skills/doc-status/scripts/audit_docs.js --type {spec,design,test-spec,plan,task} --dir docs/<dir> --json` | Windows 11 | 0 | 自スコープ（issue-262）の error/warning なし（既存の他ファイル warning は範囲外） |
| `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | Windows 11 | 0 | 自ファイルの finding なし（既存の他 IR の warning/error は範囲外・pre-existing） |
| `node .agents/skills/impl-doc/scripts/audit_experiment_log.js --json` | Windows 11 | 0 | 自ファイルの finding なし（既存 `.jsonl` の invalid-event-type/shape は pre-existing・範囲外） |

## NotRun / Unsupported（本 slice で新規実行しなかったもの）

- **NotRun**: 実 Native Dock・実 GUI・実 Plugin・Consumer 製品受入の新規検証（文書・証拠 slice のため）。macOS/Linux 実機 L2/L3。Gate C 性能測定。consumer-verify 対象 SHA run。
- **Unsupported**: macOS/Linux `native_surface_slot` backend（`cfg!(windows)` のみ）、IME（`WindowCapabilities.ime = Unsupported`）、CLAP/AU editor、L1 cross-process editor。
- **環境制限**: SendInput が現行ホストで無効化された記録あり（`evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md`）。製品 Pass ではなく環境要因。

## 補助証拠（CI run、ancestor SHA）

- run 43428172820（`e93c438a`）: 全 lane success — https://github.com/scarecrow173/ruviform/actions/runs/43428172820
- run 43439434737（`365c74ac`）: 全 20 job success — https://github.com/scarecrow173/ruviform/actions/runs/43439434737
- run 43540652567（`2613b238`）: `verify (ubuntu-latest)` FAILED（`e2e_disable_env_kills_active_transport`）— https://github.com/scarecrow173/ruviform/actions/runs/43540652567
- HEAD `7aab6302`: CI run 未実施（merge 後対象 SHA CI 証拠待ち）
