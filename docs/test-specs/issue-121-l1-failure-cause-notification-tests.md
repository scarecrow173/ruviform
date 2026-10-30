---
id: "TSPEC-034OpmNe3vUnEprFdOIHb6"
type: "test-spec"
status: "approved"
title: "L1 FailureCause・通知互換（Issue #121 — 7 variant 分類・first-observed・InstanceFailed.cause・wire 不変）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/119"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/supervisor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs"
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
    - "docs/test-specs/issue-121-l1-parity-nonregression-tests.md"
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
    - "docs/adr/issue-121-l1-diagnostics-policy.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 FailureCause・通知互換（Issue #121 — 7 variant 分類・first-observed・InstanceFailed.cause・wire 不変）

## Purpose

pin `FailureCause` 語彙・分類境界・first-observed 収束・`InstanceFailed`/`HostError` API 変化・wire 不変性 — spec R-B4/R-B5・AC4/AC5 と設計「原因の分類と収束」節。`HostError::Ipc` への畳み込みで失われていた失敗原因を、wire 契約を変えずに host 側で保持するのが本 issue の核心。分類表と first-observed 固定が将来の追加観測点でも維持されることを保証する。

## Feature

`FailureCause`（`Copy`・`#[non_exhaustive]`・7 variant）が `HostNotification::InstanceFailed` に `cause` field として載り、pre-mint 失敗は `Err(HostError::SpawnFailed{cause, diagnostics_id})` で返る。`HostError`/`HostNotification`/`ChildExit`/`FailureCause` は wire codec 対象外のまま（`WIRE_VERSION`・tag 不変）。

## Rules

- **語彙（設計確定・7 variant）**: `Spawn`（OS spawn 失敗・pre-mint のみ）・`Handshake`（定義済み handshake 拒否 — `Hello` version 不一致・decodable 非 `Hello` 先頭 frame・`Scan`/descriptor mismatch・`CreateInstance` `Err` reply）・`Timeout`（host deadline 判断 — `Hello` wait・`CreateInstance` reply wait・command waiter・watchdog kill 前記録）・`Protocol`（decode 不能 frame・`Hello` 受理後 stream 違反・2nd `Hello`・unknown kind・parent-to-child kind・scope/instance 違反・waiter-less `Reply`・instance 帰属違反）・`Transport`（stdout EOF/read error・stdin write 失敗 — phase 不問・handshake 中の EOF も `Transport`）・`ExitObserved`（`try_wait`/reap で子終了観測 — status は別の `ChildExit`）・`Internal`（channel 切断・unreachable・`try_wait`/`wait` syscall error — **最初の失敗観測が reap error でも `Internal` として必ず定義される**）。
- **first-observed 固定（設計）**: 同じ `inner` lock 内で最初に遷移を確定した cause を記録・上書き禁止 — reader/driver/reap/timeout の競合は lock の直列化順で決まる。timeout は **kill 発行前**に記録（後続 EOF で `Transport` に置き換わらない）。
- **reap と cause（設計）**: reap は `ChildExit` status のみ補完し cause を変えない — reap/`wait` で status が取れなければ `ChildExit{code:None, signal:None}` を送り cause は固定済みのものを保持。
- **poison 非分類**: `Shared::lock()`/`lock_mutex` は `PoisonError::into_inner` で常に guard 取得 — lock 取得不能による記録欠落は構造上発生しない。poison 自体は分類しない（panic は EOF/timeout 経由で間接観測）。
- **`InstanceFailed` 互換（設計）**: `{handle, exit, cause}` — `Copy` 維持 + variant レベル `#[non_exhaustive]`（`SupervisionEvent` 慣例）。完全 field pattern `{handle, exit}` は source break — workspace 内 consumer 限定で受容（ADR 記録）。全 5 ファイルの pattern は `..` 付与で対応、全構築サイトは in-crate で `cause` 供給。
- **`HostError::SpawnFailed` 境界**: 定義済み拒否（`InstanceLimit`・spec-validation `Err(Ipc)`）は variant 保持・`SpawnFailed` で包まない — retention spec の詳細は姉妹 spec に委譲。`HostError` は wire codec 対象外 — 新 variant は wire tag に影響しない（Rust 側 source compatibility の受容は別保証）。
- **wire 不変（spec R-B5・AC5）**: `WIRE_VERSION`・codec・`Hello.capabilities`・`WireError` tag 不変。codec テストで `encode_host_error(SpawnFailed)` → `Err(UnsupportedVariant)`（`_` catch-all・`EndpointUnavailable` と同じ local-only 扱い）を pin。
- **crate 境界 compile（承認条件）**: 全 consumer — `composite.rs`・`supervisor.rs`・test×5 — が `check`+`lint`+**`lint-l1-real-plugin`** で compile 通過（l1_real 両ファイルは `#![cfg(feature="host")]` で既定 gate 外・lane 必須）。
- **supervisor**: watched `InstanceFailed` は既存どおり消費・`cause` を payload 転送しない（R-B8 — cause 分岐は範囲外）。`cause` が必要なら handle → 診断レコード query。

## Examples

- EOF → `Transport`；**deterministic な `ExitObserved` には `--leave-heir-after-msgs` fixture を使う**（grandchild が stdout を保持 → EOF が来ず `try_wait` が唯一の観測者）。plain crash（stdout 死亡即 EOF）は reader の EOF 観測と `try_wait` が競合するため `Transport`/`ExitObserved` のどちらが先に確定するか lock 直列化順次第 — 「crash → `ExitObserved`」を fixture なしで断言しない。command timeout → `Timeout`（kill 前記録）・protocol violation → `Protocol` — 通知 `cause` が観測できる
- 子死亡と同時に EOF+exit が競合 → lock 直列化順で先に確定した方が cause・後続は上書きしない
- `try_wait` 失敗が最初の観測 → `Internal`（`FailureCause` は必ず定義される）
- `encode_host_error(SpawnFailed{...})` → `Err(UnsupportedVariant)`・`WIRE_VERSION` 定数変更なし
- `lint-l1-real-plugin` で l1_real 両ファイルの `{handle, exit, ..}` pattern が compile 通過

## Guarantees

- minted 失敗は `InstanceFailed{handle, exit, cause}` で exactly-once 通知（既存 `Notify` 状態機械無退行 — `ChildExit` status は別 payload）
- 原因分類はエラーの意味で決まり実装位置に依存しない（demux loop vs first-frame 受理経路の差異で分類が変わらない）
- `FailureCause`・`HostNotification`・`ChildExit` は wire に serialize されない — 全子との wire 契約は変更なし
- pre-mint 失敗は handle 捏造せず `Err` 経由で caller に届く（観測保証の限定は retention spec 参照）

## Non-goals

- `HostError::Ipc` への payload 追加・rich（非 `Copy`）失敗 payload
- 全 consumer の `cause` 利用分岐 — 通知は運ぶだけで消費は caller 責務
- wire 上での失敗原因交換（`HostNotification` は wire を通らない）
- supervisor の cause 分岐・`InstanceLimit` 等の defined rejection の record 登録

## Risk

- `Handshake`/`Protocol` を「実装位置」で分けると CreateInstance 待ち中の decode 不能 frame が実装依存で分類が揺れる — 意味ベース表の pin が必要
- timeout を kill 後に記録すると EOF が先に確定し `Transport` に誤分類される
- `encode_host_error` が新 variant で catch-all を通らない形に編集されると wire tag 追加 = `WIRE_VERSION` 違反になる
- `lint-l1-real-plugin` を検証に含め忘れると `#![cfg]` ゲートの完全 pattern 破壊が CI まで検出されない
