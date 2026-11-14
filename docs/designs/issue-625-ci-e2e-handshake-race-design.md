---
id: "DESIGN-034XqAh1OycqOAK35pzCLQ"
type: "design"
status: "approved"
title: "Issue #625 設計 — handshake hello 交換の transport error を Handshake 分類へ正規化"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/625"
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-625-ci-e2e-handshake-race-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/designs/README.md"
  implements:
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/actions/runs/43540652567"
  defers: []
  deferred-by: []
---

# Issue #625 設計 — handshake hello 交換の transport error を Handshake 分類へ正規化

## Context

`TestDriver`(crates/ruviform-ui-test/src/driver.rs)は app-under-test との `rvf.uitest.v1` control channel を張り、`handshake()` で `test.hello` → `Welcome` を行う。hello 交換の失敗分類は現状 2 系統に分かれている:

- read 側の clean EOF(`FrameRead::Eof`)→ `DriverError::Handshake("control channel closed before welcome")`
- write/read の io error(`write_frame`/`read_frame` の `?` 経由)→ `DriverError::Io`

「peer が welcome 前に channel を閉じた」という同一の終了条件が、OS・timing により write 側 EPIPE(Linux pipe で reader end 消失)・TCP reset・read 側 EOF・frame 途中切断のいずれかで観測される。run 43540652567 では ubuntu-latest で write 側 `Io(BrokenPipe)` が観測され、test が期待する `Handshake` と不一致で panic した。windows-latest では同 test が pass — 分類が観測経路依存であることの実証。

child 側(`dummy_app` の `RVF_TEST_DISABLE` → `ServeOutcome::Disabled` → exit 0)は仕様どおりであり、修正対象ではない。分類を所有するのは driver 側 `handshake()` である。

## Scope

### In Scope

- `handshake()` 内の hello write/read で発生する transport io error の分類正規化。
- `spawn_tcp` accept loop の accept error 経路への同一方針(child exit 確認時の `Handshake` 化)の適用。
- 分類契約を固定する driver 内 unit test。

### Out of Scope

- `call()` 等 post-handshake 経路(`Io`/`Disconnected` の区別は session 中は意味を持つ)。
- `DriverError` variant 集合・公開 API の変更。
- child 側 serve/agent の挙動、e2e test の assertion。
- handshake read の timeout 追加(stdio stall-before-serve の hang は別問題 — evidence に記録のみ)。

## Components and Boundaries

- **`TestDriver::handshake`**(driver.rs): hello 交換の唯一の分類境界。write_frame/read_frame の io error をここで `Handshake`/`Io` に写像する。
- **`hello_io` helper**(新規 private fn): 「channel が閉じた」判定を 1 箇所に集約。error kind 判定 + child exit 確認の fallback。
- **`spawn_tcp` accept loop**: accept error 時の child exit 確認を WouldBlock 経路と揃える。
- **`FrameRead::Eof` → `Handshake`**: 既存経路は不変(同一 message を共有)。

## Data and Control Flow

`handshake()` の hello 交換で io error が発生した場合:

1. `e.kind()` が `BrokenPipe` / `ConnectionReset` / `ConnectionAborted` / `UnexpectedEof` のいずれか → peer の channel 端が消えた確定的証拠 → `Handshake("control channel closed before welcome: {e}")`。
2. 上記以外の kind でも `child.try_wait()` が exit を確認できた場合 → errno mapping が名前を持たない OS error(EIO 等)でも同一条件として `Handshake`。`try_wait` は error 経路でのみ呼ばれ、child 生存時は `Io` を維持する。
3. いずれでもなければ `DriverError::Io` を維持 — 異常な transport error を握り潰さない。

`spawn_tcp` accept loop の非 WouldBlock error でも `child.try_wait()` で exit を確認できれば既存の `"app exited before TCP handshake"` `Handshake` を返し、child kill + `Io` 伝播は従来どおり。

## Non-Functional Constraints

- **RT contract**: non-RT only(変更なし)。
- **lint**: `workspace.lints` 準拠、`unsafe` 不使用、`#[non_exhaustive]` 済み enum の変更なし。
- **後方互換**: `DriverError` variant 集合・message prefix("control channel closed before welcome" / "app exited before TCP handshake")を維持 — `looks_environment_blocked`(ruviform-ui-gpui tests)と e2e assertion が依存する文字列を壊さない。

## Risks and Trade-offs

- **過剰分類のリスク**: kind 集合を channel-death に限定し、fallback は child exit 確認済みに限定することで、local harness 起因の異常 error を `Handshake` に見せかけない。`InvalidData`(非 UTF-8 frame)等は `Io` のまま。
- **try_wait の reap**: exit 確認で child が reap されても `std::process::Child` が status を cache するため後続の `wait_exit`/`Drop` に影響しない。
- **残余 race**: unmapped errno kind かつ child がまだ zombie 化していない極小 window では `Io` が残り得る — 観測済み errno(EPIPE=BrokenPipe)は kind match で確定的に覆われるため実害は限定的。
- **stdio stall-before-serve hang**: handshake read に timeout がなく、channel を開かない child では無期限 block し得る既存問題は本件と別 — 本修正では触れず evidence/PR に記録する。

## Deferred Design Concerns

- handshake 全体への deadline 付与 — | read block の無期限化は別の latent 問題 | stall 系 fixture 拡充や timeout 要件の Issue 化時 | hang による CI 遅延の再発

## Verification Notes

- [x] CI 一次ログ(run 43540652567)で panic 箇所・error 値・他 test 結果を確認済み
- [x] `2613b238..origin/main`(5 commit)が `crates/ruviform-ui-test/` 未触であることを確認済み
- [x] driver unit test + e2e_process lane + test-full のローカル検証(Pass — `evidence/rf625/verification.md` に記録。ubuntu 固有 EPIPE 実機再現は NotRun、統合検証は merge 後 CI)
