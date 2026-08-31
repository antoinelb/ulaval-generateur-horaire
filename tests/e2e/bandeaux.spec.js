// Les bandeaux empilés : ce qu'ils cachent.
//
// Le directeur GCI (rapport du 2026-08-30) a vu la pile d'avis recouvrir
// à la fois le menu « Exporter ▾ » et les colonnes Jeudi/Vendredi de
// l'horaire hebdomadaire. Reproduit ici, à l'identique, aux deux tailles
// de fenêtre.
//
// La mesure est `document.elementFromPoint` sur le centre et les deux
// coins opposés de la cible, jamais un calcul de rectangles : c'est ce
// que le navigateur répondrait à un clic, donc la seule lecture qui dise
// vraiment « recouvert ». Un test qui comparerait des `getBoundingClientRect`
// se laisserait berner par un élément transparent ou par un `z-index`.

import { expect, test } from './aides/console-propre.js';
import {
    attendreSolveur,
    choisirProgramme,
    ouvrirApplication,
} from './aides/application.js';
import { deplacerCours } from './aides/glisser-deposer.js';

// Empile assez d'avis pour atteindre le plafond d'affichage (3 + le
// replieur « +N autres messages »).
async function empilerDesAvis(page) {
    for (const sigle of ['GCI-1007', 'GCI-1010', 'MCB-1907', 'MAT-2910']) {
        await deplacerCours(page, sigle, 0);
        await attendreSolveur(page);
    }
    await expect(page.locator('.toasts .toast').first()).toBeVisible();
}

// 'libre' | 'TOAST' | la classe de ce qui recouvre.
async function recouvrement(page, locator) {
    return locator.evaluate((element) => {
        const boite = element.getBoundingClientRect();
        const points = [
            [boite.left + boite.width / 2, boite.top + boite.height / 2],
            [boite.left + 4, boite.top + 4],
            [boite.right - 4, boite.bottom - 4],
        ];
        const verdicts = points.map(([x, y]) => {
            const dessus = document.elementFromPoint(x, y);
            if (!dessus) return 'hors-fenetre';
            if (element.contains(dessus) || dessus.contains(element)) return 'libre';
            return dessus.closest('.toasts') ? 'TOAST' : 'autre';
        });
        return verdicts.includes('TOAST') ? 'TOAST' : verdicts.join(',');
    });
}

test('la pile d\'avis reste plafonnée et chaque avis se referme', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);
    await empilerDesAvis(page);

    // ALR-4 : trois avis visibles au plus, le reste derrière un replieur
    // qui les compte — jamais une pile sans fond.
    await expect(page.locator('.toasts .toast:not(.toast--more)')).toHaveCount(3);
    await expect(page.locator('.toast--more')).toContainText('autre');

    // et chacun porte son propre rejet, atteignable au clavier
    const avant = await page.locator('.toasts .toast').count();
    await page.locator('.toasts .toast .status-dismiss').first().click();
    await expect(page.locator('.toasts .toast')).toHaveCount(avant - 1);
});

// DÉFAUT CONNU, reproduit et non corrigé au 2026-08-30 : la pile d'avis
// est en `position: fixed` au-dessus de la grille (`.toasts` dans
// `crates/ui/assets/main.css`) et recouvre « Exporter ▾ » ainsi que les
// colonnes Jeudi et Vendredi. `test.fail()` est l'annotation exacte pour
// ça : la suite reste verte tant que le défaut est là, et devient ROUGE
// (« unexpected success ») le jour où quelqu'un le corrige — ce qui force
// à retirer l'annotation plutôt qu'à oublier le test. Ne pas le
// « réparer » en assouplissant l'assertion.
test.describe('bandeaux et occlusion', () => {
    test.fail();

    for (const fenetre of [
        { width: 1280, height: 720 },
        // la fenêtre exacte des personas du 2026-08-30
        { width: 1280, height: 577 },
    ]) {
        test(`les avis ne cachent ni « Exporter ▾ » ni Jeudi/Vendredi (${fenetre.width}×${fenetre.height})`, async ({
            page,
        }) => {
            await page.setViewportSize(fenetre);
            await ouvrirApplication(page);
            await choisirProgramme(page);
            await empilerDesAvis(page);

            const exporter = page.locator('.status-exports button', {
                hasText: 'Exporter',
            });
            expect(
                await recouvrement(page, exporter),
                'le menu « Exporter ▾ » est recouvert par la pile d\'avis',
            ).not.toBe('TOAST');

            for (const jour of ['Jeudi', 'Vendredi']) {
                const entete = page.locator('.grid-day-head', { hasText: jour });
                expect(
                    await recouvrement(page, entete),
                    `l'en-tête ${jour} est recouvert par la pile d'avis`,
                ).not.toBe('TOAST');
            }
        });
    }
});
