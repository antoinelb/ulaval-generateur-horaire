# Un cours sans section de session est offert automne et hiver, horaire inconnu

Date : 2026-07-31

## Contexte

Une page de cours nouvellement créé n'a encore aucune section de session — aucun `div.collapsible-sections`, donc pas d'en-tête « Automne 2026 », pas de NRC (GCI-1011, « SIG, territoire et infrastructures »).
Avant le snapshot unique, un tel cours n'appartenait à aucune session nommable et disparaissait des données ; le planificateur ne pouvait pas le placer dans un organigramme alors qu'il sera offert.

## Décision

- Le parseur, quand la page n'a **aucune** section de session, synthétise `seasons = {fall, winter}` avec `last_offered: null` et `options: null` — jamais l'été.
- La garde est **l'absence de la section**, pas un résultat de parse vide : un bloc de session présent mais illisible (« Printemps 2026 ») laisse une anomalie et la carte des saisons vide — on n'invente rien à côté d'une anomalie.
- `options: None` signifie « offert, horaire pas encore publié », distinct de `Some([])`, « aucune combinaison d'inscription valide ».
- Solveur A (`schedule_report`) : erreur typée `ScheduleError::ScheduleUnknown` — une grille hebdomadaire ne peut rien dessiner, le refus est bruyant comme `NotOffered`/`NoOptions`.
- Solveur B (placement/faisabilité) : `build_domain` rend un `Opt` placeholder (aucun NRC, masque vide) — le cours se place, n'entre en conflit avec rien.
- Réconciliation avec `2026-07-cours-sans-offre-ecarte-par-le-harnais` : le chemin « Sans données d'offre (écartés) » ne couvre plus que les codes absents du snapshot (vrais trous de scrape) ; un cours nouveau *a* un `Course` et se place.

## Alternatives rejetées

- **Continuer d'écarter ces cours** : perd exactement les cours qu'un étudiant qui planifie son bac doit pouvoir placer — GCI-1011 est au cheminement GEX.
- **Les marquer offerts aux trois saisons** : l'été est l'exception à ULaval ; automne+hiver est l'hypothèse conservatrice qu'un horaire publié corrigera au prochain scrape.
- **Marqueur doux dans le rapport hebdomadaire plutôt qu'une erreur** : le contrat UI (`2026-07-contrat-horaire-hebdomadaire-vers-ui`) n'a pas prévu « offert, horaire inconnu » ; l'erreur dure est le patron existant pour v0, l'UI pourra demander un marqueur doux plus tard.

## Effets de bord assumés

- **MED-1911** (« Stage-Externat », page sans section de session) reçoit la même synthèse automne+hiver — plausible pour un stage, corrigé par les données le jour où la page publie des sessions.
- Un cours nouveau offert seulement l'été serait marqué automne+hiver jusqu'à la publication de son premier horaire.
