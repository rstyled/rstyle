// Pure helpers over a WL trace (see crates/rstyle-core/src/wl_trace.rs). No DOM.

/** Re-derives the score from histograms/weights; must equal trace.sim_milli. */
export function recompute(trace) {
  const used = trace.histograms.filter((h) => h.used);
  const numerator = used.reduce((s, h) => s + h.weight * h.intersection, 0);
  const maxNodes = Math.max(...trace.graphs.map((g) => g.nodes.length));
  const denominator = trace.weights.reduce((s, w) => s + w, 0) * maxNodes;
  const sim_milli = denominator ? Math.floor((numerator * 1000) / denominator) : 0;
  return { numerator, denominator, sim_milli };
}

/** Multiset intersection of two {color: count} maps. */
export function intersect(a, b) {
  return Object.entries(a).reduce((s, [c, n]) => s + Math.min(n, b[c] ?? 0), 0);
}

export function matchedAt(trace, level) {
  return trace.histograms[level].matched;
}

/** Node in the other graph paired with (g, node) at `level`, or null. Display-only pairing. */
export function partnerOf(trace, level, g, node) {
  const pair = matchedAt(trace, level).find((p) => p[g] === node);
  return pair ? pair[1 - g] : null;
}

export function recipeOf(trace, level, g, node) {
  const round = trace.rounds.find((r) => r.level === level && r.graph === g);
  return round?.recipes[node] ?? null;
}

export function colorOf(trace, level, g, node) {
  return trace.rounds.find((r) => r.level === level && r.graph === g).colors[node];
}

const termKey = (t) => `${t.tag}:${t.prev}`;

/** First place two recipes differ: {part: 'own'|'in'|'out', index, a, b} or null when equal. */
export function firstDifferingTerm(a, b) {
  if (a.own_prev !== b.own_prev) return { part: 'own', index: 0, a: a.own_prev, b: b.own_prev };
  for (const part of ['in', 'out']) {
    const n = Math.max(a[part].length, b[part].length);
    for (let i = 0; i < n; i++) {
      const [x, y] = [a[part][i], b[part][i]];
      if (!x || !y || termKey(x) !== termKey(y)) return { part, index: i, a: x ?? null, b: y ?? null };
    }
  }
  return null;
}

/** A node that matched at level-1 but not at level (the first, A before B), with its old partner. */
export function whyLower(trace, level) {
  if (level < 1) return null;
  const now = matchedAt(trace, level);
  for (const g of [0, 1]) {
    for (const [i, j] of matchedAt(trace, level - 1)) {
      const node = g === 0 ? i : j;
      if (!now.some((p) => p[g] === node)) return { g, node, partner: g === 0 ? j : i };
    }
  }
  return null;
}

/** Longest-path layering; back edges (cycles) are ignored so loops still lay out. */
export function layer(graph) {
  const n = graph.nodes.length;
  const out = Array.from({ length: n }, () => []);
  graph.edges.forEach((e) => out[e.from].push(e.to));
  const state = new Array(n).fill(0);
  const back = new Set();
  const visit = (v) => {
    state[v] = 1;
    for (const w of out[v]) {
      if (state[w] === 1) back.add(`${v}>${w}`);
      else if (state[w] === 0) visit(w);
    }
    state[v] = 2;
  };
  for (let v = 0; v < n; v++) if (state[v] === 0) visit(v);
  const layers = new Array(n).fill(0);
  for (let pass = 0; pass < n; pass++) {
    for (const e of graph.edges) {
      if (back.has(`${e.from}>${e.to}`)) continue;
      layers[e.to] = Math.max(layers[e.to], layers[e.from] + 1);
    }
  }
  return layers;
}

/** x/y per node from layers; order inside a layer follows the mean y of already placed predecessors. */
export function layout(graph, { dx = 170, dy = 52 } = {}) {
  const layers = layer(graph);
  const rows = [];
  layers.forEach((l, v) => (rows[l] ??= []).push(v));
  const y = new Array(layers.length).fill(0);
  rows.forEach((row) => {
    const bary = (v) => {
      const ps = graph.edges.filter((e) => e.to === v && layers[e.from] < layers[v]).map((e) => y[e.from]);
      return ps.length ? ps.reduce((s, p) => s + p, 0) / ps.length : v * dy;
    };
    row.sort((a, b) => bary(a) - bary(b) || a - b);
    row.forEach((v, i) => (y[v] = i * dy));
  });
  return layers.map((l, v) => ({ x: l * dx, y: y[v] }));
}

export function parseHash(hash) {
  const p = new URLSearchParams(hash.replace(/^#/, ''));
  const [g, n] = (p.get('node') ?? '').split(':').map(Number);
  return {
    pair: p.get('pair'),
    level: p.has('level') ? Number(p.get('level')) : 0,
    node: Number.isInteger(g) && Number.isInteger(n) ? { g, n } : null,
  };
}

export function formatHash({ pair, level, node }) {
  const p = new URLSearchParams();
  if (pair) p.set('pair', pair);
  p.set('level', String(level ?? 0));
  if (node) p.set('node', `${node.g}:${node.n}`);
  return `#${p}`;
}

/** Source text split into [{line, text}] with absolute file line numbers. */
export function sourceLines(graph) {
  return graph.source.split('\n').map((text, i) => ({ line: graph.first_line + i, text }));
}

/** Node indexes on an absolute source line. */
export function nodesOnLine(graph, line) {
  return graph.nodes.filter((n) => n.line === line).map((n) => n.idx);
}

// ---------- alignment (align.rs) ----------

export const STAGE_INFO = {
  seed: ['Seed', 'Pair nodes whose refined WL color is unique in both graphs: a color seen exactly once on each side can only mean one thing.'],
  settle: ['Settle', 'Repeat until nothing moves: pair same-color nodes when an already-paired neighbour vouches for them (it has the mirrored edge); then greedily pair what is left in DFS order.'],
  prune: ['Prune', 'Drop pairs whose edges are not mirrored by their partner, worst first, until the matching is edge-consistent.'],
  labels: ['Pair by raw label', 'Redo seed and settle using raw node labels instead of refined colors. Nodes whose neighbourhood differs (so their refined colors differ) can still pair here.'],
  prune_labels: ['Prune again', 'Same prune. This removes label pairs whose neighbours turned out not to line up.'],
  complete: ['Pair by position', 'Pair leftover Call / Lit / Field nodes of the same kind whose labels DIFFER, because their already-paired neighbours agree with the position (e.g. Dog::bark with Cat::meow). Each such pair is a hole. It only checks edges of the node being placed, not edges into its partner, so the next prune can undo it.'],
  final: ['Final prune', 'One last consistency prune. What remains is the alignment; coverage = pairs / larger node count.'],
};

const key = (p) => `${p[0]}:${p[1]}`;

/** Pairs added and dropped by stage `i` relative to stage i-1 (stage -1 is empty). */
export function stageDiff(trace, i) {
  const now = trace.align.stages[i].pairs.map(key);
  const before = i > 0 ? trace.align.stages[i - 1].pairs.map(key) : [];
  const split = (xs) => xs.map((k) => k.split(':').map(Number));
  return { added: split(now.filter((k) => !before.includes(k))), dropped: split(before.filter((k) => !now.includes(k))), kept: split(now.filter((k) => before.includes(k))) };
}

/** "a:b" -> hole for aligned pairs where a substitution applies. */
export function pairHoles(trace) {
  const out = new Map();
  for (const h of trace.align.holes) for (const s of h.sites) out.set(key(s), h);
  return out;
}

/** Nodes of graph g (0 or 1) not in any pair of `pairs`. */
export function unpaired(trace, g, pairs) {
  const used = new Set(pairs.map((p) => p[g]));
  return trace.graphs[g].nodes.map((n) => n.idx).filter((i) => !used.has(i));
}

export function coverageOf(trace, pairs) {
  const larger = Math.max(...trace.graphs.map((g) => g.nodes.length), 1);
  return Math.floor((pairs.length * 1000) / larger);
}
