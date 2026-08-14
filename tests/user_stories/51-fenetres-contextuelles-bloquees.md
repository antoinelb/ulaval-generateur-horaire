# US-51 — Fenêtres contextuelles bloquées

**Persona** : Mélissa, dont le navigateur bloque les fenêtres contextuelles par défaut.
**Intention** : comprendre pourquoi rien ne s'ouvre.

## Préconditions

- Le blocage des fenêtres contextuelles est actif.

## Scénario

1. Mélissa clique « Grille horaire de session ».
2. Elle clique « Imprimer le cheminement ».

## Résultats attendus

- Chaque cas affiche une alerte explicite demandant d'autoriser les fenêtres contextuelles pour ce site.
- L'application reste utilisable : aucune exception, aucun état bloqué.
- Une fois les fenêtres autorisées, un nouveau clic fonctionne normalement.

## Repères pour le test e2e

- Intercepter `page.on('dialog')` et vérifier le texte de l'alerte.
- Aucune entrée dans `page.on('pageerror')`.

## Variantes et cas limites

- La fenêtre d'horaire est construite par `document.write` depuis la fenêtre principale : elle n'a pas de fichier propre et ne survit pas à un rechargement de la page mère.
- Fermer la fenêtre principale pendant que la fenêtre d'horaire est ouverte casse l'appel `window.opener`; l'échec est avalé et la fenêtre reste figée sur son dernier rendu.
- Deux fenêtres d'horaire ne peuvent pas coexister : le second clic ramène le focus sur la première.
