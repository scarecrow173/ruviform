# RF625 — 検証台帳・NotRun 表 — Issue #625

- 対象変更: `crates/ruviform-ui-test/src/driver.rs` の handshake transport error 分類正規化(`hello_io`)+ `spawn_tcp` accept error child-exit 確認 + driver 内 unit test 7 件
- 実行環境: Windows ローカルホスト(worktree `rf625-ci-fix`)、mise 管理 toolchain — `rustc 1.99.1 (58cf8f9c3 2026-10-13)` / `cargo 1.99.1`
- 判定語彙: `POLICIES.md` §3 — `Pass`=実測成功、`NotRun`=実行可能だが未実施、`Unsupported`=機構上の非対応。推測と実測を区別する。

## 1. 実行済み検証

| # | コマンド | OS | toolchain | exit code | 結果 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `mise run fmt-check` | Windows | rustfmt(cargo fmt --check) | 0 | **Pass** | 初回は `hello_io` の chain 整形 diff で Fail → rustfmt 準拠に修正後 Pass |
| 2 | `mise run check ruviform-ui-test` | Windows | cargo 1.99.1 | 0 | **Pass** | `cargo check --locked --all-targets -p ruviform-ui-test` |
| 3 | `mise run lint ruviform-ui-test` | Windows | cargo 1.99.1 + clippy `-D warnings` | 0 | **Pass** | workspace pedantic lint 準拠 |
| 4 | `mise run test ruviform-ui-test --test e2e_process` | Windows | cargo 1.99.1 | 0 | **Pass** | 18 test 全 pass(15.18s)。`e2e_disable_env_kills_active_transport`・`e2e_fail_init_reports_failed_handshake` とも pass |
| 5 | `mise run test ruviform-ui-test` | Windows | cargo 1.99.1 | 0 | **Pass** | crate 全体 — 新規 unit test 7 件(`driver::tests::hello_*`)を含む全 binary pass |
| 6 | `mise run test-full` | Windows | cargo 1.99.1(`cargo test --workspace`) | 0 | **Pass** | workspace 全体 209 test-result block 全て `ok`・FAILED 0。`e2e_disable_env_kills_active_transport ... ok` を log で確認。compile 約 4m25s + test 実行 |

## 2. 決定的検証の内訳(unit test 7 件・`driver::tests`)

| test | 固定する分類契約 |
| --- | --- |
| `hello_write_broken_pipe_is_handshake_refusal` | hello write の `BrokenPipe`(= run 43540652567 の観測経路)→ `Handshake` + "closed before welcome" |
| `hello_write_connection_reset_is_handshake_refusal` | hello write の `ConnectionReset` → `Handshake` |
| `hello_read_connection_reset_is_handshake_refusal` | hello read の `ConnectionReset` → `Handshake` |
| `hello_read_truncated_frame_is_handshake_refusal` | frame 途中切断(`UnexpectedEof`)→ `Handshake` |
| `hello_read_clean_eof_is_handshake_refusal` | clean EOF → 従来どおり `Handshake` |
| `hello_non_channel_error_stays_io` | `InvalidData` + child 未 exit → `Io` 維持(過剰分類なし) |
| `hello_error_after_child_exit_is_handshake_refusal` | unmapped kind(`Other`)+ child exit 確認済み → `Handshake`(fallback) |

fake `FailWriter`/`FailReader`/`io::empty`/`io::Cursor`/`io::sink` と、exit 済み child として current test exe(`--help` で即終了)を注入し、OS・timing に依らず error 経路を決定的に再現する。`e2e_disable_env_kills_active_transport` の Windows ローカル pass は「たまたま通った」可能性を排除できないため、契約の証拠は本 unit test 群と CI 一次ログに置く。

## 3. NotRun / Unsupported

| lane / 検証 | 判定 | 理由 |
| --- | --- | --- |
| ubuntu 固有 EPIPE 経路の実機再現 | **NotRun** | Windows ホストでは同 timing の pipe EPIPE を決定的に踏めない。CI 一次ログの証拠 + 決定的 unit test で契約を検証。最終確認は merge 後対象 SHA の `verify (ubuntu-latest)`(Coordinator が CI で確認) |
| `verify-macos` / `ui-verify`・`ui-gpui-verify` の macOS leg | **NotRun** | macOS ホストなし。変更は OS 非依存の error 分類で、macOS 固有経路なし |
| `ui-verify`・`ui-gpui-verify` の windows leg(実 window 依存部) | **NotRun(部分)** | 変更 crate は process/transport 層で window 依存なし。`test-full` が workspace 全体を Windows で実行するため headless 部分はカバー |
| `l1-real-verify`(RT device 依存) | **NotRun** | 実デバイス/RT 環境がローカルにない。変更は ui-test driver のみで plugin host lane と無関係 |
| `ui-gpui-offscreen-verify`(Xvfb/実 window 環境依存) | **NotRun** | Linux Xvfb 環境がローカルにない。`ruviform-ui-gpui` の `looks_environment_blocked` 消費側分類は message prefix 互換をコードレビューで確認済み |
| `*-msrv` 系タスク | **NotRun(Unsupported)** | タスク指示により daily-CI scope として実行しない |
| `bench` / `consumer-verify` 等他 lane | **NotRun** | 変更は `ruviform-ui-test` のみで他 lane の対象外。merge 後 CI で確認 |
| 実機 CI での flaky 再発有無 | **NotRun(Pending)** | race の実発生確率は低くローカルでは観測不能。統合検証は merge 後の対象 SHA の必須 CI(成功で tracker auto-close)を Coordinator が確認 |

## 4. 補助証拠(canonical=mise 以外・参考記録)

- `git show --stat 2613b238` / `git diff --name-only 2613b238..origin/main -- crates/ruviform-ui-test/` — 失敗 sha が docs-only、かつ現 main tip でも同 crate 未修正であることの確認。
- `gh run view 43540652567 --log-failed` の一次ログ — panic 行・error 値・他 test 結果(`evidence/rf625/root-cause.md` §1 に引用)。
