# Issue #389 — acceptance-criteria reconciliation（close 判定材料）

`scarecrow173/ruviform#389`（Driver-neutral Plugin Editor Session・
L0/L1 Native Dock/Floating/Recovery Adapter — umbrella Issue）の
受入条件全行と、着地 PR・evidence・verdict の対応表。

**本書は close 判定の材料であり、いずれの行も本 slice で達成宣言
しない。** チェックボックス簿記・close 判定・close 対象 SHA の
3OS CI 証拠収集は Coordinator の gate 作業。

- Date: 2026-11-24
- 対象 HEAD: `65878cc1`（`main` — Issue #631 lane 文書同期・PR #921）
- 状態語彙: `pass-measured`（実走証拠あり）/ `notrun`（未実施・環境未整備）/
  `unsupported`（根拠付き技術的非対応）/ `deferred`（方針上の保留）
  — POLICIES.md §3 の Required/Deferred/NotApplicable と
  Pass/Fail/NotRun/Unsupported の分離に従う
- 正本: canonical real lane ledger
  `evidence/rf631/plugin-dock-real-scenario.jsonl`（32 case・
  `environment.repo_sha` = `8145861b817decc4d9a3213b9b7b34a9c4677cca`
  — rf917-dcomp-verdict workspace HEAD、squash merge `bf5c4107`
  として main に同内容着地）と
  `evidence/rf631/plugin-dock-real-headless.jsonl`（32 row 全て
  `not_run` + `environment_blocked`・violation 0 の fail-closed
  proof）。`evidence/rf389/` 直下の JSONL は slice snapshot —
  `plugin-dock-real-scenario.jsonl`/`plugin-dock-real-headless.jsonl`
  は `ff7013dc` 世代、`plugin-dock-scenario.jsonl`/
  `plugin-dock-console.jsonl` は `303db66b` 世代の据え置き記録で、
  現行 verdict の引用は canonical ledger へ帰る

## 現行受入条件（Issue 本文 `### 受入条件` — 8 行）

| # | AC 行 | 状態 | 根拠 |
| --- | --- | --- | --- |
| A1 | Virtual-first 移行: 本番 Plugin Host から #630 実バイナリをロードして Editor を生成し、Windows Native Editor L2/L3 の session lifecycle/crash/recovery を #631 で検証。NotRun/Fake と実 ABI Pass を区別 | `pass-measured`（`real-session-rt-boundary` のみ `unsupported` — 材料 (b) 参照） | canonical ledger 32 case: 本番 `PluginHost`（composite L0 in-process + L1 out-of-process dedicated route）が実バイナリ `ruviform_plugin_test_vst3.dll`（`test_editor`・#630 正規 ABI・artifact sha256 記録）をロードし `environment_kind=real-window` で実走。lifecycle/crash/recovery 全 case `pass`（crash/hang は #886・PR #916 `5ea61d83` の dedicated L1 child `ruviform-plugin-test-vst3-l1-runner` 経由）。NotRun/Fake と実 ABI Pass の区別は lane が fail-closed で維持（headless ledger 32 row `environment_blocked`・violation 0） |
| A2 | Session 状態機械・deadline・明示 Degrade・Fake/ScriptedOps fixture、SM-01〜17 と DSP/RT headless 非干渉 | `pass-measured`（Issue 本文で `[x]` 済み） | PR #503/#530/#544。`editor_session` suite + driver SM-01〜17 + `editor_session_l1` が green（`evidence/rf389/verification.md` L194: 57/57 + 18/18 — 2026-11-23 記録）。SH-01..17 deadline/degrade は `evidence/rf389-session-hardening/report.md` |
| A3 | Windows/macOS/Linux で利用可能 Backend の Build/Link・型付き Host Port・Session/generation/tick/timeout/cleanup/RT 境界を既存 CI 内 Headless 試験で検証。Backend 非実装なら機能欠落を明記 | `notrun`（close 対象 SHA の 3OS CI 実走証拠は Coordinator gate — `evidence/rf388-os-l1-ci` 同型の収集が未実施） | 対象 headless 契約 test は実在し Windows local で green（`editor_session`・`editor_session_driver`・`editor_session_l1`）。L1 GUI 機構の非 Windows 不在は `editor_session_l1` の unadvertised arm が defined `Unsupported` で pin（欠落明記）。`.github/workflows/ci.yml` の verify（ubuntu+windows）/verify-macos 構成は存在するが、対象 SHA での run URL 収集は Coordinator の gate 作業 |
| A4 | GuiParentToken/Parent Lease 公開 API・OS handle 隠蔽・旧 API 移行/保持・Driver-neutral boundary を L1 契約と移行表に確定 | `pass-measured` | `GuiParentToken` は `crates/**/*.rs` 0 hit（移行 commit `66fcce0b` — typed `ParentLease` へ完全移行）。`ParentLease{generation:u32,payload:u64}` が opaque 語彙・`ParentLease::new` は adapter-internal。移行表/状態遷移表/所有責務は design doc `docs/designs/issue-389-plugin-editor-session.md` D8・改訂 2 S1・R3/R4、`docs/designs/issue-389-parent-lease-design.md` に確定。`check-ui-driver-boundary` green（dep-graph/signature/allowlist） |
| A5 | Native Dock 不成立は Unsupported の確証がない限り NotRun/既知 Fail のまま保持し、Session headless Pass から実 Plugin Pass を主張しない | `pass-measured` | #917・PR #919 `bf5c4107` で `real-dcomp-{bandescape,childinv}` の as-child `WS_EX_LAYERED`/`LWA_ALPHA` 拒否を確定 class-C 期待値へ re-scope — refusal 記録（applied/win32_error/after/latched）を維持し、as-child latch 成功は `contradicts_confirmed_mechanism` flag で記録（Fail→Pass 改竄でなく期待値検証化）。verdict 写像は fail-closed を維持、headless Pass の実 Plugin Pass 転用なし（`docs/discovery/issue-532-dcomp-residual.md` が class-C 機序の権威） |
| A6 | Windows GUI-L2/EXT-L3（Required）: 本番 Host→#630 正規 ABI Test Native Editor→#631 実 Window で Open/Attach/Move/Close/Crash/Hang/Recovery/RT 非干渉を検証。市販 Plugin 固有動作は必須にしない。Native Dock 画素未達は #388/#532 へ残し、Headless/Floating 成功を Pass に転用しない | `pass-measured`（`real-session-rt-boundary` のみ `unsupported` — 材料 (b) 参照） | canonical ledger 32 case（interactive Windows 実走）: Open/Attach=`real-editor-attach`、Move=`real-editor-redock`、Close=`real-session-terminate`+`real-window-close`、Crash/Hang=`real-session-{crash,hang}`（OOS L1・`InstanceFailed`→`Terminated{HostLost}` exactly-once + lane 生存）、Recovery=`real-session-recovery`、Degraded=`real-session-degraded`、Timeout=`real-session-timeout`、RT 非干渉=`real-session-rt-{samethread,fault}` — 全 `pass`。Native Dock 画素は `real-dcomp-pixel` `pass`（#857 deocclusion 機序・`dcomp-pixel:editor-green-on-screen`）+ bandescape/childinv は #917 class-C `note` 記録で `pass` — residual 帰属は #388/#532 維持（両 CLOSED）。VST3 `test_editor` のみで充足するかは材料 (a) 参照 |
| A7 | macOS/Linux L1 のみ必須: 両 OS の既存 CI で Native Build/Link・必要 Feature/Headless/RT を対象 SHA で検証。実 GUI/物理 Device/実 Plugin は使用開始時まで Deferred（NotRun≠Pass） | `notrun`（close 対象 SHA の verify-macos/verify-ubuntu 実走証拠は Coordinator gate） + `deferred`（両 OS の L2/L3 — 利用要件発生まで） | `editor_session` 契約 test は cfg 非依存で workspace test（test-full 相当）に含まれ、verify-macos（macos-latest）/verify（ubuntu-latest）の既存 lane が対象 SHA で実走する構成。`editor_session_l1` は非 Windows で unadvertised arm が defined `Unsupported` を pin（Deferred/NotRun≠Pass の区別を lane 設計で保持）。対象 SHA での両 OS lane green 証拠（run URL）の収集は Coordinator の gate 作業 |
| A8 | 検証 Scope: Windows でも未利用デバイス・Plugin 形式・追加環境は必須とせず、採用/利用する機能・OS/Device/Format・Run URL/Oracle を明記。新規 CI Workflow は作らない | `pass-measured` | lane JSONL に `environment_kind=real-window`・capabilities/required・plugin artifact hash（`ruviform_plugin_test_vst3.dll` + sha256）・repo_sha・window identities を記録。採用 format=VST3 `test_editor`・opt-in real lane（`test-ui-plugin-dock-real`）・新規 CI Workflow なし — `evidence/rf389/verification.md`・`crates/ruviform-plugin-dock-test/README.md` に明記 |

## 履歴受入リスト（Issue 本文 `### 詳細` 内 — 現行 DoD ではない記録）

グループ化の上で全行を網羅。各行の現行行への対応と状態を記録する。

### H1 — 「直前の L1 限定 DoD（履歴・旧版）」`<details>`（5 行）

| 履歴行 | 現行行対応 | 状態 | 根拠 |
| --- | --- | --- | --- |
| `[x]` 状態機械・SM-01〜17・DSP/RT headless 非干渉（PR #503/#530/#544） | A2 | `pass-measured` | A2 と同一（Issue で `[x]` 済み） |
| 3OS Build/Link・Session headless（Backend 非実装は欠落明記） | A3 | `notrun` | A3 と同一（3OS CI 証拠は Coordinator gate） |
| GuiParentToken/Parent Lease 公開 API・移行表 | A4 | `pass-measured` | A4 と同一 |
| 実 VST3/CLAP/AU の L2/L3 試験は DoD 外・要件発生時 Activation | —（方針改訂で supersede） | `deferred` → 現行方針では Windows 利用機能の L2/L3 は Required | 2026-11-13 16:22 JST 節で明示的に改訂済み（「本節と現行受入条件が優先」）。Windows 側は A6 で `pass-measured`、AU は初期対象外（adapter crate なし） |
| Native Dock 不成立は NotRun/既知 Fail 保持・headless Pass 転用禁止 | A5 | `pass-measured` | A5 と同一 |

### H2 — 「旧受入条件（以前の L2/L3 必須を含む履歴）」`<details>`（6 行）

| 履歴行 | 現行行対応 | 状態 | 根拠 |
| --- | --- | --- | --- |
| `[x]` PluginEditorSessionId/generation・attach/detach/recreate/close・明示 Degraded・deadline/tick・terminal event・Fake Clock/Driver | A2 | `pass-measured` | PR #503/#530/#544・`editor_session` suite |
| `[x]` SM-01〜17・timeout/stale/unsupported/host-crash・DSP/RT 非干渉 headless | A2 | `pass-measured` | 同上 |
| 実 GUI driver tick/deadline・parent lease/OS Native Handle 接続・GPUI 型非漏出・旧 GuiParentToken 移行表 | A4 + A6 | `pass-measured` | tick/deadline 実経路は `real-session-timeout` `pass`（ZERO deadline + `begin_detach` + `tick_all`→`Failed{Timeout}`）。ParentLease 移行は A4 |
| Windows 実 VST3/CLAP L0/L1 lifecycle・hang/crash/focus/resize/close 応答・timeout・DSP/RT 非干渉（#532 未解消なら画素 Blocked 記録） | A6 | `pass-measured`（VST3）/ `notrun`（CLAP real-window lane — 材料 (a)） | VST3: `real-session-{crash,hang,timeout}`・`real-editor-{resize,focus,input,popup,visibility,redock}`・`real-session-rt-{samethread,fault}` 全 `pass`。CLAP: `clap.ext.gui` 実装済み（`ruviform-plugin-test-clap` `editor.rs`）だが real-window lane 未実施。画素 Blocked 記録は #917 class-C 期待値検証化で lane 収束 |
| macOS AU/VST3/CLAP・Linux X11/Wayland・L1 Sandbox 能力確認・Unsupported/NotRun の理由/Owner 明記 | A7 | `deferred` + `notrun` | macOS/Linux GUI-L2/EXT-L3 は現行方針で Deferred（利用要件発生まで）。L1 sandbox/OOS 境界は `editor_session_l1` unadvertised arm が defined `Unsupported` で pin |
| #388 連携・SV #176 製品受入境界・Native Dock 未達を Session 完成だけで Pass 化しない | A5 | `pass-measured`（境界記録）+ 外部 gate は `notrun` | #388（CLOSED）連携は design doc/lease chain で確定。`synvelis#176` 製品受入は OPEN で各 repo Owner の責務 — #389 側の完了条件ではない |

### H3 — 「2026-11-13 以前の受入条件（旧チェックリスト）」`<details>`（14 行）

| 履歴行 | 現行行対応 | 状態 | 根拠 |
| --- | --- | --- | --- |
| ID 対応・所有・generation・capability・lifecycle verbs・plugin preferred size/resize 拒否・bounded timeouts/errors/cleanup | A2/A4/A6 | `pass-measured` | R4 所有責務表・session 契約 suite・real lane lifecycle 全 `pass`（plugin 主導 resize `resizeView` は `kResultFalse` 正直拒否 — `evidence/rf389-vst3-editor/report.md`） |
| 状態機械遷移・2-phase Dock move abort/rollback・Window/DSP close 区別・terminal event 単一化 | A2/A6 | `pass-measured` | R3 状態遷移表・`real-editor-redock`（reattach `Detached`/`Reattached` event 実測）・`Terminated{HostLost}` exactly-once（`real-session-{terminate,crash,hang}` の repeated notify no-op 実測） |
| VST3/CLAP/AU/GPUI plugin adapter 分離・外部 DAW 埋込み方向の capability 整理 | — | `pass-measured`（VST3/CLAP）+ `unsupported`（AU — 初期対象外）+ `notrun`（GPUI plugin — adapter 未実装） | R5 matrix（adapter 別 capability）。外部 DAW 埋込みは capability 宣言せず（design doc L430-431） |
| 既存 L0/L1・GuiParentToken/OpenGui・旧 #115 棚卸し・NativeSurfaceSlot 段階実装 | A4 | `pass-measured` | D8/R2 移行表・GuiParentToken 0 hit・L1 unadvertised 拒否 pin・NativeSlot→lease→session の実 chain（real lane 全 case 通過） |
| Windows cross-process input queue freeze・macOS NSView・Linux X11/Wayland・sandbox/process 隔離の trade-off 明示 | A6/A7 | `pass-measured`（Windows）/ `deferred`+`notrun`（macOS/Linux） | OOS L1 lane が cross-process crash/hang 分離を実測（`host_survives`/`lane_window_alive`/`lane_settles`）。macOS/Linux は Deferred |
| 失敗時 Floating/Generic 明示 degrade（実 Dock Pass 代替でない） | A6 | `pass-measured` | `open_or_degrade` + `OpenOutcome::Degraded` + `Degraded` event 実装・`real-session-degraded` `pass`（`Attached` 非発火を実測 — Floating 成功とは別 Result） |
| Audio Thread からの UI 呼出・allocation・lock・OS GUI API 禁止・close/hang で DSP/RT 暗黙停止しない | A6 | `pass-measured` | `real-session-rt-samethread`（実 window verb 下で DSP pass-through bit-exact 維持）・`real-session-rt-fault`（`rt_alloc` oracle が armed marker を正確に捕捉）共に `pass`。cross-thread pump は `unsupported`（材料 (b)） |
| L0/L1 現行 API・Wire→Session/Parent Lease/OS Adapter 移行表 | A4 | `pass-measured` | D8/R2 移行表 + `WIRE_VERSION` v5→v6→v7 の additive revision 履歴（wire.rs） |
| fake host driver で Open/Attach/Reparent/Detach/Close・stale generation・Double close・Timeout・OS unsupported・crash の terminal-event 検証 | A2 | `pass-measured` | `editor_session`/`editor_session_driver` suite（fake ops + SM-01〜17 + `editor_session_l1` defined refusal 群） |
| 代表実 Plugin/Format 別 L0/L1 capability・GUI hang で DAW/RT を巻き込まない実機受入 | A6 | `pass-measured` | `real-session-hang` OOS `pass`（wedged leg pass-through 継続・watchdog kill・lane 生存 — DSP/RT 巻き込みなしを実測） |
| fake/native plugin fixture で Open/Attach/Resize/Hide/Move/Close・timeout/stale/crash・double-close・lost window/host process | A2/A6 | `pass-measured` | fake: `editor_session` suite。native: real lane 32 case |
| real VST3/CLAP/AU L0/L1 capability matrix・非対応/未実施明示・両 Consumer 非回帰 | A6 + consumer gate | `pass-measured`（matrix）/ `notrun`（consumer — 本 repo scope 外） | R5 matrix（Supported/Unsupported/NotRun/対象外の明示）。Consumer 非回帰は `synvelis#176`・`audio-lens#127` の各 repo Owner 責務 |
| API 境界・OS native handle 隠蔽・release 安全・process 分離時の host UI スレッド非阻害 | A4/A6 | `pass-measured` | `ParentSlotLease` opaque・`ParentLease` adapter-internal・OOS child の死滅が host/lane を殺さないことを `host_survives`/`lane_window_alive`/`lane_settles` で実測 |
| Native Slot 結合は RF-GPUI-05 実機 Gate・Adapter 責務重複なし | A6 | `pass-measured` | `NativeSlot`→`GpuiSurfaceAdapter`→`ParentSlotLease`→redeem chain が real lane 全 case の production 経路（#388 CLOSED）。Native Dock 画素 residual は #388/#532 帰属維持 |

### H4 — 「2026-11-07 横断レビュー確定仕様 v2 — Definition of Done」（3 行）

| 履歴行 | 現行行対応 | 状態 | 根拠 |
| --- | --- | --- | --- |
| L0/L1 現行 API・Wire→Session/Parent Lease/OS Adapter 移行表・状態遷移表・所有責務 | A4 | `pass-measured` | D8/R2/R3/R4 + design doc 改訂 6 で現行 reconcile |
| fake host driver 状態遷移・terminal-event 検証 | A2 | `pass-measured` | H3 同項目と同一 |
| 代表実 Plugin/Format 別 L0/L1 capability・GUI hang で DAW/RT 巻き込まない実機受入 | A6 | `pass-measured` | H3 同項目と同一（`real-session-hang` OOS `pass`） |

### H5 — 「2026-11-07 — `#### 受入`」（4 行）

| 履歴行 | 現行行対応 | 状態 | 根拠 |
| --- | --- | --- | --- |
| fake/native plugin fixture で Open/Attach/Resize/Hide/Move/Close・timeout/stale/crash・double-close・lost window/host process | A2/A6 | `pass-measured` | H3 同項目と同一 |
| real VST3/CLAP/AU L0/L1 capability matrix・非対応/未実施明示・両 Consumer 非回帰 | A6 + consumer gate | `pass-measured`（matrix）/ `notrun`（consumer） | H3 同項目と同一（CLAP real-window 未実施は材料 (a)） |
| API 境界・OS native handle 隠蔽・release 安全・process 分離時の host UI スレッド非阻害 | A4/A6 | `pass-measured` | H3 同項目と同一 |
| Native Slot 結合は RF-GPUI-05 実機 Gate・Adapter 責務重複なし | A6 | `pass-measured` | H3 同項目と同一 |

## 判定材料（uncertain — 結論は Coordinator の close 判定で下す）

事実と根拠のみを記録する。「充足する/しない」「確定境界とする/
しない」の結論は本書に書かない。

### (a) 現行 AC6「本番 Host→#630 正規 ABI Test Native Editor→#631 実 Window」は VST3 のみで充足するか

事実:

- 現行 real-window editor lane（canonical ledger 32 case）は全て
  `ruviform_plugin_test_vst3.dll`（VST3 `test_editor`・#630 正規
  ABI・`plugin_artifact` に sha256/plugin_id/name/vendor/version
  記録）をロードして実走している — CLAP binary をロードする
  real-window case は存在しない（ledger 全行の実測）。
- CLAP 側の既存証拠は `evidence/rf389-clap-editor/`（L0
  `open_gui`/`close_gui` の実外部 `.clap` module 経路 — 当該
  report 時点の fixture は `clap.ext.gui` 非実装で `open=Unsupported`
  の正直分類）までで、CLAP の real-window lane は未実施。
- CLAP `test_editor`（`ruviform-plugin-test-clap` `editor.rs`）は
  `clap.ext.gui` を実装済み — descriptor id
  `org.ruviform-test.clap-test-editor`・VST3 `test_editor` と対応する
  fixture 契約（open/close/resize/hide/show/set_parent 面）を持つ。
  実 CLAP binary での real-window dock verb lane は未実施
  （design doc R5 L0/CLAP 行: 「実 CLAP binary の verb lane は
  未実施 — VST3 lane のみ real-window oracle あり」）。
- per-format Native Editor 実 Window 証拠（Native Editor が実
  Window/child HWND/画素/Input/Focus/Resize/Popup/Close/再 Open
  を生成できること）は `scarecrow173/ruviform#630` AC4 の帰属と
  読む解釈がある — #630 は OPEN で、当該 AC 行は format 別の
  per-format 実証を要求する字義。AC6 の字義が「#630 正規 ABI
  Test Native Editor」を単数指定するため VST3 のみで字義充足と
  読む解釈と、format 横断で読む解釈の双方が成り立つ — 確定は
  close 判定。

### (b) `real-session-rt-boundary` の verdict `unsupported` を AC「RT 非干渉を検証する」の充足と両立する確定境界として扱うか

事実:

- ledger 上の記録: case `real-session-rt-boundary`、verdict
  `unsupported`、`evidence: ["session:rt:cross-thread-boundary"]`、
  `required: ["native-plugin-dock"]`。
- 構造的根拠: `test_editor`/`test_fault` は `!Send` →
  `create_sendable`=None → Plain slot → `attach_endpoint` が
  `EndpointUnavailable` を返す — cross-thread 同一 instance pump
  が現行 API で構造上不可能（`PluginHost.l0` は private）。
  `crates/ruviform-plugin-dock-test/README.md:112`・
  `real_dock.rs` モジュール doc が同一根拠を記録。
- 追跡先 `scarecrow173/ruviform#920`（OPEN — cross-thread 同一
  instance RT pump・`attach_endpoint`/`create_sendable` 契約）が
  起票済みで、#872 は CLOSED で同残余を引き取っていない。
  `crates/ruviform-plugin-dock-test/README.md` の
  `real-session-rt-boundary` 行の追跡先は #920 へ更新済み
  （PR #921・`65878cc1` — `real_dock.rs` L15331/L15347 の
  deferred 文言も #920 参照）。
- RT 非干渉自体は別 case で `pass` 済み:
  `real-session-rt-samethread`（`test_editor` を `Processing` に
  したまま `host.process` block を実 session verb の前後に
  interleave — 全 block `Continue`・bit-exact pass-through・
  lifecycle `Processing` 維持・child HWND 契約・
  `[Attached{Docked}, Closed]` 収束を実測）と
  `real-session-rt-fault`（`test_fault` の `rt_alloc` arm/disarm —
  `output[0][0]==200.0` marker が armed block に正確に追従する
  negative-control oracle）が ledger で `pass`。
- AC5 の字義は「Unsupported の確証がある」場合の保持を許容する
  — 本 case は確証（構造的不可）付き `unsupported` として記録
  されている。

## 出典

- canonical ledger:
  `evidence/rf631/plugin-dock-real-scenario.jsonl`（repo_sha
  `8145861b` — 現行 verdict の正本・再実走なしの引用）/
  `evidence/rf631/plugin-dock-real-headless.jsonl`（fail-closed
  proof・repo_sha `5099cbe5`）
- 過去 slice の実走記録: `evidence/rf389/verification.md`
  （2026-11-17 session lifecycle・2026-11-18 editor verbs・
  2026-11-23 IME @ `ff7013dc` の各記録 — 当該実走時点の記録
  として保持・改変なし）、`evidence/rf389/*.jsonl`（snapshot —
  世代は冒頭の正本節参照）
- L0 adapter 実証: `evidence/rf389-vst3-editor/report.md`（実外部
  VST3 module の HWND attach — iZotope Vinyl）、
  `evidence/rf389-clap-editor/report.md`（実 `.clap` module
  load/init + 正直な `Unsupported` 分類）、
  `evidence/rf389-dock/verification.md`（`plugin-dock-scenario`
  8 case real-abi session lifecycle @ `d5aa1b4b`）、
  `evidence/rf389-session-hardening/report.md`（SH-01..17
  deadline/degrade contract）
- 設計・契約: `docs/designs/issue-389-plugin-editor-session.md`
  （改訂 6 で現行 reconcile）、
  `docs/designs/issue-389-parent-lease-design.md`、
  `docs/discovery/issue-532-dcomp-residual.md`（class-C 機序の
  権威）、`docs/designs/issue-872-l1-oos-plugin-editor-host.md`
  （wire GUI verb の確定版）
- lane 契約: `crates/ruviform-plugin-dock-test/README.md`（case
  別 oracle 定義・#920 追跡先）、
  `crates/ruviform-plugin-dock-test/src/real_dock.rs`（モジュール
  doc の verdict intent）
- 着地 PR/SHA: #503/#530/#544（SM-01〜17）・#399 `e8adcc09`
  （NativeSurfaceSlot+session+PoC）・#427 `34d34b0a`（改訂 1）・
  #652 `b464f665`（改訂 2）・#705 `38585187`（改訂 3）・
  #754 `0450f92f`（改訂 4）・#875 `430b89ab`（改訂 5）・
  #884 `f6958503`/PR #907・#885 `dc32b8f0`/PR #887・
  #886 `5ea61d83`/PR #916・#910 `5099cbe5`/PR #914・
  #917 `bf5c4107`/PR #919・#918 `3d3f33fd`（rf630 oracle
  re-pin）・#921 `65878cc1`（#631 lane 文書同期）
- 追跡・外部 gate: `scarecrow173/ruviform#920`（OPEN —
  cross-thread RT pump）、`scarecrow173/ruviform#630`（OPEN —
  AC4 per-format Native Editor 証拠）、`scarecrow173/synvelis#176`
  （OPEN — 製品 Plugin Host Editor 受入）、
  `scarecrow173/audio-lens#127`（consumer 追従）
