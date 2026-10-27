---
id: "TASK-034NlPHnwYkRTWHq1nvYvo"
type: "task"
status: "todo"
title: "L1 wire v2 docs/metadata sweep（README・bin doc・test-spec・親文書 addenda・literal sweep）"
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
        path: "crates/ruviform-plugin-export/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-export/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-clap/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs"
      - type: "file"
        path: "docs/test-specs/plugin-l1-wire-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-proxy-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-runner-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/real-adapter-l1-e2e-tests.md"
      - type: "file"
        path: "docs/designs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/specs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/specs/issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/designs/issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/specs/engine-plugin-host-integration.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
  blocks:
    - "docs/tasks/l1-bundle-child-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-wire-version-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/119"
  defers: []
  deferred-by: []
---

# L1 wire v2 docs/metadata sweep（README・bin doc・test-spec・親文書 addenda・literal sweep）

## Work

design L323-351 の `WIRE_VERSION` 1→2 bump 棚卸しのうち、T1-T3 の code-adjacent doc（wire.rs doc・shm.rs doc・export bin doc・crate doc・lib.rs doc）を除く残りの doc/metadata 面を一括更新する。本 task は doc のみ — code 変更なし。historical record（旧 plan/task/IR/discovery）は例外ポリシーどおり書き換えない。

1. crate README — `crates/ruviform-plugin-export/README.md`（L7 `Instantiate` ごとに spawn・L13-14 argv 契約表を v2: `<runner>` + spec.args・id/key は `CreateInstance` payload へ）・`crates/ruviform-plugin-vst3/README.md`（L72）・`crates/ruviform-plugin-clap/README.md`（L64）の `run` argv 規約記述を v2 へ
2. adapter runner bin doc — `crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs`（L12）・`crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs`（L11）の「`run` argv contract (`--ruviform-shm-key`, `--ruviform-plugin-id`, …) is unchanged」記述を v2 argv（spec.args・id/key は `CreateInstance` payload）へ更新（doc のみ）
3. `crates/ruviform-plugin-export/Cargo.toml` — L13 依存コメントの `--ruviform-shm-key` attach 記述を `CreateInstance` 経由へ更新
4. 既存 test-spec ×4 を v2 へ更新（design L341-344 に対応 — 本 task は本文更新・frontmatter `verifies` は planning 時点で `issue-117` spec 追加済み）:
   - `docs/test-specs/plugin-l1-wire-codec-tests.md` — L52 `WIRE_VERSION = 1` 名指し → 2・L62（「子は singleton」・`Instantiate`/`Destroy` 非 wire 動詞 → v2 では `CreateInstance`/`DestroyInstance` が wire 動詞）・L93 non-goal「bundle 子 …将来プロトコル改訂」行 → v2 で改訂済み
   - `docs/test-specs/plugin-l1-proxy-contract-tests.md` — L52（「per-instance L1 プロキシ（`pub(crate)` — 1 子プロセス + `driver`/`reader` 2 スレッド）」→ per-child 2 スレッドを維持・per-instance spawn 前提を除去）・L80（`Hello.version != 1` → handshake 失敗 → `!= WIRE_VERSION`(v2)）・handshake に `CreateInstance` 追加・argv v2・`Failed` 粒度を `issue-117-bundle-failed-granularity-tests` + ADR へ pointer
   - `docs/test-specs/plugin-l1-runner-e2e-tests.md` — L57（argv 規約 → `<runner>` + spec.args・id/key は `CreateInstance` payload）・L60（dispatch → `instance` 解決段 + registry）・L70-71（argv 前提の examples → `CreateInstance` 経路）・L80（guarantees の argv→attach 経路）・L91（non-goals「runner は singleton」行 → registry v2）
   - `docs/test-specs/real-adapter-l1-e2e-tests.md` — L55（runner argv 規約の「不変」記述 → v2 argv）・L56（spawn→handshake 系列 → `Instantiate` が spawn+Hello+Scan+`CreateInstance` 経由）・L61/L70-71（`InstanceFailed` once・kill 系列 → per-handle once は維持するが bundled route kill は影響 instance 数分の通知列になる但書）
5. 親文書 addenda ×5（design L345-349 に対応 — 実装 bump 時の確定行へ reconcile）:
   - `docs/designs/plugin-l1-out-of-process.md` — L113（「1 子プロセス = 1 プラグインインスタンス（per-`Instantiate` spawn）」singleton モデル記述）・L115（スレッド構成表）・L142（`WIRE_VERSION = 1`）・L147-151（wire 表 — `Command{seq}` 等に `instance` なし）・L155（`Command` enum コメント「子は singleton」）・L181（handshake argv）・L196（argv shm-key 引き渡し）・L204-210（plugin-export argv 規約）を v2 へ reconcile
   - `docs/specs/plugin-l1-out-of-process.md` — L120 クラッシュ契約へ Failed-granularity 但書（bundle では子死亡 = 同居全 live instance `Failed`・粒度正本は `docs/adr/issue-117-bundle-child-failed-granularity.md`）
   - `docs/specs/issue-114-real-adapter-l1-e2e.md` — L99・L123 の argv 規約「不変/維持」記述へ #117 bump で v2 へ変わる旨の但書
   - `docs/designs/issue-114-real-adapter-l1-e2e.md` — L76「上流確定事項（再決定しない）」の v1 前提列（「1 子 = 1 インスタンス」・argv 規約）・L354 verification 行を v2 前提へ更新
   - `docs/specs/engine-plugin-host-integration.md` — L135 AC5「L1 子強制終了 → `Failed` + `InstanceFailed` 通知 once」へ bundled route kill = 影響各 live instance へ 1 つずつ（N 通知列）の Failed-granularity 但書
6. literal sweep — `grep -rn "WIRE_VERSION" crates/` で `1` 残存を確認（historical 注記以外）・第 2 grep: `--ruviform-`・`one child`・`per-instance spawn`・`1 子`・`singleton` を `crates/` と `docs/` で走査。historical record（discovery・plan・task・IR・旧 test-spec）は例外ポリシーどおり除外。`cargo doc` 警告も確認

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（T5 — docs sweep）
- [../designs/issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md) L323-351（`WIRE_VERSION` bump 棚卸し — 本 task が doc 面の正本として対応）
- [../adr/issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)（Failed 粒度但書の正本 — addenda で参照する）
- 既存 test-spec ×4（更新対象・`verified-by` で対応済み）:
  - [plugin-l1-wire-codec-tests.md](../test-specs/plugin-l1-wire-codec-tests.md)
  - [plugin-l1-proxy-contract-tests.md](../test-specs/plugin-l1-proxy-contract-tests.md)
  - [plugin-l1-runner-e2e-tests.md](../test-specs/plugin-l1-runner-e2e-tests.md)
  - [real-adapter-l1-e2e-tests.md](../test-specs/real-adapter-l1-e2e-tests.md)

## Dependencies

- 先行タスク: [l1-route-pool-and-isolation-surface.md](l1-route-pool-and-isolation-surface.md)（`OutOfProcessIsolation` 公開面・argv v2・`CreateInstance` bootstrap の事実確定が前提。T4 と並行可 — doc file と test file は非交差）

## Affected Files

- `crates/ruviform-plugin-export/README.md`・`crates/ruviform-plugin-vst3/README.md`・`crates/ruviform-plugin-clap/README.md` — crate README v2 更新
- `crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs`・`crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs` — bin doc argv v2（doc のみ）
- `crates/ruviform-plugin-export/Cargo.toml` — 依存コメント
- `docs/test-specs/plugin-l1-{wire-codec,proxy-contract,runner-e2e}-tests.md`・`docs/test-specs/real-adapter-l1-e2e-tests.md` — test-spec v2 更新
- `docs/designs/plugin-l1-out-of-process.md`・`docs/specs/plugin-l1-out-of-process.md`・`docs/specs/issue-114-real-adapter-l1-e2e.md`・`docs/designs/issue-114-real-adapter-l1-e2e.md`・`docs/specs/engine-plugin-host-integration.md` — 親文書 addenda

## Done When

- [ ] 3 crate README の argv/責務記述が v2（spec.args + `CreateInstance` payload）へ更新されている
- [ ] 2 adapter runner bin の doc argv 契約が v2 へ更新されている
- [ ] `export/Cargo.toml` L13 コメントが `CreateInstance` 経路を指す
- [ ] 4 既存 test-spec が design L341-344 の各行を v2 oracle へ更新している（本文・non-goal・argv 記述・version 名指し）
- [ ] 5 親文書の addenda 行（`plugin-l1-out-of-process` design L113-210・spec L120・`issue-114` spec L99/L123・design L76/L354・`engine-plugin-host-integration` spec L135）が v2/Failed 粒度但書へ reconcile されている
- [ ] `grep -rn "WIRE_VERSION" crates/` で `1` が historical 注記以外に残存しない・第 2 grep（`--ruviform-`・`one child`・`per-instance spawn`・`1 子`・`singleton`）で historical record 以外に残存しない
- [ ] `cargo doc` 警告なし・doc audit findings ゼロ

## Verification

- [ ] `grep -rn "WIRE_VERSION" crates/` で `1` 残存が historical 注記以外にない
- [ ] `grep -rn '\-\-ruviform-\|one child\|per-instance spawn\|1 子\|singleton' crates/ docs/` で historical record（discovery・plan・task・IR・旧 test-spec）以外に残存しない
- [ ] `mise exec -- cargo doc -p ruviform-plugin-export -p ruviform-plugin-clap -p ruviform-plugin-vst3 --no-deps` で warning 0
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す
- [ ] `mise run fmt-check` / `check` / `lint` が pass（doc のみのため test 不要 — T6 で full gate）
