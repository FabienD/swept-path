# swept-path — contexte projet

Simulateur de manœuvre d'entrée : vérifier qu'un véhicule peut franchir un
passage étroit (portail, entrée de cour, box de parking) et en combien de
manœuvres.

En français, le domaine s'appelle **épure de giration**. `swept path` est le
terme anglais consacré. Utiliser l'anglais dans le code et les identifiants,
le français dans l'interface.

## État actuel

Le prototype monofichier d'origine a été retiré du dépôt : le portage est
fait, et ce qu'il garantissait est conservé sous forme de vecteurs. Les 800
cas de `crates/swept-core/tests/fixtures/` viennent de lui, et `golden.rs`
compare le noyau à ces valeurs à 1e-12 près à chaque CI. C'est le seul oracle
indépendant du noyau avec celui de Reeds-Shepp : les autres tests comparent le
code à des valeurs écrites par la même main que lui.

## Décisions déjà prises

- **Licence** : AGPL-3.0 pour l'application. Si le noyau géométrique est
  publié en crate, `MIT OR Apache-2.0` pour lui seul.
- **Données véhicules** : licence distincte du code. Sources = documents
  constructeurs uniquement, jamais d'agrégateur (droit *sui generis* des
  bases de données, directive 96/9/CE). Provenance stockée champ par champ.
- **Cible technique** : noyau géométrique + solveur en Rust compilé en Wasm,
  interface et rendu SVG en TypeScript. Frontière étroite : on passe une
  description de scène, on récupère un tableau de poses. Calcul dans un
  Web Worker — le prototype gelait l'onglet, ce qui était le défaut à ne pas
  reproduire.
- **Base véhicules** : fichier statique JSON chargé une fois et filtré côté
  client. Pas de backend, pas de base de données.

## Modèle géométrique

Repère : origine au milieu du passage, `y = 0` au nu extérieur du mur,
`y > 0` vers la cour, `x` le long de la voie.

Le véhicule suit un modèle cinématique bicyclette. État = pose de l'essieu
arrière `(x, y, θ)`. Intégration à courbure constante par segment :

```
θ₁ = θ + κ·ds
x += R·(sin θ₁ − sin θ)      avec R = 1/κ
y −= R·(cos θ₁ − cos θ)
```

`ds < 0` pour la marche arrière. `κ = 0` → segment droit.

Paramètres véhicule : empattement, porte-à-faux avant, porte-à-faux arrière
(déduit), largeur caisse, largeur rétros déployés, largeur rétros rabattus,
rayon de braquage minimal.

Les rétroviseurs sont modélisés comme le point le plus large, à hauteur
d'essieu avant. C'est presque toujours le point critique.

Obstacles = rectangles orientés `(cx, cy, angle, hw, hh)`. Collision =
échantillonnage de l'enveloppe véhicule contre chaque rectangle, plus test
inverse des coins d'obstacle dans le rectangle véhicule (un coin de pilier
peut se trouver à l'intérieur de la caisse sans qu'aucun point échantillonné
ne soit dans l'obstacle).

## Résultats de référence à ne pas perdre

Ils servent de tests de non-régression. **Attention à leur statut** : ces
valeurs ont été produites pendant le prototypage sous Claude Desktop, sans
mesure indépendante. Deux d'entre elles se sont révélées fausses lors du
portage. Chacune est désormais marquée *vérifié* ou *corrigé*.

- **Tolérance angulaire — corrigé.** L'ancienne note affirmait qu'au-dessus
  d'une « largeur critique » `√(w² + L²)`, *tout* angle d'approche passe.
  C'est faux : un véhicule de largeur `w` franchissant un couloir de
  profondeur `L` à un écart `α` de la perpendiculaire occupe

  ```
  emprise = w / cos α  +  L · tan α
  ```

  qui croît sans borne quand `α → 90°`. Aucune ouverture ne fait donc passer
  tous les angles. La tolérance est la solution de `emprise = W`, et le noyau
  la reproduit à moins de 3° près sur quatre largeurs d'ouverture
  (`crates/swept-solver/tests/reference_results.rs`).
- **Battants à 90°, axe à mi-profondeur d'un pilier de 55 cm — corrigé.**
  L'ancienne note donnait une tolérance d'environ 4° ; mesure et géométrie
  concordent sur **environ 14°**. La conclusion qualitative tient : les
  vantaux réduisent la tolérance de plus de moitié, et rabotent aussi le
  passage libre de 2,40 m à environ 2,20 m puisque chaque axe est en retrait
  de 5 cm.
- **Coulissant — vérifié.** Le couloir se limite à l'épaisseur de pilier
  (0,55 m dans la scène de référence).
- L'angle maximal d'ouverture d'un vantail dépend de l'écartement de l'axe
  au nu intérieur du pilier **et** de sa profondeur dans le pilier. Axe à
  mi-profondeur, écartement 5 cm → 91°. Axe reporté sur la face côté cour →
  118°. Mais chaque centimètre d'écartement coûte deux centimètres de
  passage libre : au-delà de ~120° d'ouverture le gain est nul et la perte
  de largeur devient dominante.
- **Conclusion principale du projet** : la largeur du passage domine tout.
  Multiplier les manœuvres n'achète pas de marge, parce que le plafond
  théorique vaut `(W − w) / 2` quelle que soit la trajectoire.

## Défauts hérités du prototype

À corriger, pas à reproduire.

1. Calcul synchrone qui gèle l'onglet. → Web Worker.
2. Le planificateur multi-manœuvres est un hybrid A* discrétisé (primitives
   de 90 cm, cap 6°, position 18 cm). Il rate les solutions à marge
   centimétrique. Un résultat positif est fiable — la trajectoire est
   vérifiée en collision. Un résultat négatif ne prouve rien. Le passage en
   Rust doit permettre 20 cm et 1°.
3. Le mode multi était capable de rendre un résultat pire que le mode simple.
   Corrigé en amorçant par la recherche exacte à un mouvement ; garder cette
   propriété : **le multi ne doit jamais être moins bon que le simple**.
4. La scène est supposée symétrique autour de `x = 0`. À généraliser : les
   deux montants doivent être positionnés indépendamment.
5. Modèle 2D pur : une portière s'ouvre au-dessus d'une bordure basse. Il
   faut un attribut de hauteur sur les obstacles (`full` / `low`).

## Conventions

- Interface en français, code et identifiants en anglais.
- Toutes les longueurs en mètres, angles en radians dans le noyau, en degrés
  seulement à l'affichage.
- Ne jamais afficher une marge sans indiquer d'où elle vient (recherche
  exacte ou heuristique) : l'outil rend des valeurs au centimètre, la
  confiance affichée fait partie du résultat.
