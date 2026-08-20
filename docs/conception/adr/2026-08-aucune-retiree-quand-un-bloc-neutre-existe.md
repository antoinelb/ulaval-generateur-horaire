# « Aucune » disparaît quand la page porte un bloc neutre

## Contexte

Le sélecteur de concentration offrait toujours une option synthétique « Aucune » (valeur vide, ADR `2026-08-selection-concentration-et-profil-au-panneau`).
Pour un programme dont la page scrape un bloc « Cheminement sans concentration » (B-GCI : 15 cr, B-GMC : 18 cr) ou « Approche généraliste » (B-GIN), « Aucune » n'est pas un choix neutre : elle fait disparaître la règle de crédits de ce bloc du panneau, et l'étudiante sous-estime d'autant ce qu'il lui reste à faire (rapport étudiante-cegep 2026-08-19). Décision utilisateur du même jour : retirer l'option dans ce cas.

## Décision

- `cheminement_choices` porte `offers_none` : faux dès qu'une concentration scrapée est un bloc neutre (`neutral_concentration` : « Cheminement sans concentration », « Approche généraliste ») ; le knob ne rend alors pas l'option « Aucune ». Elle reste offerte pour les programmes à concentrations sans bloc neutre.
- Un plan restauré portant `concentration: None` pour un tel programme n'est pas réécrit en douce : le knob affiche simplement la sélection réelle (aucune option cochée), le premier geste de l'étudiante la fixe.
- Ceci supersède la phrase « un « Aucune » explicite est respecté et persiste » de l'ADR `2026-08-selection-concentration-et-profil-au-panneau` **pour les programmes à bloc neutre seulement**.

## Alternatives rejetées

- **Expliquer la différence sous le sélecteur** : deux options qui se ressemblent avec une note d'excuse restent un piège ; l'option en trop n'a pas de sens métier ici.
- **Détection structurelle (bloc référencé par les autres)** : les autres concentrations du B-GCI référencent bien le bloc neutre, mais pas partout (B-GIN) — les deux titres connus sont la règle simple et vérifiable.
