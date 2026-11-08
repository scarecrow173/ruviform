---
id: "DESIGN-GKjcGBxFK0ZtzxCEKsh1e7"
type: "design"
status: "approved"
title: "Issue #314 slice 1 — headless Pixel Golden Utility コアの設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-314-golden-core.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/key.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/image.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/diff.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/store.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/report.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/golden/README.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-314-golden-core.md"
  depends-on:
    - "docs/designs/issue-390-driver-contract-lab.md"
  related:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/designs/issue-390-driver-contract-lab.md"
---

# Issue #314 slice 1 — headless Pixel Golden Utility コアの設計

## 概要

`ruviform-ui-test` に `golden` モジュールを新設し、pixel screenshot の
baseline 比較に必要な driver-neutral なコアを提供する。capture（pixel の
取得）は本 slice の対象外であり、本モジュールは「得られた RGBA 画像」と
「`proto::Screenshot` wire 型」の decode、比較、baseline 管理、証拠出力を
担う。全て非 RT・headless 検証可能で、実 OS・GPU・フォント環境に依存
しない。

モジュール構成:

```text
crates/ruviform-ui-test/src/golden.rs        — module root・re-export・GoldenVerdict/GoldenError
crates/ruviform-ui-test/src/golden/key.rs    — GoldenKey / EnvironmentMeta / GpuMeta / CapturePath
crates/ruviform-ui-test/src/golden/image.rs  — RgbaImage + PNG codec + Screenshot decode
crates/ruviform-ui-test/src/golden/diff.rs   — ChannelTolerance / MaskRect / DiffPolicy / diff
crates/ruviform-ui-test/src/golden/store.rs  — BaselineStore / Baseline（layout・load・candidate・promote）
crates/ruviform-ui-test/src/golden/report.rs — GoldenReport / GoldenEvidence / evidence append
```

`driver_boundary.rs` + `driver_boundary/sig.rs` と同じ「root + 同名 dir」
配置に倣う。

## D1: 所有者は `ruviform-ui-test` の `golden` モジュール

- Issue #314 の Owner 範囲（pixel 取得・環境正規化・baseline 比較・
  差分承認・証拠）のうち、本 slice が実装するのは比較以降の utility。
  `ruviform-ui-test` は RF-GT-03 の test 共有基盤であり、driver-neutral
  契約（#390）・観測 seam（#429/#436）の既存 owner でもある。
- `ruviform-ui`（共通契約）や `ruviform-ui-gpui`（GPUI adapter）には
  置かない — pixel 比較 utility は driver 非依存であり、toolkit 型を
  一切含まない。consumer repo にも置かない（2 Consumer 共有が目的）。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| `ruviform-ui` に golden API を置く | 共通契約層に test 専用の画像比較責務を混ぜる。`ruviform-ui` は production からも使われる契約層 |
| 新規クレート `ruviform-ui-golden` を切る | 単一 crate・単一 module で収まる規模に crate 分割は過剰（YAGNI）。test-only 層の既存 owner に置くのが自然 |
| `ruviform-ui-theme` に置く | theme golden は正規化 JSON で pixel ではない。責務が異なる（Issue 本文も混同を明示禁止） |

## D2: `GoldenKey`（同一性）と `EnvironmentMeta`（証拠）を分離

- `GoldenKey` は baseline を一意に指す**安定な同一性**であり、
  比較のたびに同じ値を再構成できなければならない。構成要素は
  Issue 指定の Story ID + Theme + State + Driver/OS/GPU class +
  DPI/font + Revision。
- `EnvironmentMeta` は capture 実行時に観測した**証拠**であり、
  os/arch/backend/gpu/scale/font/capture path/revision を持つ。
  取得不能な項目は `Option` の `None` — 「不明」を「既知の値」で
  埋めない。
- `GoldenKey` の env 側フィールド（`driver_class`/`os_class`/
  `gpu_class`/`scale_factor`/`font_class`）は `EnvironmentMeta` の
  **正規化 class** であり、生の値（GPU 名・OS version・font 列挙）
  ではない。class への写像は capture 層（slice 3）の責務で、本 slice
  は「class 文字列を受け取る」だけにする — 生値の丸め規則をここで
  捏造しない。
- `GoldenKey::canonical()` は evidence の subject に使う安定文字列、
  `file_stem()` は `[A-Za-z0-9._-]` 以外を `_` に置換した fs 安全
  文字列。`scale_factor` は `{:?}`（`2.0` → `"2.0"`）で決定的に
  format する。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| key に `EnvironmentMeta` 全体を埋め込む | GPU 名・OS version 等の揮発値で key が不安定になり baseline が再利用不能になる |
| key を hash のみにする | 人間が baseline path/evidence を読めない。可読な canonical 文字列を併記する |
| font/DPI を key から外す | Issue が明示する構成要素。font raster・DPI 差は pixel 差分の主要因であり、key で分離しないと baseline が環境差で腐る |

## D3: `RgbaImage` + `png`/`base64` 依存

- 内部表現は RGBA8 の `Vec<u8>`（`width*height*4`、検証済み）。
  `from_png` は `png` crate 0.18 の `Decoder` +
  `Transformations::normalize_to_color8()`（EXPAND|STRIP_16）で
  8bit Gray/RGB/GA/RGBA を受理し RGBA8 に正規化する。
  `Indexed`/16bit は変換で吸収、その他の色形式は型付きエラー。
- `to_png` は常に RGBA8 PNG を生成（決定的 — 同一入力は同一 byte
  列になることを test で固定しないが、encoder 設定は固定する）。
- `from_screenshot` は `proto::Screenshot` の `format` が
  `"png"` / `"rgba-raw"` のみ受理し、それ以外はエラー。
  `data_base64` は `base64` crate で decode する。
- decode 側は `png::Limits` + 明示の次元上限
  （`MAX_DIMENSION = 16384`、`MAX_PIXELS = 1<<27` ≈ 512MiB RGBA）で
  暴走入力を拒否する。これは test utility の防御的上限であり、
  正当な 8K screenshot（33M px）は収まる。

依存追加の記録（暫定規約「依存追加は理由を記録」）:

- `png = "0.18"`: baseline/diff/証拠 artifact の PNG codec。
  MIT OR Apache-2.0、pure Rust・no unsafe、rust-version 1.73
  （MSRV 1.89 適合）、既に lock tree 内（`image`→gpui 経由で
  0.18.1 が解決済み）のため推移依存の増加なし。`image` crate 全体
  （exr/gif/tiff/ravif 等の多形式依存）は PNG のみ必要な本用途に
  過剰なため不採用。
- `base64 = "0.22"`: `Screenshot.data_base64` の decode。
  rust-version が低く MSRV 安全側、0.22.1 は既に lock tree 内
  （usvg/zed-reqwest 経由）。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| `image` crate | 多形式依存（ravif/exr/tiff/zune 等）を test-only crate に引き込む。PNG のみで足りる |
| baseline を raw RGBA + JSON のみで保持 | diff/actual 画像を人間がレビューできない。baseline 更新承認には viewable な PNG が必須 |
| PNG codec を自前実装 | inflate/filter/Adam7 の再実装は検証コストが利益を大きく超える |
| `Screenshot` decode を slice 3 に先送り | decode は capture ではなく format 変換であり、slice 1 の store/compare を実 wire 型で end-to-end 検証できる。先送りすると `from_screenshot` の置き場が曖昧になる |

## D4: diff の意味論

- 画素比較は **per-channel 絶対差分**: 各 channel（r/g/b/a）について
  `|actual - expected| > tolerance[channel]` ならその画素は差分。
  `ChannelTolerance` は r/g/b/a 個別の `u8`。既定は全 0（厳密一致）。
- **alpha=0 正規化**: expected・actual 双方の alpha が 0 の画素は
  RGB が未定義になりうるため差分とみなさない（両方 alpha=0 → 一致）。
  片方だけ alpha=0 なら alpha channel 差分として評価する。
- **mask**: `MaskRect`（x/y/width/height、画素座標）は画像範囲に
  クリップし、内部の画素を比較対象から除外する。mask 内画素数は
  `masked_pixels` として統計に記録し、diff 画像では減光表示する。
- **判定**: mask 適用後の比較対象で差分画素が 0 → `Pass`、1 画素でも
  超過 → `Fail`。画素数・面積比の許容は v1 では持たない
  （閾値の追加は baseline policy の改訂として扱う — YAGNI）。
- **差分画像**: actual を基底に、差分画素を不透明赤 `(255,0,0,255)`、
  masked 画素を 50% 減光で描画。同サイズ RGBA8 を返し、
  `to_png` で artifact 化できる。
- サイズ不一致（width/height が異なる）は pixel 対応が取れないため
  diff を計算せず `Fail`（reason=`size_mismatch`）。
- 比較対象が 0 画素（全面 mask 等）は「何も検査していない」ため
  `Blocked`（reason=`empty_comparison`）— vacuous pass を出さない。

検討して退けた案:

| 案 | 退けた案の理由 |
| --- | --- |
| 単一閾値（全 channel 共通） | per-channel 許容は Issue 要件。alpha と色で別閾値を許す必要がある |
| diff 画素数/面積の許容率 | 実測に基づく閾値は backend 別 baseline の責務（Issue: 普遍閾値を捏造しない）。v1 は厳密 0 差分 |
| perceptual diff（ΔE 等） | Issue は per-channel 許容差を要求。知覚距離は別 policy として将来拡張可能な構造に留める |
| mask を楕円/任意 shape で | rect で v1 の要件（region mask）を満たす。複雑 shape は YAGNI |

## D5: baseline store の layout と承認境界

- root は呼び出し側が指定（`BaselineStore::new(root)`）。慣例配置は
  `<crate>/fixtures/golden/`（`fixtures/driver-lab/` と同じ層）。
- layout: `<root>/<story_id>/<theme>/<state>/<file_stem>.png` +
  `<file_stem>.json`。story/theme/state は path segment として
  sanitize し、残りの key 要素は `file_stem()` に畳み込む
  （例: `gpui-test-windows-dx11-sf2.0-nokeyfont-r<rev>`）。
- sidecar JSON（`rvf.golden.baseline/v1`）は `key`・`environment`・
  `policy`（DiffPolicy）・`image`（file 名・次元）を保持する。
  **policy を baseline に同梱する**のは、承認時の比較条件を
  baseline と一体で固定し、比較時に黙って緩められないようにするため。
- `load(key)` は PNG + sidecar を読み、sidecar の `key` が要求 key と
  一致することを検証する。不一致・sidecar 欠落・PNG 欠落は
  `GoldenError::Store`（壊れた store は比較不能 — verdict ではなく
  tool error）。
- **承認境界**: `write_candidate(dir, key, image, env, policy)` は
  candidate（PNG+sidecar）を任意 dir（慣例 `target/golden/`）に書く
  のみで baseline を触らない。`promote(dir, key)` が staged candidate
  を baseline path へコピーする明示操作であり、人間のレビュー
  （diff 画像の目視 + PR での fixture 差分承認）を経て初めて baseline
  が更新される。**auto-update env（`UPDATE_GOLDEN` 等）は提供しない**
  — baseline 更新の approval/audit 必須という Issue 契約を機械的に
  強制するため。
- `compare(key, actual)` は baseline の policy で diff し
  `GoldenReport` を返す。baseline 不在 → `Blocked`。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| `UPDATE_GOLDEN=1` 的な自動 bless | approval 必須の契約違反。diff を人間が見ない baseline 差し替えを許してしまう |
| baseline を単一巨大 JSON/PNG に集約 | story/theme/state ごとの差分レビューができない。file 分割が git diff と相性が良い |
| policy を呼び出し側が毎回指定 | 承認済み baseline と異なる条件で比較できてしまい audit が成立しない |
| sidecar なし（PNG + 命名規則のみ） | environment/policy の証拠が残らず、環境差による fail の原因追跡ができない |

## D6: 判定語彙と `GoldenReport`/evidence

- `GoldenVerdict` は driver-lab の `pass/fail/not_run/unsupported/
  blocked` と同じ 5 値（serde `snake_case`）。`#[non_exhaustive]`。
- `GoldenReport` は `key`・`verdict`・`reason`（機械可読な短い
  snake_case 文字列 + 補足）・`stats`（diff 時のみ）・
  `baseline` path・`artifacts`（書き出した diff 画像等）を持つ。
- `NotRun`/`Unsupported` は本 slice では「型として提供・capture 層が
  使用する」語彙。`compare` が返すのは `Pass`/`Fail`/`Blocked` のみ
  （pixel が存在しない状態は verdict ではなく呼び出し側の責務 —
  `Screenshot` decode 失敗は `GoldenError`）。
- evidence は `rvf.golden.evidence/v1` の JSONL 行
  （`key`/`environment`/`verdict`/`reason`/`stats`/`baseline`/
  `artifacts`/`policy`）。`RUVIFORM_GOLDEN_EVIDENCE` env 指定時のみ
  append（driver-lab と同じく相対 path は workspace root 解決、
  生成物は commit しない）。path 解決・append の共通部は
  `driver_lab` の `pub(crate)` helper に抽出して再利用する
  （同一責務の重複実装を避ける。公開 API は不変）。

検討して退けた案:

| 案 | 退けた理由 |
| --- | --- |
| driver-lab の `emit_evidence` をそのまま使う | 行 schema（manifest/subject/layer/expected/observed/mechanism/detail）が golden の key/env/stats を表現できない |
| verdict を `Pass/Fail` の 2 値にする | Issue が `Unsupported/NotRun` の記録を必須化。語彙を 5 値に揃えないと consumer-verify lane との写像で情報が落ちる |
| baseline 不在を `Fail` にする | 「差分を測定した結果 fail」ではなく「比較不能」。`Blocked` + reason の方が正直で、runner は非 Pass を fail として扱える |

## D7: semantic oracle との境界（文書で固定）

- pixel diff の `Pass` は「その環境・その画像が baseline と一致した」
  ことのみを意味する。f64 値・NaN/gap・PTS・LOD・RT safety・
  Receipt・Plugin 実描画の正しさは semantic oracle（Rust fixture /
  #464 契約 / 製品側検証）の責務であり、本 utility は参照先を
  文書で指すに留める。
- `GoldenKey.story_id` は driver-lab `stories.json` の story ID と
  同じ名前空間を想定するが、本 slice では registry への結合を
  行わない（slice 4）。key は単なる文字列であり、story の実在を
  検査しない — 存在検査は接続側の責務。

## slice 境界の明記

| slice | 内容 | 本 slice での状態 |
| --- | --- | --- |
| 1（本件） | Golden Utility コア: key/env-meta/画像 diff/baseline store/承認・audit 手順/結果語彙/evidence/合成画像 test | 実装 |
| 2 | `VisualGuiTest`/`VisualCapture` 能力契約の #464 接続 | 対象外 — 型・語彙のみ提供 |
| 3 | capture 経路: offscreen 実測・`TestIo::screenshot` 実 backend・実 Window capture 委譲 | 対象外 — `Unsupported`/`NotRun` 記録語彙のみ |
| 4 | #426 Story fixture 接続・AudioLens/Synvelis fixture への適用 | 対象外 |
