# Format des cheminements types dans `{code}.manuel.json`

Date : 2026-08-15

> **Révisée par** `2026-08-fichier-manuel-de-programme-millesime` : le fichier est désormais `{code}-{semester}.manuel.json`, un par millésime ; le champ `admission` de chaque `CheminementType` disparaît (le nom du fichier le porte) et le `label` perd son préfixe de millésime.

## Contexte

Les cheminements types n'ont aucune source machine-lisible ; le dépôt JS les servait en CSV locaux (`{prog}/cheminements-types/`), avec un parseur maison, et trois programmes n'en avaient pas du tout (`CORRECTIFS-AMONT.md` item 9).
Le plan prévoyait déjà `data/programmes/{code}.manuel.json` (sans millésime, jamais écrit par le scraper — ADR `2026-07-cheminement-type-en-fichier-manuel`) sans en fixer le format.

## Décisions

- Format JSON, autorité dans core (`core::program_manual::ProgramManual`, round-trip testé) :

  ```json
  { "cheminements_types": [
      { "admission": "H27",
        "label": "H27 - Technique de génie mécanique - Scolarité préparatoire complétée",
        "completed": ["GMC-1024"],
        "sessions": [ { "semester": "A26", "courses": [] },
                      { "semester": "H27", "courses": ["GMC-1001", "MAT-1900"] } ] } ] }
  ```

- Un cheminement par millésime d'admission **et par variante** (profil, DEC technique, rythme) ; `label` est le libellé montré à l'étudiant, tel quel.
- `completed` reprend la colonne « cours complétés » (reconnaissances d'un DEC technique).
- Les rangées sont converties **fidèlement** : une session vide reste dans la liste (un été sans cours, une rangée d'alignement avant l'admission) — on ne retranche jamais d'information à la conversion. Seule exigence vérifiée : l'admission figure parmi les sessions.
- Conversion initiale : `B-GEX` (1 cheminement), `B-GIN` (8), `B-GMC` (17). **`B-GPH` est exclu** : son dossier côté JS contient littéralement les CSV du B-GMC (gabarit copié, jamais rempli) — convertir aurait immortalisé de fausses données ; sa grille reste à encoder quand une source existera.

## Alternatives rejetées

- Garder les CSV (les commettre ici) : parseur à réécrire dans chaque consommateur, BOM/guillemets/points-virgules fragiles, aucune validation de type.
- Sessions implicites (tableau de tableaux, calendrier déduit de l'admission) : les grilles réelles contiennent des rangées d'alignement avant l'admission (H27 commence par un A26 vide) — l'implicite les perdrait.
- Convertir B-GPH quand même : données fausses pires que données absentes.
