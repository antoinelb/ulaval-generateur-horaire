use std::collections::BTreeMap;

use crate::course::Season;
use crate::week::WeekMask;
use crate::weekly::masks_feasible;

// the width of the members bitset: the placement search rejects course
// lists past this bound before ever consulting the cache
pub const MAX_MEMBERS: usize = 128;

// A as a memoized oracle (conception §5.1) : a term's weekly feasibility is
// a pure function of (season, set of courses) — the snapshot keeps one
// offering per season (its freshest, dated by `last_offered`), so the
// verdict holds for every session of that season, next term or two years
// out. Courses are named by the search's candidate indices packed in a
// `u128` : a `Copy` key, no string set built or cloned per probe.
#[derive(Debug, Default)]
pub struct FeasibilityCache {
    verdicts: BTreeMap<(Season, u128), bool>,
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

    // The A-veto for one term. `mask_of` maps a member index to its option
    // masks for this season, consulted only on a miss; an empty slice (no
    // offering) admits no combination, hence « infeasible » : B restricts
    // domains to offered seasons before calling, so that arm only turns an
    // upstream inconsistency into a loud verdict instead of silently
    // enrolling in nothing.
    pub fn term_feasible<'a>(
        &mut self,
        season: Season,
        members: u128,
        mask_of: impl Fn(usize) -> &'a [WeekMask],
    ) -> bool {
        if let Some(&verdict) = self.verdicts.get(&(season, members)) {
            return verdict;
        }
        let domains: Vec<&[WeekMask]> = (0..MAX_MEMBERS)
            .filter(|index| members >> index & 1 == 1)
            .map(mask_of)
            .collect();
        let verdict = masks_feasible(&domains);
        self.verdicts.insert((season, members), verdict);
        verdict
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::course::{Day, Slot, Time};
    use crate::week::slots_to_mask;

    fn day_mask(day: Day) -> Vec<WeekMask> {
        vec![slots_to_mask(&[Slot {
            day,
            start: Time {
                hour: 8,
                minute: 30,
            },
            end: Time {
                hour: 11,
                minute: 20,
            },
        }])]
    }

    #[test]
    fn a_disjoint_term_is_feasible_and_a_clashing_one_is_not() {
        // members 0 and 2 share monday; member 1 sits on tuesday
        let masks = [
            day_mask(Day::Monday),
            day_mask(Day::Tuesday),
            day_mask(Day::Monday),
        ];
        let mut cache = FeasibilityCache::new();
        assert!(cache.term_feasible(Season::Fall, 0b011, |i| &masks[i]));
        assert!(!cache.term_feasible(Season::Fall, 0b101, |i| &masks[i]));
    }

    #[test]
    fn the_same_term_set_is_computed_only_once() {
        let masks = [day_mask(Day::Monday), day_mask(Day::Tuesday)];
        let mut cache = FeasibilityCache::new();
        let first = cache.term_feasible(Season::Fall, 0b11, |i| &masks[i]);
        let again = cache.term_feasible(Season::Fall, 0b11, |i| &masks[i]);
        assert_eq!(first, again);
        assert_eq!(cache.computed(), 1);
    }

    #[test]
    fn different_seasons_are_distinct_verdicts() {
        // one offering per season: the verdict is keyed by season, so the
        // same member set computes once per season, never across
        let fall = [day_mask(Day::Monday)];
        let winter: [Vec<WeekMask>; 1] = [Vec::new()];
        let mut cache = FeasibilityCache::new();
        assert!(cache.term_feasible(Season::Fall, 0b1, |i| &fall[i]));
        // not offered in winter: empty mask list, loud « infeasible »
        assert!(!cache.term_feasible(Season::Winter, 0b1, |i| &winter[i]));
        assert_eq!(cache.computed(), 2);
    }

    #[test]
    fn an_unknown_schedule_is_feasible_alongside_anything() {
        // a course whose schedule is not yet published contributes one
        // placeholder occupying nothing (ADR
        // `2026-07-cours-sans-section-de-session-offert-automne-hiver`) —
        // it never vetoes a term
        let masks = [day_mask(Day::Monday), vec![WeekMask::EMPTY]];
        let mut cache = FeasibilityCache::new();
        assert!(cache.term_feasible(Season::Fall, 0b11, |i| &masks[i]));
    }

    #[test]
    fn an_empty_term_is_vacuously_feasible() {
        let mut cache = FeasibilityCache::new();
        assert!(cache.term_feasible(Season::Fall, 0, |_| &[]));
    }
}
