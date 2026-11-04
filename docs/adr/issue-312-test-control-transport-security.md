---
id: "ADR-034S312gtUiTestAdrCC33"
type: "adr"
status: "accepted"
title: "Issue #312 テスト制御ポートの分離クレート・transport・fail-closed 認証・capability 宣言"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/312"
    - "docs/discovery/issue-312-test-gui-host-lifecycle.md"
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-ui-test"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-312-test-gui-host-lifecycle.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/243"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #312 テスト制御ポートの分離クレート・transport・fail-closed 認証・capability 宣言

## Context and Problem Statement

Issue #312 は実 Window E2E 用のテスト専用 Lifecycle/Control ポートを要求するが、同時に (a) 本番 Host/API 変更の最小化、(b) 本番配布物・RT への非必須依存、(c) release artifact での制御面の到達不能、(d) 非認証・外部接続・不正 capability の fail-closed、(e) 任意 shell/file/eval 非公開、を課す。配置先・transport・認証方式・capability 表現の選択がこの安全性を構造的に決める。

## Decision Drivers

- Release artifact での到達不能は *flag audit* ではなく *構造的事実* である方が強い。
- `WindowEvent`/`WindowHandle` は既に backend-free の完全なポート — agent は既存契約だけで全必要機能（fixture arrange・close veto・crash 模擬・shutdown・観測 tee）を実現できる。
- Windows named pipe / Unix socket は std 未提供 or 片 OS のみ — 新外部依存を入れる強い理由がない限り std-only で完結させたい。
- Agent が UI スレッドのホストオブジェクトへ触れる必要があるため、スレッドを内部に隠蔽すると実 host 配線で破綻する。
- 偽装禁止: 制御可能な clock domain・capability は実装済み事実のみ宣言する。

## Considered Options

- **Option A**: 新クレート `ruviform-ui-test`（`ruviform-ui` 公開 API のみ依存）。制御面は dep を持つ build にのみ存在。既定 transport = 子プロセス stdio フレーム（OS 権限継承・ポート不要・3 OS で std のみ）、opt-in = loopback TCP（`127.0.0.1:0` ephemeral・run-bound secret・単一接続・loopback peer 限定・secret 未設定なら TCP serve 自体を拒否）。Agent は `serve(Read+Write)` の transport 非依存 dispatcher。capability は `Welcome` の型付き宣言 + op 実行時ゲート。`RVF_TEST_DISABLE` で実行時無効化。
- **Option B**: `ruviform-ui` 内の feature-gated モジュール。feature unification で消費者経由の有効化が本番へ混入し得るため却下（到達不能性が構造的事実でなくなる）。
- **Option C**: `interprocess` 等の外部クレートで named pipe/UDS 統一 transport。クロスプラットフォームの named pipe が得られるが、stdio が既に OS 権限継承 pipe として要件を満たし、新依存の保守・lock 更新コストに見合わないため却下（unix socket は後続で additive に追加可能）。
- **Option D**: 認証に handshake 後の per-message MAC / OS credential check。loopback test channel には過剰（run-bound secret + 接続元限定で threat model を満たす）。本番認可を test flag で迂回しない方針とも整合。

## Decision Outcome

Chosen option: **Option A**.

- 新クレート `ruviform-ui-test`（利用側層・`ruviform-ui` のみ依存・publish=false 継承）。本番クレートのソース・依存は不変。
- `rvf.uitest.v1`: `ruviform_ui::ipc` envelope を再利用した `test.*` op 集合。閉じた enum — 任意 eval/shell/file API は wire 上表現不能。
- 既定 transport = stdio フレーム（u32le length + JSON・`MAX_INTENT_BYTES` 上限）。opt-in TCP = driver が `127.0.0.1:0` listen・子が `RVF_TEST_ADDR`/`RVF_TEST_SECRET` で dial・`Hello.secret` 照合・単一接続・非 loopback peer 拒否・`RVF_TEST_SECRET` 未設定時は TCP serve を拒否。
- `RVF_TEST_DISABLE=1` は全 serve 経路を即時 `Disabled` で終了（実行時無効化）。
- Capability は `Welcome.capabilities` で型付き宣言し、op 実行時に再検査（`unavailable` fail-closed）。`input_injection`/`dom_access` は常に `false`。
- Clock 制御範囲は `ClockDomain::HostTick` のみと宣言。OS 時刻/GPU/webview 内 timer は制御しないことを wire 契約で明記。

### Consequences

- Good: 到達不能性が依存グラフの事実になる;headless FakeHost で全契約が `cargo test` 可能;`serve(Read+Write)` で stdio/TCP/将来の UDS が同一経路;`run_multi` 実配線（#313）に追加 API が要らない。
- Bad: 制御コマンドは host のイベント turn（tick 等）で dispatch されるモデル — real host 配線では tick 設定が実務上必要（#313 で明示）。stdio を制御に使うため app は stdout を占有する（log は stderr へ — dummy_app が実例）。
- Neutral: secret 生成は `std::hash::RandomState`+時刻/pid の run-bound token — loopback test 用途の脅威モデルに十分であり、本番認可機構ではないことを明記。
