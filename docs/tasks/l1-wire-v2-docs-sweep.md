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
        path: "crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs"
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

design L336-364 の `WIRE_VERSION` 1→2 bump 棚卸しのうち、T1-T3 の code-adjacent doc（wire.rs doc・shm.rs doc・export bin doc・crate doc・lib.rs doc）を除く残りの doc/metadata 面を一括更新する。本 task は doc のみ — code 変更なし（下記 stale comment 修正はコメント行のみ）。historical record（旧 plan/task/IR/discovery）は例外ポリシーどおり書き換えない。

1. crate README — `crates/ruviform-plugin-export/README.md`（L7 `Instantiate` ごとに spawn・L13-14 argv 契約表を v2: `<runner>` + spec.args・id/key は `CreateInstance` payload へ）・`crates/ruviform-plugin-vst3/README.md`（L72）・`crates/ruviform-plugin-clap/README.md`（L64）の `run` argv 規約記述を v2 へ
2. adapter runner bin doc — `crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs`（L12）・`crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs`（L11）の「`run` argv contract (`--ruviform-shm-key`, `--ruviform-plugin-id`, …) is unchanged」記述を v2 argv（spec.args・id/key は `CreateInstance` payload）へ更新（doc のみ）
3. `crates/ruviform-plugin-export/Cargo.toml` — L13 依存コメントの `--ruviform-shm-key` attach 記述を `CreateInstance` 経由へ更新
4. 既存 test-spec ×4 の v2 oracle は planning commit（cd69e48）で更新適用済み — 本 task は**実装着地との verify/reconcile**を行い、divergence のみ更新する（design L354-357 の各 inventory 行が正本・frontmatter `verifies` は planning 時点で `issue-117` spec 追加済み）。verify 観点:
   - `docs/test-specs/plugin-l1-wire-codec-tests.md` — `WIRE_VERSION = 2`・envelope `[seq][instance][rest]`・新動詞表・non-goal の v2 引き継ぎ行が実装 codec と一致するか
   - `docs/test-specs/plugin-l1-proxy-contract-tests.md` — per-child proxy 形状（1 子 2 スレッド N instance）・handshake +`CreateInstance`・argv v2・`Hello.version != WIRE_VERSION`（!= 2）・`Failed` 粒度 pointer（`issue-117-bundle-failed-granularity-tests` + ADR）が着地形と一致するか
   - `docs/test-specs/plugin-l1-runner-e2e-tests.md` — argv v2・`instance` 解決 dispatch + registry・`CreateInstance` 検証 chain・`InvalidHandle` flagged reply が runner registry 実装と一致するか
   - `docs/test-specs/real-adapter-l1-e2e-tests.md` — argv/handshake v2 記述・bundled route kill の通知列但書が実装と一致するか
5. 親文書 addenda ×5（design L358-362 に対応 — 実装 bump 時の確定行へ reconcile）:
   - `docs/designs/plugin-l1-out-of-process.md` — L113（「1 子プロセス = 1 プラグインインスタンス（per-`Instantiate` spawn）」singleton モデル記述）・L115（スレッド構成表）・L142（`WIRE_VERSION = 1`）・L147-151（wire 表 — `Command{seq}` 等に `instance` なし）・L155（`Command` enum コメント「子は singleton」）・L181（handshake argv）・L196（argv shm-key 引き渡し）・L204-210（plugin-export argv 規約）を v2 へ reconcile
   - `docs/specs/plugin-l1-out-of-process.md` — L120 クラッシュ契約へ Failed-granularity 但書（bundle では子死亡 = 同居全 live instance `Failed`・粒度正本は `docs/adr/issue-117-bundle-child-failed-granularity.md`）
   - `docs/specs/issue-114-real-adapter-l1-e2e.md` — L99・L123 の argv 規約「不変/維持」記述へ #117 bump で v2 へ変わる旨の但書
   - `docs/designs/issue-114-real-adapter-l1-e2e.md` — L76「上流確定事項（再決定しない）」の v1 前提列（「1 子 = 1 インスタンス」・argv 規約）・L354 verification 行を v2 前提へ更新
   - `docs/specs/engine-plugin-host-integration.md` — L135 AC5「L1 子強制終了 → `Failed` + `InstanceFailed` 通知 once」へ bundled route kill = 影響各 live instance へ 1 つずつ（N 通知列）の Failed-granularity 但書
6. literal sweep — `grep -rn "WIRE_VERSION" crates/` で `1` 残存を確認（historical 注記以外）・第 2 grep（case-insensitive・`grep -rni`）: `--ruviform-`・`one child`・`1 child`・`per-instance spawn`・`per instance`・`1 子`・`singleton` を `crates/` と `docs/` で走査。`l1/mod.rs:28` モジュール doc・`l1/data.rs:926` は T2 item 12 で処理済みの前提 — 残存があれば本 task で reconcile。stale comment 拾い: `crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs`（`~L221-222`）・`crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs`（`~L261-262`）の「the `run` argv contract is unchanged」コメントを v2 argv（`<runner>` + spec.args・id/key は `CreateInstance` payload）へ修正（コメントのみ）。historical record（discovery・plan・task・IR・旧 test-spec）は例外ポリシーどおり除外。`cargo doc` 警告も確認

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（T5 — docs sweep）
- [../designs/issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md) L336-364（`WIRE_VERSION` bump 棚卸し — 本 task が doc 面の正本として対応）
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
- `docs/test-specs/plugin-l1-{wire-codec,proxy-contract,runner-e2e}-tests.md`・`docs/test-specs/real-adapter-l1-e2e-tests.md` — test-spec v2 verify/reconcile（planning 時点の v2 更新を実装と照合・divergence のみ更新）
- `crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs`・`crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs` — stale「argv contract is unchanged」コメントのみ
- `docs/designs/plugin-l1-out-of-process.md`・`docs/specs/plugin-l1-out-of-process.md`・`docs/specs/issue-114-real-adapter-l1-e2e.md`・`docs/designs/issue-114-real-adapter-l1-e2e.md`・`docs/specs/engine-plugin-host-integration.md` — 親文書 addenda

## Done When

- [ ] 3 crate README の argv/責務記述が v2（spec.args + `CreateInstance` payload）へ更新されている
- [ ] 2 adapter runner bin の doc argv 契約が v2 へ更新されている
- [ ] `export/Cargo.toml` L13 コメントが `CreateInstance` 経路を指す
- [ ] 4 既存 test-spec の v2 oracle（design L354-357 の各行）が実装着地と reconcile 済み — planning 時点の v2 更新を verify し divergence のみ更新した
- [ ] 5 親文書の addenda 行（`plugin-l1-out-of-process` design L113-210・spec L120・`issue-114` spec L99/L123・design L76/L354・`engine-plugin-host-integration` spec L135）が v2/Failed 粒度但書へ reconcile されている
- [ ] `grep -rn "WIRE_VERSION" crates/` で `1` が historical 注記以外に残存しない・第 2 grep（`--ruviform-`・`one child`・`1 child`・`per-instance spawn`・`per instance`・`1 子`・`singleton` — case-insensitive）で historical record 以外に残存しない
- [ ] `cargo doc` 警告なし・doc audit findings ゼロ・`mise run lint-l1-real-plugin` が pass（本 task が触る clap/vst3 bin・test file は `--features host` 時のみ compile — default `lint` では検出不能なため）

## Verification

- [ ] `grep -rn "WIRE_VERSION" crates/` で `1` 残存が historical 注記以外にない
- [ ] `grep -rni '\-\-ruviform-\|one child\|1 child\|per-instance spawn\|per instance\|1 子\|singleton' crates/ docs/` で historical record（discovery・plan・task・IR・旧 test-spec）以外に残存しない
- [ ] `mise exec -- cargo doc -p ruviform-plugin-export -p ruviform-plugin-clap -p ruviform-plugin-vst3 --no-deps` で warning 0
- [ ] `mise run lint-l1-real-plugin`（`cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets --features host -- -D warnings`）が pass — 本 task が触る adapter bin/test file は `required-features = "host"` で default gate の対象外のため
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す
- [ ] `mise run fmt-check` / `check` / `lint` が pass（doc のみのため test 不要 — T6 で full gate）
