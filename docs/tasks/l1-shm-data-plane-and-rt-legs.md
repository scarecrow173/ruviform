---
id: "TASK-034LNYmL2ezSeFFgddaeDX"
type: "task"
status: "todo"
title: "L1 共有メモリデータ面 + RT 脚 + pass-through"
created: "2026-10-25"
updated: "2026-10-25"
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
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/tasks/l1-proxy-control-plane.md"
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
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# L1 共有メモリデータ面 + RT 脚 + pass-through

## Work

approved plan `implement-issue-76-plugin-l1-out-of-process.md` の **actions 12–14 を 1 task の ordered substeps として実行**。task 3 の L1 プロキシに shm データ面（スロット配置・doorbell fence・staging）と `ruviform-rt` SPSC の RT 脚・`process` 等価フロー・pass-through を追加する。

**先行確認**: `ruviform_rt::channel` の API 形状（`Producer::try_push`/`Consumer::try_pop`・容量・型制約）・`assert_no_alloc` の測定窓形（fixture/warm-up を窓外）を既存 RT テストで確認 — design と相違なら blocker を記録。

### A12 — shm データ面（slot レイアウト・staging・doorbell fence）

- `src/l1/shm.rs` 拡張: slot レイアウト `{seq u64, frames u32, channels u32, event_count u32, flags, transport wire, result wire, events wire, samples f32[]}` — `WIRE_MAX_FRAMES = 8192`/`WIRE_MAX_CHANNELS = 16`/`WIRE_MAX_EVENTS = 1024`（slot ≈ 550 KB・D=3 ≈ 3.3 MB/インスタンス）
- `[header][in-slots][out-slots]` 固定レイアウト — shm に共有制御状態（head/tail・ロック・atomic 配置）を持たない・スロット選択は doorbell `seq`（`seq % D`）
- staging 事前確保: `Activate` で `max_frames × 総チャネル`・D = `lookahead_blocks + 1`・in/out 各を確保（RT パスに alloc を持ち込まない）
- `Activate` の `ProcessConfig` が上限超 → `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`（定義済み拒否）
- doorbell fence: writer `compiler_fence(Release)` 前・reader `compiler_fence(Acquire)` 後 — doorbell バイト列が release/acquire エッジ。厳密交互制御（producer は `done seq-D` 受領後にのみスロット再利用）・shm アクセスは `as_slice`/`as_mut_slice` のみ（自前 `unsafe` ゼロ）

### A13 — RT 脚 + `process` 等価フロー

- submit/result 両脚: `ruviform_rt::channel`（`u64` seq）— RT 側は `try_push`/`try_pop` のみ（pipe/shm/ブロッキング read は driver/reader 側に留まる）
- `process(handle, inputs, outputs, events, transport)` 等価フロー: (1) ローカル lifecycle ミラー `Processing` const 検査 → (2) 入力を `staging_in[seq % D]` へ複写・`submit.try_push(seq)`（満杯 → 投入放棄 → pass-through）→ (3) `expected = seq - lookahead`（underflow = 定義済み無音）→ (4) result ch を `expected` まで drain（`< expected` は stale 破棄・有界回数）→ (5) 一致 seq なら `staging_out` を `outputs` へ複写 + `last_output` 更新、なければ pass-through
- pass-through 規則: 健全インスタンスのアンダーラン → 直前に受信した出力ブロック（受信歴なし → 無音）・`Failed` → 無音 — いずれも `Ok(ProcessResult::CONTINUE)`・ハングしない。submit/result 満杯・応答遅延・無応答はエラーでなくフォールバック
- reader: `BlockDone` → `staging_out` 複写 + result `try_push`（RT 側の受信・複写は reader/driver 側 — RT スレッドは shm に触れない）

### A14 — RT レーン + データ面 E2E

- `rt_l1_process_path_no_alloc`: `assert_no_alloc` 窓で `process` の try 経路（staging 複写 + try_push + drain + staging_out 複写/pass-through）が 0 alloc — fixture/staging/warm-up は窓外
- `rt_l1_result_leg_try_ops`: result ch が `try_pop`/`try_push` のみを使う（blocking なし）
- `l1-test-worker`（`--hang-after-msgs`）で process 経路の gain 正当性・定義遅延（先行 2 ブロック・`seq - lookahead`）・crash/hang 時 pass-through（直前受信 or 無音・`Ok(CONTINUE)`・ハングなし）を E2E で検証
- `mise run test-rt` で `rt_l1_*` が実選択（0 件一致スキップを成功と偽らない — 一致件数を記録）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-76-plugin-l1-out-of-process.md](../plans/implement-issue-76-plugin-l1-out-of-process.md)
- [../designs/plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)
- [../test-specs/plugin-l1-proxy-contract-tests.md](../test-specs/plugin-l1-proxy-contract-tests.md)
- [../test-specs/rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)

## Dependencies

- 先行 task: [l1-proxy-control-plane.md](l1-proxy-control-plane.md) — spawn/driver/reader・shm segment・lifecycle ミラー・`Failed` 収束が前提
- `ruviform-rt`（SPSC `try_push`/`try_pop` — task 3 で dep 追加済み）・`assert_no_alloc`（dev-dep — Issue #22 レーン）

## Non-Goals

- `process` 経路の RT 化（RT は try 脚のみ — ブロック処理自体は非 RT）・video/NONRT プラグインの L1 経路
- `PluginHost` コンポジット公開面（後続 task）・shm 通知機構の多重化（doorbell はパイプ多重化で十分）
- `WIRE_MAX_*` 上限の引上げ・可変 slot・shm 内共有制御状態（head/tail・ロック — design 棄却）

## Affected Files

- `crates/ruviform-plugin-host/src/l1/mod.rs`（`process` 等価フロー・RT 脚・pass-through・`last_output`）
- `crates/ruviform-plugin-host/src/l1/shm.rs`（slot レイアウト・staging・fence）
- `crates/ruviform-plugin-host/src/lib.rs`（`process`/`lifecycle_state`/`kill_child` の L1 経路配線）
- `crates/ruviform-plugin-host/Cargo.toml`（`assert_no_alloc` dev-dep — Issue #22 形式）
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（process 経路・pass-through・`rt_l1_*` 追加）

## Completion Gate

4 entry 全 green 実 log（`check`/`lint`/`test`/`test-rt` — `test-rt` は `rt_l1_*` 実一致件数記録）+ lead review の後にのみ task 完了・buildable scope commit。review 焦点: doorbell fence（Release/Acquire `compiler_fence`）・shm に共有制御状態を持たない・厳密交互制御・RT 側が try_push/try_pop + bounded memcpy のみ（pipe/shm/ブロッキングなし）・pass-through 規則 — fake success なし。

## Verification

- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean）
- [ ] `mise run test` — exit 0（process 経路 E2E 全 pass — gain 正当性・定義遅延・crash/hang pass-through・ハングなし）
- [ ] `mise run test-rt` — exit 0 かつ `rt_l1_process_path_no_alloc`/`rt_l1_result_leg_try_ops` が実一致（0 件スキップではなく実選択 — 一致件数を記録）
- [ ] `rg -n "compiler_fence" crates/ruviform-plugin-host/src/l1` — doorbell Release/Acquire の存在
- [ ] `rg -n "atomic\|Mutex\|head\|tail" crates/ruviform-plugin-host/src/l1/shm.rs` — shm 内に共有制御状態がない（doorbell seq のみ）
