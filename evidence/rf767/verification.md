# rf767 verification — 検証記録

ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA の CI 証拠(Coordinator が確認)。本タスクの最終差分は **docs + evidence のみ**(production/test code の変更なし — 観測 probe は除去済み、test file は最終差分ゼロ)。

## 環境

- OS: Windows(OS-L1)
- worktree: `rf767-l1-submit-drop`、branch `scarecrow173/rf767-l1-submit-drop`

## 調査中の検証(instrumented、補助証拠)

| # | コマンド | 結果 |
| --- | --- | --- |
| 1 | `mise exec -- cargo test -p ruviform-plugin-test-vst3 -p ruviform-plugin-test-clap --features l1-runner --test l1_real_test_plugin_e2e --no-run` | exit 0(instrumented binary 生成) |
| 2 | `evidence/rf767/loop-observe.sh`(RUVIFORM_L1_PROBE=1、両 binary 並行 × 30 iter) | exit 0、60/60 run pass、drop 帰属データ取得 |

## 最終差分の検証(正式)

base: `origin/main` `3a5d2d77` 取り込み済み(fast-forward)。最終差分は docs + evidence のみで Rust code の差分はゼロだが、観測で test file を一時変更した履歴があるため canonical lane も実施。

| # | コマンド | exit | 結果 |
| --- | --- | --- | --- |
| 1 | `mise run test-l1-real-plugin` | 0 | canonical lane — 16 test binary 全 suite pass(実 plugin E2E 両 ABI・fault・stateful・attach)、0 failed |
| 2 | `mise run fmt-check` | 0 | `cargo fmt --all -- --check` pass |
| 3 | `mise exec -- prek run --all-files` | 0 | 全 hook pass |
| 4 | `mise run check/lint/test <crate>` | — | NotRun — 最終差分に Rust code の変更がないため doc-only 検証方針に従う(POLICIES §5・タスク Spec「文書・ポリシーのみの変更」) |

## 判定まとめ

- `dropped_submits` の発生条件は `data.rs` の 2 分岐(slot-busy / chan-full)のみで、観測 drop は全件 slot-busy。
- 設計意図(bounded queue shedding)と判定 → production 変更なし、契約を `docs/specs/issue-767-submit-drop.md` に明記。
