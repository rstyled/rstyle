import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  recompute, intersect, matchedAt, partnerOf, recipeOf, firstDifferingTerm, whyLower,
  layer, layout, parseHash, formatHash, sourceLines, nodesOnLine,
  stageDiff, pairHoles, unpaired, coverageOf, STAGE_INFO,
} from '../lib/trace.js';
import { SCORERS, verdict } from '../scorers/index.js';

const load = (id) => JSON.parse(readFileSync(new URL(`../data/${id}.json`, import.meta.url)));
const index = JSON.parse(readFileSync(new URL('../data/index.json', import.meta.url)));

test('recompute equals the Rust sim_milli for every committed trace', () => {
  for (const { id } of index) {
    const t = load(id);
    assert.equal(recompute(t).sim_milli, t.sim_milli, id);
  }
});

test('golden B1: 51/80 = 637 with per-level counts 7,5,3,2', () => {
  const t = load('b1');
  assert.deepEqual(recompute(t), { numerator: 51, denominator: 80, sim_milli: 637 });
  assert.deepEqual(t.histograms.map((h) => h.intersection), [7, 5, 3, 2]);
});

test('golden Q2: 26/60 = 433 with per-level counts 5,2,0,0', () => {
  const t = load('q2');
  assert.deepEqual(recompute(t), { numerator: 26, denominator: 60, sim_milli: 433 });
  assert.deepEqual(t.histograms.map((h) => h.intersection), [5, 2, 0, 0]);
});

test('clone scores 1000; decoy T4 is below the threshold', () => {
  assert.equal(load('clone').sim_milli, 1000);
  const t4 = load('t4');
  assert.ok(t4.sim_milli < t4.threshold_milli);
});

test('matchedAt count equals the multiset intersection at every level', () => {
  for (const { id } of index) {
    const t = load(id);
    t.histograms.forEach((h, k) => {
      assert.equal(matchedAt(t, k).length, h.intersection, `${id} level ${k}`);
      assert.equal(intersect(h.a, h.b), h.intersection, `${id} level ${k}`);
    });
  }
});

test('matched pairs have equal colors', () => {
  const t = load('b1');
  t.histograms.forEach((h, k) => {
    for (const [i, j] of h.matched) {
      const [ra, rb] = [0, 1].map((g) => t.rounds.find((r) => r.level === k && r.graph === g));
      assert.equal(ra.colors[i], rb.colors[j]);
    }
  });
});

test('B1: components has no partner at level 0; param matches at 0 but not at 1', () => {
  const t = load('b1');
  const comp = t.graphs[0].nodes.find((n) => n.callee?.endsWith('::components')).idx;
  assert.equal(partnerOf(t, 0, 0, comp), null);
  assert.notEqual(partnerOf(t, 0, 0, 0), null);
  assert.equal(partnerOf(t, 1, 0, 0), null);
});

test('firstDifferingTerm: equal, own, in-term and length mismatch', () => {
  const r = (own, ins, outs = []) => ({ own_prev: own, in: ins, out: outs });
  const t = (tag, prev) => ({ tag, prev, node: 0 });
  assert.equal(firstDifferingTerm(r(1, [t('data@0', 2)]), r(1, [t('data@0', 2)])), null);
  assert.deepEqual(firstDifferingTerm(r(1, []), r(2, [])), { part: 'own', index: 0, a: 1, b: 2 });
  const d = firstDifferingTerm(r(1, [t('data@0', 2)]), r(1, [t('data@0', 3)]));
  assert.equal(d.part, 'in');
  assert.equal(firstDifferingTerm(r(1, [], [t('data@1', 5)]), r(1, [], [])).part, 'out');
});

test('B1 level 1: map recipes differ in the data@0 in-neighbour', () => {
  const t = load('b1');
  const w = whyLower(t, 1);
  assert.ok(w, 'a node matched at level 0 loses its match at level 1');
  const mine = recipeOf(t, 1, w.g, w.node);
  const theirs = recipeOf(t, 1, 1 - w.g, w.partner);
  assert.ok(firstDifferingTerm(mine, theirs));
});

test('whyLower is null at level 0 and for the clone', () => {
  assert.equal(whyLower(load('b1'), 0), null);
  for (let k = 1; k < 4; k++) assert.equal(whyLower(load('clone'), k), null);
});

test('layer: every non-back edge goes to a strictly later layer (all fixtures)', () => {
  for (const { id } of index) {
    for (const g of load(id).graphs) {
      const l = layer(g);
      const bad = g.edges.filter((e) => l[e.to] <= l[e.from]);
      assert.equal(bad.length, 0, `${id} ${g.fn_id}`);
    }
  }
});

test('layer handles a cycle without looping forever', () => {
  const g = { nodes: [{}, {}, {}], edges: [{ from: 0, to: 1 }, { from: 1, to: 2 }, { from: 2, to: 1 }] };
  assert.deepEqual(layer(g), [0, 1, 2]);
});

test('layout gives distinct positions', () => {
  const g = load('b1').graphs[0];
  const pos = layout(g).map((p) => `${p.x},${p.y}`);
  assert.equal(new Set(pos).size, g.nodes.length);
});

test('hash round trip', () => {
  const s = { pair: 'b1', level: 2, node: { g: 1, n: 3 } };
  assert.deepEqual(parseHash(formatHash(s)), s);
  assert.deepEqual(parseHash(''), { pair: null, level: 0, node: null });
});

test('sourceLines + nodesOnLine map lines to nodes', () => {
  const g = load('b1').graphs[0];
  const lines = sourceLines(g);
  assert.equal(lines[0].line, g.first_line);
  assert.ok(g.nodes.every((n) => lines.some((l) => l.line === n.line) || n.line === 0));
  const l = g.nodes.find((n) => n.callee?.endsWith('::components')).line;
  assert.ok(nodesOnLine(g, l).length >= 1);
});

test('scorers: wl matches Rust; align reads coverage', () => {
  const t = load('b1');
  const by = Object.fromEntries(SCORERS.map((s) => [s.id, s.run(t).sim_milli]));
  assert.equal(by.wl, 637);
  assert.equal(by.align, t.align.coverage_milli);
  assert.equal(by['wl-h2'], Math.floor((19 * 1000) / 24)); // (2*7 + 1*5) / (3*8)
});

test('verdict: gated pairs are never flagged', () => {
  assert.deepEqual(verdict('low', 1000, 600, false), { flagged: false, pass: true, gated: true });
  assert.equal(verdict('high', 500, 600, true).pass, false);
  assert.equal(verdict('high', 600, 600, true).pass, true);
});

test('alignment: last stage equals pairs, coverage recomputes, every stage has copy', () => {
  for (const { id } of index) {
    const t = load(id);
    const last = t.align.stages.at(-1);
    assert.deepEqual(last.pairs, t.align.pairs, id);
    assert.equal(coverageOf(t, t.align.pairs), t.align.coverage_milli, id);
    t.align.stages.forEach((s) => assert.ok(STAGE_INFO[s.name], `${id} ${s.name}`));
  }
});

test('alignment pairs are one-to-one in every stage', () => {
  for (const { id } of index) {
    for (const s of load(id).align.stages) {
      assert.equal(new Set(s.pairs.map((p) => p[0])).size, s.pairs.length);
      assert.equal(new Set(s.pairs.map((p) => p[1])).size, s.pairs.length);
    }
  }
});

test('golden B1 alignment: 6 of 8 pair, map and components stay unpaired', () => {
  const t = load('b1');
  assert.equal(t.align.coverage_milli, 750);
  assert.equal(t.align.holes.length, 0);
  const names = unpaired(t, 0, t.align.pairs).map((i) => t.graphs[0].nodes[i].callee ?? t.graphs[0].nodes[i].kind);
  assert.equal(names.length, 2);
  assert.ok(names.some((n) => n.endsWith('::components')) && names.some((n) => n.endsWith('::map')));
  assert.deepEqual(unpaired(t, 1, t.align.pairs).length, 1);
});

test('golden B1 stages: prune drops (3,3), complete pairs components with map, final prune drops it', () => {
  const t = load('b1');
  const by = Object.fromEntries(t.align.stages.map((s, i) => [s.name, i]));
  assert.deepEqual(stageDiff(t, by.prune_labels).dropped, [[3, 3]]);
  assert.deepEqual(stageDiff(t, by.complete).added, [[4, 3]]);
  assert.deepEqual(stageDiff(t, by.final).dropped, [[4, 3]]);
});

test('golden Q2 alignment: 4 pairs by label, holes are max->min and 1->5', () => {
  const t = load('q2');
  assert.equal(t.align.coverage_milli, 666);
  assert.deepEqual(t.align.holes.map((h) => [h.var, h.a.split('::').pop(), h.b.split('::').pop()]), [['M0', 'max', 'min'], ['L0', '1', '5']]);
  assert.equal(pairHoles(t).size, 2);
  assert.equal(t.align.stages[0].pairs.length, 0, 'no refined color is unique on both sides');
});
