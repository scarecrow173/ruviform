---
id: "TASK-034MrsDzlHRpUHHLtYkZdN"
type: "task"
status: "todo"
title: "plugin-core endpoint 前提面（EndpointUnavailable・fallible encode・Send capability・view impl）"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# plugin-core endpoint 前提面（EndpointUnavailable・fallible encode・Send capability・view impl）

## Work

Plan actions 1–3。`ruviform-plugin-core` に Issue #113 全後続 task が使う 4 つの共有前提を追加する — (a) `HostError::EndpointUnavailable` local-only variant、(b) `encode`/`encode_host_error` の `Result<(), WireError>` 化 + all-or-nothing append、(c) `PluginFactory::create_sendable`/`erase_sendable` の opt-in `Send` capability、(d) `AudioBufferView`/`AudioBufferViewMut` への `BufferView`/`BufferViewMut` impl。

- **`boundary.rs`**: `HostError` に `EndpointUnavailable` variant を追加 + `Display` arm。doc コメントで「**local-only** — wire 経路に乗らない host-local の topology 事実（`Ipc`/`Unsupported` へ潰さない）」を明記。`#[non_exhaustive]` のため non-breaking
- **`wire.rs`**: `encode_host_error` を `Result<(), WireError>` 化し wildcard arm（`EndpointUnavailable` を含む非 wire variant）で `Err(WireError::UnsupportedVariant)` を返す — `encode_transport` の `Reply` 引数網羅の先例（L1260-1269 付近）と同形。`pub fn encode` を `Result<(), WireError>` 化し、エラー時は `out.truncate(start)` で **all-or-nothing append**（`REPLY_ERR` tag 等の partial frame を残さない）。`encode_reply` は `HostError` 不到達のため infallible のまま維持。roundtrip 用手動 variant リスト（wire 表現可能 variant のみ — `EndpointUnavailable` を含めない）と encode ヘルパを `Result` 対応へ更新。negative test: `Reply{Err(EndpointUnavailable)}` encode → `Err(UnsupportedVariant)` かつ既存バイトを持つ `out` が不変
- **`plugin.rs`**: `PluginFactory::create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>>`（default `None` — opt-in）+ `erase_sendable<P: Plugin + Send + 'static>(p: P) -> Box<dyn ErasedPlugin + Send>`。`Erased<P>` は `P: Send` なら `Send` — `ErasedPlugin`/`Plugin` への全域 `Send` bound は置かない（ADR rejected 分岐を守る）
- **`processor.rs`**: `impl BufferView for AudioBufferView<'_, f32>` + `impl BufferView`/`BufferViewMut` for `AudioBufferViewMut<'_, f32>` — channel-major（`channel(i) = &data[i*frames..(i+1)*frames]`）の const 実装。既存 `PlanarBufferView`/`PlanarBufferViewMut` impl（同ファイル）と同じ seam・同じ契約（`channels()`/`frames()`/`channel(i)`）
- **`lib.rs`（plugin-core）**: `erase_sendable` を re-export + crate doc 整合（endpoint 前提面の存在）。**re-export parity**: host 側の `erase` 流通経路（`crates/ruviform-plugin-host/src/plugin.rs` L16 `pub use ruviform_plugin_core::{…, erase}` → `crates/ruviform-plugin-host/src/lib.rs` L107 `pub use plugin::{…}`）に `erase_sendable` も同型で通す — Send-capable factory の構築 helper が plugin-core 直接名指しなしに host 公開面から到達できるようにする
- **call site 更新（signature 変更の波及）**: `wire::encode` → `Result` 化の**全呼出し inventory**（statement-position 呼出しは `Result` 化で `unused_must_use` — `-D warnings` ゲートで全件 fail するため網羅必須）: (i) 4 本の statement-position 呼出し — `crates/ruviform-plugin-host/src/l1/mod.rs` `write_message`（L421）・`crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` `write_message`（L160）・`crates/ruviform-plugin-export/src/lib.rs` `send`（L794）・`crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` `send`（L125）— `io::Error` 写像は `decode` 側の `InvalidData` 形と揃える; (ii) `wire.rs` 内の compile-tested doctest 3 件（L239・L641・L717 — 全て `/// ```` ブロックで `cargo test --doc` 対象 — `unwrap`/`?`/`let _ =` 等の `Result` 対応へ）; (iii) `wire.rs` inline `#[cfg(test)]` の `roundtrip`/`field_roundtrip` ヘルパと `encode` 直接呼出し一式（同じく statement-position — `Result` 対応 + `unwrap`）
- **単体テスト**: host-error roundtrip（全 wire 表現可能 variant — `EndpointUnavailable` を含めない）・`EndpointUnavailable` encode 拒否 + `out` 不変・`create_sendable` default `None` + Send fixture で `Some`・`erase_sendable` が `Box<dyn ErasedPlugin + Send>` を返す・両 view impl の `channel(i)` オフセット正当性・`BufferViewMut` 書込みが元 buffer に反映

## Classification

- `normal-plan-task`（新規機構・共有前提 — 後続 task が全てこれに依存）

## Upstream Documents

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（actions 1–3・Files/Ownership）
- `docs/designs/engine-plugin-host-integration.md`（決定サマリ・「EndpointUnavailable local-only」・「encode の Result 化 + truncate」・`create_sendable`/`erase_sendable`・view impl の pinned 形）
- `docs/specs/engine-plugin-host-integration.md`（AC4・AC5 — bounds/error 面の前提）
- `docs/adr/plugin-rt-ownership-split.md`（opt-in `Send` capability — 全域 bound 却下）

## Dependencies

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（本計画の起点 task — 他 task 前提なし）

## Affected Files

- `crates/ruviform-plugin-core/src/boundary.rs` — `HostError::EndpointUnavailable` + `Display` arm + local-only doc
- `crates/ruviform-plugin-core/src/wire.rs` — `encode_host_error`/`encode` の `Result` 化 + `truncate(start)` + variant リスト/roundtrip ヘルパ + negative test + **compile-tested doctest 3 件（L239・L641・L717）の `Result` 対応**（statement-position `unused_must_use` — `-D warnings` で fail）
- `crates/ruviform-plugin-core/src/plugin.rs` — `create_sendable`（default `None`）+ `erase_sendable<P: Plugin + Send>`
- `crates/ruviform-plugin-core/src/processor.rs` — `AudioBufferView`/`AudioBufferViewMut` への `BufferView`/`BufferViewMut` impl
- `crates/ruviform-plugin-core/src/lib.rs` — `erase_sendable` re-export + crate doc
- `crates/ruviform-plugin-host/src/l1/mod.rs` — `write_message`（L421）の `encode` call site を `Result`→`io::Error` 写像へ更新
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` — `write_message`（L160）の同更新
- `crates/ruviform-plugin-export/src/lib.rs` — `send`（L794）の同更新
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` — `send`（L125）の同更新（test helper — statement-position 呼出し）
- `crates/ruviform-plugin-host/src/plugin.rs` — `pub use ruviform_plugin_core::{…}` 行へ `erase_sendable` 追加（`erase` と同じ parity 経路）
- `crates/ruviform-plugin-host/src/lib.rs` — `pub use plugin::{…}` 行へ `erase_sendable` 追加
- `crates/ruviform-plugin-core` 内 inline `#[cfg(test)] mod tests` — 新規単体テスト

## Done When

- [ ] `HostError::EndpointUnavailable` が local-only variant として存在し、`Display` arm を持ち、wire 表現可能 variant リストに含まれない
- [ ] `encode`/`encode_host_error` が `Result<(), WireError>` を返し、`Reply{Err(EndpointUnavailable)}` encode が `Err(UnsupportedVariant)` かつ `out` が `truncate(start)` で呼出し前に不変
- [ ] `encode_reply` が infallible のまま — 全 `wire::encode` 呼出し inventory が `Result` 対応（4 call site: `l1/mod.rs` L421・`l1-test-worker.rs` L160・export `send` L794・`l1_runner_e2e.rs` `send` L125・wire.rs doctest 3 件・inline test helper — statement-position `unused_must_use` 残留なし）
- [ ] `PluginFactory::create_sendable`（default `None`）と `erase_sendable` が存在し、Send fixture が `Box<dyn ErasedPlugin + Send>` を返す・非 Send 契約は変わらない（全域 bound なし）・`erase_sendable` が `ruviform-plugin-host` の `plugin.rs`→`lib.rs` pub-use 経路でも到達可能（`erase` と parity）
- [ ] `AudioBufferView`/`AudioBufferViewMut` が `BufferView`/`BufferViewMut` を実装し、`channel(i)` の channel-major オフセットが単体テストで固定される
- [ ] narrow gate（下記 Verification）が exit 0 — 既存 wire roundtrip/テストに無退行

## Verification

- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0
- [ ] `mise run test` — exit 0（既存 wire roundtrip・host 側テストの無退行 + 新規単体テスト + wire.rs doctest 3 件の `Result` 対応）
- [ ] `mise run test-plugin-core`（core 集中の既存レーン）— exit 0・`EndpointUnavailable` encode 拒否と `out` 不変を pin
- [ ] `mise run lint-plugin-core`（core 集中 lint の既存レーン）— exit 0・`unused_must_use` 残留なし（call site/doctest/helper 全網羅の実証）
- [ ] 対象 test-spec: `docs/test-specs/plugin-endpoint-contract-tests.md` — 規則「`EndpointUnavailable` は local-only variant」「defined 拒否表」の前提面（本 task では wire/型契約のみ — endpoint 機構自体は T2/T3/T6）
