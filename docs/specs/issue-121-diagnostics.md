---
id: "SPEC-034O7hTzfdwtNrUQom8ISw"
type: "spec"
status: "approved"
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
  derived-by:
    - "../designs/issue-121-l1-diagnostics.md"
    - "../adr/issue-121-l1-diagnostics-policy.md"
    - "../plans/implement-issue-121-l1-diagnostics.md"
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
2. **子 stderr 配管** — `Stdio::inherit` を bounded piped stderr drain + 診断経路へ置き換える（`plugin-l1-out-of-process.md` L323 deferred item の re-engagement）。保持バッファの満杯や診断の消費待ちによって drain が止まり、pipe が詰まる deadlock を避けるため、常時 drain が前提条件
3. **失敗原因 payload** — `WireError`→`HostError::Ipc` collapse・transport EOF/timeout・spawn/handshake 失敗等で現在 `Err(Ipc)`/`ChildExit{None,None}` に畳み込まれる原因詳細を host 側に運ぶ。minted instance は `HostNotification` の payload、pre-mint 失敗（spawn・drain 初期化・handshake）は error result + host 診断レコードで表現する（いずれも wire を通らないため wire codec・`WIRE_VERSION` 変更不要 — 原因 enum は host 側のみ）

Issue 本文は本実装を `ruviform-observability`（#101 foundation 候補）の **candidate first consumer** と位置づける。本 Issue で同クレートを新設し、L1 diagnostics を初消費者とする。#104 PoC verdict（counter pair・`rt_bridge` 形状・`reserved: [u64; 2]` 規約・RT 経路 logging 不採用）との整合が要求条件。公開面の正確な形は design フェーズで確定する。

## Why Now

- `docs/designs/plugin-l1-out-of-process.md` L323 deferred 行（stderr 配管 — trigger「子プロセスのログ解析要求」）・L325（#120 再トリガとして本 Issue の stderr 配管を明記）の re-engagement
- `docs/impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md` Follow-ups の 3 残件 — underrun metrics（「`HostNotification` 面ができたため metrics surface の置き場候補として composite を再評価可」）・`WireError`→`Ipc` collapse（「通知 payload に原因詳細を持たせるかは将来の通知型拡張で再評価可」）・stderr piping（「`Stdio::inherit` のまま — deferred design concern」）
- #104 PoC が完了済み（verdict: ACCEPT WITH NON-BLOCKING RESIDUAL NOTES）— counter pair・`rt_bridge`・drop-on-full+overflow count の実測形状が参照形として利用可能
- #101/#103 の `rt_bridge`/`telemetry` 新設トリガは「最初の実需要 + #104 PoC 結果」— #121 は最初の実需要として `ruviform-observability` を新設する（R-F1）
- 実プラグインの L1 運用（#114 CLAP/VST3 runner）が進み、子プロセスのクラッシュ・hang・protocol 違反の原因診断が現状 `Err(Ipc)` + inherit stderr の目視に依存している — 観測手段なしの運用が継続するほど診断不能な失敗が蓄積する

## Users and Value

- ホストアプリケーション/エンジン実装者 — L1 プラグインの underrun（音切れ・レイテンシ悪化の前兆）を定量的に監視し、子プロセス失敗の原因（crash・timeout・protocol 違反・spawn 失敗）を構造化して受け取れる
- プラグイン/アダプタ開発者 — L1 子の stderr（panic・dlopen 失敗・`Hello` 前の usage rejection）をホスト側で捕捉・保持し、クラッシュ直前の診断を失わない
- 運用/サポート — `Err(Ipc)` 単発では切り分け不能だった失敗（transport EOF vs protocol 違反 vs timeout）を原因種別で分類できる

## Scope

### In Scope

- L1 audio `process` 等価経路（in-place `L1Instance::process` + detached `L1Endpoint::process` の両方）の pass-through / dropped submit / `RESULT_FAILED` 計数 — writer 側計装は wait-free・no-alloc・Relaxed bookkeeping で RT 契約に適合すること（`rt_l1_*` テストの assert_no_alloc 窓を退行させない）
- `ruviform-observability` クレートの新設と L1 diagnostics での利用 — #101 の責務・依存方向および #104 の PoC verdict に整合する最小範囲を design で確定する
- 計数値の公開面 — RT 経路を汚さない非 RT query 面（pull 型）。`PluginHost`（composite）を surface 候補として再評価し確定する。per-handle・per-child・host 全体どの粒度かは design 論点（R-F2）
- 子 stderr の bounded piped drain — 全 L1 子（audio dedicated/pooled・video dedicated）に対し `Stdio::inherit` を置き換える方針の確定と実装。drained 内容は bounded buffer（満杯 drop + 計数 — #104 verdict の drop-on-full+count 方針）に保持し、非 RT query 面で読み出せること
- 失敗原因の host 側表現 — minted instance 向けは `HostNotification` の拡張として `WireError`/`io` transport/timeout/protocol 違反の原因種別を `Copy` payload（または design が確定する形）で運び、pre-mint 失敗（spawn・`Hello`/handshake・drain 初期化）は spawn/create の error result（cause + 相関 ID）+ host bounded 診断レコードで運ぶ。`ChildExit` の既存契約（exactly-once・reaped status）は保持
- audio/video 両 route の parity — video の `VideoL1Instance` も `spawn_scanned`・`try_recv_notification`・notify 機構を共有するため、stderr 配管と原因 payload は video にも同じ契約で適用する（underrun 計数は audio RT 経路に固有）
- 検証 — 単体（計装点・原因 enum 写像・buffer bound）・proxy E2E（underrun 発生→計数観測・stderr 捕捉→読み出し・失敗→payload 観測）・RT レーン無退行・`BundleFailed` parity（bundled 子の stderr・原因 payload が同居インスタンスへ正しく帰属/共有されること）

### Out of Scope

- `ruviform-foundation` クレートの新設 — `ruviform-observability` の新設とは別の判断
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
- **R-B3**: 子 stderr は `Stdio::inherit` から piped + 常時 drain へ移行する。drain は **全 L1 子で常時動作**し、保持バッファ満杯や診断の消費待ちで pipe の読み取りを停止しない。drained 内容は bounded buffer（容量は spec/構築時固定・満杯は drop + 計数）に保持され非 RT query で読み出せる。OS pipe やスケジューリングによる一時的な書き込みブロックの不在は保証しない。audio/video・dedicated/pooled 全 spawn 経路（`L1Child::spawn_scanned`）に適用し、Bundled 時も診断情報の帰属は子単位とする。heir が pipe を保持しても drain は子終了観測 + `STDERR_DRAIN_GRACE`（500 ms・構築時定数・延長なし）で自発終了し read fd を閉じる — 終了時間は OS 別の限定保証（POSIX 限定条件下で制御可能・Windows は `PeekNamedPipe` 同期 I/O 残余リスク受容）
- **R-B4**: minted instance の `HostNotification` は `WireError`→`HostError::Ipc` collapse・transport EOF・コマンド応答 timeout・protocol 違反（帰属可能/不能）・子終了の原因種別を payload として運ぶ。pre-mint 失敗（spawn・drain 初期化・`Hello`/handshake）は minted handle が存在しないため `InstanceFailed` を発行せず、spawn/create の error result（cause + diagnostics ID）+ diagnostics ID で問い合わせ可能な host bounded 診断レコードで表現する（ID は失敗確定時の retention 登録で連続採番 — 試行前採番は行わない。found/evicted/not-found を区別できる query を提供）。`ChildExit` の既存契約（exactly-once・reaped `ExitStatus`・`{code:None,signal:None}` = status 未知）は保持し、payload は原因の種別を運ぶ（exit 情報と原因は別概念 — `converge_instance` の「生きている子への protocol 違反」と「まだ死を観測していない」の区別を回復する）
- **R-B5**: payload ・buffer・query 面は全て host 側のみ — wire codec・`WIRE_VERSION`・`Hello.capabilities`・`HostError` tag の変更は行わない（#119 単一最新方針に適合）。`WireError` 種別が wire を渡る必要はない（decode 失敗は host 側で分類する）
- **R-B6**: audio/video parity — video route（dedicated 固定・RT 経路なし）にも R-B3（stderr）・R-B4（原因 payload）を同一契約で適用。audio の underrun 計数（R-B1）は video に適用しない（video に pass-through 語彙なし）
- **R-B7**: L0 route・既存 `Failed` 契約の無退行 — `InProcessHost`/`InProcess` slot は通知も計数も出さない現行のまま、`Failed`→`Destroy` のみ・通知 exactly-once・`process` が `Failed` で `Ok(CONTINUE)`+無音・再起動なしの確定契約を壊さない
- **R-B8**: supervisor との整合 — `Supervisor::poll` の通知 drain（watched 消費・`Unsupervised` pass-through）が新 payload/variant で壊れない。supervisor が payload で分岐する拡張は本 Issue 範囲外（pass-through で透過であること）

### Decision points（design/ADR に委譲 — spec が確定しない内部構造・公開既定値）

- **R-F1**: `ruviform-observability` クレート新設 — 本 Issue で新設し、L1 diagnostics を初消費者とすることは確定。API の範囲、#101/#104 との整合および依存方向は design/ADR で確定する
- **R-F2**: メトリクス公開面の形 — 暫定案は pull 型 snapshot（`PluginHost::diagnostics`/`L1Diagnostics` 系、`&self`/`&mut self` 非 RT query）。`HostNotification` イベント化（器に合わない）・`rt_bridge` レコード輸送（counts 要求に overkill）は代替案として discovery に記録。粒度（per-handle/per-child/host 全体・warmup underflow を underrun に含めるか・`RESULT_FAILED` 原因内訳の分類有無）は design で確定
- **R-F3**: stderr 既定方針 — 常時 `Stdio::piped` + per-child drain スレッドとし、Bundled でも子単位に帰属させることは確定。実装が着地した時点で per-child 3 スレッド目と #120 再トリガ条件の関係を ledger に記録する
- **R-F4**: drained stderr の公開形 — 暫定案は per-child（`L1Child`）所有の bounded ring buffer + 非 RT snapshot 読み出し（行単位/bytes・容量既定値は design）。通知イベントへの乗せは `Copy` 制約で不可（文字列）・sink/callback 経路は後続
- **R-F5**: 原因 payload の型 — 暫定案は `Copy` 原因 enum を `HostNotification::InstanceFailed` に payload として追加し、`Copy` derive を維持する。enum 自体の `#[non_exhaustive]` は既存 struct variant の field 追加を互換にするものではないため、既存の field pattern に対するソース互換性を評価する。`Copy` を放棄する rich payload（`io::Error` 由来の詳細を保持）も破壊的方向として検討する
- **R-F6**: 原因 enum の語彙と `converge_*` 配線 — reader/driver/request/spawn/handshake 各地点で原因を分類し通知へ通す設計。現行 `converge_child(shared, Option<ChildExit>)` は exit status のみ運び、spawn/初期 handshake の失敗時には live instance や handle がないため `InstanceFailed` にそのまま載せることはできない。handle のない失敗は design で「error result + 診断レコード（`phase`/`cause`・相関 ID・bounded retention）」に確定方向 — `InstanceFailed` は minted 限定を維持。AC4 は同方向に再定義済み

### Verification

- **R-V1**: 単体 — 計装点（pass-through・channel full・slot busy・`RESULT_FAILED`・warmup 境界）の分類・累積・leg swap（`arm`/`arm_apply`）後も計数が持続すること・endpoint checkout 中の読み出し・原因 enum の各地点写像・bounded buffer の満杯 drop+計数
- **R-V2**: proxy E2E — underrun を fixture（`--hang-after-msgs`・kill 等）で発生させ計数が増えること・stderr を emit する worker の出力が drain/buffer 経由で読めること・crash/protocol 違反で通知が原因種別 payload を運ぶこと
- **R-V3**: `Failed` parity — bundled 子の crash で同居全 minted instance が `InstanceFailed`+原因 payload を受け取ること（fan-out parity）・`converge_instance` の帰属可能違反が当該 instance だけ `Failed`+「protocol violation」種別になること
- **R-V4**: audio 無退行 — 既存 `l1_proxy_e2e`/`l1_runner_e2e`/`l1_supervisor_e2e`/`plugin_graph_e2e`・codec 単体・`rt_` 命名テストが無変更でグリーン（特に `rt_l1_process_path_no_alloc` の assert_no_alloc 窓）
- **R-V5**: stderr の deadlock 耐性 — pipe capacity を超える量を書き、診断を読み出さずに処理を継続する worker（または fixture 追加分）が期限内に完了すること・buffer 満杯時に drop+計数で drain が継続すること・heir grandchild が stderr pipe を開き続ける場合に teardown が bounded で収まること
- **R-V6**: video parity — `l1_video_e2e` で video 子の stderr 配管・原因 payload（render timeout・protocol 違反）が audio と同一契約で動くこと・audio の underrun 計数が video route に混入しないこと
- **R-V7**: `HostNotification` 系の doc/API 一貫 — `Copy`/`#[non_exhaustive]` 方針・`arm_retire_overflows`/`DropCollector::overflows` 先例との語彙整合・`cargo doc` 警告なし

## Acceptance Criteria

- [ ] **AC1（underrun メトリクス）**: L1 audio `process`（in-place + detached endpoint 双方）で pass-through submit・dropped submit・`RESULT_FAILED` の発生回数が非 RT query 面から観測できる。endpoint checkout 中も host から読め、leg swap で失われない。fixture で underrun を発生させた後に計数が期待値になる E2E 証跡がある
- [ ] **AC2（RT 契約無退行）**: writer 計装が `RtLeg::process` 経路（in-place + endpoint）で alloc・lock・blocking を発生させず、`rt_l1_*`・`rt_l1_endpoint_*` 等の既存 `rt_` 命名テストが assert_no_alloc 窓を含め無退行でグリーン
- [ ] **AC3（stderr 配管）**: 全 L1 子（audio dedicated/pooled・video dedicated）で stderr が piped+常時 drain され、drained 内容が bounded buffer（満杯 drop+計数）を経て非 RT query で読み出せる。診断を消費しない状態で pipe capacity を超えて書く fixture が期限内に完了し、buffer 満杯でも drain が継続すること・heir grandchild で teardown が bounded であること
- [ ] **AC4（失敗原因 payload）**: minted instance の `HostNotification` が失敗の原因種別（crash/exit・transport EOF・protocol 違反・timeout 等）を payload として運ぶ。pre-mint 失敗（spawn・drain 初期化・`Hello`/handshake）は spawn/create の error result に cause + diagnostics ID を載せて直接 caller に報告し、host が diagnostics ID で問い合わせ可能な bounded 診断レコード（`phase`/`cause` 区別・found/evicted/not-found 識別）を保持する — mint されなかった instance の `InstanceFailed` は発行しない。retention eviction・結果の未消費は通知の guaranteed delivery を意味しない。crash→exit status+種別、`converge_instance` 帰属可能違反→protocol 種別、timeout→timeout 種別、pre-mint→`phase`/`cause` 区別が E2E で観測できる。`ChildExit` の既存契約（exactly-once・reaped status）は保持
- [ ] **AC5（wire/契約無変更）**: wire codec・`WIRE_VERSION`・`Hello.capabilities`・`HostError` tag・`read_message`/`write_message` の公開契約が無変更（payload・buffer・query は host 側のみ）。`HostNotification`/`ChildExit` 以外の公開契約に破壊的変更がない
- [ ] **AC6（parity/無退行）**: audio/video 両 route で R-B3/R-B4 が同一契約で動き、bundled 子の同居 instance 全てが原因 payload 付き `InstanceFailed` を受け取る。L0 route・`Failed`→`Destroy`・supervisor `Unsupervised` pass-through・既存 L1 E2E（audio/video/runner/supervisor/graph）が無退行
- [ ] **AC7（CI/監査）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・`doc_status.js audit` blocking 0・#120/#101 trigger ledger への記録（stderr 3 スレッド目・observability 最初の実需要）が反映されている

## Deferred Design Concerns

- `ruviform-foundation` クレート新設 | 本 Issue の確定範囲は `ruviform-observability` のみ | 別の実需要と設計判断 | 他の foundation 候補は未実装のまま
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
- 一次文書: [plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（L325 stderr deferred 行・L327 #120 再トリガ・通知型行）・[issue-76-plugin-l1-closeout-task-7-implementation.md](../impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md)（3 残件の出典 Follow-ups）・[foundation-observability-minimal-api.md](../designs/foundation-observability-minimal-api.md)（#101 候補・trigger・依存 DAG）・[rt-observability-poc.md](../impl/ir/rt-observability-poc.md)（#104 verdict — counter pair・bridge・verdict 制約）・[issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md)（Bundled 既定・per-child 所有粒度）・[issue-119-l1-wire-version-policy.md](issue-119-l1-wire-version-policy.md)（wire 変更不要の裏付け）・[issue-116-supervisor.md](../designs/issue-116-supervisor.md)（payload 持ち `SupervisionEvent` 先例・pass-through 契約）
- ローカル一次証拠: `crates/ruviform-plugin-host/src/l1/{data,mod,video}.rs`・`composite.rs`・`endpoint.rs`・`supervisor.rs`・`crates/ruviform-plugin-core/src/{wire,boundary}.rs`・`crates/ruviform-plugin-export/src/lib.rs`・`crates/ruviform-rt/src/observability_poc.rs`
