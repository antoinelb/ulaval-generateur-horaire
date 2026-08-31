// Le « gel au clic » — l'accusation à trancher.
//
// Une étudiante a rapporté (rapport du directeur GCI, 2026-08-30) qu'un
// simple clic sur la cellule de la première session avait « gelé » douze
// cases sans rien lui demander. Le directeur ne l'a pas reproduit et a
// transmis le constat sous réserve. Ces tests tranchent : ils cliquent
// tout ce qu'une étudiante clique sur une grille — la carte de session,
// un bloc de l'horaire, un sigle du ruban — et comptent les cases
// « Gelé » avant et après.
//
// Verdict (2026-08-30) : aucun clic ne gèle quoi que ce soit. Le gel
// qu'elle a vu est réel mais vient d'ailleurs — l'ouverture d'un lien de
// partage gèle tout l'horizon, par décision (ADR
// `2026-08-un-lien-rouvre-un-organigramme-gele`), et elle explorait
// justement un lien reçu du directeur. Les deux tests ci-dessous fixent
// les deux moitiés de la réponse pour qu'aucune des deux ne dérive.

import { expect, test } from './aides/console-propre.js';
import {
    attendreSolveur,
    choisirProgramme,
    ouvrirApplication,
    sessionsGelees,
    toutesLesSessions,
} from './aides/application.js';

test('aucun clic sur la grille ne gèle une session', async ({ page }) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    // état de départ : le solveur a la main sur tout l'horizon
    await expect(sessionsGelees(page)).toHaveCount(0);
    const horizon = await toutesLesSessions(page).count();
    expect(horizon).toBeGreaterThan(0);

    // 1. la carte de session — le geste précis qu'elle décrit
    await page.locator('.ribbon-card').first().locator('.ribbon-card-face').click();
    await expect(sessionsGelees(page)).toHaveCount(0);

    // 2. un bloc de l'horaire hebdomadaire (celui-là ouvre les horaires
    //    alternatifs : une action, mais pas un gel)
    await page.locator('.grid-block').first().click();
    await expect(sessionsGelees(page)).toHaveCount(0);
    await page.keyboard.press('Escape');

    // 3. un sigle dans le ruban, qui est aussi une poignée de glissement
    await page.locator('.ribbon-card-codes span').first().click();
    await expect(sessionsGelees(page)).toHaveCount(0);

    // et le solveur a toujours la main : rien ne s'est figé en coulisse
    await attendreSolveur(page);
    await expect(sessionsGelees(page)).toHaveCount(0);
});

test('la case « Gelé » est la seule affordance qui gèle, et elle le dit', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    const premiere = page.locator('.ribbon-card-freeze').first();
    // « Gelé » précède la case : cochée seule, elle ne dirait pas de quoi
    // il s'agit (INP-4). Le libellé accessible nomme la session.
    await expect(premiere).toContainText('Gelé');
    await expect(premiere.locator('input')).toHaveAttribute(
        'aria-label',
        /A1-A26/,
    );

    await premiere.locator('input').click();
    await expect(sessionsGelees(page)).toHaveCount(1);
    // et le geste est réversible d'un clic — pas de dialogue, pas de piège
    await premiere.locator('input').click();
    await expect(sessionsGelees(page)).toHaveCount(0);
});

// La troisième route vers le symptôme, et la plus probable dans un onglet
// où l'on vient de cliquer « Partager » : le fragment reste dans la barre
// d'adresse, donc le rechargement suivant réimporte le lien — et l'import
// gèle tout l'horizon, y compris celui de l'expéditeur. Aucun clic n'est
// en cause là non plus ; c'est le même mécanisme que dans un navigateur
// vierge (`persistance.spec.js`), appliqué à son propre document.
test('après « Partager », un rechargement réimporte le lien et gèle tout', async ({
    page,
    context,
}) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await ouvrirApplication(page);
    await choisirProgramme(page);
    await expect(sessionsGelees(page)).toHaveCount(0);

    await page.locator('.status-exports button', { hasText: 'Partager' }).click();
    await expect(page).toHaveURL(/#/);
    // partager ne gèle rien par soi-même
    await expect(sessionsGelees(page)).toHaveCount(0);

    const horizon = await toutesLesSessions(page).count();
    await page.reload();
    await attendreSolveur(page);
    await expect(sessionsGelees(page)).toHaveCount(horizon);
});
