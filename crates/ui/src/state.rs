use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{PrereqOverride, Season, Semester};

pub const HISTORY_CAP: usize = 100;
// 17 gives every bac headroom out of the box: the B-GMC packs ~117 cr of
// mandatories and rules into 8 sessions, which 8 × 15 cannot hold, and its
// official cheminement itself opens at 16 cr (ADR
// `2026-08-plafond-par-defaut-17-credits`). Still a setting, never a wall.
pub const DEFAULT_CREDIT_CAP: u32 = 17;
// A1→H8 : the bac's eight study sessions (étés come on top, in core)
pub const DEFAULT_STUDY_SESSIONS: usize = 8;

// The student's whole document — only codes, indices and choices, never
// Course data (the snapshot owns that). One struct so undo is one clone
// and persistence one write. Field-level `serde(default)` is the restore
// tolerance: an old save missing a field starts that field fresh.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct Plan {
    pub program: Option<ProgramChoice>,
    // the A1 identity — « A26 » = automne 2026
    pub start: Semester,
    pub study_sessions: usize,
    pub summers_open: bool,
    pub credit_cap: u32,
    pub concomitant: bool,
    // « scolarité préparatoire faite » — the 0xxx courses ride as
    // `PlaceQuery.passed` so the solver neither places nor counts them; a
    // course the student really took lives in its past session instead (ADR
    // `2026-08-retrait-de-la-notion-de-cours-reussi`)
    pub preparatory_done: bool,
    // typed additions to the program's list, in the order they were added
    pub electives: Vec<String>,
    // code → 1-based session: the student's explicit organigramme acts
    pub pinned_sessions: BTreeMap<String, usize>,
    // the organigramme as shown: pins plus the last accepted proposal —
    // also the seed of the next solve, so proposals stay close to it
    pub displayed_placement: BTreeMap<String, usize>,
    // session → code → pinned NRC set (the weekly « forcer une section »)
    pub chosen: BTreeMap<usize, BTreeMap<String, BTreeSet<String>>>,
    // session → codes added by hand outside the placement (v0 flow)
    pub manual: BTreeMap<usize, Vec<String>>,
    // session → free label (« à l'étranger ») — pure annotation
    pub special: BTreeMap<usize, String>,
    // ententes avec la direction : code → section key (« p/Règle 2 ») of
    // the rule the course counts toward; pure data, applied by
    // `panel::granted_program` before any coverage call
    pub rule_grants: BTreeMap<String, String>,
    // cours crédités par entente : acquired in advance, so they ride as
    // `PlaceQuery.passed` like the préparatoire ones — counted in the
    // credits and in the coverage, never given a session (ADR
    // `2026-08-cours-credite-hors-session`)
    pub credited: BTreeSet<String>,
    // code → the prerequisites as the student's own program vintage wrote
    // them, when they differ from today's répertoire. Layered *over* the
    // vintage file's own corrections, and applied to the catalogue before
    // the solver ever reads it (ADR
    // `2026-08-correction-des-prealables-par-millesime`)
    pub prereq_overrides: BTreeMap<String, PrereqOverride>,
}

impl Default for Plan {
    fn default() -> Self {
        Plan {
            program: None,
            start: Semester {
                season: Season::Fall,
                year: 2026,
            },
            study_sessions: DEFAULT_STUDY_SESSIONS,
            summers_open: false,
            credit_cap: DEFAULT_CREDIT_CAP,
            concomitant: false,
            preparatory_done: true,
            electives: Vec::new(),
            pinned_sessions: BTreeMap::new(),
            displayed_placement: BTreeMap::new(),
            chosen: BTreeMap::new(),
            manual: BTreeMap::new(),
            special: BTreeMap::new(),
            rule_grants: BTreeMap::new(),
            credited: BTreeSet::new(),
            prereq_overrides: BTreeMap::new(),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct ProgramChoice {
    // official code, « B-GEX »
    pub code: String,
    // the vintage the student enrolled under, « A26 » — names the snapshot
    pub semester: String,
    pub concentration: Option<String>,
    pub profile: Option<String>,
}

// The seed of a new document: everything at its default, only the
// student's calendar identity (`start`) carried over — the cap, the
// sessions and the étés are facts of the program being opened (ADR
// `2026-08-reglages-transversaux-dans-linstantane`).
pub fn fresh_plan(start: Semester, choice: ProgramChoice) -> Plan {
    Plan {
        program: Some(choice),
        start,
        ..Plan::default()
    }
}

// Ephemeral-but-restored navigation state: never undoable (undo acts on
// the document, not on where the student is looking), fully persisted so a
// reload lands exactly where he was (ACT-7).
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct View {
    // 1-based index in the horizon
    pub session: usize,
    pub search: String,
    pub subject: Option<String>,
    pub only_fitting: bool,
    pub expanded_rule: Option<String>,
}

impl Default for View {
    fn default() -> Self {
        View {
            session: 1,
            search: String::new(),
            subject: None,
            only_fitting: false,
            expanded_rule: None,
        }
    }
}

// --- the session identity walk -------------------------------------------

// the calendar arithmetic moved to core so the wasm surface serves the same
// walk to the JS interface (ADR `2026-08-surface-wasm-etendue-a-huit-fonctions`)
pub use ulaval_scheduler_core::session_semesters;

// the `schedule_intake` naming — « a2026 », « h2027 », « e2027 »
pub fn session_key(semester: Semester) -> String {
    let letter = match semester.season {
        Season::Fall => 'a',
        Season::Winter => 'h',
        Season::Summer => 'e',
    };
    format!("{letter}{}", semester.year)
}

// The ribbon label: « A3-A27 » — the ordinal counts study sessions only,
// an été carries its vintage alone (« É27 »). Total on any index: an
// out-of-range one answers « ? » instead of panicking.
pub fn session_label(semesters: &[Semester], index: usize) -> String {
    let Some(&semester) = semesters.get(index) else {
        return "?".to_string();
    };
    if semester.season == Season::Summer {
        // « É27 » on screen; the plain-ascii « E27 » stays a file/key thing
        return format!("É{:02}", semester.year % 100);
    }
    let ordinal = semesters[..=index]
        .iter()
        .filter(|semester| semester.season != Season::Summer)
        .count();
    // the été returned above: only automne and hiver carry an ordinal
    let letter = if semester.season == Season::Fall {
        'A'
    } else {
        'H'
    };
    format!("{letter}{ordinal}-{semester}")
}

// « A3 » / « H4 » — the ordinal alone, for lines that already spell the
// semester out (« Horaire - H4 — Hiver 2026 », note 15) ; an été keeps
// its bare « É27 » label
pub fn session_short(semesters: &[Semester], index: usize) -> String {
    let label = session_label(semesters, index);
    label
        .split_once('-')
        .map(|(short, _)| short.to_string())
        .unwrap_or(label)
}

// The semester the real-world clock is in — month 1–4 is an hiver, 5–8 an
// été, 9–12 an automne. Days-to-civil is Howard Hinnant's algorithm,
// exact over the whole epoch range.
pub fn semester_of_epoch_ms(epoch_ms: u64) -> Semester {
    let days = (epoch_ms / 86_400_000) as i64;
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        / 365;
    let day_of_year =
        day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let mut year = year_of_era + era * 400;
    if month <= 2 {
        year += 1;
    }
    let season = match month {
        1..=4 => Season::Winter,
        5..=8 => Season::Summer,
        _ => Season::Fall,
    };
    Semester {
        season,
        year: year.clamp(0, i64::from(u16::MAX)) as u16,
    }
}

// Where a semester falls in real time — hiver, été, automne within a civil
// year. Ordering by this beats ordering by the « A26 » spelling, which sorts
// every automne before every hiver.
pub fn semester_rank(semester: Semester) -> (u16, u8) {
    let season = match semester.season {
        Season::Winter => 0u8,
        Season::Summer => 1,
        Season::Fall => 2,
    };
    (semester.year, season)
}

// strictly before, in real time
pub fn semester_precedes(before: Semester, after: Semester) -> bool {
    semester_rank(before) < semester_rank(after)
}

// --- what one session holds ----------------------------------------------

// The codes the weekly schedule of `session` draws: the displayed
// placement's courses there, then the hand-added ones (v0 flow), in a
// stable order, without duplicates.
pub fn session_codes(plan: &Plan, session: usize) -> Vec<String> {
    let mut codes: Vec<String> = plan
        .displayed_placement
        .iter()
        .filter(|(_, &placed)| placed == session)
        .map(|(code, _)| code.clone())
        .collect();
    for code in plan.manual.get(&session).into_iter().flatten() {
        if !codes.contains(code) {
            codes.push(code.clone());
        }
    }
    codes
}

// Créditer : the course is held by agreement, so it leaves every session —
// but keeps its entente: the credit says the course is held, the grant says
// which rule it counts toward.
pub fn credit_code(plan: &mut Plan, code: &str) {
    let code = code.to_string();
    purge_codes(plan, std::slice::from_ref(&code));
    plan.credited.insert(code);
}

// Décréditer : the useful inverse of the credit. The pre-credit session is
// gone with the purge, so the course re-enters as an elective — counted
// again at once, the solver gives it a session on the next propose (ADR
// `2026-08-decrediter-reprend-le-cours-en-electif`).
pub fn uncredit_code(plan: &mut Plan, code: &str) {
    plan.credited.remove(code);
    if !plan.electives.iter().any(|held| held == code) {
        plan.electives.push(code.to_string());
    }
}

// The voluntary ✕ : every placement trace goes, and the entente with it —
// unlike the credit purge, dropping the course drops the agreement too.
pub fn remove_course(plan: &mut Plan, code: &str) {
    let code = code.to_string();
    purge_codes(plan, std::slice::from_ref(&code));
    plan.rule_grants.remove(&code);
}

// Strip the given codes from every placement structure — the derived
// correction behind « scolarité préparatoire faite » and « crédité » : a
// code acquired by hypothesis must not occupy any session, wherever it
// slipped in from.
pub fn purge_codes(plan: &mut Plan, codes: &[String]) {
    plan.electives.retain(|code| !codes.contains(code));
    plan.pinned_sessions.retain(|code, _| !codes.contains(code));
    plan.displayed_placement
        .retain(|code, _| !codes.contains(code));
    for held in plan.manual.values_mut() {
        held.retain(|code| !codes.contains(code));
    }
    for pins in plan.chosen.values_mut() {
        pins.retain(|code, _| !codes.contains(code));
    }
}

// Changing the concentration (prefix 'c') or the profile ('f') retires the
// ententes attached to the outgoing block: an entente names a rule of that
// block, and the same title under another block is another rule. The
// dropped codes come back for the caller to announce — never a silent loss
// (décision 2026-08-19).
pub fn purge_scope_grants(plan: &mut Plan, prefix: char) -> Vec<String> {
    let scoped = format!("{prefix}/");
    let dropped: Vec<String> = plan
        .rule_grants
        .iter()
        .filter(|(_, key)| key.starts_with(&scoped))
        .map(|(code, _)| code.clone())
        .collect();
    plan.rule_grants.retain(|_, key| !key.starts_with(&scoped));
    dropped
}

// --- undo/redo ------------------------------------------------------------

// ACT-2: every mutation of the Plan goes through `apply`, so every one is
// reversible and labelled — no confirmation dialog anywhere.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct History {
    undo: Vec<(String, Plan)>,
    redo: Vec<(String, Plan)>,
}

impl History {
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|(label, _)| label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|(label, _)| label.as_str())
    }
}

pub fn apply(
    plan: &mut Plan,
    history: &mut History,
    label: &str,
    edit: impl FnOnce(&mut Plan),
) {
    history.undo.push((label.to_string(), plan.clone()));
    if history.undo.len() > HISTORY_CAP {
        history.undo.remove(0);
    }
    history.redo.clear();
    edit(plan);
}

pub fn undo(plan: &mut Plan, history: &mut History) -> Option<String> {
    let (label, previous) = history.undo.pop()?;
    history.redo.push((label.clone(), plan.clone()));
    *plan = previous;
    Some(label)
}

pub fn redo(plan: &mut Plan, history: &mut History) -> Option<String> {
    let (label, next) = history.redo.pop()?;
    history.undo.push((label.clone(), plan.clone()));
    *plan = next;
    Some(label)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn semester(raw: &str) -> Semester {
        raw.parse().unwrap_or_else(|e| panic!("{e}"))
    }

    const HORIZON: [Season; 6] = [
        Season::Fall,
        Season::Winter,
        Season::Summer,
        Season::Fall,
        Season::Winter,
        Season::Summer,
    ];

    #[test]
    fn a_fresh_plan_carries_only_the_start_and_the_choice() {
        let start = semester("H27");
        let choice = ProgramChoice {
            code: "B-GIN".to_string(),
            semester: "H27".to_string(),
            concentration: Some("Approche généraliste".to_string()),
            profile: None,
        };
        let fresh = fresh_plan(start, choice.clone());
        assert_eq!(fresh.program, Some(choice));
        assert_eq!(fresh.start, start);
        // everything else is the default — field by field, so a new Plan
        // field forgetting this contract fails here
        assert_eq!(
            Plan {
                program: None,
                start: Plan::default().start,
                ..fresh
            },
            Plan::default()
        );
    }

    #[test]
    fn session_keys_speak_the_intake_naming() {
        let semesters = session_semesters(semester("A26"), &HORIZON);
        let keys: Vec<String> =
            semesters.iter().map(|&s| session_key(s)).collect();
        assert_eq!(
            keys,
            ["a2026", "h2027", "e2027", "a2027", "h2028", "e2028"]
        );
    }

    #[test]
    fn labels_number_study_sessions_and_leave_etes_bare() {
        let semesters = session_semesters(semester("A26"), &HORIZON);
        let labels: Vec<String> = (0..semesters.len())
            .map(|i| session_label(&semesters, i))
            .collect();
        assert_eq!(
            labels,
            ["A1-A26", "H2-H27", "É27", "A3-A27", "H4-H28", "É28"]
        );
        assert_eq!(session_label(&semesters, 99), "?", "total, no panic");
        let shorts: Vec<String> = (0..semesters.len())
            .map(|i| session_short(&semesters, i))
            .collect();
        assert_eq!(shorts, ["A1", "H2", "É27", "A3", "H4", "É28"]);
        assert_eq!(session_short(&semesters, 99), "?", "total, no panic");
    }

    #[test]
    fn the_clock_semester_names_the_three_seasons_and_leap_days() {
        let at = |ms: u64| semester_of_epoch_ms(ms).to_string();
        assert_eq!(at(0), "H70", "1970-01-01");
        assert_eq!(at(1_786_579_200_000), "E26", "2026-08-13");
        assert_eq!(at(1_788_220_800_000), "A26", "2026-09-01");
        assert_eq!(at(1_798_675_200_000), "A26", "2026-12-31");
        assert_eq!(at(1_799_107_200_000), "H27", "2027-01-05");
        assert_eq!(at(1_709_164_800_000), "H24", "2024-02-29 (bissextile)");
    }

    #[test]
    fn semesters_precede_in_civil_time_hiver_ete_automne() {
        let order = ["H26", "E26", "A26", "H27"].map(semester);
        for (i, &earlier) in order.iter().enumerate() {
            assert!(!semester_precedes(earlier, earlier), "irreflexive");
            for &later in &order[i + 1..] {
                assert!(semester_precedes(earlier, later));
                assert!(!semester_precedes(later, earlier));
            }
        }
    }

    #[test]
    fn purge_codes_strips_the_named_codes_from_every_structure() {
        let mut plan = Plan {
            electives: vec!["MAT-0130".to_string(), "GEX-1000".to_string()],
            pinned_sessions: BTreeMap::from([("MAT-0130".to_string(), 1)]),
            displayed_placement: BTreeMap::from([
                ("MAT-0130".to_string(), 1),
                ("GEX-1000".to_string(), 2),
            ]),
            chosen: BTreeMap::from([(
                1,
                BTreeMap::from([
                    ("MAT-0130".to_string(), BTreeSet::new()),
                    ("GEX-1000".to_string(), BTreeSet::new()),
                ]),
            )]),
            manual: BTreeMap::from([(
                1,
                vec!["MAT-0130".to_string(), "GEX-1000".to_string()],
            )]),
            ..Plan::default()
        };
        purge_codes(&mut plan, &["MAT-0130".to_string()]);
        assert_eq!(plan.electives, ["GEX-1000"]);
        assert!(plan.pinned_sessions.is_empty());
        assert!(!plan.displayed_placement.contains_key("MAT-0130"));
        assert_eq!(plan.displayed_placement["GEX-1000"], 2);
        assert_eq!(plan.manual[&1], ["GEX-1000"]);
        assert!(!plan.chosen[&1].contains_key("MAT-0130"));
        assert!(plan.chosen[&1].contains_key("GEX-1000"));
    }

    #[test]
    fn changing_a_scope_purges_its_grants_and_names_them() {
        let mut plan = Plan {
            rule_grants: BTreeMap::from([
                ("GEX-1000".to_string(), "c/Règle C1".to_string()),
                ("GEX-2000".to_string(), "p/Règle 2".to_string()),
                ("GEX-3000".to_string(), "f/Règle P1".to_string()),
            ]),
            ..Plan::default()
        };
        let dropped = purge_scope_grants(&mut plan, 'c');
        assert_eq!(dropped, ["GEX-1000"], "the outgoing block's only");
        assert_eq!(plan.rule_grants.len(), 2, "p/ and f/ grants survive");
        assert!(
            purge_scope_grants(&mut plan, 'c').is_empty(),
            "nothing left to purge, nothing announced"
        );
    }

    #[test]
    fn crediting_purges_the_placement_but_keeps_the_entente() {
        let mut plan = Plan {
            electives: vec!["GEX-1000".to_string()],
            pinned_sessions: BTreeMap::from([("GEX-1000".to_string(), 2)]),
            displayed_placement: BTreeMap::from([("GEX-1000".to_string(), 2)]),
            rule_grants: BTreeMap::from([(
                "GEX-1000".to_string(),
                "p/Règle 2".to_string(),
            )]),
            ..Plan::default()
        };
        credit_code(&mut plan, "GEX-1000");
        assert!(plan.credited.contains("GEX-1000"));
        assert!(plan.electives.is_empty());
        assert!(plan.pinned_sessions.is_empty());
        assert!(plan.displayed_placement.is_empty());
        assert_eq!(
            plan.rule_grants["GEX-1000"], "p/Règle 2",
            "the credit stacks with the entente"
        );
    }

    #[test]
    fn uncrediting_takes_the_course_back_as_an_elective() {
        let mut plan = Plan {
            displayed_placement: BTreeMap::from([("GEX-1000".to_string(), 2)]),
            ..Plan::default()
        };
        credit_code(&mut plan, "GEX-1000");
        uncredit_code(&mut plan, "GEX-1000");
        assert!(!plan.credited.contains("GEX-1000"));
        assert_eq!(
            plan.electives,
            ["GEX-1000"],
            "taken again, session left to the solver"
        );
        assert!(plan.displayed_placement.is_empty(), "no ghost session");
        // a code already elective is not duplicated
        uncredit_code(&mut plan, "GEX-1000");
        assert_eq!(plan.electives, ["GEX-1000"]);
    }

    #[test]
    fn removing_a_course_drops_its_entente_with_it() {
        let mut plan = Plan {
            electives: vec!["GLG-1001".to_string()],
            rule_grants: BTreeMap::from([(
                "GLG-1001".to_string(),
                "p/Règle 5".to_string(),
            )]),
            ..Plan::default()
        };
        remove_course(&mut plan, "GLG-1001");
        assert!(plan.electives.is_empty());
        assert!(plan.rule_grants.is_empty(), "the ✕ purges the entente");
    }

    #[test]
    fn session_codes_join_placement_and_manual_without_duplicates() {
        let plan = Plan {
            displayed_placement: BTreeMap::from([
                ("GEX-1000".to_string(), 1),
                ("GCI-1007".to_string(), 2),
            ]),
            manual: BTreeMap::from([(
                1,
                vec!["ANL-2020".to_string(), "GEX-1000".to_string()],
            )]),
            ..Plan::default()
        };
        assert_eq!(session_codes(&plan, 1), ["GEX-1000", "ANL-2020"]);
        assert_eq!(session_codes(&plan, 2), ["GCI-1007"]);
        assert!(session_codes(&plan, 3).is_empty());
    }

    #[test]
    fn apply_undo_redo_walk_the_same_labelled_steps() {
        let mut plan = Plan::default();
        let mut history = History::default();
        apply(&mut plan, &mut history, "Ajout de GEX-1000", |plan| {
            plan.electives.push("GEX-1000".to_string());
        });
        apply(&mut plan, &mut history, "Ajout de GCI-1007", |plan| {
            plan.electives.push("GCI-1007".to_string());
        });
        assert_eq!(history.undo_label(), Some("Ajout de GCI-1007"));

        assert_eq!(
            undo(&mut plan, &mut history).as_deref(),
            Some("Ajout de GCI-1007")
        );
        assert_eq!(plan.electives, ["GEX-1000"]);
        assert_eq!(history.redo_label(), Some("Ajout de GCI-1007"));

        assert_eq!(
            redo(&mut plan, &mut history).as_deref(),
            Some("Ajout de GCI-1007")
        );
        assert_eq!(plan.electives, ["GEX-1000", "GCI-1007"]);

        undo(&mut plan, &mut history);
        undo(&mut plan, &mut history);
        assert_eq!(plan, Plan::default());
        assert_eq!(undo(&mut plan, &mut history), None, "nothing left");
        assert_eq!(
            redo(&mut plan, &mut history).as_deref(),
            Some("Ajout de GEX-1000")
        );
    }

    #[test]
    fn a_new_edit_clears_the_redo_branch() {
        let mut plan = Plan::default();
        let mut history = History::default();
        apply(&mut plan, &mut history, "A", |plan| plan.credit_cap = 12);
        undo(&mut plan, &mut history);
        apply(&mut plan, &mut history, "B", |plan| plan.credit_cap = 18);
        assert_eq!(redo(&mut plan, &mut history), None, "the branch died");
        assert_eq!(plan.credit_cap, 18);
    }

    #[test]
    fn the_history_is_bounded_and_drops_its_oldest_step() {
        let mut plan = Plan::default();
        let mut history = History::default();
        for i in 0..(HISTORY_CAP + 10) {
            apply(&mut plan, &mut history, &format!("étape {i}"), |plan| {
                plan.study_sessions = i;
            });
        }
        let mut undone = 0;
        for _ in 0..(HISTORY_CAP + 10) {
            if undo(&mut plan, &mut history).is_none() {
                break;
            }
            undone += 1;
        }
        assert_eq!(undone, HISTORY_CAP, "the cap held");
        assert_eq!(plan.study_sessions, 9, "the ten oldest steps are gone");
    }

    #[test]
    fn plan_and_view_round_trip_and_tolerate_absent_fields() {
        let plan = Plan::default();
        let json = serde_json::to_string(&plan)
            .unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: Plan = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("round trip: {e}"));
        assert_eq!(back, plan);
        // an empty object restores every field to its default
        let bare: Plan =
            serde_json::from_str("{}").unwrap_or_else(|e| panic!("bare: {e}"));
        assert_eq!(bare, Plan::default());
        let bare: View = serde_json::from_str("{}")
            .unwrap_or_else(|e| panic!("bare view: {e}"));
        assert_eq!(bare, View::default());
    }
}
