---
id: "IMPL-034R171DataRegEvct9XyZq"
type: "impl"
status: "completed"
title: "Issue #171 DataRegistry eviction/unregister 契約の実装"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/171"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-171-data-registry-eviction-contract.md"
      - type: "file"
        path: "crates/ruviform-ui/tests/data_registry.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/data.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/runtime.rs"
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/scheme.rs"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-171-gate-fmt.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-171-gate-check.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-171-gate-lint.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-171-gate-test.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-171-gate-build.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-171-gate-check-ui-webview.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-171-gate-lint-ui-webview.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-171-gate-test-ui-webview.log"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-156-gui-host-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-156-gui-foundation-verification.md"
    - "docs/impl/ir/issue-173-rvf-data-paging-contract-documentation.md"
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

# Issue #171 DataRegistry eviction/unregister 契約の実装

## Summary

audio-lens #105 の GUI 実装で view op が返す payload 名(`view/<seq>`
等)が session 内で連番増殖し、長時間 session で登録済み
`DataSource` が解放されずメモリが単調増加しうる問題への対応。
`DataRegistry` に解放契約を追加した:

- 明示的解放 `unregister(name) -> bool` / `clear()`
- `DataRegistryLimits::max_sources` による登録名数の LRU 上限
  (`with_max_sources`/`with_limits` で構築、`register`/`get`/`read`
  が recency を更新し、超過時は最も使われていない名から追い出す)
- 観測用 `len()`/`is_empty()`/`max_sources()` と
  `sys.capabilities` の `data.max_sources` 報告

既定の `DataRegistry::new()`/`with_max_chunk()` は従来どおり
無制限で、既存呼出(AL GUI 含む)の挙動・API 署名は変わらない。

## Context

Issue #156 で導入した `DataRegistry` は `register` のみで解放経路が
なく、BTreeMap に登録が蓄積し続ける設計だった。Issue #171 は
「`unregister` または generation/LRU 上限の提供」を求める
audio-lens #105 起票由来の実装依頼。target は
`crates/ruviform-ui/` の `DataRegistry` と、それを使う host/op 層
(`ruviform-ui-webview` の data 面)。

## Implementation

### Changes

- `crates/ruviform-ui/src/data.rs`:
  - `DataRegistryLimits` 公開 struct 追加(`max_chunk: u64` +
    `max_sources: Option<usize>`、`Default` は従来動作=
    16 MiB・無制限)。`DataRegistryLimits::with_max_sources(n)` の
    簡易構築も提供。
  - 内部表現を `BTreeMap<String, Arc<dyn DataSource>>` から
    `Inner { map: BTreeMap<String, Entry>, tick: u64 }` へ変更。
    `Entry.last_used` に単調カウンタを記録し LRU 順序を構成する
    (I/O や時計を使わず単純な世代番号 — issue の
    "generation/LRU 上限" の generation に相当)。
  - `register`: 再登録は recency を更新。`max_sources` 超過時は
    `last_used` 最小の名を上限内まで追い出す(`Some(0)` は何も
    保持しない = 登録が即追い出される、と明文化)。
  - `unregister(name) -> bool` / `clear()` / `len()` /
    `is_empty()` / `max_sources()` を追加。
  - `get`/`read` は recency を更新するため write lock 経由に変更
    (読出し中の source が追い出し対象にならないための LRU 意味論。
    非 RT パスのみで、pull 1 回あたりの lock 取得は従来どおり)。
  - module doc に "Release contract" 節を追加:いつ(unregister/
    clear/eviction)、何が(登録名の解決と payload の Arc 参照)、
    どの程度(名前は即座に NotFound/404、メモリは最後の Arc
    drop 時・進行中 pull は完走)解放されるかを規定。
- `crates/ruviform-ui/src/lib.rs`: `DataRegistryLimits` を export。
- `crates/ruviform-ui/src/runtime.rs`: `sys.capabilities` の
  `data` に `max_sources`(`Option` → number または null)を追加。
- `crates/ruviform-ui/README.md`: 有界バイナリ bullet に解放契約
  の子項目を追加。
- `crates/ruviform-ui-webview/src/scheme.rs`: テスト
  `data_unregister_and_eviction_yield_404` を追加 — unregister
  済み・LRU 追い出し済みの名が `rvf://data` で 404(issue-173 の
  status 対応どおり `NotFound → 404`)になることを wire 境界で
  pin。
- `crates/ruviform-ui/tests/data_registry.rs`: 新規 9 テスト
  (下記 Validation)。

### Decisions During Implementation

- **LRU を採用(FIFO/insertion-order ではなく)** — issue は
  "generation/LRU 上限" を要求。pull model では frontend が複数
  payload を並行 fetch しうるため、登録順追い出しでは「表示中の
  最古 payload」が落ち得る。`get`/`read` を recency 更新対象に
  することで、pull 中の source は追い出されない契約にした。
- **上限は件数(`max_sources`)** — byte 上限は `DataSource::len`
  が仮想・lazy source で実メモリと一致しないため不適切。issue の
  「連番名の増殖」は件数の問題であり、件数上限が最も単純かつ
  予測可能(KISS)。
- **既定は無制限のまま** — `new()`/`with_max_chunk()` の既存
  挙動を変えない(AL GUI 側の既存呼出を壊さない task 制約)。
  有界化は `with_max_sources`/`with_limits` の opt-in。暗黙の
  追い出しによる突然の 404 を既存ユーザへ持ち込まない。
- **解放は Arc ベースの deferred-free** — `unregister` は登録名の
  解決を止めるが、進行中の pull(`get` で取得済みの Arc)は
  完走する。即時強制解放ではなく「新規参照を断つ」契約 — 実装も
  単純(map から remove するだけ)で、read 中に破壊しない。
- **`unregister` の返り値は `bool`** — `Option<Arc<dyn DataSource>>`
  ではなく。解放 API から Arc を返すと呼出側が不用意に保持して
  「解放したはず」のメモリが残る事故を招く。要否が出たら拡張可。
- **eviction の通知は提供しない** — `register` が追い出した名を
  返す API は今回不要(YAGNI)。観測は `names()`/`len()`/
  `sys.capabilities` で可能。
- **追い出しは O(n) 線形走査** — `register` 時・上限超過時のみ。
  LRU 用の双方向リスト+HashMap は過剰(YAGNI)。上限が数千件規模
  でも問題にならないコストで、登録数を有界化する用途では定数回。

### merge 時の upstream との調整

最終検証前の `git merge origin/main` で issue-173
(`rvf://data` paging 契約・status 分離)が取り込まれ、
`data.rs` module doc・`README.md` の同一箇所で doc 競合が発生。
双方の記述を残す形で解消(paging 契約ポインタ + 解放契約節を
併記)。`scheme.rs` の `NotFound → 404` status 対応は本変更の
テスト期待と整合するため追加点なし。`Cargo.lock` は issue-167
由来の依存更新を `cargo generate-lockfile` で再生成。

## Related Experiments

### Adopted

なし — 設計・実装とも初版で確定。

### Rejected

- `Arc<dyn DataSource>` を返す `unregister` — 解放 API の返値で
  payload を延命させる余地を作らない。
- byte ベース上限 — 実メモリと一致しない `DataSource::len` に
  依存するため不採用(上記 Decisions)。

## Validation

ゲート(全て実実行・exit 0、log は `../exp/data/issue-171-gate-*.log`):

| コマンド | 結果 | log |
| --- | --- | --- |
| `mise run fmt` | pass | `../exp/data/issue-171-gate-fmt.log` |
| `mise run check` | pass(`cargo check --workspace --all-targets`) | `../exp/data/issue-171-gate-check.log` |
| `mise run lint` | pass(clippy `-D warnings` clean) | `../exp/data/issue-171-gate-lint.log` |
| `mise run test` | pass(`cargo test --workspace` 全緑) | `../exp/data/issue-171-gate-test.log` |
| `mise run build` | pass | `../exp/data/issue-171-gate-build.log` |
| `mise run check-ui-webview` | pass | `../exp/data/issue-171-gate-check-ui-webview.log` |
| `mise run lint-ui-webview` | pass(`-D warnings`) | `../exp/data/issue-171-gate-lint-ui-webview.log` |
| `mise run test-ui-webview` | pass(scheme 7 tests) | `../exp/data/issue-171-gate-test-ui-webview.log` |

解放契約テスト(`crates/ruviform-ui/tests/data_registry.rs`・9 件):

- `unregister_releases_then_reregisters` — 登録→解放(名前が
  `get`/`read` ともに消え `NotFound`)→二重 unregister が no-op→
  再登録で新 payload が serve される。
- `clear_removes_all_sources` — 全解放で `len`/`names` が空に。
- `held_source_outlives_unregister` — 取得済み Arc は unregister
  後も読み切れる(deferred-free 契約)。
- `unbounded_default_keeps_every_registration` — 既定無制限の
  後方互換 pin。
- `max_sources_evicts_least_recently_used` — 上限超過で最古使用の
  名が追い出される。read による recency 更新で追い出し対象が
  移ることも pin。
- `reregister_counts_as_recent_use` — 同名再登録は recency 更新+
  payload 差替え。
- `sequential_payload_names_stay_bounded` — AL GUI の view op
  連番パターンを模した 1000 件連続登録で登録数が上限(8)に
  有界化され、直近名のみ解決可能。
- `max_sources_zero_retains_nothing` — `Some(0)` は登録即追い出し
  の縮退契約を pin。
- `eviction_and_unregister_drop_the_registry_arc` — 追い出し・
  unregister ともに registry 側の Arc 参照が即時に drop され、
  外部保持分のみ残存することを `strong_count` で pin。

webview 側: `data_unregister_and_eviction_yield_404` が解放・
追い出し後の `rvf://data` が 404 になることを確認(wire 境界の
解放可視性)。

## Risks

- `sys.capabilities` の `data` に `max_sources` フィールドを追加
  (additive のみ・既存フィールドは不変)。
- `get`/`read` が read lock から write lock へ変更 — 同一 registry
  を共有する pull 同士が直列化する。1 pull あたりの lock 保持は
  map 走査のみで source read は lock 外のため影響は軽微(非 RT
  パス)。AL 実規模での長時間負荷は audio-lens 移行時の別計測
  (issue-156 と同じ位置付け)。
- `max_sources` の値は caller 依存 — AL GUI 側で採用する値は
  audio-lens #105 実装側の判断(本 crate は機構のみ提供)。

## Follow-ups

- audio-lens #105 側で `with_max_sources` または view 単位の
  `unregister`/`clear` を採用する際、`sys.capabilities` の
  `data.max_sources` で実行時の上限値を確認する手順を AL 側
  文書へ転記する。
- 件数ではなく総 byte 上限の要求が出た場合は
  `DataRegistryLimits` への追加フィールドで拡張可能(本 Issue
  では YAGNI として見送り)。
