---
id: "DESIGN-I8qbKRPJ0aQEJcRId4202T"
type: "design"
status: "approved"
title: "Issue #294 consumer oracle lanes 設計 — probe/run/record 3 分岐・rec() spec 拡張・fixture_version 記録値"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-294-consumer-oracle-lanes.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
    - "docs/designs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-294-consumer-oracle-lanes.md"
    - "docs/impl/ir/issue-294-virtual-first-evidence-schema.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-consumer-oracle-lanes.md"
  references:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "ci/consumer-verify/evidence.mjs"
    - ".github/workflows/consumer-verify.yml"
  defers: []
  deferred-by: []
---

# Issue #294 consumer oracle lanes 設計

## 概要

両 verify job の rust test lane 群の後・UI lane 群の前に、consumer oracle lane の **probe step + run step(s)** を追加し、Record step に file 粒度の lane 行（AL 1 file・SV 2 file、各 3 分岐）を追加する。`rec()` helper の case spec を additive 拡張して `oracle`/`fixture_version` を記録する。`evidence.mjs`・`ci/daily`・`ci/tracker`・他 workflow・consumer repo は変更しない。

## probe step（`id: oracle_probe`）

consumer checkout（`$CONSUMER_DIR`）内の canonical test file 存在を file 単位で判定する。file 存在確認のみなので consumer checkout（resolution 後）以降ならどこに置いてもよいが、run step の直前に置いて読みやすくする。

```yaml
- name: rf628 oracle test probe (consumer checkout)
  id: oracle_probe
  if: steps.gate.outputs.gate == 'run' && steps.resolution.outputs.tuple_ok == 'true'   # SV は && steps.presence.outputs.presence == 'present' を併記
  run: |
    set -euo pipefail
    oracle=0
    [ -f "$CONSUMER_DIR/crates/al-analysis/tests/rf628_oracle.rs" ] && oracle=1   # SV は synvelis-recording/tests/rf628_oracle.rs → oracle、rf628_oracle_rt_alloc.rs → oracle_rt
    echo "oracle=$oracle" >> "$GITHUB_OUTPUT"
```

- probe は `if` に `fetch`/`check`/`test` の outcome を含めない — file 存在判定は依存解決と独立。run step 側で `fetch` 成功を要求する。
- canonical path の完全一致で判定する。`crates/*/tests/rf628_oracle*.rs` glob での緩い検出は採用しない — 実行は `mise run test <pkg> --test <file>` に package 確定で pin するため、crate 配下の予期しない移動は `0`（canonical 実行不可）として owner 付き `not_run` で正直扱いする。

## run step(s)（`id: oracle` / `id: oracle_rt`）

```yaml
- name: rf628 oracle lane (al-analysis --test rf628_oracle)
  id: oracle
  if: steps.gate.outputs.gate == 'run' && steps.resolution.outputs.tuple_ok == 'true' \
      && steps.fetch.outcome == 'success' && steps.oracle_probe.outputs.oracle == '1'   # SV は presence=present も併記
  working-directory: ${{ runner.temp }}/audio-lens   # SV は synvelis
  run: |
    set -euo pipefail
    git lfs pull
    mise run test al-analysis --test rf628_oracle   # SV: mise run test synvelis-recording --test rf628_oracle / --test rf628_oracle_rt_alloc
```

- **`git lfs pull` を各 run step で実行**: clone は `GIT_LFS_SKIP_SMUDGE=1` で LFS 実体を取得していない。test-full lane と同じ materialize を行い、欠落 asset による虚偽 fail を防ぐ（2 回目以降は no-op）。
- **`fetch` 成功のみを依存にする**: `check`/`test` lane の成否に依存しない — oracle は独立判定であり、他 lane の fail で evidence が欠落してはならない（fetch 成功 = toolchain+依存解決済みが最小の正直な依存）。
- **file 粒度で別 step**: SV は `rf628_oracle`（PCM/Project/fixture oracle）と `rf628_oracle_rt_alloc`（RT no-alloc 計測）を別 step・別 lane 行にする。`--test` 併記の単一実行は coverage 同等だが file 別 verdict を正直に attribut できないため却下（`metadata.experiments.rejected` に記録）。
- consumer の `mise run test <pkg> [cargo flags]` は `cargo test --locked -p` + `raw_args` 透過（両 repo `mise.toml` `[tasks.test]` 実測）で、`--test <file>` がそのまま cargo へ届く。autotests 無効化なし・dev-dep `ruviform-audio-test` 宣言済みのため canonical path の file は test target として解決される。

## record step の lane 行（3 分岐・file 粒度）

probe output で 3 分岐（gui_candidate 先例と同型）:

1. `"1"` → `$(o '${{ steps.oracle[_rt].outcome }}')` — run step の実 outcome を正直に写す（skipped を含む — fetch 未完了で run step が skip された場合は `skipped` 行になる）。
2. `"0"` → `not_run` + owner 理由: 「consumer ref に oracle test 未同梱（`scarecrow173/audio-lens#127` / `scarecrow173/synvelis#115` Owner — oracle test を含む consumer ref で再実行。dispatch ref/別 crate 代用の偽 pass 禁止）」。
3. 上記以外（probe output 空 = probe 未完了・tuple 未確立等）→ `$(o '${{ steps.oracle_probe.outcome }}')` + 理由「oracle probe 未完了」。

全 oracle 行の case spec は `"${CONSUMER_NAME}-rf628-oracle[-rt-alloc]|$DEF|$RUVIFORM_REV|rf628_oracle[_rt_alloc]|$FIXTURE_VER"`（SV rt は case_id `rf628-oracle-rt-alloc`・oracle id `rf628_oracle_rt_alloc`）。分類は既定 `OS-L1|deterministic-backend|Required`（headless deterministic oracle lane — #627 §5 マトリクスの deterministic-backend 行に対応）。

## `rec()` spec の additive 拡張

第 1 引数を `"<case_id>|<level>|<environment_kind>|<requirement>[|<resolved_sha>[|<oracle>[|<fixture_version>]]]"` へ拡張する:

```bash
rec() {
  local spec="$1"; shift
  local case_id level env_kind requirement rsha oracle fixture_ver
  IFS='|' read -r case_id level env_kind requirement rsha oracle fixture_ver <<< "$spec"
  local rf=() of=() ff=()
  [ -n "${rsha:-}" ] && rf=(--resolved-ruviform-sha "$rsha")
  [ -n "${oracle:-}" ] && of=(--oracle "$oracle")
  [ -n "${fixture_ver:-}" ] && ff=(--fixture-version "$fixture_ver")
  # ...既存の name/command/feature/os/outcome/[reason] 引き渡しに "${of[@]}" "${ff[@]}" を追加
}
```

- 既存 5 field 呼出しは不変（field 6/7 が無ければ oracle/fixture flag は付かない）。`IFS='|' read` の追加変数で後方互換を保つ。
- `oracle`/`fixture_version` は field 6/7 の**両方が揃ったときだけ**意味を持つ — spec grammar でもペア必須とする（CASE_SPEC_RE で pin）。
- 代替案（oracle 行のみ `node "$EV" lane` 直接呼出し）は optional flag を各分岐で手書きすることになり spec/flag 対応が分散するため却下。

## `fixture_version` の記録値

Record step 内で `$CONSUMER_DIR/Cargo.lock` から `ruviform-audio-test` package の `version` を抽出し、`ruviform-audio-test-<version>@<RUVIFORM_REV>` 形式で記録する:

```bash
FIXTURE_VER=""
audio_test_ver=$(grep -A3 '^name = "ruviform-audio-test"$' "$CONSUMER_DIR/Cargo.lock" 2>/dev/null \
  | grep -m1 '^version = ' | sed -E 's/^version = "([^"]+)".*/\1/' || true)
[ -n "$audio_test_ver" ] && FIXTURE_VER="ruviform-audio-test-${audio_test_ver}@${RUVIFORM_REV}"
```

- Cargo.lock の `[[package]]` 内で `name` 直後に `version` が来る形式に依存（`grep -A3` + `grep -m1`）。`name = "ruviform-audio-test"` は `[[package]]` ブロックにのみ出現し依存リストの文字列とは区別できる。
- single-SHA guard（AL :388-398 / SV :859-869）が `scarecrow173/ruviform.git` を含む全 source を `?rev=$RUVIFORM_REV#$RUVIFORM_REV` へ強制済みのため、SHA 部は宣言値=解決値として正直記録できる。
- 抽出失敗（Cargo.lock なし・crate 不在・version 行欠落）は `FIXTURE_VER=""` → spec の field 7 が空 → `ff` flag を省略（null）。値を捏造しない。test file が無ければそもそも oracle lane は not_run/fail で記録されるため、fixture_version 単独の欠落は outcome と切り離される。
- `ruviform-audio-test-<version>@<sha>` 形式は「同一 fixture version」AC の比較可能性を満たす（両 consumer で同一 tuple なら同一値）。version+SHA の opaque hash 化は可読性・監査性で却下。

## `oracle` 識別子と `--expected`/`--observed`

- `oracle` field には test target 名を記録する（`rf628_oracle` / `rf628_oracle_rt_alloc`）— #627 §2「独立 Oracle の識別子」。consumer test file が `ruviform_audio_test::oracle::pcm_reference_block`（被験系と独立した再実装）を期待値生成に使うことを file header comment で確認済み。
- `--expected`/`--observed` は記録しない — consumer test は機械可読の期待値/観測値を出力せず、捏造は禁止。schema v2 は optional のため null のまま。

## contract test（`consumer-verify-yml-contract.test.mjs`）更新

- `CASE_SPEC_RE`（:380-381）: spec tail を `[|$RUVIFORM_REV[|<oracle>[|<fixture_version>]]]` へ拡張。oracle+fixture_version はペア必須（`|<oracle>|$FIXTURE_VER` の形のみ受理）。
- `rshaRows` assertion（:396-400）: 件数を job 別に更新（AL 9 = gui 6 + oracle 3、SV 12 = gui 6 + oracle 6）。slug 許容集合へ `-rf628-oracle`/`-rf628-oracle-rt-alloc` を追加。
- 新規 pin（推奨）: probe が canonical file path を検査すること、canonical command（`mise run test al-analysis --test rf628_oracle`・`mise run test synvelis-recording --test rf628_oracle`・`--test rf628_oracle_rt_alloc`）が存在すること、not_run 行が owner（`#127`/`#115`）を含むこと、record 行が spec 経由で `--oracle`/`--fixture-version`/`--resolved-ruviform-sha` 相当を持つこと、`rec()` が field 6/7 を flag へ写像すること。
- 「every recorded-capable step id maps to a lane row」（:127-137）: 新規 id は else 分岐の `steps.oracle_probe.outcome` と run 分岐の `o 'steps.oracle[_rt].outcome'` で充足 — 追加対応不要だが、新規 id が record に写像されることを新規 test で明示 pin する。
- `evidence.test.mjs`: optional field は schema v2 で検証済みのため原則変更不要。

## 行配置

- step: rust test lane 群の後・UI lane 群の前（AL: `test` lane の後・`sync_ui` の前。SV: `headless_deps` の後・`sync_ui` の前）。record 行も対応する lane 群の後・GUI contract ブロックの前に配置（semantic 影響なし・record step 内ならどこでもよいが可読性のため対応順に揃える）。
- workflow ヘッダ comment の Issue #294 拡張節に oracle lane の説明を 1 段落追記する。

## 判定分離の保持

- oracle lane の fail は当該 file の lane 行のみ `fail` — `check`/`test`/UI/GUI lane の outcome に影響しない（独立 step・独立行）。status 導出は既存 `deriveStatus` の fail-closed 規則に従う（`evidence.mjs` 変更不要）。
- `unsupported` 固定行・`not_run` 固定行（gui-consumer-side 等）は無変更。

## 限界・スコープ外

- per-case (per-test fn) の verdict 粒度は consumer repo 側 emitter の後続 slice — 本 slice は file/domain 粒度。
- 実 OS lane（#629/#630/#631・#313/#314）・consumer-side GUI fixture（AL #166/#127・SV #175-177）・`contract` v2 bump・post-merge dispatch 実走は対象外（spec の Deferred 表に記録）。
- `fixture_version` は `ruviform-audio-test` crate の version+SHA であり、fixture 内部の scenario/seed 版を識別しない — 再現性は resolved SHA pin で担保される範囲に留まる（正直記録）。
