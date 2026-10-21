---
id: "SPEC-034Jcr3dOT5Uxp4znj42II"
type: "spec"
status: "implemented"
title: "RT 基盤の実装（ruviform-rt プリミティブ・スレッド優先度機構・スナップショット交換）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/rt-foundation-api-design.md"
    - "../plans/implement-rt-foundation.md"
  depends-on:
    - "establish-rust-dev-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../../AGENTS.md"
  refines: []
  refined-by:
    - "../adr/rt-foundation-primitives-implementation.md"
  derives-from:
    - "../discovery/evaluate-rt-foundation-primitives.md"
  derived-by:
    - "../adr/rt-foundation-primitives-implementation.md"
    - "../designs/rt-foundation-api-design.md"
    - "../plans/implement-rt-foundation.md"
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

# RT 基盤の実装（ruviform-rt プリミティブ・スレッド優先度機構・スナップショット交換）

## What Should Be Built

GitHub Issue #21 で定義された RT 基盤実装の成果物として、`ruviform-rt` クレートに設計ベースライン「RT／非 RT の分離方式」節とクレート一覧 A.2 で確定した責務を公開 API として実装し、あわせて `ruviform-exec` にスレッド優先度機構を実装する。現行 ruviform-rt スケルトンは `RtContext` のプレースホルダー（`_private: ()`）と `version()` のみであり、本 spec で残りの API 面を確定する。

- **契約型**: `RtContext` capability（RT 契約区間の境界トークン。単一の opaque 型とし、公開コンストラクタを持たず（`_private` フィールドでリテラル構築を防止）、オフライン/RT を区別する実行時モードフラグや variant は持たない — 「同一 API 内部で if rt 分岐しない」原則を維持する。オフライン発行コンテキストでの制約緩和は実行時状態ではなく、発行した駆動者に紐づく契約・ドキュメントレベルの記述とする。発行は RT 駆動者・オフライン駆動者の入口点に限る `#[doc(hidden)]` mint 経路）と、`prepare`/`process` のライフサイクル分離を型で表す `Processor` + `RtProcessor` の二 trait 契約（associated types で接続）。mint API 自体の `**RT contract:**` ラベルは `RT-suitable`（契約区間開始時の純粋なトークン構築）。
- **SPSC チャネル**: RT ↔ 非 RT の有界・ノンブロッキング通知プリミティブ（RT → 非 RT: メータ・イベント・退避オブジェクト。非 RT → RT: パラメータ・制御コマンド）。`try_push`/`try_pop` 型の API。満杯時方針は汎用のリングレベル enum ではなく API 単位で表現し、`try_push` は満杯時に値を呼び出し元へ返却する（`TryPushError<T>` 型）ことで drop/coalesce/keep のポリシー適用を呼び出し側に委ねる。latest-wins のパラメータ適用には consumer 側の drain-latest ヘルパーを提供する。内部実装は `rtrb`。
- **遅延ドロップ（retire channel）**: RT パスで退役した重いオブジェクトを非 RT 側へ送出して drop する仕組み。SPSC プリミティブ上に構成し、drain/drop の実行は非 RT 側のみ。退役経路は満杯時も RT 側で drop/dealloc/blocking を行わない文書化されたオーバーフロー方針を持つ（事前確保済み予備容量、および/または `core::mem::forget` による leak-on-full フォールバック）。
- **不変スナップショット交換**: 非 RT 側で構築した構成・実行プランのスナップショットを atomic に交換し、RT 側がブロック境界でノンブロッキングに参照するプリミティブ。RT 側のスナップショット参照は専用 guard 型経由でのみ取得し、guard 解放時に内部 `Arc` を retire channel へ送出する（dealloc は非 RT drain 側でのみ発生）。書き込み側の atomic 差替も、追い出された旧 `Arc` を同一の退役チャネル経由で退役させる。この保証は `arc-swap` 単体ではなく guard + retire channel の組合せで実現する。内部実装（atomic セル）は `arc-swap`。
- **`MasterClock`**: A/V 同期の基準時計となる時刻共有プリミティブ。単一 `AtomicU64` の tick + 構築時に固定される timebase（以後不変のため tearing/seqlock は不要）で表現し、単一の論理 writer（現役の駆動者 — RT・オフラインいずれでも可）が tick を更新、読み取り側は wait-free。rt は最下位クレートのため時刻表現は rt 内の汎用表現（`MediaTime` 等 media-core 型への依存は持たない）。
- **スレッド優先度機構（`ruviform-exec`）**: RT スレッドへの OS 優先度付与（MMCSS/time-constraint/`SCHED_FIFO` 系）を **`ruviform-exec`** の `non-RT only` API として提供する。`ruviform-rt` には置かない — Primitives 層は「OS 固有 API・デバイス依存を持たない」（ベースライン L160）ため OS 優先度 syscall を行う本機構は Primitives 最下位の rt に置けず、`ruviform-exec` の責務行は「OS スレッドの詳細は隠蔽」を要求するため exec が所有する（詳細は ADR）。呼び出しモデルは対象スレッド自身からの自己 promotion: exec 管理スレッドでは spawn 時セットアップで、デバイスコールバック由来スレッド（将来の audio-device）では初回コールバックの preamble — RT 契約区間の開始前（`RtContext` mint・初回 `process` 呼び出し前）に呼ぶ。ブロッキング的な syscall を伴うため `non-RT only` であり、active な RT 契約区間内からの呼び出しは契約違反。promotion 失敗は致命的でなく best-effort の `Result` + エラー型で返す。内部実装は `audio_thread_priority`（`default-features = false` で `with_dbus`/`dep:dbus` 経路を排除）。この構成では、rtkit/`CAP_SYS_NICE`/`RLIMIT_RTPRIO` のない非特権の標準デスクトップ Linux では promotion が失敗し、API はその失敗を `Result` として返す — この挙動をドキュメント化する。dbus 経路を再有効化する intent 命名の opt-in feature の追加は将来のフォローアップ候補（依存クレート自身の feature 名の再露出は外部型非露出規則で禁止されるため、独自命名の feature とする）。
- **共通エラー型**: rt の公開 API が返す `Error`/`Result` 型。
- **ドキュメント・検証**: 全公開 API に `**RT contract:**` ラベル（`RT-suitable` / `non-RT only`）、公開 API の例は doctest で記述（AGENTS.md 規約。単体テストはこれに追加する形）、`rt_` プレフィックス命名の RT 契約テスト（`mise run test-rt` で選択可能。選択フィルタは部分文字列一致のため、非 RT のテスト・doctest 名は `rt_` 部分文字列を含まない命名規約とする）、README / AGENTS.md / ルート `Cargo.toml` コメント・各 crate-level doc の記述同期。

## Why Now

- 設計ベースライン「暫定実装ロードマップ」Phase 1 の完了目安（「データモデル（時刻・バッファ・フレーム・レイアウト）と **RT 契約型・プリミティブ**が API として成立し、単体テストが通る」）を構成する残件であり、「後続実装 Issue の提案一覧」の「RT 基盤 Issue（rt プリミティブの実装方式・スレッド優先度機構・スナップショット交換方式の評価）」として起票済み（Issue #21）。
- Issue #18 でデータモデル 3 クレートの API が確定済みで、Phase 1 基盤の残りは rt・math・exec のスケルトン。rt の契約型（`Processor`/`RtContext`）は engines・audio-device・exec・plugin-core すべての上位実装が依存する境界であり、未確定のままでは後続 Issue が着手できない。
- ベースライン「未決定事項の分類」の要PoC 行「スレッド優先度付与の具体機構」（L486）と「RT 通信・退避・スナップショット交換の実装方式・依存クレート」（L489、および採否節 L360 のスナップショット具体実装）は、本 Issue が次フォーラムに指定されており、評価は discovery（[evaluate-rt-foundation-primitives.md](../discovery/evaluate-rt-foundation-primitives.md)）で完了・ADR で決定済み（accepted）。方式が固まった今、API をコードに落とすのに適切なタイミング。

## Users and Value

- **engines / audio-device / exec の実装者**: `Processor`/`RtProcessor`・`RtContext`・`MasterClock`・スナップショット交換が確定し、RT パス駆動（デバイスコールバック配線・オフライン駆動・グラフ差替）の境界 API としてそのまま利用できる。
- **プロセッサ実装者（DSP・プラグイン）**: `prepare`/`process` 分離と `RtContext` の発行規則が型で表現され、`process` を契約区間外で呼べない構造になる。SPSC・retire channel により RT パスでの alloc/dealloc・ブロッキングを回避した通知・退役が可能。
- **レビュアー・後続 Issue の作業者**: API 単位の `**RT contract:**` ラベルと `rt_` テストにより、「何が RT-suitable か・何が非 RT 専用か」をコードと検証の両面で追跡できる。

## Scope

### In Scope

- `ruviform-rt` の公開 API 確定と実装:
  - 契約型: `RtContext`（単一の opaque capability で実行時モードフラグ/variant を持たない。`#[doc(hidden)]` mint API を駆動者入口点にのみ提供。mint API 自体は `RT-suitable` ラベル）と `Processor`/`RtProcessor` 二 trait（`prepare` → RT 処理オブジェクト生成 → `process(&mut self, io, ctx)` のライフサイクル分離）
  - SPSC チャネル（有界・ノンブロッキング・`try_push`/`try_pop` 型 API。満杯時は `try_push` が値を返す `TryPushError<T>` 型で呼び出し側の drop/coalesce/keep ポリシーに委譲 + latest-wins 用 consumer 側 drain-latest ヘルパー）
  - 遅延ドロップ retire channel（SPSC 上に構成・drain は非 RT 側のみ）
  - 事前確保型の受け渡し構造: チャネル容量は構築時に固定（以後の伸長なし）とし、退役経路は満杯時でも RT 側で drop/dealloc/blocking を行わない文書化された方針（事前確保済み予備容量、および/または `core::mem::forget` による leak-on-full フォールバック）を実装する
  - 不変スナップショット交換（atomic swap + RT 側参照は専用 guard 型経由でのみ取得。guard 解放と書き込み側差替の双方が退役 `Arc` を退役チャネルへ送出し、dealloc が非 RT drain でのみ発生する契約）
  - `MasterClock`（単一 `AtomicU64` tick + 構築時固定 timebase・単一論理 writer・wait-free 読み取り）
  - 共通 `Error`/`Result` 型
  - スレッド境界の契約: チャネルペイロードと共有 `Arc<T>` スナップショット内容に `Send`/`Sync` 境界（共有 `Arc<T>` では `T: Send + Sync`）を API 契約として明示
- `ruviform-exec` の公開 API 確定と実装:
  - スレッド優先度機構（`non-RT only`。`audio_thread_priority` の `default-features = false` 経路。対象スレッド自身から呼ぶ自己 promotion モデル — exec 管理スレッドでは spawn 時セットアップ、デバイスコールバック由来スレッドでは初回コールバック preamble の RT 契約区間開始前。promotion 失敗は best-effort の `Result` + エラー型。非特権 Linux での失敗挙動をドキュメント化）
- `Cargo.toml` への依存追加: `ruviform-rt` に `rtrb`・`arc-swap`、`ruviform-exec` に `audio_thread_priority`（`default-features = false`）。いずれも private 実装詳細（型・エラー・feature 名を公開 API に露出しない）
- 全公開 API の `**RT contract:**` ラベル・公開 API 例の doctest・単体テスト、`rt_` プレフィックスの RT 契約テスト（命名規約は上記）
- README / AGENTS.md / ルート `Cargo.toml` コメント・crate-level doc（責務・Allowed dependency layers・RT contract・Contract source）の実装済み状態への同期

### Out of Scope

- `assert_no_alloc` 型 CI レーン・スレッド契約検査（thread-check）機構 — RT 検証 Issue（ベースライン分類表 L487-488 の次フォーラム）の責務（本 spec 内では、allocation 検査が必要な箇所に限り std のみのテストスコープ counting `#[global_allocator]` ヘルパーを検証手段として利用可）
- `ruviform-rt` へのスレッド優先度機構の配置 — Primitives 層の「OS 固有 API・デバイス依存を持たない」制約（ベースライン L160）に抵触するため `ruviform-exec` に配置する。再評価条件は Deferred Design Concerns を参照
- `audio_thread_priority` の dbus 経路を再有効化する feature の追加 — `default-features = false` 固定（`with_dbus`/`dep:dbus` 排除）。intent 命名の opt-in feature は将来のフォローアップ候補（Deferred Design Concerns 参照）
- 処理グラフの実行エンジン・pull スケジューリング・グラフ再コンパイル — engines 系 Issue の責務（本 spec は交換・通知のプリミティブのみ）
- `clack-extensions` thread-pool 等のプラグイン補助スレッド基盤 — プラグイン基盤 Issue
- オフライン決定論的実行の厳密保証範囲 — 後続 Issue（オフライン実行 Issue）
- `ruviform-rt` から他 ruviform クレート（media-core 等）への依存 — rt は最下位のため禁止
- スケジューリング・I/O を `ruviform-rt` に持つこと（クレート責務の非責務として確定済み）
- 採用依存クレートの型・エラー・feature 名を公開 API に露出すること（直接の規則源はベースライン クレート一覧 A.2 L190 の `ruviform-rt` 行「外部同期クレート（rtrb 等）の型は露出しない」。media-io ADR と同型の外部型非露出規則を rt・exec にも適用）
- `ringbuf`・`basedrop` 等の非採用クレートの再評価（再評価条件は Deferred Design Concerns を参照）

## Requirements

- Rust edition 2024・MSRV 1.85・stable のみ。`publish = false`（workspace 継承）、`[lints] workspace = true`、std と採用依存のみで構成
- `unsafe` は原則禁止。採用クレートの内部で unsafe が使われても rt 側の公開・実装コードは safe とし、例外的に必要な場合のみ `#[expect(unsafe_code)]` + SAFETY コメントの局所許可で対処する
- 採用依存（`ruviform-rt`: `rtrb`・`arc-swap`。`ruviform-exec`: `audio_thread_priority` `default-features = false`）は private 実装詳細とし、公開 API のシグネチャ・公開エラー・feature 名に外部型を一切露出しない（AGENTS.md 依存追加規約の確認事項 — メンテナンス・ライセンス・依存木サイズ — は ADR の Consequences に記録済み）
- 全公開 enum に enum レベル `#[non_exhaustive]` を付与（[public-enum-non-exhaustive-policy](../adr/public-enum-non-exhaustive-policy.md) 準拠）
- 全公開 API に `**RT contract:**` ラベル（`RT-suitable` / `non-RT only`）を付し、公開 API の例は doctest で記述する（AGENTS.md「公開 API の例は doctest に書く」規約）。単体テストは doctest に追加する形でカバレッジを持たせる
- RT 契約を検証するテスト（SPSC のノンブロッキング性・retire drain が非 RT 側で実行されること・スナップショット退役の dealloc 配置・退役経路の満杯時方針等）は `rt_` プレフィックスで命名し `mise run test-rt` で選択可能にする。選択フィルタは部分文字列一致のため、非 RT のテスト・doctest 名は `rt_` 部分文字列を含まない命名規約とする（Issue #18 test-spec の先例）
- 値の確保・伸長を伴う構築 API（チャネル作成・スナップショット生成・優先度付与等）は `non-RT only` とラベルし、RT 側 API（`try_push`/`try_pop`・guard 取得・時計読み取り・tick 等）は alloc/dealloc なし・ノンブロッキングの設計とする。`rt_` テストでノンブロッキング性と no-alloc を挙動として検証し、allocation 不使用の検査が必要な箇所では std のみのテストスコープ counting `#[global_allocator]`（外部依存なし）を使用してよい（`assert_no_alloc` CI レーン自体はスコープ外 — RT 検証 Issue）

## Acceptance Criteria

Issue #21・評価 discovery・採否 ADR の決定事項に基づく。

- [ ] `RtContext` は単一の opaque 型で公開コンストラクタ・公開フィールドを持たず（`_private` がリテラル構築を防止）、実行時のオフライン/RT モードフラグや variant を持たない。mint は `#[doc(hidden)]` で駆動者入口点向けに提供される API 経由でのみ到達可能であり、駆動者専用利用が契約規約 + コードレビューで強制される（型レベルでの構築完全禁止は保証しない）。mint API の `**RT contract:**` ラベルは `RT-suitable`
- [ ] `Processor`（非 RT の `prepare`）と `RtProcessor`（RT の `process`）の二 trait 分離が存在し、`process` が `RtContext` を要求するシグネチャで契約区間外の実行を型で抑止している
- [ ] SPSC チャネルが有界・ノンブロッキングの `try_push`/`try_pop` API を持ち、満杯時に `try_push` が値を呼び出し元へ返却する（`TryPushError<T>` 型）ことで drop/coalesce/keep のポリシー適用が呼び出し側に委ねられ、latest-wins 適用向けの consumer 側 drain-latest ヘルパーが API ドキュメントで定義されている
- [ ] 遅延ドロップ retire channel があり、退役オブジェクトの `Drop` が非 RT 側の drain でのみ実行されることがテストで検証されている
- [ ] 退役経路は満杯時でも RT 側で drop/dealloc/blocking を行わない（事前確保済み予備容量または文書化された leak-on-full 方針）ことが API ドキュメントで定義されテストで検証されている。なお leak-on-full はリーク総量の上限を保証しない（飽和は `overflows()` カウンタで観測可能とし、容量・drain cadence の調整は運用側契約とする）
- [ ] RT 側スナップショット参照は専用 guard 型経由で取得され、guard 解放・書き込み側差替の双方が退役チャネルへ送出し、dealloc が非 RT drain 側でのみ発生する設計が API 契約と `rt_` テストで確認できる
- [ ] `MasterClock` が単一 `AtomicU64` の tick + 構築時固定の timebase で表現され、単一の論理 writer（RT またはオフラインの現役駆動者）による tick 更新と wait-free 読み取りを提供する
- [ ] チャネルペイロードと共有 `Arc<T>` スナップショット内容に `Send`/`Sync` 境界（共有 `Arc<T>` では `T: Send + Sync`）が API 契約で明示されている
- [ ] スレッド優先度 API が `ruviform-exec` に `non-RT only` とラベルされて提供され、対象スレッド自身からの呼び出しモデル（exec 管理スレッド: spawn 時セットアップ／デバイスコールバック由来スレッド: RT 契約区間開始前の初回コールバック preamble）と、promotion 失敗がパニックでなく `Result` + エラー型で表出されることがドキュメント・テストで確認できる。`default-features = false` 構成で非特権の標準デスクトップ Linux 上の promotion 失敗が `Result` として返る挙動が文書化されている
- [ ] 共通 `Error`/`Result` 型が公開 API として存在し、rt の公開 API がそれを返す
- [ ] 全公開 enum に `#[non_exhaustive]` が付与され、全公開 API に `**RT contract:**` ラベルと公開 API 例の doctest + 単体テストが存在する
- [ ] `rt_` 命名の RT 契約テストがノンブロッキング性・no-alloc を挙動として検証する（allocation 検査が必要な箇所は std のみのテストスコープ counting `#[global_allocator]` を使用可。`assert_no_alloc` 型 CI レーンは本 spec スコープ外）
- [ ] `rtrb`・`arc-swap` が `ruviform-rt` の、`audio_thread_priority`（`default-features = false`）が `ruviform-exec` の `Cargo.toml` に追加され、外部型が公開 API に露出していない（`cargo metadata` で依存リストを、`cargo doc` / 公開シグネチャレビューで型の非露出を確認可能）
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` がグリーン（`rt_` レーンが意図した RT 契約テストのみを選択。非 RT テスト・doctest 名が `rt_` 部分文字列を含まない命名規約が守られている）
- [ ] README / AGENTS.md / ルート `Cargo.toml` コメント / crate-level doc が実装済み状態と一致している

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `assert_no_alloc` 型 CI 検査・thread-check 機構 | 本 spec では検証レーンを構築しない（`rt_` 命名のテスト配置のみ） | RT 検証 Issue（ベースライン分類表 L487-488） | RT 契約の機械検証が遅れ、違反の混入が症状化まで遅れる |
| `ringbuf` への切替 | `rtrb` 採用。ringbuf は不採用だが再評価経路を残す | rtrb が要件（SPSC・依存木・保守）を満たせなくなった時点で実装 Issue で再評価 | 依存の不整合が発生した場合の代替手順が不明確になる |
| スレッド優先度 promotion 失敗の per-OS semantics 詳細 | API は `Result` + エラー型で表出する形状のみ確定。MMCSS/time-constraint/SCHED_FIFO の実効差・パラメータ写像の詳細はドキュメントと実機検証に委ねる | オーディオデバイス実装 Issue・実機検証 | OS 差を考慮しないパラメータ設計を早期に固定する |
| スレッド優先度機構のクレート配置 | `ruviform-exec` に配置（Primitives 層の「OS 固有 API・デバイス依存を持たない」L160 との整合。rt の依存を `rtrb`+`arc-swap` のみに維持）。機構自体は配置に関わらず本 Issue のスコープ | 将来の層（audio-device 等）がより下位の配置を必要とした場合に ADR で再評価 | 配置を巡る再議論が後続 Issue で無秩序に再燃する |
| `audio_thread_priority` の dbus 経路再有効化 | `default-features = false` 固定（`with_dbus`/`dep:dbus` 排除）。rtkit/`CAP_SYS_NICE`/`RLIMIT_RTPRIO` のない非特権デスクトップ Linux では promotion が失敗し `Result` で返る挙動をドキュメント化 | dbus 経路を再有効化する intent 命名の opt-in feature を将来のフォローアップとして検討（依存 feature 名の再露出は外部型非露出規則で禁止のため独自命名とする） | 非特権 Linux 利用者が RT 優先度を得る手段が恒久的に失われる |
| `basedrop` 系の再評価 | SPSC 上の自前 retire channel を採用。basedrop は不採用 | retire channel の満杯時設計が複雑化した場合に ADR で再評価 | 自前機構が設計要件を満たせない場合の切替基準が曖昧になる |
| オフライン決定論的実行の保証範囲 | `RtContext` のオフライン発行経路の形状のみ確定。決定論モードの保証は持たない | オフライン実行 Issue（ベースライン保留項目） | 再現性保証を暗黙に期待した実装が混入する |
| `RtContext` mint 権限の将来拡張 | doc-hidden mint API で現行 2 駆動者に対応。capability factory 型は未採用 | mint 権限を持つ駆動者が増えた時点で ADR 再評価 | 発行権の追跡可能性が不足する |

## Sources and Relations

- 一次ソース: GitHub Issue [#21 [impl] RT 基盤（rt プリミティブ・スレッド優先度機構・スナップショット交換）](https://github.com/scarecrow173/ruviform/issues/21)（`relations.source`）。Issue #1 設計ベースライン「後続実装 Issue の提案一覧」L571 から起票された。
- 上流文書（`relations.derives-from`）: 評価 discovery [evaluate-rt-foundation-primitives.md](../discovery/evaluate-rt-foundation-primitives.md)（`DISC-034JcqEc2b3EHNgH9CRLaC`）。実装方式の検討経路とクレート評価の一次記録。
- 前提（`relations.depends-on`）: spec [establish-rust-dev-foundation.md](establish-rust-dev-foundation.md)（`SPEC-034IcwhwIFSNUNKtX7KK0Q`）が確立した Rust 開発基盤（workspace・lint・mise 検証タスク・`test-rt` レーン）を本 spec の検証基盤として利用する。
- 関連（`relations.related`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（クレート一覧 A.2 の `ruviform-rt` 行・「RT／非 RT の分離方式」・「RT 契約」・未決定事項分類 L486/L489）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（API 単位の保証ラベル付け規則）、リポジトリルートの [`AGENTS.md`](../../AGENTS.md)（暫定開発規約・unsafe 原則禁止・依存追加規約）。
- 派生（`relations.derived-by` / `relations.refined-by`）: 採否 ADR [rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md)（`ADR-034Jcra57UlCiVE1Poq2DO`）が依存クレート・mint 方式・trait 分離の決定を記録し本 spec を refine する。実装 plan は後続で `implemented-by`、test-spec は `verified-by` で接続予定。
- 参考（`relations.references`）: 親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（設計ベースラインの一次ソース）。
