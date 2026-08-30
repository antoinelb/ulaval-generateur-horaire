# Fonctionnalités et découvrabilité

Inventaire de toutes les fonctionnalités visibles par l'étudiant, leur mode de découverte actuel, puis le catalogue des patrons de divulgation progressive admissibles sous AIR pour aider l'étudiant à les trouver.
État au 2026-08-27 (branche `feature/hints`).
Les références `fichier:ligne` pointent dans `crates/ui/src` sauf mention contraire.

## Cote de découvrabilité

- **évident** : contrôle visible et étiqueté ; se trouve en regardant l'écran.
- **semi-caché** : demande un geste préalable (déplier, survoler, voir un toast avant qu'il parte) ou de savoir où regarder.
- **caché** : aucune affordance visible — raccourci clavier, cible de collage, glisser sans indice, mécanisme automatique, URL.

---

## Partie 1 — Inventaire des fonctionnalités

### 1. Démarrage, chargement, données

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Écran de chargement phasé | « téléchargement du catalogue / analyse du catalogue » + secondes écoulées | automatique | `components/shell.rs:31` | évident |
| Écran d'échec de chargement | erreur en 5 parties + détail technique replié | automatique | `components/shell.rs:52`, `present.rs:18` | évident |
| Restauration automatique de l'état | plan, vue, cours manuels et programmes importés reviennent du `localStorage` | automatique | `components/mod.rs:598`, `persist.rs:8` | caché |
| Sauvegarde débounce + flush à la sortie | écrit 300 ms après la dernière modification et sur `pagehide` | automatique | `components/mod.rs:1261`, `browser.rs:104` | caché |
| Sauvegarde endommagée mise de côté | copie sous une clé fraîche, état propre, note en toast | automatique | `persist.rs` (`Restored.backup`), `browser.rs:97` | semi-caché |
| Mode hors-ligne (service worker) | cache-first + revalidation ; l'app reste lisible sans réseau | automatique (site déployé) | `assets/sw.js`, `browser.rs:347` | caché |
| Avertissements de tolérance | chaque défaut toléré du catalogue devient un toast | automatique | `components/mod.rs:1299` | évident |

### 2. Choix et gestion des programmes

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Sélecteur de programme | une carte par code tant qu'aucun programme n'est choisi | automatique | `components/panel.rs:57` | évident |
| Choix du millésime | `select` des versions, la plus récente en premier | menu + « Choisir » | `components/panel.rs:148` | évident |
| Concentration par défaut experte | « sans concentration » : le bloc neutre du millésime s'il existe, « Aucune » sinon | effet de « Choisir » | `components/panel.rs:191`, `panel.rs` (`default_concentration`) | caché |
| Horizon par défaut dérivé des crédits | sessions d'études = `ceil(crédits / 15)` | effet de « Choisir » | `state.rs:119` | caché |
| Étagère par (programme, millésime) | chaque document revient intact au retour | « changer » puis « Choisir » | `persist.rs:213`, `components/mod.rs:706` | semi-caché |
| Bouton « changer » | renvoie au sélecteur en conservant le cheminement | en-tête | `components/header.rs:129` | évident |
| Changement de concentration / profil | recompose sections et bilan, la grille ne bouge pas | `select` du panneau | `components/panel.rs:729` | évident |
| Purge annoncée au changement de bloc | ententes et électifs orphelins retirés, nommés, annulables | effet du changement | `components/panel.rs:876`, `panel.rs:1231` | évident (toast) |

### 3. Imports

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Tiroir « Votre programme n'est pas là ? » | accordéon replié, verrouillé ouvert pendant l'import | bouton-accordéon | `components/panel.rs:305` | évident |
| Import de programme par URL | fetch via `corsproxy.io`, parse, persiste localement | champ + « Importer » **ou** Entrée | `components/panel.rs:618`, `import.rs:47` | évident / caché (Entrée) |
| Phases nommées d'import | Téléchargement / Analyse / Enregistrement + secondes | automatique | `components/panel.rs:683` | évident |
| Annuler l'import | abandonne vraiment le fetch (AbortController) | bouton pendant l'import | `browser.rs:362` | évident |
| Import de programme par fichier JSON | charge `{code}-{semestre}.json` sans réseau | rangée fichier + « Charger » | `components/panel.rs:647` | évident |
| Badge et provenance d'un programme importé | « importé », date UTC, lien source, anomalies repliées | carte | `components/panel.rs:89` | évident / semi-caché |
| Supprimer un programme importé | sans confirmation, annulable depuis le toast | « Supprimer » | `components/mod.rs:775` | évident |
| Annuler la suppression | bouton « ↶ Annuler » *dans* le toast, seul chemin | clic sur le toast | `components/header.rs:398` | semi-caché |
| Tiroir « Charger depuis Capsule » | mode d'emploi `ctrl-u` → `ctrl-a` → `ctrl-c` + `textarea` | bouton-accordéon | `components/panel.rs:1341` | évident |
| Import du relevé de notes | épingle le réussi, crédite les acquis, ancre `start`, un seul acte annulable | « Charger » | `capsule.rs:50`, `capsule.rs:145` | évident |
| Bilan Capsule persistant | compte + listes, survit au repli, s'efface si l'import est annulé | automatique | `components/panel.rs:1486` | évident |

### 4. Organigramme (réglages, placement, verdicts)

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Réglages Début / Sessions / Plafond | bornés, une valeur bornée se corrige et s'explique par toast | contrôles du panneau | `components/panel.rs:1105` | évident |
| Cases étés / concomitance | ouvrent les étés, tolèrent un préalable simultané | cases à cocher | `components/panel.rs:1201` | évident |
| Case « Scolarité préparatoire déjà faite » | les 0xxx comptent comme acquis | dans la section de règle dépliée | `components/panel.rs:1755` | semi-caché |
| Proposition automatique | aucun bouton : le solveur replace dès que le plan se stabilise | automatique (débounce 500 ms) | `components/mod.rs:1104` | caché |
| Vérification automatique | dès que tout est placé | automatique | `components/mod.rs:1195` | caché |
| Statut du solveur + secondes | dans la bande d'état réservée | automatique | `components/header.rs:11` | évident |
| Annuler la recherche | tue le worker, en démarre un neuf | bouton | `components/mod.rs:484` | évident |
| Verdicts de placement | vérifié ✓ / contrainte brisée / placement en cours / rien placé | automatique | `components/panel.rs:1253` | évident |
| Conflits et dépassements nommés | « ⚠ Conflit d'horaire en H4, A5 » | automatique | `components/panel.rs:1070` | évident |
| Manques de crédits par règle | une ligne ⚠ par déficit + rappel sur la rangée du cours | automatique | `components/panel.rs:1946` | évident |
| Annonces du solveur en toasts | injectés, étés forcés, laissés de côté avec raison… | automatique | `components/mod.rs:296` | évident (toast) |
| Péremption automatique des toasts | un avertissement part quand sa cause part | automatique | `components/mod.rs:987` | caché |

### 5. Règles du programme et bilan (panneau gauche)

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Groupes et sections de règles | par portée, progression par groupe | affiché | `components/panel.rs:1003` | évident |
| Accordéon de règle + badge en fraction | « 6/9 cr », état porté par le texte | clic sur l'en-tête | `components/panel.rs:1521` | évident |
| Défilement vers la section dépliée | armé par le geste seulement | effet du clic | `components/panel.rs:1568` | caché |
| Browse d'une règle « tous les cours » | `select` de matière ; prendre crée l'entente dans le même acte | menu dans la section dépliée | `components/panel.rs:1786` | semi-caché |
| Bilan « N/M cr au bac » + crédits de session | en-tête, « (min.) », ⚠ plafond | affiché | `components/header.rs:105` | évident |
| Avertissement d'exactitude | « Horaires à titre indicatif… » permanent | affiché | `components/header.rs:153` | évident |

### 6. Recherche et prise de cours

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Recherche dans tout le catalogue | champ au pied du panneau, compte affiché | frappe | `components/panel.rs:934`, `panel.rs:2073` | évident |
| Marqueur « rentrerait dans la session affichée » | indice consultatif par sonde de masque | affiché (petit texte) | `panel.rs:2141` | semi-caché |
| Chips « automatique » / geler en session | prendre et laisser choisir, ou geler ; recliquer déplace | chips du bandeau | `components/panel.rs:2009` | évident |
| Retirer un cours (✕) | sans dialogue, annulable ; absent pour un obligatoire | bouton ✕ | `components/panel.rs:2123` | évident |
| Garde à la première prise | saison, doublon, préalables : refus expliqué en toast | effet du clic | `panel.rs` (`take_verdict`) | évident (toast) |
| Bascule « créditer » | compte sans occuper de session | bouton de rangée | `components/panel.rs:2190` | évident |
| Entente (rattacher à une règle) | fait compter un cours dans une règle qui ne l'admet pas | `select` « entente… » | `components/panel.rs:2232` | semi-caché |
| Correction des préalables par cours | champ éditable, écho de grammaire, commit sur Entrée ou blur | `details` « Préalables » | `components/panel.rs:1625` | semi-caché / caché (Entrée) |

### 7. Ruban des sessions

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Naviguer entre sessions | toute la carte est un bouton | clic | `components/ribbon.rs:98` | évident |
| Carte de session | libellé, crédits, ✓, ⚠, sigles, « à planifier » | affiché | `components/ribbon.rs:62` | évident |
| Bandes d'été compactes | bande étroite si vide, carte pleine si occupée | automatique | `components/ribbon.rs:31` | évident |
| Annotation libre de session | « à l'étranger » s'affiche mais **aucun contrôle ne l'édite** | lien partagé seulement | `components/ribbon.rs:162`, `state.rs` (`Plan.special`) | caché (lecture seule) |
| Glisser un sigle du ruban vers une session | déplace le cours | glisser-déposer, `cursor: grab` | `components/ribbon.rs:246` | caché |
| Cibles de dépôt marquées | cartes offrantes pleines, autres pâlies, survolée bordée | pendant le drag | `components/ribbon.rs:50` | évident (une fois le drag commencé) |

### 8. Horaire hebdomadaire

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Grille hebdo + titre + ligne d'état | axe fixe, colonnes lun→ven, conflits nommés | affiché | `components/grid.rs:14` | évident |
| Légende de la grille | plein = retenue, pointillé = possible, hachuré = conflit | affichée | `components/grid.rs:140` | évident |
| Jeton « ⇄ N » sur un bloc | annonce au repos le nombre d'horaires alternatifs ; le `title` porte le compte (et « aucun » à N = 0) | affiché | `present.rs` (`Block.alternatives`), `components/grid.rs` | évident |
| Sélectionner un cours → plages fantômes | cliquer un bloc montre les autres sections | clic sur un bloc | `components/grid.rs:250` | évident |
| Forcer une section | cliquer un fantôme épingle ses NRC (sémantique swap) | clic sur un fantôme | `components/grid.rs:250` | évident |
| Échap désélectionne | referme les fantômes, nommé dans la légende de la grille | touche Échap | `components/shell.rs:91` | évident |
| Libérer les sections forcées | retire tous les NRC épinglés de la session | bouton conditionnel | `components/grid.rs:121` | évident |
| Hors grille + notes de bas de grille | compteur ⚠ + une ligne par cours exclu avec raison | automatique | `components/grid.rs:315` | évident |
| Glisser un bloc vers une session du ruban | déplace le cours vers cette session | glisser-déposer, **aucun indice** (curseur `pointer`) | `components/grid.rs:287` | caché |
| Conflits en couloirs mi-largeur | blocs hachurés à mi-largeur + ⚠ sur le jour | automatique | `components/grid.rs:163` | évident |

### 9. Export, impression, partage

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Exporter l'organigramme (PDF) | feuille Letter paysage une page, règles, légende, provenance | bouton | `components/print/organigramme.rs:19` | évident |
| Exporter l'horaire (PDF) | A4 portrait, une grille par session | bouton | `components/print/horaire.rs:15` | évident |
| Ajustement automatique à une page | police réduite par pas de 5 % jusqu'à tenir | effet de l'export | `browser.rs:284` | caché |
| Provenance dans les exports | version, commits, dates, **lien de partage du plan** | automatique | `export/provenance.rs:27` | évident (dans le PDF) |
| Partager par lien | tout l'organigramme en fragment `#`, copié + dans l'adresse | bouton « Partager » | `components/header.rs:68`, `persist.rs:288` | évident |
| Ouvrir un lien partagé | remplace le vôtre en un pas annulable | ouvrir l'URL | `components/mod.rs:864` | caché |

### 10. Cours manuels (hors catalogue)

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Formulaire « Cours absent du catalogue ? » | code, titre, crédits, NRC, plages, entente | `details` replié au bas du panneau | `components/panel.rs:2364` | semi-caché |
| Créer le cours | valide, ajoute au catalogue local, prend en électif | « Créer le cours » | `components/panel.rs:2388` | évident |
| Proposer au catalogue (GitHub) | issue préremplie, aucun réseau | lien sous le cours | `components/panel.rs:2641` | évident |
| Copier la fiche du cours | JSON dans le presse-papier | bouton | `components/panel.rs:2670` | évident |

### 11. Annulation, réinitialisation, alertes

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Annuler / Rétablir | actes nommés dans l'infobulle | boutons de la bande d'état | `components/header.rs:252` | évident |
| Raccourcis Ctrl+Z / Ctrl+Y | doublent les boutons, imprimés en `<kbd>` sur ceux-ci | clavier | `components/shell.rs:91` | évident |
| Bouton « Réinitialiser » | document + étagère, sans confirmation, annulable | en-tête | `components/header.rs:193` | évident |
| Toasts flottants | bas-droite, ⚠ persistants, ✓ effacés à 5 s, « +N autres » | automatique | `components/header.rs:305` | évident |
| Rejeter un message | clic n'importe où sur le toast, ou son ✕ | clic | `components/header.rs:355` | semi-caché |
| Aucune boîte de confirmation | tout acte destructeur est immédiat et réversible | propriété transversale | `components/mod.rs:686` | caché (propriété) |

### 12. Pied de page

| Fonctionnalité | Ce que ça fait | Déclencheur | Code | Cote |
|---|---|---|---|---|
| Ligne de provenance | version, commit code, date + commit données, liens GitHub | affichée | `components/shell.rs:135` | évident |
| Canal de contact | courriel + lien issues | affiché | `components/shell.rs:167` | évident |

### Les fonctionnalités cachées, par gravité

1. Glisser un bloc de la grille vers une carte du ruban — aucun indice visuel, curseur `pointer`.
2. Le lien de partage `#…` — découvert seulement en recevant un lien.
3. Entrée pour valider l'URL d'import et le champ de préalables.
4. Le placement et la vérification automatiques — délibérément sans bouton, donc invisibles tant qu'aucun statut ne s'affiche.
5. Le glisser d'un sigle depuis le ruban — seul indice, `cursor: grab`.

`Ctrl+Z` / `Ctrl+Y` / `Échap` ne figurent plus dans cette liste : le patron P1 les imprime désormais sur les boutons Annuler/Rétablir et dans la légende de la grille (voir Partie 2).

Le livre mdBook documente le consommateur JavaScript, pas l'interface étudiante : tout ce qui est « caché » ci-dessus n'a aujourd'hui nulle part où être découvert.

---

## Partie 2 — Patrons de découverte admissibles sous AIR

### Ce qu'AIR interdit d'emblée

La littérature converge avec AIR : les visites guidées et *coach marks* interrompent le flux, sont cliquées sans lecture et oubliées aussitôt fermées.
AIR les interdit par règle, pas par goût :

- **LAY-4** — explications en place, sur demande, rejetables, jamais bloquantes ; pas de tour, pas d'onboarding modal, pas de « tip » qui occulte des données.
- **LAY-2** — la divulgation s'étend en place ; rien de visible ne se déplace.
- **LAY-3** — le soutien au novice est de l'*explication ajoutée*, jamais un comportement modifié.
- **LAY-5** — les accélérateurs experts sont additifs et chacun a un équivalent pointeur découvrable.
- **Cœur** — pas d'affordance au survol seulement ; rien ne bouge sans geste de l'utilisateur.

Sont donc exclus : visite guidée, coach marks séquencés, modale de bienvenue, hotspots pulsants superposés aux données, tooltips au survol comme *seul* vecteur.

### Catalogue des patrons admissibles

| # | Patron | Principe | Contraintes AIR |
|---|---|---|---|
| P1 | **Raccourci imprimé sur le contrôle** | le `<kbd>` s'affiche à demeure sur ou près du bouton qu'il double (« Annuler ↶ `Ctrl+Z` ») | LAY-5 satisfait par construction ; visible au repos, aucune occlusion |
| P2 | **Affordance de glissement à demeure** | poignée ⠿ + `cursor: grab` sur tout élément glissable ; les cibles s'allument au `dragstart` (déjà fait pour le ruban) | révélé par le geste, pas par le survol ; rien ne bouge avant le drag |
| P3 | **Astuce contextuelle à déclenchement comportemental** | après N usages du chemin long (p. ex. 3 déplacements par chips), une ligne d'astuce apparaît dans une *région réservée* ; rejet persisté dans `localStorage`, jamais re-montrée | LAY-4 (rejetable), LAY-2 (région réservée déjà dans la mise en page, pas d'occlusion ni de déplacement) |
| P4 | **Fiche de raccourcis sur demande** | bouton visible « ? » (+ touche `?`) qui déplie la liste des raccourcis et gestes ; fermé par Échap et par le même bouton | sur demande, additif, équivalent pointeur ; ne bloque pas |
| P5 | **État vide enseignant** | chaque région vide dit quoi faire *ici* (« Glissez un sigle du ruban… ») — déjà pratiqué par la grille vide | l'espace est vide, donc aucune occlusion possible ; enseignement au moment de l'intention |
| P6 | **Légende permanente** | une ligne de légende sous la région qu'elle décrit (déjà fait pour la grille ; extensible au ruban) | affichée à demeure, aucun état, aucun risque |
| P7 | **Explication au moment de l'effet** | quand un mécanisme invisible agit (auto-placement, péremption), le statut ou le toast *nomme le mécanisme* et non seulement l'effet | transforme un automatisme caché en enseignement passif ; déjà à moitié pratiqué |
| P8 | **Divulgation « en savoir plus » dans le contrôle** | `<details>` sobre sous un réglage pour son explication longue | LAY-2 : pousse vers le bas, ne réordonne rien |
| P9 | **Info-bulle riche au focus comme au survol** | `title:`/tooltip porté aussi par le focus clavier, en *complément* d'un libellé visible | jamais le seul vecteur (règle cœur) |

Rejetés après examen : « astuce du jour » à rotation automatique (le contenu bouge sans geste), badge « Nouveau » pulsant (attire l'œil hors tâche, AIR n'a pas de canal « marketing »), tour guidé (LAY-4).

### Application aux fonctionnalités cachées de la Partie 1

| Fonctionnalité cachée | Patron | Application concrète |
|---|---|---|
| Drag bloc → ruban | P2 + P5 | poignée ⠿ et `cursor: grab` sur les blocs pleins ; au `dragstart`, les cartes du ruban s'allument (déjà codé) ; la légende de la grille gagne « glisser = déplacer de session » |
| Raccourcis undo/redo | P1 | `Ctrl+Z` / `Ctrl+Y` en `<kbd>` discret sur les boutons de la bande d'état |
| Échap désélectionne | P1 + P6 | la légende de la grille gagne « Échap = désélectionner » quand un cours est sélectionné |
| Lien de partage | P7 | le toast de « Partager » explique déjà la copie ; ajouter « ce lien contient tout l'organigramme — collez-le n'importe où » |
| Entrée dans les champs | P1 | `↵` en `<kbd>` dans le coin du champ URL et du champ préalables |
| Auto-placement invisible | P7 | le verdict « placement automatique en cours… » gagne une ligne « le placement se relance seul à chaque modification » la première fois |
| Chips vs drag (chemin long) | P3 | après 3 déplacements consécutifs par chips, astuce dans la bande d'état : « Astuce — on peut aussi glisser un cours entre les sessions » |
| Ensemble des gestes | P4 | fiche « ? » listant raccourcis, gestes de glissement, Entrée, Échap |
| Annotation de session (`Plan.special`) | — | pas un problème de découverte : la fonctionnalité n'a pas de contrôle d'édition ; à décider (l'exposer ou la retirer) |

### Sources

- [Nielsen Norman Group — Designing Empty States in Complex Applications](https://www.nngroup.com/articles/empty-state-interface-design/)
- [UXPin — What Is Progressive Disclosure in UX?](https://www.uxpin.com/studio/blog/what-is-progressive-disclosure/)
- [Setproduct — Progressive onboarding with contextual help](https://www.setproduct.com/blog/how-to-replace-onboarding-with-contextual-help)
- [Michael Lisboa — 4 reasons why onboarding tours and coach marks don't work](https://medium.com/design-bootcamp/4-reasons-why-onboarding-tours-and-coach-marks-dont-work-b0693e8e83f8)
- [UX Planet — Design Patterns: Progressive Disclosure](https://uxplanet.org/design-patterns-progressive-disclosure-for-mobile-apps-f41001a293ba)
- [Design Bootcamp — The UX of Keyboard Shortcuts](https://medium.com/design-bootcamp/the-art-of-keyboard-shortcuts-designing-for-speed-and-efficiency-9afd717fc7ed)
- Règles AIR : `~/.claude/skills/air/references/interface-rules.md` (LAY-2/3/4/5, cœur).
