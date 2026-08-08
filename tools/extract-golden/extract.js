// Produit les vecteurs de référence consommés par crates/swept-core/tests/golden.rs.
// Usage : node tools/extract-golden/extract.js

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { ob, distOB, overlapOBB, move } from "./proto.js";

// Empreinte relevée à la Task 8 du plan. Le prototype est gelé : s'il change,
// la copie de proto.js n'est plus fidèle et l'oracle est caduc.
const EXPECTED_SHA256 = "267e24e50dcfaad166e4b68dfec895cf3cf2c534608e824b436b0e9caf5e41ab";

const source = readFileSync("prototype/index.html");
const actual = createHash("sha256").update(source).digest("hex");
if (actual !== EXPECTED_SHA256) {
  console.error(`Le prototype a changé.\n  attendu : ${EXPECTED_SHA256}\n  obtenu  : ${actual}`);
  console.error("Le prototype est gelé. Si le changement est voulu, revoir proto.js puis mettre l'empreinte à jour.");
  process.exit(1);
}

// Générateur déterministe : les fixtures doivent être reproductibles.
let seed = 20260808;
const rand = () => {
  seed = (seed * 1103515245 + 12345) & 0x7fffffff;
  return seed / 0x7fffffff;
};
const between = (lo, hi) => lo + rand() * (hi - lo);

const geometry = [];
for (let i = 0; i < 400; i++) {
  const a = ob(between(-5, 5), between(-5, 5), between(-Math.PI, Math.PI), between(0.05, 3), between(0.05, 3));
  const b = ob(between(-5, 5), between(-5, 5), between(-Math.PI, Math.PI), between(0.05, 3), between(0.05, 3));
  const px = between(-8, 8), py = between(-8, 8);
  geometry.push({
    a: { cx: a.cx, cy: a.cy, ang: a.ang, hw: a.hw, hh: a.hh },
    b: { cx: b.cx, cy: b.cy, ang: b.ang, hw: b.hw, hh: b.hh },
    point: { x: px, y: py },
    distance_to_a: distOB(px, py, a),
    overlaps: overlapOBB(a, b),
  });
}

const kinematics = [];
for (let i = 0; i < 400; i++) {
  const start = { x: between(-5, 5), y: between(-5, 5), th: between(-Math.PI, Math.PI) };
  // Une courbure nulle une fois sur cinq, pour couvrir le segment droit.
  const curvature = i % 5 === 0 ? 0 : 1 / between(-12, 12);
  const distance = between(-8, 8);
  kinematics.push({ start, curvature, distance, end: move(start, curvature, distance) });
}

mkdirSync("crates/swept-core/tests/fixtures", { recursive: true });
writeFileSync("crates/swept-core/tests/fixtures/geometry.json", `${JSON.stringify(geometry, null, 1)}\n`);
writeFileSync("crates/swept-core/tests/fixtures/kinematics.json", `${JSON.stringify(kinematics, null, 1)}\n`);
console.log(`${geometry.length} cas de géométrie et ${kinematics.length} cas de cinématique écrits.`);
