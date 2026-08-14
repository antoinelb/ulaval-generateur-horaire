# US-88 — B-GPH, « Profil distinction »

**Persona** : Solène, en génie physique, admissible au profil distinction après 60 crédits.
**Intention** : voir les 12 crédits du profil — et constater qu'ils sont impossibles à combler dans l'interface.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que le profil ajoute

- Aucun cours obligatoire.
- Règle 1 : **12 crédits**, avec `courses: "negotiated"` — les cours sont convenus entre la direction de programme et l'étudiante, le répertoire n'en publie aucune liste.
- Texte brut de la règle : « Le profil est satisfait par la réussite des cours convenus entre la direction de programme et l'étudiante ou l'étudiant. »
- Une note en prose sur les conditions d'admissibilité (60 crédits acquis, moyenne exigée, entente avec la direction).
- Aucun `credits_required`.

## Scénario

1. Solène choisit « Profil distinction ».
2. Elle lit le panneau et le bilan.

## Résultats attendus — comportement actuel

- Le panneau affiche une carte « Règle 1 » avec `Aucun cours défini pour cette règle.`
- Le bilan affiche `Règle 1 : 0 cr. / 12 cr.` en avertissement, **quoi que Solène place** : aucun cours n'est rattaché à la règle, donc rien ne peut la combler.
- Le total exigé de la section vaut 12 crédits, ce qui gonfle le total global du programme sans qu'aucun cours puisse y répondre.

## Résultats attendus — ce qu'il faudrait

- Une règle négociée doit être affichée comme telle : le texte brut et la note en prose visibles, et un moyen de la déclarer satisfaite — sur le modèle de la case « Scolarité préparatoire complétée » (US-38).
- Ou bien elle est exclue du total, comme la scolarité préparatoire et les stages.

## Repères pour le test e2e

- `#log-content` contient `Règle 1 : 0 cr. / 12 cr.` avec la classe `log-warning`.
- Cette ligne ne change jamais, quel que soit le nombre de `.dropped-tile`.
- La ligne `Total :` du programme augmente de 12 crédits exigés par rapport aux autres spécialisations.

## Variantes et cas limites

- Le « Passage intégré au deuxième cycle » du B-GMC (US-80) est le même cas **sans** contrainte : il tombe à 0 crédit et reste inoffensif. La différence entre les deux tient à un seul champ.
- C'est l'écart le plus visible pour un utilisateur : une règle rouge permanente qu'aucune action ne peut éteindre.
