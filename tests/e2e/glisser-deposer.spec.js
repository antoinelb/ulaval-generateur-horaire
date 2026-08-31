// Le glisser-déposer d'un cours entre sessions, et ce qu'il déclenche.
//
// C'est le geste que le Rust ne peut pas voir : le solveur ne connaît que
// « ce cours est épinglé en session N », jamais le glissement qui l'y a
// mis. Les deux preuves qui rendent un déposé synthétique probant sont
// dans `aides/glisser-deposer.js` — lire son en-tête avant de toucher à
// ces tests.

import { expect, test } from './aides/console-propre.js';
import {
    attendreSolveur,
    choisirProgramme,
    ouvrirApplication,
} from './aides/application.js';
import { deplacerCours, glisser } from './aides/glisser-deposer.js';

// Le rang des cartes dans le ruban de B-GEX (A26) : 0 = A1-A26,
// 2 = É27, 3 = A3-A27. Les sigles viennent des données réelles.
const A1 = 0;
const A3 = 3;

async function sessionDe(page, sigle) {
    return page.evaluate((code) => {
        const cartes = [...document.querySelectorAll('.ribbon-card')];
        const rang = cartes.findIndex((carte) =>
            [...carte.querySelectorAll('.ribbon-card-codes span')].some(
                (span) => span.textContent.trim() === code,
            ),
        );
        return rang < 0 ? null : cartes[rang].querySelector('.ribbon-card-label').textContent;
    }, sigle);
}

test('glisser un cours d\'une session à l\'autre le déplace vraiment', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    expect(await sessionDe(page, 'MAT-1900')).toBe('A1-A26');

    const geste = await deplacerCours(page, 'MAT-1900', A3);
    // le drop n'est délivré que parce que dragover a prévenu ; il doit
    // prévenir à son tour, sinon le navigateur naviguerait vers la charge
    expect(geste.dropPrevenu).toBe(true);
    await attendreSolveur(page);

    expect(await sessionDe(page, 'MAT-1900')).toBe('A3-A27');
    // un cours déplacé, pas dupliqué
    await expect(
        page.locator('.ribbon-card-codes span', { hasText: /^MAT-1900$/ }),
    ).toHaveCount(1);
});

test('une session qui n\'offre pas le cours refuse le déposé', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    // GEX-1000 n'est offert qu'à l'hiver : la carte d'automne A1-A26 doit
    // refuser le glissement. Le refus ne se lit QUE sur `dragover` — une
    // carte qui ne prévient pas est une carte sur laquelle le navigateur
    // ne délivrera jamais de `drop`.
    const refus = await glisser(page, 'GEX-1000', A1);
    expect(refus.charge).toBe('GEX-1000');
    expect(
        refus.dragoverPrevenu,
        'une saison non offerte doit refuser, donc ne pas prévenir',
    ).toBe(false);
    // et l'aide n'a donc pas délivré de drop : rien n'a bougé
    expect(refus.dropPrevenu).toBeNull();

    await attendreSolveur(page);
    expect(await sessionDe(page, 'GEX-1000')).toBe('H6-H29');
});

test('empiler des cours dans une session déclenche les avertissements', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    await expect(page.locator('.header-credits')).not.toContainText('plafond');

    // quatre cours de plus dans A1-A26 : au-delà du plafond de 17 cr, et
    // dans une session que leurs préalables n'autorisent pas encore
    for (const sigle of ['GCI-1007', 'GCI-1010', 'MCB-1907', 'MAT-2910']) {
        await deplacerCours(page, sigle, A1);
        await attendreSolveur(page);
    }

    // 1. l'en-tête nomme le dépassement, avec le plafond
    await expect(page.locator('.header-credits')).toContainText(
        'plafond de 17 cr dépassé',
    );
    // 2. la carte de session le porte aussi (pas seulement une couleur)
    await expect(page.locator('.ribbon-card').first()).toHaveClass(/ribbon-card--over/);
    await expect(page.locator('.ribbon-card').first()).toContainText('⚠');
    // 3. le panneau nomme la contrainte brisée, session par session
    const verdicts = page.locator('.panel-verdicts');
    await expect(verdicts).toContainText('Plafond de crédits dépassé en A1-A26.');
    // 4. et un préalable manquant est dit avec la sortie de secours
    await expect(page.locator('.toasts')).toContainText('préalable manquant');
    await expect(page.locator('.toasts')).toContainText(
        'Permettre un préalable en concomitance',
    );
});
