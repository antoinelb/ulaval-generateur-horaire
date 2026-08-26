// The pure « document horaire » model (plan item 6): every session of the
// plan, paginated one année scolaire per page (automne, hiver, été
// together — a plan starting in hiver simply opens its first page there,
// and an empty été is left out). The course list under a grid only carries
// what the grid cannot show (à distance, horaire inconnu). Pure Rust only —
// no Dioxus, no web-sys, no clock (`generated_at` is read at the view
// boundary and handed in, exactly like `export::provenance` and
// `import::build_local_program`).

use ulaval_scheduler_core::{
    horizon_sessions, Mode, Season, Section, Semester,
};

use crate::data::Snapshot;
use crate::export::provenance::{export_provenance, ExportProvenance};
use crate::present::{self, GridModel};
use crate::solve;
use crate::state::{self, Plan};

pub struct ScheduleDocument {
    pub title: String,
    pub program_title: String,
    pub pages: Vec<SchedulePage>,
    pub provenance: ExportProvenance,
}

// One année scolaire: at most automne + hiver + été, in plan order — never
// empty (a page only exists because a sheet opened it).
pub struct SchedulePage {
    pub sheets: Vec<SessionSheet>,
}

pub struct SessionSheet {
    pub title: String,
    pub grid: Option<GridModel>,
    pub courses: Vec<CourseLine>,
    pub notes: Vec<String>,
}

pub struct CourseLine {
    pub code: String,
    pub title: String,
    pub detail: String,
}

pub fn schedule_document(
    snapshot: &Snapshot,
    plan: &Plan,
    program_title: &str,
    generated_at: &str,
) -> ScheduleDocument {
    let seasons = horizon_sessions(plan.start.season, plan.study_sessions);
    // an été with nothing at all to say is left out of the document — the
    // automne/hiver sheets stay even when empty, since their absence would
    // read as a dropped session rather than a session with no courses
    let sheets: Vec<(Season, SessionSheet)> = seasons
        .iter()
        .enumerate()
        .map(|(index, &season)| {
            (season, session_sheet(snapshot, plan, index + 1))
        })
        .filter(|(season, sheet)| {
            *season != Season::Summer
                || sheet.grid.is_some()
                || !sheet.courses.is_empty()
                || !sheet.notes.is_empty()
        })
        .collect();
    ScheduleDocument {
        title: "Horaires hebdomadaires".to_string(),
        program_title: program_title.to_string(),
        pages: paginate(sheets),
        provenance: export_provenance(
            generated_at,
            snapshot.provenance.scraped_at.as_deref(),
        ),
    }
}

// One page per année scolaire: a new page opens on each automne, so an été
// always shares its page with the sessions that precede it — and a plan
// starting in hiver (or été) simply opens its first page there.
fn paginate(sheets: Vec<(Season, SessionSheet)>) -> Vec<SchedulePage> {
    let mut pages: Vec<SchedulePage> = Vec::new();
    for (season, sheet) in sheets {
        match pages.last_mut() {
            Some(page) if season != Season::Fall => page.sheets.push(sheet),
            _ => pages.push(SchedulePage {
                sheets: vec![sheet],
            }),
        }
    }
    pages
}

fn session_sheet(
    snapshot: &Snapshot,
    plan: &Plan,
    session: usize,
) -> SessionSheet {
    let schedule = solve::weekly_schedule(snapshot, plan, session);
    let grid = present::grid_model(&schedule, snapshot, None);
    // a session drawing nothing shows no empty frame — its course lines
    // (« horaire inconnu », « à distance ») say why
    let all_days_empty = grid.days.iter().all(|day| day.blocks.is_empty());

    // the list only carries what the grid cannot show: courses whose
    // chosen option has no weekly slot (à distance) and courses excluded
    // with a reason — a course drawn on the grid already names itself there
    let mut courses: Vec<CourseLine> = schedule
        .report
        .courses
        .iter()
        .filter(|course| grid.unplaced.contains(&course.code))
        .map(|course| CourseLine {
            code: course.code.clone(),
            title: course_title(snapshot, &course.code),
            detail: unplaced_detail(&course.selected),
        })
        .collect();
    for excluded in &schedule.excluded {
        // the on-screen wording (`solve.rs`'s exclusion reason) shortened
        // for the printed parenthesis; every other reason prints as-is
        let detail = if excluded.reason == "horaire pas encore publié" {
            "horaire inconnu".to_string()
        } else {
            excluded.reason.clone()
        };
        courses.push(CourseLine {
            code: excluded.code.clone(),
            title: course_title(snapshot, &excluded.code),
            detail,
        });
    }

    SessionSheet {
        title: sheet_title(plan, session),
        grid: if all_days_empty { None } else { Some(grid) },
        courses,
        notes: schedule.notes.clone(),
    }
}

fn course_title(snapshot: &Snapshot, code: &str) -> String {
    snapshot
        .by_code
        .get(code)
        .map(|&index| snapshot.courses[index].title.clone())
        .unwrap_or_else(|| code.to_string())
}

// Why the course draws no block, in the fewest words: its mode when that
// is the reason (à distance, hybride), « horaire inconnu » otherwise — no
// code, no section letter, the line's lead already carries the code.
fn unplaced_detail(selected: &[Section]) -> String {
    let mode = selected.iter().find_map(|section| match section.mode {
        Mode::InPerson => None,
        Mode::Remote => Some("à distance"),
        Mode::Hybrid => Some("hybride"),
    });
    mode.unwrap_or("horaire inconnu").to_string()
}

// « A1 — Automne 2026 » on the on-screen heading's own idiom
// (`components/grid.rs`'s `title`), minus its leading « Horaire — »: this
// document already carries that title once, at the top. An été carries its
// long form alone (its short form IS its semester); a session the horizon
// does not reach degrades to « Session N » rather than panicking.
fn sheet_title(plan: &Plan, session: usize) -> String {
    let Some(semester) = solve::session_semester(plan, session) else {
        return format!("Session {session}");
    };
    let long = long_semester(semester);
    if semester.season == ulaval_scheduler_core::Season::Summer {
        return long;
    }
    let seasons = horizon_sessions(plan.start.season, plan.study_sessions);
    let semesters = state::session_semesters(plan.start, &seasons);
    format!("{} — {long}", state::session_short(&semesters, session - 1))
}

fn long_semester(semester: Semester) -> String {
    let season = match semester.season {
        ulaval_scheduler_core::Season::Fall => "Automne",
        ulaval_scheduler_core::Season::Winter => "Hiver",
        ulaval_scheduler_core::Season::Summer => "Été",
    };
    format!("{season} {}", semester.year)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    use ulaval_scheduler_core::Season;

    use crate::data::{parse_data, RawData};

    // GEX-1000: monday+wednesday option (fall, section A); GEX-3000:
    // offered fall, schedule not yet published; GEX-9000/8000/7000: offered
    // fall with one option carrying no weekly slot at all (remote, hybrid
    // and in-person — the three `unplaced_detail` readings).
    const COURSES: &str = r#"{"courses":[
      {"code":"GEX-1000","title":"Hydrologie","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"11111","section":"A","mode":"in-person","slots":[
            {"day":"monday","start":"08:30","end":"11:20"},
            {"day":"wednesday","start":"08:30","end":"09:20"}]}]
       ]}}},
      {"code":"GEX-3000","title":"Sans horaire","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":null}}},
      {"code":"GEX-9000","title":"À distance","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"99999","section":"Z1","mode":"remote","slots":[]}]
       ]}}},
      {"code":"GEX-8000","title":"Hybride","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"88888","section":"B","mode":"hybrid","slots":[]}]
       ]}}},
      {"code":"GEX-7000","title":"Sans plage","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"77777","section":null,"mode":"in-person","slots":[]}]
       ]}}}
    ]}"#;

    fn snapshot() -> Snapshot {
        parse_data(
            &RawData {
                courses: COURSES.to_string(),
                meta: Some(
                    r#"{"scraped_at":"2026-08-01T00:00:00Z"}"#.to_string(),
                ),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn plan_with(start: Season, study_sessions: usize) -> Plan {
        Plan {
            start: Semester {
                season: start,
                year: 2026,
            },
            study_sessions,
            ..Plan::default()
        }
    }

    #[test]
    fn a_winter_start_opens_its_first_page_at_hiver() {
        // Hiver 2026, 5 study sessions -> H,E,A,H,E,A,H,E; every été is
        // empty here, so they drop out and each année scolaire keeps one
        // page: [H], [A,H], [A,H] — the first opening at the plan start
        let plan = plan_with(Season::Winter, 5);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let lens: Vec<usize> = document
            .pages
            .iter()
            .map(|page| page.sheets.len())
            .collect();
        assert_eq!(lens, [1, 2, 2]);
        assert_eq!(document.pages[0].sheets[0].title, "H1 — Hiver 2026");
    }

    #[test]
    fn a_fall_start_gathers_each_annee_scolaire_on_one_page() {
        // Automne 2026, 5 study sessions -> A,H,E,A,H,E,A with empty étés
        // dropped: [A,H], [A,H], the trailing automne alone
        let plan = plan_with(Season::Fall, 5);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let lens: Vec<usize> = document
            .pages
            .iter()
            .map(|page| page.sheets.len())
            .collect();
        assert_eq!(lens, [2, 2, 1]);
    }

    #[test]
    fn a_placed_course_draws_its_grid_and_stays_off_the_list() {
        let mut plan = plan_with(Season::Fall, 1);
        plan.manual.insert(1, vec!["GEX-1000".to_string()]);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let sheet = &document.pages[0].sheets[0];
        assert_eq!(sheet.title, "A1 — Automne 2026");
        let grid = sheet.grid.as_ref().expect("a block was drawn");
        assert!(grid.days.iter().any(|day| !day.blocks.is_empty()));
        // the grid already names it — the list only carries what the grid
        // cannot show
        assert!(sheet.courses.is_empty());
    }

    #[test]
    fn an_unpublished_schedule_reads_horaire_inconnu() {
        let mut plan = plan_with(Season::Fall, 1);
        plan.manual.insert(1, vec!["GEX-3000".to_string()]);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let sheet = &document.pages[0].sheets[0];
        assert!(sheet.grid.is_none());
        assert_eq!(sheet.courses.len(), 1);
        assert_eq!(sheet.courses[0].code, "GEX-3000");
        assert_eq!(sheet.courses[0].detail, "horaire inconnu");
    }

    #[test]
    fn an_a_distance_course_is_named_among_the_courses() {
        let mut plan = plan_with(Season::Fall, 1);
        plan.manual
            .insert(1, vec!["GEX-1000".to_string(), "GEX-9000".to_string()]);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let sheet = &document.pages[0].sheets[0];
        // GEX-1000 draws a block, and being drawn it stays off the list:
        // only GEX-9000 is named, once
        assert!(sheet.grid.is_some());
        assert_eq!(sheet.courses.len(), 1);
        assert_eq!(sheet.courses[0].code, "GEX-9000");
        assert_eq!(sheet.courses[0].title, "À distance");
        assert_eq!(sheet.courses[0].detail, "à distance");
    }

    #[test]
    fn a_slotless_hybrid_option_names_its_mode() {
        let mut plan = plan_with(Season::Fall, 1);
        plan.manual.insert(1, vec!["GEX-8000".to_string()]);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let sheet = &document.pages[0].sheets[0];
        assert_eq!(sheet.courses[0].detail, "hybride");
    }

    #[test]
    fn a_slotless_in_person_option_reads_horaire_inconnu() {
        let mut plan = plan_with(Season::Fall, 1);
        plan.manual.insert(1, vec!["GEX-7000".to_string()]);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let sheet = &document.pages[0].sheets[0];
        assert_eq!(sheet.courses[0].detail, "horaire inconnu");
    }

    #[test]
    fn a_code_absent_from_the_catalogue_falls_back_to_itself_as_a_title() {
        let mut plan = plan_with(Season::Fall, 1);
        // a stale save can hold a code the fresh snapshot no longer has —
        // `weekly_schedule` excludes it, and its title has nowhere to come
        // from but the code itself
        plan.manual.insert(1, vec!["ZZZ-9999".to_string()]);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let sheet = &document.pages[0].sheets[0];
        assert_eq!(sheet.courses.len(), 1);
        assert_eq!(sheet.courses[0].code, "ZZZ-9999");
        assert_eq!(sheet.courses[0].title, "ZZZ-9999");
        assert_eq!(sheet.courses[0].detail, "absent du catalogue actuel");
    }

    #[test]
    fn an_entirely_empty_session_still_gets_its_titled_sheet() {
        let plan = plan_with(Season::Fall, 1);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let sheet = &document.pages[0].sheets[0];
        assert_eq!(sheet.title, "A1 — Automne 2026");
        assert!(sheet.grid.is_none());
        assert!(sheet.courses.is_empty());
    }

    #[test]
    fn a_summer_sheet_with_courses_stays_and_carries_its_long_form_alone() {
        // Hiver 2026, 1 study session -> [Hiver, Été]; a course requested
        // in the été (excluded or not) keeps its sheet on the page
        let mut plan = plan_with(Season::Winter, 1);
        plan.manual.insert(2, vec!["GEX-1000".to_string()]);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        assert_eq!(
            document.pages[0].sheets.get(1).map(|s| &s.title),
            Some(&"Été 2026".to_string())
        );
    }

    #[test]
    fn an_empty_summer_is_left_out_of_the_document() {
        // the same horizon with nothing in the été: only the hiver prints
        let plan = plan_with(Season::Winter, 1);
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        assert_eq!(document.pages.len(), 1);
        let titles: Vec<&str> = document.pages[0]
            .sheets
            .iter()
            .map(|sheet| sheet.title.as_str())
            .collect();
        assert_eq!(titles, ["H1 — Hiver 2026"]);
    }

    #[test]
    fn a_session_outside_the_horizon_degrades_to_a_bare_label() {
        let plan = plan_with(Season::Fall, 1);
        assert_eq!(sheet_title(&plan, 99), "Session 99");
    }

    #[test]
    fn schedule_notes_survive_onto_the_sheet() {
        use std::collections::{BTreeMap, BTreeSet};

        let mut plan = plan_with(Season::Fall, 1);
        plan.manual.insert(1, vec!["GEX-1000".to_string()]);
        // a stale pin for a course that is not requested this session:
        // `weekly_schedule` drops it with a note, never touching the plan
        plan.chosen.insert(
            1,
            BTreeMap::from([(
                "GEX-3000".to_string(),
                BTreeSet::from(["00000".to_string()]),
            )]),
        );
        let document =
            schedule_document(&snapshot(), &plan, "B-GEX", "2026-08-25");
        let sheet = &document.pages[0].sheets[0];
        assert_eq!(sheet.notes.len(), 1, "{:?}", sheet.notes);
        assert!(sheet.notes[0].contains("GEX-3000"), "{:?}", sheet.notes);
    }

    #[test]
    fn the_document_carries_its_title_program_and_provenance() {
        let plan = plan_with(Season::Fall, 1);
        let document = schedule_document(
            &snapshot(),
            &plan,
            "B-GEX",
            "2026-08-25T00:00:00Z",
        );
        assert_eq!(document.title, "Horaires hebdomadaires");
        assert_eq!(document.program_title, "B-GEX");
        assert_eq!(document.provenance.scraped, "2026-08-01T00:00:00Z");
    }
}
