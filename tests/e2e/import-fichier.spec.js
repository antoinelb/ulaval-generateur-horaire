// L'aide de l'import par fichier JSON.
//
// Un fichier programme n'était expliqué nulle part : ni ce que c'est, ni
// d'où il vient, ni à quoi il ressemble (Antoine, 2026-09-01 ; ADR
// `2026-09-aide-gabarit-et-lien-github-pour-l-import-json`). La spec
// vérifie ce que le Rust ne voit pas : le « ? » déplie l'aide en place et
// la replie, le gabarit est un asset réellement servi (200), et le lien
// GitHub sort de l'app proprement (nouvel onglet, noopener).

import { expect, test } from './aides/console-propre.js';
import { ouvrirApplication } from './aides/application.js';

test('le « ? » du fichier programme déplie l\'aide, le gabarit et le lien GitHub', async ({ page }) => {
    await ouvrirApplication(page);
    await page.locator('.panel-import-toggle').click();

    const bouton = page.locator('.panel-import-json .panel-help-toggle');
    const aide = page.locator('#panel-import-json-help');
    await expect(bouton).toHaveAttribute('aria-expanded', 'false');
    await expect(aide).toHaveCount(0);

    await bouton.click();
    await expect(bouton).toHaveAttribute('aria-expanded', 'true');
    await expect(aide).toBeVisible();

    // le gabarit : un vrai instantané embarqué, réellement servi — pas un
    // lien mort vers un asset que le déploiement n'aurait pas copié
    const gabarit = aide.locator('a[download]');
    await expect(gabarit).toHaveAttribute('download', 'B-GEX-A26.json');
    const href = await gabarit.getAttribute('href');
    const reponse = await page.request.get(new URL(href, page.url()).href);
    expect(reponse.status(), `le gabarit ${href} doit être servi`).toBe(200);

    // le dossier des instantanés publiés, hors de l'app : nouvel onglet,
    // sans poignée sur la page d'origine
    const github = aide.locator('a[target="_blank"]');
    await expect(github).toHaveAttribute(
        'href',
        'https://github.com/antoinelb/ulaval-generateur-horaire/tree/main/data/programmes',
    );
    await expect(github).toHaveAttribute('rel', 'noopener');

    // refermable en place (LAY-4) : le même bouton replie l'aide
    await bouton.click();
    await expect(bouton).toHaveAttribute('aria-expanded', 'false');
    await expect(aide).toHaveCount(0);
});
