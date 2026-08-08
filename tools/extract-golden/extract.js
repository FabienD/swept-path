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

const DIR = "crates/swept-core/tests/fixtures";
const OUTPUTS = [
  [`${DIR}/geometry.json`, geometry],
  [`${DIR}/kinematics.json`, kinematics],
];

// Écart toléré entre deux exécutions, en valeur absolue.
//
// Math.sin et Math.cos ne sont pas correctement arrondies au sens d'IEEE 754 :
// les implémentations libm diffèrent d'un ULP entre macOS/arm64 et
// Linux/x86-64, soit environ 1e-15. Une comparaison octet à octet échouerait
// donc selon la machine. Ce seuil est mille fois plus fin que la tolérance de
// 1e-9 des tests Rust : une fixture retouchée pour faire passer un test reste
// détectée.
const TOLERANCE = 1e-12;

/** Compare deux valeurs, en tolérant l'écart d'arrondi entre plateformes. */
function diff(expected, actual, path, out) {
  if (typeof expected === "number") {
    if (typeof actual !== "number" || Math.abs(expected - actual) > TOLERANCE) {
      out.push(`${path} : commité ${expected}, régénéré ${actual}`);
    }
  } else if (Array.isArray(expected)) {
    if (!Array.isArray(actual) || expected.length !== actual.length) {
      out.push(`${path} : ${expected.length} éléments commités, ${actual?.length} régénérés`);
    } else {
      expected.forEach((v, i) => diff(v, actual[i], `${path}[${i}]`, out));
    }
  } else if (expected !== null && typeof expected === "object") {
    for (const k of Object.keys(expected)) diff(expected[k], actual?.[k], `${path}.${k}`, out);
  } else if (expected !== actual) {
    out.push(`${path} : commité ${expected}, régénéré ${actual}`);
  }
}

if (process.argv.includes("--check")) {
  const problems = [];
  for (const [file, produced] of OUTPUTS) {
    let committed;
    try {
      committed = JSON.parse(readFileSync(file, "utf8"));
    } catch {
      problems.push(`${file} : illisible ou absent`);
      continue;
    }
    diff(committed, produced, file, problems);
  }
  if (problems.length) {
    console.error("Les fixtures commitées ne correspondent plus au prototype :");
    for (const p of problems.slice(0, 20)) console.error(`  ${p}`);
    if (problems.length > 20) console.error(`  … et ${problems.length - 20} autres écarts.`);
    process.exit(1);
  }
  console.log(`Fixtures conformes au prototype (tolérance ${TOLERANCE}).`);
} else {
  mkdirSync(DIR, { recursive: true });
  for (const [file, produced] of OUTPUTS) {
    writeFileSync(file, `${JSON.stringify(produced, null, 1)}\n`);
  }
  console.log(`${geometry.length} cas de géométrie et ${kinematics.length} cas de cinématique écrits.`);
}
