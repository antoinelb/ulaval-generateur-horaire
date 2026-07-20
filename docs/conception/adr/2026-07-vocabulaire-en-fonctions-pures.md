# Vocabulaire de la page en fonctions pures

Date : 2026-07-19

## Contexte

Le parseur de cours mêlait deux choses de nature différente dans les mêmes fonctions : naviguer le DOM (`ElementRef`, sélecteurs, imbrication des sections) et interpréter le vocabulaire d'ULaval (« À distance » → `Mode::Remote`, « Vendredi » → `Day::Friday`, « De 9h à 11h50 » → deux `Time`).

La conséquence se lisait dans les tests : une quinzaine de documents HTML n'existaient que pour atteindre un bras de `match` chaîne → énumération.
Vérifier que « Samedi » est reconnu demandait de monter un `<ul class="section-cours--liste">` avec trois `<li>` étiquetés.

La moitié « préalables » du fichier ne souffrait pas du problème : la grammaire est déjà une fonction pure `&str → PrereqTree` (ADR `2026-07-conception-du-parseur-de-cours`), testée par tables.
Il s'agissait d'amener la moitié « horaires » à la même forme, pas d'inventer une architecture.

## Décision

1. **Chaque valeur écrite par ULaval est lue par une fonction pure `&str → valeur`** : `parse_mode`, `parse_day`, `parse_schedule` sont extraites ; `parse_session_heading`, `advertised_section_count`, `parse_time` et `cycle_level` l'étaient déjà.
   Les fonctions qui prennent un `ElementRef` ne font plus que de la navigation.
2. **Les tables de vocabulaire sont testées sur des chaînes**, pas sur du HTML : les orthographes acceptées et refusées se lisent alors comme la table qu'elles sont.
3. **Un test au niveau HTML subsiste par point de propagation**, un par `?` menant d'une valeur refusée à l'erreur du cours.
   Une table pure ne prouve pas qu'une valeur refusée *remonte* : elle prouve seulement qu'elle est refusée.
   L'invariant « ne jamais ignorer une entrée non reconnue » (`docs/project_plan.md` § Contraintes) vit dans cette propagation, donc elle se teste.
4. **`parse_section_header` lit ses éléments par motif de tranche** (`[_, section, mode]` au premier niveau, `[section, mode]` pour une section liée) plutôt que par une largeur calculée et des index `items[width - 1]`.
   Le contrôle de forme et la lecture deviennent la même expression.

## Conséquences

Le fichier passe à 100 % de couverture (ADR `2026-07-couverture-par-instanciation-le-plus-petit-ecart`), et la lecture d'un vocabulaire nouveau se corrige désormais dans une fonction de dix lignes sans toucher au DOM.

Le point 3 n'est pas théorique : la conversion des tables en tests purs a d'abord *retiré* la couverture des deux `?` de `parse_section_header` et `parse_slot`, écart signalé par `make test` avant d'être comblé.

## Alternatives rejetées

- **Réécriture complète en deux couches** (DOM → `RawSection`/`RawPlage` → domaine) : ajouterait un troisième modèle de données à tenir synchronisé avec le HTML *et* avec `core::Course`, et réécrirait environ 800 lignes de tests qui passent, pour un gain de lisibilité déjà obtenu par l'extraction du vocabulaire.
  L'ADR `2026-07-conception-du-parseur-de-cours` §1 a écarté une frontière de module artificielle sur la même page pour la même raison.
- **Garder les tables en HTML** : chaque orthographe coûte un document à monter, et la table cesse d'être lisible comme une table.
- **Supprimer les tests HTML une fois le vocabulaire extrait** : c'est précisément ce qui a coûté les deux régions de propagation ci-dessus.
