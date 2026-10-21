---
id: "IMPL-034JFAMvBdMrd7hd9RAr4j"
type: "impl"
status: "completed"
title: "Harden plugin adapters from implementation review"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/planar.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/controller.rs"
      - type: "file"
        path: "docs/specs/implement-plugin-foundation.md"
      - type: "file"
        path: "docs/test-specs/plugin-foundation.md"
      - type: "file"
        path: "docs/discovery/evaluate-plugin-binding-options.md"
      - type: "file"
        path: "docs/impl/ir/poc-vst3-binding-adapter.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-plugin-adapters.md"
  implemented-by: []
  depends-on:
    - "implement-plugin-adapters.md"
    - "sync-docs-and-verify-plugin-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-plugin-foundation.md"
    - "../../specs/implement-plugin-foundation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Harden plugin adapters from implementation review

## Summary

T7 完了後の実装 review-protocol（協調 Reviewer A + 敵対 Reviewer B 並行、候補は Validator が実コードで検証・Judge が重大度判定）で確認された P1/P2 6 件と低コスト P3 8 件を修正した。spec 適合・全検証パスを維持したまま、サウンドネス・RT 有界性・ドメイン契約を強化。

## Context

- 評価基準: `docs/specs/implement-plugin-foundation.md` Requirements 104–124・方式 ADR・タスク Done When・リポジトリポリシー（unsafe 局所化・RT 契約・外部型非露出）。
- Reviewer A: spec 全要件を検証済みとして確認しつつ 8 候補（P2×1・P3×7）。Reviewer B: 13 候補（P1×3・P2×3・P3×7）。重複統合後、Validator が実コードで全候補を検証した。

## Implementation

### Changes

**P1（サウンドネス/RT 契約違反 — 必須修正）**

- **部分範囲エイリアスの検出漏れ → UB（両アダプタ + audio-core）**: エイリアス判定が開始ポインタ完全一致のみで、`out[0] = in[0]+1` のようなシフト重複を見逃し `&`/`&mut`・`&mut`/`&mut` の同居（UB）になり得た。`[ptr, ptr + frames×4)` の範囲比較へ置換 — CLAP は `collect_and_check_channel_pointers` で `channel_pair` がスライスを構築する**前**に生ポインタ列で判定（完全一致の同一 index = 合法 in-place、それ以外の重複 = ブロック拒否）、VST3 は完全一致のみバウンス・部分重複は新規 `Rejection::Overlap` で拒否。`has_duplicate_channel_region`（`planar.rs` 防御層）も範囲比較へ。
- **VST3 `collect_param_events` の非有界走査（RT パス）**: `getParameterCount`/`getPointCount` がホスト供給のまま `i32::MAX` まで回り得た。走査を `MAX_BLOCK_EVENTS` キュー/ポイントに上限化し、スクラッチ満杯で早期終了。さらに `param_id != GAIN_PARAM.id` のキューを点検査前にスキップし、外部パラメータが有界イベント枠を消費しない構造に。
- **`Cell<u32>` tail のスレッド間アクセス → データ競合 UB**: `getTailSamples` は全スレッド呼出が契約上あり得るため `AtomicU32` 化（`Release` store / `Acquire` load）。`GainController::gain` も同様に `AtomicU64`（f64 ビット）化。

**P2**

- **`RefCell` 借用失敗 panic の FFI 越え unwind → ホストプロセス abort**: 全借用点を `try_borrow_mut` に置換し、再入呼出は `kResultFalse` の定義済み拒否に（panic→abort を除去）。同時に内部可変性ドキュメントを「同一スレッド再入 = 定義済み拒否、スレッド間並行 = ホスト規約違反」へ修正。
- **`AtomicLifecycle` load→store 非 CAS → 更新喪失**: audio スレッドの `Process` と main の `StopProcessing` が競合した場合、後着 store が陳腐状態を復活させ状態機械が恒久的に固着し得た。有界リトライ（`MAX_CAS_ATTEMPTS = 8`）付き `compare_exchange_weak` ループへ置換。上限超過は定義済み拒否（CLAP スレッド規約上の並行呼出）。
- **`setupProcessing` の `maxSamplesPerBlock` 無制限 → multi-GiB alloc abort**: `MAX_NEGOTIATED_FRAMES = 1<<20`（2ch × 1MiB フレーム ≈ 8 MiB bounce）超過は `kResultFalse`。

**P3（低コストかつ整合的なため併せて修正）**

- パラメータドメイン漏れ: `setParamNormalized`/`getParamValueByString`（VST3 コントローラ）と `text_to_value`（CLAP）に有限性 + `[0,1]` 範囲検証を追加 — `f64::clamp(NaN)` が NaN を伝播し得た経路を全て閉塞。
- `setState`/`getState`（`IComponent`）が `kResultOk` で無処理だった虚偽ステータス → `kNotImplemented`（`setComponentState` と同規則）。
- CLAP `params.flush`（audio/main 両側）の走査を `MAX_EVENTS_PER_BLOCK` に上限化。
- CLAP バッファレベル `frames_count`（process レベルとは別フィールド）を交渉上限と照合して拒否 — 未検証だったスライス長経路を閉塞。
- `EventList::new` 失敗時のサイレント空リストフォールバック（VST3）→ 不変条件違反として `debug_assert` + 定義済み拒否。
- **オフセットポリシーの非対称統一（規格間）**: VST3 はクランプ・CLAP はブロック拒否だったため、両規格とも「ブロック範囲外オフセットのイベントを個別にドロップ」に統一（誤ったフレームへの適用を防ぐ最小驚異の挙動）。spec 122 に明記。

**Spec 明確化**

- `numSamples` の「0 以下」文言を「負の値は拒否、0 フレームはイベントフラッシュ経路として受理」へ明確化（厳密読みでの再争いを防止、spec 119）。
- in-place 検出に範囲重複の記述を追加（spec 120）。

### Decisions During Implementation

- **部分重複はバウンスせず拒否**: 開始ポインタの異なる重複は in-place として意味を持たない malformed 入力 — バウンスでも安全だが、誤動作を受け入れるより定義済み拒否が正直。
- **CAS リトライは有界（8）**: RT 契約の「無制限リトライ禁止」に整合。上限超過を `IllegalTransition` の定義済み拒否として表面化。
- **`RefCell` 維持 + `try_borrow_mut`**: VST3 スレッド規約が各メソッドの呼出スレッドを規定しており、スレッド間並行は防衛境界の外（Mutex 化は RT ブロッキングを持ち込み設計劣化）。同一スレッド再入のみを定義済み拒否で防御。
- **`#[expect(unsafe_code)]` の関数単位スコープは維持**: 生成 trait が `unsafe fn` シグネチャを義務付けるため構造的に必要（P3 メモとして記録・対象外）。
- **コントローラ → プロセッサの値配線は非対象**: `setParamNormalized` が `inputParameterChanges` 無しでは音声へ届かないのは分散アーキテクチャの PoC 制約。フォローアップ候補として記録（後続 Issue）。

## Related Experiments

### Adopted

なし（実験ログ対象外のレビュー修正）。

### Rejected

なし。

## Validation

- `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 -p ruviform-audio-core` — 全件パス（audio-core 39 + clap 41 + vst3 26 + doctests）。新規テスト: CLAP 部分重複拒否、VST3 シフト重複拒否・偽カウント有界走査（`rt_param_scan_stays_bounded_when_host_counts_lie`）・外部パラメータ無視・範囲外オフセットドロップ・上限拒否・`kNotImplemented`・コントローラドメイン検証、`block_offset` ドロップセマンティクス、CLAP オフセットドロップ。
- `cargo clippy`（対象 3 クレート `--all-targets -D warnings`）— クリア。
- `cargo fmt --all` — 適用済み。

## Risks

- 範囲重複チェックはポインタ加算のアドレス比較（`saturating_add`）に依拠 — `Provenance` 上の指針操作ではなくアドレス算術のため妥当。
- CLAP 側の範囲重複判定は `raw_data()` 時点の生ポインタで行う — clack が `channel_pair` 内で構築するスライスは検査済みアドレスから生成されるため、判定漏れの新規経路はない。

## Follow-ups

- **Issue #22**（RT 検証基盤）: `rt_` テストは alloc 非観測のまま — no-alloc 保証は文書契約 + レビュー対象のまま。
- **コントローラ → プロセッサ値配線**: `setComponentHandler`/`IComponentHandler` 経由の編集伝搬は PoC 範囲外 — 後続 Issue で atomic mirror（CLAP `ParamMirror` 相当）の配線を検討。
- **macOS/Linux VST3 エントリのクロスビルド実測**: cfg 構造は存在するが未検証。
- **レビュー P3 メモ（対象外として記録）**: `#[expect(unsafe_code)]` 関数スコープの広さ・イベント切り捨ての到着順ポリシー（有界化 + パラメータフィルタで実害は緩和済み）。
