# Verdicts honnêtes et panneau jamais vide (corrections post-essai)

> **Amendé le 2026-08-30.** Deux points de cette page ont changé. Le repli `uncounted_panel` ne sert plus qu'aux deux erreurs de portée (concentration ou profil inconnu) : un dépassement marque sa seule règle, en rouge (ADR `2026-08-depassement-de-regle-en-statut-rouge`). Et plus aucun message ne porte de détail technique ni d'identifiant (ADR `2026-08-messages-d-erreur-sans-detail-technique`).

Date : 2026-08-13

## Contexte

Le second essai utilisateur (`docs/ux/rapport-etudiante-2026-08-13b.md`) a montré que l'application « mentait ou se taisait » dès qu'on la modifiait : panneau des règles effacé sur une règle sur-remplie (message anglais de core), verdict « vérifié ✓ (… horaires) » à côté d'une grille hachurée, erreur anglaise « has no Course in the request » quand un cours de règle était placé par puce, verdict global muet sur *quelle* session dépasse.

## Décisions

- **Le panneau ne va jamais au blanc.** Une erreur de `coverage_report` dégrade le *comptage*, pas l'affichage : les sections se rendent depuis le `Program` avec badges neutres (`uncounted_panel`), textes bruts et notes compris. Les deux erreurs atteignables au clic (`CountOverMax`/`CreditsOverMax`) sont traduites en français actionnable (« Retirez-en un ou déplacez une entente ») ; les autres reçoivent un enrobage français générique avec le détail technique.
- **Le verdict ne contredit jamais la grille.** Deux lignes factuelles, calculées côté UI, précèdent le verdict du solveur : « ⚠ Conflit d'horaire en {sessions} » (`solve::conflicted_sessions`) et « ⚠ Plafond dépassé en {sessions} » — et chaque carte du ruban en conflit porte « ⚠ conflit d'horaire » même quand la session n'est pas affichée. La parenthèse du ✓ dit désormais « une combinaison d'horaire possible par session » : c'est ce que `verify` prouve réellement, une section *forcée* pouvant toujours choisir le conflit.
- **Un cours étalé embarque toujours son `Course`.** `place_course` inscrit le code en électif, `remove_course` l'en retire, et `request_json`/`unplaced_codes` unissent défensivement les codes de `displayed_placement` aux électifs (l'intake déduplique) — une sauvegarde d'avant correctif guérit d'elle-même. C'était la cause du « MED-1100 is passed or pinned but has no Course ».
- **Une vérification qui échoue ne boucle pas** : `verify_failed` gèle l'auto-vérification jusqu'au prochain changement du plan ; l'erreur du solveur est enveloppée en français (« Le solveur n'a pas pu répondre — détail technique : … »).
- **Divers dits plutôt que cachés** : statut « sections forcées - … » quand une section est épinglée à la main, bouton « Libérer les sections forcées », « N cours hors grille » dans l'en-tête de la grille, légende au-dessus de la grille, en-têtes de jours collants, « + » des rangées validé par `validate_new_code` (hors-saison refusé avec raison), rangées « préalables non remplis (texte source) », programme nommé avec code et millésime + bouton « changer », lien de partage posé dans la barre d'adresse au lieu d'être déversé dans l'alerte.

## Alternative rejetée

- Faire porter les conflits par le protocole verify (le worker recalculerait chaque horaire hebdomadaire) : l'UI calcule déjà chaque grille par session — la dire au ruban et au verdict est de la présentation, pas du métier.
