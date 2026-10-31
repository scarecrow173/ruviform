---
id: "IMPL-034PqMkXQjnEVxWEUQBwFb"
type: "impl"
status: "completed"
title: "Issue #173 rvf://data paging contract documentation"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/173"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-173-rvf-data-paging-contract-documentation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/README.md"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/scheme.rs"
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-ui/src/data.rs"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-156-gui-host-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-156-gui-foundation-verification.md"
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

# Issue #173 rvf://data paging contract documentation

## Summary

`rvf://data` の paging 契約を crate 文書へ明文化する Issue #173 対応。
`scheme` ハンドラの実装を読み解き、`crates/ruviform-ui-webview`
README に canonical な wire 契約節(リクエスト/レスポンス・
per-request cap・status 対応・frontend paging loop の例コード)を
追加した。調査中に 1 件の文書↔実装乖離を発見 — approved design
`issue-156-gui-host-foundation.md` は `offset`/`len` 範囲外を 400
と定めるが、実装は全 `DataError` を 404 に潰していた。task の
「文書と実装の乖離があれば最小修正まで可」の許容範囲で、
`OutOfRange → 400`・`Read` 系 → `500` へ status 対応を修正し、
テストで status を pin した。

## Context

audio-lens #105 GUI 実装の起票案として、frontend が paging loop を
書くために必要な `rvf://data` の実装依存仕様(per-request cap・
`X-Rvf-Total` による全長取得)を crate 文書/README へ明文化する
依頼。上位仕様は Issue #156 の approved design
`docs/designs/issue-156-gui-host-foundation.md` の `rvf://`
カスタムプロトコル節。

## Implementation

### Changes

- `crates/ruviform-ui-webview/README.md`: `## rvf://data paging 契約`
  節を新設。リクエスト(`<name>` slug 規則・`offset`/`len` query・
  未知 key 無視・`=` なし pair は malformed)、レスポンス header
  一覧(`Content-Type: application/octet-stream`・`ACAO: *`・
  `Cache-Control: no-store`・`X-Rvf-Total`)、status 対応表
  (400/404/500)、`offset == total` の空 200 境界、per-request cap
  (`DataRegistry::max_chunk`・既定 `DEFAULT_MAX_CHUNK` = 16 MiB・
  `with_max_chunk` で変更・cap 超過は黙って切詰め)、cap/ソースの
  実行時 discovery(`sys.capabilities` の `data.max_chunk` /
  `data.sources`)、`X-Rvf-Total` 駆動の paging loop 例
  (JS・`__rvf.dataUrl` + `fetch`)を記載。
- `crates/ruviform-ui-webview/src/scheme.rs`:
  - module doc の `rvf://data` bullet を wire 契約の normative 形
    (cap・header・status・境界挙動)へ拡張し、README を canonical
    とする旨を記した。
  - `serve_data` の `DataError` → status 対応を修正:
    `NotFound → 404`・`OutOfRange → 400`・`Read` 等その他 → `500`
    (従来は一律 404 — design の「範囲外 → 400」と乖離)。
  - テスト `data_rejects_bad_requests` を status pin 形へ更新
    (400 群と 404 を分離・`len=xyz`・`=` なし pair を追加)。
    `data_serves_bounded_ranges` に `offset == total → 200` 空
    body + `X-Rvf-Total` の境界 assert を追加。
- `crates/ruviform-ui/README.md`: 有界バイナリ bullet に wire 契約
  への参照(adapter README の paging 契約節)を追記。
- `crates/ruviform-ui/src/data.rs`: module doc に wire-level paging
  契約は adapter 側(ruviform-ui-webview README)を canonical とする
  ポインタを追記。

### Decisions During Implementation

- `OutOfRange → 400` へ修正(実装変更・影響明記) — design L258-259
  「`offset`/`len` の検証(非数・負・範囲外 → 400)」が approved な
  意図であり、404 では「未登録名」と「範囲外 offset」を frontend
  が判別できない。400 は HTTP 意味論とも整合する。影響:
  `offset > total` への応答が 404 → 400 に変わる。本 adapter は
  Issue #156 投入直後で外部 consumer は demo_app のみ(!r.ok の
  一律判定)のため影響は限定的。
- `DataError::Read` 系 → `500` — design は未規定だが、登録済み
  source の read 失敗はサーバ側障害であり 404 に潰すと「未登録」
  と誤認される。`#[non_exhaustive]` のため wildcard arm が必須で
  あり、将来 variant も 500 系へ倒す。
- `offset == total` は 200 空 body のまま維持・明文化 — 実装上は
  `avail = 0` で `want = 0` の正常経路。paging loop の終端到達点
  として契約に組み込んだ。
- 契約文書の置き場所は webview README — issue の target どおり
  adapter が wire 契約を所有する層(`ruviform-ui` の `data` は
  registry 側の意味論のみ保持・module doc にポインタのみ)。
- cap の根拠(16 MiB・全量一括を提供しない pull 設計・PoC 実測
  ~94–146 MB/s)は design/PoC 由来の値として記載。`HostConfig`
  の `max_chunk_bytes` 項目は design に残るが実装は registry
  保有 — 別件の design/impl 乖離として記録(本 Issue では変更
  しない)。

## Related Experiments

### Adopted

なし — 文書化と status 対応の最小修正のみ。

### Rejected

なし。

## Validation

- `git fetch origin main && git merge origin/main` — Already up to
  date(作業開始時点で main 最新)
- `mise run check-ui-webview` — exit 0(`cargo check -p
  ruviform-ui-webview --features webview --all-targets`)
- `mise run lint-ui-webview` — exit 0(clippy `-D warnings` clean)
- `mise run test-ui-webview` — exit 0(scheme 6 tests pass、うち
  `data_serves_bounded_ranges` に `offset == total` 空 200 境界・
  `data_rejects_bad_requests` に 400/404 の status pin を追加)
- `mise run fmt` — exit 0(新規差分のみ・予期しない再整形なし)
- `mise run check` — exit 0(`cargo check --workspace --all-targets`)
- `mise run lint` — exit 0(`cargo clippy --workspace --all-targets --
  -D warnings`)
- `mise run test` — exit 0(`cargo test --workspace` 全緑)。1 回目の
  実行で `ruviform-plugin-host` の `l1::video::tests` 2 件が
  `waiter registered` panic で flaky fail — 単体再実行・crate
  単位再実行では全 205 pass、全体再実行でも pass。本変更
  (ui-webview scheme + docs)とは無関係な既知 flake 候補として
  記録。

## Risks

- status 対応変更(404 → 400/500)は wire 挙動の変化 — demo_app
  以外の consumer が 404 厳密比較に依存していた場合は影響するが、
  現状そのような consumer は存在しない。Issue #173 の最小修正
  許容範囲として記録。
- `HostConfig` に `max_chunk_bytes` が無い(design 記載あり) —
  cap は `DataRegistry` が持つ実装の方が registry 単体利用
  (headless `demo_cli`)とも整合するため現状維持。design 側の
  更新は別 Issue 候補。

## Follow-ups

- design `issue-156-gui-host-foundation.md` の `HostConfig`
  `max_chunk_bytes` 記述と実装(`DataRegistry::with_max_chunk`)の
  乖離解消 — 別 Issue で design 側を実装へ合わせるのが自然。
- audio-lens #105 側で本契約に基づく paging loop を実装する際、
  `sys.capabilities` の `data.max_chunk` で cap を実行時取得する
  手順が README 例と一致することを確認。
