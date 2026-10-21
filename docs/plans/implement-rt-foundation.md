---
id: "PLAN-034JxNegJktSAJZ2LuO0N8"
type: "plan"
status: "completed"
title: "Issue #21 RT 基盤の実装（ruviform-rt プリミティブ・ruviform-exec スレッド優先度機構）"
created: "2026-10-21"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/error.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/context.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/processor.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/spsc.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/retire.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/snapshot.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/clock.rs"
      - type: "file"
        path: "crates/ruviform-exec/src/thread_priority.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-rt/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-exec/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-exec/Cargo.toml"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/implement-rt-foundation.md"
  implemented-by:
    - "../tasks/implement-rt-core-types.md"
    - "../tasks/implement-rt-spsc-channel.md"
    - "../tasks/implement-rt-retire-channel.md"
    - "../tasks/implement-rt-snapshot-exchange.md"
    - "../tasks/implement-exec-thread-priority.md"
    - "../tasks/sync-rt-foundation-docs-and-verify.md"
  depends-on:
    - "dev-foundation-bootstrap.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../../mise.toml"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/implement-rt-foundation.md"
    - "../designs/rt-foundation-api-design.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/rt-foundation-primitives-implementation.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/rt-channel-and-snapshot-contracts.md"
    - "../test-specs/rt-context-and-processor-contract.md"
    - "../test-specs/thread-priority-lifecycle.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# Issue #21 RT 基盤の実装（ruviform-rt プリミティブ・ruviform-exec スレッド優先度機構） Implementation Plan

## Goal

approved spec `SPEC-034Jcr3dOT5Uxp4znj42II` と approved design `DESIGN-034JgsK9khgDsPqG04Sp6i` の確定事項を、`ruviform-rt`（契約型・SPSC・退役チャネル・スナップショット交換・`MasterClock`・共通エラー）と `ruviform-exec`（スレッド優先度機構）の公開 API として実装・検証し、Issue #21 を完了する。

## Scope

単一の実装ストリームとして扱う。両クレートの変更は「Issue #21 RT 基盤」という同一リリース意図・同一クリティカルパス上にある（rt 内部で snapshot → retire → spsc の実装依存が直列し、exec の優先度機構は同一 Issue の成果物としてドキュメント同期・総合検証を共有する）。範囲外（spec Out of Scope と一致）: `assert_no_alloc` 型 CI レーン・thread-check 機構（RT 検証 Issue の責務）、`ruviform-rt` への優先度機構配置、dbus 経路を再有効化する feature、処理グラフ実行エンジン、プラグイン補助スレッド基盤、オフライン決定論的実行の保証、rt から他 ruviform クレートへの依存、採用依存の型・エラー・feature 名の公開露出、`ringbuf`・`basedrop` の再評価、複数 RT reader のスナップショット。

## Upstream Documents

- [../specs/implement-rt-foundation.md](../specs/implement-rt-foundation.md) — `SPEC-034Jcr3dOT5Uxp4znj42II`（approved）。本 plan が実装する spec。Issue #21 の範囲・要件・受け入れ基準の正本
- [GitHub Issue #21](https://github.com/scarecrow173/ruviform/issues/21) — 一次ソース（`relations.source`）

## Design Inputs

- [../designs/rt-foundation-api-design.md](../designs/rt-foundation-api-design.md) — `DESIGN-034JgsK9khgDsPqG04Sp6i`（approved）。API sketch・モジュールマップ・不変条件・テスト戦略の正本（本 plan が分解する実装入力契約）
- [../adr/rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md) — `ADR-034Jcra57UlCiVE1Poq2DO`（accepted）。依存採否（`rtrb`・`arc-swap`・`audio_thread_priority` `default-features = false`）・doc-hidden mint・二 trait 分離・満杯時 `mem::forget` 方針・retire channel 自前構成の決定記録
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT 契約 ADR（accepted）。API 単位の `**RT contract:**` ラベル付け規則・検証分担（CI ≠ 完全証明）
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — 設計ベースライン（approved）。クレート表 A.2 の `ruviform-rt`/`ruviform-exec` 行・「RT／非 RT の分離方式」節の根拠

## File Map

### `ruviform-rt`（private `mod` + フラット `pub use`。media-core と同型）

- `crates/ruviform-rt/src/error.rs` — 共通 `Error { InvalidValue(&'static str) }`（enum レベル `#[non_exhaustive]`・`Copy`・alloc なし）+ `Result<T>` エイリアス
- `crates/ruviform-rt/src/context.rs` — `RtContext`（`_private` でリテラル構築防止・実行時モードフラグ/variant なし・`#[doc(hidden)] mint`）
- `crates/ruviform-rt/src/processor.rs` — `Processor`（assoc `Setup`/`Rt`/`Error`・`prepare` は non-RT only）+ `RtProcessor`（assoc `Io`・infallible `process(&mut self, io, ctx)`）
- `crates/ruviform-rt/src/spsc.rs` — `channel`/`Producer`/`Consumer`/`TryPushError`/`TryPopError`（`rtrb` ラップ。`pop_latest` 含む）
- `crates/ruviform-rt/src/retire.rs` — `retire_channel`/`Retirer`/`DropCollector`（SPSC 上に構成。満杯時 `mem::forget` bounded-leak・Vec pending・`pub(crate) push_local`）
- `crates/ruviform-rt/src/snapshot.rs` — `slot`/`SnapshotWriter`/`SnapshotReader`/`SnapshotGuard`（`arc-swap` + reader 内 `RefCell<Retirer<Arc<T>>>`）
- `crates/ruviform-rt/src/clock.rs` — `MasterClock`（単一 `AtomicU64` tick + 構築時固定 `(u32, u32)` timebase）
- `crates/ruviform-rt/src/lib.rs` — private `mod` 宣言 + `pub use` フラット再公開 + crate doc の実装済み化（"Phase 1 skeleton"・"Placeholder for Issue #17" 注記の除去）+ `version()` 維持（`RT-suitable` ラベル付与）
- `crates/ruviform-rt/Cargo.toml` — `rtrb = "0.4"`・`arc-swap = "1"` 追加（private 実装詳細）

### `ruviform-exec`

- `crates/ruviform-exec/src/thread_priority.rs` — `promote_current_thread_to_realtime`/`demote_current_thread_from_realtime`/`ThreadPriorityParams`/`ThreadPriorityGuard`/`PriorityError`（`audio_thread_priority` ラップ）
- `crates/ruviform-exec/src/lib.rs` — private `mod` + `pub use` フラット再公開 + crate doc へ thread_priority 責務追加（「RT プリミティブは持たない」記述は維持）
- `crates/ruviform-exec/Cargo.toml` — `audio_thread_priority = { version = "0.38", default-features = false }` 追加（`with_dbus`/`dep:dbus` 排除）

### ドキュメント同期

- `Cargo.toml` — members コメントの「Phase 1 基盤スケルトン」記述更新
- `README.md` — rt/exec のスケルトン記述更新（クレート一覧・ディレクトリ表）
- `AGENTS.md` — 「math・rt・exec は引き続きスケルトン」記述の実装済み状態への同期
- `Cargo.lock` — 新規依存（`rtrb`・`arc-swap`・`audio_thread_priority`）の記録

## Implementation Sequence

### Phase A: ruviform-rt 中核型（タスク 1）

- [ ] A-1. `error.rs`: 共通 `Error`/`Result`（`#[non_exhaustive]`・`Copy`・`Display` + `core::error::Error`）
- [ ] A-2. `context.rs`: `RtContext`（`_private`・`#[doc(hidden)] mint` を `RT-suitable` ラベル付きで提供）
- [ ] A-3. `processor.rs`: `Processor`/`RtProcessor` 二 trait（associated type・`process` が `&RtContext` を要求・infallible）
- [ ] A-4. `clock.rs`: `MasterClock`（AtomicU64・timebase 構築時固定・0 成分は `Err(InvalidValue("timebase"))`・Acquire/Release）
- [ ] A-5. `lib.rs`: private mod + フラット再公開 + crate doc 更新 + `rt_` ライフサイクル/時計テスト。依存追加なし

### Phase B: SPSC チャネル（タスク 2、Phase A 依存）

- [ ] B-1. `Cargo.toml` に `rtrb = "0.4"` 追加
- [ ] B-2. `spsc.rs`: `channel`/`Producer`/`Consumer`（`try_push`・`try_pop`・`pop_latest`・診断アクセサ）+ `TryPushError<T>`/`TryPopError`（`#[non_exhaustive]`・`Display`+`Error`）
- [ ] B-3. `rt_` テスト: FIFO・満杯時 `Full(T)` 値返却・`pop_latest` latest-wins・counting allocator で no-alloc

### Phase C: 退役チャネル（タスク 3、Phase B 依存）

- [ ] C-1. `retire.rs`: `retire_channel`/`Retirer`/`DropCollector`（spsc 上に構成。`retire` は `try_push` 意味 + 満杯時 `mem::forget`。`DropCollector` は Vec pending + `pub(crate) push_local` seam を持ち drain で一括 drop）
- [ ] C-2. `rt_` テスト: Drop-flag サイドチャネルで「retire 後は未 drop → drain でのみ drop」「満杯時は forget で flag が立たない」を検証

### Phase D: スナップショット交換（タスク 4、Phase C 依存）

- [ ] D-1. `Cargo.toml` に `arc-swap = "1"` 追加
- [ ] D-2. `snapshot.rs`: `slot`/`SnapshotWriter`/`SnapshotReader`/`SnapshotGuard`（`ArcSwap` + reader 内 `RefCell<Retirer<Arc<T>>>`。guard `Drop` は `retire` へ送出・`publish` の追い出し Arc は `push_local` で同一 drain 経路へ）
- [ ] D-3. `rt_` テスト: publish 可視性・guard `Drop` の退役ルーティング（strong_count 観測）・追い出し Arc の collect 解放・`read`/guard drop の no-alloc・複数 guard 同時存在

### Phase E: exec スレッド優先度機構（タスク 5、Phase A のみ依存。Phase B–D と並行可能）

- [ ] E-1. `Cargo.toml` に `audio_thread_priority = { version = "0.38", default-features = false }` 追加
- [ ] E-2. `thread_priority.rs`: `ThreadPriorityParams`/`promote_*`/`demote_*`/`ThreadPriorityGuard`/`PriorityError`（`#[non_exhaustive]`）
- [ ] E-3. `lib.rs`: private mod + `pub use` + crate doc 更新。promotion スモークテスト（`rt_` を含まない命名）

### Phase F: ドキュメント同期・総合検証（タスク 6、Phase D・E 依存）

- [ ] F-1. 両 `lib.rs` crate doc 最終化・`AGENTS.md`/`README.md`/ルート `Cargo.toml` コメントのスケルトン記述同期
- [ ] F-2. 規約監査: 全公開 enum の `#[non_exhaustive]` grep・非 RT テスト/doctest/ヘルパー名の `rt_` 部分文字列非含有・全公開 API の `**RT contract:**` ラベル・外部型非露出
- [ ] F-3. 検証スイート実行（Verification 節の全コマンド）

## Dependencies and Risks

### Dependency Graph

| 依存元 | 依存先 | 内容 |
| --- | --- | --- |
| タスク 2（spsc） | タスク 1（中核型） | `lib.rs` のモジュール配線・`Error`/`Result`・命名規約の確定が先行 |
| タスク 3（retire） | タスク 2（spsc） | 退役チャネルは spsc プリミティブ上に構成。`Cargo.toml` の `rtrb` 追加も共有 |
| タスク 4（snapshot） | タスク 3（retire） | guard 解放・publish 追い出しの退役経路は `DropCollector`/`push_local` seam に依存 |
| タスク 5（exec 優先度） | タスク 1（中核型） | 規約（RT ラベル・enum 方針・命名）の共有のみ。実装依存はなくタスク 2–4 と並行可能 |
| タスク 6（同期・検証） | タスク 4・タスク 5 | 全 API 面確定後にドキュメント同期と総合検証を実施 |
| 本 plan | `dev-foundation-bootstrap.md` | mise 検証タスク・`test-rt` レーン・workspace lints の確立済みを前提 |

### Risk Register

| リスク | 影響 | 確率 | 対応 |
| --- | --- | --- | --- |
| 新規外部依存の保守・ライセンス・依存木サイズ | `rtrb` 0.4・`arc-swap` 1・`audio_thread_priority` 0.38 の採用自体 | 低 | ADR の Consequences に確認済み。AGENTS.md 依存追加規約の理由記録は ADR が担う。差替時は test-spec の契約を再検証 |
| 非特権 Linux での promotion 失敗 | `promote` が `Err` を返す（仕様上の失敗） | 高（同環境では確実） | `Result` 返却を仕様として文書化。テストは `Ok`/`Err` いずれも受理しパニックしないことのみ検証（成否を assert しない） |
| `rt_` 部分文字列フィルタの誤選択 | 非 RT テスト・doctest が `mise run test-rt` レーンに混入 | 中 | 命名規約をタスク完了条件化（`assert_*`/`start_*` 禁止 → `expect_*`/`spawn_*` 等）。タスク 6 で選択集合と grep を監査。Issue #18 で `convert_exact` の誤検出先例あり |
| reader 側 last-drop の dealloc 漏れ | `ArcSwap::load_full` の `Arc` を生で返すと最終 drop が RT スレッドで走りうる | 設計で対策済み | `SnapshotGuard` 経由参照のみ + guard `Drop` の退役ルーティングを中核契約として `rt_` テストで検証 |
| `RefCell` による `!Sync` | `SnapshotReader` を複数スレッドで共有不可 | 低 | 単一 RT reader（retire channel が SPSC）の前提を API 契約・doc に明示。複数 reader は Out of Scope |
| `Cargo.lock` / publish 設定の不整合 | 新規依存で lockfile が変わる | 低 | `Cargo.lock` は追跡対象のまま更新。`cargo metadata` で全パッケージ `publish = []` を検証 |
| 外部型の公開露出 | `rtrb`/`arc-swap`/`audio_thread_priority` の型・エラー・feature 名が公開 API に混入 | 低 | シグネチャレビュー + `cargo doc` 目視をタスク 6 の監査項目化。依存 feature 名の再露出となる独自 feature は追加しない |

## Task Breakdown

| Task | 内容 | 対応 Phase |
| --- | --- | --- |
| [implement-rt-core-types.md](../tasks/implement-rt-core-types.md) | ruviform-rt の error/context/processor/clock 中核型 + lib.rs 配線 | A |
| [implement-rt-spsc-channel.md](../tasks/implement-rt-spsc-channel.md) | spsc.rs + `rtrb` 依存 + `rt_` チャネルテスト | B |
| [implement-rt-retire-channel.md](../tasks/implement-rt-retire-channel.md) | retire.rs（mem::forget・`push_local` seam）+ `rt_` 退役テスト | C |
| [implement-rt-snapshot-exchange.md](../tasks/implement-rt-snapshot-exchange.md) | snapshot.rs + `arc-swap` 依存 + `rt_` ガード/可視性テスト | D |
| [implement-exec-thread-priority.md](../tasks/implement-exec-thread-priority.md) | exec thread_priority.rs + `audio_thread_priority` 依存 + スモークテスト | E |
| [sync-rt-foundation-docs-and-verify.md](../tasks/sync-rt-foundation-docs-and-verify.md) | ドキュメント同期・規約監査・総合検証 | F |

## Verification

| ID | 対象 | 検証コマンド / 確認 |
| --- | --- | --- |
| V-1 | タスク 1（rt 中核型） | `mise exec -- cargo test -p ruviform-rt`・`mise exec -- cargo test -p ruviform-rt rt_`・`mise exec -- cargo clippy -p ruviform-rt --all-targets -- -D warnings` |
| V-2 | タスク 2–4（rt プリミティブ） | `mise exec -- cargo test -p ruviform-rt`・`mise exec -- cargo test -p ruviform-rt rt_`・`mise run lint` |
| V-3 | タスク 5（exec 優先度） | `mise exec -- cargo test -p ruviform-exec`・`mise exec -- cargo clippy -p ruviform-exec --all-targets -- -D warnings`・`mise exec -- cargo test --workspace rt_` でスモークテストが非選択であること |
| V-4 | タスク 6（ワークスペース全体） | `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt`・`mise run bench`・`mise exec -- prek run --all-files`・`mise exec -- cargo metadata --format-version 1 --no-deps`（全パッケージ `publish` が `[]`・依存リストに `rtrb`/`arc-swap`/`audio_thread_priority`） |
| V-5 | 外部型非露出 | `cargo doc` / 公開シグネチャレビューで `rtrb`・`arc_swap`・`audio_thread_priority` の型名・エラー名・feature 名が公開 API に出ないこと |

振る舞いレベルの保証（SPSC・退役・スナップショットの RT 契約、契約型・時計、優先度ライフサイクル）は test spec 群（[rt-channel-and-snapshot-contracts](../test-specs/rt-channel-and-snapshot-contracts.md)・[rt-context-and-processor-contract](../test-specs/rt-context-and-processor-contract.md)・[thread-priority-lifecycle](../test-specs/thread-priority-lifecycle.md)）に委譲し、ここでは step ごとの検証入口のみを管理する。

## Rollback Strategy

| 対象 | 戻し方 | 影響 | 備考 |
| --- | --- | --- | --- |
| 本変更全体 | `git revert`（feature ブランチの変更は全て追加的コード — 既存 API の破壊なし） | 両クレートがスケルトン相当へ復帰し依存・ドキュメント記述も巻き戻る | スケルトンに対する純追加のためクリーンに revert 可能 |
| 部分的戻し | 依存順を考慮したファイル単位 revert | snapshot は retire の `push_local` seam に依存するため、retire を残して snapshot だけ戻す場合は seam の扱いに注意。タスク 4→3→2→1 の逆順でのみ安全 | 推奨は全体 revert |

## Open Questions / Gaps

- `promote_current_thread_to_realtime` の成否はプラットフォーム依存（`default-features = false` 構成で非特権 Linux では典型的に `Err`）。テストは `Result` がパニックなく返ることのみ検証し、実効差（MMCSS/time-constraint/`SCHED_FIFO`）の計量は spec の Deferred Design Concerns どおり実機検証・audio-device Issue に委ねる
- `#[doc(hidden)] mint` の悪用抑止は「doc-hidden + 駆動者限定規約 + コードレビュー」であり型レベルの完全抑止ではない（ADR がベースライン文言の弱体化として記録済み）
- `mise run test-rt` の substring フィルタ精度は既知の課題（Issue #18 で `convert_exact` doctest が `rt_` を内包し誤選択された先例）。フィルタ方式の改善自体は本 plan の範囲外
- `DropCollector` のローカル pending `Vec` は `push_local` 投入時に非 RT 側で伸長しうる（`publish` 経路のみ使用・non-RT only 契約内で許容）

## Self-Review

- [x] spec の Requirements・受け入れ基準（契約型・SPSC・retire・snapshot・MasterClock・優先度・Error・Send/Sync・ラベル/doctest・`rt_` 命名・依存追加・検証・ドキュメント同期）が Phase A–F・V-1〜V-5・6 タスクに対応している
- [x] design の API sketch・不変条件 1–13 と ADR の決定事項（依存採否・mint 方式・mem::forget・自前 retire）が実装・検証に反映されている
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は spec の Deferred Design Concerns と Open Questions / Gaps に記録）
- [x] ファイルパス・relation・status・用語が spec/design/タスクと整合している

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（タスク 2–4 の直列チェーンとタスク 5 の独立ストリームに分割可能）
- Independent work streams: タスク 5 はタスク 1 完了後にタスク 2–4 と並行可能。タスク 2→3→4 は直列。タスク 6 は全実装完了後の直列フェーズ
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 上流参照: spec `SPEC-034Jcr3dOT5Uxp4znj42II`（要件正本）・design `DESIGN-034JgsK9khgDsPqG04Sp6i`（API 形状正本）・ADR `ADR-034Jcra57UlCiVE1Poq2DO`（方式決定）・RT 契約 ADR（ラベル規則）・Issue #21・親 Issue #1
- レビュー焦点: guard `Drop` の退役ルーティング（reader 側 last-drop の穴の塞ぎ方）・満杯時 `mem::forget` 方針の doc 記述・`rt_` 命名規約の浸透（ヘルパー関数名を含む）・外部型非露出の確認方法
- 明示的仮定: 単一 RT reader・単一論理 writer・発行駆動者 2 箇所（RT 駆動者 + オフライン駆動者）の現行前提は design/ADR 確定事項どおり
