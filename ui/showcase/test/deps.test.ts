/**
 * Dependency audit (Issue #263): prove no UI framework is a required
 * dependency anywhere under `ui/*`, and document the package DAG by
 * statically scanning source imports. This test is the gate itself —
 * a manifest or source change introducing React/Solid/Vue/Lit fails
 * here. Packages are discovered from the filesystem, so a future
 * `ui/<new>` package enters the audit automatically.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const UI_DIR = resolve(HERE, "..", "..");

/** Every directory under ui/ that declares a package.json. */
const UI_PACKAGES = readdirSync(UI_DIR)
  .filter(
    (name) =>
      statSync(join(UI_DIR, name), { throwIfNoEntry: false })
        ?.isDirectory() === true &&
      statSync(join(UI_DIR, name, "package.json"), {
        throwIfNoEntry: false,
      })?.isFile() === true,
  )
  .sort();

/** Frameworks that must never appear as a required dependency. The
 *  empty-runtime-deps assertion is the real gate; this list only adds
 *  explicit naming to the failure message. */
const FORBIDDEN = [
  "react",
  "react-dom",
  "react-native",
  "solid-js",
  "solid-start",
  "@solidjs/components",
  "vue",
  "petite-vue",
  "vue-router",
  "@vue/runtime-dom",
  "svelte",
  "@sveltejs/kit",
  "preact",
  "preact/compat",
  "@angular/core",
  "@angular/forms",
  "@angular/common",
  "lit",
  "lit-html",
  "lit-element",
  "@lit/reactive-element",
  "stencil",
  "@stencil/core",
  "jquery",
  "ember-source",
  "inferno",
  "mithril",
  "hyperapp",
  "alpinejs",
  "marko",
  "htm",
  "@builder.io/qwik",
];

/** Allowed devDependencies (test/build toolchain only). */
const ALLOWED_DEV = new Set(["typescript", "@types/node", "happy-dom"]);

/** Node-side test-harness packages inside ui/: they are not browser
 *  code, so they may declare extra devDependencies and import the
 *  declared bare specifiers in src/. Entries are the complete,
 *  reviewed exception surface — adding a package here is a design
 *  decision, not a workaround. */
const HARNESS_DEV_DEPS: Record<string, readonly string[]> = {
  "browser-test": ["@playwright/test"],
};
const HARNESS_BARE_SPECS: Record<string, readonly string[]> = {
  "browser-test": ["@playwright/test"],
};

/** Direction of permitted source-level imports between ui packages.
 *  A package absent from this map may not import any ui sibling. */
const ALLOWED_IMPORTS: Record<string, readonly string[]> = {
  core: [],
  components: ["core"],
  viz: ["core", "components"],
  binding: ["core"],
  showcase: ["core", "components", "viz", "binding"],
};

interface Manifest {
  name?: string;
  private?: boolean;
  type?: string;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
}

const SOURCE_EXT = [".ts", ".mts", ".cts", ".tsx", ".js", ".mjs", ".jsx"];

function listSrc(dir: string): string[] {
  const out: string[] = [];
  const walk = (d: string): void => {
    for (const name of readdirSync(d)) {
      const p = join(d, name);
      if (statSync(p).isDirectory()) walk(p);
      else if (SOURCE_EXT.some((ext) => name.endsWith(ext))) out.push(p);
    }
  };
  if (statSync(dir, { throwIfNoEntry: false })?.isDirectory() === true) {
    walk(dir);
  }
  return out;
}

/** Import specifiers referenced by a source file (from / import /
 *  require). Block AND line comments are stripped first so prose like
 *  `… apart from "a new truth arrived" …` cannot fake a specifier. */
function specifiersOf(src: string): string[] {
  const stripped = src
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/(^|[^:"'`])\/\/[^\n]*/g, "$1");
  const out: string[] = [];
  const re =
    /(?:from|import|require)\s*\(?\s*["']([^"']+)["']/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(stripped)) !== null) out.push(m[1]!);
  return out;
}

describe("dependency audit — ui/* manifests", () => {
  for (const pkg of UI_PACKAGES) {
    it(`ui/${pkg}: no framework dependencies, dev-only toolchain`, () => {
      const mf = JSON.parse(
        readFileSync(join(UI_DIR, pkg, "package.json"), "utf8"),
      ) as Manifest;
      const runtime = Object.keys(mf.dependencies ?? {});
      const peer = Object.keys(mf.peerDependencies ?? {});
      const optional = Object.keys(mf.optionalDependencies ?? {});
      for (const dep of [...runtime, ...peer, ...optional]) {
        assert.equal(
          FORBIDDEN.includes(dep),
          false,
          `ui/${pkg} declares forbidden dependency: ${dep}`,
        );
      }
      // ui packages must have zero deps of every required kind —
      // sibling imports are source-relative, not registry deps, and
      // peer/optional slots cannot smuggle a framework in.
      assert.deepEqual(
        runtime,
        [],
        `ui/${pkg} declares runtime dependencies: ${runtime.join(", ")}`,
      );
      assert.deepEqual(
        peer,
        [],
        `ui/${pkg} declares peerDependencies: ${peer.join(", ")}`,
      );
      assert.deepEqual(
        optional,
        [],
        `ui/${pkg} declares optionalDependencies: ${optional.join(", ")}`,
      );
      for (const dev of Object.keys(mf.devDependencies ?? {})) {
        assert.ok(
          ALLOWED_DEV.has(dev) ||
            (HARNESS_DEV_DEPS[pkg] ?? []).includes(dev),
          `ui/${pkg} declares unexpected devDependency: ${dev}`,
        );
      }
    });
  }

  it("no ui/* source imports a bare specifier (framework or lib)", () => {
    for (const pkg of UI_PACKAGES) {
      for (const file of listSrc(join(UI_DIR, pkg, "src"))) {
        const rel = relative(UI_DIR, file);
        for (const spec of specifiersOf(readFileSync(file, "utf8"))) {
          const harnessOk = (HARNESS_BARE_SPECS[pkg] ?? []).some(
            (allowed) => spec === allowed || spec.startsWith(`${allowed}/`),
          );
          assert.ok(
            spec.startsWith(".") || spec.startsWith("node:") || harnessOk,
            `${rel} imports bare specifier "${spec}" — all ui imports must be source-relative (node: builtins and declared harness specs allowed)`,
          );
        }
      }
    }
  });

  it("sibling import direction matches the declared package DAG", () => {
    const edges: string[] = [];
    for (const pkg of UI_PACKAGES) {
      for (const file of listSrc(join(UI_DIR, pkg, "src"))) {
        const rel = relative(UI_DIR, file);
        for (const spec of specifiersOf(readFileSync(file, "utf8"))) {
          const m = /^\.\.\/\.\.\/([^/]+)\//.exec(spec);
          if (m === null) continue;
          const target = m[1]!;
          if (!UI_PACKAGES.includes(target)) continue;
          assert.ok(
            (ALLOWED_IMPORTS[pkg] ?? []).includes(target),
            `${rel} imports ../../${target}/ — not allowed in the ui DAG`,
          );
          edges.push(`${pkg} -> ${target}`);
        }
      }
    }
    // Documented DAG for the impl record:
    //   core ─ components ─ viz
    //   core ─ binding
    //   components + viz + binding (+ core via them) ─ showcase
    assert.ok(edges.includes("components -> core"));
    assert.ok(edges.includes("viz -> components"));
    assert.ok(edges.includes("binding -> core"));
    assert.ok(edges.includes("showcase -> components"));
    assert.ok(edges.includes("showcase -> viz"));
    assert.ok(edges.includes("showcase -> binding"));
  });
});
