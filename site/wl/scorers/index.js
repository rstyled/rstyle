// Pure scorers over a trace. Add one by writing a function and a registry line.
// A scorer returns {sim_milli, why}. `gated` pairs (not comparable) are never flagged by the miner.
import { recompute } from '../lib/trace.js';

const I = (trace) => trace.histograms.map((h) => h.intersection);
const maxNodes = (trace) => Math.max(...trace.graphs.map((g) => g.nodes.length));
const milli = (num, den) => (den ? Math.floor((num * 1000) / den) : 0);

/** Current scorer; asserts the JS arithmetic equals the Rust number. */
export function wl(trace) {
  const r = recompute(trace);
  if (r.sim_milli !== trace.sim_milli) throw new Error(`recompute ${r.sim_milli} != trace ${trace.sim_milli}`);
  return { sim_milli: r.sim_milli, why: `${r.numerator} / ${r.denominator}, all ${trace.levels_used} levels` };
}

/** Same formula with at most two levels (the rejected 4a experiment). */
export function wlH2(trace) {
  const h = Math.min(trace.levels_used, 2);
  const w = [...Array(h).keys()].map((i) => h - i);
  const num = w.reduce((s, wi, i) => s + wi * I(trace)[i], 0);
  const den = w.reduce((s, wi) => s + wi, 0) * maxNodes(trace);
  return { sim_milli: milli(num, den), why: `levels 0..${h - 1}: ${num} / ${den}` };
}

/** WL normalized by the nodes whose level-0 label has a partner: ignores missing nodes (holes). */
export function wlMatchable(trace) {
  const r = recompute(trace);
  const base = I(trace)[0];
  const den = trace.weights.reduce((s, w) => s + w, 0) * base;
  return { sim_milli: milli(r.numerator, den), why: `${r.numerator} / ${den} (only ${base} label-matched nodes count)` };
}

/** Alignment coverage from align.rs: aligned node pairs over the larger node count. */
export function align(trace) {
  const n = trace.align.pairs.length;
  return { sim_milli: trace.align.coverage_milli, why: `${n} aligned pairs / ${maxNodes(trace)} nodes` };
}

export const SCORERS = [
  { id: 'wl', title: 'WL (current)', run: wl },
  { id: 'wl-h2', title: 'WL, 2 levels', run: wlH2 },
  { id: 'wl-matchable', title: 'WL / matchable nodes', run: wlMatchable },
  { id: 'align', title: 'Alignment coverage', run: align },
];

/** Verdict against the expectation: high pairs should flag, low pairs should not. */
export function verdict(expect, sim_milli, threshold, comparableOk) {
  const flagged = comparableOk && sim_milli >= threshold;
  const pass = expect === 'high' ? flagged : !flagged;
  return { flagged, pass, gated: !comparableOk };
}
