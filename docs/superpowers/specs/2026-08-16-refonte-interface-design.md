# Refonte de l'interface — design

## Pourquoi

L'interface actuelle est un formulaire de vingt champs suivi d'un dessin. Elle
rend des résultats justes au centimètre dans une présentation qui ne dit ni ce
qu'il faut regarder d'abord, ni ce que valent les chiffres affichés.

Deux manques précis :

1. **La marge n'a pas d'échelle.** « 4,5 cm de marge » ne signifie rien sans
   son plafond. Or ce plafond, l'outil le connaît exactement : `(W − w) / 2`,
   soit 13,1 cm sur le portail de référence. Aucune trajectoire ne fera mieux.
2. **La manœuvre ne se voit pas.** Un curseur de position existe, mais il faut
   savoir qu'il est là et le tirer soi-même. Ce que l'utilisateur veut voir —
   la voiture qui rentre, et l'endroit où elle doit s'arrêter pour changer de
   sens — n'est jamais joué.

La direction visuelle retenue emprunte son vocabulaire à la voirie : bande
oblique jaune et noir, ligne d'axe discontinue, typographie de panneau. Elle a
été choisie sur maquette contre deux autres.

## Ce qui ne change pas

- Le noyau, le solveur, le Web Worker, la frontière Wasm. Aucun `crates/`
  n'est touché.
- La sémantique du tracé : couleur = proximité, pointillé = marche arrière,
  rond = changement de sens, violet = surplomb.
- Les règles du projet : Tailwind écrit à la main, interface en français,
  identifiants en anglais, jamais de marge affichée sans sa provenance.

## 1. Structure : deux volets

```
┌──────────────────────────────────────────────────────┐
│ ▨▨▨▨▨ bande oblique                                  │
├────────────────┬─────────────────────────────────────┤
│ Réglages       │ Verdict + jauge de marge            │
│ (262 px)       │                                     │
│                │ ┌─────────────────────────────────┐ │
│ véhicule       │ │                                 │ │
│ passage        │ │            le plan              │ │
│ chaussée       │ │                                 │ │
│ trottoir       │ ├─────────────────────────────────┤ │
│ piliers        │ │ ▶  AV/AR  ═══════════  marge    │ │
│ portail        │ └─────────────────────────────────┘ │
│ [Calculer]     │                                     │
│ ▸ réglages fins│ alternatives · statistiques         │
└────────────────┴─────────────────────────────────────┘
```

Sous 820 px, une seule colonne : le plan d'abord, les réglages ensuite.

**Le dépliant « réglages fins »** contient tout ce qui n'est pas une des six
mesures qu'on prend soi-même au mètre : rétroviseurs, garde au sol, empattement,
longueur, largeurs, vantaux, bateau, hauteur de bordure. Un `<details>` natif,
fermé par défaut, ouvert automatiquement si un de ses champs est signalé en
erreur — sans quoi le message « une mesure manque, signalée en rouge » pointerait
un champ invisible.

## 2. Palette

Fond `#131310`, panneaux `#1a1a15`, filets `#2c2c22`, texte `#f2f2ec`, texte
secondaire `#8f8f80`, accent `#f7d708`.

Les cinq rôles du plan sont relevés pour le fond sombre. Ce ne sont pas des
retouches à l'œil : les valeurs actuelles ont été choisies pour un fond
`stone-50` et deux d'entre elles frôlent le seuil sur fond sombre.

| rôle | actuel | retenu | contraste /fond |
|---|---|---|---|
| `--color-band-clear` | `#2c6e8f` | `#4da6d4` | 3,46 → 7,16 |
| `--color-band-watch` | `#d9a400` | `#e3b23c` | 8,58 → 9,92 |
| `--color-band-close` | `#dd6b1f` | `#f0813a` | 5,74 → 7,33 |
| `--color-band-tight` | `#c42b1c` | `#f2564b` | 3,44 → 5,75 |
| `--color-overhang`   | `#7c3aed` | `#a98bfa` | 3,42 → 7,19 |

Le contraste n'était pourtant pas le vrai problème — toutes passaient le seuil
de 3,0. Le problème est la **distinguabilité** : la bande « vigilance » actuelle
est à ΔE 23,8 de l'accent jaune, sous le seuil de 25 en deçà duquel deux teintes
de signalisation se confondent quand on ne les compare pas côte à côte. Un trait
« vigilance » dans le plan et un chiffre d'accent dans l'interface se liraient
comme la même couleur. Les valeurs retenues portent cet écart à 27,2, et la
paire de bandes la plus proche entre elles à 28,1.

Ces cinq valeurs restent des tokens dans `web/src/style.css`. Le renderer ne
code aucune couleur en dur ; cette règle tient.

## 3. Le plan et son tracé

**La jauge de marge.** Sous le verdict, le chiffre de la marge dans le passage,
posé sur une échelle qui va de 0 au plafond géométrique `(W − w) / 2`. La zone
sous 1,5 cm est teintée — ARBITRAIRE, c'est l'ordre de grandeur en deçà duquel
une marge calculée ne survit pas aux approximations de mesure du portail.

Le noyau ne publie pas ce plafond : il n'apparaît que dans un test de
`crates/swept-wasm/src/dto.rs`, qui vérifie qu'aucune marge annoncée ne le
dépasse. Il n'a pas besoin de le publier. `readRequest()` lit déjà `opening` et
`mirror_width`, cette dernière tenant compte de l'état des rétroviseurs ; le
plafond se calcule donc côté interface à partir des mêmes valeurs que celles
envoyées au solveur. C'est ce qui permet de ne toucher aucun crate.

**Le verdict devient une phrase.** « Ça passe. De justesse. » plutôt que
« Entrée possible en 2 manœuvres ». Le détail — nombre de manœuvres, provenance
de la valeur — reste sous la phrase, en clair. La provenance ne disparaît pas :
`confidenceLabel` continue d'accompagner chaque marge.

**Le tracé est dessiné deux fois.** Le trajet complet en sourdine (opacité
réduite), puis la portion déjà parcourue en pleine couleur. On voit où la
trajectoire va, et où on en est de sa lecture.

**Les fantômes s'accumulent.** Aujourd'hui `pathToPrimitives` place quatre
fantômes fixes sur tout le trajet. Ils deviennent régulièrement espacés le long
du parcours et n'apparaissent qu'une fois dépassés. À la fin de la lecture, la
figure obtenue est l'épure.

## 4. La lecture animée

### Le mécanisme

Le store porte déjà `position` (0 à 1) et `draw()` reconstruit le plan à chaque
changement. Animer, c'est faire avancer cette valeur — il n'y a ni moteur
d'animation à introduire, ni keyframes à générer.

### Avancer sur la distance, pas sur l'index

`position` est aujourd'hui convertie en index de pose par une règle de trois.
Cela suppose des poses équidistantes, ce qu'elles ne sont pas :
`sample_arc` traite `step` comme une **borne supérieure**
(`count = ceil(distance / step)`, puis répartition uniforme dans le segment).
L'espacement est donc constant à l'intérieur d'un segment mais varie de l'un à
l'autre, entre `step / 2` et `step`.

Une lecture linéaire en index ralentirait sur les segments courts et
accélérerait sur les longs. L'animation doit donc avancer sur la **distance
cumulée** : on construit une fois par manœuvre la table des longueurs cumulées,
et `position` s'y résout par recherche dichotomique.

Cette table, la conversion `position → index`, le calcul de la durée et le
placement des pauses vont dans un module pur, `web/src/render/playback.ts`,
testable sans DOM comme le reste du dossier. Seule l'horloge — la boucle
`requestAnimationFrame` qui pousse `position` dans le store — reste dans
`main.ts`.

Cela corrige au passage un défaut existant du curseur : glisser à 50 % ne donne
pas aujourd'hui le milieu du trajet, mais la pose médiane en index.

### Le rythme

- Durée `distance / 1,2` secondes, bornée à `[4 s, 20 s]`. Proportionnelle au
  trajet, pour que cinq manœuvres ne durent pas autant que deux, mais bornée
  pour qu'elles restent regardables. ARBITRAIRE, les trois valeurs.
- Arrêt de 400 ms à chaque changement de sens. C'est le moment que l'on veut
  voir, et qu'une lecture continue escamote. ARBITRAIRE.
- En fin de lecture, le véhicule reste sur la dernière pose et le bouton
  repasse à ▶ : une nouvelle pression rejoue depuis le début.

### Le sens de marche

La couleur du tracé reste la proximité. Le sens passe sur le véhicule :

- carrosserie cerclée de jaune en marche avant, de blanc en marche arrière ;
- feux de recul allumés en marche arrière — la couleur vient de la voiture
  elle-même, ce qui rend le codage évident sans légende ;
- un badge AV/AR dans la barre, qui double l'information pour qui ne
  distinguerait pas les deux teintes.

`PoseDto.reverse` porte déjà cette donnée, pose par pose.

### La barre

Le curseur existant devient la barre de lecture : progression remplie en jaune,
saisissable. La saisir met la lecture en pause. À droite, la marge de la pose
courante (`PoseDto.clearance`), en direct.

### Mouvement réduit

Sous `prefers-reduced-motion: reduce`, le bouton ne joue rien : il place
directement `position` à 1, ce qui affiche l'épure complète. Le curseur reste
utilisable.

## 5. Découpage

Deux PR, chacune se tenant seule.

**PR 1 — la peau.** Palette, deux volets, dépliant des réglages fins, verdict en
phrase, jauge de marge. Aucune animation. À l'issue, l'interface est celle du
design, avec le curseur d'aujourd'hui.

**PR 2 — la lecture.** Le module pur `render/playback.ts` d'abord — longueurs
cumulées, résolution `position → index`, durée, pauses — puis son branchement :
bouton de lecture, feux de recul et badge, tracé parcouru contre tracé en
sourdine, fantômes progressifs, mouvement réduit.

La correction du curseur vient avec la PR 2, puisqu'elle est le premier usage
de `playback.ts`. Jusque-là le curseur garde son défaut, qui est celui
d'aujourd'hui : la PR 1 ne le rend pas pire.

## Vérifications

Faites, avec ce qu'elles ont donné.

- **La table des longueurs cumulées, une fois par manœuvre plutôt qu'à chaque
  image.** Inquiétude infondée, mesure faite : sur 420 poses elle coûte 5 µs,
  soit 15 µs pour les trois appels d'une image — 0,09 % d'une image de 16,7 ms.
  Laissée telle quelle, sans mémoïsation.
- **Ce qui pesait vraiment était ailleurs.** L'abonnement du store reconstruit
  tout à chaque notification, et la lecture en émet soixante par seconde :
  `renderAlternatives` recréait donc les boutons et rattachait leurs écouteurs
  soixante fois par seconde, sous le pointeur. Un clic pouvait atterrir sur un
  bouton qui venait d'être remplacé. Les parties qui ne dépendent que du choix
  ne sont désormais rebâties que quand le choix change.
- **Le dépliant s'ouvre sur un champ signalé.** Placé dans `flag()` plutôt que
  dans `flagMissing()` : c'est le point unique par lequel passent aussi les
  champs rejetés par la frontière Wasm.
- **Le tracé en sourdine est émis en une seule polyligne**, non redécoupée par
  bande : il ne porte aucune information de proximité, seulement une
  destination.
- **La jauge affiche bien `(W − w) / 2`**, calculé depuis la largeur du
  passage lue comme `right_post.inner_edge_x − left_post.inner_edge_x` — et
  non comme `inner_edge_x × 2`, qui deviendrait faux le jour où les deux
  montants seront posés indépendamment (défaut connu n° 4 du prototype).
- **Hiérarchie visuelle mesurée** plutôt que jugée à l'œil : l'élément de
  décor le plus marqué est à 2,77 de contraste sur le fond du plan, la bande
  la plus discrète à 5,75. Rien du bâti ne concurrence le tracé.

## Hors périmètre

- **Le thème clair**, qui ira avec le lot i18n : le sélecteur de thème a sa
  place à côté de celui de langue et d'unités.
- **L'i18n et le choix d'unités**, lot déjà décidé et spécifié à part.
- **La provenance par champ** de la base véhicules, qui reste à afficher.
- **`jsdom`.** Le projet n'en a pas, et ce lot n'en introduit pas. Toute la
  logique y vit déjà dans des modules purs — `render/`, `state/`, `domain/`,
  tous testés — `main.ts` étant la seule couche DOM et la seule non testée. La
  logique de lecture suit cette règle plutôt que de la contourner : elle va
  dans `web/src/render/playback.ts`, pur et testable sans DOM (voir PR 2).
  Reste hors périmètre le test du DOM lui-même.
