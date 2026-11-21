# ruviform#855 検証記録 — popup_toggle 論理 open flag 修正（AC1 分類記録 + AC2 実 regression 修正）

実施日: 2026-11-21 / worktree: `rf855-popup-key` /
branch: `bugfix/855-popup-toggle-nonwindows`。OS: Windows /
rustc 1.99.1 (58cf8f9c3 2026-10-13)。base SHA `ec133ac2`
（作業開始時点の origin/main — PR #858 merge）+ 本 slice の変更を
適用したツリーで実施。

本 slice は test-fixture code + docs/evidence のみ。変更物:
`crates/ruviform-plugin-test-vst3/src/editor_view.rs`（`ViewState` への
`popup_open: bool` 追加・`popup_toggle` の oracle 分離・`removed()` での
flag reset・doc comment 整合・`#[cfg(test)] mod tests` へ unit test 2 本
追加）、`docs/discovery/issue-855-popup-key-ci.md` 新規、
`evidence/rf855/` 新設（本記録）。`Cargo.toml`/`mise.toml`/`Cargo.lock`/
`docs/**/README.md`/他 `.rs` 差分なし。

## 修正前の再現（no-HWND 経路の local repro）

`win32::popup` が `None` を返す状態（非 Windows stub が恒常に作る状態と
同一）を、実 HWND を take+destroy して `state.popup == None` にすることで
Windows 上に合成し、新 test
`popup_verbs_are_recorded_without_popup_hwnd` を修正前コードで実行:

```
---- editor_view::tests::popup_verbs_are_recorded_without_popup_hwnd stdout ----
assertion `left == right` failed
  left: ["po:1", "po:2", "pc:2", "po:3"]
 right: ["po:1", "pc:1", "po:2", "pc:2"]
test result: FAILED. 108 passed; 1 failed; 1 ignored
```

`state.popup == None` の press が常に open 分岐に入る（close 分岐 dead）
という、非 Windows 4 legs で決定的に fail した機構を Windows 上で再現
確認。同一 test が修正後は PASS（下表）。

## canonical lane（mise タスク — 正規入口）

| lane | 結果 | 備考 |
| --- | --- | --- |
| `mise run fmt` → `mise run fmt-check` | PASS（exit 0） | `cargo fmt --all` + `-- --check`、diff なし |
| `mise run check ruviform-plugin-test-vst3` | PASS（exit 0） | `cargo check --locked --all-targets -p` |
| `mise run lint ruviform-plugin-test-vst3` | PASS（exit 0） | clippy `-D warnings` |
| `mise run test ruviform-plugin-test-vst3` | PASS（exit 0） | 109 passed / 1 ignored — `popup_key_toggles_plugin_owned_window`・`popup_verbs_are_recorded_without_popup_hwnd`・`removed_resets_popup_open_state` 全て ok（Windows leg 非回帰の直接証拠） |
| `mise run test ruviform-plugin-test-vst3 --features l1-runner --lib --bins` | PASS（exit 0） | lib 109 + bins 3 passed — `l1-real-verify` leg の unit test selection 相当・feature 組 compile+test 非回帰 |
| `mise run check-l1-real-plugin` | PASS（exit 0） | l1 lane feature 組 compile gate |
| `mise run lint-l1-real-plugin` | PASS（exit 0） | l1 lane clippy `-D warnings` |
| `mise exec -- prek run --all-files` | PASS（exit 0） | 全 hook |
| lease 外 `git diff` | 空 — 変更は `editor_view.rs`・`docs/discovery/issue-855-popup-key-ci.md`・`evidence/rf855/` のみ | `Cargo.toml`/`mise.toml`/`docs/**/README.md`/`Cargo.lock`/他 `.rs` 差分なし |

## 補助証拠（canonical = mise）

| 項目 | 結果 | 備考 |
| --- | --- | --- |
| `mise exec -- cargo check --target x86_64-unknown-linux-gnu -p ruviform-plugin-test-vst3 --all-targets` | PASS（exit 0） | 非 Windows stub（`win32::popup → None`）・`#[cfg(test)]` module を linux target で cross compile 確認 — 修正後コードが非 Windows cfg で問題なく type-check されることの補助証拠 |

非 Windows lane（verify-macos/ubuntu・l1-real-verify-macos/ubuntu）の
実証は **merge 後 CI に委譲**: ローカルは Windows のみで失敗自体を直接
再現できず、`ci.yml` は workflow_dispatch 専用で push/PR では起動しない
設計のため。新 main SHA の ci run green は AC3 として Coordinator が確認
する（`docs/discovery/issue-855-popup-key-ci.md` Residual 参照）。

## AC 対応

- **AC1**（fail 原因の再現・特定・分類）: 実 regression（非 Windows
  決定的 defect）と分類 — 機構・失敗 legs/assertion・導入元
  `cbdf6d56`/PR #832/#630・daily-ci pass が反証にならない理由を
  `docs/discovery/issue-855-popup-key-ci.md` に記録。上記 local repro
  で機構を実測。
- **AC2**（実 regression 修正）: `popup_toggle` を論理 `popup_open`
  flag 駆動に変更し `po:`/`pc:` が OS 非依存で記録される契約を回復。
  `state.popup` の HWND 専用契約（test :789 `is_some() ==
  cfg!(windows)`）と Windows 実動作は不変 — Windows unit test 全
  green で非回帰を実測。
- **AC3**（main `ci` lane green 復帰 / 既知 Fail 文書化）: 修正 merge
  後の新 SHA の ci run が担う follow-on gate（Coordinator 担当）。
  「失敗原因が既知 Fail として文書化される」側は
  `docs/discovery/issue-855-popup-key-ci.md` が充足。
