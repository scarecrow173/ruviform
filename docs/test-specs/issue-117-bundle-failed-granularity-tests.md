---
id: "TSPEC-034NlQ8vmaETbrprsDYNkH"
type: "test-spec"
status: "proposed"
title: "L1 bundle Failed 粒度 tests（Issue #117 — child/instance scope 判定表・fan-out・Destroying doorbell）"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-117-bundle-e2e-tests.md"
    - "docs/test-specs/issue-117-wire-v2-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 bundle Failed 粒度 tests（Issue #117 — child/instance scope 判定表・fan-out・Destroying doorbell）

## Purpose

bundle 形態（1 子 N instance）における `Failed` 粒度・`InstanceFailed` fan-out・demux 判定表の oracle。子死亡/帰属不能違反 → 全 live instance 収束、帰属可能違反 → 対象 instance のみ収束、registered-but-not-live/post-removal/post-`Shutdown` frame → inert/non-event、per-command `Err(HostError)` → defined rejection の各行を、`converge_child(exit)` vs `converge_instance(i)` の split・notification 意味論・`Destroying` doorbell window と共に pin する。`Dedicated` 経路では本表が 1:1 写像へ退化することも確定事項。`Failed` 粒度の公開契約正本は `docs/adr/issue-117-bundle-child-failed-granularity.md`。

## Feature

reader demux は着信 frame を `instance` の live/registered-but-not-live/post-removal/範囲外で解決し、design L304-320 の判定表に従って「子 scope 収束」「instance scope 収束」「inert drop」「non-event」「defined rejection」のいずれかへ分岐する。`DestroyInstance` doorbell window は `Destroying` gate で新規 submit を `mark_failed` にし、in-flight doorbell の完了を待ってから entry を撤去する。

## Rules

- **子 scope 収束**（`converge_child(exit)`）: 子死亡/kill・EOF・write 失敗・decode 不能 frame・unknown kind・2nd Hello・登録済み child-scope waiter（`Scan` — `instance=NONE`）に解決しない `instance=NONE` の Reply/BlockDone（correlation-first — `Reply{NONE}` が `Scan` waiter に相関するのは正常応答）・`i≥64` の着信・帰属不能 seq/違反・command timeout・reader/driver thread panic → `Inner.terminal = true`・全 live instance `Failed`・全 waiter `Err(Ipc)`・in-flight doorbell `mark_failed`・per-instance `InstanceFailed{handle, exit}` 通知（同一 `exit` fan-out）・thread join+reap
- **instance scope 収束**（`converge_instance(i)`）: live `i` に対する帰属可能違反のみ — 未登録/未知 `(i,seq)` の `BlockDone`・`i` 宛 flagged `Reply`（waiter なし・seq 不整合）→ `InstShared` のみ `Failed` mirror + waiter/doorbell/result 解放 + `InstanceFailed{handle, exit:{None,None}}` 通知・兄弟 instance は生存継続・`Inner.terminal` 等の子 state は変更しない（invariant）
- **inert drop**: registered-but-not-live（`CreateInstance` 送信後 reply 未到達）・`i ∉ instances`（撤去済み・未登録）の着信 → 静黙 drop。違反・`Failed`・通知なし
- **non-event**: `Shutdown` 送信済み後の着信 frame → 正常 teardown として無視（収束不変）
- **defined rejection**: per-command `Err(HostError)` reply（`InvalidHandle`・`UnknownPluginId`・`InstanceLimit` 等）→ その command の `Err` 応答として返すだけで `Failed` 化しない（`kill_child` で意図的に子を殺す以外で `Failed` が発生しないパス）
- **notification 粒度**: 子死亡 → 影響各 live instance へ `InstanceFailed{handle, exit}` が 1 つずつ（N 通知列・`exit` 同値）・per-instance 収束は `ChildExit{None,None}` 1 通知・`Failed` 通知は once（重複なし）・mid-`CreateInstance` 死亡は通知なし（handle 未 mint）・mid-`DestroyInstance` 死亡は通知あり（`Destroy` は `Ok`）
- **`Destroying` doorbell window**: `DestroyInstance{i}` 送信 serialize → entry `Destroying` 化 → 新 submit は `mark_failed` → in-flight doorbell（drain 済み `BlockIn{i,seq}`）は通常完了 → reply 受領で entry 撤去・残 seq は `mark_failed` 解放・wire handle reuse（destroy→create 同一 `i`）時に stale doorbell が新 ring へ届かない
- **dedicated 退化**: `Dedicated` isolation では本表が 1:1 写像へ退化（子死亡 = 単一 instance `Failed`）— 既存 singleton 契約と観測的に同じ
- **no auto-restart**: 収束後の instance/子は再起動しない（新 `Instantiate`/`CreateInstance` が必要）

## Examples

- bundle crash fan-out: `Bundled` route で 2 instance 同一子を kill → 両者 `Failed`・各々 `InstanceFailed{handle, exit}`（`exit` 同値の 2 通知列）・waiter 全 `Err(Ipc)`・その後 `process`/`command` は `Err(Failed)`
- attributable per-instance converge: live `i` へ未知 `(i, seq)` の `BlockDone`（synthetic 注入）→ `i` のみ `Failed`（`InstanceFailed{handle,{None,None}}` 通知）・兄弟は継続 process・`Inner.terminal` は false のまま
- inert drop: registered-but-not-live `i` 宛 frame・撤去済み `i` 宛 frame → `Inner` 非変更・`Failed` なし・通知なし
- post-`Shutdown` non-event: `Shutdown` 送信後の stray `Reply`/`BlockDone` → 正常 teardown として無視・新たな `Failed`/通知なし
- defined rejection: 未登録 `i` 宛 `BlockIn` → `Reply{i,seq,Err(InvalidHandle)}` → `Err` 応答のみ・`Failed` 化なし
- `Destroying` doorbell: `Destroy` 発行中に新 submit → `mark_failed`・drain 済み `BlockIn` の `BlockDone` → 正常処理・`DestroyInstance` reply → entry 撤去・残 seq `mark_failed`・同一 `i` への `CreateInstance`（reuse）で stale doorbell が新 ring に届かない
- mid-`CreateInstance` 死亡 → `Err(Ipc)`・通知なし・entry 撤去（`live_count` リークなし）。mid-`DestroyInstance` 死亡 → entry `Failed` 維持・`InstanceFailed` 通知・`Destroy` `Ok`
- dedicated 非回帰: `isolation: Dedicated` で kill → 単一 instance `Failed`・`InstanceFailed` 1 通知 — 既存 singleton 契約と観測同値

## Guarantees

- demux 判定表の全行（子 scope/instance scope/inert/non-event/defined rejection）が単一関数に集約され、合成 frame unit test + kill E2E で pin される
- `converge_instance(i)` が `Inner.terminal`/`seen_eof`/`shutdown_initiated` 等の子 state を変更しない（invariant を test pin）
- `converge_child(exit)` が影響を受けた全 live instance を同値 `exit` で収束し、waiter/doorbell/notify を完了させる（冪等・先着者が terminal 確定）
- `Destroying` gate が doorbell window（新 submit 拒否・in-flight 完了待ち・reply 撤去・残 seq 解放・handle reuse の stale 排除）を実装する
- notification は per-instance once・fan-out 時は同値 `exit`・per-instance 収束は `{None,None}`・mid-CreateInstance は no-notify・mid-DestroyInstance は notify+`Ok`
- `Dedicated` 経路は本表を 1:1 へ退化させ、既存 singleton 契約と観測的に同じ

## Non-goals

- codec 形式検証（envelope・scope 判定・bounds — `issue-117-wire-v2-codec-tests` が対象）
- pool/isolation 公開面・runner registry（`issue-117-bundle-e2e-tests` が対象）
- instance-scoped fault-injection 語彙の fixture knob（design L388 で deferred — `converge_instance` は合成 demux unit test で代替）
- `BundleFailed` 相関通知 variant（deferred — per-instance fan-out で現契約維持）
- `Dedicated` 経路の spawn/pool 機構（`issue-117-bundle-e2e-tests` が対象 — 本 spec は粒度退化のみ pin）

## Risk

- 判定表の行漏れ → inert frame を違反と誤判定して子を殺す or 真の違反を静黙 drop して waiters が timeout まで宙吊りになる
- `converge_instance` が子 state を誤って変更 → 兄弟 instance が連鎖 `Failed` して bundle の粒度意義が失われる
- `Destroying` gate 未実装 → in-flight doorbell と撤去の race で stale seq が新 instance の ring に混入する
- notification の重複/欠落 → observer が `Failed` を見逃す or 二重処理する
- dedicated 経路で bundle の判定表が漏洩 → `Dedicated` の 1:1 契約が壊れ opt-in 隔離の意味が失われる
