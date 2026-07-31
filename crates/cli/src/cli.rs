use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clap::Parser;

use ulaval_scheduler_core::{
    coverage_report, place, resolve_offering, schedule_report, Completion,
    Course, CourseReport, CoverageReport, Day, LanguageStatus,
    MandatoryReport, Missing, Placement, PlacementRequest, Program,
    RuleReport, RuleStatus, ScheduleReport, Season, Section, Slot, Solution,
};

// The jalon-2 harness: print a conflict-free weekly schedule for a list of
// course codes of one session. All logic lives here in the lib, measured;
// `main.rs` is a shim (ADR `2026-07-harnais-cli-en-crate-dedie`,
// `2026-07-cli-dans-la-lib-et-style-derreurs`).
#[derive(Parser)]
#[command(name = "ulaval-scheduler", arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

// a subcommand rather than bare positionals: the later solveur-B commands
// (organigramme) will sit beside it in this same binary
#[derive(clap::Subcommand)]
enum Command {
    Schedule {
        // season letter + year: a2026 (automne), h2027 (hiver), e2026
        // (été). The season selects the offering; the snapshot keeps one
        // per season (its freshest), so the year names the student's
        // target session without selecting data.
        session: String,
        #[arg(required = true, num_args = 1..)]
        codes: Vec<String>,
        #[arg(long, default_value = "data")]
        data_dir: String,
    },
    Organigramme(OrganigrammeArgs),
}

// The solveur-B harness (Phase 4 of `docs/next_steps.md`): a starting
// session, a course list (a program's mandatory courses and/or explicit
// codes) and the student's constraints → the first organigramme printed
// whole, the total solution count, and — with a program — the rules
// coverage report beside it.
#[derive(clap::Args)]
struct OrganigrammeArgs {
    // starting session (a2026): seasons then alternate automne/hiver
    start: String,
    // chosen electives (or the whole list when --program is absent)
    #[arg(num_args = 0..)]
    codes: Vec<String>,
    // a program snapshot stem (baccalaureat-en-genie-des-eaux-2026): its
    // mandatory courses join the list and the coverage report is printed
    #[arg(long)]
    program: Option<String>,
    // no documented per-session cap exists (open question with the
    // director), so the value stays an explicit input, never a constant
    #[arg(long)]
    credit_cap: u32,
    #[arg(long, default_value_t = 8)]
    sessions: usize,
    #[arg(long, value_delimiter = ',')]
    passed: Vec<String>,
    // CODE=SESSION, session numbers 1-based
    #[arg(long, value_delimiter = ',')]
    pinned: Vec<String>,
    #[arg(long)]
    concomitant: bool,
    // the double bound (ADR `2026-07-budget-de-b-en-double-borne`)
    #[arg(long, default_value_t = 10_000_000)]
    max_nodes: u64,
    #[arg(long, default_value_t = 100_000)]
    max_solutions: usize,
    #[arg(long, default_value = "data")]
    data_dir: String,
}

// the same snapshot shape the scraper writes: `{"courses": [...]}`
#[derive(serde::Deserialize)]
struct Snapshot {
    courses: Vec<Course>,
}

pub fn run(args: Vec<String>) -> anyhow::Result<()> {
    let argv = std::iter::once("ulaval-scheduler".to_string()).chain(args);
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        // help and version are successful outcomes, not errors
        Err(error) if error.exit_code() == 0 => {
            // display is non-critical: a broken pipe must never kill the run
            error.print().ok();
            return Ok(());
        }
        // usage errors carry clap's rendered message through the anyhow
        // frontier: main prints it to stderr and exits 2
        Err(error) => {
            anyhow::bail!("{}", error.render().ansi().to_string().trim_end())
        }
    };

    match cli.command {
        Command::Schedule {
            session,
            codes,
            data_dir,
        } => print_schedule(&session, &codes, &data_dir),
        Command::Organigramme(args) => print_organigramme(&args),
    }
}

fn print_schedule(
    session: &str,
    codes: &[String],
    data_dir: &str,
) -> anyhow::Result<()> {
    let (season, _) = parse_session(session)?;
    let snapshot = read_snapshot(data_dir)?;
    let codes = normalize_codes(codes)?;
    let courses = select_courses(&snapshot.courses, &codes, season, session)?;

    // no pins: the harness always starts from scratch, the UI will pin
    let report = schedule_report(&courses, season, &BTreeMap::new())?;
    let total = credit_total(&courses)?;
    println!("{}", render(&report, total));

    let conflicting: Vec<&str> = report
        .courses
        .iter()
        .filter(|course| !course.valid)
        .map(|course| course.code.as_str())
        .collect();
    anyhow::ensure!(
        report.valid,
        "no conflict-free combination exists; conflicting courses : {}",
        conflicting.join(", ")
    );
    Ok(())
}

fn print_organigramme(args: &OrganigrammeArgs) -> anyhow::Result<()> {
    let (start, _) = parse_session(&args.start)?;
    let sessions = alternating_sessions(start, args.sessions);
    let program = args
        .program
        .as_ref()
        .map(|stem| read_program(&args.data_dir, stem))
        .transpose()?;
    let electives = normalize_codes(&args.codes)?;
    let passed_codes = normalize_codes(&args.passed)?;
    let list = course_list(program.as_ref(), &electives, &passed_codes);
    let snapshot = read_snapshot(&args.data_dir)?;
    // typed input (codes, passed, pins) is strictly validated — a typo
    // must not survive; program-derived courses degrade loudly instead
    // (ADR `2026-07-cours-sans-offre-ecarte-par-le-harnais`)
    let explicit: BTreeSet<&str> = electives
        .iter()
        .chain(&passed_codes)
        .map(String::as_str)
        .collect();
    let (courses, set_aside) = select_known(&list, &snapshot, &explicit)?;
    if !set_aside.is_empty() {
        println!(
            "Sans données d'offre (écartés du placement) : {}\n",
            set_aside.join(", ")
        );
    }
    let passed: BTreeSet<String> = passed_codes.into_iter().collect();
    let pinned = parse_pins(&args.pinned)?;

    let placement = place(&PlacementRequest {
        sessions: &sessions,
        credit_cap: args.credit_cap,
        concomitant: args.concomitant,
        courses: &courses,
        passed: &passed,
        pinned: &pinned,
        // the hand-encoded cheminement_type seed will plug in here once
        // `data/programmes/{code}.manuel.json` exists
        seed: &BTreeMap::new(),
        max_nodes: args.max_nodes,
        max_solutions: args.max_solutions,
    })?;

    println!("{}", render_placement(&placement, &courses, &sessions));
    if let Some(program) = &program {
        let selection: BTreeSet<String> = list.iter().cloned().collect();
        let coverage =
            coverage_report(program, None, None, &selection, &courses)?;
        println!("\n{}", render_coverage(&coverage));
    }
    anyhow::ensure!(
        placement.completion != Completion::Complete
            || !placement.solutions.is_empty(),
        "no feasible organigramme exists (proven by exhaustive search)"
    );
    Ok(())
}

// real cheminements alternate automne/hiver; a summer start flows into
// fall — été is never generated automatically
fn alternating_sessions(start: Season, count: usize) -> Vec<Season> {
    (0..count)
        .scan(start, |season, _| {
            let current = *season;
            *season = match current {
                Season::Fall => Season::Winter,
                Season::Winter | Season::Summer => Season::Fall,
            };
            Some(current)
        })
        .collect()
}

fn read_program(data_dir: &str, stem: &str) -> anyhow::Result<Program> {
    let path = Path::new(data_dir)
        .join("programmes")
        .join(format!("{stem}.json"));
    let raw = std::fs::read_to_string(&path).map_err(|source| {
        anyhow::anyhow!(
            "Reading {}: {source}\nRun `ulaval-scraper program` first.",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|source| {
        anyhow::anyhow!("Parsing {}: {source}", path.display())
    })
}

// the program's mandatory courses first (reference order), then the
// chosen electives, then the passed courses — deduplicated, so a passed
// mandatory course appears once and carries its Course object
fn course_list(
    program: Option<&Program>,
    electives: &[String],
    passed: &[String],
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    program
        .map(|program| program.mandatory.clone())
        .unwrap_or_default()
        .into_iter()
        .chain(electives.iter().cloned())
        .chain(passed.iter().cloned())
        .filter(|code| seen.insert(code.clone()))
        .collect()
}

// One Course per requested code, cloned whole — the snapshot already
// carries every season an offering exists for, each dated by its
// `last_offered`. A code the snapshot does not carry is an error when
// explicitly typed, and otherwise (program-derived) set aside and returned
// for the caller to surface — never silently dropped either way.
fn select_known(
    codes: &[String],
    snapshot: &Snapshot,
    explicit: &BTreeSet<&str>,
) -> anyhow::Result<(Vec<Course>, Vec<String>)> {
    let by_code: BTreeMap<&str, &Course> = snapshot
        .courses
        .iter()
        .map(|course| (course.code.as_str(), course))
        .collect();
    let unknown: Vec<&str> = codes
        .iter()
        .filter(|code| !by_code.contains_key(code.as_str()))
        .map(String::as_str)
        .collect();
    let typos: Vec<&str> = unknown
        .iter()
        .filter(|code| explicit.contains(**code))
        .copied()
        .collect();
    anyhow::ensure!(
        typos.is_empty(),
        "unknown course codes : {}",
        typos.join(", ")
    );
    let set_aside: Vec<String> =
        unknown.iter().map(|code| code.to_string()).collect();
    let courses = codes
        .iter()
        .filter_map(|code| by_code.get(code.as_str()))
        .map(|&course| course.clone())
        .collect();
    Ok((courses, set_aside))
}

fn parse_pins(specs: &[String]) -> anyhow::Result<BTreeMap<String, usize>> {
    specs
        .iter()
        .map(|spec| {
            let (code, session) = spec.split_once('=').ok_or_else(|| {
                anyhow::anyhow!("pinned expects CODE=SESSION : {spec}")
            })?;
            let session = session.parse().map_err(|_| {
                anyhow::anyhow!("pinned expects CODE=SESSION : {spec}")
            })?;
            Ok((code.to_uppercase(), session))
        })
        .collect()
}

// the first organigramme whole — session by session with its credit
// load — then the count and how the search ended, never confusing a
// proven-empty set with an interrupted one
fn render_placement(
    placement: &Placement,
    courses: &[Course],
    sessions: &[Season],
) -> String {
    let status = match placement.completion {
        Completion::Complete => "ensemble complet".to_string(),
        Completion::NodeBudget => {
            "budget de nœuds épuisé — ensemble partiel".to_string()
        }
        Completion::SolutionCap => {
            "plafond de solutions atteint — ensemble partiel".to_string()
        }
    };
    let count =
        format!("{} solution(s) ({status})", placement.solutions.len());
    match placement.solutions.first() {
        None => count,
        Some(first) => {
            let credits: BTreeMap<&str, u32> = courses
                .iter()
                .map(|course| {
                    (course.code.as_str(), course.credits.planning())
                })
                .collect();
            let terms: Vec<String> = sessions
                .iter()
                .enumerate()
                .map(|(i, &season)| {
                    render_term(first, i + 1, season, &credits)
                })
                .collect();
            let assumed = if first.assumed.is_empty() {
                String::new()
            } else {
                format!(
                    "\n\nPréalables présumés (à vérifier) : {}",
                    first
                        .assumed
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" ; ")
                )
            };
            format!("{}\n\n{count}{assumed}", terms.join("\n"))
        }
    }
}

fn render_term(
    solution: &Solution,
    session: usize,
    season: Season,
    credits: &BTreeMap<&str, u32>,
) -> String {
    let placed: Vec<&str> = solution
        .placement
        .iter()
        .filter(|&(_, &term)| term == session)
        .map(|(code, _)| code.as_str())
        .collect();
    let load: u32 = placed.iter().filter_map(|code| credits.get(code)).sum();
    let listing = if placed.is_empty() {
        "—".to_string()
    } else {
        format!("{} ({load} cr)", placed.join(", "))
    };
    format!("Session {session} ({}) : {listing}", season_label(season))
}

fn season_label(season: Season) -> &'static str {
    match season {
        Season::Fall => "Automne",
        Season::Winter => "Hiver",
        Season::Summer => "Été",
    }
}

// the coverage report in French prose — one line per verdict, the raw
// text of anything the grammar could not count shown verbatim
fn render_coverage(coverage: &CoverageReport) -> String {
    let mut lines = vec!["Couverture des règles :".to_string()];
    lines.extend(coverage.mandatory.iter().map(render_mandatory));
    lines.extend(coverage.rules.iter().map(render_rule));
    if let Some(language) = &coverage.language_requirement {
        lines.push(match language.status {
            LanguageStatus::Satisfied => {
                "Exigence linguistique : satisfaite".to_string()
            }
            LanguageStatus::Reported => {
                "Exigence linguistique : à valider (cours ou test de \
                 classement)"
                    .to_string()
            }
        });
    }
    lines.join("\n")
}

fn render_mandatory(block: &MandatoryReport) -> String {
    if block.missing.is_empty() {
        format!("Cours obligatoires : complets ({})", block.satisfied.len())
    } else {
        format!(
            "Cours obligatoires : manquants — {}",
            block.missing.join(", ")
        )
    }
}

fn render_rule(rule: &RuleReport) -> String {
    match rule.status {
        RuleStatus::Satisfied => format!("{} : satisfaite", rule.title),
        RuleStatus::Incomplete => {
            let missing =
                rule.missing.as_ref().map(missing_label).unwrap_or_default();
            let candidates =
                rule.candidates.as_deref().unwrap_or_default().len();
            format!(
                "{} : à combler — {missing} (candidats : {candidates})",
                rule.title
            )
        }
        RuleStatus::Reported => match &rule.raw {
            Some(raw) => {
                format!("{} : à valider — « {raw} »", rule.title)
            }
            None => format!("{} : à valider", rule.title),
        },
    }
}

fn missing_label(missing: &Missing) -> String {
    match *missing {
        Missing::Count { count } => format!("{count} cours"),
        Missing::Credits { credits } => format!("{credits} crédits"),
    }
}

// `a2026` → (Fall, 2026); a = automne, h = hiver, e = été. Only the season
// selects data — the snapshot keeps one offering per season — but the year
// is still validated: a malformed session is a typo to surface.
fn parse_session(session: &str) -> anyhow::Result<(Season, u16)> {
    let mut letters = session.chars();
    let season = match letters.next() {
        Some('a') => Season::Fall,
        Some('h') => Season::Winter,
        Some('e') => Season::Summer,
        _ => anyhow::bail!(
            "unknown session {session:?}: expected a<year>, h<year> \
             or e<year> (e.g. a2026)"
        ),
    };
    let year = letters.as_str().parse().map_err(|_| {
        anyhow::anyhow!(
            "unknown session {session:?}: expected a<year>, h<year> \
             or e<year> (e.g. a2026)"
        )
    })?;
    Ok((season, year))
}

fn read_snapshot(data_dir: &str) -> anyhow::Result<Snapshot> {
    let path = Path::new(data_dir).join("cours.json");
    let raw = std::fs::read_to_string(&path).map_err(|source| {
        anyhow::anyhow!(
            "Reading {}: {source}\nRun `ulaval-scraper courses` first.",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|source| {
        anyhow::anyhow!("Parsing {}: {source}", path.display())
    })
}

// codes are uppercased for the student's comfort; a duplicated code is a
// typo to surface, not a course to schedule twice
fn normalize_codes(codes: &[String]) -> anyhow::Result<Vec<String>> {
    let codes: Vec<String> =
        codes.iter().map(|code| code.to_uppercase()).collect();
    let mut seen = BTreeSet::new();
    let duplicated: Vec<&str> = codes
        .iter()
        .filter(|code| !seen.insert(code.as_str()))
        .map(String::as_str)
        .collect();
    anyhow::ensure!(
        duplicated.is_empty(),
        "duplicated course codes : {}",
        duplicated.join(", ")
    );
    Ok(codes)
}

// every requested course, its offering already resolved against its
// equivalents — all unknown codes are named in one error, never silently
// dropped
fn select_courses(
    all: &[Course],
    codes: &[String],
    season: Season,
    session: &str,
) -> anyhow::Result<Vec<Course>> {
    let by_code: BTreeMap<&str, &Course> = all
        .iter()
        .map(|course| (course.code.as_str(), course))
        .collect();
    let unknown: Vec<&str> = codes
        .iter()
        .filter(|code| !by_code.contains_key(code.as_str()))
        .map(String::as_str)
        .collect();
    anyhow::ensure!(
        unknown.is_empty(),
        "unknown course codes in {session} : {}",
        unknown.join(", ")
    );
    codes
        .iter()
        .map(|code| {
            effective_course(by_code[code.as_str()], &by_code, season)
                .ok_or_else(|| {
                    anyhow::anyhow!("{code} is not offered in {session}")
                })
        })
        .collect()
}

// The offering actually attended may come from an equivalent — the most
// recent `last_offered` vintage wins, ties to the course (ADR
// `2026-07-equivalences-par-millesime-de-session`, vintage in-data since
// ADR `2026-07-snapshot-unique-des-cours-millesime-par-saison`). The
// requested course keeps its identity: only the offering is borrowed.
fn effective_course(
    course: &Course,
    by_code: &BTreeMap<&str, &Course>,
    season: Season,
) -> Option<Course> {
    let seed = course.seasons.get(&season);
    let offering = course
        .equivalents
        .iter()
        .filter_map(|code| by_code.get(code.as_str()))
        .filter_map(|equivalent| equivalent.seasons.get(&season))
        .fold(seed, |acc, offering| resolve_offering(acc, Some(offering)))?;
    let mut effective = course.clone();
    effective.seasons = std::iter::once((season, offering.clone())).collect();
    Some(effective)
}

// a stage's `Credits::Range` needs the student's chosen weighting, which
// the harness has no flag for yet (open question of the plan) — the error
// is surfaced, never defaulted
fn credit_total(courses: &[Course]) -> anyhow::Result<u32> {
    courses.iter().try_fold(0u32, |total, course| {
        course
            .credits
            .resolve(None)
            .map(|credits| total + credits)
            .map_err(|error| anyhow::anyhow!("{} : {error}", course.code))
    })
}

// displayed text is French (domain rule); the report is rendered whole
// before any conflict aborts the run, so the student always sees what was
// found
fn render(report: &ScheduleReport, total: u32) -> String {
    let courses: Vec<String> =
        report.courses.iter().map(render_course).collect();
    format!("{}\n\nTotal : {total} crédits", courses.join("\n"))
}

fn render_course(course: &CourseReport) -> String {
    let marker = if course.valid { "" } else { " ⚠ conflit" };
    let sections: Vec<String> =
        course.selected.iter().map(render_section).collect();
    format!("{}{marker}\n{}", course.code, sections.join("\n"))
}

fn render_section(section: &Section) -> String {
    let label = match &section.section {
        Some(label) => format!(" [{label}]"),
        None => String::new(),
    };
    let slots: Vec<String> = section.slots.iter().map(render_slot).collect();
    let when = if slots.is_empty() {
        "à distance".to_string()
    } else {
        slots.join(", ")
    };
    format!("  {}{label} : {when}", section.nrc)
}

fn render_slot(slot: &Slot) -> String {
    let day = match slot.day {
        Day::Monday => "lundi",
        Day::Tuesday => "mardi",
        Day::Wednesday => "mercredi",
        Day::Thursday => "jeudi",
        Day::Friday => "vendredi",
        Day::Saturday => "samedi",
        Day::Sunday => "dimanche",
    };
    let start: String = slot.start.into();
    let end: String = slot.end.into();
    format!("{day} {start}–{end}")
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use ulaval_scheduler_core::Time;

    // --- parse_session ---

    #[test]
    fn every_season_letter_parses_to_its_season_and_year() {
        for (session, expected) in [
            ("a2026", (Season::Fall, 2026)),
            ("h2027", (Season::Winter, 2027)),
            ("e2026", (Season::Summer, 2026)),
        ] {
            let parsed = parse_session(session)
                .unwrap_or_else(|e| panic!("{session}: {e}"));
            assert_eq!(parsed, expected, "for {session}");
        }
    }

    #[test]
    fn a_session_outside_the_naming_scheme_is_an_error() {
        for session in ["x2026", "2026", "", "a", "a20x6"] {
            let error =
                parse_session(session).expect_err("outside the scheme");
            assert!(
                error.to_string().contains("a<year>"),
                "for {session:?}: {error}"
            );
        }
    }

    // --- normalize_codes ---

    #[test]
    fn codes_are_uppercased_for_the_student() {
        let codes = normalize_codes(&["gex-1000".to_string()])
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(codes, ["GEX-1000"]);
    }

    #[test]
    fn a_duplicated_code_is_an_error_naming_it() {
        // duplicated only once uppercased — the check runs on what will
        // actually be scheduled
        let error =
            normalize_codes(&["gex-1000".to_string(), "GEX-1000".to_string()])
                .expect_err("a duplicate is a typo");
        assert!(error.to_string().contains("GEX-1000"), "{error}");
    }

    // --- selection and equivalents ---

    fn course(code: &str, season: &str, options: &str) -> Course {
        vintage_course(code, season, "2026", options)
    }

    fn vintage_course(
        code: &str,
        season: &str,
        last_offered: &str,
        options: &str,
    ) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{{"{season}":{{"last_offered":{last_offered},
                                          "options":{options}}}}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    fn option_json(nrc: &str, day: &str, start: &str, end: &str) -> String {
        format!(
            r#"[{{"nrc":"{nrc}","section":"A","mode":"in-person",
                  "slots":[{{"day":"{day}","start":"{start}",
                  "end":"{end}"}}]}}]"#
        )
    }

    fn monday(code: &str, nrc: &str) -> Course {
        course(
            code,
            "fall",
            &format!("[{}]", option_json(nrc, "monday", "08:30", "11:20")),
        )
    }

    #[test]
    fn every_unknown_code_is_named_in_one_error() {
        let all = [monday("GEX-1000", "1")];
        let error = select_courses(
            &all,
            &["GEX-1000".to_string(), "A-1".to_string(), "B-2".to_string()],
            Season::Fall,
            "a2026",
        )
        .expect_err("two unknown codes");
        let message = error.to_string();
        assert!(message.contains("A-1"), "{message}");
        assert!(message.contains("B-2"), "{message}");
    }

    #[test]
    fn a_course_not_offered_in_the_session_is_an_error() {
        let all = [course(
            "GEX-1000",
            "winter",
            &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
        )];
        let error = select_courses(
            &all,
            &["GEX-1000".to_string()],
            Season::Fall,
            "a2026",
        )
        .expect_err("offered in winter only");
        assert!(error.to_string().contains("a2026"), "{error}");
    }

    #[test]
    fn a_missing_offering_borrows_the_equivalents() {
        // the requested course keeps its identity, only the offering is
        // borrowed from the equivalent
        let mut wanted = course("GEX-1000", "winter", "[[]]");
        wanted.equivalents = vec!["GCI-1000".to_string()];
        let all = [wanted, monday("GCI-1000", "7")];

        let courses = select_courses(
            &all,
            &["GEX-1000".to_string()],
            Season::Fall,
            "a2026",
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(courses[0].code, "GEX-1000");
        assert_eq!(selected_nrc(&courses[0]), "7");
    }

    #[test]
    fn a_courses_own_offering_wins_over_its_equivalents() {
        // equal vintages: ties go to the course itself
        let mut wanted = monday("GEX-1000", "1");
        wanted.equivalents = vec!["GCI-1000".to_string()];
        let all = [wanted, monday("GCI-1000", "7")];

        let courses = select_courses(
            &all,
            &["GEX-1000".to_string()],
            Season::Fall,
            "a2026",
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(selected_nrc(&courses[0]), "1");
    }

    #[test]
    fn an_equivalent_with_a_newer_vintage_wins_the_offering() {
        // the vintage lives in the data now: an equivalent whose season was
        // read from a fresher session shadows the course's own offering
        // (ADR `2026-07-equivalences-par-millesime-de-session`)
        let mut wanted = vintage_course(
            "GEX-1000",
            "fall",
            "2024",
            &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
        );
        wanted.equivalents = vec!["GCI-1000".to_string()];
        let all = [wanted, monday("GCI-1000", "7")];

        let courses = select_courses(
            &all,
            &["GEX-1000".to_string()],
            Season::Fall,
            "a2026",
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(courses[0].code, "GEX-1000", "identity is kept");
        assert_eq!(selected_nrc(&courses[0]), "7", "the offering is borrowed");
    }

    fn selected_nrc(course: &Course) -> &str {
        course.seasons[&Season::Fall]
            .options
            .as_deref()
            .expect("known schedule")[0][0]
            .nrc
            .as_str()
    }

    // --- credit_total ---

    #[test]
    fn fixed_credits_sum_over_the_courses() {
        let courses = [monday("A-1", "1"), monday("B-2", "2")];
        let total = credit_total(&courses).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(total, 6);
    }

    #[test]
    fn a_variable_credit_stage_surfaces_its_missing_weighting() {
        // no weighting flag in v0 (open question of the plan): the error
        // is surfaced, never defaulted to a bound
        let stage: Course = serde_json::from_str(
            r#"{"code":"GEX-2580","title":"Stage",
                "credits":{"min":6,"max":12},"cycle":1,
                "prerequisites":null,"equivalents":[],"seasons":{}}"#,
        )
        .unwrap_or_else(|e| panic!("stage literal: {e}"));
        let error = credit_total(&[stage]).expect_err("no chosen weighting");
        assert!(error.to_string().contains("GEX-2580"), "{error}");
    }

    // --- rendering ---

    #[test]
    fn every_day_renders_in_french() {
        for (day, expected) in [
            (Day::Monday, "lundi"),
            (Day::Tuesday, "mardi"),
            (Day::Wednesday, "mercredi"),
            (Day::Thursday, "jeudi"),
            (Day::Friday, "vendredi"),
            (Day::Saturday, "samedi"),
            (Day::Sunday, "dimanche"),
        ] {
            let slot = Slot {
                day,
                start: Time {
                    hour: 8,
                    minute: 30,
                },
                end: Time {
                    hour: 11,
                    minute: 20,
                },
            };
            assert_eq!(render_slot(&slot), format!("{expected} 08:30–11:20"));
        }
    }

    #[test]
    fn a_slotless_section_renders_as_remote() {
        let section = Section {
            nrc: "20907".to_string(),
            section: Some("Z1".to_string()),
            mode: ulaval_scheduler_core::Mode::Remote,
            slots: Vec::new(),
        };
        assert_eq!(render_section(&section), "  20907 [Z1] : à distance");
    }

    #[test]
    fn an_unlabelled_section_omits_the_brackets() {
        let section = Section {
            nrc: "84664".to_string(),
            section: None,
            mode: ulaval_scheduler_core::Mode::InPerson,
            slots: vec![Slot {
                day: Day::Friday,
                start: Time {
                    hour: 12,
                    minute: 30,
                },
                end: Time {
                    hour: 15,
                    minute: 20,
                },
            }],
        };
        assert_eq!(render_section(&section), "  84664 : vendredi 12:30–15:20");
    }

    #[test]
    fn a_conflicting_course_is_marked_and_the_total_closes_the_render() {
        let courses = [monday("A-1", "1"), monday("B-2", "2")];
        let report = schedule_report(&courses, Season::Fall, &BTreeMap::new())
            .unwrap_or_else(|e| panic!("{e}"));

        let rendered = render(&report, 6);

        assert!(rendered.contains("A-1 ⚠ conflit"), "{rendered}");
        assert!(rendered.contains("B-2 ⚠ conflit"), "{rendered}");
        assert!(rendered.ends_with("Total : 6 crédits"), "{rendered}");
    }

    // --- run, end to end in-process ---

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ulaval-scheduler-cli-{name}"));
        // leftovers from an earlier failed run
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("programmes"))
            .unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
        dir
    }

    fn cleanup(dir: &PathBuf) {
        std::fs::remove_dir_all(dir)
            .unwrap_or_else(|e| panic!("cleanup {}: {e}", dir.display()));
    }

    fn write_snapshot(dir: &Path, courses: &[Course]) {
        let snapshot = serde_json::json!({ "courses": courses });
        std::fs::write(dir.join("cours.json"), snapshot.to_string())
            .unwrap_or_else(|e| panic!("write the snapshot: {e}"));
    }

    fn run_args(dir: &Path, rest: &[&str]) -> Vec<String> {
        let mut args = vec!["schedule".to_string()];
        args.extend(rest.iter().map(|arg| arg.to_string()));
        args.extend(["--data-dir".to_string(), dir.display().to_string()]);
        args
    }

    #[test]
    fn a_conflict_free_request_succeeds() {
        let dir = test_dir("happy");
        write_snapshot(
            &dir,
            &[
                monday("GEX-1000", "1"),
                course(
                    "GCI-1000",
                    "fall",
                    &format!(
                        "[{}]",
                        option_json("2", "tuesday", "08:30", "11:20")
                    ),
                ),
            ],
        );

        run(run_args(&dir, &["a2026", "gex-1000", "gci-1000"]))
            .unwrap_or_else(|e| panic!("conflict-free codes: {e}"));
        cleanup(&dir);
    }

    #[test]
    fn a_conflicting_request_prints_then_fails_naming_the_courses() {
        let dir = test_dir("conflict");
        write_snapshot(
            &dir,
            &[monday("GEX-1000", "1"), monday("GCI-1000", "2")],
        );

        let error = run(run_args(&dir, &["a2026", "GEX-1000", "GCI-1000"]))
            .expect_err("same monday slot");

        let message = error.to_string();
        assert!(message.contains("GEX-1000"), "{message}");
        assert!(message.contains("GCI-1000"), "{message}");
        cleanup(&dir);
    }

    #[test]
    fn a_missing_snapshot_says_how_to_produce_it() {
        let dir = test_dir("no-snapshot");

        let error = run(run_args(&dir, &["a2026", "GEX-1000"]))
            .expect_err("nothing scraped yet");

        assert!(error.to_string().contains("ulaval-scraper"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn an_unreadable_snapshot_is_a_parsing_error() {
        let dir = test_dir("bad-snapshot");
        std::fs::write(dir.join("cours.json"), "not json")
            .unwrap_or_else(|e| panic!("write the bad snapshot: {e}"));

        let error = run(run_args(&dir, &["a2026", "GEX-1000"]))
            .expect_err("unparseable snapshot");

        assert!(error.to_string().contains("Parsing"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_duplicated_code_aborts_the_run() {
        let dir = test_dir("duplicate");
        write_snapshot(&dir, &[monday("GEX-1000", "1")]);

        let error = run(run_args(&dir, &["a2026", "gex-1000", "GEX-1000"]))
            .expect_err("the same course twice");

        assert!(error.to_string().contains("duplicated"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_course_without_any_option_aborts_the_run() {
        // an offering with no enrolment option: the report refuses to
        // invent a selection and the run surfaces it
        let dir = test_dir("no-options");
        write_snapshot(&dir, &[course("GEX-1000", "fall", "[]")]);

        let error = run(run_args(&dir, &["a2026", "GEX-1000"]))
            .expect_err("nothing to enrol in");

        assert!(error.to_string().contains("GEX-1000"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn an_unpublished_schedule_aborts_the_weekly_schedule() {
        // the GCI-1011 shape (new-course rule): offered, but nothing can be
        // drawn on a weekly grid — refused loudly, never silently dropped
        let dir = test_dir("schedule-unknown");
        write_snapshot(
            &dir,
            &[vintage_course("GCI-1011", "fall", "null", "null")],
        );

        let error = run(run_args(&dir, &["a2026", "GCI-1011"]))
            .expect_err("no schedule to draw");

        let message = error.to_string();
        assert!(message.contains("GCI-1011"), "{message}");
        assert!(message.contains("not yet published"), "{message}");
        cleanup(&dir);
    }

    #[test]
    fn an_unknown_code_aborts_the_run() {
        let dir = test_dir("unknown-code");
        write_snapshot(&dir, &[monday("GEX-1000", "1")]);

        let error = run(run_args(&dir, &["a2026", "ZZZ-9999"]))
            .expect_err("no such course in the snapshot");

        assert!(error.to_string().contains("ZZZ-9999"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_variable_credit_stage_aborts_the_run() {
        let dir = test_dir("stage");
        let stage: Course = serde_json::from_str(&format!(
            r#"{{"code":"GEX-2580","title":"Stage",
                "credits":{{"min":6,"max":12}},"cycle":1,
                "prerequisites":null,"equivalents":[],
                "seasons":{{"fall":{{"options":[{}]}}}}}}"#,
            option_json("5", "monday", "08:30", "11:20"),
        ))
        .unwrap_or_else(|e| panic!("stage literal: {e}"));
        write_snapshot(&dir, &[stage]);

        let error = run(run_args(&dir, &["a2026", "GEX-2580"]))
            .expect_err("no chosen weighting");

        assert!(error.to_string().contains("GEX-2580"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_bad_session_fails_before_touching_the_disk() {
        let error = run(vec![
            "schedule".to_string(),
            "x2026".to_string(),
            "GEX-1000".to_string(),
        ])
        .expect_err("no such season letter");
        assert!(error.to_string().contains("a<year>"), "{error}");
    }

    #[test]
    fn help_prints_and_succeeds() {
        for flag in ["--help", "-h"] {
            let result = run(vec![flag.to_string()]);
            assert!(result.is_ok(), "{flag} is a help request");
        }
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error() {
        let error = run(vec![
            "schedule".to_string(),
            "a2026".to_string(),
            "GEX-1000".to_string(),
            "--nope".to_string(),
        ])
        .expect_err("unknown flag");
        assert!(error.to_string().contains("--nope"), "{error}");
    }

    // --- organigramme: sessions and pins ---

    #[test]
    fn sessions_alternate_automne_hiver_from_the_start() {
        assert_eq!(
            alternating_sessions(Season::Fall, 4),
            [Season::Fall, Season::Winter, Season::Fall, Season::Winter]
        );
        assert_eq!(
            alternating_sessions(Season::Winter, 3),
            [Season::Winter, Season::Fall, Season::Winter]
        );
        // été is never generated: a summer start flows into fall
        assert_eq!(
            alternating_sessions(Season::Summer, 3),
            [Season::Summer, Season::Fall, Season::Winter]
        );
    }

    #[test]
    fn pins_parse_and_uppercase_their_codes() {
        let pins =
            parse_pins(&["gci-1007=2".to_string(), "GEX-1002=1".to_string()])
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(pins["GCI-1007"], 2);
        assert_eq!(pins["GEX-1002"], 1);
    }

    #[test]
    fn a_malformed_pin_is_an_error_showing_the_expected_shape() {
        for spec in ["GCI-1007", "GCI-1007=two"] {
            let error =
                parse_pins(&[spec.to_string()]).expect_err("not CODE=SESSION");
            assert!(error.to_string().contains("CODE=SESSION"), "{error}");
        }
    }

    fn program_json(mandatory: &str, rules: &str) -> String {
        format!(
            r#"{{"code":"p","year":2026,"title":"P","cycle":1,
                 "credits_required":120,"mandatory":{mandatory},
                 "rules":{rules},"concentrations":[],"profiles":[]}}"#
        )
    }

    #[test]
    fn the_course_list_orders_mandatory_electives_then_passed_deduped() {
        let program: Program =
            serde_json::from_str(&program_json(r#"["M-1","M-2"]"#, "[]"))
                .unwrap_or_else(|e| panic!("program literal: {e}"));
        let list = course_list(
            Some(&program),
            &["E-1".to_string(), "M-2".to_string()],
            &["P-1".to_string(), "E-1".to_string()],
        );
        assert_eq!(list, ["M-1", "M-2", "E-1", "P-1"]);
    }

    // --- organigramme: rendering ---

    #[test]
    fn a_placement_renders_terms_count_and_assumptions() {
        let placement = Placement {
            completion: Completion::Complete,
            solutions: vec![Solution {
                placement: BTreeMap::from([
                    ("A-1".to_string(), 1),
                    ("B-2".to_string(), 1),
                ]),
                assumed: ["MAT-0130".to_string()].into_iter().collect(),
            }],
        };
        let courses = [monday("A-1", "1"), monday("B-2", "2")];
        let rendered = render_placement(
            &placement,
            &courses,
            &[Season::Fall, Season::Winter],
        );
        assert!(
            rendered.contains("Session 1 (Automne) : A-1, B-2 (6 cr)"),
            "{rendered}"
        );
        assert!(rendered.contains("Session 2 (Hiver) : —"), "{rendered}");
        assert!(
            rendered.contains("1 solution(s) (ensemble complet)"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Préalables présumés (à vérifier) : MAT-0130"),
            "{rendered}"
        );
    }

    #[test]
    fn an_empty_set_renders_only_the_count_and_its_cause() {
        let placement = Placement {
            completion: Completion::NodeBudget,
            solutions: Vec::new(),
        };
        assert_eq!(
            render_placement(&placement, &[], &[Season::Fall]),
            "0 solution(s) (budget de nœuds épuisé — ensemble partiel)"
        );
    }

    #[test]
    fn a_capped_set_names_the_solution_cap_and_omits_empty_assumptions() {
        let placement = Placement {
            completion: Completion::SolutionCap,
            solutions: vec![Solution {
                placement: BTreeMap::from([("A-1".to_string(), 1)]),
                assumed: BTreeSet::new(),
            }],
        };
        let courses = [monday("A-1", "1")];
        let rendered = render_placement(&placement, &courses, &[Season::Fall]);
        assert!(
            rendered.contains(
                "1 solution(s) (plafond de solutions atteint — ensemble \
                 partiel)"
            ),
            "{rendered}"
        );
        assert!(!rendered.contains("Préalables présumés"), "{rendered}");
    }

    #[test]
    fn every_season_labels_in_french() {
        assert_eq!(season_label(Season::Fall), "Automne");
        assert_eq!(season_label(Season::Winter), "Hiver");
        assert_eq!(season_label(Season::Summer), "Été");
    }

    #[test]
    fn the_coverage_report_renders_every_verdict_shape() {
        use ulaval_scheduler_core::{LanguageReport, RuleReport, Scope};
        let rule = |title: &str,
                    status: RuleStatus,
                    missing: Option<Missing>,
                    candidates: Option<usize>,
                    raw: Option<&str>| {
            RuleReport {
                scope: Scope::Program,
                title: title.to_string(),
                status,
                counted: candidates.map(|_| Vec::new()),
                missing,
                candidates: candidates
                    .map(|count| vec!["X-1".to_string(); count]),
                raw: raw.map(str::to_string),
            }
        };
        let coverage = CoverageReport {
            mandatory: vec![
                MandatoryReport {
                    scope: Scope::Program,
                    satisfied: vec!["A-1".to_string()],
                    missing: Vec::new(),
                },
                MandatoryReport {
                    scope: Scope::Program,
                    satisfied: Vec::new(),
                    missing: vec!["B-2".to_string(), "C-3".to_string()],
                },
            ],
            rules: vec![
                rule("Règle 1", RuleStatus::Satisfied, None, Some(2), None),
                rule(
                    "Règle 2",
                    RuleStatus::Incomplete,
                    Some(Missing::Count { count: 1 }),
                    Some(3),
                    None,
                ),
                rule(
                    "Règle 3",
                    RuleStatus::Incomplete,
                    Some(Missing::Credits { credits: 3 }),
                    Some(4),
                    None,
                ),
                rule(
                    "Règle 5",
                    RuleStatus::Reported,
                    None,
                    None,
                    Some("tous les cours"),
                ),
                rule("Règle 6", RuleStatus::Reported, None, None, None),
            ],
            language_requirement: Some(LanguageReport {
                status: LanguageStatus::Reported,
            }),
        };
        let rendered = render_coverage(&coverage);
        assert!(
            rendered.contains("Cours obligatoires : complets (1)"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Cours obligatoires : manquants — B-2, C-3"),
            "{rendered}"
        );
        assert!(rendered.contains("Règle 1 : satisfaite"), "{rendered}");
        assert!(
            rendered.contains("Règle 2 : à combler — 1 cours (candidats : 3)"),
            "{rendered}"
        );
        assert!(
            rendered
                .contains("Règle 3 : à combler — 3 crédits (candidats : 4)"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Règle 5 : à valider — « tous les cours »"),
            "{rendered}"
        );
        assert!(rendered.contains("Règle 6 : à valider"), "{rendered}");
        assert!(
            rendered.contains("Exigence linguistique : à valider"),
            "{rendered}"
        );
    }

    #[test]
    fn a_satisfied_language_requirement_renders_as_such() {
        use ulaval_scheduler_core::LanguageReport;
        let coverage = CoverageReport {
            mandatory: Vec::new(),
            rules: Vec::new(),
            language_requirement: Some(LanguageReport {
                status: LanguageStatus::Satisfied,
            }),
        };
        assert!(render_coverage(&coverage)
            .contains("Exigence linguistique : satisfaite"));
    }

    // --- organigramme: end to end in-process ---

    fn organigramme_args(dir: &Path, rest: &[&str]) -> Vec<String> {
        let mut args = vec!["organigramme".to_string()];
        args.extend(rest.iter().map(|arg| arg.to_string()));
        args.extend(["--data-dir".to_string(), dir.display().to_string()]);
        args
    }

    // one multi-season course, the shape the single snapshot holds whole
    fn fall_winter(code: &str, fall: &str, winter: &str) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{{
                   "fall":{{"last_offered":2026,
                            "options":[{fall}]}},
                   "winter":{{"last_offered":2026,
                              "options":[{winter}]}}}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    #[test]
    fn a_feasible_organigramme_prints_and_succeeds() {
        let dir = test_dir("organigramme-happy");
        write_snapshot(
            &dir,
            &[
                fall_winter(
                    "GEX-1000",
                    &option_json("1", "monday", "08:30", "11:20"),
                    &option_json("3", "monday", "08:30", "11:20"),
                ),
                fall_winter(
                    "GCI-1000",
                    &option_json("2", "tuesday", "08:30", "11:20"),
                    &option_json("4", "monday", "08:30", "11:20"),
                ),
            ],
        );

        run(organigramme_args(
            &dir,
            &[
                "a2026",
                "gex-1000",
                "gci-1000",
                "--credit-cap",
                "6",
                "--sessions",
                "2",
                "--pinned",
                "gex-1000=1",
            ],
        ))
        .unwrap_or_else(|e| panic!("feasible organigramme: {e}"));
        cleanup(&dir);
    }

    #[test]
    fn an_unknown_typed_code_aborts_the_organigramme() {
        let dir = test_dir("organigramme-unknown");
        write_snapshot(&dir, &[monday("GEX-1000", "1")]);

        let error = run(organigramme_args(
            &dir,
            &["a2026", "ZZZ-9999", "--credit-cap", "6", "--sessions", "1"],
        ))
        .expect_err("no such course anywhere");

        assert!(error.to_string().contains("ZZZ-9999"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_proven_infeasible_placement_exits_with_an_error() {
        // two courses on the same monday slot, one single session: the
        // weekly veto forbids the only assignment — proven, not guessed
        let dir = test_dir("organigramme-infeasible");
        write_snapshot(
            &dir,
            &[monday("GEX-1000", "1"), monday("GCI-1000", "2")],
        );

        let error = run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "GCI-1000",
                "--credit-cap",
                "30",
                "--sessions",
                "1",
            ],
        ))
        .expect_err("nothing fits one session");

        assert!(error.to_string().contains("proven"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_missing_snapshot_names_the_scraper_organigramme() {
        let dir = test_dir("organigramme-no-snapshot");

        let error = run(organigramme_args(
            &dir,
            &["a2026", "GEX-1000", "--credit-cap", "6"],
        ))
        .expect_err("nothing scraped yet");

        let message = error.to_string();
        assert!(message.contains("cours.json"), "{message}");
        assert!(message.contains("ulaval-scraper"), "{message}");
        cleanup(&dir);
    }

    #[test]
    fn an_old_vintage_is_still_known_and_placeable() {
        // the founding hypothesis, per course now: an offering last read
        // from automne 2025 serves an a2026 plan — a course is never lost
        // to a newer snapshot of its season (ADR
        // `2026-07-snapshot-unique-des-cours-millesime-par-saison`)
        let dir = test_dir("organigramme-old-vintage");
        write_snapshot(
            &dir,
            &[vintage_course(
                "GEX-1000",
                "fall",
                "2025",
                &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
            )],
        );

        run(organigramme_args(
            &dir,
            &["a2026", "GEX-1000", "--credit-cap", "6", "--sessions", "1"],
        ))
        .unwrap_or_else(|e| panic!("an old vintage still places: {e}"));
        cleanup(&dir);
    }

    #[test]
    fn an_unpublished_schedule_still_places_in_the_organigramme() {
        // the GCI-1011 shape: offered fall and winter, no vintage, no
        // schedule — placeable by B even though A refuses to draw it
        let dir = test_dir("organigramme-schedule-unknown");
        write_snapshot(
            &dir,
            &[
                monday("GEX-1000", "1"),
                vintage_course("GCI-1011", "fall", "null", "null"),
            ],
        );

        run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "GCI-1011",
                "--credit-cap",
                "6",
                "--sessions",
                "1",
            ],
        ))
        .unwrap_or_else(|e| panic!("an unpublished schedule places: {e}"));
        cleanup(&dir);
    }

    #[test]
    fn a_program_pulls_its_mandatory_and_prints_coverage() {
        // GHOST-999 is mandatory but has no snapshot data: set aside
        // loudly, the rest placed, the coverage printed (ADR
        // `2026-07-cours-sans-offre-ecarte-par-le-harnais`)
        let dir = test_dir("organigramme-program");
        write_snapshot(
            &dir,
            &[
                monday("GEX-1000", "1"),
                course(
                    "GCI-1000",
                    "fall",
                    &format!(
                        "[{}]",
                        option_json("2", "tuesday", "08:30", "11:20")
                    ),
                ),
            ],
        );
        std::fs::write(
            dir.join("programmes").join("prog-2026.json"),
            program_json(
                r#"["GEX-1000","GHOST-999"]"#,
                r#"[{"title":"Règle 1","constraint":{"count":1},
                     "courses":["GCI-1000","GCI-2000"]}]"#,
            ),
        )
        .unwrap_or_else(|e| panic!("write the program: {e}"));

        run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GCI-1000",
                "--program",
                "prog-2026",
                "--credit-cap",
                "30",
                "--sessions",
                "1",
            ],
        ))
        .unwrap_or_else(|e| panic!("program-driven organigramme: {e}"));
        cleanup(&dir);
    }

    #[test]
    fn a_missing_program_file_names_the_scraper() {
        let dir = test_dir("organigramme-no-program");
        write_snapshot(&dir, &[monday("GEX-1000", "1")]);

        let error = run(organigramme_args(
            &dir,
            &[
                "a2026",
                "--program",
                "nope-2026",
                "--credit-cap",
                "6",
                "--sessions",
                "1",
            ],
        ))
        .expect_err("no such program snapshot");

        assert!(
            error.to_string().contains("ulaval-scraper program"),
            "{error}"
        );
        cleanup(&dir);
    }

    #[test]
    fn an_unreadable_program_file_is_a_parsing_error() {
        let dir = test_dir("organigramme-bad-program");
        write_snapshot(&dir, &[monday("GEX-1000", "1")]);
        std::fs::write(
            dir.join("programmes").join("bad-2026.json"),
            "not json",
        )
        .unwrap_or_else(|e| panic!("write the bad program: {e}"));

        let error = run(organigramme_args(
            &dir,
            &[
                "a2026",
                "--program",
                "bad-2026",
                "--credit-cap",
                "6",
                "--sessions",
                "1",
            ],
        ))
        .expect_err("unparseable program");

        assert!(error.to_string().contains("Parsing"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_passed_course_is_not_placed_by_the_harness() {
        let dir = test_dir("organigramme-passed");
        write_snapshot(
            &dir,
            &[monday("GEX-1000", "1"), monday("GCI-1000", "2")],
        );

        // both share the monday slot, but GEX-1000 is passed: only
        // GCI-1000 is placed, so the single session suffices
        run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "GCI-1000",
                "--passed",
                "gex-1000",
                "--credit-cap",
                "6",
                "--sessions",
                "1",
            ],
        ))
        .unwrap_or_else(|e| panic!("passed course still blocks: {e}"));
        cleanup(&dir);
    }

    #[test]
    fn a_bad_start_session_fails_before_touching_the_disk() {
        let error = run(vec![
            "organigramme".to_string(),
            "x2026".to_string(),
            "--credit-cap".to_string(),
            "6".to_string(),
        ])
        .expect_err("no such season letter");
        assert!(error.to_string().contains("a<year>"), "{error}");
    }

    #[test]
    fn duplicated_codes_or_passed_abort_the_organigramme() {
        let dir = test_dir("organigramme-duplicates");
        write_snapshot(&dir, &[monday("GEX-1000", "1")]);

        let error = run(organigramme_args(
            &dir,
            &[
                "a2026",
                "gex-1000",
                "GEX-1000",
                "--credit-cap",
                "6",
                "--sessions",
                "1",
            ],
        ))
        .expect_err("the same elective twice");
        assert!(error.to_string().contains("duplicated"), "{error}");

        let error = run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "--passed",
                "gex-1000,GEX-1000",
                "--credit-cap",
                "6",
                "--sessions",
                "1",
            ],
        ))
        .expect_err("the same passed course twice");
        assert!(error.to_string().contains("duplicated"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_malformed_pin_aborts_the_organigramme() {
        let dir = test_dir("organigramme-bad-pin");
        write_snapshot(&dir, &[monday("GEX-1000", "1")]);

        let error = run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "--pinned",
                "GEX-1000",
                "--credit-cap",
                "6",
                "--sessions",
                "1",
            ],
        ))
        .expect_err("no session number");

        assert!(error.to_string().contains("CODE=SESSION"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_course_both_passed_and_pinned_aborts_through_the_placement() {
        let dir = test_dir("organigramme-passed-pinned");
        write_snapshot(&dir, &[monday("GEX-1000", "1")]);

        let error = run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "--passed",
                "gex-1000",
                "--pinned",
                "gex-1000=1",
                "--credit-cap",
                "6",
                "--sessions",
                "1",
            ],
        ))
        .expect_err("contradictory constraints");

        assert!(error.to_string().contains("passed and pinned"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn an_over_selected_credits_rule_aborts_the_coverage() {
        // 6 credits selected on a min-3-max-3 rule: the undecided
        // semantics surface as the verifier's typed error (ADR
        // `2026-07-somme-au-dessus-du-max-en-erreur-typee`)
        let dir = test_dir("organigramme-over-max");
        write_snapshot(
            &dir,
            &[
                monday("GEX-1000", "1"),
                course(
                    "GCI-1000",
                    "fall",
                    &format!(
                        "[{}]",
                        option_json("2", "tuesday", "08:30", "11:20")
                    ),
                ),
            ],
        );
        std::fs::write(
            dir.join("programmes").join("prog-2026.json"),
            program_json(
                "[]",
                r#"[{"title":"Règle 1","constraint":{"min":3,"max":3},
                     "courses":["GEX-1000","GCI-1000"]}]"#,
            ),
        )
        .unwrap_or_else(|e| panic!("write the program: {e}"));

        let error = run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "GCI-1000",
                "--program",
                "prog-2026",
                "--credit-cap",
                "30",
                "--sessions",
                "1",
            ],
        ))
        .expect_err("6 credits above max 3");

        assert!(error.to_string().contains("above the max"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn an_unreadable_course_snapshot_aborts_the_organigramme() {
        let dir = test_dir("organigramme-bad-snapshot");
        std::fs::write(dir.join("cours.json"), "not json")
            .unwrap_or_else(|e| panic!("write the bad snapshot: {e}"));

        let error = run(organigramme_args(
            &dir,
            &["a2026", "GEX-1000", "--credit-cap", "6", "--sessions", "1"],
        ))
        .expect_err("unparseable snapshot");

        assert!(error.to_string().contains("Parsing"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn budget_bounds_are_reported_not_fatal() {
        let dir = test_dir("organigramme-budget");
        write_snapshot(
            &dir,
            &[fall_winter(
                "GEX-1000",
                &option_json("1", "monday", "08:30", "11:20"),
                &option_json("3", "monday", "08:30", "11:20"),
            )],
        );

        // an exhausted node budget is a partial set, never « infeasible »
        run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "--credit-cap",
                "6",
                "--sessions",
                "2",
                "--max-nodes",
                "1",
            ],
        ))
        .unwrap_or_else(|e| panic!("budget stop is not an error: {e}"));

        // a hit solution cap likewise
        run(organigramme_args(
            &dir,
            &[
                "a2026",
                "GEX-1000",
                "--credit-cap",
                "6",
                "--sessions",
                "2",
                "--max-solutions",
                "1",
            ],
        ))
        .unwrap_or_else(|e| panic!("solution cap is not an error: {e}"));
        cleanup(&dir);
    }
}
