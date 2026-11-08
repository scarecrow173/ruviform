---
id: "DESIGN-034UaAirEnkd41uDSZZzUH"
type: "design"
status: "approved"
title: "Issue #468 RootedDir canonical 前提の設計判断"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/468"
    - "https://github.com/scarecrow173/synvelis/issues/184"
    - "https://github.com/scarecrow173/synvelis/pull/186"
  changes:
    added:
      - type: "doc"
        path: "docs/adr/issue-468-rooteddir-canonical-premise.md"
      - type: "doc"
        path: "docs/specs/issue-468-rooteddir-canonical-premise.md"
      - type: "doc"
        path: "docs/plans/implement-issue-468-rooteddir-canonical-premise.md"
      - type: "doc"
        path: "docs/tasks/implement-issue-468-rooteddir-canonical-premise.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/src/rooted.rs"
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-468-rooteddir-canonical-premise.md"
  implemented-by: []
  depends-on:
    - "../specs/rooted-read-capability.md"
    - "rooted-read-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/issue-468-rooteddir-canonical-premise.md"
    - "../test-specs/rooted-read-capability.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-468-rooteddir-canonical-premise.md"
  derived-by:
    - "../plans/implement-issue-468-rooteddir-canonical-premise.md"
    - "../adr/issue-468-rooteddir-canonical-premise.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #468 RootedDir canonical 前提の設計判断

## Context

`RootedDir`（`crates/ruviform-io-file/src/rooted.rs`）の containment は
2 層で構成される。

1. **anchor の導出**: `RootedDir::open`/`open_with` は `path` を
   `std::path::absolute` で絶対化して open し、canonical root を open
   済み handle から導出する（`canonicalize_handle`: Linux
   `/proc/self/fd/N` readlink・Windows `GetFinalPathNameByHandleW`・
   Apple `fcntl(F_GETPATH)`。導出不能 backend のみ `fs::canonicalize`
   fallback）。`open_scoped_root` も解決済み handle の
   `canonicalize_handle` のみから新 anchor を導出し、不能なら
   `Unsupported`。**`open` に渡す path 文字列の綴りは問わない** —
   非 canonical 綴り（`/var/...`、junction 経由、8.3 名）を渡しても
   anchor は canonical になる。
2. **解決時の照合**: `resolve.rs` の two-state walk は、絶対 link
   target を遭遇すると TEXTUAL モードに入り、target の成分列を
   `canonical_comps`（anchor の正規化成分列）と成分単位で照合する。
   `virtual == canonical_comps` に達すれば FD モードへ復帰、prefix を
   外れた時点で tainted → `EscapesRoot`（sticky）。Windows では成分比較が
   case-insensitive（NT `OBJ_CASE_INSENSITIVE` に合わせた既存規則）。

問題の所在は層 2 のみである。tree 内 symlink の target 文字列は
consumer が link を作成した時点で決まる**外部データ**であり、
resolver はそれを canonical root の成分列と文字列照合するだけで、
filesystem への再解決は一切行わない。したがって `/var/...`（実体は
`/private/var/...`）や 8.3 短縮名で綴られた in-root 絶対 target は、
kernel 解決では root 内に届くが TEXTUAL 照合で prefix が外れ、
`EscapesRoot` になる。

synvelis#184 はこの誤判定を consumer fixture（`temp_dir()` の非
canonical 綴りから絶対 target を組み立て）が踏んだ事例であり、
synvelis#186 で consumer 側 `fs::canonicalize` により解消済み。

## Scope

### In Scope

- 現行 containment 契約が意図どおりかの設計判断の記録（本書 + ADR）。
- 「canonical 前提」の明示: 絶対 link target は
  `RootedDir::canonical_root()` の成分列で綴るか相対 target にする、
  という consumer 向け前提を API doc（module doc・`open`/
  `open_scoped_root`/`canonical_root`/`FollowMode::WithinRoot`）と
  crate README に記述する。
- 既存 spec/design/test-spec（`rooted-read-capability` 系）との
  relation 整備。

### Out of Scope

- resolver（`rooted.rs`/`rooted/resolve.rs`/`vfs/*`）の挙動変更。
- alias 誤判定を解消する新しい照合方式の実装（採用しない判断の記録
  までが本 Issue の範囲。将来再検討する場合は別 Issue で設計する）。
- consumer 側の修正・新規 helper API・依存追加。

## Components and Boundaries

- `RootedDir::open`/`open_with`/`open_scoped_root`: anchor 導出のみを
  担う。引数 path の綴りは canonical を要求しない（handle 由来
  canonicalize が吸収する）。本 Issue では doc comment の明確化のみ。
- `resolve`（two-state walk）: TEXTUAL 照合の規則は不変。本 Issue
  ではコード変更なし。
- `canonical_root()`: consumer が絶対 link target を組み立てる際の
  canonical 綴りの公開窓口。doc で「この成分列で綴った絶対 target のみ
  受理される」ことを既に明記しており、本 Issue で consumer 向けの
  使い方（link 作成時にこの path を基底にする）を補強する。
- 文書層: spec（`issue-468-...`）→ ADR（判断）→ design（本書）→
  plan → task。既存 `rooted-read-capability` 文書群が上流契約の正本。

## Data and Control Flow

- link target 文字列（外部データ）→ `split_target` で absolute/
  relative に分類 → absolute は TEXTUAL モードで `virtual_comps` を
  `canonical_comps` と照合 → prefix 内なら FD モードで root handle
  から suffix を walk → prefix 外なら `EscapesRoot`。
- canonical 前提が効くのは「absolute target の綴り == canonical
  anchor の成分列」という文字列契約のみ。relative target・`..` の
  chain pop・FD walk はこの前提に依存しない。

## Non-Functional Constraints

- **TOCTOU**: 「open 以降 path 文字列を再解決しない」不変条件を維持
  する。照合時正規化（Option 2）はこの不変条件を部分的に破るため
  却下（ADR 参照）。
- **escape 防止**: 誤判定は wrong-reject（fail-closed）方向にのみ
  働き、root 外 object の byte/facts を返す経路を開かない。
- **security 根拠（Issue の要求）**: 本設計は挙動を変えないため、
  TOCTOU 排除・escape 防止の保証は現行のまま維持される。仮に
  「照合時に両辺正規化」を採用する場合でも、canonicalize の結果を
  prefix 判定にのみ使い残りを handle 相対で walk すれば byte の
  保証自体は維持し得るが、(a) root 外成分への canonicalize が存在・
  種別の oracle 的接触と block し得る I/O を生む、(b) canonicalize と
  walk の間に判定対象の綴りが差し替わる TOCTOU 窓が残る、(c) link ごとに
  全 path 解決コストがかかる、というコストがあり、得られるのは
  誤判定緩和のみで不釣り合い — よって採用しない。
- RT 制約: 変更は doc comment のみで、全 API 非 RT の契約に影響なし。

## Risks and Trade-offs

| Risk | 対処 |
| --- | --- |
| consumer が canonical 前提を読まず alias 綴りで link を作り続ける | API doc・README・本 ADR で前提を明示。`canonical_root()` を link target の基底として使う手順を doc 化。残存リスクは consumer 側の doc 参照義務として受容 |
| `canonical_root()` の綴りを永続化して別環境で再利用する consumer | doc で「その capability の anchor に対する綴り」であることを明記。環境横断の永続化は契約外 |
| 将来 alias 受理を求める要件が出る | 本 ADR の再検討条件（下記 Deferred）に従い別 Issue で設計する。安易な局所修正で resolver に path 再解決を戻さない |

## Deferred Design Concerns

- alias 綴りの in-root target を受理する照合方式（例: link target の
  canonicalize + suffix の handle 相対 walk）| reason: 本 capability の
  「path 文字列を再解決しない」不変条件と衝突し、oracle 的接触・
  TOCTOU 窓・コストを伴う | re-engagement trigger: consumer から
  「alias 綴りをそのまま受理したい」という明確な要件、または
  canonical 綴りで link を作れない consumer の実在 | risk if ignored:
  現状維持でも wrong-reject のみで実害は限定的

## Verification Notes

- [ ] `mise run check-io-file` / `mise run test-io-file` が pass
  （`absolute_link_alias_spelling_escapes_root` が契約を pin したまま
  無変更で通ることを含む）。
- [ ] `mise run check` / `mise run lint` が pass。
- [ ] API doc に canonical 前提の記述が存在すること
  （`RootedDir` module doc・`open`/`open_scoped_root`/
  `canonical_root`/`FollowMode::WithinRoot`）。
- [ ] doc 監査（`doc-status` 系）で新規文書チェーンがエラーなく
  投影されること。
