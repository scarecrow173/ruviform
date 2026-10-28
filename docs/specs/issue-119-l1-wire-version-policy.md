---
id: "SPEC-034NhJTrJ1kCCldVF5XZKB"
type: "spec"
status: "implemented"
title: "Issue #119 L1 wire バージョン方針（単一最新バージョン・交渉機構なし）"
created: "2026-10-27"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/119"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "docs/designs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/specs/plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-wire-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-proxy-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-runner-e2e-tests.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/implement-issue-119-l1-wire-version-policy.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/issue-115-l1-gui-cross-process-parenting.md"
    - "../impl/ir/issue-76-l1-core-wire-task-1-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
  derived-by:
    - "../plans/implement-issue-119-l1-wire-version-policy.md"
    - "../discovery/issue-117-bundle-child-process-discovery.md"
    - "issue-117-bundle-child-process.md"
    - "../designs/issue-117-bundle-child-process.md"
    - "../plans/implement-issue-117-bundle-child-process.md"
    - "../tasks/l1-wire-v2-codec-cutover.md"
    - "../test-specs/issue-117-wire-v2-codec-tests.md"
    - "issue-118-video-l1.md"
    - "../designs/issue-118-video-l1.md"
    - "../adr/issue-118-video-l1-create-path-segment.md"
    - "../plans/implement-issue-118-video-l1.md"
    - "../tasks/l1-wire-v3-video-codec.md"
  verifies: []
  verified-by:
    - "../test-specs/plugin-l1-wire-codec-tests.md"
    - "../test-specs/plugin-l1-proxy-contract-tests.md"
    - "../test-specs/plugin-l1-runner-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/115"
  defers: []
  deferred-by: []
---

# Issue #119 L1 wire バージョン方針（単一最新バージョン・交渉機構なし）

## What Should Be Built

Issue #119 は新しいプロトコル機構の実装ではなく、**既に実装済みの挙動を束ねる恒久ポリシーを設計判断として記録し、記録した契約に対する拒否動作のカバレッジを検証する** Issue である。成果物は 2 系統:

1. **方針の記録** — L1 wire プロトコルのバージョニング方針を、正本である `docs/designs/plugin-l1-out-of-process.md`（決定ブロック）・`docs/specs/plugin-l1-out-of-process.md`（Requirements 条項）・`crates/ruviform-plugin-core/src/wire.rs` の `//!` Versioning 節へ記録し、相互に整合させる。あわせて両文書の Deferred Design Concerns 行を「初版 `WIRE_VERSION=1` で十分」という方針不在を示す記述から、Issue #119 の確定判断を指す記述へ reconcile する。
2. **契約検証** — version mismatch・予約フィールド・`WIRE_MAX_*` の既存拒否契約が実装とテストで担保されているかを確認し、文書化された契約に対してカバレッジが欠落している箇所に限りテストを追加する。

確定する方針本文:

- サポートする wire プロトコルは常に**最新の単一バージョンのみ** — 旧バージョンの decode・変換・フォールバック・共存は実装しない。
- `WIRE_VERSION` は通信契約の世代を表す単一整数 — 非互換なプロトコル変更時にのみ更新し、SemVer/Cargo バージョンとは連動させない。
- `Hello.version` は厳密一致・不一致は handshake 失敗（child kill/reap + `HostError::Ipc`）— バージョン交渉機構は追加しない。SHM 側も同一契約: セグメント header の `wire_version` フィールドは `WIRE_VERSION` との厳密一致で検証する。
- 非互換変更 → `WIRE_VERSION` bump + ホスト/ランナーの同一版更新。内部リファクタや wire に影響しない Rust API 変更では bump しない。
- `Hello.capabilities` は予約領域のまま非ゼロ `Malformed` 拒否を維持 — capability bit の割当はそれ自体がプロトコル変更であり現時点では行わない。
- `WIRE_MAX_*` は安全上限として維持 — 変更時は送受信受理範囲・SHM 容量・既存拒否動作への影響をレビューし、相互運用を壊す変更は bump 対象とする。上限交渉機構は設けない。
- 複数バージョン対応は、独立配布されるホスト/ランナー間の互換が必要になった段階でのみ再検討する。

## Why Now

- `wire.rs` は strict-equality の version 照合・reserved 拒否・上限チェックを **実装として** 既に持つが、「なぜ単一バージョンのみか」「いつ bump するか」「交渉を入れないのはなぜか」が設計判断としてどの文書にも記録されていなかった。design の Deferred Design Concerns 行は「初版 `WIRE_VERSION=1` で十分」とだけ述べ、方針の不在を示していた。
- Issue #115（L1 GUI 越境 parenting）の re-engagement 前提 (a) が「wire versioning 枠組みの確定」を要求しており、本 Issue の決定がその blocking 条件を解消する。
- 方針が記録されないままでは、将来の wire 変更者が bump 判断・同期更新義務・交渉非実装の根拠をコードから再発見する必要があり、 deferred concern の reopen 条件も曖昧なまま残る。

## Users and Value

- **wire 変更者・レビュアー**: `WIRE_VERSION` bump の判断基準・ホスト/ランナー同期更新義務・「交渉機構を入れない」判断の設計根拠を文書で参照できる。
- **Issue #115 等の後続設計**: blocking 前提 (a) が確定済みとなり、GUI 動詞追加が「`WIRE_VERSION` bump を要する非互換変更」として位置付けられる。
- **契約検証の利用者**: 拒否契約（version mismatch・reserved・上限）の根拠文書と、その契約を固定するテスト名の対応が明示される。

## Scope

### In Scope

- 方針の記録: `docs/designs/plugin-l1-out-of-process.md` の決定ブロック追加、`docs/specs/plugin-l1-out-of-process.md` の Requirements 条項追加、`wire.rs` `//!` Versioning 節の拡張 — 3 箇所を相互整合させる
- 両文書の Deferred Design Concerns 行の reconcile（`issue-115` design の前提 (a) 記述と L1 3 本の test-spec の deferred/non-goal 記述も同方針へ整合）
- 既存拒否契約の棚卸しと、文書化契約に対するカバレッジ欠落箇所のみのテスト追加（runner 側 shm header `wire_version` 不一致 → 非ゼロ終了の 1 本）
- spec/plan/task/IR の作成と README index 更新・関係整合

### Out of Scope

- バージョン交渉機構・互換レイヤー・旧バージョンの decode/変換/フォールバック/共存 — 本 Issue はこれらを**実装しないことを方針として確定**する側
- `Hello.capabilities` の bit 割当・`WIRE_MAX_*` 値の変更・上限交渉機構
- 製品コードの新機構全般（proxy handshake・runner・codec の挙動変更 — 既存挙動が既に方針どおりのため変更しない）

## Requirements

- 記録される方針は上記「確定する方針本文」の全条項を含み、design（決定ブロック）・spec（Requirements 条項）・`wire.rs` module doc の 3 箇所で矛盾なく表現されること
- Deferred Design Concerns の reconcile は「複数バージョン共存・交渉機構」を継続 deferred とし、reopen 条件を「独立配布されるホスト/ランナー間のバージョン互換要求」として明示すること（無期限の曖昧 defer や「将来交渉する」という含意を残さない）
- 契約検証は既存テストの名前を挙げてカバレッジ対応を示し、欠落が確認された契約に対してのみテストを追加すること（製品コードへ version override 等の検証用機構を追加しない — テストは意図的に破損した fixture header で検証する）
- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` と doc audit が全て通過すること

## Acceptance Criteria

- [x] **AC1**: 単一最新バージョン・厳密一致 handshake・交渉非実装・bump 判断基準・capabilities 予約維持・`WIRE_MAX_*` 固定上限・再検討条件の全条項が、design「ワイヤバージョン方針」ブロック・spec Requirements 条項・`wire.rs` `//!` Versioning 節に記録され相互整合する
- [x] **AC2**: 既存拒否契約とその固定テストの対応が確認されている — `Hello.version` 厳密一致は `handshake_version_mismatch`（proxy E2E）+ `e2e_version_mismatch_is_a_parent_side_handshake_failure`（runner E2E）、capabilities/reserved 非ゼロ `Malformed` は `decode_rejects_nonzero_reserved_and_bad_utf8`・`hello_roundtrip_and_version_is_carried_verbatim`、`WIRE_MAX_*` は `decode_rejects_declared_lengths_over_wire_max`・`decode_rejects_impossible_counts_without_allocating`、shm header 検証は `shm_header_roundtrip_matches_wire_layout`・`shm_layout_offsets_are_self_consistent`・`e2e_shm_header_mismatch`
- [x] **AC3**: カバレッジ欠落として特定された runner 側 shm header `wire_version` 不一致拒否に `e2e_shm_header_wire_version_mismatch`（`l1_runner_e2e.rs`）を追加し、`wire_version = WIRE_VERSION + 1` の segment で runner が `Hello` 送出後に非ゼロ終了することを固定
- [x] **AC4**: バージョン交渉・互換レイヤー・capability bit 割当・上限交渉・旧バージョン decode/変換/共存のいずれも実装されていない（差分に該当機構が存在しない）
- [x] **AC5**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス + `doc_status.js audit` `blocking: 0`・`findings: []` + 本 IR が impl-record audit で findings ゼロ

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| 複数 wire バージョン共存・交渉機構・capability 有効化 | 単一最新バージョン方針を確定し機構は実装しない — capability 有効化・旧版共存はその時点で `WIRE_VERSION` bump を伴うプロトコル変更として設計する | 独立配布されるホスト/ランナー間のバージョン互換が要求される段階の follow-up issue | 独立配布形態で新旧バージョンの混在が扱えない（現行は同一ワークスペース・同一リビジョン配布の前提でリスク非顕在） |
| `WIRE_MAX_*` の値変更・拡大 | 変更は送受信受理範囲・SHM 容量・既存拒否動作のレビュー前提 — 相互運用を壊す変更は bump 対象 | 上限変更の要求が発生した時点の個別 issue | 上限の silent 変更が新旧実装間の受理範囲をずらし未定義の相互運用を生む |

## Sources and Relations

- Issue [#119](https://github.com/scarecrow173/ruviform/issues/119) — 一次ソース（`relations.source`）。wire バージョニング方針の記録と拒否契約検証を要求
- [plugin-l1-out-of-process spec](plugin-l1-out-of-process.md)・[design](../designs/plugin-l1-out-of-process.md) — 方針の記録先（Issue #76 系・`derives-from`）。両文書の Deferred Design Concerns 行を本決定へ reconcile
- [issue-76-l1-core-wire-task-1-implementation.md](../impl/ir/issue-76-l1-core-wire-task-1-implementation.md) — wire プロトコル導入の実装記録（strict-equality・reserved 拒否・上限チェックの実装根拠）
- [issue-115-l1-gui-cross-process-parenting.md](../designs/issue-115-l1-gui-cross-process-parenting.md) — re-engagement 前提 (a) が本決定を要求（確定済み注記へ更新）
- 検証: [plugin-l1-wire-codec-tests.md](../test-specs/plugin-l1-wire-codec-tests.md)・[plugin-l1-proxy-contract-tests.md](../test-specs/plugin-l1-proxy-contract-tests.md)・[plugin-l1-runner-e2e-tests.md](../test-specs/plugin-l1-runner-e2e-tests.md)（`verified-by` — 3 本とも `verifies` に本 spec を追加）
- Refs: Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)・[#115](https://github.com/scarecrow173/ruviform/issues/115)
