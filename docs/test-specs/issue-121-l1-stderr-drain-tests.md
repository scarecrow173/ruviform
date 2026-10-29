---
id: "TSPEC-034OpmLKqqBA5YuKQkRrIU"
type: "test-spec"
status: "approved"
title: "L1 child stderr drain（Issue #121 — piped stderr・64KiB tail ring・snapshot 不変・500ms grace・heir bounded teardown）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/120"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/diag.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs"
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
    - "docs/test-specs/issue-121-l1-diagnostics-retention-tests.md"
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/adr/issue-121-l1-diagnostics-policy.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-121-diagnostics.md"
    - "docs/designs/issue-121-l1-diagnostics.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 child stderr drain（Issue #121 — piped stderr・64KiB tail ring・snapshot 不変・500ms grace・heir bounded teardown）

## Purpose

pin 全 L1 子（audio dedicated・audio bundled・video dedicated）の stderr 配管・drain・保持・snapshot 契约 — spec R-B3・AC3 と設計「stderr 保持の暫定契約」「採用案 A」節。子 stderr は失敗診断の主な一次情報源であり、drain の bounded 終了は heir/grandchild による無期限ブロック回避の責務境界。drain 実装が別の I/O 形態に置き換わっても、本 spec の契约は維持すべき不変条件。

## Feature

`spawn_scanned` が `.stderr(Stdio::piped())`、drain thread が `Hello` 待ち前から読み始め、64 KiB tail 優先 ring に収集し、子終了観測から `STDERR_DRAIN_GRACE`(500 ms) 以内に自発終了（EOF なら即終了）。snapshot は `{bytes, total_read_bytes, dropped_bytes, eof, drain_deadline_exceeded, unread_bytes_observed}` を同一時点で組み立てる。

## Rules

- **配管範囲（spec R-B3）**: 全 L1 子に piped stderr — audio dedicated・audio bundled・video dedicated を同一機構で扱う。`Stdio::inherit` は使わない。drain/record 初期化失敗時は既存 abort 経路で rollback（partial child・fd リークを残さない）。
- **drain 非待ち（設計）**: drain は診断 consumer を待たず、retention 満杯でも読み捨て継続 — 「読めたが保持できなかった」分を `dropped_bytes` で計数。drain 開始は `Hello` wait より前 — pre-`Hello` 出力を捕捉する。
- **ring 契約（64 KiB tail 優先）**: 新入力で古いバイトを evict（tail 保持）；1 回の read が ring 容量を超えた分は保持せず `dropped_bytes` 計数。**capture 不変条件**: `total_read_bytes = bytes.len() + dropped_bytes` — capture 開始前・deadline 後の未観測出力はいずれにも含まない。
- **snapshot 同一時点**: `bytes`/`total_read_bytes`/`dropped_bytes`/flags を record `Mutex` 内で組み立て — drain 進行中でも buffer と counter の時点混ぜをしない。`bytes` は `Vec<u8>` のまま返す（String 変換しない — stderr は任意バイト列）。
- **deadline（設計承認案 A・`STDERR_DRAIN_GRACE`=500 ms）**: 起算は子終了観測（reap/`try_wait`/converge）。出力継続でも延長しない（延長は heir 無期限保持を再導入）。EOF 到達で即終了。POSIX は `libc::poll` 周期起床、Windows は `windows-sys` `PeekNamedPipe`+短 sleep — 両者とも deadline は「目標」であって厳密な硬保証ではない（OS scheduling 遅延・Windows 同期 I/O 残余ブロックを明示受容）。drain 終了時に read fd を close — 生き残った heir の後続書き込みは error 化する（副作用を明示受容）。JoinHandle は detach（無期限 join しない）。
- **flags 语义**: `eof`（drain が EOF を観測）・`drain_deadline_exceeded`（**EOF 未到達のまま** drain deadline に達し drain を自発終了させた事実 — 未読データの観測有無とは独立。pipe を保持したまま何も書かない heir でも `true` になる）・`unread_bytes_observed`（deadline 時点または最終確認で未読データを実際に観測 — `false` は「欠損なし」でなく「未読を観測していない」）。
- **heir bounded teardown（AC3）**: pipe を保持する heir/grandchild が残っても host teardown は deadline で打ち切られる — POSIX は限定条件下で制御可能、Windows は残余リスクを flag で観測可能にする（「完全に有界」の主張はしない — OS 別の保証強度を分けて記述する）。
- **暗黙 sink なし**: stderr は他 sink（stdout/log/console）へ暗黙 echo しない — retention/query のみが出口。

## Examples

- `--stderr-bytes=128` worker → `Hello` 前出力が snapshot `bytes` に含まれる
- `--stderr-bytes=200000`（pipe/ring 容量超過）→ drain 継続・`dropped_bytes` 精確増加・`bytes` は末尾 64 KiB・`total_read = len + dropped` 成立
- 子正常終了 → EOF で drain 即終了・`eof=true`・deadline 待たず
- `--leave-heir-after-msgs` + `--heir-stderr-delay-ms` → 子 exit 観測から ~500 ms で drain 終了・fd close で heir 後続書き込みは error・POSIX は deadline 達成、Windows は `drain_deadline_exceeded`/`unread_bytes_observed` で残余観測
- record 初期化失敗注入 → child は kill+reap で rollback（partial child 残存なし）

## Guarantees

- drain は consumer 非依存 — query を誰も呼ばなくても出力は読み続けられる
- 500 ms grace は `DRIVER_JOIN_MARGIN`(1 s)・`command_timeout`(10 s)・heir fixture(~15 s) より十分短く teardown bound を維持する
- snapshot は単一時点の原子組み立て — `total_read = len + dropped` が常に成立
- 既存 `Stdio::inherit` 時代の動作（block 処理・通知・ChildExit）に無退行

## Non-goals

- 構造化 logging・stderr 内容の parse・sink 経路（retention/query のみ）
- 「完全に有界」の終了保証 — OS 別の非硬保証を契約の一部として明示する
- `Stdio::null` 化や stderr 抑制 — 全出力を ring 単位で収集する
- heir への kill/終了責任 — heir は rollback の終了対象外（既存契約）

## Risk

- drain を blocking-read のみで実装すると heir が pipe を保持したまま deadline を観測できず teardown が無期限化
- snapshot を「buffer コピー + 別時点 counter 読み」で組み立てると `total_read = len + dropped` が損なわれ欠損検証が誤作動
- fd close を省くと heir の書き込みが成功し続け deadline が実効しない
