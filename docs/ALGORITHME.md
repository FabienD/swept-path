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

**La recherche exacte** balaie toutes les combinaisons d'une grille — rayon de
braquage, position latérale de départ, point d'engagement — et retient la plus
dégagée. Comme le balayage est complet, **son échec est informatif** : il n'y a
pas d'entrée en un mouvement sur cette grille.

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

## 6. Pourquoi il n'y a pas d'horloge

Le prototype bornait sa recherche par un plafond de nœuds **et** une échéance
de 2,2 secondes par profondeur. Le résultat dépendait donc de la machine, et ne
pouvait être affirmé dans aucun test.

Ici, les budgets se comptent en nœuds explorés, jamais en millisecondes. Les
mêmes entrées produisent toujours le même plan — c'est vérifié par un test.
L'annulation par l'utilisateur relèvera du Web Worker, qu'il suffit de tuer :
le domaine n'a pas à la connaître.

## 7. Ce que valent les résultats de référence

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

## 8. Le coût de la grille de planification

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

La grille fine coûte une cinquantaine de fois plus de nœuds **et rend de moins
bons plans**. La cause n'est pas la résolution en elle-même : à pas fin,
beaucoup plus de nœuds atteignent tôt la zone d'atterrissage, si bien que le
quota de solutions se remplit de candidats à l'étroit. Relever ce quota de 14 à
200 récupère un bon plan sur une ouverture de 4 m, mais les scènes plus
serrées épuisent d'abord le budget de nœuds.

**Décision :** la grille par défaut reste celle du prototype ;
`Discretisation::fine()` demeure disponible et documentée. La bonne réponse
est le **raffinement progressif** — planifier grossier, puis affiner localement
autour de la solution trouvée — et non un budget plus gros. C'est une tâche à
part entière, à porter au lot 2.

Le quota de solutions passe en revanche de 14 à 200, sur mesure : tester un
atterrissage de plus est bon marché, c'est atteindre la zone qui coûte.

## 9. Dette assumée

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

**Rien n'est encore ancré dans le réel.** Le noyau calcule ce que la géométrie
prédit — c'est vérifié — mais il le calcule sur des dimensions supposées :
dans `data/vehicles.json`, la largeur aux rétroviseurs est `derived` et le
porte-à-faux avant `estimated` pour presque tous les modèles. Une mesure au
mètre ruban de la largeur aux rétros, déployés puis rabattus, vaudrait plus que
n'importe quel raffinement d'algorithme : le `CLAUDE.md` note que 3 cm
d'erreur y inversent une conclusion.
