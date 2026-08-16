# Comment le simulateur fonctionne

Ce document explique le noyau de bout en bout, sans qu'il soit nécessaire
d'ouvrir le code. Il décrit l'état à l'issue du lot 1b : géométrie, cinématique
et solveurs. L'interface et la frontière WebAssembly relèvent du lot 1c.

## 1. Le repère et les unités

L'origine est au milieu du passage. `y = 0` est le nu extérieur du mur,
`y > 0` va vers la cour, `x` court le long de la voie. La chaussée est donc en
`y` négatif, la cour en `y` positif.

Toutes les longueurs sont des mètres, tous les angles des radians. Les degrés
n'existent qu'à l'affichage : le noyau ne les manipule jamais, et le type
`Radians` rend la confusion impossible à commettre silencieusement.

## 2. Le véhicule

Modèle bicyclette : l'état se réduit à la pose de l'essieu arrière, soit deux
coordonnées et un cap. Dans le repère local du véhicule, l'origine est sur cet
essieu et `x` pointe vers l'avant, si bien que le pare-chocs arrière est à
`-porte_à_faux_arrière` et l'avant à `empattement + porte_à_faux_avant`.

L'enveloppe est échantillonnée en quatorze points : cinq stations le long de
chaque flanc, les deux centres de pare-chocs, et **les deux rétroviseurs à
hauteur d'essieu avant**. Ces derniers comptent plus que le reste : ils sont
presque toujours ce qui touche en premier.

Le porte-à-faux arrière n'est jamais saisi, il se déduit de la longueur totale.
Trois règles sont vérifiées à la construction : toute dimension est
strictement positive, le porte-à-faux avant laisse de la place à l'arrière, et
les rétroviseurs ne sont pas plus étroits que la caisse.

## 3. La détection de collision

Tous les obstacles sont des rectangles orientés. Deux primitives suffisent
alors : la distance d'un point à un rectangle, et le recouvrement de deux
rectangles par axes séparateurs.

La marge d'une pose se calcule par **deux tests, et les deux sont
nécessaires**.

Le **test direct** promène les quatorze points de l'enveloppe contre chaque
obstacle et retient la plus petite distance. Le **test inverse** promène les
coins des obstacles contre le rectangle de la caisse. Sans lui, un coin de
pilier peut se trouver entièrement à l'intérieur de la caisse sans qu'aucun
point échantillonné ne tombe dans le pilier — le test direct déclarerait la
pose dégagée alors que le véhicule est embouti. C'est un cas réel, vérifié par
un test qui échoue dès qu'on neutralise la boucle inverse.

Les obstacles très étendus — les murs, la chaussée d'en face — sont exclus du
test inverse : leurs coins sont hors de la scène et ne servent qu'à gaspiller
du calcul.

## 3 bis. La hauteur des obstacles

Un obstacle n'est plus un mur, mais un mur d'une certaine hauteur. Une bordure
de trottoir de douze centimètres se surplombe ; un muret de quarante arrête le
pare-chocs. La comparaison se fait entre cette hauteur et la **garde au sol**
du véhicule — le point le plus bas de la carrosserie, roues exclues.

Trois règles, et chacune existe pour un cas que les autres manquent :

1. **La carrosserie** ne voit que ce qu'elle ne peut pas survoler. Un obstacle
   surplombé est ignoré *entièrement* : ni collision, ni distance. Compter la
   distance ferait tomber la marge à zéro dès qu'un pare-chocs déborde, ce qui
   serait le même refus sous un autre masque.
2. **Les quatre roues** voient tout, bordures comprises. Une carrosserie
   surplombe, un pneu ne quitte pas ce sur quoi il roule.
3. **Le test inverse des coins** ne porte que sur les obstacles bloquants : un
   coin de trottoir à l'intérieur de la caisse est un surplomb.

Les roues sont aux coins de la caisse, à la demi-largeur de caisse et non à la
demi-voie : le bord extérieur d'un pneu affleure la tôle à deux centimètres
près, parce que les passages de roue épousent les pneus. Cela épargne deux
mesures que personne n'a sous la main — la voie et la largeur de pneu — pour
une erreur bien moindre que celle qu'aurait causée la voie seule.

Il en découle une conséquence qui n'est pas évidente : **seul un porte-à-faux
peut surplomber**. Le flanc se trouve au-dessus d'une bordure exactement quand
un pneu y est. Ce qui passe au-dessus est donc ce qui dépasse d'un essieu — le
nez, ou l'arrière. C'est bien le phénomène réel : c'est le pare-chocs qui
balaie au-dessus du trottoir quand le véhicule braque, jamais la portière.

Les hauteurs ne sont jamais comparées dans la boucle chaude. Elles le sont une
fois, à la construction du champ de marge, qui range les obstacles en deux
listes disjointes. Le coût par pose est celui d'avant, plus quatre points.

Le surplomb est **signalé, jamais pénalisé** : le résultat rapporte la distance
parcourue au-dessus d'une bordure, et le solveur ne change pas de comportement.
Le modèle reste plan — il ne connaît ni la borne, ni le panneau, ni le poteau
qui se dresse si souvent sur un trottoir, et c'est précisément pourquoi cette
distance est dite plutôt que tue.

**Ce que cela change sur la scène de référence : rien.** Mesuré au moment de
l'implémentation, sur le portail de 2,29 m — un mur, une bordure de 12 cm et
l'absence totale de trottoir rendent tous les trois 4,15 cm de marge, au même
point le plus serré, sans qu'une seule pose surplombe quoi que ce soit. Ce qui
limite cette entrée est l'ouverture, pas le trottoir. Le lot corrige un modèle
qui se trompait en silence ; il ne rend pas ce passage plus facile.

## 4. L'intégration cinématique

Une manœuvre est une chaîne de segments à courbure constante. Chacun
s'intègre en forme close :

```
θ₁ = θ + κ·ds
x += R·(sin θ₁ − sin θ)      avec R = 1/κ
y −= R·(cos θ₁ − cos θ)
```

`ds` négatif est une marche arrière. Une courbure nulle donne une droite,
traitée à part puisque `R` y est infini.

L'échantillonnage d'un segment **atterrit exactement sur le point d'arrivée**,
jamais à un pas près : c'est ce qui permet d'enchaîner des dizaines de
segments sans accumuler de dérive.

## 5. Les trois solveurs, et ce que chacun prouve

**La recherche exacte** balaie des poses de départ le long de la chaussée et
des poses d'arrivée dans l'axe du passage, relie chaque paire par toutes les
courbes de Dubins applicables à chaque rayon de braquage, et retient la plus
dégagée. Comme le balayage est complet, **son échec est informatif** : il n'y a
pas d'entrée en un mouvement sur cette grille.

L'entrée en marche arrière emprunte les mêmes courbes. Reculer le long d'un
trajet, sous le modèle bicyclette, c'est le parcourir à l'envers : il suffit
donc de résoudre le problème retourné — de l'arrivée vers le départ, les deux
caps pivotés d'un demi-tour — puis de relire le résultat à l'endroit.

Un échec dit qu'il n'existe pas d'entrée en un mouvement **sur cette grille et
dans ce modèle**. La nuance n'est pas rhétorique : un passage que la recherche
refuse peut être un passage qu'un conducteur franchit tous les jours, si le
modèle facture quelque chose que la réalité ne facture pas. Le trottoir en est
l'exemple courant — modélisé comme un mur de hauteur infinie, alors qu'un
rétroviseur à un mètre du sol survole une bordure de quinze centimètres sans
la voir.

**Le planificateur multi-manœuvres** est un A\* hybride sur `(x, y, θ, sens)`.
Le coût dominant est le nombre de changements de sens — un conducteur compte
les marches arrière, pas les mètres — la distance ne servant qu'à départager.
Il n'est fiable que dans un sens : ce qu'il trouve est vérifié en collision,
ce qu'il ne trouve pas peut exister quand même.

**La chaussée minimale** est une dichotomie sur la largeur de voie, avec la
recherche exacte comme prédicat ; elle hérite donc de sa garantie.

Cette différence est portée par le type `Confidence`, attaché à chaque
résultat : `Exact` ou `Heuristic { budget_exhausted }`. Une marge ne peut pas
être affichée sans que sa provenance suive, parce qu'elles voyagent ensemble.

De même, ne rien trouver n'est pas une erreur mais un résultat
(`Outcome::NotFound`), ce qui préserve la distinction entre *aucune solution
trouvée* et *aucune solution n'existe*.

**Le planificateur est toujours amorcé par la recherche exacte.** Ce n'est pas
une optimisation : c'est ce qui garantit qu'un plan à plusieurs manœuvres n'est
jamais moins bon que la réponse à un mouvement, puisque celle-ci figure
toujours parmi les candidats et que tout plan plus profond qui ferait pire est
écarté.

## 6. Les courbes de Dubins

Le solveur construit ses trajectoires candidates à la main : une droite, un
quart de tour, une droite. Cette forme est arbitraire, et sur un passage serré
aucune de ses 7 410 variantes ne passe.

Dubins a montré en 1957 que le chemin le plus court entre deux poses, pour un
véhicule qui ne peut pas braquer plus court qu'un certain rayon et qui n'a pas
de marche arrière, est **toujours** l'un de six mots : quatre du type
arc-droite-arc (`LSL`, `RSR`, `LSR`, `RSL`) et deux du type arc-arc-arc
(`RLR`, `LRL`). Chacun a une forme close : aucune recherche, aucune itération.

Les quatre premiers s'appliquent quand les poses sont éloignées. Les deux
derniers existent quand les **cercles de braquage** sont assez proches pour
qu'un troisième cercle les touche tous les deux — c'est-à-dire le régime d'une
entrée de cour. Le seuil est bien celui des cercles, pas celui des poses : les
caps décalent chaque centre d'un rayon, si bien que deux poses distantes de
quatre rayons et demi peuvent encore admettre un `LRL`.

**Une précision qui décide de tout l'usage.** Ces courbes minimisent la
*longueur*. Ce projet mesure la *marge*. La plus courte est celle qui rase le
plus. On n'utilise donc jamais `shortest`, mais `all` : on énumère les six,
on écarte celles qui touchent un obstacle, et on garde la plus dégagée. La
longueur ne sert qu'à départager.

**Comment on sait que les formules sont justes.** On ne les compare pas à
d'autres formules — les versions publiées divergent, sur `LSR` et `LRL` en
particulier. On construit la courbe, on l'intègre avec la cinématique du
véhicule, et on vérifie qu'elle arrive sur la pose visée à 1e-9 près, position
et cap. Ce test ne dépend d'aucune source. Il tourne sur des paires de poses
nommées et sur des paires engendrées aléatoirement par `proptest`.

Les paires engendrées ont d'ailleurs immédiatement gagné leur place. Un chemin
`RSL` commençait par un arc de 0,86 mm, qu'un seuil d'un millimètre écartait
comme négligeable ; il faisait pourtant tourner le véhicule de 0,63 mrad, et
les 20,7 m de ligne droite qui suivaient amplifiaient l'erreur en 13 mm à
l'arrivée. **Supprimer un segment coûte son cap, pas seulement sa longueur** —
et 13 mm, sur un passage où la marge totale vaut 13 cm, ne sont pas
négligeables. Le seuil est descendu au nanomètre.

Ces courbes sont désormais ce que la recherche exacte essaie. La forme figée
d'autrefois — une droite, un quart de tour, une droite — n'était rien d'autre
qu'un mot `LSL` ou `RSR` avec deux contraintes gratuites en plus : l'arc faisait
exactement 90°, et la droite d'approche exactement 5 m. Les lever ne coûte rien
et donne accès aux crochets à trois arcs, qui sont précisément ce qu'exige une
entrée serrée. Sur cinq variantes du portail mesuré, l'ancienne forme ne
trouvait **rien du tout** ; le balayage en résout quatre.

La pose d'arrivée est devenue explicite au passage, et cela corrige un défaut
qui n'avait rien à voir : le critère d'arrivée était « avoir franchi la
profondeur d'entrée », qui ne contraint ni la position ni le cap, d'où le
véhicule qui terminait de travers dans la cour. Une courbe exige une pose
complète, donc on ne balaie que des arrivées à moins de 5° de la
perpendiculaire.

**Ce que le balayage a coûté, et comment il a été payé.** Une paire de poses
donne jusqu'à six courbes, et il y a bien plus de paires qu'il n'y avait de
candidats : la première version tenait 10,5 s là où l'ancienne recherche en
mettait 0,15. Deux mesures l'ont ramenée sous la seconde sans rien concéder sur
la couverture.

La première est une passe de reconnaissance : au lieu de parcourir les poses
dans l'ordre — ce qui fait payer toute la remontée de la rue avant d'atteindre
le passage, là où un candidat meurt presque toujours — on sonde d'abord huit
poses réparties d'un bout à l'autre. La correction est intacte, puisqu'une
collision réfute le chemin où qu'on la trouve ; seul l'ordre de découverte
change.

La seconde est un rééquilibrage des axes, décidé en les mesurant un à un.
Doubler le nombre de points d'entrée, de caps d'arrivée ou de rayons n'achetait
**aucune marge** ; doubler le nombre de positions de départ le long de la rue
la faisait passer de 0,1 à 4,2 cm. Cet axe décide de *l'endroit où le virage
commence*, et la fenêtre où un virage retombe droit dans un passage étroit fait
quelques centimètres de large. Il a donc reçu le budget dont les trois autres
n'avaient pas besoin.

## 6 bis. Les courbes de Reeds-Shepp

Reeds et Shepp (1990) étendent Dubins au véhicule qui peut reculer. Le plus
court chemin entre deux poses à courbure bornée est alors l'un de quarante-huit
mots, bâtis sur douze familles fondamentales — toutes en forme close.

Deux choses les distinguent de Dubins, et les deux comptent ici.

**Elles atteignent tout.** Dubins peut échouer à relier deux poses proches et
mal orientées ; en reculant, Reeds-Shepp y arrive toujours. C'est ce qui en
fait le bon outil pour un portail étroit, où le véhicule est précisément proche
et mal orienté.

**Elles minimisent aussi le nombre de changements de sens** — la grandeur que
ce projet appelle une manœuvre et que l'utilisateur compte. Le mot le plus
court et le mot le plus lisse ne sont pas le même : le noyau expose donc les
deux, `shortest` et `fewest_reversals`, en plus de `all`.

### Douze fonctions, pas huit

Le plan de ce lot annonçait huit fonctions de base, en supposant que les douze
familles s'y réduisaient par les involutions. **C'était faux**, et trois formes
manquaient : une troisième lecture à trois arcs, et les lectures inversées des
deux formes arc-arc-droite-arc. Elles ne se déduisent d'aucune autre.

Les involutions restent nécessaires — le retournement du temps, qui parcourt le
chemin à l'envers, et la réflexion, qui échange gauche et droite. Quatre
lectures pour chacune des douze fonctions, et le compte de quarante-huit y est.

### Une garde de domaine n'est pas une garde de signe

La première version refusait une famille dès qu'une de ses longueurs sortait
négative. C'est un contresens : une longueur négative **est** un segment en
marche arrière, ce qui est toute la raison d'être de Reeds-Shepp. Une famille
ne se refuse que lorsque sa formule quitte un domaine — une racine de nombre
négatif, un `acos` hors de `[-1, 1]`.

### Ce qui les vérifie, et ce qui ne suffit pas

Chaque famille est validée en intégrant son résultat par le modèle cinématique
et en regardant où il tombe. Ce test ne dépend d'aucune publication et tranche
tout désaccord — il a d'ailleurs attrapé une erreur de signe qui faisait
atterrir les trois arcs à 2,96 radians du but.

**Mais il ne voit pas ce qui manque.** Une famille écartée à tort ne produit
pas un chemin faux, elle produit un chemin absent : les tests d'atterrissage
n'examinent que les mots rendus, et les propriétés ne demandent qu'un chemin,
pas le meilleur. Il a fallu une transcription indépendante — la crate
`reeds_shepp` en dépendance de développement — pour trouver un chemin de
15,718 m là où nous rendions 15,722 m. C'est ainsi que les trois familles
manquantes sont apparues.

Le noyau garde ses zéro dépendances de production : l'oracle ne sert qu'aux
tests, et `cargo tree --edges normal` le vérifie.

Ce module ne fait encore rien d'autre qu'exister. Le lot 2c-2 le branchera dans
le planificateur comme expansion analytique, et s'en servira pour la réduction
par raccourcis.

## 6 ter. L'expansion analytique, et ce qu'elle a coûté

Le dernier mouvement d'un plan — l'atterrissage — se construisait par une forme
figée : un arc à rayon choisi, puis une droite. Elle visait une *profondeur* et
non une pose, exactement le défaut que le lot 2b avait corrigé côté recherche
exhaustive.

Reeds-Shepp relie deux poses **exactement**. À chaque nœud proche de
l'ouverture, le planificateur tente désormais une connexion en forme close vers
une pose d'arrivée, et retient la plus dégagée.

### Les deux formes cohabitent, et il le faut

| ouverture | avant le lot 2b | après le coût | après l'expansion |
|---|---|---|---|
| 2,20 m | 11 mm, 4 manœuvres | 42 mm, 4 | 42 mm, 4 |
| 2,60 m | 24 mm, 3 manœuvres | 21 mm, 2 | **33 mm**, 2 |
| 3,00 m | 41 mm, 1 manœuvre | 41 mm, 1 | **62 mm**, 1 |
| 4,00 m | 26 mm, 1 manœuvre | 49 mm, 1 | **385 mm**, 1 |

Sur une ouverture de 4 m, la marge est multipliée par près de huit. Mais la
première version de ce lot, qui avait remplacé la forme figée au lieu de s'y
ajouter, **ne trouvait plus rien du tout à 2,20 m**.

La raison tient à ce que chacune sait faire. Une courbe optimale arrive à la
pose voulue mais souvent **en tournant encore**, ce qu'un passage laissant huit
centimètres de jeu n'autorise pas. La forme figée, elle, finit toujours par une
droite : maladroite là où il y a de la place, et seule à franchir un passage
serré. Mesuré : sous 2,60 m, la connexion en forme close ne trouve aucun
atterrissage là où la forme figée en trouve.

Les deux sont donc essayées et la plus dégagée gagne — la règle que ce projet
applique partout ailleurs.

### Le coût, et pourquoi les tentatives sont espacées

Une connexion en forme close énumère une grille de poses d'arrivée, jusqu'à
quarante-huit courbes chacune, échantillonne chacune et la promène contre tous
les obstacles : **dix-neuf millions de tests point-obstacle par nœud**, mesuré.
La suite de tests est passée de douze secondes à plus de dix minutes.

Espacer les tentatives est le remède standard de l'A\* hybride. Mais l'espacement
ne vaut que pour la partie chère : appliquer le même intervalle à la forme
figée, bon marché, est précisément ce qui vidait le passage le plus serré.

### Une faiblesse du modèle que ces courbes ont révélée

Sur une ouverture de 1,60 m, infranchissable pour ce véhicule, le solveur a
rendu un « atterrissage » de **175 mètres** qui contournait le mur par son
extrémité — la scène s'arrête à dix-huit mètres de part et d'autre — en
annonçant vingt-sept centimètres de marge. Le calcul était juste sur la
géométrie fournie ; c'est le modèle qui a des bords, et la forme figée était
trop courte pour les atteindre. Une borne de longueur sur l'atterrissage
rétablit cette propriété.

## 6 quater. La réduction par raccourcis

Un A\* hybride ne peut pas atterrir exactement sur sa grille, alors il comble
l'écart par de petites manœuvres qui n'achètent rien.

Le critère n'est pas une longueur — une marche arrière de trente centimètres
pour se dégager d'un pilier est exactement ce qu'un conducteur fait :

> Un tronçon est superflu lorsqu'une courbe de Reeds-Shepp relie la pose qui le
> précède à celle qui le suit sans collision, avec au plus autant d'inversions,
> et en laissant au moins autant de marge.

Directement vérifiable, puisque Reeds-Shepp donne cette connexion en forme
close. C'est la règle du filtre d'alternatives — *ne garder que ce qui achète
de la place* — appliquée au tronçon plutôt qu'au trajet.

Après réduction, la marge est **remesurée sur le trajet rendu** et les
manœuvres recomptées : reporter un chiffre qui ne décrit plus le trajet affiché
serait le défaut que ce projet corrige ailleurs.

## 7. Pourquoi il n'y a pas d'horloge

Le prototype bornait sa recherche par un plafond de nœuds **et** une échéance
de 2,2 secondes par profondeur. Le résultat dépendait donc de la machine, et ne
pouvait être affirmé dans aucun test.

Ici, les budgets se comptent en nœuds explorés, jamais en millisecondes. Les
mêmes entrées produisent toujours le même plan — c'est vérifié par un test.
L'annulation par l'utilisateur relèvera du Web Worker, qu'il suffit de tuer :
le domaine n'a pas à la connaître.

## 8. Ce que valent les résultats de référence

Le `CLAUDE.md` conservait quatre résultats issus du prototypage. Le portage en
a vérifié deux et **infirmé deux**.

**Vérifié — le couloir d'un portail coulissant** se limite à l'épaisseur de
pilier, 0,55 m dans la scène de référence.

**Vérifié — le plafond de marge vaut `(W − w) / 2`**, quelle que soit la
trajectoire. C'est la conclusion principale du projet : *multiplier les
manœuvres n'achète pas de marge*. La largeur du passage domine tout.

**Infirmé — la « largeur critique »**. La note affirmait qu'au-dessus de
`√(w² + L²)`, tout angle d'approche passe. C'est faux. Un véhicule de largeur
`w` franchissant un couloir de profondeur `L` à un écart `α` de la
perpendiculaire occupe :

```
emprise = w / cos α  +  L · tan α
```

Cette emprise croît sans borne quand `α → 90°` : aucune ouverture ne fait donc
passer *tous* les angles. La tolérance est la solution de `emprise = W`, et le
noyau la reproduit à moins de 3° près sur quatre largeurs.

**Infirmé — les 4° de tolérance** pour des battants ouverts à 90°. Mesure et
géométrie concordent à moins d'un degré sur **environ 14°**. La conclusion
qualitative survit : les vantaux réduisent la tolérance de plus de moitié, et
rabotent le passage de 2,40 m à environ 2,20 m puisque chaque axe est en
retrait de 5 cm.

## 9. Le coût de la grille de planification

Le `CLAUDE.md` demandait de raffiner la grille de 90 cm et 6° à 20 cm et 1°,
pour trouver les solutions à marge centimétrique que le prototype ratait.
**La mesure a contredit cette attente.**

Relevé par `cargo run -p swept-solver --release --example grid_cost` :

| grille | ouverture | nœuds | marge | manœuvres |
|---|---|---|---|---|
| défaut (90 cm / 6°) | 2,20 m | 60 000 | 12 mm | 4 |
| défaut | 2,60 m | 20 500 | 28 mm | 3 |
| défaut | 3,00 m | 38 500 | 55 mm | 1 |
| défaut | 4,00 m | 38 500 | 27 mm | 1 |
| fine (20 cm / 1°) | 2,20 m | 60 000 | *rien* | — |
| fine | 2,60 m | 60 000 | 1 mm | 2 |
| fine | 3,00 m | 60 000 | 1 mm | 2 |
| fine | 4,00 m | 60 000 | 1 mm | 2 |

La grille fine coûte des dizaines de fois plus de nœuds **et rend de moins bons
plans**.

La cause n'est pas la résolution mais **la fonction de coût**. Rien dans
`manœuvres × 5 + distance × 0,18 + heuristique` ne récompense la marge : le
planificateur cherche le chemin le plus court, donc il rase les obstacles. Des
primitives plus courtes lui permettent simplement de raser plus finement. La
grille grossière n'est pas meilleure par vertu, elle est protégée par sa
maladresse.

La localisation du point le plus serré le montre sans ambiguïté. Sur une
ouverture de 4 m :

| grille | marge minimale | position dans le plan | où |
|---|---|---|---|
| défaut | 26,6 mm | pose 66 sur 171 | devant l'ouverture |
| fine | 1,3 mm | pose 14 sur 327 | `(−5,90 ; −2,47)` — contre la bordure, six mètres avant le portail |

Le millimètre n'est donc pas frôlé contre un pilier mais **contre le trottoir,
au tout début de l'approche**, loin du passage.

**Décision :** la grille par défaut reste celle du prototype ;
`Discretisation::fine()` demeure disponible et documentée. Le correctif n'est
pas un budget plus gros ni même le raffinement progressif — ceux-là traitent le
nombre de nœuds, pas le problème. Il faut **une fonction de coût qui valorise
la marge**, ou un filtre écartant les plans qui rasent.

### Le correctif, et ce qu'il a donné

Le coût porte désormais un quatrième terme :

```
score = manœuvres × 5,0 + distance × 0,18 + pire_manque × 16,0 + heuristique
```

où `pire_manque = max(0, 0,25 − marge)` sur **le point le plus serré du
trajet**, et non la somme le long de celui-ci. Le pire ne peut que s'aggraver
en avançant, donc le coût reste monotone et l'A\* reste correct ; et
l'heuristique, qui ignore une pénalité future toujours positive, continue de
sous-estimer.

**La marge ne détrône jamais une manœuvre, par arithmétique.** Le manque est
borné par le seuil lui-même — une marge nulle manque de 25 cm — donc la
pénalité maximale d'un plan vaut `0,25 × 16 = 4,0`, contre 5,0 pour une
manœuvre. Aucune scène ne peut faire échanger l'une contre l'autre, et c'est
une propriété vérifiée par un test plutôt qu'un réglage à surveiller.

La marge lue à chaque primitive ne coûte rien : le test de collision la
calculait déjà et la jetait.

Mesuré par `grid_cost`, grille par défaut :

| ouverture | avant | après |
|---|---|---|
| 2,20 m | 11 mm, 4 manœuvres | **42 mm**, 4 manœuvres |
| 2,60 m | 24 mm, 3 manœuvres | 21 mm, **2 manœuvres** |
| 3,00 m | 41 mm, 1 manœuvre | 41 mm, 1 manœuvre |
| 4,00 m | 26 mm, 1 manœuvre | **49 mm**, 1 manœuvre |

Le cas à 2,60 m mérite un mot : le plan perd 3 mm mais gagne une manœuvre en
moins. C'est exactement la décision prise — à choisir, on préfère manœuvrer une
fois de moins.

**Une régression assumée sur la grille fine.** Aux primitives de 20 cm, les
ouvertures de 2,60 et 3,00 m passent de « trouvé » à *budget épuisé* : un A\*
qui préfère les zones dégagées explore plus large, et les 60 000 nœuds
partent plus vite. Ce qu'on perd est mince — ces plans rendaient 1 mm de marge,
c'est-à-dire le symptôme même que cette section décrivait. Mais la perte est
réelle : un résultat à 1 mm disait au moins qu'un chemin existait, alors que
*budget épuisé* ne dit rien du tout.

Le quota de solutions passe de 14 à 200, sur mesure : tester un atterrissage de
plus est bon marché, c'est atteindre la zone qui coûte.

Une remarque pour l'interface : la marge rapportée est le minimum sur **tout**
le trajet, approche lointaine comprise. Frôler une bordure sur la route n'a pas
la même portée que frôler un pilier. Distinguer les deux relèvera du lot 1c.

## 10. Dette assumée

Trente-trois constantes du noyau sont marquées `ARBITRARY` : reprises du
prototype, sans justification connue. Elles sont nommées et documentées une par
une, avec leur ligne d'origine, ce qui vaut mieux que de les laisser nues —
mais aucune n'est mesurée.

Les plus susceptibles de changer une conclusion :

| Constante | Valeur | Ce qu'il faudrait pour la justifier |
|---|---|---|
| `OVERLAP_TOLERANCE_M` | 6 mm | Une tolérance de contact réelle, ou la supprimer |
| `ENTRY_CLEARANCE_M` | 0,60 m | Quelle profondeur au-delà du seuil compte comme « entré » |
| `BODY_STATIONS` | 5 | Un test de convergence : 9 stations changent-elles un verdict ? |
| `HEADING_ERROR_WEIGHT` | 2,2 | Un réglage d'heuristique, à comparer à l'optimum |
| `MOVE_COST` / `LENGTH_COST_PER_M` | 5,0 / 0,18 | Ce qu'un conducteur accepte d'échanger contre une reprise |

Trois valeurs seulement sont marquées `MEASURED` : `DEFAULT_MAX_NODES`,
`DEFAULT_MAX_SOLUTIONS` et le choix de la grille par défaut.

**Le trottoir est un mur de hauteur infinie**, et c'est la dette qui coûte le
plus cher aujourd'hui. Sur le portail mesuré — 2,29 m de passage, 5,90 m de
chaussée, 1,30 m de trottoir — la recherche exacte ne trouve aucune entrée en
un mouvement, et la géométrie explique pourquoi : il faut au véhicule son rayon
de braquage entier, 3,59 m, entre le point de départ le plus bas et le point où
il doit être droit, et la rue n'en offre que 1,44. Or le propriétaire y entre
tous les jours, rétroviseurs déployés.

L'écart ne vient ni de la grille ni des courbes : la trajectoire idéale
construite à la main entre elle aussi en collision. Il vient du modèle. Un
rétroviseur à un mètre du sol survole une bordure de quinze centimètres, mais
en 2D pur il la heurte, ce qui interdit au véhicule d'approcher du trottoir —
c'est-à-dire exactement la place qui lui manque pour tourner. C'est le défaut
n° 5 du `CLAUDE.md`, et il faut un attribut de hauteur sur les obstacles
(`full` / `low`) avec la hauteur des points du véhicule pour le lever.

Mesuré à l'appui, sur cette rue et ce véhicule : vantaux à 90° et rétros
déployés, rien ; vantaux à 118°, 4,2 cm ; coulissant, 5,0 cm ; vantaux à 90° et
rétros rabattus, 2,6 cm. La géométrie du modèle est cohérente — c'est le modèle
qui est trop sévère.

**Rien n'est encore ancré dans le réel.** Le noyau calcule ce que la géométrie
prédit — c'est vérifié — mais il le calcule sur des dimensions supposées :
dans `data/vehicles.json`, la largeur aux rétroviseurs est `derived` et le
porte-à-faux avant `estimated` pour presque tous les modèles. Une mesure au
mètre ruban de la largeur aux rétros, déployés puis rabattus, vaudrait plus que
n'importe quel raffinement d'algorithme : le `CLAUDE.md` note que 3 cm
d'erreur y inversent une conclusion.
