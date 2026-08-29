use crate::capsule::CapsuleError;
use crate::cheminement::CheminementError;
use crate::data::{fnv1a_64, DataError};
use crate::import::ImportError;

// ERR-1: every user-facing error states five things, in French — what
// happened, what the app did about it, what is affected, what to do now,
// and a copyable id. `detail` is the technical text one click away (ERR-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiError {
    pub what: String,
    pub reaction: String,
    pub affected: String,
    pub action: String,
    pub id: String,
    pub detail: String,
}

pub fn present_data_error(error: &DataError) -> UiError {
    let what = match error {
        DataError::Fetch { file, .. } => {
            format!(
                "Le fichier de données « {file} » n'a pas pu être téléchargé."
            )
        }
        DataError::Parse { file, .. } => {
            format!("Le fichier de données « {file} » est illisible.")
        }
    };
    UiError {
        what,
        reaction: "L'application ne démarre pas tant que le catalogue \
                   n'est pas chargé; rien n'a été perdu."
            .to_string(),
        affected: "Tout l'affichage — aucun cours ni programme n'est \
                   disponible."
            .to_string(),
        action: "Vérifiez votre connexion puis rechargez la page; si \
                 l'erreur persiste, signalez-la avec l'identifiant \
                 ci-dessous."
            .to_string(),
        id: error_id(&error.to_string()),
        detail: error.to_string(),
    }
}

// The import is a non-critical path (BLD-1): whatever goes wrong, the rest
// of the app — and any program already imported, sitting in localStorage —
// keeps working. `Proxy` and `NotHtml` name corsproxy.io explicitly, since
// a third party sits in that request (TRU).
pub fn present_import_error(error: &ImportError) -> UiError {
    let (what, action) = match error {
        ImportError::InvalidUrl { .. } => (
            "L'adresse collée n'est pas une page de programme de \
             ulaval.ca."
                .to_string(),
            "Copiez l'adresse complète d'une page de programme (par \
             exemple https://www.ulaval.ca/etudes/programmes/…) puis \
             réessayez."
                .to_string(),
        ),
        ImportError::Proxy { .. } => (
            "Le service intermédiaire corsproxy.io, utilisé pour \
             récupérer la page, n'a pas répondu correctement."
                .to_string(),
            "Réessayez dans quelques minutes; si l'erreur persiste, \
             signalez-la avec l'identifiant ci-dessous."
                .to_string(),
        ),
        ImportError::NotFound { .. } => (
            "Cette page de programme n'existe pas sur ulaval.ca.".to_string(),
            "Vérifiez l'adresse collée, ou retrouvez le programme sur \
             ulaval.ca et recollez son adresse."
                .to_string(),
        ),
        ImportError::NotHtml { .. } => (
            "Le service intermédiaire corsproxy.io a renvoyé un \
             contenu qui n'est pas une page web."
                .to_string(),
            "Réessayez; si l'erreur persiste, signalez-la avec \
             l'identifiant ci-dessous."
                .to_string(),
        ),
        ImportError::Parse { .. } => (
            "La page de programme n'a pas pu être analysée.".to_string(),
            "Signalez l'erreur avec l'identifiant ci-dessous; le \
             programme n'a pas été ajouté."
                .to_string(),
        ),
        ImportError::Preparatory { .. } => (
            "Le calcul de la scolarité préparatoire de ce programme a \
             échoué."
                .to_string(),
            "Signalez l'erreur avec l'identifiant ci-dessous; le \
             programme n'a pas été ajouté."
                .to_string(),
        ),
        ImportError::Language { .. } => (
            "La règle d'exigence linguistique de ce programme n'a pas pu \
             être élargie aux cours de langue."
                .to_string(),
            "Signalez l'erreur avec l'identifiant ci-dessous; le \
             programme n'a pas été ajouté."
                .to_string(),
        ),
        ImportError::Cancelled => (
            "L'import a été annulé.".to_string(),
            "Recollez l'adresse pour réessayer quand vous le \
             souhaitez."
                .to_string(),
        ),
        ImportError::BrowserApi { .. } => (
            "Le navigateur n'a pas pu préparer cette requête.".to_string(),
            "Réessayez; si l'erreur persiste, un réglage du navigateur \
             (mode privé restrictif, extension bloquant les requêtes) \
             peut en être la cause."
                .to_string(),
        ),
        ImportError::CatalogueUnavailable => (
            "Le catalogue des cours n'est pas encore chargé.".to_string(),
            "Attendez que le catalogue termine de charger puis \
             réessayez."
                .to_string(),
        ),
        ImportError::InvalidProgramJson { .. } => (
            "Ce fichier n'est pas un instantané de programme valide."
                .to_string(),
            "Choisissez un fichier « {code}-{semestre}.json » produit par \
             le scraper, puis réessayez."
                .to_string(),
        ),
    };
    UiError {
        what,
        reaction: "Rien n'a été perdu; le reste de l'application \
                   continue de fonctionner normalement."
            .to_string(),
        affected: "L'import de ce programme seulement.".to_string(),
        action,
        id: error_id(&error.to_string()),
        detail: error.to_string(),
    }
}

// A program imported by URL that collides with an entry already in the
// catalogue — either shipped with the app, or already imported locally
// (a repeated import click): `data::add_local_program` refuses it either
// way (plan item 6) and the click must not be lost in silence — the
// student is told plainly and pointed at the entry that already covers it
// (ERR-1).
pub fn present_local_program_conflict(detail: &str) -> UiError {
    UiError {
        what: detail.to_string(),
        reaction: "Rien n'a été perdu; le reste de l'application continue \
                   de fonctionner normalement."
            .to_string(),
        affected: "L'import de ce programme seulement.".to_string(),
        action: "Choisissez directement ce programme dans la liste \
                 ci-dessus."
            .to_string(),
        id: error_id(detail),
        detail: detail.to_string(),
    }
}

// A relevé Capsule pasted into the app that the parser could not read at
// all, or that read cleanly but named no Université Laval session to
// anchor a plan on (ADR `2026-08-import-de-releve-capsule`). The load is a
// non-critical path, same as an import (BLD-1): the plan open before the
// paste is untouched either way.
pub fn present_capsule_error(error: &CapsuleError) -> UiError {
    let (what, action) = match error {
        CapsuleError::NotATranscript { .. } => (
            "Ce texte n'est pas un relevé de notes Capsule.".to_string(),
            "Depuis la page « Relevé de notes non officiel », faites \
             ctrl-u pour afficher la source de la page, puis ctrl-a et \
             ctrl-c pour tout copier, puis recollez le résultat ici."
                .to_string(),
        ),
        CapsuleError::Empty => (
            "Le relevé ne contient aucune session à l'Université Laval."
                .to_string(),
            "Vérifiez que le texte collé provient bien de la page « \
             Relevé de notes non officiel » de Capsule, puis réessayez."
                .to_string(),
        ),
        CapsuleError::CatalogueUnavailable => (
            "Le catalogue des cours n'est pas encore chargé.".to_string(),
            "Attendez que les données de l'application finissent de \
             charger, puis réessayez."
                .to_string(),
        ),
    };
    UiError {
        what,
        reaction: "Rien n'a été perdu; le reste de l'application continue \
                   de fonctionner normalement."
            .to_string(),
        affected: "Le chargement de ce relevé seulement.".to_string(),
        action,
        id: error_id(&error.to_string()),
        detail: error.to_string(),
    }
}

// A cheminement file the student chose that the app would not read. The
// load is a non-critical path (BLD-1) and refuses whole: the organigramme
// open before the click is untouched in every branch, which is what the
// « rien n'a bougé » of each action can honestly claim (ADR
// `2026-08-un-cheminement-par-fichier`).
pub fn present_cheminement_error(error: &CheminementError) -> UiError {
    let (what, action) = match error {
        CheminementError::Unreadable { detail } => (
            "Ce fichier n'est pas un cheminement lisible.".to_string(),
            format!(
                "Le lecteur s'est arrêté sur : {detail}. Le format attendu \
                 est celui du bouton « ? » à côté : « completed » et « \
                 sessions », chaque session portant « semester » (A26, H27, \
                 E27) et « courses »."
            ),
        ),
        CheminementError::NoAdmission => (
            "Ce cheminement ne nomme aucune session d'admission.".to_string(),
            "La première entrée de « sessions » doit être un automne ou un \
             hiver — c'est elle qui donne sa session de départ à \
             l'organigramme."
                .to_string(),
        ),
        CheminementError::TooLong {
            study_sessions,
            max,
        } => (
            format!(
                "Ce cheminement compte {study_sessions} sessions d'études, \
                 et l'outil en accepte {max} au plus."
            ),
            "Retirez les sessions en trop du fichier, ou répartissez le \
             cheminement sur deux fichiers."
                .to_string(),
        ),
        CheminementError::Empty => (
            "Aucun cours de ce cheminement n'existe au catalogue.".to_string(),
            "Les grilles officielles emploient des jetons — OPT-ION1, \
             AUC-HOIX, LAN-GUES — qui ne sont pas des cours : remplacez-les \
             par les sigles réellement choisis, puis rechargez le fichier."
                .to_string(),
        ),
        CheminementError::CatalogueUnavailable => (
            "Le catalogue des cours n'est pas encore chargé.".to_string(),
            "Attendez que les données de l'application finissent de \
             charger, puis réessayez."
                .to_string(),
        ),
    };
    UiError {
        what,
        reaction: "L'organigramme n'a pas bougé; le reste de l'application \
                   continue de fonctionner normalement."
            .to_string(),
        affected: "Le chargement de ce fichier seulement.".to_string(),
        action,
        id: error_id(&error.to_string()),
        detail: error.to_string(),
    }
}

// deterministic (fnv of the message): the same failure always carries the
// same id, so two reports of it can be recognized as one
// A correction the catalogue could not honour, said in French. Every case
// names the course and what the student can do about it — a correction that
// quietly did nothing would be worse than none at all.
pub fn present_override_note(
    note: &ulaval_scheduler_core::OverrideNote,
) -> String {
    use ulaval_scheduler_core::OverrideNote;
    match note {
        OverrideNote::Unparsed { code, error } => format!(
            "Préalables de {code} : la correction n'a pas pu être lue \
             ({error}); ceux du répertoire s'appliquent toujours."
        ),
        OverrideNote::UnknownCode { code } => format!(
            "Préalables de {code} : ce cours n'est pas au catalogue, la \
             correction ne s'applique à rien."
        ),
        OverrideNote::OfficialChanged { code, was, now } => format!(
            "Préalables de {code} : le répertoire a changé depuis votre \
             correction (« {was} » est devenu « {now} »). Votre version \
             reste appliquée."
        ),
    }
}

// What a correction being typed would mean, echoed before it is committed
// (INP-6). `valid` alone would be colour-shaped feedback, so `echo` always
// carries the same verdict in words (INP-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrereqDraft {
    pub valid: bool,
    pub echo: String,
}

// The parser's guards name themselves in English, like the rest of the
// code; on screen they are read by a student, in French. The fallback is
// the label itself rather than a vague « expression invalide »: a guard
// added later must show up as something, never be swallowed.
fn prereq_fault(label: &str) -> String {
    match label {
        "two operands in a row" => {
            "deux termes se suivent sans ET ni OU entre eux"
        }
        "( where an operator was expected" => {
            "une parenthèse ouvre là où ET ou OU était attendu"
        }
        ") without a left operand" => {
            "une parenthèse se ferme sans terme devant elle"
        }
        "unmatched )" => "une parenthèse fermante n'a pas d'ouvrante",
        "ET without a left operand" => "ET n'a pas de terme à sa gauche",
        "OU without a left operand" => "OU n'a pas de terme à sa gauche",
        "expression ends on an operator" => {
            "l'expression se termine sur un opérateur"
        }
        "unclosed (" => "une parenthèse reste ouverte",
        other => other,
    }
    .to_string()
}

// the same ceiling the solver's own flattening uses — a pathological
// expression must not walk forever
const MAX_DRAFT_NODES: usize = 10_000;

pub fn present_prereq_draft(text: &str) -> PrereqDraft {
    use ulaval_scheduler_core::parse_prereq_tree;

    let text = text.trim();
    if text.is_empty() {
        return PrereqDraft {
            valid: true,
            echo: "compris : ce cours n'a aucun préalable.".to_string(),
        };
    }
    let tree = match parse_prereq_tree(text) {
        Ok(tree) => tree,
        Err(error) => {
            return PrereqDraft {
                valid: false,
                echo: format!(
                    "non lu : {} - la correction n'est pas appliquée.",
                    prereq_fault(&error.error)
                ),
            };
        }
    };
    // Two readings surprise the student, and both are invisible in the
    // expression he typed: an operand no catalogue can check (an
    // examination, a range of course numbers) is presumed rather than
    // verified, and a sigle he starred is satisfied by his own session —
    // the `*` is the répertoire's shorthand, not something to be left to
    // guess (TRU-1: what the app knows, it says).
    let (concomitant, presumed) = notable_operands(&tree);
    let mut notes: Vec<String> = Vec::new();
    if !concomitant.is_empty() {
        notes.push(format!(
            "{} peut être suivi la même session (concomitance permise)",
            concomitant.join(", ")
        ));
    }
    if !presumed.is_empty() {
        notes.push(format!(
            "{} sera présumé acquis, le solveur ne peut pas le vérifier",
            presumed.join(", ")
        ));
    }
    let echo = if notes.is_empty() {
        "compris.".to_string()
    } else {
        format!("compris - {}.", notes.join("; "))
    };
    PrereqDraft { valid: true, echo }
}

// the starred sigles and the operands presumed acquis, in that order —
// a bounded walk, never a recursion: an arbitrarily deep expression is a
// student's typing, not a trusted input
fn notable_operands(
    tree: &ulaval_scheduler_core::PrereqTree,
) -> (Vec<String>, Vec<String>) {
    use ulaval_scheduler_core::PrereqTree;
    let mut concomitant = Vec::new();
    let mut presumed = Vec::new();
    let mut stack = vec![tree];
    for _ in 0..MAX_DRAFT_NODES {
        let Some(node) = stack.pop() else {
            break;
        };
        match node {
            PrereqTree::Concomitant { concomitant: code } => {
                concomitant.push(code.clone())
            }
            PrereqTree::Raw { raw } => presumed.push(format!("« {raw} »")),
            PrereqTree::All { all } => stack.extend(all.iter()),
            PrereqTree::Any { any } => stack.extend(any.iter()),
            PrereqTree::Course(_) | PrereqTree::ProgramCredits { .. } => {}
        }
    }
    (concomitant, presumed)
}

pub fn error_id(detail: &str) -> String {
    let hash = fnv1a_64(0xcbf2_9ce4_8422_2325, detail.as_bytes());
    format!("GH-{:08X}", (hash >> 32) as u32 ^ hash as u32)
}

// --- the bac credit total, explained ---------------------------------------

// F3: the header's « X/120 cr au bac » rarely matches summing the sessions
// by hand — stages ridden en-sus of the total and préparatoire (0xxx)
// credits are excluded from the count, and nothing on screen said so
// (rapport étudiante-gex 2026-08-27). `suffix` names the en-sus credits
// right next to the total they are *not* part of; `tooltip` decomposes
// the whole gap. `CreditSummary` does not distinguish a course selected
// but outside the program's own scope from one counted normally, so that
// case is never claimed here — only what the summary actually knows.
pub struct BacCreditNote {
    pub suffix: String,
    pub tooltip: String,
}

pub fn bac_credit_note(
    summary: &ulaval_scheduler_wasm::credits::CreditSummary,
) -> BacCreditNote {
    let suffix = if summary.in_addition > 0 {
        format!(" (+{} cr en sus)", summary.in_addition)
    } else {
        String::new()
    };
    let mut parts = Vec::new();
    if summary.in_addition > 0 {
        parts.push(format!(
            "{} cr de stages exigés mais ajoutés aux 120 cr, jamais \
             comptés dedans",
            summary.in_addition
        ));
    }
    if summary.preparatory > 0 {
        parts.push(format!(
            "{} cr de scolarité préparatoire, non comptés",
            summary.preparatory
        ));
    }
    let tooltip = if parts.is_empty() {
        "Le compte inclut tous les cours sélectionnés — aucun écart."
            .to_string()
    } else {
        parts.join(" ; ")
    };
    BacCreditNote { suffix, tooltip }
}

// F5 (constat d'Antoine 2026-08-27, B-GCI + concentration + profil rempli
// à 129/120) : the header said nothing when the tally passed the program's
// own total — a filet naming the overrun instead of a number that reads as
// fine. `text` composes with `BacCreditNote.suffix` rather than replacing
// it — an en-sus stage total must stay visible even past the cap.
pub struct BacCreditLabel {
    pub text: String,
    pub over: bool,
}

pub fn bac_credit_label(
    counted: u32,
    required: i64,
    note: &BacCreditNote,
) -> BacCreditLabel {
    let over = i64::from(counted) > required;
    let text = if over {
        format!(
            "⚠ {counted}/{required} cr au bac{} — au-delà des {required} cr \
             du programme",
            note.suffix
        )
    } else {
        format!("{counted}/{required} cr au bac{}", note.suffix)
    };
    BacCreditLabel { text, over }
}

// --- the weekly grid geometry ---------------------------------------------

use ulaval_scheduler_core::{Day, Section, Time};

use crate::data::Snapshot;
use crate::solve::WeeklySchedule;
use crate::state::{self, Plan};

// The whole teachable day, always shown — the axis never breathes with the
// data (notes 2026-08-13), so a block keeps its place when courses change.
// Data outside the frame still stretches it rather than being cut (TST-1).
pub const AXIS_START: u16 = 8 * 60 + 30;
pub const AXIS_END: u16 = 22 * 60 + 30;

#[derive(Debug, Clone, PartialEq)]
pub struct GridModel {
    // axis labels, one per hour — « 8:30 » … « 17:30 »
    pub hours: Vec<String>,
    pub start: u16,
    pub end: u16,
    pub days: Vec<DayColumn>,
    pub conflict: bool,
    // selected courses whose sections carry no weekly slot (à distance) —
    // listed under the grid, never interpolated (TRU-4)
    pub unplaced: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayColumn {
    pub label: &'static str,
    pub conflict: bool,
    pub blocks: Vec<Block>,
}

// One rendered block, all coordinates in percent of the day column; a
// ghost is an alternative option shown when its course is selected —
// clicking it pins `nrcs` (sémantique swap, ADR
// `2026-07-contrat-horaire-hebdomadaire-vers-ui`).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub code: String,
    pub title: String,
    pub detail: String,
    pub top: f32,
    pub height: f32,
    pub left: f32,
    pub width: f32,
    pub hue: f32,
    pub ghost: bool,
    pub valid: bool,
    // this very block overlaps another selected block: the hatch goes
    // here, not on every slot of an unlucky course (a lone Saturday slot
    // stays clean even when its course clashes elsewhere)
    pub clash: bool,
    // the option's identity — the sorted NRC set a click pins
    pub nrcs: Vec<String>,
    // how many alternative options selecting this block's course would
    // reveal as ghosts — 0 on a ghost itself, it never advertises its own
    // siblings (ADR 2026-08-jeton-de-plages-alternatives-sur-le-bloc)
    pub alternatives: usize,
    // full accessible name for a ghost's button — the visible `title`
    // stays the compact letter/NRC, but a screen reader needs the course
    // code back (régression relevée : rapport étudiante-gex 2026-08-29,
    // « seulement B, C au lieu de MAT-1900 - B »). Empty on a real block,
    // whose visible content already names the course in full.
    pub full_label: String,
}

pub fn grid_model(
    schedule: &WeeklySchedule,
    snapshot: &Snapshot,
    ghosts_for: Option<&str>,
) -> GridModel {
    let mut raw: Vec<(usize, RawBlock)> = Vec::new();
    let mut unplaced = Vec::new();
    // a hybrid option repeats the same slot in twin sections: one block
    // per (course, day, time), never a duplicate (TST-1)
    let mut seen: std::collections::BTreeSet<(String, usize, u16, u16)> =
        std::collections::BTreeSet::new();
    let mut codes: Vec<&str> = schedule
        .report
        .courses
        .iter()
        .map(|course| course.code.as_str())
        .collect();
    codes.sort_unstable();
    codes.dedup();
    for course in &schedule.report.courses {
        let hue = course_hue(&codes, &course.code);
        let title = course_title(snapshot, &course.code);
        let nrcs = option_nrcs(&course.selected);
        // same count a click on this block would reveal as ghosts below
        let alternatives = course.alternatives.len();
        let mut placed = false;
        for section in &course.selected {
            for slot in &section.slots {
                placed = true;
                if !seen.insert((
                    course.code.clone(),
                    day_index(slot.day),
                    minutes(slot.start),
                    minutes(slot.end),
                )) {
                    continue;
                }
                raw.push((
                    day_index(slot.day),
                    RawBlock {
                        start: minutes(slot.start),
                        end: minutes(slot.end),
                        block: Block {
                            code: course.code.clone(),
                            title: title.to_string(),
                            detail: section_detail(&course.code, section),
                            top: 0.0,
                            height: 0.0,
                            left: 0.0,
                            width: 100.0,
                            hue,
                            ghost: false,
                            valid: course.valid,
                            clash: false,
                            nrcs: nrcs.clone(),
                            alternatives,
                            full_label: String::new(),
                        },
                    },
                ));
            }
        }
        if !placed && !course.selected.is_empty() {
            unplaced.push(course.code.clone());
        }
        if ghosts_for == Some(course.code.as_str()) {
            // a hybrid option repeats the same slot across its twin
            // sections (in-person + remote at the same time): one ghost
            // block per (slot, option), never a duplicate — two distinct
            // options sharing a slot stay two clickable ghosts because
            // their nrcs differ (ADR
            // 2026-08-fantomes-dedupliques-et-libelles-compacts)
            let mut seen_ghosts: std::collections::BTreeSet<(
                usize,
                u16,
                u16,
                Vec<String>,
            )> = std::collections::BTreeSet::new();
            for alternative in &course.alternatives {
                let nrcs = option_nrcs(&alternative.sections);
                for section in &alternative.sections {
                    for slot in &section.slots {
                        let key = (
                            day_index(slot.day),
                            minutes(slot.start),
                            minutes(slot.end),
                            nrcs.clone(),
                        );
                        if !seen_ghosts.insert(key) {
                            continue;
                        }
                        raw.push((
                            day_index(slot.day),
                            RawBlock {
                                start: minutes(slot.start),
                                end: minutes(slot.end),
                                block: Block {
                                    code: course.code.clone(),
                                    // a ghost lane is narrow — the full
                                    // course title truncates to a couple
                                    // letters per line; the section letter
                                    // (or NRC) is enough to tell ghosts
                                    // apart and stays readable at that
                                    // width, never the course's own siblings
                                    // (comment above `alternatives: usize`)
                                    title: ghost_label(section),
                                    detail: String::new(),
                                    top: 0.0,
                                    height: 0.0,
                                    left: 0.0,
                                    width: 100.0,
                                    hue,
                                    ghost: true,
                                    valid: alternative.valid,
                                    clash: false,
                                    nrcs: nrcs.clone(),
                                    alternatives: 0,
                                    full_label: ghost_full_label(
                                        &course.code,
                                        section,
                                    ),
                                },
                            },
                        ));
                    }
                }
            }
        }
    }
    let (start, end) = axis_span(&raw);
    let days = build_days(raw, start, end);
    GridModel {
        hours: hour_labels(start, end),
        start,
        end,
        conflict: !schedule.report.valid,
        days,
        unplaced,
    }
}

struct RawBlock {
    start: u16,
    end: u16,
    block: Block,
}

fn course_title<'a>(snapshot: &'a Snapshot, code: &'a str) -> &'a str {
    snapshot
        .by_code
        .get(code)
        .map(|&index| snapshot.courses[index].title.as_str())
        .unwrap_or(code)
}

// one hue per course actually on this schedule, by alphabetical rank among
// its distinct codes (ADR 2026-08-couleur-par-cours-plutot-que-matiere) —
// `codes` is built from this same `schedule.report.courses` right above, so
// `code` is always found; the `expect` documents that rather than adding an
// unreachable `None` branch
pub(crate) fn course_hue(codes: &[&str], code: &str) -> f32 {
    let rank = codes
        .binary_search(&code)
        .expect("code comes from the same schedule the list was built from");
    rank as f32 / codes.len() as f32 * 360.0
}

// « GCI-1007 - A », « GEX-4008 - Z1 - à distance » — the section letter
// when the page gave one, the mode when it is not the in-person default
// (the page carries no Cours/Labo/TD type — nothing is invented)
fn section_detail(code: &str, section: &Section) -> String {
    let mut parts = vec![code.to_string()];
    if let Some(letter) = &section.section {
        parts.push(letter.clone());
    }
    match section.mode {
        ulaval_scheduler_core::Mode::InPerson => {}
        ulaval_scheduler_core::Mode::Remote => {
            parts.push("à distance".to_string())
        }
        ulaval_scheduler_core::Mode::Hybrid => {
            parts.push("hybride".to_string())
        }
    }
    parts.join(" - ")
}

// compact ghost label — the section letter when the page gave one, else
// the NRC, plus the same mode words as `section_detail` above (a ghost
// lane is too narrow for the full course title, unlike a real block)
fn ghost_label(section: &Section) -> String {
    let mut parts = vec![section
        .section
        .clone()
        .unwrap_or_else(|| section.nrc.clone())];
    match section.mode {
        ulaval_scheduler_core::Mode::InPerson => {}
        ulaval_scheduler_core::Mode::Remote => {
            parts.push("à distance".to_string())
        }
        ulaval_scheduler_core::Mode::Hybrid => {
            parts.push("hybride".to_string())
        }
    }
    parts.join(" - ")
}

// « MAT-1900 - B », « GEX-4008 - Z1 - à distance » — the full identity a
// ghost's compact visible label (`ghost_label`) leaves out, for the
// button's `aria_label` only (accessible name, régression du 2026-08-29)
fn ghost_full_label(code: &str, section: &Section) -> String {
    format!("{code} - {}", ghost_label(section))
}

fn option_nrcs(sections: &[Section]) -> Vec<String> {
    let mut nrcs: Vec<String> =
        sections.iter().map(|section| section.nrc.clone()).collect();
    nrcs.sort();
    nrcs
}

fn minutes(time: Time) -> u16 {
    u16::from(time.hour) * 60 + u16::from(time.minute)
}

// the design frame, stretched (rounded to the half-hour) by any block
// outside it — data wins over the frame, the frame never cuts
fn axis_span(raw: &[(usize, RawBlock)]) -> (u16, u16) {
    let lowest = raw
        .iter()
        .map(|(_, block)| block.start)
        .min()
        .unwrap_or(AXIS_START);
    let highest = raw
        .iter()
        .map(|(_, block)| block.end)
        .max()
        .unwrap_or(AXIS_END);
    let start = AXIS_START.min(lowest - lowest % 30);
    let end = AXIS_END.max(highest + (30 - highest % 30) % 30);
    (start, end)
}

fn hour_labels(start: u16, end: u16) -> Vec<String> {
    (0..=(end - start) / 60)
        .map(|hour| {
            let minute = start + hour * 60;
            format!("{}:{:02}", minute / 60, minute % 60)
        })
        .collect()
}

const DAY_LABELS: [&str; 7] = [
    "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
];

fn day_index(day: Day) -> usize {
    match day {
        Day::Monday => 0,
        Day::Tuesday => 1,
        Day::Wednesday => 2,
        Day::Thursday => 3,
        Day::Friday => 4,
        Day::Saturday => 5,
        Day::Sunday => 6,
    }
}

// Monday→Friday always (spatial constancy, LAY-1); a weekend column only
// when a slot actually lands there
fn build_days(
    raw: Vec<(usize, RawBlock)>,
    start: u16,
    end: u16,
) -> Vec<DayColumn> {
    let span = f32::from(end - start);
    let weekend_used = raw.iter().map(|&(day, _)| day).max().unwrap_or(0);
    let day_count = 5usize.max(weekend_used + 1);
    let mut days: Vec<DayColumn> = DAY_LABELS[..day_count]
        .iter()
        .map(|&label| DayColumn {
            label,
            conflict: false,
            blocks: Vec::new(),
        })
        .collect();
    let mut per_day: Vec<Vec<RawBlock>> =
        (0..day_count).map(|_| Vec::new()).collect();
    for (day, block) in raw {
        per_day[day].push(block);
    }
    for (day, mut blocks) in per_day.into_iter().enumerate() {
        assign_lanes(&mut blocks);
        // the hatch marks actual overlap between selected blocks — the
        // report's per-course verdict stays on the status line
        let clashes: Vec<bool> = blocks
            .iter()
            .map(|one| {
                !one.block.ghost
                    && blocks.iter().any(|other| {
                        !other.block.ghost
                            && other.block.code != one.block.code
                            && one.start < other.end
                            && other.start < one.end
                    })
            })
            .collect();
        for (raw_block, clash) in blocks.into_iter().zip(clashes) {
            let mut block = raw_block.block;
            block.clash = clash;
            block.top = f32::from(raw_block.start - start) / span * 100.0;
            block.height =
                f32::from(raw_block.end - raw_block.start) / span * 100.0;
            days[day].conflict |= clash;
            days[day].blocks.push(block);
        }
    }
    days
}

// Side-by-side lanes for overlapping blocks (the design's conflict view):
// blocks sorted by start form clusters of transitive overlap; within a
// cluster each block takes the first lane free at its start, and every
// member is widened to the cluster's lane count.
fn assign_lanes(blocks: &mut [RawBlock]) {
    blocks.sort_by_key(|block| (block.start, block.end));
    let mut cluster_from = 0;
    let mut cluster_end = 0u16;
    let mut lanes: Vec<u16> = Vec::new();
    let mut assigned: Vec<usize> = vec![0; blocks.len()];
    for i in 0..blocks.len() {
        if i > 0 && blocks[i].start >= cluster_end {
            close_cluster(blocks, &assigned, cluster_from..i, lanes.len());
            cluster_from = i;
            lanes.clear();
        }
        let start = blocks[i].start;
        let lane = lanes
            .iter()
            .position(|&end| end <= start)
            .unwrap_or(lanes.len());
        if lane == lanes.len() {
            lanes.push(0);
        }
        lanes[lane] = blocks[i].end;
        assigned[i] = lane;
        cluster_end = cluster_end.max(blocks[i].end);
    }
    let len = blocks.len();
    close_cluster(blocks, &assigned, cluster_from..len, lanes.len());
}

fn close_cluster(
    blocks: &mut [RawBlock],
    assigned: &[usize],
    range: std::ops::Range<usize>,
    lane_count: usize,
) {
    let width = 100.0 / lane_count.max(1) as f32;
    for i in range {
        blocks[i].block.left = assigned[i] as f32 * width;
        blocks[i].block.width = width;
    }
}

// --- the schedule's status line -------------------------------------------

// never colour alone: the glyph and the wording carry the state (INP-3)
pub fn schedule_status(schedule: &WeeklySchedule, forced: bool) -> String {
    if schedule.report.courses.is_empty() {
        "aucun cours avec horaire dans cette session".to_string()
    } else if schedule.report.valid {
        if forced {
            // the student pinned at least one section by hand — the word
            // « automatique » would lie (rapport étudiante 2026-08-13)
            "sections forcées - sans conflit ✓".to_string()
        } else {
            "combinaison automatique - sans conflit ✓".to_string()
        }
    } else {
        "⚠ conflit d'horaire — plages en cause hachurées".to_string()
    }
}

// pendant qu'une recherche tourne, l'horaire affiché peut n'être qu'une
// étape transitoire (rapport directeur-gci 2026-08-29 : un décalage de
// session ou une violation apparente de préalable, corrigés quelques
// secondes plus tard) — le statut le dit avant tout le reste, dans la même
// ligne déjà réservée (`.grid-status`, `white-space: nowrap` + ellipsis :
// pas de nouvelle hauteur, LAY-2).
pub fn grid_status_label(status: &str, searching: bool) -> String {
    if searching {
        format!("⟳ recalcul en cours… — {status}")
    } else {
        status.to_string()
    }
}

// Jamais un « ✓ » à côté d'un état provisoire (rapport directeur-gci
// 2026-08-29, scénarios « départ hiver » et « double échec ») : tant
// qu'une recherche tourne, le dernier verdict vérifié ne s'applique plus
// à ce qui est affiché — le panneau le dit au lieu de laisser le ✓
// figé pendant le recalcul. Renvoie la classe CSS et le texte ensemble :
// les deux changent de pair, jamais l'un sans l'autre.
pub fn verification_verdict(searching: bool) -> (&'static str, String) {
    if searching {
        (
            "panel-verdict panel-verdict--pending",
            "⟳ recalcul en cours… (le verdict précédent ne s'applique \
             plus)"
                .to_string(),
        )
    } else {
        (
            "panel-verdict panel-verdict--ok",
            "Placement vérifié ✓ (préalables, plafond, une combinaison \
             d'horaire possible par session)"
                .to_string(),
        )
    }
}

// « 3 cr » ou « 6–12 cr » — showing the interval whole is the UI's choice
// for a stage the student weights himself (plan § Source)
pub fn credits_label(credits: &ulaval_scheduler_core::Credits) -> String {
    match credits {
        ulaval_scheduler_core::Credits::Fixed(count) => format!("{count} cr"),
        ulaval_scheduler_core::Credits::Range { min, max } => {
            format!("{min}–{max} cr")
        }
    }
}

// --- the session ribbon ---------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RibbonCard {
    // 1-based index in the horizon
    pub index: usize,
    // « A1-A26 » — or « É27 » for an été strip
    pub label: String,
    pub summer: bool,
    pub credits: u32,
    // above the student's own cap — marked, never silent
    pub over_cap: bool,
    pub has_range: bool,
    pub codes: Vec<String>,
    // the free annotation (« à l'étranger »)
    pub special: Option<String>,
    pub current: bool,
    // the session sits before the real-world clock's semester — its
    // courses are the student's acquired past (purely visual, ADR
    // `2026-08-retrait-de-la-notion-de-cours-reussi`)
    pub passed: bool,
    // gelée : the solver neither adds nor moves anything here (ADR
    // `2026-08-sessions-gelees-generalisent-les-completees`)
    pub frozen: bool,
    // its weekly schedule clashes: the card must say so even when the
    // session is not displayed (rapport étudiante 2026-08-13)
    pub conflict: bool,
}

pub fn ribbon_model(
    snapshot: &Snapshot,
    plan: &Plan,
    current: usize,
    today: ulaval_scheduler_core::Semester,
) -> Vec<RibbonCard> {
    let seasons = ulaval_scheduler_core::horizon_sessions(
        plan.start.season,
        plan.study_sessions,
    );
    let semesters = state::session_semesters(plan.start, &seasons);
    semesters
        .iter()
        .enumerate()
        .map(|(i, &semester)| {
            let index = i + 1;
            let codes = state::session_codes(plan, index);
            let credits = crate::solve::session_credits(snapshot, plan, index);
            RibbonCard {
                index,
                label: state::session_label(&semesters, i),
                summer: semester.season
                    == ulaval_scheduler_core::Season::Summer,
                credits: credits.total,
                over_cap: credits.total > plan.credit_cap,
                has_range: credits.has_range,
                passed: state::semester_precedes(semester, today),
                frozen: plan.frozen.contains(&index),
                conflict: !codes.is_empty()
                    && !crate::solve::weekly_schedule(snapshot, plan, index)
                        .report
                        .valid,
                codes,
                special: plan.special.get(&index).cloned(),
                current: index == current,
            }
        })
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod override_note_tests {
    use ulaval_scheduler_core::OverrideNote;

    use super::{present_override_note, present_prereq_draft};

    #[test]
    fn an_empty_correction_reads_as_no_prerequisites_at_all() {
        let draft = present_prereq_draft("   ");
        assert!(draft.valid);
        assert!(draft.echo.contains("aucun préalable"), "{}", draft.echo);
    }

    #[test]
    fn a_readable_expression_says_so_without_repeating_it() {
        let draft = present_prereq_draft("GCI-1000 ET (MAT-1900 OU MAT-1902)");
        assert!(draft.valid);
        assert_eq!(draft.echo, "compris.");
    }

    #[test]
    fn an_operand_the_grammar_cannot_check_is_named_before_the_commit() {
        let draft = present_prereq_draft("Examen de langue OU GCI-1000");
        assert!(draft.valid, "{}", draft.echo);
        assert!(
            draft.echo.contains("« Examen de langue »"),
            "the student must see what the solver will only presume: {}",
            draft.echo
        );
        assert!(draft.echo.contains("présumé acquis"), "{}", draft.echo);
    }

    #[test]
    fn a_starred_sigle_is_read_back_as_concomitance_permise() {
        // the `*` is the répertoire's shorthand — on screen it becomes the
        // sentence, so the student never has to know the symbol
        let draft = present_prereq_draft("GCI-2010*");
        assert!(draft.valid, "{}", draft.echo);
        assert_eq!(
            draft.echo,
            "compris - GCI-2010 peut être suivi la même session \
             (concomitance permise)."
        );
        // both surprises at once, each said in its own clause
        let draft = present_prereq_draft("GCI-2010* ET Examen de langue");
        assert_eq!(
            draft.echo,
            "compris - GCI-2010 peut être suivi la même session \
             (concomitance permise); « Examen de langue » sera présumé \
             acquis, le solveur ne peut pas le vérifier."
        );
    }

    #[test]
    fn a_broken_expression_is_refused_in_words_not_only_in_colour() {
        let draft = present_prereq_draft("GCI-1000 ET");
        assert!(!draft.valid);
        assert_eq!(
            draft.echo,
            "non lu : l'expression se termine sur un opérateur - la \
             correction n'est pas appliquée.",
            "the fault is named in French, like everything on screen"
        );
        assert!(
            draft.echo.contains("n'est pas appliquée"),
            "the echo says the consequence, not just the fault: {}",
            draft.echo
        );
    }

    #[test]
    fn every_guard_of_the_grammar_names_itself_in_french() {
        // the eight the parser can raise, plus the fallback that keeps a
        // future one visible instead of swallowed
        for (raw, expected) in [
            ("( GLG-1900 ) GLG-1000", "deux termes se suivent"),
            ("GLG-1000 (GLG-1900)", "une parenthèse ouvre là où"),
            ("()", "se ferme sans terme devant"),
            ("GLG-1000 )", "n'a pas d'ouvrante"),
            ("GLG-1000 ET", "l'expression se termine sur un opérateur"),
            ("ET GLG-1000", "ET n'a pas de terme à sa gauche"),
            ("OU GLG-1000", "OU n'a pas de terme à sa gauche"),
            ("( GLG-1000", "une parenthèse reste ouverte"),
        ] {
            let draft = present_prereq_draft(raw);
            assert!(!draft.valid, "{raw:?} must be refused");
            assert!(
                draft.echo.contains(expected),
                "{raw:?}: expected {expected:?}, got {:?}",
                draft.echo
            );
        }
        assert_eq!(super::prereq_fault("brand new guard"), "brand new guard");
    }

    #[test]
    fn a_credits_threshold_needs_no_presumption() {
        let draft = present_prereq_draft("GEX, Crédits exigés : 60");
        assert!(draft.valid);
        assert_eq!(draft.echo, "compris.");
    }

    #[test]
    fn every_refused_correction_names_the_course_and_what_happened() {
        let cases = [
            (
                OverrideNote::Unparsed {
                    code: "GCI-2000".to_string(),
                    error: "expression ends on an operator".to_string(),
                },
                "n'a pas pu être lue",
            ),
            (
                OverrideNote::UnknownCode {
                    code: "GCI-2000".to_string(),
                },
                "n'est pas au catalogue",
            ),
            (
                OverrideNote::OfficialChanged {
                    code: "GCI-2000".to_string(),
                    was: "GCI-1000".to_string(),
                    now: "GCI-1005".to_string(),
                },
                "a changé depuis votre correction",
            ),
        ];
        for (note, expected) in cases {
            let message = present_override_note(&note);
            assert!(message.contains("GCI-2000"), "{message}");
            assert!(message.contains(expected), "{message}");
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn both_data_errors_present_their_five_parts_in_french() {
        let fetch = present_data_error(&DataError::Fetch {
            file: "cours.json".to_string(),
            detail: "HTTP 404".to_string(),
        });
        assert!(fetch.what.contains("cours.json"));
        assert!(fetch.what.contains("téléchargé"));
        let parse = present_data_error(&DataError::Parse {
            file: "meta.json".to_string(),
            detail: "expected value".to_string(),
        });
        assert!(parse.what.contains("illisible"));
        for error in [&fetch, &parse] {
            assert!(!error.reaction.is_empty());
            assert!(!error.affected.is_empty());
            assert!(!error.action.is_empty());
            assert!(error.id.starts_with("GH-"));
            assert!(!error.detail.is_empty());
        }
    }

    #[test]
    fn the_id_is_deterministic_and_separates_distinct_failures() {
        assert_eq!(error_id("same"), error_id("same"));
        assert_ne!(error_id("same"), error_id("other"));
        assert_eq!(error_id("x").len(), "GH-".len() + 8);
    }

    fn credit_summary(
        in_addition: u32,
        preparatory: u32,
    ) -> ulaval_scheduler_wasm::credits::CreditSummary {
        ulaval_scheduler_wasm::credits::CreditSummary {
            counted: 0,
            in_addition,
            preparatory,
            profile_only: 0,
            unknown: Vec::new(),
        }
    }

    #[test]
    fn no_gap_has_neither_suffix_nor_anything_to_decompose() {
        let note = bac_credit_note(&credit_summary(0, 0));
        assert_eq!(note.suffix, "");
        assert!(!note.tooltip.is_empty(), "still says there is no gap");
        assert!(!note.tooltip.contains("stage"));
        assert!(!note.tooltip.contains("préparatoire"));
    }

    #[test]
    fn stages_en_sus_show_in_the_suffix_and_the_tooltip() {
        let note = bac_credit_note(&credit_summary(9, 0));
        assert_eq!(note.suffix, " (+9 cr en sus)");
        assert!(note.tooltip.contains("9 cr de stages"));
        assert!(!note.tooltip.contains("préparatoire"), "{}", note.tooltip);
    }

    #[test]
    fn preparatory_credits_explain_the_gap_without_a_suffix() {
        let note = bac_credit_note(&credit_summary(0, 6));
        assert_eq!(note.suffix, "", "not en-sus of the required total");
        assert!(note.tooltip.contains("6 cr de scolarité préparatoire"));
        assert!(!note.tooltip.contains("stage"), "{}", note.tooltip);
    }

    #[test]
    fn a_total_at_or_under_the_required_credits_carries_no_overrun() {
        let note = bac_credit_note(&credit_summary(0, 0));
        let at = bac_credit_label(120, 120, &note);
        assert!(!at.over);
        assert_eq!(at.text, "120/120 cr au bac");
        let under = bac_credit_label(99, 120, &note);
        assert!(!under.over);
        assert_eq!(under.text, "99/120 cr au bac");
    }

    #[test]
    fn a_total_over_the_required_credits_gets_the_overrun_warning() {
        let note = bac_credit_note(&credit_summary(0, 0));
        let label = bac_credit_label(129, 120, &note);
        assert!(label.over);
        assert_eq!(
            label.text,
            "⚠ 129/120 cr au bac — au-delà des 120 cr du programme"
        );
    }

    #[test]
    fn the_overrun_warning_composes_with_the_en_sus_suffix() {
        let note = bac_credit_note(&credit_summary(9, 0));
        let label = bac_credit_label(129, 120, &note);
        assert!(label.over);
        assert_eq!(
            label.text,
            "⚠ 129/120 cr au bac (+9 cr en sus) — au-delà des 120 cr du \
             programme",
            "the suffix survives, it is never overwritten"
        );
    }

    #[test]
    fn every_import_error_variant_presents_its_five_parts_in_french() {
        let variants = [
            ImportError::InvalidUrl {
                detail: "empty url".to_string(),
            },
            ImportError::Proxy {
                detail: "HTTP 500".to_string(),
            },
            ImportError::NotFound { status: 404 },
            ImportError::NotHtml {
                content_type: "application/json".to_string(),
            },
            ImportError::Parse {
                detail: "unexpected tag".to_string(),
            },
            ImportError::Preparatory {
                detail: "the prerequisite graph exceeds 10000 nodes"
                    .to_string(),
            },
            ImportError::Language {
                detail: "the language rule would list 300 courses".to_string(),
            },
            ImportError::Cancelled,
            ImportError::BrowserApi {
                detail: "AbortController unavailable".to_string(),
            },
            ImportError::CatalogueUnavailable,
            ImportError::InvalidProgramJson {
                detail: "missing field `code`".to_string(),
            },
        ];
        for error in &variants {
            let presented = present_import_error(error);
            assert!(!presented.what.is_empty(), "{error}");
            assert!(!presented.reaction.is_empty(), "{error}");
            assert_eq!(
                presented.affected,
                "L'import de ce programme seulement."
            );
            assert!(!presented.action.is_empty(), "{error}");
            assert!(presented.id.starts_with("GH-"), "{error}");
            assert_eq!(presented.detail, error.to_string());
        }
        for error in [&variants[1], &variants[3]] {
            let presented = present_import_error(error);
            assert!(
                presented.what.contains("corsproxy.io"),
                "the student must know a third party is in the loop: {}",
                presented.what
            );
        }
    }

    #[test]
    fn two_different_import_errors_carry_two_different_ids() {
        let a = present_import_error(&ImportError::NotFound { status: 404 });
        let b = present_import_error(&ImportError::NotFound { status: 410 });
        assert_ne!(a.id, b.id);
        let repeat =
            present_import_error(&ImportError::NotFound { status: 404 });
        assert_eq!(a.id, repeat.id, "the same failure keeps the same id");
    }

    // --- present_cheminement_error ---

    #[test]
    fn every_cheminement_refusal_states_the_five_parts() {
        let variants = [
            CheminementError::Unreadable {
                detail: "expected value at line 1 column 1".to_string(),
            },
            CheminementError::NoAdmission,
            CheminementError::TooLong {
                study_sessions: 20,
                max: 16,
            },
            CheminementError::Empty,
            CheminementError::CatalogueUnavailable,
        ];
        for error in &variants {
            let presented = present_cheminement_error(error);
            assert!(!presented.what.is_empty(), "{error}");
            assert!(
                presented.reaction.contains("n'a pas bougé"),
                "the untouched organigramme is the reassurance: {}",
                presented.reaction
            );
            assert_eq!(
                presented.affected,
                "Le chargement de ce fichier seulement."
            );
            assert!(!presented.action.is_empty(), "{error}");
            assert!(presented.id.starts_with("GH-"), "{error}");
            assert_eq!(presented.detail, error.to_string());
        }
        // the parser's own words reach the student, never a shrug
        assert!(present_cheminement_error(&variants[0])
            .action
            .contains("expected value"));
        // the placeholders are named: they are why this refusal happens
        assert!(present_cheminement_error(&variants[3])
            .action
            .contains("OPT-ION1"));
    }

    #[test]
    fn two_different_cheminement_errors_carry_two_different_ids() {
        let a = present_cheminement_error(&CheminementError::NoAdmission);
        let b = present_cheminement_error(&CheminementError::Empty);
        assert_ne!(a.id, b.id);
        let repeat = present_cheminement_error(&CheminementError::NoAdmission);
        assert_eq!(a.id, repeat.id, "the same failure keeps the same id");
    }

    // --- present_local_program_conflict ---

    #[test]
    fn a_local_program_conflict_states_the_five_parts() {
        let detail = "B-GEX A26 existe déjà dans le catalogue — le \
                       programme livré prime.";
        let presented = present_local_program_conflict(detail);
        assert_eq!(presented.what, detail);
        assert!(!presented.reaction.is_empty());
        assert_eq!(presented.affected, "L'import de ce programme seulement.");
        assert!(!presented.action.is_empty());
        assert!(presented.id.starts_with("GH-"));
        assert_eq!(presented.detail, detail);
    }

    #[test]
    fn two_different_local_program_conflicts_carry_two_different_ids() {
        let a = present_local_program_conflict("B-GEX A26 existe déjà.");
        let b = present_local_program_conflict("B-GLO A26 existe déjà.");
        assert_ne!(a.id, b.id);
    }

    // --- present_capsule_error ---

    #[test]
    fn a_not_a_transcript_error_names_the_manoeuvre_to_recopy_it() {
        let error = CapsuleError::NotATranscript {
            detail: "Missing element: th.ddtitle".to_string(),
        };
        let presented = present_capsule_error(&error);
        assert!(presented.what.contains("relevé de notes Capsule"));
        assert!(
            presented.action.contains("ctrl-u"),
            "copying the rendered page instead of its source is the \
             likeliest cause of this exact error (ERR-1)"
        );
        assert!(presented.action.contains("ctrl-a"));
        assert!(presented.action.contains("ctrl-c"));
        assert_eq!(
            presented.affected,
            "Le chargement de ce relevé seulement."
        );
        assert!(!presented.reaction.is_empty());
        assert!(presented.id.starts_with("GH-"));
        assert_eq!(presented.detail, error.to_string());
    }

    #[test]
    fn an_empty_transcript_error_names_the_missing_session() {
        let error = CapsuleError::Empty;
        let presented = present_capsule_error(&error);
        assert!(presented
            .what
            .contains("aucune session à l'Université Laval"));
        assert!(!presented.action.is_empty());
        assert_eq!(
            presented.affected,
            "Le chargement de ce relevé seulement."
        );
        assert!(presented.id.starts_with("GH-"));
        assert_eq!(presented.detail, error.to_string());
    }

    #[test]
    fn a_missing_catalogue_error_says_to_wait_for_the_data() {
        let error = CapsuleError::CatalogueUnavailable;
        let presented = present_capsule_error(&error);
        assert!(presented.what.contains("catalogue des cours"));
        assert!(presented.action.contains("réessayez"));
        assert_eq!(
            presented.affected,
            "Le chargement de ce relevé seulement."
        );
        assert!(presented.id.starts_with("GH-"));
        assert_eq!(presented.detail, error.to_string());
    }

    // --- grid geometry ---

    use ulaval_scheduler_core::{Alternative, CourseReport, ScheduleReport};

    use crate::data::{parse_data, RawData};

    fn snapshot() -> Snapshot {
        parse_data(
            &RawData {
                courses: r#"{"courses":[
                  {"code":"GEX-1000","title":"Hydrologie","credits":3,
                   "cycle":1,"prerequisites":null,"equivalents":[],
                   "seasons":{}}
                ]}"#
                .to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn section(json: &str) -> Section {
        serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"))
    }

    fn monday(_code: &str, nrc: &str, start: &str, end: &str) -> Section {
        section(&format!(
            r#"{{"nrc":"{nrc}","section":"A","mode":"in-person","slots":[
                {{"day":"monday","start":"{start}","end":"{end}"}}]}}"#
        ))
    }

    fn course(
        code: &str,
        valid: bool,
        selected: Vec<Section>,
    ) -> CourseReport {
        CourseReport {
            code: code.to_string(),
            valid,
            selected,
            alternatives: Vec::new(),
        }
    }

    fn wrap(courses: Vec<CourseReport>, valid: bool) -> WeeklySchedule {
        WeeklySchedule {
            report: ScheduleReport { valid, courses },
            excluded: Vec::new(),
            notes: Vec::new(),
        }
    }

    #[test]
    fn blocks_land_at_their_time_with_title_detail_and_colour() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![monday("GEX-1000", "111", "08:30", "11:20")],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.start, AXIS_START);
        assert_eq!(grid.end, AXIS_END);
        assert_eq!(grid.hours.first().map(String::as_str), Some("8:30"));
        assert_eq!(grid.hours.last().map(String::as_str), Some("22:30"));
        let block = &grid.days[0].blocks[0];
        assert_eq!(block.title, "Hydrologie", "title read off the snapshot");
        assert_eq!(block.detail, "GEX-1000 - A");
        assert_eq!(block.hue, 0.0);
        assert!((block.top - 0.0).abs() < f32::EPSILON);
        assert!((block.height - 170.0 / 840.0 * 100.0).abs() < 0.01);
        assert!((block.width - 100.0).abs() < f32::EPSILON);
        assert_eq!(block.alternatives, 0, "no alternative option offered");
        assert!(!grid.conflict);
        assert_eq!(grid.days.len(), 5, "Lundi→Vendredi, no weekend");
    }

    #[test]
    fn hue_follows_alphabetical_rank_among_the_schedules_courses() {
        let schedule = wrap(
            vec![
                course(
                    "ZZZ-1000",
                    true,
                    vec![monday("ZZZ-1000", "1", "08:30", "09:20")],
                ),
                course(
                    "AAA-1000",
                    true,
                    vec![monday("AAA-1000", "2", "10:30", "11:20")],
                ),
                course(
                    "MMM-1000",
                    true,
                    vec![monday("MMM-1000", "3", "12:30", "13:20")],
                ),
            ],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        let hue_of = |code: &str| {
            grid.days[0]
                .blocks
                .iter()
                .find(|b| b.code == code)
                .expect("a block for this code")
                .hue
        };
        assert_eq!(hue_of("AAA-1000"), 0.0, "first alphabetically");
        assert_eq!(hue_of("MMM-1000"), 120.0, "second of three");
        assert_eq!(hue_of("ZZZ-1000"), 240.0, "last alphabetically");
    }

    #[test]
    fn courses_sharing_a_matiere_still_get_distinct_hues() {
        let schedule = wrap(
            vec![
                course(
                    "AAA-1000",
                    true,
                    vec![monday("AAA-1000", "1", "08:30", "09:20")],
                ),
                course(
                    "AAA-2000",
                    true,
                    vec![monday("AAA-2000", "2", "10:30", "11:20")],
                ),
            ],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        let hue_of = |code: &str| {
            grid.days[0]
                .blocks
                .iter()
                .find(|b| b.code == code)
                .expect("a block for this code")
                .hue
        };
        assert_eq!(hue_of("AAA-1000"), 0.0);
        assert_eq!(hue_of("AAA-2000"), 180.0, "distinct from AAA-1000's hue");
    }

    #[test]
    fn ghosts_appear_only_for_the_selected_course_with_their_nrcs() {
        let mut with_ghost = course(
            "GEX-1000",
            true,
            vec![monday("GEX-1000", "111", "08:30", "09:20")],
        );
        with_ghost.alternatives = vec![Alternative {
            sections: vec![
                section(
                    r#"{"nrc":"333","section":"B","mode":"in-person","slots":[
                        {"day":"tuesday","start":"12:30","end":"15:20"}]}"#,
                ),
                monday("GEX-1000", "222", "14:30", "15:20"),
            ],
            valid: false,
        }];
        let schedule = wrap(vec![with_ghost], true);

        let silent = grid_model(&schedule, &snapshot(), None);
        assert_eq!(silent.days[1].blocks.len(), 0, "no ghost unrequested");

        let full = &silent.days[0].blocks[0];
        assert_eq!(
            full.alternatives, 1,
            "one alternative option to reveal on click"
        );

        let grid = grid_model(&schedule, &snapshot(), Some("GEX-1000"));
        let ghost = &grid.days[1].blocks[0];
        assert!(ghost.ghost);
        assert!(!ghost.valid, "swap semantics carried through");
        assert_eq!(ghost.nrcs, ["222", "333"], "sorted option identity");
        assert_eq!(
            ghost.alternatives, 0,
            "a ghost never advertises its own siblings"
        );
        assert_eq!(ghost.title, "B", "compact label, not the course title");
        assert_eq!(ghost.detail, "", "no duplicate line under a narrow ghost");
        assert_eq!(
            ghost.full_label, "GEX-1000 - B",
            "the aria label carries the course code the compact title drops"
        );
    }

    #[test]
    fn twin_section_ghost_slots_collapse_to_one_block() {
        // one option, two twin sections (hybrid pattern) at the same slot
        let mut with_ghost = course(
            "GEX-1000",
            true,
            vec![monday("GEX-1000", "111", "08:30", "09:20")],
        );
        with_ghost.alternatives = vec![Alternative {
            sections: vec![
                monday("GEX-1000", "222", "10:30", "11:20"),
                section(
                    r#"{"nrc":"223","section":"Z1","mode":"remote","slots":[
                        {"day":"monday","start":"10:30","end":"11:20"}]}"#,
                ),
            ],
            valid: true,
        }];
        let schedule = wrap(vec![with_ghost], true);
        let grid = grid_model(&schedule, &snapshot(), Some("GEX-1000"));
        let ghosts: Vec<_> =
            grid.days[0].blocks.iter().filter(|b| b.ghost).collect();
        assert_eq!(
            ghosts.len(),
            1,
            "twin sections of one option share a slot, deduplicated"
        );
    }

    #[test]
    fn two_options_sharing_a_slot_stay_two_clickable_ghosts() {
        // two distinct options that happen to land on the same slot must
        // both stay clickable — only true twins (same option) collapse
        let mut with_ghosts = course(
            "GEX-1000",
            true,
            vec![monday("GEX-1000", "111", "08:30", "09:20")],
        );
        with_ghosts.alternatives = vec![
            Alternative {
                sections: vec![monday("GEX-1000", "222", "10:30", "11:20")],
                valid: true,
            },
            Alternative {
                sections: vec![section(
                    r#"{"nrc":"333","section":"C","mode":"in-person","slots":[
                        {"day":"monday","start":"10:30","end":"11:20"}]}"#,
                )],
                valid: true,
            },
        ];
        let schedule = wrap(vec![with_ghosts], true);
        let grid = grid_model(&schedule, &snapshot(), Some("GEX-1000"));
        let ghosts: Vec<_> =
            grid.days[0].blocks.iter().filter(|b| b.ghost).collect();
        assert_eq!(ghosts.len(), 2, "distinct options, both stay clickable");
    }

    #[test]
    fn ghosts_carry_the_section_letter_never_the_course_title() {
        let mut with_ghost = course(
            "GEX-1000",
            true,
            vec![monday("GEX-1000", "111", "08:30", "09:20")],
        );
        with_ghost.alternatives = vec![Alternative {
            sections: vec![section(
                r#"{"nrc":"444","section":"Z2","mode":"remote","slots":[
                    {"day":"tuesday","start":"12:30","end":"13:20"}]}"#,
            )],
            valid: true,
        }];
        let schedule = wrap(vec![with_ghost], true);
        let grid = grid_model(&schedule, &snapshot(), Some("GEX-1000"));
        let ghost = &grid.days[1].blocks[0];
        assert_eq!(
            ghost.title, "Z2 - à distance",
            "compact label, not the course title"
        );
        assert_eq!(
            ghost.detail, "",
            "no duplicate detail line on a narrow ghost"
        );
    }

    #[test]
    fn a_ghost_without_a_section_letter_falls_back_to_its_nrc() {
        let mut with_ghost = course(
            "GEX-1000",
            true,
            vec![monday("GEX-1000", "111", "08:30", "09:20")],
        );
        with_ghost.alternatives = vec![Alternative {
            sections: vec![section(
                r#"{"nrc":"777","section":null,"mode":"in-person","slots":[
                    {"day":"tuesday","start":"12:30","end":"13:20"}]}"#,
            )],
            valid: true,
        }];
        let schedule = wrap(vec![with_ghost], true);
        let grid = grid_model(&schedule, &snapshot(), Some("GEX-1000"));
        let ghost = &grid.days[1].blocks[0];
        assert_eq!(ghost.title, "777", "the répertoire gave no letter");
    }

    #[test]
    fn a_hybrid_ghost_section_says_so_too() {
        let mut with_ghost = course(
            "GEX-1000",
            true,
            vec![monday("GEX-1000", "111", "08:30", "09:20")],
        );
        with_ghost.alternatives = vec![Alternative {
            sections: vec![section(
                r#"{"nrc":"555","section":"Z3","mode":"hybrid","slots":[
                    {"day":"tuesday","start":"12:30","end":"13:20"}]}"#,
            )],
            valid: true,
        }];
        let schedule = wrap(vec![with_ghost], true);
        let grid = grid_model(&schedule, &snapshot(), Some("GEX-1000"));
        let ghost = &grid.days[1].blocks[0];
        assert_eq!(ghost.title, "Z3 - hybride");
    }

    #[test]
    fn several_alternatives_are_counted_on_the_full_block() {
        let mut with_alts = course(
            "GEX-1000",
            true,
            vec![monday("GEX-1000", "111", "08:30", "09:20")],
        );
        with_alts.alternatives = vec![
            Alternative {
                sections: vec![monday("GEX-1000", "222", "10:30", "11:20")],
                valid: true,
            },
            Alternative {
                sections: vec![monday("GEX-1000", "333", "12:30", "13:20")],
                valid: true,
            },
        ];
        let schedule = wrap(vec![with_alts], true);

        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(
            grid.days[0].blocks[0].alternatives, 2,
            "both alternative options counted, ghosts not drawn"
        );
    }

    #[test]
    fn overlapping_blocks_share_their_column_in_lanes() {
        let schedule = wrap(
            vec![
                course(
                    "GEX-1000",
                    false,
                    vec![monday("GEX-1000", "111", "08:30", "11:20")],
                ),
                course(
                    "GEX-2000",
                    false,
                    vec![monday("GEX-2000", "222", "09:30", "12:20")],
                ),
                course(
                    "GEX-3000",
                    true,
                    vec![monday("GEX-3000", "333", "14:30", "15:20")],
                ),
            ],
            false,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        let blocks = &grid.days[0].blocks;
        assert!((blocks[0].width - 50.0).abs() < f32::EPSILON);
        assert!((blocks[0].left - 0.0).abs() < f32::EPSILON);
        assert!((blocks[1].width - 50.0).abs() < f32::EPSILON);
        assert!((blocks[1].left - 50.0).abs() < f32::EPSILON);
        assert!(
            (blocks[2].width - 100.0).abs() < f32::EPSILON,
            "the later block starts its own cluster, full width"
        );
        assert!(grid.days[0].conflict, "the day carries the warning");
        assert!(grid.conflict);
        assert_eq!(
            schedule_status(&schedule, false),
            "⚠ conflit d'horaire — plages en cause hachurées"
        );
    }

    #[test]
    fn twin_hybrid_sections_draw_one_block_not_two() {
        // the same slot in two sections of one option (hybrid pattern)
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![
                    monday("GEX-1000", "111", "08:30", "09:20"),
                    monday("GEX-1000", "222", "08:30", "09:20"),
                ],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.days[0].blocks.len(), 1, "deduplicated");
        assert!(!grid.days[0].blocks[0].clash, "no self-conflict");
    }

    #[test]
    fn the_hatch_marks_only_the_overlapping_blocks() {
        // GEX-1000 clashes GEX-2000 on monday but its thursday slot is
        // alone: the thursday block stays clean, the day unmarked
        let schedule = wrap(
            vec![
                course(
                    "GEX-1000",
                    false,
                    vec![section(
                        r#"{"nrc":"111","section":"A","mode":"in-person",
                            "slots":[
                              {"day":"monday","start":"08:30","end":"11:20"},
                              {"day":"thursday","start":"08:30","end":"09:20"}
                            ]}"#,
                    )],
                ),
                course(
                    "GEX-2000",
                    false,
                    vec![monday("GEX-2000", "222", "09:30", "12:20")],
                ),
            ],
            false,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert!(grid.days[0].blocks.iter().all(|block| block.clash));
        assert!(grid.days[0].conflict);
        assert!(!grid.days[3].blocks[0].clash, "the lone slot is clean");
        assert!(!grid.days[3].conflict, "Jeudi unmarked");
    }

    #[test]
    fn a_freed_lane_is_reused_by_a_later_block() {
        // A 8:30–9:20 (lane 0), B 8:30–10:20 (lane 1), C 9:30–10:20 —
        // still overlapping B, so same cluster, but lane 0 is free again
        let schedule = wrap(
            vec![
                course(
                    "GEX-1000",
                    true,
                    vec![monday("GEX-1000", "111", "08:30", "09:20")],
                ),
                course(
                    "GEX-2000",
                    true,
                    vec![monday("GEX-2000", "222", "08:30", "10:20")],
                ),
                course(
                    "GEX-3000",
                    true,
                    vec![monday("GEX-3000", "333", "09:30", "10:20")],
                ),
            ],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        let blocks = &grid.days[0].blocks;
        assert!((blocks[2].left - 0.0).abs() < f32::EPSILON, "lane reused");
        assert!((blocks[2].width - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_weekend_slot_adds_its_column_and_stretches_the_axis() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![section(
                    r#"{"nrc":"111","section":null,"mode":"hybrid","slots":[
                        {"day":"saturday","start":"18:00","end":"22:45"}]}"#,
                )],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.days.len(), 6, "Samedi included");
        assert_eq!(grid.days[5].label, "Samedi");
        assert_eq!(grid.end, 23 * 60, "rounded up to the half-hour");
        assert_eq!(
            grid.days[5].blocks[0].detail, "GEX-1000 - hybride",
            "no section letter, the mode says what it is"
        );
    }

    #[test]
    fn every_weekday_lands_in_its_own_column() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![section(
                    r#"{"nrc":"111","section":"R","mode":"remote","slots":[
                        {"day":"tuesday","start":"08:30","end":"09:20"},
                        {"day":"wednesday","start":"08:30","end":"09:20"},
                        {"day":"thursday","start":"08:30","end":"09:20"},
                        {"day":"friday","start":"08:30","end":"09:20"},
                        {"day":"sunday","start":"08:30","end":"09:20"}]}"#,
                )],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.days.len(), 7, "a Sunday slot opens the whole week");
        for day in [1, 2, 3, 4, 6] {
            assert_eq!(grid.days[day].blocks.len(), 1, "day {day}");
        }
        assert_eq!(
            grid.days[1].blocks[0].detail, "GEX-1000 - R - à distance",
            "a remote slot says so on the block"
        );
    }

    #[test]
    fn a_remote_course_without_slots_is_listed_never_interpolated() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![section(
                    r#"{"nrc":"111","section":"Z1","mode":"remote",
                        "slots":[]}"#,
                )],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.unplaced, ["GEX-1000"]);
        assert!(grid.days.iter().all(|day| day.blocks.is_empty()));
    }

    #[test]
    fn an_early_slot_stretches_the_axis_downward() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![monday("GEX-1000", "111", "07:15", "08:20")],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.start, 7 * 60, "floored to the half-hour");
    }

    #[test]
    fn an_empty_schedule_keeps_the_frame_and_says_so() {
        let schedule = wrap(Vec::new(), true);
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.days.len(), 5);
        assert_eq!(grid.start, AXIS_START);
        assert_eq!(
            schedule_status(&schedule, false),
            "aucun cours avec horaire dans cette session"
        );
        let one = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![monday("GEX-1000", "111", "08:30", "09:20")],
            )],
            true,
        );
        assert_eq!(
            schedule_status(&one, false),
            "combinaison automatique - sans conflit ✓"
        );
        assert_eq!(
            schedule_status(&one, true),
            "sections forcées - sans conflit ✓",
            "a hand-pinned section must not claim « automatique »"
        );
    }

    #[test]
    fn grid_status_leads_with_recalculating_while_a_search_runs() {
        assert_eq!(
            grid_status_label(
                "combinaison automatique - sans conflit ✓",
                false
            ),
            "combinaison automatique - sans conflit ✓"
        );
        assert_eq!(
            grid_status_label(
                "combinaison automatique - sans conflit ✓",
                true
            ),
            "⟳ recalcul en cours… — combinaison automatique - sans \
             conflit ✓"
        );
    }

    #[test]
    fn verification_verdict_never_shows_a_checkmark_while_searching() {
        let (class, label) = verification_verdict(false);
        assert_eq!(class, "panel-verdict panel-verdict--ok");
        assert!(label.contains('✓'));
        let (class, label) = verification_verdict(true);
        assert_eq!(class, "panel-verdict panel-verdict--pending");
        assert!(
            !label.contains('✓'),
            "no checkmark next to a provisional state"
        );
        assert!(label.contains("recalcul en cours"));
    }

    #[test]
    fn credit_labels_show_the_whole_interval() {
        assert_eq!(
            credits_label(&ulaval_scheduler_core::Credits::Fixed(3)),
            "3 cr"
        );
        assert_eq!(
            credits_label(&ulaval_scheduler_core::Credits::Range {
                min: 6,
                max: 12
            }),
            "6–12 cr"
        );
    }

    // --- ribbon ---

    #[test]
    fn the_ribbon_walks_the_horizon_with_credits_states_and_annotations() {
        let snapshot = parse_data(
            &RawData {
                courses: r#"{"courses":[
                  {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
                   "prerequisites":null,"equivalents":[],"seasons":{}},
                  {"code":"GEX-2000","title":"T","credits":4,"cycle":1,
                   "prerequisites":null,"equivalents":[],"seasons":{}}
                ]}"#
                .to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let plan = Plan {
            study_sessions: 4,
            displayed_placement: std::collections::BTreeMap::from([
                ("GEX-1000".to_string(), 1),
                ("GEX-2000".to_string(), 2),
            ]),
            special: std::collections::BTreeMap::from([(
                3,
                "à l'étranger".to_string(),
            )]),
            ..Plan::default()
        };
        // the real-world clock sits in H27: A26 is over, H27 is not
        let today = "H27"
            .parse::<ulaval_scheduler_core::Semester>()
            .unwrap_or_else(|e| panic!("{e}"));
        let ribbon = ribbon_model(&snapshot, &plan, 2, today);
        // A26 H27 É27 A27 H28 É28 : 4 study sessions + the étés
        assert_eq!(ribbon.len(), 6);
        assert_eq!(ribbon[0].label, "A1-A26");
        assert!(ribbon[0].passed, "A26 precedes today's H27");
        assert!(!ribbon[0].conflict, "no drawable clash here");
        assert_eq!(ribbon[0].credits, 3);
        assert_eq!(ribbon[1].codes, ["GEX-2000"]);
        assert!(ribbon[1].current);
        assert!(!ribbon[1].passed, "the running semester is not past");
        assert!(ribbon[2].summer);
        assert_eq!(ribbon[2].special.as_deref(), Some("à l'étranger"));
        assert!(!ribbon[3].passed, "the future is not past either");
    }

    #[test]
    fn a_clashing_session_marks_its_ribbon_card() {
        // two fall courses with one overlapping monday option each: the
        // card must warn even when the session is not displayed
        let snapshot = parse_data(
            &RawData {
                courses: r#"{"courses":[
                  {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
                   "prerequisites":null,"equivalents":[],
                   "seasons":{"fall":{"last_offered":2026,"options":[
                     [{"nrc":"111","section":"A","mode":"in-person","slots":[
                        {"day":"monday","start":"08:30","end":"11:20"}]}]
                   ]}}},
                  {"code":"GEX-2000","title":"T","credits":3,"cycle":1,
                   "prerequisites":null,"equivalents":[],
                   "seasons":{"fall":{"last_offered":2026,"options":[
                     [{"nrc":"222","section":"A","mode":"in-person","slots":[
                        {"day":"monday","start":"09:30","end":"12:20"}]}]
                   ]}}}
                ]}"#
                .to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = Plan {
            study_sessions: 2,
            ..Plan::default()
        };
        plan.manual
            .insert(1, vec!["GEX-1000".to_string(), "GEX-2000".to_string()]);
        let today = "H26"
            .parse::<ulaval_scheduler_core::Semester>()
            .unwrap_or_else(|e| panic!("{e}"));
        let ribbon = ribbon_model(&snapshot, &plan, 1, today);
        assert!(ribbon[0].conflict, "the clash marks the card");
        assert!(!ribbon[1].conflict);
    }
}
