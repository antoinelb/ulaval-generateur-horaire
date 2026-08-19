# L'entente rattache un cours à une règle « tous les cours » (any)

Date : 2026-08-19

Révise `2026-08-entente-cours-regle-et-scolarite-preparatoire.md`, qui rangeait la règle « any » parmi les ententes inapplicables (« accepte déjà tout »).

## Contexte

La règle 5 du B-GEX (« 3 crédits parmi tous les cours de premier cycle… ») est un mot-clé `any` : core ne la compte jamais (`resolved_courses` → `None` → statut `reported`).
Le raisonnement de l'ADR précédent — « une règle any accepte déjà tout » — était erroné : elle n'accepte rien, faute de liste.
Conséquences : aucun moyen d'y rattacher un cours (ni le sélecteur d'entente, ni la navigation libre, qui ajoutait aux électifs sans rattachement), badge figé à « — », « Cheminement vérifié ✓ » mentant de 3 crédits, et le pseudo-cours AUC-HOIX crédité déclenchait l'avertissement « n'apparaît dans aucune règle ».

## Décisions

- **`Keyword::Any` devient une cible d'entente**, exactement comme `negotiated` : `panel::granted_program` transforme la règle en liste de ses ententes, core la compte alors normalement. **Core inchangé** — sans entente, `any` reste `reported` (la fixture `gex-rule5-any-reported.json` reste valable).
- **La section garde son browse et son texte** après la transformation : `rule_section`/`bare_section` lisent `free` et le repli `raw` sur la règle *originale* du programme, pas sur la règle accordée. La section liste ses cours rattachés puis offre la recherche.
- **Prendre un cours depuis le browse d'une règle any enregistre l'entente automatiquement** (`panel::grant_on_take`, dans la même transaction annulable) — uniquement au premier take (`Choice::Not`) et jamais en écrasant une entente existante. « Une entente déplace » s'applique : le cours quitte la liste de toute autre règle.
- Le ✕ purge l'entente avec le cours (voir `2026-08-decrediter-reprend-le-cours-en-electif`), sans distinguer ententes automatiques et manuelles.
- **Le badge d'une règle rapportée qui porte une contrainte dit ce qui manque** (« 0/3 cr ») au lieu de « — » : puisqu'une entente peut désormais la remplir, la règle est une vraie exigence, pas une simple mention — et elle entre du même coup dans le décompte « règles à combler » du verdict.

## Alternatives rejetées

- **Compter `any` côté core** comme « toute la sélection non consommée par une autre règle » : exigerait un ordre d'attribution entre règles qui n'existe pas (`split_selection` est une intersection ensembliste) — l'entente explicite est plus simple et plus fidèle (c'est l'étudiant qui décide quel cours remplit la règle).
- **Sélecteur d'entente seul, sans grant automatique au browse** : chercher un cours *dans la section de la règle 5* puis devoir le rattacher à la main à cette même règle est une étape que rien ne justifie ; le badge serait resté « — » pour quiconque l'ignore.
