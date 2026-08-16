/**
 * The prose of the documentation and disclaimer pages.
 *
 * Kept apart from `dictionary.ts`, which holds interface labels: these are
 * paragraphs, they change for editorial reasons rather than functional ones,
 * and mixing a hundred lines of prose into the labels would bury them.
 *
 * Both languages are written together, and the completeness test covers them
 * exactly as it covers the labels. A disclaimer nobody can read protects
 * nobody, which is why English was not deferred here.
 */
import type { Locale } from "./preferences";

export interface PageTexts {
  "nav.simulator": string;
  "nav.documentation": string;
  "nav.disclaimer": string;
  "nav.back": string;

  "doc.title": string;
  "doc.lede": string;

  "doc.width.title": string;
  "doc.width.body": string;
  "doc.width.figure": string;

  "doc.angle.title": string;
  "doc.angle.body": string;
  "doc.angle.figure": string;

  "doc.mirrors.title": string;
  "doc.mirrors.body": string;

  "doc.search.title": string;
  "doc.search.body": string;
  "doc.search.exact": string;
  "doc.search.planner": string;
  "doc.search.minroad": string;

  "doc.proof.title": string;
  "doc.proof.body": string;

  "doc.bands.title": string;
  "doc.bands.body": string;
  "doc.bands.figure": string;

  "doc.curb.title": string;
  "doc.curb.body": string;

  "doc.limits.title": string;
  "doc.limits.body": string;

  "doc.source.title": string;
  "doc.source.body": string;
  "doc.source.link": string;

  "dis.title": string;
  "dis.lede": string;
  "dis.model.title": string;
  "dis.model.body": string;
  "dis.measure.title": string;
  "dis.measure.body": string;
  "dis.traffic.title": string;
  "dis.traffic.body": string;
  "dis.driver.title": string;
  "dis.driver.body": string;
  "dis.use.title": string;
  "dis.use.body": string;
  "dis.privacy.title": string;
  "dis.privacy.body": string;
}

const FR: PageTexts = {
  "nav.simulator": "Le simulateur",
  "nav.documentation": "Comment ça marche",
  "nav.disclaimer": "Avertissement",
  "nav.back": "Retour au simulateur",

  "doc.title": "Comment ça marche",
  "doc.lede":
    "Ce simulateur répond à une question : ce véhicule franchit-il ce passage, et en combien de manœuvres. Voici comment il s'y prend, et ce que ses réponses valent.",

  "doc.width.title": "La largeur décide de tout",
  "doc.width.body":
    "C'est la conclusion principale du projet, et elle est décevante : multiplier les manœuvres n'achète pas de marge. Quelle que soit la trajectoire, le meilleur écart possible de chaque côté vaut (passage − véhicule) ÷ 2. Sur un passage de 2,40 m avec un véhicule de 2,03 m aux rétroviseurs, cela fait 18,5 cm — et aucune habileté, aucune reprise, aucun angle d'approche ne fera mieux. Une manœuvre supplémentaire sert à se présenter droit, jamais à gagner de la place.",
  "doc.width.figure":
    "Le plafond de marge : ce qui reste de chaque côté quand le véhicule est centré et droit.",

  "doc.angle.title": "Entrer en biais coûte très cher",
  "doc.angle.body":
    "Un véhicule qui franchit un couloir de travers n'occupe pas sa largeur, mais davantage. À un écart α de la perpendiculaire, l'emprise vaut w ÷ cos α + L × tan α, où w est la largeur et L la profondeur du couloir. Le second terme est le coupable : il croît sans borne. Un pilier de 55 cm de profondeur suffit à rendre 15° d'obliquité plus coûteux que 15 cm de largeur en moins : 22 cm d'emprise supplémentaire. Sur le passage de 2,40 m de la scène de référence, l'ouverture est entièrement consommée dès 21,6°. C'est pourquoi le simulateur n'accepte une entrée que si le véhicule arrive à moins de cinq degrés de la perpendiculaire.",
  "doc.angle.figure":
    "Le même véhicule, droit puis à 15° : l'emprise dans le passage n'est plus la largeur.",

  "doc.mirrors.title": "Ce sont les rétroviseurs qui touchent",
  "doc.mirrors.body":
    "Le véhicule est modélisé par quatorze points, dont deux comptent plus que les autres : les rétroviseurs, à hauteur d'essieu avant. C'est presque toujours le point le plus large, et presque toujours ce qui touche en premier. Les rabattre change le verdict plus sûrement que n'importe quel réglage — d'où la case à cocher, et d'où l'importance de saisir la vraie largeur, déployés puis rabattus. Trois centimètres d'erreur suffisent à inverser une conclusion.",

  "doc.search.title": "Trois recherches, pas une",
  "doc.search.body":
    "Le simulateur ne cherche pas au hasard : il enchaîne trois méthodes, de la plus sûre à la plus souple.",
  "doc.search.exact":
    "La recherche exacte balaie toutes les poses de départ le long de la chaussée et toutes les poses d'arrivée dans le passage, et les relie par toutes les courbes qu'un véhicule peut décrire à braquage constant — les courbes de Dubins en marche avant, de Reeds-Shepp quand la marche arrière est permise. Le balayage étant complet, son échec veut dire quelque chose.",
  "doc.search.planner":
    "Le planificateur multi-manœuvres prend le relais quand un seul mouvement ne suffit pas. C'est un A* hybride qui explore des positions, des caps et des sens de marche, en payant cher chaque changement de sens — un conducteur compte les marches arrière, pas les mètres.",
  "doc.search.minroad":
    "La chaussée minimale est une dichotomie : elle cherche la largeur de rue en deçà de laquelle l'entrée en un mouvement devient impossible.",

  "doc.proof.title": "Ce qu'un résultat prouve, et ce qu'il ne prouve pas",
  "doc.proof.body":
    "Une marge n'est jamais affichée sans sa provenance, parce que les deux ne se séparent pas. Une recherche exacte est exhaustive sur sa grille : quand elle ne trouve rien, elle le prouve. Le planificateur, lui, n'est fiable que dans un sens — ce qu'il trouve est vérifié en collision et donc réel, mais ce qu'il ne trouve pas peut exister quand même. C'est pourquoi l'interface écrit « Rien trouvé, sans preuve » plutôt que « ça ne passe pas » lorsque le budget de recherche s'épuise. Un seul cas se passe de recherche : un véhicule plus large que son passage ne passe pas, et cela se démontre sans rien calculer.",

  "doc.bands.title": "Lire une trajectoire",
  "doc.bands.body":
    "Le tracé change de couleur selon la place qui reste : bleu au large, ambre en vigilance, orange quand ça devient proche, rouge sous dix centimètres. Un trait plein signale la marche avant, un pointillé la marche arrière, et un rond marque chaque changement de sens — l'endroit où il faut s'arrêter et repasser une vitesse. C'est cette lecture par la couleur qui dit où le passage se joue, ce qu'un chiffre unique ne dira jamais.",
  "doc.bands.figure": "Les quatre bandes de proximité, du plus large au plus serré.",

  "doc.curb.title": "Une bordure n'est pas un mur",
  "doc.curb.body":
    "Le modèle est en deux dimensions, mais tous les obstacles n'arrêtent pas tout. Une bordure de trottoir de douze centimètres se surplombe : la carrosserie passe au-dessus si la garde au sol le permet. Les roues, elles, ne survolent rien. Le simulateur compare donc la hauteur de chaque obstacle à la garde au sol du véhicule, et signale les portions de trajet où la caisse déborde au-dessus du trottoir sans que cela bloque.",

  "doc.limits.title": "Ce que le simulateur ne sait pas",
  "doc.limits.body":
    "Il ignore la pente, le dévers, le braquage qui se resserre à basse vitesse, la souplesse des suspensions, les véhicules stationnés que vous n'avez pas saisis, et le fait qu'un conducteur voit ce qu'il fait. Il suppose aussi que la scène est symétrique autour du passage, ce qui n'est pas toujours vrai. Il calcule une géométrie, pas une conduite.",

  "doc.source.title": "Le détail",
  "doc.source.body":
    "Le code est ouvert, les constantes arbitraires sont marquées comme telles, et les résultats de référence portent la mention de ce qui a été vérifié ou infirmé par la mesure.",
  "doc.source.link": "Voir le code sur GitHub",

  "dis.title": "Avertissement",
  "dis.lede":
    "Cet outil est un simulateur. Il calcule ce que la géométrie autorise, à partir des chiffres que vous lui donnez. Ce n'est ni une garantie, ni un conseil, ni une mesure.",

  "dis.model.title": "Un modèle reste un modèle",
  "dis.model.body":
    "Le véhicule y est un rectangle qui roule sans glisser, sur un sol plat, entre des obstacles rectangulaires. La réalité comporte des pentes, des dévers, des bordures biseautées, des poteaux ronds, des poubelles et un braquage qui n'est pas tout à fait celui de la fiche technique. Un résultat favorable de quelques centimètres est donc à traiter comme un résultat incertain.",

  "dis.measure.title": "Vos mesures décident du résultat",
  "dis.measure.body":
    "Le calcul est exact ; les chiffres qu'il reçoit ne le sont pas forcément. Trois centimètres d'erreur sur la largeur aux rétroviseurs suffisent à faire basculer une conclusion. Mesurez votre passage au mètre ruban, au point le plus étroit, et votre véhicule rétroviseurs déployés — les fiches constructeur donnent rarement cette cote.",

  "dis.traffic.title": "La circulation complique tout",
  "dis.traffic.body":
    "Le simulateur dispose de toute la chaussée et de tout le temps du monde. Vous n'aurez ni l'un ni l'autre. Une voiture arrivant en face, un véhicule stationné en vis-à-vis, un piéton sur le trottoir, ou simplement quelqu'un qui attend derrière vous transforment une manœuvre calculée en manœuvre impossible. Une entrée qui demande trois reprises sur le papier en demandera davantage dans le trafic.",

  "dis.driver.title": "Et vous n'êtes pas un solveur",
  "dis.driver.body":
    "Les trajectoires calculées sont géométriquement optimales, ce qui ne veut pas dire reproductibles. Elles supposent un braquage posé au bon moment, au bon endroit, sans corriger. Prévoyez de la marge pour la conduite réelle, la visibilité, et le fait qu'on ne voit pas ses propres rétroviseurs frôler un pilier.",

  "dis.use.title": "En pratique",
  "dis.use.body":
    "Servez-vous en pour écarter ce qui est manifestement impossible, pour comparer des ouvertures, ou pour savoir ce qu'un élargissement de portail vous ferait gagner. Avant de couper un mur ou d'acheter un véhicule, vérifiez sur place, à vitesse d'homme, avec quelqu'un dehors pour regarder.",

  "dis.privacy.title": "Ce que ce site sait de vous",
  "dis.privacy.body":
    "Rien. Le calcul se fait entièrement dans votre navigateur : les mesures que vous saisissez ne sont envoyées nulle part, et il n'existe aucun serveur pour les recevoir. Votre choix de langue et d'unités reste chez vous, dans la mémoire du navigateur. Il n'y a ni cookie, ni compte, ni mesure d'audience. L'hébergeur tient des journaux de connexion, comme tout hébergeur.",
};

const EN: PageTexts = {
  "nav.simulator": "The simulator",
  "nav.documentation": "How it works",
  "nav.disclaimer": "Disclaimer",
  "nav.back": "Back to the simulator",

  "doc.title": "How it works",
  "doc.lede":
    "This simulator answers one question: does this vehicle clear this opening, and in how many moves. Here is how it goes about it, and what its answers are worth.",

  "doc.width.title": "Width decides everything",
  "doc.width.body":
    "This is the project's main finding, and it is a disappointing one: extra manoeuvres buy no room. Whatever the trajectory, the best possible clearance on each side is (opening − vehicle) ÷ 2. On a 2.40 m opening with a vehicle 2.03 m across the mirrors, that is 18.5 cm — and no skill, no shuffling and no approach angle will beat it. An extra move serves to arrive straight, never to gain space.",
  "doc.width.figure":
    "The clearance ceiling: what is left on each side with the vehicle centred and square.",

  "doc.angle.title": "Entering at an angle is expensive",
  "doc.angle.body":
    "A vehicle crossing a passage askew does not occupy its own width, but more. At α off the perpendicular, the span taken is w ÷ cos α + L × tan α, where w is the width and L the depth of the passage. The second term is the culprit: it grows without bound. A post 55 cm deep is enough to make 15° of skew cost more than 15 cm of lost width: 22 cm of extra span. On the reference gateway's 2.40 m opening, the whole opening is used up by 21.6°. That is why an entry only counts when the vehicle arrives within five degrees of square.",
  "doc.angle.figure":
    "The same vehicle, square and then at 15°: the span taken is no longer its width.",

  "doc.mirrors.title": "It is the mirrors that hit",
  "doc.mirrors.body":
    "The vehicle is modelled as fourteen points, two of which matter more than the rest: the mirrors, at front-axle height. They are almost always the widest point, and almost always what touches first. Folding them changes the verdict more reliably than any other setting — hence the checkbox, and hence the importance of entering the real width, out and folded. Three centimetres of error is enough to reverse a conclusion.",

  "doc.search.title": "Three searches, not one",
  "doc.search.body":
    "The simulator does not search at random: it runs three methods in turn, from the most certain to the most flexible.",
  "doc.search.exact":
    "The exact sweep tries every start pose along the roadway and every arrival pose in the gateway, joining them with every curve a vehicle can trace at constant steering — Dubins curves going forwards, Reeds-Shepp when reverse is allowed. Because the sweep is complete, its failure means something.",
  "doc.search.planner":
    "The multi-move planner takes over when one movement is not enough. It is a hybrid A* over positions, headings and gears, charging heavily for each change of direction — a driver counts reverses, not metres.",
  "doc.search.minroad":
    "The narrowest roadway is a bisection: it looks for the street width below which a single-move entry stops being possible.",

  "doc.proof.title": "What a result proves, and what it does not",
  "doc.proof.body":
    "A clearance is never shown without where it came from, because the two do not separate. An exact sweep is exhaustive on its grid: when it finds nothing, it has proved something. The planner is reliable in one direction only — what it finds is collision-checked and therefore real, but what it misses may exist all the same. That is why the interface says \"Nothing found, not proven\" rather than \"it does not fit\" when the search budget runs out. One case needs no search at all: a vehicle wider than its opening does not fit, and that is settled without computing anything.",

  "doc.bands.title": "Reading a trajectory",
  "doc.bands.body":
    "The trace changes colour with the room left: blue when clear, amber for watchfulness, orange when it gets close, red under ten centimetres. A solid line means forward, a dashed line reverse, and a circle marks every change of direction — where you have to stop and shift. Reading by colour is what shows where the passage is actually decided, which a single figure never will.",
  "doc.bands.figure": "The four proximity bands, from roomiest to tightest.",

  "doc.curb.title": "A curb is not a wall",
  "doc.curb.body":
    "The model is two-dimensional, but not every obstacle stops everything. A twelve-centimetre curb can be overhung: the bodywork passes above it if ground clearance allows. The wheels overhang nothing. The simulator therefore compares each obstacle's height with the vehicle's ground clearance, and reports the stretches where the body overhangs the sidewalk without being blocked by it.",

  "doc.limits.title": "What the simulator does not know",
  "doc.limits.body":
    "It ignores slope, camber, the steering that tightens at walking pace, suspension travel, parked vehicles you did not enter, and the fact that a driver can see what they are doing. It also assumes the scene is symmetrical about the opening, which is not always true. It computes geometry, not driving.",

  "doc.source.title": "The detail",
  "doc.source.body":
    "The code is open, arbitrary constants are marked as such, and the reference results carry a note of what measurement has confirmed or disproved.",
  "doc.source.link": "See the code on GitHub",

  "dis.title": "Disclaimer",
  "dis.lede":
    "This tool is a simulator. It computes what geometry allows, from the figures you give it. It is not a guarantee, not advice, and not a measurement.",

  "dis.model.title": "A model stays a model",
  "dis.model.body":
    "In it the vehicle is a rectangle rolling without slipping, on flat ground, between rectangular obstacles. Reality has slopes, camber, chamfered curbs, round bollards, bins, and steering that is not quite what the spec sheet claims. A favourable result of a few centimetres should therefore be treated as an uncertain one.",

  "dis.measure.title": "Your measurements decide the result",
  "dis.measure.body":
    "The computation is exact; the figures it receives may not be. Three centimetres of error on the width across the mirrors is enough to flip a conclusion. Measure your opening with a tape, at its narrowest point, and your vehicle with the mirrors out — manufacturers rarely publish that figure.",

  "dis.traffic.title": "Traffic makes everything harder",
  "dis.traffic.body":
    "The simulator has the whole roadway and all the time in the world. You will have neither. A car coming the other way, a vehicle parked opposite, a pedestrian on the sidewalk, or simply someone waiting behind you turns a computed manoeuvre into an impossible one. An entry that takes three shuffles on paper will take more in traffic.",

  "dis.driver.title": "And you are not a solver",
  "dis.driver.body":
    "The computed paths are geometrically optimal, which does not make them repeatable. They assume the steering goes on at the right moment, in the right place, without correction. Leave room for real driving, for visibility, and for the fact that you cannot see your own mirror grazing a post.",

  "dis.use.title": "In practice",
  "dis.use.body":
    "Use it to rule out what is plainly impossible, to compare openings, or to see what widening a gateway would buy you. Before cutting a wall or buying a vehicle, check on site, at walking pace, with someone standing outside to watch.",

  "dis.privacy.title": "What this site knows about you",
  "dis.privacy.body":
    "Nothing. The computation happens entirely in your browser: the measurements you enter are sent nowhere, and no server exists to receive them. Your language and unit choices stay with you, in the browser's own storage. There are no cookies, no accounts and no analytics. The host keeps connection logs, as every host does.",
};

export const PAGE_TEXTS: Record<Locale, PageTexts> = { fr: FR, en: EN };
