---
id: "ADR-034Jcra57UlCiVE1Poq2DO"
type: "adr"
status: "accepted"
title: "RT 基盤プリミティブの実装方式（SPSC・退避ドロップ・スナップショット交換・スレッド優先度・契約型）"
created: "2026-10-20"
updated: "2026-10-24"
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
  implemented-by: []
  depends-on:
    - "realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "realtime-contract-and-verification-policy.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../specs/implement-rt-foundation.md"
    - "../../AGENTS.md"
  refines:
    - "../specs/implement-rt-foundation.md"
  refined-by: []
  derives-from:
    - "../discovery/evaluate-rt-foundation-primitives.md"
    - "../specs/implement-rt-foundation.md"
  derived-by:
    - "../designs/rt-foundation-api-design.md"
    - "../plans/implement-rt-foundation.md"
    - "../designs/shared-foundations-and-modularization-policy.md"
    - "../designs/foundation-observability-minimal-api.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# RT 基盤プリミティブの実装方式（SPSC・退避ドロップ・スナップショット交換・スレッド優先度・契約型）

## Context and Problem Statement

GitHub Issue #21（RT 基盤実装）の決定として、`ruviform-rt` が提供する RT プリミティブの実装方式と依存クレート採否、およびスレッド優先度機構の配置先クレートを定める。設計ベースラインでは、RT ↔ 非 RT 通信の構造（SPSC リング・Drop 退避・不変スナップショット交換・`MasterClock`・`RtContext` capability・`Processor` の prepare/process 分離）は「決定」済みだが、実装方式・依存クレートが「要PoC」として残されていた（未決定事項分類の L486「スレッド優先度付与の具体機構」・L489「RT 通信・退避・スナップショット交換の実装方式・依存クレート」、および L360「スナップショット交換の具体実装（arc-swap 型/RCU 型）」）。評価 discovery（`DISC-034JcqEc2b3EHNgH9CRLaC`）が候補比較を完了しており、本 ADR はその結論を決定に昇華する。`ruviform-rt` は最下位クレート（他 ruviform クレートに依存不可・外部型を公開 API に露出不可）であり、採用方式が上位全層の RT 実装の土台となるため、採否は ADR として記録する。

## Decision Drivers

* ワークスペースの `unsafe` 原則禁止規約（SAFETY コメント付き局所許可のみ例外）— 手組み lock-free 実装は `UnsafeCell`/atomics で `unsafe` が必須となり規約と衝突する
* AGENTS.md の依存追加規約 — メンテナンス状況・ライセンス・依存木サイズの確認と理由記録が必須
* RT 契約（RT ADR）— RT 側の操作は alloc/dealloc 禁止・ノンブロッキング・有界であること。保証範囲は API 単位で明示
* 外部型の非露出 — ベースライン A.2 の `ruviform-rt` 行「外部同期クレート（rtrb 等）の型は露出しない」。採用クレートは private 実装詳細
* 実績と保守 — オーディオエコシステムで実績のあるクレートを優先し、自前実装の検証コストを避ける
* 最小依存木 — `ruviform-rt` は最下位の全クレート共通基盤であり、推移依存の肥大は全ワークスペースに波及する

## Considered Options

**SPSC リング実装:**

* `rtrb`（0.4.0、MIT OR Apache-2.0、依存ゼロ）を private 依存として採用し、自前の公開 API 型でラップする
* `ringbuf`（0.5.3、`crossbeam-utils` 等の依存を伴う）を採用する
* std のみで手組みする（`UnsafeCell` + atomics の自作リング。`unsafe` が必須）

**遅延ドロップ（Drop 退避）:**

* SPSC（採用リング）の上に自前の retire channel を構成する（RT 側は退役オブジェクトを `try_push`、非 RT 側の drain で drop。満杯時方針: drop-on-full は RT 側で dealloc が発生するため採用せず、事前確保済み予備容量 + `core::mem::forget` による leak-on-full フォールバックで RT 側の drop/dealloc/blocking を回避する設計とする）
* `basedrop`（0.1.3、MIT OR Apache-2.0）を採用する
* 他の lock-free collector（epoch 系・GC 系）を採用する

**不変スナップショット交換:**

* `arc-swap`（1.9.3、推移依存 `rustversion` のみ）を private 依存として採用し、`Arc` スナップショットの atomic swap を提供する。RT 側参照は専用 guard 型経由のみとし、guard の Drop が内部 `Arc` を retire channel へ送出。書き込み側の差替で追い出された `Arc` も同一 channel で退役させる（dealloc が非 RT drain でのみ発生する保証は arc-swap 単体ではなく guard + retire channel の組合せで実現）
* `AtomicPtr` 手組みの RCU 型実装を自前で書く（`unsafe` が必須）
* left-right 型（二重バッファ + 世代カウンタ）を採用・自前実装する

**スレッド優先度機構:**

* `audio_thread_priority`（0.38、MPL-2.0）を `default-features = false`（default `with_dbus` feature → `dep:dbus` のネイティブ libdbus 依存を排除し pthread 直接経路を使用）で private 依存として採用する
* `audio_thread_priority` をフル機能（`with_dbus` feature 有効、rtkit 経由の昇格を含む）で採用する
* OS API 直接 FFI を手組みする（MMCSS/time-constraint/`SCHED_FIFO` を自前 FFI。`unsafe` が必須）
* スレッド優先度機構を後続 Issue へ全面延期する

**スレッド優先度機構の配置**（採用する場合）:

* `ruviform-rt` 内の `non-RT only` モジュールに置く — ただしベースライン L160「Primitives はいかなる上位層・プラットフォーム・外部バックエンドにも依存しない。GPU・OS 固有 API・デバイス依存を持たない」に抵触する（rt は Primitives 最下位）
* `ruviform-exec`（非 RT 実行基盤側）に置く — exec の責務行は「OS スレッドの詳細は隠蔽」を要求しており、OS 抽象化である優先度 promotion と整合。rt の依存を `rtrb`+`arc-swap` のみに維持できる
* 将来の `ruviform-audio-device`（デバイス駆動側）に置く — デバイスコールバック起点としては妥当だが、exec 管理スレッドやオフライン駆動等の用途をカバーできず過小

**`RtContext` の mint 経路:**

* `#[doc(hidden)] pub` mint API を駆動者入口点（`ruviform-audio` のデバイスコールバック・`ruviform-exec` のオフライン駆動）にのみ提供する
* capability factory オブジェクト（`RtContextMint` 等の発行器型）を導入する
* sealed trait ベースで制限する

**`Processor` 契約の型形:**

* `Processor`（非 RT の `prepare`）+ `RtProcessor`（RT の `process`、`RtContext` を要求）の二 trait を associated types で接続する
* 単一 `Processor` trait に `prepare`/`process` を同一オブジェクト上で同居させる
* `dyn` dispatch 前提の API 形（`Box<dyn Processor>` 等）に設計する

## Decision Outcome

以下の組み合わせを採用する。

* **SPSC リング**: `rtrb` を private 依存として採用し、`ruviform-rt` の公開 API（チャネル端点型・`try_push`/`try_pop` 型操作）は自前型で提供する。満杯時方針は汎用のリングレベル enum ではなく API 単位で表現し、`try_push` は満杯時に値を呼び出し元へ返却する（`TryPushError<T>` 型）— drop/coalesce/keep のポリシー適用は呼び出し側に委ね、latest-wins のパラメータ適用には consumer 側の drain-latest ヘルパーを提供する。理由: SPSC リングの事実標準であり依存ゼロ・オーディオエコシステム実績があり、ベースラインが求める「有界・ノンブロッキング・満杯時方針を API で定義」を `try_push`/`try_pop` モデルに直接写像できる。`ringbuf` は需要（SPSC のみ。必須の推移依存は `crossbeam-utils` で、SPSC 限定という点は rtrb と対称）に対して機能・依存が過剰、手組みは `unsafe` 必須で規約と衝突するため不採用。
* **遅延ドロップ**: `basedrop` は採用せず、SPSC リングの上に自前の retire channel を構成する。理由: basedrop 型パターンの要件（RT 側から重い drop を非 RT 側へ逃がす）は採用済み SPSC への `try_push` + 非 RT 側 drain で完全に実現でき、追加依存の利点がない。満杯時方針: drop-on-full は RT 側で dealloc が発生するため拒否し、チャネル容量は構築時に固定（伸長なし）とした上で、事前確保済み予備容量 + `core::mem::forget` による leak-on-full フォールバックを採用する（満杯時も RT 側で drop/dealloc/blocking が起きないことを文書化した方針）。
* **スナップショット交換**: `arc-swap` を private 依存として採用する。理由: `Arc` の atomic swap の標準実装で、読み取り側が lock-free・wait-free となり「構成の稀な差替 + 毎ブロックの読み取り」のパターンに適合する。arc-swap は内部の atomic セルとして使い、no-RT-dealloc の保証は自前の guard + retire channel 組合せで実現する: RT 側のスナップショット参照は専用 guard 型経由でのみ取得でき、guard の Drop が内部 `Arc` を retire channel へ送出する（RT reader が guard を保持している間に writer が swap し非 RT drain が旧スナップショットを drop しても、reader 側 guard の解放が最終参照になって RT スレッド上で dealloc が起きる経路を塞ぐ）。書き込み側の差替で追い出された `Arc` も同一の退役チャネルで退役させる。`AtomicPtr` 手組みは `unsafe` 必須・世代管理の検証コストが自前化するため不採用、left-right 型は本件の読み書き頻度に対して実装が過剰なため不採用。
* **スレッド優先度**: `audio_thread_priority` を `default-features = false` で private 依存として採用し、**`ruviform-exec`** の `non-RT only` API として提供する（`ruviform-rt` には置かない）。配置を rt にしない理由: ベースライン L160 は Primitives 層に「GPU・OS 固有 API・デバイス依存を持たない」を課しており、OS 優先度 syscall を行う本機構は Primitives 最下位の rt には置けない。`ruviform-exec` の責務行は「OS スレッドの詳細は隠蔽」を要求しており、優先度 promotion はその OS 抽象化に属する。あわせて rt の依存は `rtrb`+`arc-swap` のみに維持できる。`ruviform-audio-device` 配置は exec 管理スレッド・オフライン駆動等の用途をカバーできず不採用。将来の層（audio-device 等）がより下位の配置を必要とした場合は配置を再評価しうるが、機構自体は配置に関わらず本 Issue のスコープとする。全面延期はベースラインが本 Issue を次フォーラムに指定しているため不採用。
  * **呼び出しモデル**: promotion は対象スレッド自身から呼ぶ自己昇格とする — exec 管理スレッドでは spawn 時セットアップで、デバイスコールバック由来スレッド（将来の audio-device）では初回コールバックの preamble（RT 契約区間の開始前 = `RtContext` mint・初回 `process` 呼び出し前）で呼ぶ。ブロッキング的な syscall を伴うため `non-RT only` とし、active な RT 契約区間内からの呼び出しは契約違反。promotion 失敗は致命的でなく best-effort の `Result` + 分類エラー型で表出する（パニックしない）。
  * **dbus 排除の帰結**: `default-features = false` で `with_dbus`（→ `dep:dbus`）経路を除去するため、rtkit・`CAP_SYS_NICE`・`RLIMIT_RTPRIO` のない非特権の標準デスクトップ Linux では promotion が失敗し、API はその失敗を `Result` として返す — この挙動をドキュメント化する。dbus 経路を再有効化する intent 命名の opt-in feature は将来のフォローアップ候補（依存クレート自身の feature 名の再露出は外部型非露出規則で禁止されるため独自命名とする）。
* **`RtContext` mint**: `#[doc(hidden)] pub` mint API を駆動者入口点にのみ提供する。`RtContext` は単一の opaque 構造（`_private` フィールドで外部構築不可・実行時のオフライン/RT モードフラグや variant は持たない — 「同一 API 内部で if rt 分岐しない」原則の維持。オフライン発行コンテキストでの制約緩和は実行時状態ではなく発行駆動者に紐づく契約・ドキュメントレベルの記述）を維持し、mint 経路だけを doc-hidden で公開する。理由: mint 権限を持つ駆動者は現状 2 箇所（audio デバイスコールバック・exec オフライン駆動）のみであり、capability factory 型の導入は間接層が過剰。sealed trait は値構築の制限に適さない。mint API 自体の `**RT contract:**` ラベルは `RT-suitable`（契約区間開始時の純粋なトークン構築）。
* **`Processor` 契約形**: `Processor`（非 RT の `prepare`）+ `RtProcessor`（RT の `process`、`RtContext` を引数に要求）の二 trait 分離を associated types で接続する。理由: ベースラインの「prepare で別の処理オブジェクトを生成する」型レベル分離をそのまま具現化し、`process` の契約区間外実行を型で抑止できる。単一 trait 同居は「prepare 済みか」を型で表せず、dyn 前提設計は最下位契約クレートで dispatch 方針を固定する早計な一般化であるため不採用。

### Consequences

* 良い点: ベースラインの要PoC 3 件（L486 スレッド優先度機構・L489 RT 通信/退避/スナップショット実装方式・L360 スナップショット具体実装）が解消され、`ruviform-rt`・`ruviform-exec` の実装が着手可能になる。採用クレートはいずれもオーディオ系で実績があり、自前 lock-free 実装の `unsafe` と検証コストを回避できる。retire channel を SPSC 上に構成することで `basedrop` 依存を不要にし、退役経路を単一プリミティブに統一できる。スレッド優先度機構を `ruviform-exec` に置くことで `ruviform-rt` の依存は `rtrb`+`arc-swap` のみに維持でき、Primitives 層の「OS 固有 API・デバイス依存を持たない」制約（L160）と整合する。
* 悪い点: `ruviform-rt` が 2 件（`rtrb`・`arc-swap`）、`ruviform-exec` が 1 件（`audio_thread_priority`）の外部依存を持ち、上位層の依存木に入る。外部型を露出しないラップ層の保守が恒常的に必要になる。`audio_thread_priority` は MPL-2.0（弱コピーレフト・ファイル単位）であり、licensing ADR の公開前ライセンスレビューで最終確認が要る（`symphonia` と同じ枠組みで管理可能）。`default-features = false` で dbus 経路を排除した帰結として、非特権の標準デスクトップ Linux（rtkit・`CAP_SYS_NICE`・`RLIMIT_RTPRIO` なし）では promotion が失敗し `Result` で返る — ドキュメント化必須で、再有効化する intent 命名 feature は将来のフォローアップ候補。
* **ベースライン L284 文言の弱体化の記録**: L284 は「アプリケーション等の任意コードは `RtContext` を構築できないため、…型で抑止できる」と型レベル抑止を述べるが、本決定の mint は cross-crate の駆動者が呼べるよう `pub` で到達可能な `#[doc(hidden)]` API である。したがって任意コードによる構築の抑止は型ではなく「doc-hidden + 駆動者入口点限定利用の契約規約 + コードレビュー」の規約レベルであり、ベースライン文言の弱体化として明示的に記録する。capability factory 等の間接化は発行器オブジェクトの配布管理に穴を移すだけで閉じないため採用していない。
* **依存追加規約の記録**（AGENTS.md「依存追加: メンテナンス状況・ライセンス・依存木サイズを確認し理由を記録する」に基づく確認結果）:
  * `rtrb` 0.4.0 — MIT OR Apache-2.0、推移依存ゼロ、オーディオ系で広く利用され安定。採用理由: SPSC リングの事実標準で需要と一致。
  * `arc-swap` 1.9.3 — MIT OR Apache-2.0、推移依存は `rustversion`（build-time ヘルパ）のみ、広く利用され安定。採用理由: `Arc` atomic swap の標準実装。
  * `audio_thread_priority` 0.38 — **MPL-2.0**（弱コピーレフト。ファイル単位のコピーレフトで、本リポジトリのファイルを改変しない限りライセンス義務は限定的。公開前ライセンスレビューで要確認項目として記録）。`default-features = false` で `with_dbus`（→ `dep:dbus`）/ネイティブ libdbus 依存を排除。推移依存は `cfg-if`・`log`・`libc`・`mach2`（macOS）・`windows-sys`（Windows）。採用理由: 3 OS の RT 優先度付与（MMCSS/time-constraint/SCHED_FIFO）を一枚 API で抽象化する唯一の実績あるクレート。
  * いずれも **private 実装詳細**であり、型・エラー・feature 名を `ruviform-rt`・`ruviform-exec` の公開 API に露出しない（直接の規則源はベースライン A.2 L190 の `ruviform-rt` 行「外部同期クレートの型は露出しない」。media-io ADR の規則と同型）。
* `unsafe` 例外: 採用クレート内部の unsafe は外部の問題として切り分け、`ruviform-rt`・`ruviform-exec` 自身のコードは safe のみで構成する方針。万一 API 表現上 `unsafe` が必要になった場合のみ `#[expect(unsafe_code)]` + SAFETY コメントの局所許可を用いる。

### Confirmation

本 ADR の遵守は次で確認する: `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` のグリーン、`cargo metadata` での依存リスト確認（`ruviform-rt`: `rtrb`・`arc-swap`。`ruviform-exec`: `audio_thread_priority` `default-features = false` で `with_dbus`/`dep:dbus` が無効）、`cargo doc` / 公開 API シグネチャレビューでの外部型非露出、spec の Acceptance Criteria との照合。

## Implementation Plan

* 影響するパス:
  * `crates/ruviform-rt/Cargo.toml` — `rtrb`・`arc-swap` の依存追加
  * `crates/ruviform-rt/src/lib.rs` と新規モジュール（契約型・SPSC・retire・snapshot・master clock・error）
  * `crates/ruviform-exec/Cargo.toml` — `audio_thread_priority`（`default-features = false`）の依存追加
  * `crates/ruviform-exec/src/lib.rs` と新規モジュール（thread priority）
  * `Cargo.lock`（依存追加に伴う自動更新）
  * `README.md` / `AGENTS.md` / ルート `Cargo.toml` コメント — rt・exec 実装済み状態への同期
* 従うパターン: `ruviform-media-core` の crate-level doc 構成（責務・Allowed dependency layers・RT contract・Contract source）に倣う。全公開 API に `**RT contract:**` ラベル、全公開 enum に `#[non_exhaustive]`、公開 API 例の doctest + 単体テスト + `rt_` プレフィックス RT 契約テスト。外部型はモジュール内部に封じ、公開面は自前型で写像する。`rt_` フィルタは部分文字列一致のため、非 RT のテスト・doctest 名は `rt_` 部分文字列を含まない命名とする
* 避けるパターン: 公開 API に `rtrb`/`arc-swap`/`audio_thread_priority` の型・エラー・feature 名を露出しない。`audio_thread_priority` の `with_dbus` feature（`dep:dbus`・ネイティブ libdbus 依存）を有効化しない。RT 側 API で alloc/dealloc・ブロッキング・無制限待機を行わない。退役オブジェクトの `Drop` を RT 側で実行しない（スナップショットの guard 解放・書き込み差替の双方が退役チャネルへ送出）。優先度 promotion を RT 契約区間内から呼ばない（exec 管理スレッドは spawn 時、デバイスコールバック由来スレッドは初回コールバック preamble の `RtContext` mint 前に呼ぶ）。`unsafe` の全面導入や手組み lock-free を行わない
* 依存関係: `ruviform-rt` に `rtrb` 0.4 系・`arc-swap` 1.x 系、`ruviform-exec` に `audio_thread_priority` 0.38 系（`default-features = false`）。いずれも MSRV 1.85・edition 2024・stable と整合することを確認済み。`Cargo.lock` は追跡対象のまま更新する

## Pros and Cons

**SPSC — `rtrb` 採用:**

* 良い点: 依存ゼロ・実績・API が RT 契約と整合。ラップ層で外部型を隠蔽しやすい形状。
* 悪い点: SPSC 限定で MPMC 需要が出た場合は追加機構が必要。

**SPSC — `ringbuf` 採用:**

* 良い点: 機能が広く将来の変種需要に耐えやすい。
* 悪い点: `crossbeam-utils` 等の推移依存で依存木が重く、SPSC 需要に対して過剰。

**SPSC — std 手組み:**

* 良い点: 依存ゼロ・完全制御。
* 悪い点: `unsafe` 必須で規約と衝突。メモリ順序の正しさの検証コストが自前に落ちる。

**退避 — SPSC 上の自前 retire channel:**

* 良い点: 追加依存ゼロで basedrop 型パターンを実現。退役経路が既存プリミティブと統一される。
* 悪い点: 満杯時の退役オブジェクト扱いを API で設計する必要がある（drop-on-full は RT 側 dealloc になるため拒否。事前確保済み予備容量 + `core::mem::forget` leak-on-full フォールバックで解決）。

**退避 — `basedrop` 採用:**

* 良い点: 目的特化の専用クレートで機構が明快。
* 悪い点: SPSC で代替可能な役割に依存を追加する利がない。版が古く保守頻度が低い。

**退避 — 他の lock-free collector（epoch 系・GC 系）採用:**

* 良い点: 高スループットの retire 管理が得られる。
* 悪い点: 本件の需要は「重いオブジェクトの drop を非 RT 側へ逃がす」だけであり機構が過剰。依存・検証コストが見合わない。

**スナップショット — `arc-swap` 採用:**

* 良い点: `Arc` swap の標準実装。読み取り lock-free で「稀な書き + 頻繁な読み」に適合。
* 悪い点: 依存 1 件追加。no-RT-dealloc 保証は arc-swap 単体では実現できず（reader 側 guard 解放が最終参照になる経路が残る）、自前の専用 guard 型 + retire channel の組合せ設計が必要。

**スナップショット — `AtomicPtr` 手組み:**

* 良い点: 依存ゼロ・読み取り完全 lock-free。
* 悪い点: 世代管理・退役タイミング・メモリ順序を自前で正しく実装する必要があり `unsafe` 必須。検証コストが最大。

**スナップショット — left-right 型:**

* 良い点: 読み取り安定・高スループット。
* 悪い点: writer が読者退出を追跡する機構が必要で実装複雑度が高い。本件の読み書きパターンには過剰。

**優先度 — `audio_thread_priority`（`default-features = false`）採用:**

* 良い点: 3 OS の優先度機構を一枚 API で抽象化する実績あるクレート。`with_dbus`（`dep:dbus`）排除でネイティブ依存を持ち込まない。
* 悪い点: MPL-2.0 のライセンス管理が要る。OS ごとの実効差・promotion 失敗 semantics の詳細は残課題。dbus 経路を切るため非特権の標準デスクトップ Linux では promotion が失敗し `Result` で返る（ドキュメント化する）。

**優先度 — フル機能（`with_dbus` 有効）採用:**

* 良い点: Linux で rtkit 経由の昇格が使える。
* 悪い点: ネイティブ libdbus 依存が基礎クレートに入り依存木・ビルド要件が重くなる。

**優先度 — OS 直接 FFI 手組み:**

* 良い点: 依存ゼロ・挙動を完全制御。
* 悪い点: 3 OS 分の `unsafe` FFI が必要で規約・保守コストの両面で不釣り合い。

**優先度 — 後続 Issue へ全面延期:**

* 良い点: 本 Issue の依存を増やさない。
* 悪い点: ベースラインが本 Issue を次フォーラムに指定しており、RT パス構築 Issue が再度ブロックされる。

**優先度配置 — `ruviform-rt` の `non-RT only` モジュール:**

* 良い点: RT 契約型と同居し利用側が機構を見つけやすい。
* 悪い点: ベースライン L160「Primitives は GPU・OS 固有 API・デバイス依存を持たない」に抵触する。rt の依存木が `audio_thread_priority` 分だけ膨らむ。

**優先度配置 — `ruviform-exec`（採用）:**

* 良い点: exec の責務（「OS スレッドの詳細は隠蔽」）と整合し、Primitives 層の OS 非依存制約を維持できる。rt の依存を `rtrb`+`arc-swap` のみに保てる。
* 悪い点: デバイスコールバック由来スレッドは exec が生成しないため、呼び出しモデル（対象スレッド自身による self-promotion）と適用タイミング（初回コールバック preamble の RT 契約区間開始前）を契約で明示する必要がある。

**優先度配置 — 将来の `ruviform-audio-device`:**

* 良い点: デバイスコールバック起点として自然。
* 悪い点: exec 管理スレッドやオフライン駆動等の用途をカバーできず過小。

**mint — `#[doc(hidden)] pub`:**

* 良い点: 機構が最小で追加型を要しない。`RtContext` の opaque 構造を維持できる。
* 悪い点: doc-hidden API の誤用防止は規約・レビューに依存し、型では完全に防げない。

**mint — capability factory:**

* 良い点: 発行権をオブジェクト所有で型追跡できる。
* 悪い点: 駆動者 2 箇所の現状では間接層が過剰。発行器オブジェクトの配布管理に抑止問題を移すだけで、任意コードの構築を型で閉じるわけではない。

**mint — sealed trait ベース:**

* 良い点: 実装可能性を型で制限する Rust 慣用パターン。
* 悪い点: sealed trait は trait 実装の制限であって値の構築制限には直接使えず、本件（`RtContext` 値の構築制限）には不適。

**Processor — 二 trait 分離:**

* 良い点: prepare/process のライフサイクル分離を型で表現し、契約区間外実行を抑止できる。
* 悪い点: trait が 2 つになり概念量が増える。

**Processor — 単一 trait 同居:**

* 良い点: API 面が単純。
* 悪い点: 「prepare 済みか」が型で表現されず、型レベル分離の目的を果たせない。

**Processor — dyn-dispatch 前提設計:**

* 良い点: グラフ実行側が具体型を意識せずに済む。
* 悪い点: オブジェクト安全性の制約が API 形状を縛り、最下位契約クレートで dispatch 方針を固定するのは早計。グラフ側で必要になった時点で adapter を置ける。

## Verification

* [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` がグリーン（`rt_` レーンが RT 契約テストのみを選択。非 RT テスト・doctest 名は `rt_` 部分文字列を含まない命名規約）
* [ ] `cargo metadata` で `ruviform-rt` の依存が `rtrb`・`arc-swap`、`ruviform-exec` の依存が `audio_thread_priority`（`default-features = false`）であり、`with_dbus`/`dep:dbus` が無効である
* [ ] 公開 API シグネチャ（`cargo doc` / コードレビュー）に `rtrb`・`arc-swap`・`audio_thread_priority` の型・エラー・feature 名が露出していない
* [ ] `RtContext` に公開コンストラクタ・実行時モードフラグ/variant がなく、mint API が `#[doc(hidden)]` で提供されている
* [ ] `Processor`/`RtProcessor` 二 trait が存在し、`process` が `RtContext` を要求するシグネチャである
* [ ] 退役オブジェクトの `Drop` が非 RT 側 drain でのみ実行され、スナップショットの guard 解放・書き込み差替の双方が退役チャネルへ送出して RT 側で dealloc を起こさないこと、および退役経路が満杯時も RT 側で drop/dealloc/blocking しないことがテストで検証されている
* [ ] `ruviform-exec` のスレッド優先度 API が `non-RT only` ラベル・self-promotion 呼び出しモデル・失敗時 `Result` 表出（非特権 Linux での失敗挙動のドキュメント化を含む）を満たす
* [ ] spec [implement-rt-foundation.md](../specs/implement-rt-foundation.md) の受け入れ基準と整合している

## More Information

* 一次ソース: GitHub Issue [#21 [impl] RT 基盤（rt プリミティブ・スレッド優先度機構・スナップショット交換）](https://github.com/scarecrow173/ruviform/issues/21)（`relations.source`）
* 上流ドキュメント（`relations.derives-from`）: [DISC-034JcqEc2b3EHNgH9CRLaC](../discovery/evaluate-rt-foundation-primitives.md)（候補比較・クレート評価の一次記録）、[SPEC-034Jcr3dOT5Uxp4znj42II](../specs/implement-rt-foundation.md)（実装要件。本 ADR は `relations.refines` で同 spec を具体化する）
* 関連ドキュメント（`relations.related`）: [RT 契約 ADR](realtime-contract-and-verification-policy.md)（API 単位の保証ラベル・検証分担）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（A.2 クレート表・RT/非 RT 分離方式・要PoC 分類表）、[AGENTS.md](../../AGENTS.md)（unsafe 原則禁止・依存追加規約・enum 方針）
* 参考（`relations.references`）: 親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)
