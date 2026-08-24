# US-89 — B-GPH, « Profil international »

**Persona** : Adam, en génie physique, qui part une session à l'étranger sans renoncer à sa concentration.
**Intention** : réserver sa session d'échange, une concentration restant sélectionnée en même temps.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que le profil ajoute

- Un seul cours obligatoire : `EHE-1GPH`.
- Aucune règle, aucun `credits_required`.

## Scénario

1. Adam choisit la concentration « Aéronautique et aérospatiale » dans le premier menu, puis le profil « Profil international » dans le second, la dernière des neuf options — les deux menus du panneau sont indépendants.
2. Il cherche `EHE-1GPH` dans le panneau.
3. Il cherche de quoi représenter les cours suivis à l'étranger.

## Résultats attendus

- Le panneau affiche deux groupes : « Concentration — Aéronautique et aérospatiale », avec ses cartes habituelles, et « Profil — Profil international », qui n'affiche qu'une carte, « Cours obligatoires ».
- Le bilan affiche une section `Profil international` sans total déclaré, juste sous la section de la concentration qui, elle, affiche sa propre progression.
- Choisir ce profil ne masque plus la concentration : les deux groupes restent visibles et se combinent, aucun des deux ne fait disparaître l'autre.

## Repères pour le test e2e

Les sélecteurs `#cheminement-select` et `.rule-card` sont ceux du DOM de l'application JS soeur (`grille-de-cheminement-interactive`).
Les textes cités entre guillemets — « Concentration — … », « Profil — … » — sont ceux de l'UI Rust (`crates/ui/src/panel.rs`), qui offre deux menus `panel-knob` séparés plutôt qu'un menu combiné.

- `#cheminement-select option` compte neuf entrées, « Profil international » en dernier.
- Le groupe « Profil — Profil international » n'affiche qu'une seule `.rule-card` ; le groupe « Concentration — Aéronautique et aérospatiale » affiche les siennes en plus, sans qu'aucune des deux listes ne change à cause de l'autre.

## Variantes et cas limites

- **Écart connu** : `EHE-1GPH` est absent du catalogue **et** de `b-gph/cours/cours-hors-catalogue.csv`, qui ne déclare que `LAN-GUES`. Titre vide, `0` crédit, avertissement de console.
- Le B-GPH n'a aucun pseudo-cours `OPT-ETR*` : Adam n'a rien à placer pour ses cours à l'étranger, contrairement au B-GEX, au B-GIN et au B-GMC.
- Le cumul concentration + profil international est désormais représentable : les deux menus indépendants du panneau permettent de les sélectionner ensemble.
  `EHE-1GPH` n'appartenant à aucune règle de concentration, rien ne se recoupe pour ce profil précis — contrairement au « Profil développement durable » du B-GCI (US-67), où un même cours crédite les deux portées à la fois.
