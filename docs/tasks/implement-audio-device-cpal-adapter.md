---
id: "TASK-034JglAvWUFQoLhvYXOw2O"
type: "task"
status: "done"
title: "ruviform-audio-device クレート新設と cpal adapter 実装"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added:
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
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
  implemented-by:
    - "../impl/ir/implement-audio-device-cpal-adapter.md"
  depends-on:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
    - "implement-audio-core-device-port.md"
  blocks:
    - "run-device-poc-examples-and-exp-log.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
    - "../designs/audio-device-boundary-and-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/audio-device-port-and-cpal-adapter.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-audio-device クレート新設と cpal adapter 実装

## Work

plan `PLAN-034JgkWCimxdvr81k6b0ow` の Phase B。設計正本 `DESIGN-034JcC9QK924kDbu4WhibC`「`ruviform-audio-device` クレート構成」「マッピング設計」「コールバック配管」節どおりに実装する。cpal API の「要実装時確認」項目は `cargo doc`/ソースで確証し、設計との乖離は実験ログ記録対象として Phase C に引き渡す。

- [x] `Cargo.toml` を作成しルート `Cargo.toml` の `members` に `crates/ruviform-audio-device` を追加する。`cpal = "0.18"`（直接依存・`default-features` 方針は設計どおり opt-in 構成）+ `ruviform-audio-core`/`ruviform-media-core` path 依存。features: `default = []`・`realtime = ["cpal/realtime"]`・`pipewire-backend = ["cpal/pipewire"]`（feature 名に固有名詞を出さない意図命名）。`publish.workspace = true`・`[lints] workspace = true`。dev-dependency に `assert_no_alloc`（試行・理由記録）。cpal 0.18.x の MSRV/edition-2024 適合を `cargo` で確証し、不適合なら plan/設計へ差し戻す — 完了: cpal `0.18.2`・assert_no_alloc `1.1.2` で解決・edition-2024/MSRV 設定下でコンパイル成功
- [x] `lib.rs`: crate doc（責務・許可依存層・RT 契約転記）+ 公開面 re-export。公開面にも cpal 型・エラー・feature 名を出さない — 完了: 公開面は `Host`/`HostId`/`default_host`/`available_host_ids`/`host` のみ
- [x] `error.rs`: `map_error(e: &cpal::Error, ctx: ErrorCtx) -> DeviceError` を実装する（`ErrorCtx` = 制御経路/err_cb 区別。設計の写像表どおり: 制御経路 `DeviceNotAvailable→DeviceNotFound`・err_cb `DeviceNotAvailable→StreamInvalidated`・`DeviceChanged` は err_cb で `Backend("device_changed")` 非致命アドバイザリ・`Xrun` は err_cb のみ・その他 `Backend("...")` 系・`#[non_exhaustive]` wildcard は `Backend("other")`）。`ErrorKind` が値構成可能なら全分岐の単体テスト、不可能なら `DeviceError` 側の等価性・`Display` を検証する（実装時確認で判断） — 完了: `cpal::Error::new(ErrorKind)` で値構成可能と確認し全分岐を合成エラーテスト。制御経路 `Xrun` は設計表「通常発火しない」のため `Backend("other")` に分類
- [x] `convert.rs`: 純粋関数として `map_sample_format`（F32→F32・F64→F64・他 `None`）・`layout_for_channels`（1→Mono・2→Stereo・4→Quad・6→5.1・8→7.1・他 `None`）・`negotiate(desired, supported)`・interleave 書込ヘルパ（`n = len/channels`・残尾 dst ゼロフィル・`n > frames_capacity` の chunk 分割・コピー失敗時 dst 残部ゼロフィル）を実装し、合成入力で単体テストする — 完了: 合成テスト 11 本
- [x] `host.rs`: `Host`（private `cpal::Host`）→ `DeviceEnumerator` 実装、`HostId` opaque newtype（Clone+Debug・`name() -> &'static str`）、`default_host()`/`available_host_ids()`/`host(id) -> DeviceResult<Host>` を実装する — 完了: 内部モジュール名は `host_impl`（`fn host` との衝突回避）
- [x] `device.rs`: cpal `Device` → `Device` 実装（`name`/`id`/`default_output_format` — open 可能な F32 形式を優先返却・`supported_output_formats` — F32-capable レンジのみ報告・`open_output_stream` — 非 F32 config は `Err(UnsupportedConfig)` で拒否） — 完了
- [x] `stream.rs`: cpal `Stream` → `OutputStream` 実装 + `DataCallbackState` コールバック配管（scratch `Vec<f32>` を open 時に `min(effective_max, FRAMES_CAPACITY_HARD_CAP)` で 1 回確保・per-callback `try_new` view 構成・露出領域ゼロフィル・data/err クロージャへの所有権分割で `Arc`+`Mutex` 共有禁止・`StreamInstant::as_nanos` 主経路 + u128→i64 飽和時の epoch 差分予備経路で `OutputTimestamp` 構成） — 完了: `FRAMES_CAPACITY_HARD_CAP = 16_384`
- [x] `tests/mock_backend.rs`: フェイク `Device`/`OutputStream` で enumerate→default→open→スクリプト発火 callback→start/stop→`now` を `Box<dyn>` 経由で検証する結合テストを書く — 完了: 4 テストでポート差替え可能性を証明
- [x] interleave 書込経路の合成 `&mut [f32]` テスト（`n<=capacity`・`n>capacity` chunk 経路・残尾ゼロフィル・コピー失敗時ゼロフィル）を書く — 完了
- [x] `assert_no_alloc` 試行テストを `rt_` プレフィックス命名で書く（view 構成 + `copy_to_interleaved` + テスト用 user cb の alloc ゼロ検証）。統合不能時は dev-dep 除去とフォールバック方針（コードレビュー＋静的 grep 証跡）を Phase C 実験ログへの引き継ぎ事項として記録する — 完了: `rt_output_callback_path_allocates_nothing` が `mise run test-rt` で選択・合格。フォールバック不要

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-audio-device-port-and-cpal-poc.md](../plans/implement-audio-device-port-and-cpal-poc.md) — `PLAN-034JgkWCimxdvr81k6b0ow`（approved）。Phase B の正本
- [../designs/audio-device-boundary-and-poc.md](../designs/audio-device-boundary-and-poc.md) — `DESIGN-034JcC9QK924kDbu4WhibC`（approved）。クレート構成・ErrorKind 写像表・F32 交渉・コールバック配管・manifest 方針の実装正本
- [../adr/select-audio-device-backend.md](../adr/select-audio-device-backend.md) — `ADR-034Jc9RslnH7QSQ1nZEEHL`。採用条件（非露出・adapter 責務・feature 命名）の正本

## Dependencies

- 前提タスク: [implement-audio-core-device-port.md](implement-audio-core-device-port.md)（`TASK-034Jglv8LtJHUgWH3wlu0o`）— ポート trait・値型・`AudioBufferViewMut::try_new` を提供
- 後続をブロック: [run-device-poc-examples-and-exp-log.md](run-device-poc-examples-and-exp-log.md)（example が adapter 公開面に依存）
- 新規外部依存: `cpal = "0.18"`（PoC 評価対象・条件付き採用候補）、dev-dep `assert_no_alloc`（非 alloc 検証試行）— 追加理由は PR/実験ログに記録し `Cargo.lock` を追跡
- Linux ビルドには `libasound2-dev` が必要（ALSA 既定）。`pipewire-backend` 有効化には `libpipewire-0.3-dev`（任意検証のみ）
- adapter→adapter 依存・コア→adapter 依存は作らない。`unsafe` 不使用（cpal 内部の unsafe は依存先の実装詳細）

## Affected Files

- `crates/ruviform-audio-device/Cargo.toml` — 新規
- `crates/ruviform-audio-device/src/{lib,host,device,stream,convert,error}.rs` — 新規
- `crates/ruviform-audio-device/tests/mock_backend.rs` — 新規
- `Cargo.toml`（root） — `members` 追加
- `Cargo.lock` — cpal/assert_no_alloc 依存の記録

## Done When

- [x] `ruviform-audio-device` が workspace メンバーとして `cargo check`/`build`/`test` に成功する
- [x] `cargo metadata` で cpal が同クレートの `[dependencies]` にのみ存在し、両クレート公開面に cpal 型・feature 名が露出しない
- [x] `map_error` が設計写像表どおり `ErrorCtx` 別に正規化される（テストまたは DeviceError 側等価検証）
- [x] モックバックエンド結合テストが非 cpal 実装でのポート実装可能性を証明する
- [x] interleave 書込経路が chunk 分割・ゼロフィル規約どおりに動作するテストがある
- [x] `rt_` 命名の alloc 検証テストが `mise run test-rt` で選択される（またはフォールバック方針が記録される）

## Verification

- [x] `mise exec -- cargo test -p ruviform-audio-device` exits 0（写像・convert・モック結合・interleave テスト合格）— 実測: 単体 30 + 結合 4 + doctest 6 全て合格
- [x] `mise exec -- cargo clippy -p ruviform-audio-device --all-targets -- -D warnings` exits 0
- [x] `mise run test-rt` exits 0（`rt_` alloc 試行テストが選択・合格）— 実測: `rt_output_callback_path_allocates_nothing` 選択・合格
- [x] `mise exec -- cargo metadata --format-version 1` で cpal が `ruviform-audio-device` の依存にのみ存在することを確認
- [x] `mise exec -- cargo check -p ruviform-audio-core` exits 0（adapter 無効構成でコアが自立）
- [x] `mise run fmt-check` exits 0
