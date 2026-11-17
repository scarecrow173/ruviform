---
id: "DESIGN-034YYnXY7447GWCMHE4dOI"
type: "design"
status: "approved"
title: "Issue #721 post-seal temp 改竄境界 — 現行契約維持の設計判断"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/721"
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/designs/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/plans/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/tasks/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/test-specs/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/impl/ir/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/impl/exp/issue-721-post-seal-temp.md"
      - type: "file"
        path: "docs/impl/exp/issue-721-post-seal-temp-exp.jsonl"
      - type: "file"
        path: "evidence/rf721/"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-721-post-seal-temp.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-721-post-seal-temp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-306-io-e2e-cases.md"
    - "docs/specs/issue-650-bench-smoke-coverage.md"
    - "docs/designs/issue-650-bench-smoke-coverage.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-721-post-seal-temp.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/721"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/650"
    - "docs/designs/issue-306-io-e2e-cases.md"
    - "crates/ruviform-io-file/README.md"
    - "crates/ruviform-io-e2e/tests/staged_lifecycle.rs"
    - "evidence/rf721/"
  defers: []
  deferred-by: []
metadata:
  issue: 721
---

# Issue #721 post-seal temp 改竄境界 — 現行契約維持の設計判断

## Context

`ruviform-io-file` の staged publish pipeline（IO-01 v0.4 /
Issue #283、OS backend Issue #296）は `StagedFile::create` →
write → `seal()` → `SealedFile::publish` の型状態 lifecycle を
提供する。`seal()` は doc 上「pure type transition — no I/O sync
happens here」と明記された write phase の終了であり、
`publish()` の pipeline（validate capability → file sync →
handle preparation → atomic publish → directory sync →
leftover-temp cleanup）は temp 内容の再検証を含まない。

この結果、seal 後〜publish 前に staging temp が外部から rewrite
されると、publish は改竄後の bytes をそのまま destination へ
publish する。`crates/ruviform-io-e2e/tests/staged_lifecycle.rs`
の `post_seal_temp_rewrite_is_not_reverified_before_publish`
（rewrite → 改竄 bytes が publish される）と
`post_seal_temp_delete_fails_publish_not_published`（delete →
`NotPublished`）がこの境界を実 FS で pin 済み
（Issue #306 Slice T / #603 slice T で固定）。

`crates/ruviform-io-file/README.md` §信頼境界・保証範囲は
「宛先 dir の権限・所有は呼び出し側の信頼境界内（同じ境界内の
悪意ある差替えまでは防衛しない）」「Temp identity … は
所有権・ロックの証明ではない」と既に宣言しているが、
「publish が seal 後の temp 内容を再検証しない」ことを直接
述べる項目はなく、seal 保証の時間的境界が読み取りにくい。

`scarecrow173/ruviform#306` の verdict は「現行契約として正直に
pin 済み。再検討する場合は io-file 側の設計 Issue」と本判断を
後送りにしており、`scarecrow173/ruviform#721` がその owner。
本設計文書が判断本体の正本となる。

## Scope

### In Scope

- post-seal temp 改竄境界の設計判断（現行契約維持）の確定と記録。
- consumer（audio-lens・synvelis）の staged publish 利用実績に
  基づく要求保証レベルの照合。
- 検討した強化案（temp 権限強化・publish 時再検証・temp 配置
  分離）の評価と却下/延期理由。
- `crates/ruviform-io-file/README.md` §信頼境界への最小 doc
  補強（post-seal 再検証なしの明示）。
- bench-smoke lane 配線の実測照合（残件 a・検証のみ）。
- `evidence/rf721/` と内部文書チェーン。

### Out of Scope

- `crates/` の実装変更（現行維持のためコード差分なし）。
- `crates/ruviform-io-e2e/tests/staged_lifecycle.rs` の変更
  （既存 `post_seal_temp_*` 試験が採用契約を pin するため
  期待更新・新規試験は不要）。
- `docs/development-environment.md`（`scarecrow173/ruviform#720`
  が占有中）・`mise.toml`・docs index・consumer リポジトリ。

## Components and Boundaries

- **`ruviform-io-file`（契約所有者）**: `StagedFile`/`SealedFile`
  の型状態 pipeline と `PublishOptions`/`PublishReceipt`/
  `RecoveryHandle` の結果語彙。post-seal temp の内容完全性は
  契約の保証範囲外であり、責務は「seal 時点で write phase を
  終了した bytes を、要求された durability 付きで atomic に
  公開する」ことに限定される。
- **`ruviform-io-e2e`（契約 pin）**: `staged_lifecycle.rs` の
  `post_seal_temp_*` 2 試験が実 FS で境界を固定する。mock 正本
  （`src/staged.rs` fault-injection）は誘導可能な故障を担い、
  e2e は実 FS で再現可能な振る舞いのみを assert する責務分離を
  維持する。
- **consumer（信頼境界の内側）**: audio-lens `al-job`/`al-media`、
  synvelis `synvelis-library`/`synvelis-player` はいずれも
  `StagedFile::create` → write/seal → `publish` を同一プロセス・
  同一ユーザの通常経路で連続実行し、seal 後に temp を外部から
  改竄する正当なユースケースを持たない。

## Data and Control Flow

1. `StagedFile::create(dest)` — 宛先 dir に temp sibling を排他作成。
2. write phase — `Write + Seek` で bytes を供給。format/codec の
   `finish()` は呼び出し側責務（seal は内容の完成・妥当性を
   主張しない、と doc 明記済み）。
3. `seal()` — 純粋な型遷移。I/O sync なし。
4. **post-seal 窓** — temp path は `SealedFile::temp_path()` で
   公開されるが diagnostic 用であり、所有権・ロックの証明では
   ない。この窓で temp が外部改竄・削除されても io-file は
   検出しない（契約上の保証外）。
5. `publish(options)` — capability 検証 → 要求された file sync →
   handle 準備 → atomic publish（rename/link）→ 要求された
   directory sync → leftover-temp cleanup。temp 内容の再検証は
   pipeline に存在しない。

## Design

### 決定 1: 現行契約を維持する — publish は post-seal temp を再検証しない

post-seal temp 改竄を「seal 保証の外・呼び出し側信頼境界内の
事象」として契約に明記し、現行動作（rewrite は publish される・
delete は `NotPublished`）を維持する。これは #306 で pin された
境界の追認であり、io-file の動作変更を伴わない。

根拠:

- **既存の信頼境界宣言と整合**: README §信頼境界は宛先 dir の
  権限・所有を呼び出し側境界とし「同じ境界内の悪意ある差替え
  までは防衛しない」と明示済み。temp は宛先 dir の sibling で
  あり、同じ境界に置かれる。post-seal 改竄はまさにその境界内の
  事象であり、防衛しないことは既に宣言された設計判断の範囲内。
- **seal の意味論と整合**: `seal()` は「write phase の型レベル
  終了」であって「publish まで内容が固定される」という連続保証
  ではない。format 完成・妥当性は呼び出し側責務と doc 明記済み
  であり、seal〜publish 間の外部改竄を io-file が検出する責務は
  契約に存在しない。
- **consumer 要件を超えない**: audio-lens・synvelis の全利用箇所
  は同一信頼境界内の連続 pipeline であり、post-seal 改竄への
  防御を要求する consumer は存在しない（後述の照合）。

### 決定 2: consumer 要件照合 — 全利用が同一信頼境界内で完結

| consumer | 利用箇所 | 用途 | post-seal 保証への要求 |
| --- | --- | --- | --- |
| audio-lens | `crates/al-job/src/fsutil.rs`（`stage_file`/`publish_sealed`） | job 成果物の staged publish + temp 回復 policy | なし — 同一プロセスで seal→publish を連続実行 |
| audio-lens | `crates/al-media/src/wav.rs`（`publish_sealed`） | WAV 出力の staged publish | なし — 同上 |
| synvelis | `crates/synvelis-library/src/import.rs`・`cache.rs`・`preview.rs` | import/manifest/payload の staged publish | なし — 同上 |
| synvelis | `crates/synvelis-player`（tests 含む） | 再生出力の staged publish | なし — 同上 |

いずれも「アプリ自身が書いた bytes を同じ信頼境界内で公開する」
通常経路であり、seal 後の temp に対する外部改竄は脅威モデルに
含まれない。consumer 側に検出・再検証を要求する Issue・要件は
確認されなかった（関連 Issue `scarecrow173/audio-lens#127`・
`scarecrow173/synvelis#28`/`#84`/`#128`/`#17`/`#91` は強化選択時の
受入側候補として spec に記録済み）。

### 決定 3: 検討した強化案はいずれも却下または延期

- **seal 時点での temp 権限強化（read-only 化・owner-only ACL）**:
  同一信頼境界内の攻撃者（同一ユーザ・同一権限）は permission
  bit/ACL を変更するか temp を削除して再作成できるため実効的な
  境界を生まない。Windows（readonly 属性/ACL・share-lock）と
  POSIX（mode bit）で仕組みが異なり、OS 別挙動差の検証・維持
  コストが増す割に防げる脅威が増えない。
- **publish 直前の内容再検証（seal 時ハッシュ→publish で再計算）**:
  publish 毎に temp 全体の追加 read が必要で hot path のコストに
  なる。再検証〜rename の間に改竄される TOCTOU は残り、完全な
  保証には file handle を保持した同一 inode 検証など更なる複雑さ
  が要る。consumer が要求しない防御にこのコストと複雑さを載せる
  のは YAGNI に反する。
- **temp の配置分離（宛先 dir 外の非公開 temp 領域）**: staged
  publish の前提は temp と宛先が同一 filesystem 上にあること
  （atomic rename の成立条件）であり、配置分離は契約の根幹を
  変える。宛先 dir 自体が呼び出し側信頼境界である以上、別 dir に
  置いても境界は同じ。

これらは「実装しない」ではなく「現時点の consumer 要件に対して
コストが便益を超える」として spec の Deferred Design Concerns に
記録し、再検討トリガー（consumer が post-seal 改竄を脅威モデルに
含める要件を出した場合）を明示する。

### 決定 4: README §信頼境界への最小 doc 補強

`crates/ruviform-io-file/README.md` §信頼境界・保証範囲に、
「publish は seal 後の temp 内容を再検証しない。post-seal の
temp 外部改竄・削除は呼び出し側信頼境界内の事象として扱い、
改竄 bytes の publish・削除による `NotPublished` は契約上の
定義済み結果である」旨を 1 項目追加する。実装・試験の変更は
なく、既存 `post_seal_temp_*` 試験が採用契約を pin し続ける
ことをここで明記する。

## Non-Functional Constraints

- **信頼性**: 失敗分類（`PublishEffect`/`TempState`）と recovery
  契約は不変。doc 補強のみで動作・結果語彙に影響しない。
- **性能**: 実装変更なしのため publish コスト・bench 値に影響
  なし（`io_composition` の `staged_publish` 等の計測系に差分なし）。
- **セキュリティ**: 境界の正直な記録が目的。防御を装った部分的
  検証を追加しない（実効境界を生まない保証は書かない）。
- **運用**: OS 別挙動差（Windows ACL/readonly・POSIX 権限 bit・
  share-lock）は新規検証対象を生まない — 実装変更がないため。

## Risks and Trade-offs

- **誤読リスク**: 「seal = publish まで内容固定」と誤読されると
  consumer が暗黙の完全性保証に依存しかねない → README 補強と
  本設計文書で境界を明示して緩和。`post_seal_temp_*` 試験名の
  `not_reverified` が機械的な検出点として残る。
- **将来の要件変化**: consumer が post-seal 改竄防御を要求した
  場合は本設計を supersede して再検討する。Deferred に再検討
  トリガーを記録済み。
- **強化案の先送り**: 「とりあえず read-only 化」的な部分防御は
  実効境界を生まず複雑さだけを残すため、採用しない判断自体が
  trade-off の記録となる。

## Deferred Design Concerns

- publish 時の内容再検証（seal 時ハッシュ→publish 直前再計算・
  同一 inode 検証を含む完全形）| consumer 要件を超える防御で
  TOCTOU を残す部分実装は実効境界を生まない | consumer が
  post-seal 改竄を脅威モデルに含める要件を出した場合 | 境界の
  曖昧さが残る — README 補強と試験 pin で緩和済み。

## Verification Notes

- [x] `mise run bench-smoke` で `ruviform-io-e2e` `io_composition`
      全 6 case が計測なし 1 回実行され exit 0（残件 a 実測 —
      `evidence/rf721/bench-smoke-io-composition-excerpt.txt`）。
- [x] `docs/development-environment.md` の bench 系記述と
      `mise.toml` `[tasks.bench-smoke]` の照合（read-only —
      `evidence/rf721/verification.md`）。
- [x] `mise run test ruviform-io-file` / `mise run test-io-e2e`
      が pass（既存 suite 回帰なし — 実測 exit 0、
      `evidence/rf721/verification.md`）。
- [x] `mise run fmt-check` 等の文書・構成の軽量チェック
      （exit 0）。check/lint lane も io-file/io-e2e で実測 pass。
- [x] review-protocol による文書チェーンの self-review
      （blocker なし）。

## Alternatives Considered

- **ADR（`docs/adr/`）への記録**: 契約判断としては妥当だが、
  本 slice は「設計判断 + doc 補強 + 検証」の設計文書型作業で
  あり、`docs/designs/issue-306-io-e2e-cases.md`・
  `docs/designs/issue-650-bench-smoke-coverage.md` の先例に倣い
  design doc を採用。plan-doc の design gate（approved design
  doc 必須）とも整合する。
- **「強化」実装の採用**: 上記決定 3 の通り、consumer 要件を
  超えるコストと OS 別複雑さに対し実効的な境界を生まないため
  不採用。

実測結果は `evidence/rf721/`・`docs/impl/ir/`・`docs/impl/exp/` に記録する。
