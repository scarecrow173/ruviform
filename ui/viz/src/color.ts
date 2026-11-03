/**
 * Resolved CSS-color text → RGBA components (`0..1` floats).
 *
 * Canvas2D consumes resolved colors as strings; the WebGL2 executor
 * needs float uniforms. Supported forms: `#rgb`, `#rgba`, `#rrggbb`,
 * `#rrggbbaa`, `rgb()/rgba()` (comma or space separated, optional
 * percentage channels, `/` or comma alpha), and `transparent`.
 * Anything else returns `undefined` — callers surface a
 * `color-unparseable` capability note and substitute a declared
 * fallback rather than silently painting black.
 */

export interface Rgba {
  readonly r: number;
  readonly g: number;
  readonly b: number;
  readonly a: number;
}

const HEX = /^#([0-9a-f]{3,8})$/i;
const FN = /^rgba?\((.+)\)$/i;

export function parseCssColor(text: string): Rgba | undefined {
  const t = text.trim().toLowerCase();
  if (t === "transparent") return { r: 0, g: 0, b: 0, a: 0 };
  const hex = HEX.exec(t);
  if (hex !== null) return fromHex(hex[1] ?? "");
  const fn = FN.exec(t);
  if (fn !== null) return fromFn(fn[1] ?? "");
  return undefined;
}

function fromHex(h: string): Rgba | undefined {
  const n = (i: number) => Number.parseInt(h.slice(i, i + 2), 16) / 255;
  const s = (i: number) => Number.parseInt(h.slice(i, i + 1).repeat(2), 16) / 255;
  switch (h.length) {
    case 3:
      return { r: s(0), g: s(1), b: s(2), a: 1 };
    case 4:
      return { r: s(0), g: s(1), b: s(2), a: s(3) };
    case 6:
      return { r: n(0), g: n(2), b: n(4), a: 1 };
    case 8:
      return { r: n(0), g: n(2), b: n(4), a: n(6) };
    default:
      return undefined;
  }
}

function fromFn(body: string): Rgba | undefined {
  // `rgb(255, 0, 0, .5)`, `rgb(100% 0% 0% / 50%)`, `rgba(0,0,0,0.25)`.
  const slash = body.split("/");
  const main = (slash[0] ?? "")
    .trim()
    .split(/[\s,]+/)
    .filter((p) => p.length > 0);
  const alphaPart = slash[1]?.trim() ?? (main.length > 3 ? main.pop() : undefined);
  if (main.length !== 3) return undefined;
  const ch = main.map(channel);
  if (ch.some((c) => c === undefined)) return undefined;
  const a = alphaPart === undefined ? 1 : alpha(alphaPart);
  if (a === undefined) return undefined;
  return { r: ch[0] ?? 0, g: ch[1] ?? 0, b: ch[2] ?? 0, a };
}

function channel(p: string): number | undefined {
  if (p.endsWith("%")) {
    const v = Number.parseFloat(p.slice(0, -1));
    return Number.isFinite(v) ? clamp01(v / 100) : undefined;
  }
  const v = Number.parseFloat(p);
  return Number.isFinite(v) ? clamp01(v / 255) : undefined;
}

function alpha(p: string): number | undefined {
  if (p.endsWith("%")) {
    const v = Number.parseFloat(p.slice(0, -1));
    return Number.isFinite(v) ? clamp01(v / 100) : undefined;
  }
  const v = Number.parseFloat(p);
  return Number.isFinite(v) ? clamp01(v) : undefined;
}

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}
