# RF625 — verify(ubuntu-latest) `e2e_process` handshake refusal 分類 race の root cause 分析 — Issue #625

- Issue: https://github.com/scarecrow173/ruviform/issues/625
- 失敗 run: https://github.com/scarecrow173/ruviform/actions/runs/43540652567
- 失敗 sha: `2613b2388e0413af15913032421c57857e393682`(PR #619 merge commit)
- 作業 branch: `bugfix/625-ci-failure-main`
- 収集方法: `gh run view 43540652567 --repo scarecrow173/ruviform --log-failed` + repo 現行ソースの code path 追跡

## 1. 失敗の一次証拠(CI ログ引用)

`verify (ubuntu-latest)` job の `mise run test-full`(`cargo test --workspace --profile ci`)step で exit 101。`e2e_process` binary(18 test)のうち 1 件のみ失敗。

```
test e2e_fail_init_reports_failed_handshake ... ok
dummy_app: Disabled { reason: "RVF_TEST_DISABLE is set" }
test e2e_disable_env_kills_active_transport ... FAILED
...
---- e2e_disable_env_kills_active_transport stdout ----

thread 'e2e_disable_env_kills_active_transport' (18154) panicked at crates/ruviform-ui-test/tests/e2e_process.rs:325:18:
expected handshake refusal, got Io(Os { code: 32, kind: BrokenPipe, message: "Broken pipe" })
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    e2e_disable_env_kills_active_transport

test result: FAILED. 17 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.02s
```

- `e2e_process.rs:325` は `other => panic!("expected handshake refusal, got {other:?}")` — test は `DriverError::Handshake` かつ message に "closed before welcome" を含むことを要求している(L320-326)。
- 同 binary の他 17 test は全て pass。child stderr の `dummy_app: Disabled { reason: "RVF_TEST_DISABLE is set" }` は disable 経路が正常に発火したことを示す。
- 他の必須 job(rt-verify・compression-verify・icc-verify・io-structured-verify・ui-gpui-verify ×2OS・bench・verify-macos・l1-real-verify ×3OS・verify windows-latest・msrv-verify・ui-verify ×3OS・ui-gpui-offscreen-verify)は全て success。`ci-summary` は必須 job 失敗に伴う派生失敗。

## 2. 失敗 sha は regression ではない

- `2613b238` は PR #619「docs(issue-127): Linux 共通配布設計の既存 CI 照合と文書確定」の merge commit で、変更は `docs/`・`evidence/rf127/` のみ(src/test コード差分なし)。
- `git diff --name-only 2613b238..origin/main -- crates/ruviform-ui-test/` は空 — 失敗 sha 以降の 5 commit(PR #620〜#624)も同 crate を未触。既修正は main に存在しない。
- したがって本件は merge 起因の regression ではなく、test lane 内の潜在 race/flaky がこの run で初顕在化したもの。

## 3. Race の code path

`e2e_disable_env_kills_active_transport`(tests/e2e_process.rs:315-327)は `RVF_TEST_DISABLE=1` を付けて `TestDriver::spawn_stdio` を呼ぶ。child 側の流れ:

1. `dummy_app` が `serve_from_env` → `disabled_from_env()` で `ServeOutcome::Disabled` を返し、stderr に `Disabled { reason: "RVF_TEST_DISABLE is set" }` を出して **exit 0**(serve を拒否して即終了 — 仕様どおりの正しい挙動)。
2. 親の `TestDriver::handshake`(driver.rs)は `test.hello` を `write_frame` で書き(`?` で io error が `DriverError::Io` としてそのまま漏れる)、その後 `read_frame` の `FrameRead::Eof` で `Handshake("control channel closed before welcome")` を返す。

「welcome 前に channel が閉じた」は**同一の終了条件**だが、観測経路が 2 系統に分かれていた:

| 観測経路 | 条件 | 修正前の分類 |
| --- | --- | --- |
| write 側 | child exit が hello write より先 → Linux で stdin pipe の read end が消失し write が EPIPE(errno 32) | `Io(BrokenPipe)` ← 今回の観測 |
| read 側 | write が pipe buffer に入ってから child exit → read が clean EOF | `Handshake("control channel closed before welcome")` ← test の期待 |

どちらが先に観測されるかは process scheduling と OS の pipe 意味論に依存する timing race。windows-latest では read 側 EOF 経路が観測されて pass、ubuntu-latest では write 側 EPIPE が観測されて fail — 結果分布と整合する。

TCP transport でも同型の非対称があった:`spawn_tcp` の accept loop は WouldBlock 経路でのみ `child.try_wait()` を確認して `Handshake("app exited before TCP handshake")` を返し、その他の accept error は child 生死を見ずに `Io` 伝播 + kill していた。

## 4. 修正境界の判断

- **child 側は直さない**: `dummy_app` の disable → serve 拒否 → exit 0 は仕様どおり。`serve_from_env`/`TestAgent`/`transport.rs`/`dummy_app.rs` は無変更。
- **test assertion は緩めない**: `Handshake` 以外を許容する方向は分類 nondeterminism の隠蔽であり禁止事項。
- **分類を所有する `TestDriver::handshake` で正規化する**: hello 交換中の transport io error を `hello_io` helper で `Handshake`/`Io` に写像するのが責務的に正しい境界。`DriverError` variant 集合・公開 API・post-handshake(`call()`)の `Io`/`Disconnected` 区別は不変。
- **同種前例**: `f02f0bc6`(wrong-secret テストの Windows RST race を FIN-only + socket 保持で解消)、`63d3d126`(認証拒否テストで graceful close(FIN)を送信)はいずれも「同一の channel 終了条件が OS で異なる error に化ける」class の修正。本件はその driver 側対称形。

## 5. 同種問題の棚卸し(lease 内)

- `driver.rs` が handshake 分類の唯一の実装 — 他 module に重複実装なし。
- `e2e_fail_init_reports_failed_handshake` は `--fail-init` で `test.hello` が `ok:false`/`not ready` の result frame として返る経路(transport error ではない)で、本修正の影響を受けない。
- `e2e_tcp_wrong_secret_is_unauthenticated` / `e2e_unauthenticated_ops_rejected_before_hello` は `TestDriver::handshake` を経由しない raw frame 経路で、認証拒否は `ok:false`/`unauthenticated` result として返る — 影響なし。
- `looks_environment_blocked`(crates/ruviform-ui-gpui/tests/windowed_uitest.rs)は `Io`/`Disconnected`/`Timeout` を environment-blocked とし、`Handshake` は "control channel closed"/"app exited"/"bad welcome" を含む message のみ blocked 扱い — 新 message `"control channel closed before welcome: {e}"` は prefix を保持するため互換。
- lease 外の同種問題は検出されず。`spawn_tcp` accept error の child-exit 未確認は同一ファイル内の同一方針の非対称として本修正に含めた。

## 6. 残余リスク・別問題(記録のみ・本タスク対象外)

- **stdio `--stall before-serve` の handshake hang**: `handshake()` の read に timeout がなく、control channel を一度も開かない child では無期限 block し得る既存の latent 問題。本 run では `e2e_stall_before_serve_tcp_connect_times_out`(TCP 側は `CONNECT_TIMEOUT` で `Timeout` 化済み)は pass しており、stdio 側 stall の hang は今回顕在化していない。timeout 付与は別 Issue 相当として design の Deferred Design Concerns に記録。
- **unmapped errno kind の残余 race**: `ErrorKind` が名前を持たない OS error(EIO 等)かつ child がまだ zombie 化していない極小 window では `Io` が残り得る。観測済み errno(EPIPE=BrokenPipe)は kind match で確定的に覆われるため実害は限定的 — design Risks に記録済み。
