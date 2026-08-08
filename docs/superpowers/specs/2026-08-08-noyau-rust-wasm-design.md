# Lot 1 — Noyau Rust/Wasm et interface TypeScript

*Design validé le 8 août 2026.*

Refonte du prototype `prototype/index.html` en une application structurée : noyau
géométrique et solveurs en Rust compilés en WebAssembly, interface TypeScript,
calcul dans un Web Worker, hébergement statique sur Vercel.

## 1. Objectif et périmètre

Le prototype est un fichier unique de 859 lignes en JavaScript sans dépendance.
Il fonctionne, il sert de référence de comportement, mais il n'est ni testé, ni
documenté, ni déployable. Le lot 1 livre la même valeur fonctionnelle dans une
architecture qui supporte les lots suivants.

### Dans le périmètre

Tout ce que fait le prototype aujourd'hui : scène d'entrée de cour, portail
battant ou coulissant, recherche exacte d'une entrée à un mouvement,
planification jusqu'à quatre manœuvres, chaussée minimale, vue en plan avec
trajectoire, positions fantômes, bandes de proximité et curseur de parcours.

S'y ajoutent les corrections des défauts qui relèvent d'un choix d'architecture,
parce que les reproduire pour les défaire ensuite coûte plus cher que de les
faire correctement d'emblée :

| Défaut connu | Traitement au lot 1 |
|---|---|
| 1. Calcul synchrone qui gèle l'onglet | Calcul dans un Web Worker, par construction |
| 2. Planificateur trop grossier | Discrétisation paramétrable, défauts affinés (§ 4.2) |
| 3. Le multi pouvait être pire que le simple | Invariant vérifié par test de propriété |
| 4. Scène supposée symétrique autour de `x = 0` | Montants gauche et droit indépendants dans le modèle |

### Hors périmètre

- **Défaut 5** — hauteur des obstacles (`full` / `low`) : lot 2.
- Enveloppe d'usage, dégagements et les trois verdicts de `docs/SPEC.md` : lot 3.
- Box de parking, mur de fond, allée bornée : lot 4.
- Schéma de base véhicules à provenance et validation à l'import : lot 5,
  indépendant, peut avancer en parallèle.

Le lot 1 conserve donc les six véhicules en dur du prototype, portés tels quels
dans une constante Rust documentée comme provisoire. `data/vehicles.json` n'est
pas encore consommé.

### Critères d'acceptation

1. Les quatre résultats de référence du `CLAUDE.md` sont reproduits par des
   tests automatisés (§ 6.2).
2. Aucune interaction ne bloque le thread principal, quelle que soit la durée
   du calcul.
3. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`,
   `cargo doc` et les tests TypeScript passent en intégration continue.
4. L'application est déployée et accessible en production sur Vercel.
5. Chaque constante numérique du noyau est nommée et documentée par sa
   justification et sa provenance.

## 2. Décisions actées

| Décision | Choix | Raison |
|---|---|---|
| Périmètre | Parité fonctionnelle + défauts structurels corrigés | Ne pas reproduire ce qu'on devra défaire |
| Découpage Rust | Trois crates | Licences distinctes, dépendances dirigées vers l'intérieur |
| Interface | TypeScript nu + Vite + Tailwind | Un SVG régénéré en bloc ne tire rien d'un moteur de diff |
| Oracle de test | Mixte selon le niveau | Figer le bas niveau sans figer les défauts des solveurs |
| Flux de PR | Séquentiel sur `main`, une PR ouverte à la fois | Relecteur unique, pas de rebase en cascade |
| Rendu | Liste de primitives + backend SVG | Rend le moteur de rendu remplaçable pour un coût marginal |

## 3. Architecture

Trois crates Rust, un paquet web, les dépendances ne vont que vers l'intérieur.

```
web/  ──▶  swept-wasm  ──▶  swept-solver  ──▶  swept-core
(TS)       (AGPL)           (AGPL)             (MIT OR Apache-2.0)
```

### 3.1 `swept-core` — le domaine

Aucune dépendance externe. C'est ce qui le rend publiable sous une licence
distincte de l'application, comme prévu dans le `CLAUDE.md`, et ce qui garantit
qu'aucun détail de sérialisation ou de plateforme ne remonte dans la géométrie.

| Module | Responsabilité |
|---|---|
| `units` | `Radians(f64)` en newtype. Les longueurs restent des `f64` en mètres. On ne protège que la confusion réellement constatée : degré contre radian. Les degrés n'existent qu'à l'affichage. |
| `geometry` | `Point`, `Obb { center, angle, half_width, half_height }`, ses coins, la distance d'un point à un OBB, le recouvrement de deux OBB par axes séparateurs. |
| `vehicle` | `Vehicle` validé à la construction : empattement, porte-à-faux avant, porte-à-faux arrière déduit, largeur caisse, largeur rétros déployés et rabattus, rayon de braquage minimal. Échantillonnage de l'enveloppe. |
| `scene` | `Scene` : montants **gauche et droit positionnés indépendamment**, profondeur et largeur de pilier, murets, portail, trottoir, bateau, chaussée, sens d'arrivée. Génération des obstacles. Angle d'ouverture maximal d'un vantail. |
| `kinematics` | `Pose { x, y, heading }`, intégration à courbure constante, arcs et segments droits. |
| `clearance` | Marge d'une pose contre la scène, incluant le test inverse des coins d'obstacle dans le rectangle véhicule. |

Le modèle cinématique est le bicyclette, l'état étant la pose de l'essieu
arrière. Intégration par segment à courbure constante `κ` sur une longueur `ds`,
négative en marche arrière :

```
θ₁ = θ + κ·ds
x += R·(sin θ₁ − sin θ)      avec R = 1/κ
y −= R·(cos θ₁ − cos θ)
```

Repère : origine au milieu du passage, `y = 0` au nu extérieur du mur, `y > 0`
vers la cour, `x` le long de la voie.

### 3.2 `swept-solver` — les stratégies de recherche

Dépend de `swept-core`, ignore tout du Web.

- **`exact`** — recherche exhaustive d'une entrée à un mouvement par balayage
  du rayon de braquage, de la position latérale de départ et du point
  d'engagement. Exhaustive sur sa grille, donc fiable : c'est la référence.
- **`multi`** — A\* hybride sur `(x, y, θ, sens)`. Coût principal : le nombre de
  changements de sens ; coût secondaire : la longueur parcourue. Heuristique :
  distance à la zone cible plus erreur de cap. **Toujours amorcé par `exact`** :
  si une solution à un mouvement existe, elle est retournée sans planification.
- **`min_road`** — dichotomie sur la largeur de chaussée avec `exact`.

Deux propriétés de conception y comptent plus que le reste.

**Le noyau n'a pas d'horloge.** Le budget de recherche s'exprime en nœuds
explorés et en pas de discrétisation, jamais en millisecondes. Les mêmes entrées
produisent donc toujours le même résultat, condition nécessaire pour que les
tests soient reproductibles — le prototype, lui, s'arrête au bout de 2,2 s par
profondeur et rend un résultat qui dépend de la machine. L'annulation par
l'utilisateur se fait en terminant le Worker, hors du domaine.

**Chaque résultat porte sa provenance.** Le `CLAUDE.md` interdit d'afficher une
marge sans indiquer d'où elle vient ; le type le rend impossible à oublier :

```rust
pub enum Confidence {
    /// Balayage exhaustif de la grille : un échec prouve l'absence de solution
    /// sur cette grille.
    Exact,
    /// Recherche heuristique : un succès est vérifié en collision, un échec ne
    /// prouve rien.
    Heuristic { budget_exhausted: bool },
}

pub struct Maneuver {
    pub poses: Vec<DirectedPose>,  // pose + sens de marche
    pub min_clearance: f64,        // mètres
    pub moves: u8,
    pub confidence: Confidence,
}

pub enum Outcome {
    Found(Vec<Maneuver>),          // une alternative par nombre de manœuvres
    NotFound { budget_exhausted: bool },
}
```

L'absence de solution n'est pas une erreur mais un résultat : cela préserve la
distinction que le prototype affiche déjà, *aucune solution trouvée* ne
signifiant pas *aucune solution n'existe*.

La progression passe par un trait `Progress` injecté, avec une implémentation
vide par défaut. Le solveur ne connaît pas `postMessage`.

### 3.3 `swept-wasm` — l'adaptateur

`serde`, `serde-wasm-bindgen`, `wasm-bindgen`. Types de transfert, conversions
vers et depuis le domaine, traduction des erreurs. Rien d'autre : aucune règle
métier ne vit ici.

## 4. Paramètres et constantes

### 4.1 Aucune constante nue

Le prototype est truffé de valeurs sans justification : `+0.6` dans `deep()`,
`+0.45` dans `goalDepth()`, le facteur `2.2` de l'heuristique A\*, le `+0.04`
appliqué à la largeur des rétros rabattus, les bandes `0.50 / 0.25 / 0.10`.
Personne ne sait aujourd'hui lesquelles sont mesurées, lesquelles sont des
marges de confort et lesquelles sont arbitraires.

Chacune devient une `const` nommée, documentée par sa **justification** et sa
**provenance**. Celles qu'on ne sait pas justifier sont explicitement marquées
comme arbitraires et à revalider — c'est le principal savoir que le portage
risque de perdre.

### 4.2 Discrétisation

Le `CLAUDE.md` décrit la grille actuelle — « primitives de 90 cm, cap 6°,
position 18 cm » — et fixe la cible : « le passage en Rust doit permettre 20 cm
et 1° ». Les deux valeurs citées correspondent aux deux premiers paramètres.
**Interprétation retenue** : primitives de 20 cm et pas de cap de 1°.

La résolution de la grille de visite suit, mais pas par simple proportionnalité :
elle doit rester nettement plus fine que la longueur d'une primitive, sinon deux
états séparés par un mouvement complet retombent dans la même cellule et l'A\*
en élimine un à tort. Le prototype tient ce rapport à un cinquième
(18 cm pour 90 cm) ; on retient un rapport de un tiers, plus prudent, soit 6 cm
pour des primitives de 20 cm.

```rust
pub struct Discretisation {
    pub primitive_length: f64,  // m, défaut 0.20 (prototype : 0.90)
    pub heading_step: Radians,  // défaut 1° (prototype : 6°)
    pub position_step: f64,     // m, défaut 0.06 (prototype : 0.18)
}
```

Ces valeurs multiplient le nombre de nœuds par un facteur important. Le coût
réel est à mesurer dès que l'A\* fonctionne ; si le budget explose, la réponse
est un raffinement progressif — recherche grossière puis affinage local — et non
un retour à une grille grossière. Le paramétrage étant explicite, cette mesure
est un test de performance, pas une réécriture.

## 5. Frontière Wasm et flux de données

Frontière étroite, trois fonctions :

```
solve(SolveRequest) -> SolveResponse
min_road(MinRoadRequest) -> MinRoadResponse
max_gate_angle(SceneDto) -> f64   // radians, comme partout hors affichage
```

Échange en JSON via `serde`. Une trajectoire compte quelques milliers de points
au plus : le coût de sérialisation est négligeable devant celui du calcul, et le
gain d'un format binaire ne justifierait pas sa complexité.

```
UI (thread principal)  ──postMessage(SolveRequest)──▶  Worker
                                                         │ appelle le Wasm
UI ◀──postMessage(Progress | SolveResponse | Error)──────┘
```

L'UI ne connaît que le Worker ; le Worker ne connaît que le Wasm. Le gel de
l'onglet disparaît par construction. Une nouvelle recherche lancée pendant qu'une
autre tourne termine le Worker courant et en crée un neuf — c'est le mécanisme
d'annulation, et il ne demande rien au noyau.

## 6. Erreurs et tests

### 6.1 Erreurs

Le noyau ne panique pas : `Result<T, ValidationError>`, où `ValidationError`
nomme le champ fautif et la règle violée. Les onze contrôles de `badInputs()`
deviennent des variantes typées — porte-à-faux avant supérieur à la différence
entre longueur et empattement, largeur de rétros inférieure à la largeur de
caisse, dimension négative ou nulle, etc.

**Les libellés français vivent exclusivement dans l'interface.** Le noyau renvoie
des codes et des identifiants ; la traduction est une table côté TypeScript.
C'est ce qui permet de tenir « code en anglais, interface en français » sans que
le domaine porte de la langue.

`console_error_panic_hook` est activé en debug uniquement.

### 6.2 Oracle de test

L'oracle diffère selon le niveau, parce que les exigences diffèrent.

**Bas niveau — le comportement ne doit pas changer.** Un harnais Node jetable
dans `tools/extract-golden/` charge le prototype, exécute l'intégration
cinématique, les distances point-OBB et le calcul de marge sur un échantillon
de configurations, et exporte les résultats en fixtures JSON. Le Rust doit les
reproduire à 1e-9 près.

**Solveurs — le comportement va justement changer**, puisque la discrétisation
s'affine et que la scène se généralise. Couverture par `proptest` sur les
invariants :

- toute trajectoire retournée est sans collision sur tout son parcours ;
- la marge minimale annoncée est cohérente avec la trajectoire retournée ;
- **le multi n'est jamais moins bon que le simple** (défaut 3) ;
- si `W ≥ √(w² + L²)`, tout angle d'approche passe (largeur critique).

**Résultats de référence** du `CLAUDE.md`, en tests explicites :

1. Largeur critique : au-dessus de `√(w² + L²)`, tout angle passe ; en dessous,
   la tolérance angulaire s'effondre.
2. Battants ouverts à 90°, axe à mi-profondeur d'un pilier de 55 cm : couloir
   d'environ une longueur de vantail, tolérance d'environ 4°.
3. Coulissant : couloir égal à la seule épaisseur de pilier.
4. Angle maximal d'ouverture : axe à mi-profondeur avec 5 cm d'écartement → 91° ;
   axe reporté sur la face côté cour → 118°. Au-delà d'environ 120°, le gain est
   nul et la perte de largeur devient dominante.

**Interface** : Vitest sur le rendu — la liste de primitives est une fonction
pure, donc testable sans DOM — et sur le store. **Frontière Wasm** : test
d'intégration exécuté en Node via `wasm-bindgen-test`. Pas de tests bout en bout
au lot 1.

## 7. Interface

TypeScript nu, Vite, sortie statique.

**Style : Tailwind CSS 4**, intégré par `@tailwindcss/vite` et configuré en CSS
via `@theme` — la version 4 n'utilise plus de `tailwind.config.js`. Pas de
bibliothèque de composants : shadcn/ui et ses équivalents imposent React ou
Svelte, et l'inventaire des contrôles ne le justifie pas. L'interface compte
dix-neuf champs numériques, cinq listes déroulantes, deux curseurs, une case à
cocher, deux boutons et une liste d'alternatives sélectionnables — que des
éléments natifs. Radix et ses portages apportent l'accessibilité des
interactions complexes, boîtes de dialogue, popovers, listes combinées, dont
aucune n'apparaît ici.

La question se rouvrira si les lots 3 et 4 amènent de vrais composants
composites — comparaison de scénarios, paramétrage des dégagements. Le rendu
étant isolé derrière la liste de primitives, ce changement resterait confiné à
la couche de contrôles.

Le rendu se fait en deux temps. Une fonction pure produit une **liste de
primitives de dessin** — segments, polygones, arcs, étiquettes, chacun portant sa
bande de proximité — et un backend la traduit. Le lot 1 fournit un seul backend,
SVG. Cette indirection coûte peu et rend le moteur remplaçable : un backend
Canvas ou WebGL, voire une vue 3D au lot 2 quand les obstacles gagneront une
hauteur, se branche sans toucher à la logique qui décide *quoi* dessiner. SVG
reste le défaut pour la vue en plan, parce qu'il donne l'export et l'impression
d'un plan coté — ce qu'un utilisateur veut emporter pour vérifier sa cour.

Bandes de proximité, reprises du prototype : au-delà de 50 cm, 25 à 50, 10 à 25,
en dessous de 10. Trait plein en marche avant, pointillé en marche arrière,
cercle aux inversions. Indicateurs de distance parcourue sous 25 cm et sous
10 cm.

L'état applicatif tient dans un petit store observable : paramètres du
formulaire, résultat courant, alternative sélectionnée, position du curseur.

## 8. Structure du dépôt

```
swept-path/
├── crates/
│   ├── swept-core/        # domaine, MIT OR Apache-2.0
│   ├── swept-solver/      # stratégies de recherche, AGPL-3.0
│   └── swept-wasm/        # adaptateur wasm-bindgen, AGPL-3.0
├── web/                   # Vite + TypeScript, AGPL-3.0
├── tools/extract-golden/  # harnais jetable d'extraction des fixtures
├── data/                  # base véhicules (consommée au lot 5)
├── docs/
│   ├── SPEC.md
│   ├── ALGORITHME.md      # le simulateur expliqué de bout en bout
│   └── superpowers/specs/
└── prototype/             # gelé, référence de comportement
```

## 9. Documentation

Trois niveaux, chacun avec un rôle distinct :

- **API** (`///`) — le contrat de chaque élément public ;
- **module** (`//!`) — le modèle mathématique et le repère supposé ;
- **narratif** (`docs/ALGORITHME.md`) — le simulateur de bout en bout, lisible
  sans ouvrir le code.

`#![deny(missing_docs)]` sur les trois crates, vérifié en intégration continue :
la documentation manquante casse le build, elle ne dérive pas.

Les **doctests sont le levier principal**. `cargo test` exécute les exemples de
la documentation : un exemple numérique dans la doc de l'intégration cinématique
est à la fois l'explication et sa preuve, et une documentation devenue fausse
apparaît comme un test rouge.

## 10. Dépendances

Politique : dernière version stable au moment où la dépendance est ajoutée,
`Cargo.lock` et `package-lock.json` commités, mises à jour groupées par
Dependabot et validées par la CI. `swept-core` n'ayant aucune dépendance
externe, la surface à maintenir se limite aux couches extérieures.

Versions constatées le 8 août 2026 :

| Rust | | TypeScript | |
|---|---|---|---|
| toolchain | 1.97.1, édition 2024 | typescript | 7.0.2 |
| wasm-bindgen | 0.2.127 | vite | 8.2.1 |
| serde | 1.0.229 | vitest | 4.1.10 |
| serde_json | 1.0.151 | tailwindcss | 4.3.3 |
| | | @tailwindcss/vite | 4.3.3 |
| serde-wasm-bindgen | 0.6.5 | | |
| thiserror | 2.0.20 | | |
| proptest | 1.11.0 | | |
| wasm-bindgen-test | 0.3.77 | | |

Deux points de vigilance. **TypeScript 7** est la réécriture native du
compilateur : gain de vitesse important, mais version majeure jeune, dont la
compatibilité avec l'outillage doit être vérifiée dès la première PR web —
repli sur la dernière 5.x si nécessaire. **Vite 8** est également récent.
`rust-toolchain.toml` épingle la version de Rust pour que CI et postes de travail
compilent à l'identique.

À installer, absents du poste : la cible `wasm32-unknown-unknown` et
`wasm-pack`. Par ailleurs, `nvm` ne se charge pas dans les shells non
interactifs : les scripts et la CI doivent référencer Node explicitement.

## 11. Méthode de travail

**Une PR = une capacité vérifiable, pas une couche.** Découper horizontalement
— toutes les entités, puis tous les cas d'usage — produit des PR que personne ne
peut relire, parce qu'une couche ne se juge pas sans son usage. Chaque PR
traverse donc les couches dont elle a besoin et livre un comportement
observable.

- **200 à 400 lignes de diff**, tests inclus. C'est un plafond, pas un quota.
- **TDD à l'intérieur de la PR** : premier commit, le test qui échoue, avec la
  sortie d'échec citée dans le message ; commits suivants, l'implémentation
  minimale qui le fait passer. Le relecteur qui lit le premier commit sait ce
  qu'on cherchait à garantir.
- **Une seule PR ouverte à la fois**, branchée sur `main`, mergée avant
  d'ouvrir la suivante.
- **`main` reste vert et déployable.**
- **Description en trois points** : ce que ça change, pourquoi maintenant,
  comment le vérifier à la main — plus le lien vers la ligne du prototype ou du
  `CLAUDE.md` qui sert de référence. C'est ce qui rend relisable la revue d'un
  calcul géométrique sans refaire les mathématiques.

## 12. Découpage prévisionnel en PR

| # | Livre | Vérifiable par |
|---|---|---|
| 0 | Committer l'existant, `.gitignore`, gel du prototype | l'arbre est propre |
| 1 | Workspace Cargo, `rust-toolchain.toml`, CI qualité | la CI passe sur un test trivial |
| 2 | `units` et `geometry` : points, OBB, coins, distance | doctests et fixtures golden |
| 3 | Recouvrement OBB par axes séparateurs | cas du coin de pilier à l'intérieur de la caisse |
| 4 | `vehicle` : validation et enveloppe échantillonnée | les règles de `badInputs()` |
| 5 | `scene` : montants indépendants, portail, obstacles | angle maximal de vantail (référence 4) |
| 6 | `kinematics` : pose et intégration à courbure constante | fixtures golden à 1e-9 |
| 7 | `clearance` : marge d'une pose | marge connue sur une scène figée |
| 8 | `exact` : recherche à un mouvement, `Confidence` | **références 1, 2 et 3** |
| 9 | `min_road` : dichotomie de chaussée | encadrement de la largeur trouvée |
| 10 | `multi` : A\* hybride, budget en nœuds, trait `Progress` | invariant multi ≥ simple |
| 11 | `swept-wasm` : les trois fonctions, types de transfert | test d'intégration en Node |
| 12 | Coque web (Vite, Tailwind), Worker, appel de bout en bout | verdict textuel affiché, onglet réactif |
| 13 | Déploiement Vercel prébuild, preview par PR | l'URL de production répond |
| 14 | Liste de primitives et backend SVG, scène statique | instantané du rendu |
| 15 | Trajectoire, bandes, positions fantômes, curseur | parité visuelle avec le prototype |
| 16 | Alternatives multi-manœuvres, indicateurs d'alerte | les quatre alternatives s'affichent |
| 17 | `docs/ALGORITHME.md` | relecture |

Le jalon qui compte est la **PR 8** : à partir de là, le noyau Rust est prouvé
équivalent au prototype sur les seuls résultats documentés. La **PR 13** met la
chaîne de déploiement à l'épreuve tôt, avant que l'interface ne représente un
investissement important.

## 13. Intégration continue et déploiement

GitHub Actions construit tout — `fmt`, `clippy -D warnings`, `test`, `doc`,
`wasm-pack build`, tests TypeScript — puis déploie sur Vercel en `--prebuilt`.
Vercel ne compile donc jamais de Rust : il sert un dossier statique. Une preview
par PR, la production sur `main`.

## 14. Risques

**Le prototype comme oracle.** Ses valeurs peuvent être fausses. On ne fige en
fixtures que ce qui est vérifiable indépendamment par le calcul — intégration
cinématique, distances géométriques — et jamais les sorties de solveurs.

**Coût de la discrétisation fine.** Passer de 90 cm et 6° à 20 cm et 1°
multiplie fortement l'espace de recherche. Mesure dès la PR 10 ; en cas de
dépassement, raffinement progressif plutôt que retour à une grille grossière.

**Constantes non justifiables.** Certaines valeurs du prototype resteront
inexpliquées. Elles seront portées telles quelles, marquées comme arbitraires,
et listées dans `docs/ALGORITHME.md` comme dette documentaire assumée.

**Majeures jeunes côté TypeScript.** TypeScript 7 et Vite 8 sont récents ; repli
documenté sur les dernières mineures de la génération précédente si l'outillage
résiste.
