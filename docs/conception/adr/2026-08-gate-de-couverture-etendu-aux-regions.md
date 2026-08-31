# Gate de couverture étendu aux régions

Date : 2026-08-30

## Contexte

`make test` ne verrouillait que les lignes (`--fail-under-lines 100`).
Le rapport affichait donc 100,00 % de lignes et 99,99 % de régions sans que rien n'échoue : **une** région manquée sur 17 589, dans `core/src/language.rs`.

La région est le `?` de `course_number` :

```rust
fn course_number(code: &str) -> Option<u32> {
    code.split_once('-')?.1.parse().ok()
}
```

L'opérateur `?` engendre une région propre pour la branche `None`, région qui ne possède aucune ligne à elle — d'où le 100 % en lignes malgré le trou.
Les deux appelants ne peuvent pas l'atteindre : `subject_courses` ne laisse passer que les sigles dont `split_once('-')` est `Some`, et `english_floor` filtre sur `starts_with(ENGLISH_SUBJECT)` des sigles extraits par `course_codes`.
C'est une garde défensive sur une fonction privée, exactement ce que le principe « valider les entrées même venant d'appelants de confiance » demande d'écrire — et exactement ce qu'un gate sur les lignes ne voit jamais.

## Décision

1. **Le gate porte aussi sur les régions** : `--fail-under-regions 100` s'ajoute à `--fail-under-lines 100`.
   Les lignes cachent les branches sans ligne propre (`?`, bras de `match` repliés, gardes booléennes) ; c'est précisément là que se logent les chemins d'erreur, donc la partie du code qui compte.
2. **Une garde inatteignable depuis la production se couvre par un test direct sur la fonction privée**, pas en tordant un scénario pour la joindre par l'API publique.
   Ici : `assert_eq!(course_number("LANGUES"), None)` dans le `mod tests` en ligne, avec le commentaire qui dit pourquoi aucun appelant ne l'atteint.
3. **Le seuil ne justifie jamais de supprimer la garde.**
   Un `?` inatteignable aujourd'hui est ce qui empêche un panic le jour où l'instantané cesse de ne contenir que des sigles ; le gate demande un test, pas une amputation.

Le total passe à 17 589 régions, 1 631 fonctions et 13 210 lignes, toutes à 100,00 %.

## Alternatives rejetées

- **Garder le gate sur les seules lignes** : le 100 % du jour tient, mais rien ne le retient — le prochain `?` sur un chemin non testé le fait redériver en silence, et l'écart entre les deux colonnes redevient invisible.
- **Retirer la garde `?`** pour supprimer la région : `split_once` rend une `Option`, il faudrait de toute façon la traiter ; la seule façon de « supprimer » la branche serait un `expect`, que le projet proscrit en production.
- **Exclure `language.rs` de la mesure** : 100 % par non-mesure, déjà rejeté dans `2026-07-couverture-100-et-frontiere-io`.
- **Verrouiller aussi les fonctions** (`--fail-under-functions 100`) : la colonne est déjà à 100 %, mais elle est redondante avec les régions — une fonction non exécutée n'a aucune de ses régions couverte. Un drapeau de plus pour aucun signal de plus.
