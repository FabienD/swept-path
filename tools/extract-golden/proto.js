// Copie littérale des fonctions géométriques pures de prototype/index.html.
// Source : lignes 214-227, 251-261 et 398-412 du prototype, qui est gelé.
// La fidélité de cette copie est vérifiée par extract.js, qui refuse de
// tourner si l'empreinte du prototype a changé.

export function ob(cx, cy, ang, hw, hh) {
  return { cx, cy, ang, hw, hh, c: Math.cos(ang), s: Math.sin(ang) };
}

export function box(x0, x1, y0, y1) {
  return ob((x0 + x1) / 2, (y0 + y1) / 2, 0, (x1 - x0) / 2, (y1 - y0) / 2);
}

export function corners(o) {
  const p = [];
  for (const [a, b] of [[-1, -1], [1, -1], [1, 1], [-1, 1]])
    p.push([o.cx + a * o.hw * o.c - b * o.hh * o.s, o.cy + a * o.hw * o.s + b * o.hh * o.c]);
  return p;
}

export function distOB(px, py, o) {
  const dx = px - o.cx, dy = py - o.cy;
  const lx = dx * o.c + dy * o.s, ly = -dx * o.s + dy * o.c;
  const ax = Math.max(Math.abs(lx) - o.hw, 0), ay = Math.max(Math.abs(ly) - o.hh, 0);
  return (ax === 0 && ay === 0) ? -1 : Math.hypot(ax, ay);
}

export function overlapOBB(a, b) {
  const axes = [[a.c, a.s], [-a.s, a.c], [b.c, b.s], [-b.s, b.c]];
  const ca = corners(a), cb = corners(b);
  for (const [ux, uy] of axes) {
    let a0 = Infinity, a1 = -Infinity, b0 = Infinity, b1 = -Infinity;
    for (const p of ca) { const v = p[0] * ux + p[1] * uy; if (v < a0) a0 = v; if (v > a1) a1 = v; }
    for (const p of cb) { const v = p[0] * ux + p[1] * uy; if (v < b0) b0 = v; if (v > b1) b1 = v; }
    if (a1 < b0 + 0.006 || b1 < a0 + 0.006) return false;
  }
  return true;
}

export function move(p, kap, dist) {
  const th1 = p.th + kap * dist;
  let x = p.x, y = p.y;
  if (kap === 0) { x += Math.cos(p.th) * dist; y += Math.sin(p.th) * dist; }
  else { const R = 1 / kap; x += R * (Math.sin(th1) - Math.sin(p.th)); y -= R * (Math.cos(th1) - Math.cos(p.th)); }
  return { x, y, th: th1 };
}
