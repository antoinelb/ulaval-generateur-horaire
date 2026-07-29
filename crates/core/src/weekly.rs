use std::collections::BTreeSet;

use crate::course::{SeasonOffering, Section};
use crate::week::{slots_to_mask, WeekMask};

// One enrolment alternative of a course: the NRC of every section taken
// together, and the union of their occupied buckets. The set is ordered so
// a chosen schedule serializes deterministically (URL sharing later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opt {
    pub nrc_set: BTreeSet<String>,
    pub mask: WeekMask,
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

    proptest! {
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
