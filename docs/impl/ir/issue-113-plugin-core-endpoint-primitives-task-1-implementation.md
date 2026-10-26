---
id: "IMPL-034MuVj3LeNgeJ60Yn4MvU"
type: "impl"
status: "completed"
title: "Issue #113 plugin-core endpoint primitives — Task 1 implementation"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-113-plugin-core-endpoint-primitives-task-1-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/processor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/plugin.rs"
      - type: "file"
        path: "docs/tasks/plugin-core-endpoint-primitives.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-113-engine-plugin-integration.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/plugin-core-endpoint-primitives.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #113 plugin-core endpoint primitives — Task 1 implementation

## Summary

Issue #113 Task 1 完了 — plan actions 1–3（plugin-core の endpoint
前提面 4 件）を全範囲実施。(a) `HostError::EndpointUnavailable`
local-only variant（`Display` arm + wire 不載の doc 明記 —
`Ipc`/`Unsupported` へ潰さない）、(b) `encode`/
`encode_host_error` の `Result<(), WireError>` 化 +
`encode_body` 抽出による **function-boundary rollback**（全 Err
exit が `encode` 単一点で `out.truncate(start)` — all-or-nothing
append）、(c) `PluginFactory::create_sendable`（default `None`
opt-in）+ `erase_sendable`（`Erased<P>: Send` iff `P: Send` —
`Plugin`/`ErasedPlugin` への全域 `Send` bound なし、ADR
rejected 分岐を遵守）、(d) `AudioBufferView`/`AudioBufferViewMut`
への `BufferView`/`BufferViewMut` impl（channel-major slab —
`PlanarBufferView*` impl と同じ seam）。call site 4 件
（statement-position `unused_must_use` は `-D warnings` で全件
fail するため網羅必須）+ wire.rs doctest 3 件 + inline test
helper の `Result` 対応 + `erase_sendable` の host 側 re-export
parity（`plugin.rs`→`lib.rs` — `erase` と同型経路）。独立レビュー
2 件: A=APPROVE・B=APPROVE-WITH-FIXES — 確定修正 4 件を全て適用。
全 gate green 実 log。

## Context

approved plan `docs/plans/implement-issue-113-engine-plugin-integration.md`
の Task 1（`docs/tasks/plugin-core-endpoint-primitives.md`・
actions 1–3）。upstream: approved design
`docs/designs/engine-plugin-host-integration.md`（決定サマリ —
`EndpointUnavailable` local-only・`encode` の `Result` 化 +
`truncate`・`create_sendable`/`erase_sendable`・view impl の pinned
形）・spec `docs/specs/engine-plugin-host-integration.md`
（AC4・AC5）・ADR `docs/adr/plugin-rt-ownership-split.md`
（opt-in `Send` capability — 全域 bound 却下）・TSpec
`docs/test-specs/plugin-endpoint-contract-tests.md`（proposed —
「`EndpointUnavailable` は local-only variant」「defined 拒否表」
の前提面）。本 task は全後続の共有前提 — Task 2（L0 Shared
スロット + endpoint inner）・Task 3（L1 leg 移出 + arm/返却経路）
が新契約を消費する。後続 task は全て未着手。

## Implementation

### Changes

- `crates/ruviform-plugin-core/src/boundary.rs`（+20 行）:
  - `HostError::EndpointUnavailable` variant 追加 —
    `#[non_exhaustive]` enum への追加のため non-breaking。
    **local-only** — wire 経路に乗らない host-local の topology
    事実（factory が `create_sendable` を opt-in していない・
    endpoint 既 attached・L1 processing leg が checkout 中）。
    `wire::encode`/`encode_host_error` が
    `Err(WireError::UnsupportedVariant)` で拒否し `Ipc`/
    `Unsupported` へ潰さない（transport 失敗でも plugin 由来の
    decline でもない）旨を variant doc に明記。
  - L1-runner consequence doc（レビュー修正 — 後述）: L1 child
    runner 内の `Plugin` hook が `EndpointUnavailable` を返すのは
    contract violation — defined outcome は上記
    `UnsupportedVariant` 拒否 + runner の child 終了で host は
    `Ipc`/`Failed` を観測する、と明記。
  - `Display` arm: `"RT endpoint is unavailable for this
    instance"`。
- `crates/ruviform-plugin-core/src/wire.rs`（+184/-49 行）:
  - `pub fn encode` → `Result<(), WireError>`。**rollback を
    function boundary に集約**（レビュー修正 — 後述）: 本体を
    private `fn encode_body` へ抽出し、`encode` は length
    placeholder を emit 後に `encode_body(message, out)` を呼び、
    `Err` なら `out.truncate(start)` で **all-or-nothing
    append**（dangling `REPLY_ERR` tag 等の partial frame を残さ
    ない）→ `Err` 返却、成功なら `payload_len` backpatch して
    `Ok(())`。`encode_body` の全 `Err` exit が `encode` 単一点の
    rollback を通る構造 — 将来の fallible arm 追加でも rollback
    漏れが起きない。
  - `encode_host_error` → `Result<(), WireError>`: 全 wire 表現
    可能 variant（tag 0–8）を `Ok(())` arm に、wildcard arm
    （`EndpointUnavailable` を含む非 wire variant・将来の
    `#[non_exhaustive]` 追加 variant）で
    `Err(WireError::UnsupportedVariant)` — `encode_transport`
    の `Reply` 引数網羅の先例と同形。呼出し単位の rollback
    （`let start = out.len()` → 失敗時 `truncate(start)`）で
    「nothing is appended on failure」を per-call で保証。
  - `encode_body` 内の `Reply{Err(error)}` arm は
    `encode_host_error(error, out)?` で伝播 — 現時点で唯一の
    fallible step。`encode_reply` は `HostError` 不到達のため
    infallible のまま維持。
  - doc: `encode` の `# Errors` 節（`UnsupportedVariant` —
    `Message::Reply` の `Err` が local-only
    `EndpointUnavailable` または post-revision variant を運ぶ
    場合・`out` 不変）+ all-or-nothing append の説明 +
    「encoding never fails on **wire-representable** values」への
    文言修正（doc drift レビュー修正 — 後述）。
    `encode_host_error` に `# Errors` 節追加。
  - doctest 3 件（`Message` L235・`encode` L650・`decode`
    L747 付近の ` ``` ` block — `cargo test --doc` 対象）を
    `.unwrap()` 追従で `Result` 対応。
  - inline `#[cfg(test)]`: `roundtrip` ヘルパと direct `encode`
    呼出し一式を `.unwrap()` 追従。wire 表現可能 variant リスト
    2 件（`reply_roundtrip_carries_every_host_error`・
    `host_error_field_roundtrip`）に `EndpointUnavailable` を
    意図的に含めない旨の comment 追加。
  - `host_error_field_roundtrip` は infallible 型の汎用
    `field_roundtrip` ヘルパを使わず `encode_host_error` を直接
    駆動する形へ（deviation — 後述）。
  - 新規テスト `encode_rejects_endpoint_unavailable_and_restores_out`:
    (i) `encode_host_error(&EndpointUnavailable)` →
    `Err(UnsupportedVariant)` かつ payload 空のまま、(ii)
    `Reply{Err(EndpointUnavailable)}` の `encode` →
    `Err(UnsupportedVariant)` かつ pre-existing bytes `[0xAA,
    0xBB, 0xCC]` を持つ `out` が **byte-identity で不変**、
    (iii) 拒否後に `Ok` reply を encode すると先頭 bytes 無傷で
    complete frame が append・decode 成功 — rollback が後続
    append を汚さないことまで pin。
- `crates/ruviform-plugin-core/src/plugin.rs`（+156 行）:
  - `PluginFactory::create_sendable(&self) -> Option<Box<dyn
    ErasedPlugin + Send>>` — default `None`（opt-in: `Send`
    instance を作れない factory は未実装のまま host-side
    surface のみ駆動し、endpoint 系 consumer は
    `EndpointUnavailable` で拒否）。`Some` を返す factory は
    `create()` と descriptor・挙動等価の instance を構築し
    なければならない旨を doc に明記（`create_sendable`
    equivalence doc — レビュー修正）。
  - `erase_sendable<P: Plugin + Send + 'static>(p) -> Box<dyn
    ErasedPlugin + Send>` — `Erased<P>` は `P: Send` なら
    `Send` なので capability は opt-in のまま（`Plugin`/
    `ErasedPlugin` に全域 `Send` bound を置かない — ADR
    rejected 分岐どおり）。
  - inline `#[cfg(test)] mod tests` 新設（4 tests）:
    `SendGain`（minimal `Send` plugin fixture）・`PlainFactory`
    （`create_sendable` 未実装）・`SendFactory`（opt-in）。
    `create_sendable_defaults_to_none`（default `None`）・
    `create_sendable_returns_send_erased_plugin`（`expect_send`
    compile-time `Send` witness + descriptor 等価）・
    `erase_sendable_boxes_a_send_erased_plugin`（`Box<dyn
    ErasedPlugin + Send>` 返却 + `erase()` の非 `Send` 契約不変
    pin）・`send_erased_plugin_drives_the_erased_surface`
    （`prepare`/`process_host` が erased 面を駆動）。
- `crates/ruviform-plugin-core/src/processor.rs`（+111 行）:
  - `impl BufferView for AudioBufferView<'_, f32>`・`impl
    BufferView for AudioBufferViewMut<'_, f32>`・`impl
    BufferViewMut for AudioBufferViewMut<'_, f32>` —
    `PlanarBufferView*` impl と同じ seam の const 委譲実装。
    channel-major: `channel(i) = &data[i*frames..(i+1)*frames]`。
  - module doc・`BufferView` trait doc の「canonical
    implementors」記述を `PlanarBufferView*`（per-channel slice
    list）+ `AudioBufferView*`（channel-major slab）の 2 系統へ
    整合（doc drift 修正 — 後述）。
  - テスト 2 件: `audio_buffer_view_satisfies_buffer_view_channel_major`
    （2ch×4f slab で `channels`/`frames`/`channel(i)` offset と
    out-of-range `None`）・`audio_buffer_view_mut_writes_reach_the_slab`
    （`channel_mut` 書込みが元 slab に反映 + `view_mut` 経路）。
- `crates/ruviform-plugin-core/src/lib.rs`（+27/-10 行）:
  `erase_sendable` をルート re-export へ追加 + crate doc 整合
  （view 2 系統・`create_sendable`/`erase_sendable` opt-in・
  `EndpointUnavailable` は local-only で wire を渡らない旨）。
- call site 4 件の `Result` 対応（signature 変更の波及 —
  statement-position `unused_must_use` は `-D warnings` で fail）:
  - `crates/ruviform-plugin-host/src/l1/mod.rs` `write_message`
    （L421）— `.map_err(|e| io::Error::new(io::ErrorKind::
    InvalidData, e))?`（`decode` 側の `InvalidData` 写像と同形）
  - `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`
    `write_message`（L160）— 同上
  - `crates/ruviform-plugin-export/src/lib.rs` `send`（L794）—
    同上
  - `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` `send`
    （L125）— `.expect("encode message")`（deviation — 後述）
- `erase_sendable` re-export parity（`erase` と同型経路）:
  - `crates/ruviform-plugin-host/src/plugin.rs` — `pub use
    ruviform_plugin_core::{…, erase_sendable}` へ追加
  - `crates/ruviform-plugin-host/src/lib.rs` — `pub use
    plugin::{…, erase_sendable}` へ追加（Send-capable factory の
    構築 helper が plugin-core 直接名指しなしに host 公開面から
    到達可能）

### Review findings and resolutions（独立レビュー 2 件 — A=APPROVE・B=APPROVE-WITH-FIXES・確定修正 4 件全適用）

- **function-boundary rollback（fixed）**: 初版は `encode` 本体が
  fallible arm 個所で `truncate(start)` + `return Err` を内職して
  いた — 将来の fallible arm 追加時に rollback 漏れし得る構造。
  本体を private `encode_body` へ抽出し、`encode` が Err
  propagation を単一点で受けて rollback する形へ改めた —
  `encode_body` の全 `Err` exit（現在は `encode_host_error` の
  `?` 一本）が必ず `out.truncate(start)` を通る。`encode` doc
  に「every `Err` exit from `encode_body` lands here, not just the
  arms fallible today」と明記。`encode_host_error` 側も per-call
  `truncate(start)` で「nothing appended on failure」を独立保証
  （2 層の防御 — caller 側全量 rollback + callee 側 per-call
  不変条件）。
- **doc drift（fixed）**: `encode` doc が「encoding never fails
  on well-formed values — every wire type serializes verbatim」の
  ままだった（`Result` 化と矛盾）→「wire-representable values」
  へ文言修正 + all-or-nothing append + `# Errors` 節追加。
  `processor.rs`/`lib.rs` の canonical implementor 記述も
  `AudioBufferView*` を含む 2 系統記述へ整合。
- **`create_sendable` equivalence doc（fixed）**: `Some` を返す
  factory が `create()` と descriptor・挙動等価の instance を
  構築する契約（host は endpoint attach 可能性のある経路で
  `create()` 代替として呼び得る）を method doc に明文化。
- **`EndpointUnavailable` L1-runner consequence doc（fixed）**:
  L1 child runner 内の `Plugin` hook が `EndpointUnavailable` を
  返した場合の defined outcome（`UnsupportedVariant` 拒否 +
  runner の child 終了 → host は `Ipc`/`Failed` を観測）を
  variant doc に明記 — wire 不載の語義を「返してはいけない場所
  では何が起きるか」まで pin。

### Decisions During Implementation

- **`host_error_field_roundtrip` は infallible 型 `field_roundtrip`
  ヘルパを通さない**（deviation・deliberate）— 汎用
  `field_roundtrip` は `fn(&T, &mut Vec<u8>)` 形状の infallible
  型で、全 `decode_*` が `Result` を返す非対称と同じく
  `encode_*` 側は「信頼入力を verbatim に書く」前提だった。
  `encode_host_error` だけ `Result` 化したため、この 1 テストは
  ヘルパを使わず `encode_host_error` → `Reader` →
  `decode_host_error` を直接駆動する形へ（`trailing bytes` 検査
  も保持）。ヘルパ自体を `Result` 型へ一般化すると他の全
  infallible encoder 呼出しに波及するため見送り — YAGNI。
- **`l1_runner_e2e.rs` の `send` は `.expect("encode message")`**
  （deviation・deliberate）— task doc は「4 call site を
  `io::Error` 写像へ」とするが、E2E helper の `send` は
  `io::Result` を返さない test-only 経路（`MiniHost::send` は
  panic ベースの fixture — `write_all`/`flush` も `expect`）。
  `io::Result` 化は signature 変更を test 内部型へ波及させる
  だけで観測点を増やさないため、fixture 慣行どおり `expect` に
  留めた — encode 失敗は panic として同じく検出される。
- **rollback byte-identity をテストで pin** — 「`out` 不変」の
  語義を len 一致ではなく byte 列等価 + 後続 append 無傷まで
  固定（`vec![0xAA,0xBB,0xCC]` を pre-fill → 拒否後 `out ==
  before` → `Ok` reply append が先頭 bytes を壊さず decode 成功）。
- **`WireError::UnsupportedVariant` は既存 variant**（新規追加
  なし）— `encode_transport` の `Reply` 引数網羅先例が
  `WIRE_VERSION` 下で表現不能な値の reject に既に使用。wildcard
  arm は同 variant を返すのみで wire error vocabulary を増やさ
  ない。
- **`encode_reply` は infallible 維持** — `Reply` payload（
  `Ok`/`Scanned`/`State`/`Opened`）は `HostError` を運ばず全て
  wire 表現可能。`Result` 化すると signature churn のみで検査
  能力は増えない。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替実装の評価は design の確定事項をそのまま実装した
（新規 experiment なし）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean — statement-position
    `unused_must_use` 残留なし = call site/doctest/helper 全網羅
    の実証）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **55 test binaries 全 pass**。初回実行は
    `l1_proxy_e2e::wedged_child_write_converges_and_teardown_is_bounded`
    が `L1Instance::spawn` `Err(Ipc)` で 1 度失敗 — 並列負荷下の
    handshake 競合による flake で本 task の変更面（encode
    `Result` 化は `Hello`/`Scan` の wire 表現可能値では失敗しない）
    と無関係。単独再実行・`l1_proxy_e2e` 全量再実行・`cargo test
    --workspace` 全量再実行の 3 経路全てで pass — Risks 参照）
  - `mise run test-plugin-core` — exit 0（core 集中レーン —
    lib tests **67**・`video_contract` **8**・doctests **26**・
    compile-fail **2** = **103 pass** — plugin.rs 4 + processor.rs
    2 + wire.rs 1（`encode_rejects_endpoint_unavailable_and_restores_out`）
    の新規 7 観測点を含む）
  - `mise run lint-plugin-core` — exit 0
- 構造確認（実コマンド）:
  - `rg -n "EndpointUnavailable" crates/ruviform-plugin-core/src`
    — boundary.rs variant + Display arm + wire.rs wildcard 拒否 +
    doc/comment 実在。roundtrip variant リスト 2 件に含まれない
    ことを実確認（local-only — wire 表現可能 variant は tag 0–8）
  - `rg -n "erase_sendable" crates/ruviform-plugin-host/src` —
    `plugin.rs`（core `pub use` 行）→ `lib.rs`（`pub use plugin`
    行）の parity 経路実在 — `erase` と同型
  - `git diff` 全 inventory — `wire::encode` 呼出しは call site
    4 件 + doctest 3 件 + inline helper/direct call のみで全件
    `Result` 対応（`unused_must_use` 残留なしを lint gate で実証）

## Non-Goals

- endpoint 機構自体（`PluginEndpoint`/`EndpointInner`/`prepare_endpoint`/
  `EndpointBounds`）— Task 2（L0 inner）・Task 3（L1 inner）・
  Task 4（公開面）の scope。本 task が供給するのは前提語彙と
  codec 契約のみ。
- `EndpointUnavailable` を実際に返す経路 — Task 4 の
  `PluginHost::prepare_endpoint` が最初の発行点。本 task は
  variant + Display + wire 拒否のみ供給（roundtrip リスト非含有
  を以て local-only を pin）。
- `create_sendable` の実装者（既存 factory は全て default `None`
  のまま — Send-capable 供給は各 adapter/T2 以降の配線時に
  opt-in）。
- test-spec `plugin-endpoint-contract-tests.md` の `proposed →
  confirmed` 更新 — endpoint 機構（T2–T6）実装後の final
  verification で実施（plan の規律どおり）。
- `cargo doc` shm doc table の pre-existing 警告 8 件 — 本 task
  非起因（Follow-ups 参照）。

## Risks

- **`encode`/`encode_host_error` は公開 API の signature 変更**
  — pre-1.0 許容・plan で開示済み。workspace 内の全呼出しは
  同 commit で `Result` 対応済み（lint gate が残留を検出する
  構造）。downstream 外部 crate（存在する場合）は compile error
  で検出される — silent behavior change ではない。
- **`l1_proxy_e2e::wedged_child_write_converges_and_teardown_is_bounded`
  の flake 観測** — workspace 全量 test の並列負荷下で
  `L1Instance::spawn`（Hello/Scan handshake）が `Err(Ipc)` を 1 度
  返した。本 task の変更面と無関係（spawn/handshake 経路は
  wire 表現可能値のみで encode が失敗しない）。既存の
  timing-sensitive E2E として再発可能性あり — Follow-ups に記録。
- **`encode` は `WIRE_MAX_*` を検査しない非対称責務を維持** —
  over-bound field は verbatim に書かれ peer の decode が defined
  rejection する（issue-76 T1 確定の設計・doc 明記済み）。
  `Result` 化で「encode は失敗し得る」表面が増えたが、実際の
  `Err` は非 wire 表現 variant のみ — bounds 超過で `Err` を
  期待する誤用に注意（`# Errors` doc が scope を限定）。
- **rollback は `Vec<u8>` への append 前提** — `out` を
  `truncate(start)` で戻すため caller が別スレッドと共有する
  buffer には使えない（API 上 `&mut Vec<u8>` なので構造的に
  排他 — 変更不要）。

## Follow-ups

- Task 2 `docs/tasks/l0-shared-slot-and-endpoint-inner.md` —
  `create_sendable` 分岐・`Shared` スロット・`EndpointUnavailable`
  の最初の host 側発行経路（`try_lock` busy →
  `Err(EndpointUnavailable)`）が本 task の前提面を消費する最初の
  下游。
- Task 3 `docs/tasks/l1-endpoint-arm-and-leg-return.md` — leg 移出
  で `EndpointUnavailable`（leg checkout 中）が L1 側から発行される。
- `wedged_child_write_converges_and_teardown_is_bounded` flake —
  workspace 全量並列負荷下で spawn handshake `Err(Ipc)` を 1 度
  観測（本 task 非起因・単独/全量再実行で pass）。再発した場合は
  handshake timeout 値または test 直列化を別途 triage。
- `cargo doc` shm doc table の警告 8 件 — issue-76 由来の
  pre-existing warning（`wire::shm` doc table）。Full Gate の
  rustdoc warning 0 検証（plan action 20）までに別途対処が必要。
- `field_roundtrip` helper — 将来さらに fallible encoder が増えた
  場合は `Result` 型への一般化を再評価（現在は
  `encode_host_error` 1 本のみ・非対称維持が最小）。
