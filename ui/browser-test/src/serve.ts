/**
 * Hermetic static serving over `page.route` (Issue #310).
 *
 * The harness does not spawn a web server: one route interception
 * fulfills every request the test page makes from the workspace file
 * system. That keeps runs hermetic (no port, no process, no external
 * network) and deterministic (the same bytes on every OS).
 *
 * URL space — fixed origin {@link RVF_ORIGIN}:
 * - `/pages/<file>`            → `ui/browser-test/pages/`
 * - `/browser-test/dist/<sub>` → `ui/browser-test/dist/`
 * - `/showcase/dist/<sub>`     → `ui/showcase/dist/`
 *
 * Anything outside the mapped prefixes — including every non-`rvf.test`
 * origin — is aborted, so an unexpected external fetch fails loudly
 * instead of silently reaching the network.
 */

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Page } from "@playwright/test";

/** Fixed same-origin base every harness page and module is served under. */
export const RVF_ORIGIN = "http://rvf.test";

const MIME: Readonly<Record<string, string>> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".wasm": "application/wasm",
};

const PACKAGE_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

/** URL prefix → filesystem root, checked longest-prefix first. */
export interface ServeRoot {
  readonly urlPrefix: string;
  readonly dir: string;
}

/** Default roots for the ruviform browser harness. */
export function defaultServeRoots(): ServeRoot[] {
  return [
    { urlPrefix: "/browser-test/dist/", dir: path.join(PACKAGE_ROOT, "dist") },
    { urlPrefix: "/showcase/dist/", dir: path.resolve(PACKAGE_ROOT, "../showcase/dist") },
    { urlPrefix: "/pages/", dir: path.join(PACKAGE_ROOT, "pages") },
  ];
}

export interface ServeStats {
  /** Fulfilled requests by url prefix (diagnostics on failure). */
  readonly served: Map<string, number>;
  /** Aborted requests (non-mapped or non-rvf.test origins). */
  readonly aborted: string[];
}

/**
 * Install the route interception on `page`. Returns stats the caller
 * can inspect in diagnostics. Must run before `page.goto`.
 */
export async function installStaticServe(
  page: Page,
  roots: ServeRoot[] = defaultServeRoots(),
): Promise<ServeStats> {
  const served = new Map<string, number>();
  const aborted: string[] = [];
  const sorted = [...roots].sort(
    (a, b) => b.urlPrefix.length - a.urlPrefix.length,
  );

  await page.route("**/*", async (route) => {
    const url = new URL(route.request().url());
    if (url.origin !== RVF_ORIGIN) {
      aborted.push(route.request().url());
      await route.abort();
      return;
    }
    const root = sorted.find((r) => url.pathname.startsWith(r.urlPrefix));
    if (root === undefined) {
      aborted.push(url.pathname);
      await route.abort();
      return;
    }
    const sub = decodeURIComponent(url.pathname.slice(root.urlPrefix.length));
    const file = path.resolve(root.dir, sub);
    if (!file.startsWith(path.resolve(root.dir) + path.sep)) {
      aborted.push(url.pathname);
      await route.abort();
      return;
    }
    try {
      const body = await readFile(file);
      served.set(root.urlPrefix, (served.get(root.urlPrefix) ?? 0) + 1);
      await route.fulfill({
        status: 200,
        contentType: MIME[path.extname(file)] ?? "application/octet-stream",
        body,
      });
    } catch {
      aborted.push(url.pathname);
      await route.fulfill({ status: 404, body: "not found" });
    }
  });
  return { served, aborted };
}
