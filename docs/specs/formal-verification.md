---
id: "SPEC-rf806frmlvrf0slice1"
type: "spec"
status: "proposed"
title: "Issue #806 Formal Verification D 層 slice 1 — wire codec の proptest + Kani 検証基盤"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/806"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/tests/formal_ipc.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/formal_transport.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/formal_ipc.proptest-regressions"
      - type: "doc"
        path: "docs/specs/formal-verification.md"
      - type: "dir"
        path: "evidence/rf806/"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".agents/skills/verify-ruviform/SKILL.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  supersedes: []
  superseded-by: []
  related:
    - ".agents/skills/verify-ruviform/SKILL.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/806"
  derived-by: []
---

# Formal Verification — wire codec slice（rvf.ipc.v1 / rvf.uitest.v1）

Issue `scarecrow173/ruviform#806` slice 1 の実装 spec。D 層検証
（property-based testing → bounded model checking → 将来の TLA+/Lean
昇格）の第一弾として、unsafe を含まない wire codec を対象に
proptest と Kani の 2 層を常設する。

## 対象と検証する契約

| 対象 | ファイル | 契約 |
| --- | --- | --- |
| `parse_intent` | `crates/ruviform-ui/src/ipc.rs` | 任意入力で panic しない・`Err` は安定コード集合（`MESSAGE_TOO_LARGE` / `MALFORMED_MESSAGE`）に留まる・`Ok` なら envelope gate（`v=1`・`kind="intent"`・`valid_op`）を満たす・`MAX_INTENT_BYTES` 超過は `MESSAGE_TOO_LARGE` |
| `valid_op` | 同上 | 受理は「非空の `.` 区切り・各セグメント非空・`[a-z0-9._-]`・128 文字以下」のみ。全入力に対し total（panic なし）かつ sound（受理は仕様形状を満たす） |
| `Event::to_json` | 同上 | 常に parse 可能な `v=1` envelope を返し、variant kind と全フィールドを保持する |
| `read_frame` | `crates/ruviform-ui-test/src/transport.rs` | `Frame` / `Eof` / `TooLarge` / `Err(UnexpectedEof)` / `Err(InvalidData)` の tri-class が宣言長と残バイト数で一意に決まる。`TooLarge` は宣言本文を drain してストリーム整列を保つ |
| `write_frame` | 同上 | `MAX_INTENT_BYTES` 超過を `InvalidInput` で拒否。in-cap 本文は `u32le` 長 + UTF-8 で書き出し、`read_frame` で同一文字列が復元される |

## Lane 構成

| Lane | 実行 | 環境要件 |
| --- | --- | --- |
| `mise run test-formal` | `cargo test --locked -p ruviform-ui --test formal_ipc` + `cargo test --locked -p ruviform-ui-test --test formal_transport` | なし（pure Rust・MSRV 内・CI と同一入口） |
| `mise run test-formal-kani` | `cargo kani --tests -p ruviform-ui -p ruviform-ui-test` | ネイティブ `cargo kani`、または `cargo` + `cargo kani` を持つ WSL2 ディストロ |

`test-formal` は常時実行可能な第一層。`test-formal-kani` は
環境検出つきの第二層で、次の順に解決する。

1. ネイティブ `cargo kani` があればそれを使う。
2. なければ `wsl.exe -l -q` でディストロを列挙し、`cargo` と
   `cargo kani` の両方が PATH にあるものを先着で採用する。
3. カレントの Windows パスを `/mnt/<drive>/...` に写像し、WSL 側は
   `CARGO_TARGET_DIR=~/.cache/rf806-kani-target` の WSL ネイティブ
   領域で実行する（`/mnt` 越しのビルド IO を避ける）。
4. どちらも見つからなければ `not_run`（`environment_blocked`）を
   報告して exit 0 — 環境欠如は fail でも pass でもない。
   真の検証失敗（CBMC が反例を返した場合）のみ `fail` + exit 1。

## Kani の bound と仮定

証明は全入力ではなく bounded symbolic input に対する exhaustive
検証（CBMC の性質上、未証明領域は残る — 正直な bound 記録が必須）。

| Harness | Bound | 未証明領域 | 補完 |
| --- | --- | --- | --- |
| `parse_intent_error_contract` | envelope field（`v:u8`・`kind` 5 択・`id`/`op` 各 6 byte valid UTF-8・`params` 7 択 fragment）を固定 JSON skeleton 上で symbolic に全網羅。`#[kani::unwind(80)]` | 任意 malformed byte 列・1 MiB cap 経路（bound 内では構造的に到達不可） | malformed 入力は proptest `parse_intent_arbitrary_error_set`、cap 経路は `parse_intent_oversize_is_too_large`（1 MiB 実測）で担保 |
| `valid_op_soundness` | op = 8 byte 任意列のうち valid UTF-8。`#[kani::unwind(16)]` | 8 文字超の入力・`MAX_OP_LEN=128` をまたぐ受理/拒否 | proptest `valid_op_total_and_sound` / `valid_op_accepts_generated` で担保 |
| `event_to_json_wellformed` | id = 6 byte 任意列のうち valid UTF-8・`result_ok`・`params=Null`。`#[kani::unwind(80)]` | 他 variant・大きな params | proptest `event_to_json_versioned_envelope` が全 variant・任意 `Value` で担保 |
| `read_frame_contract` | stream = ≤32 byte 任意 byte 列。`#[kani::unwind(48)]` | 32 byte 超の stream・1 MiB 超 `declared` の drain 実走（symbolic u32 で `TooLarge` 判定自体は到達可） | proptest `read_frame_arbitrary_stream` / `too_large_drains_and_stays_aligned` で担保 |
| `write_read_roundtrip` | body = 16 byte 任意列のうち valid UTF-8。`#[kani::unwind(40)]` | in-cap だが 16 byte 超または無効 UTF-8 の body（`&str` 引数のため構造上到達不可） | proptest `write_read_roundtrip`（in-cap 任意・exact-cap 境界含む）で担保 |

仮定:

- `kani::assume(len <= STREAM_LEN)` で stream 長を bound 内に制限
  （それ以外は unconditional — 入力を絞る assume は契約を弱める
  ので置かない）。
- `#[kani::unwind(N)]` は library loop（UTF-8 検証・`Chars`・
  `Read`/`copy` 内部）の unwind 上限。bound 未指定では CBMC が
  symbolic 状態上の std loop を無制限 unwind して非実用的になる
  ことが実測で判明したため、入力サイズ＋余裕に設定する。証明は
  あくまでこの loop bound 内の主張であり、bound 超過は
  unwinding assertion として検出される。
- symbolic `&str` は `core::str::from_utf8(&bytes)` を `Ok` のときのみ
  使用する — allocation-free の固定長 byte 列から得るため
  `String` 生成の RawVec unwind コストを回避できる。`Ok` 以外を
  early return するのは入力型の制約であって契約弱化の `assume`
  ではない（`&str` 引数に invalid UTF-8 は型上到達不能）。
- serde_json の float 復号精度: `float_roundtrip` feature は無効の
  ため、最短表現 decimal が parse 時に 1 ulp ずれる場合がある。
  proptest 側は `json_value_eq`（整数は厳密・float は 1 ulp 以内）
  で「wire 上の意味的同一性」を検証する。これは codec 契約の
  外側にある serde_json 既知の精度限界であり、契約違反ではない。

## Kani 環境のプロビジョニング

`test-formal-kani` が `not_run`（`environment_blocked`）を返す環境
では、次のいずれかを用意する。

- Linux / macOS: `rustup` + `cargo install --locked kani-verifier` +
  `cargo kani setup`（CBMC 等の solver は `kani-verifier` の
  setup が取得する）。
- Windows: WSL2 ディストロ内で同上。lane は `wsl.exe -l -q` で
  列挙した各ディストロに `cargo` + `cargo kani` があるかを先着
  検出し、カレント Windows パスを `/mnt/<drive>/` へ写像して
  実行する。ビルド成果物は WSL ネイティブ側
  （`CARGO_TARGET_DIR=$HOME/.cache/ruviform-kani/target`）に置き、
  `/mnt` 越しの遅い IO と Windows 側 target の混線を避ける。

solver 取得はネットワーク依存のため、CI で Kani を必須化する場合は
イメージ焼き込み等の別途検討が必要（本 slice の scope 外 — lane は
manual/opt-in）。

## Mutation-fail 証跡（「fail できること」の証明）

プロパティ suite は緑であることだけでなく、壊れた実装を検出できる
ことが証拠として必要。手順:

1. 対象関数の契約を 1 箇所だけ一時的に破る（例: `valid_op` の
   セグメント非空チェックを外す、`parse_intent` の `v` gate を
   緩める、`read_frame` の drain を省略する）。
2. `mise run test-formal`（または対象 `--test` 単独）を実行し、
   該当 property が shrink つきで `fail` することを記録する。
3. 変異を完全に revert し、緑に戻ったことを再実行で確認する。

変異・観測した fail・revert の 3 点セットを `evidence/rf806/` に
記録する。変異の持ち越し（revert 漏れ）は禁止 — verification.md に
revert 後の再検証結果を残す。

## 判定語彙と fail-closed

`pass` / `fail` / `not_run` / `unsupported` / `blocked` — verify-
ruviform skill の語彙に従う。環境欠如は `not_run` +
`environment_blocked` で exit 0、証拠のない lane を `pass` と記録
しない。ローカル検証は変更範囲の検証であり、merge 後の対象 SHA
に対する必須 CI 結果が統合検証の証拠（本記録はそれを代替しない）。

## 昇格経路（本 slice 外）

- ordering / state machine 系（session・dock・close 協議）は
  TLA+ 候補（`scarecrow173/ruviform#389` 系）。
- 全サイズの数学的不変条件（DSP・算術）は Lean 4 候補。
- 本 slice で確立した「coverage 記録 + sync commit + 1 コマンド
  再実行 + mutation-fail 証跡」の型を、そのまま昇格先にも適用する。
