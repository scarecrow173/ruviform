---
id: "SPEC-034NiVkp0Q1fpkaBoUGNY0"
type: "spec"
status: "implemented"
title: "Issue #117 L1 bundle 子プロセス（1 子プロセス N インスタンス）"
created: "2026-10-28"
updated: "2026-10-28"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/issue-117-bundle-child-process.md"
    - "../plans/implement-issue-117-bundle-child-process.md"
  depends-on:
    - "issue-119-l1-wire-version-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "plugin-l1-out-of-process.md"
    - "issue-119-l1-wire-version-policy.md"
    - "issue-114-real-adapter-l1-e2e.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/issue-122-shared-platform-process-ipc-eval.md"
    - "../designs/issue-115-l1-gui-cross-process-parenting.md"
    - "../discovery/issue-117-bundle-child-process-discovery.md"
    - "../impl/ir/issue-76-l1-core-wire-task-1-implementation.md"
    - "../adr/plugin-isolation-strategy.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-117-bundle-child-process-discovery.md"
    - "plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "issue-119-l1-wire-version-policy.md"
  derived-by:
    - "../designs/issue-117-bundle-child-process.md"
    - "../adr/issue-117-bundle-child-failed-granularity.md"
    - "../plans/implement-issue-117-bundle-child-process.md"
    - "../discovery/issue-116-supervisor.md"
    - "issue-116-supervisor.md"
    - "../designs/issue-116-supervisor.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-117-wire-v2-codec-tests.md"
    - "../test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "../test-specs/issue-117-bundle-e2e-tests.md"
    - "../test-specs/plugin-l1-wire-codec-tests.md"
    - "../test-specs/plugin-l1-proxy-contract-tests.md"
    - "../test-specs/plugin-l1-runner-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/120"
    - "https://github.com/scarecrow173/ruviform/issues/122"
    - "https://github.com/scarecrow173/ruviform/issues/123"
  defers: []
  deferred-by: []
---

# Issue #117 L1 bundle 子プロセス（1 子プロセス N インスタンス）

> **IMPLEMENTED** — 本書は discovery（[issue-117-bundle-child-process-discovery](../discovery/issue-117-bundle-child-process-discovery.md)）を下流として記録した設計前仕様である。decision point は design `../designs/issue-117-bundle-child-process.md`（軸 1-14）と ADR `../adr/issue-117-bundle-child-failed-granularity.md` で確定済み — 本書が確定させるのは Issue が要求する振る舞い・不変条件・受入基準であり、内部構造の確定ではない。実装記録: `docs/impl/ir/issue-117-bundle-child-implementation.md`（AC1–AC12 証拠写像を収録）。

## What Should Be Built

現行 L1 は 1 インスタンス = 1 子プロセス（singleton runner）前提（`l1/mod.rs` L1-4・export `lib.rs` L14-15・`composite.rs` L164-167）。メモリ/spawn コスト削減のため、**1 子プロセスが N 個のプラグインインスタンスを host する bundle 形態**を L1 wire プロトコル・host プロキシ・runner・`PluginHost` 公開面に導入する。

Issue 要求範囲:

1. **`wire::Command` の handle 修飾** — 現行は singleton 前提で handle を落としている（`wire.rs` L303-308）。全インスタンス宛メッセージに `instance` 識別子を導入する
2. **子側インスタンス集合管理** — runner が `instance` で dispatch する registry を持つ
3. **seq/correlation 空間分離** — bundle 化後も `Reply`/`BlockDone` の相関が一意に解決できる
4. **`Failed` 粒度の契約確定** — 子プロセス単位 vs インスタンス単位（OPEN → ADR 候補）
5. **wire バージョニング** — Issue #119 方針（単一最新版・交渉なし）に従い `WIRE_VERSION` 1→2 bump

公開面・RT 契約・`Failed`+通知+pass-through・自動再起動なし等の L1 基底契約は維持する。

## Why Now

- Issue [#117](https://github.com/scarecrow173/ruviform/issues/117) — design `docs/designs/plugin-l1-out-of-process.md` L294 deferred 行（「bundle 子（1 子 N インスタンス）| per-instance spawn の方が `Failed` 粒度に整合・単純 | メモリ/spawn コスト要求 | インスタンス数スケールに弱い」）の re-engagement。同設計 L174 は「将来の bundle 子は `Instantiate`/handle 修飾を wire に追加するプロトコル改訂」と予告済み — **bundle 化は wire 改訂として当初から織り込み済みの進化**。
- Issue #119（wire バージョン方針）が 2026-10-27 に implemented — bump 前提が確定し、#117 の「順序付け」懸念は解消済み。
- インスタンス数スケールのコスト: per-instance spawn は 1 インスタンス毎に 1 子プロセス + 2 I/O スレッド + 1 SHM segment + spawn/handshake コストを支払う（`l1/mod.rs` L976-1088）。同一 runner binary を繰り返し spawn する形態は、bundle 可能なユースケース（同一 plugin_id の複数インスタンス等）でプロセス数・メモリ・起動待ちを N 倍払い続ける。

## Users and Value

- **ホストアプリケーション / エンジン実装者** — 同一 L1 route の複数インスタンスを、子プロセス数を増やさず生成できる。隔離強度（dedicated vs bundle）は route 属性として選ぶ形になり、「クラッシュ隔離を取るかプロセスコストを取るか」の選択肢が API に表れる
- **プラグイン制作側（runner オーサリング）** — `run(factories)` 形の runner が変更後も 1 エントリで bundle に対応する（複数 factory 内蔵 runner は `CreateInstance{plugin_id}` で異種 plugin_id の混在 bundle まで自然に対応可能 — 選択形は設計で確定）
- **observer（`HostNotification` 消費者）** — bundle 内クラッシュでも per-instance `InstanceFailed` を受け取れる現契約を維持

## Scope

### In Scope

- **wire v2 プロトコル**（`crates/ruviform-plugin-core/src/wire.rs`）:
  - `Message::Command`/`BlockIn`/`BlockDone`（`Reply` は設計で確定）への `instance` 識別子追加（envelope 化）
  - `Command` に `CreateInstance`/`DestroyInstance` 相当の新動詞追加（`Instantiate`/`Destroy` の語彙写像を維持するため）
  - `WIRE_VERSION` 1→2 bump + 関連 codec/decode/`WIRE_MAX_*` 系の見直し（新上限が要るかは Requirements 参照）
  - shm segment は **per-instance 維持** — `wire::shm` レイアウト自体は変更しない見込み（共有 segment 化は out of scope）
- **host プロキシ**（`crates/ruviform-plugin-host/src/l1/` + `composite.rs`）:
  - `Shared`/`L1Instance` を「子共有部」（child・I/O スレッド・waiters・notify）と「インスタンス部」（mirror・staging・`RtLeg`・block 簿記・notify 状態）へ分離
  - `PluginHost`/`OutOfProcessSpec` に bundle/dedicated の選択面を追加（形は OPEN — 公開 API 追加のため ADR/design 確定対象）
  - `Instantiate` → 「pool から子を引く or spawn → `CreateInstance`」・`Destroy` → 「`DestroyInstance`（+ 必要なら子を畳む）」の写像
  - `Failed` 収束の確定（OPEN — 推奨: 子スコープ terminal → bundle 全インスタンス `Failed`）
  - `HostNotification::InstanceFailed` の意味確定（OPEN — 推奨: bundle 内各インスタンスへ fan-out・現契約維持）
- **runner**（`crates/ruviform-plugin-export/src/`）:
  - `Runner` singleton → `instance` キーの registry 化（instance/lifecycle/segment/staging を per-instance entry へ）
  - argv 規約の見直し（`--ruviform-shm-key` は per-instance segment と矛盾するため、`CreateInstance` が `shm_key` を運ぶ形に移す設計を推奨）
  - `serve` ループに instance dispatch 段の挿入
- **検証**:
  - wire codec の v2 round-trip/拒否テスト
  - bundle 制御面 E2E（1 子 N インスタンスの lifecycle/dispatch/BlockIn 経路）
  - `Failed` 粒度検証（子死亡 → 全インスタンス `Failed` + 通知 fan-out 等、確定した粒度の検証）
  - `rt_` 命名 RT 契約テストの無退行（per-instance `RtLeg` 不変条件）
- spec/discovery のリンク・README index 整合（本フェーズ）

### Out of Scope

- **共有 shm segment / データ面のレイアウト変更** — per-instance segment 維持が推奨方向。`wire::shm` の第 2 形状（video/NONRT 面等）は Issue #118 側の領域
- **wire v1 との共存・交渉・旧版 decode** — Issue #119 方針により存在しない（v1 実装は bump と同時に撤去 or v2 のみで再定義）
- **自動再起動・supervisor 機構** — L1 契約不変（再起動は呼出し側とプラグイン責務）
- **L2+ サンドボックス強化** — bundle は隔離粒度をプロセス単位に留める（強化は別 Issue）
- **L1 GUI の確定** — `OpenGui`/`CloseGui` は現行 defined rejection のまま維持できる形を維持。GUI handle の instance 紐付け等は L1 GUI 本実装 Issue（#115 系）側の論点
- **frame-I/O dedup・`runner_program()` 解決等の残件** — Issue #123 の領域（bundle で 4 系統が instance 付き frame を読み書きする変更量には影響するが、本 Issue で先行義務は負わない）
- **platform/process/ipc 共通クレート抽出** — Issue #122 評価は #117 着地を前提に延期継続を確定済み。本 Issue で抽出しない
- **エンジン/利用側への bundle 配線** — `PluginHost` の公開面（`OutOfProcessSpec` 等）に選択語彙を足すが、エンジン側での選択運用は consumer 責務

## Requirements

### 基底契約（維持）

- `PluginHandle`・`HostCommand`/`HostReply`/`HostError` の公開コマンド面は L0/L1 で不変 — caller は L0/L1・dedicated/bundle を意識しない（`plugin_id` 登録時選択の原則維持）
- RT 契約: RT スレッドは pipe/shm/mutex/I/O を触らない — `RtLeg` の SPSC try-leg + staging のみ（`l1/data.rs` L20-34 の不変条件を維持）
- `Failed`+通知+pass-through・自動再起動なし・定義済み拒否（`Err(HostError)` 語彙）・`#[non_exhaustive]` 公開 enum 方針
- decode は untrusted peer 入力の検証境界のまま — 新フィールド・新動詞も decode 側で宣言長・reserved・count・handle 妥当性を定義済み拒否する

### wire v2（OPEN を含む）

- **R-W1**: `Message::Command`/`BlockIn`/`BlockDone`（`Reply` は設計で確定）が `instance` 識別子を持つ — 型は **`u32`・host mint** を推奨（子ローカル名空間・公開 `PluginHandle` とは `to_wire` 層で写像）。OPEN: 正確な型幅・allocator・sentinel/scope 表現
- **R-W2**: `CreateInstance`（`{instance, plugin_id, shm_key, depth情報}` 相当）・`DestroyInstance` 相当の新動詞 — `Instantiate`/`Destroy` の語彙写像を「spawn+`CreateInstance`」「`DestroyInstance`（+必要時 `Shutdown`+reap）」に書き換える。OPEN: payload 確定形・`Scan` 等の子スコープ動詞の表現（envelope sentinel or 別扱い）
- **R-W3**: seq/correlation — **推奨: コマンド seq は子ワイド単一空間・block seq は per-instance**（`RtLeg::next_block` 独立・`BLOCK_SEQ_FLAG` 分割維持）。OPEN: `Reply` に instance echo を載せるか・flagged `Reply{seq,Err}`（BlockIn 拒否）の demux 規則
- **R-W4**: `WIRE_VERSION` = **2** — Issue #119 方針どおり厳密一致・bump で host/runner 同時更新・交渉機構なし
- **R-W5**: 上限 — **推奨: `WIRE_MAX_INSTANCES`（仮称）** 相当の子あたりインスタンス上限を wire/子側 registry 上限として導入し、超過 `CreateInstance` は defined `Err`。既存 `WIRE_MAX_*` はメッセージ単位のまま維持。OPEN: 上限値・handle 空間の reuse 規則（単調のみ推奨）

### bundle/dedicated 選択面（OPEN → ADR/design 確定）

- **R-G1**: `OutOfProcessSpec` に bundle/dedicated の選択を表現する面を追加 — **推奨: route（登録 `plugin_id`/`OutOfProcessSpec`）単位の pool が自然な既定**（(program, args) 同一性が pool キーとして機能）。明示 pool キーで route 跨ぎ bundle を可能にする拡張はこの枠組みの上で検討。OPEN: フィールド形（`Isolation` enum vs `bundle_key` 等）・既定値（dedicated が安全側既定か、bundle が既定か）・pool の寿命規則（最後の instance destroy で子を畳むか keep-alive するか）
- **R-G2**: dedicated（現行 per-instance spawn）は隔離オプションとして残す — bundle への一本化は隔離粒度の退化を全利用者に強制するため採用しない
- **R-G3（隔離の意味の限定）**: bundle のインスタンス分離は **state/attribution 分離**（lifecycle・segment・seq 空間・`Failed` 帰属の分離）であって **アドレス空間隔離ではない** — 全 per-instance segment は同一子プロセス空間に map されるため、悪意ある・破損した plugin は同居インスタンスの slot へ書き込み得る（host 側の防衛は `read_out_slot` 相当の fail-closed 検査で defined per-instance 失敗に封じる）。crash 隔離に加え **liveness 隔離も trade する** — 1 インスタンスの plugin hang は子の単一スレッド `serve` を wedge させ command timeout 経由で同居全員を `Failed` にし、大きな frame（state payload 等）が共有 stdin/stdout を serialize して他インスタンスの応答を遅らせ得る。選択語彙の doc にこれらの限定を公開契約として明記する

### `Failed`・通知 semantics（OPEN → ADR 確定）

- **R-F1**: `Failed` 粒度 — **推奨: 子スコープ terminal**（子死亡・pipe/プロトコル破壊は bundle 内全インスタンスを `Failed` 収束。per-instance `Err(HostError)` 系の定義済み拒否は現行どおり）。ADR で確定記録を推奨（公開 `LifecycleState`/`HostNotification` の意味に関わるため）
- **R-F2**: `HostNotification::InstanceFailed` — **推奨: bundle crash でも影響を受けた各インスタンスへ `handle`+`exit` を 1 つずつ fan-out**（現契約維持・`exit` は全て同値）。OPEN: bundle 相関を表す新 variant を初期に入れるか（`#[non_exhaustive]` で後追加可能なので初期不要と推奨）

### runner 側

- **R-R1**: runner は `instance` キーの registry を持ち、`Command`/`BlockIn` を対象 instance に dispatch する — plugin hook panic は現行どおり `Err(PluginFailed)` に正規化（instance-scoped・runner は生き続ける）
- **R-R2**: argv 規約 — **推奨: v2 runner は `<runner>` 単体（+`--ruviform-*` 管理フラグのみ）で起動し、`--ruviform-shm-key`・`--ruviform-plugin-id` は `CreateInstance` payload へ移す**。OPEN: 確定 argv 形・`--ruviform-plugin-id` が複数 factory runner で `CreateInstance{plugin_id}` に移る際の意味
- **R-R3**: 終了規約 — **推奨: 子の終了は host 送信 `Shutdown`（または stdin EOF）のみ**（最後の `DestroyInstance` で host が `Shutdown` を送る現行意味の延長）。子の自律 exit は新語彙を要るため入れない
- **R-R4**: runner registry は bounded — `WIRE_MAX_INSTANCES` 相当上限を超える `CreateInstance` は defined `Err` で拒否（untrusted host からの無制限 registry 膨張を防ぐ）

### 検証

- **R-V1**: wire v2 の codec round-trip・拒否（instance field・新 tag・上限超過・reserved）テスト
- **R-V2**: bundle E2E — 1 子 N インスタンスで scan→create→lifecycle→block→state→destroy が並行・interleave で動くこと・instance 跨ぎの seq/reply 混線がないこと
- **R-V3**: 確定した `Failed` 粒度の検証 — 子死亡/プロトコル破壊時に確定範囲のインスタンスが `Failed` 収束し通知が確定形で出ること・残存インスタンスへの後続コマンドが `Err(Failed)` で拒否されること
- **R-V4**: `rt_` 命名 RT 契約テスト — per-instance `RtLeg` の no-block/no-alloc 不変条件が bundle 経路でも維持されること
- **R-V5**: dedicated 経路の無退行（既存 E2E が bundle 導入後も通ること）

## Acceptance Criteria

> 各 AC は「確定した選択肢が採用された場合に検証されるべき振る舞い」を記す。~~OPEN と印した論点は design/ADR 確定後に対応する検証へ落とす~~ → **design `../designs/issue-117-bundle-child-process.md`（軸 1-14）と ADR `../adr/issue-117-bundle-child-failed-granularity.md` で全 OPEN 論点が確定済み** — 以下は確定契約に対する検証として記述する。

- [ ] **AC1（wire v2 codec）**: `WIRE_VERSION = 2` で、`instance` 識別子を持つ `Command`/`Reply`/`BlockIn`/`BlockDone` と `CreateInstance`/`DestroyInstance` 動詞が codec round-trip を持ち、`Hello.version` 不一致（v1 子・v1 host）は handshake 失敗で定義済み拒否される。decode が不正 `instance`（`WIRE_MAX_INSTANCES` 以上・`WIRE_INSTANCE_NONE` 不整合）は defined 拒否し、panic・無制限アロケーションしない（未登録 handle は decode ではなく runner dispatch/host inert 契約の管轄 — AC6）。`encode_host_error(HostError::InstanceLimit) → tag 9`・`decode(9) → InstanceLimit` の両向き round-trip が pin される（`encode_host_error` の `_ => Err(UnsupportedVariant)` wildcard arm — 追加忘れで defined 拒否が子死亡に化ける退行を防ぐ）
- [ ] **AC2（bundle 制御面）**: 同一 L1 route の 2 回目以降の `Instantiate` が **子プロセスを追加 spawn せず** 既存子へ `CreateInstance` で成立し、各インスタンスが独立した `PluginHandle`・独立した lifecycle（init→activate→start→stop→deactivate→save/restore→destroy）・独立した `process` 等価経路（per-instance shm segment + `RtLeg`）を持つ。`Scan`/`Instantiate`/`Destroy` 等の caller 側コマンド系列は L0/dedicated と同一面を維持する。pool 選択述語 `live_count < WIRE_MAX_INSTANCES && !terminal && !shutdown_initiated` に従い、該当子が無ければ新規 spawn する
- [ ] **AC3（seq/correlation）**: 2 インスタンスが同一子内で並行してコマンド・ブロックを処理しても、`Reply`/`BlockDone` が正しい instance・seq に相関し、混線・他インスタンスへの誤配・waiter ハングがない（コマンド seq 子ワイド + `waiter.instance` 照合・block seq per-instance + `block_in_flight: HashSet<(u32,u64)>` の確定規則）
- [ ] **AC4（`Failed` 粒度 — 確定）**: 子スコープ terminal 基礎 + 帰属可能な per-instance 収束 + inert 規則の全分岐を検証する: (a) 子死亡/帰属不能違反で **bundle 内全 live インスタンスが `LifecycleState::Failed`** に収束・全 pending waiters が `Err(Ipc)` wake・全 RT leg が pass-through・以後 `Err(Failed)`; (b) 帰属可能違反（live instance の `block_in_flight` に無い block seq）は対象 instance のみ `Failed`・兄弟生存; (c) **inert** — registered-but-not-live（`converge_instance` 済み `Failed`・`Destroying`）宛 frame（drain 済み waiter 宛 `Reply`・合成済み doorbell 宛 `BlockDone`）と `i ∉ Inner.instances` 宛完了は静かに drop され違反・収束・通知を起こさない; (d) `Shutdown` 送信後の子 frame・EOF は non-event（clean teardown 不変）; (e) `DestroyInstance` doorbell window — serialize で entry が `Destroying` 化・以後の submit は `mark_failed` 合成・in-flight doorbell は通常完了・reply で entry 除去（残存 seq は `mark_failed` 解放）
- [ ] **AC5（通知）**: bundle crash で **影響を受けた各インスタンスに `HostNotification::InstanceFailed{handle, exit}` が 1 つずつ** 観測され、`exit` は全て同一の `ChildExit`。per-instance 収束（子生存）は `ChildExit{None,None}` で 1 通知。mid-`CreateInstance` の子死亡は **通知なし**（公開 handle 未 mint — `Err(Ipc)` で `Instantiate` 失敗）、mid-`DestroyInstance` の子死亡は entry が registered のまま `Failed` + 通知 emit + `Destroy` は `Ok` を返す
- [ ] **AC6（runner registry）**: runner が `instance` で dispatch し、plugin hook panic が `Err(PluginFailed)` に正規化されて対象 instance のみ失敗・runner と他 instance は生き続ける。`CreateInstance` 超過（上限越え `Err(InstanceLimit)`・既登録 handle 重複 `Err(Ipc)`）と **未登録 registry key 宛の `BlockIn`/動詞 → flagged `Reply{i,seq,Err(InvalidHandle)}`** は defined `Err` で拒否され runner が死なない（runner 側 protocol violation にならない）。Scan 照合済み id に対する `Err(UnknownPluginId)` reply は host が `Err(Ipc)` に畳む
- [ ] **AC7（dedicated 無退行 — 確定 seam）**: `PluginHost` + `isolation: Dedicated` の route（または `#[path]` harness が駆動する `pub(crate)` spawn+`CreateInstance` helper）で既存 E2E（`l1_proxy_e2e`・`l1_runner_e2e` 相当）が無退行 — v2 handshake（spawn→Hello→`Scan`→`CreateInstance`）経由でも lifecycle/block/crash 契約が変わらない
- [ ] **AC8（RT 契約）**: bundle 経路でも `rt_` 命名テストが `mise run test-rt` でグリーン — endpoint detach→process→leg return の per-instance 不変条件が維持される
- [ ] **AC9（上限・handle reuse・live_count）**: `CreateInstance` が `WIRE_MAX_INSTANCES` 相当上限を超えると defined `Err(HostError::InstanceLimit)` で拒否され、host 側は `Err` で `Instantiate` を失敗させる（panic・無限待ちなし）。上限内の destroy→create で handle 空間が再利用規則どおり振る舞う — wire-ID reuse（destroy→create で同一 `i`）時に driver が前世代の queued submit を持ち得る場合、`Destroying` 化が新規 doorbell を止め `mark_failed` で解放し、新インスタンスの ring に stale doorbell が届かない。`live_count` は `Failed` 未 `Destroy` entry を含む registered 全件を数える。`Failed` + live 子への `Destroy` は `DestroyInstance` を送信して runner slot を回収し、terminal 子への `Destroy` はローカル撤去のみで `Ok` を返す
- [ ] **AC10（CI/監査）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・`doc_status.js audit` blocking 0・findings なし
- [ ] **AC11（pool/子 lifecycle — shutdown-on-empty）**: 最後の `Destroy` で live=0 になった子は `Shutdown`+grace+reap+pool eviction。`converge_child` は terminal 子を pool から除去。spawn/handshake/`CreateInstance` 失敗は pool entry を巻き戻す（`live_count` も戻る）— fresh-spawn 子の失敗は子ごと teardown、既存共有子への `CreateInstance` 失敗は instance-entry のみ巻き戻し（live 兄弟を持つ子を殺さない）。`L1Instance` drop は live 兄弟を持つ子を殺さず、最後の参照が外れた子は最後の referrer が bounded teardown（`Shutdown` or kill + reap + join）を完了する
- [ ] **AC12（in-flight lifecycle crash）**: `CreateInstance` 送信中の子死亡 → ローカル収束のみ・`InstanceFailed` 通知なし・`Instantiate` caller へ `Err(Ipc)`・registry entry 除去（`live_count` リークなし）。`DestroyInstance` 送信中の子死亡 → entry は registered のまま `Failed` 収束 + per-instance 通知 emit + `Destroy` は `Ok` を返す（公開 slot は解放 — 現行意味維持）

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `Failed` 粒度の最終確定（子スコープ vs 限定 instance-scope） | OPEN — 子スコープ terminal を推奨として記録 | design/ADR（公開契約に関わるため ADR 推奨） | `Failed`/`InstanceFailed` の意味が曖昧なまま実装され observer 契約が破れる |
| bundle/dedicated 選択面の API 形・既定値・pool 寿命 | OPEN — route 単位 pool を既定推奨・dedicated 残存を要求として記録 | design/ADR（公開 spec フィールド） | 公開 API を後で変えることになり互換破壊 or 冗長な 2 系統が残る |
| `Reply`/`BlockDone` の instance echo・flagged `Reply` demux 規則 | OPEN — seq 子ワイド＋block per-instance を推奨 | design（wire v2 codec 詳細） | 相関が一意に解けない wire 定義になる |
| 子スコープ動詞の scope 表現（envelope sentinel vs 別扱い） | OPEN | design | `Scan` 等の dispatch が曖昧化 |
| `WIRE_MAX_INSTANCES`（仮）値・handle reuse 規則 | OPEN — 上限導入自体は要求として記録 | design | 子 registry が untrusted 入力で無制限化 |
| bundle 相関通知 variant（`BundleFailed` 等） | 初期不要と推奨（per-instance fan-out で契約維持・`#[non_exhaustive]` で後追加可） | observer が bundle 相関を要求した時点の follow-up | observer が「同一子の死」を識別できない（現状でも推測可能: 同一 `ChildExit` の連続通知） |
| pool keep-alive（最後の instance destroy 後も子を残すか） | OPEN — `Shutdown` 明示送信を推奨 | design | warm pool で spawn コスト削減効果が薄れる or 空子がリーク |
| 異種 plugin_id の混在 bundle（同一 runner binary・複数 factory） | 選択形として記録（明示 pool キー拡張で可能） | design — 必須要件ではない | runner binary が plugin_id 毎に別 process を要し bundle 効果が限定される |
| `kill_child`（検証用）の bundle 版形 | OPEN — handle 指定で所属子 kill（現行意味延長）を推奨 | design | 検証面で bundle crash 注入が書けない |
| 共有 shm segment / `wire::shm` レイアウト改訂 | **out of scope と確定** — per-instance segment 維持 | Issue #118 系 | 本 Issue が shm 再設計に沈む |
| frame-I/O dedup・`runner_program()` 解決 | 別 Issue 継続 | Issue #123 | bundle で instance 付き frame を 4 系統が読み書きする変更量が増える（先行で縮むが必須ではない） |

## Sources and Relations

- Issue [#117](https://github.com/scarecrow173/ruviform/issues/117)（本 spec の対象 Issue — bundle 子 1 子 N インスタンス）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED — bundle を non-goal とした起点）・Issue [#119](https://github.com/scarecrow173/ruviform/issues/119)（wire バージョン方針・implemented — bump 前提を確定）・Issue [#115](https://github.com/scarecrow173/ruviform/issues/115)（L1 GUI 越境 parenting — GUI handle の instance 紐付け関連論点）・Issue [#118](https://github.com/scarecrow173/ruviform/issues/118)（video/NONRT データ面 — shm レイアウト別改訂）・Issue [#120](https://github.com/scarecrow173/ruviform/issues/120)（共有リーパー/io 多重化 — bundle 後の子単位 I/O スレッドと相性）・Issue [#122](https://github.com/scarecrow173/ruviform/issues/122)（platform/process/ipc 評価 — #117 を再設計前提に延期）・Issue [#123](https://github.com/scarecrow173/ruviform/issues/123)（frame-I/O dedup 等残件）
- [issue-117-bundle-child-process-discovery](../discovery/issue-117-bundle-child-process-discovery.md) — 本 spec の discovery（コード観測事実・軸別選択肢比較・推奨方向）
- [plugin-l1-out-of-process spec](plugin-l1-out-of-process.md)・[design](../designs/plugin-l1-out-of-process.md) — #76 系上流（singleton 前提の原点・deferred 行・語彙写像・`Failed`/通知/pass-through/RT 契約）
- [issue-119-l1-wire-version-policy](issue-119-l1-wire-version-policy.md) — `WIRE_VERSION` bump 前提の確定方針（depends-on）
- [issue-76-l1-core-wire-task-1-implementation IR](../impl/ir/issue-76-l1-core-wire-task-1-implementation.md) — wire 導入の実装記録（bundle non-goal 記録・singleton 反映の根拠）
- [plugin-isolation-strategy ADR](../adr/plugin-isolation-strategy.md) — 階層化分離の決定（bundle/dedicated の選択面設計の前提）
- [issue-122-shared-platform-process-ipc-eval](../designs/issue-122-shared-platform-process-ipc-eval.md) — #117 を再設計前提とする抽出延期判定
- 検証 oracle（本 spec 実装時に `verified-by` で紐付け見込み）: `docs/test-specs/plugin-l1-wire-codec-tests.md`・`docs/test-specs/plugin-l1-proxy-contract-tests.md`・`docs/test-specs/plugin-l1-runner-e2e-tests.md`
