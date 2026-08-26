use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    horizon_sessions, schedule_intake, schedule_report, IntakeError,
    ScheduleError, ScheduleReport, Semester,
};

use crate::data::Snapshot;
use crate::state::{self, Plan};

// One session's weekly answer: the report over the drawable courses, plus
// every course the report could not draw, each with its reason in French —
// listed, never dropped (ERR-5: partial failure renders partially; the
// invariant « ne jamais rien perdre silencieusement »).
#[derive(Debug, Clone, PartialEq)]
pub struct WeeklySchedule {
    pub report: ScheduleReport,
    pub excluded: Vec<Excluded>,
    // recoveries that touched no user data (a stale pin ignored)
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Excluded {
    pub code: String,
    pub reason: String,
}

// Total, never an Err: `schedule_report` refuses whole when one course is
// undrawable, so the orchestration removes the named culprit, records why,
// and retries — bounded by the number of removable causes. The worst case
// is an empty report with every course named.
pub fn weekly_schedule(
    snapshot: &Snapshot,
    plan: &Plan,
    session: usize,
) -> WeeklySchedule {
    let mut schedule = WeeklySchedule {
        report: ScheduleReport {
            valid: true,
            courses: Vec::new(),
        },
        excluded: Vec::new(),
        notes: Vec::new(),
    };
    let Some(semester) = session_semester(plan, session) else {
        schedule.notes.push(format!(
            "Session {session} hors de l'horizon — rien à afficher."
        ));
        return schedule;
    };
    let key = state::session_key(semester);
    let mut codes = state::session_codes(plan, session);
    let mut chosen = plan.chosen.get(&session).cloned().unwrap_or_default();
    let bound = codes.len() + chosen.len() + 1;
    for _ in 0..bound {
        if codes.is_empty() {
            break;
        }
        match try_report(snapshot, &key, &codes, &chosen) {
            Ok(report) => {
                schedule.report = report;
                return schedule;
            }
            Err(failure) => {
                recover(failure, &mut codes, &mut chosen, &mut schedule)
            }
        }
    }
    // nothing drawable is left; everything removed was named above
    schedule
}

pub fn session_semester(plan: &Plan, session: usize) -> Option<Semester> {
    let seasons = horizon_sessions(plan.start.season, plan.study_sessions);
    let semesters = state::session_semesters(plan.start, &seasons);
    semesters.get(session.wrapping_sub(1)).copied()
}

// The sessions whose weekly schedule currently clashes — the verdict and
// the ribbon must never say ✓ while a grid shows hatching (rapport
// étudiante 2026-08-13).
pub fn conflicted_sessions(snapshot: &Snapshot, plan: &Plan) -> Vec<usize> {
    let seasons = horizon_sessions(plan.start.season, plan.study_sessions);
    (1..=seasons.len())
        .filter(|&session| {
            !weekly_schedule(snapshot, plan, session).report.valid
        })
        .collect()
}

enum Failure {
    Intake(IntakeError),
    Schedule(ScheduleError),
}

fn try_report(
    snapshot: &Snapshot,
    key: &str,
    codes: &[String],
    chosen: &BTreeMap<String, BTreeSet<String>>,
) -> Result<ScheduleReport, Failure> {
    let intake = schedule_intake(&snapshot.courses, key, codes)
        .map_err(Failure::Intake)?;
    schedule_report(&intake.courses, intake.season, chosen)
        .map_err(Failure::Schedule)
}

// every arm removes at least one course, pin or duplicate — that progress
// is what bounds the retry loop above
fn recover(
    failure: Failure,
    codes: &mut Vec<String>,
    chosen: &mut BTreeMap<String, BTreeSet<String>>,
    out: &mut WeeklySchedule,
) {
    match failure {
        Failure::Intake(IntakeError::UnknownCodes { codes: unknown }) => {
            for code in unknown {
                exclude(codes, out, &code, "absent du catalogue actuel");
            }
        }
        Failure::Intake(IntakeError::NotOffered { code })
        | Failure::Schedule(ScheduleError::NotOffered { code }) => {
            exclude(codes, out, &code, "pas offert à cette session");
        }
        Failure::Intake(IntakeError::DuplicatedCodes { codes: doubled }) => {
            for code in doubled {
                let mut seen = false;
                codes.retain(|candidate| {
                    if candidate != &code {
                        return true;
                    }
                    if seen {
                        false
                    } else {
                        seen = true;
                        true
                    }
                });
                out.notes.push(format!("Doublon de {code} ignoré."));
            }
        }
        Failure::Schedule(ScheduleError::NoOptions { code }) => {
            exclude(
                codes,
                out,
                &code,
                "aucune option d'inscription valide publiée",
            );
        }
        Failure::Schedule(ScheduleError::ScheduleUnknown { code }) => {
            exclude(codes, out, &code, "horaire pas encore publié");
        }
        Failure::Schedule(ScheduleError::UnknownChosenCourse { code }) => {
            chosen.remove(&code);
            out.notes.push(format!(
                "Épinglage de {code} ignoré : le cours n'est plus demandé."
            ));
        }
        Failure::Schedule(ScheduleError::ChosenOptionAbsent { code }) => {
            chosen.remove(&code);
            out.notes.push(format!(
                "Épinglage de {code} ignoré : ses sections n'existent plus."
            ));
        }
        // no removable culprit (unknown session, malformed pin, unresolved
        // credits): name every course and stop drawing — loud, bounded
        Failure::Intake(error) => {
            for code in codes.drain(..) {
                out.excluded.push(Excluded {
                    code,
                    reason: error.to_string(),
                });
            }
        }
    }
}

fn exclude(
    codes: &mut Vec<String>,
    out: &mut WeeklySchedule,
    code: &str,
    reason: &str,
) {
    codes.retain(|candidate| candidate != code);
    out.excluded.push(Excluded {
        code: code.to_string(),
        reason: reason.to_string(),
    });
}

// --- the session's credit line --------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCredits {
    pub total: u32,
    // a stage's Range counted at its lower bound — the line says so
    pub has_range: bool,
}

pub fn session_credits(
    snapshot: &Snapshot,
    plan: &Plan,
    session: usize,
) -> SessionCredits {
    state::session_codes(plan, session).iter().fold(
        SessionCredits {
            total: 0,
            has_range: false,
        },
        |mut credits, code| {
            if let Some(&index) = snapshot.by_code.get(code) {
                let course = &snapshot.courses[index];
                credits.total += course.credits.planning();
                credits.has_range |= matches!(
                    course.credits,
                    ulaval_scheduler_core::Credits::Range { .. }
                );
            }
            credits
        },
    )
}

// --- typed entry of a new course (INP-7: validated, never cleared) ---------

// an accepted code, possibly with something the student should hear —
// accepted anyway (his call), never silently (« jamais rien perdre »)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCode {
    pub code: String,
    pub warning: Option<String>,
}

// The one gate every add goes through. `session` is `None` for the
// « automatique » choice — the course is taken, the solver picks where — so
// the session-bound checks (already there, wrong season, closed summer) are
// the only ones that step aside; nothing else softens (ADR
// `2026-08-choix-automatique-ou-session-gelee`).
pub fn validate_new_code(
    snapshot: &Snapshot,
    plan: &Plan,
    session: Option<usize>,
    raw: &str,
) -> Result<NewCode, String> {
    let code = raw.trim().to_uppercase();
    if code.is_empty() {
        return Err("Entrez un code de cours (ex. GEX-1000).".to_string());
    }
    let Some(&index) = snapshot.by_code.get(&code) else {
        return Err(format!(
            "« {code} » est introuvable dans le catalogue — vérifiez le \
             sigle (ex. GEX-1000)."
        ));
    };
    // the checked box is an invariant, not a suggestion: an acquired
    // préparatoire course must not enter any session by any door
    if acquired_preparatory(snapshot, plan).contains(&code) {
        return Err(format!(
            "{code} fait partie de la scolarité préparatoire cochée \
             « déjà faite » — décochez la case pour le placer."
        ));
    }
    if plan.credited.contains(&code) {
        return Err(format!(
            "{code} est crédité par entente — retirez le crédit pour le \
             placer."
        ));
    }
    if session.is_some_and(|session| {
        state::session_codes(plan, session).contains(&code)
    }) {
        return Err(format!("{code} est déjà dans cette session."));
    }
    // placed at this very session would have been caught just above
    if let Some(&elsewhere) = plan
        .displayed_placement
        .get(&code)
        .filter(|&&placed| Some(placed) != session)
    {
        return Err(format!(
            "{code} est déjà placé en {} — retirez-le de là d'abord.",
            session_label_of(plan, elsewhere)
        ));
    }
    let course = &snapshot.courses[index];
    if let Some(semester) =
        session.and_then(|session| session_semester(plan, session))
    {
        if !course.seasons.contains_key(&semester.season) {
            let offered: Vec<&str> = course
                .seasons
                .keys()
                .map(|season| match season {
                    ulaval_scheduler_core::Season::Fall => "automne",
                    ulaval_scheduler_core::Season::Winter => "hiver",
                    ulaval_scheduler_core::Season::Summer => "été",
                })
                .collect();
            let offered = if offered.is_empty() {
                "aucune saison connue".to_string()
            } else {
                offered.join(" et ")
            };
            return Err(format!(
                "{code} n'est pas offert à cette saison (offert : {offered})."
            ));
        }
    }
    // prerequisites: a warning, not a wall — the student may know better,
    // but never learns it after the fact
    let held: std::collections::BTreeSet<String> = plan
        .displayed_placement
        .keys()
        .chain(plan.manual.values().flatten())
        .cloned()
        .collect();
    let credits = held
        .iter()
        .filter_map(|held_code| snapshot.by_code.get(held_code))
        .map(|&i| snapshot.courses[i].credits.planning())
        .sum();
    // the source text, extracted for every course so the student learns
    // *which* prerequisites when the verdict is Unmet (only a Parsed tree
    // can be Unmet — a raw-only one is presumed satisfied)
    let source = match &course.prerequisites {
        Some(ulaval_scheduler_core::Prerequisites::Parsed { raw, .. }) => {
            format!(" (préalables : {raw})")
        }
        _ => String::new(),
    };
    let warning =
        match ulaval_scheduler_core::prerequisites_met(
            course,
            &held,
            &std::collections::BTreeSet::new(),
            credits,
        ) {
            Ok(ulaval_scheduler_core::PrereqStatus::Unmet) => Some(format!(
                "{code} ajouté, mais ses préalables ne semblent pas \
                 remplis{source}."
            )),
            Ok(_) => None,
            Err(error) => Some(format!(
                "{code} ajouté; préalables illisibles : {error}."
            )),
        };
    // a summer explicitly closed is not a wall either — but adding into
    // it must never be silent (rapport étudiante 2026-08-14)
    let summer_note = session
        .and_then(|session| session_semester(plan, session))
        .filter(|semester| {
            semester.season == ulaval_scheduler_core::Season::Summer
                && !plan.summers_open
        })
        .map(|_| {
            format!(
                "{code} ajouté dans un été fermé aux cours réguliers — \
                 cochez « Ouvrir les étés » si c'est voulu."
            )
        });
    let warning = match (summer_note, warning) {
        (Some(summer), Some(other)) => Some(format!("{summer} {other}")),
        (summer, other) => summer.or(other),
    };
    Ok(NewCode { code, warning })
}

fn session_label_of(plan: &Plan, session: usize) -> String {
    let seasons = horizon_sessions(plan.start.season, plan.study_sessions);
    let semesters = state::session_semesters(plan.start, &seasons);
    state::session_label(&semesters, session.wrapping_sub(1))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    use crate::data::{parse_data, RawData};

    // GEX-1000: monday+wednesday option and tuesday option (fall);
    // GEX-2000: monday option only (fall); ANL-1010: winter only;
    // GEX-3000: offered fall, schedule not yet published;
    // GEX-4000: offered fall, no valid combination
    const COURSES: &str = r#"{"courses":[
      {"code":"ANL-1010","title":"Anglais","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"winter":{"last_offered":2026,"options":null}}},
      {"code":"GEX-1000","title":"Hydrologie","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"11111","section":"A","mode":"in-person","slots":[
            {"day":"monday","start":"08:30","end":"11:20"},
            {"day":"wednesday","start":"08:30","end":"09:20"}]}],
         [{"nrc":"11112","section":"B","mode":"in-person","slots":[
            {"day":"tuesday","start":"12:30","end":"15:20"}]}]
       ]}}},
      {"code":"GEX-2000","title":"Hydraulique","credits":4,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"22222","section":"A","mode":"in-person","slots":[
            {"day":"monday","start":"09:30","end":"12:20"}]}]
       ]}}},
      {"code":"GEX-3000","title":"Sans horaire","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":null}}},
      {"code":"GEX-4000","title":"Sans option","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[]}}}
    ]}"#;

    fn snapshot() -> Snapshot {
        parse_data(
            &RawData {
                courses: COURSES.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn plan_with(codes: &[&str]) -> Plan {
        let mut plan = Plan::default();
        plan.manual
            .insert(1, codes.iter().map(|code| code.to_string()).collect());
        plan
    }

    #[test]
    fn a_drawable_session_reports_and_excludes_nothing() {
        let schedule =
            weekly_schedule(&snapshot(), &plan_with(&["GEX-1000"]), 1);
        assert!(schedule.report.valid);
        assert_eq!(schedule.report.courses[0].code, "GEX-1000");
        assert_eq!(schedule.report.courses[0].alternatives.len(), 1);
        assert!(schedule.excluded.is_empty());
        assert!(schedule.notes.is_empty());
    }

    #[test]
    fn an_empty_session_is_a_valid_empty_report() {
        let schedule = weekly_schedule(&snapshot(), &Plan::default(), 1);
        assert!(schedule.report.valid);
        assert!(schedule.report.courses.is_empty());
        assert!(schedule.excluded.is_empty());
    }

    #[test]
    fn a_session_outside_the_horizon_says_so_instead_of_panicking() {
        let schedule = weekly_schedule(&snapshot(), &Plan::default(), 99);
        assert!(schedule.notes[0].contains("hors de l'horizon"));
        let schedule = weekly_schedule(&snapshot(), &Plan::default(), 0);
        assert!(schedule.notes[0].contains("hors de l'horizon"));
    }

    #[test]
    fn every_undrawable_course_is_named_with_its_reason() {
        // a stale save can hold codes the fresh snapshot no longer has
        let plan = plan_with(&[
            "GEX-1000", "ZZZ-9999", "ANL-1010", "GEX-3000", "GEX-4000",
        ]);
        let schedule = weekly_schedule(&snapshot(), &plan, 1);
        assert_eq!(schedule.report.courses.len(), 1, "GEX-1000 still drawn");
        let reasons: BTreeMap<&str, &str> = schedule
            .excluded
            .iter()
            .map(|excluded| (excluded.code.as_str(), excluded.reason.as_str()))
            .collect();
        assert_eq!(reasons["ZZZ-9999"], "absent du catalogue actuel");
        assert_eq!(reasons["ANL-1010"], "pas offert à cette session");
        assert_eq!(reasons["GEX-3000"], "horaire pas encore publié");
        assert_eq!(
            reasons["GEX-4000"],
            "aucune option d'inscription valide publiée"
        );
    }

    #[test]
    fn a_stale_pin_is_ignored_with_a_note_and_no_user_data_touched() {
        let mut plan = plan_with(&["GEX-1000"]);
        // a pin for a course no longer requested, and one for vanished NRCs
        plan.chosen.insert(
            1,
            BTreeMap::from([
                (
                    "GEX-2000".to_string(),
                    BTreeSet::from(["22222".to_string()]),
                ),
                (
                    "GEX-1000".to_string(),
                    BTreeSet::from(["99999".to_string()]),
                ),
            ]),
        );
        let schedule = weekly_schedule(&snapshot(), &plan, 1);
        assert_eq!(schedule.report.courses.len(), 1, "still drawn");
        assert_eq!(schedule.notes.len(), 2, "{:?}", schedule.notes);
        assert!(plan.chosen[&1].contains_key("GEX-2000"), "plan untouched");
    }

    #[test]
    fn conflicted_sessions_name_the_clashing_ones_only() {
        let snapshot = snapshot();
        let mut plan = plan_with(&["GEX-1000", "GEX-2000"]);
        assert!(
            conflicted_sessions(&snapshot, &plan).is_empty(),
            "the solver dodges the clash through the tuesday option"
        );
        // forcing GEX-1000 onto its monday option creates the clash
        plan.chosen.insert(
            1,
            BTreeMap::from([(
                "GEX-1000".to_string(),
                BTreeSet::from(["11111".to_string()]),
            )]),
        );
        assert_eq!(conflicted_sessions(&snapshot, &plan), [1]);
    }

    #[test]
    fn a_pinned_option_is_honoured() {
        let mut plan = plan_with(&["GEX-1000"]);
        plan.chosen.insert(
            1,
            BTreeMap::from([(
                "GEX-1000".to_string(),
                BTreeSet::from(["11112".to_string()]),
            )]),
        );
        let schedule = weekly_schedule(&snapshot(), &plan, 1);
        assert_eq!(schedule.report.courses[0].selected[0].nrc, "11112");
    }

    #[test]
    fn recover_dedupes_and_names_a_doubled_code() {
        let mut codes = vec![
            "GEX-1000".to_string(),
            "GEX-1000".to_string(),
            "GEX-2000".to_string(),
        ];
        let mut chosen = BTreeMap::new();
        let mut out = WeeklySchedule {
            report: ScheduleReport {
                valid: true,
                courses: Vec::new(),
            },
            excluded: Vec::new(),
            notes: Vec::new(),
        };
        recover(
            Failure::Intake(IntakeError::DuplicatedCodes {
                codes: vec!["GEX-1000".to_string()],
            }),
            &mut codes,
            &mut chosen,
            &mut out,
        );
        assert_eq!(codes, ["GEX-1000", "GEX-2000"], "first kept");
        assert!(out.notes[0].contains("Doublon"));
    }

    #[test]
    fn recover_handles_the_schedule_side_of_not_offered_too() {
        // `schedule_report` can name the same verdict after the intake
        // passed (equivalence resolution) — same removal, same reason
        let mut codes = vec!["GEX-1000".to_string()];
        let mut chosen = BTreeMap::new();
        let mut out = WeeklySchedule {
            report: ScheduleReport {
                valid: true,
                courses: Vec::new(),
            },
            excluded: Vec::new(),
            notes: Vec::new(),
        };
        recover(
            Failure::Schedule(ScheduleError::NotOffered {
                code: "GEX-1000".to_string(),
            }),
            &mut codes,
            &mut chosen,
            &mut out,
        );
        assert!(codes.is_empty());
        assert_eq!(out.excluded[0].reason, "pas offert à cette session");
    }

    #[test]
    fn recover_without_a_culprit_names_everything_and_stops() {
        let mut codes = vec!["GEX-1000".to_string(), "GEX-2000".to_string()];
        let mut chosen = BTreeMap::new();
        let mut out = WeeklySchedule {
            report: ScheduleReport {
                valid: true,
                courses: Vec::new(),
            },
            excluded: Vec::new(),
            notes: Vec::new(),
        };
        recover(
            Failure::Intake(IntakeError::UnknownSession {
                session: "x9999".to_string(),
            }),
            &mut codes,
            &mut chosen,
            &mut out,
        );
        assert!(codes.is_empty());
        assert_eq!(out.excluded.len(), 2, "every course named");
    }

    #[test]
    fn session_credits_sum_planning_values_and_flag_ranges() {
        let credits = session_credits(
            &snapshot(),
            &plan_with(&["GEX-1000", "GEX-2000"]),
            1,
        );
        assert_eq!(credits.total, 7);
        assert!(!credits.has_range);
        // an unknown code contributes nothing — it is surfaced elsewhere
        let credits =
            session_credits(&snapshot(), &plan_with(&["ZZZ-9999"]), 1);
        assert_eq!(credits.total, 0);
    }

    #[test]
    fn a_new_code_is_normalized_and_every_rejection_says_why() {
        let snapshot = snapshot();
        let plan = plan_with(&["GEX-1000"]);
        assert_eq!(
            validate_new_code(&snapshot, &plan, Some(1), "  gex-2000 "),
            Ok(NewCode {
                code: "GEX-2000".to_string(),
                warning: None
            })
        );
        assert!(validate_new_code(&snapshot, &plan, Some(1), "")
            .expect_err("empty")
            .contains("Entrez un code"));
        assert!(validate_new_code(&snapshot, &plan, Some(1), "zzz-1")
            .expect_err("unknown")
            .contains("introuvable"));
        assert!(validate_new_code(&snapshot, &plan, Some(1), "gex-1000")
            .expect_err("doubled")
            .contains("déjà"));
    }

    #[test]
    fn an_out_of_season_or_already_placed_code_is_refused_with_its_reason() {
        let snapshot = snapshot();
        // ANL-1010 is winter-only; session 1 is automne
        let error = validate_new_code(
            &snapshot,
            &Plan::default(),
            Some(1),
            "ANL-1010",
        )
        .expect_err("out of season");
        assert!(error.contains("offert : hiver"), "{error}");

        let mut plan = Plan::default();
        plan.displayed_placement.insert("GEX-1000".to_string(), 4);
        let error = validate_new_code(&snapshot, &plan, Some(1), "GEX-1000")
            .expect_err("placed elsewhere");
        assert!(error.contains("déjà placé en A3-A27"), "{error}");
    }

    #[test]
    fn taking_a_course_without_a_session_drops_only_the_session_checks() {
        let snapshot = snapshot();
        // ANL-1010 is winter-only: no session, no season to be wrong about
        assert_eq!(
            validate_new_code(&snapshot, &Plan::default(), None, "anl-1010")
                .expect("taken, the solver will find it a hiver")
                .code,
            "ANL-1010"
        );
        // what does not depend on a session still walls
        let error =
            validate_new_code(&snapshot, &Plan::default(), None, "ZZZ-1")
                .expect_err("not in the catalogue");
        assert!(error.contains("introuvable"), "{error}");
        let mut plan = Plan::default();
        plan.credited.insert("GEX-1000".to_string());
        let error = validate_new_code(&snapshot, &plan, None, "GEX-1000")
            .expect_err("credited");
        assert!(error.contains("crédité par entente"), "{error}");
        // already laid out somewhere: taking it again is still refused
        let mut plan = Plan::default();
        plan.displayed_placement.insert("GEX-1000".to_string(), 4);
        let error = validate_new_code(&snapshot, &plan, None, "GEX-1000")
            .expect_err("placed already");
        assert!(error.contains("déjà placé en A3-A27"), "{error}");
    }

    #[test]
    fn the_offered_seasons_line_speaks_every_season_and_the_none_case() {
        let raw = r#"{"courses":[
          {"code":"ETE-2000","title":"Été seulement","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null},
                      "summer":{"last_offered":2026,"options":null}}},
          {"code":"NUL-1000","title":"Jamais offert","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],"seasons":{}}
        ]}"#;
        let snapshot = crate::data::parse_data(
            &crate::data::RawData {
                courses: raw.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        // session 2 is hiver: ETE-2000 offers automne and été only
        let error = validate_new_code(
            &snapshot,
            &Plan::default(),
            Some(2),
            "ETE-2000",
        )
        .expect_err("out of season");
        assert!(error.contains("automne et été"), "{error}");
        let error = validate_new_code(
            &snapshot,
            &Plan::default(),
            Some(1),
            "NUL-1000",
        )
        .expect_err("never offered");
        assert!(error.contains("aucune saison connue"), "{error}");
    }

    #[test]
    fn adding_into_a_closed_summer_warns_but_never_walls() {
        let raw = r#"{"courses":[
          {"code":"ETE-2000","title":"Été aussi","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null},
                      "summer":{"last_offered":2026,"options":null}}},
          {"code":"ETE-3000","title":"Été exigeant","credits":3,"cycle":1,
           "prerequisites":{"raw":"ZZZ-1111","tree":"ZZZ-1111"},
           "equivalents":[],
           "seasons":{"summer":{"last_offered":2026,"options":null}}}
        ]}"#;
        let snapshot = crate::data::parse_data(
            &crate::data::RawData {
                courses: raw.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        // session 3 is the first été (A1 → H2 → É)
        let mut plan = Plan::default();
        let accepted =
            validate_new_code(&snapshot, &plan, Some(3), "ETE-2000")
                .unwrap_or_else(|e| panic!("{e}"));
        let warning = accepted.warning.expect("a closed summer must speak");
        assert!(warning.contains("été fermé"), "{warning}");
        // a closed summer AND unmet prerequisites: both spoken, in order
        let accepted =
            validate_new_code(&snapshot, &plan, Some(3), "ETE-3000")
                .unwrap_or_else(|e| panic!("{e}"));
        let warning = accepted.warning.expect("both warnings must speak");
        assert!(warning.contains("été fermé"), "{warning}");
        assert!(warning.contains("préalables"), "{warning}");
        // summers opened: nothing left to warn about
        plan.summers_open = true;
        let accepted =
            validate_new_code(&snapshot, &plan, Some(3), "ETE-2000")
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(accepted.warning.is_none());
    }

    #[test]
    fn a_session_outside_the_horizon_accepts_without_a_season_check() {
        // the weekly path will say « hors de l'horizon »; the entry check
        // simply has no season to judge against
        let accepted = validate_new_code(
            &snapshot(),
            &Plan::default(),
            Some(99),
            "GEX-1000",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(accepted.code, "GEX-1000");
    }

    #[test]
    fn an_unreadable_prerequisite_tree_still_warns_at_entry() {
        let mut snapshot = snapshot();
        let deep = (0..10_000).fold(
            ulaval_scheduler_core::PrereqTree::Course("X-1".to_string()),
            |child, _| ulaval_scheduler_core::PrereqTree::All {
                all: vec![child],
            },
        );
        let index = snapshot.by_code["GEX-1000"];
        snapshot.courses[index].prerequisites =
            Some(ulaval_scheduler_core::Prerequisites::Parsed {
                raw: "deep".to_string(),
                tree: deep,
            });
        let accepted = validate_new_code(
            &snapshot,
            &Plan::default(),
            Some(1),
            "GEX-1000",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let warning = accepted.warning.expect("must warn");
        assert!(warning.contains("illisibles"), "{warning}");
    }

    #[test]
    fn unmet_prerequisites_warn_and_name_the_source_text() {
        // a course requiring an unknown university code: Unmet statically
        let raw = r#"{"courses":[
          {"code":"GEX-5000","title":"Avancé","credits":3,"cycle":1,
           "prerequisites":{"raw":"ZZZ-1111","tree":"ZZZ-1111"},
           "equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}}
        ]}"#;
        let snapshot = crate::data::parse_data(
            &crate::data::RawData {
                courses: raw.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let accepted = validate_new_code(
            &snapshot,
            &Plan::default(),
            Some(1),
            "GEX-5000",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let warning = accepted.warning.expect("must warn");
        assert!(warning.contains("préalables"), "{warning}");
        assert!(warning.contains("ZZZ-1111"), "{warning}");
    }
}

// --- the solver-B worker dialogue (pure halves) ----------------------------
// The worker (crates/ui-calculations) holds the snapshot; the app sends
// JSON strings and receives JSON strings. Building and parsing them is
// pure and tested here; the postMessage plumbing lives in `browser`.

// one proposal, small budget: the first solution lands in tens of
// milliseconds — the automatic re-placement fires on every edit, so the
// budget must stay cheap (ADR `2026-08-organigramme-en-continu-sans-bouton`)
// 2 M nodes ≈ 3 s native, ~10 s in the worker — the cap only bites on the
// hard programs (B-GMC grinds its aggregates), where a longer one-shot
// beats an empty grid; the easy ones return long before it
pub const PROPOSE_MAX_NODES: u64 = 2_000_000;

pub fn place_request(
    id: u64,
    plan: &Plan,
    program: Option<&ulaval_scheduler_core::Program>,
    max_nodes: u64,
) -> String {
    request_json("place", id, plan, program, max_nodes)
}

// prove the displayed organigramme: everything already laid out is pinned,
// so the search validates instead of building (ADR
// `2026-08-module-wasm-quatre-fonctions-js`)
pub fn verify_request(
    id: u64,
    plan: &Plan,
    program: Option<&ulaval_scheduler_core::Program>,
) -> String {
    let mut pinned_everything = plan.clone();
    pinned_everything.pinned_sessions = plan.displayed_placement.clone();
    request_json("verify", id, &pinned_everything, program, PROPOSE_MAX_NODES)
}

// The codes of the program's « Scolarité préparatoire » rule — the only
// courses that still ride as `passed`: done by hypothesis (the checkbox,
// checked by default), never to be placed in the horizon.
pub fn preparatory_codes(
    program: &ulaval_scheduler_core::Program,
) -> Vec<String> {
    program
        .rules
        .iter()
        .find(|rule| {
            rule.title == ulaval_scheduler_core::PREPARATORY_RULE_TITLE
        })
        .and_then(|rule| match &rule.courses {
            ulaval_scheduler_core::RuleCourses::List { courses } => {
                Some(courses.clone())
            }
            _ => None,
        })
        .unwrap_or_default()
}

// Everything the student holds without occupying a session: the
// « préparatoire faite » 0xxx and the courses an agreement credited him.
// Both ride as `PlaceQuery.passed`, which is what keeps them out of the
// placement while their credits and prerequisites still count.
pub fn passed_codes(
    plan: &Plan,
    program: Option<&ulaval_scheduler_core::Program>,
) -> Vec<String> {
    let mut codes: Vec<String> = if plan.preparatory_done {
        program.map(preparatory_codes).unwrap_or_default()
    } else {
        Vec::new()
    };
    for code in &plan.credited {
        if !codes.contains(code) {
            codes.push(code.clone());
        }
    }
    codes
}

// The préparatoire codes « acquis d'office » by the checked box — unless
// an entente moved one into another rule (the granted program strips it
// from the préparatoire list, so `rule_grants` mirrors that here).
pub fn acquired_preparatory(
    snapshot: &Snapshot,
    plan: &Plan,
) -> BTreeSet<String> {
    if !plan.preparatory_done {
        return BTreeSet::new();
    }
    crate::panel::chosen_program(snapshot, plan)
        .map(|program| {
            preparatory_codes(program)
                .into_iter()
                .filter(|code| !plan.rule_grants.contains_key(code))
                .collect()
        })
        .unwrap_or_default()
}

// The acquired codes that still occupy placement state — préparatoire
// under a checked box, or credited by an agreement. That is what the
// healing effect purges, so the grid never draws a course every solver
// request silently drops. Takes the effective (granted) program, like
// every request builder.
pub fn acquired_leftovers(
    plan: &Plan,
    program: &ulaval_scheduler_core::Program,
) -> Vec<String> {
    passed_codes(plan, Some(program))
        .into_iter()
        .filter(|code| {
            plan.displayed_placement.contains_key(code)
                || plan.pinned_sessions.contains_key(code)
                || plan.electives.contains(code)
                || plan.manual.values().flatten().any(|held| held == code)
                || plan.chosen.values().any(|pins| pins.contains_key(code))
        })
        .collect()
}

// « MAT-0130 retiré des sessions : … » — plural and way out follow the
// family that acquired the course
pub fn purge_note(codes: &[String], credited: bool) -> String {
    let (mark, them) = if codes.len() > 1 {
        ("s", "les")
    } else {
        ("", "le")
    };
    let list = codes.join(", ");
    if credited {
        format!(
            "{list} retiré{mark} des sessions : crédité{mark} par entente. \
             Retirez le crédit pour {them} replacer."
        )
    } else {
        format!(
            "{list} retiré{mark} des sessions : la scolarité préparatoire \
             est marquée « déjà faite ». Décochez la case pour {them} \
             replacer."
        )
    }
}

// Verification is only worth asking once every requested course has a
// session — same inputs as `request_json`, the answer names what still
// floats. An intake error (typo'd code…) comes back as the Err.
pub fn unplaced_codes(
    snapshot: &Snapshot,
    plan: &Plan,
    program: Option<&ulaval_scheduler_core::Program>,
) -> Result<Vec<String>, String> {
    let mut electives = plan.electives.clone();
    // `pinned_sessions` chained for the same reason as `request_json`: a
    // pin astray from `displayed_placement` must not kill the intake
    for code in plan
        .manual
        .values()
        .flatten()
        .chain(plan.displayed_placement.keys())
        .chain(plan.pinned_sessions.keys())
    {
        if !electives.contains(code) {
            electives.push(code.clone());
        }
    }
    let passed = passed_codes(plan, program);
    let (concentration, profile) = scope_choice(plan);
    let intake = ulaval_scheduler_core::placement_intake(
        program,
        concentration.as_deref(),
        profile.as_deref(),
        &electives,
        &passed,
        &[],
        &snapshot.courses,
    )
    .map_err(|error| error.to_string())?;
    let placed: BTreeSet<&str> = plan
        .displayed_placement
        .keys()
        .chain(plan.manual.values().flatten())
        .map(String::as_str)
        .collect();
    Ok(intake
        .courses
        .iter()
        .map(|course| course.code.clone())
        .filter(|code| {
            !placed.contains(code.as_str()) && !intake.passed.contains(code)
        })
        .collect())
}

// the chosen concentration and profile titles, read off the plan's choice
fn scope_choice(plan: &Plan) -> (Option<String>, Option<String>) {
    match plan.program.as_ref() {
        None => (None, None),
        Some(choice) => (choice.concentration.clone(), choice.profile.clone()),
    }
}

// the query mirrors `ui-calculations::protocol::PlaceQuery`: the manual
// courses ride as electives pinned to their session (adding one was an
// explicit act), the displayed placement seeds the search so proposals
// stay close to what the student sees
fn request_json(
    kind: &str,
    id: u64,
    plan: &Plan,
    program: Option<&ulaval_scheduler_core::Program>,
    max_nodes: u64,
) -> String {
    let mut pinned = plan.pinned_sessions.clone();
    let mut electives: Vec<String> = plan.electives.clone();
    for (&session, codes) in &plan.manual {
        for manual_code in codes {
            pinned.entry(manual_code.clone()).or_insert(session);
            if !electives.contains(manual_code) {
                electives.push(manual_code.clone());
            }
        }
    }
    // every laid-out course rides with its Course: a pin without one is a
    // PlacementError — and a save from before a fix may hold a placement
    // with no elective entry (the intake dedups against the program list).
    // `pinned_sessions` is chained too: a stale solver answer adopted over
    // a fresh import can leave a pin out of `displayed_placement`, and the
    // pin must still ride with its Course (« BIO-1904 is passed or pinned
    // but has no Course in the request », 2026-08-26)
    for code in plan
        .displayed_placement
        .keys()
        .chain(plan.pinned_sessions.keys())
    {
        if !electives.contains(code) {
            electives.push(code.clone());
        }
    }
    let passed = passed_codes(plan, program);
    // a passed course can be neither pinned nor an elective to place
    pinned.retain(|code, _| !passed.contains(code));
    electives.retain(|code| !passed.contains(code));
    // the chosen scopes ride with every ask: the solver places their
    // mandatory courses too (décision 2026-08-19)
    let (concentration, profile) = scope_choice(plan);
    let request = serde_json::json!({
        "kind": kind,
        "id": id,
        "query": {
            "program": program,
            "concentration": concentration,
            "profile": profile,
            "electives": electives,
            "passed": passed,
            "pinned": pinned,
            "start": plan.start.season,
            "study_sessions": plan.study_sessions,
            "credit_cap": plan.credit_cap,
            "concomitant": plan.concomitant,
            "summers_open": plan.summers_open,
            "completed_sessions": plan.completed_sessions,
            "seed": plan.displayed_placement,
            "max_nodes": max_nodes,
            "max_solutions": 1,
        },
    });
    // expect over `?`: serializing maps, vecs and strings provably
    // cannot fail
    serde_json::to_string(&request)
        .expect("Request serialization always succeeds")
}

// --- the answers -----------------------------------------------------------

// mirror of the worker's `Response` (core's types serialize only, so the
// app re-reads the JSON into these), plus the shim's own ready envelope
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WorkerAnswer {
    Ready { id: u64, summary: ReadySummary },
    Report { id: u64, report: PlacementReport },
    Error { id: u64, message: String },
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ReadySummary {
    pub course_count: usize,
    #[serde(default)]
    pub collisions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PlacementReport {
    pub sessions: Vec<ulaval_scheduler_core::Season>,
    pub placement: PlacementAnswer,
    #[serde(default)]
    pub set_aside: Vec<String>,
    // electives the solver's intake added because a candidate's
    // prerequisites force them — adopted into the plan and announced (ADR
    // `2026-08-injection-des-electifs-forces-par-les-prealables`)
    #[serde(default)]
    pub injected: Vec<String>,
    // regular courses the escalation seated in an été the plan keeps
    // closed — announced, the setting itself never touched (ADR
    // `2026-08-escalade-etes-ouverts-dans-le-repli`)
    #[serde(default)]
    pub summers_forced: Vec<String>,
    #[serde(default)]
    pub credit_shortfalls: Vec<CreditShortfallAnswer>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PlacementAnswer {
    pub completion: String,
    pub solutions: Vec<SolutionAnswer>,
    #[serde(default)]
    pub blocked: Vec<BlockedAnswer>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SolutionAnswer {
    pub placement: BTreeMap<String, usize>,
    #[serde(default)]
    pub assumed: BTreeSet<String>,
    // courses the best-effort pass could not seat — non-empty only when
    // the exact search found nothing (ADR
    // `2026-08-placement-au-mieux-en-repli`)
    #[serde(default)]
    pub left_out: BTreeSet<String>,
    #[serde(default)]
    pub credit_shortfalls: Vec<CreditShortfallAnswer>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct CreditShortfallAnswer {
    pub code: String,
    pub session: usize,
    pub earned_before: u32,
    pub required: u32,
}

pub fn credit_shortfall_message(
    shortfall: &CreditShortfallAnswer,
    plan: &Plan,
) -> String {
    let session = session_semester(plan, shortfall.session)
        .map(|semester| semester.to_string())
        .unwrap_or_else(|| format!("session {}", shortfall.session));
    let missing = shortfall.required.saturating_sub(shortfall.earned_before);
    format!(
        "{} est placé en {} avec {} crédits acquis avant cette session; le \
         minimum est {} crédits. Le solveur l'a placé au plus tard \
         disponible. Répartissez {} crédits de plus avant {} ou déplacez \
         le cours.",
        shortfall.code,
        session,
        shortfall.earned_before,
        shortfall.required,
        missing,
        session,
    )
}

pub fn course_shortfall_messages(
    code: &str,
    shortfalls: &[CreditShortfallAnswer],
    plan: &Plan,
) -> Vec<String> {
    shortfalls
        .iter()
        .filter(|shortfall| shortfall.code == code)
        .map(|shortfall| credit_shortfall_message(shortfall, plan))
        .collect()
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct BlockedAnswer {
    pub code: String,
    pub reason: String,
    // `unsatisfiable-prerequisites` names its proof: each entry one
    // requirement, its interchangeable alternatives listed together —
    // absent on the wire when the reason has no course to name
    #[serde(default)]
    pub missing: Vec<Vec<String>>,
}

pub fn parse_worker_answer(text: &str) -> Result<WorkerAnswer, String> {
    serde_json::from_str(text)
        .map_err(|error| format!("réponse du solveur illisible : {error}"))
}

// truncation is never silent (ADR `2026-07-budget-de-b-en-double-borne`)
pub fn completion_note(answer: &PlacementAnswer) -> Option<String> {
    // A best-effort answer speaks for itself through `left_out_note`: its
    // `completion` describes the *relaxed* enumeration, so « d'autres
    // agencements équivalents existent » or « rien trouvé » would both
    // contradict the grid the student is looking at (ADR
    // `2026-08-verdicts-honnetes-et-panneau-jamais-vide`).
    if answer.solutions.iter().any(|s| !s.left_out.is_empty()) {
        return None;
    }
    match answer.completion.as_str() {
        // nothing found ≠ nothing exists: the empty answer must say which
        // (rapport étudiante 2026-08-14 : « rien ne change » sans verdict)
        "node-budget" if answer.solutions.is_empty() => Some(
            "La recherche s'est arrêtée avant d'avoir tout exploré, sans \
             rien trouver pour l'instant — un agencement peut quand même \
             exister. Simplifiez (plafond, sessions, cours) pour aider la \
             recherche."
                .to_string(),
        ),
        "node-budget" => Some(
            "La recherche s'est arrêtée avant d'avoir tout exploré : il \
             peut exister d'autres agencements — ou un agencement là où \
             rien n'a été trouvé. Simplifiez (plafond, sessions, cours) \
             pour aider la recherche."
                .to_string(),
        ),
        "solution-cap" => Some(
            "D'autres agencements équivalents existent; celui proposé \
             suit votre cheminement actuel du plus près. Vous pouvez \
             déplacer des cours en les glissant."
                .to_string(),
        ),
        "complete"
            if answer.solutions.is_empty() && answer.blocked.is_empty() =>
        {
            Some(
                "Aucun agencement n'est possible avec ces contraintes — \
                 c'est certain, inutile de chercher plus longtemps. \
                 Ajustez le plafond, les sessions ou les cours."
                    .to_string(),
            )
        }
        _ => None,
    }
}

// What the best-effort pass had to leave out, and why — one line per code
// so each message stands and retires with its own cause. `blocked` carries
// the reason when the pre-screen named the culprit; a session the student
// pinned deserves the message the act asked for — never « chaque session »
// when he chose exactly one (rapport étudiante-gex 2026-08-19); otherwise
// the honest default is that no room was left, which is exactly what the
// search found.
pub fn left_out_line(
    code: &str,
    blocked: Option<&BlockedAnswer>,
    plan: &Plan,
    snapshot: Option<&Snapshot>,
) -> String {
    match (blocked, plan.pinned_sessions.get(code)) {
        // the pre-screen's reason is more precise than the pin (a pin
        // toward a season the course never offers is an empty domain)
        (Some(blocked), _) => blocked_note(blocked),
        (None, Some(&session)) => {
            pinned_refusal_line(code, session, plan, snapshot)
        }
        (None, None) => format!(
            "{code} : aucune place ne restait — les autres cours et le \
             plafond remplissent déjà chaque session où il est offert."
        ),
    }
}

// Why the session the student pinned refuses the course, the constraint
// named instead of the old « (plafond, horaire ou préalables) » triple:
// the cap with its numbers, the season not offered, the missing
// préalables by code — and « l'horaire » stays the honest remainder when
// none of the checkable three is at fault (retour d'Antoine 2026-08-26 :
// le message générique ne dit ni la cause ni où la trouver).
fn pinned_refusal_line(
    code: &str,
    session: usize,
    plan: &Plan,
    snapshot: Option<&Snapshot>,
) -> String {
    let label = session_label_of(plan, session);
    let causes = snapshot
        .map(|snapshot| pinned_refusal_causes(code, session, plan, snapshot))
        .unwrap_or_default();
    if causes.is_empty() {
        return format!(
            "{code} : la session {label} que vous avez épinglée ne peut \
             pas l'accueillir — aucune combinaison d'horaire n'y tient \
             avec les autres cours. Dépinglez-le ou déplacez un cours de \
             cette session."
        );
    }
    format!(
        "{code} : la session {label} que vous avez épinglée ne peut pas \
         l'accueillir — {}. Dépinglez-le ou corrigez ce qui bloque.",
        causes.join(" ; ")
    )
}

// the checkable causes, in the order the student can act on them
fn pinned_refusal_causes(
    code: &str,
    session: usize,
    plan: &Plan,
    snapshot: &Snapshot,
) -> Vec<String> {
    let Some(&index) = snapshot.by_code.get(code) else {
        return Vec::new();
    };
    let course = &snapshot.courses[index];
    let mut causes = Vec::new();
    if let Some(season) = session_season(plan, session) {
        if !course.seasons.contains_key(&season) {
            causes.push(format!(
                "il n'est pas offert en {}",
                season_name(season)
            ));
        }
    }
    let load = session_load(plan, snapshot, session);
    if load > plan.credit_cap {
        causes.push(format!(
            "le plafond de {} cr y est dépassé ({load} cr posés)",
            plan.credit_cap
        ));
    }
    let (satisfied, same_session, credits) =
        acquired_before(code, session, plan, snapshot);
    let missing = ulaval_scheduler_core::unmet_prerequisites(
        course,
        &satisfied,
        &same_session,
        credits,
    )
    .unwrap_or_default();
    // a requirement the student *is* taking, only not early enough, is a
    // different fact from one he holds nowhere — and a different fix
    // (the répertoire's `*`, or the dérogation): naming them alike sent
    // him hunting for a course already on his grid
    let (concurrent, absent): (Vec<Vec<String>>, Vec<Vec<String>>) = missing
        .into_iter()
        .partition(|group| {
            group.iter().all(|code| same_session.contains(code))
        });
    if !absent.is_empty() {
        causes.push(format!(
            "préalable manquant avant cette session : {}",
            requirement_list(&absent)
        ));
    }
    if !concurrent.is_empty() {
        causes.push(format!(
            "préalable suivi la même session sans concomitance permise : {}",
            requirement_list(&concurrent)
        ));
    }
    causes
}

fn session_season(
    plan: &Plan,
    session: usize,
) -> Option<ulaval_scheduler_core::Season> {
    ulaval_scheduler_core::horizon_sessions(
        plan.start.season,
        plan.study_sessions,
    )
    .get(session.wrapping_sub(1))
    .copied()
}

fn season_name(season: ulaval_scheduler_core::Season) -> &'static str {
    match season {
        ulaval_scheduler_core::Season::Fall => "automne",
        ulaval_scheduler_core::Season::Winter => "hiver",
        ulaval_scheduler_core::Season::Summer => "été",
    }
}

// the credits the displayed grid (plus the hand-added courses) put on one
// session — what the cap judges
fn session_load(plan: &Plan, snapshot: &Snapshot, session: usize) -> u32 {
    crate::state::session_codes(plan, session)
        .iter()
        .filter_map(|code| snapshot.by_code.get(code))
        .map(|&index| snapshot.courses[index].credits.planning())
        .sum()
}

// what the student holds strictly before the session — earlier displayed
// courses and the credited codes; the same session's courses join only
// under the concomitance toggle, mirroring the solver's reading
// what counts as acquired for a course judged at `session`: strictly
// before (plus the credited), what shares the session — which only a
// starred leaf may use — and the credits earned strictly before
fn acquired_before(
    code: &str,
    session: usize,
    plan: &Plan,
    snapshot: &Snapshot,
) -> (BTreeSet<String>, BTreeSet<String>, u32) {
    let mut satisfied: BTreeSet<String> = plan
        .displayed_placement
        .iter()
        .filter(|(_, &seated)| seated < session)
        .map(|(held, _)| held.clone())
        .collect();
    satisfied.extend(plan.credited.iter().cloned());
    // a credits threshold counts strictly before the session, so the sum
    // stops here even when the concomitant codes join `satisfied` below
    let credits = satisfied
        .iter()
        .filter_map(|held| snapshot.by_code.get(held))
        .map(|&index| snapshot.courses[index].credits.planning())
        .sum();
    let same_session: BTreeSet<String> = plan
        .displayed_placement
        .iter()
        .filter(|(held, &seated)| seated == session && held.as_str() != code)
        .map(|(held, _)| held.clone())
        .collect();
    // the global toggle is a blanket dérogation now: it grants every leaf
    // what the répertoire's `*` grants a starred one
    if plan.concomitant {
        satisfied.extend(same_session.iter().cloned());
    }
    (satisfied, same_session, credits)
}

// The left-out entries whose cause has disappeared: a code no longer
// floating sits somewhere (a later answer, a hand placement) or left the
// plan — its warning has nothing left to warn about. The codes an answer
// has *just* reported still float by construction (its auto-application
// writes them nowhere), so they survive: the 6f36d0c lesson (ADR
// `2026-08-peremption-des-toasts-par-cause`).
pub fn stale_left_out(
    left_out: &BTreeSet<String>,
    still_floating: &[String],
) -> BTreeSet<String> {
    left_out
        .iter()
        .filter(|code| !still_floating.iter().any(|float| &float == code))
        .cloned()
        .collect()
}

// Nothing placed at all is a verdict of its own, never a silent empty grid
// — and the étés were already tried by the escalation, so the remaining
// levers are the cap, the sessions and the courses.
pub fn empty_grid_note() -> String {
    "Aucun cours n'a pu être placé sans briser une contrainte — la \
     grille reste vide. Montez le plafond de crédits, ajoutez des \
     sessions ou retirez des cours."
        .to_string()
}

// The escalation had to open the étés the plan keeps closed — the setting
// stays the student's, so the note explains instead of silently checking
// the box (ADR `2026-08-escalade-etes-ouverts-dans-le-repli`)
pub fn summers_forced_note(codes: &[String]) -> String {
    format!(
        "Les étés ont dû être ouverts pour tout placer : {} en été. \
         Cochez « Ouvrir les étés » pour l'assumer, ou montez le plafond \
         de crédits / ajoutez des sessions pour l'éviter.",
        codes.join(", ")
    )
}

// `Placement.blocked` surfaced by name, with the way out the student can
// act on (rapport étudiante-cegep 2026-08-19 : nommer le coupable)
pub fn blocked_note(blocked: &BlockedAnswer) -> String {
    match blocked.reason.as_str() {
        "empty-domain" => format!(
            "{} : aucune session de l'horizon ne peut l'accueillir — \
             ajoutez des sessions à l'horizon ou vérifiez les saisons où \
             il est offert.",
            blocked.code
        ),
        "unsatisfiable-prerequisites" if !blocked.missing.is_empty() => {
            format!(
                "{} : préalable manquant — il faudrait {}, ni acquis ni \
                 prévu au cheminement. Ajoutez-le aux cours à option, ou \
                 réglez-le par entente avec la direction.",
                blocked.code,
                requirement_list(&blocked.missing)
            )
        }
        "unsatisfiable-prerequisites" => format!(
            "{} : ses préalables exigent un seuil de crédits qu'aucun \
             agencement ne peut atteindre avant lui — ajoutez des cours \
             ou des sessions avant, ou réglez-le par entente avec la \
             direction.",
            blocked.code
        ),
        "stage-without-summer" => format!(
            "{} : aucun été ne peut accueillir ce stage — ouvrez les étés \
             ou épinglez-le à une session.",
            blocked.code
        ),
        other => format!("{} : {other}", blocked.code),
    }
}

// « GCI-1011 » / « ECN-2901 ou ECN-4901 » / « GCI-1011, et ECN-2901 ou
// ECN-4901 » — every missing requirement in one breath, the alternatives
// of a same requirement joined by « ou »
fn requirement_list(missing: &[Vec<String>]) -> String {
    missing
        .iter()
        .map(|alternatives| alternatives.join(" ou "))
        .collect::<Vec<_>>()
        .join(", et ")
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod worker_tests {
    use super::*;

    use crate::state::ProgramChoice;

    #[test]
    fn a_place_request_carries_the_plan_faithfully() {
        let mut plan = Plan {
            program: Some(ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: Some("Génie urbain".to_string()),
                profile: None,
            }),
            ..Plan::default()
        };
        plan.electives.push("ANL-2020".to_string());
        plan.manual.insert(3, vec!["GAE-1000".to_string()]);
        plan.pinned_sessions.insert("GCI-1000".to_string(), 2);
        plan.displayed_placement.insert("GCI-1000".to_string(), 2);
        let request = place_request(7, &plan, None, PROPOSE_MAX_NODES);
        let value: serde_json::Value =
            serde_json::from_str(&request).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(value["kind"], "place");
        assert_eq!(value["id"], 7);
        let query = &value["query"];
        assert_eq!(
            query["passed"],
            serde_json::json!([]),
            "no program in the request: nothing is acquired by hypothesis"
        );
        assert_eq!(
            query["electives"],
            serde_json::json!(["ANL-2020", "GAE-1000", "GCI-1000"]),
            "manual and laid-out courses ride as electives — a pin \
             without its Course is a PlacementError"
        );
        assert_eq!(query["pinned"]["GAE-1000"], 3, "pinned to its session");
        assert_eq!(query["pinned"]["GCI-1000"], 2);
        assert_eq!(query["start"], "fall");
        assert_eq!(query["max_solutions"], 1);
        assert_eq!(query["seed"]["GCI-1000"], 2);
        // the chosen scopes ride with the ask (décision 2026-08-19)
        assert_eq!(query["concentration"], "Génie urbain");
        assert_eq!(query["profile"], serde_json::Value::Null);
    }

    #[test]
    fn a_pin_astray_from_the_displayed_placement_still_rides_its_course() {
        // the divergent state a stale adopted answer leaves behind (the
        // BIO-1904 bug, 2026-08-26): pinned but no longer displayed — the
        // request must still carry the code as an elective, and the
        // intake must not die on it
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("GEX-1000".to_string(), 1);
        let request = place_request(9, &plan, None, PROPOSE_MAX_NODES);
        let value: serde_json::Value =
            serde_json::from_str(&request).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            value["query"]["electives"],
            serde_json::json!(["GEX-1000"]),
            "the astray pin rides as an elective, so its Course rides too"
        );
        assert_eq!(value["query"]["pinned"]["GEX-1000"], 1);

        let snapshot = snapshot();
        let unplaced = unplaced_codes(&snapshot, &plan, None)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(
            unplaced.contains(&"GEX-1000".to_string()),
            "not displayed: the code still floats, so auto-propose fires \
             and the adopted answer heals the display"
        );
    }

    #[test]
    fn the_preparatory_rule_rides_as_passed_only_while_checked() {
        let program: ulaval_scheduler_core::Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26","title":"GEX",
                "cycle":1,"credits_required":120,"mandatory":[],
                "rules":[{"title":"Scolarité préparatoire",
                          "courses":["MAT-0130","PHY-0110"]}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(preparatory_codes(&program), ["MAT-0130", "PHY-0110"]);

        // checked (the default): acquired by hypothesis, and such a code
        // can then ride neither as pin nor as elective
        let mut plan = Plan::default();
        plan.manual.insert(1, vec!["MAT-0130".to_string()]);
        plan.electives.push("MAT-0130".to_string());
        let request =
            place_request(1, &plan, Some(&program), PROPOSE_MAX_NODES);
        let value: serde_json::Value =
            serde_json::from_str(&request).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            value["query"]["passed"],
            serde_json::json!(["MAT-0130", "PHY-0110"])
        );
        assert_eq!(value["query"]["electives"], serde_json::json!([]));
        assert!(value["query"]["pinned"]
            .as_object()
            .is_some_and(|pinned| pinned.is_empty()));

        // unchecked: the préparatoire courses are ordinary work to place
        plan.preparatory_done = false;
        let request =
            place_request(1, &plan, Some(&program), PROPOSE_MAX_NODES);
        let value: serde_json::Value =
            serde_json::from_str(&request).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(value["query"]["passed"], serde_json::json!([]));
        assert_eq!(
            value["query"]["electives"],
            serde_json::json!(["MAT-0130"])
        );

        // a program whose préparatoire rule is absent or not a plain list
        let bare: ulaval_scheduler_core::Program = serde_json::from_str(
            r#"{"code":"X","slug":"x","semester":"A26","title":"X",
                "cycle":1,"credits_required":6,"mandatory":[],
                "rules":[{"title":"Scolarité préparatoire",
                          "courses":"negotiated","raw":"à convenir"}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(preparatory_codes(&bare).is_empty());
    }

    fn snapshot() -> crate::data::Snapshot {
        let courses = r#"{"courses":[
          {"code":"ANL-1010","title":"Anglais","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"winter":{"last_offered":2026,"options":null}}},
          {"code":"GEX-1000","title":"Hydrologie","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}},
          {"code":"GEX-2000","title":"Hydraulique","credits":4,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}}
        ]}"#;
        crate::data::parse_data(
            &crate::data::RawData {
                courses: courses.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn snapshot_with_preparatory() -> crate::data::Snapshot {
        let courses = r#"{"courses":[
          {"code":"GEX-1000","title":"Hydrologie","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}},
          {"code":"GEX-2000","title":"Hydraulique","credits":4,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}}
        ]}"#;
        let program = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
            "title":"P","cycle":1,"credits_required":6,"mandatory":[],
            "rules":[{"title":"Scolarité préparatoire",
                      "courses":["GEX-1000"]}],
            "concentrations":[],"profiles":[]}"#;
        crate::data::parse_data(
            &crate::data::RawData {
                courses: courses.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: vec![(
                    "B-GEX-A26.json".to_string(),
                    program.to_string(),
                )],
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn gex_choice() -> ProgramChoice {
        ProgramChoice {
            code: "B-GEX".to_string(),
            semester: "A26".to_string(),
            concentration: None,
            profile: None,
        }
    }

    #[test]
    fn acquired_preparatory_follows_the_box_the_program_and_the_grants() {
        let snapshot = snapshot_with_preparatory();
        let mut plan = Plan::default();
        // no program chosen: nothing is acquired by hypothesis
        assert!(acquired_preparatory(&snapshot, &plan).is_empty());
        plan.program = Some(gex_choice());
        assert_eq!(
            acquired_preparatory(&snapshot, &plan),
            BTreeSet::from(["GEX-1000".to_string()])
        );
        // an entente moved the code into another rule: ordinary work again
        plan.rule_grants
            .insert("GEX-1000".to_string(), "p/Règle 1".to_string());
        assert!(acquired_preparatory(&snapshot, &plan).is_empty());
        plan.rule_grants.clear();
        plan.preparatory_done = false;
        assert!(acquired_preparatory(&snapshot, &plan).is_empty());
    }

    #[test]
    fn acquired_leftovers_find_the_code_in_every_placement_structure() {
        let program: ulaval_scheduler_core::Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26","title":"P",
                "cycle":1,"credits_required":6,"mandatory":[],
                "rules":[{"title":"Scolarité préparatoire",
                          "courses":["MAT-0130","PHY-0110","CHM-0100",
                                     "BIO-0150","MAT-0110"]}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = Plan::default();
        assert!(acquired_leftovers(&plan, &program).is_empty());
        plan.displayed_placement.insert("MAT-0130".to_string(), 1);
        plan.pinned_sessions.insert("PHY-0110".to_string(), 2);
        plan.electives.push("CHM-0100".to_string());
        plan.manual.insert(1, vec!["BIO-0150".to_string()]);
        plan.chosen.insert(
            1,
            BTreeMap::from([("MAT-0110".to_string(), BTreeSet::new())]),
        );
        assert_eq!(
            acquired_leftovers(&plan, &program),
            ["MAT-0130", "PHY-0110", "CHM-0100", "BIO-0150", "MAT-0110"]
        );
        // unchecked: the same occupations are legitimate work
        plan.preparatory_done = false;
        assert!(acquired_leftovers(&plan, &program).is_empty());
        // a credited course is acquired the same way, box or no box
        plan.credited.insert("CHM-0100".to_string());
        assert_eq!(acquired_leftovers(&plan, &program), ["CHM-0100"]);
    }

    #[test]
    fn adding_an_acquired_preparatory_course_is_refused_with_the_way_out() {
        let snapshot = snapshot_with_preparatory();
        let mut plan = Plan {
            program: Some(gex_choice()),
            ..Plan::default()
        };
        let error = validate_new_code(&snapshot, &plan, Some(1), "gex-1000")
            .expect_err("acquired by the checked box");
        assert!(error.contains("décochez la case"), "{error}");
        // unchecked: the ordinary gates apply and the course is welcome
        plan.preparatory_done = false;
        let accepted =
            validate_new_code(&snapshot, &plan, Some(1), "gex-1000")
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(accepted.code, "GEX-1000");
        // credited by an agreement: refused too, with its own way out
        plan.credited.insert("GEX-1000".to_string());
        let error = validate_new_code(&snapshot, &plan, Some(1), "gex-1000")
            .expect_err("credited holds no session");
        assert!(error.contains("retirez le crédit"), "{error}");
    }

    #[test]
    fn passed_codes_gather_both_families_and_the_purge_note_names_the_way_out()
    {
        let snapshot = snapshot_with_preparatory();
        let program = crate::panel::chosen_program(
            &snapshot,
            &Plan {
                program: Some(gex_choice()),
                ..Plan::default()
            },
        )
        .cloned()
        .unwrap_or_else(|| panic!("the GEX snapshot carries its program"));
        let mut plan = Plan::default();
        // the box is checked by default, so the préparatoire rides alone
        assert_eq!(passed_codes(&plan, Some(&program)), ["GEX-1000"]);
        // a credited course joins it, and the already-passed one is not
        // repeated when an entente credits it too
        plan.credited.insert("GEX-1000".to_string());
        plan.credited.insert("GEX-2000".to_string());
        assert_eq!(
            passed_codes(&plan, Some(&program)),
            ["GEX-1000", "GEX-2000"]
        );
        plan.preparatory_done = false;
        assert_eq!(
            passed_codes(&plan, Some(&program)),
            ["GEX-1000", "GEX-2000"]
        );
        assert_eq!(passed_codes(&Plan::default(), None), Vec::<String>::new());
        // the note names the control that undoes the purge, in both
        // families and both numbers
        let one = ["GEX-1000".to_string()];
        let two = ["GEX-1000".to_string(), "GEX-2000".to_string()];
        assert!(purge_note(&one, true).contains("crédité par entente"));
        assert!(purge_note(&one, true).contains("pour le replacer"));
        assert!(purge_note(&two, true).contains("crédités par entente"));
        assert!(purge_note(&two, true).contains("pour les replacer"));
        assert!(purge_note(&one, false).contains("Décochez la case"));
        assert!(purge_note(&one, false).contains("retiré des sessions"));
        assert!(purge_note(&two, false).contains("retirés des sessions"));
    }

    #[test]
    fn unplaced_codes_name_what_still_floats_and_carry_intake_errors() {
        let snapshot = snapshot();
        let program: ulaval_scheduler_core::Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26","title":"GEX",
                "cycle":1,"credits_required":120,
                "mandatory":["GEX-1000","GEX-2000"],
                "rules":[],"concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = Plan::default();
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        plan.manual.insert(2, vec!["ANL-1010".to_string()]);
        let unplaced = unplaced_codes(&snapshot, &plan, Some(&program))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(unplaced, ["GEX-2000"], "placed and manual are covered");

        plan.displayed_placement.insert("GEX-2000".to_string(), 3);
        let unplaced = unplaced_codes(&snapshot, &plan, Some(&program))
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(unplaced.is_empty(), "{unplaced:?}");

        plan.electives.push("ZZZ-9999".to_string());
        let error = unplaced_codes(&snapshot, &plan, Some(&program))
            .expect_err("a typo'd code is an intake error");
        assert!(error.contains("ZZZ-9999"), "{error}");

        // préparatoire unchecked: nothing rides as passed anymore — and an
        // elective already covering a manual code is not doubled
        plan.electives.clear();
        plan.electives.push("ANL-1010".to_string());
        plan.preparatory_done = false;
        let unplaced = unplaced_codes(&snapshot, &plan, Some(&program))
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(unplaced.is_empty(), "{unplaced:?}");
    }

    #[test]
    fn unplaced_codes_include_the_chosen_concentrations_mandatory() {
        let snapshot = snapshot();
        let program: ulaval_scheduler_core::Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26","title":"GEX",
                "cycle":1,"credits_required":120,
                "mandatory":["GEX-1000"],"rules":[],
                "concentrations":[{"title":"Urbain",
                                   "mandatory":["GEX-2000"],"rules":[]}],
                "profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = Plan {
            program: Some(ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: None,
                profile: None,
            }),
            ..Plan::default()
        };
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        let unplaced = unplaced_codes(&snapshot, &plan, Some(&program))
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(
            unplaced.is_empty(),
            "unchosen, the concentration asks nothing: {unplaced:?}"
        );

        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Urbain".to_string());
        }
        let unplaced = unplaced_codes(&snapshot, &plan, Some(&program))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(unplaced, ["GEX-2000"], "chosen, its mandatory floats");
    }

    #[test]
    fn a_verify_request_pins_the_whole_displayed_placement() {
        let mut plan = Plan::default();
        // one placed course is already an elective: it must ride once,
        // never twice
        plan.electives.push("GEX-1000".to_string());
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        plan.displayed_placement.insert("GEX-2000".to_string(), 4);
        let request = verify_request(2, &plan, None);
        let value: serde_json::Value =
            serde_json::from_str(&request).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(value["kind"], "verify");
        assert_eq!(value["query"]["pinned"]["GEX-1000"], 1);
        assert_eq!(value["query"]["pinned"]["GEX-2000"], 4);
        assert_eq!(
            value["query"]["electives"],
            serde_json::json!(["GEX-1000", "GEX-2000"])
        );
    }

    #[test]
    fn every_worker_answer_shape_parses() {
        let ready = parse_worker_answer(
            r#"{"kind":"ready","id":0,
                 "summary":{"course_count":8834,"collisions":[]}}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(
            ready,
            WorkerAnswer::Ready { summary, .. } if summary.course_count == 8834
        ));

        let report = parse_worker_answer(
            r#"{"kind":"report","id":5,"report":{
                 "sessions":["fall","winter","summer"],
                 "placement":{"completion":"complete",
                   "solutions":[{"placement":{"GEX-1000":1},
                                 "assumed":["MAT-0130"]}],
                   "blocked":[]},
                 "set_aside":["GHOST-1"]}}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let WorkerAnswer::Report { id, report } = report else {
            panic!("expected a report");
        };
        assert_eq!(id, 5);
        assert_eq!(report.sessions.len(), 3);
        assert_eq!(report.placement.solutions[0].placement["GEX-1000"], 1);
        assert_eq!(report.set_aside, ["GHOST-1"]);

        let error =
            parse_worker_answer(r#"{"kind":"error","id":9,"message":"boom"}"#)
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(error, WorkerAnswer::Error { id: 9, .. }));

        assert!(parse_worker_answer("pas du json")
            .expect_err("must fail")
            .contains("illisible"));
    }

    #[test]
    fn stale_left_out_keeps_what_still_floats() {
        let left_out =
            BTreeSet::from(["ANL-1010".to_string(), "GMN-2902".to_string()]);
        // ANL-1010 still floats (the answer that reported it did not seat
        // it) — only GMN-2902, placed or removed since, is stale
        let stale = stale_left_out(&left_out, &["ANL-1010".to_string()]);
        assert_eq!(stale, BTreeSet::from(["GMN-2902".to_string()]));
        // nothing floating: everything retires
        let stale = stale_left_out(&left_out, &[]);
        assert_eq!(stale, left_out);
    }

    #[test]
    fn the_summers_forced_note_names_the_codes_and_the_levers() {
        let note = summers_forced_note(&[
            "GEX-1002".to_string(),
            "GMC-2580".to_string(),
        ]);
        assert!(note.contains("GEX-1002, GMC-2580"), "{note}");
        assert!(note.contains("Ouvrir les étés"), "{note}");
        assert!(note.contains("plafond"), "{note}");
    }

    #[test]
    fn the_credit_shortfall_message_is_persistent_french_copy() {
        let plan = Plan::default();
        let shortfall = CreditShortfallAnswer {
            code: "GCI-3333".to_string(),
            session: 2,
            earned_before: 54,
            required: 60,
        };
        assert_eq!(
            credit_shortfall_message(&shortfall, &plan),
            "GCI-3333 est placé en H27 avec 54 crédits acquis avant cette \
             session; le minimum est 60 crédits. Le solveur l'a placé au \
             plus tard disponible. Répartissez 6 crédits de plus avant H27 \
             ou déplacez le cours."
        );
        assert_eq!(
            course_shortfall_messages("GCI-3333", &[shortfall], &plan).len(),
            1
        );
        let outside = CreditShortfallAnswer {
            code: "GCI-3333".to_string(),
            session: 99,
            earned_before: 60,
            required: 60,
        };
        assert!(
            credit_shortfall_message(&outside, &plan).contains("session 99")
        );
    }

    // The best-effort messages: one line per code, each with its own
    // cause, and the completion note silenced — it describes the *relaxed*
    // enumeration and would contradict the grid (ADR
    // `2026-08-placement-au-mieux-en-repli`).
    #[test]
    fn a_best_effort_answer_names_what_it_left_out() {
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("ANL-1010".to_string(), 1);
        // a pinned course names the very session the student chose —
        // never « chaque session » when he chose exactly one; without a
        // snapshot to diagnose against, the honest remainder stands
        let line = left_out_line("ANL-1010", None, &plan, None);
        assert!(line.contains("que vous avez épinglée"), "{line}");
        assert!(line.starts_with("ANL-1010"), "{line}");
        // the one only the search could rule out gets the honest default
        // rather than an invented reason
        let line = left_out_line("MAT-0130", None, &plan, None);
        assert!(line.contains("aucune place ne restait"), "{line}");
        // the pre-screen's reason is more precise than the pin
        let blocked = BlockedAnswer {
            code: "ANL-1010".to_string(),
            reason: "empty-domain".to_string(),
            missing: Vec::new(),
        };
        let line = left_out_line("ANL-1010", Some(&blocked), &plan, None);
        assert!(line.contains("aucune session de l'horizon"), "{line}");
        // nothing placed at all is a verdict of its own, never a silent
        // empty grid — the étés were already escalated, so the note names
        // the levers that remain
        let note = empty_grid_note();
        assert!(note.starts_with("Aucun cours n'a pu être placé"), "{note}");
        assert!(note.contains("plafond"), "{note}");

        // the completion note must not contradict a filled grid
        let filled = PlacementAnswer {
            completion: "solution-cap".to_string(),
            solutions: vec![SolutionAnswer {
                placement: BTreeMap::from([("GEX-1000".to_string(), 1)]),
                assumed: BTreeSet::new(),
                left_out: BTreeSet::from(["GEX-1002".to_string()]),
                credit_shortfalls: Vec::new(),
            }],
            blocked: Vec::new(),
        };
        assert!(completion_note(&filled).is_none());
        // an exact answer is untouched: the completion note still does its
        // old job — whether it placed everything...
        let complete = PlacementAnswer {
            solutions: vec![SolutionAnswer {
                placement: BTreeMap::from([("GEX-1000".to_string(), 1)]),
                assumed: BTreeSet::new(),
                left_out: BTreeSet::new(),
                credit_shortfalls: Vec::new(),
            }],
            ..filled.clone()
        };
        assert!(completion_note(&complete).is_some());
        // ...or nothing at all
        let exact = PlacementAnswer {
            completion: "node-budget".to_string(),
            solutions: Vec::new(),
            blocked: Vec::new(),
        };
        assert!(completion_note(&exact).is_some());
    }

    #[test]
    fn a_pinned_refusal_names_the_checkable_causes() {
        // a dedicated catalogue: GEX-2000 needs an unknown code, HIV-1000
        // is winter-only — enough to trip each checkable cause on demand
        let courses = r#"{"courses":[
          {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}},
          {"code":"GEX-2000","title":"T","credits":4,"cycle":1,
           "prerequisites":{"raw":"GEX-1000 ET GEX-9999",
                            "tree":{"all":["GEX-1000","GEX-9999"]}},
           "equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}},
          {"code":"HIV-1000","title":"T","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"winter":{"last_offered":2026,"options":null}}}
        ]}"#;
        let snapshot = crate::data::parse_data(
            &crate::data::RawData {
                courses: courses.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));

        // session 1 is an automne (Plan::default starts Fall): the
        // winter-only course names the season
        let mut plan = Plan {
            credit_cap: 3,
            ..Plan::default()
        };
        plan.pinned_sessions.insert("HIV-1000".to_string(), 1);
        plan.displayed_placement.insert("HIV-1000".to_string(), 1);
        let line = left_out_line("HIV-1000", None, &plan, Some(&snapshot));
        assert!(line.contains("pas offert en automne"), "{line}");

        // over-cap and missing prerequisite, both named with their facts
        let mut plan = Plan {
            credit_cap: 3,
            ..Plan::default()
        };
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        plan.pinned_sessions.insert("GEX-2000".to_string(), 1);
        plan.displayed_placement.insert("GEX-2000".to_string(), 1);
        let line = left_out_line("GEX-2000", None, &plan, Some(&snapshot));
        assert!(line.contains("plafond de 3 cr"), "{line}");
        assert!(line.contains("7 cr posés"), "{line}");
        // GEX-1000 sits in the very session judged and GEX-9999 nowhere:
        // two different facts, named apart
        assert!(
            line.contains("préalable manquant avant cette session : GEX-9999"),
            "{line}"
        );
        assert!(
            line.contains(
                "préalable suivi la même session sans concomitance \
                 permise : GEX-1000"
            ),
            "{line}"
        );

        // nothing checkable at fault: the honest remainder is the horaire
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("GEX-1000".to_string(), 1);
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        let line = left_out_line("GEX-1000", None, &plan, Some(&snapshot));
        assert!(line.contains("aucune combinaison d'horaire"), "{line}");

        // a code the snapshot does not carry falls back the same way
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("ZZZ-9999".to_string(), 1);
        let line = left_out_line("ZZZ-9999", None, &plan, Some(&snapshot));
        assert!(line.contains("aucune combinaison d'horaire"), "{line}");

        // every season speaks its own name: session 2 is an hiver,
        // session 3 an été (the horizon always carries them)
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("GEX-1000".to_string(), 2);
        plan.displayed_placement.insert("GEX-1000".to_string(), 2);
        let line = left_out_line("GEX-1000", None, &plan, Some(&snapshot));
        assert!(line.contains("pas offert en hiver"), "{line}");
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("GEX-1000".to_string(), 3);
        plan.displayed_placement.insert("GEX-1000".to_string(), 3);
        let line = left_out_line("GEX-1000", None, &plan, Some(&snapshot));
        assert!(line.contains("pas offert en été"), "{line}");

        // a prerequisite seated a session *earlier* is held (its credits
        // counted): only the truly unknown GEX-9999 stays blamed
        let mut plan = Plan::default();
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        plan.pinned_sessions.insert("GEX-2000".to_string(), 2);
        plan.displayed_placement.insert("GEX-2000".to_string(), 2);
        let line = left_out_line("GEX-2000", None, &plan, Some(&snapshot));
        assert!(!line.contains("GEX-1000, et"), "{line}");
        assert!(line.contains("GEX-9999"), "{line}");

        // a pin beyond the horizon has no season to accuse — the honest
        // remainder stands (a corrupt save, not a reachable state)
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("GEX-1000".to_string(), 99);
        let line = left_out_line("GEX-1000", None, &plan, Some(&snapshot));
        assert!(line.contains("aucune combinaison d'horaire"), "{line}");

        // the concomitance toggle reads a same-session prerequisite as
        // held — mirroring the solver: GEX-1000 leaves the blame, the
        // truly unknown GEX-9999 stays
        let mut plan = Plan {
            concomitant: true,
            ..Plan::default()
        };
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        plan.pinned_sessions.insert("GEX-2000".to_string(), 1);
        plan.displayed_placement.insert("GEX-2000".to_string(), 1);
        let line = left_out_line("GEX-2000", None, &plan, Some(&snapshot));
        assert!(!line.contains("GEX-1000, et"), "{line}");
        assert!(line.contains("GEX-9999"), "{line}");
    }

    #[test]
    fn completion_and_blocked_speak_french() {
        let answer = |completion: &str, solutions: usize| PlacementAnswer {
            completion: completion.to_string(),
            solutions: (0..solutions)
                .map(|_| SolutionAnswer {
                    placement: BTreeMap::new(),
                    assumed: BTreeSet::new(),
                    left_out: BTreeSet::new(),
                    credit_shortfalls: Vec::new(),
                })
                .collect(),
            blocked: Vec::new(),
        };
        assert!(completion_note(&answer("complete", 1)).is_none());
        assert!(completion_note(&answer("complete", 0))
            .expect("an empty proof speaks")
            .contains("c'est certain"));
        assert!(completion_note(&answer("node-budget", 1))
            .expect("truncation speaks")
            .contains("avant d'avoir tout exploré"));
        assert!(completion_note(&answer("node-budget", 0))
            .expect("an empty truncated answer says which of the two")
            .contains("sans rien trouver"));
        assert!(completion_note(&answer("solution-cap", 1))
            .expect("cap speaks")
            .contains("agencements"));

        let note = |reason: &str| {
            blocked_note(&BlockedAnswer {
                code: "GEX-1580".to_string(),
                reason: reason.to_string(),
                missing: Vec::new(),
            })
        };
        assert!(note("empty-domain").contains("aucune session"));
        assert!(note("unsatisfiable-prerequisites").contains("préalables"));
        assert!(note("stage-without-summer").contains("été"));
        // the proof named: alternatives joined by « ou », requirements by
        // « , et » (« GEX-3333 : préalable manquant — il faudrait … »,
        // retour d'Antoine 2026-08-26)
        let named = blocked_note(&BlockedAnswer {
            code: "GEX-3333".to_string(),
            reason: "unsatisfiable-prerequisites".to_string(),
            missing: vec![
                vec!["ECN-2901".to_string(), "ECN-4901".to_string()],
                vec!["GCI-1011".to_string()],
            ],
        });
        assert!(
            named.contains("ECN-2901 ou ECN-4901, et GCI-1011"),
            "{named}"
        );
        assert!(named.contains("préalable manquant"), "{named}");
        assert!(note("autre-chose").contains("autre-chose"));
    }
}
