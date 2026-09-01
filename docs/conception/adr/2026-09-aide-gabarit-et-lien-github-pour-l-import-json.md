# Aide, gabarit et lien GitHub pour l'import de programme par fichier JSON

Date : 2026-09-01

## Contexte

Le tiroir « Votre programme n'est pas là ? » offre l'import par fichier `{code}-{semestre}.json` (ADR `2026-08-import-de-programme-par-fichier-json`) sans aucune explication : rien ne dit ce qu'est un fichier programme, d'où il vient, ni à quoi il ressemble.
Antoine demande plus d'information, un gabarit téléchargeable et un lien vers les programmes déjà publiés (2026-09-01).

## Décision

- L'information suit le patron LAY-4 existant (ADR `2026-08-vocabulaire-explique-en-place-a-la-demande`) : un bouton « ? » à côté du libellé « Fichier programme » déplie l'explication en place — zéro poids au repos, jamais de modal ni d'infobulle seule.
- Le gabarit est un **vrai instantané embarqué**, pas un squelette écrit à la main : un lien « Télécharger un exemple » porte l'attribut natif `download` vers l'asset `B-GEX-A26.json` que l'app embarque déjà — B-GEX (le programme du mandat) si présent, sinon le premier du manifeste (`present::example_program_file`, le repli évitant un lien mort sur un jeu de données réduit).
- Un second lien « Tous les programmes (GitHub) » ouvre `https://github.com/antoinelb/ulaval-generateur-horaire/tree/main/data/programmes` dans un nouvel onglet, même patron que le chip d'issue des cours manuels.
- Le message d'erreur `InvalidProgramJson` pointe désormais vers cette aide au lieu de citer « le scraper » avec des accolades littérales.

Cette décision ne rouvre pas l'ADR `2026-08-retrait-de-l-aller-retour-json-du-cheminement` : celui-ci visait le format *cheminement*, un état interne dont le JSON n'avait pas à remonter à l'écran.
L'instantané de programme est au contraire un artefact publié du scraper, déjà servi comme asset ; aucun helper JS de téléchargement (`browser::download_text`) n'est réintroduit — un `<a download>` même-origine suffit.

## Alternatives rejetées

- **Un gabarit squelette écrit à la main** : une seconde source de vérité du schéma `Program`, à maintenir à chaque évolution ; un vrai instantané est toujours juste et le dépôt GitHub montre déjà toute la variété.
- **Les liens toujours visibles sous le champ** : plus découvrables, mais un poids permanent dans un tiroir déjà dense — écarté par Antoine sur maquettes.
- **Ressusciter `browser::download_text`** : inutile pour un fichier déjà servi par l'app ; l'attribut `download` du navigateur rend son vrai nom au fichier haché.
