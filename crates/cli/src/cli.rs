use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clap::Parser;

use ulaval_scheduler_core::{
    resolve_offering, schedule_report, Course, CourseReport, Day,
    ScheduleReport, Season, Section, Slot,
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
        // season letter + year, matching the snapshot file names:
        // a2026 (automne), h2027 (hiver), e2026 (été)
        session: String,
        #[arg(required = true, num_args = 1..)]
        codes: Vec<String>,
        #[arg(long, default_value = "data")]
        data_dir: String,
    },
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
    }
}

fn print_schedule(
    session: &str,
    codes: &[String],
    data_dir: &str,
) -> anyhow::Result<()> {
    let (season, year) = parse_session(session)?;
    let snapshot = read_snapshot(data_dir, session)?;
    let codes = normalize_codes(codes)?;
    let courses =
        select_courses(&snapshot.courses, &codes, season, year, session)?;

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

// `a2026` → (Fall, 2026); the letter is the season of the snapshot file
// names (a = automne, h = hiver, e = été)
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

fn read_snapshot(data_dir: &str, session: &str) -> anyhow::Result<Snapshot> {
    let path = Path::new(data_dir)
        .join("cours")
        .join(format!("{session}.json"));
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
    year: u16,
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
            effective_course(by_code[code.as_str()], &by_code, season, year)
                .ok_or_else(|| {
                    anyhow::anyhow!("{code} is not offered in {session}")
                })
        })
        .collect()
}

// The offering actually attended may come from an equivalent — the most
// recent session vintage wins, ties to the course (ADR
// `2026-07-equivalences-par-millesime-de-session`). One snapshot means one
// vintage everywhere, so today the course's own offering wins whenever it
// exists; the fold is already shaped for the multi-snapshot fallback. The
// requested course keeps its identity: only the offering is borrowed.
fn effective_course(
    course: &Course,
    by_code: &BTreeMap<&str, &Course>,
    season: Season,
    year: u16,
) -> Option<Course> {
    let seed = course.seasons.get(&season).map(|offering| (offering, year));
    let (offering, _) = course
        .equivalents
        .iter()
        .filter_map(|code| by_code.get(code.as_str()))
        .filter_map(|equivalent| {
            equivalent
                .seasons
                .get(&season)
                .map(|offering| (offering, year))
        })
        .fold(seed, |acc, pair| resolve_offering(acc, Some(pair)))?;
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
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{{"{season}":{{"options":{options}}}}}}}"#
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
            2026,
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
            2026,
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
            2026,
            "a2026",
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(courses[0].code, "GEX-1000");
        let offering = &courses[0].seasons[&Season::Fall];
        assert_eq!(offering.options[0][0].nrc, "7");
    }

    #[test]
    fn a_courses_own_offering_wins_over_its_equivalents() {
        // one snapshot = one vintage everywhere, and ties go to the course
        let mut wanted = monday("GEX-1000", "1");
        wanted.equivalents = vec!["GCI-1000".to_string()];
        let all = [wanted, monday("GCI-1000", "7")];

        let courses = select_courses(
            &all,
            &["GEX-1000".to_string()],
            Season::Fall,
            2026,
            "a2026",
        )
        .unwrap_or_else(|e| panic!("{e}"));

        let offering = &courses[0].seasons[&Season::Fall];
        assert_eq!(offering.options[0][0].nrc, "1");
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
        std::fs::create_dir_all(dir.join("cours"))
            .unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
        dir
    }

    fn cleanup(dir: &PathBuf) {
        std::fs::remove_dir_all(dir)
            .unwrap_or_else(|e| panic!("cleanup {}: {e}", dir.display()));
    }

    fn write_snapshot(dir: &Path, session: &str, courses: &[Course]) {
        let snapshot = serde_json::json!({ "courses": courses });
        std::fs::write(
            dir.join("cours").join(format!("{session}.json")),
            snapshot.to_string(),
        )
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
            "a2026",
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
            "a2026",
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
        std::fs::write(dir.join("cours").join("a2026.json"), "not json")
            .unwrap_or_else(|e| panic!("write the bad snapshot: {e}"));

        let error = run(run_args(&dir, &["a2026", "GEX-1000"]))
            .expect_err("unparseable snapshot");

        assert!(error.to_string().contains("Parsing"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn a_duplicated_code_aborts_the_run() {
        let dir = test_dir("duplicate");
        write_snapshot(&dir, "a2026", &[monday("GEX-1000", "1")]);

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
        write_snapshot(&dir, "a2026", &[course("GEX-1000", "fall", "[]")]);

        let error = run(run_args(&dir, &["a2026", "GEX-1000"]))
            .expect_err("nothing to enrol in");

        assert!(error.to_string().contains("GEX-1000"), "{error}");
        cleanup(&dir);
    }

    #[test]
    fn an_unknown_code_aborts_the_run() {
        let dir = test_dir("unknown-code");
        write_snapshot(&dir, "a2026", &[monday("GEX-1000", "1")]);

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
        write_snapshot(&dir, "a2026", &[stage]);

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
}
