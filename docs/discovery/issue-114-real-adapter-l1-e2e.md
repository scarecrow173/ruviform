---
id: "DISC-034MHDcCcgTMOTFraQb90l"
type: "discovery"
status: "resolved"
title: "L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP/VST3/OFX）の探索（Issue #114）"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/123"
    - "https://github.com/scarecrow173/ruviform/issues/118"
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
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/plugin-rt-ownership-split.md"
    - "../adr/ofx-host-video-contract-and-c-abi.md"
    - "../specs/ofx-host-adapter.md"
    - "../designs/ofx-host-adapter.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/plugin-l1-out-of-process.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../discovery/engine-plugin-host-integration.md"
    - "../specs/engine-plugin-host-integration.md"
    - "../designs/engine-plugin-host-integration.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/plugin-isolation-strategy.md"
    - "../designs/plugin-isolation.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
  derived-by:
    - "../specs/issue-114-real-adapter-l1-e2e.md"
    - "../designs/issue-114-real-adapter-l1-e2e.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP/VST3/OFX）の探索（Issue #114）

## Exploration Goal

Issue [#114](https://github.com/scarecrow173/ruviform/issues/114) は、#76（L1 アウトプロセス経路）の検証が `GainPlugin`/`l1-test-worker` のモック範囲に留まることを指摘し、**規格アダプタ経由の実プラグインを子プロセスで動かす経路の実装・実証**を要求する。

Issue スコープ:

- `ruviform-plugin-export::run(factories)` エントリを規格アダプタ経由の実プラグイン構成で exe 化する形の確定（runner バイナリのオーサリング面）
- 実 CLAP/VST3/OFX プラグインでの spawn→lifecycle→process→destroy E2E
- 実バイナリ依存のため CI lane 設計（opt-in/手動）を含む

トリガ: `docs/designs/plugin-l1-out-of-process.md` L272 deferred 行（「規格アダプタ（clap/vst3/ofx）の子プロセス組込み・実プラグイン L1 E2E | runner 形態の前提が本書で確定・別 Issue 級の配線 | 実プラグインでの L1 需要 | 検証がモック範囲に留まる」）— 本 Issue がその trigger。#113 merge により host 側の endpoint 面・graph E2E 配線も揃った状態。

本探索は **issue framing・コード観測事実（runner オーサリング面・規格アダプタの現状と向き・実プラグインバイナリの在否・CI lane 慣行）・選択肢比較・確認済みスコープの記録**を行う。採否・方式の確定権限は design フェーズにあり、本書は最終判断を含まない。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — design 論点ではない）

- **runner のプロセス形態**: #76 design 決定サマリ `a` 行で確定 — `run(factories)` エントリを持つ薄いライブラリ + 参照 runner bin。#76 discovery 軸 4 の採用案は「`ruviform-plugin-export` が runner エントリ API を提供し、**利用側/規格アダプタが子プロセスバイナリを構成** … 外部プラグインもアダプタ経由で load 可能」（`docs/discovery/plugin-l1-out-of-process.md` Alternatives 軸 4 (a) — design 確定の配置）。従って「子プロセス内で実プラグインを規格アダプタ経由で instantiate する」方向自体は確定済みであり、残るのは **オーサリング面と配線の設計**。
- **1 子 = 1 インスタンス**: per-`Instantiate` spawn、`--ruviform-plugin-id` で factory 選択（design L97・`run` の argv 規約）。
- **wire/runner は audio `process` 専用**: #113 design 決定 `k` — 「L1 video wire は確立しない」（`docs/designs/engine-plugin-host-integration.md` L284）。shm スロットは planar `f32` audio ブロック固定レイアウト（`ruviform_plugin_core::wire::shm`）。
- **video plugin は `VideoPluginInstance`（全操作 NONRT）**: `crates/ruviform-plugin-core/src/video.rs:94` — `render` 中心の別 trait であり、`PluginFactory→ErasedPlugin` の audio 系表面には載らない。
- **自動再起動なし・`Failed`+通知+pass-through**: L1 契約は design/実装済み・本 Issue が再決定しない。
- **L1 の RT 側は proxy 側の `RtLeg`**: 子プロセス内プラグインの `Send` 性は host 側 RT endpoint 契約と無関係（`create_sendable` は L0 endpoint 用の opt-in — L1 経路では不要求）。foreign ABI オブジェクトが `!Send` でも L1 経路は成立する（#113 ADR の所有分離は host 側で完結）。

### コード観測事実（confirmed — 前提が最初に崩れる地点）

#### `run(factories)` オーサリング面

- 公開エントリは [`run`](../../crates/ruviform-plugin-export/src/lib.rs) のみ: `pub fn run(factories: Vec<Box<dyn PluginFactory>>) -> ExitCode`（`lib.rs:124`）。argv は内部で `std::env::args_os().skip(1)` を直接読み（`lib.rs:125`）、受理する flag は `--ruviform-shm-key` / `--ruviform-plugin-id` のみ — **未知引数は `ArgvReject::Usage` → exit 2 の defined rejection**（`lib.rs:322-376`）。
- 参照 runner bin `src/bin/ruviform-plugin-l1-runner.rs:39-45` — `fn main() { ruviform_plugin_export::run(vec![GainFactory, PanicFactory, UnderwriteFactory]) }`。**今日のオーサリング形 = 手書き `main` + コンパイル時埋込み factory リスト**。macro・loader 仲介・引数注入 API は存在しない。
- `PluginFactory`（`crates/ruviform-plugin-core/src/plugin.rs:204-237`）— `descriptor()` + `create() -> Box<dyn ErasedPlugin>` + opt-in `create_sendable()`（default `None`）。実プラグインの L1 実行に `create_sendable` は**不要**（host 側 RT endpoint は proxy `RtLeg` で既に `Send`）。
- `OutOfProcessSpec`（`composite.rs:65-101`）— `descriptor`/`program`/`args`/`lookahead_blocks`/`command_timeout`。`args` は bootstrap flag **より前に**挿入される（`l1/mod.rs:1005-1011`: `.args(&spec.args).arg("--ruviform-plugin-id")…`）。
- **`spec.args` は `run()` ベースの runner へ到達不能** — `run()` は未知引数を拒否するため、`--plugin-path` のような per-route 引数を `spec.args` で渡すと usage reject になる。**loader 型 runner（実行時にプラグインバイナリ path を受け取る）を作るには `run` の argv 規約拡張 or factory 側への設定経路が要**（compile-time 埋込みなら現行 API で可能）。
- descriptor ハンドシェイク: `spec.descriptor` は子の `Scanned` 報告と field-equality が要求される（`composite.rs` doc — 不一致は spawn `Err(Ipc)`）。実プラグイン経由では、子が報告する `PluginDescriptor` は**アダプタが実バイナリの metadata から合成**する必要があり、host 側 `spec.descriptor` の値の出どころ（アダプタ scan API or 固定値）が要確定。

#### 規格アダプタの現状と向き（実プラグインを子で動かす欠落部品）

- `ruviform-plugin-clap` — **制作側（plugin-side）アダプタ**。`clack_export_entry!` で `clap_entry` を export する cdylib（`src/lib.rs:76`）。公開 API は中立 `PluginProcessor` 実装の `GainProcessor` と `GAIN_PARAM` のみ（`GainClapPlugin` は `pub(crate)`）。**`PluginFactory` 実装は提供しない**。`clack-host` は dev-dependency のみ（`Cargo.toml:37` — test で `PluginEntry::load_from_clack` による in-process typed-entry 経路を実 ABI 駆動に使用。**ファイルからの `PluginEntry::load(path)`（dlopen）経路は upstream に存在するが本 repo では未使用**）。
- `ruviform-plugin-vst3` — **制作側アダプタ**。`.vst3` module の cdylib（`GetPluginFactory` + platform module hooks、`src/lib.rs:61-138`）。**公開 Rust API を持たず**（`lib.rs` doc: "the crate exports no public Rust API"）。依存の `vst3` crate（coupler-rs, 0.3.0）は raw COM bindings（`ComPtr`/`ComRef`/`Class`/`ComWrapper`）のみ — **host 側 loader API は含まれない**。
- `ruviform-plugin-ofx` — **ホスト側アダプタ**（唯一 host 方向）。`OfxHost::load_library` が real `.ofx` バイナリを dlopen し `Load`/`Describe`/`DescribeInContext` を private C bridge 経由で実行（`src/lib.rs:1-27`）。ただし生成物は **`VideoPluginInstance`（video・全操作 NONRT）** であり、audio 系 `PluginFactory→ErasedPlugin` 表面にも shm audio データプレーンにも載らない。
- **host 側 CLAP/VST3 アダプタ（実バイナリ → `ErasedPlugin` bridge）は workspace に存在しない**。`run(factories)` に実プラグインを食わせる部品そのものが未実装 — #114 の実質的な新規コードはこの **bridge + runner 構成** に集中する。
- ベースライン層ルール: `ruviform-plugin-clap`/`-vst3` の許可依存には `ruviform-plugin-core`・`-host`・`-export`（拡張層）が含まれる（`ruviform-architecture-design-baseline.md` クレート表 L273）— **アダプタ crate が runner bin を持つ・loader bridge を持つ配置は層ルール上可能**。`ruviform-plugin-export` の許可依存は `ruviform-plugin-core` のみ（adapter への依存は逆向き禁止）。

#### 実プラグインバイナリの在否（E2E fixture 供給源）

- **workspace build が実バイナリを生成する** — `cargo build --workspace` で `target/<profile>/ruviform_plugin_clap.dll`・`ruviform_plugin_vst3.dll` が cdylib として出力される（本 repo で実測確認済み）。CLAP は `clap_entry` export、VST3 は `GetPluginFactory` export を持つ**真の規格バイナリ**（dlopen + entry symbol で load 可能 — `.clap`/`.vst3` 拡張子改名や bundle 化は規格準拠の配布形だが、dlopen 自体は拡張子非依存）。
- **OFX 実 fixture バイナリは既に存在** — `ruviform-plugin-ofx/build.rs` が `tests/fixture/ofx_test_fixture.c` + `ofx_fixture_probes.c` を本物の shared library として `OUT_DIR` にコンパイルし `OFX_FIXTURE_DLL` rustc-env で export。`tests/bridge_native.rs`/`native_host.rs` がこれを実 ABI で駆動（in-process E2E は既存）。
- **third-party/ダウンロード取得のプラグインは repo に存在しない** — 全 fixture は workspace 内でビルド可能。これは「実バイナリ依存 = 外部取得」の前提を緩め、CI lane が opt-in を要するかどうかの論点になる。
- macOS 注意点: CLAP の規格準拠配布形は bundle（`.clap` bundle）であり、VST3 は platform 毎に module hook が分岐（`bundleEntry`/`ModuleEntry`/`InitDll` 実装済み・`vst3/lib.rs:69-138`）。raw cdylib の dlopen は技術的には可能だが、規格準拠ホストとしての発見経路（bundle/ディレクトリ構造）は別論点。

#### spawn/E2E ハーネス現状（再利用可能部品）

- host 側 E2E: `tests/l1_proxy_e2e.rs` の `runner_bin_serves_real_gain_plugin`（実 `ruviform-plugin-l1-runner` を `OutOfProcessSpec.program` に spawn → handshake → lifecycle → shm ブロック → state → Destroy）— **実 adapter runner E2E はこの形状の踏襲で成立する**。
- runner 実行ファイル解決: `runner_program()`（`l1_proxy_e2e.rs:1591-1609`）は `current_exe` → `target/<profile>/deps` → `target/<profile>/` の相対解決 — **Issue #123 の対象（`CARGO_BIN_EXE_*` cross-package 代替 or cargo metadata への置換）**。実プラグイン `.dll`/`.so` も同じ `target/<profile>/` に出るため同一の解決問題を共有する。
- export 側 E2E: `tests/l1_runner_e2e.rs` の `MiniHost`（shm 生成・stdio wire 会話）— 子側単体検証の再利用候補。

#### CI lane・opt-in 慣行

- `.github/workflows/ci.yml` は **`workflow_dispatch` 専用**（自動トリガーなし）。jobs: `verify`（3-OS matrix）・`rt-verify`・`icc-verify`・`bench`。
- feature-gated lane の先例: `icc-verify` job → `mise run test-video-proc-icc` / `lint-video-proc-icc`（`mise.toml:125-130` — `cargo test -p ruviform-video-proc --features icc`）。OFX 系は `test-ofx`/`test-ofx-bridge`/`lint-ofx`/`test-ofx-msrv`（`mise.toml:132-143`）。
- `#[ignore]` や env-gated skip のテスト慣行は**存在しない** — env var は spawn される子への fixture 制御（`RUVIFORM_PANIC_DESCRIPTOR`）としてのみ使用。`runner_program()` は `assert!(path.is_file())` で**欠落時 skip ではなく fail**。

#### Issue #123 との関係

- #123（OPEN）: `runner_program()` 解決・frame-I/O helper dedup・`gui_open` write-only field・audio-core doc 警告 — #76 由来の小粒残件集約。
- **重なり**: 実 adapter runner/プラグインバイナリの `target/<profile>/` 解決は同一問題 — #114 の E2E ハーネスは #123 が決める解決形に依存するか、同一 helper を必要とする（blocking ではないが ordering 注意）。
- **frame-I/O dedup**: 実 adapter runner が wire frame I/O を追加実装する場合に重複が増える可能性 — dedup 先に済ませると #114 の diff が縮む。

## Alternatives and Comparison

### 軸 1: runner バイナリのオーサリング面（`run(factories)` を exe 化する形）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 手書き `main` + factory 埋込み（現行 `ruviform-plugin-l1-runner` 踏襲 — adapter が `PluginFactory` を供給し runner bin は薄い `main` のみ） | API 変更不要・#76 確定形状と整合・per-standard bin で規約が単純 | プラグイン path/構成が compile-time 固定 — 実行時に binary を差し替える loader 形は取れない（fixture が変わる度に bin 追加） | candidate |
| (b) `run` argv 規約拡張（例: `--ruviform-*` 以外の未知引数の扱い変更 or `run_with_args(factories, argv)`/bin 側 pre-parse 用 API）+ `spec.args` でプラグイン path 受け渡し | 単一 generic runner bin で plugin binary を実行時差替え可能・`OutOfProcessSpec.args` が既に route 毎の引数経路を持つ | `run` の公開契約拡張が要（argv 規約は wire 契約の一部）。`spec.args`→runner 到達性の設計が要 | candidate |
| (c) 汎用 loader runner（export 提供・argv `--plugin-path` で dlopen） | bin 不要・単一 exe で規格横断 | #76 discovery 軸 4 (c) で「動的ロードの `unsafe`/ABI 管理が規約と衝突」として既に rejected — ABI 知識は adapter crate 責務であって export に置くと層が破れる | no（#76 判断継続） |
| (d) proc-macro/declarative `runner_main!` オーサリング | `main` 記述が 1 行 | `fn main() { run(vec![..]) }` が既に 1 行 — 新 API 面の見合いがない（YAGNI） | no |

### 軸 2: 子プロセス内での実プラグイン instantiate 経路

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) host 側規格アダプタ（child = mini-host）: 規格バイナリを dlopen → adapter が `ErasedPlugin`/`PluginFactory` へ bridge（CLAP は `clack-host`、VST3 は `vst3` COM 直駆動） | 「実プラグイン」の最強解釈 — 真の foreign バイナリが real ABI 経由で L1 wire に乗る。#76 discovery の想定形 | **bridge コードが未存在** — CLAP は `clack-host`（safe host API・dev-dep 昇格）で比較的小さく、VST3 は `vst3` が raw bindings のみで host 実装が大きい。foreign ロードに `unsafe`/libloading が要（scoped 許可 or crate 隔離） | candidate（CLAP 先行） |
| (b) adapter 制作物の中立側再利用（`GainProcessor` 等を `Plugin`/`PluginFactory` に wrap） | 追加コード最小・`unsafe` 不要 | wire から見ればモックと同型 — **「実プラグイン E2E」の issue 意図を満たさない**（規格 ABI・実バイナリ経路を一切通らない） | no（要件不適合） |
| (c) `load_from_clack` typed entry（in-process CLAP entry を bridge — 実 ABI・file load なし） | 実 CLAP ABI を通る・dlopen/path 解決不要で (a) より小さい | バイナリ単位の実証ではない（compile-time linked entry）。file-load E2E との併用で段階的証拠にはなる | maybe（(a) の段階として） |

### 軸 3: 規格別スコープと実施順序

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) CLAP 先行（`clack-host` 完備）→ VST3・OFX は条件付き/後続 | 最小差分で実バイナリ L1 E2E を確立・harness/runner 形を CLAP で実証してから横展開 | Issue 文言の CLAP/VST3/OFX 全列挙を一 Issue で完了しない — VST3/OFX の disposition（In scope or 後続 trigger）を明示する必要 | candidate |
| (b) 3 規格全部を本 Issue で E2E | Issue 文言の忠実な完遂 | VST3 host 実装（raw COM）が別 Issue 級の大仕事・OFX は video wire 非確立で構造的に不可 — 単一 Issue が巨大化し CLAP の価値も遅れる | no（OFX は物理的に不可能 — wire なし） |
| (c) harness/オーサリング面のみ確立・規格 E2E は各 adapter Issue へ | Issue が薄い | 「実プラグイン E2E」が Issue の中核要求であり、実プラグインなしの harness は動機を満たさない | no |

### 軸 4: OFX の disposition（video・NONRT・`VideoPluginInstance`）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 本 Issue 範囲外 + trigger 付き延期（video L1 wire は #113 design で非確立と確定済み） | 構造的事实と整合 — `VideoPluginInstance` は `ErasedPlugin`/`run(factories)`/shm audio 面に載らない。OFX の実バイナリ E2E は in-process で既に存在（`native_host.rs`） | Issue 列挙の OFX をカバーしない — spec に明示的な境界記録が要 | candidate |
| (b) lifecycle-only の子プロセス E2E（process/render なしで spawn→lifecycle→destroy のみ） | Issue の一部を形式的に満たす | wire verb は `ErasedPlugin` 表面に写像する設計 — `VideoPluginInstance` を wire に載せる新 surface が要で、実質 video wire の一部着手 | maybe（設計変更なら別 Issue 級） |
| (c) wire/video surface 拡張して OFX も process まで E2E | 完全カバー | #113 design 決定 `k`（L1 video wire 非確立）と正面衝突 — 単独 Issue の範囲を大きく超える | no |

### 軸 5: 実バイナリ依存 E2E の CI lane 設計

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 通常 `cargo test --workspace` lane に常時実行（binaries = workspace 生成物 — `runner_bin_serves_real_gain_plugin` が既に runner exe spawn を通常 lane で実行している先例） | 追加 CI 設計最小・回帰が常時検出・慣行（spawn E2E は既に通常 lane）と一致 | 実プラグイン load の安定性が未実証 — 規格 load 失敗時の flake が全体を壊す恐れ。Issue は「opt-in/手動」を明示要求 | candidate（fixture が workspace 生成なら） |
| (b) feature-gated lane（`l1-real-plugin` feature → `mise run test-…` + `icc-verify` 型の専用 job 追加） | Issue の opt-in 要求に整合・icc 先例に倣う形 | opt-in は default lane で silent regression を許す — binaries が workspace 生成物なら gating の技術的必要性は薄い | candidate |
| (c) env-gated skip（`RUVIFORM_…` env で skip 明示） | 最小コード | env-gated テスト慣行が repo に存在しない・silent skip は検証穴 | no（慣行なし） |
| (d) 別 workflow 分離 | 完全分離 | 単一 workflow_dispatch 運用の慣行と不整合・維持面が増える | maybe |

Issue 文言は「opt-in/手動」を前提するが、実 fixture が workspace 生成物であれば (a) の常時 lane が技術的根拠を持つ — **どちらを採るかは design 確定対象**（third-party バイナリを将来 E2E に使う場合は (b) が必須になる点は確定的）。

## Tentative Conclusions and Hypotheses

- **confirmed**: 「実プラグイン」は真の規格バイナリ（`.clap`/`.vst3` 形 cdylib or `.ofx` fixture）を指し、workspace build が CLAP/VST3 を・OFX build.rs が OFX fixture を既に生成する — **外部ダウンロード取得の fixture は不要**
- **confirmed**: host 側 CLAP/VST3 アダプタ（foreign binary → `ErasedPlugin` bridge）は**未存在** — #114 の実装中核は bridge + runner 構成 + E2E harness + CI lane であり、L1 wire/host/runner 基盤の変更は不要（#76 確定形状のまま）
- **confirmed**: `run()` の argv 規約は `--ruviform-*` のみ受理し未知引数を reject — loader 型 runner（実行時 plugin path 受け渡し）は `run` の契約拡張 or factory 埋込み構成の選択が要（design 論点）
- **confirmed**: OFX（`VideoPluginInstance`・NONRT・video wire 非確立）は現行構造上 L1 に乗せられない — Issue 列挙の OFX 部分は**範囲外化 + trigger 付き延期**が構造整合的（OFX 実バイナリ E2E は in-process で既存）
- **confirmed**: 実バイナリが workspace 生成物である以上、E2E は通常 `cargo test` lane で技術的に実行可能（runner exe spawn の先例あり）— opt-in lane は Issue 文言の要求であり、技術的必要性は design が判定
- **confirmed**: 子プロセス内プラグインの `Send` 性は非問題 — runner はシングルスレッドで `&mut dyn ErasedPlugin` を駆動し、`create_sendable` は L1 経路で不要求
- **confirmed**: `descriptor` handshake は `spec.descriptor` と子 `Scanned` の field-equality — アダプタが実バイナリ metadata から `PluginDescriptor` を合成する写像が bridge 設計の一部
- **hypothesis**: CLAP 先行が最小リスク — `clack-host` が安全な host API（`PluginEntry::load`/`PluginInstance`）を完備し、bridge は `ErasedPlugin` への薄い写像で済む見込み。VST3 は raw COM bindings のみで host 実装が別 Issue 級
- **deferred（Issue スコープ外として確認）**: ~~VST3 host adapter の本格実装~~（探索時点では別 Issue 級の見込み — **2026-10-26 ユーザー決定により本 Issue スコープに昇格**）・video L1 wire / OFX（#113 deferred 継続 — Issue #118 で追跡）・L1 GUI・third-party プラグイン取得 E2E・自動再起動

## Open Questions

> すべて design フェーズが答える論点 — architecture 未決ではなく実装設計の確定対象。
>
> **（2026-10-26 追記）** runner オーサリング形・規格別スコープ・CI lane 形の 3 項目はユーザー決定で確定（下記「ユーザー決定」節）。残項目は `docs/designs/issue-114-real-adapter-l1-e2e.md` で確定。

- runner オーサリング形の確定 — (a) 手書き `main` + factory 埋込み踏襲 vs (b) `run` argv 規約拡張（`spec.args` 経由の plugin path 受け渡し）vs その他（軸 1）→ **ユーザー決定で (a) compile-time 固定に確定**
- CLAP host 側 bridge の配置 — `ruviform-plugin-clap` 内（baseline 許可依存に `-export` を含む）vs 新規 crate vs runner bin 内
- `clack-host` の dev-dep → production dep 昇格 — 依存追加規約の理由記録に加え、`select-audio-device-backend` 型の採否記録（条件付き ADR）が要るか（design/plan フェーズの責務）
- VST3 の disposition — 本 Issue 内で E2E（host 実装を含む）か、CLAP で確立した構成への後続 Issue 化か（host 実装量の見積もりが前提）→ **ユーザー決定で本 Issue に含める**
- OFX の disposition — 範囲外 + trigger 付き延期の確定文言（video wire 前提の明記）か、何らかの lifecycle-only 形か → **ユーザー決定で範囲外 + Issue #118 追跡に確定**
- 実バイナリ E2E の CI lane 形 — 常時 lane（workspace 生成物前提）vs feature-gated opt-in（Issue 文言・icc 先例）vs third-party バイナリ前提の区別設計 → **ユーザー決定で専用 mise task + `#[ignore]` ゲートに確定**
- `spec.descriptor` の出どころ — アダプタ scan API から取得するか fixture 固定値か（handshake の field-equality 前提）
- file-load 経路の plugin binary path 解決 — `runner_program()` 同型の `target/<profile>/` 解決（#123 との共有論点）
- `.clap`/`.vst3` の規格準拠配布形（bundle/拡張子改名）を E2E で再現するか、raw cdylib dlopen で十分か

## ユーザー決定（2026-10-26 — briefing 後のスコープ確定・binding）

本探索の後、Issue #114 の実施スコープについて以下のユーザー決定があった。Alternatives の lean および Tentative Conclusions の hypothesis との差分は、確定内容が優先する（探索時点の推奨は当時の見積もりに基づく記録として残す）。

1. **規格スコープ = CLAP + VST3 両方** — host-side bridge crate/実装作業を両規格で本 Issue に含める。軸 3 (a)「CLAP 先行・VST3/OFX は条件付き/後続」の lean および「VST3 は別 Issue 級」の hypothesis を上書きする。**OFX は明示的に範囲外** — video・NONRT・`VideoPluginInstance` 構造上 L1 に載らないため Issue [#118](https://github.com/scarecrow173/ruviform/issues/118) で追跡（軸 4 (a) 相当 — spec 側で trigger 付き延期を明文化）
2. **runner オーサリング = compile-time 固定** — `ruviform-plugin-export::run(factories)` の static-registration 形を維持する。argv/`--plugin-path` による実行時動的ロード（軸 1 (b) `run` argv 規約拡張・(c) 汎用 loader runner）は rejected alternative として design に記録
3. **CI lane = 専用 mise task + `#[ignore]` ゲート** — 実バイナリ lane は専用 mise task + 手動/opt-in テストで運用する。別 workflow ファイルは作らない。常時 lane（軸 5 (a)）は採用しない — opt-in 形は Issue 文言・本決定と整合し、`#[ignore]`/`--ignored` が silent skip にならない明示的ゲートとして機能する（env-gated skip 慣行は repo に存在しない — 導入しない）

## Promotion Candidates

- [x] spec-doc: [issue-114-real-adapter-l1-e2e.md](../specs/issue-114-real-adapter-l1-e2e.md)（**created — proposed**。confirmed scope・受入基準を formalize）
- [x] adr-doc: **不要と判断** — runner 形態（export エントリ + adapter 構成 bin）は approved design `plugin-l1-out-of-process.md`（#76 決定サマリ `a`・#76 discovery 軸 4 採用案「外部プラグインもアダプタ経由で load 可能」）で確定済み、分離戦略は accepted ADR `plugin-isolation-strategy`、規格アダプタ方針は accepted `plugin-architecture-clap-vst3` + binding-method ADR で網羅。残る open 選択（オーサリング面・bridge 配置・規格順序・CI lane 形）は実装設計レベル — **briefing 時点で決定していない選択を ADR に記録しない**（#76/#99 と同型）。`clack-host` production 昇格等の依存採否が確定する場合は、先例 `select-audio-device-backend` 型の採否 ADR が design/plan フェーズの責務
- [x] design-doc: [issue-114-real-adapter-l1-e2e.md](../designs/issue-114-real-adapter-l1-e2e.md)（**created — draft**。Open Questions + ユーザー決定の確定を記録）

## Sources and Relations

- Issue [#114](https://github.com/scarecrow173/ruviform/issues/114)（本探索対象）・[#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED）・[#113](https://github.com/scarecrow173/ruviform/issues/113)（engine 統合・CLOSED — endpoint/graph 面が完成）・[#123](https://github.com/scarecrow173/ruviform/issues/123)（OPEN — `runner_program` 解決等の残件・path 解決論点を共有）・[#118](https://github.com/scarecrow173/ruviform/issues/118)（OFX/video 側の追跡先 — ユーザー決定）
- 下流文書: [issue-114-real-adapter-l1-e2e.md](../specs/issue-114-real-adapter-l1-e2e.md)（spec — ユーザー決定反映済み）・[issue-114-real-adapter-l1-e2e.md](../designs/issue-114-real-adapter-l1-e2e.md)（design — Open Questions 確定先）
- 一次文書: [plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（L272 deferred 行 — trigger・runner 形態確定）・[plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)（Out of Scope L97）・[plugin-l1-out-of-process.md](../discovery/plugin-l1-out-of-process.md)（軸 4 採用案 — adapter 構成・外部プラグイン load）・[engine-plugin-host-integration.md](../designs/engine-plugin-host-integration.md)（決定 `k` — L1 video wire 非確立・#114 参照 deferred）・[engine-plugin-host-integration.md](../specs/engine-plugin-host-integration.md)・[engine-plugin-host-integration.md](../discovery/engine-plugin-host-integration.md)・[ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)（adapter 層・許可依存 L273）・[plugin-architecture-clap-vst3.md](../adr/plugin-architecture-clap-vst3.md)（ホスト・制作双方の方針）・[plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md](../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md)・[ofx-host-video-contract-and-c-abi.md](../adr/ofx-host-video-contract-and-c-abi.md)・[ofx-host-adapter.md](../specs/ofx-host-adapter.md)（OFX host adapter・L1 前提としない）
- ローカル一次証拠: `crates/ruviform-plugin-export/src/lib.rs`（`run(factories)` 唯一の公開エントリ L124・argv 規約 L27-40/L317-381・未知引数 reject）・`src/bin/ruviform-plugin-l1-runner.rs`（mock factory 埋込みの手書き main L39-45）・`tests/l1_runner_e2e.rs`（`MiniHost` — 子側最小 host E2E）・`crates/ruviform-plugin-core/src/plugin.rs`（`PluginFactory` L204-237・`ErasedPlugin` L243-・`Plugin` surface L82-197）・`src/video.rs`（`VideoPluginInstance` NONRT L94-）・`crates/ruviform-plugin-host/src/composite.rs`（`OutOfProcessSpec` L65-101・descriptor handshake）・`l1/mod.rs`（spawn arg 順序 L1005-1011・`RtLeg`）・`tests/l1_proxy_e2e.rs`（`runner_bin_serves_real_gain_plugin`・`runner_program()` L1591-1609）・`crates/ruviform-plugin-clap/src/lib.rs`（`clap_entry` cdylib L76・`GainProcessor` 公開・`GainClapPlugin` private）・`src/plugin.rs`（`load_from_clack` typed entry L906/L968・`org.ruviform.clap-gain-poc`）・`Cargo.toml`（`clack-host` dev-dep L37）・`crates/ruviform-plugin-vst3/src/lib.rs`（`GetPluginFactory` L61・公開 API なし）・`crates/ruviform-plugin-ofx/src/lib.rs`（`OfxHost::load_library`）・`build.rs`（`ofx_test_fixture` 実バイナリ生成 → `OFX_FIXTURE_DLL`）・`tests/native_host.rs`/`bridge_native.rs`・`.github/workflows/ci.yml`（workflow_dispatch・`icc-verify` feature lane 先例）・`mise.toml`（`test-video-proc-icc`/`test-ofx` 等 L125-143）・`target/debug/ruviform_plugin_clap.dll`/`ruviform_plugin_vst3.dll`（workspace 生成物の実在確認）
- 外部一次証拠: `clack-host` 0.2.0 `PluginEntry::load(path)`（file-load/dlopen 経路 — crate registry src）・`vst3` 0.3.0（raw COM bindings のみ — host API なし）
