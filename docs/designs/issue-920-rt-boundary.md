---
id: "DESIGN-rf920-rt-boundary-01"
type: "design"
status: "approved"
title: "Issue #920 cross-thread 同一 instance RT pump 収束設計（L1 endpoint 経路）"
created: "2026-11-27"
updated: "2026-11-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/920"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-920-rt-boundary.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-874-rt-oracle.md"
      - type: "src"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "src"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "evidence"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
      - type: "evidence"
        path: "evidence/rf631/plugin-dock-real-headless.jsonl"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-874-rt-oracle.md"
    - "docs/designs/issue-872-l1-oos-plugin-editor-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-874-rt-oracle.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/rf631/"
  defers: []
  deferred-by: []
---

# Issue #920 cross-thread 同一 instance RT pump 収束設計（L1 endpoint 経路）

## Context

`rf631.plugin-dock-real` lane の `real-session-rt-boundary` case
は Issue #874 で architectural-boundary の正直記録として
verdict `Unsupported` に置かれた。当時の詳細:
`test_editor`/`test_fault` は `!Send`（`create_sendable` →
`None` → `Plain` slot）、`InProcessHost::attach_endpoint` は
`pub(crate)`、`PluginHost.l0` は private — 公開 API では同一
instance を RT-class スレッドから pump しながら UI スレッドで
session verbs を駆動する手段がなかった。

Issue #920 の設計問いは、この cross-thread 同一 instance pump
の可否を設計判断として確定し、(a) 契約拡張 / (b) 既存公開
API / (c) `Unsupported` 確定のいずれかで収束させること。

**結論: (b) を採用した。** `PluginHost::prepare_endpoint`
（`crates/ruviform-plugin-host/src/composite.rs` — `pub`）は
`Slot::OutOfProcess` の instance に対し `Send` な
`PluginEndpoint` を mint する。Issue #884/#872 で main に
統合済みのこの公開 API が、Issue #874 時点では存在しなかった
「同一 instance の RT leg を RT-class スレッドへ移送する」
正規の出口である。host/core/fixture の契約変更は不要で、
収束は dock-test crate 内の case 実装のみで完結した。

## 3 選択肢の評価

### (a) 契約拡張（GUI-capable `create_sendable` / L0 endpoint 公開）— 不採用

- `Vst3Plugin`/`ClapPluginInstance` は raw `!Send` 状態を保持
  し、`create_sendable` を正直に `Some` 化できない
  （vst3 `host.rs`、clap `host.rs` — fixture host 両経路とも
  `None` を返す）。
- L0 では `InProcessHost::attach_endpoint` が `pub(crate)`、
  `PluginHost.l0` が private で、`InProcessEditorOps`（docked
  session の ops）と endpoint mint を同一 instance に公開 API
  で両立できない。
- 実現には plugin-core/host の新公開契約＋fixture/proxy 側の
  対応が必要で、契約変更規模が本 slice を超える。
  `crates/ruviform-plugin-host/**`・
  `crates/ruviform-plugin-core/**`・
  `crates/ruviform-plugin-test-*/**` は当該 slice の lease 外で
  あることも判断材料 — (b) が公開 API で完結する以上、(a) の
  契約拡張は費用対効果を失う。

### (b) L1 経路の既存公開 API — 採用

- `PluginHost::prepare_endpoint` は `Slot::OutOfProcess` に
  `Send` な `PluginEndpoint` を mint する（`endpoint.rs` —
  `process`/`bounds`/`latency_frames` は RT-suitable 契約:
  呼び出し内で block しない・alloc しない）。
- `L1EditorOps` は同一 `PluginHost` 上の同一 instance に対し
  endpoint と併用できる — command channel（session 面）と
  detached RtLeg（data plane）は別 surface。
- `l1_proxy_e2e`（`crates/ruviform-plugin-host/tests/`）が
  detached endpoint 駆動・counter 追跡・checkout 中
  `EndpointUnavailable` → drop で leg 返還の先例を実証済み。
- 変更は `ruviform-plugin-dock-test` 内に閉じ、最小工数で
  cross-thread 同一 instance pump の観測を実現する。

### (c) `Unsupported` 確定（fallback）— 不採用

- docs のみの最小収束。(b) が実現可能と確認されたため
  fallback は発動しなかった。`Unsupported` 経路は
  capability-gated pre-flight として残る（後述）。

## 採用実装の設計

### 経路構成（`real_dock.rs` `oos_rt_boundary_body`）

1. lane init 済みの `OosRoute`（`probe_out_of_process_route`
   が session-probe した runner exe + `test_editor` descriptor）
   を `OutOfProcessSpec{isolation: Dedicated}` で
   `register_out_of_process` → `oos_instantiate`
   （`SpawnFailed{Timeout}` bounded retry は crash/hang 先例）。
2. `negotiated_gui_capabilities` の `GUI_FLOATING` bit を当該
   spawn で再確認（probe 時 claim と spawn 後 negotiated の
   乖離は `Fail`）。
3. `oos_drive_to_processing`（`Initialize` → `Activate`
   (48kHz, 1..=512) → `StartProcessing`）で `Processing` へ
   駆動し、`oos_drive_clean_blocks` で in-place leg の matched
   clean block baseline を採る（detached leg の照合基準）。
4. `host.prepare_endpoint(handle, EndpointBounds{max_frames:
   512, channels: 2})` で `Send` な `PluginEndpoint` を mint —
   `RtLeg` が instance から checkout される。この時点で
   `host.process` は defined `EndpointUnavailable` を、二重
   mint も同じく `EndpointUnavailable` を返すことを step 検証
   する（契約の実測、仮定しない）。
5. endpoint を `std::thread`（pump thread）へ move し、
   `oos_rt_pump` が activated block period（512 frames @
   48kHz ≈ 10.667ms）の cadence で `endpoint.process` を駆動
   する。停止は stop flag（`AtomicBool`）＋ call bound（1024
   calls）＋ wall bound（15s）の三重 bounded。
6. pump 駆動中、UI スレッドは同一 `PluginHost` 上の同一
   instance に `L1EditorOps` で session を駆動:
   `begin_session` → `open(AttachMode::Floating)` →
   `RuviformL1GuiContainer` 子 HWND oracle → `set_size` →
   `set_visibility(false)` → `set_visibility(true)` →
   `set_focus`。
7. pump 停止 → report を bounded recv（10s join bound — 各 call
   が返ったことの証明）→ endpoint は pump 関数の return で
   drop され leg が返還 channel へ → 次の `&mut host` entry で
   drain され `host.process` が `Ok(Continue)` を再開するまで
   step 検証。
8. `close` で session を閉じ、floating container 消滅 oracle
   （`oos_wait_floating_gone` — `oos_observe_floating_window`
   の逆写像）と `check_session_converged` の exact event
   stream 照合（driven verbs が emit した `Hidden`/`Shown` のみ
   を期待値に含める正直構成）で収束。
9. `StopProcessing` → `Deactivate` → `Destroy` → slot freed →
   `Scan`（host 生存）→ lane window alive/settle の
   lane-survival 系は crash/hang と同型。

### RT 境界規律の観測内容（oracle として何を測るか）

- **non-blocking**: 各 `endpoint.process` call の measured
  elapsed が全て gross bound（250ms）未満であること、および
  pump report が join bound 内に届くこと。L1 leg の
  `try_push`/staging/`try_pop` は µs オーダーなので、ms 級の
  call は真の block を意味する — **これは「call が
  block しない」の検出器であり、RT deadline の主張ではない**
  （後述の正直な制約）。
- **deadline**: pump 全体が stop flag・call bound・wall bound
  のいずれかで確定的に終了すること。pump thread が wedge
  した場合は report 未到達 → `Fail`（吸収しない）。
- **no-alloc**: endpoint 契約として `process` の呼び出し経路は
  `in_scratch` 確保済み staging のみを使う。観測可能な代理と
  して、buffer/view は call ごとに stack 上に組み立て、loop 内
  で heap traffic を持たない pump 実装＋call 単位の elapsed
  観測を記録する（allocator counter ではなく contract
  citation + 測定として正直に記録）。
- **data-plane 成立**: `audio_counters` delta で pump window の
  submit を会計する — `dropped_submits`（backpressure）・
  `passthrough_submits`（最初の `lookahead_blocks` underflow
  含む defined substituted output）は記録し、`results_failed_
  observed`/`results_dropped` は 0 を要求、`calls - passthrough
  - dropped` を matched block の lower bound として > 0 を要求
  する（dropped submit は output も substitute し得るため真の
  下限; 実数を装わない）。
- **session 面の同時成立**: pump 駆動中に全 verb が応答を返す
  こと（plugin の defined `Unsupported` refusal も応答として
  正当に数える）、event stream が駆動した状態変化に厳密に
  一致すること、checkout 中 `host.process` →
  `EndpointUnavailable` / leg 返還後 `Ok(Continue)` の写像が
  実測されること。

### `L1EditorOps` の Floating 制約と case 意味論の変化

`L1EditorOps` は `AttachMode::Floating` のみを駆動する
（Docked は wire v6 で defined Unsupported — Issue #872 設計）。
そのため (b) 収束は case の意味論を **docked session から
L1 floating/OOS 経路へ re-scope** する — これは本 doc で
正直に記録する設計判断である。

- Issue #389 の「RT 非干渉」受入は docked session 上の観測を
  構成する行として書かれていた。(b) は「同一 instance を
  cross-thread で pump しながら session 面が成立する」という
  boundary 行の核心 — API 的に不可能と記録されていた排他
  主張の反証 — を L1 floating で立てる。docked session 特有の
  観測（`InProcessEditorOps` 経由の same-instance pump）は
  依然として (a) 契約が要り、lease 外のため本 slice では
  取らない。case 名は維持し、意味論の変化は本 doc・case
  detail・catalog module doc に正直に記録する（marker を
  `session:rt:cross-thread-boundary` から
  `session:rt:l1-endpoint` / `rt:pump-thread` /
  `l1:floating-session` へ置き換えたのもこの re-scope の
  表れ）。
- L0 docked 固有の観測を必須と判断する設計は (a) 寄りになる
  が、(a) は契約変更規模のため本 slice の範囲外 — coordinator
  gate に残す残課題として正直に記録する。

### 「RT-class pump thread」の正直な主張

pump は `std::thread` 上の通常優先度スレッドであり、真の
RT callback thread（MMCSS/Pro Audio priority）ではない。
Issue #874 doc が記録した boundary leg の欠落面（真の RT
callback thread との同時性）のうち、(b) で観測できたのは:

- endpoint の RT contract に従う `process` が cross-thread
  （host/session スレッドとは別の所有権・別スレッド）で駆動
  できること — かつて `!Send` が理由で不可能と記録された
  所有権境界の解消。
- 全 call の応答・bounded elapsed・counter 会計・session 面の
  同時成立。

観測できないのは scheduling latency / priority inversion 等の
真の RT timing 特性で、これは thread priority を上げる等の
装飾をせず残る欠落面として記録する。

## verdict 規律

- `required` = `[NativePluginDock, OutOfProcessPluginHost]` —
  route 未証明 host は `run_scenario` pre-flight の
  capability-gated `Unsupported`（crash/hang 行と同型）。
- headless host は `NotRun` + `environment_blocked`
  （`blocked_probe` 経路、`required` 欄は entry から自動追随）。
- plugin artifact 欠落 → `NotRun`+`plugin_error.prerequisite`、
  spawn/mint/pump/session/leg 返還/teardown の失敗 →
  `Fail` + step 記録。verdict を弱化する分岐は存在しない。

## Deferred Design Concerns

- L0 docked session 上の cross-thread pump（(a) 契約:
  GUI-capable `create_sendable` + in-process endpoint 公開）—
  契約変更規模・lease 外のため本 slice では不採用。#389 の
  docked 固有受入を構成するなら別 Issue で追跡する。
- 真の RT callback thread（RT priority）上の pump —
  `std::thread` pump が観測するのは endpoint contract の
  cross-thread 成立であり、RT timing 特性は残る欠落面。

## Verification Notes

- `mise run fmt-check`・`mise run check/lint/test
  ruviform-plugin-dock-test`（±`--features real-dock`）・
  `mise run test ruviform-ui-test`・`mise exec -- prek run
  --all-files`。
- `mise run test-ui-plugin-dock-real-headless`（`--blocked-probe`
  — required 更新で行の `required` 欄が変わるため JSONL 再生成
  必須、fail-closed lattice 非回帰の証拠）。
- `mise run test-ui-plugin-dock-real` — interactive Windows
  session でのみ `Pass` 証拠が採れる canonical lane。
  `evidence/rf631/plugin-dock-real-scenario.jsonl` は当該 lane
  実走でのみ再生成（手編集禁止）。
