# Issue #646 検証記録

対象: `crates/ruviform-numeric` lib test target の bench profile
コンパイル不能修正 + `docs/development-environment.md` drift 解消。

環境: Windows 11 x64, rustc/cargo 1.99.1 (mise 管理 toolchain)。
ブランチ: `fix/646-numeric-bench-profile`。

## 再現（修正前）

`mise exec -- cargo bench -p ruviform-numeric --no-run` が以下で
失敗することを確認（`bench-numeric-no-run-before.log`）:

- `error[E0425]: cannot find type AllocDisabler in crate
  assert_no_alloc` / `cannot find value AllocDisabler`
  — `assert_no_alloc` の default feature `disable_release` が
  `not(debug_assertions)` で同型を cfg 外にするため、bench
  profile（debug_assertions off）で宣言が解決不能。
- `error[E0282]` / `error[E0283]` — `corr.rs` の
  `assert_eq!(<f64 式>, [])` 10 サイトで、criterion 0.8.2 経由の
  `serde_json` `PartialEq<Value> for f64` 実装と競合し要素型が
  曖昧。

## 修正後検証（全て exit 0）

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | Pass |
| `mise run check ruviform-numeric` | Pass（`cargo check --locked --all-targets -p ruviform-numeric`） |
| `mise run lint ruviform-numeric` | Pass（`clippy -- -D warnings`） |
| `mise run test ruviform-numeric` | Pass — 116 tests + 7 doctests。`rt_tests::rt_processing_paths_do_not_allocate` / `rt_tests::rt_spectral_paths_do_not_allocate` が dev profile で収集・実行され、`AllocDisabler` による RT alloc 強制が維持されていることを確認 |
| `mise exec -- cargo bench -p ruviform-numeric --no-run` | Pass — bench profile で lib test target と `corr` bench がコンパイル（Issue AC、補助証拠） |
| `mise exec -- cargo bench --workspace --no-run` | Pass — workspace 全 bench target のコンパイル成功、他 crate の潜在 bench 破損なし |
| `mise exec -- prek run --all-files` | Pass |

ログ: `bench-numeric-no-run-after.log`、`bench-workspace-no-run.log`、
`verification.log`（同ディレクトリ、`*.log` は .gitignore 対象の
ため本ファイルに要約を転記）。

## 補足

- `mise run bench`（実測）・`mise run bench-smoke` は本 slice の
  検証対象外（コンパイル検証は `--no-run` で実施）。
- `*-msrv`・`*-full` lane・3 OS 横断はタスク指定により対象外。
- 統合受入は merge 後の main SHA に対する必須 CI 結果で確認する
  （POLICIES.md — ローカル検証は変更範囲の検証）。
