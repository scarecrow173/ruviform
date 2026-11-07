---
id: "DESIGN-z4LQPvCYdpedQfNlIZbQ7J"
type: "design"
status: "draft"
title: "Issue #404/#405 NativeSurfaceSlot host 境界修復設計（clip 整合 + teardown reconcile）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/404"
    - "https://github.com/scarecrow173/ruviform/issues/405"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-404-405-native-surface-host-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-404-405-native-surface-host-boundary.md"
    - "docs/designs/issue-388-native-surface-slot.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-386-driver-neutral-gui-host.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-404-405-native-surface-host-boundary.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-404-405-native-surface-host-boundary.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [404, 405]
---

# Issue #404/#405 NativeSurfaceSlot host 境界修復設計

## Context

`GpuiSurfaceAdapter`（standalone `new(&driver)` と RF387 canonical
`bound(&inner)` 両経路で使用）は、共有 `Rc<RefCell<Inner>>` を既に
保持しており、driver 側の authoritative な死亡記録 — `retire`
による `windows.remove` + `destroyed.insert` — を同一オブジェクト
経由で観測できる。破られた不変条件は 2 系統ある。

- **clip 不変条件**（#404）: 「container の region ≡
  `clip_local(binding.clip, spec.geometry)`」が `set_clip(None)`
  と `set_geometry` で破れる。さらに `binding.clip` を OS 適用前に
  記録する順序のため、OS 失敗時に帳簿が OS 事実を先行する。
- **teardown 不変条件**（#405）: 「live ledger を持つ slot の
  owner window は live」が `retire` で破れる。adapter は op を
  `Ok` + event で通し、死んだ `Container`（死んだ HWND）を抱えた
  まま `bindings`/`registry` が無期限に成長する。

## 決定

### D1（#405）: teardown は adapter 側 reconcile — `sweep_dead`

`retire`→adapter の push hook（Weak 登録・共有構造体の相互参照）は
**採用しない**。adapter の各 `&mut self` op と `drain_events` の
入口で `sweep_dead` を実行し、死亡した binding を adapter 自身が
回収する。死亡検出は `Option<DeadKind>` を返す `binding_death`
で、owner 死亡（slot 自体が終了）と container 死亡（lease の
OS surface のみ喪失）を区別する:

```rust
fn binding_death(&self, b: &Binding) -> Option<DeadKind> {
    let inner = self.inner.borrow();
    if inner.destroyed.contains(&b.window) { return Some(Owner); }
    if inner.windows.get(&b.window)
        .and_then(|s| s.raw_parent)
        .is_some_and(|p| !os_window_alive(p)) { return Some(Owner); }
    b.container.as_ref().filter(|c| !c.is_alive()).map(|_| Container)
}
```

- `destroyed.contains(window)`: `retire` が記録する authoritative
  な死亡。`retire` は `apply_destroy`・`sweep_destroyed`・
  `on_window_closed` 経由・`open_window` 生成失敗の全経路を
  通る唯一の終端点（`Inner.destroyed` は driver 寿命中
  append-only — adapter の検出はこれに依存、フィールドに明記）。
- `raw_parent` の `IsWindow` probe: `retire` 未処理の
  pre-retire gap・driver ループ終了後など `destroyed` に載らない
  残留経路、**および container を持たない declared-only
  binding** を補完（外部 `DestroyWindow`/強制終了で GPUI HWND が
  消えた場合）。`IsWindow == false` は死亡の証明だが `true` は
  HWND 値の再利用により生存を証明しない — 検出にのみ使う。
- `container.is_alive() == false`: lease 中 container の死亡
  （子 HWND は parent と共に死ぬ OS 事実）。

push を退ける理由: adapter は既に `inner` を共有しており、
`destroyed` は `retire` が維持する同じスレッド・同じ事実の
authoritative record — これ以上の通知機構は重複する真実源を
作る。`Inner`/`retire` のコード変更はゼロ（`destroyed` への
不変条件コメントのみ追加）。また ledger の書き手が adapter
単一に保たれ、「OS container の死の記録は adapter が負う」
という Issue の自然な分割と一致する。

再入安全: `sweep_dead` は死亡 index を先に収集してから mutate
する二段構成で、`inner` の `RefCell` guard は `release_slot` /
`bindings.remove` / `Container::drop` を跨がない。
`DestroyWindow` は子孫へ同期 message dispatch するため
editor wndproc → host callback 再入があり得るが、既知死亡
container は `abandon()`（`mem::forget` で `DestroyWindow`
抑止）するため sweep 経路では発火しない。live container の
`DestroyWindow` が走るのは consumer 明示 release のみ
（従来通りの構成上の注意）。

### D2（#405）: owner 死亡は auto-release、container 死亡は lease revoke のみ

死亡の種類に応じて既存 vocabulary で段階的に reconcile する:

- **`DeadKind::Owner`**: `registry.release_slot(id)` — live
  lease を revoke して `LeaseRevoked` + 終端 `Destroyed` を
  発行し、index を generation++ で再利用可能にする。slot は
  二度と parent できないため終端化が正しい。binding は除去、
  既知死亡 container は `abandon()`。
- **`DeadKind::Container`**: live owner 上の container のみ
  死亡（例: plugin 側が parent token を誤って
  `DestroyWindow`）。slot 自体は再 lease 可能なため、
  `binding.lease`（mint 時に保持した token の写し）で
  `registry.release_lease(lease)` を呼び `LeaseRevoked` +
  `Declared` に戻す — `binding.clip`/`z` は保持され、次の
  `lease_parent` で新 container に再適用される。container は
  `abandon()`。

これにより:

- consumer の既存 vocabulary（`LeaseRevoked`/`Destroyed`）で
  teardown が観測可能。新しい `SlotEvent`/`SlotError` variant・
  `ruviform-ui` neutral 契約への API 追加は不要 — adapter が
  mint した lease token を `Binding.lease` に保持する方式を採り、
  `release_lease` の既存 API で発行者側 revoke を実現する。
- `Destroyed` は「consumer による release」と「owner death による
  auto-release」を区別しない終端 event — いずれも slot が二度と
  parent できない OS・帳簿上の事実であり、AC-X-1 の consumer
  lifecycle 所有（declare/lease/release の主体）は侵さない。
- `bindings` から除去されるため window open/close loop での
  単調増大が止まり、index 再利用で registry record も収束する
  （AC-405-4）。
- consumer の事後 `release_slot`/`release_lease` は
  `Released`/`LeaseRevoked` の typed error — 偽 `Ok` でなく
  冪等に収束する（AC-405-2）。

新しい状態（`Dead`/`OwnerLost` 等）を導入しない。`state()` には
`SlotState::Released` が既にあり、owner-dead を「Released と
区別されるべき第三状態」にする根拠は現行 consumer にない。

### D3（#405）: `&self` クエリは「sweep が記録するはずの実効状態」を返す

`state()`/`container_token()`/`validate_lease()`/`redeem()` は
`&self` で mutate できないため、`binding_death` を読み取り専用に
適用し、sweep 後の帳簿が答える値と一致する結果を即座に返す:

| クエリ | owner 死亡・sweep 未実施 | sweep 後 |
| --- | --- | --- |
| `state(slot)` | `Some(Released)` | `Some(Released)` → index 再利用後 `None` |
| `state(slot)` — container のみ死亡 | `Some(Declared)`（leased 時） | `Some(Declared)` |
| `container_token(slot)` | `None` | `None` |
| `validate_lease(lease)` | `Err(LeaseRevoked)` | `Err(LeaseRevoked)` |
| `redeem(payload)` | `Err(LeaseRevoked)` | `Err(LeaseRevoked)` |
| `capabilities(slot)` | `Err(Released)`（owner 死亡） | `Err(Released)`/`StaleGeneration`/`UnknownSlot` — id 解決は mutation op と同一 |
| `spec(slot)` | 記録上の宣言値（liveness でない） | 同左 |

いずれも index key のみでなく `binding.id == slot` で照合する —
index 再利用後の stale id が新世代 binding の container token・
死亡状態を盗み見ることを防ぐ（generation 安全性）。

`state()` が `Released` を先に答えてから `Destroyed` event が
`drain_events` で届く順序は一貫している — event は遷移通知、
`state()` は現在の実効状態であり、「sweep が記録するはずの値」と
一致させる。`capabilities` も同じ解決を通す — `Released` record
の caps を `Ok` で返すと state() との矛盾になるため、owner 死亡
検出時は `Err(Released)`（container のみ死亡 = `Declared` 存続中
は宣言面として正しいので据え置き）。`spec` は宣言面の記録であり
liveness ではないため据え置く（Released slot でも record が残る
registry の既存意味論に一致）。

### D4（#404-1）: `set_clip(None)` は `SetWindowRgn(hwnd, NULL, TRUE)`

`Container::set_clip` を `Result<(), SlotError>` 化し、
`local` が `None` のとき明示的に `SetWindowRgn(hwnd, NULL, TRUE)`
を発行して region を解除する（MSDN の正規解除手段）。
`Some(..)` の `CreateRectRgn` 失敗・`SetWindowRgn` 失敗は
`SlotError::Backend` で返す（region は失敗時に `DeleteObject` —
現行の所有契約を維持）。

### D5（#404-2）: `set_geometry` は成功後に保持 clip を再投影

`registry.set_geometry` 成功後、container がある場合に
`container.set_rect(rect)?` → `binding.clip.is_some()` なら
`container.set_clip(clip_local(binding.clip, geometry))?` で
再投影する（`lease_parent` の re-apply pattern と同型）。
`MoveWindow`・clip 再投影の失敗は `Err(Backend)` — geometry
適用済み・clip 未適用の部分失敗を `Ok` に潰さない（再試行で
自己修復する点のみ注意: 帳簿 geometry は正しく、region は旧
origin のまま残る）。

### D6（#404-3）: ledger 記録は OS 適用成功後 — `binding.clip` の順序修復

`set_clip` は binding/id 解決 → caps/geometry 検証の後、
`container` がある場合は先に `container.set_clip(...)?` を試し、
成功時のみ `binding.clip = clip` を記録する（container なしの
declared-only は記録のみ — `lease_parent` で再適用される）。
これで「帳簿が OS 事実を先行する」経路を断つ。

backend op の Result 化は失敗シグナルが存在するものに限定:

- `set_clip`: `Some(..)` は `CreateRectRgn` 失敗を先に
  `Err(Backend)`（NULL に落とさない — `SetWindowRgn(NULL)` は
  clip 解除として成功してしまう）。`None` は
  `SetWindowRgn(hwnd, NULL)`、失敗は `Err(Backend)`。
- `set_rect`/`raise`: `MoveWindow`/`SetWindowPos` の FALSE を
  `Err(Backend)`。
- `focus`: `SetFocus` の戻り値は**以前の** focus 窓（NULL も正当
  な成功）のため失敗判定に使えない — 呼出後 `GetFocus() == hwnd`
  で検証し `Err(Backend)`。
- `set_visible`: `ShowWindow` の戻り値は前の可視状態であり
  失敗シグナルではない — `()` のまま。

`lease_parent` は `Container::create`・`apply_recorded_state`
（clip/z/visibility 再適用）のいずれの OS 失敗でも
`registry.release_lease(lease)` で rollback し `Err` —
mint 済み lease が OS 失敗で残留する経路を断つ。さらに
`Container::create` は `IsWindow(parent)` guard を持ち、
dead parent HWND への attach を `Backend` で明示拒否する
（recycled 値への attach は `IsWindow` の検出限界として残存）。

### D7: 変更面を `gpui_driver/surface.rs` に閉じる

- `Binding` に `id: SurfaceSlotId`（auto-release に generation
  付き id が必要）と `lease: Option<SlotLease>`（container 死亡
  時の発行者側 revoke 用の token 写し —
  `container.is_some()` と同期）を保持。
- `sweep_dead` を `declare_slot`/`release_slot`/`set_geometry`/
  `set_visibility`/`set_clip`/`set_zorder`/`request_focus`/
  `lease_parent`/`release_lease`/`mark_busy`/`clear_busy`/
  `drain_events` の各 `&mut` 入口に置く（trait `drain_events`
  は同名 inherent 実装に委譲）。
- `dead_slot_error(slot)`: binding 欠落時の typed error 解決 —
  `bindings.get(index)` が別世代 id を持てば `StaleGeneration`、
  registry record が `Released` なら `Released`、index に
  tombstone record が残っていれば `StaleGeneration`（registry の
  `index_of` と同じ判別）、それ以外は `UnknownSlot`。
  tombstone 判別は adapter 側の `released: BTreeSet<u32>` で追跡
  （auto-release/consumer `release_slot` で insert、index 再利用
  で remove — neutral registry への API 追加を避けるための
  adapter 局所簿記）。caps gate より先に `binding.id == slot`
  照合を行い、stale id が capability 検査にすり抜けないようにする。
- `impl Drop for GpuiSurfaceAdapter`: 最後の sweep 以後に死亡した
  container が `Container::drop` の無条件 `DestroyWindow` に
  到達しないよう、adapter drop で `!container.is_alive()` の
  container を `abandon()` する。sweep_dead は使わない —
  `inner` が teardown 中に借用中で panic できるため、container
  probe のみで検出する（owner 死亡は OS が child HWND を道連れ
  に破棄するため、検出可能な死亡は container probe で網羅される）。
- `lease_parent` の binding `get_mut` は `let-else` で mint 済み
  lease の rollback を明示（到達不能だが invariant をコードで
  表明する）。
- `lib.rs`: `Inner.destroyed` に append-only 依存のコメント追加
  のみ（コード変更なし）。`ruviform-ui` neutral 契約・PoC
  harness は**無変更**。
- RF387 の `bound(&inner)`/`surface()`/`bind_native_slot`
  (`state()`→Released/None gate) とは同じ `inner` を観測する
  構成のまま整合する想定 — candidate PR406 未 merge のため、
  canonical 経路との適合は merge 時の統合確認事項として残す
  （本 branch では `new(&driver)` 経路のみで検証）。

## Data and Control Flow

```text
consumer op / drain_events
  → adapter.sweep_dead()                        (&mut 入口)
      ├─ inner.destroyed.contains(window)      ← retire の終端記録
      ├─ IsWindow(raw_parent) == false         ← 未記録死亡（全 slot）
      └─ container.is_alive() == false         ← container のみ死亡
      → Owner:     bindings.remove → container.abandon()
                   → registry.release_slot(id) → LeaseRevoked+Destroyed
      → Container: binding.lease → registry.release_lease
                   → LeaseRevoked + Declared、container.abandon()
  → 残りの slot は live — id 照合 + registry 検証が typed error
    を保証（Released/StaleGeneration/Backend/LeaseRevoked）
```

```text
set_clip(slot, clip):
  sweep → binding.id==slot 照合（StaleGeneration/Released/UnknownSlot）
  → caps.clip + spec.geometry 取得
  → container あれば container.set_clip(clip_local(clip, geometry))?
      None → SetWindowRgn(hwnd, NULL)            （#404-1）
      Some → CreateRectRgn（失敗→Backend）+ SetWindowRgn、
             失敗は DeleteObject+Backend
  → 成功時のみ binding.clip = clip               （#404-3）

set_geometry(slot, geometry):
  sweep → registry.set_geometry（検証+記録+GeometryChanged）
  → container あれば set_rect? + clip 保持時は clip_local で再投影?（#404-2）
```

## Non-Functional Constraints

- `unsafe` は既存 `#[expect(unsafe_code)]` の backend mod 内に限定。
  新規 unsafe 節なし（テストの HWND 操作は test mod の
  `#[expect(unsafe_code)]` に局所化）。
- 新規依存・feature・workflow 追加なし。`windows-sys` 既存 feature
  内で完結（`GetWindowRgn`/`GetRgnBox`/`CreateRectRgn` は
  windows-sys では `Win32_Graphics_Gdi` — 有効化済み）。
- RT 契約不変（UI スレッドのみ）。公開 API 面に破壊的変更なし —
  `GpuiSurfaceAdapter` の公開 signature は同一、意味の精緻化のみ。

## Known limits（検出限界 — 後続検討事項）

- **`raw_parent` 未捕捉の owner**: open 時に `raw_parent` を
  取得できなかった window（Windows backend では常時捕捉されるが
  契約上 `Option`）は、`destroyed` に記録されるまで owner 死亡を
  declared-only binding で検出できない（container probe のみ）。
  `retire` 到達で収束する。
- **HWND recycling の検出限界**: 死亡済み handle 値が新しい
  無関係 window に再利用されると `IsWindow==true` が返り、
  container/owner 死亡は検出不能で `DestroyWindow` が外部
  window に届きうる。probe 系（`abandon` 選択含む）共通の
  残留リスクであり、本層では区別不能 — 必要なら
  `GetWindowThreadProcessId` 等の追加シグナルを別途検討する。
- **`capabilities`/`spec` の record 意味論**: tombstone が残る
  限り宣言面を返す registry の既存挙動 — adapter の
  `capabilities` は死亡検出で絞ったが neutral 側の `spec` は
  据え置き（契約変更を要する）。

## Alternatives considered

- **Push hook（Weak watcher）**: `Inner.surface_watchers:
  Vec<Weak<RefCell<Shared>>>` + adapter 全体を shared-state 化し、
  `retire` が `upgrade` して即時 ledger 更新。`&self` クエリが
  常に帳簿 literal に一致し、container が死亡瞬間に drop される
  利点があるが、(a) lib.rs 変更が RF387 の大規模改修と衝突、
  (b) adapter 内部を `Rc<RefCell>` 共有化する構造転換が本 bugfix
  の範囲を超える、(c) `destroyed` が既に authoritative なのに
  第二の通知 channel を増設する — を理由に退けた。
- **dead flag 保持（auto-release しない）**: `binding.dead` 印で
  ops だけ `Backend` にする案は、`state()` が `Declared`/`Leased`
  を返し続け・consumer release まで `bindings`/record が無期限
  残留するため orphan 成長を解決しない。
- **`Destroyed` に owner-death 区別 variant を追加**: neutral
  契約（`ruviform-ui`）への公開 API 追加が必要で、終端性は現行
  vocabulary で足りる。区別が必要になった時点の拡張点とする。

## Verification

- `crates/ruviform-ui-gpui` の `#[cfg(all(test, windows))]` test
  （`mise run test-ui-gpui`、実 OS session・非表示 test HWND・
  message loop 不要の同期 API のみ使用）。window 製造は
  `GpuiDriver::new()` + `create_window`（id 予約のみ）+ test 側で
  `inner.windows[id].raw_parent = test_hwnd` を注入し、teardown は
  `super::retire(&inner, id)`（本物の終端経路）または test window
  への直接 `DestroyWindow`（未記録死亡経路）で再現する:
  - `set_clip(Some)`→`GetWindowRgn` region 存在 → `set_clip(None)`
    → region なし（AC-404-1・RED→GREEN）
  - `set_clip(Some(C))` → `set_geometry(G2)` → `GetRgnBox` が
    `clip_local(C, G2)`（AC-404-2・RED→GREEN）
  - declare/lease → `retire` → `drain_events` が
    `LeaseRevoked`+`Destroyed`・`state()` が `Released`・各 op が
    typed error・`container_token` None・`redeem` `LeaseRevoked`
    （AC-405-1..4・RED→GREEN）
  - retire 未経由の owner `DestroyWindow`（raw_parent probe）・
    declared-only slot の owner destroy・container 単独死亡→
    Declared+再 lease・repeated destroy・release-after-destroy・
    index reuse generation・dead parent への lease 拒否・adapter
    drop で container 破棄。
- `mise run fmt-check`/`check`/`lint`/`test`/`check-ui-gpui`/
  `lint-ui-gpui` green。非 Windows OS 行は compile-level のみ
  （backend stub・`Unsupported` 経路不変）。
