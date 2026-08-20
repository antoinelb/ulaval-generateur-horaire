# Plan archivé, les ADR deviennent la source de vérité

Date : 2026-08-19

## Contexte

Le projet atteint sa première version livrée : scraper, solveurs A et B, UI des jalons 3–9 ; seules les préférences (jalon 10) restent.
Jusqu'ici `docs/conception/project_plan.md` était la source de vérité, tenue à jour à chaque décision en plus de l'ADR — un double emploi qui coûtait à chaque changement et divergeait dès qu'une mise à jour était oubliée.

## Décision

- `docs/conception/` au complet, plan inclus, devient un **historique** : on le consulte pour le contexte, on ne le met plus à jour.
- Les **ADR** sous `docs/conception/adr/` sont la seule source de vérité ; en cas de contradiction avec un document de conception, l'ADR a préséance.
- `CLAUDE.md` est simplifié en conséquence : il décrit l'état livré et pointe vers les ADR, sans plus exiger la mise à jour du plan.

## Alternatives rejetées

- **Continuer à maintenir le plan** : le *quoi* vit maintenant dans le code livré et ses tests ; maintenir une seconde description en prose n'ajoute que du risque de divergence.
- **Supprimer le plan** : il documente le mandat et les raisonnements d'origine ; l'historique a de la valeur, seule son autorité est retirée.
