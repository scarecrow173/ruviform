---
id: "IMPL-034LT7uqDVQ21ywpL4up76"
type: "impl"
status: "completed"
title: "Issue #76 plugin-export crate + L1 runner — Task 2 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-export/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-export/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "docs/impl/ir/issue-76-plugin-export-task-2-implementation.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/plugin-export-crate-and-l1-runner.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/plans/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/plugin-export-crate-and-l1-runner.md"
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

# Issue #76 plugin-export crate + L1 runner — Task 2 implementation

## Summary

Issue #76 Task 2 完了 — actions 5–8（`ruviform-plugin-export` クレート
新設 + `run()` ランナー + `l1-runner` bin + spawn E2E）を全範囲実施。
子プロセス側の薄いランタイムとして唯一の公開 API `run(Vec<Box<dyn
PluginFactory>>) -> ExitCode` を実装し、argv 規約・`Hello` handshake・
shm attach + header 検証・全 10 wire 動詞の serve loop・L0 完全一致の
lifecycle 意味論・catch_unwind による panic→`PluginFailed`（子生存）を
提供する。参照 runner `ruviform-plugin-l1-runner` は `GainPlugin`・
`PanicPlugin`・`UnderwritePlugin` の 3 fixture factory を内蔵し、
`tests/l1_runner_e2e.rs` の 11 E2E が実 `CARGO_BIN_EXE` 子プロセスで
全経路を検証。レビュー修正群（guarded `descriptor()` panic・
`staging_out` 全要素再ゼロ化・argv exit-code-2 pin・bounded inbound
label・shm 定数の `wire::shm` 集約）を全て適用済み。全 gate green
実 log。

## Context

approved plan `docs/plans/implement-issue-76-plugin-l1-out-of-process.md`
の Task 2（`docs/tasks/plugin-export-crate-and-l1-runner.md`・
actions 5–8）。upstream: approved spec/design
（`docs/specs/plugin-l1-out-of-process.md`・
`docs/designs/plugin-l1-out-of-process.md`）・Task 1 IR
（`docs/impl/ir/issue-76-l1-core-wire-task-1-implementation.md` —
wire codec・`ErasedPlugin`・`Lifecycle`・`HostError` の共有語彙は
Task 1 成果物をそのまま消費）・TSpec
`docs/test-specs/plugin-l1-runner-e2e-tests.md`（runner E2E の正常系/
異常系の正本）・`docs/test-specs/plugin-l1-wire-codec-tests.md`。本
task は L1 リンクの子側半分 — Task 3（plugin-host `l1` proxy 制御面）
が spawn/handshake/クラッシュ検出の親側を実装し、Task 4 が shm
データ面の slot 所有・RT 脚を追加する。

## Implementation

### Changes

- `crates/ruviform-plugin-export/`（新規クレート — `publish.workspace`
  継承・`[lints] workspace`）:
  - `Cargo.toml` — `ruviform-plugin-core`（path）+ `shared_memory
    = "0.12"`（plan「依存の確定」どおり — design の依存表は
    plugin-core のみを記載するが `--ruviform-shm-key` attach は
    shared_memory なしでは成立せず、自前 `unsafe` 代替は task doc
    で禁止 — host 側と同一の採用記録・隔離条件を適用。manifest
    コメントにこの判断を記録）+ dev-deps に `ruviform-audio-core`/
    `ruviform-media-core`（E2E の最小 host が `ProcessConfig`/
    `TransportInfo` を構築するため）。
  - `src/lib.rs`（787 行）— 唯一の公開 API `pub fn run(factories:
    Vec<Box<dyn PluginFactory>>) -> ExitCode`（`Box<dyn>` で
    アダプタ非依存・`#[must_use]`）+ 内部 `mod shm`:
    - argv 規約 — `--ruviform-shm-key <key>`（必須）・
      `--ruviform-plugin-id <id>`（複数 factory 時必須・単一
      factory では optional — deviation 後述）。不足・未知・
      duplicate・非 UTF-8 引数・plugin-id 不一致は
      `ArgvReject::Usage` → exit 2（`EXIT_USAGE`・`Hello` 送出前）。
      `factory.descriptor()` は plugin code のため
      `catch_unwind` で guard し panic は `ArgvReject::Panic` →
      exit 1（`create()` panic と同じ扱い — abort signature 101
      にしない）。
    - `Hello { version: WIRE_VERSION, capabilities: 0 }` を最初の
      frame として shm attach 前に送出 — attach 失敗は親から
      `Hello` + EOF として観測される。
    - `serve` — シングルスレッド frame loop: `Command` →
      `dispatch` → `Reply { seq, result }`・`BlockIn { seq }` →
      `Lifecycle` の `Process` 受理確認後 `process_block`（成功は
      `BlockDone { seq, result }`・`Err(HostError)` は
      `Reply { seq, Err(..) }` で返す — BlockIn 拒否は
      `BlockDone` ではなく `Reply` seq 空間に載る契約）・
      `Shutdown`/クリーン EOF → `normalize`（best-effort
      lifecycle 正規化）→ exit 0。inbound `Hello`/`Reply`/
      `BlockDone` は protocol violation → exit 1（診断は bounded
      な `inbound_label` — kind 名 + seq のみ・payload は dump
      しない）。
    - `Runner::dispatch` — 全 10 wire 動詞を `ErasedPlugin` に
      マップ。lifecycle 意味論は L0 `InProcessHost` と一致:
      `Initialize`/`Activate` は validate-first・失敗時
      `InitFailed`/`ActivateFailed` へ報告遷移・`StartProcessing`
      失敗は `Activated` へ rollback・`StopProcessing` 失敗は
      advisory。`Activate` は `max_frames > WIRE_MAX_FRAMES` を
      `Prepare(UnsupportedConfig)` で拒否。`Scan` は
      `factory.descriptor()` を guarded に `wire::Descriptor` 化
      （panic → `PluginFailed`・子生存）。未知動詞（
      `#[non_exhaustive]` の `_` arm）は `Err(Unsupported)`。
    - `guarded` — 全 hook/`process_host` を `catch_unwind
      (AssertUnwindSafe)` 下で実行し panic を
      `Err(HostError::PluginFailed)` に写像 — 子プロセスを殺さず
      dispatch を継続する（design の確定契約）。
    - `process_block` — doorbell `seq` で `segment.in_slot(seq)`
      を parse/検証（slot `seq` echo・`frames`/`channels`/
      `event_count`/`flags` の `WIRE_MAX_*` 上限・
      `activated_max_frames` 上限）→ transport/event 復号 →
      `staging_in` へ planar `f32` 確保 → `SlotInView`/
      `SlotOutView`（`BufferView`/`BufferViewMut` の借用
      newtype）→ `process_host`（panic guard 下）→
      `write_out_slot` で `ProcessResult` + samples 書戻し。
      `staging_out` は毎 block `fill(0.0)` で全要素再ゼロ化
      （`resize` は成長分のみゼロ化のため — 合法 under-write が
      前 block の sample を out-slot へ漏らさない・レビュー修正）。
    - frame I/O — `read_frame` は `[u32 LE payload_len]` を先に
      `WIRE_MAX_PAYLOAD` で拒否してから確保（bounded・部分消費
      なし・hang なし）・`send` は共有 `encode_buf` を再利用する
      1 alloc/1 frame + flush。`compiler_fence`（Acquire before
      slot read・Release before `BlockDone`）で doorbell 境界を
      明示 — 正式な cross-process ordering 契約は Task 4 の
      データ面で確定する（follow-up 記録）。
  - `src/shm.rs`（265 行）— `shared_memory` を閉じ込める内部
    モジュール（公開 API・feature 名に露出しない）。`Segment::
    attach` は `ShmemConf::os_id(key).open()` のみ（作成・unlink
    しない — 所有は親）+ 28B header 全フィールド検証（magic
    `"RVL1"`・`wire_version`・depth 非ゼロ・`slot_bytes`・
    `in_off`/`out_off`）+ 宣言 layout coverage 検査（`==` では
    なく `≥` — Windows の `VirtualQuery` は 64 KiB 割当粒度で
    切上げた mapping 長を返す・deviation 後述）。`in_slot` は
    slot の逐次検査付き parse・`write_out_slot` は scalars +
    `ProcessResult` + samples を書込む。レイアウト定数は全て
    `wire::shm` から消費（再宣言なし）。
  - `src/bin/ruviform-plugin-l1-runner.rs`（329 行）— 参照
    runner・3 factory 内蔵:
    - `GainPlugin`（`dev.ruviform.mock.gain`）— 決定的 gain モック
      （planar `f32` × normalized `ParamValue` event の
      denormalize・block 内 latest-wins・全 `Plugin` hook 実装・
      `save_state`/`restore_state` は `"RFG1"` magic + `f32` LE）。
    - `PanicPlugin`（`dev.ruviform.mock.panic`）— `init` panic
      fixture（panic → `PluginFailed` + 子生存の検証用）+
      `PanicFactory::descriptor` が `RUVIFORM_PANIC_DESCRIPTOR`
      env で panic する fixture（argv 解決 → exit 1・`Scan` →
      `PluginFailed` の guarded `descriptor()` 経路検証用）。
    - `UnderwritePlugin`（`dev.ruviform.mock.underwrite`）— 初回
      `process` は全 channel を書き以後は ch0 のみ —
      `staging_out` 再ゼロ化の正当性を検証する fixture（合法
      under-write が前 block を漏らさない）。
  - `tests/l1_runner_e2e.rs`（740 行・11 tests）—
    `env!("CARGO_BIN_EXE_ruviform-plugin-l1-runner")` で実子
    プロセスを spawn（`tests/` 内に閉じる — cross-package bin
    解決に依存しない）+ test 側最小 host（codec/frame/shm
    segment 作成・in/out slot 組立を test 側で実装）:
    - `e2e_happy_path` — `Hello` → `Scan`（`Scanned` の
      `wire::Descriptor` が `GainPlugin` descriptor と `matches`
      フィールド等価）→ `Initialize`/`Activate`/`StartProcessing`
      → shm in-slot 既知入力 → `BlockIn` → `BlockDone` + out-slot
      gain 正当性 → `SaveState`/`RestoreState` 往復 →
      `StopProcessing`/`Deactivate` → `Shutdown` exit 0。
    - 異常系 — `e2e_blockin_requires_processing`（非 Processing
      `BlockIn` → `Reply { seq, Err(Lifecycle(_)) }`）・
      `e2e_argv_rejections`（引数不足/未知/重複/plugin-id 不一致 →
      exit 2 pin）・`e2e_shm_attach_failure`・
      `e2e_shm_header_mismatch`（bad magic → exit 1 + Hello + EOF
      観測）・`e2e_stdin_eof_exits_zero`・
      `e2e_version_mismatch_is_a_parent_side_handshake_failure`
      （test-spec どおり親側判定の契約 — 子は WIRE_VERSION を
      正直に送る）・`e2e_plugin_panic_is_plugin_failed_and_
      child_survives`・`e2e_descriptor_panic_at_argv_exits_failure`
      （env-gated panic → exit 1）・
      `e2e_descriptor_panic_at_scan_is_plugin_failed`・
      `e2e_underwritten_channels_are_zero_filled`。
  - `README.md` — crate 索引どおりの責務・argv 契約・終了コード
    表・設計境界（非 RT・`shared_memory` 封じ込め・scoped unsafe
    例外）・層構造。
- `crates/ruviform-plugin-core/src/wire.rs`（+155 行）— `pub mod
  wire::shm` 新設: `[header][in-slots][out-slots]` セグメントと
  slot record の **authoritative wire-side 定義**（`MAGIC`/
  `HEADER_LEN`/`OFF_*`/`FLAG_TRANSPORT`/slot field offsets/
  `TRANSPORT_LEN`/`RESULT_LEN`/`EVENT_WIRE_SIZE`/`SAMPLE_STRIDE`/
  `SAMPLES_LEN`/`SLOT_BYTES`/`IN_SLOTS_OFF`/`out_slots_off`/
  `segment_len`）。shm layout は wire data のため wire 契約所有者
  が定義する — runner `shm.rs`（本 task）と host `l1::shm`
  （Task 3）・e2e の 3 箇所に手同期されていた定数群を単一正本化
  （Task 3 review で確定した集約の runner 側半分 — export はこの
  commit で `wire::shm` 消費へ移行済み）。`shm_layout_offsets_
  are_self_consistent` テストで offset 連鎖の自己整合を const/
  assert 検証。
- root `Cargo.toml` — `members` += `crates/ruviform-plugin-export`
  （層内順序: 拡張層 plugin-* の末尾・Host 層の直前）+ 先頭
  コメントのクレート列記同期（`+ plugin-export（Issue #76 新設）`）。
- `crates/README.md` — 索引行追加・計 20 → 21 クレート。
- `Cargo.lock` — `shared_memory 0.12.4` + transitive（`nix 0.23`・
  `rand 0.8`・`bitflags 1.3.2`・`win-sys`・`windows 0.34` 系等）+
  `ruviform-plugin-export` エントリ。

### Review findings and resolutions（独立レビュー + fix — mutation 検証済み）

- **guarded `descriptor()` panics（fixed）** — `Scan` dispatch と
  argv 解決の `factory.descriptor()` が unguarded だった: plugin
  code の panic が子プロセスを abort signature で殺し得た。
  `dispatch::Scan` を `catch_unwind` → `Err(PluginFailed)` に、
  argv 解決を `ArgvReject::Panic` → exit 1 に変更 +
  `PanicFactory::descriptor` の `RUVIFORM_PANIC_DESCRIPTOR`
  env-gated fixture と 2 E2E で両経路を実検証。
- **`staging_out` 全要素再ゼロ化（fixed）** — `resize(…, 0.0)` は
  成長要素のみゼロ化するため、縮小→再拡大で前 block の sample が
  `SlotOutView` 経由で plugin 未記述領域から out-slot へ漏れ得た。
  `staging_out.fill(0.0)` を毎 block 追加 +
  `UnderwritePlugin` fixture + `e2e_underwritten_channels_are_
  zero_filled` で mutation 検証（fill 除去で失敗）。
- **argv exit-code-2 pin（fixed）** — usage error の exit code が
  実装と doc（crate doc・README の終了コード表）で暗黙一致のみ
  だった — `EXIT_USAGE = 2` const + `e2e_argv_rejections` で全
  usage 経路の code 2 を pin。
- **bounded inbound debug label（fixed）** — protocol violation の
  stderr 診断が `{message:?}` 的に payload を含み得た — 最大
  `Reply::State`（16 MiB）を丸ごと dump しないよう `inbound_
  label`（kind 名 + seq のみ）に変更。
- **shm 定数の `wire::shm` 集約（fixed）** — segment header/slot
  layout 定数が export `shm.rs`・test 組立・（Task 3 の）host
  `l1::shm` に手同期で三重化されていた — `wire::shm` pub モジュ
  ールを正本として plugin-core に新設し、export 側は全て
  `wire::shm` 消費へ移行（host 側は Task 3 で同移行 — 本 commit
  が layout 正本を供給）。
- **runner doc 精度（fixed）** — crate doc の exit semantics/
  dispatch semantics と実装の差異（`descriptor()` guard・
  `BlockIn` 拒否の `Reply` 経路・bounded label）を実態に同期。

### Decisions During Implementation

- **`shared_memory` を export にも採用**（plan-sanctioned・design
  の依存表からの明示的拡張）— design の子側依存表は plugin-core
  のみだったが `--ruviform-shm-key` attach は shared_memory なし
  では成立せず、自前 `unsafe`（`shm_open`/`MapViewOfFile` 直叩き）
  は task doc が禁止。host と同一の採用記録・隔離条件（MIT OR
  Apache-2.0・内部 `shm` モジュール閉じ込め・公開 API/feature 名
  非露出）を適用し manifest コメントに記録。
- **`shared_memory` 0.12.4 の slice accessor は `unsafe fn`**
  （deviation・採用条件の事実修正）— design は `as_slice`/
  `as_mut_slice` を safe API と記録し「自前 unsafe ゼロ」を採用
  条件としたが、実効 API（0.12.4・latest 2022-03）は `unsafe fn
  as_slice`/`as_slice_mut` で safe accessor は存在しない。
  `bytes`/`bytes_mut` の 2 箇所に scoped `#[expect(unsafe_code)]`
  + SAFETY コメント（per-region 排他性: header は owner が spawn
  前に一度だけ書き子は read のみ・slot は strict-alternation
  doorbell で一度に片側のみ・`&mut Shmem` borrow が単一
  スレッド内排他を担保。whole-mapping slice 取得が caveat と
  して残る旨を SAFETY コメントに記録）で局所許可 — 代替の safe
  経路は存在しない。lead へ採用記録更新として報告済み（crate
  doc・`shm.rs` module doc・Cargo.toml コメント・README に明記）。
- **segment 長は `==` ではなく `≥` で coverage 検査**（deviation
  — Windows 実態）— Windows の `Shmem` は `VirtualQuery` の
  region size（64 KiB 割当粒度で切上げ）を mapping 長として返す
  ため declared layout との一致比較は fail する。不変条件は
  「mapping が宣言 layout を cover する」なので `≥` に変更
  （POSIX では事実上 equality — `shm.rs` doc に記録）。
- **`--ruviform-plugin-id` は単一 factory バイナリで optional**
  （deviation・deliberate）— design 記述は常時必須の読みが可能
  だったが、factory が 1 つの binary では選択肢が 1 つしかなく
  必須化は利用側の純粋な負担。`factories.len() == 1` → index 0・
  `!= 1` → 必須（0/複数では usage 拒否）とした。多 factory
  バイナリ（参照 runner は 3 内蔵）では依然必須。
- **`BlockIn` 拒否は `Reply { seq, Err(..) }` に載る**（deliberate
  — follow-up 記録）— `BlockDone` は `ProcessResult` のみを運び
  `HostError` を持たないため、lifecycle/slot 検証の defined
  rejection は `Reply` の seq 空間で返す。doorbell seq 空間と
  command seq 空間の disjoint 化は Task 4 の seq-space partition
  で確定する（`BlockIn` arm に TODO 記録 — 両空間が交差すると
  `BlockDone`/`Reply` correlation が command waiter と alias
  し得る）。
- **panic の catch 範囲は plugin code のみ** — wire codec・shm
  parse・staging など runner 自身のコードは catch しない
  （runner の defect は process death — 親の crash 検出が
  `Failed` に収束する別系統）。plugin hook/`descriptor()`/
  `create()`/`process_host` のみ `guarded`。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — `shared_memory` 以外の shm attach 代替（自前 unsafe）は
task doc で明示禁止済み（新規 experiment なし）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean — 2 scoped
    `#[expect(unsafe_code)]` は局所許可・他の unsafe なし）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **55 test suites 全 pass・0 fail** — `l1_runner_e2e` 11 tests
    含む・既存 L0/wire テスト無退行）
  - `mise run test-rt` — exit 0（`rt_` 名前フィルタ既存レーン
    無退行 — export は非 RT のみ・新規 `rt_*` 対象なし）
- 構造確認（実コマンド）:
  - `cargo tree -p ruviform-plugin-export` — 依存は
    `ruviform-plugin-core` + `shared_memory`（+ transitive）のみ
    （dev-deps: audio-core/media-core）
  - `rg -n "unsafe" crates/ruviform-plugin-export/src` —
    `shm.rs` の 2 scoped `#[expect(unsafe_code)]` call site のみ
    （`bytes`/`bytes_mut` — 各 SAFETY コメント付き）
  - `rg -n "pub " crates/ruviform-plugin-export/src/lib.rs` —
    公開 API は `run` のみ（`mod shm` は private）
- レビュー修正の mutation 検証（実施済み）:
  - `staging_out.fill(0.0)` 除去で `e2e_underwritten_channels_
    are_zero_filled` 失敗
  - `descriptor()` guard 除去で `e2e_descriptor_panic_at_*` が
    abort signature（観測 exit 変化）で失敗

## Non-Goals

- proxy/host 側機構（spawn・handshake・`Failed` 収束・通知）—
  Task 3。shm slot の producer 側（in-slot 記述・out-slot 読取り）
  ・RT 脚・`process` 等価フロー — Task 4。`PluginHost`
  composite・`register_out_of_process` 公開 API — Task 5。
- 実プラグイン（CLAP/VST3/OFX）の L1 組込み — `GainPlugin` 等の
  モックで検証（adapter 側の `run` 呼出し配線は adapter task）。
- bundle 子（handle 修飾）・wire バージョン進化・stderr 配管・
  自動再起動・shm segment の作成/所有（runner は attach/検証のみ
  — owner は host `l1::shm`）。
- doorbell `seq` 空間と command `seq` 空間の disjoint 化 —
  Task 4 の seq-space partition で確定（`BlockIn` arm に TODO
  記録。現状は runner が `Reply`/`BlockDone` を着信 seq で
  そのまま返すため親側 correlation は成立するが、両空間の衝突
  規則は未定義 — Task 4 の設計論点）。
- `WIRE_MAX_FRAMES`/`WIRE_MAX_CHANNELS` の shm slot 強制は
  `in_slot` 検証で実施済みだが、`Activate` config の
  frames/channels 意味的上限全体は Task 4 のデータ面と共に
  再評価（plan action 12）。

## Risks

- `shared_memory` 0.12.4 は 2022-03 止まりの停滞クレート —
  `unsafe fn` accessor の事実を含め、隔離条件（内部 `shm`
  モジュール閉じ込め）で封じ込め済みだが、将来の代替 crate
  移行は両端（export/host）の `shm` モジュール交換で済む構造に
  なっている（採用記録の更新は lead に報告済み）。
- `whole-mapping` slice — `as_slice`/`as_slice_mut` は mapping
  全体の slice を返すため、SAFETY 論証は「同時に触れるのは
  doorbell で排他された slot のみ」の protocol 依存（slot
  外への書込み自体は別 process の read と競合し得る）。
  Task 4 で slot accessor を region-scoped に狭める余地あり
  （follow-up）。
- `compiler_fence` は compiler-level barrier のみ — cross-process
  の memory ordering は OS の named-mapping 一貫性と doorbell の
  happens-before に依る。現状の fence は writer/reader 側の
  compiler reordering 抑止として正直に置いたもので、正式な
  ordering 契約は Task 4 のデータ面設計で確定する（follow-up）。
- `AssertUnwindSafe` — panic した plugin は内部状態が不整合に
  なり得るが、`PluginFailed` 契約上 runner は dispatch を継続
  する（documented contract — 子は壊れた plugin を detach
  できず、親の lifecycle `Failed` 注入が真の回収経路）。
- `Runnner::dispatch` の GUI 動詞（`OpenGui`/`CloseGui`）は
  `ErasedPlugin` hook へ到達する — host 側 proxy が定義済み
  `Unsupported` 拒否で転送しないため到達自体が protocol
  違反だが、子側も hook に委譲し plugin の `Unsupported`
  default が返る（二重防御・design どおり）。

## Follow-ups

- Task 3 `docs/tasks/l1-proxy-control-plane.md`（actions 9–11）
  — `L1Instance` proxy が本 task の argv 規約・`Hello`/`Scan`
  handshake・`Reply`/`BlockDone` 意味論の親側を実装する最初の
  消費者。`wire::shm` 定数は本 task が正本化済み — host
  `l1::shm` は同じ `wire::shm` を消費する。
- Task 4 `docs/tasks/l1-shm-data-plane-and-rt-legs.md`（actions
  12–14）— seq-space partition（`BlockIn` arm の TODO）・slot
  producer 側・doorbell fence の正式 ordering 契約・RT try-legs・
  `process` 等価。`BlockIn` 拒否が `Reply { seq }` に載る現行
  契約との整合もここで確定する。
- `compiler_fence` note — export `serve` の Acquire/Release は
  compiler barrier として置いた最小限。Task 4 で cross-process
  ordering の正式契約（fence 種別・mapping 一貫性の依存記述）
  を確定すること。
- frame helper dedup — `read_frame`/`send`（export lib.rs）・
  `read_message`/`write_message`（host `l1/mod.rs`・
  `l1-test-worker.rs`・両 e2e test）が同形状に 4 箇所散在 —
  wire 層への共通 helper 格上げ or test-only dedup を Task 4 以降
  で評価（現状は各 crate の単純な必要最小限コピー）。
- `wire::Descriptor::matches` 順序感度 — Task 1 IR の follow-up
  継続（本 task の E2E は runner が正しく同一 descriptor を返す
  検証であり、順序違いの false negative 影響は Task 3 proxy 照合
  仕様と共に再評価）。
- `as_slice`/`as_slice_mut` の region-scoped 化 — Task 4 で slot
  accessor を subrange 化する際、whole-mapping slice 取得の
  caveat を SAFETY 論証から外せるか再評価。
