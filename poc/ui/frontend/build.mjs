// dist/ 集約スクリプト（rust-embed の #[folder="frontend/dist/"] が埋め込む単位）。
// - src/（host 実験ページ index.html）を dist/ ルートへコピー
// - 各 UI variant の dist（vanilla/dist 等）を dist/<variant>/ へコピー
// - `.ui-poc-built` マーカーを配置（Rust 側 `frontend_built()` の skip 判定根拠）
// workspaces 各 dist は `npm run build --workspaces` で先に生成される前提。
// variant dist が無い場合は「ビルド未完の dist」を残さないよう失敗させる。

import { cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const dist = join(here, "dist");

// ベンチ UI variant（T5 で solid を追加）。配信 URL は poc://app/<variant>/index.html。
const VARIANTS = ["vanilla"];

const missing = VARIANTS.filter((v) => !existsSync(join(here, v, "dist", "index.html")));
if (missing.length > 0) {
  console.error(
    `[ui-poc-frontend] missing variant dist: ${missing.join(", ")} — run workspace builds first`
  );
  process.exit(1);
}

rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });
cpSync(join(here, "src"), dist, { recursive: true });
for (const v of VARIANTS) {
  cpSync(join(here, v, "dist"), join(dist, v), { recursive: true });
}
writeFileSync(join(dist, ".ui-poc-built"), `${new Date().toISOString()}\n`, "utf8");
console.log(
  `[ui-poc-frontend] assembled dist/ (src/ + ${VARIANTS.map((v) => `${v}/dist`).join(" + ")})`
);
