---
id: "DESIGN-034YnAzVIIwmdeNr0QJCE0"
type: "design"
status: "approved"
title: "Issue #629 virtual endpoint lane registration design"
created: "2026-11-16"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-629-virtual-endpoint-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  defers: []
  deferred-by: []
---

# Issue #629 virtual endpoint lane registration design

## Context

Issue #629 5th slice。4th slice（`docs/designs/
issue-629-wasapi-endpoint.md`）で WASAPI streaming verifier
（`crates/ruviform-audio-device/tests/virtual_endpoint.rs`）と
`endpoint_probe` 拡張が merge 済みで、AC6 lane 設計は同 design
の「AC6 lane 設計記録」節に確定済み。本 slice はその lane 設計
を `mise.toml` に登録し、stale 参照を同期し、#628 Test Runtime
（`ruviform-audio-test`）との結線を headless 範囲で検証する。

## Scope

### In Scope

- `mise.toml` への `[tasks.test-audio-virtual-endpoint]` 追加
  — 記録済み設計どおり
  `cargo test -p ruviform-audio-device --features real-device
  --test virtual_endpoint -- --ignored --test-threads=1` を
  実行する opt-in manual task。description に provisioning
  必須・manual・never CI を明記。
- `docs/development-environment.md` の lane 一覧表・opt-in
  lane 注記への追記。
- stale 参照の同期: `virtual_endpoint.rs` module doc、
  `endpoint_probe.rs` verdict 文言、wasapi-endpoint design の
  AC6 関連記述、survey doc の lane 未登録記述。
- `EnvironmentKind` への additive variant 追加
  （`VirtualOsEndpoint`/`PhysicalDevice`）と serde
  round-trip/spelling テスト。
- #628 結線 IF の照合結果・verdict→RunMetadata 対応付け・
  既知語彙 gap の docs 記録。
- lane self-run の fail-closed evidence と docs chain。

### Out of Scope

- provisioned runner での実走 Pass・実 provisioning・
  `.github/**` CI/runner 統合・consumer 照合（AC7）。
- `ruviform-audio-device` → `ruviform-audio-test` dev-dep
  （Coordinator 不承認 — typed 結線は後続）。
- `RealAbiTestPlugin` の serde 綴り `real-abi-test-plugin`
  ↔ schema `test-plugin` の解消（破壊的 — 後続 slice の
  serde 移行判断へ）。
  status: rf#725 Phase A で consumer 影響評価（`evidence/rf725/consumer-impact-assessment.md`）と deserialize alias `test-plugin` 受容を適用済み —
  emit flip は第2 slice（`docs/designs/issue-725-serde-vocab-phase-a.md`）。
  status: rf#725 Phase B で emit flip を実施・検証済み —
  serde emit・`as_str()`・plugin-dock-test assert が全て
  `test-plugin` に統一され語彙 gap は解消（deserialize は
  新旧両綴りを恒久受容 —
  `docs/designs/issue-725-serde-vocab-phase-b.md`・
  `docs/impl/ir/issue-725-serde-vocab-phase-b.md`）。
- production 契約・verifier/probe の挙動変更・新規
  dependency/feature/`unsafe`・docs index 再生成。

## Components and Boundaries

- **`mise.toml` `[tasks.test-audio-virtual-endpoint]`** —
  lane の唯一の canonical 入口。`run` は 4th slice design の
  AC6 lane 設計記録どおりの cargo test 起動式。`real-device`
  feature + `#[ignore]` + `--test-threads=1` で
  `test-audio-real-input` と同型。配置は `lint-audio-real-
  input` 直後・`test-midi-real-device` 直前（audio-device
  系 opt-in lane のまとまりを維持）。
- **lint coverage** — 専用 lint lane は追加しない。新 test
  target `virtual_endpoint.rs` は `#![cfg(feature =
  "real-device")]` 配下のため、既存
  `lint-audio-real-input`（`cargo clippy -p
  ruviform-audio-device --all-targets --features real-device
  -- -D warnings`）が `--all-targets` で既に lint する。
  Coordinator 判断で確定。
- **`docs/development-environment.md`** — lane 一覧表に
  `test-audio-virtual-endpoint` 行を `lint-audio-real-input`
  の次に追加し、opt-in lane 注記節に説明 bullet を追加。
  `test-midi-real-device` は表に行を持たず「未収録の opt-in
  タスク」注記でカバーされる先例どおり、本 lane は表に載せる
  （`test-audio-real-input` と同じ audio-device 系の扱い）。
- **stale 参照の同期先** — `virtual_endpoint.rs` module doc
  （「no dedicated mise lane is registered in this slice」）、
  `endpoint_probe.rs` verdict の streaming proof 行（「no
  mise task is registered yet」）、wasapi-endpoint design の
  Deferred Design Concerns AC6 項目・Verification Notes AC6
  lane 設計記録・Non-Functional Constraints lane 項目、
  survey doc Evidence 節の「canonical=mise lane 未登録」記述。
  survey doc の Evidence 節は append-only 規約のため、既存
  entry は編集せず新規 dated entry で lane 登録と self-run
  を記録し、旧記述が本 slice で解消された旨を明記する。
- **`EnvironmentKind`（`crates/ruviform-audio-test/src/
  metadata.rs`）** — Case Schema v1 closed set 5 値に対し
  typed enum は 3 variant。`VirtualOsEndpoint`・
  `PhysicalDevice` を additive 追加し、`as_str`・serde
  `kebab-case`・doc コメントを schema 語彙に一致させる。
  既存 variant・serde 語彙・API は不変（AL/SV が dev-dep
  消費する公開 enum — additive のみ）。
- **#628 結線 IF の照合** — `PcmFixture`（`src/pcm.rs`）・
  `oracle::pcm_reference_block`（`src/oracle.rs`）・
  `RunMetadata` JSON save/load（`src/metadata.rs`）の
  merge 済み API を確認し、4th slice docs に記録した結線 IF
  との齟齬を docs に記録する。typed 実結線（dev-dep）は
  不承認のため行わない。

## Data and Control Flow

- lane 実行: `mise run test-audio-virtual-endpoint` →
  `cargo test -p ruviform-audio-device --features real-device
  --test virtual_endpoint -- --ignored --test-threads=1` →
  verifier が pair candidate を探索 → 未 provisioning では
  `verdict: NotRun — NOT_PROVISIONED` で exit 0（fail-closed）。
- evidence 採取: libtest は passing test の stdout を capture
  するため、verdict 行を evidence に採る場合は `--nocapture`
  が必要（survey doc の 4th slice 補足どおり）。lane の
  canonical `run` には `--nocapture` を含めない — self-run
  evidence は lane 実行ログそのものを保存する。
- `EnvironmentKind` serde: variant 追加は `#[serde(
  rename_all = "kebab-case")]` により `virtual-os-endpoint`・
  `physical-device` を自動生成。`as_str` の match に同 spelling
  を追加し、テストで serde↔as_str round-trip と schema 語彙の
  literal pin を検証する。

## Non-Functional Constraints

- **fail-closed 維持**: lane self-run が未 provisioning で
  `verdict: NotRun` を返すことを確認し、Pass を主張しない。
  lint/compile の Pass を `virtual-os-endpoint` の Pass や
  実 OS L3 実走 Pass と主張しない。
- **additive のみ**: `ruviform-audio-test` の既存 variant・
  serde 語彙・API・`Cargo.toml`/`Cargo.lock` は無変更。
  `mise.toml` の変更は task 追加のみで既存 task・他 OS 経路は
  不変。
- **3OS lane green**: `virtual_endpoint.rs` は
  `#![cfg(feature = "real-device")]` 配下、probe の Win32/
  PowerShell は `#[cfg(windows)]` module 内 — 非 Windows で
  compile 通過・`NotRun` のみ。
- **lint**: workspace clippy（pedantic + `-D warnings`）。
  `unsafe`・新規 dependency・feature flag の追加なし。
- **MSRV lane は実行しない**（daily CI 管轄 — task spec 指定）。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| lane 登録で既存 task を壊す | 3OS lane / opt-in lane の退行 | `mise.toml` は task ブロック追加のみ — 既存 task 定義に触れない。`prek run --all-files` の `check-toml` で TOML 検査 |
| `EnvironmentKind` variant 追加が consumer を壊す | AL/SV/plugin-dock-test の compile/証拠破壊 | additive variant のみ・既存 variant 不変。`#[non_exhaustive]` 既付で外部 match は wildcard 必須。check/lint/test を `ruviform-audio-test` と消費側に実施 |
| `real-abi-test-plugin`↔`test-plugin` の語彙差を見落とす | schema 語彙との不一致を「解消済み」と誤記 | Coordinator 回答 (A) に従い gap を enum doc・design・PR 本文に明記し、serde 移行は後続 slice へ（status: rf#725 Phase A で deserialize alias `test-plugin` 受容適用済み・emit flip は第2 slice — `docs/designs/issue-725-serde-vocab-phase-a.md`；rf#725 Phase B で emit flip 実施・検証済み — emit/`as_str()`/plugin-dock-test assert が `test-plugin` に統一・語彙 gap 解消 — `docs/designs/issue-725-serde-vocab-phase-b.md`） |
| self-run を Pass と誤記 | 未 provisioning の NotRun を実走 Pass と誤読 | evidence・docs・PR で `verdict: NotRun — NOT_PROVISIONED`・exit 0 をそのまま記録し、Pass 主張を明示的に否定 |
| probe 文言変更で evidence が古いまま | verdict 行と実装の不一致 | probe を Windows 実機で再実行し `endpoint_probe-windows-local.txt` を更新（best-effort・exit 0 維持の既存契約は不変） |

## Deferred Design Concerns

- provisioned runner での verifier 実走 Pass（AC1/AC2/AC5）・
  実 provisioning・driver install・license 判断 — 後続
  slice / repo 外工程。再開条件: 専用 runner での
  provisioning 確立。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 後続の
  外部 gate。POLICIES §5 の opt-in lane 規範に対応。
- Test Runtime 完全結線 — **解消済み（AC3 wiring slice）**:
  依存方向は逆方向 dev-dep（`ruviform-audio-test` → 両 device
  crate、test target のみ）を
  `docs/designs/issue-629-test-runtime-wiring.md` §1 で裁定・
  採用。device→audio-test dev-dep は不承認のまま。verdict→
  `RunMetadata` 対応付けは同 design §2・`tests/os_endpoint.rs`
  に記録・実装済み。provisioned 実走 Pass は外部 gate に残る。
- `EnvironmentKind::RealAbiTestPlugin` の serde 綴り
  `real-abi-test-plugin` ↔ schema `test-plugin` の gap —
  破壊的変更のため後続 slice で serde 移行方針（rename+alias
  等）とともに決定。再開条件: consumer 影響評価 + 承認。
  status: rf#725 Phase A で consumer 影響評価
  （`evidence/rf725/consumer-impact-assessment.md`）と
  deserialize alias `test-plugin` 受容を適用済み — emit flip
  は rf#631 占有解除後の第2 slice
  （`docs/designs/issue-725-serde-vocab-phase-a.md`）。
  status: rf#725 Phase B で emit flip を実施・検証済み —
  serde emit・`as_str()`・plugin-dock-test assert が全て
  `test-plugin` に統一され語彙 gap は解消（deserialize は
  新旧両綴りを恒久受容 —
  `docs/designs/issue-725-serde-vocab-phase-b.md`・
  `docs/impl/ir/issue-725-serde-vocab-phase-b.md`）。
- reconnect/xrun/permission 実測（AC5）— provisioned runner
  での実測が残る。capability matrix の確定版・provisioning
  runbook・consumer 照合（AC7 — `scarecrow173/synvelis#115`/
  `#117`/`#126`・`scarecrow173/audio-lens#127`）の headless
  記録は closeout slice
  （`docs/designs/issue-629-provisioning-runbook.md`）で解消
  済み — consumer 側の独立 Oracle での実受入判定は後続。
- AC6 の MIDI 側 lane 分離（`test-midi-virtual-endpoint`）は
  closeout slice で `mise.toml` に登録済み — 本 slice の
  audio 側 lane 登録と対になる残部であり、詳細は
  `docs/designs/issue-629-provisioning-runbook.md` を参照。

## Verification Notes

- [ ] `mise run fmt-check` / `mise run check
  ruviform-audio-device` / `mise run lint
  ruviform-audio-device` / `mise run test
  ruviform-audio-device` — Pass。
- [ ] `mise run check|lint|test ruviform-audio-test`（
  EnvironmentKind additive 変更の検証）— Pass。
- [ ] `mise run lint-audio-real-input`（`--all-targets
  --features real-device` の `-D warnings`）— Pass。
- [ ] `mise run test-audio-virtual-endpoint`（本 slice 登録
  lane の self-run）— 未 provisioning のため exit 0 +
  `verdict: NotRun` を `evidence/rf629/` に記録（Pass を
  主張しない）。
- [ ] `mise exec -- cargo run -p ruviform-audio-device
  --example endpoint_probe`（Windows 実機）exit 0 +
  `evidence/rf629/endpoint_probe-windows-local.txt` 更新。
- [ ] `mise exec -- prek run --all-files`（`check-toml` が
  `mise.toml` の TOML 検査を兼ねる）— Pass。
- [ ] MSRV lane — NotRun（daily CI 管轄、task spec 指定どおり
  未実行）。
