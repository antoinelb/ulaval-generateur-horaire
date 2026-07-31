# Les crédits en intervalle comptent pour leur borne basse en planification

**Date :** 2026-07-30
**Statut :** accepté (décision Antoine) ; ferme « pondération des crédits en intervalle : requise d'emblée, ou défaut à la borne basse » de `docs/next_steps.md` pour le solveur B et le vérificateur de règles.
`Credits::resolve` (A, total de crédits du CLI) garde son contrat strict (`2026-07-resolution-des-credits-choisis`) : là, aucun défaut n'est inventé.

## Contexte

Un stage `Credits::Range` n'a de valeur qu'une fois la pondération choisie par l'étudiant.
B (plafond par session, seuils `program_credits`, crédits réussis précomptés) et le vérificateur (somme d'une règle à crédits) ont besoin d'un nombre avant que l'UI ne porte ce choix.

## Décision

- En **planification** (solveur B et vérificateur de règles), un cours `Range { min, .. }` compte pour `min` quand aucune pondération n'est fournie — un seul point d'entrée `core` (`planning_credits`) porte la politique.
- Le défaut est assumé asymétrique : **permissif** pour le plafond par session (le stage à sa pondération réelle pourrait ne plus rentrer), **pessimiste** pour les seuils de crédits accumulés et les règles à crédits (la vraie somme ne peut être que supérieure).
  La borne basse est la seule valeur qui ne fabrique jamais de crédits que l'étudiant n'a pas.
- Quand l'UI portera la pondération choisie, elle passera par le mécanisme existant (`Credits::resolve(Some(chosen))`) ; le défaut ne s'applique qu'en son absence — l'ajout ne casse rien.

## Alternatives rejetées

- **Pondération requise, erreur sinon** : fidèle mais bloque le placement tant que la tuyauterie UI n'existe pas ; le défaut borne basse est remplaçable sans rupture.
- **Défaut borne haute** : fabrique des crédits non acquis — un seuil `program_credits` pourrait passer à tort, un plafond bloquer à tort.
