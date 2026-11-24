# ruviform#958 検証記録 — weak-identity FS での foreign-object refusal 修正

実施日: 2026-11-25 / worktree: `rf958-weakfs` /
branch: `scarecrow173/rf958-weakfs`。
host OS: Windows + Docker Desktop（Docker Engine 29.9.2、linux engine）。
コンテナ内 OS: Ubuntu 24.04.5 LTS。base SHA `c7af983b` + 最終検証前に
`origin/main`（`102a972a` まで、Issue #912 系・io-archive 無関係）を取り込み済み。

対象 Issue: <https://github.com/scarecrow173/ruviform/issues/958>
一次記録（発見元）: `evidence/rf953/verification.md`（read のみ・変更なし）。

## 採用した設計（強化方針の確定）

**「一致した identity が世代を持つ file handle を含む場合のみ同一性の根拠とし、
それ以外では削除前に二次検証を必須とする」** — io-archive 内完結の修正として確定。

### 判定根拠

- `VfsProbe.strength` は「この FS が recreate 判別チャネルを報告するか」を示すが、
  その判別力（timestamp granularity）は io-archive 側から検査できない。
  docker lane 実測で、btime を返す `Strong` 環境でも delete+recreate が
  同一 `NodeIdentity`（inode 再利用 + 同一 discriminator tick）にエイリアス
  することを確認した（後述「docker lane での中間 regression」）。よって
  `strength == Strong` を gate に使う設計は不可。
- `NodeIdentity.handle`（unix `name_to_handle_at` / generation-bearing file
  handle）は構造上 recreate を判別する（generation が変わるため偽装不能）。
  一致した identity が handle を含むなら、等価比較だけで同一性が証明できる。
- handle を持たない一致（Weak FS 全般 + btime のみの Strong FS + Windows —
  Windows は常に `handle: None` で file-id 128bit が `index` に入る）では、
  型別の二次検証で「session が作った物」と証明できた場合のみ削除する:
  - **file**: 検証済み no-follow `open_leaf`（`VfsDir::open_regular_leaf`）で
    再 probe（identity 再照合 + `kind == File` + `byte_len` 一致）し、
    session が書き込んだ `(byte_len, content hash)` と全内容を照合。
    hash は `copy_payload` 側が書き込み済み分だけ逐次記録するため、
    部分書き込みの file も「書けた prefix と一致」すれば削除可能。
  - **dir**: no-follow `open_dir` + `list_names` で direct child を列挙し、
    全ての名が session の record にあるときのみ証明成立（deepest-first で
    record 済み child は先に消えているため、session の dir は空に見える）。
    foreign object を抱える dir は retained。
- **保持中 handle の pin 論証**: `run_cleanup` の root は session 開始から
  open し続けているため inode が pin され、parent 経由の等価 probe は
  強度に関わらず同一物体を指す（`remove_root` 内の再照合はこの pin に依存
  — 従来どおり等価比較のみで正当）。
- **recovery 経路**（`ExtractionRecoveryHandle::inspect` / `cleanup`）:
  reopen した root は pin が切れているため、等価 + handle 一致のみ信頼。
  handle なしでは「root 直下に未記録の名を持たない」構造チェックを要求
  （record-only の tree は再 cleanup 可能、foreign tree は Mismatch / refusal）。
- 祖先 dir の identity 照合は従来どおり等価のみ（下り止まりとしての
  防御）— エイリアスした foreign dir に潜っても、leaf の二次検証と
  dir の構造検証が foreign object の削除を個別に防ぐ。

### トレードオフ（正直な記録）

handle を持たない FS では、cleanup は記録済み file の内容を読み戻して
hash 照合する（安全側の対価として staged bytes 分の追加 read I/O）。
`weak_proof` は `byte_len` 不一致を先に拒否するため、サイズの異なる
foreign object は内容 read なしで retained になる。

## 変更範囲

`crates/ruviform-io-archive/` のみ（lease 内。`ruviform-io-file` 側の
`VfsProbe` / `VfsDir::open_regular_leaf` / `list` / `stat_self` /
`IdentityStrength` は既存 API を参照するのみ・変更なし）:

- `src/extract/fs.rs` — `StagingDir::identity`/`probe_identity` →
  `probe`/`probe_child`（`VfsProbe` を返す）へ拡張、`open_leaf`
  （no-follow 検証済み read-open）と `list_names` を追加、
  `StagingFile::identity` → `probe`。OS adapter は `VfsDir::stat_self`
  / `probe` / `open_regular_leaf` / `list` へ写像。mock を strength +
  content 付き node モデルへ再構成（weak/aliasing を注入可能）。
- `src/extract/session.rs` — `NodeRecord` に `ContentProof{byte_len,hash}`
  を追加、`copy_payload` が逐次 hash を記録、`cleanup_node` が
  handle なし一致で `weak_proof` を要求、`run_cleanup` が record ベースの
  children index を構築、recovery `inspect`/`cleanup` に reopen-root の
  構造チェック（`reopened_root_proven`）を追加。

## weak-identity 再現の検証方法

(a) mock seam に Weak/strength/content 注入の単体試験を追加、
(b) docker lane（`mise run verify-docker`）で実再現 — 両方を実施。
(c) の代替は不要だった。

追加した mock 単体試験（`session.rs` `#[cfg(test)]`）:

- `weak_fs_legitimate_cleanup_removes_the_tree` — weak FS でも正常系
  cleanup が機能する（二次検証で証明して削除）
- `weak_fs_aliased_foreign_file_is_retained` — 同一 identity・同一長・
  内容違いの foreign file が retained
- `reported_strong_but_aliased_foreign_file_is_retained` — 報告 `Strong`
  でも coarse-discriminator alias は retained（docker regression の固定化）
- `weak_fs_dir_holding_a_planted_child_is_retained` — 未記録 child を
  抱える dir は retained
- `weak_fs_recovery_inspect_rejects_an_unrecorded_root` — reopen root が
  未記録名を持つと `Mismatch` + `cleanup` 拒否
- `weak_fs_recovery_retry_still_cleans_a_legit_tree` — legit tree の
  recovery retry は引き続き成功

## docker lane での中間 regression（設計確定の根拠データ）

初回実装は `observed.strength == Strong → 等価のみで削除` を gate に
していた。`mise run verify-docker test ruviform-io-archive` で
`cleanup_refuses_to_delete_a_swapped_root_tree` と
`recovery_handle_reports_absent_and_cleanup_succeeds` が失敗
（`attempted:2 removed:2 retained:0` — foreign file が消去された）。
debug print で観測した record/observed は:

```
record   id={vol:118, idx:393484, File, discr:1795548002743577740, handle:None}
         strength=Strong  proof={byte_len:4, hash}
observed id={同一} kind=File len=Some(7) strength=Strong
```

この環境は statx btime を返す（`strength=Strong`、`discriminator`=btime）
にもかかわらず、delete+recreate された foreign file が inode と btime tick
を含む identity 全体にエイリアスしていた（btime granularity が粗いと
見られる）。「`Strong` が報告される = recreate が判別可能」とは限らない
ことが実測で確定し、現設計（handle のみ信頼 + 二次検証）へ修正した。

## 検証記録（全 lane）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run check ruviform-io-archive` | 0 | Windows host、`cargo check --all-targets` clean |
| `mise run lint ruviform-io-archive` | 0 | clippy `-D warnings` clean |
| `mise run test ruviform-io-archive` | 0 | Windows: lib 22 + extract_staging 12 + 他 test target 全 pass・doctest 10 pass。追加 weak-FS 試験 6 件含む |
| `mise run fmt-check` | 0 | `cargo fmt --all -- --check` clean |
| `mise exec -- prek run --all-files` | 0 | 全 hook Passed |
| `mise run check-linux` | 0 | x86_64-unknown-linux-gnu check + clippy clean |
| `mise run test ruviform-io-e2e` | 0 | 直接下流 — extraction 系含め全 pass（正常系 cleanup 回帰なし） |
| `mise run verify-docker test ruviform-io-archive` | 0 | docker 内 Ubuntu 24.04、`cargo test --locked -p ruviform-io-archive`。**`cleanup_retains_a_foreign_file_at_a_recorded_name`・`cleanup_refuses_to_delete_a_swapped_root_tree`・`recovery_handle_reports_absent_and_cleanup_succeeds` を含む 12/12 pass**（Issue 本文記載の failure を再現環境で解消。29.80s） |

Issue 受入条件との対応:

- AC1（修正または強化方針の確定）: 「一致 identity が handle を持つ場合
  以外は二次検証を必須とする」io-archive 内完結の修正として確定（本記録）。
- AC2（docker linux-verify で該当 2 test pass）: 上記 lane で
  `cleanup_retains_a_foreign_file_at_a_recorded_name` /
  `cleanup_refuses_to_delete_a_swapped_root_tree` とも pass。環境ゲートの
  NotRun/Unsupported への分離は不要になった。
- AC3（`mise run test ruviform-io-archive` が Windows/Linux pass）:
  Windows lane と docker lane（Linux）でそれぞれ全 pass。

## 補助証拠

- 上記 docker lane の中間 regression は debug print 付きの一時計測で
  観測（canonical は mise タスク。計測コードは除去済み・最終物に残さない）。
- `Cargo.toml` / `Cargo.lock` 差分なし（依存追加なし）。`docs/**` ・
  `docker/` ・ `evidence/rf953/` ・他 crate への変更なし。
- 設計正本 `docs/designs/issue-304-io-archive-extract.md` との関係:
  cleanup の foreign-object refusal が「identity 一致時の handle 有無 +
  二次検証」を見る契約は本 slice で追加した厳密化であり、doc 側の
  identity 契約記述との整合は Coordinator が統合時に整備する想定。

## 未対応・既知の残件

- handle を持たない FS では cleanup が記録済み file の内容を読み戻す
  （staged bytes 分の追加 read I/O）— 安全側動作の対価として受容。
  パフォーマンス懸念が問題になる場合は別 Issue での最適化検討が筋。
- `NodeRecord` は strength を保持しない（現設計では参照しないため）。
  `VfsProbe` の strength は trait surface で見えているため、将来 handle
  ベースの pin 経路等へ拡張する場合の材料は残っている。
