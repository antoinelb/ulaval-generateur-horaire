// Le glisser-déposer HTML5 ne se déclenche PAS par de vrais mouvements de
// souris sous Playwright : `mouse.down/move/up` ne produit aucun
// `DragEvent`. On dispatche donc les événements à la main, avec un seul
// `DataTransfer` partagé — comme le fait le navigateur.
//
// Un tel déposé synthétique ne prouve rien par lui-même : dispatcher
// `drop` sur n'importe quel élément « réussit » toujours, y compris sur
// une cible qui, dans un vrai navigateur, refuserait le déposé. Deux
// vérifications le rendent probant, et ce module les renvoie toujours
// plutôt que de les cacher :
//
// 1. `dragoverPrevenu` — un navigateur ne délivre `drop` que si le
//    gestionnaire de `dragover` a appelé `preventDefault()`. Sans cette
//    lecture, un test verrait passer un déposé que le vrai navigateur
//    aurait rejeté. C'est aussi ce qui distingue une carte de session qui
//    accueille le cours d'une carte qui le refuse (saison non offerte) :
//    la seconde ne prévient pas, et c'est la seule preuve du refus.
// 2. `charge` — le contenu de `dataTransfer` après `dragstart`. Firefox
//    refuse de porter un glissement dont le `DataTransfer` est vide ;
//    l'interface écrit donc un jeton `text/plain` (voir
//    `RibbonCode::ondragstart` dans `crates/ui/src/components/ribbon.rs`).
//    Si ce jeton disparaissait, Chromium continuerait de passer et
//    Firefox casserait en silence — la lecture le rend visible ici.

import { expect } from './console-propre.js';

// Renvoie { charge, dragoverPrevenu, dropPrevenu } sans rien affirmer :
// c'est à l'appelant de dire ce qu'il attend (accueil ou refus).
// La cible est désignée par son *rang* dans `.ribbon-card` : les cartes
// de session et les bandes d'été partagent la classe sans partager le
// type d'élément, donc `nth-of-type` mentirait.
async function glisser(page, sigle, indexCible) {
    return page.evaluate(
        ({ texte, index }) => {
            const depart = [
                ...document.querySelectorAll('.ribbon-card-codes span'),
            ].find((element) => element.textContent.trim() === texte);
            if (!depart) {
                throw new Error(`Sigle absent du ruban : ${texte}`);
            }
            const arrivee = document.querySelectorAll('.ribbon-card')[index];
            if (!arrivee) {
                throw new Error(`Aucune carte de session au rang ${index}`);
            }
            const transfert = new DataTransfer();
            const evenement = (type) =>
                new DragEvent(type, {
                    bubbles: true,
                    cancelable: true,
                    dataTransfer: transfert,
                });

            depart.dispatchEvent(evenement('dragstart'));
            const charge = transfert.getData('text/plain');

            const survol = evenement('dragover');
            arrivee.dispatchEvent(survol);
            const dragoverPrevenu = survol.defaultPrevented;

            // Le vrai navigateur ne délivrerait pas `drop` sans le
            // `preventDefault` ci-dessus : ne pas le délivrer non plus,
            // sinon le test simulerait un geste impossible.
            let dropPrevenu = null;
            if (dragoverPrevenu) {
                const depot = evenement('drop');
                arrivee.dispatchEvent(depot);
                dropPrevenu = depot.defaultPrevented;
            }
            depart.dispatchEvent(evenement('dragend'));
            return { charge, dragoverPrevenu, dropPrevenu };
        },
        { texte: sigle, index: indexCible },
    );
}

// Le cas courant : déplacer un sigle du ruban vers la carte de session
// d'indice `indexCible`, en exigeant les deux preuves.
async function deplacerCours(page, sigle, indexCible) {
    const resultat = await glisser(page, sigle, indexCible);
    expect(
        resultat.charge,
        'dragstart doit écrire un jeton text/plain, sinon Firefox annule le glissement',
    ).toBe(sigle);
    expect(
        resultat.dragoverPrevenu,
        'dragover doit appeler preventDefault, sinon aucun navigateur ne délivrerait le drop',
    ).toBe(true);
    return resultat;
}

export { deplacerCours, glisser };
