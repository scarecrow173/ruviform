---
id: "TASK-034OpskKobU4qEsX7F7IYc"
type: "task"
status: "done"
title: "T6. stderr piped + drain thread（案A・FFI 局所化・fixture knob）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/120"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/diag.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs"
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
  references: []
  defers: []
  deferred-by: []
---

# T6. stderr piped + drain thread（案A・FFI 局所化・fixture knob）

## Work

全 L1 子の stderr を piped 化し、案A（定期起床 + 期限後自発終了）の drain thread を実装する。

- **配管**: **`spawn_scanned`** の `.stderr(Stdio::inherit())`（`l1/mod.rs:1852`）→ `Stdio::piped()`；`spawn_child` が `child.stderr.take()` で取得（stdout と同列）。`SpawnRx` を拡張して stderr + 診断レコードを handoff（payload は新 record 型）。drain thread は `Hello` wait 前に開始 — pre-`Hello` 出力を捕捉。
- **drain（案A）**: POSIX `libc::poll`（`POLLIN|POLLHUP`・timeout 起床）/ Windows `windows-sys` `PeekNamedPipe`+短 sleep。`STDERR_DRAIN_GRACE`=500 ms・起算は子終了観測・延長なし・EOF 即終了・終了時 read fd close（heir 後続書き込みの error 化は明示受容）・JoinHandle detach（無期限 join しない）。`unread_bytes_observed`/`drain_deadline_exceeded`/`eof` を記録。drain/record 初期化失敗時は既存 abort 経路で rollback（partial child 残存なし）。
- **FFI 局所化**: `poll`/`PeekNamedPipe` は std 不在 — unix `libc`・windows `windows-sys`（**`Win32_System_Pipes`,`Win32_Foundation`** — `PeekNamedPipe` は `windows_sys::Win32::System::Pipes` 所在）の **platform-gated dep を plugin-host `Cargo.toml` に追加**。FFI は `l1/diag.rs` 内の `#[expect(unsafe_code)]` + SAFETY コメントに閉じ込める（workspace lint は `unsafe_code` 実質 deny）。
- **fixture knob**: `l1-test-worker.rs`/`l1-test-video-worker.rs` に同一 flag 名で `--stderr-bytes=N`（起動時・Hello 前に N bytes 出力）+ `--heir-stderr-bytes=N`・`--heir-stderr-delay-ms=M`（heir が遅延出力 — deadline/unread flags 検証用）を追加（両 worker で flag 名を揃える — T11 parity）。heir は `current_exe` の別 spawn（現行は `--sleep-ms=15000` のみ — `l1-test-worker.rs:726-727`）なので、parse した `--heir-stderr-*` は **heir の argv に転送しないと出力されない** — parent が heir spawn 時に値を引き継ぐ配線を含める。
- **in-code doc 同期**: `l1/mod.rs` 冒頭のスレッドモデル表（:5・:31-36 —「per child = 1 process + 2 threads」）に stderr drain スレッド行を追加（fd close/detach 規約を含む）。
- spawn-scoped owner 構造を維持 — `Child` 生成から drain 起動までの間に cancellable 処理を挟まない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`（「採用案 A」節・deadline pin・実装依存注記）

## Dependencies

- `docs/tasks/add-l1-diag-record-and-registry.md`（record/registry 型の前提）

## Affected Files

- `crates/ruviform-plugin-host/Cargo.toml`・`src/l1/mod.rs`・`src/l1/diag.rs`
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`・`src/bin/l1-test-video-worker.rs`

## Done When

- [ ] 全 L1 子が piped stderr で spawn され `Hello` 前から drain が動く
- [ ] drain が 500 ms deadline・EOF 即終了・fd close・detach の規約で終了する
- [ ] platform-gated FFI が `l1::diag` 内 `#[expect(unsafe_code)]` に閉じている
- [ ] 両 fixture が `--stderr-bytes`/`--heir-stderr-*` flag を持つ

## Verification

- [ ] `mise run check` exits 0
- [ ] `mise run lint` exits 0（unsafe 局所許可を含む）
- [ ] `mise exec -- cargo test -p ruviform-plugin-host` exits 0
- [ ] `mise exec -- cargo test --test l1_proxy_e2e` exits 0（stderr 捕捉・bounded teardown — OS 別期待値）
