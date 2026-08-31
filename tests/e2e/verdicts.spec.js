// Les deux « ✓ », et pourquoi ils ne doivent pas se lire l'un pour
// l'autre.
//
// - le verdict de SESSION, au-dessus de l'horaire hebdomadaire : il ne
//   parle que de chevauchements de plages, dans la session affichée ;
// - le verdict GLOBAL, dans le panneau : « Placement vérifié ✓ » couvre
//   préalables, plafond et faisabilité d'horaire sur tout l'organigramme,
//   et ne dit rien de la complétude du programme.
//
// Le directeur GCI (2026-08-30) : « mon premier réflexe en voyant un ✓
// vert est de penser c'est bon, publiable ». L'étudiante GEX avait lu le
// verdict de session comme un verdict de tout. Le libellé du premier a
// été précisé — « sans conflit d'horaire ✓ » plutôt que « sans conflit ✓ »
// — et ce test empêche le retour en arrière : le mot qui restreint la
// portée doit rester là, et les deux textes doivent rester distincts.

import { expect, test } from './aides/console-propre.js';
import {
    attendreSolveur,
    choisirProgramme,
    ouvrirApplication,
} from './aides/application.js';
import { deplacerCours } from './aides/glisser-deposer.js';

test('le verdict de session nomme sa portée : l\'horaire, pas le reste', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    const session = page.locator('.grid-status').first();
    // « sans conflit ✓ » tout court laisserait croire à un verdict global
    await expect(session).toContainText("sans conflit d'horaire ✓");
    await expect(session).not.toHaveText(/sans conflit ✓/);
    // et il dit d'où viennent les sections affichées
    await expect(session).toContainText(/combinaison automatique|sections forcées/);
});

test('le verdict global couvre plus, et ne se confond pas avec la complétude', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    const global = page.locator('.panel-verdict', { hasText: 'Placement vérifié' });
    await expect(global).toContainText(
        "Placement vérifié ✓ (préalables, plafond, une combinaison d'horaire possible par session)",
    );
    // le ✓ n'affirme pas que le bac est complet : la ligne suivante dit
    // ce qui manque, et B-GEX laisse cinq sections de règles à combler
    await expect(page.locator('.panel-verdicts')).toContainText(
        /sections de règles restent à combler/,
    );

    // les deux verdicts sont deux textes distincts, à deux endroits
    const texteSession = await page.locator('.grid-status').first().innerText();
    const texteGlobal = await global.innerText();
    expect(texteSession).not.toBe(texteGlobal);
    expect(texteGlobal).not.toContain("sans conflit d'horaire");
});

test('un conflit d\'horaire retire le ✓ des deux verdicts, chacun à sa portée', async ({
    page,
}) => {
    await ouvrirApplication(page);
    await choisirProgramme(page);

    // empiler A1-A26 jusqu'au chevauchement de plages
    for (const sigle of ['GCI-1007', 'GCI-1010', 'MCB-1907', 'MAT-2910']) {
        await deplacerCours(page, sigle, 0);
        await attendreSolveur(page);
    }

    // le verdict de session passe au conflit, et nomme le hachurage
    const session = page.locator('.grid-status').first();
    await expect(session).toContainText("⚠ conflit d'horaire");
    await expect(session).not.toContainText('✓');

    // le verdict global tombe aussi, mais en nommant TOUTES les
    // contraintes possibles — c'est la différence de portée
    const verdicts = page.locator('.panel-verdicts');
    await expect(verdicts).not.toContainText('Placement vérifié ✓');
    await expect(verdicts).toContainText("Conflit d'horaire en A1-A26");
    await expect(verdicts).toContainText('Plafond de crédits dépassé en A1-A26.');
});
