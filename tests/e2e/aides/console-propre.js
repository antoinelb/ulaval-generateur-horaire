// La console propre est une **assertion**, pas une inspection manuelle.
//
// Pourquoi ici plus qu'ailleurs : l'interface est du Rust compilé en WASM.
// Une panique dans un composant ne vide pas l'écran — elle remonte au
// bord wasm-bindgen comme exception non capturée, et le DOM du dernier
// rendu reste affiché tel quel. La page a donc exactement l'air vivante
// qu'elle avait une milliseconde plus tôt, pendant que le module est mort.
// Sans ce garde-fou, chaque `expect` qui suit interroge un cadavre et
// passe : la suite entière deviendrait verte au moment précis où elle
// devrait crier. Les trois personas du 2026-08-30 ont fini chacun leur
// rapport par « Erreur console : aucune » — ils l'ont vérifié à la main,
// une fois. Ceci le vérifie à chaque test, pour toujours.
//
// Chaque spec importe `test` / `expect` d'ici plutôt que de
// '@playwright/test' : la fixture `consolePropre` est automatique, donc
// on ne peut pas oublier de la brancher.

import { test as base, expect } from '@playwright/test';

// Un test qui provoque volontairement une erreur (import d'un programme
// dont l'URL échoue, donnée corrompue) déclare sa tolérance par
// `test.use({ toleranceConsole: [/motif/] })`, scopée à CE test seul —
// jamais un motif global.
//
// La raison de refuser le global : les motifs qu'on serait tenté d'écrire
// (`/404/`, `/Failed to fetch/`) couvrent aussi bien l'échec voulu que le
// même échec produit par un asset manquant du bundle. Or c'est
// exactement le piège du service statique de ce dépôt : `asset!()` émet
// des URL absolues sous `/ulaval-generateur-horaire/`, et servir le
// bundle ailleurs donne une page nue et des 404 partout, sans autre
// symptôme. Une tolérance globale au 404 rendrait ce mode de défaillance
// invisible ; scopée à un test, les autres specs continuent de le
// dénoncer.
const test = base.extend({
    toleranceConsole: [[], { option: true }],

    consolePropre: [async ({ page, context, toleranceConsole }, use) => {
        const erreurs = [];

        function estToleree(message) {
            return toleranceConsole.some((motif) => motif.test(message));
        }

        function brancher(cible) {
            // exception non capturée, promesse rejetée non gérée, panique
            // Rust remontée au bord wasm-bindgen
            cible.on('pageerror', (erreur) => {
                erreurs.push(`erreur de page (${cible.url()}) : ${erreur.message}`);
            });
            cible.on('console', (message) => {
                if (message.type() !== 'error') return;
                const composite = `${message.text()} — ${message.location().url}`;
                if (estToleree(composite)) return;
                erreurs.push(`console.error (${cible.url()}) : ${composite}`);
            });
        }

        brancher(page);
        // Rien dans l'interface n'appelle `window.open` aujourd'hui
        // (l'export imprime la page courante), mais une page ouverte plus
        // tard échapperait au garde-fou sans ce branchement — le coût est
        // d'une ligne, l'angle mort serait silencieux.
        context.on('page', brancher);

        await use();

        if (erreurs.length > 0) {
            throw new Error(
                `${erreurs.length} erreur(s) console/page pendant le test :\n${erreurs.join('\n')}`,
            );
        }
    }, { auto: true }],
});

export { test, expect };
