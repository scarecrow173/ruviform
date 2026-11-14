// RF-SCRIPT-02 / Issue #560 — Luau.Analysis FFI shim (C ABI).
//
// このヘッダは Rust 側 `src/ffi.rs` の extern "C" 宣言と 1:1 で対応する。
// C++ 型・Luau 型・例外はこの境界を越えない。文字列は UTF-8。
// shim が返す文字列は `rf_luau_free` でのみ解放すること。
#ifndef RF_LUAU_BRIDGE_H
#define RF_LUAU_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ABI version of this shim. Rust 側は初期化時にこの値を検査し、
 * 不一致なら fail-closed で Analyzer を利用不可とする。 */
#define RF_LUAU_BRIDGE_ABI_VERSION 1u

/* Script/Analyzer 契約（docs/designs/issue-558-script-contract-design.md C5）
 * の PoC ミラー版数。不一致は fail-closed。 */
#define RF_LUAU_BRIDGE_CONTRACT_VERSION 1u

/* shim 実装バージョン（semver 文字列） */
const char* rf_luau_bridge_impl_version(void);
/* 契約バージョン（数値） */
uint32_t rf_luau_bridge_contract_version(void);
/* リンクされた Luau release 文字列（例: "0.740"） */
const char* rf_luau_bridge_luau_version(void);

typedef struct RfLuauAnalyzer RfLuauAnalyzer;

typedef struct {
    uint32_t struct_size; /* sizeof(RfLuauCreateOptions) — 前方互換用 */
    uint32_t abi_version; /* 呼び出し側が要求する ABI version */
    uint32_t contract_version; /* 呼び出し側が要求する契約 version */
    int32_t mode_strict;  /* 0 = nonstrict, 1 = strict */
    int32_t enable_completion; /* 0 で Completion 能力を無効化（FX-N-13 検証用） */
    uint32_t max_source_bytes; /* 0 = 無制限 */
    double module_time_limit_sec; /* 0 = 無制限（timeout fixture 用） */
    int32_t source_type_module; /* 0 = Script, 1 = Module */
} RfLuauCreateOptions;

/* ステータスコード（create / analyze / load_definition 共通） */
#define RF_LUAU_OK 0
#define RF_LUAU_ERR_INVALID_UTF8 1
#define RF_LUAU_ERR_CURSOR_RANGE 2
#define RF_LUAU_ERR_SOURCE_TOO_LARGE 3
#define RF_LUAU_ERR_THREAD_MISMATCH 4
#define RF_LUAU_ERR_COMPLETION_UNSUPPORTED 5
#define RF_LUAU_ERR_INTERNAL 6
#define RF_LUAU_ERR_INJECTED 7 /* fault injection fixture 専用 */
#define RF_LUAU_ERR_VERSION 8   /* ABI/契約 version 不一致（fail-closed） */

/* 生成。成功時は RF_LUAU_OK を返し out にハンドルを返す。
 * 失敗時はステータスコードを返し、out_error に理由
 * （`rf_luau_free` 対象）を返す。 */
int32_t rf_luau_analyzer_create(const RfLuauCreateOptions* options,
                                RfLuauAnalyzer** out,
                                char** out_error);
void rf_luau_analyzer_destroy(RfLuauAnalyzer* analyzer);

/* 型定義（.d.luau 相当）をグローバル環境へ追加する。
 * 戻り値: 0 = 成功、非 0 = 失敗（out_error に理由）。 */
int32_t rf_luau_analyzer_load_definition(RfLuauAnalyzer* analyzer,
                                         const char* package_name,
                                         const char* definition_source,
                                         char** out_error);

/* 解析実行。source は UTF-8 バイト列（NUL 終端を要求しない）。
 * cursor_offset は UTF-8 バイトオフセット、負値で Completion を要求しない。
 * 成功時は RF_LUAU_OK を返し json_out に JSON 文字列
 * （`rf_luau_free` 対象）を返す。失敗時は上記コードを返し、
 * out_error に理由（`rf_luau_free` 対象）を返す。 */
int32_t rf_luau_analyzer_analyze(RfLuauAnalyzer* analyzer,
                                 const char* source,
                                 size_t source_len,
                                 int64_t cursor_offset,
                                 char** json_out,
                                 char** out_error);

/* shim が確保した文字列の解放 */
void rf_luau_free(void* ptr);

/* Fault injection（PoC 検証用シーム）:
 * 環境変数 RF_FFI_POC_FAIL_INIT=1 で create を失敗させる。
 * 環境変数 RF_FFI_POC_FAIL_ANALYZE=1 で analyze を明示エラーにする。
 * これらは PoC の fault fixture 専用であり製品経路では使わない。 */

#ifdef __cplusplus
}
#endif

#endif /* RF_LUAU_BRIDGE_H */
