# Issue #721 post-seal temp 境界確定 — 実装実験記録（2026-11-17）

`docs/tasks/issue-721-post-seal-temp.md` の slice（post-seal temp
改竄境界の設計判断確定 + bench-smoke 配線照合 + README 補強）で
試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-721-post-seal-temp.md`](../ir/issue-721-post-seal-temp.md)、
イベントログは
[`issue-721-post-seal-temp-exp.jsonl`](issue-721-post-seal-temp-exp.jsonl)
を参照。

## 試行 1: consumer 要件の read-only 照合

- **目的**: 「現行契約維持」か「検出/強化の実装」かの判断材料と
  して、実 consumer が post-seal temp に要求する保証レベルを
  確定する。
- **観測**: audio-lens は `crates/al-job/src/fsutil.rs` の
  `stage_file`（`StagedFile::create`）→ `publish_sealed`
  （`SealedFile::publish` + temp 回復 policy）と
  `crates/al-media/src/wav.rs` の `publish_sealed` を使用。
  synvelis は `crates/synvelis-library/src/import.rs`・
  `cache.rs`・`preview.rs` と `crates/synvelis-player`（test 含む）
  で `StagedFile`/`SealedFile` pipeline を使用。いずれも
  create→write→seal→publish を同一プロセス・同一ユーザで
  連続実行する通常経路であり、seal 後に temp を外部から改竄
  する正当なユースケースも、改竄検出を要求する Issue・要件も
  見つからなかった（関連 Issue 群は強化選択時の受入側候補と
  して spec に記録済み）。
- **結論**: consumer 要件は現行契約の保証範囲内で完結しており、
  「強化」は要件を超える投資になる。

## 試行 2: 現行契約の pin 箇所と信頼境界宣言の確認

- **目的**: 「publish は post-seal temp を再検証しない」が
  意図的契約か事故的挙動かを、pin 試験・doc・先行設計で
  切り分ける。
- **観測**: `crates/ruviform-io-e2e/tests/staged_lifecycle.rs`
  の `post_seal_temp_rewrite_is_not_reverified_before_publish`
  は rewrite → 改竄 bytes が publish されることを、
  `post_seal_temp_delete_fails_publish_not_published` は delete
  → `NotPublished` を実 FS で assert（#306 Slice T / #603
  slice T で固定）。`docs/designs/issue-306-io-e2e-cases.md`
  Case 1/Case 2 は「seal 時点保証の境界を契約として固定する
  もので、io-file の動作変更を意図しない」「将来の再検証実装時
  に契約見直しを強制する」と明記。`crates/ruviform-io-file/
  README.md` §信頼境界は「同じ境界内の悪意ある差替えまでは
  防衛しない」「Temp identity … は所有権・ロックの証明では
  ない」を宣言済みだが、post-seal 再検証なしを直接述べる項目は
  無かった。
- **結論**: 境界は既に「正直に pin」された意図的契約。残る
  課題は判断本体の不在と README の読み取りやすさのみ。

## 試行 3: 強化案 3 件の評価

- **目的**: Issue が要求する「維持/強化」の判断として、
  実現可能な強化案を漏れなく評価する。
- **観測**:
  - seal 時 read-only 化・owner-only ACL: 同一信頼境界内の
    攻撃者（同一ユーザ）は権限変更・削除再作成が可能で実効的な
    境界を生まない。Windows（readonly 属性/ACL・share-lock）と
    POSIX（mode bit）で仕組みが異なり OS 別検証コストが増す。
  - publish 直前の内容再検証（seal 時ハッシュ→publish で再計算）:
    publish 毎に temp 全体の追加 read が必要で hot path コストに
    なる。再検証〜rename 間の TOCTOU は残り、完全な保証には
    同一 inode 検証等の更なる複雑さが要る。
  - temp 配置分離: staged publish の前提は temp と宛先が同一
    filesystem 上（atomic rename 成立条件）であり、宛先 dir が
    信頼境界である以上、別 dir に置いても境界は同じ。
- **結論**: いずれも consumer 要件を超えるコストに対し実効的な
  境界を生まない。「実装しない」ではなく Deferred（spec/design
  の Deferred Design Concerns）として再検討トリガー付きで
  記録する形を採用。

## 試行 4: 判断記録の置き場（design doc vs ADR）

- **目的**: Issue が許可する `docs/adr/`・`docs/designs/` の
  どちらに判断を記録するか決める。
- **観測**: ADR status は `accepted` であり、plan-doc の design
  gate（`--design` 対象に `status: "approved"` が必要）を
  満たさない。`docs/designs/issue-306-io-e2e-cases.md`・
  `docs/designs/issue-650-bench-smoke-coverage.md` の先例も
  design doc。本 slice は「設計判断 + doc 補強 + 検証」の
  設計文書型作業であり MADR の architecture decision 記法より
  design doc が整合する。
- **結論**: `docs/designs/issue-721-post-seal-temp.md` を採用。

## 試行 5: bench-smoke 配線の実測照合（残件 a）

- **目的**: `mise run bench-smoke` が `ruviform-io-e2e`
  `io_composition` を計測なし 1 回実行することを実測確認し、
  dev-env doc との drift を照合する。
- **観測**: Windows x86_64・rustc 1.99.1 で exit 0。
  `io_composition` の 6 case（`staged_write`・`staged_seal`・
  `staged_publish`・`staged_readback`・`tar_gzip_staged`・
  `tar_zstd_staged`）が全て `Testing ... Success`。抜粋は
  `evidence/rf721/bench-smoke-io-composition-excerpt.txt`。
  `docs/development-environment.md` L20/L21 の bench 系記述は
  `mise.toml` `[tasks.bench]`/`[tasks.bench-smoke]` と `-p`
  列挙・説明とも一致（drift なし）。同ファイルは
  `scarecrow173/ruviform#720` 占有のため read-only 照合に留めた。
- **結論**: 残件 (a) は配線済み・実測確認済みとして確定。

## 残リスク

- 「seal = publish までの完全性保証」との誤読 — README 補強と
  設計文書で緩和済み。`post_seal_temp_*` 試験名の
  `not_reverified` が機械的検出点として残る。
- publish 時再検証の将来要件 — Deferred に再検討トリガーを
  記録済み（consumer が post-seal 改竄を脅威モデルに含める
  要件を出した場合）。
- bench-smoke `-p` 列挙の将来 drift — #650 と同じ構造的リスク、
  本 slice は実測確認のみ。
- 実 FS fault の OS 別挙動差 — headless lane Pass を実 OS 受入に
  昇格しない（POLICIES.md §4）。本 slice は実装変更なしのため
  新規の OS 別差は発生しない。

## 検証証拠

- `evidence/rf721/bench-smoke-io-composition-excerpt.txt` —
  bench-smoke 実行抜粋（環境・6 case Success）。
- `evidence/rf721/verification.md` — 検証 lane の実測記録。
- canonical 検証: `mise run` 各 lane の exit code は
  `docs/impl/ir/issue-721-post-seal-temp.md` Validation と
  `evidence/rf721/verification.md` に記録。
- 補助証拠（canonical = mise）: cargo 直接実行は未使用。
- NotRun: `*-msrv`・`*-full`・3 OS 横断（Daily Full CI 委譲）、
  実 FS fault の OS 別挙動差（headless Pass を実 OS 受入に
  昇格しない方針）。
- Unsupported: なし。
