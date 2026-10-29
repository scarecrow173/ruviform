---
id: "TASK-034OpsDG34f3LJc5NmwGNX"
type: "task"
status: "done"
title: "T8. pre-mint 診断（phase・retention 登録・SpawnFailed 境界・query 面）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/76"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/diag.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/tasks/add-l1-diag-record-and-registry.md"
    - "docs/tasks/pipe-and-drain-l1-stderr.md"
    - "docs/tasks/wire-failure-cause-classification.md"
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
    - "docs/test-specs/issue-121-l1-diagnostics-retention-tests.md"
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T8. pre-mint 診断（phase・retention 登録・SpawnFailed 境界・query 面）

## Work

pre-mint 失敗の診断レコード登録・`diagnostics_id` 発行・`SpawnFailed` 返却を配線する。

- **所有・受け渡し**: `PluginHost` が `Arc<DiagRegistry>` を保持し、spawn 系関数（`spawn_scanned`/`create_instance`/`create_video_instance`/`instantiate_bundled` 系）へ `diag: &DiagRegistry` 引数で渡す。record 生成は spawn 経路内・**失敗確定時は `Err` を返す関数自身が登録** — `Shared` drop 後も registry の `Arc` が所有。テスト経路（`L1Instance::spawn` 等）は呼出側が `DiagRegistry::new()` の fresh registry。
- **`phase`**: pre-spawn 検証 / OS spawn / drain 初期化 / mint・handshake を記録（stderr の有無は phase 依存）。
- **`SpawnFailed` 境界の pin**: 定義済み拒否は variant を保持し `SpawnFailed` で包まない・レコードも登録しない — `Err(HostError::InstanceLimit)`（`mod.rs` の unflattened match arm `:2005`/`:2123`・doc `:2433`・~8 pin test）と spec-validation `Err(Ipc)`。live 子への pooled `create_instance` `Reply::Err` → `SpawnFailed{cause: Handshake}` + **試行レコード**（その時点の子 stderr の frozen snapshot copy — live 子の active レコードは mutate 継続・分離）。
- **cleanup_error**: cleanup（kill/reap 戻り）失敗は `cleanup_error` 副次診断 — 主 cause は不変。
- **minted 失敗**: child finalize で retention 登録 + handle ベース検索 index（通知に ID field なし — handle が相関）。
- **assert 移行**: spawn/Instantiate 経路の `Err(HostError::Ipc)`/`expect_err` ~12 件を `matches!`/`SpawnFailed{cause,..}` に（`diagnostics_id` は実行時採番で値 assert 不可 — `cause` を oracle に）。post-mint waiter の `Err(Ipc)`（`l1_proxy_e2e.rs:230/273/320`・`l1_video_e2e.rs:1251` 等）は変更しない — 識別子で区別。
- **query 面**: composite に `DiagnosticQuery` の公開（`Found`/`Evicted`/`NotFound`）— 戻り型は `pub` 化して `lib.rs` から re-export（public API consumer が結果型を命名可能に）。
- **in-code doc 同期**: `L1Instance::spawn`/`create_instance`/`create_video_instance` の error doc（`mod.rs:~2427` 等「Any failure … is `Err(HostError::Ipc)`」）を `SpawnFailed` 経路に合わせて書き換え — `boundary.rs` の `HostError::Ipc` doc（「spawn, handshake, …timeout」:145-146）も Ipc の守備範囲が狭まる旨に合わせる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`（「生成前失敗の diagnostics ID」「registry の所有・受け渡し」「`SpawnFailed` の境界」節）

## Dependencies

- `docs/tasks/add-l1-diag-record-and-registry.md`（`DiagRegistry`）
- `docs/tasks/pipe-and-drain-l1-stderr.md`（record handoff・drain）
- `docs/tasks/wire-failure-cause-classification.md`（cause 分類）

## Affected Files

- `crates/ruviform-plugin-host/src/l1/mod.rs`・`src/l1/diag.rs`・`src/l1/video.rs`・`src/composite.rs`・`src/lib.rs`（`mod l1` は `pub(crate)` — 公開 query メソッドが `DiagnosticQuery`/`StderrSnapshot`/record 型を返すには `pub` 型 + `lib.rs` re-export が必要・E0446 回避）
- `crates/ruviform-plugin-host/tests/{l1_proxy_e2e,l1_video_e2e,l1_supervisor_e2e}.rs`（assert 移行）

## Done When

- [x] 失敗確定時に retention 登録 + `diagnostics_id` が発行され `Err(SpawnFailed{cause, diagnostics_id})` が返る
- [x] `Err(InstanceLimit)`・spec-validation `Err(Ipc)` が variant 保持でパススルーされる（無退行）
- [x] minted 失敗が handle index で検索可能・`found/evicted/not-found` が正しい
- [x] ~12 件の assert 移行が `cause` を oracle に `matches!` 化されている

## Verification

- [x] `mise run check` exits 0
- [x] `mise run lint` exits 0
- [x] `mise exec -- cargo test -p ruviform-plugin-host` exits 0
- [x] `mise exec -- cargo test --test l1_proxy_e2e` exits 0（pre-mint `phase`/`cause` 区別・rollback で partial child なし・ID query 3 値）
