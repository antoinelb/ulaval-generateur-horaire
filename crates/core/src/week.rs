use crate::course::{Day, Slot, Time};

// A week in 5-minute buckets: 7 days × 288. Necessary and sufficient
// granularity, measured over every slot in `data/cours/` (2026-07-28,
// 24 042 boundaries): 299 fall off a 10-minute grid, and only the 3
// « 23:59 » ends fall off the 5-minute one — absorbed by the outward
// rounding below (ADR `2026-07-encodage-semaine-en-seaux-de-5-minutes`).
const BUCKETS_PER_DAY: usize = 288;
const WORD_BITS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekMask(pub(crate) [u64; 32]);

impl WeekMask {
    pub const EMPTY: WeekMask = WeekMask([0; 32]);

    pub fn is_empty(&self) -> bool {
        *self == Self::EMPTY
    }

    pub fn overlaps(&self, other: &WeekMask) -> bool {
        self.0.iter().zip(&other.0).any(|(a, b)| a & b != 0)
    }

    pub fn merge(&self, other: &WeekMask) -> WeekMask {
        WeekMask(std::array::from_fn(|i| self.0[i] | other.0[i]))
    }
}

// Union of the occupied buckets of every slot; no slots (a remote section)
// is the empty mask. Bucket by bucket rather than word-masked arithmetic:
// a real slot spans at most ~36 buckets and masks are built once per
// option, so obvious beats clever here.
pub fn slots_to_mask(slots: &[Slot]) -> WeekMask {
    slots
        .iter()
        .flat_map(slot_buckets)
        .fold(WeekMask::EMPTY, set_bucket)
}

// The half-open bucket range [start, end) of one slot. An inverted slot
// (start >= end, which nothing upstream forbids) is the empty range, not
// an error. Off-grid minutes round outward — floor the start, ceil the
// end — so occupied time is never reported free.
fn slot_buckets(slot: &Slot) -> std::ops::Range<usize> {
    let day = day_index(slot.day) * BUCKETS_PER_DAY;
    (day + start_bucket(slot.start))..(day + end_bucket(slot.end))
}

fn day_index(day: Day) -> usize {
    match day {
        Day::Monday => 0,
        Day::Tuesday => 1,
        Day::Wednesday => 2,
        Day::Thursday => 3,
        Day::Friday => 4,
        Day::Saturday => 5,
        Day::Sunday => 6,
    }
}

fn start_bucket(t: Time) -> usize {
    t.hour as usize * 12 + t.minute as usize / 5
}

fn end_bucket(t: Time) -> usize {
    t.hour as usize * 12 + (t.minute as usize).div_ceil(5)
}

fn set_bucket(mask: WeekMask, bucket: usize) -> WeekMask {
    let mut words = mask.0;
    words[bucket / WORD_BITS] |= 1u64 << (bucket % WORD_BITS);
    WeekMask(words)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::course::{Day, Time};
    use proptest::prelude::*;

    fn slot(day: Day, start: (u8, u8), end: (u8, u8)) -> Slot {
        Slot {
            day,
            start: Time {
                hour: start.0,
                minute: start.1,
            },
            end: Time {
                hour: end.0,
                minute: end.1,
            },
        }
    }

    fn bit(mask: &WeekMask, bucket: usize) -> bool {
        mask.0[bucket / 64] & (1u64 << (bucket % 64)) != 0
    }

    fn popcount(mask: &WeekMask) -> u32 {
        mask.0.iter().map(|word| word.count_ones()).sum()
    }

    // --- WeekMask: empty, merge, overlaps ---

    #[test]
    fn empty_mask_has_no_bits_and_never_overlaps() {
        assert!(WeekMask::EMPTY.is_empty());
        assert!(!WeekMask::EMPTY.overlaps(&WeekMask::EMPTY));

        let mut words = [0u64; 32];
        words[7] = 0b1010;
        let busy = WeekMask(words);
        assert!(!busy.is_empty());
        assert!(!busy.overlaps(&WeekMask::EMPTY));
        assert!(!WeekMask::EMPTY.overlaps(&busy));
    }

    #[test]
    fn merge_unions_bits_and_overlap_sees_both_operands() {
        // bits in different words, so word-wise AND/OR is exercised beyond
        // word zero
        let mut low = [0u64; 32];
        low[0] = 1;
        let mut high = [0u64; 32];
        high[31] = 1 << 5;
        let (low, high) = (WeekMask(low), WeekMask(high));

        assert!(!low.overlaps(&high));
        let merged = low.merge(&high);
        assert!(bit(&merged, 0));
        assert!(bit(&merged, 31 * 64 + 5));
        assert_eq!(popcount(&merged), 2);
        assert!(low.overlaps(&merged));
        assert!(high.overlaps(&merged));
    }

    // --- slots_to_mask: bucket layout ---

    #[test]
    fn a_single_slot_sets_its_five_minute_buckets() {
        // GCI-1007's real lecture, NRC 84664: friday 12:30-15:20 is 170
        // minutes = 34 buckets, at friday(4)*288 + 12*12 + 30/5 = 1302
        let mask = slots_to_mask(&[slot(Day::Friday, (12, 30), (15, 20))]);
        assert!(!bit(&mask, 1301));
        assert!(bit(&mask, 1302));
        assert!(bit(&mask, 1335));
        assert!(!bit(&mask, 1336));
        assert_eq!(popcount(&mask), 34);
    }

    #[test]
    fn morning_and_afternoon_slots_on_one_day_do_not_overlap() {
        // GCI-1007's two friday meetings: lab B then the lecture
        let morning = slots_to_mask(&[slot(Day::Friday, (8, 30), (11, 20))]);
        let afternoon =
            slots_to_mask(&[slot(Day::Friday, (12, 30), (15, 20))]);
        assert!(!morning.overlaps(&afternoon));
    }

    #[test]
    fn same_time_on_different_days_does_not_overlap() {
        // GCI-1007's lecture and lab A share a time, not a day
        let friday = slots_to_mask(&[slot(Day::Friday, (12, 30), (15, 20))]);
        let wednesday =
            slots_to_mask(&[slot(Day::Wednesday, (12, 30), (15, 20))]);
        assert!(!friday.overlaps(&wednesday));
    }

    #[test]
    fn touching_slots_do_not_overlap_at_the_half_open_boundary() {
        // back-to-back courses share an instant, not a bucket: [a, b) then
        // [b, c) — the schedules fixture `back-to-back.json` relies on this
        let first = slots_to_mask(&[slot(Day::Monday, (8, 30), (11, 20))]);
        let second = slots_to_mask(&[slot(Day::Monday, (11, 20), (14, 20))]);
        assert!(!first.overlaps(&second));
    }

    #[test]
    fn a_slot_range_crosses_a_word_boundary() {
        // monday 05:15-05:30 is buckets 63..66: one bit in word 0, two in
        // word 1 — a mask built word by word would drop one side
        let mask = slots_to_mask(&[slot(Day::Monday, (5, 15), (5, 30))]);
        assert!(bit(&mask, 63));
        assert!(bit(&mask, 64));
        assert!(bit(&mask, 65));
        assert_eq!(popcount(&mask), 3);
    }

    #[test]
    fn sunday_last_bucket_is_the_final_bit() {
        // the highest reachable bucket: sunday(6)*288 + 23*12 + 55/5 = 2015,
        // the last bit of the last word — one further would index out of
        // bounds
        let mask = slots_to_mask(&[slot(Day::Sunday, (23, 55), (23, 59))]);
        assert!(bit(&mask, 2015));
        assert_eq!(popcount(&mask), 1);
    }

    #[test]
    fn each_day_maps_to_its_own_bucket_range() {
        let days = [
            Day::Monday,
            Day::Tuesday,
            Day::Wednesday,
            Day::Thursday,
            Day::Friday,
            Day::Saturday,
            Day::Sunday,
        ];
        let masks: Vec<WeekMask> = days
            .iter()
            .map(|day| slots_to_mask(&[slot(*day, (8, 30), (11, 20))]))
            .collect();
        for (i, a) in masks.iter().enumerate() {
            assert_eq!(popcount(a), 34);
            for b in masks.iter().skip(i + 1) {
                assert!(!a.overlaps(b), "days {i} and later should not meet");
            }
        }
    }

    #[test]
    fn minutes_off_the_five_minute_grid_round_outward() {
        // 299 real boundaries sit off the 10-minute grid and 3 off the
        // 5-minute one (all « 23:59 », SIN-3150) : the start floors, the end
        // ceils, so occupied time is never reported free
        let off_grid = slots_to_mask(&[slot(Day::Monday, (12, 32), (13, 3))]);
        let on_grid = slots_to_mask(&[slot(Day::Monday, (12, 30), (13, 5))]);
        assert_eq!(off_grid, on_grid);
    }

    #[test]
    fn an_inverted_slot_contributes_no_bits() {
        // nothing in `core` enforces start < end on `Slot`; an inverted
        // range is the empty half-open interval, not a panic
        let mask = slots_to_mask(&[slot(Day::Monday, (15, 0), (12, 0))]);
        assert!(mask.is_empty());
    }

    #[test]
    fn no_slots_gives_the_empty_mask() {
        // a remote section carries no slots and must never conflict
        assert!(slots_to_mask(&[]).is_empty());
    }

    // --- properties ---

    fn arb_mask() -> impl Strategy<Value = WeekMask> {
        proptest::array::uniform32(proptest::num::u64::ANY).prop_map(WeekMask)
    }

    fn grid_time(bucket: usize) -> Time {
        Time {
            hour: (bucket * 5 / 60) as u8,
            minute: (bucket * 5 % 60) as u8,
        }
    }

    fn arb_slot() -> impl Strategy<Value = Slot> {
        let days = vec![
            Day::Monday,
            Day::Tuesday,
            Day::Wednesday,
            Day::Thursday,
            Day::Friday,
            Day::Saturday,
            Day::Sunday,
        ];
        (proptest::sample::select(days), 0usize..287).prop_flat_map(
            |(day, start)| {
                ((start + 1)..288).prop_map(move |end| Slot {
                    day,
                    start: grid_time(start),
                    end: grid_time(end),
                })
            },
        )
    }

    proptest! {
        #[test]
        fn overlaps_is_symmetric(a in arb_mask(), b in arb_mask()) {
            prop_assert_eq!(a.overlaps(&b), b.overlaps(&a));
        }

        #[test]
        fn merge_is_commutative(a in arb_mask(), b in arb_mask()) {
            prop_assert_eq!(a.merge(&b), b.merge(&a));
        }

        #[test]
        fn merge_is_associative(
            a in arb_mask(),
            b in arb_mask(),
            c in arb_mask(),
        ) {
            prop_assert_eq!(a.merge(&b).merge(&c), a.merge(&b.merge(&c)));
        }

        #[test]
        fn merge_with_empty_is_identity(a in arb_mask()) {
            prop_assert_eq!(a.merge(&WeekMask::EMPTY), a);
        }

        #[test]
        fn nothing_overlaps_the_empty_mask(a in arb_mask()) {
            prop_assert!(!a.overlaps(&WeekMask::EMPTY));
        }

        #[test]
        fn a_slot_mask_covers_exactly_its_duration(s in arb_slot()) {
            let minutes = (s.end.hour as u32 * 60 + s.end.minute as u32)
                - (s.start.hour as u32 * 60 + s.start.minute as u32);
            prop_assert_eq!(
                popcount(&slots_to_mask(std::slice::from_ref(&s))),
                minutes / 5
            );
        }

        #[test]
        fn slots_to_mask_equals_the_merge_of_each_slot_alone(
            slots in proptest::collection::vec(arb_slot(), 0..6),
        ) {
            let merged = slots
                .iter()
                .map(|s| slots_to_mask(std::slice::from_ref(s)))
                .fold(WeekMask::EMPTY, |acc, m| acc.merge(&m));
            prop_assert_eq!(slots_to_mask(&slots), merged);
        }
    }
}
