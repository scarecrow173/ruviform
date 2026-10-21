---
id: "IMPL-uFiKGxxXItHvsYSwD74IJg"
type: "impl"
status: "completed"
title: "VST3 binding adapter PoC"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/gain.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/strings.rs"
      - type: "file"
        path: "docs/impl/ir/poc-vst3-binding-adapter.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/discovery/evaluate-plugin-binding-options.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/poc-vst3-binding-adapter.md"
  implemented-by: []
  depends-on:
    - "create-plugin-core-foundation.md"
    - "implement-borrowed-planar-view.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-plugin-foundation.md"
    - "../../specs/implement-plugin-foundation.md"
    - "../../discovery/evaluate-plugin-binding-options.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/coupler-rs/vst3-rs"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# VST3 binding adapter PoC

## Summary

task `TASK-034JgsHeZt7Bm40uBMo9wg`（plan `PLAN-034JcvpHIlt5JmqvgOquvI` T4）として、coupler-rs `vst3` 0.3.0 上の最小 VST3 プラグイン PoC `crates/ruviform-plugin-vst3` を実装した。1 ステレオ入力バス・1 ステレオ出力バス・正規化ゲインパラメータ 1 個（ParamID 0、範囲 0.0–1.0、デフォルト 1.0、自動化可）のゲインプラグインで、`IComponent`/`IAudioProcessor`/`IProcessContextRequirements` を持つコンポーネントと `IEditController` コントローラの 2 COM クラスを `IPluginFactory`/`2`/`3` 経由で公開する。`process` は `channelBuffers32` の独立チャンネルポインタ列を検証済みのまま T2 の `PlanarBufferView`/`PlanarBufferViewMut` にゼロコピーでラップし、エイリアスチャンネルのみ activate 時確保の bounce 領域へ逃がす。`GetPluginFactory`＋プラットフォームモジュールエントリのエクスポートを PE エクスポートディレクトリ実測で確認した。

## Context

- 実施タスク: [../../tasks/poc-vst3-binding-adapter.md](../../tasks/poc-vst3-binding-adapter.md)
- 上流: [plan](../../plans/implement-plugin-foundation.md) T4 節、[spec](../../specs/implement-plugin-foundation.md)、[discovery](../../discovery/evaluate-plugin-binding-options.md)（`vst3` 0.3.0 第一候補・既知未確定事項）、[RT 契約 ADR](../../adr/realtime-contract-and-verification-policy.md)
- 前提: T1 の規格中立契約（`PluginProcessor`/`Lifecycle`/`Event`/`ParamValue`/`TransportInfo`）と T2 の借用 view（`PlanarBufferView{,Mut}`）
- 並行作業: T3（`ruviform-plugin-clap`）は別スレッドの作業物であり本実装では一切触れていない。`members` は `ruviform-plugin-clap` の後に `ruviform-plugin-vst3` を追加

## Implementation

### Changes

| File | 内容 |
| --- | --- |
| `Cargo.toml` | workspace メタデータ継承・`crate-type = ["cdylib", "rlib"]`・`vst3 = "0.3.0"`・path 依存（plugin-core / audio-core / media-core）・workspace lints |
| `lib.rs` | `GetPluginFactory`＋条件付きモジュールエントリ（Windows `InitDll`/`ExitDll`・macOS `bundleEntry`/`bundleExit`・Linux `ModuleEntry`/`ModuleExit`）。全エクスポートに `#[unsafe(no_mangle)]`＋`#[expect(unsafe_code)]`＋根拠コメント |
| `component.rs` | `GainComponent`：`IComponentTrait`（バス列挙・activate・state スタブ）・`IAudioProcessorTrait`（`setupProcessing`/`setProcessing`/`process`・stereo 配置のみ）・`IProcessContextRequirementsTrait`。`RefCell<ComponentState>`（`try_borrow_mut` で再入を定義済み拒否 — レビュー後確定）＋`AtomicU32` tail、ライフサイクル判定、`IParameterChanges`/`IParamValueQueue` のイベント収集、`ProcessContext`→`TransportInfo` 変換、COM テストダブル＋ライフサイクル/エイリアス/拒否テスト |
| `block.rs` | `ChannelScratch`（ポインタ列→スライス列リスト・host 出力ポインタ・bounce フラグ・フラット bounce 領域）と `EventScratch`（`MAX_BLOCK_EVENTS = 1_024`・範囲外オフセットドロップ・stable sort・定義済み切り捨て）。`prepare_block` が境界検証＋範囲重複エイリアス判定＋view 構築、`finish_block` が bounce 書き戻し |
| `controller.rs` | `GainController`：`IEditControllerTrait`（`getParameterInfo`/正規化↔プレーン変換・文字列変換・state スタブ・`IPlugView` は `kNotImplemented`）。`AtomicU64`（f64 ビット）ゲイン保持＋正規化ドメイン検証（レビュー後確定） |
| `factory.rs` | `GainFactory`：`IPluginFactory`/`2`/`3` を実装し component/controller 両 CID を `ComWrapper` で生成・要求インターフェースへクエリ。ASCII/Unicode クラス情報 |
| `gain.rs` | `GainProcessor`：中立 `PluginProcessor` 実装。`prepare` でゲイン 1.0 リセット、`process` で ParamID 0 のパラメータイベント適用→チャンネル対乗算 |
| `strings.rs` | `copy_wstring`（UTF-16 `TChar`）・`copy_cstring` — 長さ付き固定バッファへの終端付きコピー |
| ルート `Cargo.toml` | `members` に `crates/ruviform-plugin-vst3` 追加（`ruviform-plugin-clap` の後） |
| `Cargo.lock` | `vst3` 0.3.0 と依存（`com-scrape-types` 等）を記録 |

### Decisions During Implementation

- **生成 trait は全メソッド `unsafe fn`・レシーバ `&self`**: `vst3` の設計どおり safe ラッパーは存在しない。アダプタ側 unsafe は 54 の `unsafe fn` シグネチャ義務と ~51 の局所 `unsafe {}` ブロック（うち約半数はテストの COM ダブル/エントリ読み取り、`block.rs` は実質 3 ブロック = `slice` 再解釈 ×2＋bounce 書き戻し）。すべて SAFETY コメント＋`#[expect(unsafe_code)]` でゲート通過
- **COM `&self` → `RefCell` + アトミック**: COM メソッドは `&self` しか取れないため内部可変性が必須。`RefCell` は `try_borrow_mut` 経由で再入を `kResultFalse` の定義済み拒否とする（レビュー後確定 — panic→FFI 越え unwind はホスト abort のため不採用）— VST3 の呼び出し規律（同一スレッド・非再入）は crate doc に明記
- **ライフサイクル写像**: `initialize`→`Initialize`、`setActive(true)`→`Activate`、`setupProcessing`→中立 `prepare`＋スクラッチ確保（Activated のまま）、`setProcessing(true)`→`StartProcessing`、`process`→`Process`、`setProcessing(false)`→`StopProcessing`、`setActive(false)`→`Deactivate`、`terminate`→`Destroy`。不正順序は `kResultFalse`（未定義でなく定義済み拒否）
- **エイリアス戦略**: 全エイリアスパターン（`out[i]==in[j]` 部分交差・出力ポインタ重複を含む per-channel 対比較）で検出し、衝突出力チャンネルのみフラット bounce 領域へ。非衝突出力はホストバッファ直書き（ゼロコピー経路維持）。出力重複は決定的 last-writer-wins 書き戻し — T2 の bounce-scratch 安全デフォルトを採用し専用 in-place 分岐は不採用。レビューで完全一致→バウンス・部分重複→`Rejection::Overlap` 拒否の範囲比較へ強化
- **イベント経路**: `ComRef::from_raw` で借用（refcount 非操作・非保持）、`sampleOffset` はブロック範囲外をドロップ（レビューで規格間ポリシーを「個別ドロップ」に統一 — 当初は `[0, frames-1]` へクランプ）、`ParamValue` 非有限値は定義済み drop、容量超過は定義済み切り捨て、stable sort 後に `EventList` 構築
- **`kSample64` は拒否**: f64 入力を f32 へ再解釈しない — `setupProcessing`/`process` 双方で `kResultFalse`
- **エントリポイント命名の不整合（記録済み finding）**: upstream `vst3` クレート example は macOS フックを `BundleEntry`/`BundleExit` と命名するが、VST3 spec（・本タスクの文言）は小文字 `bundleEntry`/`bundleExit`。spec 名を採用し `lib.rs` コメントと discovery 追記に記録
- **`IPlugView` の実装位置**: 生成バインディングでは `vst3::Steinberg::IPlugView`（トップレベル、`Vst` 下ではない）。PoC は GUI なしのため `createView` は `kNotImplemented`
- **edition 2024 / 生成定数の型**: `#[no_mangle]` は `#[unsafe(no_mangle)]` と書く必要があり lint 的にも unsafe 扱い（`#[expect]` で明示）。`ProcessContext_::StatesAndFlags_` 等の生成定数はプラットフォーム依存型のため `as` キャスト＋局所 `unnecessary_cast` 許容。`&mut x`→`*mut` の暗黙変換は `borrow_as_ptr` で `&raw mut` が必須
- **state ストリームは非実装**: `IComponent::getState`/`setState` と `IEditController` state は `kNotImplemented` スタブ（PoC 範囲外、T6 へ記録）

## Related Experiments

### Adopted

- `vst3` 0.3.0 の生成バインディングはクレート内同梱で **VST3 SDK ヘッダ・サブモジュールはビルド時に不要**（crates.io 依存のみで `cargo build` 完走を実測 — discovery の Open Question を解消）
- `ComWrapper::new(...).to_com_ptr::<Interface>()`＋`ComPtr::into_raw()` の所有権移譲パターンで `createInstance` を成立させた

### Rejected

- `vst3-host` を dev-dependency にしたロード実証 — 未未採用（極新規・プラグイン側対象外）。エントリ検証は COM vtable 経由の単体テスト＋PE エクスポートディレクトリ解析で代替
- `no_mangle` 単体属性 — edition 2024 では `#[unsafe(no_mangle)]` が必須（採用済み形態）
- `transmute` による fat-pointer 再解釈 — clippy `transmute_ptr_to_ptr` で `slice::from_raw_parts{,_mut}` + `.cast::<&[f32]>()` 形に置換（等価・より明示的）

## Validation

実施環境: Windows x64・Git Bash・pinned toolchain 1.99.0。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 型チェック | `mise exec -- cargo check -p ruviform-plugin-vst3 --all-targets` | clean |
| テスト | `mise exec -- cargo test -p ruviform-plugin-vst3` | 17 単体テスト全合格（gain/イベント・ブロック検証・bounce/エイリアス・ライフサイクル・状態外拒否・in-place・ゼロコピー直書き） |
| lint | `mise exec -- cargo clippy -p ruviform-plugin-vst3 --all-targets -- -D warnings` | clean — SAFETY コメント付き局所 `#[expect(unsafe_code)]` のみ |
| fmt | `mise exec -- cargo fmt -p ruviform-plugin-vst3` | 適用済み |
| cdylib | `mise exec -- cargo build -p ruviform-plugin-vst3` | `target/debug/ruviform_plugin_vst3.dll` 生成 |
| エクスポート | PE エクスポートディレクトリ解析（Python スクリプト、`nm`/`dumpbin` 非搭載環境の代替） | `GetPluginFactory`・`InitDll`・`ExitDll` の 3 シンボル実在を確認。macOS `bundleEntry`/`bundleExit`・Linux `ModuleEntry`/`ModuleExit` は `cfg` ゲート済みコード（他ターゲットビルドは本環境では未実測） |
| `rt_` 契約 | `rt_prepare_process_finish_round_trip` 等 `rt_` テスト＋`prepare_block`/`finish_block`/`process` のドキュメント契約 | alloc 観測は Issue #22 基盤待ち — 事前確保スクラッチ＋レビュー可能な契約で代替 |

## Risks

- `vst3` の `rust_version` は未宣言 — pinned 1.99.0 でビルド実測済みだが MSRV 1.85 単独でのビルドは未実測（生成コードは素の `repr(C)`/extern で構成され新 API 依存は見られないが、MSRV ゲートでの再確認を T5/T6 に残す）
- `RefCell` の borrow 失敗は panic — レビューで `try_borrow_mut` の `kResultFalse` 定義済み拒否へ置換済み（panic→FFI 越え unwind はホスト abort のため不採用。スレッド間並行は VST3 スレッド規約違反として防衛境界外）
- `process` no-alloc は alloc 検証レーン（Issue #22）未整備のためテスト観測ではなく構造契約 — スクラッチは `setupProcessing`/`setActive` 時確保、`process` 内に `Vec` 伸長・ヒープ経路なしをコードレビューで担保
- `component`/`controller` の `setComponentState`/`getState`、GUI（`IPlugView`）、`setHostContext` 活用は未実装スタブ — T6 の薄アダプタ完成タスクへ
- Windows の `InitDll`/`ExitDll` は spec 任意フックを trivially-true で実装（統一形状のため）— macOS/Linux エントリは `cfg` コードのみでクロスビルド未実測

## Follow-ups

- T5（方式 ADR）: 本 IR と discovery 追記の実測（unsafe 量と局所性・エントリ成立・ゼロコピー経路・`RefCell` 必要性・SDK 非依存のビルド要件）を clack 側 PoC 結果と比較して採否を決定
- T6（薄アダプタ完成）: state ストリーム・`IPlugView`・複数パラメータ・f64 経路・`kNotImplemented` スタブ群の本実装、MSRV 1.85 単独ビルドの再確認、macOS/Linux エントリの実機検証
- Issue #22 連携: alloc 検証基盤が来たら `process` 経路の `assert_no_alloc` 検証へ差し替え
