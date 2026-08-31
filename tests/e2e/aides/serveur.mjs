// Sert le bundle de production (`_ui/public`) sous le segment que la
// macro `asset!()` a gravé dans le HTML.
//
// `make ui-build` construit avec `--base-path ulaval-generateur-horaire`,
// et `asset!()` émet des URL **absolues** :
// `/ulaval-generateur-horaire/assets/…`. Servir `_ui/public` à la racine
// donne donc 404 sur chaque asset — sans plantage, sans message : juste
// une page nue. C'est le mode de défaillance le plus coûteux du dépôt,
// parce qu'il ressemble à un test qui n'a pas fini de charger.
//
// Ce serveur refuse donc silencieusement une seule chose : rien. Un
// bundle absent tue le processus avec la commande à lancer ; un chemin
// hors du préfixe répond 404 en le disant. Le type MIME de `.wasm` est
// obligatoire : sans lui `WebAssembly.instantiateStreaming` refuse le
// module et l'application ne démarre jamais.

import { createServer } from 'node:http';
import { createReadStream, statSync } from 'node:fs';
import { join, normalize, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const RACINE = fileURLToPath(new URL('../../../_ui/public', import.meta.url));
const PREFIXE = '/ulaval-generateur-horaire';
const PORT = Number(process.env.PORT_E2E ?? 8317);

const TYPES = {
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.mjs': 'text/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8',
    '.json': 'application/json; charset=utf-8',
    '.wasm': 'application/wasm',
    '.ico': 'image/x-icon',
    '.svg': 'image/svg+xml',
    '.png': 'image/png',
    '.txt': 'text/plain; charset=utf-8',
};

try {
    statSync(join(RACINE, 'index.html'));
} catch {
    process.stderr.write(
        `Bundle introuvable : ${RACINE}/index.html\n` +
            'Construisez-le d\'abord — `make ui-build` (ce que fait `make e2e`).\n',
    );
    process.exit(1);
}

function repondre(reponse, code, texte) {
    reponse.writeHead(code, { 'content-type': 'text/plain; charset=utf-8' });
    reponse.end(texte);
}

const serveur = createServer((requete, reponse) => {
    const chemin = decodeURIComponent(new URL(requete.url, 'http://localhost').pathname);
    if (chemin !== PREFIXE && !chemin.startsWith(`${PREFIXE}/`)) {
        repondre(
            reponse,
            404,
            `Hors du préfixe : l'application se sert sous ${PREFIXE}/ ` +
                '(les URL d\'`asset!()` sont absolues sous ce segment).',
        );
        return;
    }
    const relatif = chemin.slice(PREFIXE.length) || '/';
    // `normalize` d'abord, puis `join` sur la racine : une remontée
    // `..` est écrasée avant de toucher au disque
    const fichier = join(RACINE, normalize(relatif).replace(/^(\.\.[/\\])+/, ''));
    let cible = fichier;
    try {
        if (statSync(cible).isDirectory()) {
            cible = join(cible, 'index.html');
        }
        statSync(cible);
    } catch {
        // l'application est une SPA : un chemin inconnu rend l'index,
        // comme le fait GitHub Pages pour un site à route unique
        cible = join(RACINE, 'index.html');
    }
    reponse.writeHead(200, {
        'content-type': TYPES[extname(cible)] ?? 'application/octet-stream',
        // aucune mise en cache HTTP : deux tests d'affilée doivent voir
        // le même bundle que celui sur le disque, jamais une copie
        'cache-control': 'no-store',
    });
    createReadStream(cible).pipe(reponse);
});

serveur.listen(PORT, '127.0.0.1', () => {
    process.stdout.write(`Bundle servi sur http://127.0.0.1:${PORT}${PREFIXE}/\n`);
});
