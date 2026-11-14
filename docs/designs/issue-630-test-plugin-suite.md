---
id: "DESIGN-034XC4o1eLquthCFAcOrS8"
type: "design"
status: "approved"
title: "Issue #630 Test Plugin Suite — VST3 test_gain first slice 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-630-test-plugin-suite.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-630-test-plugin-suite.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks:
    - "docs/plans/issue-630-test-plugin-suite.md"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "crates/ruviform-plugin-vst3/README.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
  derived-by:
    - "docs/plans/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  references:
    - "evidence/rf630/verification.md"
  defers: []
  deferred-by: []
---

# Issue #630 Test Plugin Suite — VST3 test_gain first slice 設計

## Context

Issue #630 は Ruviform 本番 Plugin Host が実 ABI で扱える決定的テストプラグイン群を要求する。本設計はその first slice — VST3 `test_gain` — の構造を固定し、あわせて suite 全体の規約（将来の `test_instrument`/`test_stateful`/`test_fault`/`test_editor` と CLAP 版が従う命名・分割・CID・検証境界）を pin する。

制約の要点:

- **独立実装**: Issue #630 は「test plugin と host を同じ adapter 実装だけで組み立てる自己検証ループ」を禁止する。`ruviform-plugin-vst3` の内部 items は `pub(crate)` でそもそも到達不能であり、本 crate は `vst3` binding + neutral 契約（`ruviform-plugin-core`/`ruviform-audio-core`）の上に plugin 側実装を**直接**書く。両者が共有するのは `vst3` crate（SDK 相当の共通 binding 層）のみであり、これは design 上「SDK 相当の共通層」として記録する — plugin 固有の DSP・ライフサイクル・バス・パラメータ写像はすべて独立実装。
- **test-only**: cdylib 成果物は test fixture であり product distribution に含めない。production crate は本 crate に依存しない。
- **runtime 非依存**: `scarecrow173/ruviform#628` の `ruviform-audio-test` は未 merge — 依存 edge ゼロ。統合点は後続 slice。
- **RT**: process 経路は allocation/blocking/I/O なし。scratch は `setupProcessing` で事前確保。

## Scope

### In Scope

- crate `crates/ruviform-plugin-test-vst3`（`cdylib`+`rlib`、公開 Rust API なし）
- `test_gain` component/controller の VST3 実装（`IPluginFactory`/`2`/`3`、`IComponent`+`IAudioProcessor`+`IProcessContextRequirements`、`IEditController`）
- sample-accurate gain DSP と golden-vector テスト、in-process COM round-trip、本番 host path テスト、`rt_` no-alloc テスト
- suite 規約の pin（下記「Suite 規約」）

### Out of Scope

- `test_instrument`/`test_stateful`/`test_fault`/`test_editor`、CLAP crate、#628 runtime 統合、L1 runner、Native Editor、外部 SDK validator、CI lane、consumer oracle — すべて後続 slice（spec の Out of Scope と同一）

## Components and Boundaries

```
crates/ruviform-plugin-test-vst3/
├── Cargo.toml            # cdylib+rlib; deps: vst3, plugin-core, audio-core; dev-deps: assert_no_alloc, plugin-vst3(host)
├── README.md             # test-only 宣言・成果物の位置づけ
└── src/
    ├── lib.rs            # crate docs + GetPluginFactory + platform hooks + sdk_enum_i32
    ├── strings.rs        # copy_cstring/copy_wstring（factory/class-info の非 RT 経路）
    ├── gain.rs           # GAIN_PARAM + TestGainProcessor（neutral PluginProcessor、sample-accurate 区間適用）
    ├── block.rs          # ChannelScratch/EventScratch/prepare_block/finish_block/block_offset（RT 境界の testable core）
    ├── component.rs      # TestGainComponent（IComponent+IAudioProcessor+IProcessContextRequirements）
    ├── controller.rs     # TestGainController（IEditController、plain↔normalized 実写像）
    └── factory.rs        # TestPluginFactory（IPluginFactory+2+3、2 class 登録）
└── tests/
    └── host_path.rs      # 本番 host path 統合テスト（Vst3Library::load → ErasedPlugin）
```

境界:

- **ABI 境界**（component/controller/factory）: `unsafe` は COM 境界に局所化（`#[expect(unsafe_code)]` + SAFETY コメント）。binding 型は `pub(crate)` に留め公開 Rust API を持たない。
- **DSP 境界**（gain.rs）: `PluginProcessor` 実装。`EventList` の検証済みソート済みイベントを区間境界として gain を適用。COM 非依存で unit test 可能。
- **block.rs**: host `ProcessData` の生ポインタを検証済み zero-copy view へ変換する testable core。aliasing/bounce ポリシーは adapter と同じ不変条件（出力のみ bounce、部分 overlap は reject）を独立実装で持つ。
- **tests/host_path.rs**: crate の公開 API を必要としない — 自身の cdylib を `deps/` から解決して dlopen する consumer 側テスト。

## Data and Control Flow

### `test_gain` 意味論（決定的 law）

- パラメータ `Gain`（VST3 ParamID `0`）: plain domain `[0.0, 2.0]`・default `1.0`・unit `""`・`kCanAutomate`。normalized `[0.0,1.0]` ↔ plain は `ParamInfo::normalize`/`denormalize` の線形写像（normalized ≠ plain — host/plugin 双方の写像ミスを検出できる oracle 性を持たせる意図的選択）。
- processor の内部 gain は plain domain f64 で保持し、区間ごとに f32 へ cast して乗算。
- `process`: `EventList` を frame offset 昇順に走査し、イベント offset を境界に `[cursor, offset)` を現行 gain、`[offset, …)` を新 gain で処理。同一 offset の連続イベントは列挙順に適用され最後が勝つ（`EventList` の last-wins と一致）。offset 0 のイベントはブロック全体に即時適用。フレーム `n` は `offset <= n` の最後のイベント値を使う — **offset は影響を受ける最初のフレーム**（off-by-one の定義をここに固定）。
- ゼロフレームブロック: イベントのみ flush（offset 0 のみ合法）— gain 状態は次ブロックへ持続。
- bus: `getBusInfo` は audio in/out 各 1 本・`kMain`・channelCount 2（stereo）を宣言。`setBusArrangements` は in==out の `kStereo`/`kMono` を受理。`process` は bus の `numChannels` ∈ {1,2} を受理し `min(in, out)` ch を処理 — law はチャンネル数非依存（n-ch への拡張は bus 宣言側の後続課題）。
- transport/processContext: `test_gain` は消費しない — `IProcessContextRequirements` は 0 を返し、`PluginProcessor::process` には `None` を渡す（Tempo/Loop は後続 class の責務）。
- MIDI: `inputEvents` は読まない（`test_instrument` の責務）。
- State: component `setState`/`getState`、controller `setComponentState`/`setState`/`getState` は `kNotImplemented`（State Save/Reload は後続 class — 偽の成功を返さない）。
- plugin→host reports: `drain_output_reports` は default no-op（report 経路の検証は後続 slice）。

### COM ライフサイクル（adapter と同一の中立 `Lifecycle` 写像）

`initialize`→`Initialize`、`setActive(true)`→`Activate`、`setupProcessing`→`prepare`+scratch 確保（`Activated` のみ合法）、`setProcessing(true)`→`StartProcessing`（prepare 済み必須）、`process`→`Process` self-loop（`Processing` のみ合法・違反は `kResultFalse`）、`setProcessing(false)`→`StopProcessing`、`setActive(false)`→`Deactivate`、`terminate`→`Destroy`。プロトコル違反はすべて defined reject で UB/panic を FFI 越しに漏らさない（`try_borrow_mut`）。

### 検証経路（2 系統・独立）

1. **In-process COM round-trip**（crate 内 `#[cfg(test)]`）: `GetPluginFactory()` を直接呼び、`countClasses`/`getClassInfo*`/`createInstance` で component+controller を instantiate し、手組み `ProcessData`（`IParameterChanges` queue 付き）で golden PCM を process。adapter host bridge を経由しない plugin 側 ABI の直接検証。
2. **本番 host path**（`tests/host_path.rs`、dev-dep `ruviform-plugin-vst3` feature `host`）: `Vst3Library::load` が自 crate の cdylib を dlopen → `descriptors()`/`factories()` → `create()`/`init()`/`prepare()`/`process_host()`/teardown。host bridge が生成する `ProcessData`・`IParameterChanges`・bus 構造は production code が作る — plugin が host の実出力を正しく処理できることの証明。

## Suite 規約（後続 slice が従う pin）

- **1 module = 複数 class**: 1 つの `.vst3`/`.dll` が複数 plugin class を host する。`test_gain` は component+controller の 2 class を 1 module に収める。将来の `test_instrument`/`test_stateful`/`test_fault`/`test_editor` は同 module への class 追加か、責務分割が必要になった時点で別 module を検討する。
- **命名**: vendor `"Ruviform Test"`、component 名は `test_<role>` 形式、controller 名は `test_<role> Controller`。factory vendor/email/url は `"Ruviform Test"`/`ruviform-test@example.invalid`/repo URL。
- **CID**: class ごとに一意な `uid(...)` TUID を発行。adapter fixture の CID と衝突しないこと。同一 host scan で `"Ruviform"`（fixture）と `"Ruviform Test"`（本 suite）が vendor で区別される。
- **format 別クレート分割**: VST3 は `ruviform-plugin-test-vst3`、CLAP は将来 `ruviform-plugin-test-clap`。DSP law が format 非依存な部分は将来共通化を検討するが、本 slice では各 crate が独立実装（YAGNI — 実際に 2 format 目が来るまで抽象化しない）。
- **#628 runtime 統合点**: `ruviform-audio-test`（Test Runtime）が merge された後、golden vector・case 実行・oracle 比較を runtime API へ載せ替える。plugin crate 自体は runtime 非依存を維持する。
- **外部 SDK validator**: Steinberg VST3 validator 等の実行は License・環境確保後の別 slice。実施までは「adapter 自己検証のみ」を証拠に明記し外部 ABI 適合を主張しない。
- **証拠語彙**: #627 の `environment_kind=test-plugin`・Case Schema v1・Test Depth に従い、実行した case のみ Pass と記録。L0 in-process の成功を L1 out-of-process・CLAP・実機の Pass と読み替えない。

## Non-Functional Constraints

- **RT**: `process` は allocation/dealloc/blocking/I/O/unbounded work なし。channel/event scratch は `setupProcessing` で確保。`rt_` テストが `assert_no_alloc` で機械検証。
- **unsafe**: COM 境界のみ。`#[expect(unsafe_code)]` + SAFETY コメントの局所許可。`unsafe_code = "warn"` → `-D warnings` ゲート。
- **lint**: workspace lints（pedantic clippy 含む）`-D warnings` 通過。公開 enum を持たないため `#[non_exhaustive]` 対象なし。`#[allow]` による lint 隠蔽なし。
- **依存**: `vst3 = "0.3.0"`（adapter と同一版 — SDK 相当の共通層として pin）、path deps は `ruviform-plugin-core`/`ruviform-audio-core` のみ。`ruviform-media-core` は直接参照しない（`EventKind` 経由の推移的依存）。dev-deps: `assert_no_alloc = "1.1"`、`ruviform-plugin-vst3`（`features = ["host"]`、dev-dep edge のみ）。
- **MSRV**: workspace `rust-version`（1.89）・edition 2024 に従う。

## Risks and Trade-offs

- **adapter との実装類似**: block.rs の aliasing/bounce・component の lifecycle 写像は adapter と同じ不変条件を持つが、コードは独立実装（共有は neutral 契約と `vst3` binding のみ）。「独立実装」の要件は plugin 固有ロジックの独立性であり、SDK binding 層の共有は許容と解釈 — design doc に記録済み。残リスク: binding 層の共通バグは両者に波及する → 外部 SDK validator が後続 slice でこの残リスクを潰す。
- **sample-accurate 適用の複雑さ**: 区間 walk は単純だが off-by-one を誤ると oracle として無価値 → golden test で offset 境界（block 中間・offset 0・ゼロフレーム flush・同一 offset 連続）を明示的に固定。
- **cdylib の test 解決**: `deps/` 相対解決は adapter 先例踏襲。`cargo test -p` で cdylib が生成されることを実測で確認する（未確認なら evidence に NotRun 記録）。
- **host bridge の channelCount 依存**: host は `getBusInfo.channelCount` で bus を構築し `setBusArrangements` を呼ばない — plugin は `process` の `numChannels` を権威として扱う設計にした（宣言 2ch・受理 1..=2ch）。

## Deferred Design Concerns

- `test_instrument`/`test_stateful`/`test_fault`/`test_editor` の class 設計 | 本 slice は `test_gain` のみ | 各 class の後続 slice | suite 規約（命名・module 構成）は pin 済みのため着手可能
- CLAP crate の DSP law 共通化 | 2 format 目が存在しない | `ruviform-plugin-test-clap` slice | 現時点では各 crate 独立実装（YAGNI）
- `IProcessContextRequirements`/transport・MIDI・plugin→host reports・State stream の実装 | `test_gain` の law に不要 | 対応 class の slice | `kNotImplemented`/no-op で正直に応答済み
- 8ch 超の multichannel bus 宣言 | VST3 `SpeakerArrangement` が任意 ch を表現しない | multichannel 要求が出た slice | process law は ch 非依存のため DSP 側は対応済み

## Verification Notes

- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `build`（`ruviform-plugin-test-vst3`）— 実施結果は `docs/impl/ir` + `evidence/rf630/verification.md` に記録
- [ ] `rt_` テストが `cargo test --workspace rt_` で実際に収集・実行されたことをテスト名で確認
- [ ] in-process COM round-trip と本番 host path の両経路で golden PCM が一致
