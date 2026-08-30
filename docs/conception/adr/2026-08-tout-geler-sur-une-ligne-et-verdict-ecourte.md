# « Tout geler » tient sur une ligne, sans son ❄, et le verdict s'arrête à « ci-dessous »

Date : 2026-08-30

> Suite : les 2 rem retenus ici passent à 0.5 rem par l'ADR `2026-08-ecart-reduit-entre-tout-geler-et-reinitialiser`, qui emballe les deux boutons plutôt que d'écarter le second, et constate que l'écart ne porte plus ACT-5.

## Contexte

Trois constats d'Antoine, sans rapport l'un avec l'autre.

**Le bouton se repliait.** Mesuré au navigateur à 1280 px de large, « Tout geler » occupait 52 × 50 px — deux lignes — quand « Réinitialiser », son voisin, tenait en 92 × 32 px.
Ce n'était pas un manque de place dans la barre mais une compression : les deux boutons sont des éléments flexibles de `.header-bar` et gardaient le `flex-shrink: 1` par défaut, si bien que la barre serrée leur reprenait de la largeur et que le libellé passait à la ligne.
L'écart entre les deux valait 48 px : le `gap: 1rem` de la barre plus le `margin-left: 2rem` de `.header-reset`.

**Le glyphe ❄ préfixait un des deux libellés.** La bascule disait « Tout geler » dans un sens et « ❄ Tout dégeler » dans l'autre.

**Le verdict en disait un mot de trop.** Le bandeau d'avertissement du panneau se lisait « ⚠ mais N sections de règles restent à combler ci-dessous — le bac n'est pas complet. »

## Décision

- **Les boutons de la barre du haut ne se compriment plus** : `.header-bar .status-undo { flex: none; }`.
  On traite la compression, pas le repli qu'elle provoque : un `white-space: nowrap` aurait laissé le bouton rétréci et son libellé déborder de sa bordure, ce qui remplace un défaut visible par un défaut pire.
  La place se prend sur `.header-notice`, prose qui se recompose sans rien perdre — `.header-subtitle` porte déjà `min-width: max-content` et ne cède pas.
- **L'écart de `.header-reset` passe de 2 rem à 1 rem**, soit 2 rem au total avec le `gap` de la barre au lieu de 3 rem.
- **Le ❄ quitte le libellé** : `present::freeze_all` renvoie « Tout dégeler » tout court.
  Le mot est le porteur de l'état — c'est déjà ce que dit l'ADR `2026-08-bouton-tout-geler-dans-la-barre-du-haut` (« Le libellé porte l'état, jamais la couleur seule ») — et le glyphe ne faisait que le doubler ; **INP-3 reste satisfait sans lui**.
  Les ❄ que d'autres ADR justifient par INP-3 (`2026-08-gel-en-case-a-cocher-dans-la-carte`, `2026-08-sessions-gelees-generalisent-les-completees`) sont ceux du **ruban**, où le glyphe accompagne une case à cocher qui, seule, ne dirait pas de quoi il s'agit : ils restent en place, rien ici ne les touche.
  C'est « Tout dégeler », le plus long des deux libellés, qui décide si le bouton tient sur une ligne ; les deux caractères gagnés n'y suffiraient pas, et ne sont pas ce qui règle le repli — `flex: none` le fait, quel que soit le libellé.
- **La phrase du verdict s'arrête après « ci-dessous »**, dans `crates/ui/src/components/panel.rs`.
  La demi-phrase retirée ne disait rien que le début ne disait déjà, et énonçait une conclusion (« le bac n'est pas complet ») que le compte de crédits de la barre du haut porte de son côté, chiffré.

## L'écart retenu face à ACT-5

ACT-5 interdit qu'un contrôle destructeur jouxte un contrôle *fréquent*.
Les 3 rem d'origine (ADR `2026-08-barre-du-haut-degarnie`) répondaient à un voisinage précis : « Partager », geste courant, collé à « Réinitialiser », qui vide le document — un clic de travers coûtait tout l'organigramme.

Ce voisinage n'existe plus. « Partager » est parti dans la bande de statut, et le voisin de « Réinitialiser » est « Tout geler » : un geste rare, entièrement annulable, et annulable par lui-même puisque le second clic défait le premier.
Le coût d'un clic de travers dans ce sens est nul ; dans l'autre sens, « Réinitialiser » garde son propre avis avec « Annuler » dedans (ADR `2026-08-reinitialiser-annulable-depuis-son-avis`).

L'écart ne tombe pas à zéro pour autant : 2 rem restent le double du rythme de la barre, donc une séparation qui se voit, doublée par la teinte d'accent que `.header-reset` est seul à porter (INP-3 — la couleur ne porte jamais seule la différence).
« Réinitialiser » reste à découvert, jamais dans un menu (LAY-7).

## Alternatives rejetées

- **`white-space: nowrap` sur le bouton** : masque l'effet, laisse la cause. Le bouton reste comprimé et son texte sort de sa bordure.
- **Élargir le bouton (`min-width` en dur)** : une largeur figée sur un libellé qui change (« Tout geler » / « Tout dégeler ») se périme au premier changement de mot.
- **`flex: none` sur `.status-undo` tout court** : la règle sert aussi la bande de statut et le ruban ; la restreindre à `.header-bar` laisse ces voisinages tels quels.
- **Ramener l'écart à zéro**, comme le laissait entendre « réduire l'espace blanc » : ACT-5 demande une séparation *visible*, et un `gap: 1rem` identique à celui de tous les autres voisinages ne dirait plus rien.
- **Garder le ❄ et raccourcir ailleurs** : le glyphe redit ce que le verbe dit déjà, et il n'est le seul porteur d'aucun état — contrairement à celui de la case du ruban, qu'on garde.
- **Garder la fin de phrase** : la conclusion est déjà à l'écran, chiffrée, dans le compte du bac.
