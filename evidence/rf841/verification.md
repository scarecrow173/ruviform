# ruviform#841 検証記録 — spawn-deadline flake 定量・分類 slice

実施日: 2026-11-21 / worktree: `rf841-spawn-flake` /
branch: `chore/841-spawn-deadline-flake`。OS: Windows / rustc 1.99.1。
計測・canonical run は全て SHA `6016bc64`（計測開始時点の
origin/main）で実施。その後 origin/main は PR #844/#845 へ進み、
本ブランチは同 SHA 上へ rebase 済み（本 slice の変更は docs/
evidence/shell script のみで code 非依存 — host/test crate の計測
対象箇所は rebase 後も不変であることを確認済み）。

本 slice は調査・文書化のみ（production・test file 変更なし）。
変更物: `evidence/rf841/` 新設（loop script・binaries・logs・本記録）、
`docs/discovery/issue-841-spawn-deadline-flake.md` 新規、
`docs/test-specs/issue-630-test-fault.md` 参照追記。

## canonical lane 連続 run（`mise run test-l1-real-plugin`）

| run | 結果 | signature / 備考 |
| --- | --- | --- |
| run 1 | **FAIL**（task exit 非 0） | vst3 fault suite `real_test_fault_l1_hang_converges_on_timeout` が `instantiate` で `SpawnFailed { cause: Timeout, diagnostics_id: 1 }`（`canonical-run1.log`） |
| run 2 | PASS（exit 0） | 全 suite green（`canonical-run2.log`、16 `test result: ok`） |
| run 3 | PASS（exit 0） | 全 suite green（`canonical-run3.log`） |
| run 4 | **FAIL** | clap fault suite 同 test・同 signature（`canonical-run4.log`、`:148` unwrap） |
| run 5 | PASS（exit 0） | 全 suite green（`canonical-run5.log`） |
| run 6 | PASS（exit 0） | 全 suite green（`canonical-run6.log`） |

結果: **6 run 中 2 fail — 全て同一 C1 signature**
（`SpawnFailed{Timeout}` @ `hang_converges_on_timeout` instantiate、
vst3 1 件・clap 1 件）。再実行 PASS（run2/3/5/6）との組で環境 flake
として正直に記録 — fail を pass 扱いしない。lane 単位の fail 率
~33%（本観測）は full-arm 計測（per-binary-run ~27%）から両 ABI
独立で 1-(1-p)²≈47% と整合する範囲。

## その他 lane

| lane | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS（exit 0 — `cargo fmt --all -- --check`） |
| `mise run check-l1-real-plugin` | PASS（exit 0 — 全対象 crate check） |
| `mise run lint-l1-real-plugin` | PASS（exit 0 — clippy `-D warnings`） |
| `mise exec -- prek run --all-files` | PASS（exit 0 — 全 hook） |
| lease 外 `git diff` | 空 — 変更は `evidence/rf841/`・`docs/discovery/issue-841-*.md`・`docs/test-specs/issue-630-test-fault.md` のみ。`Cargo.lock` 差分なし |

`test-l1-real-plugin` の flake 観測（run 1・4）は本 slice の docs/
evidence/shell script 変更とは無関係 — code 変更なしのため lane
健全性への影響はなく、観測された fail はすべて本 slice で定量対象の
flake signature そのもの。

## 補助 loop（補助証拠 — canonical = 上記 mise lane）

`evidence/rf841/loop.sh`（arm/load パラメータ化・fail 許容の計測 run —
失敗率の記録が目的）の結果は `reproduction.md` を参照: 合計
240 binary run・21 fail・全て C1 signature。cold（孤立 spawn）は
LOAD=1 含め 0/100、suite 内条件で 5-30%、fail は iter 単位で両 ABI
相関。分類ルール（C1 pre-mint `SpawnFailed` vs C2 post-mint
staleness）と対処方針の決定は
`docs/discovery/issue-841-spawn-deadline-flake.md`。
