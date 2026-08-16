# Interface bilingue et choix d'unités — design

Complément au [design de la refonte](2026-08-16-refonte-interface-design.md),
qui l'annonçait comme le lot suivant.

## Ce qui est décidé

**Deux langues**, français et anglais américain. Le français reste la
référence : le projet est français et son auteur écrit le français d'abord.

**Deux réglages indépendants**, langue et unités. Les combinaisons qui
paraissent bizarres sont les vraies : un Français qui mesure une voiture
importée veut des pouces en français, un anglophone vivant en France veut des
mètres en anglais. Choisir l'anglais ne touche donc pas aux unités.

**Mètres par défaut pour tout le monde**, y compris en `en-US`. La langue est
devinée depuis le navigateur ; les unités ne le sont jamais. Un portail est
une chose qu'on va mesurer soi-même, et le mètre ruban qu'on possède se
déduit mal de la langue du navigateur. L'impérial est un choix qu'on fait, pas
un choix qu'on subit.

**Le réglage est retenu dans le navigateur** (`localStorage`), et l'emporte
ensuite sur toute détection.

## L'unité dépend de ce que la longueur représente

L'impérial n'est pas une unité unique. Une fiche constructeur américaine donne
un empattement en pouces — « Wheelbase 101.6 in » — et une voirie se décrit en
pieds. Trois natures, donc, déclarées par le HTML dans `data-magnitude` :

| nature | métrique | US | ce que c'est |
|---|---|---|---|
| `clearance` | cm | in | une marge |
| `dimension` | m | in | un véhicule, un passage, un pilier |
| `distance` | m | ft | une chaussée, un trottoir, un trajet |

Tout en pouces afficherait la chaussée de 5,90 m à 232 in : exact, illisible.

Les décimales suivent : deux en métrique, une en impérial. 2,58 m vaut
101,5748 in, et tout ce qui suit la première décimale vient de la conversion,
non d'une mesure.

## Où vivent les mots

- `web/src/i18n/dictionary.ts` — tout ce qui ne prend ni mesure ni compte.
  Deux dictionnaires du même type, donc une traduction incomplète ne compile
  pas.
- `web/src/domain/labels.ts` — tout ce qui se construit autour d'un chiffre,
  assemblé dans l'ordre propre à chaque langue. Coller des fragments traduits
  est ce qui fait qu'une interface *sent* la traduction.

Le noyau et la frontière Wasm ne renvoient que des codes, jamais des phrases.
C'est cette règle, déjà tenue, qui a rendu ce lot bon marché.

## La conversion se fait à un seul endroit

`ui/form.ts` convertit en mètres à la lecture du formulaire. C'est la seule
frontière que toute mesure traverse. Sans cela, passer en pouces enverrait
90,2 **mètres** au solveur.

Changer de système réécrit les champs : ils contiennent ce que le lecteur a
tapé, dans l'unité où il l'a tapé. Laisser 2,29 dans un champ désormais
étiqueté « in » transformerait silencieusement un passage de 2,29 m en 5,8 cm.

## Ce que le HTML déclare, et pourquoi c'est testé

Le balisage porte deux contrats que TypeScript ne voit pas : `data-i18n` dit
par quelle clé un libellé se traduit, `data-magnitude` dit quelle unité porte
un champ. Les deux échouent en silence — une clé absente s'affiche vide, une
magnitude absente envoie des pouces à un solveur qui attend des mètres.

`i18n/page.test.ts` lit donc `index.html` comme du texte et vérifie les deux,
sans DOM. Il a déjà attrapé le nom du produit resté non traduit dans
l'en-tête.

## Hors périmètre

- **Le thème clair**, qui reste à faire et qui est désormais une réécriture du
  bloc de tokens, pas une passe sur le code de dessin.
- **Une troisième langue.** Rien ne l'empêche : ajouter une entrée au
  `Record<Locale, …>` fait échouer la compilation partout où il manque une
  traduction. Mais les phrases de `labels.ts` branchent sur `locale === "en"`,
  ce qui devra devenir une table le jour venu.
