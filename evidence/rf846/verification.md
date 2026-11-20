# ruviform#846 検証記録 — L1 spawn/handshake deadline の `command_timeout` 分離

実施日: 2026-11-21 / worktree: `rf846-spawn-deadline` /
branch: `bugfix/846-spawn-deadline-split`。OS: Windows /
rustc 1.99.1 (58cf8f9c3 2026-10-13)。計測・canonical run は
base SHA `c7db8df1`（計測開始時点の origin/main）+ 本 slice の
変更を適用したツリーで実施。`git fetch` 確認時点で origin/main
は `c7db8df1` のまま（rebase 不要）。

本 slice は production 根治 slice: `OutOfProcessSpec`/
`VideoOutOfProcessSpec`・内部 `SpawnSpec`/`VideoSpawnSpec`/
`SpawnSpecView` に `spawn_timeout`（既定 10 s）を additive 追加し、
spawn hand-off・`Hello`・`Scan`・`CreateInstance`/
`CreateVideoInstance` の各 pre-mint wait を `command_timeout` から
分離。post-mint reply wait は `command_timeout`、video `Render` は
`render_timeout` のまま不変。test assertion/gate は不変
（呼出側整合のみ: 短時間の pre-mint 発火を期待する test が
`spawn_timeout` を設定）。

## canonical lane 連続 run（`mise run test-l1-real-plugin`）

| run | 結果 | signature / 備考 |
| --- | --- | --- |
| run 1 | PASS（exit 0） | 全 suite green — 両 ABI の `real_test_fault_l1_hang_converges_on_timeout`・`real_test_spawn_l1_hang_before_hello_times_out` を含む（`logs/run1.log`、16 `test result: ok`） |
| run 2 | PASS（exit 0） | 全 suite green（`logs/run2.log`） |
| run 3 | PASS（exit 0） | 全 suite green（`logs/run3.log`） |
| run 4 | PASS（exit 0） | 全 suite green（`logs/run4.log`） |
| run 5 | PASS（exit 0） | 全 suite green（`logs/run5.log`） |
| run 6 | PASS（exit 0） | 全 suite green（`logs/run6.log`） |

結果: **6/6 PASS — `SpawnFailed{cause: Timeout}` signature 0 件**
（`grep -c 'SpawnFailed { cause: Timeout'` を各 run log に対して
実施・全て 0）。base では直近観測 6 run 中 2 fail（同 signature —
`evidence/rf841/verification.md`）だった flake が本測定では観測
されなかった。AC4 充足（観測 0 件として正直に記録 — 未計測・
fail の pass 扱いではない）。

## その他 lane

| lane | 結果 |
| --- | --- |
| `mise run fmt` → `mise run fmt-check` | PASS（exit 0） |
| `mise run check ruviform-plugin-host` | PASS（exit 0） |
| `mise run lint ruviform-plugin-host` | PASS（exit 0 — clippy `-D warnings`） |
| `mise run build ruviform-plugin-export` | PASS（exit 0 — `l1_video_e2e` real-runner lane の runner bin 前提） |
| `mise run test ruviform-plugin-host` | PASS（exit 0 — lib 226・`l1_proxy_e2e` 187・`l1_scale_probe` 112・`l1_supervisor_e2e` 23・`l1_video_e2e` 36・`plugin_graph_e2e` 15・doc-tests 3、全 green）。timeout 帰属 test が全て test 名で pass: `handshake_wedge_uses_spawn_timeout`（video・rename 済 — pre-mint は `spawn_timeout`）、`command_wedge_uses_command_timeout`（AC2 — post-mint は `command_timeout`）、`render_wedge_uses_render_timeout`（AC3 — `render_timeout` 分離不変）、`handshake_scan_timeout`・両 lane `spawn_failed_attempt_record_captures_stderr`・`hang_converges_on_command_timeout`・`request_timed_honors_the_explicit_timeout` |
| `mise run check-l1-real-plugin` | PASS（exit 0） |
| `mise run lint-l1-real-plugin` | PASS（exit 0 — clippy `-D warnings`） |
| `mise exec -- prek run --all-files` | PASS（exit 0 — 全 hook） |
| lease 外 `git diff` | 空 — 変更は `crates/ruviform-plugin-host/{src,tests}`・両 ABI fault e2e・`docs/{designs,test-specs}`・`evidence/rf846/` のみ。`Cargo.toml`/`mise.toml`/`docs/**/README.md`/`Cargo.lock` 差分なし |

## 補助 loop（補助証拠 — canonical = 上記 mise lane）

`evidence/rf846/loop.sh`（`evidence/rf841/loop.sh` 同型 —
copied fault binaries・分類 summary は `logs-<arm>-load<N>/`）。
rf841 計測の fault test 群は `command_timeout=500ms` を持つ
`hang_converges_on_timeout` を含む — AC1 の「`command_timeout=500ms`
設定下の suite 内 spawn」そのもの。

| arm | N/ABI | vst3 | clap | `spawn_timeout` signature |
| --- | --- | --- | --- | --- |
| full（suite 並列・canonical 露出 shape） | 20 | 20/20 pass | 20/20 pass | **0** |
| warm（WARMUP 先行・probe は warmed spawn） | 20 | 20/20 pass | 20/20 pass | **0** |
| full + LOAD=1（並行 cargo build 負荷） | 20 | 20/20 pass | 20/20 pass | **0** |

base での同型計測（`evidence/rf841/reproduction.md`）: full 6/20、
warm 2/40、full+load 5/20 — 全 fail が `SpawnFailed{Timeout}` @
`hang_converges_on_timeout` instantiate。修正後は同条件で
signature 0 件。pre-mint wait が `command_timeout` 非依存の
`spawn_timeout`（既定 10 s）へ移ったことが AC1 の構造的根拠と
整合（コード: `l1/mod.rs` `spawn_rx`/`hello_rx` の
`recv_timeout(spawn_timeout)`、`scan_match`/`create_instance`/
`create_video_instance` の `request_timed(..., spec.spawn_timeout)`）。
