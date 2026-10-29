---
id: "DESIGN-034O9RrQutLFWyaHJGOF3G"
type: "design"
status: "approved"
title: "Issue 121 L1 diagnostics"
created: "2026-10-29"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
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
    - "docs/discovery/issue-121-diagnostics.md"
    - "docs/adr/issue-121-l1-diagnostics-policy.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/impl/ir/rt-observability-poc.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-121-diagnostics.md"
    - "docs/discovery/issue-121-diagnostics.md"
    - "docs/adr/issue-121-l1-diagnostics-policy.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue 121 L1 diagnostics

## Context

Issue #121 は L1 audio の欠落メトリクス、子 stderr の収集、失敗原因の通知を対象とする。確定済み判断は `ruviform-observability` の新設と L1 による初消費、全子の常時 piped stderr、子単位の drain である。設計は approved — 承認前項目 1–5 はすべて解決済みであり、本書を実装・計画の根拠とする。

## Scope

### In Scope

- 共有 counter による L1 audio の RT 計数と非 RT snapshot。
- audio dedicated/Bundled と video dedicated の per-child stderr drain・bounded 保持・非 RT 読み出し。
- host 側で分類した失敗原因の通知と、生成前失敗の観測。

### Out of Scope

- wire、`HostError`、`ruviform-foundation`、RT logging backend、I/O 多重化、supervisor の原因別復旧。
- #104 の `ruviform-rt` 内 PoC を公開 API に昇格すること。`rt_bridge` は計数用途の必須条件ではない。

## Components and Boundaries

- **observability**: #104 の counter pair を参考に、共有累積 counter の writer と reader を所有する。生成・clone・最終 drop は非 RT、RT writer は `Relaxed` atomic 更新のみ。plugin-host が下向きに依存し、`ruviform-rt` からの逆依存は追加しない。
- **instance**: audio の計数は `InstShared`、host facade、`RtLeg` が共有する単位で所有する。endpoint checkout や arm 時の leg 交換でも同一 instance の値を保持し、handle の破棄・再利用時は新しい instance の値と混ぜない。
- **child diagnostics**: stderr pipe と drain は子スコープであり、Bundled の各 handle から得る値は同一子の snapshot である。保持領域は `L1Child` が生成される前に作る独立した診断レコードとし、host が成功・失敗の両経路から参照できるようにする。`L1Child` は生存中にその参照を保持するが、レコードの寿命を `L1Child` の生成成功や破棄に依存させない。
- **notification**: 失敗原因は host の語彙で保持し、`ChildExit` は実 exit status の事実に留める。`InstanceFailed` の既存の exactly-once と handle の世代を保つ。`HostNotification` は wire を通らない。

## Data and Control Flow

1. spawn 試行ごとに子スコープの診断レコード（in-flight では ID なしの active capture）を用意し、child spawn 時に stderr を pipe とする。現行の `SpawnRx` は stdout のみを運ぶため、stderr と診断レコードの handoff を追加し、初期 `Hello` を待つ前から drain を開始する。spawn、reader/drain のスレッド生成、`Hello`、`Scan`、create の失敗経路でも caller がレコードを引き継ぎ、成功後の child 破棄や最終 handle の破棄後も限定期間読める。保持領域が満杯でも consumer を待たずに読み捨て、損失を数える。diagnostics ID は失敗確定時の retention 登録で採番する（「生成前失敗の diagnostics ID」節 — 試行前採番は欠番で watermark を破るため不採用）。
2. audio `RtLeg::process` は submit を捨てた場合、結果不足による出力 fallback、観測した失敗結果をそれぞれ計数する（「audio counter の暫定定義」節 — spec 照合済み）。
3. reader、driver、timeout、生成・handshake の各境界でエラーの原因を失う前に分類する。子単位の収束は同居する minted instance に、帰属可能な protocol 違反は当該 instance にだけ伝える。原因は shared state の同一ロック内で最初に記録された分類を固定し、後続の EOF や reap による上書きを禁止する。timeout など host 自身の判断は kill の前に記録する。先に既に terminal となった事象は上書きせず、競合した観測の選択はロックの線形化順に依存する。原因と後から回収した exit status は別の情報として扱う。
4. 現行 `spawn_scanned` と Creating phase は public handle を mint する前の失敗を通知しない。pre-mint 失敗は spawn/create の `Result` 経由で直接 caller に報告される。ただし **すべての失敗が必ず観測される保証はない** — caller が `Result` を破棄・上位 task が cancel・RPC 切断等で `Err` が消費されないケースを明示的に除外し、host retention はあくまで追加の query 面（guaranteed delivery ではない）。`HostNotification` に handle 無し variant は追加しない（未存在 handle の捏造になる）。順序は「spawn/create 試行 → 失敗確定 → retention lock 内で diagnostics ID 発行 → 失敗レコード登録 → `Err(cause, diagnostics_id)` 返却」— `Err` を受け取った caller は ID で診断を引けるが、retention eviction 済みなら「evicted / not-found」を区別できる結果を返す（query 成功は保証しない）。失敗レコードは `phase` と `cause` を区別し stderr の有無は phase 依存（rollback 節参照）。pre-mint 操作は supervisor 監視対象外と明示し、watched 通知は引き続き `InstanceFailed` のみ。spec AC4 は通知契約の実質変更として受入条件自体を再定義する（同期結果と非同期通知では観測主体・タイミングが異なるため文言修正ではない）。

診断レコードは host 側の bounded retention から diagnostics ID で検索できる。active child の参照と、終了または生成失敗後の retention を分離し、上限到達時の eviction と消失数を caller に見えるようにする。レコード作成・drain 開始に失敗した場合は child を生かしたまま放置せず、既存の失敗戻り値・kill/reap 契約に従う。容量は確定済み（64 KiB ring・retention 8 件）。ID の発行・識別・evicted/not-found 区別は下記で確定。

### 生成前失敗の diagnostics ID

- **採番タイミング**: 失敗レコードを retention に登録するとき、retention lock 内で連続した `diagnostics_id: u64` を発行する（欠番なし・再利用なし）。**spawn/create 試行前の相関 ID 発行は行わない** — 試行前採番では成功試行・並行 spawn の登録順逆転で欠番が生じ、「`id ≤ watermark → evicted`」の判定が未登録 ID を evicted と誤認するため。登録は lock 内で直列化されるため、並行 spawn でも欠番は発生しない。
- **query の意味論**: `id ≤ evicted_watermark` → `evicted`（FIFO eviction は登録順 = ID 順のため watermark が成立）。`evicted_watermark < id ≤ issued_max` で retention 不在は内部不整合。`id > issued_max` → `not-found`（未発行 ID）。`evicted_watermark` は最後に evict されたレコードの ID、`issued_max` は発行済み最大 ID — いずれも retention lock 内で更新・読み出し。
- **登録対象**: retention への登録は失敗レコードのみ — 成功した spawn の in-flight レコードは active capture として残り、子が finalize した時点で失敗なら retention 登録（この時点で ID 採番）、成功・正常終了なら破棄される。minted 失敗の retention レコードにも同一の ID 空間で採番され、handle ベースの検索 index を別途持つ（通知に ID field は追加しない — minted は handle が相関を担い、2 系統の相関を防ぐ）。
- **registry の所有・受け渡し**: bounded retention + ID 空間は **`DiagRegistry` として `PluginHost` が `Arc` で保持**し、spawn 系関数（`spawn_scanned`/`create_instance`/`create_video_instance`/`instantiate_bundled` 系）へ `diag: &DiagRegistry` 引数で渡す。`Shared` は spawn 内で生成・`Err` で drop されるため、**失敗確定時の登録は `Err` を返す関数自身が行う** — record の `Arc` が registry に移り `Shared` drop 後も生存する。テスト経路（`L1Instance::spawn` 等）は呼出側が `DiagRegistry::new()` の fresh registry を渡す（テスト間で retention が漏れない）。global/static registry は採用しない — per-host の bounded retention と ID 空間の意味論を維持する。

### 原因の分類と収束

`FailureCause`（新規 `pub` `Copy` enum・`#[non_exhaustive]`）— 実装位置との写像:

| variant | 分類する地点 |
| --- | --- |
| `Spawn` | OS spawn 自体の失敗（pre-mint のみ） |
| `Handshake` | handshake 手順の定義済み拒否 — `Hello` version 不一致、decode は可能だが `Hello` でない先頭 frame、`Scan`/descriptor mismatch、`CreateInstance` の Err reply |
| `Timeout` | host 側の期限判断 — `Hello` 待ち・`CreateInstance` reply 待ち・command waiter timeout・driver watchdog kill（host が kill を出す **前** に記録。`command_timeout` は各 waiter に個別適用される現行定数どおりで、handshake 全体の合計上限ではない — phase 不問） |
| `Protocol` | decode 不能 frame（phase 不問 — 先頭 frame の garbage もここ）・`Hello` 受理後の stream 違反（2nd `Hello`・unknown kind・parent→child kind・scope/`instance` 範囲外・waiter-less `Reply` 異常・帰属可能な per-instance 違反） |
| `Transport` | stdout EOF・stdout read error・stdin write 失敗 |
| `ExitObserved` | `try_wait`/reap で子の終了を観測（exit status は別の `ChildExit`） |
| `Internal` | host 側 machinery 失敗（channel 切断・unreachable state・`try_wait`/`wait` syscall error。最初の失敗観測が reap error の場合もここ） |

これらは **観測した最初の分類**であり、真の根本原因を断定しない。

**境界の pin**（エラーの意味で分類 — 実装位置依存にしない）:

| 事象 | `FailureCause` |
| --- | --- |
| `Hello` の version 不一致 | `Handshake` |
| decode は可能だが `Hello` でない先頭 frame | `Handshake` |
| `Scan`/descriptor mismatch・`CreateInstance` の Err reply | `Handshake` |
| `Hello` 待ち timeout | `Timeout` |
| `CreateInstance` reply 待ち timeout | `Timeout` |
| command waiter timeout・driver watchdog kill | `Timeout` |
| decode 不能 frame（phase 不問） | `Protocol` |
| `Hello` 受理後の stream 違反（2nd `Hello`・kind/scope/帰属違反） | `Protocol` |
| handshake 中を含む任意 phase の stdout EOF・read/write error | `Transport` |
| `try_wait`/reap で子終了を観測 | `ExitObserved` |
| `try_wait`/`wait` error・channel 切断・unreachable state | `Internal` |

- `Internal` は host 側 machinery 失敗 — `try_wait`/`wait` error・channel 切断・unreachable state。**reap error が最初の失敗観測でも `Internal` に分類されるため、すべての first-failure は `FailureCause` を持つ**。
- poisoned lock: `Shared::lock()`/`lock_mutex` は既に poison-tolerant（`PoisonError::into_inner` で guard を必ず取得）— **lock 取得不能による記録欠落は構造上発生しない**。poison そのものは分類対象とせず（panic は reader/driver スレッドの死亡として EOF/timeout 経由で観測される）、`Internal` は lock 以外の machinery 失敗を担う。

**収束規則**:

- 原因は `Inner` の同一ロック内で **first-observed を固定・上書き禁止**。EOF・timeout・reap の競合はロックの線形化順に依存することを明示する。
- host が kill を出す判断（timeout）は **kill 発行前に原因を記録** — 後続 EOF で `Transport` に置き換わらない。
- reap は `ChildExit`（status の事実）のみ補完し、先行 cause を変更しない。reap 失敗・`wait().ok()` 失敗 → `ChildExit{None,None}` で cause は残る。
- 帰属可能な違反は当該 instance に `Protocol`、子・同居 instance は失敗扱いにしない。
- minted instance ごとに cause と通知状態 `Idle`/`Pending`/`Sent` を保持。通知は status 取得で exactly once、status が取得できなければ `{None,None}` + fixed cause で送る。instance が待機中に削除される場合の既存 unknown flush を維持 — **保留上限は既存契約のまま（子 reap または instance 削除まで。新しい保留上限は設けない）**。

### 公開 API 面

- **`HostNotification::InstanceFailed` に `cause: FailureCause` field を追加 + variant レベル `#[non_exhaustive]` を付与** — `Copy` を維持（rich payload は `Copy` 破壊 + diagnostics record の ID で代替のため不採用）。enum 自体の `#[non_exhaustive]` は struct-variant field pattern を保護しないため、**downstream の `InstanceFailed { handle, exit }` 完全 pattern は source break** — `publish = false`・consumer は workspace 内に限定される前提で受容し ADR に記録。variant への `#[non_exhaustive]` は repo 慣例（`SupervisionEvent` が各 struct variant に付与 — `supervisor.rs:217-297`）に合致し、将来の field 追加を非破壊化する。downstream 構築は不能になるが、全構築サイトは in-crate（`composite.rs:2983`・`supervisor.rs:1328`）に限定されると確認済み — 実害なし。
- **in-workspace 修正サイト**: `composite.rs`（生成 1165・合成 2983・destructure 2645）、`supervisor.rs`（合成 1328 — `:933` の `{handle, ..}` は無影響）、tests（vst3/clap `l1_real_adapter_e2e`・`l1_video_e2e`・`l1_supervisor_e2e`・`l1_proxy_e2e` の `{handle, exit}` 完全 pattern・構築）— いずれも `..` 付与または `cause` 供給で対応可能。
- **pre-mint の Err payload**: `HostError` に新 variant `SpawnFailed { cause: FailureCause, diagnostics_id: u64 }`（`Copy` payload、`diagnostics_id` は retention 登録時採番 — 「生成前失敗の diagnostics ID」節）。**`SpawnFailed` の境界**: `Err(HostError::InstanceLimit)`（unflattened 契約）・spec-validation `Err(Ipc)` などの **定義済み拒否は variant を保持**し `SpawnFailed` で包まない — 定義済み応答は子の失敗イベントではないためレコードも登録しない（`diagnostics_id` 無し）。`SpawnFailed` は真の失敗（spawn/handshake/protocol/transport/timeout/exit）のみに使う。live 子への pooled `create_instance` `Reply::Err` は `SpawnFailed{cause: Handshake}` + 試行レコード（その時点の子 stderr の frozen snapshot copy — live 子の active レコードは mutate を継続・分離）。`HostError` は `#[non_exhaustive]` のため variant 追加は downstream match を壊さないが、in-crate の exhaustive match は修正要。**`HostError` は wire codec の対象外** — variant 追加は wire tag に影響しないため AC5 と整合（`WireError` tag・codec・`WIRE_VERSION` の不変と Rust 側 `HostError` の source compatibility は別の保証。source break は ADR で受容済みなので追加の互換性対策は設けない）。
- **supervisor**: watched `InstanceFailed` は既存どおり消費し `cause` を payload 転送しない（spec R-B8 — cause 分岐は範囲外）。`cause` が必要な consumer は handle → 診断レコード query で引く（retention レコードは minted handle でも検索可能）。新 variant を追加しないため pass-through 影響なし。
- **diagnostics ID**: 失敗レコードの retention 登録時に host が連続採番（「生成前失敗の diagnostics ID」節 — 試行前採番なし）。minted 失敗は handle が相関を担う — 通知に ID field は追加しない（2 系統の相関を防ぐ）。

### audio counter の定義

**所有・寿命**: `ruviform_observability::{Counter, CounterReader}` pair 型を使用（spec R-F1「最初の実需要者」— 裸の `Arc<AtomicU64>` 直置きは path dep を死蔵させるため不可。内部表現は共有 `Arc<AtomicU64>`・Relaxed）。4 pair は **`InstKind::Audio` arm が所有** — `InstShared` 本体の field には置かない（`InstShared` は `InstKind::Video` エントリも保持するため、素の field 配置は video エントリに audio counter を持たせる = spec R-B6 の非混入 pin に型レベルで反する）。writer `Counter` の clone を `RtLeg`/`push_result` 経路へ、reader `CounterReader` を query 面へ — in-place `RtLeg`・detached endpoint leg・host query facade が同一 counter を共有し、leg swap・endpoint checkout・`arm`/`arm_apply` で値は失われない。再 mint された handle は別 registry entry で新しい counter（旧値と混ぜない）。destroy 後は entry 削除とともに query が not-found になる（per-instance 計数の死後保持はしない — 死後診断は stderr retention の責務）。

| counter | 増分する点（実装位置） | 増分しない例 |
| --- | --- | --- |
| `dropped_submits` | `RtLeg::process` で slot busy（`last_pushed` 未クリア）により submit を送らなかったとき、または submit channel `try_push` 失敗時 — block ごとに **最大 1 回**（slot busy は `try_push` を呼ばないため同一 block で両原因は併発しない — 二重計数しない構造的不変条件） | 正常 submit。channel-full / slot-busy の原因区別は現行 1 counter に集約（分割は後続検討可 — spec R-B1 は単一 metric を要求） |
| `passthrough_submits` | `RtLeg::process` / `L1Endpoint::process` で代替出力（`last_output` または無音）が**実際に成立した** block ごとに 1 回 — 計装点は `pass_through` への進入ではなく代替出力の成立地点。**warmup（`block < lookahead` の underflow 分岐）も含める** — spec R-B1「実際に出力された回数」に忠実な事実計数。warmup は instance 生涯で最初の `lookahead` 回のみの有界フェーズで、起動時の増分は測定誤差ではなく仕様どおりの値（warmup 除外の分析が必要なら別指標で対応） | `Failed` mirror の silence（pass_through ではない）、Lifecycle/Ipc の defined Err、入力検証エラー |
| `results_failed_observed` | `drain_to` で result channel から `RESULT_FAILED` ビット付き要素を初めて `try_pop` したとき 1 回。stale 判定・held 行きの場合もその初回 pop で計数 | `held` からの再取り出し（二重計数しない）、producer が enqueue できなかった結果、drain 前に子が終了した未送達結果。**発生した全 `RESULT_FAILED` ではなく RT が観測した数** |
| `results_dropped` | reader 側 `InstShared::push_result` で result channel `try_push` 失敗により result 要素を破棄したとき 1 回（非 RT スレッド）。**enqueue 失敗による消失件数のみ** — 観測前の消失全体を測る指標ではない | enqueue 成功後の消失（generation 切替・queue 破棄・RT 未観測） |

**相関の明示**: post-warmup の pass-through は `results_failed_observed` の strict superset ではない（観測した failed result が held/stale 行きの場合、対応 block の pass-through とは別カウント）。`results_dropped` は enqueue 失敗で破棄された要素のみを数え、**観測前の消失全体を測るものではない** — enqueue 成功後も generation 切替や queue 破棄で RT 観測前に失われ得るため、`results_failed_observed` との差から全消失量を導出することはできない。

**overflow・読み出し**: `u64`（~5.8×10⁸ blocks/秒 × 1 年でも 2⁶⁴ の 0.1% 未満 — wrap は構成上到達不能として文書化、saturating 措置なし）。複数 counter の読み出しは同一瞬間を保証しない Relaxed 逐次読みで、整合した transaction ではないことを公開契約に明記する。

## Non-Functional Constraints

- 計数を読む側は non-RT。複数 atomic の snapshot は同時点の整合した transaction ではないことを明示する。
- bounded buffer の満杯または読み手不在で drain が待機しない。OS の scheduling や pipe の一時的な backpressure が一切起きない保証はしない。
- 子終了後に heir が pipe を保持しても host teardown は無期限に待たない。単純 detach は保持された pipe ごとにスレッドが残り得るため、反復失敗時の資源上限を明示して解決する必要がある。
- stderr は秘密や任意のバイト列を含み得る。暗黙に別 sink へ出力せず、公開 API の byte/文字列変換、保持上限、破棄時点を設計で明示する。

## Risks and Trade-offs

- **生成前失敗**: `InstanceFailed` は minted handle が前提。単なる payload 追加では不足する。host-owned の失敗記録を候補とし、生成失敗時の stderr と通知の寿命を設計する。
- **supervisor**: 既存 `Supervisor::poll` は監視対象の `InstanceFailed` を消費し、raw notification を返さない。非監視対象と新しい通知 variant は `Unsupervised` で通す。監視対象の原因を後から読む要件が必要か、既存契約として明示的に対象外にするかは spec レビューで確認する。
- **互換性**: `HostNotification::InstanceFailed` への field 追加は既存の完全 field pattern を破壊する。`Copy` を保つか、別 variant／別 query とするかの trade-off をレビューする。
- **原因競合**: EOF、timeout、exit の観測順は競合する。原因を上書きする規則と、通知が exit status 待ちで永久に保留されない条件を定める。
- **OS 差**: drain の停止機構と thread ownership は Windows、Linux、macOS の別々の振る舞いを確認する。単に stdout と同様に detach するだけでは反復失敗時の資源有界性を証明できない。定期起床 drain（後述）を採用するが、終了時間の保証は OS 別であり「完全に有界」は主張しない。
- **結果の消失**: 現行 `InstShared::push_result` は結果 channel の `try_push` 失敗を捨てる。`RtLeg::drain_to` で観測した `RESULT_FAILED` を数える方式では、push 失敗や `Failed` への移行で読まれない結果は数えられない。発生回数か RT 側の観測回数かを仕様として区別する必要がある。

### pipe 終了に関する調査

- [Linux pipe(7)](https://man7.org/linux/man-pages/man7/pipe.7.html): writer が残る空 pipe の blocking read は待機し、`O_NONBLOCK` では `EAGAIN` になる。これは macOS や Windows の動作の証明にはならない。
- [Windows anonymous pipe operations](https://learn.microsoft.com/en-us/windows/win32/ipc/anonymous-pipe-operations): anonymous pipe の read は書き込み、全 write handle の close、または error を待ち、anonymous pipe は overlapped read/write をサポートしない。
- [Windows CancelSynchronousIo](https://learn.microsoft.com/en-us/windows/win32/fileio/cancelsynchronousio-func): cancellation は要求を mark するが、完了を待たず正常完了や別のエラーもあり得る。[取消しのガイド](https://learn.microsoft.com/en-us/windows/win32/fileio/canceling-pending-i-o-operations) は driver が取消しに対応する保証もないと明記している。取消し API を呼ぶだけでは終了期限を証明できない。
- [Windows PeekNamedPipe](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-peeknamedpipe) は anonymous pipe を対象にできるが、使用中の handle の共有・同期条件に注意が必要。**公式仕様は PeekNamedPipe が常に非ブロッキングで返ることを保証しない** — 実用上は即座にバッファ状態を返す回避策として有効だが、同期 I/O に起因する残余のブロック可能性を残す。
- Go `os/exec.Cmd` の [`WaitDelay`](https://pkg.go.dev/os/exec#Cmd) は「子終了後も pipe が閉じられない（stdio 継承の子孫が fd を保持）」を契約上明示的に扱い、期限経過で wait と I/O 完了待ちを打ち切る先行実装として参照できる。

### 採用案 A: 定期起床 drain + 期限後の自発終了

**POSIX**: `poll(fd, POLLIN|POLLHUP, timeout)` で drain が一定間隔で起床し、子の終了 + grace 経過を見て自発終了する（参照: [pipe(7)](https://man7.org/linux/man-pages/man7/pipe.7.html)、[poll(2)](https://man7.org/linux/man-pages/man2/poll.2.html)）。`O_NONBLOCK` は必須でない — poll が timeout で起きる時点で停止判定を入れられる。**限定条件下（poll が timeout を遵守し、準備状態が通知された後の read がブロックしない前提）で終了時間を制御できる**。**実装依存**: `poll`/`PeekNamedPipe` は std に存在しない — unix `libc`（`poll`）・windows `windows-sys`（`PeekNamedPipe` — `Win32_System_Pipes`,`Win32_Foundation`・`Win32::System::Pipes` 所在）の platform-gated dep を plugin-host に追加し、FFI は `l1::diag` モジュール内の `#[expect(unsafe_code)]` + SAFETY コメントで局所化する（workspace lint は `unsafe_code` を実質 deny）。

**deadline の pin**: `STDERR_DRAIN_GRACE: Duration = Duration::from_millis(500)`（構築時定数）。起算点は **子プロセス終了の観測時点**（reap/`try_wait`・converge による exit 検出）。stderr EOF 到達なら即終了で deadline を待たない。**出力が継続していても deadline は延長しない**（activity-based 延長は heir の無期限保持を再導入する）。500 ms の根拠: pipe buffer 全量（~64 KiB）の drain は read 自体ほぼ即時で残りは OS scheduling jitter — `DRIVER_JOIN_MARGIN`（1 s）・`command_timeout`（10 s）より十分小さく、heir fixture（~15 s）より大幅に短いため teardown bound を維持する。POSIX でも OS scheduling による検出遅延を許容し、Windows は合意済みの同期 I/O 残余リスクを持つ — どちらも deadline は「目標」であって厳密な硬保証ではない。deadline 検出の起床周期は drain の poll timeout / sleep 間隔（構築時定数・5–10 ms 程度を想定）。

**Windows**: `PeekNamedPipe` で利用可能バイトを確認し、`avail > 0` の場合のみ `ReadFile`、空なら短い sleep で停止フラグを再確認する。**実用的な回避策であり厳密な終了保証ではない** — PeekNamedPipe 自体がブロックし得る同期 I/O 経路の残余リスクを明示的に受容する（例外的な代わりに CancelSynchronousIo のような「保証しない」機構より、より確実な fallback である）。

**終了後の fd と heir**: drain thread が自発終了するとき、その read fd は host 側から見て閉じられるか retain されるかで heir への副作用が変わる:

- **fd close（採用）**: read 端を閉じると heir の**後続書き込みは EPIPE/SIGPIPE 系の error になる**（POSIX: write on pipe with no readers → SIGPIPE; Windows: broken pipe error）。これは detach（read 端を持つ thread が残り続ける）と意味論が異なり、heir の write を拒否する動作変更として**明示的に受容・文書化**する。heir 自身は殺さないが、その stderr 出力は error になる。
- 代替（fd retain）: read 端を診断レコードに委譲して残すと heir は pipe 満杯まで書けるが、fd の無期限累積になり、終了の有界性を再び損なう。採用しない。

**truncation の語彙**: 未来の出力のバイト数は正確に計数できない。事実は 2 つに分離する:

- `dropped_bytes: u64`（counter）— **実際に read してから** buffer 満杯で捨てたバイト数のみを計数する。未読のまま放置された出力は計数しない。
- `drain_deadline_exceeded: bool`（flag）— EOF 未到達のまま drain deadline に達し、drain を自発終了させた事実。
- `unread_bytes_observed: bool`（flag）— deadline 時点またはその後の最終確認で **未読データを実際に観測した** 場合のみ `true`。`false` は「欠損なし」ではなく「未読を観測していない」でしかない（観測手段は best-effort — Windows の `PeekNamedPipe` はブロックし得る、POSIX 標準には fd の未読バイト数を返す API がなく `poll` の POLLIN で「何かある」ことの確認に留まる。FIONREAD は非 POSIX のため採用しない）。

この方式により drain thread の資源は heir に関わらず自発終了する（POSIX は制御可能な範囲で保証、Windows は残余リスクを受容）。子終了時点までの出力は保持され、その後は書き込み側が error を受ける — 診断価値の高い「死亡前後」の出力は失われない。stdout の detach 機構（heir では thread 自体が残る既存契約）より厳しい形を stderr では新たに取る;stdout の既存 detach は変更しない。

### stderr 保持の暫定契約（承認前項目 2）

- **読み取り義務と保持の分離**: pipe を読み続けることは deadlock 防止の義務であり、**buffer 満杯で読み取りを止める設計は採用しない**（子が write でブロックし得るため）。保持は読み取り後段で任意に落とす。drain は buffer 満杯の有無に関わらず終了条件（EOF / deadline）まで読み続ける。
- **保持形式**: 子スコープ診断レコードごとに 1 つの bounded byte ring（容量は spawn 時固定・既定案 64 KiB）。writer は drain thread 1 つ、reader は非 RT snapshot — 双方とも非 RT なので `Mutex` 保護の単純リングで十分（lock-free・wait-free は不要）。追い出しは最古から（**tail 優先** — クラッシュ直前の診断が最も価値を持つ。pre-Hello 失敗時は出力自体が小さく全量入る前提）。
- **drop 計数**: `dropped_bytes`（u64）は「**収集したが ring に保持できなかったバイト数**」— 全出力の欠損量ではない。増加するのは (a) 新入力で古いバイトを追い出した分、(b) 1 回の read が ring 容量を超えて保持できなかった分。増加しないのは retention eviction・deadline 後の未読出力・ring に収まった分。#104 verdict の drop-on-full + count と同系。`total_read_bytes`（u64）を併記し、capture 単位の不変条件 `total_read_bytes = retained_bytes + dropped_bytes` を検証可能にする（capture 開始前・deadline 後の未観測出力はいずれの値にも含まない）。
- **snapshot**: 非 RT query は `{ bytes: Vec<u8>, total_read_bytes: u64, dropped_bytes: u64, eof: bool, drain_deadline_exceeded: bool, unread_bytes_observed: bool }` 相当を返す。`bytes` は保持分のコピー（caller スレッドで alloc — 非 RT なので許容）。**`bytes`・`total_read_bytes`・`dropped_bytes`・flags は同一時点の状態として取得する** — snapshot はレコードの `Mutex` 内で組み立て、取得中に drain が進行しても異時点の counter と buffer を組み合わせない。不変条件 `total_read_bytes = bytes.len() + dropped_bytes` が各 capture で成立する。stderr は任意バイト列なので **String 変換は行わず bytes のまま返す**。
- **retention**: 生存中の子のレコードはレコード側で参照可能。子終了（finalized: EOF or deadline-stop）または生成失敗のレコードは host の bounded retention へ移す（既定案 直近 8 件・上限超過は最古を evict + `evicted_records: u64` 計数）。**512 KiB は「完了した失敗 retention が保持する stderr payload」の合計上限であり、プロセス全体のメモリ上限ではない** — 最大 8 レコード × 各 64 KiB。active な capture（同時進行 spawn 中の ring — 例: 100 件同時なら ring だけで ~6.25 MiB）・snapshot コピー・Err payload・読み取り一時バッファ・レコードメタデータは別計上であることを明記する。
- **既定値の性質**: 64 KiB（OS pipe 容量程度・panic backtrace / dlopen 出力を概ね収容）と 8 件は暫定値 — 実用レビューで調整可。これらは spec 生成値ではなく構築時定数とする。

### 生成前失敗の rollback（項目 2・4 共通の必須確定事項）

OS spawn 成功・instance mint 前の失敗では、**実在する子プロセスを残したまま Err を返すことはできない**。`std::process::Child` の `Drop` は `wait()` も `kill()` も行わず（[std 仕様](https://doc.rust-lang.org/std/process/struct.Child.html)）、Tokio でも `kill_on_drop` は既定 `false`（[docs.rs](https://docs.rs/tokio/latest/tokio/process/struct.Child.html)）— 単純な drop は cleanup を保証しない。

採用ポリシーは (a)「失敗を確定させて終了・回収」:

1. 失敗を確定した時点で既存の kill + reap 契約に従い **direct child** を終了・回収する（部分的に起動した子を残さない。意図的に存続させる heir は終了対象にしない）。所有権を別管理主体へ移す案 (b) は mint 前の回収責任が不明瞭になるため採用しない。
2. 既存 teardown の実機構（`l1/mod.rs`）との整合: `Child` は `spawn_child` を driver thread 内で生成し、所有権は thread 境界を越えない。caller 側の失敗経路は `kill_child(&shared)` + `driver_tx` disconnect で所有側（driver）へ終了を委譲し、`driver_join` で待つ — **caller のスタックに cleanup 責任を置かない構造のため、Err 経路でも unwind でも責任が失われない**。reap は deadline 経過で `kill()` → blocking `wait()`。「kill は全対象 OS で forceful」が bounded の根拠で、`wait().ok()` 失敗時は `ChildExit{None,None}`（status 未知 = cleanup 不完了の明示記録）となる。**bounded rollback は期限内に OS 終了・reap が完了したことを意味しない** — 期限超過後も `Child` 所有権は `Shared` が保持し（独立 reaper への移譲は設けない）、未回収は status-unknown として記録される。`kill` 自体の error を握りつぶす既存経路の residual（kill 失敗 + 子生存時に `wait()` がブロックし得る理論上の経路）は現行契約の残余リスクとして引き継ぐ。
3. drain cleanup — stderr drain は Option A の自発終了（stop flag + 期限）に従い、rollback は **無期限の thread join を行わない**（既存 reader は JoinHandle を drop して detach する規律と同じ）。
4. cleanup 失敗は **元の失敗原因を上書きしない** — 失敗レコードの主 `cause` は元の分類のまま、cleanup 失敗は副次診断 `cleanup_error` として記録する。
5. 失敗レコードは `phase` を区別する — pre-spawn 検証失敗（プロセス無し・stderr 空）/ OS spawn 失敗（通常プロセス無し・stderr 空）/ spawn 成功・drain 初期化失敗（プロセス実在・stderr 部分的または取得不能）/ spawn 成功・mint・handshake 失敗（プロセス実在・stderr 取得済みの場合あり）。「process は作られたが instance は生成されなかった」を handle なしに表現できる。
6. 新 stderr 機構も同じ所有構造を維持する — 診断レコード・stderr pipe・drain は spawn-scoped owner（`Shared` 系）が保持し、`spawn_child` 成功から drain 開始失敗までの間に cancellable な処理を挟まない。

## Deferred Design Concerns

- structured logging/export | 本件の観測は pull 型で十分 | 外部 sink の実需要 | アプリケーション側の連携が必要。
- I/O multiplexing | #120 の責務 | stderr 配管導入時に再評価を記録 | per-child thread cost が残る。

## 承認前項目の解決記録

1. counter の暫定表を spec と照合し、overflow・handle 寿命を pin — **解決（設計承認）**: 4 counter（`dropped_submits`/`passthrough_submits`/`results_failed_observed`/`results_dropped`）、warmup 込み・`results_dropped` 追加を承認。`results_dropped` は enqueue 失敗件数のみで差分から全消失量を導出しない旨を明記済み。受入ケース表を Verification Notes に記録。
2. stderr の容量・保持順・drop 単位、snapshot の同期と破棄 — **解決（設計承認）**: 分離・counter 意味論・retention 適用範囲は承認済み。rollback 4 条件は既存機構で検証済み（「生成前失敗の rollback」節 — driver thread が `Child` を所有し、reap 失敗は `ChildExit{None,None}` で未回収を明示）。
3. pipe を保持する heir が無期限に存続する場合の drain 終了方式 — **解決**: 採用案 A（定期起床 + 期限後自発終了）。ただし終了保証は OS 別: POSIX は限定条件下で制御可能、Windows は PeekNamedPipe の同期 I/O 残余リスクを受容。fd close による heir 後続書き込みの error 化も明示受容。deadline は `STDERR_DRAIN_GRACE` = 500 ms・起算は子終了観測・延長禁止で確定。
4. 生成前失敗の通知・stderr 所有者、retention、ID 相関 — **解決（設計承認）**: 観測保証の限定・登録順序・`phase`/`cause` 区別・AC4 意味論変更を反映済み。diagnostics ID は retention 登録時の連続採番（試行前採番は欠番で watermark が破れるため不採用）+ `evicted_watermark`/`issued_max` による found/evicted/not-found 区別で確定。rollback の cancellation/未回収処理は項目2と同じ検証で確認済み。
5. 原因分類と競合時の優先規則、reap 失敗、notification exactly-once、公開 API の source break — **解決（設計承認・互換検証は実装時）**: `FailureCause` 7 variant、first-observed 固定、Handshake/Protocol/Transport の境界を phase・観測種別で pin、reap error→`Internal` で全 first-failure が cause を持つ、poisoned lock は poison-tolerant で記録欠落なし。`InstanceFailed` に `cause` + variant レベル `#[non_exhaustive]`、`HostError` に pre-mint 用 variant。crate 境界 compile・wire codec 無変更の検証を Verification Notes に記録。

## Verification Notes

項目1 counter の最低限の受入ケース（設計レビューで確定）:

| ケース | 期待値 |
| --- | --- |
| warmup pass-through を 3 回実行 | `passthrough_submits += 3` |
| slot busy で 1 block 破棄 | `dropped_submits += 1` |
| 同一 block で複数の失敗判定 | `dropped_submits` は最大 +1 |
| `RESULT_FAILED` を初回 pop | `results_failed_observed += 1` |
| held の同じ result を再利用 | 追加増分なし |
| reader の enqueue 失敗 | `results_dropped += 1` |
| enqueue 成功・RT 未観測 | `results_dropped` 増えず |
| leg swap 後 | counter 累積値を維持 |
| re-mint 後 | counter は 0 から開始 |

- [ ] 上記の承認前項目をコードと OS 契約に照らして解消し、ADR・spec と矛盾しないことをレビューする。
- [ ] 通知互換（項目5 承認条件）: `InstanceFailed` field 追加 + variant `#[non_exhaustive]` 後に crate 境界を跨ぐ全 consumer で compile が通ること（`mise run check`/`lint` — in-workspace の完全 pattern・構築サイト修正を含む）。`FailureCause`/`HostNotification`/`ChildExit` が wire codec に serialize されないこと・`WIRE_VERSION` 不変であることを codec テストで確認。`HostError` 新 variant が wire reply の error 表現を変えないこと（host 側のみの分類値 — encoder に到達しない経路を確認）。
- [ ] endpoint checkout／leg 交換／handle 再利用、Bundled fan-out、audio/video、生成前失敗と saturated stderr のテスト仕様を準備する。
- [ ] `mise run fmt-check`、`check`、`lint`、`test`、`test-rt` と文書監査を実行する。
