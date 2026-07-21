#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "u8", into = "u8")]
pub enum Cycle {
    First,
    Second,
}

impl TryFrom<u8> for Cycle {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Cycle::First),
            2 => Ok(Cycle::Second),
            other => Err(format!("invalid level : {other}")),
        }
    }
}

impl From<Cycle> for u8 {
    fn from(c: Cycle) -> u8 {
        match c {
            Cycle::First => 1,
            Cycle::Second => 2,
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn cycle_deserializes_valid_levels() {
        let first: Cycle = serde_json::from_str("1").expect("level 1");
        let second: Cycle = serde_json::from_str("2").expect("level 2");
        assert_eq!(first, Cycle::First);
        assert_eq!(second, Cycle::Second);
    }

    #[test]
    fn cycle_serializes_back_to_u8() {
        assert_eq!(serde_json::to_string(&Cycle::First).expect("ser"), "1");
        assert_eq!(serde_json::to_string(&Cycle::Second).expect("ser"), "2");
    }

    #[test]
    fn cycle_rejects_out_of_range() {
        // a programme's cycle is first or second only: rejecting 0 means it
        // can never be préuniversitaire — that level lives on `CourseCycle`
        // (ADR `2026-07-cycle-preuniversitaire-cours-seulement`)
        assert!(serde_json::from_str::<Cycle>("0").is_err());
        assert!(serde_json::from_str::<Cycle>("3").is_err());
    }
}
