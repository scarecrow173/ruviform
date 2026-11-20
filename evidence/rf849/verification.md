# ruviform#849 検証記録 — l1_fault E2E `SpawnFailed{Timeout}` bounded re-instantiate

実施日: 2026-11-21 / worktree: `rf849-reinstantiate` /
branch: `test/849-spawn-failed-reinstantiate`。OS: Windows /
rustc 1.99.1 (58cf8f9c3 2026-10-13)。canonical・誘発 run は base
SHA `19c529cc`（作業開始時点の origin/main — PR #850 merge）+ 本
slice の変更を適用したツリーで実施。

本 slice は test-file-only slice: 両 ABI の
`l1_fault_test_plugin_e2e.rs` の共有 `instantiate` helper に
rf#841 Decision C の bounded re-instantiate を実装。
`SpawnFailed{cause: Timeout}` かつ retained diagnostics record が
pre-mint phase（`PreSpawn`/`Spawn`/`DrainInit`/`Handshake`）・
`cause == Some(Timeout)` の signature を満たす場合のみ最大
3 retry（= 4 attempt）re-instantiate。bound 超過は正直に fail。
attempt ごとに attempt index + `diagnostics_id` + `record.phase`
+ `record.cause` を stderr へ直接 print — libtest の per-test
capture を bypass し、retry が recover しても lane log に証跡が
残る。assertion・gate・`#[ignore]`・失敗期待 helper
`instantiate_spawn_failed` の厳格性は不変。

## canonical lane 連続 run（`mise run test-l1-real-plugin`）

| run | 結果 | signature / 備考 |
| --- | --- | --- |
| run 1 | PASS（exit 0） | 全 suite green — `instantiate SpawnFailed trace` 0 件（`logs/canonical-run1.log`、16 `test result: ok`） |
| run 2 | PASS（exit 0） | 同上 0 件（`logs/canonical-run2.log`） |
| run 3 | PASS（exit 0） | 同上 0 件（`logs/canonical-run3.log`） |
| run 4 | PASS（exit 0） | 同上 0 件（`logs/canonical-run4.log`） |
| run 5 | PASS（exit 0） | 同上 0 件（`logs/canonical-run5.log`） |
| run 6 | PASS（exit 0） | 同上 0 件（`logs/canonical-run6.log`） |

結果: **6/6 PASS — `instantiate SpawnFailed trace` 0/6**
（`grep -c 'instantiate SpawnFailed trace'` を各 run log に対して
実施・全て 0）。flake signature は本測定では発火しなかった
（post-#846 の `spawn_timeout` 既定 10 s 下では期待どおり稀少）。
AC4 は「0/6・全 PASS」として正直に記録し、retry 実動証拠は下記
誘発 run の補助証拠で示す。fault suite は両 ABI とも全 test
green（vst3 20・clap 19 — `hang_before_hello_times_out`・
`usage_rejection_is_spawn_failed`・`descriptor_mismatch_is_handshake`
を含む → 失敗期待 helper `instantiate_spawn_failed` の厳格性と
既存 assertion が不変であることを実測）。

## 誘発 run（補助証拠 — canonical = 上記 mise lane）

`hang_converges_on_timeout` の spec に局所 patch を当てて
`instantiate` 経路に `SpawnFailed` を確実に発火させ、signature
分類・bounded retry・bound 超過 panic・stderr trace を実測した
補助証拠（`evidence/rf841/loop.sh` 型の誘発設計 — `cargo test`
直接実行は canonical ではない。patch は検証後に revert し、
committed diff には残らない）。

| run | patch | ABI | 観測 |
| --- | --- | --- | --- |
| A | `spec.spawn_timeout = 1ms` | vst3 | attempt=0 `Timeout`@`Spawn` → retry、attempt=1 `Timeout`@`Handshake` → retry、attempt=2 で instantiate 成功 → **test PASS** — recovered retry で trace が capture を bypass し pass test の lane log に残留することを実測（AC4 の機構実証）。`logs/induce-A-spawn1ms-vst3.log` |
| A | 同上 | clap | attempt=0 `Timeout`@`Spawn` → retry → attempt=1 成功 → **test PASS**（同上）。`logs/induce-A-spawn1ms-clap.log` |
| B | `--ruviform-test-fault=hang` + `spawn_timeout=500ms` | vst3 | attempt=0..3 全て `Timeout`@`Handshake`（4 trace）→ `3-retry bound` panic で **正直に FAIL**。`logs/induce-B-hang500ms-vst3.log` |
| B | 同上 | clap | 同上（4 trace → bound panic・`phase=Some(Handshake)` を panic message に表示）。`logs/induce-B-hang500ms-clap.log` |
| C | `--definitely-not-a-real-flag` | vst3 | `SpawnFailed{Internal}`@`Handshake` → signature 不一致 → retry 無し・1 trace のみで即 panic「outside the pre-mint timeout signature — not retried」。`logs/induce-C-usage-miss-vst3.log` |
| C | 同上 | clap | `SpawnFailed{Transport}`@`Handshake` → 同上。`logs/induce-C-usage-miss-clap.log` |

## AC 対応

- **AC1**（両 ABI `instantiate` で signature 限定 bounded
  re-instantiate 動作）: 誘発 A/B — signature を満たす
  `SpawnFailed{Timeout}` で attempt=0..3 の trace が残り retry が
  実行される。bound = 3 retry / 4 attempt（`SPAWN_TIMEOUT_RETRIES`
  const — discovery doc の例値 3 を採用。stall は machine 相関の
  事象のため更なる回数の限界効用は小さい）。
- **AC2**（staleness・signature 外を誤リトライしない分類維持）:
  誘発 C — 非 `Timeout` cause は 1 attempt で panic・retry 無し。
  code 上 signature は `cause == Timeout` かつ `Found` record の
  `phase` ∈ {`PreSpawn`,`Spawn`,`DrainInit`,`Handshake`} かつ
  `record.cause == Some(Timeout)` の 3 条件。`SpawnFailed` は
  pre-mint only のため post-mint staleness（C2 class）は構造的に
  `SpawnFailed` に畳まれず誤リトライ不可。`instantiate_spawn_failed`
  は無変更 — canonical で `hang_before_hello`・`usage_rejection`・
  `descriptor_mismatch` が green。
- **AC3**（panic log に deadline phase）: stderr trace が attempt
  ごとに `record.phase` を print し、panic message も
  `phase=...` を含む（B の bound panic は
  `phase=Some(Handshake)` を表示 — 誘発 log 実測）。
- **AC4**（lane log に retry 証跡）: canonical 0/6（未発火 —
  正直に記録）+ 誘発 A で **pass test の lane log に trace が
  残る** ことを実測（libtest capture を bypass する stderr 直
  書きにより、retry が recover しても証跡が消えない）。

## 実装上の注意（coverage boundary — 実装時に判明）

`Scan`/`CreateInstance` wait は `L1Child::spawn` が `Hello` 成功
直後に `set_phase(Live)`（`l1/mod.rs:2184`）してから実行される
ため、これらの timeout は `phase == Live` の record を残し、
4-phase signature では retry されない（signature 不一致で即
fail）。retry が有効な pre-mint wait は spawn hand-off（`Spawn`）
と `Hello`（`Handshake`）系に限定される — 誘発 run A は
`Spawn`・`Handshake` 両 phase での signature match を実測済み。
`Scan`/`CreateInstance` timeout を retry に含めるかは signature
保守性と coverage のトレードオフであり、本 slice は doc 判定表
どおりの保守境界で実装した（拡張判断は follow-up —
`docs/discovery/issue-841-spawn-deadline-flake.md` Residual に
記録済み）。

## その他 lane

| lane | 結果 |
| --- | --- |
| `mise run fmt` → `mise run fmt-check` | PASS（exit 0） |
| `mise run check-l1-real-plugin` | PASS（exit 0） |
| `mise run lint-l1-real-plugin` | PASS（exit 0 — clippy `-D warnings`） |
| `mise exec -- prek run --all-files` | PASS（exit 0 — 全 hook） |
| lease 外 `git diff` | 空 — 変更は両 ABI fault e2e・`docs/{test-specs,discovery}`・`evidence/rf849/` のみ。`Cargo.toml`/`mise.toml`/`docs/**/README.md`/`Cargo.lock`/production `src/**` 差分なし |
