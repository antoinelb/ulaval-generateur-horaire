# Cours, options et NRC

## Vocabulaire

- **Sigle** (`GEX-1000`) : l'identifiant d'un cours ; le préfixe (`GEX-`) est sa **matière**.
- **NRC** : le numéro d'une section précise à une session précise — c'est ce qu'on saisit à l'inscription.
- **Section** : un groupe (cours magistral, laboratoire, à distance…) avec son mode et ses plages horaires.
- **Option** : une *inscription complète* — l'ensemble des sections qu'on prend ensemble.

## Une option se prend entière

C'est la maille centrale du modèle : `options` liste des combinaisons valides, pas des sections isolées.

```json
"options": [
  [{"nrc": "84664"}, {"nrc": "84665", "section": "A"}],
  [{"nrc": "84664"}, {"nrc": "84666", "section": "B"}]
]
```

Ici le cours magistral 84664 accompagne le laboratoire A *ou* le laboratoire B : deux options, le magistral figurant dans les deux.
L'horaire occupé d'une option est l'union des plages de ses sections.
Un NRC peut se répéter *entre* options (elles sont des alternatives) ; jamais *dans* une option.

Comme une option n'a pas d'identifiant propre, on la désigne par l'ensemble de ses NRC — c'est la forme du champ `chosen` de l'horaire hebdomadaire.

## Modes et plages

- `in-person` : toutes les plages comptent ;
- `remote` : aucune plage, jamais de conflit ;
- `hybrid` : seules les rencontres en présentiel portent jour et heures — l'option ne bloque que celles-là.

Les heures sont des chaînes `"HH:MM"` ; un créneau est `{day, start, end}`.

## Crédits et cycles

`credits` est un nombre — ou `{min, max}` pour un stage dont l'étudiant choisit la pondération ; la planification compte alors la borne basse, la seule qui n'invente jamais de crédits.
`cycle` vaut `0` (préuniversitaire, les cours d'appoint `XXX-0150`), `1` ou `2`.

## Équivalences

`equivalents` liste les sigles reconnus équivalents : réussir l'un satisfait les préalables exigeant l'autre, et l'offre d'un équivalent peut servir de repli.
