// Ce qui survit au rechargement, et ce qu'un lien de partage donne à
// quelqu'un qui n'a jamais ouvert l'outil.
//
// Chaque test Playwright reçoit un contexte neuf : le `localStorage` est
// donc réellement vide au départ, sans qu'aucune spec ait à le purger.
// C'est ce qui rend le second test honnête — les personas, elles, ont dû
// vider la session à la main pour ne pas fausser le premier contact.

import { expect, test } from './aides/console-propre.js';
import {
    attendreSolveur,
    choisirProgramme,
    ouvrirApplication,
    sessionsGelees,
    toutesLesSessions,
} from './aides/application.js';

test('le travail survit à un rechargement', async ({ page }) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    const credits = await page.locator('.header-credits').innerText();
    const ruban = await page.locator('.ribbon-card-codes span').allInnerTexts();
    expect(ruban.length).toBeGreaterThan(10);

    await page.reload();
    await attendreSolveur(page);

    await expect(page.locator('.header-subtitle')).toContainText('génie des eaux');
    await expect(page.locator('.header-credits')).toHaveText(credits);
    expect(await page.locator('.ribbon-card-codes span').allInnerTexts()).toEqual(ruban);
});

test('un lien de partage rouvre tout dans un navigateur vierge', async ({
    page,
    context,
    browser,
}) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await ouvrirApplication(page);
    await choisirProgramme(page);

    const credits = await page.locator('.header-credits').innerText();
    const ruban = await page.locator('.ribbon-card-codes span').allInnerTexts();
    // l'expéditeur n'a rien gelé : le gel qu'on verra à l'arrivée vient
    // de l'import, pas du document envoyé
    await expect(sessionsGelees(page)).toHaveCount(0);

    await page.locator('.status-exports button', { hasText: 'Partager' }).click();
    await expect(page.locator('.toasts')).toContainText('Lien copié');
    const lien = page.url();
    expect(lien).toContain('#');

    // un contexte neuf : autre `localStorage`, autre cache, personne
    const vierge = await browser.newContext({ viewport: { width: 1280, height: 720 } });
    const destinataire = await vierge.newPage();
    const erreurs = [];
    destinataire.on('pageerror', (erreur) => erreurs.push(erreur.message));
    destinataire.on('console', (message) => {
        if (message.type() === 'error') erreurs.push(message.text());
    });

    await destinataire.goto(lien);
    // L'avis d'import est vérifié AVANT d'attendre le solveur, et ce
    // n'est pas un détail de style : c'est un ✓ qui s'auto-efface au bout
    // d'environ cinq secondes, alors que le gel qu'il explique, lui, est
    // permanent. Une étudiante qui regarde ailleurs cinq secondes se
    // retrouve avec tout l'horizon gelé et plus une ligne à l'écran pour
    // dire pourquoi — c'est exactement le malentendu du 2026-08-30.
    await expect(destinataire.locator('.ribbon-card').first()).toBeVisible({
        timeout: 60_000,
    });
    await expect(destinataire.locator('.toasts')).toContainText(
        'toutes ses sessions gelées',
    );
    await expect(destinataire.locator('.toasts')).toContainText('Tout dégeler');

    await attendreSolveur(destinataire);

    await expect(destinataire.locator('.header-subtitle')).toContainText(
        'génie des eaux',
    );
    await expect(destinataire.locator('.header-credits')).toHaveText(credits);
    expect(
        await destinataire.locator('.ribbon-card-codes span').allInnerTexts(),
    ).toEqual(ruban);

    // Un lien rouvre l'organigramme gelé (ADR
    // `2026-08-un-lien-rouvre-un-organigramme-gele`) : le solveur ne
    // déplace rien, le destinataire voit ce que l'expéditeur a envoyé.
    // C'est la source du gel que l'étudiante a pris pour un effet de son
    // clic — voir `gel.spec.js`.
    const horizon = await toutesLesSessions(destinataire).count();
    await expect(sessionsGelees(destinataire)).toHaveCount(horizon);

    expect(erreurs, 'console de la page destinataire').toEqual([]);
    await vierge.close();
});
