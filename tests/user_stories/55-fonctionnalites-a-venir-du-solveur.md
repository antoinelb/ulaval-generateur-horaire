# US-55 — Fonctionnalités à venir apportées par le solveur

**Persona** : Daniel, directeur du B-GEX, qui a commandé le générateur.
**Intention** : passer d'une grille qu'on remplit à la main à une grille qui se propose elle-même.

Ces histoires ne sont pas testables aujourd'hui : elles décrivent ce que le module wasm du dépôt `generateur_horaire` apportera.
Elles sont écrites ici pour que les tests e2e soient prêts le jour de l'intégration.

## Génération de l'organigramme

1. L'étudiante saisit ses contraintes : cours déjà réussis, cours voulus, sessions figées à la main, session à l'étranger, plafond de crédits par session.
2. L'application appelle `generate_organigramme` et remplit les sessions restantes.
3. La grille reste entièrement modifiable après coup.

**Attendu** : le placement respecte les règles du programme, l'ordre des préalables et les saisons d'offre.
Une contrainte impossible à satisfaire est expliquée, jamais résolue en silence.

## Vérification d'un organigramme

1. L'étudiant remplit sa grille à la main et demande une vérification.
2. L'application appelle `verify_organigramme`, qui exécute à la fois le placement avec tout figé et le rapport de couverture des règles.

**Attendu** : vérifier une grille où un cours n'est pas figé est une **erreur**, jamais un verdict inventé.

## Horaire hebdomadaire automatique

1. L'étudiante choisit une session de sa grille.
2. L'application appelle `generate_schedule`, qui choisit une combinaison de sections sans conflit.

**Attendu** : les autres sections restent visibles; cliquer une section la force et le reste se recalcule autour.
S'il n'existe aucune combinaison valide, les plages en conflit sont identifiées et surlignées.

## Préférences d'horaire

1. L'étudiant exprime ses préférences : journées compactes, matins libres, pause dîner.
2. Les combinaisons valides sont classées selon ces préférences.

**Attendu** : la préférence ordonne, elle ne contraint pas — une combinaison mal classée reste choisissable.

## Repères pour le test e2e

- Ces histoires exigeront des données figées et des appels wasm déterministes.
- Le contrat d'échange est déjà défini côté Rust : `tests/fixtures/test_cases/organigrammes/*.json` et `schedules/*.json`.
- Aucune logique métier ne doit être réimplémentée ici : ce dépôt affiche, il ne calcule pas.
