/**
 * Sole import boundary into `@ruviform/ui-core` — the Issue #258 headless
 * value/gesture contract. Everything presentation-side that needs the
 * contract enters through this re-export; component code never
 * re-implements value mapping, snapping, or gesture semantics
 * (Issue #257 §12.4: a skin may change appearance but must reuse the
 * shared headless logic, events, and a11y props).
 */
export * from "../../core/src/index.ts";
