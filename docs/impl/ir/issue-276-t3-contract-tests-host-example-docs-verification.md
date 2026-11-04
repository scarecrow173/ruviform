---
id: "IMPL-034RwLuQ6IZyb2JEQ9I1Sz"
type: "impl"
status: "completed"
title: "Issue 276 T3: contract tests + host example + docs + verification"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/276"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/tests/logging.rs"
      - type: "file"
        path: "crates/ruviform-observability/examples/logging_host.rs"
      - type: "file"
        path: "docs/impl/ir/issue-276-t1-logging-config-api-types-feature-wiring.md"
      - type: "file"
        path: "docs/impl/ir/issue-276-t2-bounded-writer-sinks-build-result.md"
      - type: "file"
        path: "docs/impl/ir/issue-276-t3-contract-tests-host-example-docs-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/test-specs/issue-276-generic-logging-adapter.md"
      - type: "file"
        path: "docs/plans/issue-276-generic-logging-adapter.md"
      - type: "file"
        path: "docs/tasks/task-276-t1-config-and-wiring.md"
      - type: "file"
        path: "docs/tasks/task-276-t2-writer-sinks-build.md"
      - type: "file"
        path: "docs/tasks/task-276-t3-tests-docs-verify.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/task-276-t3-tests-docs-verify.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/task-276-t2-writer-sinks-build.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/test-specs/issue-276-generic-logging-adapter.md"
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

# Issue 276 T3: contract tests + host example + docs + verification

## Summary

test-spec E1–E6・E11–E12 を pin する `tests/logging.rs` 契約試験(19 件)、
Host 所有権の一通りを示す `examples/logging_host.rs`、README の `logging`
feature 節を追加し、feature lane / MSRV lane / workspace lane の全検証を
実施した。`origin/main` は 0 commit behind で merge 不要を確認済み。

## Context

- Task: `docs/tasks/task-276-t3-tests-docs-verify.md`(前提 T1・T2)
- Test-spec: `docs/test-specs/issue-276-generic-logging-adapter.md`
- Plan: `docs/plans/issue-276-generic-logging-adapter.md`

## Implementation

### Changes

- `tests/logging.rs`(19 件): dual sink filter routing・target 別 filter・
  JSONL の level/target/fields・Text の level/target/message・retention
  (seeded 5 件 → max_files=3 で古い順削除)・Required bad-dir →
  `RequiredSinkFailed`・BestEffort → `degraded`+残 sink 継続・全滅 →
  `AllSinksFailed`・空 sinks → `NoSinks`・不正 filter → `InvalidFilter`・
  guard drop で drain+join・global 未設定確認・stderr sink 経路・
  shutdown 後 emit → `channel_disconnected`・snapshot 非再帰・
  `build_logging_layers` → host registry 合成・multi-thread emit・
  片 sink shutdown で他 sink 継続。
- `examples/logging_host.rs`: Host config 写像 → `build_logging` → scoped
  `with_default` で emit → health snapshot → guard shutdown。
  stderr Text + daily JSONL file(7 世代)の AudioLens 想定構成。
- `README.md`: `logging` feature 節(依存理由・所有権契約・sink 構成・
  失敗モデル・writer/ライフサイクル・RT 契約・センシティブデータ・sample)。
- task 3 件を `done`、plan の検証チェックを実施済みに更新、test-spec を
  `approved` に格上げ、`docs/impl/ir/README.md` に本行を追加。

### Decisions During Implementation

- 試験の決定性のため file sink は主に `RotationPeriod::Never`
  (file 名が `{prefix}` 固定)を使い、retention は seeded files で
  構築時 prune を検証。実 rotation の時間経過は flaky のため対象外
  (test-spec Non-goals と一致)。
- global subscriber 未設定の検証は「with_default 無しで emit しても file に
  書かれない」で代替(試験プロセス内で global を一度も設定しない)。
- stdout 非出力は構造的保証(`SinkDestination` に stdout 経路が存在しない)
  + example を `2>/dev/null` で実行し stdout が 0 byte であることを確認。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise run fmt-check` — pass。
- `mise run check`(`cargo check --workspace --all-targets`)— pass。
- `mise run lint`(`cargo clippy --workspace --all-targets -- -D warnings`)— pass。
- `mise run test`(`cargo test --workspace`)— pass。
- `cargo check -p ruviform-observability --no-default-features` — pass
  (core 無依存)。
- `cargo tree -p ruviform-observability --no-default-features --edges normal` —
  normal dep ゼロ。
- `cargo check -p ruviform-observability --features logging --all-targets` —
  pass。
- `cargo clippy -p ruviform-observability --features logging --all-targets
  -- -D warnings` — pass。
- `cargo test -p ruviform-observability --features logging` — 13 unit + 19
  contract + 3 doctest 全 pass。
- MSRV: `mise exec rust@1.89.0 -- cargo check -p ruviform-observability
  --features logging --all-targets` — pass。
- MSRV clippy: `mise exec rust@1.89.0 -- cargo clippy -p ruviform-observability
  --all-targets --features logging -- -D warnings -A clippy::float_cmp
  -A clippy::similar_names` — pass。
- `cargo run -p ruviform-observability --features logging --example
  logging_host` — 動作確認済み(stderr Text・daily JSONL `example.YYYY-MM-DD`・
  health snapshot・clean shutdown)。`2>/dev/null` で stdout 0 byte 確認。
- `git fetch origin` 済み・`HEAD..origin/main` = 0 commit(merge 不要)。
- 未実施: 実時間経過による rotation 切替、readonly-dir/disk-full の実 OS
  検証(代替試験で dir-is-file 失敗を検証)、Linux/macOS での検証
  (本機 Windows のみ — test-spec Non-goals どおり)。

## Risks

- 実 rotation の日跨ぎ・readonly-dir/disk-full・非 Windows OS の挙動は
  未検証。CI の 3-OS matrix と AudioLens 導入 Issue(#138)側で確認する。
- `tracing-appender` の rotation 時ファイル作成失敗は upstream が eprintln
  して旧ファイル継続 — counter に到達しない経路として契約明記済み。

## Follow-ups

- OBS-11(#293)の Host 統合(AudioLens AL_LOG/flag/GUI 優先順位の config
  写像・Synvelis GUI/headless subscriber 所有方針)は別 Issue。
- OBS-06(#281)の cross-consumer 互換・性能ゲート証跡は導入側実装後に実施。
