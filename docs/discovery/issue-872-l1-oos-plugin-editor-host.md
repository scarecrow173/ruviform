---
id: "DISC-0351ujnK1guLRroTqT6pcS"
type: "discovery"
status: "resolved"
lifecycle: "draft"
title: "Issue #872 L1 out-of-process plugin editor host 調査記録"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/872"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/115"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-872-l1-oos-plugin-editor-host.md"
      - type: "doc"
        path: "docs/designs/issue-872-l1-oos-plugin-editor-host.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  motivates: []
  informs:
    - "docs/designs/issue-872-l1-oos-plugin-editor-host.md"
  supersedes: []
---

# Issue #872 L1 out-of-process plugin editor host 調査記録

## Summary

`real-session-crash`/`real-session-hang` が `Unsupported` の
まま残る原因を「in-process lane が `test_fault` の abort/
exit/hang に構造的に耐えられない」ことと確定し、解除には
L1 out-of-process editor host（wire GUI 動詞 + プロセス隔離）が
必要と結論づけた。#115 が確定した「越境埋め込みは脆弱/不可」
判定と lane 生存を両立させるため、editor は child 所有
toplevel（`AttachMode::Floating`）に限定する設計に至った。
確度: 高 — 全て実コード/approved 設計/evidence で確認済み
（正本は `docs/designs/issue-872-l1-oos-plugin-editor-host.md`、
本書は検討過程と却下案の記録）。

## Exploration Goal

#389 の real-dock lane で `real-session-crash`/`real-session-hang`
が `Unsupported` のまま残っている理由と、それを解除するために
必要な L1 plugin editor host の形を調査する。具体的には:

- なぜ現行 lane が crash/hang を観測できないのか（構造限界の
  正確な所在）
- L1 で editor GUI を実現するには wire/runner/host のどこに
  何が足りないか
- #115 が残した解除条件 (a)-(d) の充足状況と、docked 越境
  attach と lane 生存の両立可能性
- crash/hang を「観測できる形」で起動する fault 経路の有無
- verdict/capability 語彙をどう flip させるか

## Key Issues and Assumptions

- **in-process lane 非存続性の確定**: `test_fault` の
  abort/exit/hang は plugin の `process()` 内で発火する
  （`crates/ruviform-plugin-test-vst3/src/fault.rs`）。
  in-process lane では plugin cdylib が host process を共有
  するため、abort/exit は host ごと死に、hang は host ごと
  固まる — `real_dock.rs`:13235-13288 の `in-process-lane-
  nonviable` marker がこの構造限界を正直に記録している。
- **#115 の確定事実との整合**: 越境 `SetParent`/`WS_CHILD`
  は AttachThreadInput 系の入力キュー暗黙結合を伴い、child
  の hang が host UI thread へ波及する（approved design）。
  これをそのまま採ると「child の障害を観測して lane が
  生き残る」という lane の目的と矛盾する — 検証対象が
  観測者を殺す。
- **session 層は既に完成**: `EditorSessionOps`/
  `AttachMode::{Docked,Floating}`/`Terminated{HostLost}`/
  `notify_terminated`/`tick` は実装済みで、L1 側に必要なのは
  その下の transport（wire 動詞）と adapter（`L1EditorOps`）
  と runner 機構（UI thread）だけである。
- **L1 収束語彙は既に実証済み**: `try_wait`/EOF/
  `command_timeout`→`Err(Ipc)`→child-scope 収束→watchdog
  kill→`InstanceFailed` exactly-once は l1_fault E2E が pin
  済み — 新しい観測機構を設計する必要はなく、既存語彙への
  写像だけが要る。

## Alternatives and Comparison

### プロセス構成（D-topology）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. child 所有 toplevel（`AttachMode::Floating`） — `ruviform-plugin-export::run` 内に lazy UI thread/pump | 入力キュー結合なしで lane 生存と両立。child 死亡で window も道連れ（orphan HWND が残らない）。runner 共有で binary 分割なし | host dock layout に直接入らない（floating 形の見た目） | **yes — 採用** |
| B. docked 越境 attach（`SetParent`/`WS_CHILD`） | host dock に直接入る見た目 | #115 確定の入力キュー結合で child hang が host UI へ波及 — crash/hang 観測 lane の目的と構造的に矛盾 | no — `mode` 語彙だけ残して Defined `Unsupported` |
| C. editor-host 専用別プロセス | editor 障害が DSP child と分離 | 失敗ドメインが 2 つになり `Terminated` cause が曖昧化、プロセス/wire/監視が二重 | no |
| D. Dedicated vs Bundled | Dedicated は editor=child=1 対応で収束が素直 | Bundled は route（plugin id）ごとの pool で、editor fixture と fault fixture（別 descriptor）を同居させられない — 異種同居には route 横断 pool の新機構が要る | Dedicated 採用 |
| E. runner 拡張 vs 別 GUI runner binary | `run` は test/production runner 共通の単一エントリ — lazy UI thread なら非 GUI child のコストはゼロ | 別 binary 化は `Hello` 能力・build/test 経路を二重化する | runner 拡張採用 |

### wire 語彙（D-verbs）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| keep-for-future の tag 8/9 を payload 拡張して再利用 + tag 24-27 新設 | v5 の `OpenGui.parent` 型変更と同じ「bump 時 payload 変更」先例。`CloseGui`/`Reply::Opened` の語彙がそのまま効く | payload 互換は失われるが単一最新版方針では問題なし | **yes — 採用** |
| tag 8/9 を凍結して GUI 系を全て新 tag で並設 | v5 payload が残る | 同じ意味の動詞が 2 系統になり語彙が死に体重複（keep-for-future の趣旨が失われる） | no |
| GUI verb を専用 message 系（Command 外）に分離 | wire 表が GUI で汚れない | frame/envelope/reply 語彙の再設計が要り over-engineering | no |
| `reattach`/`GuiCapabilities` を専用動詞として載せる | 語彙の完全性 | `reattach` は docked-family で v6 対象外、`capabilities` は Hello∩Opened で足りる（L0 も pre-open caps query なし）— YAGNI | no — 動詞非設置 |

### capabilities（D-version）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `Hello.capabilities` bit0 = `GUI_FLOATING`（機構族 bit） | 「runner が GUI 機構を持つ」1 事実を 1 bit で表し、verb 個別可否は reply/`Opened.capabilities` へ分離できる | per-verb の細粒度がない（ただし需要なし） | **yes — 採用** |
| D6 草案の per-verb bit（`GUI_OPEN`/`GUI_LIFECYCLE`/`GUI_REPARENT`） | verb 単位の交渉 | bit はあるのに reply が Unsupported の組合せが増え語彙が不正直に。D6 は非拘束 sketch で確定対象 | no — D6 草案を supersede |
| request/response 型 negotiation protocol | 双方向の要求/受理を表現できる | parent→child の要求経路を新設する必要があり、1 bit の需要に過剰 | no — 厳格等値+一方向広告 |

### fault 起動経路（D-oracle）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `test_editor` に in-band `Fault` param を載せる（`FAULT_PARAM`/`decode_fault` 再利用） | 「plugin コード自身が死ぬ/固まる」代表 fault。hang の唯一の主経路 | fixture 拡張が要る（slice 4） | **yes — 主経路** |
| `PluginHost::kill_child` | 実装済み・DSP 駆動不要・Dedicated child を直接殺せる | 「plugin が死ぬ」ではなく「child が殺される」trigger で representative 性は低い（観測経路は同一） | **yes — crash の主経路として併用** |
| argv `--ruviform-test-fault` | 実装済み | pre-Hello で発火するため session が存在しない — session crash/hang の対象にならない | no — spawn 系 lane 候補として Deferred |
| Bundled sibling に fault を仕込む | fault source を別 instance に分けられる | Bundled pool は route ごと — `test_fault`/`test_editor` は別 descriptor で同居不可（route 横断 pool の新機構が要る） | no — 構成不能 |
| headless 代替 oracle | CI で走る | POLICIES の「headless は real window/native ABI 証拠の代替にならない」に反する | no |

## Tentative Conclusions and Hypotheses

- **[confirmed]** crash/hang の `Unsupported` は「`test_fault`
  が in-process lane を殺す」構造限界に由来し、解除には
  out-of-process editor host が必要 —
  `real_dock.rs`:13235-13288 と `discovery/issue-389-real-
  session-lifecycle.md` が確定済み。
- **[confirmed]** docked 越境 attach は #115 の入力キュー結合
  確定と lane 生存が構造的に両立しない — editor は child
  所有 toplevel（`AttachMode::Floating`）に限定し、docked は
  Defined `Unsupported` とするのが両立の唯一の形。
- **[confirmed]** session 層・L1 収束語彙・isolation 語彙・
  fault 語彙・verdict/capability 語彙は全て実装済みで、
  不足しているのは (a) wire GUI 動詞・version・capabilities、
  (b) runner の UI thread/pump・container window、(c)
  `L1EditorOps`・host routing、(d) `test_editor` の in-band
  `Fault` param、(e) lane の case 実装・capability claim の
  5 点のみ — D-followup の 5 slice がそのまま実装単位になる。
- **[confirmed]** `Hello.capabilities` は child→parent の
  一方向広告（wire.rs:22 の frame table）で、parent 側の
  要求を載せる経路は存在しない — 交渉は child 広告の honor
  に限定する。
- **[confirmed]** `OpenGui`/`CloseGui` の dispatch 腕は runner
  側に既存（lib.rs:593-600）で、欠けているのは UI thread/
  pump・container window・新動詞の dispatch・capabilities
  応答である。
- **[hypothesis→確定へ]** `SetOwner`/`GWLP_OWNER` は input
  queue を結合しない（#115 の対象は child-window 化に伴う
  AttachThreadInput 系で、owned toplevel は別機構）— 実装
  slice で再検証項目として design doc の Verification Notes
  に記録済み。
- **[confirmed]** `Bundled` では route ごとに pool が分かれ、
  `test_fault`（fault あり・editor なし）と `test_editor`
  （editor あり・fault なし）を同一 child に同居させられない
  — hang 経路には `test_editor` 自身への `Fault` param 追加
  （slice 4）が必須。

## Open Questions

- `test_editor` に載せる `Fault` param の param id/value
  写像を `test_fault` と揃える具体形 — slice (4) の実装詳細
  （`FAULT_PARAM`/`decode_fault` の共有方針は確定、細部は
  実装時に決める）。
- `L1EditorOps` が notification を session へ渡す pump を
  ops 側に置くか driver/lane 側に置くか — slice (3) で決める
  （本設計は delivery contract のみ確定）。
- `Opened.capabilities` の child 側 bit 計算を adapter 別に
  どう正規化するか（VST3 `canResize` 等の個別判定をどこで
  bitfield へ畳むか）— slice (2)/(3) の実装詳細。
- `mode:1`（`Docked`）を「decode 受理して child が
  `Unsupported` reply」にするか「decode で `Malformed`」に
  するか — 本設計は「定義済み値ゆえ受理して defined
  reply」を確定としたが、未知値と既知未実装値の境界をどう
  運用するかは wire codec 規約として slice (1) で最終確認。

## Promotion Candidates

- [x] design-doc created: `docs/designs/issue-872-l1-oos-plugin-
  editor-host.md` — 本調査の結論を D-topology〜D-followup の
  裁定として確定済み
- [ ] spec-doc needed: wire v6 の動詞/payload/capability bit の
  spec 正本は実装 slice (1) の PR で `docs/specs/` に切る余地
  あり（本設計が実質の語彙定義を兼ねているため必須ではない）
- [ ] adr-doc needed: 「L1 GUI は docked ではなく child 所有
  toplevel（Floating）を採る」トポロジー裁定は ADR 化を
  検討できる（#115 判定の下での topology 選定として独立記録
  の価値あり — 本設計 doc 内の D-topology が代替になり得る）

## Sources and Relations

- `docs/designs/issue-872-l1-oos-plugin-editor-host.md` — 本調査の
  確定版（design doc が結論の正本）。
- `docs/designs/issue-115-l1-gui-cross-process-parenting.md` —
  越境埋め込み採否の approved 設計（入力キュー結合・解除条件
  (a)-(d) の確定事実）。
- `docs/designs/issue-389-plugin-editor-session.md` — session 層の
  approved 設計（D6 capabilities 草案は本設計が supersede）。
- `docs/discovery/issue-389-real-session-lifecycle.md` —
  crash/hang に out-of-process が要るという先行 discovery。
- `docs/specs/issue-119-l1-wire-version-policy.md` — 単一最新版
  方針。
- `docs/specs/plugin-l1-out-of-process.md` — L1 spec（L1 GUI を
  Deferred とした正本）。
- `evidence/rf631/plugin-dock-real-scenario.jsonl` — 現行
  `Unsupported` verdict と `session:{crash,hang}:in-process-lane-
  nonviable` marker の canonical 帳簿。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`:13235-13288 —
  `Unsupported` case 本体（boundary の正直な記録）。
- `crates/ruviform-plugin-test-vst3/src/fault.rs` — `test_fault`
  の in-band fault 機構。
- `crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_
  e2e.rs` — L1 crash/hang 収束語彙の実証。

## Evidence

### 2026-11-21 — コード/doc 調査の確定事実

- `crates/ruviform-plugin-core/src/wire.rs` — `WIRE_VERSION=5`
  （:119）、`Hello` frame は child→parent 単方向・
  `u16 version`+`u32 capabilities`（:22）、`OpenGui{parent:
  ParentLease}`/`CloseGui`/`Reply::Opened{gui}` 既載、
  `capabilities != 0` → `Malformed`（Hello decode）、
  `read_f64`/`push_f64` 既存。
- `crates/ruviform-plugin-host/src/l1/mod.rs` — `L1Instance::
  execute` が `OpenGui`/`CloseGui` を `Err(Unsupported)` で
  short-circuit（:2995-3035）、driver/reader/stderr-drain 3
  thread model、`try_wait` crash watch・`command_timeout`→
  `Err(Ipc)`→child-scope 収束・wedged child kill・mint 全
  instance へ exactly-one `InstanceFailed`。
- `crates/ruviform-plugin-host/src/composite.rs` — `ChildExit
  {code,signal}`（:195）、`HostNotification::InstanceFailed
  {handle,exit,cause}`（:229-243）、`OutOfProcessIsolation::
  {Dedicated,Bundled}`、`kill_child`、`process`（:1026）。
- `crates/ruviform-plugin-host/src/editor_session.rs` —
  `AttachMode::{Docked,Floating}`（:51-56）、`EditorCapabilities`
  10 field（:109-130）、`SessionFailCause::{…,Timeout,HostLost,
  Transport,…}`（:141-162）、`EditorSessionEvent`（:228-275）、
  `EditorOpError`（:285-308）、`EditorSessionOps`（:507-593）、
  `fail_cause`（:483-495）、`map_host_error`（:1665-1678）、
  `InProcessEditorOps`/`LeaseRedeemer`/`with_redeemer`
  （:1615-1659）、`L1EditorOps` refusal-only（:1840-1919）、
  `notify_terminated`（:1010,1587）、`tick`/`tick_all`
  （:1070,1602）、`SessionDeadlinePolicy`（:444-449）、
  `OpenOutcome::Degraded`（:465-474）。
- `crates/ruviform-plugin-export/src/lib.rs` — `Hello` 送出
  `capabilities:0` 固定（:230-237）、`dispatch` に
  `OpenGui`/`CloseGui` 腕（:593-600）、`guarded`/
  `catch_unwind`（:563）、単一スレッド stdin/stdout loop。
- `crates/ruviform-plugin-test-vst3/src/bin/ruviform-plugin-
  test-vst3-l1-runner.rs`:30-40 — argv fault 語彙
  `abort|exit:N|hang|stderr:N|bad-fixture`（pre-Hello）。
- `crates/ruviform-ui-test/src/scenario.rs`:204 —
  `ScenarioCapability`（`NativePluginDock` 含む）、:253 —
  `ScenarioVerdict`（`pass`/`fail`/`not_run`/`unsupported`/
  `blocked`）。
- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`:259-267 —
  crash/hang 行 `required:&[NativePluginDock]`。同:85-88 —
  `test_editor` の `RuviformTestEditor` class HWND oracle。
- `crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_
  e2e.rs` — `real_test_fault_l1_abort_fails_instance`・
  `real_test_fault_l1_hang_converges_on_timeout`・
  `real_test_fault_l1_bundled_crash_fans_out`・
  `real_test_fault_l1_dedicated_isolation` 他が L1 収束語彙を
  実証。

### 2026-11-21 — 未探索・残件

- `SetOwner`/`GWLP_OWNER` の input queue 非結合は Win32 の
  標準意味論として記録したが、#115 が確定した事実表に
  直接の行はない — 実装 slice で再検証（design doc の
  Verification Notes に明記）。
- `test_editor` の `process()` を lane から駆動する際の
  `Initialize`/`Activate`/`StartProcessing` 遷移と editor
  open の順序制約（open が `Initialized` 以上を要するか）は
  fixture/adapter 実装に依存 — slice (4)/(5) で確定。
- GPUI/consumer 側での floating editor window の合成・配置は
  本調査の対象外（consumer 別 slice）。
