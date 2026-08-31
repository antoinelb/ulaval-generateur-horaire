// Fondation des tests navigateur (ADR `2026-08-fondation-playwright`).
//
// L'application testée est le **bundle de production**, celui que
// `make ui-build` dépose dans `_ui/public` et que la CI publie — pas
// `dx serve`. Trois raisons, détaillées dans l'ADR : le service worker
// n'existe que dans le bundle (« Under `dx serve` it is absent and
// nothing registers », `crates/ui/assets/sw.js`), donc tout un chemin de
// lecture hors ligne serait hors de portée ; le serveur de développement
// injecte son client de rechargement à chaud et son bandeau « Your app is
// being rebuilt », que la persona du 2026-08-30 a vu passer au milieu
// d'un test et qui rendrait l'assertion de console propre ingouvernable ;
// et un artefact reconstruit sous nos pieds n'est pas un sujet de test.

import { defineConfig, devices } from '@playwright/test';

const PORT = Number(process.env.PORT_E2E ?? 8317);
const BASE = `http://127.0.0.1:${PORT}/ulaval-generateur-horaire/`;

export default defineConfig({
    testDir: 'tests/e2e',
    fullyParallel: true,
    // le solveur WASM est gourmand : au-delà de quatre onglets le calcul
    // s'étire et les fenêtres de quiétude d'`attendreSolveur` deviennent
    // une course plutôt qu'une mesure
    workers: 4,
    forbidOnly: !!process.env.CI,
    retries: 0,
    reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : [['list']],
    // le solveur tourne dans un worker WASM : une session complète de
    // placement d'un bac dépasse le défaut de 5 s d'`expect`
    expect: { timeout: 15_000 },
    timeout: 90_000,
    use: {
        baseURL: BASE,
        // 1280×720 : la fenêtre des trois personas était plus courte
        // (1280×577) et c'est là qu'elles ont vu les zones de défilement
        // à l'étroit. On teste la taille confortable ; les specs qui
        // portent sur l'occlusion fixent la leur.
        viewport: { width: 1280, height: 720 },
        trace: 'retain-on-failure',
    },
    projects: [
        { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    ],
    webServer: {
        command: 'node tests/e2e/aides/serveur.mjs',
        url: BASE,
        reuseExistingServer: !process.env.CI,
        stdout: 'pipe',
        stderr: 'pipe',
    },
});
