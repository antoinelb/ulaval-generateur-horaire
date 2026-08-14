# US-52 — Sigles inconnus ou mal formés

**Persona** : Daniel, qui vérifie la qualité des données après un scrape.
**Intention** : voir signalées les anomalies de sigles plutôt que de les découvrir dans une grille fausse.

## Préconditions

- Un programme chargé.

## Scénario

1. Daniel ouvre l'application et regarde le journal et la console.
2. Il examine un cours dont les préalables citent un sigle non conforme, par exemple `IFT 10426*`.

## Résultats attendus

- Un sigle non conforme dans les préalables d'un cours **du programme affiché** produit un avertissement du journal : « Sigle non standard dans les préalables de XXX-0000 : « … » (N car., attendu 8) ».
- La détection est restreinte aux sigles du programme : passer tout le catalogue noierait le journal sous les préalables de milliers de cours sans rapport.
- Un même sigle fautif n'est signalé qu'une fois par cours.
- Un sigle cité dans une règle mais absent du catalogue produit un avertissement de console `Sigle introuvable dans le catalogue de cours : …`.
- Un sigle mal formé est neutralisé lors de l'évaluation des préalables : il ne valide jamais un cours par accident.

## Repères pour le test e2e

- `#log-content .log-warning` contient l'avertissement de sigle non standard, avec l'horodatage `[hh:mm:ss]`.
- Les messages de console sont observables via `page.on('console')`.

## Variantes et cas limites

- Le format attendu est `[A-Z]{2,4}-\d{4}` : un sigle contenant une espace au lieu d'un tiret est détecté, un sigle à cinq lettres aussi.
- Un jeton contenant une espace interne n'est pas signalé, pour éviter les faux positifs sur la prose.
- Le journal est effacé à chaque vérification : les avertissements émis au chargement du programme disparaissent dès la première modification de la grille. C'est un défaut connu qui rend ces messages faciles à manquer.
