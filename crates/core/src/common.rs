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

// --- sigles -----------------------------------------------------------

// Sigle extraction out of French prose. It lives here rather than in the
// parser because the language rule widens itself against the catalogue with
// no HTML in sight, and `core` compiles without the `parser` feature for the
// wasm build. The « LLL-DDDD » shape itself is `prereq_parse`'s.

// the first « LLL-DDDD » sigle in the sentence — the course to pass
pub fn first_course_code(text: &str) -> Option<String> {
    course_code_tokens(text).next().map(str::to_string)
}

// every sigle of the sentence, in the order it names them — in the stage
// prose the mandatory stage comes first, the optional ones after
pub fn course_codes(text: &str) -> Vec<String> {
    course_code_tokens(text).map(str::to_string).collect()
}

// punctuation clings to a sigle in prose (« du cours FLS-2093, requis »):
// trimmed off both ends before the shape is tested
fn course_code_tokens(text: &str) -> impl Iterator<Item = &str> {
    use crate::prereq_parse::is_course_code;
    text.split_whitespace()
        .map(|token| token.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
        .filter(|token| is_course_code(token))
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

    #[test]
    fn first_course_code_finds_the_sigle_or_nothing() {
        assert_eq!(
            first_course_code("Réussir le cours ANL-2020 Intermediate"),
            Some("ANL-2020".to_string())
        );
        // trailing punctuation is trimmed off the token
        assert_eq!(
            first_course_code("… du cours FLS-2093, requis."),
            Some("FLS-2093".to_string())
        );
        // a dash alone is not a LLL-DDDD sigle
        assert_eq!(first_course_code("(TCF-TP/ÉÉ: 14)"), None);
        assert_eq!(first_course_code("aucun sigle ici"), None);
    }

    #[test]
    fn course_codes_keeps_every_sigle_in_order() {
        assert_eq!(
            course_codes(
                "ANL-2020 ou ANL-3010, sinon FLS-2093 — pas LAN-GUES."
            ),
            vec![
                "ANL-2020".to_string(),
                "ANL-3010".to_string(),
                "FLS-2093".to_string()
            ]
        );
        assert!(course_codes("aucun sigle ici").is_empty());
    }
}
