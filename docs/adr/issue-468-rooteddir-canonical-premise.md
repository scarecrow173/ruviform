---
id: "ADR-034UayXObVNeujQhc8JEP5"
type: "adr"
status: "proposed"
title: "Issue #468 RootedDir containment の canonical 前提を契約として維持する"
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
        path: "docs/specs/issue-468-rooteddir-canonical-premise.md"
      - type: "doc"
        path: "docs/designs/issue-468-rooteddir-canonical-premise.md"
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
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/rooted-read-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-468-rooteddir-canonical-premise.md"
    - "../designs/issue-468-rooteddir-canonical-premise.md"
    - "../specs/rooted-read-capability.md"
    - "../designs/rooted-read-capability.md"
    - "../test-specs/rooted-read-capability.md"
    - "rooted-read-capability-placement.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-468-rooteddir-canonical-premise.md"
    - "../designs/issue-468-rooteddir-canonical-premise.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Issue #468 RootedDir containment の canonical 前提を契約として維持する

## Context and Problem Statement

`ruviform-io-file` の `RootedDir` は、`FollowMode::WithinRoot` で絶対 link
target を canonical root の成分列と TEXTUAL に照合する
（`src/rooted/resolve.rs` の two-state walk）。このため、root 内を指す
symlink でも、target の綴りが canonical root の成分列と一致しない
「非 canonical alias」（macOS `/var` vs `/private/var`、Windows の
junction・8.3 短縮名・`subst` ドライブ等）で書かれていると、kernel 上は
root 内に解決されるのに `EscapesRoot` で拒否される。

synvelis#184（CI `verify-macos` / `verify (windows-latest)` の失敗）で
この誤判定が観測された。原因は consumer 側 fixture が
`std::env::temp_dir()` の非 canonical 綴りから絶対 symlink target を
組み立てていたことで、synvelis は fixture を `fs::canonicalize` する
修正（PR scarecrow173/synvelis#186、production コード無変更）で解消
済みである。

Issue #468 は、(a) 現行の containment 契約が意図どおりかの設計判断を
記録し、(b) 「canonical 前提」を契約文書・API doc・debug check の
いずれかで明示することを求める。

前提として確認した事実:

- `RootedDir::open` / `open_with` は `path` を `std::path::absolute` で
  絶対化したうえで handle を open し、canonical root は open した
  handle から導出する（Linux `/proc/self/fd/N` readlink、Windows
  `GetFinalPathNameByHandleW`、Apple `fcntl(F_GETPATH)`。backend が
  導出不能なら `fs::canonicalize` へ fallback）。つまり **`open` に
  渡す path の綴りは canonical でなくてよい** — anchor は常に
  handle 由来の canonical 綴りになる。
- `open_scoped_root` も同様に、解決した directory handle の
  `canonicalize_handle` のみから新しい anchor を導出し、導出不能なら
  `Unsupported` で失敗する（anchor の捏造・path 文字列への再 bind は
  しない）。
- 問題となるのは **tree 内に既に存在する symlink の target 文字列の
  綴り**であり、これは consumer が link を作成した時点で決まる外部
  データである。resolver が比較するのはその文字列と `canonical_root()`
  の成分列だけであり、filesystem への再解決は行わない。

## Decision Drivers

- containment の中核不変条件「open 以降、tree 内で path 文字列を再解決
  しない」を維持する（この不変条件が TOCTOU を構造的に排除している）。
- escape 防止を弱めない — 誤判定は保守方向（wrong-reject）にのみ働き、
  root 外 object の byte/facts を返す経路を新たに開かない。
- consumer が契約を読んだだけで canonical 前提を理解できること
  （synvelis#184 型の再発防止）。
- 変更は最小の整合的変更に留め、resolver の挙動・公開 API 形状を
  変えない。

## Considered Options

1. **現状維持 + 契約明確化**（採用）— TEXTUAL canonical-prefix 照合を
   そのまま維持し、「絶対 link target は `canonical_root()` の成分列で
   綴るか相対にする」という前提を API doc・module doc・README・
   本 ADR で明示する。resolver のコード変更は行わない。
2. **照合時に両辺を正規化** — 絶対 target を遭遇した時点で
   `fs::canonicalize` 等で正規化してから prefix 判定する。alias 綴りの
   誤判定は解消するが、任意の絶対 path（root 外を含む）の再解決を
   resolver に持ち込み、capability の中核不変条件と衝突する。却下。
3. **ルート登録時の canonicalize 強化** — `open`/`open_scoped_root` の
   anchor 導出は既に handle 由来 canonical であり、問題は root 引数の
   綴りではなく link target の綴りなので、この選択肢は問題を解かない。
   追加の canonicalize を入れても anchor は変わらない。却下。

## Decision Outcome

Chosen option: **1（現状維持 + 契約明確化）**。現行契約は意図どおりで
あり、canonical 前提を契約として明示する。

理由:

- 誤判定は保守方向（wrong-reject）にのみ働く。alias 綴りの in-root
  target を `EscapesRoot` で拒否しても、root 外の byte/facts が返る
  経路は一切開かない。fail-closed であり、セキュリティ上の実害はない。
- Option 2 は「open 以降 path 文字列を再解決しない」という本
  capability の存在理由そのものを部分的に再導入する。canonicalize は
  root 外の path 成分にも触れ、判定と後続の handle 相対 walk の間に
  TOCTOU の窓を残し、link ごとの全 path 解決コストを課す。得られるのは
  誤判定の緩和だけであり、対価が不釣り合い。
- Option 3 は既に実施済み（handle 由来 anchor）であり、問題の所在
  （link target の綴り）に効かない。
- consumer 側の対処は確立済み: 絶対 target を `canonical_root()` から
  組み立てる、相対 target を使う、または作成側 path を
  `fs::canonicalize` する（synvelis#186 の先例）。
- debug check（debug assertion）は採用しない: canonical 前提は link
  target という外部データの綴りに関する契約であり、resolver が assert
  できる内部不変条件を持たない（alias 綴りの wrong-reject は仕様
  どおりの動作であって不変条件の破れではない）。契約の実行可能な
  pin は既存の実 FS 試験 `absolute_link_alias_spelling_escapes_root`
  が既に担っている。

### Consequences

* Good, because containment 契約・resolver・公開 API 形状は不変であり、
  TOCTOU 排除と escape 防止の保証は一切弱まらない。
* Good, because consumer は API doc と README から canonical 前提を
  直接読めるようになり、synvelis#184 型の誤判定の原因を自己診断できる。
* Bad, because 非 canonical alias 綴りの絶対 in-root link は今後も
  `EscapesRoot` になる。これは意図した縮小であり、consumer が
  canonical 綴りまたは相対 target を使うことで回避する。
* Bad, because `canonical_root()` が返す綴りは OS/backend 依存であり、
  consumer がそれを永続化して別環境で使い回すと同じ誤判定を踏み得る
  （契約上は「その capability の anchor に対する綴り」であることを
  doc で明示する）。

### Confirmation

- `mise run test-io-file`（`cargo test -p ruviform-io-file` を含む）の
  実 FS 試験 `absolute_link_alias_spelling_escapes_root` が、非 canonical
  alias 綴りの in-root 絶対 target が `EscapesRoot` になる契約をそのまま
  pin し続けること（本 ADR は挙動を変えないため、この試験が回帰
  防止として機能する）。
- API doc・README に canonical 前提の記述が存在すること
  （`RootedDir` module doc・`open`/`open_scoped_root`/`canonical_root`/
  `FollowMode::WithinRoot`）。

## Implementation Plan

* Affected paths: `crates/ruviform-io-file/src/rooted.rs`（module doc と
  `RootedDir::open`/`open_with`/`open_scoped_root`/`canonical_root`/
  `FollowMode::WithinRoot` の doc comment のみ。コード不変）、
  `crates/ruviform-io-file/README.md`（rooted 節への前提追記）。
* Patterns to follow: 既存 doc comment の英語トーンと「RT contract」
  記法、spec `rooted-read-capability.md` の縮小記述、README の箇条書き
  構成。
* Patterns to avoid: resolver の挙動変更、`canonicalize` の resolver への
  持ち込み、consumer 向けの新規 helper API 追加（YAGNI — 前提の明示が
  本 Issue の範囲）。
* Dependencies or configuration: なし（依存追加・feature 追加なし）。

## Verification

* [ ] `mise run check-io-file` が pass する（doc comment のみの変更で
  型・lint に影響しないことの確認を兼ねる）。
* [ ] `mise run test-io-file` が pass する（`absolute_link_alias_spelling_escapes_root`
  を含む既存試験が無変更で通る = 契約不変の実測）。
* [ ] `mise run check` / `mise run lint` が pass する。
* [ ] `docs/adr`・`docs/specs`・`docs/designs`・`docs/plans`・
  `docs/tasks` の doc 監査（`doc-status` 系）が新規文書でエラーを
  出さない。

## Pros and Cons of the Options

### Option 1: 現状維持 + 契約明確化（採用）

* Good, because 中核不変条件と保証を一切変更せず、consumer の誤解だけを
  doc で解消できる。変更量が最小で回帰リスクがない。
* Bad, because alias 誤判定そのものは残る。consumer が前提を読まない
  限り再発し得る（doc 明示で緩和するのみ）。

### Option 2: 照合時に両辺正規化

* Good, because alias 綴りの in-root target が正しく受理され、
  consumer 側の canonicalize 義務が不要になる。
* Bad, because 任意絶対 path の再解決を resolver に持ち込み、「open
  以降 path 文字列を再解決しない」不変条件を破る。root 外成分への
  canonicalize は存在・種別の oracle 的接触と block し得る I/O を
  生じ、canonicalize→walk の間に TOCTOU 窓を残す。link ごとの
  全 path 解決コストも増える。保証を守るために保証の根拠を削る
  変更であり、採用しない。

### Option 3: ルート登録時の canonicalize 強化

* Good, because anchor の canonical 性は既に handle 由来で保証済み。
* Bad, because 問題は root 引数の綴りではなく tree 内 link target の
  綴りであり、anchor をいくら正規化しても誤判定は変わらない。
  実質 no-op の変更になる。

## More Information

- 観測元: synvelis#184（CI symlink fixture 失敗）、consumer 側修正
  PR scarecrow173/synvelis#186。
- 既存契約の正本: `docs/specs/rooted-read-capability.md`（WithinRoot の
  alias 縮小を意図的と明記）、`docs/designs/rooted-read-capability.md`、
  `docs/test-specs/rooted-read-capability.md`。
- 実装: `crates/ruviform-io-file/src/rooted.rs`（`RootedDir`・
  `canonical_root`・`FollowMode::WithinRoot` の doc）、
  `crates/ruviform-io-file/src/rooted/resolve.rs`（two-state walk、
  本 ADR では不変）、`crates/ruviform-io-file/tests/rooted_read.rs`
  （`absolute_link_alias_spelling_escapes_root` が契約を pin）。
- 関連 ADR: `rooted-read-capability-placement.md`（rooted capability の
  配置判断）。
