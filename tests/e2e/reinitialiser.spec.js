// « Réinitialiser » vide le document d'un clic, sans confirmation.
//
// C'est délibéré : AIR ACT-2 refuse le dialogue « Êtes-vous sûr ? » comme
// mécanisme de sûreté — sous pression on clique à travers — et exige à la
// place que le geste soit annulable. Ce qui rend le clic acceptable n'est
// donc pas une question posée avant, c'est l'annulation offerte après,
// dans l'avis même du geste (ADR
// `2026-08-reinitialiser-annulable-depuis-son-avis`).
//
// Ce test est donc l'exact opposé de « vérifier qu'on demande
// confirmation » : il vérifie qu'on n'en demande pas, ET que le filet
// existe et tient. Si le filet cède, l'absence de confirmation devient une
// perte sèche.

import { expect, test } from './aides/console-propre.js';
import {
    attendreSolveur,
    choisirProgramme,
    ouvrirApplication,
} from './aides/application.js';

test('« Réinitialiser » efface sans confirmation, et son avis rend le travail', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    const credits = page.locator('.header-credits');
    const avant = await credits.innerText();
    expect(avant).toContain('/120 cr au bac');
    const cours = await page.locator('.ribbon-card-codes span').count();
    expect(cours).toBeGreaterThan(10);

    // aucun dialogue natif ne doit s'ouvrir : s'il en apparaissait un, ce
    // handler le prendrait et le test le dirait
    const dialogues = [];
    page.on('dialog', (dialogue) => {
        dialogues.push(dialogue.message());
        dialogue.dismiss();
    });

    await page.locator('.header-reset').click();
    await attendreSolveur(page);

    expect(dialogues, 'ACT-2 : pas de « Êtes-vous sûr ? »').toEqual([]);
    await expect(credits).toContainText('0/120 cr au bac');
    await expect(page.locator('.ribbon-card-codes span')).toHaveCount(0);

    // le filet : l'avis du geste porte lui-même « Annuler »
    const avis = page.locator('.toasts .toast', {
        hasText: 'réinitialisé',
    });
    await expect(avis).toBeVisible();
    await expect(avis).toContainText('B-GEX');
    const annuler = avis.locator('.toast-undo');
    await expect(annuler).toHaveText('↶ Annuler');

    await annuler.click();
    await attendreSolveur(page);

    await expect(credits).toHaveText(avant);
    await expect(page.locator('.ribbon-card-codes span')).toHaveCount(cours);
    // l'annulation consomme son avis plutôt que de le laisser traîner
    await expect(avis).toHaveCount(0);
});
