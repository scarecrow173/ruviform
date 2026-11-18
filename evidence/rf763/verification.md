# ruviform#763 検証記録 — l1_real_test_plugin_e2e instrument tests flaky 修復

実施日: 2026-11-18 / worktree: `leatherjacket` /
branch: `scarecrow173/rf763-flaky-e2e`

## 変更概要

- `crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs`
- `crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs`
- `docs/test-specs/issue-630-test-instrument.md`（R7 + Risk）
- production code 変更なし（test-file-only slice）

oracle: `PluginHost::audio_counters` の per-call delta で
matched（`passthrough_submits` delta = 0 → source = call − 2 の
child rendering）/ substituted（delta > 0 → `last_output` replay、
content assert skip）を分類。`dropped_submits` delta > 0 は
その call の event 喪失を示し、単発 NoteOff は上限付き
（`NOTE_OFF_RESEND_CALLS = 8`）resend-until-landed で着地確認。
release tail は arrived-block 数（`arrived_between` ≥ 6）で
rendered space の release 完了を判定。assertion・最低到達数
gate は不変。

## mise lane 結果

| lane | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS (exit 0) |
| `mise run check ruviform-plugin-test-clap` | PASS |
| `mise run check ruviform-plugin-test-vst3` | PASS |
| `mise run lint ruviform-plugin-test-clap` | PASS |
| `mise run lint ruviform-plugin-test-vst3` | PASS |
| `mise run test ruviform-plugin-test-clap` | PASS |
| `mise run test ruviform-plugin-test-vst3` | PASS |
| `mise run check-l1-real-plugin` | PASS |
| `mise run lint-l1-real-plugin` | PASS（初回は `cast_possible_truncation` ×5 + `items_after_statements` ×1 per file — `usize::try_from` / module-scope const へ修正後 PASS） |
| `mise run test-l1-real-plugin` run 1 | PASS（adapter 3×2 + instrument 10×2 + fault 16×2 + stateful 2×2 全 green） |
| `mise run test-l1-real-plugin` run 2 試行 A | FAIL — `cargo build` が fixture DLL を削除できず `os error 5`（実行中の loop binary が DLL を使用中 — 自らの並行実行による file lock、`canonical-run2.log` 初回分は上書き済み） |
| `mise run test-l1-real-plugin` run 2 試行 B | FAIL — `real_test_fault_l1_hang_converges_on_timeout`（vst3 fault suite、本 slice 対象外の test）が `SpawnFailed { cause: Timeout }`。spawn timeout の環境 flake と判断し再実行。`canonical-run2.log` |
| `mise run test-l1-real-plugin` run 3 | PASS（run 2 と同一 code で全 suite green — `canonical-run3.log`） |
| `mise exec -- prek run --all-files` | PASS |

### review-protocol 修正後の再検証（最終 code）

review 2 rounds（Reviewer A 4 件・B2 3 件）の指摘修正後に全 lane
を再実行:

| lane | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS (exit 0) |
| `mise run check-l1-real-plugin` | PASS |
| `mise run lint-l1-real-plugin` | PASS（`-D warnings`、clippy clean） |
| `mise run test-l1-real-plugin`（最終 code） | PASS（adapter `--lib --bins` + instrument 10×2 + fault 16×2 + stateful 2×2 全 green） |
| instrument suite targeted（`cargo test --features l1-runner --test l1_real_test_plugin_e2e -- --ignored`） | PASS 10/10 × 2 crate |
| `loop4.sh`（post-review binary、40 iter、iters 31-40 load 併走） | **40/40 green**（`logs5/`・`logs6/`） |

review 修正の内容は `docs/impl/ir/issue-763-flaky-e2e.md` の
「review-protocol round」「round 2」節を参照。

## Looped e2e 検証（補助証拠 — canonical = 上記 mise lane）

`loop3.sh`: 両 e2e binary（clap = e2e-a, vst3 = e2e-b）を各
iteration 同時実行。iters 1-30 は通常 concurrent、iters 31-40
は concurrent `cargo build` 負荷併走。

| run | 対象コード | 結果 |
| --- | --- | --- |
| pre-fix 再現（`loop2.sh`, 60 iter） | 未修正 `e4eb117f` | 12/120 binary-run 失敗（`logs2/`）— `reproduction.md` |
| window-variant（`loop3.sh` 初回） | oracle v1（window 維持） | 37/40 iter green、3 gate 割れ — `window-variant-failures.md` |
| exact-attribution v1 | oracle v2（clippy 修正前） | **40/40 green**（`logs3-preclippy/`・`logs4-preclippy/`） |
| exact-attribution v2（最終・review 前） | oracle v2 + clippy 修正 | **40/40 green**（`logs3/`・`logs4/`） |
| review 修正後（`loop4.sh`） | oracle v2 + review 2 rounds 修正 | **40/40 green**（`logs5/`・`logs6/`） |

## dropped_submits 観測（residual — production 側残件）

負荷下で `dropped_submits` が周期的に発生することを counter
delta で観測（submit channel full / slot busy — `data.rs:695-712`）。
本 slice では test oracle が drop を分類・resend で吸収するが、
drop 自体の発生は production 側の調査候補として残件化
（`docs/tasks/issue-763-flaky-e2e.md`・discovery 参照）。
assertion・gate を緩めて隠したものではなく、drop は各 assert
message の `(dropped=N)` 診断として記録される。
