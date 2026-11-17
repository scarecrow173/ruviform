---
id: "DESIGN-ItrgZQG5ROZFAEevhfFR9H"
type: "design"
status: "approved"
title: "Issue #629 AC3 Test Runtime wiring design — dependency direction adjudication and verdict to RunMetadata mapping"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-629-test-runtime-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-test-runtime-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-test-runtime-wiring.md"
    - "docs/tasks/issue-629-test-runtime-wiring.md"
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-test-runtime-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
  references:
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  defers: []
  deferred-by: []
---

# Issue #629 AC3 Test Runtime wiring design

本 design は Issue `scarecrow173/ruviform#629` AC3 headless slice の
実装設計であり、**依存方向の裁定（ADR 相当の決定記録）**を含む。

## 1. 依存方向の裁定（決定記録）

### 背景

AC3 結線には「deterministic signal/Event を `ruviform-audio-test`
が供給し、本番 device adapter 入口へ投入する」経路が必要である。
供給側の型（`PcmFixture`・`MidiScenario`/`run_midi_trace`・
`RunMetadata`）は `ruviform-audio-test` にのみ存在し、被試験側の
入口（`DeviceEnumerator`/`Device`/`OutputStream`/`InputStream`、
MIDI port）は `ruviform-audio-device`/`ruviform-midi-device` に
存在する。両者を同一 test target で結線するには、どちらかの
crate が他方へ依存する必要がある。

### 候補と評価

| 候補 | 構造 | 評価 |
| --- | --- | --- |
| (禁止) `ruviform-audio-device`/`ruviform-midi-device` → `ruviform-audio-test` dev-dep | device test が audio-test を参照 | **却下（前回不承認の再確認）**。`ruviform-audio-test` は `ruviform-audio`/`plugin-bridge`/`graph` 系を `[dependencies]` に持ち、dev-dep でも `cargo test -p ruviform-audio-device` の compile graph にそれらが載る。device crate を production 境界として薄く保つ設計意図（`docs/designs/issue-629-wasapi-endpoint.md` の不承認記録）に反する。再提案不可。 |
| (採用) `ruviform-audio-test` `[dev-dependencies]` → `ruviform-audio-device` + `ruviform-midi-device` | audio-test の test target が device adapter を被試験側として参照 | **採用**。crate cycle なし（両 device crate は audio-test に依存しない）。dev-dep のため `ruviform-audio-test` の lib compile graph と、AL/SV/plugin-dock-test 等の consumer の dependency resolution を汚染しない（Cargo は依存 crate の dev-deps を build しない）。先例: `ruviform-audio-plugin-bridge` が `ruviform-midi-device` に dev-dep（`tests/midi_wiring.rs`）、`ruviform-plugin-test-vst3` が `ruviform-audio-test`+`ruviform-midi-device` に dev-dep（`tests/os_endpoint_midi_e2e.rs`）。 |
| (却下) dev-dep 不要の代替: device verifier が RunMetadata 準拠 JSON を自前 emit | schema を device test 側に複製 | **却下**。Case Schema v1 語彙（`CaseResult`/`VerificationLevel`/`EnvironmentKind`/`RunParameters`）の第二の権威が生まれ、serde 語彙との乖離リスクと保守コストが dev-dep 1 本の compile graph 拡大を上回る。AC3 の「Test Runtime 生成側の供給」要件（`PcmFixture`/`MidiScenario` の型を被試験側へ渡す）も満たせない — 供給側コードが audio-test に依存できないため。 |

### 裁定

**候補 (i) 逆方向を採用する。** `crates/ruviform-audio-test/Cargo.toml`
の `[dev-dependencies]` に `ruviform-audio-device` と
`ruviform-midi-device` を path 依存として追加し、device wiring
test を `crates/ruviform-audio-test/tests/` の新規 test target に
置く。両 device crate の `[dependencies]` への追加は不要
（test target のみが参照する）。`Cargo.lock` は path 依存の
edge 追加として再生成する。

### 制約と trade-off の記録

- `cargo test -p ruviform-audio-test` の compile graph に
  cpal（→ Windows=WASAPI / Linux=ALSA / macOS=CoreAudio）と
  windows-sys（WinMM）が載る。Linux CI lane は既に
  `libasound2-dev` を install 済み（workspace build で
  `ruviform-audio-device` が必要なため）であり、新規の外部
  crate・feature flag の追加はない。
- device crate の `src/`（production 契約）は一切変更しない。
  wiring は test target のみで行う。
- `ruviform-midi-device` は非 Windows で `Unsupported` を返す
  設計のため、3 OS の `cargo test -p ruviform-audio-test` で
  wiring test は全 OS で compile でき、OS endpoint leg は
  `#[ignore]` + fail-closed `NotRun` で振る舞う。
- AL/SV 等の consumer が `ruviform-audio-test` を dev-dep で
  消費する場合でも、device crate の dev-dep は consumer の
  build graph に現れない（dev-deps は推移しない）。

## 2. verdict→`RunMetadata` 対応付け

### Case 宣言

各 endpoint Case は以下の固定値で宣言する（Case Schema v1、
kebab-case serde 語彙）:

| Case | case_id | level | environment_kind | requirement |
| --- | --- | --- | --- | --- |
| audio render/capture loopback | `rf629.audio.virtual-endpoint.render-capture` | `ext-l3` | `virtual-os-endpoint` | `required` |
| MIDI port loopback | `rf629.midi.virtual-endpoint.loopback` | `ext-l3` | `virtual-os-endpoint` | `required` |
| PCM fixture→scripted backend | `rf628.pcm.fixture-block` | `os-l1` | `deterministic-backend` | `required` |
| MIDI scenario→trace | `rf628.midi.scenario-trace` | `os-l1` | `deterministic-backend` | `required` |

### verdict 写像

| verifier/test の終端状態 | `CaseResult` | 備考 |
| --- | --- | --- |
| endpoint pair が実際に open/stream し oracle/contract 全通過 | `pass` | `environment_kind=virtual-os-endpoint` のみ主張可 |
| stream/open/oracle/contract のいずれかが失敗 | `fail` | observed に失敗内容を記録してから Fail を返す |
| host/backend 非対応、endpoint 未列挙、pair 不在（NOT_PROVISIONED） | `not-run` | fail-closed。`Unsupported` にしない（環境が Case を表現不能な場合を除く） |
| 環境が Case を技術的に表現不能（例: MIDI backend が OS 未対応を返す） | `unsupported` | observed に理由を記録 |

### `RunMetadata` 構築

- `metadata.rs` に additive な `RunMetadata::for_environment`
  コンストラクタを追加する。`deterministic_backed()` は
  `DeterministicBackend`/`OsL1` 固定の既存経路であり、endpoint
  Case には level/environment を引数で受ける構築経路が必要な
  ため。result は `NotRun` で開始し、`with_result` /
  `with_expected` / `with_platform` / `with_evidence` で確定
  する。既存 API・serde 語彙・variant は不変。
- `parameters`: `RunParameters { seed, fixture_version,
  clock_config }`。endpoint leg は `VirtualClockConfig`
  （deterministic jitter 無し）を宣言値として記録する。
- `oracle`: 独立 oracle の識別子（audio: marker/RMS/contract
  系、MIDI: ordered-subsequence literal 系、deterministic:
  `oracle::pcm_reference_block` / `oracle::midi_expected_playthrough`）。
- `expected`/`observed`: 人間可読の照合サマリ（期待 marker
  数・observed PCM 統計・event 一致数等）。
- `repo`/`sha`: `RUVIFORM_RUN_REPO`/`RUVIFORM_RUN_SHA` 環境変数
  → `CI_COMMIT_SHA`/`GITHUB_SHA` → `"unknown"` の順で解決。
- emit: `RunMetadata::to_json` を `CARGO_TARGET_TMPDIR` 配下に
  `save_json` し、同時に JSON を stderr へ出力する
  （`--nocapture` evidence 採取用）。JSON の save/load は
  非 RT 経路（control thread）でのみ呼ぶ。

### honesty rule の実装上の対応

- name-match による pair 選択は candidate であり、
  `virtual-os-endpoint` Pass は実際に stream が開き oracle が
  観測した run に限る — verifier 現行契約を wiring test でも
  踏襲する。
- 未 provisioning（endpoint 列挙 0 件・pair 不在・host 非対応）
  は `not-run` を記録して正常終了する。`pass` を記録しない。
- deterministic leg は `deterministic-backend`/`os-l1` で記録し、
  endpoint leg の `virtual-os-endpoint`/`ext-l3` と混同しない。

## 3. deterministic signal/Event の供給

### audio leg（deterministic）

`PcmFixture`（`PcmPattern::Noise` 等 Exact domain）で生成した
`PcmBlock` を `oracle::pcm_reference_block` の独立参照と
`pcm_assert_eq` で照合し、結果を `RunMetadata`
（`deterministic-backend`/`os-l1`）へ記録する。
scripted backend（`ruviform_audio_test::backend`）の input
replay 経路を通すことで「Test Runtime 生成 → 本番 port 契約
入口 → 観測 → 独立 oracle」の typed 経路を canonical lane
（`mise run test ruviform-audio-test`）で実行可能にする。

### MIDI leg（deterministic）

`MidiScenario` → `run_midi_trace` で生成した trace を
`oracle::midi_expected_playthrough` の独立参照と照合し、
`RunMetadata`（`deterministic-backend`/`os-l1`）へ記録する。

### OS endpoint leg（`#[ignore]`、opt-in）

- audio: `ruviform_audio_device::default_host()` で列挙 →
  virtual-name/stem pair 選択（device verifier と同一規則、
  同期コメント付き）→ F32 format 交渉 → `open_output_stream`/
  `open_input_stream` → output callback で `PcmFixture::
  fill_view` が本番 `AudioBufferViewMut` を供給（RT-safe
  経路）→ capture channel-0 を bounded buffer へ → control
  thread で marker/RMS/contract oracle → `RunMetadata`
  （`virtual-os-endpoint`/`ext-l3`）。
- MIDI: `ruviform_midi_device::backend()` で port 列挙 →
  bidirectional name-match pair → `run_midi_trace` 由来の
  event 列を output port へ送信 → input drain → ordered
  subsequence + stamp contract で照合 → `RunMetadata`。

### 自己参照 Oracle 禁止

供給側は期待値生成（Fixture/scenario/trace）のみを担い、
observed 側の oracle は被試験 adapter の内部状態に依存しない
独立実装を維持する（device verifier の oracle と同じ規律）。

## 4. 既存資産の扱い

- `virtual_endpoint.rs`/`loopback.rs`/`endpoint_probe.rs`/
  `mise.toml` lane: 挙動・出力形式・終了コード・`#[ignore]`/
  feature gate を変更しない。module doc の stale な
  「typed 結線は後続」記述のみ同期する。
- `ruviform-audio-test` の `src/pcm.rs`/`src/oracle.rs`/
  `src/midi.rs`: 変更不要（既存 API で足りる）。
- `docs/**/README.md`: 生成物のため一切触れない。

## 5. リスク

- audio endpoint leg は device verifier の drive logic
  （negotiate/open/callback/Probe）を wiring test 側に最小限
  再構成する必要がある（device→audio-test dev-dep 禁止のため
  共有不可）。重複は同期コメントで管理し、深い tolerance
  oracle は device verifier に残す — wiring test の oracle は
  contract 級（marker/peak/monotonic/domain/non-finite/
  callback counts）に限定する。
- `#[ignore]` wiring test の canonical lane は本 slice 時点
  で未登録だった（`mise.toml` は本 slice 範囲外）。self-run
  は直接 cargo 実行の補助証拠として記録した — 後続の
  os-endpoint-lane slice（`docs/designs/
  issue-629-os-endpoint-lane.md`）で `mise run
  test-os-endpoint-wiring`（`cargo test -p
  ruviform-audio-test --test os_endpoint -- --ignored
  --test-threads=1`）として登録済み。
- provisioned 環境でのみ `pass` が出る設計のため、本ホストの
  self-run は `not-run` のみを記録する — これは仕様どおりで
  あり Pass の欠落ではない。
