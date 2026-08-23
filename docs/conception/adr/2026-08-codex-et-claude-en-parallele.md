# Codex et Claude Code restent configurés en parallèle

## Contexte

Le dépôt était configuré uniquement pour Claude Code au moyen de `CLAUDE.md` et de `.claude/`.
Codex doit devenir utilisable sans interrompre les parcours Claude Code existants pendant la transition.
Dupliquer les longues consignes de projet, de Dioxus et des rôles UX créerait deux sources susceptibles de diverger.

## Décision

`AGENTS.md` devient le point d'entrée natif de Codex et charge les consignes partagées de `CLAUDE.md`.
La compétence Codex `dioxus-0-7` charge la référence partagée `.claude/dioxus.md` avant tout travail Dioxus.
`.codex/config.toml` expose les rôles `etudiante_cegep` et `etudiante_gex`, dont les fichiers de rôle chargent les spécifications existantes sous `.claude/agents/`.
Les rôles Codex héritent du modèle de la session appelante, car `sonnet` n'a pas d'équivalent Codex univoque.
Les fichiers Claude Code restent inchangés et fonctionnels pendant la période parallèle.

## Solutions rejetées

Copier toutes les consignes sous des chemins Codex a été rejeté parce que chaque correction devrait alors être synchronisée manuellement.
Configurer Codex pour traiter `CLAUDE.md` comme simple nom de repli global a été rejeté parce que cela dépendrait de la configuration personnelle de chaque développeur.
Supprimer immédiatement `.claude/` a été rejeté parce que la transition parallèle est explicitement requise.
