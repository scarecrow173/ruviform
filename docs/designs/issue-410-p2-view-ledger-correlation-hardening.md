---
id: "DESIGN-034UwcZWs6jYHWBRpUdA8D"
type: "design"
status: "approved"
title: "Issue #410 P2 — view ledger correlation hardening"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
    - "https://github.com/scarecrow173/ruviform/pull/502"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
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
    - "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/designs/issue-410-view-command-ledger-bounded-retention-correlation-semantics.md"
    - "docs/specs/issue-387-component-view-specs.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
  derived-by:
    - "docs/plans/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 P2 — view ledger correlation hardening

## Context

slice 1（PR #502 / `docs/designs/issue-410-view-command-ledger-
bounded-retention-correlation-semantics.md`）で `ViewHost`/
`CommandBinding` の retention 系 hardening は実装済み:

- `command_index` は `MAX_COMMAND_INDEX` = 512 で liveness-class
  eviction（Dead → Settled → LiveNonExpect → LiveExpect、全
  LiveExpect なら yield）。
- `CommandBinding::outbox` は `MAX_OUTBOX_DISPATCHES` = 64 で
  drop-oldest bound。権威ある配送経路は `ViewEvent::Intent`。
- `push_event` の 512-cap は droppable 優先 + 最古 drop の正直な
  契約で、drop された `Intent` は `retire_undelivered_intent` が
  receipt routing を除去し pending record を delivery-lost
  （`Timeout`）にする。
- `validate_spec` が実効 command owner id の重複を reject し、
  `key_for_control` は canonical mount 優先で detach 帰属を一意化。
- 全単調 `u64` カウンタは `checked_add(1).expect(...)` 化済み。

残る未対応は projection correlation である。`apply_projection`
（`view.rs`）は `control_id` のみで binding に相関し、revision
順序の guard は `CommandBinding::on_projection` の
`last_observed.revision` との比較だけが担う。`unmount` は
`bindings.clear()` で binding を全て捨てるため、remount 後の
fresh binding は `last_observed = None` から始まり、**remount 前に
発行された古い revision の projection が一度だけ受理され得る**
（次の新しい projection で自己修復するが、その一度の誤適用は
controlled display に stale 値を書き込み得る）。`detach` →
`attach` では binding が存続するため `last_observed` guard が効く
が、投影が detached 中に drop される点は契約として固定する必要が
ある。

envelope への epoch segment 追加は wire-protocol 変更であり、
slice 1 spec の Deferred Design Concerns として別スライスに
譲られている。本設計は wire 変更なしに host 側で実装可能な
correlation 強化 — **受理済み revision の高水位マーク
（projection floor）を attach epoch の存続期間中維持する** — を
定義する。

## Scope

### In Scope

- `ViewHost` への `control_id` 別 projection revision floor の
  追加と、`apply_projection` での強制。
- `CommandBinding::on_projection` の revision 比較を
  `pub(crate)` helper に抽出し、floor と binding guard が同一の
  advance 規則を共有すること（DRY: 同じ順序規則の二重実装を
  避ける）。
- epoch 境界（detach→attach・unmount→mount）での順序逆転・
  remount 誤適用を固定する回帰 unit test。
- slice 1 実装の受入条件 5 行に対応する既存 test の棚卸しと、
  不足分の補完。

### Out of Scope

- projection envelope（`Event::Notify` `data` /
  `StateProjection`）への epoch segment — wire-protocol 変更。
- `ruviform-ui-test`/`ruviform-ui-gpui`/driver/consumer の変更。
  `StateProjection` の field 追加は `ruviform-ui-test` の struct
  literal 構築を壊すため本 slice では不可能（制約として記録）。
- `binding_mut()` 経由の `on_projection` 直接呼出し — host の
  canonical entry point（`apply_projection`/`deliver_event`）が
  floor を強制する契約とし、直接 binding 操作は caller が
  correlation を所有する上級 API として文書化する（receipt 側の
  `command_index` 迂回と同じ既存の境界）。
- `PendingRecord` dead-generation zombie、reentrant-borrow nit
  （レビュー記録のみ）。

## Components and Boundaries

- `ViewHost::projection_floors: HashMap<ControlId, Revision>`
  （新設・private）: `control_id` ごとの「受理済み projection
  revision の高水位」。`unmount`/`detach`/`update` のいずれでも
  clear しない — attach epoch（`ViewHost` の存続期間）の終端は
  `Drop` のみであり、floor はその全期間で維持する。支配する不変
  条件: 「この host が一度でも受理した projection より古い
  revision は、以後どの binding epoch でも受理しない」。
- `ViewHost::apply_projection`: canonical な projection 入口
  （`deliver_event` の `Notify` decode・外部 caller の双方が
  通る）。floor 検査 → `binding.on_projection` → 受理時に floor
  更新 → controlled display 駆動、の順で処理する。
- `CommandBinding::on_projection`: binding 内の revision guard
  （`last_observed` 比較）は不変。floor は binding の外側
  （host レベル）に置く — binding は unmount で破棄されるため、
  epoch を跨ぐ状態は host が所有するのが正しい境界。
- `crate::behavior::revision_advances`（新設・`pub(crate)`）:
  `Revision` の advance 判定を一箇所に集約。`on_projection` と
  floor 検査が同じ規則を使う。

## Data and Control Flow

### projection の受理経路（変更後）

1. `deliver_event(Event::Notify)` または直接
   `apply_projection(control_id, projection)`。
2. `projection_floors[control_id]` が存在し、
   `!revision_advances(floor, projection.revision)` なら
   `false` を返して drop（unrouted と同じ誠実な棄却 — 状態を
   捏造しない）。
3. `bindings[control_id].on_projection(projection)` — binding
   存在・`attached`・`last_observed` 比較の既存 guard。
4. 受理（`true`）なら `projection_floors[control_id] =
   projection.revision` に更新し、従来どおり
   `projection_value_input` → `control.apply_external` で
   controlled display を駆動する。

floor は受理時のみ更新する。棄却された projection は floor を
動かさない（古い値で floor を巻き戻さない）。binding の
`last_observed` と floor は binding 存続中は同じ値を取るため、
floor が実効的に効くのは binding 再生成後（`last_observed =
None`）の最初の projection に対してである。

### epoch 境界ごとの保証

| 境界 | binding | last_observed | floor | 古い projection の扱い |
| --- | --- | --- | --- | --- |
| 同一 mount 内・順序逆転 | 存続 | guard | guard | `on_projection` が drop |
| `detach` → `attach` | 存続（generation bump） | guard | guard | detached 中は `attached=false` で drop、reattach 後は revision guard で drop |
| `unmount` → `mount` | 破棄→再生成 | `None` にリセット | **維持** | floor が remount 前の高水位を保持し、advance しない revision を drop |
| `ViewHost` 破棄→新規 host | 別 host | `None` | `None` | floor の管轄外 — delivery path が attach_epoch で pin する義務（契約として文書化） |

### revision advance 規則（`revision_advances`）

`CommandBinding::on_projection` の現行比較をそのまま helper 化
する: `Num(a)` vs `Num(b)` は `b > a`、`Str(a)` vs `Str(b)` は
`b != a`、型跨ぎは常に advance。`Str` revision は順序を持たない
ため「異なる string は常に新しい」とする既存の近似を floor も
共有する（契約として明記 — 順序保証が必要な consumer は `Num`
を使う）。

## Contract（correlation semantics の定義）

- **revision 単調性のスコープ**: `Revision` は `control_id` ごとに
  `ViewHost` の attach epoch 存続期間で単調な順序 token でなければ
  ならない。`unmount`/`mount`（mount_epoch 増分）はこのスコープを
  リセットしない — floor が跨ぐ。revision domain を本当に
  リセットする consumer は、attach epoch 境界（新しい
  `ViewHost`/`AttachEpoch`）でのみ行うこと。これは binding 内で
  既に仮定されていた単調性契約を epoch 境界へ一貫させるもので
  あり、新しい失敗型ではない（binding 内で revision を巻き戻す
  consumer は現行でも display が凍結する）。
- **floor の意味**: 「この host が受理した最後の revision」
  ではなく「受理し得る最小の次 revision」。受理のたびに更新
  されるため実装上は high-water mark と同値だが、契約としては
  「advance しないものは stale」という guard である。
- **delivery path の義務（host が検証できない残余）**: floor は
  同一 `ViewHost` 内の revision 順序だけを検証する。別 host への
  誤配送・attach_epoch を跨ぐ receipt/projection の混入・
  revision 単調性そのものの違反は、envelope に epoch segment が
  無い以上 host には検証不能であり、delivery path（driver/
  adapter、ScenarioTarget の attach_epoch pin 等）が負う。wire
  envelope への epoch segment 追加は deferred のまま。
- **`binding_mut()` 迂回**: `apply_projection`/`deliver_event`
  が canonical entry point であり floor はそこで強制される。
  `binding_mut().on_projection(...)` の直接呼出しは host の
  guard を通らない上級 API であり、correlation の責務は caller
  にある（`on_receipt` が `command_index` を迂回する既存境界と
  同型）。

## Non-Functional Constraints

- 公開 API 互換: `apply_projection`/`deliver_event`/`on_projection`
  の signature は不変。`StateProjection`/`Revision` の型は不変
  （`ruviform-ui-test` が struct literal で構築するため field
  追加不可 — 調査済み）。`projection_floors` は private。
- RT 制約: non-RT の UI 側契約。floor 検査は `HashMap` 参照 1 回
  + `Revision` 比較で、projection 受理頻度では問題にならない。
- consumer 影響: revision を単調に維持している consumer は無影響。
  remount を跨いで revision を意図的に巻き戻す consumer は今後
  stale drop される — これは本 slice が意図する契約変更であり、
  spec の Requirements に明記する。

## Risks and Trade-offs

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | consumer が revision domain をリセットし、floor が全 projection を永久 reject して display が凍結 | low | high | 契約として「revision は attach epoch 内で単調」を明記。binding 内の既存 guard と同じ失敗型であり新規リスクではない。envelope epoch segment（deferred）が来れば delivery path 側でより強く検証可能 |
| 2 | floor が binding の `last_observed` と二重管理になり不整合 | low | low | floor は受理時のみ更新され binding の `last_observed` と同じ advance 規則・同じ値を取る。binding 存続中は floor 判定が binding guard を包含するため実効差分は remount 直後のみ |
| 3 | `Str` revision の unordered 性で floor が誤判定 | low | low | `b != a` の既存近似を共有。順序保証が必要な consumer は `Num` を使う契約を文書化 |
| 4 | `binding_mut()` 経由の迂回で floor が効かない | low | low | canonical entry point を契約化し doc comment に明記。receipt 側の同型境界と一貫 |
| 5 | floor map が host 生存中に増え続ける | very low | low | key は `control_id` で spec 規模に bound され、mount 毎に消えないのは attach epoch あたりの distinct control 数まで。`bindings`/`mounted` と同程度のメモリ特性 |

## Review Record（wraparound / reentrant borrow 契約）

Issue 受入条件の末尾要件として、以下を記録する。

- **wraparound**: `generation`/`mount_epoch`/`action_sequence`/
  `CommandBinding::{counter,generation,observed_seq}`/
  `InteractionControl::{sequence,atomic_counter}`/
  `InteractionDispatcher::synthetic_seq` は slice 1 で全て
  `checked_add(1).expect(...)` 化済み（`AttachEpochAllocator::
  mint` 先例）。u64 枯渇は到達不能だが wrap は順序を静かに壊す
  ため、正直な panic を契約とする。直接 panic を持つ public
  関数には `# Panics` 節を付与済み。本 slice で新規に単調
  カウンタは追加しない（floor は `Revision` の比較のみで
  独自の連番を持たない）。
- **reentrant borrow**: `GpuiSurfaceAdapter` の driver `Inner`
  と adapter 間に nested borrow 経路が現れた場合の latent hazard
  は本 slice の範囲外（rf388-s3 領域）。現行コードに reentrant
  経路は存在せず、発生時は borrow 規則違反として検出される
  設計（`&mut self` メソッドが外部 callback を呼ばない）を維持
  する。契約の明示的 pin として、ViewHost の public メソッドは
  引数として `&mut self` 以外の host 内部状態への参照を受け取ら
  ないことをここに記録する。

## Deferred Design Concerns

- projection envelope への attach/mount epoch segment: wire
  protocol 変更。floor はその先行実装として位置づけ、envelope
  側が決まれば floor は防御的多層の一段として残せる。
- `PendingRecord` dead-generation zombie: slice 1 からの持ち越し。
- reentrant-borrow nit の実修正: rf388-s3。

## Verification Notes

- [x] `mise run fmt-check` / `mise run check` / `mise run lint` /
  `mise run test` / `mise run test-rt` が exit 0。
- [x] `mise run test-ui-*` の Rust 側 lane（component-view /
  driver-contract / parity / native-slot / golden /
  scenario-contract / gpui / consumer-observation /
  test-msrv / webview）が exit 0。`ui/*` TypeScript package
  向け lane は本 crate に非依存のため NotRun（impl record
  Validation 参照）。
- [x] 新規 unit test: remount 後の stale revision reject・
  detach/attach またぎの順序逆転 reject・detached 中 drop・
  control_id 別 floor 独立・`deliver_event` 経路の floor 強制・
  floor が受理時のみ更新されること。
- [x] slice 1 の対応 test（`command_index_*`・`outbox_*`・
  `event_backlog_drop_*`・`duplicate_command_owner_id_*`・
  `tab_alias_*`・`alias_invocation_*`）が pass し続けること。
