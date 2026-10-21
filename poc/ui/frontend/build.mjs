// T1 minimal build: copy src/ → dist/ and write the `.ui-poc-built` marker that
// the Rust harness uses to detect a real frontend build (vs the compile-time stub
// that build.rs generates when dist/ is missing). T2 replaces this with Vite.
import { cpSync, mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const src = join(here, "src");
const dist = join(here, "dist");

mkdirSync(dist, { recursive: true });
cpSync(src, dist, { recursive: true });
writeFileSync(join(dist, ".ui-poc-built"), `${new Date().toISOString()}\n`, "utf8");
console.log("[ui-poc-frontend] built src/ -> dist/");
