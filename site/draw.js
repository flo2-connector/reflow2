// The drawing helpers the explainer pages share (index.html, walkthrough.html).
// Drawn after the owner's "Project Brain" one-pager: the same helpers and layouts,
// re-coloured for the site's dark palette. Each page draws its own figures with
// these, into <svg id="..."> elements, after this file has loaded.
const COL = {R: '#2a78d6', D: '#eb6834', C: '#1baf7a', F: '#6a5ad0', X: '#8a8880'};
const LET = {R: 'R', D: 'D', C: 'C', F: 'F', X: 'Δ'};
const INK = '#ecebe6', INK2 = '#b9b7b0', MUTED = '#8d8b85', HAIR = '#2d2f2b', AXIS = '#4b4e48',
      SURF = '#151614', PLANE = '#1c1e1b', PANEL = '#22241f', EDGE = '#6f716b',
      ACC = '#5cc3a6', ACC_INK = '#0f1a16', WARN_BG = '#3b261b', WARN = '#f2a57f';
// The three partners, coloured the same on every page (and in style.css's .who chips).
const WHO = {you: ACC, ai: '#7f8cf0', r2: '#eb6834'};
const FONT = 'system-ui,-apple-system,"Segoe UI",Roboto,sans-serif';
const cv = document.createElement('canvas').getContext('2d');
function tw(s, size = 15, w = 600) { cv.font = `${w} ${size}px ${FONT}`; return cv.measureText(s).width; }

function nd(x, y, t, r = 14, o = {}) {
  const hollow = !!o.hollow;
  return `<g opacity="${o.op ?? 1}"><circle cx="${x}" cy="${y}" r="${r}" fill="${hollow ? SURF : COL[t]}" stroke="${hollow ? COL[t] : SURF}" stroke-width="${hollow ? 2.2 : 2}" ${o.dash ? 'stroke-dasharray="4 3"' : ''}/>` +
    `<text x="${x}" y="${y + 0.5}" text-anchor="middle" dominant-baseline="central" font-size="${Math.round(r * (t === 'X' ? 1.05 : 0.98))}" font-weight="700" fill="${hollow ? COL[t] : '#fff'}">${LET[t]}</text></g>`;
}
function tx(x, y, s, o = {}) {
  return `<text x="${x}" y="${y}" font-size="${o.size || 16}" font-weight="${o.w || 400}" fill="${o.c || INK}" text-anchor="${o.a || 'start'}" ${o.italic ? 'font-style="italic"' : ''} ${o.base ? `dominant-baseline="${o.base}"` : ''} opacity="${o.op ?? 1}">${s}</text>`;
}
function ln(x1, y1, x2, y2, o = {}) {
  return `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke="${o.c || AXIS}" stroke-width="${o.w || 1.5}" ${o.dash ? `stroke-dasharray="${o.dash}"` : ''} ${o.arrow ? `marker-end="url(#${o.arrow})"` : ''} stroke-linecap="round" opacity="${o.op ?? 1}"/>`;
}
function path(d, o = {}) {
  return `<path d="${d}" fill="${o.fill || 'none'}" stroke="${o.c || AXIS}" stroke-width="${o.w || 1.5}" ${o.dash ? `stroke-dasharray="${o.dash}"` : ''} ${o.arrow ? `marker-end="url(#${o.arrow})"` : ''} stroke-linecap="round" stroke-linejoin="round" opacity="${o.op ?? 1}"/>`;
}
function rect(x, y, w, h, o = {}) {
  return `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${o.rx ?? 14}" fill="${o.fill || PANEL}" stroke="${o.c || HAIR}" stroke-width="${o.w || 1.25}" ${o.dash ? `stroke-dasharray="${o.dash}"` : ''} opacity="${o.op ?? 1}"/>`;
}
// Feather-style icons (24-unit grid)
const ICONS = {
  monitor: '<rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>',
  search: '<circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.2" y2="16.2"/>',
  tool: '<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>',
  user: '<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
  lock: '<rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>',
  chat: '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>',
  terminal: '<polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>',
  refresh: '<polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>',
  cpu: '<rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="14" x2="23" y2="14"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="14" x2="4" y2="14"/>',
};
function icon(name, x, y, size = 24, c = INK, sw = 2) {
  const k = size / 24;
  return `<g transform="translate(${x - size / 2},${y - size / 2}) scale(${k})" fill="none" stroke="${c}" stroke-width="${sw / k}" stroke-linecap="round" stroke-linejoin="round">${ICONS[name]}</g>`;
}
// A small graph glyph: three linked facts. Stands for reflow2's record.
function graphGlyph(cx, cy, s = 1) {
  const p = [[-16, -12], [18, -4], [-6, 18]].map(([x, y]) => [cx + x * s, cy + y * s]);
  let g = path(`M${p[0][0]},${p[0][1]} L${p[1][0]},${p[1][1]} L${p[2][0]},${p[2][1]} Z`, {c: INK2, w: 1.6});
  g += `<circle cx="${p[0][0]}" cy="${p[0][1]}" r="${7 * s}" fill="${COL.R}"/><circle cx="${p[1][0]}" cy="${p[1][1]}" r="${7 * s}" fill="${COL.D}"/><circle cx="${p[2][0]}" cy="${p[2][1]}" r="${7 * s}" fill="${COL.C}"/>`;
  return g;
}
// A repository "pill": a tiny three-node glyph (its own graph) and its name.
function pill(cx, cy, label, o = {}) {
  const size = o.size || 15, h = o.h || 38;
  const lockW = o.lock ? 22 : 0;
  const w = tw(label, size, 600) + 36 + lockW + 22;
  const x = cx - w / 2, y = cy - h / 2;
  let s = `<g opacity="${o.op ?? 1}">`;
  s += `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${h / 2}" fill="${PANEL}" stroke="${o.stroke || EDGE}" stroke-width="${o.sw || 1.25}" ${o.dash ? 'stroke-dasharray="5 4"' : ''}/>`;
  const gx = x + 20, gy = cy;
  s += `<path d="M${gx - 5},${gy - 5} L${gx + 6},${gy - 1} L${gx - 3},${gy + 7} Z" fill="none" stroke="${AXIS}" stroke-width="1"/>`;
  s += `<circle cx="${gx - 5}" cy="${gy - 5}" r="3.4" fill="${COL.R}"/><circle cx="${gx + 6}" cy="${gy - 1}" r="3.4" fill="${COL.D}"/><circle cx="${gx - 3}" cy="${gy + 7}" r="3.4" fill="${COL.C}"/>`;
  s += tx(x + 36, cy + 0.5, label, {size, w: 600, base: 'central'});
  if (o.lock) s += icon('lock', x + w - 22, cy, 15, INK2, 2.2);
  s += '</g>';
  return {s, w, h, x, y};
}
function tag(cx, cy, label, o = {}) {
  const size = o.size || 13, w = tw(label, size, 600) + 18, h = 22;
  return `<rect x="${cx - w / 2}" y="${cy - h / 2}" width="${w}" height="${h}" rx="11" fill="${o.fill || PLANE}" stroke="${o.c || 'none'}"/>` +
    tx(cx, cy + 0.5, label, {size, w: 600, c: o.tc || INK2, a: 'middle', base: 'central'});
}
// A hub, or project brain: the one place over several designs (hub.html).
function hubMark(cx, cy, r, size) {
  let s = `<circle cx="${cx}" cy="${cy}" r="${r}" fill="${ACC}"/>`;
  s += tx(cx, cy - size * 0.45, 'project', {size, w: 700, c: ACC_INK, a: 'middle'});
  s += tx(cx, cy + size * 0.85, 'brain', {size, w: 700, c: ACC_INK, a: 'middle'});
  s += tx(cx, cy + size * 2.1, '(hub)', {size: 13, c: ACC_INK, a: 'middle', op: 0.75});
  return s;
}
function badge(x, y, n) {
  return `<circle cx="${x}" cy="${y}" r="12" fill="${ACC}"/>` + tx(x, y + 0.5, String(n), {size: 13, w: 700, c: ACC_INK, a: 'middle', base: 'central'});
}
function set(id, s) { document.getElementById(id).innerHTML = s; }
