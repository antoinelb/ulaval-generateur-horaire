# US-27 — Imprimer le cheminement

**Persona** : Roxane, qui apporte sa grille à sa rencontre avec la direction de programme.
**Intention** : obtenir une page imprimable lisible, avec le bilan.

## Préconditions

- Une grille remplie et un journal contenant le bilan des crédits.

## Scénario

1. Roxane clique « Sauvegarder un cheminement » puis « Imprimer le cheminement ».
2. Une fenêtre s'ouvre et la boîte d'impression du navigateur apparaît.

## Résultats attendus

- La fenêtre contient le tableau complet et le contenu du journal sous le titre « Bilan et vérifications ».
- La mise en page est en lettre paysage, les couleurs des pastilles sont conservées à l'impression.
- Les menus de section des pastilles sont masqués sur l'imprimé.
- La boîte d'impression se déclenche automatiquement au chargement de la fenêtre.

## Repères pour le test e2e

- Écouter `page.on('popup')` puis vérifier la présence de `table` et de `.log-section` dans la fenêtre.
- `select.section-select` y est en `display: none`.
- Le titre du document imprimé est « Grille de cheminement ».

## Variantes et cas limites

- Si le navigateur bloque les fenêtres contextuelles, une alerte explique qu'il faut les autoriser (US-51).
- Si aucun tableau n'existe, une alerte le signale plutôt que d'ouvrir une fenêtre vide.
- Le journal imprimé est celui de l'instant : imprimer avant que la vérification ait tourné donnerait un bilan absent.
