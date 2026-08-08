# Spécification fonctionnelle

## Scènes supportées

### 1. Entrée de cour (implémenté au prototype)

Une voie bordée d'un trottoir, un bateau, un mur percé d'un passage encadré
de deux piliers, une cour libre au-delà.

Paramètres : passage entre piliers, profondeur et largeur des piliers,
épaisseur des murets, type de portail, largeur du trottoir, largeur du
bateau, largeur de chaussée, sens d'arrivée.

Pour un portail battant : longueur et épaisseur de vantail, écartement de
l'axe au nu intérieur du pilier, position de l'axe en profondeur, angle
d'ouverture.

### 2. Box de parking (à faire)

Généralisation de la précédente. Ajouts :

- **Mur de fond** : le véhicule doit tenir entièrement, pas seulement
  franchir un seuil. La condition d'arrivée devient un test d'inclusion dans
  une zone cible, pas un franchissement de frontière.
- **Allée bornée** : un mur en bout d'allée, d'un côté ou des deux. Supprime
  la liberté de recul illimité, qui est aujourd'hui implicite.
- **Ouverture non centrée** : les deux montants sont positionnés
  indépendamment par rapport à l'axe du box. Le sens d'arrivée cesse d'être
  un simple effet miroir et devient une variable à comparer.
- **Parois latérales du box**, distinctes de la largeur d'ouverture.

## Contrainte d'utilisabilité

Distinguer deux enveloppes :

- **enveloppe de collision** = la caisse. S'applique à tout le trajet.
- **enveloppe d'usage** = caisse + dégagements. S'applique à la pose finale
  uniquement.

Dégagements de référence, paramétrables :

| Zone | Minimum | Correct | Confortable |
|---|---|---|---|
| Latéral conducteur | 30 cm | 45 cm | 55–60 cm |
| Latéral passager | 20 cm | 35 cm | 50 cm |
| Arrière (hayon) | 40 cm | 70 cm | 90 cm |

Le solveur doit optimiser la pose finale sous cette double contrainte et
rendre trois verdicts distincts : *ne rentre pas*, *rentre mais inutilisable*,
*rentre et utilisable*. Dans le second cas, indiquer quelle contrainte est
bloquante et de combien se déporter pour la lever.

## Solveurs

### Recherche exacte à un mouvement

Balayage systématique : rayon de braquage, position latérale de départ,
point d'engagement. Exhaustif sur sa grille, donc fiable. C'est la référence.

Sortie : trajectoire, marge minimale, rayon utilisé, écart au bord.

### Planificateur multi-manœuvres

Hybrid A* sur `(x, y, θ, sens)`. Coût = nombre de changements de sens
prioritaire, longueur parcourue en secondaire, heuristique = distance à la
zone cible plus erreur de cap.

Doit **toujours** être amorcé par la recherche exacte : si une solution à un
mouvement existe, elle est retournée sans planification.

Sortie : une alternative par nombre de manœuvres, de 1 à 4, chacune avec sa
marge. Permet de montrer l'arbitrage reprises / confort.

### Chaussée minimale

Dichotomie sur la largeur de chaussée avec la recherche exacte. Répond à
« combien de place me faut-il en face ».

## Restitution

- Vue en plan à l'échelle, trajectoire, positions fantômes, curseur de
  parcours.
- Coloration du trajet par bandes de proximité, façon radar de recul :
  au-delà de 50 cm, 25–50, 10–25, en dessous de 10. Trait plein en marche
  avant, pointillé en marche arrière, cercle aux inversions.
- Indicateurs de durée d'alerte : distance parcourue sous 25 cm, sous 10 cm.
  Plus parlant que la marge minimale seule, qui ne décrit qu'un instant.

## Base de véhicules

Schéma par champ : valeur, unité, provenance (`measured`, `manufacturer`,
`derived`, `estimated`), référence du document source.

Champs rarement publiés, à traiter avec soin :

- **porte-à-faux avant** : quasiment jamais publié. Plans cotés des dossiers
  de presse, ou répartition approchée de `longueur − empattement` selon la
  carrosserie (≈ 52/48 traction, 55/45 SUV, 48/52 break).
- **largeur rétros déployés** : peu publiée. Rabattus, encore moins.
- **rayon de braquage** : publié tantôt entre trottoirs, tantôt entre murs,
  souvent sans préciser, et variable selon la jante. Donnée la plus piégeuse
  car son impact est direct.

Validation à l'import : largeur rétros ≥ largeur caisse, somme des
porte-à-faux = `longueur − empattement`, rayon de braquage dans une plage
plausible rapportée à l'empattement.

Prévoir un niveau **version** sous le modèle : jante, finition et motorisation
font varier ces valeurs. Ne jamais moyenner.

Ne pas viser l'exhaustivité : 50 à 100 modèles bien renseignés couvrent
l'essentiel, la saisie manuelle absorbe le reste. Une base incomplète mais
fiable vaut mieux qu'une base large et approximative — 3 cm d'erreur sur la
largeur rétros inverse une conclusion.
