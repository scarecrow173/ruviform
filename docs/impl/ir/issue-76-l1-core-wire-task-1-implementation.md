---
id: "IMPL-034LPNEgk770ZFr8yU3823"
type: "impl"
status: "completed"
title: "Issue #76 L1 core contracts + wire codec — Task 1 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "docs/impl/ir/issue-76-l1-core-wire-task-1-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/command.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/README.md"
      - type: "file"
        path: "docs/tasks/l1-core-contracts-and-wire-codec.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/l1-core-contracts-and-wire-codec.md"
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

# Issue #76 L1 core contracts + wire codec — Task 1 implementation

## Summary

Issue #76 Task 1 完了 — actions 1–4（契約移設 + `LifecycleState::Failed`
拡張 + `wire` codec + narrow gate）を全範囲実施。plugin-core に
`boundary.rs`（新規・175 行・共有境界語彙）・`plugin.rs`（新規・
321 行・プラグイン側契約）・`wire.rs`（新規・1954 行・手書き LE
framed codec）を新設し、host 側は `pub use` 再エクスポートへ転換
（`ruviform_plugin_host::*` 既存パス不変・既存テスト全パスで互換
実証）。`lifecycle.rs` に終端準状態 `Failed`（到達は reported
`LifecycleEvent::Failed` のみ・脱出は `Destroy` のみ）を追加。
独立レビュー 2 件 + fix agent: P1 ×1（host.rs instantiate の
stale-handle aliasing — #74 由来の既存欠陥を Task 5 依存のため
本 task で修正）・P2 ×1（CLAP adapt.rs の `Failed` 状態符号化
欠落）を修正済み、coverage 追加・doc link/nit 修正を全て
mutation 検証済み。全 gate green 実 log。

## Context

approved plan `docs/plans/implement-issue-76-plugin-l1-out-of-process.md`
の Task 1（`docs/tasks/l1-core-contracts-and-wire-codec.md`・
actions 1–4）。upstream: approved spec/design
（`docs/specs/plugin-l1-out-of-process.md`・
`docs/designs/plugin-l1-out-of-process.md`）・ADR
`docs/adr/plugin-isolation-strategy.md`・#74 L0 実績（CLOSED — host
側の `Plugin`/`PluginFactory`/`HostError` 公開面はそのまま共有語彙
として継承）・TSpec `docs/test-specs/plugin-l1-wire-codec-tests.md`
（proposed — wire codec・Failed 遷移の roundtrip/malformed/境界
ケースの正本）。本 task は全後続の共有前提 — Task 2
（plugin-export ランナー）・Task 3（L1 proxy 制御面）の双方が
wire/移設契約を使う。後続 task は全て未着手。

## Implementation

### Changes

- `crates/ruviform-plugin-core/src/boundary.rs`（新規・175 行・
  `lib.rs` で `mod boundary` + ルート re-export）:
  - host `src/command.rs` からの verbatim 移設:
    `PluginHandle`（`{slot: u32, generation: u16}` 不透明 handle —
    `PluginHandle::new` は task doc どおり `#[doc(hidden)] pub` 化:
    mint は host 内部操作だが型は共有語彙）・`GuiHandle`・
    `GuiParentToken`・`HostError`。
  - `HostError` に 3 variant 追加（`#[non_exhaustive]` で
    non-breaking）: `Failed`（terminal `LifecycleState::Failed` の
    インスタンスへの `Destroy` 以外のコマンド拒否 —
    `PluginFailed` とは別物で plugin hook 非起因）・`Ipc`（L1
    プロセス間 transport 失敗 — spawn/handshake/version
    mismatch/I/O/timeout）・`DuplicatePluginId`（plugin_id 二重
    登録 — Task 5 composite の `register_out_of_process` が使う）。
    Display arm 3 件追加。
  - `From<LifecycleError> for HostError` impl を host.rs から移設
    — `HostError` も `LifecycleError` も plugin-core 所有になった
    ため orphan rule 上ここが唯一の正当な置き場所（host 側 impl
    は削除）。
- `crates/ruviform-plugin-core/src/plugin.rs`（新規・321 行・
  `mod plugin` + ルート re-export）:
  - host `src/plugin.rs` からの verbatim 移設: `Plugin` trait（
    `PluginProcessor` + 非 RT instance hooks — `init`/処理
    通知/state/gui 全てデフォルト実装付き）・`PluginFactory`
    trait・`ErasedPlugin` trait + `erase<P: Plugin>` helper・
    erased I/O view `HostInput`/`HostOutput`（`&dyn BufferView` /
    `&mut dyn BufferViewMut` の薄い newtype — heterogeneous
    instance を `dyn` 越しに処理させる dispatch 面）。
  - `crate::` 参照に調整（`crate::boundary::{GuiHandle,
    GuiParentToken, HostError}`）・doc 互換維持 — 唯一の変更は
    intra-doc link 1 件（`InProcessHost::processor` →
    `InProcessHost::plugin_mut` 実 API 名への修正、レビュー修正群
    に含む — 後述）。
- `crates/ruviform-plugin-core/src/wire.rs`（新規・1954 行・
  `pub mod wire` — export/host 双方が参照する共有面）:
  - framing — `[u32 LE payload_len][u8 kind][payload]`・
    `payload_len` は payload バイトのみ計数（kind byte・length
    field 自身は含まず）・`WIRE_FRAME_HEADER_LEN = 5`。
  - 定数 — `WIRE_VERSION = 1`（`Hello` のみが version を運ぶ —
    他メッセージは繰返さない・version mismatch は codec ではなく
    proxy が handshake 失敗として裁定）・task doc どおりの
    `WIRE_MAX_FRAMES = 8192`・`WIRE_MAX_CHANNELS = 16`・
    `WIRE_MAX_EVENTS = 1024` に加え、bounded read 担保用に
    **追加の 4 定数**（deviation — 後述）:
    `WIRE_MAX_PAYLOAD = 16 MiB`・`WIRE_MAX_STRING_BYTES = 4096`・
    `WIRE_MAX_ELEMENTS = 1024`・`WIRE_MAX_STATE_BYTES = 16 MiB`。
  - 型 — `Message` 6 種（`Hello{version, capabilities}`・
    `Command{seq, cmd}`・`Reply{seq, result: Result<Reply,
    HostError>}`・`BlockIn{seq}`・`BlockDone{seq, result:
    ProcessResult}`・`Shutdown`）・`wire::Command` 10 動詞（
    `Scan`/`Initialize`/`Activate{config}`/`StartProcessing`/
    `StopProcessing`/`Deactivate`/`SaveState`/`RestoreState{bytes}`/
    `OpenGui{parent}`/`CloseGui` — `Instantiate`/`Destroy` は
    spawn+handshake と `Shutdown`+reap が運ぶため動詞を持たず、
    handle も持たない — runner 子は singleton instance）・
    `wire::Reply`（`Ok`/`Scanned(Vec<Descriptor>)`/`State{bytes}`/
    `Opened{gui}`）・enum は全て `#[non_exhaustive]`。
  - `wire::Descriptor` — 所有型 struct（`String` ×4 +
    `Vec<String>`）。`PluginDescriptor` の `&'static` フィールドは
    wire を渡れないため所有文字列で運ぶ — `From<PluginDescriptor>`
    で encode 時に一度だけ owned 化・逆変換は存在しない（
    `HostReply::Scanned` は登録供給の `'static` を返し続ける —
    `Box::leak`/forged static なし）。`matches(&PluginDescriptor)`
    でフィールド等価照合（proxy が scan 結果を spec と照合する
    検査 — features は len 一致 + 逐順 pairwise 比較、
    order-sensitive — follow-up 記録）。
  - codec — `encode(&Message, &mut Vec<u8>)`（append 型・1 呼出し
    = 1 frame）・`decode(&[u8]) -> Result<Message, WireError>`。
    `WireError` は `#[non_exhaustive]`（`Truncated`/`LengthMismatch`/
    `UnknownKind`/`Malformed`/`LimitExceeded`）。decode は panic
    なし・部分消費なし — frame 全体を検証してから値を返す。
  - `Reader<'a>` — 内部カーソル型。全読取りは `take()` 経由で
    remaining 検査（`Err(Truncated)`）・`read_count`/`read_bytes`/
    `read_string` は `WIRE_MAX_*` 上限で宣言値を先に cap してから
    確保（declared count だけで巨大確保しない —
    `decode_rejects_impossible_counts_without_allocating` で検証）。
  - 固定 wire レイアウト — `ProcessConfig`/`ProcessResult`/
    `GuiHandle`/`GuiParentToken`/`PluginHandle`/`HostError`（+
    内部 `LifecycleError`/`LifecycleState`/`LifecycleEvent`/
    `PrepareError` 符号化）/`TransportInfo`/`Event`/`EventList`/
    `MediaTime` は accessor encode・ctor decode — `repr(Rust)`
    memcpy 横断なし。decode は ctor 経由で型不変条件（
    `MediaTime` の base 正規化・`Event` の NaN 拒否・reserved
    field ゼロ・`TransportInfo` flags）を再検証 — 不正値は
    defined `Err` 拒否。
  - inline `#[cfg(test)] mod tests` — 24 tests（module doc +
    `Descriptor::matches` doctest で 25 観測点）: 全 `Message`
    kind・全 `Command` 動詞・全 `Reply` payload・全 `HostError`
    variant の roundtrip・frame レイアウト（LE・length-prefix・
    append 型）・truncated/length-mismatch/unknown-kind/trailing
    payload・宣言長超過・不可能 count 非確保・bad tag/不変条件・
    nonzero reserved/bad UTF-8・`Descriptor` From/matches・
    `WireError` Display。
- `crates/ruviform-plugin-core/src/lifecycle.rs`（+113 行）:
  - `LifecycleState::Failed` — 終端準状態（terminal semi-state）。
    到達は reported `LifecycleEvent::Failed` のみ・脱出は
    `Destroy → Destroyed` のみ・それ以外の全 event は
    `IllegalTransition`。retryable `InitFailed`/`ActivateFailed`
    とは意味が異なり terminal（回復不能・teardown のみ）である旨
    を variant doc・module doc（遷移表 2 行追加 + 「`Failed` is a
    terminal semi-state」定義文）・event doc に明記。
  - 遷移表 const 拡張 — `(Created|Initialized|Activated|Processing,
    Failed) → Failed`・`(Failed, Destroy) → Destroyed`（
    `(Created|Initialized|Failed, Destroy) → Destroyed` に併合）。
    `accepts_processing()` は `Failed` で `false`・`is_failed()`
    追加（`const`・RT-suitable）。
  - `LifecycleEvent::Failed` — host が注入する reported 遷移（L1
    子プロセス死亡・応答喪失・プロトコル喪失 — plugin 自身は
    発行しない旨を doc 明記）。
  - テスト — `lifecycle_at` の precondition path に `Failed` 経路
    追加 + `failed_is_reachable_from_every_live_state`・
    `failed_is_terminal_and_only_destroy_escapes`・
    `failed_reports_via_state_queries` + Destroyed の全 event 拒否
    行に `Failed`/`InitFailed`/`ActivateFailed` を追加（coverage
    レビュー修正）。
- `crates/ruviform-plugin-core/src/lib.rs` — `mod boundary`/
  `mod plugin` + `pub mod wire` + ルート re-export（
  `GuiHandle`/`GuiParentToken`/`HostError`/`PluginHandle`・
  `Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`/`HostInput`/
  `HostOutput`）+ crate doc に移設契約・共有語彙・wire codec の
  言及。plugin-core は外部（third-party）依存ゼロ維持 — wire は
  手書き codec（serde/postcard 不使用）で workspace foundations
  （audio-core/media-core/video-core の `path` 依存）のみ参照
  （`cargo tree -p ruviform-plugin-core` で実確認）。
- host 側の再エクスポート化:
  - `src/plugin.rs` — 318 行の型定義を `pub use
    ruviform_plugin_core::{ErasedPlugin, HostInput, HostOutput,
    Plugin, PluginFactory, erase}` へ差替え（16 行の薄い
    re-export モジュール — 移設由来の module doc は残す）。
  - `src/command.rs` — `PluginHandle`/`GuiHandle`/`GuiParentToken`/
    `HostError` の定義を `pub use ruviform_plugin_core::{…}` へ
    差替え・`HostCommand`/`HostReply` は host に残る（128 行）。
    `PluginDescriptor`/`ProcessConfig` の use は core から取得。
  - `src/host.rs` — `From<LifecycleError> for HostError` impl 削除
    （core `boundary.rs` へ移設）・doc link 修正 2 件
    （`InProcessHost::processor` → `plugin_mut`・`Plugin::process_host`
    → `ErasedPlugin::process_host`）+ **generation 修正**
    （reviewer P1 — 後述）。
  - `src/lib.rs` — doc link 修正 + link definition 追加
    （`LifecycleEvent`/`PluginProcessor` — `command.rs` が core 側へ
    移った後も crate doc の参照が解決するよう定義を追加）+
    リグレッションテスト `reused_slot_generations_do_not_alias`。
  - `README.md` — `InProcessHost::processor` → `plugin_mut` の doc
    修正（実 API 名と一致）。
- `crates/ruviform-plugin-clap/src/adapt.rs`（+32 行）—
  `LifecycleState` atomic 符号化表に `STATE_FAILED = 5` を追加
  （encode + decode の往復 — `Failed` が `Destroyed` へ decode
  collapse するのを防ぐ。enum 拡張の必然的追従）+ テスト
  `atomic_lifecycle_failed_survives_the_atomic_round_trip` +
  precondition path に `Failed` 経路追加。

### Review findings and resolutions（独立レビュー 2 件 + fix agent — 全て mutation 検証済み）

- **P1（fixed）— stale-handle aliasing（pre-existing #74 defect）**:
  `InProcessHost::instantiate` の slot 再利用時、generation を
  `self.instances[index].as_ref().map_or(0, …)` から読んでいた
  — しかし destroy は slot を `None` にするため再利用世代は常に
  `0 + 1 = 1` にリセットされ、stale handle `{slot, gen 1}` が
  新しい live instance と alias し得た。Task 5（統一ハンドル
  空間の `PluginHost` composite）が generation 規則に依存する
  ため本 task で修正: `generations: Vec<u16>` フィールドを新設
  （destroy を跨いで生存する per-slot 世代カウンタ）・再利用は
  `generations[index].wrapping_add(1)` で単調増加・新規 slot は
  `generations.push(0)` で pair 管理。regression test
  `reused_slot_generations_do_not_alias`（slot 3 回 churn →
  generation 1,2 と単調増・stale handle1 は `InvalidHandle`・
  live handle2 は受理）— mutation 検証済み（旧実装に戻すと
  失敗）。**task 先行時点の欠陥であり task scope を越えるため
  個別に記録** — Task 5 の前提となる正しさの担保のため Task 1 で
  修正した。
- **P2（fixed）— CLAP `Failed` 符号化欠落**: `adapt.rs` の
  atomic lifecycle mirror は `LifecycleState` を `u8` に符号化
  するが、`Failed` variant 追加時に表が未拡張のままだった —
  `_ => STATE_UNKNOWN` → decode は `Destroyed` に collapse し、
  表の唯一の escape（`Destroy`）すら illegal になって wedge
  し得た。`STATE_FAILED = 5` を encode/decode に追加して往復を
  保存 + テスト追加（mutation 検証: 表から `STATE_FAILED` を
  除くと失敗）。
- **coverage 追加（fixed）**: lifecycle の Destroyed 全 event 拒否
  行に `Failed`/`InitFailed`/`ActivateFailed` を追加・wire.rs の
  `decode_transport`/`decode_event` nonzero-reserved 拒否テスト・
  `Descriptor::matches` の id/vendor/version mutation ケース追加。
- **doc link 修正（fixed）**: cargo doc 警告 6 → 0 —
  `PluginProcessor`（crate doc link def 追加）・
  `ErasedPlugin::process_host`・`InProcessHost::processor →
  plugin_mut`（host.rs/lib.rs/README の 3 箇所 — `plugin.rs`
  移設後も残っていた実 API 名との不整合）。
- **wire.rs nit 修正（fixed）**: `Malformed` doc の timebase 文言
  整理・`Hello.capabilities` 非ゼロを `Malformed` で無条件拒否
  へ hardening（`WIRE_VERSION = 1` では capability bit は未定義
  — 他の nonzero reserved field と同じ defined rejection。将来
  capability を定義する拡張は必ず `WIRE_VERSION` bump を伴う
  ため、v1 下での非ゼロ受理は未知語彙の黙認になる — この
  rationale は variant doc に明記）+ テスト・`encode` の
  未検査 `WIRE_MAX_*` 超過が dead zone である旨の doc（encode は
  over-bound field を verbatim に書き、peer の decode が defined
  rejection する非対称 — encode 側の panic は `len_u32` の
  `u32::MAX` 超過時のみで実データでは到達不能、と `# Panics`
  doc に明記）。

### Decisions During Implementation

- **追加の `WIRE_MAX_*` 定数 4 件**（deviation・追加）— task doc
  の定数集合は `WIRE_VERSION`/`WIRE_MAX_FRAMES`/`WIRE_MAX_CHANNELS`/
  `WIRE_MAX_EVENTS` のみだったが、codec が「unbounded な宣言長を
  信用しない」という bounded-read 契約を守るには payload 総量・
  文字列・要素数・state blob の各上限が必要。`WIRE_MAX_PAYLOAD =
  16 MiB`（frame 総量）・`WIRE_MAX_STRING_BYTES = 4096`（
  `Descriptor` 文字列）・`WIRE_MAX_ELEMENTS = 1024`（feature
  vec）・`WIRE_MAX_STATE_BYTES = 16 MiB`（state blob）を追加 —
  decode 側の検証境界として `WireError::LimitExceeded` で拒否。
  encode はこれらを検査しない（peer の decode が拒否する非対称 —
  doc 明記）。
- **`Hello.capabilities` 非ゼロは `Malformed` で無条件拒否**（
  deviation・hardening）— 「reserved field を 0 強制するかどうか」
  は task doc 未指定だったが、`Event`/`TransportInfo` 等他の
  nonzero reserved field は全て `Malformed` 拒否で統一されて
  おり、capability 拡張は必ず `WIRE_VERSION` bump を伴う設計
  （payload 変更は version bump を運ぶ — module doc「
  Versioning」節）のため v1 下での非ゼロは malformed と同義。
- **`Descriptor::matches` は features の順序を区別する**（
  deviation・deliberate）— design の「フィールド等価比較」は
  順序非依存か不明瞭だったが、`PluginDescriptor.features` は
  registration-supplied `'static` 配列で順序は記述子の一部
  （category tags の列挙順）。proxy の照合は「子が正しく同一の
  descriptor を返したか」の検証であり、順序違いは spec との
  不一致として正直に拒否する方が契約として正しい（緩めは後から
  可能・逆は困難）。follow-up に順序感度の再評価を記録。
- **`wire::Command` は `Instantiate`/`Destroy` 動詞を持たない**
  — plan/design どおり。`Instantiate` = spawn + `Hello`
  handshake・`Destroy` = `Shutdown` + reap。`Command` variant が
  handle を持たないのも singleton runner 前提の正直な反映
  （module doc「`Instantiate`/`Destroy` vocabulary mapping」節
  に明記）。
- **encode/decode の非対称責務** — encode は信頼入力（自側が
  構築した `Message`）を verbatim に書き `WIRE_MAX_*` を検査
  しない。decode は untrusted peer input の検証境界として全ての
  宣言長・reserved field・型不変条件を拒否する。`len_u32` の
  `expect` は encode 側 contract（untrusted bytes が encode を
  通る経路は存在しない）として doc 明記。
- **plugin-core は外部依存ゼロ維持** — wire は完全手書き
  codec。serde/postcard 等の serialization crate は採用せず
  （design の確定事項どおり — `cargo tree -p
  ruviform-plugin-core` で third-party dep 不在を実確認）。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替 codec/serialization 評価は design の確定事項を
そのまま実装した（新規 experiment なし）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean — `#[non_exhaustive]`
    違反なし・wire.rs の pub enum 全て準拠）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **49 test binaries 全 pass**・既存 L0 テスト無退行で
    `pub use` 互換を実証）
  - `mise run test-plugin-core` — exit 0（core 集中レーン —
    lib tests **59**・`video_contract` **8**・doctests **26**・
    他 **2** = **95 pass** — wire.rs 24 tests・lifecycle
    `Failed` 系を含む）
  - `mise run lint-plugin-core` — exit 0
  - `mise run test-rt` — exit 0（`rt_` 名前フィルタで **109
    件**実選択 — plugin-core 移設型の RT 契約テストを含み
    無退行）
- 構造確認（実コマンド）:
  - `cargo tree -p ruviform-plugin-core` — third-party 依存ゼロ
    （audio-core/media-core/video-core の `path` 依存のみ —
    serde/postcard 不在）を実確認
  - `rg -n "pub use ruviform_plugin_core"
    crates/ruviform-plugin-host/src/plugin.rs
    crates/ruviform-plugin-host/src/command.rs` — 再エクスポート
    化を確認（plugin.rs L16・command.rs L22）
  - `rg -n "Failed" crates/ruviform-plugin-core/src/lifecycle.rs`
    — `LifecycleState::Failed`/`LifecycleEvent::Failed`/
    `is_failed` 存在
  - `cargo doc -p ruviform-plugin-host` — **0 warnings**
    （doc link 6 件修正後。隣接 `ruviform-audio-core` に
    pre-existing 警告 2 件あり — 本 task 非起因・follow-up
    記録）
- レビュー修正の mutation 検証（実施済み）:
  - generation 修正 — 旧実装（`instances[index]` 参照）へ戻すと
    `reused_slot_generations_do_not_alias` が `InvalidHandle`
    ではなく受理または世代不一致で失敗
  - `STATE_FAILED` 削除で adapt.rs テスト失敗
  - `Descriptor::matches` 各フィールド mutation でテスト失敗
  - `Hello.capabilities` 非ゼロ受理へ戻すと hardening テスト失敗

## Non-Goals

- L1 proxy 機構・runner・`PluginHost` composite — Task 2–5 の
  scope（wire 型・`HostError::{Failed,Ipc,DuplicatePluginId}` は
  本 task が供給する共有語彙だが、それを使う機構は後続 task）。
- wire バージョン進化機構（`WIRE_VERSION > 1` の移行手順）・
  capability bit 定義・bundle 子（handle 修飾）— plan non-goal。
- shm スロットレイアウト・`WIRE_MAX_FRAMES`/`WIRE_MAX_CHANNELS`
  の強制ポイント（`Activate` の config 上限検査）— Task 4 の
  shm データ面で `staging` 確保と共に強制する（本 task の codec
  は宣言長の上限のみ強制 — frame/channel 数の意味的妥当性は
  shm slot layout 所有者が拒否する）。
- `InProcessHost` の挙動変更（generation 修正を除く — これは
  pre-existing 欠陥の修正であり挙動変更ではなく正しさの回復）・
  `HostCommand`/`HostReply` のコマンド面変更 — `pub use` で
  完全互換（既存パス不変）。
- `rt_l1_*` 命名の L1 RT 契約テスト — Task 4 の RT 脚で追加
  （本 task の `test-rt` 109 件は既存レーンの無退行確認）。
- `HostError::Failed` を実際に返す経路 — Task 3（L1 proxy が
  lifecycle mirror `Failed` のインスタンスへの非 `Destroy`
  コマンドを `Failed` で拒否する段階で初めて observable。
  本 task は variant + wire 符号化のみ供給）。

## Risks

- `generations: Vec<u16>` の世代 wrap は `u16` 周回後に理論上
  stale handle と同値になり得る（65,536 回の churn が必要 —
  実用上不可達だが、Task 5 が unified handle space で同規則を
  引き継ぐため設計判断として認識済み）。
- `Descriptor::matches` の features 順序感度 — proxy が
  `Scan` 結果を spec と照合する際、同一集合でも列挙順が異なる
  子実装を拒否する。正しい照合契約として意図的だが、将来の
  plugin 供給側が順序を変えると false negative になり得る
  （follow-up 記録）。
- `Hello.capabilities` の無条件拒否は `WIRE_VERSION = 1` 限定
  の契約 — 将来 capability bit を定義する拡張は `WIRE_VERSION`
  bump と decode の同時更新が必要（module doc に明記済み）。
- wire codec は untrusted input 検証境界だが、shm slot 内の
  データ面（frame/channel 数・sample 配置）は Task 4 の shm
  モジュール責務 — codec 単体では shm 内容は検証しない
  （制御面のみ）。
- `gui_open` フィールド（host.rs `Instance`）は `open_gui`/
  `close_gui` で書込まれるが read されない dead field —
  L0 では GUI 状態を拒否条件に使わないため現状無害だが、
  L1 composite 化時に使途を決めるか除去が必要（follow-up）。

## Follow-ups

- Task 2 `docs/tasks/plugin-export-crate-and-l1-runner.md`
  （actions 5–8）— export クレート + runner が本 task の
  wire/`Plugin`/`ErasedPlugin` 契約を消費する最初の下游。
- Task 3 `docs/tasks/l1-proxy-control-plane.md`（actions 9–11）
  — `HostError::Failed`/`Ipc` の実経路・`Descriptor::matches`
  照合・lifecycle mirror `Failed` 注入が実装される。
- `gui_open` dead field — L1 composite（Task 5）で GUI surface
  の所有規則と共に使途を決定 or 除去（現状 write-only）。
- `HostError::Failed` mapping — `Plugin::open_gui` 等の hook
  失敗が `PluginFailed` を返す現行経路と、proxy が注入する
  `Failed` の語義差を Task 3 の wire `Command` 拒否経路で
  固定する。
- `WIRE_MAX_FRAMES`/`WIRE_MAX_CHANNELS` enforcement — codec は
  宣言長のみ検査し、config の frames/channels 意味的上限は
  Task 4 runner/proxy 側で `Prepare(UnsupportedConfig)` として
  強制する（plan action 12）。
- `Descriptor::matches` 順序感度 — features の集合等価が必要に
  なった場合の緩和は Task 3 proxy の照合仕様と共に再評価。
- `ruviform-audio-core` の cargo doc 警告 2 件 — 本 task 非起因
  の pre-existing issue（別途 triage 対象）。
- pre-existing host.rs generation defect — 本 task で修正済み・
  regression test 追加済み。独立記録として本 IR に残す
  （#74 由来・Task 5 前提のため前倒し修正）。
