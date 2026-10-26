---
id: "TSPEC-034MLRCbJ8DILg5vtGVsQB"
type: "test-spec"
status: "proposed"
title: "実プラグイン L1 opt-in lane（mise task・`#[ignore]` ゲート・fixture 欠落 fail・CI `l1-real-verify` job）テスト仕様"
created: "2026-10-26"
updated: "2026-10-27"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
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
    - "docs/test-specs/adapter-host-bridge-contract-tests.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 実プラグイン L1 opt-in lane（mise task・`#[ignore]` ゲート・fixture 欠落 fail・CI `l1-real-verify` job）テスト仕様

> status: proposed — Refs #114。`docs/specs/issue-114-real-adapter-l1-e2e.md`（AC6・AC8 の lane 側）を verify。対象: `mise.toml` の `test-l1-real-plugin`/`lint-l1-real-plugin` task・`.github/workflows/ci.yml` 内の `l1-real-verify` job・`#[ignore]`/`#![cfg]`/`required-features` のゲート機構 — **lane 自身の配線が正しく、default 経路を汚さず、fixture 欠落を silent skip せず、opt-in で実検証できる**ことを pin。

## Purpose

実規格 plugin E2E は fixture cdylib + feature `host` runner を要求するため、default `cargo test`/`mise run test` には載せられない — だが「lane を追加して誰も実行しない」は検証の偽装になる。本 spec は lane 機構自体（mise task 定義・`#[ignore]` ゲート・fixture 欠落の fail 設計・CI `l1-real-verify` job）が、**opt-in で実際に実行可能・欠落時は確実に fail・default 経路を一切汚さない**ことを pin する。目的は (a) lane が実機能する（build → `--lib` unit test → `--ignored` E2E の 3 段構成で fixture+runner が揃った状態で bridge 契約層と E2E が走る）、(b) 欠落条件（fixture/bin/feature 不在）が全て fail で収束し silent skip を残さない、(c) `cargo test --workspace`・`mise run test` 等 default 経路が本変更で不変グリーンのままであることを保証する、(d) workflow_dispatch 専用の既存 ci.yml 内に `l1-real-verify` job が正しく配線される（別 workflow ファイルを作らない）。

## Feature

実規格 plugin L1 E2E の opt-in 実行 lane — `test-l1-real-plugin`/`lint-l1-real-plugin` mise task・`ci.yml` `l1-real-verify` job・`#[ignore]`/`#![cfg]`/`required-features` ゲート・fixture 欠落 fail 設計。

## Rules

- **`test-l1-real-plugin` task（mise.toml）**: run 配列は (1) `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host`（runner bin + fixture cdylib を同一 build で生成 — `-p` で adapter 2 crate に限定・workspace 全量ではない）→ (2) `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --lib`（`#[cfg(all(test, feature = "host"))]` の in-process bridge unit test 層 — `--test`/`--ignored` 指定では実行されないため明示 step・これを省くと lane/CI で bridge 契約層が未実行になる）→ (3) `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --test l1_real_adapter_e2e -- --ignored`（`#[ignore]` 全テストを `--ignored` で実行）。task 名は `test-l1-real-plugin`・description に fixture+feature 要件を明記
- **`lint-l1-real-plugin` task（mise.toml）**: `cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets --features host -- -D warnings` — host feature の clippy ゲートを default `lint` から独立して実行可能にする（`lint` は `--workspace` 既定で feature `host` を含まない — host 側の scoped `#[expect(unsafe_code)]` + SAFETY・COM vtable・dlopen 箇所を `-D warnings` で別 lane 検証）
- **`#[ignore]` ゲートの定義**: `tests/l1_real_adapter_e2e.rs` の全 `#[test]` に `#[ignore = "<理由: fixture cdylib + runner bin + feature host を要求 — mise run test-l1-real-plugin で実行>"]` を付与 — 既定 `cargo test`/`cargo test --features host`（`--ignored` なし）では実行されない。`#![cfg(feature = "host")]` はファイル先頭に — feature off で test ファイルが空になり `#[ignore]` も compile 対象外
- **fixture 欠落 = fail（silent skip 禁止）**: テスト内 `assert!(fixture_path.is_file(), "...")`・runner path の存在確認・`spec.program` の存在確認は全て `assert!`/`expect` — 環境変数ゲート・`return`・`eprintln!` で抜ける skip は**一切禁止**。`RUVIFORM_L1_REAL_PLUGIN_DIR` は runner 側の shell-only fixture override で、テスト側の skip 条件には**ならない**
- **default 経路不変**: `cargo test --workspace`・`mise run test`・`mise run check`・`mise run lint`・`test-rt`・`verify-ci` は本変更で**一切差分なし** — `#[ignore]` は `cargo test` 既定で実行されない・`#![cfg(feature = "host")]` で feature off は空・`required-features = ["host"]` の runner bin は default build で生成されない・新規 optional dep（`clack-host`/`libloading`/`ruviform-plugin-export`）は `host` 未指定で dep 木に出現しない（`cargo tree` で検証）
- **CI `l1-real-verify` job（ci.yml 内・別ファイルなし）**: 既存 workflow_dispatch workflow 内に job 追加 — `verify`/`rt-verify`/`icc-verify` と同型 steps（actions/checkout → jdx/mise-action → mise install → cargo cache → 当該コマンド）・3-OS matrix（`ubuntu-latest`/`windows-latest`/`macos-latest`・`fail-fast: false` で全 OS 結果収集）・run コマンドは `mise run test-l1-real-plugin` + `mise run lint-l1-real-plugin`（または該当 cargo コマンド直接 — mise task 呼出しで local/CI 同一入口を維持 — `test-l1-real-plugin` が build→`--lib` unit test→`--ignored` E2E の 3 段を含むため CI でも bridge unit test 層が実行される）・**別 workflow ファイル・新規 trigger は作らない**（workflow_dispatch 専用のまま）
- **lane 実行の順序**: `cargo build` で runner bin + fixture cdylib の両生成を**保証**してから `cargo test --lib`（unit test 層）→ `cargo test --test l1_real_adapter_e2e -- --ignored` — same-crate bin は `--features host` を満たす `cargo test` でも自動 build される（`CARGO_BIN_EXE_*` が有効になる）が、**明示の `cargo build` step が artifact 生成を deterministic に pin する** lane 契約とする（cargo test の target 選択・incremental 状態に fixture/cdylib の存在を依存させない）
- **欠落時の defined 挙動**: `RUVIFORM_L1_REAL_PLUGIN_DIR` を指定しなくても fixture は `current_exe` sibling で解決される（shell/lane 実行時）・runner が fixture を見つけられない場合は `eprintln!` + `exit(1)` pre-Hello → 親 spawn `Err(HostError::Ipc)` — これ自体が E2E の「binary 不在」テストケースとして pin 済み（[real-adapter-l1-e2e-tests](real-adapter-l1-e2e-tests.md) Rules の `Err(Ipc)` spawn 拒否）

## Examples

- Given `mise.toml` に task 定義済み、When `mise run test-l1-real-plugin`、Then (1) `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` が runner bin（`required-features` のため default build では出ない）+ fixture cdylib を生成 → (2) `cargo test ... --lib` で in-process bridge unit test 層が実行され → (3) `cargo test ... --test l1_real_adapter_e2e -- --ignored` で全 `#[ignore]` テストが実行され pass — `#[ignore]` なしの unit test は `--ignored` で除外される（別層 — そのため明示 `--lib` step が必要）
- Given `cargo test --workspace`（feature `host` 未指定）、When default gate、Then `l1_real_adapter_e2e.rs` は `#![cfg]` で空・runner bin は `required-features` で非生成・`clack-host`/`libloading`/`ruviform-plugin-export` は optional dep として dep 木に出現しない — `cargo tree -p ruviform-plugin-clap -p ruviform-plugin-vst3` で `host` off 時に新規 dep が non-optional 木に出ないことを目視/`cargo tree` 出力で検証
- Given fixture cdylib 未生成（`target/<profile>/` も `deps/` も空）、When lane 実行（または runner 直接起動 `RUVIFORM_L1_REAL_PLUGIN_DIR=<empty>`）、Then テスト内 `assert!` が fail するか runner が `eprintln!` + `exit(1)` pre-Hello で spawn `Err(Ipc)` — **`return`/`eprintln!` のみで抜ける skip 経路は存在しない**
- Given ci.yml に `l1-real-verify` job、When GitHub Actions の手動 `workflow_dispatch`（`l1-real-verify` job を選択/実行）、Then 3-OS matrix（`fail-fast: false`）で `mise run test-l1-real-plugin` + `mise run lint-l1-real-plugin` が全 OS で走り、fixture 未生成や runner 欠落の環境でも fail で検出される — `verify`/`icc-verify` と同型 steps（checkout→mise→install→cache→run）で再現性を保つ
- Given `cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets --features host -- -D warnings`、When `mise run lint-l1-real-plugin`、Then host feature の全 target（`host.rs`・runner bin・test ファイル）が `-D warnings` で 0 warning — scoped `#[expect(unsafe_code)]` + SAFETY 箇所のみが unsafe を含み、警告なしで exit 0

## Guarantees

- lane は **opt-in で実機能する**: `mise run test-l1-real-plugin` の 3 段構成（build → `--lib` unit test → `--ignored` E2E）で fixture+runner が揃い、bridge unit test 層と `#[ignore]` E2E が実際に走る — 「task はあるが実行しない」偽装を排除
- **欠落は全て fail**: fixture/bin/feature 不在は `assert!`/spawn `Err`/`exit(1)` で確実に検出 — env-gated silent skip・`return` 早期終了・`eprintln!` のみの抜けは**存在しない**
- **default 経路は不変**: `cargo test --workspace`/`mise run test`/`check`/`lint`/`test-rt`/`verify-ci`/`verify` job は本変更で一切差分なし — `#[ignore]`+`#![cfg]`+`required-features`+optional dep の 4 層で feature `host` を default から完全分離
- **CI は workflow_dispatch 専用・別ファイルなし**: `l1-real-verify` job は既存 ci.yml 内の additive job — 新規 workflow ファイル・push/PR trigger は作らない・3-OS matrix で platform 差を collect
- **同一入口**: local `mise run test-l1-real-plugin` と CI job が同一コマンド列で実行される — 「local で通ったが CI で違う」再現の断絶を防ぐ

## Non-goals

- E2E テスト内容自体（動詞経路・データ正当性・異常系）は [real-adapter-l1-e2e-tests](real-adapter-l1-e2e-tests.md) が担う — 本 spec は lane 機構の配線・ゲート・欠落挙動のみ
- bridge 写像の検証は [adapter-host-bridge-contract-tests](adapter-host-bridge-contract-tests.md) が担う
- `verify`/`rt-verify`/`icc-verify` 等既存 job の内容・`verify-ci`/`check-ci` 等既存 mise task の内容は変更しない — additive のみ
- `cargo test --workspace` への本 lane の組込みは対象外（`#[ignore]` opt-in が設計意図 — default 経路に載せると fixture 必須で全開発者に強制になる）
- OFX lane（`test-ofx`・`lint-ofx`・`ofx-verify`）の変更・拡張は対象外 — 本 lane は `test-l1-real-plugin`/`lint-l1-real-plugin`/`l1-real-verify` のみ
- `cargo test` 既定経路で `#[ignore]` を実行するための `--include-ignored` や `ignored` フラグの設定変更は対象外

## Risk

- `#[ignore]` opt-in lane は実行されなければ腐る — `l1-real-verify` job（workflow_dispatch）の手動 dispatch と runbook の `mise run test-l1-real-plugin` 明記で緩和（opt-in の性質上不可避 — spec AC6・design 決定 h 整合）
- lane を `cargo test --test l1_real_adapter_e2e -- --ignored` 単独にまとめると `--lib` unit test 層（in-process dlopen 契約）が lane に載らず、runner/cdylib の生成が `cargo test` の target 選択に依存する — explicit `cargo build` + `--lib` + `--ignored` の 3 段構成を pin
- `RUVIFORM_L1_REAL_PLUGIN_DIR` を skip 条件として誤用すると「fixture があれば実行・なければ pass」という silent skip が混入する — テスト側では assert のみ・skip 条件にしないを Rules で固定
- `cargo test --workspace` 既定経路に `#[ignore]` テストを誤って載せると（`--include-ignored`・`#[ignore]` 忘れ等）全開発者の default gate が fixture 必須で壊れる — `#![cfg]`+`#[ignore]` の二重ゲートを pin
- optional dep を非 optional に誤記すると default dep 木が汚染される — `cargo tree` の dep 木監査を action 14 の gate に組込み
