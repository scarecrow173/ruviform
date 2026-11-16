---
id: "DESIGN-034YxD6W7IXEfl432tYKlt"
type: "design"
status: "approved"
title: "Issue #532 foreground-probe 設計（変異 module・段階・復元・verdict 意味論）"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-foreground-probe.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/specs/issue-532-foreground-probe.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-631-real-window-dock.md"
    - "evidence/rf631/verification.md"
  refines:
    - "docs/designs/issue-532-dcomp-occlusion.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-foreground-probe.md"
    - "docs/designs/issue-532-dcomp-occlusion.md"
  derived-by:
    - "docs/plans/issue-532-foreground-probe.md"
    - "docs/test-specs/issue-532-foreground-probe.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-foreground-probe.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 foreground-probe 設計（変異 module・段階・復元・verdict 意味論）

## Context

前回 slice（`issue-532-dcomp-occlusion`、PR `#686` merge `a55a7131`）
の query-only 診断で確定済み: docked `test_editor` child rect 内の
`WindowFromPoint` 5 点全てが外部 `Chrome_RenderWidgetHostHWND` を返し
（cover-vs-skip 判別済み — chain 全 visible/alive/enabled）、GPUI
内部 z-order に遮蔽なし、real lane の mount 経路に foreground 化
呼出なし。**遮蔽の第一境界は ruviform 外側の window stacking —
compositor 内部遮蔽は `unresolved` のまま**。

本 slice はその残存二分を変異 probe で行う: GPUI root HWND（
`GetAncestor(GA_ROOT)` — `osoccl`/`ospix::presented` の先例）の
z-order/foreground を段階的に変異させ、各段階で `WindowFromPoint`
（既存 `osoccl::probe` 経由）+ screen green fraction（既存
`ospix` 経由）を再観測する。変異後に green が出れば外部 stacking
が遮蔽の全原因、`WindowFromPoint` が自 window 系を返すのに green
が出なければ compositor 内部遮蔽が残る — という二分結果を得る。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test` に新規 catalog case
  `real-dcomp-foreground`（14 行目・`real-dcomp-pixel` 直後 —
  `NativePluginDock`+`VisualCapture`）。
- 新規 sibling module `src/osmut.rs`（`real-dock` feature、
  `#[cfg(windows)]` + `#[cfg(not(windows))]` stub）— z-order/
  foreground 変異の適用・観測・復元。`osoccl`/`ospix` の
  query-only 契約は不変。
- `REAL_CATALOG`（13→14 行）・`real_entries`（`[...; 13]`→
  `[...; 14]`・同一順）・`tests/console.rs` 件数/list 期待値
  （21→22 行）・`real_dock.rs` module doc「thirteen cases」表記
  と case 表・crate README・bin doc の整合更新。
- `Cargo.toml` の windows-sys dep コメントを mutation+restore
  責務へ更新（feature 追加なし — 対象 API は全て既存
  `Win32_UI_WindowsAndMessaging`/`Win32_Foundation` 内、
  `AttachThreadInput` は `Win32_System_Threading` も既存 feature）。
- baseline（env 未設定 canonical lane）と env-on
  （`GPUI_DISABLE_DIRECT_COMPOSITION=1`・直接 `cargo run` +
  別 `--out`）の interactive session 実走と記録。
- 遮蔽層の二分結論・方式 A patch/B-region/B-dcomp2/C viability
  への帰結を doc chain・`evidence/rf631/verification.md`・
  `evidence/rf532/` 結論文書に記録。

### Out of Scope

- 方式採否・本 fix（`ruviform-ui-gpui` adapter の foreground/
  z-order 恒常化・`[patch]`/fork・env var 恒常化 — 全て後続）。
- GPUI root への `PrintWindow` 等の別系観測（後続 slice）。
- `osoccl`/`ospix` への変異 API 追加（query-only 契約は不変）。
- `AttachThreadInput`/`LockSetForegroundWindow`/`SetWindowPlacement`
  等の foreground lock 回避手段（design で評価し不採用 — 下記）。
- `mise.toml`（stale コメント L166-169 含む — lease 外、PR 本文に
  明記）・docs index・`Cargo.lock`・ルート `Cargo.toml`・他 crate。
- Focus/Resize/ReDock/入力の実 Windows 実走・merge-SHA CI・
  macOS/Linux・consumer 通知・Issue auto-close・`*-msrv`。

## Components and Boundaries

### `src/osmut.rs`（新規、`#[cfg(feature = "real-dock")]`）

GPUI root HWND の**可変 window 状態**（exstyle・top-level z-order
位置・foreground 所有）の snapshot・変異適用・復元を担う
private module。境界規約は `osoccl`/`ospix` と同じ — crate 外に
出る値は `u64` HWND・bool・整数・文字列の plain 値のみ、
Win32 型は module 内に閉じる。`unsafe` は FFI 境界の
`#[expect(unsafe_code)]` + SAFETY コメントのみ。

```rust
/// root の可変状態 snapshot — plain 値のみ。
pub(crate) struct RootSnapshot {
    pub supported: bool,
    /// GetAncestor(child, GA_ROOT)。
    pub root: Option<u64>,
    /// GWL_EXSTYLE（osoccl::ExstyleFacts 再利用 — query-only
    /// module の型を参照する分には query 契約を侵さない）。
    pub exstyle: Option<osoccl::ExstyleFacts>,
    /// WS_EX_TOPMOST bit。
    pub topmost: bool,
    /// top-level z-order で root より上にある window 数
    /// （GW_HWNDPREV walk、cap 512 — `truncated` で打切り記録）。
    pub above_count: Option<u32>,
    /// 直上 neighbor HWND（先頭 GW_HWNDPREV 結果）。
    pub above_hwnd: Option<u64>,
    /// 直下 neighbor HWND（先頭 GW_HWNDNEXT 結果）。
    pub below_hwnd: Option<u64>,
    /// GetForegroundWindow — NULL は None。foreground 旧所有者の
    /// 識別（外部 window か自 window かは caller が role 分類）。
    pub foreground: Option<u64>,
    /// z-order walk が cap に達した場合 true。
    pub truncated: bool,
}

/// 変異適用の結果 — API 呼出名・戻り値・error・適用後 snapshot。
pub(crate) struct MutationOutcome {
    /// "SetWindowPos(HWND_TOP,...)" 等の記録用 API 名。
    pub api: &'static str,
    /// API の BOOL 戻り値（TRUE=適用、FALSE=拒否/失敗）。
    pub applied: bool,
    /// FALSE 時の GetLastError 値。
    pub win32_error: Option<u32>,
    /// 適用後の RootSnapshot（exstyle/z/foreground 再読）。
    pub after: RootSnapshot,
}

/// 復元の結果 — 各復元 op の結果と最終状態。
pub(crate) struct RestoreOutcome {
    /// 発行した復元 op の列（api/applied/win32_error）。
    pub ops: Vec<MutationOutcome>,
    /// 復元後の RootSnapshot。
    pub final_state: RootSnapshot,
    /// topmost flag が baseline と一致したか — 復元の検証可能部。
    pub topmost_restored: bool,
    /// z-order 近似復元の記録（above_count/above_hwnd の
    /// baseline 対比 — 完全一致は保証しない、観測値として記録）。
    pub zorder_before: (Option<u32>, Option<u64>),
    pub zorder_after: (Option<u32>, Option<u64>),
    /// foreground 復帰試行の記録（baseline owner への
    /// SetForegroundWindow — 他 process owner の強制復帰は
    /// Windows 上保証できないため観測記録のみ）。
    pub foreground_before: Option<u64>,
    pub foreground_attempted: bool,
    pub foreground_final: Option<u64>,
}

/// child HWND → GA_ROOT → 可変状態 snapshot（query 部分）。
pub(crate) fn observe(child: u64) -> RootSnapshot;
/// SetWindowPos(root, HWND_TOP, SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)
/// — non-topmost band の最上位化（活性化を伴わない純粋 z-order）。
pub(crate) fn raise_to_top(root: u64) -> MutationOutcome;
/// SetForegroundWindow(root) — foreground lock で FALSE になり
/// 得る。戻り値をそのまま記録し成功に偽装しない。
pub(crate) fn set_foreground(root: u64) -> MutationOutcome;
/// SetWindowPos(root, HWND_TOPMOST, SWP_NOMOVE|SWP_NOSIZE|
/// SWP_NOACTIVATE) — topmost band 化。
pub(crate) fn set_topmost(root: u64) -> MutationOutcome;
/// baseline snapshot に対する復元 — topmost flag を baseline 値へ
/// 戻し（NOTOPMOST/TOPMOST）、baseline の直上 neighbor がいれば
/// その直下へ reposition、foreground 旧所有者が自 window 以外なら
/// 復帰を試行して結果を記録。検証は最終 observe の topmost flag
/// 一致で行い、z-order/foreground は観測値として正直に記録する
/// （完全復元を装わない）。
pub(crate) fn restore(baseline: &RootSnapshot) -> RestoreOutcome;
```

非 Windows stub は `supported:false`・全 field None/空を返す
（`osoccl`/`ospix` stub と同形 — 捏造した観測値を出さない）。

### `src/real_dock.rs`

- `real_entries` に `real-dcomp-foreground` を `real-dcomp-pixel`
  の直後（6 番目）に挿入 — `real-capture`/`real-dcomp-pixel` 等の
  画素依存 case が変異前の自然状態を観測した後に変異する
  run-order 制約。`real-session-terminate`（13 番目・host-loss
  lifecycle）より後の末尾配置は、その後の新規 editor session が
  host 存続前提で成立するか未証明のため採用しない。
- `editor_foreground_body` — `editor_pixel_body` と同じ
  `prepare_editor_host` + `attempt_real_attach` 骨格で、
  `observe_open` に `observe_editor_foreground_probe` を渡す。
  verdict 写像は `PixelVerification` に相当する
  `ForegroundVerification` を使う（下記）。
- `observe_editor_foreground_probe` —
  `observe_editor_pixel_lifecycle` と同型: Phase1
  `oschild::observe` + `check_attached_children`（attach 契約は
  `real-editor-attach` と同一）→ Phase1b 変異 probe（attach 中・
  lease 生存中に実施）→ Phase2-4 `conclude_editor_session`
  （close/detach/converge は共有）。
- `foreground_probe_children` — child 列に対し:
  1. baseline: 各 child の `osoccl::probe` hit-test +
     `ospix` screen capture green fraction + `osmut::observe`
     snapshot（変異前の外部遮蔽物識別を含む）。
  2. 段階 (a) `raise_to_top` → `osmut::observe` 再読 +
     各 child の hit-test/screen green 再観測。
  3. 段階 (b) `set_foreground` → 同上。
  4. 段階 (c) `set_topmost` → 同上。
  5. `osmut::restore(baseline)` → 復元 step row（専用）+
     復元後 hit-test/screen green の最終観測。
- verdict 集約は下記「verdict 意味論」。二分結果自体は
  `observed`/step detail の生データとして記録し、解釈は
  doc chain/evidence に書く（case は機械的解釈を emit しない）。

### `src/real_catalog.rs` / `tests/console.rs` / docs

- `REAL_CATALOG` 14 行目を `real-dcomp-pixel` 直後に追加
  （`required: [NativePluginDock, VisualCapture]`）、
  「thirteen」→「fourteen」表記更新。
- `tests/console.rs`: `rows.len()==21`→`22`、expected case 一覧に
  `real-dcomp-foreground` を `real-dcomp-pixel` 直後へ挿入、
  コメントの「thirteen real-lane rows」→「fourteen」。
- `real_dock.rs` module doc の case 表に `real-dcomp-foreground`
  行を追加し「thirteen cases」→「fourteen cases」。
- crate README の real lane case 表に同 case の verdict 意味論を
  追記、`--include-real` の「thirteen」→「fourteen」。
- `plugin-dock-real-scenario.rs` bin doc に case 追加は不要
  （case 非依存の記述のみ — 変更しない）。

## Data and Control Flow

### 変異段階（design 確定 — 実装指針 3 の確定）

child 列挙後・close 前（attach 中・lease 生存中）に、root は
全 child で共有のため**変異は root に対し 1 系列だけ**適用し、
各段階で全 child を再観測する（child ごとに変異を繰り返さない
— root の z-order/foreground は child 非依存）。

| # | step | 内容 |
| --- | --- | --- |
| 0 | `fg_baseline_state` | `osmut::observe` — root/exstyle/topmost/above_count/above_hwnd/below_hwnd/foreground |
| 1 | `fg_baseline_observe`（child 毎） | `osoccl::probe` hit-test（role 付き）+ screen `BitBlt` green fraction + artifact（opt-in） |
| 2 | `fg_apply_raise` | `SetWindowPos(root, HWND_TOP, 0,0,0,0, SWP_NOMOVE\|SWP_NOSIZE\|SWP_NOACTIVATE)` — non-topmost band 最上位化・活性化なし。`BringWindowToTop` は活性化を伴うため不採用（変数分離のため SetWindowPos を選択） |
| 3 | `fg_apply_foreground` | `SetForegroundWindow(root)` — foreground lock で FALSE になり得る。BOOL+GetLastError をそのまま記録 |
| 4 | `fg_apply_topmost` | `SetWindowPos(root, HWND_TOPMOST, ..., SWP_NOMOVE\|SWP_NOSIZE\|SWP_NOACTIVATE)` — topmost band 化 |
| 5 | `fg_restore` | `osmut::restore` — (i) `SetWindowPos(root, baseline.topmost ? HWND_TOPMOST : HWND_NOTOPMOST, ...)` で band を baseline 値へ、(ii) baseline の `above_hwnd` があれば `SetWindowPos(root, above_hwnd, ...)` で直上 neighbor 直下へ近似 reposition、(iii) `foreground_before` が root 以外の生存 window なら `SetForegroundWindow(foreground_before)` を試行。全 op の戻り値を記録し、最終 `osmut::observe` で topmost flag 一致を検証 |
| 6 | `fg_restore_observe`（child 毎） | 復元後の hit-test + screen green（cover 状態が戻ったかの観測） |

各 `fg_apply_*`/`fg_baseline_*`/`fg_restore*` row は `verdict`
語彙を持つ（後述）。観測 row は `verdict:"note"`。apply row は
`applied`/`win32_error`/適用後 `RootSnapshot` を持ち、
z-order 変異（`raise`/`topmost`）の `applied:false` は `fail`、
`SetForegroundWindow` の `applied:false` は foreground lock の
**予期される拒否**として `note`（後述の verdict 規則）。

### 観測の再利用

- hit-test: `osoccl::probe(child)` を各段階で呼び、`hits` を
  `occlusion_role`/`occlusion_window_json` で role 付き plain JSON
  化（既存 helper 再利用 — 新規 serializer は最小限）。
- 画素: `ospix::window_screen_rect(child)` →
  `ospix::capture_screen_rect` → `green_stats`/`green_fraction`
  （既存 helper 再利用）。capture 失敗は row に `error` 記録 —
  verdict 入力にしない（計測データの欠損であって probe 実行の
  失敗ではない）。
- artifact: `record_pixel_artifact` 相当を `real-dcomp-foreground-*`
  名で再利用（opt-in `--pixel-artifacts`）。

### verdict 意味論（tautology 回避 — 実装指針 4 の確定）

case verdict の入力は **probe 実行 + 状態復元 + 共有 lifecycle
契約**のみ。screen green の有無・`WindowFromPoint` の回答内容は
**二分の結果データ**であり verdict に入れない。

- `Pass`: attach lifecycle 全 check 合格（`check_attached_children`
  + `conclude_editor_session` — `real-editor-attach` と同一）
  **かつ** probe が評価可能で全変異段階を実行（z-order 変異
  `raise`/`topmost` は `applied:true`、`set_foreground` は
  API 呼出が definitive な結果を返した = TRUE または記録済み
  FALSE）**かつ** `osmut::restore` の検証が成立
  （`topmost_restored==true` かつ復元 op が全て API 呼出を
  完了 — 後述の非対称規則）。
- `Fail`: lifecycle 失敗、または z-order 変異 API の FALSE
  （自 window の z-order 変更が拒否されるのは異常 — probe が
  計画どおり実行できなかった正直な失敗）、または復元検証の
  失敗（topmost flag が baseline に戻らない・復元 op の API
  失敗）。
- `NotRun`: session/attach 前提不成立（artifact 不在・open 拒否
  系は既存写像に従う）または probe 評価自体が不能
  （child ゼロ/root 解決不能 — lifecycle が先に Fail になる
  経路が通常）。
- `Unsupported`: 非 Windows stub（`supported:false`）または
  定義済み `Unsupported` 拒否。

**foreground 操作の非対称規則（design 確定）**: 自 process window
への `SetWindowPos`（z-order/band）は foreground lock の対象外で
実質常に成功するべき — FALSE は異常として `fail` step +
case `Fail`。`SetForegroundWindow` は foreground lock により
FALSE が**仕様上予期される結果**であり、拒否自体が「foreground
経路はこの host では試せない」という二分データ — apply step は
`note`（`applied:false`+`win32_error` を記録）で case verdict
を下げない。復元側も同じ規則: `HWND_NOTOPMOST`/reposition の
`SetWindowPos` 失敗は `Fail`、foreground 旧所有者への復帰試行の
FALSE は記録のみ（`foreground_final` が baseline と異なっても
観測事実として残し、成功を装わない）。

**復元の限界（明示）**: foreground 旧所有者（外部 Chrome 等他
process window）への強制復帰は Windows 上保証できない —
`SetForegroundWindow(old)` を試行し結果を記録するが、
`foreground_final != foreground_before` は「復元できなかった
部分」として正直に記録し、復元できたかのように書かない。
z-order も above neighbor 直下への近似 reposition であり、
window 間の完全な順序復元は保証しない — `zorder_before/after`
を併記して観測値とする。topmost flag の復元のみが検証可能な
復元契約。

### `AttachThreadInput`/`LockSetForegroundWindow`/`SetWindowPlacement` の評価（不採用）

- `LockSetForegroundWindow` — system 全体の foreground lock を
  変更する副作用が大きく test probe の責務を超える → 不採用。
- `AttachThreadInput` — 自 thread を foreground owner thread に
  attach して `SetForegroundWindow` 成功率を上げる手段だが、
  thread input queue の結合は window message 経路に影響し得る
  副作用があり、probe の「最小変異」原則に反する → 不採用
  （必要になれば後続 slice で再評価）。
- `SetWindowPlacement`/`GetWindowPlacement` — show state を含む
  復元に使えるが、z-order は `SetWindowPos` の hwndInsertAfter
  で十分かつ show state 変更は本 probe の目的外 → 不採用。
- `AllowSetForegroundWindow` — 他 process からの許可を得る API
  であり自 process には無関係 → 不採用。

## Non-Functional Constraints

- `unsafe` は Win32 FFI 境界の `#[expect(unsafe_code)]` +
  SAFETY コメントの局所許可のみ（`osoccl`/`ospix`/`oschild`
  先例）。
- HWND は `u64` — Win32 型・GPUI 型を公開 API/evidence schema
  に出さない（`check-ui-driver-boundary` green）。
- `osoccl`/`ospix` の query-only 契約は不変 — 変異 API は
  `osmut` のみに置く。`osoccl` の module doc の query-only
  記述は変えず、「変異は `osmut` の責務」の参照を足すのは可。
- 変異は attach 中・lease 生存中・`conclude_editor_session`
  （close/detach/converge）**より前**に完了させ、復元検証も
  close 前に行う — 後続 case（`real-multiwindow-focus`/
  `real-os-input`）の観測条件を変異状態のまま残さない。
- 変異と capture の間に z-order が race し得る（DWM/他
  process の window 操作）— 各段階の `RootSnapshot` と
  hit-test/screen green を同じ step 群に記録し、時点差を
  limitation として明記する。
- RT contract: non-RT only。新規 dependency・feature 追加なし
  （`Cargo.lock` 差分が出た場合は再生成のみ）。
- `SetWindowPos` の flags は `SWP_NOMOVE|SWP_NOSIZE|
  SWP_NOACTIVATE` — 移動・resize・活性化を伴わない純粋な
  z-order/band 変異に限定（window 幾何と foreground を不用意に
  変えない）。

## Risks and Trade-offs

- **`SetForegroundWindow` の拒否**: foreground lock で FALSE
  になり得る — 予期される結果として `note` 記録し、拒否時は
  foreground branch が `unresolved` のまま残ることを
  bisect 結論に正直に書く。拒否を成功に偽装しない。
- **外部遮蔽物の環境依存**: 前回は Chrome だったが今回は別
  window の可能性 — baseline hit-test が識別情報を記録する
  ため、遮蔽物が変わっても記録は成立する。
- **topmost band の副作用**: `HWND_TOPMOST` 化中は他 app の
  操作を一時的に覆う — 変異〜復元の区間は stage ごとの
  hit-test + screen capture（`--pixel-artifacts` 指定時は
  PNG encode を含む）を挟むため、children 数に比例して
  秒級になり得る。直後に `NOTOPMOST` 復元する。
  interactive session での実行を前提とする opt-in lane の
  ため許容範囲（canonical = mise lane は対話 session 前提）。
- **復元の不完全さ**: z-order の完全復元・foreground 旧所有者
  への強制復帰は保証不能 — `zorder_before/after`・
  `foreground_before/final` を併記し、検証は topmost flag
  一致に限定する（保証できる範囲だけを契約にする）。
- **probe 中の window 破棄 race**: 変異中に root/child が
  破棄される可能性は session が生存している間は低いが、
  `IsWindow` 的な alive 確認は `RootSnapshot`/`WindowRow` の
  `alive` 記録で担保（API 失敗は `win32_error` に出る）。
- **headless 化リスク**: pool が headless 化した場合 real lane
  は全 case `NotRun`+`environment_blocked`（exit 0）—
  `--blocked-probe` lane が fail-closed を証明する。

## Deferred Design Concerns

- 方式 A patch・B-region・B-dcomp2・C の採否と本 fix | 本
  slice の二分結果が入力 | 後続 slice | 観測なしの採否は
  flip-model 提示 plane 系機序を見落とすリスク。
- adapter/lane 側への foreground/z-order 制御の恒常化
  （`RealOsGuiTest`/`RealWindowMount` mount 経路への
  `SetForegroundWindow` 追加等） | 本 slice は probe のみ、
  恒常化は方式採否後 | 後続 slice | probe 結果が「window
  管理で直る」を示した場合の実装先。
- `AttachThreadInput` による foreground lock 回避 | `SetForegroundWindow`
  が拒否され foreground branch が unresolved のまま残った場合
  の拡張候補 | 二分結果が foreground 経路を要求した場合 |
  thread input 結合の副作用評価が必要。
- GPUI root への `PrintWindow` 等の compositor 内部直接観測 |
  本 slice の二分で compositor 内部遮蔽が確定した場合の次段
  診断 | 二分結果次第 | `PrintWindow` は root の合成結果を
  返すか未検証。
- macOS/Linux での同等 probe | `NativePluginDock` は
  Windows-only | GUI-L2 利用要件発生時 | 3OS 対応は Deferred。

## Verification Notes

- [x] 対象 API が既存 `windows-sys 0.61.2` source で
  `Win32_UI_WindowsAndMessaging`（`SetWindowPos`/`HWND_TOP`/
  `HWND_TOPMOST`/`HWND_NOTOPMOST`/`SWP_NOMOVE`/`SWP_NOSIZE`/
  `SWP_NOACTIVATE`/`SetForegroundWindow`/`GetForegroundWindow`/
  `BringWindowToTop`/`GetWindow`/`GW_HWNDPREV`）と
  `Win32_Foundation`（`HWND`/`BOOL`）feature 内にあることを
  crates.io source で確認。`AttachThreadInput` は
  `Win32_System_Threading`（既存 feature）内 — 不採用だが
  依存追加不要であることも確認。
- [x] 変異は query-only module（`osoccl`/`ospix`）に置かず
  `osmut` に隔離する設計 — module 契約の不変条件を維持。
- [x] `SetForegroundWindow` FALSE を成功に偽装しない
  記録設計（`applied`/`win32_error`/`foreground_final`）。
- [x] 復元を topmost flag 一致に限定し、保証不能な部分
  （foreground 旧所有者・完全 z-order）を観測記録に留める
  設計。
- [ ] `mise run check/lint/test ruviform-plugin-dock-test`
  （default + `real-dock`）green — 実装後に確認。
- [ ] 実機 lane baseline/env-on の実走記録 — plan/task の
  検証手順どおり実施。
