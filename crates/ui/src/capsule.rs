// Loading a relevé Capsule pasted into the app: parsing the pasted HTML,
// turning the parse into the plan changes it demands, and reporting a
// bilan in French. Pure and testable — the paste itself is a browser text
// area, wired up by the wasm-only Capsule drawer, which carries no logic
// of its own (the `import.rs` / `ImportDrawer` split is the model this
// module copies; ADR `2026-08-import-de-releve-capsule`).

use std::collections::BTreeSet;

use ulaval_scheduler_core::{
    IgnoredCourse, IgnoredReason, TranscriptApplication,
};

use crate::state::{self, Plan};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapsuleError {
    #[error("this text is not a Capsule transcript : {detail}")]
    NotATranscript { detail: String },
    #[error("the transcript holds no Université Laval session")]
    Empty,
    // raised by the drawer itself, before `load` is even callable: without
    // the snapshot there is no catalogue to hold the sigle gate against
    #[error("the course catalogue is not loaded yet")]
    CatalogueUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapsuleLoad {
    pub application: TranscriptApplication,
    pub summary: CapsuleSummary,
}

// Every field a finished French sentence or label — `rsx!` only prints
// them (AP-5), it never builds prose out of raw data. The headline carries
// the counts; only what needs the student's attention is listed — the
// ignored rows and the unrecognized lines — never the placed or credited
// courses, which the grid itself already shows (demande d'Antoine,
// 2026-08-26).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapsuleSummary {
    pub ignored: Vec<String>,
    pub unrecognized: Vec<String>,
    pub headline: String,
}

// `known` is the catalogue's sigles (`Snapshot::by_code`'s keys): a relevé
// code outside it lands in the bilan as ignoré, never in the plan (ADR
// `2026-08-sigles-inconnus-du-releve-ignores`).
pub fn load(
    html: &str,
    minimum_study_sessions: usize,
    known: &BTreeSet<String>,
) -> Result<CapsuleLoad, CapsuleError> {
    let page = ulaval_scheduler_core::parser::transcript::parse(html)
        .map_err(|error| CapsuleError::NotATranscript {
            detail: error.to_string(),
        })?;
    let application = ulaval_scheduler_core::apply_transcript(
        &page.transcript,
        minimum_study_sessions,
        page.program_floor,
        known,
    )
    .ok_or(CapsuleError::Empty)?;
    let unrecognized: Vec<String> =
        page.anomalies.iter().map(ToString::to_string).collect();
    let summary = build_summary(&application, unrecognized);
    Ok(CapsuleLoad {
        application,
        summary,
    })
}

fn build_summary(
    application: &TranscriptApplication,
    unrecognized: Vec<String>,
) -> CapsuleSummary {
    let ignored: Vec<String> =
        application.ignored.iter().map(ignored_label).collect();

    let headline = format!(
        "{} cours {}, {} {}, {} {}, {} {}.",
        application.pinned.len(),
        agree(application.pinned.len(), "placé", "placés"),
        application.credited.len(),
        agree(application.credited.len(), "crédité", "crédités"),
        ignored.len(),
        agree(ignored.len(), "ignoré", "ignorés"),
        unrecognized.len(),
        if unrecognized.len() <= 1 {
            "ligne non reconnue"
        } else {
            "lignes non reconnues"
        },
    );

    CapsuleSummary {
        ignored,
        unrecognized,
        headline,
    }
}

// « 1 crédité » but « 0 ignoré » : French agreement singularizes both 0
// and 1, only 2 and above take the plural form.
fn agree(
    count: usize,
    singular: &'static str,
    plural: &'static str,
) -> &'static str {
    if count <= 1 {
        singular
    } else {
        plural
    }
}

fn ignored_label(entry: &IgnoredCourse) -> String {
    match &entry.reason {
        IgnoredReason::Failed(grade) => {
            format!("{} — échec ({grade})", entry.code)
        }
        IgnoredReason::OutsideHorizon => {
            format!("{} — hors de l'horizon", entry.code)
        }
        IgnoredReason::UnexpectedGrade(grade) => {
            format!("{} — note inattendue ({grade})", entry.code)
        }
        IgnoredReason::NotInCatalogue => {
            format!("{} — introuvable dans le catalogue", entry.code)
        }
    }
}

// Item 4 (plan): notes de passage and cours en cours pin their real
// session, RECONNAISSANCE DES ACQUIS credits without one, and the horizon
// grows to match. One function so the caller wraps the whole import in a
// single undoable `edit_plan` act (ACT-2).
//
// An application whose indices exceed the horizon it itself computed is
// impossible by construction — `apply_transcript` derives `study_sessions`
// from this very same `horizon_sessions` walk — but the insert is still
// guarded rather than trusted (total function, not a partial one).
pub fn apply_to_plan(plan: &mut Plan, application: &TranscriptApplication) {
    plan.start = application.start;
    plan.study_sessions = application.study_sessions;
    // the relevé is the authority on what is already behind the student:
    // the solver stops seating unpinned courses in those sessions
    plan.completed_sessions = application.completed_sessions;
    // raise-only: the heaviest relevé session sets the floor of the cap —
    // a cap below a load the student actually carried would make his own
    // past infeasible; a light relevé never shrinks the setting (ADR
    // `2026-08-plancher-du-plafond-de-credits-depuis-le-releve`)
    plan.credit_cap = plan.credit_cap.max(application.max_session_credits);
    // plan item 4 : « un été présent au relevé ouvre `summers_open` » — an
    // import only ever opens it, never closes an étudiant's own choice to
    // use summers when the relevé itself happens to hold none
    plan.summers_open |= application.summers_open;

    let seasons = ulaval_scheduler_core::horizon_sessions(
        plan.start.season,
        plan.study_sessions,
    );
    let horizon =
        ulaval_scheduler_core::session_semesters(plan.start, &seasons).len();

    let codes: Vec<String> = application.pinned.keys().cloned().collect();
    state::purge_codes(plan, &codes);

    for (code, &index) in &application.pinned {
        if index >= 1 && index <= horizon {
            // a code the plan already carries as credited (an entente from
            // an earlier import or a manual credit) must lose that status
            // the moment it is pinned instead — the rest of the codebase
            // treats "credited and pinned at once" as an invariant breach
            // (`validate_new_code` refuses to place a credited course)
            plan.credited.remove(code);
            plan.pinned_sessions.insert(code.clone(), index);
            plan.displayed_placement.insert(code.clone(), index);
        }
    }

    for code in &application.credited {
        state::credit_code(plan, code);
    }

    // a sigle the relevé names but today's catalogue does not may still
    // occupy the plan — pinned there by an import that predates the
    // catalogue gate. Purge it everywhere: no solver request can resolve
    // it (`placement_intake` dies on the whole plan, taking the automatic
    // placement down with it), and `validate_new_code` already refuses it
    // at every other door.
    let stale: Vec<String> = application
        .ignored
        .iter()
        .filter(|entry| entry.reason == IgnoredReason::NotInCatalogue)
        .map(|entry| entry.code.clone())
        .collect();
    state::purge_codes(plan, &stale);
    for code in &stale {
        plan.credited.remove(code);
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // --- load : the committed fixture (plan Acceptance) --------------------

    // every sigle the fixture itself mentions — the catalogue of a test
    // that wants the gate open for all of them
    fn codes_of(html: &str) -> BTreeSet<String> {
        let page = ulaval_scheduler_core::parser::transcript::parse(html)
            .unwrap_or_else(|e| panic!("{e}"));
        page.transcript
            .sessions
            .iter()
            .flat_map(|session| session.courses.iter())
            .map(|course| course.code.clone())
            .collect()
    }

    #[test]
    fn the_fixture_pins_passed_and_in_progress_courses_at_real_sessions() {
        let html = include_str!(
            "../../../tests/fixtures/test_cases/transcripts/exemple.html"
        );
        let load =
            load(html, 2, &codes_of(html)).unwrap_or_else(|e| panic!("{e}"));

        let a24: ulaval_scheduler_core::Semester =
            "A24".parse().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load.application.start, a24);
        assert!(
            load.application.summers_open,
            "l'É25 du relevé ouvre les étés"
        );
        assert!(
            load.application.ignored.is_empty(),
            "got {:?}",
            load.application.ignored
        );

        assert!(load.application.credited.contains("MAT-1910"));
        assert!(
            !load.application.pinned.contains_key("MAT-1910"),
            "crédité, jamais placé"
        );

        // one course from each transcript session, pinned at its real
        // (1-based) index over the horizon this same relevé grows
        assert_eq!(load.application.pinned["BIO-1904"], 1, "A24");
        assert_eq!(load.application.pinned["GEX-1580"], 3, "É25");
        assert_eq!(load.application.pinned["GEX-2590"], 6, "É26 en cours");
        assert_eq!(load.application.pinned["GEX-3333"], 7, "A26 en cours");

        assert_eq!(load.application.pinned.len(), 18);
        assert_eq!(
            load.application.completed_sessions, 3,
            "l'É25 est la dernière session notée de la fixture : 1..=3 \
             sont complétées, les sessions en cours restent ouvertes"
        );
        assert_eq!(
            load.application.max_session_credits, 22,
            "l'A24 de la fixture porte 22 crédits épinglés"
        );
        assert!(load.application.credited.contains("MAT-1910"));
        assert!(load.summary.ignored.is_empty());
        assert!(load.summary.unrecognized.is_empty());
        assert_eq!(
            load.summary.headline,
            "18 cours placés, 1 crédité, 0 ignoré, 0 ligne non reconnue."
        );
    }

    #[test]
    fn several_unrecognized_lines_pluralize_the_headline() {
        // a session with no clean course row at all, and two malformed
        // ones (wrong cell count) — enough to anchor a start (the header
        // alone counts) without pinning, crediting or ignoring anything
        let html = concat!(
            "<html><body><table>",
            r#"<tr><th class="ddtitle">CRÉDITS DE L'UNIVERSITÉ LAVAL</th></tr>"#,
            r#"<tr><th class="ddlabel">"#,
            r#"<span class="fieldOrangetextbold">Automne 2024</span>"#,
            "</th></tr>",
            r#"<tr><td class="dddefault">BIO-1904</td>"#,
            r#"<td class="dddefault">1</td></tr>"#,
            r#"<tr><td class="dddefault">CHM-1903</td>"#,
            r#"<td class="dddefault">1</td></tr>"#,
            "</table></body></html>",
        );
        let load =
            load(html, 2, &BTreeSet::new()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load.summary.unrecognized.len(), 2);
        assert_eq!(
            load.summary.headline,
            "0 cours placé, 0 crédité, 0 ignoré, 2 lignes non reconnues."
        );
    }

    #[test]
    fn a_sigle_the_catalogue_does_not_carry_is_reported_ignored_not_placed() {
        let html = include_str!(
            "../../../tests/fixtures/test_cases/transcripts/exemple.html"
        );
        // the same fixture, with one of its passed courses retired from
        // today's répertoire
        let mut known = codes_of(html);
        known.remove("BIO-1904");
        let load = load(html, 2, &known).unwrap_or_else(|e| panic!("{e}"));

        assert!(!load.application.pinned.contains_key("BIO-1904"));
        assert_eq!(load.application.pinned.len(), 17);
        assert_eq!(
            load.summary.ignored,
            vec!["BIO-1904 — introuvable dans le catalogue".to_string()]
        );
        assert_eq!(
            load.summary.headline,
            "17 cours placés, 1 crédité, 1 ignoré, 0 ligne non reconnue."
        );
    }

    // --- load : errors -------------------------------------------------

    #[test]
    fn garbage_html_is_not_a_transcript() {
        let error =
            load("<html><body>rien</body></html>", 2, &BTreeSet::new())
                .expect_err("no recognized banner at all");
        assert!(matches!(error, CapsuleError::NotATranscript { .. }));
    }

    #[test]
    fn a_valid_page_with_no_ulaval_session_is_empty() {
        // a recognized banner, but only a RECONNAISSANCE row — no Laval or
        // en-cours session for `apply_transcript` to anchor a start on
        let html = concat!(
            "<html><body><table>",
            r#"<tr><th class="ddtitle">RECONNAISSANCE DES ACQUIS</th></tr>"#,
            r#"<tr><th class="ddlabel">"#,
            r#"<span class="fieldOrangetextbold">Hiver 2013:</span>"#,
            r#"</th><td class="dddefault">Université de Montréal</td></tr>"#,
            r#"<tr><td class="dddefault">MAT-1910</td>"#,
            r#"<td class="dddefault">1</td>"#,
            r#"<td class="dddefault">Maths</td>"#,
            r#"<td class="dddefault">V</td>"#,
            r#"<td class="dddefault">3</td></tr>"#,
            "</table></body></html>",
        );
        let error = load(html, 2, &codes_of(html))
            .expect_err("no Laval/en-cours session");
        assert_eq!(error, CapsuleError::Empty);
    }

    // --- headline / labels ------------------------------------------------

    #[test]
    fn ignored_reasons_are_worded_by_kind() {
        let entries = [
            IgnoredCourse {
                code: "GEX-1002".to_string(),
                reason: IgnoredReason::Failed("E".to_string()),
            },
            IgnoredCourse {
                code: "GEX-9999".to_string(),
                reason: IgnoredReason::OutsideHorizon,
            },
            IgnoredCourse {
                code: "MAT-1910".to_string(),
                reason: IgnoredReason::UnexpectedGrade("B".to_string()),
            },
        ];
        assert_eq!(ignored_label(&entries[0]), "GEX-1002 — échec (E)");
        assert_eq!(ignored_label(&entries[1]), "GEX-9999 — hors de l'horizon");
        assert_eq!(
            ignored_label(&entries[2]),
            "MAT-1910 — note inattendue (B)"
        );
        let retired = IgnoredCourse {
            code: "ECN-2901".to_string(),
            reason: IgnoredReason::NotInCatalogue,
        };
        assert_eq!(
            ignored_label(&retired),
            "ECN-2901 — introuvable dans le catalogue"
        );
    }

    // --- apply_to_plan -------------------------------------------------

    fn application(
        start: &str,
        study_sessions: usize,
        pinned: &[(&str, usize)],
        credited: &[&str],
    ) -> TranscriptApplication {
        TranscriptApplication {
            start: start.parse().unwrap_or_else(|e| panic!("{e}")),
            study_sessions,
            completed_sessions: 0,
            max_session_credits: 0,
            summers_open: false,
            pinned: pinned
                .iter()
                .map(|&(code, index)| (code.to_string(), index))
                .collect(),
            credited: credited.iter().map(|&code| code.to_string()).collect(),
            ignored: Vec::new(),
        }
    }

    #[test]
    fn apply_to_plan_sets_calendar_facts_and_pins_and_credits_codes() {
        let mut plan = Plan::default();
        let mut application = application(
            "A24",
            2,
            &[("GEX-1000", 1), ("GEX-2000", 2)],
            &["MAT-1910"],
        );
        application.completed_sessions = 1;
        apply_to_plan(&mut plan, &application);

        assert_eq!(plan.start, application.start);
        assert_eq!(plan.study_sessions, 2);
        assert_eq!(
            plan.completed_sessions, 1,
            "le relevé ferme les sessions déjà notées au solveur"
        );
        assert!(!plan.summers_open);
        assert_eq!(plan.pinned_sessions["GEX-1000"], 1);
        assert_eq!(plan.pinned_sessions["GEX-2000"], 2);
        assert_eq!(plan.displayed_placement["GEX-1000"], 1);
        assert!(plan.credited.contains("MAT-1910"));
    }

    #[test]
    fn an_import_with_no_ete_never_closes_summers_the_student_already_opened()
    {
        let mut plan = Plan {
            summers_open: true,
            ..Plan::default()
        };
        let application = application("A24", 2, &[], &[]);
        assert!(!application.summers_open, "the relevé itself has no été");

        apply_to_plan(&mut plan, &application);

        assert!(
            plan.summers_open,
            "an import only ever opens summers, never closes them"
        );
    }

    #[test]
    fn the_cap_rises_to_the_heaviest_releve_session_and_never_shrinks() {
        let mut plan = Plan::default();
        assert_eq!(plan.credit_cap, crate::state::DEFAULT_CREDIT_CAP);

        let mut heavy = application("A24", 2, &[("GEX-1000", 1)], &[]);
        heavy.max_session_credits = 22;
        apply_to_plan(&mut plan, &heavy);
        assert_eq!(
            plan.credit_cap, 22,
            "l'A24 à 22 crédits relève le plafond"
        );

        let mut light = application("A24", 2, &[], &[]);
        light.max_session_credits = 12;
        apply_to_plan(&mut plan, &light);
        assert_eq!(
            plan.credit_cap, 22,
            "un relevé léger ne réduit jamais le plafond"
        );
    }

    #[test]
    fn a_code_already_pinned_elsewhere_is_moved_not_duplicated() {
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("GEX-1000".to_string(), 5);
        plan.displayed_placement.insert("GEX-1000".to_string(), 5);
        plan.manual.insert(5, vec!["GEX-1000".to_string()]);

        let application = application("A24", 2, &[("GEX-1000", 1)], &[]);
        apply_to_plan(&mut plan, &application);

        assert_eq!(
            plan.pinned_sessions.len(),
            1,
            "moved, not duplicated in another slot"
        );
        assert_eq!(plan.pinned_sessions["GEX-1000"], 1);
        assert_eq!(plan.displayed_placement["GEX-1000"], 1);
        assert!(
            plan.manual.get(&5).is_none_or(Vec::is_empty),
            "the stale manual entry at the old session is gone"
        );
    }

    #[test]
    fn a_code_already_credited_is_uncredited_when_pinned_this_time() {
        let mut plan = Plan::default();
        plan.credited.insert("GEX-1000".to_string());

        let application = application("A24", 2, &[("GEX-1000", 1)], &[]);
        apply_to_plan(&mut plan, &application);

        assert!(
            !plan.credited.contains("GEX-1000"),
            "credited and pinned at once is the invariant \
             `validate_new_code` refuses"
        );
        assert_eq!(plan.pinned_sessions["GEX-1000"], 1);
        assert_eq!(plan.displayed_placement["GEX-1000"], 1);
    }

    #[test]
    fn codes_the_application_does_not_mention_are_left_untouched() {
        let mut plan = Plan::default();
        plan.electives.push("GLG-1000".to_string());
        plan.pinned_sessions.insert("GLG-1001".to_string(), 3);
        plan.displayed_placement.insert("GLG-1001".to_string(), 3);

        let application = application("A24", 2, &[("GEX-1000", 1)], &[]);
        apply_to_plan(&mut plan, &application);

        assert_eq!(plan.electives, ["GLG-1000"]);
        assert_eq!(plan.pinned_sessions["GLG-1001"], 3, "untouched");
        assert_eq!(plan.displayed_placement["GLG-1001"], 3, "untouched");
    }

    #[test]
    fn a_leftover_of_a_retired_sigle_is_purged_by_the_next_import() {
        // the corruption an import made before the catalogue gate existed:
        // a retired sigle pinned, displayed, even credited — the next
        // import of the same relevé must clean all of it out
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("ECN-2901".to_string(), 2);
        plan.displayed_placement.insert("ECN-2901".to_string(), 2);
        plan.credited.insert("ECN-0000".to_string());

        let mut app = application("A24", 2, &[("GEX-1000", 1)], &[]);
        app.ignored.push(IgnoredCourse {
            code: "ECN-2901".to_string(),
            reason: IgnoredReason::NotInCatalogue,
        });
        app.ignored.push(IgnoredCourse {
            code: "ECN-0000".to_string(),
            reason: IgnoredReason::NotInCatalogue,
        });
        app.ignored.push(IgnoredCourse {
            code: "GLG-2000".to_string(),
            reason: IgnoredReason::Failed("E".to_string()),
        });
        apply_to_plan(&mut plan, &app);

        assert!(!plan.pinned_sessions.contains_key("ECN-2901"));
        assert!(!plan.displayed_placement.contains_key("ECN-2901"));
        assert!(!plan.credited.contains("ECN-0000"));
        assert_eq!(plan.pinned_sessions["GEX-1000"], 1);
    }

    #[test]
    fn a_failed_course_the_student_replanned_is_not_purged() {
        // an échec is ignored by the import, but the student may have
        // pinned it himself to retake it — only `NotInCatalogue` leftovers
        // are purged, never a plan the student made on purpose
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("GLG-2000".to_string(), 4);
        plan.displayed_placement.insert("GLG-2000".to_string(), 4);

        let mut app = application("A24", 4, &[], &[]);
        app.ignored.push(IgnoredCourse {
            code: "GLG-2000".to_string(),
            reason: IgnoredReason::Failed("E".to_string()),
        });
        apply_to_plan(&mut plan, &app);

        assert_eq!(plan.pinned_sessions["GLG-2000"], 4, "untouched");
        assert_eq!(plan.displayed_placement["GLG-2000"], 4, "untouched");
    }

    #[test]
    fn an_index_outside_the_recomputed_horizon_is_not_inserted() {
        // impossible by construction from a real `apply_transcript` result,
        // but the guard is exercised directly here rather than trusted
        let mut plan = Plan::default();
        let application = application("A24", 1, &[("GEX-9999", 99)], &[]);
        apply_to_plan(&mut plan, &application);

        assert!(
            !plan.pinned_sessions.contains_key("GEX-9999"),
            "an out-of-horizon index is guarded, not inserted"
        );
        assert!(!plan.displayed_placement.contains_key("GEX-9999"));
    }
}
