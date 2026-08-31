// Les gestes d'ouverture, partagés par toutes les specs.
//
// Le solveur tourne dans un Web Worker : rien de ce qu'une spec observe
// n'est vrai tant qu'il calcule encore. `attendreSolveur` est donc la
// seule façon correcte d'attendre — jamais un `waitForTimeout`, qui
// passerait sur une machine rapide et mentirait sur une machine lente.

import { expect } from './console-propre.js';

// B-GEX est le programme du mandat, et celui que la persona « étudiante
// GEX » a exploré : les sigles cités dans les specs viennent de ses
// données réelles.
const PROGRAMME = 'génie des eaux';

async function ouvrirApplication(page) {
    await page.goto('./');
    await expect(page.locator('.shell')).toBeVisible({ timeout: 60_000 });
}

async function choisirProgramme(page, nom = PROGRAMME) {
    await page
        .locator('.panel-picker-item')
        .filter({ hasText: nom })
        .first()
        .locator('.panel-picker-choose')
        .click();
    // Condition POSITIVE d'abord : le ruban porte huit cartes de session
    // vides avant même qu'un programme soit choisi (constat du directeur
    // GCI, 2026-08-30), donc leur simple présence ne prouve rien.
    await expect(page.locator('.ribbon-card-codes span').first()).toBeVisible({
        timeout: 60_000,
    });
    await attendreSolveur(page);
}

// Le solveur tourne dans un Web Worker, derrière une temporisation de
// 500 ms (`crate::solve::RECALC_DEBOUNCE_MS`). Entre le geste et le
// moment où `.status-running` apparaît, l'écran est donc immobile et
// muet — mesuré à ~840 ms après un choix de programme. Un test qui
// lirait deux fois « rien ne bouge » dans cette fenêtre conclurait que
// tout est fini alors que rien n'a commencé : c'est exactement le faux
// positif qui a fait passer six specs sur un ruban vide au premier
// jet.
//
// D'où la quiétude exigée ici : CINQ lectures consécutives identiques,
// espacées de 400 ms — 1,6 s de calme continu, trois fois la
// temporisation. La signature couvre le compteur de crédits ET le
// nombre de sigles au ruban, parce qu'un replacement peut laisser le
// total identique en déplaçant des cours.
const ECHANTILLON_MS = 400;
const QUIETUDE = 5;

async function attendreSolveur(page) {
    await expect(page.locator('.ribbon-card').first()).toBeVisible({
        timeout: 60_000,
    });
    let calmes = 0;
    let precedente = null;
    await expect
        .poll(
            async () => {
                const etat = await page.evaluate(() => ({
                    enCours: document.querySelectorAll('.status-running').length,
                    credits:
                        document.querySelector('.header-credits')?.innerText ?? '',
                    sigles: document.querySelectorAll('.ribbon-card-codes span')
                        .length,
                }));
                const signature = `${etat.credits}|${etat.sigles}`;
                calmes =
                    etat.enCours === 0 && signature === precedente ? calmes + 1 : 0;
                precedente = signature;
                return calmes;
            },
            {
                timeout: 90_000,
                intervals: [ECHANTILLON_MS],
                message: 'le solveur ne s\'est jamais posé',
            },
        )
        .toBeGreaterThanOrEqual(QUIETUDE);
}

// Le nombre de sessions que le solveur ne touchera plus. C'est la mesure
// du « gel » : `.ribbon-card-freeze input` est la case « Gelé » d'une
// carte de session, la seule affordance de gel de l'interface.
function sessionsGelees(page) {
    return page.locator('.ribbon-card-freeze input:checked');
}

function toutesLesSessions(page) {
    return page.locator('.ribbon-card-freeze input');
}

export {
    PROGRAMME,
    attendreSolveur,
    choisirProgramme,
    ouvrirApplication,
    sessionsGelees,
    toutesLesSessions,
};
