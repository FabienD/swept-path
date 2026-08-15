# Lot 3 — La hauteur des obstacles

## Le problème

Le modèle est purement 2D : tout obstacle est un mur de hauteur infinie. Une
bordure de trottoir de douze centimètres arrête donc un rétroviseur qui, dans
la réalité, passe un mètre au-dessus.

Ce n'est pas une imprécision de plus : le solveur a raison sur sa géométrie et
tort sur le monde, et rien dans ses résultats ne le signale.

**Correction mesurée à l'implémentation.** Cette section affirmait d'abord que
le portail de référence — 2,29 m de passage libre, trottoir de 1,30 m — était
justement bloqué par là. C'est faux, et la mesure le dit : sur cette scène, un
mur, une bordure de 12 cm et l'absence totale de trottoir rendent **tous les
trois 4,15 cm**, au même point le plus serré, sans qu'une seule pose surplombe
quoi que ce soit. Ce qui limite cette entrée est l'ouverture, pas le trottoir.

Le lot garde sa raison d'être — un modèle qui traite une bordure comme un mur
se trompera ailleurs, et se trompe en silence — mais il ne faut pas lui prêter
un gain qu'il n'apporte pas ici. La conclusion du projet tient une fois de
plus : la largeur du passage domine tout.

Ce lot donne une hauteur aux obstacles et une garde au sol au véhicule, pour
que la carrosserie puisse surplomber ce qui est plus bas qu'elle.

## Les quatre décisions

Prises avec Fabien avant conception, et qui déterminent tout le reste.

**Le surplomb seul, jamais le franchissement.** Une roue ne monte jamais sur un
trottoir. Sur la scène de référence, les quatre roues restent sur la chaussée
puis sur le bateau ; seule la carrosserie déborde. Modéliser le franchissement
d'une bordure — avec quelle vitesse, quel angle d'attaque, quel risque pour un
pneu — serait un autre problème, et il n'est pas demandé.

**Une hauteur de chaque côté, pas trois.** L'obstacle porte une hauteur en
mètres, le véhicule une garde au sol. Un modèle plus fin — bas de pare-chocs,
bas de caisse, hauteur des rétroviseurs — serait plus juste physiquement, mais
demanderait trois mesures qu'aucun constructeur ne publie de façon fiable, et
que personne n'a sous la main. Le modèle à deux valeurs traite correctement le
cas qui compte : une bordure basse se surplombe, un muret de quarante
centimètres arrête le pare-chocs.

**Le surplomb est signalé, jamais pénalisé.** Le résultat rapporte la distance
parcourue en surplomb ; le solveur ne change pas de comportement pour autant.
Pénaliser relèverait de la fonction de coût, qui est le chantier suivant :
mêler les deux rendrait toute régression impossible à attribuer.

**La garde au sol est une donnée constructeur.** Elle entre dans
`data/vehicles.json` avec sa provenance et devient un paramètre de plein droit
de `Vehicle::new`, au même titre que l'empattement. Pas de valeur par défaut
cachée sur une dimension qui décide du verdict.

## Le modèle de données

### L'obstacle porte sa hauteur

```rust
// crates/swept-core/src/scene/obstacles.rs
pub struct Obstacle {
    pub shape: Obb,
    /// Hauteur au-dessus du sol, en mètres. `f64::INFINITY` pour un mur.
    pub height: f64,
}

impl Obstacle {
    pub fn wall(shape: Obb) -> Self;              // height = INFINITY
    pub fn low(shape: Obb, height: f64) -> Self;
}
```

`Obb` reste un type de géométrie pure : la hauteur est une propriété de scène,
pas de rectangle. `Scene::obstacles()` rend désormais `Vec<Obstacle>` — quatre
sites d'appel, tous des tests hormis `ClearanceField::new`.

Le mur, les piliers, les vantaux et le mur d'en face sont des `wall`. Les deux
morceaux de trottoir sont des `low`, à la hauteur portée par la scène.

### La scène porte la hauteur de bordure

```rust
pub struct Scene {
    // …
    /// Hauteur de la bordure de trottoir, en mètres.
    pub kerb_height: f64,
}
```

Une seule hauteur pour tout le trottoir. Le bateau n'est pas un obstacle : il
est déjà modélisé comme un trou découpé dans la bande de trottoir, et le reste.

### Le véhicule porte sa garde au sol

```rust
pub struct Vehicle {
    // …
    /// Point le plus bas de la carrosserie, roues exclues, en mètres.
    pub ground_clearance: f64,
}

pub fn new(
    wheelbase: f64,
    length: f64,
    front_overhang: f64,
    width: f64,
    mirror_width: f64,
    ground_clearance: f64,
    min_turning_radius: f64,
) -> Result<Self, VehicleError>;
```

Septième paramètre, inséré avant le rayon pour garder les dimensions groupées.
Vingt-deux appels à mettre à jour, dont une moitié de fixtures `lbx()`.

Rejeté par `VehicleError::NonPositive("ground_clearance")` s'il n'est pas
strictement positif, comme les autres dimensions.

### Les roues

```rust
impl Vehicle {
    /// Les quatre points de contact au sol, dans le repère du véhicule.
    pub fn wheels(&self) -> [Point; 4];
}
```

Aux coins d'un rectangle `width × wheelbase` : `(0, ±width/2)` et
`(wheelbase, ±width/2)`.

**Pourquoi la largeur de caisse et non la voie.** La voie sépare les *plans
médians* des roues ; le bord extérieur du pneu se trouve une demi-largeur de
pneu plus loin. Sur le LBX — 1,56 m de voie, pneus de 225 mm — ce bord tombe à
0,893 m de l'axe, contre 0,913 m pour la demi-largeur de caisse : le pneu
affleure la tôle à deux centimètres près, parce que les passages de roue
épousent les pneus. Prendre la demi-largeur de caisse évite donc de demander
deux mesures supplémentaires pour une erreur de l'ordre de deux centimètres,
là où la voie seule en aurait introduit treize.

Réserve : selon le montage — jantes larges, déport différent — un pneu peut
affleurer voire dépasser légèrement l'aile. L'erreur est petite mais pas
garantie du côté sûr, et le doit dire.

**Conséquence à connaître : seul un porte-à-faux peut surplomber.** Les roues
étant aux coins de la caisse, le flanc se trouve au-dessus d'une bordure
exactement quand un pneu y est. Ce qui peut passer au-dessus est donc ce qui
dépasse d'un essieu — le porte-à-faux avant, ou l'arrière. C'est physiquement
juste, et c'est bien le phénomène observé : c'est le nez qui balaie au-dessus
du trottoir quand le véhicule braque, jamais la portière.

## L'évaluation

### Le pré-tri

`ClearanceField::at` est le point chaud du projet : une passe fine l'appelle
des centaines de milliers de fois. Les hauteurs n'y entrent donc jamais. Elles
sont comparées **une seule fois**, à la construction :

```rust
pub struct ClearanceField {
    /// Ce que la carrosserie heurte : height > ground_clearance.
    blocking: Vec<Obb>,
    /// Ce qu'elle surplombe, et que seules les roues heurtent.
    overhung: Vec<Obb>,
    /// Coins retenus, obstacles bloquants uniquement.
    corners: Vec<Point>,
    envelope: Vec<Point>,
    wheels: [Point; 4],
    // … demi-largeur, avant, arrière, inchangés
}
```

**Deux listes disjointes, et non « les bloquants » plus « tous ».** Les roues
parcourent les deux, ce qui coûte exactement le même nombre de tests, et
`overhangs` a alors la liste dont il a besoin sans avoir à la reconstruire.

Un obstacle est bloquant lorsque `height > ground_clearance`. Un obstacle
exactement à la hauteur de la garde au sol est donc surplombé — cas limite
assumé et testé, puisqu'un modèle qui hésite au millimètre près ne rendrait
service à personne.

### Les trois passes de `at()`

1. **La carrosserie contre `blocking` seulement.** Un obstacle surplombé est
   *entièrement ignoré* : ni collision, ni distance. C'est essentiel — compter
   la distance ferait tomber la marge à zéro dès qu'un pare-chocs déborde, ce
   qui est précisément ce que le lot doit cesser de faire.
2. **Les quatre roues contre `blocking` et `overhung`.** Une roue dans un
   obstacle, fût-il bas, est une collision. La distance roue-bordure compte
   dans la marge rendue : c'est une contrainte réelle.
3. **Le test inverse des coins, restreint aux obstacles bloquants.** Un coin de
   trottoir à l'intérieur de la caisse est un surplomb, pas une collision.

### Ce que la marge veut dire ensuite

La marge rendue reste « la plus petite distance à ce qui peut être heurté ».
Sa définition ne change pas ; ce qui change est la liste de ce qui peut l'être.
Une trajectoire qui surplombe le trottoir sur un mètre peut donc afficher une
marge confortable — c'est correct, et c'est la raison d'être de la métrique de
surplomb.

## Le surplomb, mesuré à la restitution

Comme `metres_under_25cm`, il se calcule **une fois sur la trajectoire
retenue**, jamais pendant la recherche.

```rust
impl ClearanceField {
    /// Un point de carrosserie se trouve-t-il au-dessus d'un obstacle bas ?
    pub fn overhangs(&self, pose: Pose) -> bool;
}
```

Il teste les points d'enveloppe contre `overhung` seulement, et rend vrai dès
le premier qui tombe dedans. Sur une scène dont la bordure est déclarée pleine,
`overhung` est vide et le prédicat est toujours faux — les tests de référence
n'ont donc rien à mesurer de neuf.

`swept-wasm` marque **chaque pose** d'un `overhanging: bool` et en somme les
distances, exactement comme il le fait déjà pour les bandes d'alerte ;
`ManeuverDto` gagne `metres_overhanging: f64`. Coût nul pour le solveur.

Le drapeau par pose n'est pas une commodité : le rendu segmente le tracé en
groupant les poses voisines qui partagent une clé, laquelle combine le sens de
marche et la bande de marge. Sans drapeau par pose, un total ne permettrait pas
de dessiner où le surplomb commence.

## L'interface

Deux champs de saisie : **hauteur de bordure** côté scène (0,12 m par défaut,
`ARBITRARY`, ordre de grandeur d'une bordure française) et **garde au sol**
côté véhicule, renseignée par le modèle choisi.

Une carte de résultat, « Surplomb du trottoir », affichée seulement quand la
valeur est non nulle — une carte à zéro sur toutes les trajectoires ordinaires
n'apprendrait rien. Et le segment concerné tracé distinctement dans l'épure,
avec sa mention en légende.

## La base véhicules

`data/vehicles.json` gagne un champ `ground_clearance` sur chacun de ses quatre
modèles, avec sa provenance, initialisé à `{ "v": null, "source": null }` comme
le fait déjà `width_mirrors_folded`. Les valeurs demandent les documents
constructeurs et seront renseignées séparément : la règle du projet interdit de
les prendre chez un agrégateur.

`schema_version` passe de 1 à 2. Aucun code ne le lit aujourd'hui, mais un
fichier de données publié qui change de forme sans changer de version est une
dette qu'on ne remarque qu'une fois qu'elle coûte cher. L'interface doit alors
traiter un `ground_clearance` absent ou nul comme « à saisir », jamais comme
zéro : une garde au sol nulle ferait de tout obstacle un mur, ce qui est
précisément l'état dont ce lot sort.

Trois réserves à documenter sur le champ, parce qu'elles vont dans des sens
différents :

- la valeur publiée est donnée **à vide** ; en charge, un véhicule s'affaisse
  de deux à quatre centimètres ;
- la **norme de mesure varie** — sous l'essieu chez les uns, au point le plus
  bas du châssis chez les autres, et une batterie sous plancher déplace ce
  point sur un hybride rechargeable ;
- les **éléments souples en sont généralement exclus**, or la lèvre de
  pare-chocs avant est justement ce qui surplombe en premier quand le véhicule
  braque.

Le biais est donc conservateur sur le flanc — la garde au sol publiée est un
minimum sur tout le véhicule, donc elle sous-estime le bas de caisse latéral —
et potentiellement optimiste sur le nez.

## Les tests

### Non-régression : aucune assertion existante n'est affaiblie

Les scènes des tests actuels déclarent leur bordure **pleine**
(`kerb_height: f64::INFINITY`). Leurs résultats sont ainsi préservés au bit
près : ils décrivent le monde dans lequel les résultats de référence du projet
ont été établis, et doivent continuer de le décrire.

Les vecteurs dorés ne sont pas concernés : ils portent sur la distance
point-rectangle, le chevauchement et l'intégration à courbure constante, dont
aucun ne connaît la scène.

### Le comportement neuf

Dans `swept-core` :

- une bordure plus basse que la garde au sol n'arrête pas la carrosserie ;
- la même bordure arrête une roue ;
- un muret plus haut que la garde au sol arrête tout ;
- une bordure exactement à la hauteur de la garde au sol est surplombée ;
- un coin d'obstacle bas à l'intérieur de la caisse n'est pas une collision ;
- les quatre roues sont aux coins de la caisse ;
- une garde au sol nulle ou négative est refusée.

Dans `swept-solver`, le test qui porte le lot : **sur le portail de Fabien, une
bordure déclarée basse donne au moins autant de marge qu'une bordure pleine**,
et le nombre de manœuvres ne peut qu'y gagner. Une régression ici dirait que le
pré-tri retire des obstacles qu'il ne devrait pas.

Dans `swept-wasm` :

- `metres_overhanging` vaut zéro quand rien ne surplombe ;
- il est strictement positif sur une trajectoire qui déborde ;
- il ne dépasse jamais la distance totale parcourue.

## Ce que le lot ne fait pas

- **Pas d'obstacles arbitraires** — jardinière, borne, poteau. La seule hauteur
  saisissable est celle de la bordure. Le jour où des obstacles libres
  arriveront, ils porteront une hauteur par construction.
- **Pas de franchissement** : une roue ne monte jamais sur un trottoir.
- **Rien dans la fonction de coût.** Le surplomb est signalé, pas pénalisé.
- **Pas de hauteurs détaillées** côté véhicule.
- **Pas de troisième dimension** : le modèle reste plan, avec une hauteur
  scalaire par obstacle. Un obstacle en surplomb — un balcon, un linteau de
  porche — n'est toujours pas représentable.

## Les réserves connues

**La garde au sol décide du verdict.** Pour un SUV compact elle vaut environ
0,18 m, pour une berline basse 0,12 m — soit exactement la hauteur d'une
bordure. Le verdict bascule alors sur un centimètre, sur une valeur publiée à
vide dont la norme de mesure varie. C'est un paramètre que l'utilisateur doit
regarder, pas subir : d'où la provenance affichée.

**L'empreinte de pneu est réduite à un point.** Une empreinte fait une
vingtaine de centimètres ; la traiter comme un point suppose que les marges en
jeu sont plus grandes, ce qui est vrai sur un bateau de 3,20 m mais pas
garanti partout.

**Le surplomb ne connaît que le plan.** Un pare-chocs qui surplombe légalement
un trottoir peut heurter un poteau, un panneau ou une borne que le modèle
ignore. C'est précisément pourquoi la distance de surplomb est rapportée plutôt
que passée sous silence.
