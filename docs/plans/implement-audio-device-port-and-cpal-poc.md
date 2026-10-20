---
id: "PLAN-034JgkWCimxdvr81k6b0ow"
type: "plan"
status: "approved"
title: "オーディオデバイス境界ポートと ruviform-audio-device PoC の実装（Issue #23）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-core/src/device.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio-device/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/host.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/device.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/convert.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/error.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/tests/mock_backend.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/examples/list_devices.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/examples/play_sine.rs"
      - type: "file"
        path: "docs/impl/exp/*.md（実験ログ。new_experiment_log.js で生成）"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/buffer.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/adr/select-audio-device-backend.md"
      - type: "file"
        path: "docs/designs/overview.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/audio-device-backend-evaluation-poc.md"
  implemented-by:
    - "../tasks/implement-audio-core-device-port.md"
    - "../tasks/implement-audio-device-cpal-adapter.md"
    - "../tasks/run-device-poc-examples-and-exp-log.md"
    - "../tasks/finalize-backend-adr-and-sync-docs.md"
  depends-on:
    - "apply-issue-1-architecture-baseline-to-repo.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/select-audio-device-backend.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../discovery/evaluate-audio-device-backends.md"
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../../mise.toml"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/audio-device-backend-evaluation-poc.md"
    - "../designs/audio-device-boundary-and-poc.md"
    - "../adr/select-audio-device-backend.md"
    - "../discovery/evaluate-audio-device-backends.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/audio-device-port-and-cpal-adapter.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "https://github.com/RustAudio/cpal"
  defers: []
  deferred-by: []
---

# オーディオデバイス境界ポートと ruviform-audio-device PoC の実装（Issue #23） Implementation Plan

## Goal

approved 設計 `DESIGN-034JcC9QK924kDbu4WhibC` どおり、`ruviform-audio-core` にデバイス境界ポート（`device` モジュール: `DeviceEnumerator` / `Device` / `OutputStream`・値型・`DeviceError`）を定義し、`ruviform-audio-device`（cpal 0.18 adapter）を新設して「デバイス列挙 → 既定出力構成 → 出力ストリーム開始 → コールバック到達（planar f32 境界変換）」の PoC を実証し、採否 ADR を `accepted` 昇格または条件見直しで確定する（spec `SPEC-034Jc9QEqYLVfg1CXXTiPV` の Acceptance Criteria 全件を充足）。

## Scope

単一の実装ストリームとして扱う。ポート定義（audio-core）→ adapter 実装（ruviform-audio-device）→ 実機 PoC と実験ログ → ADR 確定が同一クリティカルパス上にあり、後段は前段の成果物なしに検証不能なため plan 分割はしない。範囲外（spec Out of Scope・設計 Deferred Design Concerns と一致）: 入力/duplex 実装（`output_*` 命名で追加余地のみ確保）、engine 層 `ruviform-audio` のセッション調停・RT 配線、`RtContext` mint、CI 実機ループバック、ASIO、cubeb/rtaudio/直結 adapter、F64・整数フォーマット経路、任意チャンネル数の `ChannelLayout` variant。

## Upstream Documents

- [../specs/audio-device-backend-evaluation-poc.md](../specs/audio-device-backend-evaluation-poc.md) — `SPEC-034Jc9QEqYLVfg1CXXTiPV`（proposed）。本 plan が実装する spec。Issue #23 の作業範囲・受け入れ基準の正本
- [../adr/select-audio-device-backend.md](../adr/select-audio-device-backend.md) — `ADR-034Jc9RslnH7QSQ1nZEEHL`（proposed・cpal 条件付き採用）。PoC 結果で Confirmation/Verification 項目を確認し `accepted` 昇格または差し戻す確定対象
- [../discovery/evaluate-audio-device-backends.md](../discovery/evaluate-audio-device-backends.md) — `DISC-034Jc8wchRH37hqfhNLRPJ`（resolved）。評価軸・候補比較・Open Questions（実機で確認すべき未確定点）の正本
- [GitHub Issue #23](https://github.com/scarecrow173/ruviform/issues/23) — 一次ソース（`relations.source`）

## Design Inputs

- [../designs/audio-device-boundary-and-poc.md](../designs/audio-device-boundary-and-poc.md) — `DESIGN-034JcC9QK924kDbu4WhibC`（approved）。本 plan の**実装正本**: ポート trait/値型/エラーの完全シグネチャ、ErrorKind→DeviceError 写像表（制御経路/err_cb 別）、コールバック配管（scratch・chunk 分割・zero-fill）、F32 交渉方針、manifest/feature 方針、検証設計、ADR 確定計画はすべて同書が定義する。本 plan はシグネチャを再記述せず工程・ファイル・検証へ分解する
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。責務表（ポート定義所有は audio-core・adapter 依存方向）・RT 契約・分類表の根拠
- [../adr/media-io-backend-abstraction.md](../adr/media-io-backend-abstraction.md) — 外部型・エラー・feature 名の非露出、分離クレート＋opt-in、実行時文字列選択禁止の抽象境界規則
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — コールバック内 RT 契約・API 単位 `**RT contract:**` ラベル規則
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — 全公開 enum の `#[non_exhaustive]` 強制（lint ゲート）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-audio-core/src/device.rs` | ポート定義の正本: `DeviceEnumerator`/`Device`/`OutputStream` 3 trait・値型（`OutputStreamConfig`/`BufferSize`/`SupportedBufferSize`/`SupportedOutputFormat`/`DeviceId`/`OutputTimestamp`/`OutputCallbackInfo`/`DEVICE_CLOCK_TIMEBASE`）・コールバック型（`OutputDataCallback`/`OutputErrorCallback`）・`DeviceError`/`DeviceResult` | 新規。全公開 enum `#[non_exhaustive]`・メソッド毎 `**RT contract:**` ラベル |
| `crates/ruviform-audio-core/src/buffer.rs` | `AudioBufferViewMut::try_new(data: &mut [S], channels: usize, frames: usize) -> Result<Self>` 追加 | 借用のみ・非確保・RT-suitable。`channels==0` または長さ不一致で `Err(Error::InvalidValue)` |
| `crates/ruviform-audio-core/src/lib.rs` | `pub mod device;` 公開（crate root へのフラット再エクスポートはしない。名前空間 `ruviform_audio_core::device::*`）+ crate doc 更新 + `#[cfg(test)]` 契約テスト | device モジュールのみ名前空間付き公開という設計判断 |
| `crates/ruviform-audio-device/Cargo.toml` | 新クレート manifest。`cpal = "0.18"` 直接依存（optional+feature 方式は却下済み）+ `ruviform-audio-core`/`ruviform-media-core` path 依存。features: `default = []`、`realtime = ["cpal/realtime"]`、`pipewire-backend = ["cpal/pipewire"]`。dev-dep `assert_no_alloc`（試行） | feature 名に固有名詞を出さない（意図命名）。`publish.workspace = true`・`[lints] workspace = true` |
| `crates/ruviform-audio-device/src/lib.rs` | crate doc（責務・許可依存層・RT 契約転記）+ 公開面 re-export | 公開面にも cpal 型を出さない |
| `crates/ruviform-audio-device/src/host.rs` | `Host`（private `cpal::Host`）→ `DeviceEnumerator` 実装、`HostId` opaque newtype、`default_host()`/`available_host_ids()`/`host(id)` | cpal API 形状は実装時確認項目 |
| `crates/ruviform-audio-device/src/device.rs` | cpal `Device` → `Device` 実装（`name`/`id`/`default_output_format`/`supported_output_formats`/`open_output_stream`） | F32 優先交渉・`Err(UnsupportedConfig)` 規則 |
| `crates/ruviform-audio-device/src/stream.rs` | cpal `Stream` → `OutputStream` 実装 + コールバック配管（`DataCallbackState`・scratch `Vec<f32>`・`try_new` per-callback view・chunk 分割・zero-fill・`OutputTimestamp` 構成） | data/err クロージャに所有権分割（`Arc`+`Mutex` 共有禁止） |
| `crates/ruviform-audio-device/src/convert.rs` | 純粋関数: `map_sample_format`/`layout_for_channels`/`negotiate`/interleave 書込ヘルパ | 単体テスト可能に分離 |
| `crates/ruviform-audio-device/src/error.rs` | `map_error(e: &cpal::Error, ctx: ErrorCtx) -> DeviceError` 1 関数集約（設計の写像表どおり ctx 別） | wildcard arm は `Backend("other")` |
| `crates/ruviform-audio-device/tests/mock_backend.rs` | モックバックエンド結合テスト（非 cpal 実装でポート実装可能性を証明） | ハードウェア不要 |
| `crates/ruviform-audio-device/examples/list_devices.rs` | ホスト/デバイス/既定構成の列挙ダンプ | 手動検証用（CI 非実行） |
| `crates/ruviform-audio-device/examples/play_sine.rs` | 既定出力へ sine 送出。コールバック観測は `AtomicUsize`/`AtomicU64` のみ（回数・min/max フレーム・xrun・最終 `OutputTimestamp`） | 手動検証用。`select_f32_format` 補助関数を同梱 |
| `Cargo.toml`（root） | `members` に `crates/ruviform-audio-device` 追加 | 帳簿同期 |
| `Cargo.lock` | cpal・assert_no_alloc 依存の記録 | 追跡継続（依存追加理由は PR/実験ログに記録） |
| `AGENTS.md` | ワークスペース構成記述へ ruviform-audio-device を追記 | 帳簿同期 |
| `docs/impl/exp/*.md` | PoC 実験ログ（環境・手順・観測値・判定・依存追加理由・試行の限界） | `new_experiment_log.js` で生成 |
| `docs/adr/select-audio-device-backend.md` | `status: proposed → accepted` 昇格、または再評価トリガ観測時は条件見直し版の起票へ差し戻し | Phase D の確定対象 |
| `docs/designs/overview.md` | 必要に応じて索引を同期 | 帳簿同期 |

## Implementation Sequence

### Phase A: audio-core device ポート定義（タスク `implement-audio-core-device-port`）

- [ ] A-1. `buffer.rs` に `AudioBufferViewMut::try_new` を実装し、正常構成・`channels==0`・長さ不一致の単体テストと `copy_to_interleaved` 結合テストを書く（テストを先に書き red 確認 → 実装 → green）
- [ ] A-2. `device.rs` に `DeviceError`/`DeviceResult`（`Copy`+`&'static str` payload・`Display`/`core::error::Error` 実装）と全値型・コールバック型・3 trait を設計正本のシグネチャどおり定義する。`DEVICE_CLOCK_TIMEBASE` は `TimeBase::new` が const-eval 可能か実装時に確認し `const`/`fn`/`OnceLock` の形を決める
- [ ] A-3. 全公開 API に `**RT contract:**` ラベルを付け、`lib.rs` に `pub mod device;` と crate doc を追加する。object safety（`Box<dyn DeviceEnumerator/Device/OutputStream>` 構成可能）・trait object 経由 `Send`・`OutputDataCallback` への HRTB クロージャ強制変換・`DeviceError: Copy`・全公開 enum `#[non_exhaustive]` をコンパイルレベルで検証する `#[cfg(test)]` を書く。doctest はモック構成または `no_run`/`ignore` でハードウェア非依存に保つ

### Phase B: ruviform-audio-device cpal adapter（タスク `implement-audio-device-cpal-adapter`）

- [ ] B-1. `crates/ruviform-audio-device/Cargo.toml` を作成しルート `members` に追加（`cpal = "0.18"`・path 依存×2・features・`publish.workspace = true`・`[lints] workspace = true`・dev-dep `assert_no_alloc`）。`lib.rs` に crate doc（責務・許可依存層・RT 制約転記）を書く
- [ ] B-2. `error.rs` の `map_error(e, ctx)` を設計の ErrorKind 写像表どおり実装し、`ErrorKind` が値構成可能なら全分岐の単体テスト、不可能なら `DeviceError` 等価性・`Display` のテストを書く（実装時確認でフォールバック判断）
- [ ] B-3. `convert.rs` の `map_sample_format`（F32→F32・F64→F64・他 `None`）・`layout_for_channels`（1/2/4/6/8 のみ）・`negotiate`・interleave 書込ヘルパ（chunk 分割・残尾/失敗時 zero-fill）を純粋関数として実装し合成入力で単体テストする
- [ ] B-4. `host.rs`/`device.rs`/`stream.rs` を実装する（`Host`/`HostId` opaque 化・F32 優先 `default_output_format`・`DataCallbackState` の move 所有・scratch `min(effective_max, FRAMES_CAPACITY_HARD_CAP)` ・`StreamInstant::as_nanos` 主経路＋飽和時 epoch 差分予備経路）。cpal API 差異は `cargo doc`/ソースで確証する
- [ ] B-5. `tests/mock_backend.rs`（フェイク `Device`/`OutputStream` で enumerate→default→open→スクリプト発火 callback→start/stop→`now` を `Box<dyn>` 経由で検証）と、合成 `&mut [f32]` への interleave 書込経路テスト（`n>frames_capacity` chunk 経路含む）、`rt_` プレフィックスの `assert_no_alloc` 試行テストを書く

### Phase C: PoC example・実機手順・実験ログ（タスク `run-device-poc-examples-and-exp-log`）

- [ ] C-1. `examples/list_devices.rs` を実装する（`available_host_ids`・デバイス列挙・既定デバイス・`default_output_format`・`supported_output_formats` のダンプ）
- [ ] C-2. `examples/play_sine.rs` を実装する（`select_f32_format` フォールバック付き既定出力 open → `start` → 数秒送出 → `stop` 後に非 RT 側で `AtomicUsize`/`AtomicU64` 観測値を要約出力）
- [ ] C-3. 実機で `cargo run -p ruviform-audio-device --example list_devices` と `--example play_sine` を実行し、`docs/impl/exp/` に `new_experiment_log.js` で実験ログを作成して環境（OS/ホスト/cpal バージョン）・手順・観測値・判定を記録する（WASAPI xrun 未報告・`realtime` が Windows/Android 限定・PipeWire 既定非列挙の注意点も記録。任意: `--features pipewire-backend`/`realtime`）

### Phase D: ADR 確定・帳簿同期・総合検証（タスク `finalize-backend-adr-and-sync-docs`）

- [ ] D-1. 設計「ADR 確定計画」の対応表どおり、ADR `Verification`/`Confirmation` 項目を実験ログ・テスト証跡と対応付けて確認し、`select-audio-device-backend.md` を `proposed → accepted` へ昇格する（再評価トリガ観測時は昇格保留・条件見直し版を起票する判断を記録）
- [ ] D-2. `AGENTS.md` のワークスペース構成記述・ルート `Cargo.toml` のメンバーコメント・必要に応じ `docs/designs/overview.md` 索引を同期する
- [ ] D-3. 総合検証を実行する（検証マトリクス V-4 の全項目）

## Dependencies and Risks

### Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| Phase A（audio-core ポート） | — | Phase B |
| Phase B（cpal adapter） | Phase A | Phase C |
| Phase C（example・実機・実験ログ） | Phase B | Phase D |
| Phase D（ADR 確定・同期・総合検証） | Phase A・B・C | — |

- クリティカルパス: A → B → C → D の直列。並行可能な作業は存在しない（B はポート型・`try_new` に、C は adapter 実装に、D は PoC 証跡に依存）
- `ruviform-audio-device` は `ruviform-audio-core`（→`ruviform-media-core`）にのみ依存。adapter→adapter・core→adapter の依存は作らない

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | データ損失リスク: なし（新規ファイル追加と additive 変更のみ・永続データ/既存 API の破壊なし） | — | — | — |
| 2 | 破壊的変更: `AudioBufferViewMut::try_new`・`device` モジュール追加は additive。`publish = false` で外部公開面なし | low | low | workspace 内影響のみ。revert で復帰可能 |
| 3 | 外部依存の利用可否: cpal 0.18.x の MSRV/edition-2024 適合が公開情報ベースで未確証 | med | high | B-1 で `cargo` による実測を最初に行い、不適合なら設計差し戻し（`cargo doc`/lock で確認） |
| 4 | cpal API 形状差異（設計の「要実装時確認」多点: `description()` vs `Display`・`id()`・`available_host_ids`・`StreamInstant`・`negotiate` 戻り型・`ErrorKind` 構成可能性） | high | med | 各ステップで `cargo doc`/`cargo` エラー・ソース確認を義務化。乖離は実験ログに記録し設計書フォローアップ対象とする |
| 5 | `Send`/`Sync` の backend ばらつき（PipeWire `!Send` リスク含む）で adapter 実装が成立しない | med | high | trait は `Send` のみ要求の設計済み。`pipewire-backend` は opt-in 任意検証。成立しない backend は実験ログに記録し ADR 条件へ反映 |
| 6 | `BufferSize::Fixed` が backend に尊重されず可変フレームが常時発生 | med | low | 設計の chunk 分割経路が構造的に吸収。実機で観測・記録 |
| 7 | `assert_no_alloc` 統合が環境都合で失敗 | med | low | コードレビュー証跡＋コールバック経路に確保 API が無い静的 grep 証跡へフォールバック（設計の検証方針どおり） |
| 8 | Linux で `libasound2-dev`（必須）/`libpipewire-0.3-dev`（任意）欠如でビルド不可 | med | med | 環境前提を task・実験ログに明記。CI は現状手動 dispatch のみで実機不要テストが通ることを確認 |
| 9 | ASIO `build.rs` の SDK 自動ダウンロードによる再現性低下 | low | low | ASIO feature は既定無効・配線しない。`CPAL_ASIO_DIR` 回避を実験ログの再現性注意として記録のみ |

## Task Breakdown

| Task | 内容 | 対応 Phase |
| --- | --- | --- |
| [implement-audio-core-device-port.md](../tasks/implement-audio-core-device-port.md) | `AudioBufferViewMut::try_new` 追加 + `device` モジュール（ポート trait・値型・コールバック型・`DeviceError`）定義 + 契約テスト | A |
| [implement-audio-device-cpal-adapter.md](../tasks/implement-audio-device-cpal-adapter.md) | `ruviform-audio-device` 新設 + manifest/feature + `map_error`/convert 純粋関数 + `Host`/`Device`/`OutputStream` 実装 + モック結合・interleave・`rt_` alloc テスト | B |
| [run-device-poc-examples-and-exp-log.md](../tasks/run-device-poc-examples-and-exp-log.md) | `list_devices`/`play_sine` example + 実機手動実行 + `docs/impl/exp/` 実験ログ記録 | C |
| [finalize-backend-adr-and-sync-docs.md](../tasks/finalize-backend-adr-and-sync-docs.md) | ADR `accepted` 昇格/差し戻し + `AGENTS.md`/`Cargo.toml`/`overview.md` 同期 + 総合検証 | D |

## Verification

| Step | Verification command or criteria | Pass condition |
| --- | --- | --- |
| Phase A | `mise exec -- cargo test -p ruviform-audio-core` | exit 0。object safety/`Send`/HRTB 強制変換・`try_new`・`copy_to_interleaved` 結合テスト合格 |
| Phase A | `mise exec -- cargo clippy -p ruviform-audio-core --all-targets -- -D warnings` | exit 0・warnings 0（`exhaustive_enums` ゲート含む） |
| Phase B | `mise exec -- cargo test -p ruviform-audio-device` | exit 0。`map_error` 写像・convert 純粋関数・モック結合・interleave chunk 分割テスト合格 |
| Phase B | `mise run test-rt` | exit 0。`rt_` プレフィックスの `assert_no_alloc` 試行テストが選択・合格（統合不能時はフォールバック証跡を実験ログに記録） |
| Phase B | `mise exec -- cargo metadata --format-version 1` | cpal が `ruviform-audio-device` の `[dependencies]` にのみ存在し、コアクレートに波及しないことを確認 |
| Phase B | `mise exec -- cargo check -p ruviform-audio-core` | exit 0（audio-device 無効構成でコアが型チェック可能） |
| Phase C | `cargo run -p ruviform-audio-device --example list_devices`（実機・手動） | ホスト・デバイス・既定構成がダンプされ、結果が実験ログに記録される |
| Phase C | `cargo run -p ruviform-audio-device --example play_sine`（実機・手動） | コールバック到達・min/max 実フレーム・xrun・最終 `OutputTimestamp` が観測され、実験ログに記録される |
| Phase D | `mise exec -- cargo doc -p ruviform-audio-core` 出力レビュー | 公開面に cpal 由来の型・エラー・feature 名が存在しない |
| Phase D | `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench` | 全 exit 0 |
| Phase D | `mise exec -- prek run --all-files` | exit 0 |
| Phase D | `docs/adr/select-audio-device-backend.md` の status | `accepted` 昇格、または再評価トリガ観測時は保留判断と条件見直し版起票の記録がある |

振る舞いレベルの保証（ポート契約・エラー写像・コールバック配管の不変条件）は [test spec](../test-specs/audio-device-port-and-cpal-adapter.md) に委譲し、本表は step ごとの検証入口のみ管理する。

## Rollback Strategy

| 対象 | Trigger | Procedure | Verification |
| --- | --- | --- | --- |
| 変更全体（新規クレート＋audio-core 追加＋docs） | PoC 失敗・設計の根本見直し・ADR 差し戻し | 対象 feature commit/merge を `git revert`（`crates/ruviform-audio-device/` ディレクトリ削除・root `members` 除去・`Cargo.lock` 再生成を含む） | `mise run check` exit 0・`cargo metadata` に ruviform-audio-device が存在しない |
| audio-core 側のみの部分戻し | adapter 側を残してポートを手直しする場合 | `crates/ruviform-audio-core/src/device.rs`・`buffer.rs` の `try_new` 差分・`lib.rs` 差分をファイル単位で revert | `mise exec -- cargo test -p ruviform-audio-core` exit 0（adapter クレートが残ると build が壊れるため同時 revert 推奨） |
| 依存追加のみの戻し（`assert_no_alloc` 統合失敗時） | dev-dep が機能しない | `Cargo.toml` から dev-dep を除去し `Cargo.lock` を再生成、フォールバック証跡を実験ログに残す | `mise exec -- cargo test -p ruviform-audio-device` exit 0 |

## Open Questions / Gaps

設計書が「要実装時確認」と明示した項目は本 plan の**意図的に残す既知ギャップ**であり、実装中に `cargo doc`/ソース/実機で確証し、結果を実験ログと（必要なら）設計フォローアップへ記録する:

- cpal 0.18 の API 形状: `Device` 名の写像（`description()` vs `Display`）、`Device::id()` の形状、`available_host_ids`/`host_from_id`/`Host::id().name()` の正確な形、`now()`/`buffer_size()` が backend へ到達するか、`StreamInstant` の時計保証、`negotiate` 戻り型（`cpal::StreamConfig` + sample_format の保持形）、`ErrorKind` の値構成可能性（写像テストの方式分岐）
- `TimeBase::new` の const-eval 可否（`DEVICE_CLOCK_TIMEBASE` の宣言形: `const`/`fn`/`OnceLock`）
- `BufferSize::Fixed` の実機尊重度・ALSA 系での cpal 内部変換の有無・`Send`/`Sync` ばらつき（PipeWire `!Send` リスク）・WASAPI xrun 未報告の扱い — いずれも実機観測対象
- cpal 0.18.x の MSRV 適合（公開情報 1.85 = workspace MSRV と一致するが `cargo` で確証する）

## Self-Review

- [x] spec Acceptance Criteria 全件が Phase A–D・検証マトリクス・4 タスクへ対応している（候補比較確定 → Phase D / ポート定義と非露出 → Phase A / 分離クレートと cpal 閉じ込め → Phase B / PoC 実証と手動手順 → Phase C / 非 alloc 契約と検証 → Phase A・B / mise・prek 検証 → Phase D）
- [x] 設計の決定事項（3 trait 分解・object safety・`Send` 境界・F32 交渉・scratch 上限・chunk 分割・zero-fill・err_cb 所有権分割・`ErrorCtx` 別写像・`try_new`・`RtContext` 非搭載）は工程に反映し、シグネチャの再定義は設計正本へ委譲した
- [x] TBD/TODO/「後で実装」のプレースホルダは無い。未知項目は「要実装時確認」として Open Questions / Gaps に列挙済み
- [x] ファイルパス・relation キー・status・コマンド・用語が spec/design/既存 docs と整合している

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（Phase A→B→C→D は直列だが、各タスクは独立した実施・検証単位として分割済み。実機依存の Phase C 以外はエージェント実行可能）
- Independent work streams: なし（全タスクが直列依存。`TASK-034Jglv8LtJHUgWH3wlu0o` → `TASK-034JglAvWUFQoLhvYXOw2O` → `TASK-034JglHs8cNMDOvyZFeUpc` → `TASK-034JglHTIvjo18tMdbO3DP`）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 実装正本は設計 `DESIGN-034JcC9QK924kDbu4WhibC` であり、レビューはシグネチャ自体ではなく工程分割・検証充足・帳簿同期の抜け漏れに集中する
- spec `SPEC-034Jc9QEqYLVfg1CXXTiPV`・ADR `ADR-034Jc9RslnH7QSQ1nZEEHL` はともに `proposed` であり、本 plan は両者の確定（spec は充足・ADR は昇格判定）を成果物に含む。spec 側の status 昇格扱いは親エージェントのレビュー方針に従う
- 実機依存の検証（Phase C の list_devices/play_sine・Send/Sync・Fixed 尊重度・xrun 観測）は CI 不能であり、実験ログへの記録をもって完了とする判断をレビューで確認する
- 「要実装時確認」の各項目で設計と実際の cpal 形状が乖離した場合の扱い（実験ログ記録＋設計フォローアップ）は plan の Open Questions / Gaps に集約済み
