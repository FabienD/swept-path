/**
 * Every fixed string the interface shows.
 *
 * Fixed meaning: it takes no measurement and no count. Anything that has to
 * be built around a figure lives in `domain/labels.ts`, where it can be
 * assembled in each language's own word order rather than by gluing fragments
 * together — the mistake that makes translated interfaces read like
 * translations.
 *
 * French is the reference: the project is French and its author writes the
 * French first. English follows it.
 */
import type { Locale } from "./preferences";

/** The keys, and therefore what a complete translation must supply. */
export interface Texts {
  "app.name": string;
  "app.nameAccent": string;
  "app.tagline": string;
  "app.planLabel": string;

  "settings.language": string;
  "settings.units": string;
  "units.metric": string;
  "units.us": string;

  "field.vehicle": string;
  "field.vehicleFilter": string;
  "field.opening": string;
  "field.road": string;
  "field.sidewalk": string;
  "field.posts": string;
  "field.gate": string;
  "gate.sliding": string;
  "gate.swinging": string;

  "action.compute": string;
  "action.stop": string;
  "action.minRoad": string;
  "fine.summary": string;

  "group.leaves": string;
  "field.leafLength": string;
  "field.leafThickness": string;
  "field.hingeOffset": string;
  "field.hingeDepth": string;
  "field.openAngle": string;

  "group.gateway": string;
  "field.postDepth": string;
  "field.wall": string;

  "group.street": string;
  "field.curbCut": string;
  "field.curbHeight": string;
  "field.side": string;
  "side.fromLeft": string;
  "side.fromRight": string;

  "group.vehicle": string;
  "field.mirrors": string;
  "mirrors.out": string;
  "mirrors.folded": string;
  "field.radius": string;
  "field.wheelbase": string;
  "field.length": string;
  "field.frontOverhang": string;
  "field.bodyWidth": string;
  "field.groundClearance": string;
  "field.mirrorWidth": string;
  "field.mirrorWidthFolded": string;

  "group.search": string;
  "field.direction": string;
  "direction.any": string;
  "direction.forward": string;
  "direction.reverse": string;

  "gauge.title": string;
  "gauge.ceiling": string;

  "play.play": string;
  "play.pause": string;
  "play.still": string;
  "play.scrub": string;
  "gear.forward": string;
  "gear.reverse": string;

  "band.clear": string;
  "band.watch": string;
  "band.close": string;
  "band.tight": string;
  "legend.overhang": string;
  "legend.gears": string;

  "stats.moves": string;
  "stats.gatewayClearance": string;
  "stats.tripClearance": string;
  "stats.distance": string;
  "stats.overhang": string;

  "msg.interrupted": string;
  "msg.firstPass": string;
  "msg.minRoadSearching": string;
}

export type TextKey = keyof Texts;

const FR: Texts = {
  "app.name": "Épure",
  "app.nameAccent": "de giration",
  "app.tagline": "Ce véhicule franchit-il ce passage, et en combien de manœuvres.",
  "app.planLabel": "Vue en plan de la manœuvre",

  "settings.language": "Langue",
  "settings.units": "Unités",
  "units.metric": "Mètres",
  "units.us": "Pouces",

  "field.vehicle": "Véhicule",
  "field.vehicleFilter": "Filtrer — marque ou modèle",
  "field.opening": "Passage",
  "field.road": "Chaussée",
  "field.sidewalk": "Trottoir",
  "field.posts": "Piliers",
  "field.gate": "Portail",
  "gate.sliding": "Coulissant",
  "gate.swinging": "Battants",

  "action.compute": "Calculer",
  "action.stop": "Arrêter",
  "action.minRoad": "Chaussée minimale",
  "fine.summary": "Réglages fins",

  "group.leaves": "Les vantaux",
  "field.leafLength": "Longueur",
  "field.leafThickness": "Épaisseur",
  "field.hingeOffset": "Écartement de l'axe",
  "field.hingeDepth": "Position de l'axe %",
  "field.openAngle": "Ouverture",

  "group.gateway": "Le passage",
  "field.postDepth": "Profondeur des piliers",
  "field.wall": "Épaisseur des murets",

  "group.street": "La voirie",
  "field.curbCut": "Largeur du bateau",
  "field.curbHeight": "Hauteur de bordure",
  "field.side": "Sens d'arrivée",
  "side.fromLeft": "Par la gauche",
  "side.fromRight": "Par la droite",

  "group.vehicle": "Le véhicule",
  "field.mirrors": "Rétroviseurs",
  "mirrors.out": "Déployés",
  "mirrors.folded": "Rabattus",
  "field.radius": "Rayon de pivot",
  "field.wheelbase": "Empattement",
  "field.length": "Longueur totale",
  "field.frontOverhang": "Porte-à-faux avant",
  "field.bodyWidth": "Largeur caisse",
  "field.groundClearance": "Garde au sol",
  "field.mirrorWidth": "Rétros déployés",
  "field.mirrorWidthFolded": "Rétros rabattus",

  "group.search": "La recherche",
  "field.direction": "Sens de marche",
  "direction.any": "Avant ou arrière",
  "direction.forward": "Marche avant seulement",
  "direction.reverse": "Marche arrière seulement",

  "gauge.title": "Marge la plus étroite",
  "gauge.ceiling": "le maximum possible ici",

  "play.play": "Lire la manœuvre",
  "play.pause": "Pause",
  "play.still": "Montrer l'épure",
  "play.scrub": "Position sur le trajet",
  "gear.forward": "AVANT",
  "gear.reverse": "ARRIÈRE",

  "band.clear": "au large",
  "band.watch": "vigilance",
  "band.close": "proche",
  "band.tight": "très proche",
  "legend.overhang": "surplomb du trottoir",
  "legend.gears": "trait plein : marche avant · pointillé : marche arrière",

  "stats.moves": "Manœuvres",
  "stats.gatewayClearance": "Marge dans le passage",
  "stats.tripClearance": "Marge minimale du trajet",
  "stats.distance": "Distance parcourue",
  "stats.overhang": "Surplomb du trottoir",

  "msg.interrupted": "Calcul interrompu.",
  "msg.firstPass": "Calcul des trajectoires en une manœuvre…",
  "msg.minRoadSearching": "Recherche de la chaussée minimale…",
};

const EN: Texts = {
  "app.name": "Swept",
  "app.nameAccent": "path",
  "app.tagline": "Does this vehicle clear this opening, and in how many moves.",
  "app.planLabel": "Plan view of the manoeuvre",

  "settings.language": "Language",
  "settings.units": "Units",
  "units.metric": "Meters",
  "units.us": "Inches",

  "field.vehicle": "Vehicle",
  "field.vehicleFilter": "Filter — make or model",
  "field.opening": "Opening",
  "field.road": "Roadway",
  "field.sidewalk": "Sidewalk",
  "field.posts": "Posts",
  "field.gate": "Gate",
  "gate.sliding": "Sliding",
  "gate.swinging": "Swinging",

  "action.compute": "Compute",
  "action.stop": "Stop",
  "action.minRoad": "Narrowest roadway",
  "fine.summary": "Detailed settings",

  "group.leaves": "The leaves",
  "field.leafLength": "Length",
  "field.leafThickness": "Thickness",
  "field.hingeOffset": "Hinge offset",
  "field.hingeDepth": "Hinge depth %",
  "field.openAngle": "Opening angle",

  "group.gateway": "The gateway",
  "field.postDepth": "Post depth",
  "field.wall": "Wall thickness",

  "group.street": "The street",
  "field.curbCut": "Curb cut width",
  "field.curbHeight": "Curb height",
  "field.side": "Approach",
  "side.fromLeft": "From the left",
  "side.fromRight": "From the right",

  "group.vehicle": "The vehicle",
  "field.mirrors": "Mirrors",
  "mirrors.out": "Extended",
  "mirrors.folded": "Folded",
  "field.radius": "Pivot radius",
  "field.wheelbase": "Wheelbase",
  "field.length": "Overall length",
  "field.frontOverhang": "Front overhang",
  "field.bodyWidth": "Body width",
  "field.groundClearance": "Ground clearance",
  "field.mirrorWidth": "Width, mirrors out",
  "field.mirrorWidthFolded": "Width, mirrors folded",

  "group.search": "The search",
  "field.direction": "Gear",
  "direction.any": "Forward or reverse",
  "direction.forward": "Forward only",
  "direction.reverse": "Reverse only",

  "gauge.title": "Tightest clearance",
  "gauge.ceiling": "the most possible here",

  "play.play": "Play the manoeuvre",
  "play.pause": "Pause",
  "play.still": "Show the swept path",
  "play.scrub": "Position along the path",
  "gear.forward": "FORWARD",
  "gear.reverse": "REVERSE",

  "band.clear": "clear",
  "band.watch": "watch",
  "band.close": "close",
  "band.tight": "very close",
  "legend.overhang": "overhanging the sidewalk",
  "legend.gears": "solid: forward · dashed: reverse",

  "stats.moves": "Moves",
  "stats.gatewayClearance": "Clearance in the gateway",
  "stats.tripClearance": "Tightest anywhere on the trip",
  "stats.distance": "Distance travelled",
  "stats.overhang": "Overhanging the sidewalk",

  "msg.interrupted": "Search stopped.",
  "msg.firstPass": "Computing single-move trajectories…",
  "msg.minRoadSearching": "Looking for the narrowest roadway…",
};

const DICTIONARIES: Record<Locale, Texts> = { fr: FR, en: EN };

/** One fixed string, in one language. */
export function text(locale: Locale, key: TextKey): string {
  return DICTIONARIES[locale][key];
}

/** Every key, so the page can be walked and a translation checked complete. */
export const TEXT_KEYS = Object.keys(FR) as TextKey[];
