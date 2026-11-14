---
id: "IMPL-034XqUtbQR6b9Xie900zZv"
type: "impl"
status: "draft"
title: "Issue #625 ci e2e handshake race implementation record"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/625"
    - "https://github.com/scarecrow173/ruviform/actions/runs/43540652567"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "docs/specs/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "docs/designs/issue-625-ci-e2e-handshake-race-design.md"
      - type: "file"
        path: "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
      - type: "file"
        path: "docs/tasks/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "evidence/rf625/root-cause.md"
      - type: "file"
        path: "evidence/rf625/verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/issue-625-ci-e2e-handshake-race.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-625-ci-e2e-handshake-race.md"
    - "docs/designs/issue-625-ci-e2e-handshake-race-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-582-ci-lane-failures.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-625-ci-e2e-handshake-race.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/actions/runs/43540652567"
    - "https://github.com/scarecrow173/ruviform/pull/619"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #625 ci e2e handshake race implementation record

## Summary

run 43540652567 の `verify (ubuntu-latest)` 失敗 — `e2e_process::e2e_disable_env_kills_active_transport` が `expected handshake refusal, got Io(BrokenPipe)` で panic — の root cause を修正した。「welcome 前に control channel が閉じた」同一条件が `TestDriver::handshake` の hello 交換で write 側 `Io(BrokenPipe)`/read 側 `Handshake(Eof)` に OS・timing 依存で分かれていた分類 race を、分類を所有する `handshake()` で正規化し、channel-death 系 transport error を一意に `DriverError::Handshake`("control channel closed before welcome: {e}")へ写像した。child 側・e2e assertion・`DriverError` variant 集合は不変。

## Context

`docs/specs/issue-625-ci-e2e-handshake-race.md`(approved)→ `docs/designs/issue-625-ci-e2e-handshake-race-design.md`(approved)→ `docs/plans/issue-625-ci-e2e-handshake-race-plan.md`(approved)→ `docs/tasks/issue-625-ci-e2e-handshake-race.md` の doc-chain に従い、T1 として実装。一次ログ・code path・OS 差異の分析は `evidence/rf625/root-cause.md`、検証台帳は `evidence/rf625/verification.md`。

## Changes

### `crates/ruviform-ui-test/src/driver.rs`

- **`hello_io` helper(新規 private fn)**: hello 交換中の transport io error を `DriverError` に写像。`e.kind()` が `BrokenPipe`/`ConnectionReset`/`ConnectionAborted`/`UnexpectedEof` のいずれか、または `child.try_wait()` が exit を確認できた場合(unmapped errno kind の fallback)は `Handshake("control channel closed before welcome: {e}")`、それ以外は `DriverError::Io` を維持。`try_wait` は error 経路でのみ呼ばれる。
- **`handshake()`**: hello `write_frame`/`read_frame` の `?` 伝播を `map_err(|e| hello_io(child.as_mut(), e))` に置き換え。`child` を `mut` 化。`FrameRead::Eof` → `Handshake` の既存経路と message prefix("control channel closed before welcome")は不変。`FrameRead::TooLarge`/bad json/bad welcome/`ok:false` の各 `Handshake` 経路も不変。
- **`spawn_tcp` accept loop**: WouldBlock 以外の accept error でも `child.try_wait()` で exit を確認できれば、WouldBlock 経路と同じ `Handshake("app exited before TCP handshake")` を返す(従来は無条件に kill + `Io` 伝播 — stdio 側との分類非対称を解消)。
- **`#[cfg(test)] mod tests`(新規)**: fake `FailWriter`/`FailReader`/`io::empty`/`io::Cursor`/`io::sink` と exit 済み child fixture で error 経路を決定的に再現する unit test 7 件。`handshake` は private のため同一 module 内で直接呼ぶ。

変更しなかったもの: `dummy_app.rs`/`serve_from_env`/`agent.rs`/`transport.rs`/`e2e_process.rs` の assertion/`DriverError` variant 集合/post-handshake `call()` の `Io`/`Disconnected` 区別/他 crate/workflow/Issue #625 管理 state。

## Decisions During Implementation

- **kind 集合 + child-exit fallback の 2 段判定**: channel-death を意味する `ErrorKind` 4 種を明示 match し、errno mapping が名前を持たない OS error は `child.try_wait()` の exit 確認でのみ `Handshake` 化。`InvalidData` 等の非 channel error は child 未 exit なら `Io` のまま — 異常を握り潰さない。
- **message は prefix 共有**: `"control channel closed before welcome: {e}"` とし、`e2e_process.rs` の "closed before welcome" assertion と `looks_environment_blocked`(ruviform-ui-gpui)の "control channel closed" 判定を両立。
- **`spawn_tcp` accept error も同一方針**: child exit 確認時の `Handshake` 化は WouldBlock 経路の既存契約と同じ message を再利用 — 新しい分類を発明しない。
- **child 側・assertion は不変**: disable → serve 拒否 → exit 0 は仕様どおり。variant 受容拡大は分類 nondeterminism の隠蔽のため不採用。

## Related Experiments

### Adopted

- なし(実験 log なし — 設計判断は design doc の Risks/Trade-offs に記録)。

### Rejected

- **assertion を `Handshake`/`Io` 両 variant 許容へ緩和** — 分類 nondeterminism を隠すだけで契約を修復しない。spec Non-goals・タスク禁止事項に抵触。
- **child 側(`dummy_app`)で exit を遅延/制御** — disable 経路の exit 0 は仕様どおりであり、test fixture を遅くするのは race を隠す方向。
- **`DriverError` に新 variant 追加** — 公開 API 変更であり、既存 `Handshake` message 契約で十分表現できる。

## Validation

`evidence/rf625/verification.md` の台帳が正本(実行したもののみ Pass と記録)。

- `mise run fmt-check` → exit 0(Pass)
- `mise run check ruviform-ui-test` → exit 0(Pass)
- `mise run lint ruviform-ui-test` → exit 0(Pass、`-D warnings`)
- `mise run test ruviform-ui-test --test e2e_process` → exit 0(Pass、18 test)
- `mise run test ruviform-ui-test` → exit 0(Pass、新規 unit test 7 件含む)
- `mise run test-full` → 実行結果は verification.md に記録
- NotRun: ubuntu 固有 EPIPE 実機再現・macOS/実 window/RT device lane・`*-msrv` — 理由は verification.md §3。統合検証は merge 後対象 SHA の CI(Coordinator 確認)。

## Risks

- **unmapped errno kind の残余 race**: `ErrorKind` が名前を持たない OS error かつ child が zombie 化前の極小 window では `Io` が残り得る。観測済み EPIPE は kind match で確定的に覆われる(design Risks 記録済み)。
- **stdio `--stall before-serve` の handshake hang**: read に timeout がない既存 latent 問題。本 run では顕在化せず、別 Issue 相当として design Deferred Design Concerns に記録のみ。
- **`try_wait` の reap**: exit 確認で child が reap されても `Child` が status を cache するため後続の `wait_exit`/`Drop` に影響しない。

## Follow-ups

- merge 後の対象 SHA の `ci` run で `verify (ubuntu-latest)` を含む全必須 job success を確認し、ci-issue-tracker が Issue #625 を auto-close する( Coordinator が実施)。
- stdio handshake への deadline 付与は別 Issue 候補(design Deferred Design Concerns)。
