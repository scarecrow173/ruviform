---
id: "SPEC-034O7hTzfdwtNrUQom8ISw"
type: "spec"
status: "draft"
title: "Issue #121 L1 診断・可観測性（underrun メトリクス・子 stderr 配管・失敗原因 payload）"
created: "2026-10-29"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/104"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "issue-119-l1-wire-version-policy.md"
    - "issue-117-bundle-child-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "plugin-l1-out-of-process.md"
    - "issue-119-l1-wire-version-policy.md"
    - "issue-117-bundle-child-process.md"
    - "issue-116-supervisor.md"
    - "issue-118-video-l1.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../designs/issue-116-supervisor.md"
    - "../designs/issue-118-video-l1.md"
    - "../designs/foundation-observability-minimal-api.md"
    - "../designs/shared-foundations-and-modularization-policy.md"
    - "../designs/issue-122-shared-platform-process-ipc-eval.md"
    - "../discovery/issue-121-diagnostics.md"
    - "../impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md"
    - "../impl/ir/rt-observability-poc.md"
    - "../impl/exp/rt-observability-poc-20261023.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-121-diagnostics.md"
    - "plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/foundation-observability-minimal-api.md"
    - "../impl/ir/rt-observability-poc.md"
    - "issue-117-bundle-child-process.md"
    - "issue-119-l1-wire-version-policy.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/103"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/123"
  defers: []
  deferred-by: []
---

# Issue #121 L1 診断・可観測性（underrun メトリクス・子 stderr 配管・失敗原因 payload）

## What Should Be Built

Issue #121 — #76 closeout の残余フォローアップ 3 件をまとめた L1 診断・可観測性の実装:

1. **underrun メトリクス面** — L1 `process` 等価経路の pass-through submit（`last_output`/無音フォールバック）・dropped submit（submit channel full・slot busy）・`RESULT_FAILED`（子の block 拒否・収束合成）の発生回数を、RT-non-blocking 契約と両立する形で観測可能にする。issue は `HostNotification` / composite を surface 候補として再評価せよと要求（discovery 軸 1 の暫定 lean は pull 型 snapshot + 共有 atomic カウンタ）
2. **子 stderr 配管** — `Stdio::inherit` を bounded piped stderr drain + 診断経路へ置き換える（`plugin-l1-out-of-process.md` L323 deferred item の re-engagement）。bounded-buffer deadlock（pipe 満杯で子がブロック）を構造的に防ぐ常時 drain が前提条件
3. **失敗原因 payload** — `WireError`→`HostError::Ipc` collapse・transport EOF/timeout・spawn/handshake 失敗等で現在 `Err(Ipc)`/`ChildExit{None,None}` に畳み込まれる原因詳細を、通知型の payload として host 側に運ぶ（`HostNotification` は wire を通らないため wire codec・`WIRE_VERSION` 変更不要 — 原因 enum は host 側のみ）

Issue 本文は本実装を `ruviform-observability`（#101 foundation 候補）の **candidate first consumer** と位置づける — クレート新設そのものは本 spec の前提ではなく、#104 PoC verdict（counter pair・`rt_bridge` 形状・`reserved: [u64; 2]` 規約・RT 経路 logging 不採用）との整合が要求条件。公開面の正確な形は design フェーズで確定する。

## Why Now

- `docs/designs/plugin-l1-out-of-process.md` L323 deferred 行（stderr 配管 — trigger「子プロセスのログ解析要求」）・L325（#120 再トリガとして本 Issue の stderr 配管を明記）の re-engagement
- `docs/impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md` Follow-ups の 3 残件 — underrun metrics（「`HostNotification` 面ができたため metrics surface の置き場候補として composite を再評価可」）・`WireError`→`Ipc` collapse（「通知 payload に原因詳細を持たせるかは将来の通知型拡張で再評価可」）・stderr piping（「`Stdio::inherit` のまま — deferred design concern」）
- #104 PoC が完了済み（verdict: ACCEPT WITH NON-BLOCKING RESIDUAL NOTES）— counter pair・`rt_bridge`・drop-on-full+overflow count の実測形状が参照形として利用可能
- #101/#103 の `rt_bridge`/`telemetry` 新設トリガは「最初の実需要 + #104 PoC 結果」— #121 はこの最初の実需要の候補（クレート新設自体は trigger Issue 管轄・本 spec では所有者ローカル実装を既定とする — R-F1）
- 実プラグインの L1 運用（#114 CLAP/VST3 runner）が進み、子プロセスのクラッシュ・hang・protocol 違反の原因診断が現状 `Err(Ipc)` + inherit stderr の目視に依存している — 観測手段なしの運用が継続するほど診断不能な失敗が蓄積する

## Users and Value

- ホストアプリケーション/エンジン実装者 — L1 プラグインの underrun（音切れ・レイテンシ悪化の前兆）を定量的に監視し、子プロセス失敗の原因（crash・timeout・protocol 違反・spawn 失敗）を構造化して受け取れる
- プラグイン/アダプタ開発者 — L1 子の stderr（panic・dlopen 失敗・`Hello` 前の usage rejection）をホスト側で捕捉・保持し、クラッシュ直前の診断を失わない
- 運用/サポート — `Err(Ipc)` 単発では切り分け不能だった失敗（transport EOF vs protocol 違反 vs timeout）を原因種別で分類できる

## Scope

### In Scope

- L1 audio `process` 等価経路（in-place `L1Instance::process` + detached `L1Endpoint::process` の両方）の pass-through / dropped submit / `RESULT_FAILED` 計数 — writer 側計装は wait-free・no-alloc・Relaxed bookkeeping で RT 契約に適合すること（`rt_l1_*` テストの assert_no_alloc 窓を退行させない）
- 計数値の公開面 — RT 経路を汚さない非 RT query 面（pull 型）。`PluginHost`（composite）を surface 候補として再評価し確定する。per-handle・per-child・host 全体どの粒度かは design 論点（R-F2）
- 子 stderr の bounded piped drain — 全 L1 子（audio dedicated/pooled・video dedicated）に対し `Stdio::inherit` を置き換える方針の確定と実装。drained 内容は bounded buffer（満杯 drop + 計数 — #104 verdict の drop-on-full+count 方針）に保持し、非 RT query 面で読み出せること
- 通知型への失敗原因 payload — `HostNotification` の拡張として、`WireError`/`io` transport/`Hello`/spawn/timeout/protocol 違反の原因種別を `Copy` payload（または design が確定する形）で運ぶ。`ChildExit` の既存契約（exactly-once・reaped status）は保持
- audio/video 両 route の parity — video の `VideoL1Instance` も `spawn_scanned`・`try_recv_notification`・notify 機構を共有するため、stderr 配管と原因 payload は video にも同じ契約で適用する（underrun 計数は audio RT 経路に固有）
- 検証 — 単体（計装点・原因 enum 写像・buffer bound）・proxy E2E（underrun 発生→計数観測・stderr 捕捉→読み出し・失敗→payload 観測）・RT レーン無退行・`BundleFailed` parity（bundled 子の stderr・原因 payload が同居インスタンスへ正しく帰属/共有されること）

### Out of Scope

- `ruviform-observability` / `ruviform-foundation` クレートの新設 — 第一 consumer 候補としての trigger 記録は行うが、クレート新設自体は別 Issue（#101 トリガ管理）の判断（R-F1 で lead 裁定）
- `ruviform_rt::observability_poc` の本番昇格 — `#[doc(hidden)]` 実験 API のまま（昇格は #101 trigger Issue の権限）
- 構造化 logging・tracing/metrics バックエンド・export — #104 verdict で RT 経路 logging 不採用・backend 需要なしのまま deferred
- `HostError::Ipc` 自体への payload 追加・wire エンコード拡張 — issue scope は通知型 payload。`HostError` は wire tag を持つ契約型であり変更は #119 bump 規則に触れる別論点
- `BundleFailed` 相関通知 variant（#117 design L395 deferred）— 子スコープ原因 payload で相関の一部は表現可能になるが、専用 variant は別 Issue のまま
- L1 GUI・#115 deferred 項目・`MiniHost` 等テスト側の stderr — 生産側 proxy/公開面のみ（テスト側の `Stdio::null` 選択は維持可）
- supervisor の拡張（原因 payload への分岐）— 既存の `Unsupervised` pass-through 動作を壊さない範囲に留め、supervisor が payload で分岐する拡張は後続 Issue
- io 多重化（#120）— stderr drain スレッド追加は #120 の再トリガ記録として ledger へ報告するが、io-mux 実装は #120 の範囲

## Requirements

### Functional / Behavioral

- **R-B1**: L1 audio `process` 等価経路で、pass-through submit（`last_output` または無音へのフォールバックが実際に出力された回数）・dropped submit（submit channel full・strict-alternation slot busy で block が捨てられた回数）・`RESULT_FAILED`（`mark_failed` 要素を観測した回数 — 子の flagged `Reply` 拒否・driver/reader/converge の合成を含む）が非 RT query 面から読める。計数は in-place leg と detached endpoint leg の両経路を含む（endpoint checkout 中も host から読めること — leg の move で観測が欠落しない）
- **R-B2**: 計装は RT 契約に適合する — `RtLeg::process` 経路（in-place + endpoint 双方）で追加の alloc・lock・blocking I/O を発生させない。writer は Relaxed atomic 更新のみ（#104 counter pair 形状）。`rt_l1_process_path_no_alloc`/`rt_l1_endpoint_*` 等の既存 `rt_` テストが無退行
- **R-B3**: 子 stderr は `Stdio::inherit` から piped + 常時 drain へ移行する（または design が確定する opt-in 形 — R-F3）。piped を選択した場合、drain は **全 L1 子で常時動作**し、pipe 満杯による子ブロック（bounded-buffer deadlock）を構造的に防ぐ。drained 内容は bounded buffer（容量は spec/構築時固定・満杯は drop + 計数）に保持され非 RT query で読み出せる。audio/video・dedicated/pooled 全 spawn 経路（`L1Child::spawn_scanned`）に適用
- **R-B4**: `HostNotification` は `WireError`→`HostError::Ipc` collapse・transport EOF・`Hello`/handshake 失敗・spawn 失敗・コマンド応答 timeout・protocol 違反（帰属可能/不能）の原因種別を payload として運ぶ。`ChildExit` の既存契約（exactly-once・reaped `ExitStatus`・`{code:None,signal:None}` = status 未知）は保持し、payload は原因の種別を運ぶ（exit 情報と原因は別概念 — `converge_instance` の「生きている子への protocol 違反」と「まだ死を観測していない」の区別を回復する）
- **R-B5**: payload ・buffer・query 面は全て host 側のみ — wire codec・`WIRE_VERSION`・`Hello.capabilities`・`HostError` tag の変更は行わない（#119 単一最新方針に適合）。`WireError` 種別が wire を渡る必要はない（decode 失敗は host 側で分類する）
- **R-B6**: audio/video parity — video route（dedicated 固定・RT 経路なし）にも R-B3（stderr）・R-B4（原因 payload）を同一契約で適用。audio の underrun 計数（R-B1）は video に適用しない（video に pass-through 語彙なし）
- **R-B7**: L0 route・既存 `Failed` 契約の無退行 — `InProcessHost`/`InProcess` slot は通知も計数も出さない現行のまま、`Failed`→`Destroy` のみ・通知 exactly-once・`process` が `Failed` で `Ok(CONTINUE)`+無音・再起動なしの確定契約を壊さない
- **R-B8**: supervisor との整合 — `Supervisor::poll` の通知 drain（watched 消費・`Unsupervised` pass-through）が新 payload/variant で壊れない。supervisor が payload で分岐する拡張は本 Issue 範囲外（pass-through で透過であること）

### Decision points（design/ADR に委譲 — spec が確定しない内部構造・公開既定値）

- **R-F1**: `ruviform-observability` クレート新設の範囲 — **lead 裁定**: host 内 owner-local counter set（`Arc<AtomicU64>` 系・`rt_state` 先例・最小 scope）を既定案とするが、issue の「candidate first consumer」文言に対し crate 新設を本 Issue 内で行うかは lead 判断。既定案でも #101 trigger Issue への「最初の実需要」記録は行う
- **R-F2**: メトリクス公開面の形 — 暫定案は pull 型 snapshot（`PluginHost::diagnostics`/`L1Diagnostics` 系、`&self`/`&mut self` 非 RT query）。`HostNotification` イベント化（器に合わない）・`rt_bridge` レコード輸送（counts 要求に overkill）は代替案として discovery に記録。粒度（per-handle/per-child/host 全体・warmup underflow を underrun に含めるか・`RESULT_FAILED` 原因内訳の分類有無）は design で確定
- **R-F3**: stderr 既定方針 — 暫定案は常時 `Stdio::piped` + per-child drain スレッド（per-child 3 スレッド目・#120 再トリガ発火の ledger 記録を伴う）。opt-in spec flag（既定 inherit 維持）はコスト懸念が実測された場合の代替案
- **R-F4**: drained stderr の公開形 — 暫定案は per-child（`L1Child`）所有の bounded ring buffer + 非 RT snapshot 読み出し（行単位/bytes・容量既定値は design）。通知イベントへの乗せは `Copy` 制約で不可（文字列）・sink/callback 経路は後続
- **R-F5**: 原因 payload の型 — 暫定案は `Copy` 原因 enum を `HostNotification::InstanceFailed` に payload として追加（`Copy` derive 維持・`#[non_exhaustive]` で additive）。`Copy` を放棄する rich payload（`io::Error` 由来の詳細を保持）は破壊的方向として lead 裁定の余地
- **R-F6**: 原因 enum の語彙と `converge_*` 配線 — reader/driver/request/spawn/handshake 各地点で原因を分類し `converge_child`/`converge_instance`/notify 経路へ通す設計（現行 `converge_child(shared, Option<ChildExit>)` は exit status のみ運ぶ — 原因値を通す引数/構造の設計が要る）

### Verification

- **R-V1**: 単体 — 計装点（pass-through・channel full・slot busy・`RESULT_FAILED`・warmup 境界）の分類・累積・leg swap（`arm`/`arm_apply`）後も計数が持続すること・endpoint checkout 中の読み出し・原因 enum の各地点写像・bounded buffer の満杯 drop+計数
- **R-V2**: proxy E2E — underrun を fixture（`--hang-after-msgs`・kill 等）で発生させ計数が増えること・stderr を emit する worker の出力が drain/buffer 経由で読めること・crash/protocol 違反で通知が原因種別 payload を運ぶこと
- **R-V3**: `Failed` parity — bundled 子の crash で同居全 minted instance が `InstanceFailed`+原因 payload を受け取ること（fan-out parity）・`converge_instance` の帰属可能違反が当該 instance だけ `Failed`+「protocol violation」種別になること
- **R-V4**: audio 無退行 — 既存 `l1_proxy_e2e`/`l1_runner_e2e`/`l1_supervisor_e2e`/`plugin_graph_e2e`・codec 単体・`rt_` 命名テストが無変更でグリーン（特に `rt_l1_process_path_no_alloc` の assert_no_alloc 窓）
- **R-V5**: stderr の deadlock 耐性 — pipe を高頻度/大量に書く worker（または fixture 追加分）で drain が追いつき子がブロックしないこと・buffer 満杯時に drop+計数で子が止まらないこと・heir grandchild が stderr pipe を開き続ける場合に teardown が bounded で収まること
- **R-V6**: video parity — `l1_video_e2e` で video 子の stderr 配管・原因 payload（render timeout・protocol 違反）が audio と同一契約で動くこと・audio の underrun 計数が video route に混入しないこと
- **R-V7**: `HostNotification` 系の doc/API 一貫 — `Copy`/`#[non_exhaustive]` 方針・`arm_retire_overflows`/`DropCollector::overflows` 先例との語彙整合・`cargo doc` 警告なし

## Acceptance Criteria

- [ ] **AC1（underrun メトリクス）**: L1 audio `process`（in-place + detached endpoint 双方）で pass-through submit・dropped submit・`RESULT_FAILED` の発生回数が非 RT query 面から観測できる。endpoint checkout 中も host から読め、leg swap で失われない。fixture で underrun を発生させた後に計数が期待値になる E2E 証跡がある
- [ ] **AC2（RT 契約無退行）**: writer 計装が `RtLeg::process` 経路（in-place + endpoint）で alloc・lock・blocking を発生させず、`rt_l1_*`・`rt_l1_endpoint_*` 等の既存 `rt_` 命名テストが assert_no_alloc 窓を含め無退行でグリーン
- [ ] **AC3（stderr 配管）**: 全 L1 子（audio dedicated/pooled・video dedicated）で stderr が piped+常時 drain され、drained 内容が bounded buffer（満杯 drop+計数）を経て非 RT query で読み出せる。stderr を大量に書く子で子ブロックが発生せず、buffer 満杯で drop+計数に留まること・heir grandchild で teardown が bounded であること
- [ ] **AC4（失敗原因 payload）**: `HostNotification` が失敗の原因種別（crash/exit・transport EOF・protocol 違反・timeout・spawn/handshake 失敗等）を payload として運ぶ。crash→exit status+種別、`converge_instance` 帰属可能違反→protocol 種別、timeout→timeout 種別が E2E で観測できる。`ChildExit` の既存契約（exactly-once・reaped status）は保持
- [ ] **AC5（wire/契約無変更）**: wire codec・`WIRE_VERSION`・`Hello.capabilities`・`HostError` tag・`read_message`/`write_message` の公開契約が無変更（payload・buffer・query は host 側のみ）。`HostNotification`/`ChildExit` 以外の公開契約に破壊的変更がない
- [ ] **AC6（parity/無退行）**: audio/video 両 route で R-B3/R-B4 が同一契約で動き、bundled 子の同居 instance 全てが原因 payload 付き `InstanceFailed` を受け取る。L0 route・`Failed`→`Destroy`・supervisor `Unsupervised` pass-through・既存 L1 E2E（audio/video/runner/supervisor/graph）が無退行
- [ ] **AC7（CI/監査）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・`doc_status.js audit` blocking 0・#120/#101 trigger ledger への記録（stderr 3 スレッド目・observability 最初の実需要）が反映されている

## Deferred Design Concerns

- `ruviform-observability` / `ruviform-foundation` クレート新設 | 第一 consumer 候補の trigger は名目充足するが、新設は DAG・公開 API 方針・ADR を含む別 Issue 級の作業 | #101 trigger Issue での採否（本 Issue の実需要記録を入力として） | カウンタ形状が host 内実装に留まり、crate 化時に移行が要る
- `ruviform_rt::observability_poc` 本番昇格 | `#[doc(hidden)]` 実験 API・昇格判断は trigger Issue の権限 | 同 trigger Issue | 実測形状を host 内で再実装する重複が残る（小さい — counter pair は薄い wrapper）
- 構造化 logging・tracing/metrics backend・export | #104 verdict で RT 経路不採用・需要なし | ドメインをまたぐ構造化出力需要 | 観測データの外部連携は手作業
- `HostError::Ipc` への payload 追加 | wire tag を持つ契約型・#119 bump 規則に触れる別論点 | waiter `Err` にも原因が要る需要 | `Err(Ipc)` の切り分けは通知側 payload でのみ可能
- `BundleFailed` 相関通知 | #117 deferred のまま | observer が相関を明示要求 | 同一子の死が N 件の `InstanceFailed` で届く（原因 payload で相関は推測可能）
- drained stderr の sink/callback 経路 | v1 は buffer+snapshot が最小 | 外部ログ基盤への流れが要る需要 | 読み出しは pull のみ
- supervisor の原因 payload 分岐 | 現行の `Unsupervised` pass-through で透過的・拡張は別論点 | supervisor が失敗種別で recovery 方針を分ける需要 | 分岐しない限り payload は素通しのまま
- io 多重化（#120）| stderr drain で per-child 3 スレッド目が増えるが、io-mux は #120 の範囲 | #120 再トリガ条件の他の充足 | per-child 3 スレッド継続

## Sources and Relations

- Issue [#121](https://github.com/scarecrow173/ruviform/issues/121)（要求元 — #76 closeout 残余 3 件・`ruviform-observability` first consumer 候補・#104 verdict 整合要求）
- 下流 discovery: [issue-121-diagnostics.md](../discovery/issue-121-diagnostics.md) — 全コード観測・decision matrix（軸 1-6）・OPEN 論点 Q1-Q10
- 一次文書: [plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（L323 stderr deferred 行・L325 #120 再トリガ・L255-261 通知型）・[issue-76-plugin-l1-closeout-task-7-implementation.md](../impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md)（3 残件の出典 Follow-ups）・[foundation-observability-minimal-api.md](../designs/foundation-observability-minimal-api.md)（#101 候補・trigger・依存 DAG）・[rt-observability-poc.md](../impl/ir/rt-observability-poc.md)（#104 verdict — counter pair・bridge・verdict 制約）・[issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md)（Bundled 既定・per-child 所有粒度）・[issue-119-l1-wire-version-policy.md](issue-119-l1-wire-version-policy.md)（wire 変更不要の裏付け）・[issue-116-supervisor.md](../designs/issue-116-supervisor.md)（payload 持ち `SupervisionEvent` 先例・pass-through 契約）
- ローカル一次証拠: `crates/ruviform-plugin-host/src/l1/{data,mod,video}.rs`・`composite.rs`・`endpoint.rs`・`supervisor.rs`・`crates/ruviform-plugin-core/src/{wire,boundary}.rs`・`crates/ruviform-plugin-export/src/lib.rs`・`crates/ruviform-rt/src/observability_poc.rs`
