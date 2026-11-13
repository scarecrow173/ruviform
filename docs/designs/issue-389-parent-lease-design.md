---
id: "DESIGN-034WXSz6r3CcQjiLo08b8H"
type: "design"
status: "approved"
title: "Issue #389 Parent Lease 公開 API 移行設計"
created: "2026-11-13"
updated: "2026-11-13"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-389-parent-lease.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-parent-lease.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
  issues: [389]
---

# Issue #389 Parent Lease 公開 API 移行設計

正本 spec: `docs/specs/issue-389-parent-lease.md`。
前提: PR #427 の移行表（`ParentSlotLease` session 側語彙の導入と
`LeaseRedeemer` による raw token 境界の隔離）は本設計の出発点。

## 決定事項

### D1: `ParentLease` — generation 付き不透明型を plugin-core に新設

`crates/ruviform-plugin-core/src/boundary.rs` に
`ParentLease { generation: u32, payload: u64 }` を追加する。

- `generation`: 発行側 lease の世代（`ruviform-ui` の
  `SlotLease::seq()` に相当）。lease 発行 epoch を型に束縛し、
  stale/forged lease の構造的識別を可能にする。
- `payload`: redeem 済みの raw OS parent token（`u64` に narrow
  された HWND/`NSView` 等）。plugin 側 adapter が OS 境界でのみ
  解釈する。
- `new(generation, payload)` は `#[doc(hidden)]` public — mint は
  adapter/redeemer 内部操作。session 面は受け取るだけ。
- `generation()`/`payload()` accessor は `const`。
- `Copy`/`PartialEq`/`Eq`/`Hash`/`Debug` derive、RT-suitable 契約を
  明記（`GuiParentToken` と同じ value-type 性質）。

`ParentLease` は **認証ではなく opaque vocabulary** である。
stale/revoked/forged lease の拒否は引き続き redeem 時の ledger
検証（`SurfaceSlotRegistry::validate_lease` → `SlotError`）が
権威であり、`generation` はその結果を型として運ぶための世代情報。
同一プロセス trust boundary 内の語彙であり、trust boundary を
越える場合は別途認証が必要（`SlotLease` の既存契約と同じ）。

### D2: `GuiParentToken` を公開 API から除去

`GuiParentToken` 型自体を `boundary.rs` から削除し、以下を
`ParentLease` に置き換える:

| 旧 API | 新 API | 保持方針 |
| --- | --- | --- |
| `GuiParentToken(u64)` | `ParentLease{generation,payload}` | 型ごと置換。`new`/`raw` → `new(generation,payload)`/`generation()`/`payload()` |
| `Plugin::open_gui(GuiParentToken)` | `Plugin::open_gui(ParentLease)` | シグネチャ置換。default は `Unsupported` のまま |
| `ErasedPlugin::open_gui(GuiParentToken)` | 同上 `ParentLease` | 委譲のみ、置換 |
| `HostCommand::OpenGui{parent: GuiParentToken}` | `parent: ParentLease` | in-process command、置換 |
| `wire::Command::OpenGui{parent: GuiParentToken}` | `parent: ParentLease` | wire 語彙置換（下記 D4） |
| `encode/decode_gui_parent_token` | `encode/decode_parent_lease` | u64×1 → u32+u64 の 12 bytes |
| `LeaseRedeemer -> GuiParentToken` | `LeaseRedeemer -> ParentLease` | 戻り値型のみ置換。検証責務は不変 |
| `ParentSlotLease`（session 側） | 変更なし | session-facing 語彙として保持 |

`GuiParentToken` の完全削除を選ぶ理由: 本スライスの編集範囲内の
全使用箇所（plugin-core/host/vst3/clap/poc/ui-gpui docs）を移行
可能であり、`ruviform-plugin-export` は `parent` を pattern bind
で opaque に転送するため型名に非依存でコンパイルが維持される。
「raw token 語彙を低層に残す」旧設計（PR #427 時点）は、本タスクが
明示的にその残件の解消を要求するため supersede される。

### D3: redeem seam は `ParentLease` を生成する

`LeaseRedeemer<'a> = dyn FnMut(&ParentSlotLease) -> Result<ParentLease, EditorOpError>`。

composition 側（`ruviform-poc-native-dock`）の redeemer は:

1. `adapter.redeem(lease.raw())` で ledger 検証 + OS token 取得
   （stale/revoked → `SlotError` → `EditorOpError` 写像は現行維持）
2. `SlotLease::new(lease.raw()).seq()` で generation を復元
   （`ParentSlotLease` の payload は `SlotLease` の packed
   `{index,seq}` であり、poc は既に `SlotLease` を import 済み）
3. `ParentLease::new(generation, token)` を返す

これにより「検証済み lease のみが `ParentLease` になる」不変条件が
redeem seam に集約される。`plugin-host` 自身は generation の
意味を解釈しない（opaque に運ぶだけ）。

### D4: wire `OpenGui` の符号化変更

`wire::Command::OpenGui` の parent フィールド型変更に伴い、
`encode_parent_lease`/`decode_parent_lease` を新設し
`generation:u32` + `payload:u64` を連結して符号化する
（既存 `push_u32`/`push_u64`・`read_u32`/`read_u64` ヘルパー使用、
フィールド順は generation → payload）。

wire protocol のバージョニング: `OpenGui`/`CloseGui` は L1 GUI
未実装のため実運用 consumer は存在せず、符号化変更は実害を持た
ない。verb 自体は将来の L1 GUI 用に残し、動作を主張しない。
`plugin-export` の child runner は `parent` を opaque に
`plugin.open_gui(parent)` へ転送するため変更不要。

### D5: adapter 側の OS handle 閉じ込め

- `ruviform-plugin-vst3/src/host.rs`: `parent.raw() as *mut c_void`
  → `parent.payload() as *mut c_void`。`generation` は adapter では
  未検証（検証は redeem seam の責務）。非 Windows の
  `Unsupported`・二重 open・未初期化 open の既存契約は不変。
- `ruviform-plugin-clap`: `open_gui`/`close_gui` は default
  `Unsupported` のまま。CLAP GUI の実装は追加しない。
- `ruviform-ui-gpui/src/gpui_driver/surface.rs`: `redeem(payload)`
  のシグネチャ・ledger 検証は不変。doc コメント内の
  `GuiParentToken`/`ParentSlotLease` 言及を `ParentLease` 語彙に
  合わせて更新（動作変更なし）。
- `ruviform-poc-native-dock`: 3 箇所の redeemer closure を
  `ParentLease` 生成に更新。`HostCommand::OpenGui` を直接組み立てる
  経路（test fixture 等）があれば同様に更新。

### D6: session API の非露出を維持

`EditorSessionOps`/`PluginEditorSessions` は `ParentSlotLease` を
受ける現行契約のまま。`ParentLease` は `HostCommand`/wire/plugin
層の語彙であり、session-facing surface には出さない。
`PluginEditorSessionId`/`ParentSlotLease` の定義は不変。

## L1 契約

- `L1EditorOps` は全操作 `Unsupported` のまま（refusal-only）。
- `l1/mod.rs` の `OpenGui`/`CloseGui` 拒否は不変。
- wire `OpenGui` は語彙として残るが L1 GUI 実装を主張しない。
- silent fallback L1→L0 なし。

## 検証境界

- headless: plugin-core wire roundtrip・host command/session
  contract・vst3/clap build/link・poc-native-dock compile。
- 実 VST3/CLAP GUI・Native Dock 実成立・外部 plugin 受入は
  NotRun（#388/#532/SV#176 gate）。Session headless Pass から
  実 Plugin Pass を主張しない。
- cross-repo consumer（audio-lens/synvelis）への公開 API 影響は
  PR 本文に明記（RF #219 互換契約）。
