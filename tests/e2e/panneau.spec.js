// Le panneau de gauche après un épinglage.
//
// « Le panneau revient tout en haut après chaque épinglage » — rapporté au
// moins cinq fois par la persona étudiante GEX (2026-08-30). C'est une
// violation directe d'AIR LAY-2 : rien de ce qui est à l'écran ne bouge
// si l'utilisatrice ne l'a pas bougé. Le symptôme est invisible au Rust,
// qui ne connaît que l'état du plan.
//
// Le piège de mesure : `.panel` défile *à l'intérieur*, la fenêtre ne
// défile pas. Une spec qui lirait `window.scrollY` ne verrait jamais
// rien — c'est `aside.panel.scrollTop` qu'il faut lire.

import { expect, test } from './aides/console-propre.js';
import {
    attendreSolveur,
    choisirProgramme,
    ouvrirApplication,
} from './aides/application.js';

test('épingler un cours ne renvoie pas le panneau en haut', async ({ page }) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    const panneau = page.locator('aside.panel');
    // ouvrir une règle à option : c'est là que vivent les cours qu'on
    // épingle un par un, et c'est ce qui rend le panneau assez long pour
    // que la position compte
    await page.locator('.panel-rule-head', { hasText: 'Règle 1' }).click();
    const chip = page.locator('.panel-course').nth(2).locator('.panel-chip').nth(1);
    await chip.scrollIntoViewIfNeeded();

    const avant = await panneau.evaluate((element) => element.scrollTop);
    expect(avant, 'le panneau doit être descendu pour que le test ait un sens')
        .toBeGreaterThan(200);

    await chip.click();
    await attendreSolveur(page);

    const apres = await panneau.evaluate((element) => element.scrollTop);
    // LAY-2 : pas de retour en haut
    expect(apres, 'le panneau est remonté tout en haut après l\'épinglage')
        .toBeGreaterThan(0);
    // et pas de saut : le cours épinglé reste sous les yeux. La tolérance
    // est la hauteur visible du panneau — au-delà, il a fallu rechercher
    // le cours, ce qui est exactement la friction rapportée.
    const visible = await panneau.evaluate((element) => element.clientHeight);
    expect(
        Math.abs(apres - avant),
        `le panneau a sauté de ${Math.abs(apres - avant)} px`,
    ).toBeLessThan(visible);

    // et le chip cliqué est bien encore dans la zone visible du panneau
    const dansLaVue = await chip.evaluate((element) => {
        const cadre = element.closest('aside.panel').getBoundingClientRect();
        const boite = element.getBoundingClientRect();
        return boite.top >= cadre.top - 1 && boite.bottom <= cadre.bottom + 1;
    });
    expect(dansLaVue, 'le cours épinglé est sorti de la vue').toBe(true);

    // le clic a bien fait son travail — sinon le test ci-dessus serait
    // vrai pour une raison sans intérêt
    await expect(chip).toHaveAttribute('aria-pressed', 'true');
});
