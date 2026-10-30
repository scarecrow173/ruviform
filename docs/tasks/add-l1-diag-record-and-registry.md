---
id: "TASK-034Opsj1KiLPMYmnY6yGmq"
type: "task"
status: "done"
title: "T5. l1::diag 診断レコード・DiagRegistry（ring・snapshot・retention・watermark）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/diag.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-121-l1-stderr-drain-tests.md"
    - "docs/test-specs/issue-121-l1-diagnostics-retention-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T5. l1::diag 診断レコード・DiagRegistry（ring・snapshot・retention・watermark）

## Work

`l1/diag.rs` を新規作成し、子スコープ診断レコードと host `DiagRegistry` の型を実装する（`l1/mod.rs` ~5750 行を肥大化させない責務境界）。

- **診断レコード**: `Mutex` 保護の 64 KiB tail 優先 byte ring + `total_read_bytes`/`dropped_bytes` counter + flags（`eof`/`drain_deadline_exceeded`/`unread_bytes_observed`）。`StderrSnapshot{bytes: Vec<u8>, total_read_bytes, dropped_bytes, eof, drain_deadline_exceeded, unread_bytes_observed}` を **同一 lock 内で同一時点に組み立て** — 不変条件 `total_read = bytes.len() + dropped` が常に成立。`bytes` は `Vec<u8>` のまま返す（String 変換しない — stderr は任意バイト列）。`phase`（pre-spawn 検証 / OS spawn / drain 初期化 / mint・handshake）と `cause`/`cleanup_error` の記録面を持つ。
- **`DiagRegistry`**: host bounded retention — 完了した失敗レコード 8 件・retained stderr payload 合計 512 KiB 上限（active capture・snapshot コピー・メタデータは別計上）。上限超過は最古 evict + **`evicted_records: u64` 計数**。retention lock 内で連続採番 `diagnostics_id: u64`（欠番なし・再利用なし・試行前採番禁止）+ `evicted_watermark`/`issued_max` で `DiagnosticQuery::{Found, Evicted, NotFound}`。minted 失敗は handle ベース検索 index を別途持つ。登録は `Err` を返す関数自身が行い `Arc` が registry へ移る（`Shared` drop 後も生存）。
- **契約**: stderr は他 sink へ暗黙 echo しない（retention/query のみが出口）。命名は既存 `arm_retire_overflows`/`DropCollector::overflows` 系語彙と整合確認（spec R-V7）。
- **`#[path]` harness discipline**: `crate::` パス禁止（harness では `crate::` が test crate root を指すため — `tests/l1_proxy_e2e.rs:16-17` の `#[path]` graft）。参照は `ruviform_plugin_core::`/`ruviform_rt::`/`ruviform_observability::`/`libc::`/`windows_sys::`/`std::` 等の **crate 名**か `self::`/`super::` で行う — whitelist ではなく「`crate::` を使わない」が不変条件（T6 の FFI も `libc::`/`windows_sys::` で解決）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`（「stderr 保持の暫定契約」「生成前失敗の diagnostics ID」「registry の所有・受け渡し」節）

## Dependencies

- なし（先行独立タスク — drain 本体は T6）

## Affected Files

- `crates/ruviform-plugin-host/src/l1/diag.rs`（新規）
- `crates/ruviform-plugin-host/src/l1/mod.rs`（`mod diag;` 宣言のみ）

## Done When

- [ ] 診断レコード・`StderrSnapshot`・`DiagRegistry`・`DiagnosticQuery` の型と単体テストが存在する
- [ ] ring 不変条件 `total_read = len + dropped` と watermark 3 値識別が単体テストで pin されている
- [ ] `crate::` パスを含まない（`#[path]` harness 適合）

## Verification

- [ ] `mise run check` exits 0
- [ ] `mise exec -- cargo test -p ruviform-plugin-host` exits 0（diag 単体テストを含む）
