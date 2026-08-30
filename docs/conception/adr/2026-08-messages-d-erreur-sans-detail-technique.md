# Les messages d'erreur ne portent ni identifiant ni texte du code source

Date : 2026-08-30

**Statut :** accepté (décision d'Antoine). **Dérogation AIR GOV-2** à ERR-1 (« *a copyable diagnostic ID* ») et ERR-3 (« *technical detail is always one click away* »). **Amende** `2026-08-refus-du-solveur-en-francais` et `2026-08-verdicts-honnetes-et-panneau-jamais-vide`.

## Contexte

Chaque erreur affichée portait cinq parties : quoi / réaction / affecté / quoi faire, plus un identifiant `GH-XXXXXXXX` (FNV du message) et un repli « Détail technique » contenant le texte anglais de `core` — « *the selection sums 15 credits, above the max 12 — semantics await the director's ruling* ».

Cinq endroits l'affichaient : bandeau du panneau, toast global, écran d'échec de chargement, import de programme, tiroir Capsule.

Antoine, le 2026-08-30 : « Ne pas ajouter de code ou de message d'erreur relié au code source. Le message devrait expliquer ce qu'est l'erreur et ensuite un usager me rapporterait l'erreur simplement de là. »

## Décision

`UiError` passe de six champs à quatre : `what`, `reaction`, `affected`, `action`. `error_id` disparaît, ainsi que les cinq rendus du détail technique, les classes CSS qui les habillaient, et les clauses d'action qui disaient « signalez-la avec l'identifiant ci-dessous » — chacune remplacée par une suite à donner réelle.

Le texte anglais du solveur ne monte plus jusqu'à l'écran : le seul refus qu'un étudiant peut provoquer est reconnu à son préfixe et dit en français, tout le reste reçoit l'enrobage générique. Un test gèle cette frontière — aucun fragment anglais (« scope », « selection », « above the max », « pinned ») ne peut apparaître dans l'une des quatre parties.

Le parseur qui relisait l'anglais de `core` pour en refaire une valeur typée (`parse_over_max`) disparaît avec son sujet : depuis `2026-08-depassement-de-regle-en-statut-rouge`, un dépassement ne voyage plus comme une chaîne.

### La dérogation, dans la forme que GOV-2 exige

- **Règles écartées :** AIR ERR-1 (identifiant diagnostic copiable) et ERR-3 (détail technique à un clic).
- **Risque accepté :** un rapport de bogue moins précis — l'utilisateur décrit ce qu'il a lu au lieu de coller un identifiant qui désigne le message exact.
- **Contrôle compensatoire :** le public est étudiant, pas opérateur, et le canal de signalement est direct (le mandant est le directeur de programme, l'auteur est joignable). Les quatre parties françaises restent obligatoires et nomment toujours le sujet — règle, cours, concentration —, donc le message identifie l'erreur sans hash. Aucune erreur ne disparaît : ce qui partait dans le repli est soit dit en français, soit sans intérêt pour qui la rapporte.
- **Responsable :** Antoine.
- **Expiration :** 2027-08-30. À revoir si l'application sert un jour un public plus large que les étudiants du programme, où le volume de signalements rendrait l'identifiant utile.

## Alternatives rejetées

- **Garder l'identifiant, retirer seulement le texte anglais** : `GH-4F2A19C0` seul n'aide personne — ni l'étudiant, qui ne sait pas ce qu'il désigne, ni l'auteur, qui ne peut pas le remonter à une ligne sans le message.
- **Ne nettoyer que le bandeau du panneau** (le message montré) : deux styles d'erreur coexisteraient dans la même application.
