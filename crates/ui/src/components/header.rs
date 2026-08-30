use dioxus::prelude::*;

use super::{AlertBody, AlertStack, SolverHandle, SolverState};
use crate::data::Snapshot;
use crate::state::{self, History, Plan, View};

// LAT-4: a visible search is never a bare spinner — elapsed seconds and a
// real cancel (the worker dies, a fresh one boots)
#[component]
fn SolverStatus() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let alerts = use_context::<Signal<AlertStack>>();
    let solver = use_context::<Signal<SolverState>>();
    let handle = use_context::<SolverHandle>();
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    let snapshot = use_context::<Signal<Option<crate::data::Snapshot>>>();
    let mut now_ms = use_signal(crate::browser::now_epoch_ms);
    use_future(move || async move {
        // bounded ticker; idles harmlessly when nothing runs
        for _ in 0..86_400u32 {
            crate::browser::sleep_ms(1_000).await;
            now_ms.set(crate::browser::now_epoch_ms());
        }
    });
    let Some(running) = solver.read().running else {
        return rsx! {};
    };
    let elapsed = now_ms().saturating_sub(running.started_ms) / 1_000;
    let what = match running.kind {
        super::QueryKind::Propose => "recherche d'un organigramme",
        super::QueryKind::Verify => "vérification du cheminement",
    };
    rsx! {
        span { class: "status-running",
            "{what} - "
            // LAY-2 : largeur réservée pour 3 chiffres (999 s ≈ 16 min,
            // bien au-delà d'une recherche réelle avant annulation) —
            // au-delà le texte s'élargirait quand même, ce cas est assumé
            span { class: "status-running-elapsed", "{elapsed} s" }
            button {
                class: "status-undo",
                onclick: move |_| {
                    super::cancel_search(
                        &handle, solver, plan, alerts, manual, snapshot,
                    );
                },
                "Annuler la recherche"
            }
        }
    }
}

#[component]
pub fn HeaderBar() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    // the Signal itself, kept from the shadowing below: « changer » swaps
    // documents and needs the signals, not one render's borrow
    let snapshot_signal = snapshot;
    let solver = use_context::<Signal<SolverState>>();
    let handle = use_context::<SolverHandle>();
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let alerts = use_context::<Signal<AlertStack>>();
    // « changer » emporte le document entier : les cours manuels voyagent
    // avec lui
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    // (code, millésime) — two vintages of one program are two programs,
    // and the chosen one stays named on screen, concentration et profil
    // compris (rapport étudiante ; décision 2026-08-19)
    let program_title = crate::panel::program_subtitle(snapshot, &plan.read())
        .unwrap_or_else(|| "aucun programme choisi".to_string());
    let has_program =
        crate::panel::chosen_program(snapshot, &plan.read()).is_some();
    let history = use_context::<Signal<crate::state::History>>();
    // note 8: the whole bac's tally next to the session's — counted by
    // `ui-calculations`, en-sus and préparatoire credits kept out. Both
    // tallies are composed together by `panel::credit_readout` so the hold
    // below can never freeze one and refresh the other.
    let current = crate::panel::credit_readout(
        snapshot,
        &plan.read(),
        view.read().session,
    );
    // LAT-6 : tant qu'une réponse est attendue, les totaux gardent leur
    // dernière valeur arrêtée, atténuée et dite comme telle — jamais la
    // valeur intermédiaire que le recalcul est en train de remplacer
    // (« 30/120 cr » pendant trois secondes, rapport directeur-gci
    // 2026-08-29). Le premier calcul n'a rien à tenir : il passe tel quel.
    let awaited = solver.read().awaited_since.is_some();
    let mut settled = use_signal(|| None::<crate::panel::CreditReadout>);
    // Le seul instant où une valeur mérite d'être retenue est celui où le
    // solveur vient de se prononcer sur ce qui est affiché : un verdict
    // présent et non périmé. S'abonner au plan à la place retiendrait
    // l'état intermédiaire lui-même — c'est le « 21/120 cr » qu'il faut
    // tenir à l'écart, pas le figer. `peek` sur le plan, la vue et
    // l'instantané pour cette même raison : eux seuls ne déclenchent
    // aucune capture.
    use_effect(move || {
        let (vouched, verdictless) = {
            let state = solver.read();
            (
                state.verification.is_some() && !state.verification_stale,
                state.verification.is_none(),
            )
        };
        let read = snapshot_signal.peek();
        let fresh = read.as_ref().filter(|_| vouched).map(|snapshot| {
            crate::panel::credit_readout(
                snapshot,
                &plan.peek(),
                view.peek().session,
            )
        });
        // un verdict frais remplace la valeur tenue; un verdict *disparu*
        // (bascule de document) l'efface — tenir celle du document quitté
        // serait un mensonge d'un autre genre; un verdict simplement
        // périmé la laisse telle quelle, c'est tout son emploi
        if fresh.is_some() || verdictless {
            settled.set(fresh);
        }
    });
    let (shown, stale) = crate::present::held_while_awaited(
        settled.read().as_ref(),
        &current,
        awaited,
    );
    let credits = shown.session;
    // a Range counted at its lower bound says so (TRU-6: never a false
    // precision)
    let range_mark = if credits.has_range { " (min.)" } else { "" };
    let cap = shown.cap;
    let over_cap = credits.total > cap;
    let bac = shown.bac;
    let stale_title = if stale {
        " — valeur de la solution précédente, recalcul en cours"
    } else {
        ""
    };
    rsx! {
        header { class: "header-bar",
            div { class: "header-logo", aria_hidden: true }
            h1 { "Générateur d'horaire" }
            span { class: "header-subtitle", "{program_title}" }
            if has_program {
                button {
                    class: "status-undo",
                    title: "Revenir au choix de programme — ce cheminement \
                            est conservé et revient en le rechoisissant",
                    onclick: {
                        let handle = handle.clone();
                        move |_| {
                            // « Annuler » goes dark with the history: the
                            // way back is the shelf, so the swap says so
                            // on screen and not only in a hover title
                            let kept = plan.peek().program.as_ref().map(
                                |choice| {
                                    crate::present::shelved_note(
                                        &choice.code,
                                        &choice.semester,
                                    )
                                },
                            );
                            // the document goes to its shelf whole; the
                            // picker takes over with an empty grid (US-10)
                            let swap = crate::persist::leave_document(
                                &plan.peek(),
                            );
                            super::swap_document(
                                plan,
                                view,
                                history,
                                alerts,
                                solver,
                                &handle,
                                manual,
                                snapshot_signal,
                                swap,
                            );
                            if let Some(note) = kept {
                                // `Standing`, not `Success`: the ✓ that
                                // clears itself after five seconds was
                                // gone before the student had read the
                                // rest of the screen, which meanwhile
                                // reads as a total loss. `Document`, not
                                // `Sticky`: pushed after the purge it
                                // survives its own swap and leaves at the
                                // next one, so three programs browsed
                                // never stack three avis (ALR-3) — ADR
                                // `2026-08-la-bascule-dit-ou-va-le-travail-et-pourquoi-annuler-est-eteint`
                                super::push_caused_alert(
                                    alerts,
                                    AlertBody::Standing(note),
                                    super::AlertCause::Document,
                                );
                            }
                        }
                    },
                    "changer"
                }
            }
            span { class: "header-notice",
                "⚠ Horaires à titre indicatif, d'après le site web de \
                 l'Université. Vérifiez-les et revalidez l'organigramme \
                 à chaque session au cas où des horaires de cours \
                 auraient changé."
            }
            span {
                class: "header-credits",
                // la couleur seule ne dit jamais rien (INP-3) : le titre
                // porte le fait, l'atténuation ne fait que le rappeler
                class: if stale { "header-credits--stale" },
                if let Some(label) = bac {
                    span {
                        class: if label.over { "header-credits--over" },
                        title: "{label.tooltip}{stale_title}",
                        "{label.text} - "
                    }
                }
                b {
                    class: if over_cap { "header-credits--over" },
                    title: "{stale_title}",
                    "{credits.total} cr{range_mark} cette session"
                }
                if over_cap {
                    span { class: "header-credits--over",
                        " ⚠ plafond de {cap} cr dépassé"
                    }
                }
            }
            // « Tout geler » se range *avant* « Réinitialiser », jamais
            // après : les 2 rem de `.header-reset` tiennent le geste
            // destructeur à l'écart de ce qui le borde (ACT-5), et un
            // bouton posé après lui mangerait cet écart.
            FreezeAllButton {}
            // ACT-5: « Réinitialiser » vide le document et se tenait juste
            // à côté de « Partager », le geste courant — un clic de travers
            // coûtait tout l'organigramme (rapport Camille, 2026-08-29).
            // « Partager » a rejoint « Exporter » dans la bande de statut
            // (Antoine, 2026-08-30) : plus aucun geste courant ne le
            // jouxte, et il reste à découvert, jamais dans un menu (LAY-7).
            ResetButton {}
        }
    }
}

// La bascule de gel du ruban, portée sur l'horizon entier : un clic ferme
// toutes les sessions au solveur, le suivant les rouvre. Un seul bouton,
// annulable par lui-même autant que par « Annuler » (ACT-2) — ADR
// `2026-08-bouton-tout-geler-dans-la-barre-du-haut`.
#[component]
fn FreezeAllButton() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    // libellé, titre et étiquette d'annulation viennent du module pur : la
    // vue ne décide rien
    let act = crate::present::freeze_all(&plan.read());
    rsx! {
        button {
            class: "status-undo",
            title: "{act.title}",
            onclick: move |_| {
                // relu au clic : le plan a pu changer depuis ce rendu
                let crate::present::FreezeAll {
                    undo_label, frozen, ..
                } = crate::present::freeze_all(&plan.peek());
                super::edit_plan(plan, history, undo_label, move |plan| {
                    plan.frozen = frozen;
                });
            },
            "{act.label}"
        }
    }
}

// « Réinitialiser » : one click, undoable — never a confirmation (ACT-2).
// The document empties *without leaving its program* — dropping the choice
// dumped the student in the picker, and the click that got him out of it
// (`swap_document`) cleared `History`, so the undo the reset had just
// armed died before it could be used (ADR
// `2026-08-reinitialiser-reste-dans-le-programme`). Its shelf copy goes
// with the content — re-choosing the program must not resurrect the
// pre-reset grid; the other programs' shelves survive (ADR
// `2026-08-reinitialiser-le-document-courant-et-son-etagere`). The
// hand-entered course fiches stay, they extend the catalogue, not the
// document — an undo may restore a plan that references them (ADR
// `2026-08-bouton-tout-reinitialiser`).
// Its own avis carries the way back: « Annuler » stayed lit but nothing
// on screen said the work was recoverable, so a reflex click read as a
// silent, total loss (ADR
// `2026-08-reinitialiser-annulable-depuis-son-avis`).
#[component]
fn ResetButton() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let mut view = use_context::<Signal<View>>();
    let history = use_context::<Signal<History>>();
    let alerts = use_context::<Signal<AlertStack>>();
    let solver = use_context::<Signal<SolverState>>();
    let handle = use_context::<SolverHandle>();
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    let snapshot = use_context::<Signal<Option<crate::data::Snapshot>>>();
    rsx! {
        button {
            class: "status-undo header-reset",
            title: "Repartir de zéro — ce programme seulement, annulable \
                    depuis l'avis qui suit et avec « Annuler »",
            onclick: move |_| {
                // a search in flight must not land its proposal in the
                // fresh plan
                if solver.peek().running.is_some() {
                    super::cancel_search(
                        &handle, solver, plan, alerts, manual, snapshot,
                    );
                }
                // a shared link left in the address bar would reimport
                // everything at the next reload
                crate::browser::strip_query();
                // the horizon a brand-new document of this program would
                // get, exactly as « Choisir » computes it
                let study_sessions = {
                    let plan_read = plan.peek();
                    let read = snapshot.peek();
                    plan_read
                        .program
                        .as_ref()
                        .zip(read.as_ref())
                        .and_then(|(choice, snapshot)| {
                            crate::panel::program_credits_required(
                                snapshot,
                                &choice.code,
                                &choice.semester,
                            )
                        })
                        .map(crate::state::default_study_sessions)
                        .unwrap_or(crate::state::DEFAULT_STUDY_SESSIONS)
                };
                // repartir de zéro, mais jamais dans le passé : le
                // « Début » d'usine est un A26 en dur (ADR
                // `2026-08-debut-ancre-sur-lhorloge`)
                let reset = crate::persist::reset_document(
                    &plan.peek(),
                    study_sessions,
                    crate::state::semester_of_epoch_ms(
                        crate::browser::now_epoch_ms(),
                    ),
                );
                let shelf = reset.shelf;
                let next = reset.next;
                // the document as it stood, carried by the avis so its
                // « Annuler » works whatever the student does next
                let before = plan.peek().clone();
                super::edit_plan(plan, history, "Réinitialisation", |plan| {
                    *plan = next;
                });
                if let Some(key) = shelf {
                    crate::browser::local_remove(&key);
                }
                view.set(View::default());
                // `Document`, never `Sticky` : l'avis transporte le plan
                // d'*un* programme, et son « Annuler » le réinstallerait
                // dans un autre si l'étudiante changeait de programme
                // entre-temps — la bascule doit l'emporter avec le
                // document qu'il décrit (ADR
                // `2026-08-historique-par-document-vide-a-la-bascule`)
                super::push_caused_alert(
                    alerts,
                    AlertBody::DocumentReset(Box::new(before)),
                    super::AlertCause::Document,
                );
            },
            "Réinitialiser"
        }
    }
}

// The reserved status region (ALR-6): always present, same place, one
// line tall forever — the undo/redo controls, then the solver status. The
// buttons come first so the status's appearance and disappearance never has
// anything of its own to displace (LAY-2). The alerts float apart in
// `Toasts`, so this strip never pushes the panel or the grid down.
#[component]
pub fn StatusStrip() -> Element {
    let mut plan = use_context::<Signal<Plan>>();
    let mut history = use_context::<Signal<History>>();
    // the wording, dark state included, comes from the pure module (AP-5)
    let undo_label = crate::present::undo_title(history.read().undo_label());
    let redo_label = crate::present::redo_title(history.read().redo_label());
    let can_undo = history.read().undo_label().is_some();
    let can_redo = history.read().redo_label().is_some();
    let print_target = use_context::<super::PrintTarget>().0;
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let alerts = use_context::<Signal<AlertStack>>();
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    let mut export_open = use_signal(|| false);
    // bumped by every focusin inside the menu, so a deferred close can
    // tell « le focus a quitté le menu » from « le focus s'est déplacé
    // dans le menu »
    let mut focus_inside = use_signal(|| 0u64);
    // the menu's rows, wording included, come from the pure module (AP-5)
    let export_entries = crate::export::menu::entries();
    // le menu dit lui-même qu'un export lancé pendant un recalcul fige un
    // état provisoire (ADR
    // `2026-08-le-debut-n-herite-pas-d-un-placement-hors-saison`)
    let solver = use_context::<Signal<SolverState>>();
    // la temporisation compte : un export lancé dans les 500 ms qui suivent
    // une modification fige lui aussi un état que le solveur va réécrire
    let export_pending = crate::export::menu::pending_note(
        solver.read().awaited_since.is_some(),
    );

    // note 9 : le lien emporte tout l'organigramme, pas la session
    // affichée — il se range donc avec « Exporter », l'autre geste qui sort
    // le document entier (Antoine, 2026-08-30), et non plus dans la bande
    // du haut. Rien à l'écran ne bougeait quand il partait (rapports
    // Camille et Élodie, 2026-08-29) : il confirme désormais, et dit
    // honnêtement quand le navigateur refuse le presse-papiers (ADR
    // `2026-08-partager-confirme-ou-dit-son-echec`).
    let share = move |_| {
        let (url, payload) = {
            let plan_read = plan.read();
            let manual_read = manual.read();
            let payload =
                crate::persist::encode_organigramme(&plan_read, &manual_read);
            (crate::browser::share_url(&payload), payload)
        };
        // the link also lands in the address bar — copyable there even if
        // the clipboard was blocked, without flooding the status strip
        crate::browser::set_fragment(&payload);
        spawn(async move {
            let copied = crate::browser::clipboard_write(&url).await;
            let note = crate::present::share_note(copied);
            // a ✓ that clears itself after five seconds is fine for a
            // confirmation nobody has to act on; a refusal is a standing
            // instruction (ALR-4), so it stays until dismissed
            let body = if copied {
                AlertBody::Success(note)
            } else {
                AlertBody::Note(note)
            };
            super::push_alert(alerts, body);
        });
    };

    // The two print paths are unchanged; the JSON one writes the very file
    // « Charger depuis JSON » reads back (ADR
    // `2026-08-un-cheminement-par-fichier`). Nothing here decides anything:
    // the document is built by `cheminement::export`, the sentence by
    // `export::download_note`, both tested natively.
    let run_export = move |choice: crate::export::menu::ExportChoice| {
        match choice {
            crate::export::menu::ExportChoice::OrganigrammePdf => {
                super::print::start_print(
                    print_target,
                    super::print::PrintKind::Organigramme,
                );
            }
            crate::export::menu::ExportChoice::HorairePdf => {
                super::print::start_print(
                    print_target,
                    super::print::PrintKind::Horaire,
                );
            }
            crate::export::menu::ExportChoice::OrganigrammeJson => {
                let generated_at = crate::browser::now_iso();
                let scraped_at =
                    snapshot.peek().as_ref().and_then(|snapshot| {
                        snapshot.provenance.scraped_at.clone()
                    });
                let provenance = crate::export::provenance::export_provenance(
                    &generated_at,
                    scraped_at.as_deref(),
                );
                let document = plan.peek().clone();
                let file_name =
                    crate::cheminement::export_file_name(&document);
                let body = crate::cheminement::export(
                    &document,
                    &generated_at,
                    &provenance,
                );
                let taken = crate::browser::download_text(
                    &file_name,
                    "application/json",
                    &body,
                );
                let note =
                    crate::export::menu::download_note(&file_name, taken);
                // a refused download is a note that stays, never a ✓ that
                // clears itself after five seconds (TRU-1, ALR-4)
                let alert = if taken {
                    AlertBody::Success(note)
                } else {
                    AlertBody::Note(note)
                };
                super::push_alert(alerts, alert);
            }
        }
    };

    rsx! {
        div { class: "status-strip", role: "status",
            button {
                class: "status-undo",
                disabled: !can_undo,
                title: "{undo_label}",
                onclick: move |_| {
                    let mut plan = plan.write();
                    let mut history = history.write();
                    state::undo(&mut plan, &mut history);
                },
                "↶ Annuler"
                kbd { "Ctrl+Z" }
            }
            button {
                class: "status-undo",
                disabled: !can_redo,
                title: "{redo_label}",
                onclick: move |_| {
                    let mut plan = plan.write();
                    let mut history = history.write();
                    state::redo(&mut plan, &mut history);
                },
                "↷ Rétablir"
                kbd { "Ctrl+Y" }
            }
            SolverStatus {}
            // le partage et les exports se rangent à droite de la bande,
            // figés avec elle : la grille défile sous eux sans les emporter
            div {
                class: "status-exports",
                // Échap referme, comme partout ailleurs. Il s'ouvre au
                // clic, jamais au survol (INP-5).
                onkeydown: move |event| {
                    if event.key() == Key::Escape {
                        export_open.set(false);
                    }
                },
                onfocusin: move |_| {
                    *focus_inside.write() += 1;
                },
                // Refermer sur-le-champ mangerait le clic : cliquer une
                // entrée retire d'abord le focus du bouton, ce qui
                // démonterait l'entrée avant que son `click` ne parte — le
                // menu se refermait sans rien faire. Un macrotask plus
                // tard, le clic a été distribué. Et si le focus n'a fait
                // que passer à une entrée (Tab), le compteur a bougé et le
                // menu reste ouvert (INP-4).
                onfocusout: move |_| {
                    let seen = focus_inside();
                    spawn(async move {
                        crate::browser::next_frame().await;
                        if *focus_inside.peek() == seen {
                            export_open.set(false);
                        }
                    });
                },
                button {
                    class: "grid-share",
                    r#type: "button",
                    title: "Copier un lien qui rouvre tout l'organigramme",
                    onclick: share,
                    "Partager"
                }
                button {
                    class: "grid-share",
                    r#type: "button",
                    aria_haspopup: "menu",
                    aria_expanded: export_open(),
                    title: "PDF ou JSON pour l'organigramme, PDF pour \
                            l'horaire",
                    onclick: move |_| {
                        let open = !export_open();
                        export_open.set(open);
                    },
                    "Exporter ▾"
                }
                if export_open() {
                    div { class: "status-export-menu", role: "menu",
                        if let Some(pending) = export_pending {
                            p { class: "status-export-pending", "{pending}" }
                        }
                        for entry in export_entries.iter().cloned() {
                            // one keyed root per row (AP-8): the group
                            // heading rides inside it, so the key still
                            // sits on the first node of the block
                            div {
                                key: "{entry.key}",
                                class: "status-export-row",
                            if let Some(group) = entry.group {
                                p {
                                    class: "status-export-group",
                                    "{group}"
                                }
                            }
                            button {
                                class: "status-export-item",
                                r#type: "button",
                                role: "menuitem",
                                onclick: move |_| {
                                    export_open.set(false);
                                    run_export(entry.choice);
                                },
                                span { class: "status-export-label",
                                    "{entry.label}"
                                }
                            }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Les alertes flottent en coin bas-droite, par-dessus la grille — le
// panneau et l'horaire ne bougent jamais (ADR
// `2026-08-alertes-en-toasts-flottants`). Les ⚠ persistent jusqu'au clic
// (ALR-4) ; seuls les ✓ s'auto-effacent ; au-delà de TOASTS_VISIBLE la
// pile se résume en « +N autres » et se déplie sur demande (ALR-3).
const TOASTS_VISIBLE: usize = 3;
const SUCCESS_TOAST_MS: u32 = 5_000;

#[component]
pub fn Toasts() -> Element {
    let mut alerts = use_context::<Signal<AlertStack>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let super::LocalPrograms(local_programs) =
        use_context::<super::LocalPrograms>();
    // « Réinitialiser » is undone from its own toast, so the stack needs
    // the two signals its undo writes
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let mut expanded = use_signal(|| false);
    // chaque ✓ n'arme qu'une seule minuterie (peek : l'effet ne dépend
    // que de la liste, jamais de sa propre comptabilité)
    let mut timed = use_signal(std::collections::BTreeSet::<u64>::new);
    use_effect(move || {
        let successes: Vec<u64> = alerts
            .read()
            .alerts()
            .iter()
            .filter(|alert| matches!(alert.body, AlertBody::Success(_)))
            .map(|alert| alert.key)
            .collect();
        for key in successes {
            if timed.peek().contains(&key) {
                continue;
            }
            timed.write().insert(key);
            let mut alerts = alerts;
            spawn(async move {
                crate::browser::sleep_ms(SUCCESS_TOAST_MS).await;
                // an auto-clear is not a rejection: nothing is memorized
                alerts.write().retire(|kept| kept.key == key);
            });
        }
    });
    let all = alerts.read().alerts().to_vec();
    if all.is_empty() {
        return rsx! {};
    }
    let hidden = all.len().saturating_sub(TOASTS_VISIBLE);
    let show_all = expanded() || hidden == 0;
    let first_visible = if show_all {
        0
    } else {
        all.len() - TOASTS_VISIBLE
    };
    rsx! {
        div { class: "toasts", role: "status",
            if !show_all {
                button {
                    class: "toast toast--more",
                    onclick: move |_| expanded.set(true),
                    if hidden == 1 {
                        "+1 autre message - tout afficher"
                    } else {
                        "+{hidden} autres messages - tout afficher"
                    }
                }
            }
            for alert in all[first_visible..].iter().cloned() {
                div {
                    key: "{alert.key}",
                    class: "toast",
                    class: if matches!(
                        alert.body,
                        AlertBody::Success(_)
                            | AlertBody::Standing(_)
                            | AlertBody::LocalProgramRemoved(_)
                            | AlertBody::DocumentReset(_)
                    ) {
                        "toast--success"
                    },
                    // note 12: the whole message is its own dismiss —
                    // any link inside would stop the propagation. A real
                    // rejection: its subject stays silent until its
                    // wording changes (ADR
                    // `2026-08-toasts-un-par-sujet-et-rejet-memorise`)
                    onclick: {
                        let key = alert.key;
                        move |_| {
                            alerts.write().dismiss(key);
                        }
                    },
                    match &alert.body {
                        AlertBody::Note(note) => rsx! {
                            span { "⚠ {note}" }
                        },
                        AlertBody::Success(note)
                        | AlertBody::Standing(note) => rsx! {
                            span { class: "status-alert-ok", "✓ {note}" }
                        },
                        // ERR-1 : les cinq parties, toutes en français ;
                        // ERR-3 : le texte technique (anglais, celui du
                        // solveur ou du navigateur) reste derrière le
                        // repli, jamais dans le message principal
                        AlertBody::Error(error) => rsx! {
                            div { class: "toast-error",
                                span { class: "status-alert-error",
                                    "⚠ {error.what}"
                                }
                                span { "{error.reaction}" }
                                span { "{error.affected}" }
                                span { class: "toast-error-action",
                                    "{error.action}"
                                }
                                details {
                                    class: "toast-detail",
                                    // le message entier est son propre
                                    // rejet (note 12) : déplier ne doit
                                    // pas le fermer
                                    onclick: move |event: Event<MouseData>| {
                                        event.stop_propagation();
                                    },
                                    summary { "Détail technique" }
                                    pre { "{error.id} — {error.detail}" }
                                }
                            }
                        },
                        // ACT-2: the destructive act carries its own way
                        // back, right where the eye already is — the
                        // status strip's « Annuler » says nothing about
                        // what just happened
                        AlertBody::DocumentReset(before) => {
                            let before = (**before).clone();
                            let note = crate::present::reset_note(
                                before.program.as_ref().map(|choice| {
                                    (
                                        choice.code.as_str(),
                                        choice.semester.as_str(),
                                    )
                                }),
                            );
                            let key = alert.key;
                            rsx! {
                                span { class: "status-alert-ok", "✓ {note}" }
                                button {
                                    class: "toast-undo",
                                    onclick: move |event: Event<MouseData>| {
                                        // the toast is its own dismiss
                                        // (note 12) — it must not fire
                                        // before the undo has read `before`
                                        event.stop_propagation();
                                        super::restore_document(
                                            plan,
                                            history,
                                            before.clone(),
                                        );
                                        // consumed, not rejected
                                        alerts
                                            .write()
                                            .retire(|kept| kept.key == key);
                                    },
                                    "↶ Annuler"
                                }
                            }
                        }
                        AlertBody::LocalProgramRemoved(local) => {
                            let local = (**local).clone();
                            let key = alert.key;
                            rsx! {
                                span { class: "status-alert-ok",
                                    "✓ Programme supprimé — "
                                }
                                button {
                                    class: "toast-undo",
                                    onclick: move |event: Event<MouseData>| {
                                        // the toast is its own dismiss
                                        // (note 12) — undo must not also
                                        // trigger it before it has had a
                                        // chance to read `local`
                                        event.stop_propagation();
                                        super::restore_local_program(
                                            snapshot,
                                            local_programs,
                                            alerts,
                                            local.clone(),
                                        );
                                        // the undo consumed the toast —
                                        // a retirement, not a rejection
                                        alerts
                                            .write()
                                            .retire(|kept| kept.key == key);
                                    },
                                    "↶ Annuler"
                                }
                            }
                        }
                    }
                    button {
                        class: "status-dismiss",
                        aria_label: "Rejeter ce message",
                        onclick: move |_| {
                            alerts.write().dismiss(alert.key);
                        },
                        "✕"
                    }
                }
            }
        }
    }
}
