// RF-SCRIPT-02 / Issue #560 — Luau.Analysis FFI shim implementation.
//
// Luau.Analysis (vendored Luau 0.740, MIT) を C ABI 経由で Rust へ公開する
// 狭小 Bridge。C++ 型・Luau 型・例外はこの境界を越えず、結果は JSON 文字列
// として返す。unsafe 境界はこのファイルと Rust 側 ffi.rs に隔離する。

#include "rf_luau_bridge.h"

#if defined(_WIN32)
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#endif

#include "Luau/Autocomplete.h"
#include "Luau/BuiltinDefinitions.h"
#include "Luau/Config.h"
#include "Luau/ConfigResolver.h"
#include "Luau/Error.h"
#include "Luau/FileResolver.h"
#include "Luau/Frontend.h"
#include "Luau/GlobalTypes.h"
#include "Luau/ParseOptions.h"
#include "Luau/ToString.h"
#include "Luau/TypeArena.h"
#include "Luau/Type.h"

#include <cstdlib>
#include <cstring>
#include <memory>
#include <optional>
#include <string>
#include <thread>
#include <unordered_map>
#include <vector>

namespace
{

constexpr const char* kModuleName = "RFScriptPocMain";
constexpr const char* kLuauVersion = "0.740";

char* copyString(const std::string& value)
{
    char* out = new char[value.size() + 1];
    std::memcpy(out, value.data(), value.size());
    out[value.size()] = '\0';
    return out;
}

void appendJsonString(std::string& out, const std::string& value)
{
    out.push_back('"');
    for (unsigned char c : value)
    {
        switch (c)
        {
        case '"': out += "\\\""; break;
        case '\\': out += "\\\\"; break;
        case '\b': out += "\\b"; break;
        case '\f': out += "\\f"; break;
        case '\n': out += "\\n"; break;
        case '\r': out += "\\r"; break;
        case '\t': out += "\\t"; break;
        default:
            if (c < 0x20)
            {
                char buf[8];
                std::snprintf(buf, sizeof(buf), "\\u%04x", c);
                out += buf;
            }
            else
            {
                out.push_back(static_cast<char>(c));
            }
        }
    }
    out.push_back('"');
}

void appendJsonKey(std::string& out, const char* key)
{
    appendJsonString(out, key);
    out.push_back(':');
}

// UTF-8 妥当性検査。Luau の Lexer はバイト列を走査するが、契約上
// Invalid UTF-8 は明示 Error とするため shim 境界で検査する。
bool isValidUtf8(const char* data, size_t len)
{
    size_t i = 0;
    while (i < len)
    {
        unsigned char c = static_cast<unsigned char>(data[i]);
        if (c < 0x80)
        {
            ++i;
            continue;
        }
        int need;
        uint32_t cp;
        if ((c & 0xE0) == 0xC0)
        {
            need = 1;
            cp = c & 0x1F;
            if (cp < 2) return false; // overlong
        }
        else if ((c & 0xF0) == 0xE0)
        {
            need = 2;
            cp = c & 0x0F;
        }
        else if ((c & 0xF8) == 0xF0)
        {
            need = 3;
            cp = c & 0x07;
            if (cp > 4) return false;
        }
        else
        {
            return false;
        }
        if (i + need >= len) return false;
        for (int j = 1; j <= need; ++j)
        {
            unsigned char cc = static_cast<unsigned char>(data[i + j]);
            if ((cc & 0xC0) != 0x80) return false;
            cp = (cp << 6) | (cc & 0x3F);
        }
        // overlong / surrogate / range checks
        if ((need == 1 && cp < 0x80) || (need == 2 && cp < 0x800) ||
            (need == 3 && cp < 0x10000) || cp > 0x10FFFF ||
            (cp >= 0xD800 && cp <= 0xDFFF))
        {
            return false;
        }
        i += need + 1;
    }
    return true;
}

// バイトオフセット → Luau::Position（line は 0 始まり、column は行内バイト
// オフセット — Luau Lexer の Position(line, offset - lineOffset) と同じ
// 意味論）。改行は \n・\r\n・単独 \r を扱う。
Luau::Position offsetToPosition(const std::string& source, size_t offset)
{
    unsigned int line = 0;
    size_t lineStart = 0;
    size_t i = 0;
    while (i < offset && i < source.size())
    {
        char c = source[i];
        if (c == '\n')
        {
            ++line;
            lineStart = i + 1;
        }
        else if (c == '\r')
        {
            if (i + 1 < source.size() && source[i + 1] == '\n')
            {
                // \r\n: 'n' 側でカウントするのでここでは何もしない
            }
            else
            {
                ++line;
                lineStart = i + 1;
            }
        }
        ++i;
    }
    return Luau::Position{line, static_cast<unsigned int>(offset - lineStart)};
}

const char* autocompleteKindName(Luau::AutocompleteEntryKind kind)
{
    switch (kind)
    {
    case Luau::AutocompleteEntryKind::Property: return "property";
    case Luau::AutocompleteEntryKind::Binding: return "binding";
    case Luau::AutocompleteEntryKind::Keyword: return "keyword";
    case Luau::AutocompleteEntryKind::String: return "string";
    case Luau::AutocompleteEntryKind::Type: return "type";
    case Luau::AutocompleteEntryKind::Module: return "module";
    case Luau::AutocompleteEntryKind::GeneratedFunction: return "generated_function";
    case Luau::AutocompleteEntryKind::RequirePath: return "require_path";
    case Luau::AutocompleteEntryKind::HotComment: return "hot_comment";
    }
    return "unknown";
}

struct MemoryFileResolver : Luau::FileResolver
{
    std::unordered_map<std::string, Luau::SourceCode> sources;

    std::optional<Luau::SourceCode> readSource(const Luau::ModuleName& name) override
    {
        auto it = sources.find(name);
        if (it == sources.end())
            return std::nullopt;
        return it->second;
    }

    std::string getHumanReadableModuleName(const Luau::ModuleName& name) const override
    {
        return name;
    }
};

struct AnalyzerImpl
{
    MemoryFileResolver fileResolver;
    Luau::NullConfigResolver configResolver;
    Luau::Frontend frontend;

    bool enableCompletion = true;
    uint32_t maxSourceBytes = 0;
    Luau::SourceCode::Type sourceType = Luau::SourceCode::Script;
    std::thread::id ownerThread = std::this_thread::get_id();

    AnalyzerImpl(Luau::SolverMode solverMode, const RfLuauCreateOptions& opts)
        : frontend(solverMode, &fileResolver, &configResolver, makeFrontendOptions(opts))
    {
        configResolver.defaultConfig.mode = opts.mode_strict ? Luau::Mode::Strict : Luau::Mode::Nonstrict;

        Luau::registerBuiltinGlobals(frontend, frontend.globals);
        Luau::registerBuiltinGlobals(frontend, frontend.globalsForAutocomplete, true);
        Luau::freeze(frontend.globals.globalTypes);
        Luau::freeze(frontend.globalsForAutocomplete.globalTypes);
    }

    static Luau::FrontendOptions makeFrontendOptions(const RfLuauCreateOptions& opts)
    {
        Luau::FrontendOptions fopts;
        fopts.retainFullTypeGraphs = true;
        fopts.runLintChecks = false;
        if (opts.module_time_limit_sec > 0.0)
            fopts.moduleTimeLimitSec = opts.module_time_limit_sec;
        return fopts;
    }
};

bool envFlag(const char* name)
{
#if defined(_WIN32)
    // Windows では Rust の std::env::set_var/remove_var が更新するのは
    // Win32 の PEB env block(SetEnvironmentVariableW)であり、CRT の
    // _environ(std::getenv が読む起動時 snapshot)は同期されない。
    // fault injection fixture が in-process の env 変更を観測できるよう、
    // 同じ env block を読む Win32 API で読む。値が空/"0" なら無効。
    char buf[64];
    DWORD n = GetEnvironmentVariableA(name, buf, sizeof(buf));
    if (n == 0 || n >= sizeof(buf))
        return false;
    return buf[0] != '0';
#else
    const char* v = std::getenv(name);
    return v != nullptr && v[0] != '\0' && v[0] != '0';
#endif
}

void appendPosition(std::string& out, const Luau::Position& pos)
{
    out += "{\"line\":";
    out += std::to_string(pos.line);
    out += ",\"column\":";
    out += std::to_string(pos.column);
    out += "}";
}

void appendRange(std::string& out, const Luau::Location& loc)
{
    out += "{\"start\":";
    appendPosition(out, loc.begin);
    out += ",\"end\":";
    appendPosition(out, loc.end);
    out += "}";
}

void appendDiagnostics(std::string& out, AnalyzerImpl& analyzer, const Luau::CheckResult& result)
{
    out += "\"diagnostics\":[";
    bool first = true;
    for (const Luau::TypeError& error : result.errors)
    {
        if (!first) out += ",";
        first = false;

        out += "{\"severity\":\"error\",\"kind\":";
        if (const Luau::SyntaxError* syntaxError = Luau::get_if<Luau::SyntaxError>(&error.data))
        {
            out += "\"syntax\",\"code\":\"parse\",\"message\":";
            appendJsonString(out, syntaxError->message);
        }
        else
        {
            out += "\"type\",\"code\":";
            out += std::to_string(error.code());
            out += ",\"message\":";
            appendJsonString(out, Luau::toString(error, Luau::TypeErrorToStringOptions{&analyzer.fileResolver}));
        }
        out += ",\"module\":";
        appendJsonString(out, error.moduleName);
        out += ",\"range\":";
        appendRange(out, error.location);
        out += "}";
    }
    out += "]";
}

void appendCompletions(std::string& out, AnalyzerImpl& analyzer,
                       const std::string& source, size_t cursorOffset)
{
    Luau::FrontendOptions acOpts;
    acOpts.forAutocomplete = true;
    acOpts.retainFullTypeGraphs = true;
    acOpts.moduleTimeLimitSec = analyzer.frontend.options.moduleTimeLimitSec;

    analyzer.frontend.check(kModuleName, acOpts);

    Luau::Position pos = offsetToPosition(source, cursorOffset);
    Luau::AutocompleteResult ac = Luau::autocomplete(
        analyzer.frontend, kModuleName, pos,
        [](std::string, std::optional<const Luau::ExternType*>, std::optional<std::string>) -> std::optional<Luau::AutocompleteEntryMap>
        {
            return std::nullopt;
        }
    );

    out += "\"completions\":[";
    bool first = true;
    for (const auto& [label, entry] : ac.entryMap)
    {
        if (!first) out += ",";
        first = false;

        out += "{\"label\":";
        appendJsonString(out, label);
        out += ",\"kind\":";
        appendJsonString(out, autocompleteKindName(entry.kind));
        out += ",\"deprecated\":";
        out += entry.deprecated ? "true" : "false";

        out += ",\"detail\":";
        if (entry.type)
        {
            try
            {
                appendJsonString(out, Luau::toString(*entry.type));
            }
            catch (...)
            {
                out += "null";
            }
        }
        else if (entry.documentationSymbol)
        {
            appendJsonString(out, *entry.documentationSymbol);
        }
        else
        {
            out += "null";
        }

        out += ",\"insert_text\":";
        if (entry.insertText)
            appendJsonString(out, *entry.insertText);
        else
            out += "null";

        // replace_range: カーソル直前の識別子プレフィックスを対象とする
        // （[A-Za-z0-9_] の連続）。Completion の実置換範囲は Editor 側で
        // 調整する前提の PoC 簡易実装。
        size_t prefixStart = cursorOffset;
        while (prefixStart > 0)
        {
            unsigned char c = static_cast<unsigned char>(source[prefixStart - 1]);
            if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
                  (c >= '0' && c <= '9') || c == '_'))
            {
                break;
            }
            --prefixStart;
        }
        Luau::Position startPos = offsetToPosition(source, prefixStart);
        out += ",\"replace_range\":{\"start\":";
        appendPosition(out, startPos);
        out += ",\"end\":";
        appendPosition(out, pos);
        out += "}}";
    }
    out += "]";
    out += ",\"completion_context\":";
    appendJsonString(out, std::to_string(static_cast<int>(ac.context)));
}

} // namespace

extern "C" const char* rf_luau_bridge_impl_version(void)
{
    return "ruviform-ffi-poc/0.1.0";
}

extern "C" uint32_t rf_luau_bridge_contract_version(void)
{
    return RF_LUAU_BRIDGE_CONTRACT_VERSION;
}

extern "C" const char* rf_luau_bridge_luau_version(void)
{
    return kLuauVersion;
}

extern "C" int32_t rf_luau_analyzer_create(const RfLuauCreateOptions* options,
                                          RfLuauAnalyzer** out,
                                          char** out_error)
{
    if (out_error) *out_error = nullptr;
    if (!options || !out)
    {
        if (out_error) *out_error = copyString("null arguments");
        return RF_LUAU_ERR_INTERNAL;
    }
    if (options->struct_size < sizeof(RfLuauCreateOptions))
    {
        if (out_error) *out_error = copyString("options struct_size too small");
        return RF_LUAU_ERR_INTERNAL;
    }
    if (options->abi_version != RF_LUAU_BRIDGE_ABI_VERSION ||
        options->contract_version != RF_LUAU_BRIDGE_CONTRACT_VERSION)
    {
        if (out_error)
        {
            *out_error = copyString("version mismatch: expected abi=" +
                std::to_string(RF_LUAU_BRIDGE_ABI_VERSION) + " contract=" +
                std::to_string(RF_LUAU_BRIDGE_CONTRACT_VERSION));
        }
        return RF_LUAU_ERR_VERSION;
    }
    if (envFlag("RF_FFI_POC_FAIL_INIT"))
    {
        if (out_error) *out_error = copyString("injected init failure");
        return RF_LUAU_ERR_INJECTED;
    }

    try
    {
        Luau::SolverMode solverMode = Luau::SolverMode::New;
        auto impl = std::make_unique<AnalyzerImpl>(solverMode, *options);
        impl->enableCompletion = options->enable_completion != 0;
        impl->maxSourceBytes = options->max_source_bytes;
        impl->sourceType = options->source_type_module ? Luau::SourceCode::Module : Luau::SourceCode::Script;
        *out = reinterpret_cast<RfLuauAnalyzer*>(impl.release());
        return RF_LUAU_OK;
    }
    catch (const std::exception& e)
    {
        if (out_error) *out_error = copyString(std::string("create failed: ") + e.what());
        return RF_LUAU_ERR_INTERNAL;
    }
    catch (...)
    {
        if (out_error) *out_error = copyString("create failed: unknown native error");
        return RF_LUAU_ERR_INTERNAL;
    }
}

extern "C" void rf_luau_analyzer_destroy(RfLuauAnalyzer* analyzer)
{
    if (!analyzer) return;
    delete reinterpret_cast<AnalyzerImpl*>(analyzer);
}

extern "C" int32_t rf_luau_analyzer_load_definition(RfLuauAnalyzer* analyzer,
                                                   const char* package_name,
                                                   const char* definition_source,
                                                   char** out_error)
{
    if (out_error) *out_error = nullptr;
    if (!analyzer || !package_name || !definition_source)
    {
        if (out_error) *out_error = copyString("null arguments");
        return RF_LUAU_ERR_INTERNAL;
    }
    AnalyzerImpl* impl = reinterpret_cast<AnalyzerImpl*>(analyzer);
    if (impl->ownerThread != std::this_thread::get_id())
    {
        if (out_error) *out_error = copyString("analyzer used from a different native thread");
        return RF_LUAU_ERR_THREAD_MISMATCH;
    }
    if (envFlag("RF_FFI_POC_FAIL_LOADDEF"))
    {
        if (out_error) *out_error = copyString("injected load_definition failure");
        return RF_LUAU_ERR_INJECTED;
    }

    try
    {
        std::string def(definition_source);
        if (!isValidUtf8(def.data(), def.size()))
        {
            if (out_error) *out_error = copyString("definition source is not valid UTF-8");
            return RF_LUAU_ERR_INVALID_UTF8;
        }

        // Fixture::loadDefinition と同じ手順: globals と
        // globalsForAutocomplete の両方へ登録し、Autocomplete 経路でも
        // 型定義が見えるようにする。
        for (int pass = 0; pass < 2; ++pass)
        {
            Luau::GlobalTypes& globals = pass == 0 ? impl->frontend.globals : impl->frontend.globalsForAutocomplete;
            bool forAutocomplete = pass == 1;
            Luau::unfreeze(globals.globalTypes);
            Luau::LoadDefinitionFileResult result = impl->frontend.loadDefinitionFile(
                globals, globals.globalScope, def, package_name,
                /* captureComments */ false, /* typeCheckForAutocomplete */ forAutocomplete);
            Luau::freeze(globals.globalTypes);
            if (!result.success)
            {
                if (out_error) *out_error = copyString("definition file rejected by Luau.Analysis");
                return RF_LUAU_ERR_INTERNAL;
            }
        }
        return RF_LUAU_OK;
    }
    catch (const std::exception& e)
    {
        if (out_error) *out_error = copyString(std::string("load_definition failed: ") + e.what());
        return RF_LUAU_ERR_INTERNAL;
    }
    catch (...)
    {
        if (out_error) *out_error = copyString("load_definition failed: unknown native error");
        return RF_LUAU_ERR_INTERNAL;
    }
}

extern "C" int32_t rf_luau_analyzer_analyze(RfLuauAnalyzer* analyzer,
                                           const char* source,
                                           size_t source_len,
                                           int64_t cursor_offset,
                                           char** json_out,
                                           char** out_error)
{
    if (json_out) *json_out = nullptr;
    if (out_error) *out_error = nullptr;
    if (!analyzer || !source)
    {
        if (out_error) *out_error = copyString("null arguments");
        return RF_LUAU_ERR_INTERNAL;
    }
    AnalyzerImpl* impl = reinterpret_cast<AnalyzerImpl*>(analyzer);
    if (impl->ownerThread != std::this_thread::get_id())
    {
        if (out_error) *out_error = copyString("analyzer used from a different native thread");
        return RF_LUAU_ERR_THREAD_MISMATCH;
    }
    if (envFlag("RF_FFI_POC_FAIL_ANALYZE"))
    {
        if (out_error) *out_error = copyString("injected analyze failure");
        return RF_LUAU_ERR_INJECTED;
    }
    if (cursor_offset >= 0 && impl->enableCompletion == false)
    {
        if (out_error) *out_error = copyString("completion capability disabled");
        return RF_LUAU_ERR_COMPLETION_UNSUPPORTED;
    }
    if (impl->maxSourceBytes != 0 && source_len > impl->maxSourceBytes)
    {
        if (out_error) *out_error = copyString("source exceeds max_source_bytes");
        return RF_LUAU_ERR_SOURCE_TOO_LARGE;
    }
    if (cursor_offset >= 0 && static_cast<uint64_t>(cursor_offset) > source_len)
    {
        if (out_error) *out_error = copyString("cursor offset out of range");
        return RF_LUAU_ERR_CURSOR_RANGE;
    }
    if (!isValidUtf8(source, source_len))
    {
        if (out_error) *out_error = copyString("source is not valid UTF-8");
        return RF_LUAU_ERR_INVALID_UTF8;
    }

    try
    {
        std::string src(source, source_len);
        impl->fileResolver.sources[kModuleName] = Luau::SourceCode{src, impl->sourceType};
        impl->frontend.markDirty(kModuleName);

        Luau::CheckResult result = impl->frontend.check(kModuleName);

        std::string json = "{";
        appendJsonKey(json, "ok"); json += "true,";
        appendJsonKey(json, "luau_version"); appendJsonString(json, kLuauVersion); json += ",";
        appendJsonKey(json, "source_bytes"); json += std::to_string(source_len); json += ",";
        appendDiagnostics(json, *impl, result);
        if (cursor_offset >= 0)
        {
            json += ",";
            appendCompletions(json, *impl, src, static_cast<size_t>(cursor_offset));
        }
        else
        {
            json += ",\"completions\":null";
        }
        json += "}";

        if (json_out) *json_out = copyString(json);
        return RF_LUAU_OK;
    }
    catch (const std::exception& e)
    {
        if (out_error) *out_error = copyString(std::string("analyze failed: ") + e.what());
        return RF_LUAU_ERR_INTERNAL;
    }
    catch (...)
    {
        if (out_error) *out_error = copyString("analyze failed: unknown native error");
        return RF_LUAU_ERR_INTERNAL;
    }
}

extern "C" void rf_luau_free(void* ptr)
{
    delete[] static_cast<char*>(ptr);
}
