# Trajectoires optimales — Dubins et Reeds-Shepp

*Design du 10 août 2026.*

Remplacer les trajectoires candidates construites à la main par les familles de
courbes optimales de la littérature, et brancher Reeds-Shepp en expansion
analytique dans le planificateur.

## 1. Le constat

Trois observations convergentes, toutes mesurées sur la scène réelle de
Fabien — passage libre 2,29 m, battants à 90°, trottoir 1,25 m, chaussée
6,20 m, Lexus LBX.

**La recherche exhaustive ne trouve rien.** Zéro candidat sur 7 410, quel que
soit le rayon de braquage. Sa forme est figée : une ligne droite parallèle au
trottoir, un quart de tour à rayon rigoureusement constant, une ligne droite.
Le conducteur, lui, se déporte, braque, ajuste — trois gestes qu'aucune de ces
phases ne sait représenter.

**Le planificateur rase.** Rien dans son coût — `manœuvres × 5 +
distance × 0,18 + heuristique` — ne récompense la marge, donc il frôle le
vantail droit et termine décalé dans la cour. Sa réponse est de surcroît
heuristique : elle ne prouve rien.

**Il bricole des micro-manœuvres.** À défaut de pouvoir s'aligner exactement
sur une grille de 90 cm et 6°, il compense par des allers-retours de quelques
dizaines de centimètres. Non qu'une manœuvre courte soit illégitime en
soi — se dégager d'un pilier en demande une — mais celles-ci ne gagnent aucune
place : elles rattrapent une erreur de discrétisation. La distinction est
reprise en §7 bis, car c'est elle, et non la longueur, qui doit servir de
critère.

## 2. Ce que les courbes apportent

**Dubins** donne, en forme close, le chemin le plus court entre deux poses pour
un véhicule à rayon de braquage borné, **en marche avant seule**. Six familles
suffisent : `LSL`, `RSR`, `LSR`, `RSL`, `RLR`, `LRL`, où `L` et `R` sont des
arcs au rayon minimal et `S` un segment droit.

**Reeds-Shepp** généralise au cas avec marche arrière : douze familles
fondamentales, déclinées par inversion du temps et réflexion, soit quarante-huit
mots. Il minimise la longueur **et** donne le nombre minimal de changements de
sens — exactement la grandeur que ce projet compte comme « manœuvres ».

**L'expansion analytique** est la technique standard qui manque au
planificateur : à chaque nœud développé, tenter une connexion Reeds-Shepp
directe vers la pose cible ; si elle est sans collision, la recherche s'arrête
là. C'est ce qui supprime les micro-manœuvres, puisque l'atterrissage devient
exact au lieu d'être approché sur une grille.

## 3. Une nuance qui décide de la conception

**Ces courbes minimisent la longueur, pas la marge.** Prise seule, la plus
courte rase davantage.

Ce n'est pas un obstacle, parce que ce n'est pas ainsi qu'on les emploie. La
recherche exhaustive actuelle retient déjà le candidat **le plus dégagé** parmi
ceux qu'elle essaie : son défaut n'est pas son critère de choix mais la pauvreté
de sa famille de candidats. Dubins lui en fournit une bien plus riche, et la
sélection par la marge fait le reste.

La conséquence est nette : on n'utilise jamais *la* courbe optimale, mais
**l'ensemble des courbes admissibles**, qu'on filtre par la collision et
qu'on trie par la marge. La longueur ne sert que de départage.

## 4. Où cela vit

Dans **`swept-core`**, module `curves`. Ce sont des primitives cinématiques
pures : elles ne connaissent ni scène, ni obstacle, ni stratégie de recherche —
seulement deux poses et un rayon.

**Implémentées, pas importées.** La crate [`reeds_shepp`](https://crates.io/crates/reeds_shepp)
existe, mais `swept-core` n'a aucune dépendance externe, et c'est cette
propriété qui permet sa publication sous `MIT OR Apache-2.0` à côté d'une
application AGPL. Dubins tient en environ 150 lignes, Reeds-Shepp en 500, tous
deux entièrement spécifiés dans la littérature — et ce sont précisément le
genre de fonctions que des tests de propriétés valident bien.

## 5. Ce que cela remplace

| Aujourd'hui | Demain |
|---|---|
| `path::forward_path` / `reverse_path` | énumération Dubins vers des poses cibles |
| `landing::landings` | connexion Reeds-Shepp exacte |
| `exact::search` balayant rayon × latéral × point d'engagement | balayage de poses de départ et d'arrivée, chemins par Dubins |
| A\* atterrissant sur une grille | A\* avec expansion analytique Reeds-Shepp |

**La pose cible devient explicite.** Le critère d'arrivée actuel — « avoir
franchi `entry_depth` » — ne contraint ni la position ni le cap final, d'où le
véhicule qui termine de travers dans la cour. Une courbe exige une pose
d'arrivée complète : on balaiera donc des poses cibles dans l'axe du passage,
ce qui corrige le défaut au passage.

## 6. Ce qui ne change pas

Rien de ce qui a été acquis ne doit être perdu.

- **Le déterminisme.** Aucune horloge, budgets en nœuds. Les formes closes ne
  font que renforcer la reproductibilité.
- **`Confidence`.** Une énumération Dubins exhaustive sur sa grille de poses
  reste `Exact` ; l'A\* reste `Heuristic`.
- **L'invariant multi ≥ simple**, et le filtre des alternatives dominées.
- **Les deux marges**, dans le passage et sur tout le trajet.
- **Les vecteurs de référence** du prototype gelé : la cinématique de base ne
  bouge pas.

## 7. Critères d'acceptation

1. Sur la scène de Fabien, la recherche exhaustive rend une entrée **à une
   manœuvre**, marquée `Exact` — aujourd'hui elle ne rend rien.
2. La marge de cette entrée est **au moins égale** à celle que le planificateur
   trouve aujourd'hui, et son point le plus serré se situe dans le passage,
   non contre une bordure à six mètres.
3. Le véhicule termine **aligné sur l'axe du passage**, à moins de 5° de la
   perpendiculaire.
4. **Aucun trajet ne contient d'inversion superflue** — voir §7 bis, qui définit
   ce que « superflu » veut dire, et pourquoi ce n'est pas une longueur.
5. Tous les tests existants passent, invariants `proptest` compris.

## 7 bis. Ce qu'est une manœuvre superflue

Une première rédaction proscrivait « toute inversion parcourant moins de
50 cm ». Le critère était faux, et Fabien l'a relevé : une marche arrière de
trente centimètres pour se dégager d'un pilier est exactement ce qu'un
conducteur fait. Sa brièveté ne la rend pas illégitime.

Ce qu'on veut écarter, c'est la manœuvre **qui n'achète rien** — celle que le
planificateur bricole faute de pouvoir s'aligner exactement sur sa grille de
90 cm et 6°. La longueur ne sépare pas les deux cas ; l'utilité, si.

> Un segment est **superflu** s'il existe une connexion Reeds-Shepp sans
> collision entre la pose qui le précède et celle qui le suit, comportant au
> plus autant d'inversions et laissant une marge au moins égale.

C'est directement vérifiable, puisque Reeds-Shepp donne cette connexion en
forme close. Et c'est le même principe que le filtre d'alternatives dominées
déjà en place — *on ne garde que ce qui achète de la place* — appliqué au
segment plutôt qu'au trajet entier.

La technique porte un nom, le *path shortcutting*, et devient un post-traitement
du lot 2c : tant qu'un raccourci existe, l'appliquer. Le trajet rendu est
**irréductible**, ce qui est à la fois plus fort et plus honnête qu'un seuil
arbitraire en centimètres.

## 8. Découpage

Trois lots, chacun livrant quelque chose de vérifiable.

**Lot 2a — Dubins.** Les six familles, l'échantillonnage en poses, les tests de
propriétés — longueur optimale, courbure bornée, atterrissage exact sur la pose
cible. Aucune intégration : la crate gagne une capacité, rien ne change encore.

**Lot 2b — La recherche exacte par Dubins.** Balayage de poses de départ et
d'arrivée, sélection par la marge. C'est le lot qui doit faire tomber les
critères 1 à 3.

**Lot 2c — Reeds-Shepp, l'expansion analytique et la réduction.** Les douze
familles ; leur branchement dans l'A\* comme tentative de connexion directe à
chaque nœud ; puis la réduction par raccourcis décrite en §7 bis. Critère 4.

La fonction de coût valorisant la marge n'est **pas** dans ce périmètre : elle
concerne le planificateur multi-manœuvres, que l'expansion analytique va déjà
largement soulager. À réévaluer après le lot 2c, sur mesure plutôt que sur
intuition.

## 9. Risques

**Le volume de candidats.** Balayer des poses de départ *et* d'arrivée, avec
six familles Dubins chacune, peut faire exploser le nombre de chemins à
évaluer. Le coût actuel de la recherche exhaustive est de 150 ms ; il faudra
mesurer et, si nécessaire, élaguer par une borne inférieure de longueur avant
tout test de collision.

**Les cas dégénérés.** Poses confondues, rayon nul, familles inapplicables :
les formulations publiées comportent des divisions et des `acos` dont le
domaine doit être vérifié. Les tests de propriétés sont là pour ça.

**La tentation de l'optimalité.** Ces courbes sont optimales *en longueur*, et
il sera tentant de s'en satisfaire. Le critère du projet reste la marge —
`(W − w) / 2` au mieux, treize centimètres sur la scène de référence. Une
trajectoire plus courte qui rase est un moins bon résultat.
