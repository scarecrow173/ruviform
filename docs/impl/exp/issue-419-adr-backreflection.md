# Issue #419 後継ADR verdict逆反映 — verdict 転記・語彙適用・文書規約試行記録（2026-11-15）

`docs/tasks/issue-419-adr-backreflection.md` の docs-only slice で試行した手法と結果の記録。コード変更を伴わないため、実験は「決裁済み verdict の ADR への転記方法・判定語彙の適用・文書規約の適用」に限定される。正本の判定は `evidence/rf262-gpui-verdict/verdict-matrix.md`・`report.md`・`handoff-419.md`、契約は `docs/designs/issue-419-adr-backreflection.md`・`docs/test-specs/issue-419-adr-backreflection.md` を参照。

## 試行 1: 「推奨 verdict」と「決裁済み outcome」の転記分離

- **目的**: `handoff-419.md`/`verdict-matrix.md` が verdict を「推奨」と記す一方、Coordinator は outcome = CONDITIONAL を決裁済み。本 slice が ADR に書くべき表現を決める。
- **試行**: handoff の「#419 への引き渡し」節（「L1 のみ評価済み、L2/L3 非保証」・条件付き判定材料・OS 別 Capability 記録・出典）と verdict-matrix 末尾の Verdict 規則適用を照合し、「決裁は Coordinator・本欄はその結果の記録」という枠組みで転記する文面を設計（design D1/D2）。
- **観測**: handoff は「#419 を accepted にしない」条件を明示し、ADR 側の受け皿（Confirmation の逆反映義務・More Information の未記入欄）と一致する。Worker が採否を再判定する余地はなく、転記は機械的に可能。
- **結論**: ADR 逆反映欄に「判定の決裁は Coordinator が行い、本欄はその結果の記録である」と明記した上で outcome = CONDITIONAL を記録。spec/design/test-spec/IR で「推奨→決裁」の読み替えを禁止事項として固定。

## 試行 2: 判定語彙の ADR 記述への写像

- **目的**: verdict-matrix の Case 別判定（Gate A〜E）を、ADR の集約記述へどう圧縮するか決める。
- **試行**: matrix の「推奨 verdict」表（領域別の判定と要点）を ADR の逆反映欄のサブ箇条書き構造（評価範囲の限定・採用範囲・OS 別 Capability・既知制約）へ写像。Case 表の複写はせず集約結果のみ記録。
- **観測**: OS-L1 Pass（ancestor SHA CI 証拠）・Windows L2 部分実証・Windows L3/Gate D 未達・macOS/Linux L2/L3 Deferred・Gate C 性能/Gate E NotRun の区分が matrix と一致。`NotRun`/`Deferred`/`Unsupported`/`Fail` の `Pass` 化なし。
- **結論**: handoff-419.md の 3 要件（明示すべきこと 1〜3）+ 出典リンクを ADR 記述へそのまま写像。個別 CaseID は `verdict-matrix.md` を正本として参照させ、ADR には詳細を複写しない。

## 試行 3: 宣言 Capability と実測 Pass の分離記述

- **目的**: `native_surface_slot`/`ime`/focus release の宣言値を ADR に記録する際、実測 Pass との混同を避ける表現を決める。
- **試行**: report.md §1.4（`WindowCapabilities` 宣言値）と matrix GB-B4/B5・GD-D8 を照合。「宣言」と「実測」の語を明示的に分ける記述を設計。
- **観測**: `native_surface_slot` = Windows `Supported` / macOS・Linux `Unsupported`（`BACKEND_AVAILABLE = cfg!(windows)`）、`ime` = 全 OS `Unsupported`、focus release = `Err(Unsupported)` — いずれも宣言値であり、Windows L2 の部分実証（実測 Pass）や Gate D 画素 Fail とは別軸。
- **結論**: ADR 逆反映欄に「OS 別 Capability 記録（宣言値。宣言と実測 Pass を混同しない）」として明記。Capability Matrix 宣言テンプレート表は原則不変とし、宣言記録は逆反映欄へ集約。

## 試行 4: `docs/impl/exp` の `.md` 記録と管理 index の関係

- **目的**: `.md` 自由形式の exp 記録（issue-262 先例）を追加した際、`docs/impl/exp/README.md` をどう扱うか決める。
- **試行**: `sync_impl_index.js`/`audit_experiment_log.js` の対象範囲を確認（`.jsonl` のみが監査・index 対象）。issue-262 PR（d32640eb）の `docs/impl/exp/README.md` 差分を確認 — hand-curated「Reports and raw data」節（生成領域 `<!-- doc-suite:generated-index -->` の外）に `.md` 記録の 1 行を追加していた。
- **観測**: 生成 index は `.jsonl` のみを列挙し、`.md` 記録は curated 節で管理される。curated 節は生成領域外の hand-written 領域であり、`sync_impl_index` は byte-for-byte で保持する。
- **結論**: issue-262 先例どおり、`.md` exp 記録を curated 節に 1 行追加し、生成領域は `sync_impl_index.js --kind exp` で再生成。PR 本文にこの扱いを明記する。

## 試行 5: 文書チェーンの front matter・relation 規約適用

- **目的**: #262 等の先例と同じ規約で #419 の文書チェーン（spec/design/test-spec/plan/task/IR/exp）を作成する。
- **試行**: `new_spec.js`/`new_design.js`/`new_test_spec.js`/`new_plan.js`/`new_task.js`/`new_impl_record.js` で skeleton を生成し `<PREFIX>-<22-char Base62>` ID を採番。`SPEC-`/`DESIGN-`/`TSPEC-`/`PLAN-`/`TASK-`/`IMPL-` の relation（`implements`/`depends-on`/`derives-from`/`verified-by`/`references`/`changes`）を issue-262 チェーンの構造に倣って記述。IR は `metadata.experiments.adopted` で exp を参照。
- **観測**: status は spec=proposed・design=approved・test-spec=proposed・plan=completed・task=done・ir=completed。`docs/impl/exp` は `.md` 自由形式記録を許容（#262・#553 等の先例）。`new_impl_record.js` は `--name` 非対応のため title slug で生成後に `issue-419-adr-backreflection.md` へ rename。
- **結論**: 文書チェーンを作成。`doc-maintenance index apply`/`sync_impl_index`/`docs/adr` index 手順で管理 index を再生成し `in-sync` を確認。
