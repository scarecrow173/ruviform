---
id: "DISC-034O7hCIm5rGi3tA4rmxuc"
type: "discovery"
status: "active"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/plugin-l1-out-of-process.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../designs/issue-118-video-l1.md"
    - "../designs/issue-116-supervisor.md"
    - "../designs/foundation-observability-minimal-api.md"
    - "../designs/shared-foundations-and-modularization-policy.md"
    - "../designs/issue-122-shared-platform-process-ipc-eval.md"
    - "../impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md"
    - "../impl/ir/rt-observability-poc.md"
    - "../impl/ir/issue-76-l1-shm-data-plane-task-4-implementation.md"
    - "../impl/ir/issue-76-plugin-host-composite-task-5-implementation.md"
    - "../impl/exp/rt-observability-poc-20261023.md"
    - "../specs/issue-119-l1-wire-version-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/plugin-l1-out-of-process.md"
    - "../impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md"
    - "../designs/foundation-observability-minimal-api.md"
    - "../impl/ir/rt-observability-poc.md"
    - "../designs/issue-117-bundle-child-process.md"
  derived-by:
    - "../specs/issue-121-diagnostics.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/123"
  defers: []
  deferred-by: []
---

# Issue #121 L1 診断・可観測性（underrun メトリクス・子 stderr 配管・失敗原因 payload）

## Exploration Goal

Issue [#121](https://github.com/scarecrow173/ruviform/issues/121) は **#76 closeout の残余フォローアップ 3 件**をまとめた L1 診断・可観測性の実装 Issue である:

1. **underrun メトリクス** — pass-through submit・dropped submit・`RESULT_FAILED` の観測面（`HostNotification` / composite を surface 候補として再評価）
2. **子 stderr 配管** — `Stdio::inherit` → bounded piped stderr drain + 診断経路（#76 design L323 deferred item の re-engagement）
3. **失敗原因 payload** — `WireError`→`HostError::Ipc` collapse で失われる原因詳細を通知型の payload として運ぶ

Issue 本文は本 Issue を「`ruviform-observability`（#101 foundation）の最初の consumer 候補」と位置づけ、「#104 PoC verdict（counter pair・`rt_bridge` 形状）との整合」を要求条件とする。

トリガ（upstream の deferred 行 — 本 Issue がその re-engagement）:

- `docs/designs/plugin-l1-out-of-process.md` L323 deferred 行 —「stderr ログ配管 | inherit で運用可能・診断価値は副次的 | 子プロセスのログ解析要求 | クラッシュ時の手がかりが stderr 依存」
- `docs/impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md` Follow-ups — underrun metrics（「`HostNotification` 面ができたため metrics surface の置き場候補として composite を再評価可」）・`WireError`→`HostError::Ipc` collapse（「通知 payload に原因詳細を持たせるかは将来の通知型拡張で再評価可」）・stderr piping
- `docs/designs/issue-117-bundle-child-process.md` L395 deferred 行 — `BundleFailed` 相関通知（per-instance fan-out では「同一子の死」を 1 イベントとして処理できない）

本探索が渡す成果物: (a) 3 つの scope 項目それぞれの現行コード形状と観測手段の有無の正確な特定、(b) stderr 配管が持つ bounded-buffer deadlock・スレッド構成・所有粒度（per-child vs per-instance）の制約整理、(c) 失敗原因が失われる具体地点の列挙と payload 化の経路、(d) `ruviform-observability` 候補の現状と #104 verdict が形状に課す制約、(e) design/spec フェーズへ渡す decision matrix と OPEN 論点。最終的な公開 API・配置・crate 新設の確定権限は design/ADR フェーズにある。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — design 論点ではない）

- **`HostNotification` は `#[non_exhaustive]` の `Copy` enum・単一 variant** — `composite.rs` L213-225。`InstanceFailed{handle, exit}` のみ。payload を `Copy` でない型へ広げると `Copy` derive を落とす破壊的変更になる（crate は `publish = false` の未公開 0.1.0 だが、方向は ADR 級の判断）。同じ制約は `SupervisionError` が doc で明記している（`supervisor.rs` L312-315「stays `Copy` — future payloads are limited to `Copy` types; dropping `Copy` for a richer payload is a separate breaking-change decision」）。
- **wire は単一最新バージョン（v3）・交渉機構なし・非互換変更は `WIRE_VERSION` bump** — `docs/specs/issue-119-l1-wire-version-policy.md`（implemented）。新動詞・新 tag は bump 要。`Hello.capabilities` は v3 で reserved-zero 維持（`wire.rs` L595-598）。
- **`PluginHost` は `!Send`/`!Sync`・単一スレッド** — `composite.rs` L351-355。`process`/`plugin_mut` が RT-suitable、`execute`/`register_*`/`prepare_endpoint`/`try_recv_notification` は non-RT only（L340-349）。RT 経路の計装は wait-free・no-alloc が契約。
- **`ruviform-rt → foundation` は非既定依存路線** — `docs/designs/foundation-observability-minimal-api.md` L306-309。rt プリミティブ（`channel`/`retire_channel`/`slot`）へ foundation 型を適用するには依存 DAG 再設計と ADR 改訂が先行する。逆に **domain crate（plugin-host）→ foundation/observability の下向き依存は候補として許容**（同 L297-300 — イベント発行・ポート型に限定）。
- **`ruviform-observability` / `ruviform-foundation` クレートは未新設・構成候補のまま** — `crates/` 一覧に実在しない（計 19 クレート）。`docs/designs/foundation-observability-minimal-api.md`（approved）は API スケッチと新設時期トリガのみを確定: `telemetry`・`rt_bridge` は「構成候補として維持・要 PoC」、`metrics`/`logging`/`profiling`/`diagnostics`/`debug`/`export` は保留。クレート新設トリガは「`rt_bridge` または logging/metrics/diagnostics の **最初の実需要** + #104 PoC 結果」（L321-322）— **#121 はこの「最初の実需要」の候補**。
- **#104 PoC verdict: ACCEPT WITH NON-BLOCKING RESIDUAL NOTES（承認済み・本番昇格ではない）** — `docs/impl/ir/rt-observability-poc.md`。scaffolding は `ruviform_rt::observability_poc`（`#[doc(hidden)]`・実験 API・本番契約対象外）に封じ込め。verdict が形状に課す制約: counter pair（`Arc<AtomicUsize>`・Relaxed fetch_add、計装デルタ +3.99 ns/op）・`rt_bridge`（rtrb 上の fixed-capacity ring・drop-on-full + 共有 overflow カウンタ・bounded drain — record ~5.4-5.7 ns/op・drain ~4.1/record）・レコードは固定サイズ `Copy` + `reserved: [u64; 2]` 規約・タイムスタンプは呼出し側提供 epoch + 必要箇所のみスタンプ（`Instant::now` ~29 ns/op）・**logging バックエンドは RT 経路不採用**（Subscriber 側で任意の alloc/blocking が走り得る構造的理由）・metrics/profiling バックエンドは需要なしで延期。
- **stderr は per-process（per-child）の資源** — #117 で Bundled 既定（1 子 ≤64 インスタンス）。stderr 行を個別インスタンスへ帰属する一般的手法は存在しない（子が書くものはインスタンス id を含まない）。配管の所有単位は `L1Child`（子スコープ）。
- **#120 deferred 行が本 Issue を明示的な再トリガとして挙げる** — `plugin-l1-out-of-process.md` L325:「再トリガ: … / **#121 stderr 配管導入（3 スレッド目）** / …」。stderr drain スレッドの追加は per-child スレッド構成（現行 2 — `l1/mod.rs` L33-36 の thread model 表）を 3 へ変える最初の変更であり、io 多重化評価の再トリガ条件でもある。
- **自動再起動は host util 層の opt-in `Supervisor` の責務** — #116 確定。supervisor は通知を `poll` で drain し、watched `InstanceFailed` を消費・unwatched は `SupervisionEvent::Unsupervised` へ素通し（`supervisor.rs` L294-301・L915-933）。

### コード観測事実（confirmed — 現行構造の正確な形）

#### 軸 A: underrun メトリクス — イベント発生点と観測手段の欠如

pass-through / dropped submit / `RESULT_FAILED` は全て **`l1/data.rs` の `RtLeg`**（RT caller 側）と **`l1/mod.rs` の driver/reader**（非 RT）の 2 側で発生する。**カウンタ・メトリクス・構造化記録は全サイトに存在しない**（棚卸し時点 `90c90f5`）。

**RT caller 側 — `RtLeg::process`（`l1/data.rs` L601-690。pass-through 判定表はモジュール doc L60-68）:**

| 条件 | サイト | 現在の外向き信号 |
| --- | --- | --- |
| mirror `Failed` → 無音 + `Ok(CONTINUE)` | `data.rs` L612-615 | なし（無音化のみ） |
| `submit.try_push` 失敗（submit channel full）→ seq 消費・doorbell なし → pass-through | `data.rs` L643-656（コメント L654-655「Channel full → submission abandoned → pass-through」） | なし |
| slot busy（`cleared[i] < last_pushed[i]`）→ block drop → pass-through | `data.rs` L641・L657 | なし |
| `block - lookahead` underflow（warmup horizon）→ 定義無音 | `data.rs` L662-666 | なし（定義動作 — underrun として数えるかは設計論点） |
| expected result 未到達 または `RESULT_FAILED` flagged → `last_output` pass-through | `data.rs` L676-689 | なし |

**`RESULT_FAILED` 要素の生産側（非 RT — 全て `mark_failed` 経由 `data.rs` L138-140）:**

| 経路 | サイト | 意味 |
| --- | --- | --- |
| 子が `BlockIn{seq}` に flagged `Reply{seq, Err}` で応答 | reader demux（`data.rs` L136-138 コメント・`mod.rs` demux の `Complete{result: None}` 行 L885-891） | slot parse 失敗・lifecycle 拒否・plugin panic |
| driver `drain_submits` の doorbell なし drop — stale generation・`Destroying`/`Failed`/`Creating` entry・dead/shutting-down child・view 欠落 | `mod.rs` L1483-1607（5 箇所の `mark_failed` 合成） | seq ごとに exactly 1 補完を合成（alternation gate の wedge 防止） |
| `converge_child` の全 in-flight doorbell | `mod.rs` L795-799 | 子スコープ収束 |
| `converge_instance` の当該 instance の in-flight | `mod.rs` L858-868 | 帰属可能違反 |
| `remove_instance` の残存 doorbell | `mod.rs` L691-695 | `DestroyInstance` reply-granular 解放 |

**計装のための構造的観察:**

- **`RtLeg` は in-place 経路と endpoint 経路で共有される** — `L1Instance::process`（`mod.rs` L2793-2814）は `self.rt.as_mut()` の in-place leg を、`L1Endpoint::process`（`endpoint.rs` L520-592）は checkout された同型 leg を呼ぶ。**単一の `RtLeg::process` が全 pass-through 分岐を持つ**ため計装は 1 点で済む — ただし **leg は `prepare_endpoint` で move され `Drop` で戻る**（`mod.rs` L2377 `rt: Option<RtLeg>`・L2397 `return_rx`）ため、checkout 中も host から読めるカウンタの置き場は **leg 自身ではなく共有状態**でなければならない。先例は `rt_state: Arc<AtomicU8>`（`InstShared` L413 — RT 側が `Acquire` 読みする lock-free mirror）であり、**`Arc<AtomicU*>` を leg と `InstShared`/facade で共有する形が既存パターンに整合**する（#104 counter pair と同形状）。
- 既存の観測点先例は `L1Instance::arm_retire_overflows`（`mod.rs` L3163-3167 — `#[cfg_attr(not(test), expect(dead_code))]` で **現状 test-only**・`DropCollector::overflows` を読む `&self` 非 drain 観測点）。
- 通知キューは per-instance `std::mpsc::channel()`（**unbounded** — `mod.rs` L1899・`InstShared.notify_tx` L420）。`PluginHost::try_recv_notification` は全 slot を線形走査して `drain_endpoint_channels` + 各 `notify_rx.try_recv`（`composite.rs` L1141-1172）。`mpsc::Sender::send` は内部で allocate し得るため RT 経路には使えないが、通知送信は全て非 RT スレッド側。
- `drain_to`（`data.rs` L799-824）は result channel を bounded に pop — drained 要素の `RESULT_FAILED` bit は RT 側で観測可能（`el & RESULT_FAILED` — L677）。driver/reader 側でも `mark_failed` push 時点で分類可能。**計数を「消費時（RT 側 drain）」にするか「生産時（非 RT 側合成）」にするかで観測粒度が変わる** — 生産側なら drop の原因別（stale gen・Destroying・dead child・reader rejection）内訳が取れる。

#### 軸 B: 子 stderr 配管 — 現行 inherit と 3 スレッド目の制約

- **spawn 地点は `L1Child::spawn_scanned` の 1 箇所**（`l1/mod.rs` L1844-1861）: `.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())`・コメント「stderr piping is a deferred design concern — inherit keeps child diagnostics visible without extra plumbing」（L1850-1852）。audio（dedicated `L1Instance::spawn`・pooled `L1Child`）と video（`VideoL1Instance::spawn` → `video.rs` L150-154 `spawn_scanned`）の **全 L1 子がこの 1 経路を通る** — stderr 方針の変更点は 1 箇所。
- **子側 stderr 生産者**: `ruviform-plugin-export` の serve loop（`lib.rs` L226 usage rejection・L281 undecodable frame・L292 汎用 error・L311/L330 control-plane write failed・L344）・`l1-test-worker.rs` L151・`l1-test-video-worker.rs` L169・adapter runner（`ruviform-plugin-clap-l1-runner.rs` L48/L55・`ruviform-plugin-vst3-l1-runner.rs` L49/L56 — plugin 解決/ロード失敗は **stderr + exit（`Hello` 前）**の defined rejection）。実プラグインの stderr は自由形式（printf・panic メッセージ・ASAN 出力等）。**pre-`Hello` 失敗の診断情報は stderr にしか存在しない** — handshake 失敗が `Err(Ipc)` で返る現行では、原因解明は inherit された stderr の目視に依存する。
- **bounded-buffer deadlock（配管の必須条件）**: `Stdio::piped` で誰も drain しない stderr は OS pipe capacity（~64 KiB）で満杯になり、子の次の write がブロック → serve loop が wedge → `command_timeout` で `Failed` 収束という **配管が新たに持ち込む故障モード**。piped を選ぶなら **drain は常時必須**であり、drain する専用スレッド（per-child 3 スレッド目）か drain を兼務する機構が要る。現行 2 スレッドモデル（driver=stdin 所有者 + reader=stdout 所有者 — `mod.rs` L33-36）は stderr の所有先を持たない。
- **heir grandchild の問題**: `--leave-heir` fixture（`l1-test-worker.rs` L25・テスト `destroy_detaches_reader_when_heir_holds_stdout` `l1_proxy_e2e.rs` L478-483）は stdio 継承の grandchild が親死後も stdout pipe を開いたままにする — **stderr も同じく開き続ける**。stdout reader は detached never-join（`mod.rs` L606-610）でこの問題を回避済み。stderr drain スレッドも **EOF 未到達を正常として許容する detach 方針**が要る（join に bound を掛けるか、drain を非 join にするか）。
- **テスト側先例**: runner E2E の `MiniHost` は stderr を `Stdio::null`（`l1_runner_e2e.rs` L85-87・`l1_video_runner_e2e.rs` L85-87）— 制御可能だが診断を捨てる。
- **#117 以降の所有粒度**: stderr は `L1Child`（子）の資源 — Bundled では N インスタンスが 1 stderr を共有するため、drain 済み内容を per-instance 通知で fan-out するか子スコープの面に出すかは設計論点。

#### 軸 C: `WireError` → `HostError::Ipc` collapse — 原因が失われる地点

- **wire 層のエラー運搬形**: `read_message`/`write_message` は `io::Result` — `WireError` は `io::Error{kind: InvalidData}` に wrap されて運ばれる（`wire.rs` L3026 decode・L3071 encode）。transport の `io::Error` はそのまま、EOF は `UnexpectedEof`、over-limit `payload_len` は `InvalidData`（L2957-2962）。`WireError` 自体は `Copy` 5 variant（`wire.rs` L1011-1035: `Truncated`/`Malformed`/`UnknownKind(u8)`/`LimitExceeded`/`UnsupportedVariant`）— **decode 時点では原因種別が完全に利用可能**。
- **原因喪失地点（host 側・全サイト列挙）:**

| 地点 | サイト | 喪失の形 |
| --- | --- | --- |
| reader 主ループ | `mod.rs` L1211-1218 | `let Ok(msg) = wire::read_message(...) else { seen_eof=true; converge_child(shared, None) }` — `io::Error`（clean EOF・mid-frame EOF・`InvalidData(WireError)`・transport error の区別付き）を **まるごと drop** |
| `Hello` handshake | `mod.rs` L1199-1208 | `hello_tx` は `Result<u16, ()>` — first-frame の decode 失敗と missing Hello は同じ `Err(())` |
| spawn hand-off | `mod.rs` L1612 `SpawnRx = mpsc::Receiver<Result<ChildStdout, ()>>` | spawn 失敗の `io::Error`（`spawn()` 結果）を `()` に潰す |
| driver stdin write | `mod.rs` L1448-1456・L1598-1603 | `write_message(...).is_err()` → `converge_and_reap` — EPIPE と encode 失敗の区別を drop |
| コマンド応答 timeout | `mod.rs` L2216-2225 | `Err(Ipc)` + `converge_child` — timeout と transport が同一形 |
| converge 時の waiter 起床 | `mod.rs` L790・L853・L944・L1444 | 全 waiter に裸の `Err(HostError::Ipc)` |
| create 失敗 | `mod.rs` L2004-2007 | `InstanceLimit` 以外は全て `Err(Ipc)`（descriptor mismatch も含む — `scan_match` L1810-1824） |

- **通知側**: `converge_instance`（帰属可能違反・子生存）は `ChildExit{code:None, signal:None}`（`mod.rs` L837-840）、`converge_instance`/`converge_child`（EOF path・reap 前）は同じ `ChildExit{None,None}` を status 未知の意味で送る — **「プロトコル違反で kill した」と「まだ死を観測していない」が現行の payload では区別不能**（`Notify::Pending` 機構 L297-316 が後続の実 status で補完する設計だが、原因種別は運ばない）。
- **`HostError::Ipc` 自体は Copy unit variant（wire tag 7）** — `boundary.rs` L146-147。`HostNotification` は **host 側のみの構成物で wire を通らない** ため、payload へ `WireError` 種別や原因 enum を載せても **wire codec・`WIRE_VERSION`・wire tag の変更は不要**（issue 想定の「wire v3 additive space」への依存なし — payload は wire の外側にある）。
- **payload 持ち通知の先例**: `SupervisionEvent`（`supervisor.rs` L228-308）は `Restarted{old,new,bounds,restored}`・`AttemptFailed{error: HostError}`・`GaveUp{last_error}`・`Orphaned{handle}` の payload 持ち variant を持ち、enum と各 struct variant の双方に `#[non_exhaustive]`。`Supervisor` は watched `InstanceFailed` の `exit` を消費するが `ChildExit` には分岐しない（L220-225「the value is also ambiguous」— 現行 payload の曖昧性を upstream が認識済み）。
- **`io::Error` は `Copy`/`Clone` でない** — そのまま通知 payload には載せられない（`HostNotification: Copy` 制約）。`io::ErrorKind` は `Copy` だが wire の `WireError` 種別までは戻せない（`InvalidData` の内訳は `io::Error::get_ref`→`WireError` への downcast が必要 — 現行コードは downcast しない）。**原因を運ぶなら、捕捉地点で専用の `Copy` 原因 enum（例: spawn/handshake/timeout/transport/protocol(violation kind)/exit 系）に畳み込む設計が要る**。

#### 軸 D: `ruviform-observability` 候補の現状と #104 verdict が課す形状

- クレート非存在（`crates/` 計 19・`Cargo.toml` members に未列挙）。`foundation-observability-minimal-api.md`（approved）の判定: `telemetry`=`Counter`/`CounterReader` 共有 `Arc<AtomicUsize>` ペアのスケッチ・`rt_bridge`= `bridge<R: Copy + Send + 'static>(capacity) -> (RtRecorder, RecordSink)` の drop-on-full + overflow カウンタ・bounded drain。いずれも「構成候補として維持・要 PoC」— 実装着手は「最初の実需要 + #104 PoC 結果」。
- **#104 PoC（`ruviform_rt::observability_poc`・`#[doc(hidden)]`）の形状が本 Issue の参照形**: `counter() -> (Counter, CounterReader)`（`increment`=Relaxed `fetch_add`・`read`/`reader`）・`bridge()`（`record`=wait-free no-alloc bounded・満杯は drop+共有 overflow 計数・`drain`=1 呼出あたり capacity 上限）・`ObsRecord`（`reserved: [u64; 2]` 規約）。これは **実験 API であり昇格は trigger Issue の責務**（IR L179）— #121 が採用する場合の選択肢は (a) この形状を plugin-host 内に所有者ローカル実装（crate 新設なし）、(b) `ruviform-observability` を本 Issue で新設して第一 consumer になる、(c) `ruviform-rt` の PoC scaffolding を本番昇格（ADR 改訂要・重い）。
- verdict の制約に整合する形: underrun 計数は**レコード（イベント）ではなく事前確保カウンタ**が適する（pass-through はブロック周期で発火し得る高頻度イベント — 1 ブロック 1 レコードの `rt_bridge` 輸送は overkill・カウンタは Relaxed fetch_add 1 回で済む）。`rt_bridge` が適するのは「イベントの内容が欲しい」場合（例: 失敗したブロックの seq）だが、issue の要求は counts（aggregate）であり counter pair 形状に自然対応する。**#104 verdict 的には counter pair が第一候補・rt_bridge は第二候補（必要なら将来）**。
- `metrics` モジュール候補（計数・集約・時系列化・レポート面）は #101 で「将来消費者は既知・着手は #104 PoC 結果待ち」の保留 — underrun 計数の読み出し面はこの責務域に触れるが、crate が無い以上 **当面は host 側がカウンタ所有 + 読み出し面を提供**する形が最小。

#### 軸 E: 通知面の現行機構

- `HostNotification`（`composite.rs` L213-225）: `#[non_exhaustive]`・`Copy`・`InstanceFailed{handle, exit: ChildExit}` 単一 variant。doc に「L0 instances never emit notifications; the queue only carries L1 `Failed` transitions (exactly once while the instance is live)」。
- `ChildExit{code: Option<i32>, signal: Option<i32>}`（`composite.rs` L180-189、`l1::ChildExit` L147-167 からの写像）— `Copy`・`Default` derive。unix signal のみ `Some`。
- `PluginHost::try_recv_notification(&mut self) -> Option<HostNotification>`（`composite.rs` L1141-1172）: 全 slot 線形走査・audio L1 slot は `drain_endpoint_channels`（leg return + arm resend の `&mut` entry-point drain も兼務 — `mod.rs` L3015-3095）→ `notify_rx.try_recv` → 見つけた `ChildExit` に slot index+generation から handle を mint して `InstanceFailed` で返す。
- video slot は `VideoL1Instance::try_recv_notification`（`video.rs` L501-507）で同じ `Option<ChildExit>` を返す — audio/video で通知機構は共通。
- `Supervisor::poll`（#116・`supervisor.rs` L915-933）: host の `try_recv_notification` を sole drain point として消費し、watched handle は recovery 処理・unwatched は `SupervisionEvent::Unsupervised{notification}` で素通し — **通知型の拡張は supervisor の pass-through 経路に透過的だが、supervisor が分岐する場合の影響は個別評価要**（`supervisor.rs` L924-933 の `let HostNotification::InstanceFailed{..} = notification else { Unsupervised }` 分岐は `#[allow]` 付きで単一 variant 前提コメントあり — 新 variant は `Unsupervised` に落ちる既定動作になる）。

### 主要な構造的不整合（この Issue が解く対象）

1. **3 種の underrun（pass-through submit・dropped submit・`RESULT_FAILED`）に観測手段が全くない** — RT 側でも非 RT 側でも計数されず、RT 契約により「観測できるのは共有 atomic か pull 型 query のみ」という制約がある。
2. **失敗原因の喪失は最早地点（reader の `io::Error` drop）で発生し、下流（waiter `Err(Ipc)`・`ChildExit` 通知）は全て畳み込み済み** — payload を運ぶには converge 呼出し点に原因値を通す配線が要る（現行 `converge_child(shared, Option<ChildExit>)` の引数は exit status のみ）。
3. **stderr が診断の唯一の生の情報源でありながら管理されていない** — inherit は「診断をホストプロセスの stderr に混ぜる」形で、収集・保持・構造化の経路がない。piped に転じると bounded-buffer deadlock と 3 スレッド目という新コストが発生する。
4. **通知は「失敗の発生」の exactly-once イベントで、連続量（counts）とは器が違う** — underrun をイベント化すると unbounded mpsc と O(slots) drain を高頻度に打つ形になり器に合わない。counts は pull 型スナップショットが自然。
5. **leg は endpoint checkout で move する** — 計装値を leg フィールドに置くと checkout 中は host から観測不能になる（`EndpointUnavailable` と同じ「processing surface が移動した」状況）。共有 atomic（`rt_state` 先例）が置き場の自然解。
6. **stderr/drain/diagnostics の所有粒度が per-child なのに公開面は per-instance/per-handle** — Bundled 既定では帰属不能なストリームをどこに面として出すかが設計の核心。

## Alternatives and Comparison

### 軸 1: underrun メトリクスの公開面

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. `HostNotification` 拡張（イベント駆動） | 既存 drain 面の再利用・型の統一 | pass-through はブロック毎に発火し得る高頻度 — unbounded mpsc 詰まり・O(slots) 走査で drain が支配負荷になる。器が合わない | no |
| B. **counter snapshot API**（`PluginHost::diagnostics(handle)`/`DiagnosticsSnapshot` 系・pull 型） | RT 非ブロッキング両立（writer=Relaxed fetch_add・reader=非 RT query）・`!Send`/`&mut self` 制約に整合・#104 counter pair 形状の第一適用例・`arm_retire_overflows` 先例と同系 | 新しい公開 query 面の設計が要る・per-handle vs per-child 粒度の決定が要る | **yes** |
| C. `rt_bridge` レコード輸送（per-event record drain） | イベントの内容（seq・原因 tag）まで運べる・#104 で実測済み形状 | 高頻度イベントに対し構造・容量・drain 周期の契約が重い — issue の「counts」要求に対し overkill | maybe（payload 付きイベントが将来要るなら） |
| D. `ruviform-observability` 新設して consumer 化 | issue が「candidate first consumer」と明記・#101 trigger の名目充足 | crate 新設は別 trigger Issue 級の判断（DAG・公開 API・ADR 改訂を含む）・#121 の実装 scope を大きく超える | open — 最小形は B の host 内実装・crate 化は分離判断 |

### 軸 2: カウンタの所有場所（leg の checkout に耐える置き場）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. `RtLeg` フィールド | 計装点に最も近い | **checkout 中は host から観測不能**（`EndpointUnavailable` と同じ状況） — endpoint 使用時にメトリクスが止まる | no |
| B. **`Arc` 共有 counter set**（`InstShared`/facade/leg が `Arc<…AtomicU64>` を共有 — `rt_state` 先例） | checkout 中も host 側から読める・leg swap（`arm`/`arm_apply`）で失われない・#104 counter pair の形状と一致・writer 側は `&self` で fetch_add 可能 | フィールドより 1 段の参照が要る・カウンタ群をまとめた型の設計が要る | **yes** |
| C. `Shared`/`Inner`（child-scope）下に置く | Bundled で子単位集計が自然 | RT leg が mutex 配下の値を直接触れない（RT は `inner` lock を取らない — `mod.rs` L491-495） → 結局 lock-free な別経路が要る | no（RT writer の置き場としては） |
| D. composite（`PluginHost`）直下 | 読み出し面と同居 | RT writer（leg）から到達不能・`!Send` host には leg から逆参照する手段がない | no |

### 軸 3: stderr 配管の形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. **常時 `Stdio::piped` + per-child drain スレッド**（bounded ring + drop-with-counter） | 全子の全診断を捕捉（pre-`Hello` 失敗・panic・ASAN 含む）・運用者が予測不能なクラッシュ時こそ有効 | per-child 3 スレッド目（#120 再トリガ）・drain/buffer の所有と teardown が要る・heir grandchild が pipe を開き続ける問題 | **yes**（drain が必須なら一律が最も単純） |
| B. spec flag opt-in（`OutOfProcessSpec`/`SpawnSpecView` に `stderr: piped` 相当） | 既定のスレッド・メモリコストを避けられる・既存 inherit 動作を default で保持 | 「必要な時には有効化していない」— 診断が一番要る予測不能な失敗に使えない・spawn 経路に分岐が増える | maybe（cost 懸念が実測されるなら） |
| C. `Stdio::null` / inherit のまま | 変更なし | 本 Issue の scope を放棄 | no |
| D. drain を reader スレッドに兼務 | スレッド数を増やさない | reader は stdout で blocking read — 2 本の fd を同時に待てない（io-mux が要る = #120 の本丸。std のみでは poll 出来ない） | no |

**drained 内容の surface（A 採用時の副次論点）:**

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| bounded ring buffer（例: 末尾 N 行/N バイト・満杯は drop + 計数）を子スコープで保持・snapshot で読み出し | #104 drop-on-full+count 方針と一致・失敗直前の状況が残る・文字列の所有は非 RT 側のみ | buffer 契約（容量・行単位か bytes か）の設計が要る | **yes** |
| drain 行を通知イベントとして送出 | 生情報に近い | 通知は `Copy` — 文字列は載らない・per-line イベントは器に合わない | no |
| 呼出し側 sink/callback に流す | 柔軟 | ライブラリがログ・I/O を呼び出し側に流す設計は #101 logging 方針（ライブラリが暗黙の sink を持たない）と整合するが、v1 では buffer+snapshot が最小 | maybe（sink 経路は後続） |

### 軸 4: 失敗原因 payload の形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. `InstanceFailed` に payload フィールド追加（`cause: …`） | variant 数を増やさない・`#[non_exhaustive]` の下 additive | `Copy` 制約内の新 payload 型が要る・downstream の `InstanceFailed{handle, exit}` パターンは `..` で継続動作 | **yes** |
| B. 新 variant（例: `InstanceFailed` とは別の詳細通知） | 既存 variant を触らない | 「失敗」が 2 系統に分岐して observer が追いにくい・`Unsupervised` 既定動作に落ちる | no |
| C. `ChildExit` 拡張（cause を内側に） | payload の入口を 1 箇所に集約 | `ChildExit` は「プロセスがどう死んだか」の値 — 「protocol violation・timeout・spawn 失敗」は exit ではない概念で、型の責務が混ざる | no（`ChildExit` は exit fact に留める方が綺麗） |
| payload 型の選択: **`Copy` 原因 enum**（例: `Spawn`/`Handshake`/`Timeout`/`Transport`/`Protocol(WireError 相当)`/`Exit`/`Unknown` の分類） | `Copy` を維持できる・`io::Error`/`WireError` を現場で分類する配線のみ要る | 全喪失地点で分類を決めるマッピング層が要る | **yes** |
| `HostError::Ipc` への payload 追加 | — | `HostError` も `Copy`・wire tag 7・variant への field 追加は wire codec 変更を伴い #119 の bump 規則に触れる — **issue の scope は notification 側 payload であり HostError 変更は別論点** | scope 外寄り |

### 軸 5: stderr/原因の所有粒度と surface の帰属

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. **per-child（`L1Child`）所有 + per-instance snapshot 経由で公開** | 構造が事実に忠実（stderr は子の資源）・Bundled で重複しない・`child()` accessor 既存 | handle から辿る公開面が「instance の child」の診断になる — API は「この handle の属する子の診断」という語彙になる | **yes** |
| B. per-instance fan-out | 既存通知の fan-out 先例 | stderr 行に instance 帰属がないのに N 分岐は擬制的・buffer が N 個複製 | no |
| C. route/host レベル集約 | 単一の観測点 | どの子の出力か分からなくなる | no |

### 軸 6: 実装の配置（#101/#104 との整合）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. **plugin-host 内に所有者ローカルの counter set + snapshot 型**（counter pair 形状をドメイン語彙で） | 最小 scope・依存ゼロ追加・`rt → foundation` 問題に触れない・`observability_poc` の実測形状をそのまま参照 | 「foundation 型を使う」のではなく「同じ形状を所有する」ため後で crate 化する場合は移行が要る | **yes** |
| B. `ruviform-observability` クレート新設（本 Issue で第一 consumer） | issue の「candidate first consumer」文言・trigger 名目充足 | DAG 評価・公開 API 方針・`publish=false` 運用・ADR 改訂を含む別 Issue 級の作業 — #121 の 3 scope を大きく超える | open — lead 裁定 |
| C. `ruviform-rt::observability_poc` を本番昇格 | 実測済みの形 | `#[doc(hidden)]` 実験 API・rt に observability を持つことは `rt → foundation` 非既定問題に準ずる設計論点 | no（本 Issue で昇格権限なし） |

## Tentative Conclusions and Hypotheses

後続のユーザー判断により、Q1 は `ruviform-observability` を本 Issue で新設して L1 diagnostics を初消費者とし、Q4 は常時 piped + per-child drain とする。以下の探索時点の既定案や未確定扱いはこの判断に優先しない。stderr 導入と #120 再トリガ条件の関係は、実装後に記録する。

1. **（hypothesis）メトリクス面は pull 型 snapshot + 共有 atomic カウンタ** — `HostNotification` 拡張ではなく、`PluginHost`/`L1Instance` 経由の `diagnostics` 系 query（`&self`/`&mut self` 非 RT）+ leg と共有状態の間で `Arc<AtomicU64>` カウンタセット（`rt_state` 先例・#104 counter pair 形状）。writer 側計装は `Relaxed fetch_add` 1 回（実測 ~4 ns/op）で RT 契約に適合。
2. **（hypothesis）計数の意味づけは「RT leg の pass-through 分岐点」で分類する** — `pass_through`/`silence`/`slot busy`/`channel full`/`RESULT_FAILED` は全て `RtLeg::process` 内の分岐として数えるのが最も単一な計装点（`drain_to` で RESULT_FAILED を見ると「観測された回数」、driver/reader の合成点で見ると「発生した回数」 — 一致はするが分類の意味が変わるので設計で選択）。
3. **（hypothesis）stderr は常時 piped + per-child drain スレッド + bounded ring（drop+count）+ 子スコープの読み出し面** — 3 スレッド目の追加と #120 再トリガの記録を伴う。drain は detach・EOF 未到達を許容（heir 問題）。opt-in flag は代替案として残す。
4. **（hypothesis）失敗原因は `Copy` 原因 enum を `InstanceFailed` payload に追加** — `HostNotification` の `Copy` を維持、`ChildExit` は exit fact に留め、cause は `converge_*` 呼出し点で分類した値を通知に乗せる。`WireError` 種別は host 側の enum に写像（wire 変更なし）。
5. **（confirmed）`ruviform-observability` 新設は本 Issue の前提ではない** — counter pair 形状を host 内に所有する形が最小で、crate 新設は「最初の実需要が #121 で確定した」という trigger の記録として upstream ledger へ報告するに留まる。
6. **（confirmed）公開 API 影響の範囲** — `HostNotification` の variant payload・新 query 面・`OutOfProcessSpec`/`VideoSpawnSpec` 系の spec 字段（stderr 方針を選ぶ場合）は全て公開面であり、`publish = false` の未公開 0.1.0 であっても破壊的方向は ADR/明記が要る。

## Open Questions

- **Q1（lead 裁定）**: `ruviform-observability` クレートの新設を本 Issue の範囲に含めるか、host 内 owner-local 実装に留めて trigger 記録のみとするか — issue 本文の「candidate first consumer」と #101 trigger「最初の実需要」の解釈。
- **Q2**: underrun の計数単位・定義 — warmup horizon（underflow）を underrun に含めるか、`Failed` 無音を含めるか、per-handle・per-child・host 全体どの粒度で公開するか。
- **Q3**: `RESULT_FAILED` の原因内訳（stale gen・Destroying・dead child・reader rejection）を分類計数するか、単一 count に畳むか — 分類するなら生産側（非 RT）での tag 付けが要る。
- **Q4**: stderr を既定 piped にするか opt-in にするか — per-child +1 スレッドのコスト受容判断（#120 再トリガの発火記録を伴う）。
- **Q5**: drained stderr 内容の公開形 — bounded ring の snapshot 読み出し（行単位? bytes? 容量既定値?）・sink/callback 経路は後続か。
- **Q6**: `HostNotification` の `Copy` を維持するため payload は `Copy` 原因 enum に限定するか、より rich（`Copy` 放棄）にするか — 破壊的方向の採否。
- **Q7**: 原因 enum の語彙と境界 — `converge_child`/`converge_instance`/`request_timed`/spawn/handshake の各地点で付与する cause 分類の一覧をどう設計するか（design 作業）。`ChildExit{None,None}` の「violation と early-convergence の区別不能」を同時に解くか。
- **Q8**: `BundleFailed` 等の bundle 相関通知（#117 deferred）を本 Issue に取り込むか別 Issue のままか — 子スコープの原因 payload があれば相関の一部は payload で表現可能。
- **Q9**: supervisor への影響 — 新 payload/variant が `Unsupervised` pass-through で素通しするか、`InstanceFailed` の形が変わると `Supervisor` の `else` arm（`supervisor.rs` L924-933）に波及するか。
- **Q10**: drain スレッドの teardown 契約 — join bound・detach・heir grandchild で pipe が閉じない場合の drain thread の寿命。
- **Q11**: spawn/初期 handshake 失敗時は live instance と handle が存在せず、現行コードは通知せずに `Err` を返す。この失敗を観測可能にしつつ `InstanceFailed` の既存契約を保つ通知経路と、公開 API の互換性をどう設計するか。

## Promotion Candidates

- [x] spec-doc: `docs/specs/issue-121-diagnostics.md`（draft — 本 Issue の AC/R 要件を起票）
- [ ] design-doc needed: メトリクス面（query 形・カウンタ語彙・計装点）・stderr drain（所有・バッファ・スレッド契約）・原因 payload（enum 語彙・converge 配線）の 3 設計
- [ ] adr-doc needed (候補): `HostNotification`/`HostError` 系の payload 拡張方針・stderr 既定方針（常時 piped か opt-in か）・`ruviform-observability` trigger 記録方針 — design フェーズで要否を確定
- [ ] plan-doc / tasks / test-specs: design approval 後

## Sources and Relations

- Issue [#121](https://github.com/scarecrow173/ruviform/issues/121)（要求元 — #76 closeout 残余 3 件の集約・`ruviform-observability` first consumer 候補・#104 PoC verdict 整合要求）
- 上流確定物: [plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（L323 stderr deferred 行・L325 #120 再トリガに #121 stderr 明記・L255-261 `HostNotification`/`ChildExit` 形状）・[issue-76-plugin-l1-closeout-task-7-implementation.md](../impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md)（Follow-ups — underrun metrics/`WireError`→`Ipc` collapse/stderr piping の 3 残件の出典）
- observability 正本: [foundation-observability-minimal-api.md](../designs/foundation-observability-minimal-api.md)（telemetry/rt_bridge スケッチ・トリガ・依存 DAG）・[shared-foundations-and-modularization-policy.md](../designs/shared-foundations-and-modularization-policy.md)（#101 方針）・[rt-observability-poc.md](../impl/ir/rt-observability-poc.md)（#104 verdict — counter pair・bridge・verdict 制約）・`crates/ruviform-rt/src/observability_poc.rs`（実測形状）
- 関連設計: [issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md)（Bundled 既定・`BundleFailed` deferred L395）・[issue-116-supervisor.md](../designs/issue-116-supervisor.md)（`SupervisionEvent` payload 先例）・[issue-118-video-l1.md](../designs/issue-118-video-l1.md)（video spawn が `spawn_scanned` に合流する現状）・[issue-122-shared-platform-process-ipc-eval.md](../designs/issue-122-shared-platform-process-ipc-eval.md)（spawn/reap 抽出延期の現状 — stderr drain の新設は plugin-host 内に留まる方向を支持）・[issue-119-l1-wire-version-policy.md](../specs/issue-119-l1-wire-version-policy.md)（wire 変更を要しない payload 化の裏付け）
- 一次証拠コード: `crates/ruviform-plugin-host/src/l1/data.rs`（RtLeg・pass-through 分岐・`RESULT_FAILED`/`mark_failed`）・`crates/ruviform-plugin-host/src/l1/mod.rs`（spawn L1844-1861・reader L1193-1223・converge L734-870・`request_timed` L2166-2228・`L1Instance`/`InstShared`/`Notify`/`InstKind`）・`crates/ruviform-plugin-host/src/composite.rs`（`HostNotification` L213-225・`ChildExit` L180-189・`try_recv_notification` L1141-1172）・`crates/ruviform-plugin-host/src/endpoint.rs`（`L1Endpoint` L475-592）・`crates/ruviform-plugin-host/src/l1/video.rs`（video spawn L150-154・`try_recv_notification` L501-507）・`crates/ruviform-plugin-host/src/supervisor.rs`（`SupervisionEvent` L228-308・drain L915-933）・`crates/ruviform-plugin-core/src/wire.rs`（`WireError` L1011-1035・`read_message`/`write_message` L3019-3074・`encode_host_error` L1898-1957）・`crates/ruviform-plugin-core/src/boundary.rs`（`HostError` L120-183）・`crates/ruviform-plugin-export/src/lib.rs`（`eprintln` 診断サイト）
