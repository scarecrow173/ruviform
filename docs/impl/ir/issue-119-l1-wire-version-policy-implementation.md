---
id: "IMPL-034NhQ4TXcRIJZJtbtzDPv"
type: "impl"
status: "completed"
title: "Issue #119 L1 wire version policy — implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/119"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-119-l1-wire-version-policy-implementation.md"
      - type: "file"
        path: "docs/specs/issue-119-l1-wire-version-policy.md"
      - type: "file"
        path: "docs/plans/implement-issue-119-l1-wire-version-policy.md"
      - type: "file"
        path: "docs/tasks/l1-wire-version-policy.md"
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
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/l1-wire-version-policy.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-119-l1-wire-version-policy.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/impl/ir/issue-76-l1-core-wire-task-1-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-119-l1-wire-version-policy.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/115"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #119 L1 wire version policy — implementation

## Summary

Issue #119 は新しいプロトコル機構の実装ではなく、Issue #76 で導入済みの wire バージョン厳密一致・reserved 拒否・`WIRE_MAX_*` 上限の挙動を束ねる**単一最新バージョン方針**を設計判断として記録し、記録した契約に対するカバレッジ欠落のみをテストで補強する Issue だった。方針本文を `docs/designs/plugin-l1-out-of-process.md` の「ワイヤバージョン方針」決定ブロック（正本）・`docs/specs/plugin-l1-out-of-process.md` の Requirements 条項・`crates/ruviform-plugin-core/src/wire.rs` の `//!` Versioning 節の 3 箇所へ記録し、両親文書の Deferred Design Concerns 行を確定判断を指す記述へ reconcile した。契約検証では既存テストで全拒否契約が固定済みであることを確認し、唯一の欠落だった runner 側 shm header `wire_version` 不一致拒否に `e2e_shm_header_wire_version_mismatch` を追加した。**製品コードの挙動変更はゼロ** — バージョン交渉・互換レイヤー・capability 割当・上限交渉はいずれも実装していない。

## Context

- Upstream: spec `docs/specs/issue-119-l1-wire-version-policy.md`（Issue #119・`status: implemented`）→ plan `docs/plans/implement-issue-119-l1-wire-version-policy.md` → task `docs/tasks/l1-wire-version-policy.md`（単一タスク）。
- 経緯: `wire.rs` は strict-equality の version 照合・reserved/`capabilities` 非ゼロの `Malformed` 拒否・`WIRE_MAX_*` の `LimitExceeded` 拒否を実装として持っていたが、「なぜ単一バージョンのみか」「いつ bump するか」「交渉を入れないのはなぜか」が設計判断として未記録だった。design の Deferred Design Concerns 行は「初版 `WIRE_VERSION=1` で十分」とだけ述べていた。
- 消費者: `docs/designs/issue-115-l1-gui-cross-process-parenting.md` の re-engagement 前提 (a) が「wire versioning 枠組みの確定」を要求 — 本 Issue の確定により該当前提へ解決注記を追記した。
- 検証の正本: 3 本の L1 test-spec（wire-codec / proxy-contract / runner-e2e）が契約↔テスト対応を記述しており、本 Issue で `verifies` に spec を追加し deferred/non-goal 記述を確定方針へ整合させた。

## Implementation

### Changes

- `docs/designs/plugin-l1-out-of-process.md` — wire/codec 設計節に「ワイヤバージョン方針（Issue #119 で確定 — 初期開発期の単一バージョンポリシー）」決定ブロックを追加。条項: 単一最新バージョンのみ（旧版 decode/変換/フォールバック/共存なし）・`WIRE_VERSION` は通信契約世代の単一整数（SemVer/Cargo 非連動）・`Hello.version` 厳密一致（不一致は kill + `HostError::Ipc`）・shm header `wire_version` 同一契約・交渉機構なし・非互換変更のみ bump + ホスト/ランナー同一版更新・`Hello.capabilities` 予約維持・`WIRE_MAX_*` 固定上限（上限交渉なし）・再検討条件 = 独立配布の互換要求。Deferred Design Concerns 行を「wire バージョニング/進化 | 初版 `WIRE_VERSION=1` で十分」から「複数 wire バージョン共存・交渉機構 | Issue #119 で単一最新バージョン方針を確定」へ書き換え、reopen 条件を「独立配布されるホスト/ランナー間のバージョン互換が要求される段階」として明示。`updated`/`derived-by` 更新。
- `docs/specs/plugin-l1-out-of-process.md` — Requirements に「ワイヤバージョン方針（Issue #119 で確定）」条項を追加・Deferred Design Concerns 行を同内容で reconcile・`updated`/`derived-by` 更新。
- `crates/ruviform-plugin-core/src/wire.rs` — `//!` Versioning 節へ方針要約を追記（SHM `wire_version` の厳密一致・単一最新バージョンのみ・旧版 decode/変換/フォールバック/共存なし・バージョン/上限交渉なし・`WIRE_VERSION` は通信契約世代で SemVer 非連動・非互換変更時にホスト/ランナー同一版で bump・capabilities 予約維持）し、完全な方針の正本として L1 design を参照する一文を追加。**コード変更なし（doc comment のみ）**。
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` — `create_segment` を `create_segment_with(key, magic, wire_version)` へ一般化（既存呼出しは canonical 値を明示渡し・`e2e_shm_header_mismatch` の bad-magic 経路も canonical version を渡す形へ更新）し、新規テスト `e2e_shm_header_wire_version_mismatch` を追加。`wire_version = u32::from(wire::WIRE_VERSION) + 1` の segment を作成して runner を spawn、`Hello` を消費した後に runner が非ゼロ終了することを固定する（製品コードへ version override 機構は追加しない — header 破損 fixture のみ）。
- `docs/designs/issue-115-l1-gui-cross-process-parenting.md` — re-engagement 前提 (a) を「Issue #119 で確定済み」へ更新し、GUI 動詞追加は bump を要する非互換変更・capability 有効化はその時点のプロトコル変更として位置付け。`related` に spec 追加。
- `docs/test-specs/plugin-l1-wire-codec-tests.md` — `verifies` へ spec 追加・non-goal の「wire バージョニング/進化（初版で十分 — design deferred）」を「複数バージョン共存・交渉機構の検証（方針は Issue #119 で確定）」へ更新。
- `docs/test-specs/plugin-l1-proxy-contract-tests.md` — `verifies` へ spec 追加（本文の `Hello.version != 1` 失敗 example は既存どおり維持）。
- `docs/test-specs/plugin-l1-runner-e2e-tests.md` — `verifies` へ spec 追加・「shm header 不一致」example へ `wire_version` を明記（`Hello.version` と同一の厳密一致契約である旨を注記）・non-goal の「wire バージョン進化」を確定方針参照へ更新。
- `docs/specs/README.md`・`docs/plans/README.md`・`docs/tasks/README.md` — scaffold スクリプトによる index 行追加。`docs/impl/ir/README.md` — 手動で行追加（generated marker なしの hand-curated index）。

### 契約検証の棚卸し（既存テストとの対応）

| 記録した契約 | 固定テスト | 状態 |
| --- | --- | --- |
| `Hello.version` 厳密一致・不一致は handshake 失敗 | `handshake_version_mismatch`（proxy E2E）・`e2e_version_mismatch_is_a_parent_side_handshake_failure`（runner E2E） | 既存 |
| `Hello` の version 保持 + capabilities/reserved 非ゼロ `Malformed` | `hello_roundtrip_and_version_is_carried_verbatim`・`decode_rejects_nonzero_reserved_and_bad_utf8`（wire.rs） | 既存 |
| `WIRE_MAX_*` declared-length/count 拒否（`LimitExceeded`/early reject） | `decode_rejects_declared_lengths_over_wire_max`・`decode_rejects_impossible_counts_without_allocating`（wire.rs） | 既存 |
| shm header レイアウト・magic/形状検証 | `shm_layout_offsets_are_self_consistent`（wire.rs）・`shm_header_roundtrip_matches_wire_layout`（proxy E2E）・`e2e_shm_header_mismatch`（runner E2E） | 既存 |
| shm header `wire_version` 厳密一致（runner attach 時） | **`e2e_shm_header_wire_version_mismatch`（runner E2E・本 Issue で追加）** | 追加 |

## Related Experiments

### Adopted

- なし — 本 Issue は新規実験を伴わない記録 + 契約検証作業（実験ログへの参照なし）。

### Rejected

- なし — 採用しなかった実装手段は「Decisions During Implementation」に記録（実験ログ文書は作成していない）。

## Validation

| Gate | 結果 |
| --- | --- |
| `cargo test -p ruviform-plugin-export --test l1_runner_e2e` | exit 0 — 12 tests 全 pass（`e2e_shm_header_wire_version_mismatch` 含む・対象変更の直接検証） |
| `mise run fmt-check` | exit 0 |
| `mise run check` | exit 0（workspace --all-targets） |
| `mise run lint` | exit 0（workspace --all-targets・`-D warnings`） |
| `mise run test` | exit 0 — 全 test binary pass（`0 failed`・新規テスト含む） |
| `mise run test-rt` | exit 0 — 全 `rt_` テスト pass（本変更は RT 経路に非接触・件数変化なし） |
| `node .agents/skills/doc-status/scripts/doc_status.js audit --json` | `blocking: 0`・`findings: []` |
| `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | 本 IR は findings ゼロ（`decide-plugin-binding-method-adr.md`・`implement-plugin-adapters.md`・`sync-docs-and-verify-plugin-foundation.md` の missing-section errors と `poc-clap-binding-adapter.md` の missing-experiment-link warning は pre-existing の legacy findings で本 Issue の対象外） |

## Risks

- **3 箇所の記録の drift** — design（正本）・spec（要求条項）・`wire.rs` `//!`（要約 + design 参照）に分けたが、将来の方針変更時に 3 箇所の同期が必要。文言一致ではなく役割分担で管理する旨を module doc に明記した。
- **「実装しない」旨の誤読** — 方針記述が交渉機構の実装要求と誤読されないよう、spec の Out of Scope と design 決定ブロックで「実装しないことの確定」を明示した。
- **fixture 破損方式の限界** — `create_segment_with` は header の `wire_version` を任意値で書けるため、将来のテストが誤って非 canonical 値を使う余地がある。既存呼出しは全て `u32::from(wire::WIRE_VERSION)` を明示渡しする形に揃え、破損はテスト内で意図的なものだけに限定した。

## Follow-ups

- **複数バージョン共存・交渉機構** — 単一最新バージョン方針を確定したため継続 deferred。reopen 条件: 独立配布されるホスト/ランナー間のバージョン互換が要求される段階（両親文書の Deferred Design Concerns に記録済み）。
- **`WIRE_MAX_*` の値変更** — 変更要求が発生した場合は送受信受理範囲・SHM 容量・既存拒否動作のレビュー前提で個別 issue として扱い、相互運用を壊す変更は `WIRE_VERSION` bump とセットにする（方針本文に記録済み）。
- **capability 有効化** — `Hello.capabilities` の bit 割当はそれ自体がプロトコル変更としてその時点で設計する（Issue #115 の GUI 動詞追加が最初の候補 — bump とセット）。

## Decisions During Implementation

- **記録の役割分担** — design を方針の正本（判断根拠込み）・spec を要求条項（検証可能な形）・`wire.rs` `//!` を要約 + design 参照とした。3 箇所へ全文複写すると drift するため、module doc は「完全な方針は L1 design を参照」の一文で正本へ誘導する形にした。
- **追加テストは runner shm `wire_version` 不一致のみ** — 棚卸しの結果、Hello version・reserved・上限・shm magic/形状は既存テストで固定済みだった。runner shm attach の `wire_version` 検証（`crates/ruviform-plugin-export/src/shm.rs`）は実装済みだが専用テストがなかったため、唯一の欠落として `e2e_shm_header_wire_version_mismatch` で固定した。
- **製品コードへの検証用機構は追加しない** — runner 側に version override を入れず、test 側が segment header を `WIRE_VERSION + 1` で書く fixture 破損方式を採用。`create_segment` → `create_segment_with` の一般化で magic/version のどちらも破損可能にし、既存 `e2e_shm_header_mismatch` は canonical version を明示渡しに更新した。
- **issue-115 design は注記更新のみ** — 前提 (a) を「確定済み」へ更新したが、deferred status 自体は変更しない（残る前提 (b)(c)(d) は未解決のため）。

## Sources and Relations

- Issue [#119](https://github.com/scarecrow173/ruviform/issues/119): L1 wire バージョニング方針の記録と拒否契約検証 — 一次ソース。
- Spec [issue-119-l1-wire-version-policy.md](../specs/issue-119-l1-wire-version-policy.md)・Plan [implement-issue-119-l1-wire-version-policy.md](../plans/implement-issue-119-l1-wire-version-policy.md)・Task [l1-wire-version-policy.md](../tasks/l1-wire-version-policy.md)。
- 方針記録先: [plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（design・「ワイヤバージョン方針」決定ブロック）・[plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)（spec・Requirements 条項）・`crates/ruviform-plugin-core/src/wire.rs` `//!` Versioning 節。
- 実装根拠: [issue-76-l1-core-wire-task-1-implementation.md](issue-76-l1-core-wire-task-1-implementation.md)（wire プロトコル導入 — strict-equality・`Malformed`/`LimitExceeded` 拒否の実装記録）。
- 消費者: [issue-115-l1-gui-cross-process-parenting.md](../designs/issue-115-l1-gui-cross-process-parenting.md)（前提 (a) を確定済みへ更新）。
- 検証: [plugin-l1-wire-codec-tests.md](../test-specs/plugin-l1-wire-codec-tests.md)・[plugin-l1-proxy-contract-tests.md](../test-specs/plugin-l1-proxy-contract-tests.md)・[plugin-l1-runner-e2e-tests.md](../test-specs/plugin-l1-runner-e2e-tests.md)（`verified-by` — 3 本とも `verifies` に spec を追加）。
- Refs: Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)・[#115](https://github.com/scarecrow173/ruviform/issues/115)。

## Review Handoff

- 焦点: 方針本文が design/spec/`wire.rs` の 3 箇所で矛盾なく記録されているか・deferred 行の reconcile が reopen 条件（独立配布互換要求）を残しているか・`e2e_shm_header_wire_version_mismatch` が製品コードへ検証用機構を追加せず fixture 破損のみで検証しているか・`create_segment` 呼出しが全て canonical version を明示渡しになっているか・diff に交渉機構/互換レイヤー/capability 割当/上限交渉が存在しないか。**GitHub issue close・merge 判定は本 IR の scope 外 — push/PR は行っていない。**
