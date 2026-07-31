use std::collections::{BTreeMap, BTreeSet};

use crate::course::{Course, Season};
use crate::weekly::{build_domain, is_feasible};

// A as a memoized oracle (conception §5.1) : a term's weekly feasibility is
// a pure function of (season, set of course codes) — the snapshot keeps one
// offering per season (its freshest, dated by `last_offered`), so the
// verdict holds for every session of that season, next term or two years
// out.
#[derive(Debug, Default)]
pub struct FeasibilityCache {
    verdicts: BTreeMap<(Season, BTreeSet<String>), bool>,
}

impl FeasibilityCache {
    pub fn new() -> Self {
        Self::default()
    }

    // how many verdicts were actually computed — the hook the memoization
    // tests count instead of trusting an internal call counter
    pub fn computed(&self) -> usize {
        self.verdicts.len()
    }

    // The A-veto for one term. A code without a Course or without an
    // offering in the season yields an empty domain, hence « infeasible » :
    // B restricts domains to offered seasons before calling, so this arm
    // only turns an upstream inconsistency into a loud verdict instead of
    // silently enrolling in nothing.
    pub fn term_feasible(
        &mut self,
        season: Season,
        codes: &BTreeSet<String>,
        by_code: &BTreeMap<&str, &Course>,
    ) -> bool {
        let key = (season, codes.clone());
        if let Some(&verdict) = self.verdicts.get(&key) {
            return verdict;
        }
        let domains: Vec<_> = codes
            .iter()
            .map(|code| {
                by_code
                    .get(code.as_str())
                    .and_then(|course| course.seasons.get(&season))
                    .map(build_domain)
                    .unwrap_or_default()
            })
            .collect();
        let verdict = is_feasible(&domains);
        self.verdicts.insert(key, verdict);
        verdict
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn course(code: &str, day: &str) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{{"fall":{{"last_offered":2026,"options":[[
                   {{"nrc":"1","section":"A","mode":"in-person",
                     "slots":[{{"day":"{day}","start":"08:30",
                                "end":"11:20"}}]}}]]}}}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    fn codes(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn by_code(courses: &[Course]) -> BTreeMap<&str, &Course> {
        courses
            .iter()
            .map(|course| (course.code.as_str(), course))
            .collect()
    }

    #[test]
    fn a_disjoint_term_is_feasible_and_a_clashing_one_is_not() {
        let courses = [
            course("A-1", "monday"),
            course("B-2", "tuesday"),
            course("C-3", "monday"),
        ];
        let mut cache = FeasibilityCache::new();
        assert!(cache.term_feasible(
            Season::Fall,
            &codes(&["A-1", "B-2"]),
            &by_code(&courses),
        ));
        assert!(!cache.term_feasible(
            Season::Fall,
            &codes(&["A-1", "C-3"]),
            &by_code(&courses),
        ));
    }

    #[test]
    fn the_same_term_set_is_computed_only_once() {
        let courses = [course("A-1", "monday"), course("B-2", "tuesday")];
        let mut cache = FeasibilityCache::new();
        let set = codes(&["A-1", "B-2"]);
        let first =
            cache.term_feasible(Season::Fall, &set, &by_code(&courses));
        let again =
            cache.term_feasible(Season::Fall, &set, &by_code(&courses));
        assert_eq!(first, again);
        assert_eq!(cache.computed(), 1);
    }

    #[test]
    fn different_seasons_are_distinct_verdicts() {
        // one snapshot per season: the verdict is keyed by season, so the
        // same set computes once per season, never across
        let courses = [course("A-1", "monday")];
        let mut cache = FeasibilityCache::new();
        let set = codes(&["A-1"]);
        assert!(cache.term_feasible(Season::Fall, &set, &by_code(&courses)));
        // not offered in winter: empty domain, loud « infeasible »
        assert!(!cache.term_feasible(
            Season::Winter,
            &set,
            &by_code(&courses)
        ));
        assert_eq!(cache.computed(), 2);
    }

    #[test]
    fn a_code_without_a_course_is_loudly_infeasible() {
        let courses = [course("A-1", "monday")];
        let mut cache = FeasibilityCache::new();
        assert!(!cache.term_feasible(
            Season::Fall,
            &codes(&["Z-9"]),
            &by_code(&courses)
        ));
    }

    #[test]
    fn an_unknown_schedule_is_feasible_alongside_anything() {
        // the new-course rule: `options: null` builds a placeholder domain
        // that occupies nothing, so the course places with any set — the
        // whole point of keeping GCI-1011-shaped courses in the snapshot
        let mut courses = vec![course("A-1", "monday")];
        courses.push(
            serde_json::from_str(
                r#"{"code":"N-1011","title":"T","credits":3,"cycle":1,
                    "prerequisites":null,"equivalents":[],
                    "seasons":{"fall":{"last_offered":null,
                                       "options":null}}}"#,
            )
            .unwrap_or_else(|e| panic!("course literal: {e}")),
        );
        let mut cache = FeasibilityCache::new();
        assert!(cache.term_feasible(
            Season::Fall,
            &codes(&["A-1", "N-1011"]),
            &by_code(&courses),
        ));
    }

    #[test]
    fn an_empty_term_is_vacuously_feasible() {
        let mut cache = FeasibilityCache::new();
        assert!(cache.term_feasible(
            Season::Fall,
            &BTreeSet::new(),
            &BTreeMap::new()
        ));
    }
}
