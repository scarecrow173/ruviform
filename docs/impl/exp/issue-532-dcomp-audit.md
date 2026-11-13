# Issue #532 DComp 監査文書チェーン — 監査手法の試行記録（2026-11-14）

`docs/tasks/issue-532-dcomp-audit-task.md` の文書監査で試行した手法と結果の記録。コード変更を伴わないため、実験は「監査手法の選択と補正」に限定される。正本の監査結果は
[`docs/discovery/issue-532-dcomp-native-dock.md`](../../discovery/issue-532-dcomp-native-dock.md)
を参照。

## 試行 1: `cargo metadata` の dep_kinds 解釈（既存手法の再利用確認）

- **目的**: Issue #192 の TA 監査で確立した `dep_kinds` 解釈手法が、本監査の依存境界確認にそのまま使えるか確認する。
- **試行**: `crates/ruviform-ui*` の `Cargo.toml` を読み、`ruviform-ui`（中立契約）→ `ruviform-ui-test`（fake/test）→ `ruviform-ui-gpui`（adapter）→ `ruviform-poc-native-dock`（consumer PoC）の依存方向を確認。
- **観測**: `ruviform-ui` は GPUI/OS handle を含まない中立契約。`ruviform-ui-test` は `ruviform-ui` に依存し fake を提供。`ruviform-ui-gpui` は `ruviform-ui` + `gpui`（optional feature）に依存。`ruviform-poc-native-dock` は consumer composition root として `ruviform-ui-gpui` + `ruviform-plugin-host` を結線。依存方向は `plugin`/`ui` 相互依存禁止の baseline に整合。
- **結論**: 依存境界の確認に `dep_kinds` の詳細解析は不要（manifest の直接読みで十分）。採用（discovery F2 の層構造に反映）。

## 試行 2: CI 証拠 SHA と監査 HEAD の差分評価

- **目的**: 直近の全量 CI run `43428172820`（対象 SHA `e93c438a`）の証拠を、監査 HEAD `8944a360` に対してどの程度代表的に対応付けられるか評価する。
- **試行**: `git diff e93c438a..8944a360 -- crates/ruviform-ui crates/ruviform-ui-test crates/ruviform-ui-gpui crates/ruviform-poc-native-dock` で差分を確認。
- **観測**: `native_slot.rs`/`fake_native_slot.rs`/`native_slot_contract.rs`/`driver_contract.rs` は無変更。`surface.rs` は doc comment の語彙 rename（`GuiParentToken`→`ParentLease`、PR #609）のみで挙動に影響しない。`poc` 配下と `ui-test` に軽微な変更（`e2e_process.rs` 追加・`dummy_app.rs` 追加）があるが契約層の証拠には影響しない。
- **結論**: e93c438a の CI 証拠は契約層について現行 HEAD に対しても代表的。ただし同一 SHA の新規 run は本 slice で未取得のため、discovery に「Run URL 収集は Coordinator 責務」と明記。採用（discovery F3・Key Issues に反映）。

## 試行 3: Issue 依存状態の確認方法

- **目的**: Issue #532 が他 Issue の Blocked by として誤登録されていないか確認する。
- **試行**: `gh issue view 532 --json body` で本文の「着手条件（Blocked by）」を確認。関連 Issue（#388/#389/#262/#419/#313/#243/#465）の状態を `gh issue view` で確認。
- **観測**: #532 本文の「着手条件（Blocked by）」は「なし」。関連 Issue はいずれも open で、#532 への依存登録は見当たらない。#465（Legacy WebView 撤去）が #532 を未解決 Windows DComp 制約として追跡している記述は残るが、これは「Blocked by」ではなく「未解決制約のトラッキング」。
- **結論**: Blocked by 誤登録は観測されず、除外提案の実作業は不要。根拠のみを discovery F4 に記録。採用。

## 試行 4: 文書生成スクリプトの出力先

- **目的**: `new_discovery.js`/`new_spec.js`/`new_design.js`/`new_plan.js`/`new_task.js`/`new_test_spec.js`/`new_impl_record.js`/`new_experiment_log.js` で skeleton を生成し、所定のファイル名に rename する。
- **試行**: 各 script を `--dir`/`--name`（対応するもの）で実行。`new_impl_record.js`/`new_experiment_log.js` は `--name` を取らず title から slug を生成するため、生成後に `mv` で所定名に rename。
- **観測**: `new_impl_record.js` は `issue-532-dcomp-audit-doc-chain.md` を生成（title slug）。`new_experiment_log.js` は `issue-532-dcomp.jsonl` を生成（jsonl 形式）。所定の `docs/impl/exp/issue-532-dcomp-audit.md` は `.md` 形式の試行記録として手動作成（issue-192 の precedent に倣う）。誤って生成した `issue-532-dcomp.jsonl` は削除。
- **結論**: skeleton 生成 → 内容記述 → rename/手動作成の流れを採用。`docs/impl/exp/` の `.md` は試行記録、`.jsonl` は実験ログの raw data として区別。
