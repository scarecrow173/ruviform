---
id: "TASK-034NlQlfkFL52FRHRjhYmr"
type: "task"
status: "todo"
title: "L1 wire v2 codec + 全系統 v2 cutover（envelope・新動詞・InstanceLimit・registry・harness）"
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
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
  blocks:
    - "docs/tasks/l1-child-instance-split.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-wire-version-policy.md"
    - "docs/tasks/l1-internal-cleanup.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-117-wire-v2-codec-tests.md"
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/119"
  defers: []
  deferred-by: []
---

# L1 wire v2 codec + 全系統 v2 cutover（envelope・新動詞・InstanceLimit・registry・harness）

## Work

wire v2 の atomic cutover — `Message` の `instance` field 追加は enum breaking change で中間コンパイル状態が存在せず、bootstrap 契約（argv `--ruviform-*` → `CreateInstance` payload）も host・runner・worker で同時切替が必要なため、本 task で全 consumer を最終 v2 形へ揃える。

1. `crates/ruviform-plugin-core/src/wire.rs` — `WIRE_VERSION` 1→2、`WIRE_MAX_INSTANCES: u32 = 64`・`WIRE_INSTANCE_NONE: u32 = u32::MAX` を追加。`Command`/`Reply`/`BlockIn`/`BlockDone` payload を `[u64 seq][u32 instance][rest]` envelope 化。`Command` に `CreateInstance{plugin_id: String, shm_key: String, in_depth: u32, out_depth: u32}`(tag 10)・`DestroyInstance`(tag 11) を追加（`#[non_exhaustive]` 維持）。decode で scope 判定表を実装: `Scan` は `instance == WIRE_INSTANCE_NONE` 必須・11 個の instance-scope 動詞は non-NONE 必須・`BlockIn`/`BlockDone`/`Reply` の instance は `< WIRE_MAX_INSTANCES || == NONE`（動詞に応じた必須条件）・違反は `Malformed`。`encode_host_error` に `InstanceLimit → 9` 追加・decode tag 9 → `InstanceLimit`（wildcard arm trap 対策として両方向 pin）。既存 tag 0-9 の payload 意味論は維持。codec doc の `WIRE_VERSION` 参照・wire 契約記述を v2 へ更新。codec unit test: 全動詞 round-trip・scope 違反 → `Malformed` 行列・`InstanceLimit` round-trip・`u32::MAX`/`>64` の instance 拒否・bounded decode（adversarial instance で alloc/panic なし）・`segment_len(3,3) * WIRE_MAX_INSTANCES`（≈204 MiB/子 — `WIRE_MAX_INSTANCES=64` の根拠）を layout-chain const test で pin（design L375）
2. `crates/ruviform-plugin-core/src/boundary.rs` — `HostError::InstanceLimit` variant を追加（`#[non_exhaustive]` 維持）・`Display` impl・doc に「runner registry 満杯・host pool handle 枯渇」の意味を記述
3. `crates/ruviform-plugin-export/src/lib.rs` — `Runner` singleton → `instances: HashMap<u32, InstanceEntry>` registry。`InstanceEntry{instance, lifecycle, context, activated_max_frames, segment, staging_in, staging_out}`。`serve` を instance 解決 dispatch へ変更: `Scan` → factories enumerate（child-scope・`instance=NONE`）・`CreateInstance{i,plugin_id,shm_key,in_depth,out_depth}` → `plugin_id ∈ factories`（不在→`Err(UnknownPluginId)`）→ `i` 未登録（重複→`Err(Ipc)`）→ `registry.len() < WIRE_MAX_INSTANCES`（超過→`Err(InstanceLimit)`）→ shm attach + header 検証 + 宣言 depth 照合（失敗→`Err(Ipc)`）→ insert。instance-scope 動詞/`BlockIn` → `instances[i]`（未登録→`Reply{i,seq,Err(InvalidHandle)}` flagged・子は継続）。`DestroyInstance{i}` → entry 撤去 + 該当 instance の normalize。echo は verbatim（Reply/BlockDone の `instance` は受信値をそのまま写す）。argv v2: `<runner>` + spec.args のみ・`--ruviform-shm-key`/`--ruviform-plugin-id` を除去（受信 → usage reject exit 2 維持）。`Shutdown`/stdin EOF → 全 entry normalize → exit 0。panic → `Err(PluginFailed)` per-command 正規化は維持 — **`CreateInstance` dispatch 内の `factory.descriptor()`（plugin_id 照合）/`factory.create()` も `catch_unwind` で guard し、panic は `Reply{i, seq, Err(PluginFailed)}` で返して process-exit させない**（mid-serve の exit は同居 sibling instance を全て殺す — spec AC4/AC6・design L257。現行は pre-serve のみで `catch_unwind` → exit(1)（`export/lib.rs:134-137`/`167-171`）— mid-serve では `Scan` guard pattern（`export/lib.rs:412-419`）と同形を dispatch 内へ維持）。`events_scratch`（`export/lib.rs:396`）は `InstanceEntry` に移さず outer `Runner` に残す — 単一スレッド `serve` が全 dispatch で共有する scratch のため per-instance 化は不要。crate doc の argv/registry 記述を v2 へ更新
4. `crates/ruviform-plugin-export/src/shm.rs` + `crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs` — doc の `--ruviform-shm-key` 参照（shm.rs L8・bin doc L28-29「`--ruviform-plugin-id <id>` is required on spawn; `--ruviform-shm-key <key>` is always required」）を `CreateInstance` payload 経路へ更新（両方とも doc のみ）
5. `crates/ruviform-plugin-host/src/l1/mod.rs` — 単一 instance v2（monolithic `L1Instance` 維持・`instance: u32` field 追加・T1 時点では `0` 固定 = dedicated/capacity-1 経路）。argv から `--ruviform-shm-key`/`--ruviform-plugin-id` を除去し `<runner>` + `spec.args` のみへ。spawn handshake を `Hello → Scan{instance: NONE} → CreateInstance{0, spec.descriptor.id, shm_key, in_depth, out_depth}` へ変更。`execute`/`request` は envelope `{seq, instance, cmd}` 化 — 動詞→scope 写像は `Scan` → `WIRE_INSTANCE_NONE`・残り動詞 → `self.instance`（design L341 の写像 — envelope は `request`/`execute` が付ける）。`Destroy` → `DestroyInstance{i}` → `Shutdown` + reap。reader は Reply/BlockDone の `instance` を parse + `reply.instance == waiter.instance` を照合（`Scan` waiter は `instance = NONE` で登録されるため `self.instance` との単純一致ではない — design L190。不一致 = 帰属不能 → 子 scope 違反として converge）。`Waiter{instance, tx}` に instance を記録 + reply 照合。`block_in_flight` は `HashSet<u64>` 維持（単一 instance で一意・`HashSet<(u32,u64)>` 化は T2）。`Scan`-verified id で `Err(UnknownPluginId)` が返った場合は `Err(Ipc)` へ fold（design どおり）
6. `crates/ruviform-plugin-host/src/composite.rs` — `expected_reply_kind`/`host_reply` で `CreateInstance`/`DestroyInstance` → `Reply::Ok` を explicit pin（wildcard でも正しいが明示）。`Route`/`to_wire` は本 task では変更なし（pool/isolation は T3）
7. `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` — wire v2 serve + registry（export runner と同形の `instances` map・`CreateInstance`/`DestroyInstance`/`InvalidHandle` dispatch）。子 scope fault 注入 knob（crash/hang/reject/stop-reading/leave-heir/hello-version/fail-stop-processing）は現行語彙を維持。argv v2（`--ruviform-*` 除去 → usage reject 維持）+ argv doc 表（bin doc L16-17）・`Flags::shm_key` doc（L92）・`Segment` doc（L214）の `--ruviform-shm-key` 記述を `CreateInstance` payload 経路へ更新。NOTE: 現行 `parse_flags`（`l1-test-worker.rs:96-138`）は未知 arg を**黙殺**する — `--ruviform-*` 受信の usage reject（exit 2）は新規挙動であり追加が必要。また dispatch wildcard（`export/lib.rs:451`・`l1-test-worker.rs:209`）は未知動詞を `Err(Unsupported)` で黙って捌くため、`CreateInstance`/`DestroyInstance` arm の実装漏れはコンパイルで検出できない — 両 bin で明示 arm を実装する
8. `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` — `MiniHost` を v2 化（per-instance segment create + `CreateInstance` 駆動・envelope frame）。registry bundle E2E 追加: 同一子へ複数 `CreateInstance`・`InvalidHandle` flagged reply・`InstanceLimit`（64 超 CreateInstance）・dup `CreateInstance`→`Err(Ipc)`・`DestroyInstance` 後の `InvalidHandle`・全 entry normalize exit・**`CreateInstance` 内 `factory.descriptor()` panic（`RUVIFORM_PANIC_DESCRIPTOR` env-gated `PanicFactory` — `ruviform-plugin-l1-runner.rs:226-243`）→ `Reply{i,seq,Err(PluginFailed)}` + sibling instance 生存**（process-exit しない — AC4/AC6）。既存 test の意味論変更（shape 変換で済まない 3 点）: (a) `e2e_argv_rejections`（`:512-537`）— `spawn(&[])` は v2 で **valid**（argv 不要 — shm key/plugin id は `CreateInstance` payload）のため exit 2 assert を zero-argv → Hello+serve に書き換え、unknown arg → exit 2・`--ruviform-*` 受信 → usage reject exit 2 を新たに pin。(b) `e2e_shm_attach_failure`/`e2e_shm_header_mismatch`/`e2e_shm_header_wire_version_mismatch`（`:541-580`）— attach/header 検証が spawn-argv 経路から `CreateInstance` dispatch へ移るため「`Hello` + 非ゼロ exit」assert を「`Reply{i,seq,Err(Ipc)}` + 子生存（serving 継続）」へ書き換え。(c) worker の `--crash-after-msgs=N`/`--hang-after-msgs=N` 系 message count が +1 shift — v2 handshake で `CreateInstance` frame が index 2 に入る（Scan=1・CreateInstance=2・以後 instance 動詞）ため、`l1_proxy_e2e` 側（`:223`/`:266`/`:312`/`:354`/`:816` 等）と本 suite の全 N 値を機械保持せず再導出する
9. `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` — v2 機械更新（`L1Instance::spawn` の新 handshake 経路・envelope 観測・既存 dedicated 経路の非回帰確認）。`--crash-after-msgs=N`/`--hang-after-msgs=N`/`--stop-reading-after-msgs=N`/`--leave-heir-after-msgs=N` の全 N 値は item 8(c) の +1 shift に従い再導出

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（本 task は T1 — plan の atomic cutover 節が範囲根拠）
- [../designs/issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md) L115-197（wire v2 契約 — envelope・新動詞・scope 規則・`InstanceLimit`・handle minting・seq/correlation）・L252-259（runner registry・`CreateInstance` 検証 chain・argv v2）・L336-364（inventory）
- [../specs/issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md) AC1/AC6/partial AC7
- [../adr/issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)（`InvalidHandle`・defined rejection の語彙）
- [../specs/issue-119-l1-wire-version-policy.md](../specs/issue-119-l1-wire-version-policy.md)（wholesale bump・単一最新版方針）
- [issue-117-wire-v2-codec-tests.md](../test-specs/issue-117-wire-v2-codec-tests.md)（本 task の oracle）
- [plugin-l1-wire-codec-tests.md](../test-specs/plugin-l1-wire-codec-tests.md)・[plugin-l1-runner-e2e-tests.md](../test-specs/plugin-l1-runner-e2e-tests.md)（v2 修正 oracle — T5 で本文書更新・本 task はコード側を先に実装）

## Dependencies

- 先行タスク: なし — plan T1 は全後続の基盤
- 前提文書: `docs/designs/issue-117-bundle-child-process.md` approved・`docs/specs/issue-119-l1-wire-version-policy.md` approved

## Affected Files

- `crates/ruviform-plugin-core/src/wire.rs`・`crates/ruviform-plugin-core/src/boundary.rs` — codec・定数・新動詞・`InstanceLimit`
- `crates/ruviform-plugin-export/src/lib.rs`・`crates/ruviform-plugin-export/src/shm.rs`・`crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs` — runner registry・dispatch・argv v2・shm/bin doc
- `crates/ruviform-plugin-host/src/l1/mod.rs`・`crates/ruviform-plugin-host/src/composite.rs` — host 単一 instance v2・reply-kind pin
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` — worker registry・argv v2
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`・`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` — harness v2・MiniHost bundle E2E

## Done When

- [ ] `WIRE_VERSION == 2`・`WIRE_MAX_INSTANCES == 64`・`WIRE_INSTANCE_NONE == u32::MAX` が codec に存在し、`Command`/`Reply`/`BlockIn`/`BlockDone` payload が `[u64 seq][u32 instance][rest]` を使う
- [ ] `CreateInstance`(tag 10)・`DestroyInstance`(tag 11) が encode/decode され、scope 判定表（Scan↔NONE・instance 動詞↔non-NONE・境界値）が `Malformed` で拒否される
- [ ] `HostError::InstanceLimit` が boundary + wire tag 9 の両方向で round-trip し、wildcard arm によって `Malformed` へ潰れない
- [ ] runner/worker が `instances` registry を持ち、`CreateInstance` 検証 chain（UnknownPluginId・dup→Ipc・InstanceLimit・attach/depth→Ipc）と未登録 `InvalidHandle` flagged reply・verbatim echo が動作し、`CreateInstance` dispatch 内の `factory.descriptor()`/`factory.create()` panic が `Reply{i,seq,Err(PluginFailed)}` で正規化され（process-exit しない）子・sibling instance が生存する
- [ ] host が argv `--ruviform-*` を送らず、`Hello→Scan→CreateInstance` handshake で v2 子を spawn し、envelope `instance` を全 frame に付ける
- [ ] MiniHost が per-instance segment + `CreateInstance` を駆動し、runner bundle E2E（multi-instance・InvalidHandle・InstanceLimit・dup）が pass する
- [ ] 既存 `l1_proxy_e2e` dedicated suite が v2 経路で green に留まる（T1 時点の非回帰証拠）

## Verification

- [ ] `cargo test -p ruviform-plugin-core` — wire codec unit test（round-trip・scope 行列・InstanceLimit）が pass
- [ ] `cargo test -p ruviform-plugin-export --test l1_runner_e2e` — MiniHost v2 + registry bundle E2E が pass
- [ ] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` — dedicated v2 経路が pass
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass
- [ ] `--ruviform-` の **code site** が残存しない — `l1/mod.rs` の `.arg("--ruviform-*")` 呼出し（`:994,996`）・`export/lib.rs` の `USAGE` const（`:99`）・`RunnerArgs`/`ArgvReject` argv-parse literal・worker `parse_flags` の bootstrap flag 受理・`MiniHost::spawn_runner` の argv pair が全て除去/書換済み。doc-level の `--ruviform-` 言及（T1 file 内: `export/lib.rs` crate doc・`RunnerArgs`/`run` doc・`export/shm.rs`・runner/worker bin doc・`l1/mod.rs:140,145` SpawnSpec doc・`composite.rs:75` — items 3/4/5/7 で更新）は T1 の範囲で処理し、残る doc 言及（`l1/shm.rs` — T2、`export/Cargo.toml`・adapter bin doc・README×3 — T5、`l1/mod.rs:28`/`l1/data.rs:926` — T2）は本 task の gate 対象外とする（全量 sweep は T5/T6 の gate）
- [ ] `grep -rn "WIRE_VERSION" crates/` で `1` が historical 注記以外にない
