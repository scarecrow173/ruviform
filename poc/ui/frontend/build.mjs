// フロントエンド全体ビルド + dist/ 集約スクリプト
// （rust-embed の #[folder="frontend/dist/"] が埋め込む単位）。
// - 各 npm workspace の `build` スクリプトを順に実行し、ワークスペース毎の
//   ビルド時間を計測する（AC5 記録項目「Vite ビルド時間」の単一源泉）。
//   `npm run build --workspace <pkg>` は tsc --noEmit + vite build を含む。
// - src/（host 実験ページ index.html）を dist/ ルートへコピー
// - 各 UI variant の dist（vanilla/dist 等）を dist/<variant>/ へコピーし、
//   バイト内訳・ビルド時間を記録した dist/<variant>/build-meta.json を生成
//   （Rust 側 exp/frontend.rs が埋め込みアセット経由で読み取るメタファイル）
// - `.ui-poc-built` マーカーを配置（Rust 側 `frontend_built()` の skip 判定根拠）
// variant dist が無い場合は「ビルド未完の dist」を残さないよう失敗させる。

import { cpSync, existsSync, mkdirSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, relative } from "node:path";
import { spawnSync } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));
const dist = join(here, "dist");

// ビルド対象ワークスペース（bench-shared を先に型検査して早期失敗させる順序）。
const WORKSPACES = [
  { dir: "bench-shared", pkg: "@ui-poc/bench-shared" },
  { dir: "vanilla", pkg: "@ui-poc/vanilla" },
  { dir: "ipc", pkg: "@ui-poc/ipc" },
  { dir: "solid", pkg: "@ui-poc/solid" },
];

// ベンチ UI variant。配信 URL は poc://app/<variant>/index.html。
const VARIANTS = ["vanilla", "ipc", "solid"];

// --- 1. ワークスペース別ビルド（計測つき）---
// Windows では `spawnSync("npm.cmd")` が Node ≥18.20 の spawn 制限（CVE-2024-27980
// 対策）で EINVAL になるため、npm 経由実行時は `npm_execpath`（npm CLI JS）を
// `process.execPath` で起動する。直に `node build.mjs` された場合は
// cmd.exe 経由の `npm` 呼出にフォールバックする（引数は全て固定値）。
const npmPrefix = process.env.npm_execpath
  ? [process.execPath, process.env.npm_execpath]
  : process.platform === "win32"
    ? ["cmd.exe", "/d", "/s", "/c", "npm"]
    : ["npm"];

const buildMs = {};
for (const ws of WORKSPACES) {
  const t0 = performance.now();
  const r = spawnSync(
    npmPrefix[0],
    [...npmPrefix.slice(1), "run", "build", "--workspace", ws.pkg, "--if-present"],
    { cwd: here, stdio: "inherit" }
  );
  buildMs[ws.pkg] = Math.round(performance.now() - t0);
  if (r.error) {
    console.error(`[ui-poc-frontend] workspace build spawn failed: ${ws.pkg} — ${r.error}`);
    process.exit(1);
  }
  if (r.status !== 0) {
    console.error(`[ui-poc-frontend] workspace build failed: ${ws.pkg}`);
    process.exit(r.status ?? 1);
  }
}

const missing = VARIANTS.filter(
  (v) => !existsSync(join(here, v, "dist", "index.html"))
);
if (missing.length > 0) {
  console.error(
    `[ui-poc-frontend] missing variant dist: ${missing.join(", ")} — workspace builds did not produce dist/index.html`
  );
  process.exit(1);
}

// --- 2. dist/ 集約 ---
rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });
cpSync(join(here, "src"), dist, { recursive: true });

// variant dist 配下のファイル一覧（相対パス → バイト数）。
function fileSizes(dir) {
  const out = {};
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.isFile()) {
        out[relative(dir, p).replaceAll("\\", "/")] = statSync(p).size;
      }
    }
  };
  walk(dir);
  return Object.fromEntries(Object.entries(out).sort(([a], [b]) => a.localeCompare(b)));
}

for (const v of VARIANTS) {
  const vdist = join(here, v, "dist");
  const files = fileSizes(vdist);
  const total = Object.values(files).reduce((a, b) => a + b, 0);
  cpSync(vdist, join(dist, v), { recursive: true });
  // variant 別メタ（AC5: Vite ビルド時間・dist 合計バイトのワークフロー側証跡）。
  // build-meta.json 自体は vite 産物ではない計測メタデータのため、
  // Rust 側の dist バイト集計では除外される（exp/frontend.rs と契約一致）。
  writeFileSync(
    join(dist, v, "build-meta.json"),
    JSON.stringify(
      {
        schema: "ui-poc-build-meta.v1",
        variant: v,
        built_at: new Date().toISOString(),
        build_ms: buildMs[`@ui-poc/${v}`],
        build_ms_scope:
          "npm run build --workspace @ui-poc/<variant>（tsc --noEmit + vite build）",
        bench_shared_build_ms: buildMs["@ui-poc/bench-shared"],
        total_bytes: total,
        files,
      },
      null,
      2
    ) + "\n",
    "utf8"
  );
}
writeFileSync(join(dist, ".ui-poc-built"), `${new Date().toISOString()}\n`, "utf8");
console.log(
  `[ui-poc-frontend] assembled dist/ (src/ + ${VARIANTS.map((v) => `${v}/dist`).join(" + ")})`
);
console.log(
  `[ui-poc-frontend] build_ms: ${WORKSPACES.map((w) => `${w.pkg}=${buildMs[w.pkg]}ms`).join(", ")}`
);
