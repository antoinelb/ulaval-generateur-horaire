# CLI : adoption de clap

Date : 2026-07-18

## Contexte

L'ADR `2026-07-cli-dans-la-lib-et-style-derreurs` avait rejeté clap (« dépendance lourde pour une sous-commande à deux arguments positionnels ») et imitait à la main le style d'erreurs de clap/uv — uv étant lui-même construit sur clap.
Cette imitation (format_usage, blocs d'erreurs peints, grammaire par slice patterns) est jugée une complication inutile, et le CLI est sur le point de grandir (pages cours, reprise, sessions) : autant adopter clap maintenant plutôt que refaire le travail.

## Décision

- **clap 4 (derive)** ; les structs `Cli`/`Command` vivent dans `cli.rs` (lib, mesurée) et `Cli::try_parse_from` tourne dans `run(args)` — `main.rs` reste le shim existant (stderr, code 2).
- Aide et version (code de sortie clap 0) sont des succès : `error.print()` puis `Ok(())` ; toute autre erreur clap traverse la frontière `anyhow` avec son rendu ANSI intact.
- **Arguments en drapeaux** : `catalogue [--output-dir <DIR>] [--url <URL>]`, défauts `data` et l'URL de production — les overrides n'existent que pour les tests ; l'invocation cron reste `catalogue` nu.
- **Conventions clap/uv adoptées telles quelles** : invocation nue = aide complète + code 2 (`arg_required_else_help`) ; l'ancien cas particulier « aide + code 0 » disparaît.
- Supprimés : `format_usage`, les chaînes d'usage, les `bail!` d'erreurs peints, et les couleurs `GREEN`/`YELLOW`/`RED` de `print.rs` devenues orphelines.
- **Palette d'origine conservée** via `Command::styles` (clap 4 est monochrome par défaut) : couleurs pleines sans gras ni souligné — `Usage:`/en-têtes verts, littéraux et placeholders bleus, `error:` rouge, valeur fautive jaune.
- Vérifié : la couverture reste à 100 % (l'expansion du derive ne crée aucune région fantôme) — la contingence envisagée (module `args.rs` exclu du makefile) n'a pas été nécessaire.

## Alternatives rejetées

- **Continuer l'imitation artisanale** : c'était copier la sortie de clap sans la dépendance ; le travail serait refait à chaque option ajoutée.
- **API builder de clap** : envisagée comme parade aux régions fantômes du derive ; sans objet, le derive couvre à 100 %.
- **Arguments positionnels conservés** : optionnels et indépendants, ils composent mal par position (impossible de passer l'URL sans le répertoire).
