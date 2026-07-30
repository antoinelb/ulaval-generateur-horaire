use std::collections::{BTreeMap, BTreeSet};

use crate::course::{Course, Season, SeasonOffering, Section};
use crate::week::{slots_to_mask, WeekMask};

// One enrolment alternative of a course: the NRC of every section taken
// together, and the union of their occupied buckets. The set is ordered so
// a chosen schedule serializes deterministically (URL sharing later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opt {
    pub nrc_set: BTreeSet<String>,
    pub mask: WeekMask,
}

// The chosen NRC of a whole week — ordered so it serializes
// deterministically (URL sharing later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub nrcs: BTreeSet<String>,
}

// The weekly-schedule report the UI renders (ADR
// `2026-07-contrat-horaire-hebdomadaire-vers-ui`): sections embarked whole
// so the output is self-contained, selection by the deterministic « first
// feasible » rule, and the optional `valid: false` markers the UI
// highlights. Jalon 10's preference ranking will replace the selection
// rule; the rest of the contract does not depend on it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ScheduleReport {
    pub valid: bool,
    pub courses: Vec<CourseReport>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CourseReport {
    pub code: String,
    // marked on the course itself iff its selection overlaps another
    // course's selection
    #[serde(skip_serializing_if = "is_true")]
    pub valid: bool,
    pub selected: Vec<Section>,
    pub alternatives: Vec<Alternative>,
}

// a non-selected option, in snapshot order
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Alternative {
    pub sections: Vec<Section>,
    // swap semantics: invalid iff it overlaps the current selection of
    // *another* course, the other courses never moving
    #[serde(skip_serializing_if = "is_true")]
    pub valid: bool,
}

// Inputs the report refuses to guess about — surfaced to the student,
// never patched over (« never lose input silently »).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScheduleError {
    #[error("{code} is not offered in the requested season")]
    NotOffered { code: String },
    #[error("{code} has no enrolment option in the requested season")]
    NoOptions { code: String },
    #[error("a pinned option names no requested course : {code}")]
    UnknownChosenCourse { code: String },
    #[error("no option of {code} matches the pinned NRC set")]
    ChosenOptionAbsent { code: String },
}

// a course's snapshot options next to their precomputed masks, the domain
// already restricted to the pinned option when one is given
struct CourseDomain<'a> {
    code: &'a str,
    options: &'a [Vec<Section>],
    opts: Vec<Opt>,
    allowed: Vec<usize>,
}

// Between a course's offering and its equivalent's, keep the most recent
// session vintage; the year comes from the snapshot file name, which
// `core` never sees, so the caller supplies it (ADR
// `2026-07-equivalences-par-millesime-de-session`). Ties go to the course
// itself, and the winning *pair* is returned so several equivalents fold:
// `equivalents.fold(Some(course_pair), |acc, e| resolve_offering(acc, Some(e)))`.
pub fn resolve_offering<'a>(
    course: Option<(&'a SeasonOffering, u16)>,
    equivalent: Option<(&'a SeasonOffering, u16)>,
) -> Option<(&'a SeasonOffering, u16)> {
    match (course, equivalent) {
        (Some((_, year)), Some(pair)) if pair.1 > year => Some(pair),
        (Some(pair), _) => Some(pair),
        (None, other) => other,
    }
}

// One `Opt` per `options[i]`: an option is a *complete* enrolment (ADR
// `2026-07-sections-en-combinaisons-valides`), so its mask is the union of
// its sections' slots.
pub fn build_domain(offering: &SeasonOffering) -> Vec<Opt> {
    offering
        .options
        .iter()
        .map(|sections| build_opt(sections))
        .collect()
}

fn build_opt(sections: &[Section]) -> Opt {
    Opt {
        nrc_set: sections.iter().map(|s| s.nrc.clone()).collect(),
        mask: sections.iter().fold(WeekMask::EMPTY, |acc, s| {
            acc.merge(&slots_to_mask(&s.slots))
        }),
    }
}

// Forcing a NRC keeps every option whose section set *contains* it — never
// « option k » : a NRC may sit in several options (CSO-6702's common
// seminar), and forcing it must keep them all. An absent NRC empties the
// domain.
pub fn force_nrc(domain: Vec<Opt>, nrc: &str) -> Vec<Opt> {
    domain
        .into_iter()
        .filter(|opt| opt.nrc_set.contains(nrc))
        .collect()
}

// The pure function the UI calls on every add, removal or section change
// (ADR `2026-07-contrat-horaire-hebdomadaire-vers-ui`). `chosen` pins one
// option per course, identified by its sorted NRC set — an option has no
// identifier of its own. When no conflict-free combination exists, the
// pins are kept (the student's explicit choice is never undone), every
// other course takes its first option, and the overlapping courses are
// marked.
pub fn schedule_report(
    courses: &[Course],
    season: Season,
    chosen: &BTreeMap<String, BTreeSet<String>>,
) -> Result<ScheduleReport, ScheduleError> {
    if let Some(code) = chosen
        .keys()
        .find(|code| !courses.iter().any(|course| &course.code == *code))
    {
        return Err(ScheduleError::UnknownChosenCourse { code: code.clone() });
    }

    let domains = courses
        .iter()
        .map(|course| course_domain(course, season, chosen.get(&course.code)))
        .collect::<Result<Vec<_>, _>>()?;

    let restricted: Vec<Vec<Opt>> = domains
        .iter()
        .map(|domain| {
            domain
                .allowed
                .iter()
                .map(|&k| domain.opts[k].clone())
                .collect()
        })
        .collect();
    let first = enumerate(&restricted).into_iter().next();
    let valid = first.is_some();
    // leaf indices point into `allowed`; every course keeps a non-empty
    // `allowed` (guaranteed above), so the infeasible fallback — the pinned
    // option, or the first snapshot option — is `allowed[0]` either way
    let selection: Vec<usize> = match first {
        Some(leaf) => domains
            .iter()
            .zip(&leaf)
            .map(|(domain, &k)| domain.allowed[k])
            .collect(),
        None => domains.iter().map(|domain| domain.allowed[0]).collect(),
    };
    let masks: Vec<WeekMask> = domains
        .iter()
        .zip(&selection)
        .map(|(domain, &k)| domain.opts[k].mask)
        .collect();

    Ok(ScheduleReport {
        valid,
        courses: domains
            .iter()
            .enumerate()
            .map(|(i, domain)| course_report(domain, i, selection[i], &masks))
            .collect(),
    })
}

fn course_domain<'a>(
    course: &'a Course,
    season: Season,
    pin: Option<&BTreeSet<String>>,
) -> Result<CourseDomain<'a>, ScheduleError> {
    let offering = course.seasons.get(&season).ok_or_else(|| {
        ScheduleError::NotOffered {
            code: course.code.clone(),
        }
    })?;
    let opts = build_domain(offering);
    if opts.is_empty() {
        return Err(ScheduleError::NoOptions {
            code: course.code.clone(),
        });
    }
    let allowed: Vec<usize> = match pin {
        // a pin names an option by its whole sorted NRC set — equality, not
        // the single-NRC containment of `force_nrc`
        Some(nrcs) => opts
            .iter()
            .enumerate()
            .filter(|(_, opt)| &opt.nrc_set == nrcs)
            .map(|(k, _)| k)
            .collect(),
        None => (0..opts.len()).collect(),
    };
    if allowed.is_empty() {
        return Err(ScheduleError::ChosenOptionAbsent {
            code: course.code.clone(),
        });
    }
    Ok(CourseDomain {
        code: &course.code,
        options: &offering.options,
        opts,
        allowed,
    })
}

fn course_report(
    domain: &CourseDomain,
    course: usize,
    selected: usize,
    selection_masks: &[WeekMask],
) -> CourseReport {
    CourseReport {
        code: domain.code.to_string(),
        valid: fits_alongside_others(
            &domain.opts[selected].mask,
            course,
            selection_masks,
        ),
        selected: domain.options[selected].clone(),
        alternatives: domain
            .options
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != selected)
            .map(|(k, sections)| Alternative {
                sections: sections.clone(),
                valid: fits_alongside_others(
                    &domain.opts[k].mask,
                    course,
                    selection_masks,
                ),
            })
            .collect(),
    }
}

// swap semantics: a mask fits iff it overlaps no *other* course's current
// selection — the other courses never move
fn fits_alongside_others(
    mask: &WeekMask,
    course: usize,
    selection_masks: &[WeekMask],
) -> bool {
    selection_masks
        .iter()
        .enumerate()
        .all(|(other, selected)| other == course || !mask.overlaps(selected))
}

// the `valid` key is omitted when true, on courses and alternatives alike —
// the fixtures only ever write it as `false`
fn is_true(valid: &bool) -> bool {
    *valid
}

// The deterministic « first feasible schedule » of the frozen contract:
// courses in input order, options in snapshot order — exactly the first
// leaf of `enumerate`'s fold. `Score` waits for jalon 10's preference
// semantics (ADR `2026-07-score-de-a-reporte-au-jalon-10`).
pub fn best_schedule(courses: &[Vec<Opt>]) -> Option<Schedule> {
    enumerate(courses)
        .into_iter()
        .next()
        .map(|leaf| schedule_from(courses, &leaf))
}

fn schedule_from(courses: &[Vec<Opt>], leaf: &[usize]) -> Schedule {
    Schedule {
        // indexing is safe: a leaf of `enumerate(courses)` holds one
        // in-range option index per course by construction
        nrcs: courses
            .iter()
            .zip(leaf)
            .flat_map(|(options, &k)| options[k].nrc_set.iter().cloned())
            .collect(),
    }
}

// B's veto: only whether a conflict-free combination exists. Carries bare
// mask frontiers and short-circuits the moment one empties, instead of
// paying `enumerate`'s full collection — this is B's hot path, even
// memoized.
pub fn is_feasible(courses: &[Vec<Opt>]) -> bool {
    courses
        .iter()
        .try_fold(vec![WeekMask::EMPTY], |frontier, options| {
            let next: Vec<WeekMask> = frontier
                .iter()
                .flat_map(|acc| {
                    options
                        .iter()
                        .filter(|opt| !acc.overlaps(&opt.mask))
                        .map(|opt| acc.merge(&opt.mask))
                })
                .collect();
            (!next.is_empty()).then_some(next)
        })
        .is_some()
}

// The incremental pruned product (conception §4): from the empty
// assignment, extend every valid prefix by each compatible option, course
// by course — a `fold`, satisfying both « no `while` » and « no
// recursion ». The space is tiny (a2026: 1.21 options/course on average,
// n ≈ 5 per session), so **all** valid leaves are collected — jalon 10's
// ranking needs them. Leaves carry option indices rather than the
// sketch's `Schedule`: `Opt.nrc_set` loses the snapshot order the UI
// contract requires.
pub fn enumerate(courses: &[Vec<Opt>]) -> Vec<Vec<usize>> {
    courses
        .iter()
        .fold(vec![(WeekMask::EMPTY, Vec::new())], |partials, options| {
            partials
                .iter()
                .flat_map(|(acc, chosen)| {
                    options
                        .iter()
                        .enumerate()
                        .filter(|(_, opt)| !acc.overlaps(&opt.mask))
                        .map(move |(k, opt)| {
                            let mut next = chosen.clone();
                            next.push(k);
                            (acc.merge(&opt.mask), next)
                        })
                })
                .collect()
        })
        .into_iter()
        .map(|(_, chosen)| chosen)
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::course::Course;
    use proptest::prelude::*;

    const GCI_1007: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/test_cases/courses/gci-1007.json"
    ));
    const CSO_6702: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/test_cases/courses/cso-6702.json"
    ));

    // The single offering of a one-season fixture course, whichever season
    // carries it.
    fn only_offering(raw: &str) -> SeasonOffering {
        let course: Course = serde_json::from_str(raw).expect("fixture");
        let mut seasons = course.seasons.into_values();
        let offering = seasons.next().expect("one offering");
        assert!(seasons.next().is_none(), "expected a one-season fixture");
        offering
    }

    fn nrcs(opt: &Opt) -> Vec<&str> {
        opt.nrc_set.iter().map(String::as_str).collect()
    }

    // --- build_domain: one Opt per option, mask = union of its sections ---

    #[test]
    fn gci_1007_builds_one_opt_per_option_with_unioned_masks() {
        // real multi-option course: the friday lecture 84664 rides along
        // with lab A (wednesday) or lab B (friday morning)
        let domain = build_domain(&only_offering(GCI_1007));
        assert_eq!(domain.len(), 2);
        assert_eq!(nrcs(&domain[0]), ["84664", "84665"]);
        assert_eq!(nrcs(&domain[1]), ["84664", "84666"]);
        for opt in &domain {
            assert!(!opt.mask.is_empty());
        }
        // both options contain the same friday lecture, so their unioned
        // masks share its buckets
        assert!(domain[0].mask.overlaps(&domain[1].mask));
    }

    #[test]
    fn a_fully_remote_offering_yields_opts_with_empty_masks() {
        // a remote option never conflicts and always fits
        let offering: SeasonOffering = serde_json::from_str(
            r#"{"options":[[{"nrc":"20907","section":"Z1","mode":"remote","slots":[]}]]}"#,
        )
        .expect("offering");
        let domain = build_domain(&offering);
        assert_eq!(domain.len(), 1);
        assert_eq!(nrcs(&domain[0]), ["20907"]);
        assert!(domain[0].mask.is_empty());
    }

    #[test]
    fn an_offering_with_no_options_yields_an_empty_domain() {
        let offering: SeasonOffering =
            serde_json::from_str(r#"{"options":[]}"#).expect("offering");
        assert!(build_domain(&offering).is_empty());
    }

    #[test]
    fn an_option_with_no_sections_yields_an_empty_opt() {
        // degenerate but representable input: no panic, an empty Opt that
        // any later `force_nrc` naturally drops
        let offering: SeasonOffering =
            serde_json::from_str(r#"{"options":[[]]}"#).expect("offering");
        let domain = build_domain(&offering);
        assert_eq!(domain.len(), 1);
        assert!(domain[0].nrc_set.is_empty());
        assert!(domain[0].mask.is_empty());
    }

    // --- force_nrc: containment, never « option k » ---

    #[test]
    fn forcing_a_shared_nrc_keeps_every_option_containing_it() {
        // CSO-6702 hangs both sections off the common seminar 13449:
        // forcing it must keep both options (cf. course.rs test
        // `one_nrc_may_appear_in_several_options`)
        let domain = build_domain(&only_offering(CSO_6702));
        let forced = force_nrc(domain, "13449");
        assert_eq!(forced.len(), 2);
    }

    #[test]
    fn forcing_an_exclusive_nrc_keeps_only_its_option() {
        let domain = build_domain(&only_offering(CSO_6702));
        let forced = force_nrc(domain, "13450");
        assert_eq!(forced.len(), 1);
        assert!(forced[0].nrc_set.contains("13450"));
    }

    #[test]
    fn forcing_an_absent_nrc_empties_the_domain() {
        let domain = build_domain(&only_offering(CSO_6702));
        assert!(force_nrc(domain, "99999").is_empty());
    }

    // --- enumerate / is_feasible / best_schedule: the search ---

    // one full word of the week: same word → conflict, different → disjoint
    fn word_mask(word: usize) -> WeekMask {
        let mut words = [0u64; 32];
        words[word] = u64::MAX;
        WeekMask(words)
    }

    fn opt(nrc: &str, mask: WeekMask) -> Opt {
        Opt {
            nrc_set: std::iter::once(nrc.to_string()).collect(),
            mask,
        }
    }

    #[test]
    fn enumerate_collects_every_leaf_in_lexicographic_order() {
        // two courses × two disjoint options each: all four combinations,
        // course-major — the first leaf is the frozen « first feasible »
        let a = vec![opt("1", word_mask(0)), opt("2", word_mask(1))];
        let b = vec![opt("3", word_mask(2)), opt("4", word_mask(3))];
        assert_eq!(
            enumerate(&[a, b]),
            vec![vec![0, 0], vec![0, 1], vec![1, 0], vec![1, 1]]
        );
    }

    #[test]
    fn enumerate_prunes_a_conflicting_branch() {
        let a = vec![opt("1", word_mask(0)), opt("2", word_mask(1))];
        let b = vec![opt("3", word_mask(0))];
        assert_eq!(enumerate(&[a, b]), vec![vec![1, 0]]);
    }

    #[test]
    fn enumerate_of_no_courses_is_one_empty_schedule() {
        assert_eq!(enumerate(&[]), vec![Vec::<usize>::new()]);
    }

    #[test]
    fn enumerate_with_an_empty_domain_finds_nothing() {
        let a = vec![opt("1", word_mask(0))];
        assert!(enumerate(&[a, Vec::new()]).is_empty());
    }

    #[test]
    fn a_disjoint_pair_is_feasible_and_an_overlapping_one_is_not() {
        let a = || vec![opt("1", word_mask(0))];
        let b = vec![opt("2", word_mask(1))];
        assert!(is_feasible(&[a(), b]));
        assert!(!is_feasible(&[a(), a()]));
    }

    #[test]
    fn no_courses_are_vacuously_feasible() {
        assert!(is_feasible(&[]));
    }

    #[test]
    fn three_pairwise_compatible_courses_can_still_be_infeasible() {
        // the §7 trap: two time windows, three courses — every pair fits,
        // the triple cannot (pigeonhole)
        let domain = || vec![opt("a", word_mask(0)), opt("b", word_mask(1))];
        assert!(is_feasible(&[domain(), domain()]));
        assert!(!is_feasible(&[domain(), domain(), domain()]));
    }

    #[test]
    fn best_schedule_returns_the_first_feasible_combination() {
        // a's first option conflicts with b, so the first feasible leaf
        // skips it
        let a = vec![opt("1", word_mask(0)), opt("2", word_mask(1))];
        let b = vec![opt("3", word_mask(0))];
        let schedule =
            best_schedule(&[a, b]).expect("a feasible combination exists");
        let nrcs: Vec<&str> =
            schedule.nrcs.iter().map(String::as_str).collect();
        assert_eq!(nrcs, ["2", "3"]);
    }

    #[test]
    fn best_schedule_of_an_infeasible_set_is_none() {
        let a = || vec![opt("1", word_mask(0))];
        assert_eq!(best_schedule(&[a(), a()]), None);
    }

    #[test]
    fn best_schedule_of_no_courses_is_the_empty_schedule() {
        let schedule = best_schedule(&[]).expect("vacuously feasible");
        assert!(schedule.nrcs.is_empty());
    }

    // --- schedule_report: the frozen UI contract ---

    fn course(code: &str, options: &str) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{{"fall":{{"options":{options}}}}}}}"#
        ))
        .expect("course literal")
    }

    // one single-section, single-slot enrolment option
    fn option_json(nrc: &str, day: &str, start: &str, end: &str) -> String {
        format!(
            r#"[{{"nrc":"{nrc}","section":"A","mode":"in-person",
                  "slots":[{{"day":"{day}","start":"{start}","end":"{end}"}}]}}]"#
        )
    }

    fn no_pins() -> BTreeMap<String, BTreeSet<String>> {
        BTreeMap::new()
    }

    fn pin(code: &str, nrcs: &[&str]) -> BTreeMap<String, BTreeSet<String>> {
        std::iter::once((
            code.to_string(),
            nrcs.iter().map(|nrc| nrc.to_string()).collect(),
        ))
        .collect()
    }

    #[test]
    fn a_conflict_free_pair_reports_valid_and_omits_every_marker() {
        let courses = [
            course(
                "A-1000",
                &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
            ),
            course(
                "B-1000",
                &format!(
                    "[{}]",
                    option_json("2", "tuesday", "08:30", "11:20")
                ),
            ),
        ];

        let report = schedule_report(&courses, Season::Fall, &no_pins())
            .expect("well-formed input");

        assert!(report.valid);
        assert!(report.courses.iter().all(|course| course.valid));
        // the `valid` key is omitted when true — courses and alternatives
        // alike — while the top-level one is always present
        let json = serde_json::to_value(&report).expect("serialize");
        assert_eq!(json["valid"], serde_json::Value::Bool(true));
        assert!(json["courses"][0].get("valid").is_none(), "{json}");
        assert_eq!(json["courses"][0]["selected"][0]["nrc"], "1");
    }

    #[test]
    fn an_infeasible_pair_keeps_first_options_and_marks_both() {
        let courses = [
            course(
                "A-1000",
                &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
            ),
            course(
                "B-1000",
                &format!("[{}]", option_json("2", "monday", "10:30", "13:20")),
            ),
        ];

        let report = schedule_report(&courses, Season::Fall, &no_pins())
            .expect("well-formed input");

        assert!(!report.valid);
        assert!(report.courses.iter().all(|course| !course.valid));
        let json = serde_json::to_value(&report).expect("serialize");
        assert_eq!(
            json["courses"][0]["valid"],
            serde_json::Value::Bool(false)
        );
    }

    #[test]
    fn the_selection_skips_a_conflicting_option_and_marks_the_alternative() {
        // a's first option collides with b, so the first feasible schedule
        // uses its second; the skipped option, listed as an alternative, is
        // invalid under swap semantics (b never moves)
        let courses = [
            course(
                "A-1000",
                &format!(
                    "[{},{}]",
                    option_json("1", "monday", "08:30", "11:20"),
                    option_json("2", "tuesday", "08:30", "11:20"),
                ),
            ),
            course(
                "B-1000",
                &format!("[{}]", option_json("3", "monday", "08:30", "11:20")),
            ),
        ];

        let report = schedule_report(&courses, Season::Fall, &no_pins())
            .expect("well-formed input");

        assert!(report.valid);
        assert_eq!(report.courses[0].selected[0].nrc, "2");
        assert_eq!(report.courses[0].alternatives.len(), 1);
        assert!(!report.courses[0].alternatives[0].valid);
        let json = serde_json::to_value(&report).expect("serialize");
        assert_eq!(
            json["courses"][0]["alternatives"][0]["valid"],
            serde_json::Value::Bool(false)
        );
    }

    #[test]
    fn a_pinned_option_is_selected_even_when_not_first() {
        let courses = [course(
            "A-1000",
            &format!(
                "[{},{}]",
                option_json("1", "monday", "08:30", "11:20"),
                option_json("2", "tuesday", "08:30", "11:20"),
            ),
        )];

        let report =
            schedule_report(&courses, Season::Fall, &pin("A-1000", &["2"]))
                .expect("well-formed input");

        assert!(report.valid);
        assert_eq!(report.courses[0].selected[0].nrc, "2");
        assert_eq!(report.courses[0].alternatives[0].sections[0].nrc, "1");
    }

    #[test]
    fn a_pinned_conflict_is_kept_and_marked_rather_than_undone() {
        // both pins collide while both second options are free: the
        // student's explicit choice is never undone, so the report keeps
        // the pins, marks both courses, and leaves the alternatives clean
        let mut pins = pin("A-1000", &["1"]);
        pins.append(&mut pin("B-1000", &["3"]));
        let courses = [
            course(
                "A-1000",
                &format!(
                    "[{},{}]",
                    option_json("1", "monday", "08:30", "11:20"),
                    option_json("2", "tuesday", "08:30", "11:20"),
                ),
            ),
            course(
                "B-1000",
                &format!(
                    "[{},{}]",
                    option_json("3", "monday", "08:30", "11:20"),
                    option_json("4", "wednesday", "08:30", "11:20"),
                ),
            ),
        ];

        let report = schedule_report(&courses, Season::Fall, &pins)
            .expect("well-formed input");

        assert!(!report.valid);
        assert_eq!(report.courses[0].selected[0].nrc, "1");
        assert_eq!(report.courses[1].selected[0].nrc, "3");
        assert!(report.courses.iter().all(|course| !course.valid));
        assert!(report.courses.iter().all(|course| {
            course
                .alternatives
                .iter()
                .all(|alternative| alternative.valid)
        }));
    }

    #[test]
    fn a_course_not_offered_in_the_season_is_an_error() {
        let courses = [course(
            "A-1000",
            &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
        )];
        assert_eq!(
            schedule_report(&courses, Season::Winter, &no_pins()),
            Err(ScheduleError::NotOffered {
                code: "A-1000".to_string()
            })
        );
    }

    #[test]
    fn a_course_with_no_option_is_an_error() {
        let courses = [course("A-1000", "[]")];
        assert_eq!(
            schedule_report(&courses, Season::Fall, &no_pins()),
            Err(ScheduleError::NoOptions {
                code: "A-1000".to_string()
            })
        );
    }

    #[test]
    fn a_pin_naming_an_unknown_course_is_an_error() {
        let courses = [course(
            "A-1000",
            &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
        )];
        assert_eq!(
            schedule_report(&courses, Season::Fall, &pin("Z-9999", &["1"])),
            Err(ScheduleError::UnknownChosenCourse {
                code: "Z-9999".to_string()
            })
        );
    }

    #[test]
    fn a_pin_matching_no_option_is_an_error() {
        let courses = [course(
            "A-1000",
            &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
        )];
        assert_eq!(
            schedule_report(&courses, Season::Fall, &pin("A-1000", &["9"])),
            Err(ScheduleError::ChosenOptionAbsent {
                code: "A-1000".to_string()
            })
        );
    }

    #[test]
    fn every_schedule_error_names_its_course() {
        let code = || "A-1000".to_string();
        for error in [
            ScheduleError::NotOffered { code: code() },
            ScheduleError::NoOptions { code: code() },
            ScheduleError::UnknownChosenCourse { code: code() },
            ScheduleError::ChosenOptionAbsent { code: code() },
        ] {
            assert!(error.to_string().contains("A-1000"), "{error}");
        }
    }

    // --- resolve_offering: most recent vintage wins, ties to the course ---

    fn offering_with(options: usize) -> SeasonOffering {
        SeasonOffering {
            options: vec![Vec::new(); options],
        }
    }

    #[test]
    fn resolve_offering_prefers_the_most_recent_vintage() {
        let course = offering_with(1);
        let equivalent = offering_with(2);
        let resolved =
            resolve_offering(Some((&course, 2025)), Some((&equivalent, 2026)));
        assert_eq!(resolved, Some((&equivalent, 2026)));
    }

    #[test]
    fn resolve_offering_ties_go_to_the_course_itself() {
        let course = offering_with(1);
        let equivalent = offering_with(2);
        assert_eq!(
            resolve_offering(Some((&course, 2026)), Some((&equivalent, 2026))),
            Some((&course, 2026))
        );
        assert_eq!(
            resolve_offering(Some((&course, 2026)), Some((&equivalent, 2024))),
            Some((&course, 2026))
        );
    }

    #[test]
    fn resolve_offering_falls_back_when_one_side_is_missing() {
        let offering = offering_with(1);
        assert_eq!(
            resolve_offering(Some((&offering, 2024)), None),
            Some((&offering, 2024))
        );
        assert_eq!(
            resolve_offering(None, Some((&offering, 2024))),
            Some((&offering, 2024))
        );
    }

    #[test]
    fn resolve_offering_of_nothing_is_nothing() {
        assert_eq!(resolve_offering(None, None), None);
    }

    #[test]
    fn resolve_offering_folds_across_several_equivalents() {
        // returning the winning *pair* makes the function foldable when a
        // course lists several equivalents; left-wins-on-tie keeps the
        // course ahead of any equally recent equivalent
        let course = offering_with(1);
        let older = offering_with(2);
        let newer = offering_with(3);
        // the seed is itself a resolution, as a real caller's would be — an
        // accumulator of `None` must keep folding (the next equivalent may
        // win), so `try_fold`'s short-circuit would be wrong here
        let seed = resolve_offering(Some((&course, 2024)), None);
        let resolved = [(&older, 2026u16), (&newer, 2026u16)]
            .into_iter()
            .fold(seed, |acc, pair| resolve_offering(acc, Some(pair)));
        assert_eq!(resolved, Some((&older, 2026)));
    }

    // --- properties ---

    fn arb_mask() -> impl Strategy<Value = WeekMask> {
        proptest::array::uniform32(proptest::num::u64::ANY).prop_map(WeekMask)
    }

    fn arb_nrc() -> impl Strategy<Value = String> {
        // a tiny alphabet so forced NRCs are sometimes present
        "[0-3]"
    }

    fn arb_domain() -> impl Strategy<Value = Vec<Opt>> {
        proptest::collection::vec(
            (proptest::collection::btree_set(arb_nrc(), 0..4), arb_mask())
                .prop_map(|(nrc_set, mask)| Opt { nrc_set, mask }),
            0..5,
        )
    }

    // masks confined to one random word: overlaps happen often enough to
    // exercise pruning, misses often enough to exercise feasible leaves —
    // fully random 2016-bit masks would make every pair collide
    fn arb_word_mask() -> impl Strategy<Value = WeekMask> {
        (0usize..32, proptest::num::u64::ANY).prop_map(|(word, bits)| {
            let mut words = [0u64; 32];
            words[word] = bits;
            WeekMask(words)
        })
    }

    fn arb_search_domain() -> impl Strategy<Value = Vec<Opt>> {
        proptest::collection::vec(
            (arb_nrc(), arb_word_mask()).prop_map(|(nrc, mask)| Opt {
                nrc_set: std::iter::once(nrc).collect(),
                mask,
            }),
            0..4,
        )
    }

    fn arb_search_courses() -> impl Strategy<Value = Vec<Vec<Opt>>> {
        proptest::collection::vec(arb_search_domain(), 0..4)
    }

    proptest! {
        #[test]
        fn every_enumerated_leaf_is_conflict_free(
            courses in arb_search_courses(),
        ) {
            for leaf in enumerate(&courses) {
                let chosen: Vec<&Opt> = courses
                    .iter()
                    .zip(&leaf)
                    .map(|(options, &k)| &options[k])
                    .collect();
                for (i, first) in chosen.iter().enumerate() {
                    for second in &chosen[i + 1..] {
                        prop_assert!(!first.mask.overlaps(&second.mask));
                    }
                }
            }
        }

        #[test]
        fn the_feasibility_verdict_matches_the_full_enumeration(
            courses in arb_search_courses(),
        ) {
            let feasible = is_feasible(&courses);
            prop_assert_eq!(feasible, !enumerate(&courses).is_empty());
            prop_assert_eq!(feasible, best_schedule(&courses).is_some());
        }

        #[test]
        fn adding_a_course_never_makes_an_infeasible_set_feasible(
            courses in arb_search_courses(),
            extra in arb_search_domain(),
        ) {
            if !is_feasible(&courses) {
                let mut extended = courses;
                extended.push(extra);
                prop_assert!(!is_feasible(&extended));
            }
        }

        #[test]
        fn a_domain_from_slotless_sections_never_overlaps_anything(
            options in proptest::collection::vec(
                proptest::collection::vec(arb_nrc(), 0..3),
                0..4,
            ),
            busy in arb_mask(),
        ) {
            use crate::course::{Mode, Section};
            let offering = SeasonOffering {
                options: options
                    .into_iter()
                    .map(|nrcs| {
                        nrcs.into_iter()
                            .map(|nrc| Section {
                                nrc,
                                section: None,
                                mode: Mode::Remote,
                                slots: Vec::new(),
                            })
                            .collect()
                    })
                    .collect(),
            };
            for opt in build_domain(&offering) {
                prop_assert!(opt.mask.is_empty());
                prop_assert!(!opt.mask.overlaps(&busy));
            }
        }

        #[test]
        fn force_nrc_returns_a_subset_of_the_domain(
            domain in arb_domain(),
            nrc in arb_nrc(),
        ) {
            let forced = force_nrc(domain.clone(), &nrc);
            for opt in &forced {
                prop_assert!(opt.nrc_set.contains(&nrc));
                prop_assert!(domain.contains(opt));
            }
        }

        #[test]
        fn forcing_the_same_nrc_twice_changes_nothing(
            domain in arb_domain(),
            nrc in arb_nrc(),
        ) {
            let once = force_nrc(domain, &nrc);
            prop_assert_eq!(force_nrc(once.clone(), &nrc), once);
        }
    }
}
