import {
  recompute, partnerOf, recipeOf, colorOf, firstDifferingTerm, whyLower, matchedAt,
  layout, parseHash, formatHash, sourceLines, nodesOnLine,
  stageDiff, pairHoles, unpaired, coverageOf, STAGE_INFO,
} from './lib/trace.js';
import { SCORERS, verdict } from './scorers/index.js';

const NODE_W = 150, NODE_H = 34, PAD = 12;
const $ = (id) => document.getElementById(id);
const svgEl = (tag, attrs = {}) => {
  const e = document.createElementNS('http://www.w3.org/2000/svg', tag);
  Object.entries(attrs).forEach(([k, v]) => e.setAttribute(k, v));
  return e;
};
const el = (tag, props = {}, ...kids) => {
  const e = Object.assign(document.createElement(tag), props);
  kids.flat().forEach((k) => e.append(k));
  return e;
};

const state = { index: [], traces: {}, pair: null, trace: null, level: 0, pinned: null, hover: null, hoverLevel: null, pos: [], stage: 0, alignHover: null };

const hue = (c) => `hsl(${(c * 47) % 360} 65% 86%)`;
const short = (n) => {
  const base = n.callee ? n.callee.split('::').slice(-2).join('::') : n.label;
  const text = `${n.idx} ${base}`;
  return text.length > 24 ? `${text.slice(0, 23)}…` : text;
};
const nodeTip = (n) => `[${n.idx}] ${n.label}${n.callee ? `\ncallee ${n.callee}` : ''}${n.detail ? `\n${n.detail}` : ''}\nline ${n.line}`;

// ---------- build (once per pair) ----------

function buildCode(g, graph) {
  const pre = el('pre', { className: 'code' });
  sourceLines(graph).forEach(({ line, text }) => {
    const s = el('span', { className: 'src', tabIndex: 0 }, el('span', { className: 'no', textContent: String(line) }), text);
    Object.assign(s.dataset, { g, line });
    s.dataset.tip = nodesOnLine(graph, line).map((i) => `[${i}] ${graph.nodes[i].label}`).join('\n') || 'no graph node on this line';
    pre.append(s);
  });
  return pre;
}

function buildGraph(g, graph, cls = 'node') {
  const pos = layout(graph);
  state.pos[g] = pos;
  const w = Math.max(...pos.map((p) => p.x)) + NODE_W + 2 * PAD;
  const h = Math.max(...pos.map((p) => p.y)) + NODE_H + 2 * PAD;
  const svg = svgEl('svg', { class: 'graph', viewBox: `0 0 ${w} ${h}`, role: 'img', 'aria-label': `dependence graph of ${graph.fn_id}` });
  const edges = svgEl('g');
  graph.edges.forEach((e, i) => {
    const [a, b] = [pos[e.from], pos[e.to]];
    const [x1, y1, x2, y2] = [a.x + NODE_W + PAD, a.y + NODE_H / 2 + PAD, b.x + PAD, b.y + NODE_H / 2 + PAD];
    const p = svgEl('path', { class: `edge ${e.kind}`, d: `M${x1},${y1} C${x1 + 30},${y1} ${x2 - 30},${y2} ${x2},${y2}` });
    p.dataset.edge = i;
    edges.append(p);
    const t = svgEl('text', { class: 'edgelabel', x: (x1 + x2) / 2, y: (y1 + y2) / 2 - 3 });
    t.textContent = `${e.kind === 'ctrl' ? 'c' : ''}${e.arg_pos}`;
    edges.append(t);
  });
  svg.append(edges);
  graph.nodes.forEach((n) => {
    const grp = svgEl('g', { class: cls, tabindex: 0, transform: `translate(${pos[n.idx].x + PAD},${pos[n.idx].y + PAD})` });
    Object.assign(grp.dataset, { g, n: n.idx, tip: nodeTip(n) });
    grp.append(svgEl('rect', { width: NODE_W, height: NODE_H }));
    const t = svgEl('text', { x: 8, y: 21 });
    t.textContent = short(n);
    grp.append(t);
    svg.append(grp);
  });
  return svg;
}

// ---------- alignment section ----------

const nodeName = (g, n) => {
  const node = state.trace.graphs[g].nodes[n];
  return `${g ? 'B' : 'A'}[${n}] ${node.callee ? node.callee.split('::').slice(-2).join('::') : node.label}`;
};

function buildAlign() {
  const t = state.trace;
  state.stage = t.align.stages.length - 1;
  const bar = $('stages');
  bar.replaceChildren(...t.align.stages.map((s, i) => {
    const b = el('button', { type: 'button', textContent: `${i + 1}. ${STAGE_INFO[s.name][0]}` });
    b.dataset.stage = i;
    return b;
  }));
  [0, 1].forEach((g) => $(`apane${g}`).replaceChildren(el('h3', { textContent: `${g ? 'B' : 'A'}: ${t.graphs[g].fn_id}` }), buildGraph(g, t.graphs[g], 'anode')));
  paintAlign();
}

function alignRow(cls, a, b, extra) {
  const tr = el('tr', { className: `arow ${cls}`, tabIndex: 0 }, el('td', { className: 'l', textContent: a === null ? '—' : nodeName(0, a) }), el('td', { className: 'l', textContent: b === null ? '—' : nodeName(1, b) }), el('td', { className: 'l', textContent: extra }));
  Object.assign(tr.dataset, { a: a ?? '', b: b ?? '' });
  return tr;
}

function paintAlign() {
  const t = state.trace, i = state.stage, st = t.align.stages[i], d = stageDiff(t, i);
  const holes = pairHoles(t), hl = state.alignHover;
  document.querySelectorAll('#stages button').forEach((b) => b.classList.toggle('cur', Number(b.dataset.stage) === i));
  const [title, text] = STAGE_INFO[st.name];
  const gain = `+${d.added.length} paired, −${d.dropped.length} dropped`;
  $('stageinfo').innerHTML = `<b>${i + 1}/${t.align.stages.length} ${title}</b> (${gain}). ${text}<br>After this step: ${st.pairs.length} pairs of ${Math.max(...t.graphs.map((g) => g.nodes.length))} nodes → coverage ${coverageOf(t, st.pairs)}.`;
  const partner = new Map(st.pairs.flatMap(([a, b]) => [[`0:${a}`, b], [`1:${b}`, a]]));
  const added = new Set(d.added.flatMap(([a, b]) => [`0:${a}`, `1:${b}`]));
  const dropped = new Set(d.dropped.flatMap(([a, b]) => [`0:${a}`, `1:${b}`]));
  document.querySelectorAll('.anode').forEach((e) => {
    const { g, n } = e.dataset, k = `${g}:${n}`, p = partner.get(k);
    const pairA = p === undefined ? null : Number(g) === 0 ? Number(n) : p;
    e.firstChild.style.fill = pairA === null ? '#fff' : `hsl(${(pairA * 53) % 360} 60% 85%)`;
    e.classList.toggle('nomatch', p === undefined);
    e.classList.toggle('added', added.has(k));
    e.classList.toggle('dropped', dropped.has(k));
    e.classList.toggle('sel', !!hl && ((Number(g) === 0 && Number(n) === hl.a) || (Number(g) === 1 && Number(n) === hl.b)));
    e.querySelector('.badge')?.remove();
    if (p !== undefined) {
      const badge = svgEl('text', { class: 'badge', x: 142, y: 11, 'text-anchor': 'end' });
      badge.textContent = `${Number(g) ? 'A' : 'B'}${p}`;
      e.append(badge);
    }
  });
  const rows = [
    ...st.pairs.map(([a, b]) => {
      const h = holes.get(`${a}:${b}`);
      const same = t.graphs[0].nodes[a].label === t.graphs[1].nodes[b].label;
      const note = h ? `hole ${h.var} (${h.kind}): ${h.a.split('::').pop()} → ${h.b.split('::').pop()}` : same ? 'same label' : 'labels differ, no hole kind';
      return alignRow(d.added.some((x) => x[0] === a && x[1] === b) ? 'new' : '', a, b, note);
    }),
    ...d.dropped.map(([a, b]) => alignRow('gone', a, b, 'dropped by this step')),
    ...unpaired(t, 0, st.pairs).map((a) => alignRow('none', a, null, 'unpaired')),
    ...unpaired(t, 1, st.pairs).map((b) => alignRow('none', null, b, 'unpaired')),
  ];
  $('pairs').replaceChildren(el('table', {}, el('tr', {}, ...['A node', 'B node', 'note'].map((x) => el('th', { className: 'l', textContent: x }))), ...rows));
  const hs = t.align.holes;
  $('holes').replaceChildren(el('h3', { textContent: hs.length ? 'Holes in the final alignment' : 'Holes in the final alignment: none' }), hs.length ? el('table', {}, el('tr', {}, ...['var', 'kind', 'A has', 'B has', 'at pairs'].map((x) => el('th', { className: 'l', textContent: x }))), ...hs.map((h) => el('tr', {}, ...[h.var, h.kind, h.a, h.b, h.sites.map(([a, b]) => `A${a}↔B${b}`).join(', ')].map((x) => el('td', { className: 'l', textContent: x }))))) : el('p', { textContent: 'Every aligned pair has the same label, so there is nothing to parameterize. The unpaired nodes are missing structure, not holes: they only cost coverage.' }));
}

function buildPanes() {
  state.trace.graphs.forEach((graph, g) => {
    const pane = $(`pane${g}`);
    pane.replaceChildren(el('h3', { textContent: `${g ? 'B' : 'A'}: ${graph.fn_id}  (${graph.path}:${graph.first_line}, ${graph.nodes.length} nodes, diameter ${graph.diameter})` }), buildCode(g, graph), buildGraph(g, graph));
  });
}

function buildHeader() {
  const t = state.trace, r = recompute(t);
  const meta = state.index.find((p) => p.id === state.pair);
  $('score').textContent = `${r.numerator} / ${r.denominator} = ${t.sim_milli}`;
  const c = t.comparable;
  const flagged = c.ok && t.sim_milli >= t.threshold_milli;
  $('chips').replaceChildren(
    el('span', { className: `chip ${flagged ? 'ok' : 'bad'}`, textContent: `${flagged ? '≥' : '<'} threshold ${t.threshold_milli}` }),
    el('span', { className: `chip ${c.ok ? 'ok' : 'bad'}`, textContent: c.ok ? 'comparable' : 'not comparable (gated)', title: `nodes ${c.nodes}, calls ${c.calls}; need ≥${c.min_nodes} nodes, ≥${c.min_calls} call, size ratio ≤ ${c.scale_limit}` }),
    el('span', { className: 'chip', textContent: `h = ${t.levels_used} levels, weights ${t.weights.join(',')}` }),
  );
  $('note').textContent = meta?.note ?? '';
  const err = $('error');
  try { SCORERS[0].run(t); err.hidden = true; } catch (e) { err.hidden = false; err.textContent = `Score mismatch: ${e.message}`; }
  $('level').max = String(t.max_levels - 1);
}

function buildTable() {
  const t = state.trace;
  let running = 0;
  const rows = t.histograms.map((h) => {
    const contrib = h.weight * h.intersection;
    running += contrib;
    const cells = [`${h.level}`, h.used ? `${h.weight}` : '—', `${Object.keys(h.a).length} / ${Object.keys(h.b).length}`, `${h.intersection}`, h.used ? `${contrib}` : 'not scored', h.used ? `${running}` : ''];
    const tr = el('tr', { className: h.used ? '' : 'unused' });
    tr.dataset.level = h.level;
    cells.forEach((c, i) => {
      const td = el('td', { textContent: c, className: i === 0 ? 'l' : '', tabIndex: 0 });
      td.dataset.level = h.level;
      td.dataset.tip = h.used ? `Level ${h.level}: ${h.intersection} nodes have a color present on both sides; × weight ${h.weight} = ${contrib}` : `Level ${h.level} is beyond h = ${t.levels_used} (graph diameter caps the depth), so it is not scored`;
      tr.append(td);
    });
    return tr;
  });
  const head = el('tr', {}, ...['level', 'weight', 'distinct colors A / B', '|A ∩ B|', 'weight × ∩', 'running sum'].map((x, i) => el('th', { textContent: x, className: i ? '' : 'l' })));
  $('table').replaceChildren(el('table', {}, head, ...rows));
  const used = t.histograms.filter((h) => h.used);
  const maxN = Math.max(...t.graphs.map((g) => g.nodes.length));
  const f = $('formula');
  f.replaceChildren('sim = Σ wᵢ·|∩ᵢ| / Σ wᵢ·max(|A|,|B|) = ');
  used.forEach((h, i) => {
    const s = el('span', { className: 'term', tabIndex: 0, textContent: `${h.weight}×${h.intersection}` });
    Object.assign(s.dataset, { level: h.level, tip: `level ${h.level}: weight ${h.weight} × ${h.intersection} matched nodes` });
    f.append(i ? ' + ' : '', s);
  });
  const sumW = t.weights.reduce((a, b) => a + b, 0);
  f.append(` = ${recompute(t).numerator}  over  (${t.weights.join('+')}) × ${maxN} = ${sumW * maxN}  →  ${t.sim_milli}`);
}

async function buildVariants() {
  const pairs = await Promise.all(state.index.map(async (p) => [p, await loadTrace(p.id)]));
  const head = el('tr', {}, el('th', { className: 'l', textContent: 'scorer' }), ...pairs.map(([p]) => el('th', { textContent: `${p.id} (${p.expect})`, title: p.title })));
  const rows = SCORERS.map((s) => el('tr', {}, el('th', { className: 'l', textContent: s.title }), ...pairs.map(([p, t]) => {
    const r = s.run(t);
    const v = verdict(p.expect, r.sim_milli, t.threshold_milli, t.comparable.ok);
    const td = el('td', { className: v.gated ? 'gated' : v.pass ? 'pass' : 'fail', textContent: `${r.sim_milli} ${v.pass ? '✓' : '✗'}${v.gated ? ' gated' : ''}`, tabIndex: 0 });
    td.dataset.tip = `${r.why}${v.gated ? '\nnot comparable: the miner never scores this pair' : ''}`;
    return td;
  })));
  const all = SCORERS.map((s, i) => `${s.title}: ${pairs.filter(([p, t]) => verdict(p.expect, s.run(t).sim_milli, t.threshold_milli, t.comparable.ok).pass).length}/${pairs.length}`);
  $('variants').replaceChildren(el('table', {}, head, ...rows), el('p', { textContent: `Pass count — ${all.join(' · ')}. A scorer is interesting only if it lifts b1 and q2, keeps the clone, and leaves decoys below the threshold.` }));
}

// ---------- paint (cheap, on every hover) ----------

function activeNodes() {
  const h = state.hover ?? state.pinned;
  if (!h) return [];
  if (h.line !== undefined) return nodesOnLine(state.trace.graphs[h.g], h.line).map((n) => ({ g: h.g, n }));
  return [{ g: h.g, n: h.n }];
}

function feedersOf(a) {
  const r = state.level > 0 ? recipeOf(state.trace, state.level, a.g, a.n) : null;
  return r ? new Set([...r.in, ...r.out].map((x) => x.node)) : new Set();
}

function paintNodes(act) {
  const t = state.trace, k = state.level;
  const matched = state.hoverLevel !== null ? new Set(matchedAt(t, state.hoverLevel).flatMap(([i, j]) => [`0:${i}`, `1:${j}`])) : null;
  const partners = new Set(act.map((a) => partnerOf(t, k, a.g, a.n)).map((p, i) => (p === null ? null : `${1 - act[i].g}:${p}`)));
  const feeders = new Set(act.flatMap((a) => [...feedersOf(a)].map((n) => `${a.g}:${n}`)));
  document.querySelectorAll('.panes .node').forEach((e) => {
    const { g, n } = e.dataset, key = `${g}:${n}`;
    const gi = Number(g), ni = Number(n);
    e.firstChild.style.fill = hue(colorOf(t, k, gi, ni));
    e.classList.toggle('nomatch', partnerOf(t, k, gi, ni) === null);
    e.classList.toggle('sel', act.some((a) => a.g === gi && a.n === ni));
    e.classList.toggle('partner', partners.has(key));
    e.classList.toggle('feeder', feeders.has(key));
    e.classList.toggle('dim', matched ? !matched.has(key) : false);
  });
  document.querySelectorAll('.edge').forEach((p) => p.classList.remove('hot'));
}

function paintText(act) {
  const lines = new Set(act.map((a) => `${a.g}:${state.trace.graphs[a.g].nodes[a.n].line}`));
  if (state.hover?.line !== undefined) lines.add(`${state.hover.g}:${state.hover.line}`);
  document.querySelectorAll('.src').forEach((s) => s.classList.toggle('hl', lines.has(`${s.dataset.g}:${s.dataset.line}`)));
  document.querySelectorAll('#table tr[data-level]').forEach((r) => r.classList.toggle('cur', Number(r.dataset.level) === state.level));
  document.querySelectorAll('[data-level]').forEach((e) => e.classList.toggle('hl', state.hoverLevel !== null && Number(e.dataset.level) === state.hoverLevel && e.tagName !== 'TR'));
  $('level').value = String(state.level);
  const t = state.trace, h = t.histograms[state.level];
  $('levelinfo').textContent = h.used
    ? `level ${state.level}: ${h.intersection} of ${Math.max(...t.graphs.map((g) => g.nodes.length))} nodes match (weight ${h.weight})`
    : `level ${state.level} is not scored for this pair (h = ${t.levels_used})`;
}

const termHtml = (t, trace, level, g) => {
  const n = trace.graphs[g].nodes[t.node];
  return `<code>${t.tag}</code> ← [${t.node}] ${n.callee ? n.callee.split('::').slice(-2).join('::') : n.kind} <code>c${t.prev}</code>`;
};

function describeNode(a) {
  const t = state.trace, k = state.level, node = t.graphs[a.g].nodes[a.n];
  const side = a.g ? 'B' : 'A', other = a.g ? 'A' : 'B';
  const head = `<p><b>${side}[${a.n}]</b> ${node.label} <small>(line ${node.line})</small></p>`;
  const partner = partnerOf(t, k, a.g, a.n);
  const status = partner !== null
    ? `<p style="color:var(--ok)">Level ${k}: same color as ${other}[${partner}] (c${colorOf(t, k, a.g, a.n)}).</p>`
    : `<p style="color:var(--bad)">Level ${k}: no node in ${other} has color c${colorOf(t, k, a.g, a.n)}.</p>`;
  if (k === 0) return head + status + `<p>Level 0 color = hash of the label only. Callee names, field names and literal values are not part of it.</p>`;
  const r = recipeOf(t, k, a.g, a.n);
  const list = (xs) => (xs.length ? `<ul>${xs.map((x) => `<li>${termHtml(x, t, k, a.g)}</li>`).join('')}</ul>` : ' none');
  const body = `<p>c${colorOf(t, k, a.g, a.n)} = hash( own level-${k - 1} <code>c${r.own_prev}</code>, in:${list(r.in)} out:${list(r.out)} )</p>`;
  return head + status + body + explainMismatch(a, partner);
}

function explainMismatch(a, partner) {
  const t = state.trace, k = state.level;
  if (partner !== null) return '';
  const prev = k > 0 ? partnerOf(t, k - 1, a.g, a.n) : null;
  if (prev === null) return '<p>It already had no partner one level earlier, so the difference started at or before this node’s neighbourhood.</p>';
  const mine = recipeOf(t, k, a.g, a.n), theirs = recipeOf(t, k, 1 - a.g, prev);
  const d = firstDifferingTerm(mine, theirs);
  if (!d) return '<p>Recipes look equal; the pairing shown is display-only (greedy), a different node took its partner.</p>';
  const show = (x) => (x && typeof x === 'object' ? `${x.tag} c${x.prev}` : x === null ? 'nothing' : `c${x}`);
  return `<p class="diff">At level ${k - 1} it was paired with ${a.g ? 'A' : 'B'}[${prev}]. Their recipes first differ in <b>${d.part}</b> #${d.index}: this side has <code>${show(d.a)}</code>, the other <code>${show(d.b)}</code>. That neighbour’s own color already differed one level earlier.</p>`;
}

function paint() {
  const act = activeNodes();
  paintNodes(act);
  paintText(act);
  $('recipebody').innerHTML = act.length ? act.slice(0, 3).map(describeNode).join('<hr>') : 'Hover or focus a node or source line.';
  history.replaceState(null, '', formatHash({ pair: state.pair, level: state.level, node: state.pinned && state.pinned.n !== undefined ? { g: state.pinned.g, n: state.pinned.n } : null }));
}

// ---------- events ----------

function targetOf(e) {
  const n = e.target.closest?.('.node'), s = e.target.closest?.('.src'), d = e.target.closest?.('[data-level]');
  return { n, s, d: d && d.tagName !== 'TR' ? d : null };
}

function onOver(e) {
  const { n, s, d } = targetOf(e);
  state.hover = n ? { g: Number(n.dataset.g), n: Number(n.dataset.n) } : s ? { g: Number(s.dataset.g), line: Number(s.dataset.line) } : null;
  state.hoverLevel = d ? Number(d.dataset.level) : null;
  const tipEl = n || s || d || e.target.closest?.('[data-tip]');
  const tip = $('tip');
  if (tipEl?.dataset.tip) {
    tip.hidden = false;
    tip.textContent = tipEl.dataset.tip;
    const r = tipEl.getBoundingClientRect();
    tip.style.left = `${Math.min(r.left, innerWidth - 380)}px`;
    tip.style.top = `${r.bottom + 6}px`;
  } else tip.hidden = true;
  paint();
}

function onOut() { state.hover = null; state.hoverLevel = null; $('tip').hidden = true; paint(); }

function onAlign(e) {
  const stage = e.target.closest?.('#stages button'), row = e.target.closest?.('.arow'), node = e.target.closest?.('.anode');
  if (stage && e.type === 'click') state.stage = Number(stage.dataset.stage);
  else if (row) state.alignHover = { a: row.dataset.a === '' ? -1 : Number(row.dataset.a), b: row.dataset.b === '' ? -1 : Number(row.dataset.b) };
  else if (node) {
    const { g, n } = node.dataset, p = state.trace.align.stages[state.stage].pairs.find((x) => x[Number(g)] === Number(n));
    state.alignHover = p ? { a: p[0], b: p[1] } : Number(g) === 0 ? { a: Number(n), b: -1 } : { a: -1, b: Number(n) };
  } else if (e.type !== 'click') state.alignHover = null;
  else return;
  paintAlign();
}

function onClick(e) {
  const { n, s, d } = targetOf(e);
  if (n) state.pinned = { g: Number(n.dataset.g), n: Number(n.dataset.n) };
  else if (s) state.pinned = { g: Number(s.dataset.g), line: Number(s.dataset.line) };
  else if (d) state.level = Number(d.dataset.level);
  else return;
  paint();
}

function onWhy() {
  const w = whyLower(state.trace, state.level);
  if (!w) { $('levelinfo').textContent = state.level === 0 ? 'Level 0 has no previous level.' : `Nothing matched at level ${state.level - 1} was lost at level ${state.level}.`; return; }
  state.pinned = { g: w.g, n: w.node };
  paint();
}

// ---------- boot ----------

async function loadTrace(id) {
  if (!state.traces[id]) {
    const meta = state.index.find((p) => p.id === id);
    state.traces[id] = await (await fetch(`data/${meta.file}`)).json();
  }
  return state.traces[id];
}

async function selectPair(id, level = 0, node = null) {
  state.pair = id;
  state.trace = await loadTrace(id);
  state.level = Math.min(level, state.trace.max_levels - 1);
  state.pinned = node && state.trace.graphs[node.g]?.nodes[node.n] ? node : null;
  state.hover = null;
  $('pair').value = id;
  buildHeader(); buildPanes(); buildTable(); buildAlign(); paint();
}

async function boot() {
  state.index = await (await fetch('data/index.json')).json();
  $('pair').replaceChildren(...state.index.map((p) => el('option', { value: p.id, textContent: p.title })));
  const h = parseHash(location.hash);
  await selectPair(state.index.some((p) => p.id === h.pair) ? h.pair : state.index[0].id, h.level, h.node);
  $('pair').addEventListener('change', (e) => selectPair(e.target.value));
  $('level').addEventListener('input', (e) => { state.level = Number(e.target.value); paint(); });
  $('why').addEventListener('click', onWhy);
  window.addEventListener('hashchange', () => {
    const next = parseHash(location.hash);
    const same = next.pair === state.pair && next.level === state.level && formatHash(next) === formatHash({ pair: state.pair, level: state.level, node: state.pinned?.n !== undefined ? state.pinned : null });
    if (!same && state.index.some((p) => p.id === next.pair)) selectPair(next.pair, next.level, next.node);
  });
  document.body.addEventListener('mouseover', onOver);
  document.body.addEventListener('focusin', onOver);
  document.body.addEventListener('mouseout', onOut);
  document.body.addEventListener('focusout', onOut);
  document.body.addEventListener('click', onClick);
  ['click', 'mouseover', 'focusin', 'mouseout', 'focusout'].forEach((ev) => $('align').addEventListener(ev, onAlign));
  document.body.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') { if (e.target.closest?.('.node, .src, td, .term')) { e.preventDefault(); onClick(e); } } });
  await buildVariants();
}

boot().catch((e) => { const err = $('error'); err.hidden = false; err.textContent = `Failed to load: ${e.message}`; });
