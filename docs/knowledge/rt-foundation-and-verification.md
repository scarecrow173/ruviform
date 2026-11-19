---
id: "KNOW-9LYC7Lmtlb7cwj6IPkbc88"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "RT 基盤と RT 検証の仕組み"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/21", "https://github.com/scarecrow173/ruviform/issues/22", "https://github.com/scarecrow173/ruviform/issues/126", "https://github.com/scarecrow173/ruviform/issues/130"]
  informs: []
  explains: []
  supersedes: []
---

# RT 基盤と RT 検証の仕組み

この文書は、リアルタイム（RT）基盤の設計と RT 検証の仕組みに関する再利用可能な知見をまとめたものである。内容は歴史的な実装記録から抽出したものであり、元の記録は削除済みのため、この文書は単独で読める形にしてある。各知見には出典の Issue 番号を併記し、観測された事実と解釈を区別する。

## RT 基盤の構成要素

RT 基盤は、SPSC リング、退役チャネル、マスタークロック、スレッド優先度、コンテキスト資格情報の各部品で構成される。以下の節では部品ごとの設計判断とその理由を記録する。

### SPSC リングと外部型の非露出

SPSC リングには crates.io の事実標準である rtrb 0.4 を採用した（Issue #21）。rtrb は MIT OR Apache-2.0 ライセンス・推移依存ゼロ・オーディオ系の実績が選定根拠である。ringbuf は依存木が過剰であり、手組みは unsafe が必須で unsafe 不使用の規約に抵触するため、いずれも ADR で不採用とした（Issue #21）。

Producer/Consumer は rtrb の端点を private フィールドに持つ自有型として定義し、rtrb の型とエラーは公開シグネチャに一切露出しない設計とした（Issue #21）。capacity() は内部バッファの capacity() に委譲し、固定値の重複保持を回避した（Issue #21）。

### 退役チャネルと leak-on-full

RT 側で退役した重いオブジェクトの Drop（dealloc・ロック・無制限のユーザコードを含み得る）を非 RT 側の drain へ送る SPSC ベースの退役機構を実装した（Issue #21）。満杯時は `core::mem::forget` による leak-on-full に倒し、drop-on-full は RT 側の dealloc になるため ADR で不採用とした。退役操作自体は RT-suitable・alloc なし・有界である（Issue #21）。

leak-on-full の保証範囲はレビューで訂正された。retire 呼出回数・レートに契約上の上限がないため、リーク総量は drain 間隔・生涯ともに非有界である。「drain 間隔あたり bounded」という表現も不正確と判明し、正確な保証は「キュー容量・retire ごとの処理量が有界・飽和件数が観測可能」までと明記された（Issue #21）。bounded-leak から leak-on-full への改名が全正典で行われた。

silent leak を避けるため、Retirer/DropCollector 間で共有する `Arc<AtomicUsize>` の overflow カウンタと両端点の `overflows()` を追加した（Issue #21）。capacity 0 の拒否は debug_assert から `assert!` へ昇格し、全ビルドで拒否する（Issue #21）。

DropCollector の drain はチャネル側ループを capacity 回を上限に `try_pop` する bounded-loop 規則とし、drain 中の並行 retire で非有界ループにならないことを保証した（Issue #21）。`is_empty` はチャネル近似値と pending 実値の合成であり、「観測時点近似」であることを doc に明記した（Issue #21）。

`push_local` は snapshot が呼出元になるまで本番呼出がなく `-D warnings` で dead_code になるため `#[allow(dead_code)]` を付与した（Issue #21）。`#[expect]` は利用開始後に unfulfilled となり再び lint エラーになるため allow を選択し、snapshot 到着で除去予定とした（Issue #21）。

### pop_latest の条件と有界性

`pop_latest` は保留要素を drain し、置換された要素を呼出スレッドで inline drop する（Issue #21）。「RT-suitable」のラベルは T のデストラクタが RT-safe な場合にのみ成り立つ条件付きであり、重い T は `try_pop` + retire 経路へ誘導する doc を付けた（Issue #21）。

有界 drain は呼出時点の保留要素のみを対象とし、producer が高速に push し続けると最新値を取りこぼし得る。容量有界を優先する設計である（Issue #21）。

### RtContext と mint 抑止

RtContext は `_private: ()` のみを持つ opaque 型で、公開コンストラクタ・公開フィールド・実行時モードフラグを持たない（Issue #21）。mint() は `#[doc(hidden)]` `#[must_use]` であり、mint 抑止は型レベルの完全抑止ではなく規約レベル（doc-hidden + レビュー）の意図的な弱体化として ADR に記録された（Issue #21）。process は `&RtContext` を要求し infallible で Result を返さない。mint() に doctest を付けないのは doc-hidden の意図（docs に表れない）を損なわないためである（Issue #21）。

### MasterClock

MasterClock は単一の `AtomicU64` tick と構築時固定の `(u32, u32)` timebase を持つ（Issue #21）。publish_tick は Release store、tick は Acquire load で、timebase は 0 成分で `Err(Error::InvalidValue("timebase"))` を返す。全メソッドが RT-suitable で、Release/Acquire の本来用途であるスレッド越境（Arc 越しの別スレッド reader）で可視性を検証した（Issue #21）。Error は `#[non_exhaustive]`・Copy の最小 variant で alloc なしとした（Issue #21）。

### !Sync 検証と expect_send

!Sync は stable Rust で負の bound をテストできないため、Producer/Consumer の !Sync は wrapped rtrb 端点の auto-derive 継承に依存し、doc 記述とシグネチャレビューで担保する。rtrb の semver bump で崩れた場合に検出されないリスクとして記録された（Issue #21）。代わりに `expect_send`/`expect_send_sync` の generic ヘルパーで Send+Sync の正側を型レベル検証し、端点境界を検証した（Issue #21）。

### ThreadPriorityGuard の !Send/!Sync 強制

ThreadPriorityGuard の Send/Sync は当初 upstream handle から構造導出のままで、unix fallback では handle が Send のため guard を別スレッドへ move でき、そこでの Drop が upstream の `assert!(pthread_self() == pthread_id)` に衝突し safe API から panic 可能と判明した（Issue #21）。`PhantomData<*const ()>` フィールド `_not_send` を追加して全プラットフォームで !Send/!Sync に強制し、demotion の calling thread=昇格スレッドのセマンティクスを型で強制した（Issue #21）。

guard の Drop は demotion を best-effort 試行して失敗を捨て、demote は `Option::take` で guard を by-value 消費して二重 demote を型で防止する（Issue #21）。

### audio_thread_priority の依存方針

3 OS の RT 優先度を一枚 API で抽象化する唯一の実績あるクレートとして audio_thread_priority 0.38 を `default-features = false` で private 依存に追加した（Issue #21）。dbus/dep:dbus 経路・ネイティブ libdbus 依存を排除し、推移依存は cfg-if/log/libc/mach2/windows-sys のみであることを cargo tree で確認した（Issue #21）。

PriorityError はモジュール単位 `#[non_exhaustive]` で、失敗系統が 1 系統のみのため crate 共通の Error/Result を作らない YAGNI 判断とした（Issue #21）。非特権 Linux では典型的に Err が返る仕様上の失敗として doc に記載した（Issue #21）。

### counting allocator とテスト共有

counting allocator はスレッドローカルゲート方式を採用した（Issue #21）。グローバルフラグでは並列実行中の他テストの確保を原理的に除外できないため、`const {}` 初期化の `thread_local! Cell<(bool, usize)>`（lazy 登録・アクセス時確保なしで allocator 内再帰を回避）でスレッド単位にゲートし、alloc/dealloc 双方を計測した（Issue #21）。

`try_with` でスレッド解体中のアクセスは Err→非計測に倒して allocator 内 panic を回避した（Issue #21）。

`#[global_allocator]` は lib テストバイナリに 1 つしか置けないため、後続タスクの no-alloc テストは crate 内共有の `#[cfg(test)]` alloc_counter を再利用し、別実装を置かない（Issue #21）。

退役オブジェクトの dealloc 地点観測には Drop で `Arc<AtomicBool>`/`AtomicUsize` を立てる DropFlag プローブ型を使う（Issue #21）。unsafe impl GlobalAlloc はテストスコープのみで `#[expect(unsafe_code)]` + SAFETY コメントにより局所許可し、製品コードの unsafe 不使用を維持した（Issue #21）。

## RT 検証の仕組み

RT 検証は、rt_ プレフィックスの契約テスト、assert_no_alloc による窓内 alloc 計測、AllocDisabler 宣言、ホスト側 thread-check の 4 系統で構成される。以下では命名規約・測定窓規約・観測限界・保証範囲の限定を記録する。

### rt_ テストの命名規約と substring 一致の注意

`mise run test-rt`（`cargo test --workspace rt_`）は部分文字列フィルタである（Issue #21）。RT 契約テストは `rt_` プレフィックス必須だが、`assert_*`/`start_*`/`convert_*` 等 `rt_` を内包する非 RT 名は禁止とした（Issue #21）。

逆に `short_input_zero_pads_and_long_input_truncates`（sho**rt_**input）や `resolved_tables_repo**rt_t**he_applied_value` のような偶然一致が lane に混入し得るため、一致件数の報告では RT 意味論の有無を区別して記録する（Issue #21）。doctest/テスト内 fn 名も同規約で、`on_rt_path`→`needs_context` 改名の先例がある（Issue #21）。

### assert_no_alloc の測定窓規約

測定窓の内側は RT 経路呼出（process・prepare_block・finish_block・COM vtable 経由 processor.process 等）のみで、fixture/確保・ctor/prepare（Result を返す非 RT 構築段階）・warm-up 呼出・assert 評価は窓外に置く（Issue #126）。`Kernel::new().process()` の chained call は `let mut kernel` へ hoist して ctor を窓外に配置する。ctor を窓内に残すと偽陽性となるため棄却された（Issue #126）。

warm-up 呼出は窓外に維持し、後続の各 process 呼出のみを個別に窓内測定した。初回呼出を窓内に含める案は期待値確立が測定対象でないため棄却された（Issue #126）。

AllocDisabler は test binary ごとに 1 個・`#[cfg(debug_assertions)]`・crate-root `mod tests` 内に宣言する規約を確立した（Issue #126）。`#[global_allocator]` は test binary ごとに 1 個しか宣言できず複数宣言はコンパイルエラーのため、crate-root の mod tests に集約するのが pattern である。integration test binary を持つ crate では各 binary ごとに同パターンの宣言が必要である（Issue #126）。

AllocDisabler 未宣言の assert_no_alloc 窓は常に成功し何も計測しない。media-core・dsp・audio-analysis の 3 crate は assert_no_alloc 測定窓と dev-dependency を持ちながら AllocDisabler を宣言しておらず、窓内 alloc を abort で機械検出できない状態だった（Issue #130）。従来の rt_ テスト報告は alloc-free の機械検証を意味していなかった。audio-analysis は crate-root mod tests がなかったため宣言のみの `#[cfg(test)] mod tests` を lib.rs に新設した（Issue #130）。

宣言配置の選択では、level.rs/spectrum.rs 側の mod tests に置く案は配置の任意性と既存 pattern 不一致で棄却し、crate-level `#[cfg(test)]` 配置も mod tests 内配置の既存 convention（video-proc/plugin-clap/vst3/math/audio-device 全て）と不一致で棄却した（Issue #130）。Cargo.toml の dev-dep コメントは依存変更がないため追加せず、差分を武装に限定した（Issue #130）。

### assert_no_alloc の観測限界

assert_no_alloc が観測するのは Rust global allocator のみで、C/FFI 側 alloc は対象外である（Issue #126）。ruviform-plugin-ofx は (1) VideoPluginInstance 全操作が NONRT 契約（render 含む）、(2) OfxInstance が Rc 由来で !Send/!Sync、(3) render の per-frame staging が C bridge の stage_alloc（Windows `_aligned_malloc`/POSIX malloc）で Rust AllocDisabler の観測外、の 3 根拠で lane 適用外と判断した（Issue #126）。窓を置くと「pass するが実際は C 側で alloc している」誤解を招く（Issue #126）。

### alloc_counter と assert_no_alloc の二系統ルール

新規 lane は assert_no_alloc を使い、ruviform-rt の既存 alloc_counter は併存維持とする二系統ルールを文書化した（Issue #22）。

### RT 検証の保証範囲と静的検証不能な制約

CI の rt_* テスト成功は RT 安全性の完全証明ではない。保証は API 単位で定義され実装責任とし、OS 全体のハードリアルタイム保証は行わないという 3 点を README・AGENTS.md・crate doc に明記した（Issue #22）。

静的に検証できない制約（ブロッキング操作・ファイル/ネットワーク I/O・無制限待機・実行時間無界・ロック保持時間）はドキュメント契約 + コードレビュー観点として列挙し、CI 非完全性を明文化した（Issue #22）。

### CLAP thread-check 統合

GainShared に host/thread_check を保持し、process 冒頭で is_audio_thread を照会して `Some(false)` を defined rejection、None/Some(true) は従来処理とする（Issue #22）。`with_arbitrary_lifetime` の unsafe は plugin lifetime が host に包含される CLAP 契約で正当化した（Issue #22）。

is_audio_thread None は graceful degradation（ホスト非対応時は検査無効）として文書化した（Issue #22）。テストホスト RtCheckHost は HostThreadCheckImpl を実装し `AtomicBool` で is_audio_thread を制御可能にした（Issue #22）。

### rt_ テスト実行結果の記録

#22 は fmt-check/check/lint/test（30 バイナリ全パス）/test-rt（clap 7・vst3 4 の rt_ テスト含む）/bench-smoke/prek run --all-files 全パス・doc audit findings 0 であった（Issue #22）。

#126 は video-proc lib test binary で test-rt 27 件一致（rt_process_* 26 件 + substring 偶然一致 1 件）・icc lane で rt_process_icc_* 2 件が窓内実行・test-ofx/lint-ofx green であった（Issue #126）。

#130 は `mise run test` で 59 result block 全て 0 failed・flake なしであった（Issue #126）。

## 未確定・保留事項

記録に残された未解決・保留事項を列挙する。

- Issue #126/#130 の GitHub issue の close/merge 状態は記録上「scope 外」と明記されており、文書からは判定不能である。
- 上流文書（docs/specs・docs/adr・docs/designs・docs/test-specs 配下）と Issue #126 の実装記録が参照する実験記録側の gate 記録（sanity abort 検証の実験記録）は未読であり、findings は task/plan/実装記録側の記述に基づく。
- locations の行範囲は文書構造からの推定値であり、正確な行番号は各ファイルを直接確認する必要がある。
