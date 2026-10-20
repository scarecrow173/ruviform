---
id: "DISC-034JcqEc2b3EHNgH9CRLaC"
type: "discovery"
status: "draft"
title: "RT 基盤プリミティブの実装方式評価（SPSC・退避ドロップ・スナップショット交換・スレッド優先度）"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/implement-rt-foundation.md"
    - "../adr/rt-foundation-primitives-implementation.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# RT 基盤プリミティブの実装方式評価（SPSC・退避ドロップ・スナップショット交換・スレッド優先度）

## Exploration Goal

本書は GitHub Issue [#21 [impl] RT 基盤（rt プリミティブ・スレッド優先度機構・スナップショット交換）](https://github.com/scarecrow173/ruviform/issues/21) の評価フェーズの中間成果を記録する canonical ドキュメントである。

設計ベースライン（[ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)）では RT/非 RT 分離の**構造**は確定済みだが、実装方式と依存クレートの採否が「要PoC」として残されている。本書が回答する問い:

- `ruviform-rt` が所有する RT プリミティブ（SPSC リング・退避ドロップ・不変スナップショット交換）を、外部クレート採用と自前実装のどちらで実装するか（ベースライン「未決定事項の分類」L489「RT 通信・退避・スナップショット交換の実装方式・依存クレート」、および L360「スナップショット交換の具体実装（arc-swap 型/RCU 型）」の解消）
- RT スレッドへの OS 優先度付与（MMCSS/time-constraint/SCHED_FIFO 系）をどの機構で提供するか（ベースライン L486「スレッド優先度付与の具体機構（audio_thread_priority 型）」の解消）
- `RtContext` capability の発行経路と `Processor` 契約の型形（trait 分離方法）をどう定義するか

> **位置づけ**: 本書は推論経路と一次情報を記録するものであり、最終決定を含めない。「傾向・推奨」節の各項目はすべて仮説であり、決定は ADR（[rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md)）、要求は spec（[implement-rt-foundation.md](../specs/implement-rt-foundation.md)）に昇華される。

## Key Issues and Assumptions

評価の前提となる制約（ベースライン・RT ADR・AGENTS.md で確定済み。再評価対象外）:

- **ruviform-rt は最下位クレート**: 他の ruviform クレートに依存できない（media-core の `MediaTime` 等は使えない）。公開 API に外部同期クレートの型を露出しない（ベースライン クレート一覧 A.2 L190）。
- **RT 契約**: RT クリティカルパスは alloc/dealloc 禁止・ブロッキング/I/O/無制限待機/非有界処理を制限。保証範囲は API 単位で `**RT contract:**` ラベルとして明示（RT ADR）。CI 成功は完全証明ではない。
- **`RtContext` 発行規則（決定済み）**: RT 契約区間を開始した駆動者のみが発行できる capability。発行主体は RT 駆動者（デバイスコールバック内の `ruviform-audio`）とオフライン駆動者（`ruviform-exec` の非 RT ドライバ）に限定し、任意コードは構築不可（ベースライン L283-285）。（→ ADR で mint は cross-crate 駆動者が呼べる `pub` 到達可能な `#[doc(hidden)]` API と決定され、「型で抑止」は「規約 + コードレビューによる抑止」への弱体化として明示記録された）
- **通信方向の定型（決定済み）**: RT → 非 RT はメータ・イベント・退避オブジェクト、非 RT → RT はパラメータ・制御コマンド。いずれも有界・ノンブロッキングで、満杯時の方針（ドロップ/統合）は API 単位で定義（ベースライン L286）。
- **暫定開発規約**: `unsafe` 原則禁止（SAFETY コメント付き局所許可のみ例外的に認める）。依存追加はメンテナンス状況・ライセンス・依存木サイズの確認と理由記録が必須。edition 2024・MSRV 1.85・stable・`publish = false`・全公開 enum に `#[non_exhaustive]`。
- **スコープ境界（兄弟 Issue との分担）**: `assert_no_alloc` 型 CI レーンと thread-check 機構は RT 検証 Issue、pull グラフ実行エンジンは engines 系 Issue、`clack-extensions` thread-pool はプラグイン基盤 Issue、オフライン決定論的実行の保証は後続 Issue。本評価はこれらを含めない。

## Alternatives and Comparison

機構ごとの選択肢比較。`Lean` は評価時点の傾向（仮説）であり採否決定ではない。

### SPSC リング（RT ↔ 非 RT 通知の中核）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `rtrb`（0.4.0、MIT/Apache-2.0、依存ゼロ） | SPSC リングの事実標準。オーディオエコシステム（Firewheel 等）で実績。ゼロ依存で依存木が最小。wait-free SPSC で RT 契約に整合 | SPSC 限定（MPMC なし。ringbuf も同様に SPSC 限定であり対称）。生の型は公開 API に露出させられないためラップが必要 | yes |
| `ringbuf`（0.5.3） | 機能が広い（heap/static/atomic backed） | 必須の推移依存は `crossbeam-utils` 1 件だが依存木が重い。本件の需要は SPSC のみで過剰 | no（rtrb が不足した場合の再評価候補） |
| std のみで手組み（`UnsafeCell` + `Atomic*` の自作リング） | 依存ゼロ。容量・メモリ配置を完全制御 | lock-free リングの正しい実装には `unsafe` と順序付けの精密な設計が必須。ワークスペースの「unsafe 原則禁止」規約と衝突し、検証コストが自前に落ちる | no |

- 含意: ベースラインが要求する「有界・ノンブロッキング・満杯時方針を API で定義」は rtrb の `try_push`/`try_pop` モデルにそのまま写像できる。公開 API は `try_push` が満杯時に値を呼び出し元へ返す `TryPushError<T>` 型等の自前型に写像して外部型を隠蔽する（汎用のリングレベル方針 enum は持たず、drop/coalesce/keep の適用は呼び出し側ポリシー、latest-wins は consumer 側 drain-latest ヘルパーで提供 — ADR で決定）。

### 遅延ドロップ（Drop 退避）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| SPSC 上の自前 retire channel | RT 側は退避オブジェクトを SPSC に `try_push` するだけ（ノンブロッキング・alloc なし）。非 RT 側の drain が `Drop` を実行。採用済みの SPSC と同一機構で実現でき、追加依存ゼロ | `Arc` 等で包んだ退避オブジェクトを流す設計と drain 契約（非 RT 側のみ）を自前で定義する必要がある | yes |
| `basedrop`（0.1.3）採用 | RT → 非 RT への Drop 移譲に特化した専用クレート。参照実装として適格 | 役割は SPSC 上の retire channel で完全に代替可能。専用化の利点より依存追加コストが勝る。版が古くメンテナンス頻度も低い | no（参照パターンとして採用） |
| 他の lock-free collector（epoch 系・GC 系） | 高スループットの retire 管理 | 本件の需要は「重いオブジェクトの drop を非 RT 側へ逃がす」だけであり過剰 | no |

- 含意: ベースラインの「basedrop 型パターン」は機構要件であってクレート採用の決定ではない。SPSC（rtrb）採用下では同一プリミティブの上に retire channel を構成するのが最も単純。満杯時方針は drop-on-full を採らない（RT 側で dealloc が発生するため）— チャネル容量を構築時に固定（伸長なし）とし、事前確保済み予備容量 + `core::mem::forget` による bounded-leak フォールバックで満杯時も RT 側の drop/dealloc/blocking を回避する方針を採用（ADR で決定）。

### 不変スナップショット交換（グラフ・構成の切替）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `arc-swap`（1.9.3、`rustversion` 依存） | `Arc` の atomic swap の標準実装。読み取り側は lock-free・wait-free で RT 契約に整合 | 依存 1 件追加（推移依存は `rustversion` のみで軽い）。旧スナップショットの最終 `Drop` が RT スレッド側で起き得る経路は arc-swap 単体では塞げず（reader 側の最終 `Arc` 解放が RT スレッド上で起きる穴）、専用 guard 型 + retire channel の組合せで解決（guard の Drop が内部 `Arc` を退役チャネルへ送出 — ADR で決定） | yes |
| `AtomicPtr` 手組み（RCU 型の自前実装） | 依存ゼロ。読み取りは完全に lock-free | 世代管理・退役タイミング・メモリ順序を自前で正しく実装する必要があり `unsafe` 必須。ワークスペース規約と衝突し検証コストが最大 | no |
| left-right 型（二重バッファ + 世代カウンタ） | 読み取りが安定・高スループット | 実装複雑度が高く、writer が読者の退出を追跡する機構が要る。本件は「構成の稀な差替 + 毎ブロックの読み取り」で arc-swap 型が十分 | no |

### スレッド優先度付与

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `audio_thread_priority`（0.38、MPL-2.0）`default-features = false` | MMCSS（Windows）/time-constraint policy（macOS）/`SCHED_FIFO`（Linux）の差を一枚の API に写像する実績あるクレート。default `with_dbus` feature（`dep:dbus`）を切ればネイティブ libdbus 依存を排除し、Linux でも pthread 直接経路で動作 | MPL-2.0（弱コピーレフト・ファイル単位。symphonia と同じ枠組みで管理可能だが要記録）。promotion 失敗時の semantics は OS 差があり API で明示が必要。dbus 経路を切るため非特権の標準デスクトップ Linux（rtkit・`CAP_SYS_NICE`・`RLIMIT_RTPRIO` なし）では promotion が失敗し `Result` で返る挙動となる | yes |
| `audio_thread_priority` フル機能（default `with_dbus`（`dep:dbus`）有効） | Linux で RealtimeKit（rtkit）経由の権限昇格が使える | ネイティブ libdbus への依存が入り、依存木・ビルド要件が重くなる。基礎クレートにネイティブシステム依存を持ち込むのは不適切 | no |
| OS API 直接 FFI の手組み | 依存ゼロ・挙動を完全制御 | `unsafe` FFI が 3 OS 分必要で、ワークスペース規約・保守コストの両面で不釣り合い | no |
| 後続 Issue へ全面延期 | 本 Issue の依存を増やさない | ベースライン L486 が本 Issue を次フォーラムに指定済み。スレッド優先度機構は `ruviform-rt` の責務候補として評価が求められており、延期すると RT パス構築 Issue が再度ブロックされる | no |

- **配置の選択肢**（採用する場合の置き場所）: (a) `ruviform-rt` 内の `non-RT only` モジュール — 優先度付与はスレッド生成時の非 RT 区間で呼ぶものであり RT 契約型と同居できる。crate 表の RT 制約「公開 API はすべて RT 適合または『非 RT 専用』の明示ラベル」の後者に該当、(b) `ruviform-exec` — ワーカースレッド基盤側に置く案。ただしデバイスコールバック由来の RT スレッドは exec が生成しないため責務がずれる、(c) 将来の `ruviform-audio-device` — 起点は妥当だが、オフライン駆動や他の RT スレッド用途（MIDI 等）をカバーできず過小。傾向は (a)。**→ 最終決定（ADR）は (b) `ruviform-exec` 配置**: ベースライン L160「Primitives は GPU・OS 固有 API・デバイス依存を持たない」により rt 配置は不可と判断され、exec の責務（「OS スレッドの詳細は隠蔽」）に適合するとされた。呼び出しモデルは対象スレッド自身による self-promotion（exec 管理スレッドは spawn 時セットアップ、デバイスコールバック由来スレッドは初回コールバック preamble の RT 契約区間開始前）。将来の層がより下位の配置を必要とした場合は再評価しうる。
- 含意: 「優先度付与 API は非 RT 専用」とラベル付けし、RT コールバック内からの呼び出しを契約違反とする設計が RT/非 RT 分離の型レベル規則と整合する。

### `RtContext` の発行経路（mint 機構）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `#[doc(hidden)] pub` mint API（駆動者入口点のみが使う） | 型は安全側（`_private: ()` で外部構築不可）を維持しつつ、駆動者クレート（audio / exec）が入口点で mint できる。機構が最小で追加型を要しない | 「公開だが隠す」運用は規約依存で、doc-hidden API の誤用を完全には防げない（lint/レビュー観点で補う） | yes |
| capability factory オブジェクト（`RtContextMint` 等の発行器を駆動者のみが構築） | 発行権をオブジェクト所有で表現でき、mint 権限の受け渡しを型で追跡可能 | 間接層が増え API 面が広がる。現段階では駆動者は 2 箇所のみで過剰 | maybe（将来 mint 権限が増えた場合の再評価） |
| sealed trait ベースの制限 | 実装可能性を型で制限する Rust 惯用 | sealed trait は実装の制限であって値の構築制限には直接使えず、本件には不適 | no |

### `Processor` 契約の型形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `Processor` + `RtProcessor` の二 trait 分離（`prepare` → 別の RT 処理オブジェクトを生成、associated types で接続） | ベースラインの「prepare(setup) で処理オブジェクトを生成し、`process` を持つ別オブジェクトにする」型レベル分離をそのまま具現化。`process` が `RtContext` を要求する形で契約区間外実行を型で抑止 | trait が 2 つに分かれ概念量が増える。ジェネリック前提で `dyn` 適合性は別途検討が要る | yes |
| 単一 `Processor` trait・同一オブジェクトに prepare/process を同居 | API 面が単純 | 「prepare 済みか」が型で表現されず、`process` 前呼び出しや再 `prepare` を実行時エラーで処理する必要があり型レベル分離の目的を果たせない | no |
| dyn-dispatch 前提設計（`Box<dyn Processor>` をAPI 形にする） | グラフ実行側が具体型を意識しない | オブジェクト安全性の制約が API 形状を縛り、最下位契約クレートで dispatch 方針を固定するのは早計。グラフ側で必要になった時点で adapter を置ける | no |

### 依存クレート評価一覧

| crate | 版 | ライセンス | 依存木 | メンテナンス | 適合性 |
| --- | --- | --- | --- | --- | --- |
| `rtrb` | 0.4.0 | MIT OR Apache-2.0 | ゼロ | オーディオ系で広く利用・安定 | SPSC 通知の定石。採用有力 |
| `ringbuf` | 0.5.3 | MIT OR Apache-2.0 | `crossbeam-utils` 等を伴い中程度 | 活発 | 機能過剰。rtrb 不足時の再評価候補 |
| `basedrop` | 0.1.3 | MIT OR Apache-2.0 | 極小 | 低頻度（成熟・機能限定） | 役割は SPSC 上の自前 retire channel で代替可能。参照パターン |
| `arc-swap` | 1.9.3 | MIT OR Apache-2.0 | `rustversion` のみ | 広く利用・安定 | スナップショット atomic swap の標準実装。採用有力 |
| `audio_thread_priority` | 0.38 | MPL-2.0 | default `with_dbus` feature（→ `dep:dbus`）はネイティブ libdbus を要求 → `default-features = false` で排除可（pthread 直接経路）。推移依存は `cfg-if`・`log`・`libc`・`mach2`（macOS）・`windows-sys`（Windows） | オーディオ系で実績 | 3 OS の RT 優先度付与を一枚 API で抽象化。採用有力（条件付き） |
| `assert_no_alloc` | — | — | — | — | **本件スコープ外**（RT 検証 Issue が採否・導入を担う） |
| 手組み lock-free（`UnsafeCell`/atomics/`AtomicPtr`） | — | — | ゼロ | 自前 | `unsafe` 必須で暫定規約と衝突。検証コストが自前化。不採用方向 |

## Tentative Conclusions and Hypotheses

各項目は仮説（傾向）であり、確定は ADR `rt-foundation-primitives-implementation` で行う。

- **[仮説] SPSC リングは `rtrb` を private 依存として採用し、公開 API は自前型（`Producer`/`Consumer` 相当・`try_push` が満杯時に値を返す `TryPushError<T>` 型）でラップする。** 満杯時方針は汎用のリングレベル enum ではなく API 単位（呼び出し側の drop/coalesce/keep ポリシー + consumer 側 drain-latest ヘルパー）で表現する。根拠: ゼロ依存・オーディオ系実績・SPSC 限定が需要と一致。`ringbuf` は依存木が重く機能過剰（SPSC 限定は rtrb と対称）、手組みは `unsafe` 必須で規約と衝突。
- **[仮説] 遅延ドロップは `basedrop` を採用せず、SPSC（rtrb）上に自前の retire channel を構成する。** RT 側は `try_push` で退役オブジェクトを送出、drop の実行は非 RT 側 drain のみ。追加依存ゼロで basedrop 型パターンを満たす。満杯時は drop-on-full を採らず（RT 側 dealloc になるため）、事前確保済み予備容量 + `core::mem::forget` bounded-leak フォールバックで RT 側の drop/dealloc/blocking を回避する（ADR で決定）。
- **[仮説] スナップショット交換は `arc-swap` を private 依存として採用する（arc-swap 型）。** 読み取り lock-free の `Arc` atomic swap が「構成差替は稀・読み取りは毎ブロック」のパターンに適合。旧スナップショットの退役は retire channel と組み合わせ、RT 側で dealloc が起きないよう API 契約で固定する。`AtomicPtr` 手組み・left-right 型は不採用方向。
- **[仮説] スレッド優先度は `audio_thread_priority` を `default-features = false`（`with_dbus`/`dep:dbus` 排除・pthread 直接経路）で private 依存として採用する。** 配置の傾向は `ruviform-rt` の `non-RT only` モジュールだったが、**最終決定（ADR）は `ruviform-exec` 配置**（Primitives 層の OS 固有 API 禁止 L160 との整合。呼び出しは対象スレッド自身による self-promotion — exec 管理スレッドは spawn 時、デバイスコールバック由来は初回コールバック preamble の RT 契約区間開始前）。promotion 失敗はパニックでなく `Result` + 分類されたエラー型で表出する方向。`default-features = false` 構成では非特権の標準デスクトップ Linux で promotion が失敗する挙動をドキュメント化する。OS 差（MMCSS/time-constraint/SCHED_FIFO の実効差）は API ドキュメントと別途検証に委ねる。
- **[仮説] `RtContext` は現行の opaque 構造（`_private` で外部構築不可）を維持し、mint API は `#[doc(hidden)] pub` 関数として駆動者入口点（audio デバイスコールバック・exec オフライン駆動）にのみ公開する。** 単一型とし実行時のオフライン/RT モードフラグや variant は持たない（「同一 API 内部で if rt 分岐しない」原則を維持。オフライン発行での制約緩和は発行駆動者に紐づく契約・ドキュメントレベルの記述 — ADR で決定）。capability factory 型は mint 権限が 2 箇所の現状では過剰。
- **[仮説] `Processor` 契約は `Processor`（非 RT・`prepare`）と `RtProcessor`（RT・`process` を要求、引数に `RtContext`）の二 trait 分離とし、associated types で接続する。** ベースラインの「別の処理オブジェクト/状態を生成」する型レベル分離をそのまま具現化する形。
- **[仮説] 採用する外部依存（rtrb・arc-swap・audio_thread_priority）はいずれも private 実装詳細とし、型・エラー・feature 名を公開 API に露出しない。** AGENTS.md の依存追加規約に基づくライセンス・保守・依存木確認は ADR の Consequences に記録する。
- **[確認済み] `MasterClock` 時刻共有プリミティブ・事前確保型ハンドオフ構造・共通エラー型は `ruviform-rt` の責務**（ベースライン A.2/L190 で決定済み）。
- **[仮説] `MasterClock` の具体表現は単一 `AtomicU64` の tick + 構築時固定の timebase とし、単一の論理 writer（現役の駆動者 — RT・オフラインいずれでも可）が tick、読み取り側は wait-free という形が設計と整合**（単一 writer tick 構造はベースラインの明示的記述ではなく設計推論として記録。timebase が不変のため tearing/seqlock は不要）。
- **[確認済み] スコープ外**: `assert_no_alloc` CI レーン・thread-check（RT 検証 Issue）、pull グラフ実行（engines）、`clack-extensions` thread-pool（プラグイン基盤 Issue）、オフライン決定論的実行の厳密保証（後続 Issue）。

## Open Questions

- **OS 別の優先度付与の具体差**: MMCSS「Pro Audio」、macOS time-constraint policy、Linux `SCHED_FIFO`/`SCHED_OTHER`+rtkit の実効・上限・失敗条件は OS ごとに異なり、実機検証を要する。API 形状は確定できるが、各 OS での具体的パラメータ写像（要求レイテンシ・周期からの算出）は実装・実機評価で確定する。
- **promotion 失敗の semantics**: 優先度付与が拒否/降格された場合に、呼び出し側へ返す情報の粒度（エラー分類・フォールバック方針・再試行可否）は API 設計で固定すべきか、呼び出し側ポリシーに委ねるべきかが未確定。→ 方針確定: 失敗は `Result` + 分類エラー型で表出し best-effort（致命ではない）。`default-features = false` 構成では非特権の標準デスクトップ Linux で失敗しうる挙動をドキュメント化（ADR で決定）。
- **`basedrop` 非採用の再評価条件**: retire channel を SPSC 上に自前構成する方針で、RT 側 `try_push` が満杯で失敗した場合の退役オブジェクト扱い（RT 側 drop は禁止のため、満杯時は非 RT 側での事前確保済み予備スロット等の設計）が API 設計で必要。複雑化した場合は basedrop 系への差替を再評価する。→ 解決済み: drop-on-full は RT 側 dealloc になるため拒否し、チャネル容量を構築時に固定（伸長なし）の上で事前確保済み予備容量 + `core::mem::forget` bounded-leak フォールバックを採用（ADR で決定）。複雑化時の basedrop 系再評価の条件のみ残る。
- **満杯時方針の API 表現**: 「ドロップ/統合」方針を API 単位で定義する際の型形（`PushError` への分類 enum、コールバック、上書き式リングの使い分け基準）は spec/plan で具体化する。→ 解決済み: 汎用のリングレベル方針 enum は採用せず、`try_push` が満杯時に値を返す `TryPushError<T>` 型で呼び出し側ポリシー（drop/coalesce/keep）に委譲し、latest-wins は consumer 側 drain-latest ヘルパーで提供（ADR で決定）。
- **`RtContext` の中身**: 現行はプレースホルダー（`_private`）。オフライン発行で「RT 制約を適用しない」区別を型で持つか（フラグ・ variant）を API 設計で確定する。→ 解決済み: 単一の opaque 型とし実行時モードフラグ/variant は持たない（「同一 API 内部で if rt 分岐しない」原則を維持）。オフライン発行コンテキストでの制約緩和は発行駆動者に紐づく契約・ドキュメントレベルの記述（ADR で決定）。

## Promotion Candidates

- [x] spec-doc needed: `ruviform-rt` 公開 API の確定要件を [implement-rt-foundation.md](../specs/implement-rt-foundation.md)（本書 `relations.derived-by`）として昇華済み
- [x] adr-doc needed: 実装方式・依存採否（SPSC= rtrb・退避=SPSC 上自前・スナップショット=arc-swap・優先度=audio_thread_priority・mint=doc-hidden・Processor=二 trait）の決定を [rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md)（本書 `relations.derived-by`）として昇華済み
- [ ] 今後の昇格候補: 実装 plan・test-spec は spec 承認後に作成（`implemented-by`/`verified-by` で接続予定）

## Sources and Relations

- 一次ソース: GitHub Issue [#21](https://github.com/scarecrow173/ruviform/issues/21)（`relations.source`）。ベースライン「後続実装 Issue の提案一覧」L571 の「RT 基盤 Issue」として起票された。
- 評価対象の正本: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md) の「RT／非 RT の分離方式」（L280-306。`RtContext` 発行規則・SPSC 通信方向・Drop 退避・スナップショット交換・スレッド優先度の構造決定）、「処理グラフモデルとスケジューリング採否」（pull ベース決定）、「RT 契約（API 単位の保証定義）」（L428-432）、「未決定事項の分類」の要PoC 行（L486 スレッド優先度機構・L489 RT 通信/退避/スナップショット実装方式）、「暫定実装ロードマップ」Phase 1（L507-511）、クレート一覧 A.2 の `ruviform-rt` 行（L190）。
- 契約の上位規範: [RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（適用範囲・禁止事項・分離方式・検証分担）。
- 参考: 親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（`relations.references`）、Issue #1 のエコシステム調査（[explore-architecture-design-baseline.md](explore-architecture-design-baseline.md)「RT 実践パターン」表。`rtrb`/`basedrop` パターン/`audio_thread_priority` の一次調査）— なおクレート版・依存木の評価は本 Issue で再確認した値を用いる。
