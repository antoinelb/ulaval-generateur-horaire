# Découpage des jalons en versions livrables v0/v1/v2

Date : 2026-07-10

## Contexte

Les dix jalons hebdomadaires formaient une liste plate, découpée seulement selon la portée du mandat (cœur = semaines 1–5, vision complète = 6–10).
Ce découpage mesure les heures facturables, pas ce qui est livrable : rien n'identifiait le plus petit produit utilisable de bout en bout ni les paliers suivants.

## Décision

Trois versions, chacune utilisable de bout en bout :

- **v0 (MVP)** — jalons 1–3 : entrer des codes de cours pour une session → horaire créé automatiquement, conflits évidents surlignés, ajout/retrait de cours, nombre de crédits affiché.
- **v1** — jalons 4–6 : choisir les cours d'une liste (recherche, filtres, catalogue complet), le programme présentant ses cours selon ses règles et profils.
- **v2** — jalons 7–10 : le bac complet — les sessions se remplissent automatiquement (organigramme, couverture des règles, génération sous contraintes) et restent modifiables ; préférences et partage en polissage final.

Deux détails migrent entre jalons pour respecter la définition de la v0 :

- Le surlignage des plages en conflit passe du jalon 4 au jalon 3 (la v0 exige les « conflits évidents »).
- La recherche et les filtres passent du jalon 3 au jalon 4 (la v0 entre les cours par code, sans liste).

Deux exigences deviennent explicites dans le plan :

- L'affichage du nombre total de crédits (v0) — absent de la liste de fonctionnalités jusqu'ici.
- Les **profils** de programme (v1) — déjà présents dans les cas de test du parseur (`tests/fixtures/test_cases/programs/*.json`, clé `profiles`), mais absents des fonctionnalités et du jalon 6 ; aucune nouvelle source de données n'est requise.

Le découpage cœur/vision reste en vigueur pour les heures du mandat ; il ne coïncide pas avec les versions (le cœur = v0 + jalons 4–5).

## Alternatives rejetées

- **Aligner les versions sur la frontière cœur/vision (v0 = semaines 1–5)** : la v0 serait livrable seulement à la semaine 5 alors que les jalons 1–3 suffisent à un produit utilisable ; les deux axes mesurent des choses différentes.
- **Une v3 pour préférences et partage (jalon 10)** : une version d'un seul jalon de polissage n'apporte rien ; le jalon 10 clôt la v2.
- **Réordonner les jalons** : l'ordre de construction (scraper → cœur → UI → cron) reste optimal ; seuls deux détails changent de jalon.
