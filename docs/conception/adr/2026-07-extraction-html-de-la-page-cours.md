# Extraction HTML de la page cours

Date : 2026-07-19

## Contexte

La grammaire des préalables étant close (`2026-07-conception-du-parseur-de-cours`), il reste à cartographier les sélecteurs de la page cours.
Les six fixtures gelées révèlent plusieurs pièges que la structure du modèle (`2026-07-sections-en-groupes-de-choix`) ne couvre pas.

## Décision

1. **Champs simples** : `span.fe--titre-type` (code), `span.fe--titre-nom` (titre), `span.promo-entete--titre` (crédits), le bloc `p.promo-paragraphe` = « Cycle du cours » → `li > strong` (cycle), `div.fe--prealables p.etiquette-container` (texte brut des préalables, nœuds texte concaténés — les sigles sont des `<a>` enfants).
2. **Le mode se lit dans l'en-tête `button.header-wrapper`**, pas dans l'étiquette « Type: ».
   « Type: » emploie un vocabulaire hétérogène et par plage horaire (« En classe », « Sur Internet », « Laboratoire », « Rencontre »), alors que l'en-tête est cohérent sur les six fixtures : « En classe » → `in-person`, « À distance » → `remote`.
3. **L'identifiant de section se lit aussi dans l'en-tête**, mais sa position dépend du niveau : `[code, section, mode, …]` au premier niveau (chaîne vide → `None`), `[section, mode, …]` pour une section liée.
4. **Les blocs de session sont imbriqués** : les sections liées sont des `div.toggle-section` *descendants* de celui de la section principale, si bien qu'un sélecteur plat rend 9 éléments pour GCI-1007 au lieu de 3.
   Discriminants : un bloc de session relève d'un `.collapsible-sections` dont le `.sections-controls` porte un `p.controls-title` ; les sections liées vivent sous `div.toggle-section--content-wrapper.dark`.
5. **Une seule session par saison est retenue**, la plus récente, lue dans `p.controls-title > strong` (« Automne 2026 – ») et non dans l'attribut `id` — celui-ci est un *slug* sans accents (`ete-2026`) et mal orthographié (`ncr-` au lieu de `nrc-`).
6. **Équivalents** : seuls les `li.bloc-cours.carte-accessible` sont retenus ; un `li.bloc-cours` nu, sans lien vers une page de cours, désigne une équivalence échue (ECN-4901 garde ECN-6901 et écarte ECN-2901).
   Le discriminant est structurel, donc stable dans une fixture gelée, contrairement à la lecture des plages de validité (« Depuis … » vs « De … à … ») qui dépendrait de la session courante.
7. **Horaire** : le format n'est pas uniformément `{H}h{MM}` — GCI-2010 porte « De 9h à 11h50 ».
   Une heure sans minutes vaut `:00` ; le remplissage à deux chiffres est déjà assuré par `From<Time> for String`.
8. **Valeur d'énumération inconnue** (mode, jour, saison) : anomalie dans `CoursePage.anomalies` et section écartée.
   Le cours reste analysable, la surprise est remontée dans le journal d'erreurs, et le test d'intégration exigeant `anomalies.is_empty()` fait échouer la CI dès qu'un vocabulaire nouveau apparaît.

## Alternatives rejetées

- **Lire le mode dans « Type: »** : ULaval y écrit « Sur Internet » là où l'en-tête écrit « À distance », et « Laboratoire » sur des sections dont le mode attendu est `in-person`.
- **Dériver le genre de composante de « Type: »** : les laboratoires 2024 et 2025 de GCI-1007 y sont annoncés « En classe » — la source est incohérente d'une année à l'autre (voir `2026-07-sections-en-groupes-de-choix`).
- **Exploiter l'attribut `id`** : il concentre saison, année, code, section et mode en un jeton, mais dépend d'un *slugifier* tiers qui écrase les accents.
- **Filtrer les équivalents sur la plage de validité** : sémantiquement plus juste, mais implicitement relatif à la date d'exécution et exigeant l'analyse de « automne 2025 » en français.
- **Erreur dure sur valeur inconnue** : une chaîne inattendue coûterait le cours entier, et un changement de vocabulaire côté site ferait échouer toute la moisson.
